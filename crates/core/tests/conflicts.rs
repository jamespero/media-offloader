//! Never overwriting: when a transfer goes into `copy-conflicted`.

mod support;

use std::fs;
use std::path::Path;

use offloader_core::{Job, Outcome};
use support::*;
use tempfile::tempdir;

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
