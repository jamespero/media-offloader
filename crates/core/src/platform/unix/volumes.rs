//! Volume details, listing sources, and ejecting.

use std::fs;
use std::io;
use std::path::Path;

use super::cstr;
use crate::platform::VolumeInfo;

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
pub fn list_volumes() -> Vec<crate::platform::VolumeEntry> {
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
        out.push(crate::platform::VolumeEntry {
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
pub fn list_volumes() -> Vec<crate::platform::VolumeEntry> {
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
