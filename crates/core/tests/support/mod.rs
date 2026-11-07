//! Fake camera cards and tree comparison helpers for the engine tests.
#![allow(dead_code)]

use std::fs::{self, File, FileTimes};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use offloader_core::{CopyMode, Event, HashConfig, Job, JobReport, JobSpec};

pub const MIB: usize = 1 << 20;
/// The engine's chunk size; sizes in the fake card straddle it.
pub const CHUNK: usize = 8 * MIB;

/// Deterministic pseudo-random bytes (xorshift64).
pub fn bytes(len: usize, seed: u64) -> Vec<u8> {
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut out = Vec::with_capacity(len + 8);
    while out.len() < len {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        out.extend_from_slice(&x.to_le_bytes());
    }
    out.truncate(len);
    out
}

pub fn write_file(path: &Path, len: usize, seed: u64, mtime: SystemTime) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = File::create(path).unwrap();
    f.write_all(&bytes(len, seed)).unwrap();
    f.set_times(FileTimes::new().set_modified(mtime).set_accessed(mtime)).unwrap();
}

pub struct FakeCard {
    pub root: PathBuf,
    /// Files that should be copied, relative with `/`.
    pub files: Vec<String>,
    /// Folders that must exist at the destination even though they're empty.
    pub empty_dirs: Vec<String>,
    /// OS junk that's skipped by default.
    pub junk: Vec<String>,
}

/// 2026-10-05 08:12:00 UTC, roughly when the fake card was "shot".
pub fn shoot_time() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_791_187_920)
}

/// A Nikon-style card: DCIM tree, sizes around the chunk boundaries, odd names, a
/// very long path, an empty folder, a file named like our temp files, and OS junk.
pub fn nikon_card(root: &Path) -> FakeCard {
    let mut files: Vec<(String, usize)> = vec![
        ("DCIM/100NCZ_R/DSC_0001.NEF".into(), 3 * MIB + 17),
        ("DCIM/100NCZ_R/DSC_0002.MOV".into(), 2 * CHUNK + 12_345),
        ("DCIM/100NCZ_R/DSC_0003.NEV".into(), CHUNK),
        ("DCIM/100NCZ_R/DSC_0004.NEF".into(), CHUNK + 1),
        ("DCIM/100NCZ_R/DSC_0005.JPG".into(), 4097),
        ("NIKON/SETTINGS/empty.dat".into(), 0),
        ("NIKON/SETTINGS/one-byte.dat".into(), 1),
        ("NIKON/SETTINGS/sector-511.dat".into(), 511),
        ("NIKON/SETTINGS/sector-512.dat".into(), 512),
        ("NIKON/SETTINGS/page-4095.dat".into(), 4095),
        ("NIKON/SETTINGS/page-4096.dat".into(), 4096),
        ("MISC/Ünïcödé 文件 🎬.txt".into(), 100),
        ("MISC/spaces (and) & stuff, too.xml".into(), 2000),
        ("MISC/#hash%percent!.dat".into(), 4096),
        ("MISC/.hidden_camera_file".into(), 10),
        ("MISC/A.MOV".into(), 1000),
        ("MISC/A.MOV.partial".into(), 1001),
    ];
    let deep: Vec<String> = (1..=8).map(|i| format!("deep_folder_level_with_a_long_name_{i:02}")).collect();
    files.push((format!("PRIVATE/{}/clip.mp4", deep.join("/")), 70_000));

    let junk = [
        ".Spotlight-V100/Store-V2/store.db",
        ".fseventsd/0000000000123abc",
        ".Trashes/501/deleted.mov",
        ".DS_Store",
        "DCIM/100NCZ_R/._DSC_0001.NEF",
        "System Volume Information/IndexerVolumeGuid",
    ];
    for (i, (rel, len)) in files.iter().enumerate() {
        write_file(&root.join(rel), *len, i as u64 + 1, shoot_time() + Duration::from_secs(240 * i as u64));
    }
    for (i, rel) in junk.iter().enumerate() {
        write_file(&root.join(rel), 64, 1000 + i as u64, shoot_time());
    }
    let empty_dirs = vec!["DCIM/101NCZ_R".to_string()];
    for d in &empty_dirs {
        fs::create_dir_all(root.join(d)).unwrap();
    }
    FakeCard {
        root: root.to_path_buf(),
        files: files.into_iter().map(|(r, _)| r).collect(),
        empty_dirs,
        junk: junk.iter().map(|s| s.to_string()).collect(),
    }
}

/// Every file under `root` as a `/`-separated relative path, sorted.
pub fn list_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk(root, root, &mut out, false);
    out.sort();
    out
}

pub fn list_dirs(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk(root, root, &mut out, true);
    out.sort();
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>, dirs: bool) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd {
        let e = e.unwrap();
        let p = e.path();
        let rel = p
            .strip_prefix(root)
            .unwrap()
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if e.file_type().unwrap().is_dir() {
            if dirs {
                out.push(rel);
            }
            walk(root, &p, out, dirs);
        } else if !dirs {
            out.push(rel);
        }
    }
}

/// Asserts `dest` holds exactly the card's files (ignoring `copy-conflicted*` folders),
/// byte for byte with preserved modification times, its empty folders, and nothing partial.
pub fn assert_copied(card: &FakeCard, dest: &Path) {
    let got: Vec<String> = list_files(dest).into_iter().filter(|p| !is_sidecar(p)).collect();
    let mut want = card.files.clone();
    want.sort();
    assert_eq!(got, want, "files at {}", dest.display());
    for rel in &card.files {
        let (a, b) = (card.root.join(rel), dest.join(rel));
        assert!(fs::read(&a).unwrap() == fs::read(&b).unwrap(), "contents differ: {rel}");
        let (ma, mb) = (fs::metadata(&a).unwrap().modified().unwrap(), fs::metadata(&b).unwrap().modified().unwrap());
        let skew = ma.duration_since(mb).or_else(|_| mb.duration_since(ma)).unwrap();
        assert!(skew <= Duration::from_secs(2), "mtime not preserved: {rel}");
    }
    for d in &card.empty_dirs {
        assert!(dest.join(d).is_dir(), "empty folder missing: {d}");
    }
    assert!(
        !list_files(dest).iter().any(|p| p.contains(".partial") && !card.files.contains(p)),
        "temp files left behind"
    );
}

pub fn spec(source: &Path, copy_to: &Path) -> JobSpec {
    JobSpec { source: source.into(), copy_to: copy_to.into(), ..Default::default() }
}

pub fn spec_with_backup(source: &Path, copy_to: &Path, backup: &Path, mode: CopyMode, md5: bool) -> JobSpec {
    JobSpec {
        source: source.into(),
        copy_to: copy_to.into(),
        backups: vec![backup.into()],
        mode,
        hash: HashConfig { md5 },
        ..Default::default()
    }
}

/// Runs a job, collecting every event.
pub fn run(job: &Job) -> (JobReport, Vec<Event>) {
    let events = Mutex::new(Vec::new());
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let report = job.run(&cancel, &|e| events.lock().unwrap().push(e.clone()));
    (report, events.into_inner().unwrap())
}

pub fn file_index(job: &Job, rel: &str) -> usize {
    job.plan()
        .files
        .iter()
        .position(|f| {
            f.src_rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/") == rel
        })
        .unwrap_or_else(|| panic!("{rel} not in plan"))
}

/// Files the engine writes next to the footage, plus other runs' folders.
pub fn is_sidecar(rel: &str) -> bool {
    rel.starts_with("copy-conflicted")
        || rel.starts_with("ascmhl/")
        || (rel.starts_with("checksums") && rel.ends_with(".md5"))
}
