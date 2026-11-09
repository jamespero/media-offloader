//! Reading past the OS cache, and opening the files a transfer works on.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::ERROR_INVALID_PARAMETER;
use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_NO_BUFFERING, FILE_FLAG_SEQUENTIAL_SCAN};

use super::is_err;
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

pub fn open_source(path: &Path) -> io::Result<File> {
    OpenOptions::new().read(true).custom_flags(FILE_FLAG_SEQUENTIAL_SCAN).open(path)
}

pub fn create_partial(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

pub fn evict_from_cache(_path: &Path) {}
