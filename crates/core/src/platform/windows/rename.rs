//! Renaming without ever replacing anything.

use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION};
use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};

use super::is_err;

pub fn sync_dir(_dir: &Path) -> io::Result<()> {
    Ok(())
}

pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    let (from_w, to_w) = (verbatim_wide(from)?, verbatim_wide(to)?);
    // Without MOVEFILE_REPLACE_EXISTING this fails if `to` exists. Antivirus and the
    // search indexer briefly open new files, so retry sharing violations a few times.
    let mut attempt = 0;
    loop {
        // SAFETY: both are NUL-terminated wide strings that outlive the call.
        if unsafe { MoveFileExW(from_w.as_ptr(), to_w.as_ptr(), MOVEFILE_WRITE_THROUGH) } != 0 {
            return Ok(());
        }
        let err = io::Error::last_os_error();
        let transient = is_err(&err, ERROR_SHARING_VIOLATION) || is_err(&err, ERROR_ACCESS_DENIED);
        if transient && attempt < 8 && !to.exists() {
            attempt += 1;
            sleep(Duration::from_millis(25 << attempt.min(4)));
            continue;
        }
        return Err(err);
    }
}

/// Absolute `\\?\` form, so paths longer than MAX_PATH work in raw Win32 calls.
fn verbatim_wide(path: &Path) -> io::Result<Vec<u16>> {
    let abs = std::path::absolute(path)?;
    let s: Vec<u16> = abs.as_os_str().encode_wide().collect();
    let bs = u16::from(b'\\');
    let q = u16::from(b'?');
    let mut out: Vec<u16> = Vec::with_capacity(s.len() + 8);
    if s.starts_with(&[bs, bs, q, bs]) {
        out.extend_from_slice(&s);
    } else if s.starts_with(&[bs, bs]) {
        out.extend("\\\\?\\UNC\\".encode_utf16());
        out.extend_from_slice(&s[2..]);
    } else {
        out.extend("\\\\?\\".encode_utf16());
        out.extend_from_slice(&s);
    }
    out.push(0);
    Ok(out)
}
