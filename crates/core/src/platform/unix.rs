use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use super::VolumeInfo;

fn cstr(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in path"))
}

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

#[cfg(target_os = "macos")]
pub fn volume_info(existing: &Path) -> io::Result<VolumeInfo> {
    use std::ffi::CStr;
    use std::path::PathBuf;

    let p = cstr(existing)?;
    // SAFETY: zeroed statfs is a valid out-parameter; p is NUL-terminated.
    let mut st: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(p.as_ptr(), &mut st) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the kernel NUL-terminates these fixed-size name fields.
    let fs_type = unsafe { CStr::from_ptr(st.f_fstypename.as_ptr()) }.to_string_lossy().into_owned();
    let mount = unsafe { CStr::from_ptr(st.f_mntonname.as_ptr()) }.to_string_lossy().into_owned();
    let mount_root = PathBuf::from(&mount);
    let label = mount_root
        .strip_prefix("/Volumes")
        .ok()
        .and_then(|rest| rest.file_name())
        .map(|n| n.to_string_lossy().into_owned());
    Ok(VolumeInfo {
        id: mount,
        mount_root,
        label,
        fs_type,
        available: st.f_bavail * u64::from(st.f_bsize),
        total: st.f_blocks * u64::from(st.f_bsize),
        cluster_size: u64::from(st.f_bsize).max(512),
    })
}

#[cfg(not(target_os = "macos"))]
pub fn volume_info(existing: &Path) -> io::Result<VolumeInfo> {
    use std::os::unix::fs::MetadataExt;

    let p = cstr(existing)?;
    // SAFETY: zeroed statvfs is a valid out-parameter; p is NUL-terminated.
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(p.as_ptr(), &mut st) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let dev = fs::metadata(existing)?.dev();
    // Walk up while still on the same device to find the mount point.
    let mut mount_root = existing.to_path_buf();
    while let Some(parent) = mount_root.parent() {
        match fs::metadata(parent) {
            Ok(m) if m.dev() == dev => mount_root = parent.to_path_buf(),
            _ => break,
        }
    }
    Ok(VolumeInfo {
        id: format!("dev:{dev}"),
        mount_root,
        label: None,
        fs_type: String::new(),
        available: st.f_bavail as u64 * st.f_frsize as u64,
        total: st.f_blocks as u64 * st.f_frsize as u64,
        cluster_size: (st.f_frsize as u64).max(512),
    })
}

#[cfg(target_os = "macos")]
pub fn list_volumes() -> Vec<super::VolumeEntry> {
    let Ok(entries) = fs::read_dir("/Volumes") else { return Vec::new() };
    let mut out = Vec::new();
    for e in entries.flatten() {
        let path = e.path();
        // The boot volume appears as a symlink to "/".
        if e.file_type().map(|t| t.is_symlink()).unwrap_or(true) {
            continue;
        }
        let Ok(v) = volume_info(&path) else { continue };
        if v.mount_root != path {
            continue; // a plain folder, not a mount point
        }
        let removable = matches!(v.fs_type.as_str(), "msdos" | "exfat");
        out.push(super::VolumeEntry {
            label: v.label.unwrap_or_else(|| e.file_name().to_string_lossy().into_owned()),
            path,
            removable,
            fs_type: v.fs_type,
            total: v.total,
            available: v.available,
        });
    }
    out.sort_by_key(|v| v.label.to_lowercase());
    out
}

#[cfg(not(target_os = "macos"))]
pub fn list_volumes() -> Vec<super::VolumeEntry> {
    Vec::new()
}

/// Unmounts (and on macOS ejects) a volume so the card is safe to pull.
pub fn eject(mount_root: &Path) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    let out = std::process::Command::new("diskutil").arg("eject").arg(mount_root).output()?;
    #[cfg(not(target_os = "macos"))]
    let out = std::process::Command::new("umount").arg(mount_root).output()?;
    if out.status.success() {
        return Ok(());
    }
    let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(io::Error::other(if msg.is_empty() {
        "the volume couldn't be ejected (is it in use?)".to_string()
    } else {
        msg
    }))
}

/// Keeps the Mac from idle-sleeping while held, via `caffeinate` tied to our process
/// (so it ends even if we crash). No-op elsewhere.
pub struct KeepAwake(Option<std::process::Child>);

#[allow(clippy::new_without_default)] // acquiring it has an effect; no silent defaults
impl KeepAwake {
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            let child = std::process::Command::new("caffeinate")
                .args(["-i", "-w", &std::process::id().to_string()])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .ok();
            KeepAwake(child)
        }
        #[cfg(not(target_os = "macos"))]
        KeepAwake(None)
    }
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        if let Some(mut c) = self.0.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}
