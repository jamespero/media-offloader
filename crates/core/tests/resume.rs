//! The journal, resuming an interrupted offload, and retrying failed files.

mod support;

use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use offloader_core::journal::Record;
use offloader_core::mhl::History;
use offloader_core::{CopyMode, Event, FileFailure, FileStatus, Job, JobSpec, Outcome, Pass, Stage};
use support::*;
use tempfile::tempdir;

fn with_journal(spec: JobSpec, journal: &Path) -> JobSpec {
    JobSpec { journal: Some(journal.to_path_buf()), ..spec }
}

/// Runs a job, cancelling once file `stop_at` is half copied in `pass`.
fn run_and_stop(job: &Job, pass: Pass, stop_at: usize) -> Outcome {
    let cancel = AtomicBool::new(false);
    let report = job.run(&cancel, &|e| {
        if let Event::Progress { pass: p, file, stage: Stage::Copy, file_bytes, .. } = e
            && *p == pass
            && *file == stop_at
            && *file_bytes >= CHUNK as u64
        {
            cancel.store(true, Ordering::SeqCst);
        }
    });
    report.outcome
}

fn started_files(events: &[Event], pass: Pass) -> usize {
    events.iter().filter(|e| matches!(e, Event::FileStarted { pass: p, .. } if *p == pass)).count()
}

fn partials(root: &Path) -> Vec<String> {
    list_files(root).into_iter().filter(|p| p.contains(".partial") && !p.ends_with("A.MOV.partial")).collect()
}

#[test]
fn the_journal_records_a_finished_offload() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let journal = tmp.path().join("jobs/one.jsonl");
    let (report, _) = run(&Job::prepare(with_journal(spec(&card.root, &dest), &journal)).unwrap());
    assert_eq!(report.outcome, Outcome::Verified);

    let r = Record::read(&journal).unwrap();
    assert_eq!(r.runs, 1);
    assert_eq!(r.outcome, Some(Outcome::Verified));
    assert!(r.released && !r.resumable());
    assert_eq!(r.verified_count(0), card.files.len());
    assert_eq!(r.header.roots, std::slice::from_ref(&dest));
    assert_eq!(r.header.source_label, "card");
    assert_eq!(r.header.files.len(), card.files.len());
    assert_eq!(r.sidecars.len(), 1, "the MHL manifest");
}

#[test]
fn resuming_after_stop_copies_only_what_is_left_into_the_same_folder() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let journal = tmp.path().join("jobs/two.jsonl");
    let job = Job::prepare(with_journal(spec(&card.root, &dest), &journal)).unwrap();
    let big = file_index(&job, "DCIM/100NCZ_R/DSC_0002.MOV");
    assert_eq!(run_and_stop(&job, Pass::Copy, big), Outcome::Cancelled);
    assert!(!dest.join("ascmhl").exists(), "no manifest for a stopped run");

    let r = Record::read(&journal).unwrap();
    assert_eq!(r.outcome, Some(Outcome::Cancelled));
    assert!(r.resumable());
    assert_eq!(r.verified_count(0), big);

    // A stale temp file, as if the app had crashed instead of stopping cleanly.
    let stale = dest.join(&job.plan().files[big].dest_rel).with_extension("MOV.partial");
    fs::write(&stale, b"half a file").unwrap();

    let resumed = Job::prepare(r.resume_spec(None)).unwrap();
    assert_eq!(resumed.destinations()[0].root, dest, "resumes in place, not in copy-conflicted");
    assert!(resumed.destinations()[0].required < job.destinations()[0].required);
    let (report, events) = run(&resumed);
    assert_eq!(report.outcome, Outcome::Verified);
    assert_eq!(started_files(&events, Pass::Copy), card.files.len() - big, "verified files aren't copied again");
    assert_copied(&card, &dest);
    assert!(partials(&dest).is_empty(), "{:?}", partials(&dest));

    let r = Record::read(&journal).unwrap();
    assert_eq!((r.runs, r.outcome.clone()), (2, Some(Outcome::Verified)));
    assert_eq!(r.verified_count(0), card.files.len());
    let h = History::load(&dest).unwrap().unwrap();
    assert_eq!(h.generations.len(), 1);
    assert_eq!(h.generations[0].entries.len(), card.files.len(), "the first manifest lists every file");
}

