//! Windows.

mod direct_io;
mod keep_awake;
mod rename;
mod volumes;

use std::io;

pub use direct_io::{UncachedReader, create_partial, evict_from_cache, open_source};
pub use keep_awake::KeepAwake;
pub use rename::{rename_no_replace, sync_dir};
pub use volumes::{eject, list_volumes, volume_info};

fn is_err(e: &io::Error, code: u32) -> bool {
    e.raw_os_error() == Some(code as i32)
}
