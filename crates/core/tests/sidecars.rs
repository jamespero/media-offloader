//! MHL manifests and `.md5` lists written next to the footage.

mod support;

use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use offloader_core::mhl::{HashFormat, History};
use offloader_core::reverify::{self, ReverifyError, ReverifyOutcome};
use offloader_core::{CopyMode, HashConfig, Job, JobSpec, Outcome, Sidecars};
use support::*;
use tempfile::tempdir;

fn reverify_quiet(root: &Path) -> Result<reverify::ReverifyReport, ReverifyError> {
    reverify::reverify(root, None, &AtomicBool::new(false), &|_| {})
}

#[test]
fn writes_an_mhl_generation_with_a_valid_chain_and_an_md5_list() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest").join("Day1");
    let spec = JobSpec {
        hash: HashConfig { md5: true },
        sidecars: Sidecars { mhl: true, md5_list: true },
        ..spec(&card.root, &dest)
    };
    let job = Job::prepare(spec).unwrap();
    let (report, _) = run(&job);
    assert_eq!(report.outcome, Outcome::Verified);
    assert_eq!(report.sidecars.len(), 2, "{:?}", report.sidecars);

    let names: Vec<String> = fs::read_dir(dest.join("ascmhl"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"ascmhl_chain.xml".to_string()), "{names:?}");
    let manifest = names.iter().find(|n| n.ends_with(".mhl")).expect("manifest");
    assert!(manifest.starts_with("0001_Day1_") && manifest.ends_with("Z.mhl"), "{manifest}");

    let h = History::load(&dest).unwrap().expect("history");
    assert!(h.problems.is_empty(), "{:?}", h.problems);
    assert_eq!(h.generations.len(), 1);
    let g = &h.generations[0];
    assert_eq!(g.c4_ok, Some(true));
    assert_eq!(g.process.as_deref(), Some("transfer"));
    assert!(g.ignore.contains(&"ascmhl/".to_string()) && g.ignore.contains(&".Spotlight-V100/".to_string()));
    assert_eq!(g.entries.len(), card.files.len());
    for (i, f) in job.plan().files.iter().enumerate() {
        let rel = offloader_core::format::rel_path(&f.dest_rel);
        let e = g.entries.iter().find(|e| e.path == rel).unwrap_or_else(|| panic!("{rel} missing from MHL"));
        assert_eq!(e.size, Some(f.size));
        let hashes = report.files[i].hashes.unwrap();
        // Schema order: md5 before xxh64; both "original" on a first offload.
        assert_eq!(e.hashes.iter().map(|h| h.format).collect::<Vec<_>>(), [HashFormat::Md5, HashFormat::Xxh64]);
        assert_eq!(e.hashes[1].value, hashes.xxh64_hex());
        assert_eq!(e.hashes[0].value, hashes.md5_hex().unwrap());
        assert!(e.hashes.iter().all(|h| h.action.as_deref() == Some("original")));
    }

    let list = fs::read_to_string(dest.join("checksums.md5")).unwrap();
    assert_eq!(list.lines().count(), card.files.len());
    assert!(list.contains("  DCIM/100NCZ_R/DSC_0001.NEF\n"));
}

#[test]
fn no_md5_list_without_md5_and_no_mhl_when_off() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let spec = JobSpec { sidecars: Sidecars { mhl: false, md5_list: true }, ..spec(&card.root, &dest) };
    let (report, _) = run(&Job::prepare(spec).unwrap());
    assert!(report.sidecars.is_empty());
    assert!(!dest.join("ascmhl").exists() && !dest.join("checksums.md5").exists());
}

#[test]
fn an_mhl_history_on_the_source_is_carried_over_and_extended() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    // First offload gives the folder an MHL history (generation 1)…
    let a = tmp.path().join("A");
    run(&Job::prepare(spec(&card.root, &a)).unwrap());
    // …then that folder is offloaded again, sequentially with a backup.
    let (b, c) = (tmp.path().join("B"), tmp.path().join("C"));
    let job = Job::prepare(spec_with_backup(&a, &b, &c, CopyMode::Sequential, false)).unwrap();
    let (report, _) = run(&job);
    assert_eq!(
        report.outcome,
        Outcome::Verified,
        "{:?}",
        report.files.iter().filter(|f| !f.dests.iter().all(|d| d.is_verified())).collect::<Vec<_>>()
    );

    for root in [&b, &c] {
        let h = History::load(root).unwrap().unwrap();
        assert!(h.problems.is_empty(), "{:?}", h.problems);
        assert_eq!(h.generations.iter().map(|g| g.number).collect::<Vec<_>>(), [1, 2]);
        assert!(h.generations.iter().all(|g| g.c4_ok == Some(true)));
        // Same data as recorded in generation 1, so generation 2 says "verified".
        let g2 = &h.generations[1];
        assert_eq!(g2.entries.len(), card.files.len(), "generation 2 lists the footage, not the copied ascmhl files");
        assert!(g2.entries.iter().all(|e| e.hashes.iter().all(|h| h.action.as_deref() == Some("verified"))));
        assert_eq!(reverify_quiet(root).unwrap().outcome, ReverifyOutcome::Verified);
    }
}
