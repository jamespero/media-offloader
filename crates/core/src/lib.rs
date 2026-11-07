//! Copy + verify engine for camera-card offloads.
//!
//! UI-agnostic: the `offload` CLI and the desktop app both drive it the same way:
//! [`Job::prepare`] scans the source and plans every destination without writing
//! anything, then [`Job::run`] copies, verifies and reports progress as [`Event`]s.
//!
//! Safety rules enforced here (see CLAUDE.md):
//! 1. Never overwrite. If a destination already holds any of this source's files, the
//!    whole transfer goes into `copy-conflicted`, `copy-conflicted-1`, … Temp files are
//!    created exclusively and renamed with a no-replace rename.
//! 2. Never modify the source. It is only ever opened for reading, and a destination
//!    inside the source is refused.
//! 3. Data is written to `<name>.partial`, fsynced, re-read from disk with the OS cache
//!    bypassed, and only renamed to its real name once the checksums match. A file with
//!    its final name is a verified file.
//! 4. Free space is checked on every destination before anything is written.
//! 5. The source's folder structure is reproduced exactly (empty folders included);
//!    only known OS junk is skipped, and only when asked.

pub mod error;
pub mod event;
pub mod format;
pub mod hash;
pub mod job;
pub mod journal;
pub mod log;
pub mod mhl;
pub mod plan;
pub mod platform;
pub mod reverify;
pub mod scan;

mod pipeline;
mod serde_util;
mod transfer;

pub use error::{FileFailure, PrepareError};
pub use event::{Event, FileStatus, Outcome, Pass, Stage};
pub use hash::{HashConfig, Hashes};
pub use job::{
    CopyMode, DestRole, Destination, Faults, FileReport, Job, JobReport, JobSpec, Resume, Sidecars, VerifiedFile,
};
pub use plan::{Plan, PlannedFile};
pub use scan::{SourceInfo, SourceKind};

/// Working product name. Used for the log header and the default log folder.
pub const APP_NAME: &str = "Media Offloader";

/// Engine version, written into every transfer log.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
