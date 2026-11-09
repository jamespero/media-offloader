//! The step that makes a file count as done.

use std::io;
use std::path::Path;

use crate::error::FileFailure;
use crate::event::FileStatus;
use crate::platform;
use crate::transfer;

/// Gives a verified temp file its real name, without ever replacing anything.
pub(super) fn finalize(partial: &Path, final_path: &Path) -> FileStatus {
    match platform::rename_no_replace(partial, final_path) {
        Ok(()) => {
            if let Some(dir) = final_path.parent() {
                let _ = platform::sync_dir(dir);
            }
            FileStatus::Verified
        }
        Err(e) => {
            transfer::discard(partial);
            let failure = if e.kind() == io::ErrorKind::AlreadyExists {
                FileFailure::AlreadyExists { path: final_path.display().to_string() }
            } else {
                FileFailure::write(&e)
            };
            FileStatus::Failed { failure }
        }
    }
}
