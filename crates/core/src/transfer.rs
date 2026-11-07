//! One file: read it once into `.partial` temp files on every target, then re-read each
//! target from disk and compare. Renaming to the final name is the job's call.

use std::fs::{self, File, FileTimes};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::SystemTime;

use crate::hash::{HashConfig, Hashes, Md5Sink, Xxh64Sink};
use crate::job::Faults;
use crate::pipeline::{self, BufferPool, ChunkSink, PumpError};
use crate::platform::{self, UncachedReader};

/// What we know about the source file from the scan; the copy must match it.
pub(crate) struct SourceExpect {
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
}

pub(crate) enum CopyOutcome {
    /// The source was read completely and unchanged. Each target either holds a
    /// fully written, fsynced `.partial` (Ok) or failed and was cleaned up (Err).
    Copied {
        hashes: Hashes,
        targets: Vec<io::Result<()>>,
    },
    /// The source couldn't be read (all partials removed).
    SourceError(io::Error),
    /// The source changed during or since the scan (all partials removed).
    SourceChanged(&'static str),
    Cancelled,
}

pub(crate) enum VerifyOutcome {
    Match { cache_bypassed: bool },
    Mismatch { actual: Hashes },
    ReadError(io::Error),
    Cancelled,
}

struct WriteSink(File);

impl ChunkSink for WriteSink {
    fn consume(&mut self, data: &[u8]) -> io::Result<()> {
        self.0.write_all(data)
    }
}

/// Wraps the source reader for fault injection in tests.
struct SourceReader<'a> {
    file: File,
    path: &'a Path,
    offset: u64,
    faults: Option<&'a dyn Faults>,
}

impl Read for SourceReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(e) = self.faults.and_then(|f| f.source_read_error(self.path, self.offset)) {
            return Err(e);
        }
        let n = self.file.read(buf)?;
        self.offset += n as u64;
        if let Some(f) = self.faults {
            f.after_source_read(self.path, self.offset);
        }
        Ok(n)
    }
}

pub(crate) fn discard(path: &Path) {
    let _ = fs::remove_file(path);
}

