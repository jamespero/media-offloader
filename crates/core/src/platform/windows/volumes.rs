//! Volume details, listing sources, and ejecting.

use std::ffi::OsString;
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDiskFreeSpaceW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW, GetVolumePathNameW,
};

use crate::platform::{VolumeEntry, VolumeInfo};

pub fn volume_info(existing: &Path) -> io::Result<VolumeInfo> {
    let path_w = wide(existing.as_os_str());
    let mut root = vec![0u16; 1024];
    // SAFETY: buffers are valid for the lengths passed.
    if unsafe { GetVolumePathNameW(path_w.as_ptr(), root.as_mut_ptr(), root.len() as u32) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let root_len = root.iter().position(|&c| c == 0).unwrap_or(root.len());
    root.truncate(root_len + 1);

    let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
    // SAFETY: valid pointers to locals; root is NUL-terminated.
    if unsafe { GetDiskFreeSpaceExW(root.as_ptr(), &mut avail, &mut total, &mut free) } == 0 {
        return Err(io::Error::last_os_error());
    }

    let (mut spc, mut bps, mut fc, mut tc) = (0u32, 0u32, 0u32, 0u32);
    // SAFETY: as above.
    let cluster_size = if unsafe { GetDiskFreeSpaceW(root.as_ptr(), &mut spc, &mut bps, &mut fc, &mut tc) } != 0 {
        u64::from(spc) * u64::from(bps)
    } else {
        4096
    };

    let mut label = [0u16; 261];
    let mut fs_name = [0u16; 261];
    let mut serial = 0u32;
    // SAFETY: buffers are valid for the lengths passed; unused outputs are null.
    let ok = unsafe {
        GetVolumeInformationW(
            root.as_ptr(),
            label.as_mut_ptr(),
            label.len() as u32,
            &mut serial,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            fs_name.as_mut_ptr(),
            fs_name.len() as u32,
        )
    } != 0;
    let mount_root = PathBuf::from(OsString::from_wide(&root[..root_len]));
    let (label, fs_type) = if ok { (from_wide(&label), from_wide(&fs_name)) } else { (String::new(), String::new()) };

    Ok(VolumeInfo {
        id: format!("{}#{serial:08x}", mount_root.display()).to_uppercase(),
        mount_root,
        label: (!label.is_empty()).then_some(label),
        fs_type,
        available: avail,
        total,
        cluster_size: cluster_size.max(512),
    })
}

pub fn list_volumes() -> Vec<VolumeEntry> {
    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;
    let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into()).to_uppercase();
    // SAFETY: no arguments.
    let mask = unsafe { GetLogicalDrives() };
    let mut out = Vec::new();
    for (i, letter) in (b'A'..=b'Z').enumerate() {
        if mask & (1 << i) == 0 {
            continue;
        }
        let drive = format!("{}:", letter as char);
        if drive == system {
            continue;
        }
        let root = PathBuf::from(format!("{drive}\\"));
        let root_w = wide(root.as_os_str());
        // SAFETY: NUL-terminated wide string.
        let kind = unsafe { GetDriveTypeW(root_w.as_ptr()) };
        if kind != DRIVE_REMOVABLE && kind != DRIVE_FIXED {
            continue; // network, optical, RAM disks
        }
        // An empty card-reader slot fails here ("device not ready").
        let Ok(v) = volume_info(&root) else { continue };
        out.push(VolumeEntry {
            label: v.label.unwrap_or_else(|| drive.clone()),
            path: root,
            removable: kind == DRIVE_REMOVABLE,
            fs_type: v.fs_type,
            total: v.total,
            available: v.available,
        });
    }
    out
}

/// Makes a card safe to pull: lock the volume (fails if anything has files open on it),
/// dismount it, allow removal, and eject the media (the documented sequence for
/// removable media). Fixed USB drives can't always be ejected this way, but once
/// dismounted nothing is writing to them.
pub fn eject(mount_root: &Path) -> io::Result<()> {
    use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{
        FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME, IOCTL_STORAGE_EJECT_MEDIA, IOCTL_STORAGE_MEDIA_REMOVAL,
        PREVENT_MEDIA_REMOVAL,
    };

    let s = mount_root.to_string_lossy();
    let letter = s.trim_end_matches(['\\', '/']);
    if letter.len() != 2 || !letter.ends_with(':') {
        return Err(io::Error::new(io::ErrorKind::Unsupported, "only drives with a letter can be ejected"));
    }
    let device = wide(std::ffi::OsStr::new(&format!("\\\\.\\{letter}")));
    // SAFETY: valid NUL-terminated path; null security attributes and template are allowed.
    let h = unsafe {
        CreateFileW(
            device.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    let ioctl = |code: u32, input: *const std::ffi::c_void, len: u32| -> bool {
        let mut returned = 0u32;
        // SAFETY: h is a valid volume handle; buffers are valid for the given sizes.
        unsafe {
            DeviceIoControl(h, code, input, len, std::ptr::null_mut(), 0, &mut returned, std::ptr::null_mut()) != 0
        }
    };
    let result = (|| {
        // Explorer and the indexer let go of a fresh card after a moment; retry briefly.
        let mut locked = false;
        for _ in 0..20 {
            if ioctl(FSCTL_LOCK_VOLUME, std::ptr::null(), 0) {
                locked = true;
                break;
            }
            sleep(Duration::from_millis(100));
        }
        if !locked {
            return Err(io::Error::new(
                io::ErrorKind::ResourceBusy,
                "it's in use — close any windows or apps that have files open on it",
            ));
        }
        if !ioctl(FSCTL_DISMOUNT_VOLUME, std::ptr::null(), 0) {
            return Err(io::Error::last_os_error());
        }
        let allow = PREVENT_MEDIA_REMOVAL { PreventMediaRemoval: false };
        ioctl(
            IOCTL_STORAGE_MEDIA_REMOVAL,
            (&allow as *const PREVENT_MEDIA_REMOVAL).cast(),
            std::mem::size_of::<PREVENT_MEDIA_REMOVAL>() as u32,
        );
        // Not every device supports eject (fixed USB drives); dismounted is still safe.
        ioctl(IOCTL_STORAGE_EJECT_MEDIA, std::ptr::null(), 0);
        Ok(())
    })();
    // SAFETY: h came from CreateFileW above and is closed once.
    unsafe { CloseHandle(h) };
    result
}

fn from_wide(buf: &[u16]) -> String {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
    s.encode_wide().chain(Some(0)).collect()
}
