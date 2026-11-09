//! What a run reports back.

use std::path::PathBuf;
use std::time::SystemTime;

use serde::Serialize;

use crate::event::{FileStatus, Outcome};
use crate::hash::Hashes;
use crate::serde_util;

/// The state of one file at the end of a run.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileReport {
    /// Checksums of the source, once it has been read.
    pub hashes: Option<Hashes>,
    /// One status per destination, indexed like [`crate::Job::destinations`].
    pub dests: Vec<FileStatus>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobReport {
    pub outcome: Outcome,
    /// True once every file was verified somewhere other than the source.
    pub source_released: bool,
    #[serde(serialize_with = "serde_util::opt_time_ms")]
    pub started: Option<SystemTime>,
    #[serde(serialize_with = "serde_util::opt_time_ms")]
    pub finished: Option<SystemTime>,
    /// Indexed like [`crate::Plan::files`].
    pub files: Vec<FileReport>,
    pub log_path: Option<PathBuf>,
    /// MHL manifests and `.md5` lists written next to the footage.
    pub sidecars: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

impl JobReport {
    pub fn verified_count(&self, dest: usize) -> usize {
        self.files.iter().filter(|f| f.dests[dest].is_verified()).count()
    }
    pub fn failed_count(&self, dest: usize) -> usize {
        self.files.iter().filter(|f| f.dests[dest].is_failed()).count()
    }
}
