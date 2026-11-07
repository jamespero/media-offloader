//! End-to-end engine tests against generated fake cards.

mod support;

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use md5::{Digest, Md5};
use offloader_core::hash::{self, Hasher};
use offloader_core::{
    CopyMode, Event, FileFailure, FileStatus, HashConfig, Job, JobSpec, Outcome, Pass, PrepareError, Stage,
};
use support::*;
use tempfile::tempdir;

fn name_of(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().into_owned()
}

#[test]
fn copies_every_file_and_folder_and_verifies() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("shuttle/Jobs/Day1");
    let job = Job::prepare(spec(&card.root, &dest)).unwrap();

    assert_eq!(job.plan().files.len(), card.files.len());
    assert_eq!(job.destinations()[0].root, dest);
    assert_eq!(job.destinations()[0].conflict, None);

    let (report, events) = run(&job);
    assert_eq!(report.outcome, Outcome::Verified);
    assert!(report.source_released);
    assert!(report.files.iter().all(|f| f.dests[0].is_verified()));
    assert_copied(&card, &dest);

    // Junk was skipped and nothing else was.
    for j in &card.junk {
        assert!(!dest.join(j).exists(), "{j} should be skipped");
    }
    assert_eq!(job.plan().skipped.len(), 6, "{:?}", job.plan().skipped);

    // Hashes are real: compare with an independent hash of each source file.
    for (i, f) in job.plan().files.iter().enumerate() {
        let mut h = Hasher::new(HashConfig::default());
        h.update(&fs::read(card.root.join(&f.src_rel)).unwrap());
        assert_eq!(report.files[i].hashes.unwrap().xxh64, h.finish().xxh64);
    }

    // Progress never goes backwards or past the end, and every file finished.
    let mut last = 0;
    let mut finished = 0;
    for e in &events {
        match e {
            Event::Progress { done, total, .. } => {
                assert!(*done >= last && done <= total);
                last = *done;
            }
            Event::FileFinished { .. } => finished += 1,
            _ => {}
        }
    }
    assert_eq!(finished, card.files.len());
    assert!(matches!(events.last(), Some(Event::Finished { outcome: Outcome::Verified })));
}

#[test]
fn system_files_are_copied_when_asked() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let job = Job::prepare(JobSpec { include_system_files: true, ..spec(&card.root, &dest) }).unwrap();
    let (report, _) = run(&job);
    assert_eq!(report.outcome, Outcome::Verified);
    for j in &card.junk {
        assert!(dest.join(j).is_file(), "{j} should be copied");
    }
}

#[test]
fn md5_is_computed_from_the_same_read() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let job = Job::prepare(JobSpec { hash: HashConfig { md5: true }, ..spec(&card.root, &dest) }).unwrap();
    let (report, _) = run(&job);
    assert_eq!(report.outcome, Outcome::Verified);
    for (i, f) in job.plan().files.iter().enumerate() {
        let want: [u8; 16] = Md5::digest(fs::read(card.root.join(&f.src_rel)).unwrap()).into();
        assert_eq!(report.files[i].hashes.unwrap().md5, Some(want), "{}", f.src_rel.display());
    }
}

#[test]
fn repeat_offloads_go_into_copy_conflicted_and_never_overwrite() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");

    let first = Job::prepare(spec(&card.root, &dest)).unwrap();
    assert_eq!(run(&first).0.outcome, Outcome::Verified);
    let snapshot = |root: &Path| -> Vec<(String, Vec<u8>, std::time::SystemTime)> {
        list_files(root)
            .into_iter()
            .filter(|p| !p.starts_with("copy-conflicted"))
            .map(|p| {
                let full = root.join(&p);
                (p, fs::read(&full).unwrap(), fs::metadata(&full).unwrap().modified().unwrap())
            })
            .collect()
    };
    let before = snapshot(&dest);

    let second = Job::prepare(spec(&card.root, &dest)).unwrap();
    assert_eq!(second.destinations()[0].conflict, Some(0));
    assert_eq!(second.destinations()[0].root, dest.join("copy-conflicted"));
    assert_eq!(run(&second).0.outcome, Outcome::Verified);

    let third = Job::prepare(spec(&card.root, &dest)).unwrap();
    assert_eq!(third.destinations()[0].root, dest.join("copy-conflicted-1"));
    assert_eq!(run(&third).0.outcome, Outcome::Verified);

    assert_copied(&card, &dest.join("copy-conflicted"));
    assert_copied(&card, &dest.join("copy-conflicted-1"));
    assert!(before == snapshot(&dest), "the first offload was modified");
}

