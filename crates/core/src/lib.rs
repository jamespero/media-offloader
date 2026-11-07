//! Copy + verify engine for camera-card offloads.
//!
//! UI-agnostic: the `offload` CLI and the desktop app both drive it the same way.

pub mod hash;
pub mod platform;

mod pipeline;

pub use hash::{HashConfig, Hashes};

/// Working product name. Used for the log header and the default log folder.
pub const APP_NAME: &str = "Media Offloader";

/// Engine version, written into every transfer log.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
