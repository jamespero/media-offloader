//! macOS, with best-effort fallbacks on other Unixes.

mod direct_io;
mod keep_awake;
mod rename;
mod volumes;

use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub use direct_io::{UncachedReader, create_partial, evict_from_cache, open_source};
pub use keep_awake::KeepAwake;
pub use rename::{rename_no_replace, sync_dir};
pub use volumes::{eject, list_volumes, volume_info};

fn cstr(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in path"))
}