#[test]
fn unrelated_files_at_the_destination_do_not_trigger_a_conflict() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    write_file(&dest.join("notes.txt"), 50, 7, shoot_time());
    write_file(&dest.join("DCIM/other-camera.jpg"), 50, 8, shoot_time());

    let job = Job::prepare(spec(&card.root, &dest)).unwrap();
    assert_eq!(job.destinations()[0].conflict, None);
    assert_eq!(run(&job).0.outcome, Outcome::Verified);
    assert_eq!(fs::read(dest.join("notes.txt")).unwrap(), bytes(50, 7));
    assert_eq!(fs::read(dest.join("DCIM/other-camera.jpg")).unwrap(), bytes(50, 8));
}

#[test]
fn one_existing_file_sends_the_whole_transfer_to_copy_conflicted() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let existing = dest.join("DCIM/100NCZ_R/DSC_0003.NEV");
    write_file(&existing, 99, 42, shoot_time());

    let job = Job::prepare(spec(&card.root, &dest)).unwrap();
    assert_eq!(job.destinations()[0].root, dest.join("copy-conflicted"));
    assert_eq!(run(&job).0.outcome, Outcome::Verified);
    assert_eq!(fs::read(&existing).unwrap(), bytes(99, 42), "existing file was overwritten");
    assert_copied(&card, &dest.join("copy-conflicted"));
}

#[test]
fn sequential_backup_reads_from_the_verified_copy() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let (copy, backup) = (tmp.path().join("copy"), tmp.path().join("backup"));
    let pulled = tmp.path().join("card-pulled");
    let job = Job::prepare(spec_with_backup(&card.root, &copy, &backup, CopyMode::Sequential, true)).unwrap();

    // Pull the card the moment it's released: the backup must not need it.
    let events = Mutex::new(Vec::new());
    let cancel = AtomicBool::new(false);
    let report = job.run(&cancel, &|e| {
        if matches!(e, Event::SourceReleased) {
            fs::rename(&card.root, &pulled).unwrap();
        }
        events.lock().unwrap().push(e.clone());
    });
    let events = events.into_inner().unwrap();

    assert_eq!(report.outcome, Outcome::Verified);
    assert!(report.files.iter().all(|f| f.dests.iter().all(FileStatus::is_verified)));
    let moved = FakeCard { root: pulled, ..card };
    assert_copied(&moved, &copy);
    assert_copied(&moved, &backup);

    // Order: copy pass, release, backup pass.
    let pos = |pred: &dyn Fn(&Event) -> bool| events.iter().position(pred).unwrap();
    let copy_done = pos(&|e| matches!(e, Event::PassFinished { pass: Pass::Copy, .. }));
    let released = pos(&|e| matches!(e, Event::SourceReleased));
    let backup_start = pos(&|e| matches!(e, Event::PassStarted { pass: Pass::Backup, .. }));
    assert!(copy_done < released && released < backup_start);
}

