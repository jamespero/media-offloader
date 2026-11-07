//! Re-verifies a folder against its ASC MHL history (or a `.md5` list): every listed
//! file is re-read from disk with the OS cache bypassed and its checksums compared with
//! what was recorded. Read-only: nothing in the folder is created or changed.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use chrono::Local;
use serde::Serialize;

use crate::format;
use crate::mhl::{self, Expected, HashFormat, History, MultiHasher};
use crate::pipeline::{self, BufferPool, ChunkSink, PumpError};
use crate::platform::UncachedReader;
use crate::scan;
use crate::{APP_NAME, VERSION};

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ManifestSource {
    Mhl { generations: usize },
    Md5List { file: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum CheckStatus {
    Pending,
    Verified,
    Mismatch { format: HashFormat, expected: String, actual: String },
    SizeDiffers { expected: u64, actual: u64 },
    Missing,
    ReadError { message: String },
}

impl CheckStatus {
    pub fn is_ok(&self) -> bool {
        matches!(self, CheckStatus::Verified)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckedFile {
    /// `/`-separated, relative to the folder.
    pub path: String,
    pub size: Option<u64>,
    pub formats: Vec<HashFormat>,
    pub status: CheckStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ReverifyOutcome {
    Verified,
    Failed { files: usize },
    Cancelled,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverifyReport {
    pub root: PathBuf,
    pub source: ManifestSource,
    pub files: Vec<CheckedFile>,
    /// Files in the folder that no manifest lists (not a failure, but worth knowing).
    pub not_listed: Vec<String>,
    /// Damaged or unlisted manifests, chain mismatches.
    pub manifest_problems: Vec<String>,
    pub outcome: ReverifyOutcome,
    pub log_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ReverifyEvent {
    Started { source: ManifestSource, files: usize, bytes: u64 },
    FileStarted { file: usize, path: String },
    Progress { file: usize, file_bytes: u64, done: u64, total: u64 },
    FileFinished { file: usize, status: CheckStatus },
    Finished { outcome: ReverifyOutcome },
}

#[derive(Debug, thiserror::Error)]
pub enum ReverifyError {
    #[error("{0} has no ASC MHL history (an ascmhl folder) or .md5 list to check against")]
    NoManifest(PathBuf),
    #[error("{0} isn't a folder")]
    NotAFolder(PathBuf),
    #[error("couldn't read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

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

fn write_log(
    dir: &Path,
    r: &ReverifyReport,
    started: chrono::DateTime<Local>,
    elapsed: std::time::Duration,
) -> io::Result<PathBuf> {
    use std::io::Write;
    fs::create_dir_all(dir)?;
    let name = r.root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "folder".into());
    let safe: String = name.chars().map(|c| if c.is_alphanumeric() || " -_.".contains(c) { c } else { '_' }).collect();
    let mut path = dir.join(format!("{} re-verify {safe}.log", started.format("%Y-%m-%d %H%M%S")));
    let mut n = 1;
    while path.exists() {
        n += 1;
        path = dir.join(format!("{} re-verify {safe} ({n}).log", started.format("%Y-%m-%d %H%M%S")));
    }
    let mut f = io::BufWriter::new(fs::OpenOptions::new().write(true).create_new(true).open(&path)?);
    writeln!(f, "{APP_NAME} {VERSION} — re-verify log")?;
    writeln!(f, "Started      {}", started.format("%Y-%m-%d %H:%M:%S %:z"))?;
    writeln!(f, "Folder       {}", r.root.display())?;
    match &r.source {
        ManifestSource::Mhl { generations } => {
            writeln!(f, "Checked by   ASC MHL history ({generations} generation(s))")?
        }
        ManifestSource::Md5List { file } => writeln!(f, "Checked by   {file}")?,
    }
    writeln!(f, "Reads        bypass the OS cache")?;
    for p in &r.manifest_problems {
        writeln!(f, "MANIFEST     {p}")?;
    }
    writeln!(f)?;
    for file in &r.files {
        let size = file.size.map(format::size).unwrap_or_default();
        match &file.status {
            CheckStatus::Verified => writeln!(f, "OK    {}  {size}", file.path)?,
            CheckStatus::Pending => writeln!(f, "--    {}  not checked", file.path)?,
            CheckStatus::Missing => writeln!(f, "FAIL  {}  — missing", file.path)?,
            CheckStatus::SizeDiffers { expected, actual } => {
                writeln!(f, "FAIL  {}  — size is {actual} bytes, manifest says {expected}", file.path)?
            }
            CheckStatus::Mismatch { format, expected, actual } => {
                writeln!(f, "FAIL  {}  — {} is {actual}, manifest says {expected}", file.path, format.name())?
            }
            CheckStatus::ReadError { message } => writeln!(f, "FAIL  {}  — couldn't read: {message}", file.path)?,
        }
    }
    for p in &r.not_listed {
        writeln!(f, "NEW   {p}  — not in any manifest")?;
    }
    writeln!(f)?;
    writeln!(f, "Finished     {}  ({})", Local::now().format("%Y-%m-%d %H:%M:%S %:z"), format::duration(elapsed))?;
    let result = match &r.outcome {
        ReverifyOutcome::Verified => format!("All {} files match their recorded checksums.", r.files.len()),
        ReverifyOutcome::Failed { files } => format!("{files} file(s) do NOT match their recorded checksums."),
        ReverifyOutcome::Cancelled => "Stopped before every file was checked.".to_string(),
    };
    writeln!(f, "Result       {result}")?;
    f.flush()?;
    Ok(path)
}
