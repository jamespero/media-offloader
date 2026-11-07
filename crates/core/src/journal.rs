//! The journal: one append-only JSON-lines file per offload, recording the plan and
//! every file as it verifies (flushed to disk line by line). It is what lets an
//! interrupted offload resume, failed files be retried, and past offloads be listed.
//!
//! A resume or retry appends a new run (a new `start` line) to the same journal, so one
//! offload stays one history entry. Reading takes verified files from every run, and
//! failures and the result from the latest run only.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::event::Outcome;
use crate::hash::{HashConfig, Hashes};
use crate::job::{CopyMode, DestRole, Job, JobSpec, Resume, Sidecars, VerifiedFile};
use crate::scan::SourceKind;
use crate::{APP_NAME, format};

/// `%LOCALAPPDATA%\Media Offloader\Jobs` on Windows, `~/Library/Application Support/Media Offloader/Jobs` on macOS.
pub fn default_journal_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join(APP_NAME).join("Jobs"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library/Application Support").join(APP_NAME).join("Jobs"))
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
            .map(|d| d.join("media-offloader").join("jobs"))
    }
}

/// A new journal path in `dir`, named for the start time and the source.
pub fn new_journal_path(dir: &Path, label: &str) -> PathBuf {
    let now = chrono::Local::now();
    let safe: String = label.chars().map(|c| if c.is_alphanumeric() || "-_".contains(c) { c } else { '_' }).collect();
    let stem = format!("{}_{safe}", now.format("%Y%m%d-%H%M%S"));
    (0..)
        .map(|n| dir.join(if n == 0 { format!("{stem}.jsonl") } else { format!("{stem}-{n}.jsonl") }))
        .find(|p| !p.exists())
        .expect("unbounded")
}

pub(crate) fn ms(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn from_ms(ms: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(ms)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalFile {
    /// `/`-separated paths.
    pub src: String,
    pub dest: String,
    pub size: u64,
    pub modified: Option<u64>,
}

/// Everything needed to show this offload again or run it again.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub started: u64,
    pub source: PathBuf,
    pub source_label: String,
    pub source_kind: SourceKind,
    pub roles: Vec<DestRole>,
    /// What was asked for, and where files actually go (copy-conflicted included).
    pub requested: Vec<PathBuf>,
    pub roots: Vec<PathBuf>,
    pub mode: CopyMode,
    pub hash: HashConfig,
    pub sidecars: Sidecars,
    pub include_system_files: bool,
    pub log: Option<PathBuf>,
    pub total_bytes: u64,
    pub files: Vec<JournalFile>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "camelCase", rename_all_fields = "camelCase")]
enum Line {
    Start(Box<Header>),
    Verified {
        dest: usize,
        path: String,
        size: u64,
        modified: Option<u64>,
        xxh64: String,
        md5: Option<String>,
        at: u64,
    },
    Failed {
        dest: usize,
        path: String,
        reason: String,
    },
    Released,
    End {
        outcome: Outcome,
        sidecars: Vec<PathBuf>,
        at: u64,
    },
}

pub(crate) struct JournalWriter {
    file: File,
}

impl JournalWriter {
    pub fn open(path: &Path) -> io::Result<Self> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        Ok(Self { file: OpenOptions::new().create(true).append(true).open(path)? })
    }

    /// Best effort: a journal problem must never fail the offload itself.
    fn write(&mut self, line: &Line) {
        if let Ok(mut s) = serde_json::to_string(line) {
            s.push('\n');
            let _ = self.file.write_all(s.as_bytes()).and_then(|()| self.file.sync_data());
        }
    }

    pub fn start(&mut self, job: &Job, log: Option<&Path>) {
        let plan = job.plan();
        let spec = job.spec();
        let header = Header {
            started: ms(SystemTime::now()),
            source: plan.source.root.clone(),
            source_label: plan.source.label.clone(),
            source_kind: plan.source.kind,
            roles: job.destinations().iter().map(|d| d.role).collect(),
            requested: job.destinations().iter().map(|d| d.requested.clone()).collect(),
            roots: job.destinations().iter().map(|d| d.root.clone()).collect(),
            mode: spec.mode,
            hash: spec.hash,
            sidecars: spec.sidecars,
            include_system_files: spec.include_system_files,
            log: log.map(Path::to_path_buf),
            total_bytes: plan.total_bytes,
            files: plan
                .files
                .iter()
                .map(|f| JournalFile {
                    src: format::rel_path(&f.src_rel),
                    dest: format::rel_path(&f.dest_rel),
                    size: f.size,
                    modified: f.modified.map(ms),
                })
                .collect(),
        };
        self.write(&Line::Start(Box::new(header)));
    }

    pub fn verified(
        &mut self,
        dest: usize,
        path: String,
        size: u64,
        modified: Option<SystemTime>,
        hashes: &Hashes,
        at: SystemTime,
    ) {
        self.write(&Line::Verified {
            dest,
            path,
            size,
            modified: modified.map(ms),
            xxh64: hashes.xxh64_hex(),
            md5: hashes.md5_hex(),
            at: ms(at),
        });
    }

    pub fn failed(&mut self, dest: usize, path: String, reason: String) {
        self.write(&Line::Failed { dest, path, reason });
    }

    pub fn released(&mut self) {
        self.write(&Line::Released);
    }

    pub fn end(&mut self, outcome: &Outcome, sidecars: &[PathBuf]) {
        self.write(&Line::End { outcome: outcome.clone(), sidecars: sidecars.to_vec(), at: ms(SystemTime::now()) });
    }
}

