//! What a re-verify reports.

use std::io;
use std::path::PathBuf;

use serde::Serialize;

use crate::mhl::HashFormat;

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
