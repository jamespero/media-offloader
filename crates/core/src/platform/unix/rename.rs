//! Renaming without ever replacing anything.

use std::fs::{self, File};
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::Path;

#[cfg(target_os = "macos")]
use super::cstr;

pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let (f, t) = (cstr(from)?, cstr(to)?);
        // SAFETY: NUL-terminated paths valid for the call.
        if unsafe { libc::renamex_np(f.as_ptr(), t.as_ptr(), libc::RENAME_EXCL) } == 0 {
            return Ok(());
        }
        let err = io::Error::last_os_error();
        // Some file systems (exFAT, SMB) don't support RENAME_EXCL; fall through.
        if !matches!(err.raw_os_error(), Some(libc::ENOTSUP) | Some(libc::EINVAL)) {
            return Err(err);
        }
    }
    // Fallback: nothing else writes into a fresh offload folder, so check-then-rename
    // only races with ourselves.
    if fs::symlink_metadata(to).is_ok() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{} already exists", to.display())));
    }
    fs::rename(from, to)
}

pub fn sync_dir(dir: &Path) -> io::Result<()> {
    let d = File::open(dir)?;
    // SAFETY: valid fd. Plain fsync: F_FULLFSYNC isn't supported on directories everywhere.
    if unsafe { libc::fsync(d.as_raw_fd()) } == 0 {
        return Ok(());
    }
    let err = io::Error::last_os_error();
    match err.raw_os_error() {
        Some(libc::EINVAL) | Some(libc::ENOTSUP) => Ok(()),
        _ => Err(err),
    }
}
