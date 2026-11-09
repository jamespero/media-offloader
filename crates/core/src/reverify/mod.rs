//! Re-verifies a folder against its ASC MHL history (or a `.md5` list): every listed
//! file is re-read from disk with the OS cache bypassed and its checksums compared with
//! what was recorded. Read-only: nothing in the folder is created or changed.

mod log;
mod types;

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use chrono::Local;

use log::write_log;
pub use types::{
    CheckStatus, CheckedFile, ManifestSource, ReverifyError, ReverifyEvent, ReverifyOutcome, ReverifyReport,
};

use crate::mhl::{self, Expected, HashFormat, History, MultiHasher};
use crate::pipeline::{self, BufferPool, ChunkSink, PumpError};
use crate::platform::UncachedReader;
use crate::scan;

impl ChunkSink for MultiHasher {
    fn consume(&mut self, data: &[u8]) -> io::Result<()> {
        self.update(data);
        Ok(())
    }
}

/// Finds what to check `root` against: its MHL history, else a `.md5` list at the root.
/// (where the checksums came from, expected values by path, ignore patterns, manifest problems)
type Loaded = (ManifestSource, BTreeMap<String, Expected>, Vec<String>, Vec<String>);

fn load_expected(root: &Path) -> Result<Loaded, ReverifyError> {
    let io_err = |e| ReverifyError::Io { path: root.to_path_buf(), source: e };
    if let Some(h) = History::load(root).map_err(io_err)? {
        let expected = h.expected();
        if !expected.is_empty() {
            return Ok((
                ManifestSource::Mhl { generations: h.generations.len() },
                expected,
                h.ignore_patterns(),
                h.problems,
            ));
        }
    }
    let mut lists: Vec<PathBuf> = fs::read_dir(root)
        .map_err(io_err)?
        .flatten()
        .filter(|e| !scan::is_system_name(&e.file_name(), false))
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("md5")))
        .collect();
    // Prefer our own list, then any other.
    lists.sort_by_key(|p| (p.file_name() != Some(mhl::MD5_LIST.as_ref()), p.clone()));
    for list in lists {
        let expected = mhl::read_md5_list(&list).map_err(io_err)?;
        if !expected.is_empty() {
            let file = list.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let ignore = vec![file.clone()];
            return Ok((ManifestSource::Md5List { file }, expected, ignore, Vec::new()));
        }
    }
    Err(ReverifyError::NoManifest(root.to_path_buf()))
}

