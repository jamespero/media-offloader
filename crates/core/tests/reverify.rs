//! Re-checking a folder against its recorded checksums.

mod support;

use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::AtomicBool;

use offloader_core::mhl::{self, HashFormat, MultiHasher};
use offloader_core::reverify::{self, CheckStatus, ManifestSource, ReverifyError, ReverifyOutcome};
use offloader_core::{HashConfig, Job, JobSpec, Outcome, Sidecars};
use support::*;
use tempfile::tempdir;

fn reverify_quiet(root: &Path) -> Result<reverify::ReverifyReport, ReverifyError> {
    reverify::reverify(root, None, &AtomicBool::new(false), &|_| {})
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
