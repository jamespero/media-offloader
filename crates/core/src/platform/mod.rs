//! OS-specific file operations: cache-bypassing reads, no-replace renames, fsync,
//! free space and volume details. Windows and macOS are the targets; other Unixes get
//! best-effort fallbacks so the engine still builds and tests there.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as imp;

/// While held, keeps the computer from going to sleep (a transfer is running).
pub use imp::KeepAwake;
pub use imp::UncachedReader;

/// Facts about the volume a path lives on.
#[derive(Clone, Debug)]
pub struct VolumeInfo {
    /// Identifies the volume, for spotting two destinations on the same drive.
    pub id: String,
    /// Where the volume is mounted (`E:\`, `/Volumes/CARD`).
    pub mount_root: PathBuf,
    /// Volume label, if it has one.
    pub label: Option<String>,
    /// File system name as the OS reports it (`NTFS`, `exFAT`, `apfs`, `msdos`…).
    pub fs_type: String,
    /// Bytes available to this user.
    pub available: u64,
    /// Capacity of the volume.
    pub total: u64,
    /// Allocation unit; every file occupies a whole number of these.
    pub cluster_size: u64,
}

impl VolumeInfo {
    /// Whether file names must follow Windows rules (no `:*?"<>|`, no trailing dots or
    /// spaces, no CON/NUL/…). Always true on Windows; on macOS for FAT/exFAT/NTFS/SMB.
    pub fn windows_name_rules(&self) -> bool {
        cfg!(windows)
            || matches!(
                self.fs_type.to_ascii_lowercase().as_str(),
                "msdos" | "exfat" | "ntfs" | "smbfs" | "cifs" | "vfat" | "fat" | "fat32" | "fuseblk"
            )
    }
}

/// A mounted volume that could be a source: a card, or an external drive.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeEntry {
    pub path: PathBuf,
    /// Volume label, or the drive letter / mount name when it has none.
    pub label: String,
    /// A card slot or stick rather than a drive. Windows: the OS says so. macOS:
    /// FAT/exFAT volumes, which is what cards use.
    pub removable: bool,
    pub fs_type: String,
    pub total: u64,
    pub available: u64,
}

/// Volumes that could be offloaded from, excluding the system drive. Card-reader slots
/// with no card in them are left out.
pub fn list_volumes() -> Vec<VolumeEntry> {
    imp::list_volumes()
}

/// Volume details for `path`, which may not exist yet: its nearest existing ancestor is used.
pub fn volume_info(path: &Path) -> io::Result<VolumeInfo> {
    imp::volume_info(&nearest_existing(path)?)
}

/// The closest ancestor of `path` (or `path` itself) that exists, made absolute.
pub fn nearest_existing(path: &Path) -> io::Result<PathBuf> {
    let abs = std::path::absolute(path)?;
    let mut cur: &Path = &abs;
    loop {
        if fs::symlink_metadata(cur).is_ok() {
            return Ok(cur.to_path_buf());
        }
        cur = cur
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("no part of {} exists", path.display())))?;
    }
}

/// `path` with symlinks and `..` resolved, even if its tail doesn't exist yet.
pub fn canonical_lenient(path: &Path) -> io::Result<PathBuf> {
    let existing = nearest_existing(path)?;
    let abs = std::path::absolute(path)?;
    let tail = abs.strip_prefix(&existing).unwrap_or(Path::new(""));
    Ok(fs::canonicalize(&existing)?.join(tail))
}

/// Renames `from` to `to`, failing with `AlreadyExists` rather than replacing `to`.
pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    imp::rename_no_replace(from, to)
}

/// Makes a card or drive safe to remove (dismount/eject). Refuses folders and the
/// system drive.
pub fn eject(mount_root: &Path) -> io::Result<()> {
    let v = volume_info(mount_root)?;
    let system = if cfg!(windows) { std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into()) } else { "/".into() };
    if let Some(e) = eject_refusal(&std::path::absolute(mount_root)?, &v.mount_root, Path::new(&system)) {
        return Err(e);
    }
    imp::eject(mount_root)
}

/// Why `requested` mustn't be ejected, if it mustn't: it isn't the root of the volume it
/// lives on (`mount_root`), or that volume is the system drive. Pure, so it can be
/// tested without going near a real disk.
fn eject_refusal(requested: &Path, mount_root: &Path, system: &Path) -> Option<io::Error> {
    let norm = |p: &Path| p.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase();
    if norm(mount_root) != norm(requested) {
        return Some(io::Error::new(io::ErrorKind::InvalidInput, "that's a folder, not a card or drive"));
    }
    if norm(mount_root) == norm(system) {
        return Some(io::Error::new(io::ErrorKind::PermissionDenied, "that's the system drive"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eject_guard_refuses_folders_and_the_system_drive() {
        let p = Path::new;
        // Windows-style
        assert!(eject_refusal(p(r"E:\"), p(r"E:\"), p("C:")).is_none());
        assert!(eject_refusal(p(r"e:"), p(r"E:\"), p("C:")).is_none());
        assert!(eject_refusal(p(r"C:\"), p(r"C:\"), p("C:")).is_some());
        assert!(eject_refusal(p(r"E:\DCIM"), p(r"E:\"), p("C:")).is_some());
        // macOS-style
        assert!(eject_refusal(p("/Volumes/CARD"), p("/Volumes/CARD"), p("/")).is_none());
        assert!(eject_refusal(p("/"), p("/"), p("/")).is_some());
        assert!(eject_refusal(p("/Users/me/Desktop"), p("/"), p("/")).is_some());
    }
}
