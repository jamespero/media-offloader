//! Errors. [`PrepareError`] stops a job before anything is written.

use std::io;
use std::path::PathBuf;

use crate::format;

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
