use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER, ERROR_SHARING_VIOLATION};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_NO_BUFFERING, FILE_FLAG_SEQUENTIAL_SCAN, GetDiskFreeSpaceExW, GetDiskFreeSpaceW, GetDriveTypeW,
    GetLogicalDrives, GetVolumeInformationW, GetVolumePathNameW, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};

use super::VolumeInfo;
use crate::pipeline::BUF_ALIGN;

/// Reads a file with `FILE_FLAG_NO_BUFFERING`, so the bytes come from the disk rather
/// than the system cache that still holds what we just wrote.
///
/// Unbuffered reads need sector-aligned buffers, sizes and offsets. The pipeline's
/// buffers are aligned and every read but the last is a whole chunk, so this holds; if
/// it ever doesn't (a short read mid-file, a network share that refuses), the reader
/// reopens the file normally and carries on, and `cache_bypassed()` turns false so the
/// log can say so.
pub struct UncachedReader {
    file: File,
    path: PathBuf,
    pos: u64,
    len: u64,
    direct: bool,
    eof: bool,
}

impl UncachedReader {
    pub fn open(path: &Path) -> io::Result<Self> {
        let (file, direct) = match OpenOptions::new().read(true).custom_flags(FILE_FLAG_NO_BUFFERING).open(path) {
            Ok(f) => (f, true),
            Err(e) if is_err(&e, ERROR_INVALID_PARAMETER) => (File::open(path)?, false),
            Err(e) => return Err(e),
        };
        let len = file.metadata()?.len();
        Ok(Self { file, path: path.to_path_buf(), pos: 0, len, direct, eof: false })
    }

    pub fn cache_bypassed(&self) -> bool {
        self.direct
    }

    pub fn file_len(&self) -> u64 {
        self.len
    }

    fn fall_back(&mut self) -> io::Result<()> {
        let mut f = File::open(&self.path)?;
        f.seek(SeekFrom::Start(self.pos))?;
        self.file = f;
        self.direct = false;
        Ok(())
    }
}

impl Read for UncachedReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.eof || buf.is_empty() {
            return Ok(0);
        }
        if self.direct {
            let aligned = (buf.as_ptr() as usize).is_multiple_of(BUF_ALIGN) && buf.len().is_multiple_of(BUF_ALIGN);
            if aligned {
                match self.file.read(buf) {
                    Ok(n) => {
                        self.pos += n as u64;
                        if n < buf.len() {
                            if self.pos >= self.len {
                                self.eof = true; // unbuffered reads return short only at EOF
                            } else {
                                self.fall_back()?;
                            }
                        }
                        return Ok(n);
                    }
                    Err(e) if is_err(&e, ERROR_INVALID_PARAMETER) => self.fall_back()?,
                    Err(e) => return Err(e),
                }
            } else {
                self.fall_back()?;
            }
        }
        let n = self.file.read(buf)?;
        self.pos += n as u64;
        Ok(n)
    }
}

fn is_err(e: &io::Error, code: u32) -> bool {
    e.raw_os_error() == Some(code as i32)
}

pub fn open_source(path: &Path) -> io::Result<File> {
    OpenOptions::new().read(true).custom_flags(FILE_FLAG_SEQUENTIAL_SCAN).open(path)
}

pub fn create_partial(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

pub fn evict_from_cache(_path: &Path) {}

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

/// Keeps the computer from sleeping while held (system sleep only; the display may
/// still turn off). Must be dropped on the thread that created it.
pub struct KeepAwake(());

#[allow(clippy::new_without_default)] // acquiring it has an effect; no silent defaults
impl KeepAwake {
    pub fn new() -> Self {
        use windows_sys::Win32::System::Power::{ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetThreadExecutionState};
        // SAFETY: plain flag call.
        unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) };
        KeepAwake(())
    }
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        use windows_sys::Win32::System::Power::{ES_CONTINUOUS, SetThreadExecutionState};
        // SAFETY: plain flag call; clears this thread's requirement.
        unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
    }
}

pub fn list_volumes() -> Vec<super::VolumeEntry> {
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
        out.push(super::VolumeEntry {
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

fn from_wide(buf: &[u16]) -> String {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
    s.encode_wide().chain(Some(0)).collect()
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
