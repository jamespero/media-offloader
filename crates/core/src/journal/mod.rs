//! The journal: one append-only JSON-lines file per offload, recording the plan and
//! every file as it verifies (flushed to disk line by line). It is what lets an
//! interrupted offload resume, failed files be retried, and past offloads be listed.
//!
//! A resume or retry appends a new run (a new `start` line) to the same journal, so one
//! offload stays one history entry. Reading takes verified files from every run, and
//! failures and the result from the latest run only.

mod record;
mod writer;

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub use record::{Record, list};
pub(crate) use writer::JournalWriter;

use crate::APP_NAME;
use crate::event::Outcome;
use crate::hash::HashConfig;
use crate::job::{CopyMode, DestRole, Sidecars};
use crate::scan::SourceKind;

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
