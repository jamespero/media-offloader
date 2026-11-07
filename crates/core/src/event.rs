//! Progress events emitted while a job runs. The UI maps these onto the panels
//! (pass progress) and the thumbnail grid (per-file state).

use serde::Serialize;

use crate::error::FileFailure;
use crate::hash::Hashes;

/// A pass reads every file once and writes it to one or more destinations.
///
/// Sequential mode runs `Copy` (card → copy drive), releases the card, then `Backup`
/// (verified copy → backup drives). Fan-out mode runs a single `FanOut` pass
/// (card → every drive at once).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Pass {
    Copy,
    Backup,
    FanOut,
}

/// What's happening to the current file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    /// Reading the source and writing the destination(s).
    Copy,
    /// Re-reading the destination(s) from disk and comparing checksums.
    Verify,
}

/// A file's state on one destination.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum FileStatus {
    /// Not attempted yet (or the job stopped before reaching it, or the backup is on hold).
    #[default]
    Pending,
    /// Written, re-read from disk and checksum-matched. Has its final name.
    Verified,
    /// Not verified; nothing with the final name was left behind.
    Failed { failure: FileFailure },
}

impl FileStatus {
    pub fn is_verified(&self) -> bool {
        matches!(self, FileStatus::Verified)
    }
    pub fn is_failed(&self) -> bool {
        matches!(self, FileStatus::Failed { .. })
    }
}

/// How a job ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Outcome {
    /// Every file verified on every destination.
    Verified,
    /// Some files failed (or couldn't be read from the source); `files` counts them.
    Failed { files: usize },
    /// Stopped by the user. Verified files were kept; the file in progress was removed.
    Cancelled,
    /// Stopped because a drive went away or filled up.
    Aborted { reason: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Event {
    /// A pass is starting. `targets` are indices into [`crate::Job::destinations`].
    PassStarted {
        pass: Pass,
        targets: Vec<usize>,
        files: usize,
        bytes: u64,
    },

    /// Work on file `file` (an index into [`crate::Plan::files`]) is starting.
    FileStarted {
        pass: Pass,
        file: usize,
    },

    /// Progress within the current file. `file_bytes` is how far this stage has got
    /// through the file; `done`/`total` measure the whole pass, where every file counts
    /// twice its size (once for copying, once for verifying).
    Progress {
        pass: Pass,
        file: usize,
        stage: Stage,
        file_bytes: u64,
        done: u64,
        total: u64,
    },

    /// A file finished on this pass's destinations. `results` pairs a destination
    /// index with its new status.
    FileFinished {
        pass: Pass,
        file: usize,
        hashes: Option<Hashes>,
        results: Vec<(usize, FileStatus)>,
    },

    PassFinished {
        pass: Pass,
        verified: usize,
        failed: usize,
    },

    /// Every file is verified somewhere other than the source: the card can be removed.
    SourceReleased,

    Warning {
        message: String,
    },

    Finished {
        outcome: Outcome,
    },
}