struct FlipOnce(&'static str);
impl offloader_core::Faults for FlipOnce {
    fn before_verify(&self, partial: &Path) {
        if partial.file_name().unwrap().to_string_lossy().starts_with(self.0) {
            let mut f = OpenOptions::new().read(true).write(true).open(partial).unwrap();
            let mut b = [0u8];
            f.seek(SeekFrom::Start(100)).unwrap();
            f.read_exact(&mut b).unwrap();
            f.seek(SeekFrom::Start(100)).unwrap();
            f.write_all(&[b[0] ^ 1]).unwrap();
        }
    }
}

#[test]
fn retry_redoes_only_the_failed_files() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let journal = tmp.path().join("jobs/three.jsonl");
    let spec =
        JobSpec { faults: Some(Arc::new(FlipOnce("DSC_0004.NEF"))), ..with_journal(spec(&card.root, &dest), &journal) };
    let (report, _) = run(&Job::prepare(spec).unwrap());
    assert_eq!(report.outcome, Outcome::Failed { files: 1 });

    let r = Record::read(&journal).unwrap();
    assert_eq!(r.failed[0].keys().collect::<Vec<_>>(), ["DCIM/100NCZ_R/DSC_0004.NEF"]);
    let (report, events) = run(&Job::prepare(r.resume_spec(None)).unwrap());
    assert_eq!(report.outcome, Outcome::Verified);
    assert_eq!(started_files(&events, Pass::Copy), 1);
    assert_copied(&card, &dest);

    // Generation 1 from the first run, generation 2 with just the retried file.
    let h = History::load(&dest).unwrap().unwrap();
    assert!(h.problems.is_empty(), "{:?}", h.problems);
    assert_eq!(h.generations.iter().map(|g| g.entries.len()).collect::<Vec<_>>(), [card.files.len() - 1, 1]);
    assert!(Record::read(&journal).unwrap().failed[0].is_empty());
}

#[test]
fn resume_never_overwrites_a_file_that_appeared_meanwhile() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let journal = tmp.path().join("jobs/four.jsonl");
    let job = Job::prepare(with_journal(spec(&card.root, &dest), &journal)).unwrap();
    let big = file_index(&job, "DCIM/100NCZ_R/DSC_0002.MOV");
    run_and_stop(&job, Pass::Copy, big);

    let intruder: PathBuf = dest.join("NIKON/SETTINGS/one-byte.dat");
    write_file(&intruder, 7, 777, shoot_time());
    let resumed = Job::prepare(Record::read(&journal).unwrap().resume_spec(None)).unwrap();
    let (report, _) = run(&resumed);
    let i = file_index(&resumed, "NIKON/SETTINGS/one-byte.dat");
    assert!(matches!(&report.files[i].dests[0], FileStatus::Failed { failure: FileFailure::AlreadyExists { .. } }));
    assert_eq!(report.outcome, Outcome::Failed { files: 1 });
    assert_eq!(fs::read(&intruder).unwrap(), bytes(7, 777), "the other file was overwritten");
}

#[test]
fn resuming_a_sequential_offload_finishes_the_backup_without_recopying() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let (copy, backup) = (tmp.path().join("copy"), tmp.path().join("backup"));
    let journal = tmp.path().join("jobs/five.jsonl");
    let job =
        Job::prepare(with_journal(spec_with_backup(&card.root, &copy, &backup, CopyMode::Sequential, true), &journal))
            .unwrap();
    let big = file_index(&job, "DCIM/100NCZ_R/DSC_0002.MOV");
    assert_eq!(run_and_stop(&job, Pass::Backup, big), Outcome::Cancelled);
    let r = Record::read(&journal).unwrap();
    assert!(r.released);
    assert_eq!((r.verified_count(0), r.verified_count(1)), (card.files.len(), big));

    let (report, events) = run(&Job::prepare(r.resume_spec(None)).unwrap());
    assert_eq!(report.outcome, Outcome::Verified);
    assert_eq!(started_files(&events, Pass::Copy), 0, "the copy was already verified");
    assert_eq!(started_files(&events, Pass::Backup), card.files.len() - big);
    assert_copied(&card, &copy);
    assert_copied(&card, &backup);
    assert!(report.files.iter().all(|f| f.hashes.is_some_and(|h| h.md5.is_some())));
}
