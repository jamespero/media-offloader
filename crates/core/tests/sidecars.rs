//! ASC MHL / .md5 sidecars and re-verification.

mod support;

use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::AtomicBool;

use offloader_core::mhl::{self, HashFormat, History, MultiHasher};
use offloader_core::reverify::{self, CheckStatus, ManifestSource, ReverifyError, ReverifyOutcome};
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
fn reverify_passes_a_good_copy_and_catches_damage_without_writing() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let (report, _) = run(&Job::prepare(spec(&card.root, &dest)).unwrap());
    assert_eq!(report.outcome, Outcome::Verified);

    let good = reverify_quiet(&dest).unwrap();
    assert_eq!(
        good.outcome,
        ReverifyOutcome::Verified,
        "{:?}",
        good.files.iter().filter(|f| !f.status.is_ok()).collect::<Vec<_>>()
    );
    assert!(matches!(good.source, ManifestSource::Mhl { generations: 1 }));
    assert_eq!(good.files.len(), card.files.len());
    assert!(good.not_listed.is_empty(), "{:?}", good.not_listed);

    // Damage the copy: flip a byte, delete a file, add a stranger.
    let mut f = OpenOptions::new().write(true).open(dest.join("DCIM/100NCZ_R/DSC_0002.MOV")).unwrap();
    f.seek(SeekFrom::Start(5_000_000)).unwrap();
    f.write_all(b"X").unwrap();
    drop(f);
    fs::remove_file(dest.join("MISC/A.MOV")).unwrap();
    write_file(&dest.join("DCIM/100NCZ_R/DSC_9999.JPG"), 10, 99, shoot_time());

    let before = list_files(&dest);
    let bad = reverify_quiet(&dest).unwrap();
    assert_eq!(list_files(&dest), before, "re-verify must not write into the folder");
    assert_eq!(bad.outcome, ReverifyOutcome::Failed { files: 2 });
    let status = |p: &str| &bad.files.iter().find(|f| f.path == p).unwrap().status;
    assert!(matches!(status("DCIM/100NCZ_R/DSC_0002.MOV"), CheckStatus::Mismatch { format: HashFormat::Xxh64, .. }));
    assert!(matches!(status("MISC/A.MOV"), CheckStatus::Missing));
    assert_eq!(bad.not_listed, ["DCIM/100NCZ_R/DSC_9999.JPG"]);
}

#[test]
fn reverify_uses_an_md5_list_when_there_is_no_mhl() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let spec = JobSpec {
        hash: HashConfig { md5: true },
        sidecars: Sidecars { mhl: false, md5_list: true },
        ..spec(&card.root, &dest)
    };
    run(&Job::prepare(spec).unwrap());
    let r = reverify_quiet(&dest).unwrap();
    assert!(matches!(&r.source, ManifestSource::Md5List { file } if file == "checksums.md5"));
    assert_eq!(r.outcome, ReverifyOutcome::Verified);
}

#[test]
fn reverify_without_a_manifest_says_so() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    assert!(matches!(reverify_quiet(&card.root), Err(ReverifyError::NoManifest(_))));
}