pub fn reverify(
    root: &Path,
    log_dir: Option<&Path>,
    cancel: &AtomicBool,
    on_event: &(dyn Fn(&ReverifyEvent) + Sync),
) -> Result<ReverifyReport, ReverifyError> {
    if !root.is_dir() {
        return Err(ReverifyError::NotAFolder(root.to_path_buf()));
    }
    let root = std::path::absolute(root).map_err(|e| ReverifyError::Io { path: root.to_path_buf(), source: e })?;
    let started = Local::now();
    let t0 = Instant::now();
    let (source, expected, ignore, manifest_problems) = load_expected(&root)?;

    let mut files: Vec<CheckedFile> = expected
        .iter()
        .map(|(path, e)| CheckedFile {
            path: path.clone(),
            size: e.size,
            formats: e.hashes.iter().map(|(f, _)| *f).collect(),
            status: CheckStatus::Pending,
        })
        .collect();
    let local = |rel: &str| rel.split('/').fold(root.clone(), |p, c| p.join(c));
    let sizes: Vec<u64> = files
        .iter()
        .map(|f| f.size.or_else(|| fs::metadata(local(&f.path)).ok().map(|m| m.len())).unwrap_or(0))
        .collect();
    let total: u64 = sizes.iter().sum();
    on_event(&ReverifyEvent::Started { source: source.clone(), files: files.len(), bytes: total });

    let pool = BufferPool::new();
    let mut done = 0u64;
    let mut cancelled = false;
    for (i, file) in files.iter_mut().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        on_event(&ReverifyEvent::FileStarted { file: i, path: file.path.clone() });
        let exp = &expected[&file.path];
        let path = local(&file.path);
        let status = match fs::metadata(&path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => CheckStatus::Missing,
            Err(e) => CheckStatus::ReadError { message: e.to_string() },
            Ok(m) if m.is_dir() => CheckStatus::Missing,
            Ok(m) if exp.size.is_some_and(|s| s != m.len()) => {
                CheckStatus::SizeDiffers { expected: exp.size.unwrap_or(0), actual: m.len() }
            }
            Ok(m) => {
                let formats: Vec<HashFormat> = exp.hashes.iter().map(|(f, _)| *f).collect();
                match check(&path, m.len(), &formats, &exp.hashes, &pool, cancel, &mut |b| {
                    on_event(&ReverifyEvent::Progress { file: i, file_bytes: b, done: done + b, total });
                }) {
                    Some(s) => s,
                    None => {
                        cancelled = true;
                        break;
                    }
                }
            }
        };
        done += sizes[i];
        on_event(&ReverifyEvent::FileFinished { file: i, status: status.clone() });
        file.status = status;
    }

    let not_listed = unlisted(&root, &expected, &ignore);
    let failed = files.iter().filter(|f| !f.status.is_ok() && !matches!(f.status, CheckStatus::Pending)).count();
    let outcome = if cancelled {
        ReverifyOutcome::Cancelled
    } else if failed > 0 {
        ReverifyOutcome::Failed { files: failed }
    } else {
        ReverifyOutcome::Verified
    };
    on_event(&ReverifyEvent::Finished { outcome: outcome.clone() });

    let mut report = ReverifyReport { root, source, files, not_listed, manifest_problems, outcome, log_path: None };
    if let Some(dir) = log_dir {
        report.log_path = write_log(dir, &report, started, t0.elapsed()).ok();
    }
    Ok(report)
}

/// Hashes `path` (cache bypassed) and compares. `None` if cancelled.
fn check(
    path: &Path,
    len: u64,
    formats: &[HashFormat],
    want: &[(HashFormat, String)],
    pool: &BufferPool,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64),
) -> Option<CheckStatus> {
    let mut reader = match UncachedReader::open(path) {
        Ok(r) => r,
        Err(e) => return Some(CheckStatus::ReadError { message: e.to_string() }),
    };
    let mut hasher = MultiHasher::new(formats);
    match pipeline::pump(&mut reader, &mut [&mut hasher], len, pool, cancel, progress) {
        Err(PumpError::Cancelled) => return None,
        Err(PumpError::Read(e)) => return Some(CheckStatus::ReadError { message: e.to_string() }),
        Ok(_) => {}
    }
    let got: BTreeMap<HashFormat, String> = hasher.finish().into_iter().collect();
    for (format, expected) in want {
        let actual = got.get(format).cloned().unwrap_or_default();
        if &actual != expected {
            return Some(CheckStatus::Mismatch { format: *format, expected: expected.clone(), actual });
        }
    }
    Some(CheckStatus::Verified)
}

/// Files under `root` that no manifest mentions, minus ignored and OS housekeeping.
fn unlisted(root: &Path, expected: &BTreeMap<String, Expected>, ignore: &[String]) -> Vec<String> {
    let mut patterns = ignore.to_vec();
    patterns.extend(scan::system_patterns());
    patterns.extend(["ascmhl/".to_string(), "checksums*.md5".to_string()]);
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel)) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let r = if rel.is_empty() { name } else { format!("{rel}/{name}") };
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_symlink() || mhl::ignored(&r, ft.is_dir(), &patterns) {
                continue;
            }
            if ft.is_dir() {
                stack.push((e.path(), r));
            } else if !expected.contains_key(&r) {
                out.push(r);
            }
        }
    }
    out.sort();
    out
}
