//! Errors. [`PrepareError`] stops a job before anything is written;
//! [`FileFailure`] is why one file didn't verify on one destination.

use std::io;
use std::path::PathBuf;

use serde::Serialize;

use crate::format;
use crate::hash::Hashes;

/// Why a job couldn't be planned. Nothing has been written when this is returned.
#[derive(Debug, thiserror::Error)]
pub enum PrepareError {
    #[error("the source {0} isn't there (not connected, or the path is wrong)")]
    SourceMissing(PathBuf),

    #[error("the source {0} is a file, not a folder or drive")]
    SourceNotFolder(PathBuf),

    #[error("couldn't read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("the destination {dest} is inside the source; that would write onto the card")]
    DestinationInsideSource { dest: PathBuf },

    #[error("{a} and {b} overlap; each destination needs its own folder")]
    OverlappingDestinations { a: PathBuf, b: PathBuf },

    #[error("the destination {dest} isn't available: {source}")]
    DestinationUnavailable {
        dest: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "not enough free space for {dest}: needs {}, only {} free",
        format::size(*needed),
        format::size(*available)
    )]
    InsufficientSpace { dest: PathBuf, needed: u64, available: u64 },

    #[error("couldn't find an unused copy-conflicted folder in {0}")]
    NoConflictFolder(PathBuf),
}

/// Why a file is not verified on a destination.
#[derive(Clone, Debug, Serialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FileFailure {
    #[error("couldn't read the source: {message}")]
    SourceRead { message: String },

    #[error("the source file changed {when}")]
    SourceChanged { when: String },

    #[error("couldn't write to the destination: {message}")]
    Write { message: String },

    #[error("couldn't read the copy back to verify it: {message}")]
    VerifyRead { message: String },

    #[error("checksum mismatch: source {expected}, destination {actual}")]
    Mismatch { expected: Hashes, actual: Hashes },

    #[error("the verified copy being backed up no longer matches its checksum (expected {expected}, read {actual})")]
    CopyChanged { expected: Hashes, actual: Hashes },

    #[error("{path} already exists; not overwriting it")]
    AlreadyExists { path: String },
}

impl FileFailure {
    pub(crate) fn source_read(e: &io::Error) -> Self {
        FileFailure::SourceRead { message: e.to_string() }
    }
    pub(crate) fn write(e: &io::Error) -> Self {
        FileFailure::Write { message: e.to_string() }
    }
    pub(crate) fn verify_read(e: &io::Error) -> Self {
        FileFailure::VerifyRead { message: e.to_string() }
    }
}
