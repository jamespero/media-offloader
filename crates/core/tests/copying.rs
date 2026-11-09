//! Copying a card: the normal path, what gets copied, and the transfer log.

mod support;

use std::fs;

use md5::{Digest, Md5};
use offloader_core::hash::{self, Hasher};
use offloader_core::{Event, HashConfig, Job, JobSpec, Outcome};
use support::*;
use tempfile::tempdir;

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
