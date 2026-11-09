//! What happens when things go wrong during a transfer.

mod support;

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use offloader_core::{CopyMode, Event, FileFailure, FileStatus, Job, JobSpec, Outcome, Pass, Stage};
use support::*;
use tempfile::tempdir;

fn name_of(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().into_owned()
}

struct FlipByteBeforeVerify(&'static str);
impl offloader_core::Faults for FlipByteBeforeVerify {
    fn before_verify(&self, partial: &Path) {
        if name_of(partial).starts_with(self.0) {
            let mut f = OpenOptions::new().read(true).write(true).open(partial).unwrap();
            let mut b = [0u8];
            f.seek(SeekFrom::Start(1000)).unwrap();
            f.read_exact(&mut b).unwrap();
            f.seek(SeekFrom::Start(1000)).unwrap();
            f.write_all(&[b[0] ^ 0xFF]).unwrap();
            f.sync_all().unwrap();
        }
    }
}

#[test]
fn corruption_at_the_destination_is_caught_and_holds_the_backup() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let (copy, backup) = (tmp.path().join("copy"), tmp.path().join("backup"));
    let spec = JobSpec {
        faults: Some(Arc::new(FlipByteBeforeVerify("DSC_0002.MOV"))),
        ..spec_with_backup(&card.root, &copy, &backup, CopyMode::Sequential, true)
    };
    let job = Job::prepare(spec).unwrap();
    let (report, events) = run(&job);

    let bad = file_index(&job, "DCIM/100NCZ_R/DSC_0002.MOV");
    assert_eq!(report.outcome, Outcome::Failed { files: 1 });
    assert!(!report.source_released, "a card with a failed file must not be released");
    assert!(matches!(&report.files[bad].dests[0], FileStatus::Failed { failure: FileFailure::Mismatch { .. } }));
    assert!(!copy.join("DCIM/100NCZ_R/DSC_0002.MOV").exists(), "a failed file must not get its final name");
    assert!(!list_files(&copy).iter().any(|p| p.ends_with("DSC_0002.MOV.partial")));
    // Everything else verified; the backup never started.
    let others_ok = report.files.iter().enumerate().filter(|(i, _)| *i != bad).all(|(_, f)| f.dests[0].is_verified());
    assert!(others_ok);
    assert!(report.files.iter().all(|f| matches!(f.dests[1], FileStatus::Pending)));
    assert!(!events.iter().any(|e| matches!(e, Event::PassStarted { pass: Pass::Backup, .. } | Event::SourceReleased)));
    assert!(list_files(&backup).is_empty());
}

#[test]
fn a_source_that_changes_mid_copy_fails_that_file() {
    struct GrowMidCopy(AtomicBool);
    impl offloader_core::Faults for GrowMidCopy {
        fn after_source_read(&self, src: &Path, offset: u64) {
            if name_of(src) == "DSC_0002.MOV" && offset >= CHUNK as u64 && !self.0.swap(true, Ordering::SeqCst) {
                let mut f = OpenOptions::new().append(true).open(src).unwrap();
                f.write_all(b"more data").unwrap();
            }
        }
    }
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let job = Job::prepare(JobSpec {
        faults: Some(Arc::new(GrowMidCopy(AtomicBool::new(false)))),
        ..spec(&card.root, &dest)
    })
    .unwrap();
    let (report, _) = run(&job);

    let i = file_index(&job, "DCIM/100NCZ_R/DSC_0002.MOV");
    assert!(matches!(&report.files[i].dests[0], FileStatus::Failed { failure: FileFailure::SourceChanged { .. } }));
    assert_eq!(report.outcome, Outcome::Failed { files: 1 });
    assert!(!dest.join("DCIM/100NCZ_R/DSC_0002.MOV").exists());
    assert!(!list_files(&dest).iter().any(|p| p.ends_with("DSC_0002.MOV.partial")));
}

#[test]
fn a_card_read_error_fails_that_file_and_the_rest_carry_on() {
    struct BadSector;
    impl offloader_core::Faults for BadSector {
        fn source_read_error(&self, src: &Path, offset: u64) -> Option<io::Error> {
            (name_of(src) == "DSC_0004.NEF" && offset >= CHUNK as u64).then(|| io::Error::other("simulated bad sector"))
        }
    }
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let job = Job::prepare(JobSpec { faults: Some(Arc::new(BadSector)), ..spec(&card.root, &dest) }).unwrap();
    let (report, _) = run(&job);

    let i = file_index(&job, "DCIM/100NCZ_R/DSC_0004.NEF");
    assert!(matches!(&report.files[i].dests[0], FileStatus::Failed { failure: FileFailure::SourceRead { .. } }));
    assert_eq!(report.outcome, Outcome::Failed { files: 1 });
    assert_eq!(report.verified_count(0), card.files.len() - 1);
    assert!(!list_files(&dest).iter().any(|p| p.contains("DSC_0004")));
}

#[test]
fn unplugging_the_card_aborts_and_keeps_verified_files() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let unplugged = tmp.path().join("card-unplugged");
    let job = Job::prepare(spec(&card.root, &dest)).unwrap();

    let report = job.run(&AtomicBool::new(false), &|e| {
        if matches!(e, Event::FileStarted { file: 2, .. }) {
            fs::rename(&card.root, &unplugged).unwrap();
        }
    });
    assert!(
        matches!(&report.outcome, Outcome::Aborted { reason } if reason.contains("disconnected")),
        "{:?}",
        report.outcome
    );
    assert!(report.files[0].dests[0].is_verified() && report.files[1].dests[0].is_verified());
    assert!(report.files[3..].iter().all(|f| matches!(f.dests[0], FileStatus::Pending)));
    assert!(!list_files(&dest).iter().any(|p| p.contains(".partial") && !p.ends_with("A.MOV.partial")));
}

#[test]
fn cancelling_keeps_verified_files_and_removes_the_partial_one() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let job = Job::prepare(spec(&card.root, &dest)).unwrap();
    let big = file_index(&job, "DCIM/100NCZ_R/DSC_0002.MOV");

    // Cancel part-way through copying the 16 MB file.
    let cancel = AtomicBool::new(false);
    let report = job.run(&cancel, &|e| {
        if let Event::Progress { file, stage: Stage::Copy, file_bytes, .. } = e
            && *file == big
            && *file_bytes >= CHUNK as u64
        {
            cancel.store(true, Ordering::SeqCst);
        }
    });
    assert_eq!(report.outcome, Outcome::Cancelled);
    let plan = job.plan();
    for (i, f) in plan.files.iter().enumerate() {
        let exists = dest.join(&f.dest_rel).exists();
        assert_eq!(exists, i < big, "{}", f.dest_rel.display());
        assert_eq!(report.files[i].dests[0].is_verified(), i < big);
    }
    assert!(!list_files(&dest).iter().any(|p| p.contains(".partial")), "partial left behind");
}