#[test]
fn fan_out_writes_every_drive_from_one_read_and_releases_at_the_end() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let (copy, backup) = (tmp.path().join("copy"), tmp.path().join("backup"));

    // Count how often each card file is opened: fan-out reads each one exactly once.
    // (The first read of an open file is the one at offset 0.)
    struct CountOpens(Mutex<std::collections::HashMap<String, u32>>);
    impl offloader_core::Faults for CountOpens {
        fn source_read_error(&self, src: &Path, offset: u64) -> Option<io::Error> {
            if offset == 0 {
                *self.0.lock().unwrap().entry(src.display().to_string()).or_default() += 1;
            }
            None
        }
    }
    let counter = Arc::new(CountOpens(Mutex::new(Default::default())));
    let spec = JobSpec {
        faults: Some(counter.clone()),
        ..spec_with_backup(&card.root, &copy, &backup, CopyMode::FanOut, false)
    };
    let job = Job::prepare(spec).unwrap();

    let (report, events) = run(&job);
    assert_eq!(report.outcome, Outcome::Verified);
    assert_copied(&card, &copy);
    assert_copied(&card, &backup);
    assert!(counter.0.lock().unwrap().values().all(|&n| n == 1), "a card file was read more than once");

    assert_eq!(events.iter().filter(|e| matches!(e, Event::PassStarted { .. })).count(), 1);
    let released = events.iter().position(|e| matches!(e, Event::SourceReleased)).unwrap();
    let last_file = events.iter().rposition(|e| matches!(e, Event::FileFinished { .. })).unwrap();
    assert!(released > last_file);
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
        if let Event::Progress { file, stage: Stage::Copy, file_bytes, .. } = e {
            if *file == big && *file_bytes >= CHUNK as u64 {
                cancel.store(true, Ordering::SeqCst);
            }
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

#[test]
fn not_enough_space_is_refused_before_writing_anything() {
    struct Tiny;
    impl offloader_core::Faults for Tiny {
        fn available_space(&self, _dest: &Path) -> Option<u64> {
            Some(10 << 20)
        }
    }
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let err = Job::prepare(JobSpec { faults: Some(Arc::new(Tiny)), ..spec(&card.root, &dest) }).err().unwrap();
    assert!(matches!(err, PrepareError::InsufficientSpace { .. }), "{err}");
    assert!(!dest.exists());
}

#[test]
fn refuses_to_write_inside_the_source() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let err = Job::prepare(spec(&card.root, &card.root.join("DCIM/backup"))).err().unwrap();
    assert!(matches!(err, PrepareError::DestinationInsideSource { .. }), "{err}");
    let err = Job::prepare(spec(&card.root, &card.root)).err().unwrap();
    assert!(matches!(err, PrepareError::DestinationInsideSource { .. }), "{err}");
}

#[test]
fn refuses_overlapping_destinations() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let d = tmp.path().join("dest");
    let s = spec_with_backup(&card.root, &d, &d.join("backup"), CopyMode::Sequential, false);
    assert!(matches!(Job::prepare(s).err().unwrap(), PrepareError::OverlappingDestinations { .. }));
    let s = spec_with_backup(&card.root, &d, &d, CopyMode::Sequential, false);
    assert!(matches!(Job::prepare(s).err().unwrap(), PrepareError::OverlappingDestinations { .. }));
}

#[test]
fn missing_source_is_reported() {
    let tmp = tempdir().unwrap();
    let err = Job::prepare(spec(&tmp.path().join("nope"), &tmp.path().join("dest"))).err().unwrap();
    assert!(matches!(err, PrepareError::SourceMissing(_)), "{err}");
}

#[test]
fn an_empty_card_is_verified() {
    let tmp = tempdir().unwrap();
    let card = tmp.path().join("card");
    fs::create_dir_all(card.join("DCIM/100MSDCF")).unwrap();
    let dest = tmp.path().join("dest");
    let job = Job::prepare(spec(&card, &dest)).unwrap();
    let (report, _) = run(&job);
    assert_eq!(report.outcome, Outcome::Verified);
    assert!(dest.join("DCIM/100MSDCF").is_dir());
}

#[test]
fn writes_a_transfer_log() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let logs = tmp.path().join("logs");
    let job =
        Job::prepare(JobSpec { log_dir: Some(logs.clone()), ..spec(&card.root, &tmp.path().join("dest")) }).unwrap();
    let (report, _) = run(&job);
    let path = report.log_path.expect("log path");
    assert!(path.starts_with(&logs));
    let text = fs::read_to_string(path).unwrap();
    assert!(text.contains("All files verified."), "{text}");
    for f in &card.files {
        assert!(text.contains(f.as_str()), "log is missing {f}");
    }
    assert!(text.contains("Skipped      .Spotlight-V100"));
}

#[test]
fn uncached_reads_handle_unaligned_sizes() {
    let tmp = tempdir().unwrap();
    for len in [0, 1, 511, 512, 4095, 4096, 4097, 65_535, 65_536, CHUNK - 1, CHUNK, CHUNK + 1, 2 * CHUNK + 3] {
        let p = tmp.path().join(format!("f{len}"));
        write_file(&p, len, len as u64, shoot_time());
        let (got, bypassed) = hash::hash_file(&p, HashConfig { md5: true }, true).unwrap();
        let mut h = Hasher::new(HashConfig { md5: true });
        h.update(&bytes(len, len as u64));
        assert_eq!(got, h.finish(), "len {len}");
        if cfg!(any(windows, target_os = "macos")) {
            assert!(bypassed, "cache not bypassed for len {len}");
        }
    }
}
