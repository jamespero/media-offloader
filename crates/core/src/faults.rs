//! Test hooks for simulating failures.

use std::io;
use std::path::Path;

/// Test hooks for simulating failures. Not part of the stable API.
#[doc(hidden)]
pub trait Faults: Send + Sync {
    /// Called after each read from a source file, with the bytes read so far.
    fn after_source_read(&self, _src: &Path, _offset: u64) {}
    /// Return an error to make the next read from `src` fail.
    fn source_read_error(&self, _src: &Path, _offset: u64) -> Option<io::Error> {
        None
    }
    /// Called with a fully written `.partial` right before it is verified.
    fn before_verify(&self, _partial: &Path) {}
    /// Override the free space reported for a destination.
    fn available_space(&self, _dest: &Path) -> Option<u64> {
        None
    }
}
