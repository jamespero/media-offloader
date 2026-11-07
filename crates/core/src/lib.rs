//! Copy + verify engine for camera-card offloads.
//!
//! UI-agnostic: the `offload` CLI and the desktop app both drive it the same way.

pub mod error;
pub mod format;
pub mod hash;
pub mod plan;
pub mod platform;
pub mod scan;

mod pipeline;
mod serde_util;

pub use error::PrepareError;
pub use hash::{HashConfig, Hashes};
pub use plan::{Plan, PlannedFile};
pub use scan::{SourceInfo, SourceKind};

/// Working product name. Used for the log header and the default log folder.
pub const APP_NAME: &str = "Media Offloader";

/// Engine version, written into every transfer log.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
