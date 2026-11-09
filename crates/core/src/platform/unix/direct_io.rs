//! Reading past the OS cache, and opening the files a transfer works on.

use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::io::AsRawFd;
use std::path::Path;

/// Reads a file without going through the OS page cache.
///
/// macOS: `F_NOCACHE`. That alone doesn't evict pages already cached, which is why
/// destination files are also written with `F_NOCACHE` and evicted with
/// [`evict_from_cache`] before the verify read. Linux (not a target, used for tests):
/// `POSIX_FADV_DONTNEED` on the fsynced file.
pub struct UncachedReader {
    file: File,
    len: u64,
    bypassed: bool,
}

impl UncachedReader {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        let bypassed = no_cache(&file) || drop_cached_pages(&file);
        Ok(Self { file, len, bypassed })
    }

    pub fn cache_bypassed(&self) -> bool {
        self.bypassed
    }

    pub fn file_len(&self) -> u64 {
        self.len
    }
}

impl Read for UncachedReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

/// Turns off caching for this descriptor (macOS). Returns whether it took.
#[cfg(target_os = "macos")]
fn no_cache(file: &File) -> bool {
    // SAFETY: valid fd for the lifetime of `file`.
    unsafe { libc::fcntl(file.as_raw_fd(), libc::F_NOCACHE, 1) != -1 }
}

#[cfg(not(target_os = "macos"))]
fn no_cache(_file: &File) -> bool {
    false
}

/// Asks the kernel to drop this file's clean cached pages (Linux).
#[cfg(any(target_os = "linux", target_os = "android"))]
fn drop_cached_pages(file: &File) -> bool {
    // SAFETY: valid fd; offset 0 / len 0 means the whole file.
    unsafe { libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) == 0 }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn drop_cached_pages(_file: &File) -> bool {
    false
}

pub fn open_source(path: &Path) -> io::Result<File> {
    File::open(path)
}

pub fn create_partial(path: &Path) -> io::Result<File> {
    let f = OpenOptions::new().write(true).create_new(true).open(path)?;
    // Keep what we write out of the cache so the verify read can't be served from it.
    no_cache(&f);
    Ok(f)
}

#[cfg(target_os = "macos")]
pub fn evict_from_cache(path: &Path) {
    // msync(MS_INVALIDATE) on a shared mapping throws away the file's cached pages
    // (the same trick vmtouch uses to evict on macOS). Best effort.
    let Ok(file) = File::open(path) else { return };
    let Ok(len) = file.metadata().map(|m| m.len() as usize) else { return };
    if len == 0 {
        return;
    }
    // SAFETY: read-only shared mapping of a file we hold open; unmapped before return.
    unsafe {
        let p = libc::mmap(std::ptr::null_mut(), len, libc::PROT_READ, libc::MAP_SHARED, file.as_raw_fd(), 0);
        if p != libc::MAP_FAILED {
            libc::msync(p, len, libc::MS_INVALIDATE);
            libc::munmap(p, len);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn evict_from_cache(path: &Path) {
    if let Ok(f) = File::open(path) {
        drop_cached_pages(&f);
    }
}
