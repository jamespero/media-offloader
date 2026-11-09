//! Writing a second drive, sequentially or fanned out.

mod support;

use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use offloader_core::{CopyMode, Event, FileStatus, Job, JobSpec, Outcome, Pass};
use support::*;
use tempfile::tempdir;

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
