//! What to offload, and where.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::faults::Faults;
use crate::hash::{HashConfig, Hashes};

/// How copy and backup are scheduled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CopyMode {
    /// Copy and verify, release the card, then back up from the verified copy (classic).
    #[default]
    Sequential,
    /// Read the card once and write every destination at the same time; the card is
    /// released when all of them verify.
    FanOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DestRole {
    Copy,
    Backup,
}

/// What to offload, and where.
#[derive(Clone, Default)]
pub struct JobSpec {
    /// Card, drive or folder to copy from.
    pub source: PathBuf,
    /// Folder to copy into: the chosen destination plus any subfolders.
    pub copy_to: PathBuf,
    /// Backup folders (usually zero or one).
    pub backups: Vec<PathBuf>,
    pub mode: CopyMode,
    pub hash: HashConfig,
    /// Copy OS housekeeping (.Spotlight-V100, System Volume Information…) too.
    pub include_system_files: bool,
    /// Where to write the transfer log; `None` for no log.
    pub log_dir: Option<PathBuf>,
    /// Checksum files to write next to the footage.
    pub sidecars: Sidecars,
    /// Continue an earlier run of this offload in the same folders instead of starting
    /// fresh (see [`crate::journal::Record::resume_spec`]).
    pub resume: Option<Resume>,
    /// Record progress in this journal file (appending, for a resumed offload).
    pub journal: Option<PathBuf>,
    /// Keep the computer from sleeping while the job runs.
    pub keep_awake: bool,
    #[doc(hidden)]
    pub faults: Option<Arc<dyn Faults>>,
}

/// What an earlier run of the same offload achieved.
#[derive(Clone, Debug, Default)]
pub struct Resume {
    /// The folders that run wrote to: copy first, then backups.
    pub roots: Vec<PathBuf>,
    /// Per destination: the files it verified, by `/`-separated path.
    pub verified: Vec<HashMap<String, VerifiedFile>>,
}

#[derive(Clone, Debug)]
pub struct VerifiedFile {
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub hashes: Hashes,
    pub at: SystemTime,
}

/// Checksum files written at each destination root once its files are verified.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sidecars {
    /// An ASC MHL generation in `ascmhl/` listing every verified file.
    pub mhl: bool,
    /// `checksums.md5` (md5sum format), when MD5 is on and every file verified.
    pub md5_list: bool,
}

impl Default for Sidecars {
    fn default() -> Self {
        Self { mhl: true, md5_list: false }
    }
}