#[test]
fn a_damaged_manifest_is_reported() {
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    run(&Job::prepare(spec(&card.root, &dest)).unwrap());
    let manifest = fs::read_dir(dest.join("ascmhl"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().unwrap() == "mhl")
        .unwrap();
    let text = fs::read_to_string(&manifest).unwrap().replace("<hostname>", "<hostname>edited-");
    fs::write(&manifest, text).unwrap();
    let r = reverify_quiet(&dest).unwrap();
    assert!(r.manifest_problems.iter().any(|p| p.contains("C4")), "{:?}", r.manifest_problems);
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

#[test]
fn reverify_reads_formats_from_other_tools() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("archive");
    write_file(&root.join("Clips/A001.mov"), 300_000, 1, shoot_time());
    write_file(&root.join("Sound/A001.wav"), 12_345, 2, shoot_time());
    let digest = |p: &Path, f: HashFormat| {
        let mut h = MultiHasher::new(&[f]);
        h.update(&fs::read(p).unwrap());
        h.finish().remove(0).1
    };
    let mov = root.join("Clips/A001.mov");
    let wav = root.join("Sound/A001.wav");
    let manifest = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<hashlist version="2.0" xmlns="urn:ASC:MHL:v2.0">
  <creatorinfo><creationdate>2026-01-01T10:00:00+00:00</creationdate><hostname>dit-cart</hostname><tool version="9">OtherTool</tool></creatorinfo>
  <processinfo><process>in-place</process><ignore><pattern>.DS_Store</pattern><pattern>ascmhl</pattern><pattern>ascmhl/</pattern></ignore></processinfo>
  <hashes>
    <hash><path size="300000">Clips/A001.mov</path><c4 action="original">{}</c4><xxh128 action="original">{}</xxh128></hash>
    <hash><path size="12345">Sound/A001.wav</path><sha1 action="original">{}</sha1><xxh3 action="original">{}</xxh3></hash>
    <directoryhash><path>Clips</path><content><xxh128>00</xxh128></content><structure><xxh128>00</xxh128></structure></directoryhash>
  </hashes>
</hashlist>
"#,
        digest(&mov, HashFormat::C4),
        digest(&mov, HashFormat::Xxh128),
        digest(&wav, HashFormat::Sha1),
        digest(&wav, HashFormat::Xxh3),
    );
    fs::create_dir_all(root.join("ascmhl")).unwrap();
    let name = "0001_archive_2026-01-01_100000Z.mhl";
    fs::write(root.join("ascmhl").join(name), &manifest).unwrap();
    let c4 = mhl::c4_of_file(&root.join("ascmhl").join(name)).unwrap();
    fs::write(
        root.join("ascmhl/ascmhl_chain.xml"),
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ascmhldirectory xmlns=\"urn:ASC:MHL:DIRECTORY:v2.0\">\n  <hashlist sequencenr=\"1\">\n    <path>{name}</path>\n    <c4>{c4}</c4>\n  </hashlist>\n</ascmhldirectory>\n"),
    )
    .unwrap();

    let r = reverify_quiet(&root).unwrap();
    assert!(r.manifest_problems.is_empty(), "{:?}", r.manifest_problems);
    assert_eq!(r.outcome, ReverifyOutcome::Verified, "{:?}", r.files);
    assert_eq!(r.files.len(), 2);
}

/// macOS writes `._<name>` metadata files next to everything it writes on exFAT/FAT
/// drives. They aren't manifests or checksum lists and must not be read as one.
#[test]
fn reverify_ignores_macos_appledouble_files() {
    // AppleDouble header: binary, and not valid XML or md5sum text.
    let apple_double = b"\x00\x05\x16\x07\x00\x02\x00\x00Mac OS X        \x00\x02&#\xff\xfe";

    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    run(&Job::prepare(spec(&card.root, &dest)).unwrap());
    let manifest = fs::read_dir(dest.join("ascmhl"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|n| n.ends_with(".mhl"))
        .unwrap();
    fs::write(dest.join("ascmhl").join(format!("._{manifest}")), apple_double).unwrap();
    fs::write(dest.join("ascmhl/._ascmhl_chain.xml"), apple_double).unwrap();
    let r = reverify_quiet(&dest).unwrap();
    assert_eq!(r.outcome, ReverifyOutcome::Verified);
    assert!(matches!(r.source, ManifestSource::Mhl { generations: 1 }));
    assert!(r.manifest_problems.is_empty(), "{:?}", r.manifest_problems);
    assert!(r.not_listed.is_empty(), "{:?}", r.not_listed);

    // Same for a checksum list from another tool, whose `._` twin sorts first.
    let tmp = tempdir().unwrap();
    let card = nikon_card(&tmp.path().join("card"));
    let dest = tmp.path().join("dest");
    let spec = JobSpec {
        hash: HashConfig { md5: true },
        sidecars: Sidecars { mhl: false, md5_list: true },
        ..spec(&card.root, &dest)
    };
    run(&Job::prepare(spec).unwrap());
    fs::rename(dest.join("checksums.md5"), dest.join("card.md5")).unwrap();
    fs::write(dest.join("._card.md5"), apple_double).unwrap();
    let r = reverify_quiet(&dest).unwrap();
    assert!(matches!(&r.source, ManifestSource::Md5List { file } if file == "card.md5"), "{:?}", r.source);
    assert_eq!(r.outcome, ReverifyOutcome::Verified);
}