fn unchanged(meta: &fs::Metadata, expect: &SourceExpect) -> bool {
    meta.len() == expect.size && (expect.modified.is_none() || meta.modified().ok() == expect.modified)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn copy_to_partials(
    src: &Path,
    expect: &SourceExpect,
    partials: &[PathBuf],
    cfg: HashConfig,
    pool: &BufferPool,
    cancel: &AtomicBool,
    faults: Option<&dyn Faults>,
    progress: &mut dyn FnMut(u64),
) -> CopyOutcome {
    match fs::metadata(src) {
        Ok(m) if unchanged(&m, expect) => {}
        Ok(_) => return CopyOutcome::SourceChanged("after it was scanned"),
        Err(e) => return CopyOutcome::SourceError(e),
    }
    let file = match platform::open_source(src) {
        Ok(f) => f,
        Err(e) => return CopyOutcome::SourceError(e),
    };
    let mut reader = SourceReader { file, path: src, offset: 0, faults };

    // Open every target; a target that can't be created just fails on its own.
    let mut targets: Vec<io::Result<()>> = Vec::with_capacity(partials.len());
    let mut writers: Vec<(usize, WriteSink)> = Vec::new();
    for (i, p) in partials.iter().enumerate() {
        match platform::create_partial(p) {
            Ok(f) => {
                targets.push(Ok(()));
                writers.push((i, WriteSink(f)));
            }
            Err(e) => targets.push(Err(e)),
        }
    }
    let mut xxh = Xxh64Sink::new();
    let mut md5 = cfg.md5.then(Md5Sink::new);
    let pumped = {
        let mut sinks: Vec<&mut dyn ChunkSink> = vec![&mut xxh];
        if let Some(m) = md5.as_mut() {
            sinks.push(m);
        }
        for (_, w) in writers.iter_mut() {
            sinks.push(w);
        }
        pipeline::pump(&mut reader, &mut sinks, expect.size, pool, cancel, progress)
    };
    let pumped = match pumped {
        Ok(p) => p,
        Err(PumpError::Cancelled) => {
            abandon(writers, partials);
            return CopyOutcome::Cancelled;
        }
        Err(PumpError::Read(e)) => {
            abandon(writers, partials);
            return CopyOutcome::SourceError(e);
        }
    };
    let hashes = Hashes { xxh64: xxh.finish(), md5: md5.map(Md5Sink::finish) };

    // Same size and timestamp as scanned, and we read exactly that many bytes?
    let still_same = fs::metadata(src).map(|m| unchanged(&m, expect)).unwrap_or(false);
    if !still_same || pumped.bytes != expect.size {
        abandon(writers, partials);
        return CopyOutcome::SourceChanged("while it was being copied");
    }

    // The first sink errors belong to the hashers (which never fail); the rest map to writers.
    let hash_sinks = 1 + usize::from(cfg.md5);
    let mut times = FileTimes::new();
    if let Some(m) = expect.modified {
        times = times.set_modified(m);
    }
    if let Some(a) = expect.accessed {
        times = times.set_accessed(a);
    }
    times = with_created(times, expect.created);
    for ((i, w), err) in writers.into_iter().zip(pumped.sink_errors.into_iter().skip(hash_sinks)) {
        let result = match err {
            Some(e) => Err(e),
            // Timestamps first (they're metadata the fsync should cover), then flush to disk.
            None => w.0.set_times(times).and_then(|()| w.0.sync_all()),
        };
        drop(w);
        if let Err(e) = result {
            discard(&partials[i]);
            targets[i] = Err(e);
        }
    }
    CopyOutcome::Copied { hashes, targets }
}

/// Closes the temp files (Windows can't delete open files), then deletes them.
fn abandon(writers: Vec<(usize, WriteSink)>, partials: &[PathBuf]) {
    let indices: Vec<usize> = writers.iter().map(|(i, _)| *i).collect();
    drop(writers);
    for i in indices {
        discard(&partials[i]);
    }
}

#[cfg(windows)]
fn with_created(times: FileTimes, created: Option<SystemTime>) -> FileTimes {
    use std::os::windows::fs::FileTimesExt;
    match created {
        Some(c) => times.set_created(c),
        None => times,
    }
}

#[cfg(target_os = "macos")]
fn with_created(times: FileTimes, created: Option<SystemTime>) -> FileTimes {
    use std::os::macos::fs::FileTimesExt;
    match created {
        Some(c) => times.set_created(c),
        None => times,
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn with_created(times: FileTimes, _created: Option<SystemTime>) -> FileTimes {
    times
}

/// Re-reads `path` from disk (cache bypassed) and compares it with `expected`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn verify(
    path: &Path,
    expected: &Hashes,
    expected_size: u64,
    cfg: HashConfig,
    pool: &BufferPool,
    cancel: &AtomicBool,
    faults: Option<&dyn Faults>,
    progress: &mut dyn FnMut(u64),
) -> VerifyOutcome {
    if let Some(f) = faults {
        f.before_verify(path);
    }
    platform::evict_from_cache(path);
    let mut reader = match UncachedReader::open(path) {
        Ok(r) => r,
        Err(e) => return VerifyOutcome::ReadError(e),
    };
    match pipeline::hash_stream(&mut reader, expected_size, cfg, pool, cancel, progress) {
        Ok((actual, bytes)) => {
            if bytes == expected_size && actual.matches(expected) {
                VerifyOutcome::Match { cache_bypassed: reader.cache_bypassed() }
            } else {
                VerifyOutcome::Mismatch { actual }
            }
        }
        Err(PumpError::Cancelled) => VerifyOutcome::Cancelled,
        Err(PumpError::Read(e)) => VerifyOutcome::ReadError(e),
    }
}