/// A journal as read back.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub path: PathBuf,
    /// The latest run's header.
    pub header: Header,
    /// When the first run started.
    pub first_started: u64,
    pub runs: usize,
    /// Per destination: path → verified file (from every run).
    #[serde(skip)]
    pub verified: Vec<HashMap<String, VerifiedFile>>,
    /// Per destination: path → why it failed (latest run only).
    pub failed: Vec<HashMap<String, String>>,
    pub released: bool,
    /// `None` while running, or if the app stopped without finishing (crash, power loss).
    pub outcome: Option<Outcome>,
    pub finished: Option<u64>,
    pub sidecars: Vec<PathBuf>,
}

impl Record {
    pub fn read(path: &Path) -> io::Result<Record> {
        let reader = BufReader::new(File::open(path)?);
        let mut header: Option<Header> = None;
        let mut first_started = 0;
        let mut runs = 0;
        let mut verified: Vec<HashMap<String, VerifiedFile>> = Vec::new();
        let mut failed: Vec<HashMap<String, String>> = Vec::new();
        let (mut released, mut outcome, mut finished, mut sidecars) = (false, None, None, Vec::new());
        for line in reader.lines() {
            let Ok(line) = line else { break };
            // A torn last line (power loss mid-write) is simply ignored.
            let Ok(parsed) = serde_json::from_str::<Line>(&line) else { continue };
            match parsed {
                Line::Start(h) => {
                    if runs == 0 {
                        first_started = h.started;
                    }
                    runs += 1;
                    let n = h.roots.len();
                    verified.resize_with(n, HashMap::new);
                    failed = vec![HashMap::new(); n];
                    (released, outcome, finished) = (false, None, None);
                    header = Some(*h);
                }
                Line::Verified { dest, path, size, modified, xxh64, md5, at } => {
                    if let (Some(map), Some(hashes)) =
                        (verified.get_mut(dest), Hashes::from_hex(&xxh64, md5.as_deref()))
                    {
                        if let Some(f) = failed.get_mut(dest) {
                            f.remove(&path);
                        }
                        map.insert(
                            path,
                            VerifiedFile { size, modified: modified.map(from_ms), hashes, at: from_ms(at) },
                        );
                    }
                }
                Line::Failed { dest, path, reason } => {
                    if let Some(f) = failed.get_mut(dest) {
                        f.insert(path, reason);
                    }
                }
                Line::Released => released = true,
                Line::End { outcome: o, sidecars: s, at } => {
                    outcome = Some(o);
                    finished = Some(at);
                    sidecars.extend(s);
                }
            }
        }
        let header = header.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "not a journal"))?;
        Ok(Record {
            path: path.to_path_buf(),
            header,
            first_started,
            runs,
            verified,
            failed,
            released,
            outcome,
            finished,
            sidecars,
        })
    }

    pub fn verified_count(&self, dest: usize) -> usize {
        self.verified.get(dest).map_or(0, HashMap::len)
    }

    /// Paths verified on every destination, for the history view.
    pub fn verified_paths(&self) -> Vec<Vec<String>> {
        self.verified.iter().map(|m| m.keys().cloned().collect()).collect()
    }

    /// Whether there's anything left to do: interrupted, stopped, or files failed.
    pub fn resumable(&self) -> bool {
        !matches!(self.outcome, Some(Outcome::Verified))
    }

    /// A spec that continues this offload into the same folders: already verified files
    /// are kept, failed and missing ones are copied again, and the journal continues.
    pub fn resume_spec(&self, log_dir: Option<PathBuf>) -> JobSpec {
        let h = &self.header;
        JobSpec {
            source: h.source.clone(),
            copy_to: h.roots[0].clone(),
            backups: h.roots[1..].to_vec(),
            mode: h.mode,
            hash: h.hash,
            include_system_files: h.include_system_files,
            log_dir,
            sidecars: h.sidecars,
            resume: Some(Resume { roots: h.roots.clone(), verified: self.verified.clone() }),
            journal: Some(self.path.clone()),
            keep_awake: true,
            faults: None,
        }
    }
}

/// Every readable journal in `dir`, newest first.
pub fn list(dir: &Path) -> Vec<Record> {
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Record> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .filter_map(|p| Record::read(&p).ok())
        .collect();
    out.sort_by_key(|r| std::cmp::Reverse(r.first_started));
    out
}
