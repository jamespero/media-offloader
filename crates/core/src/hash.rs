//! Checksums. xxHash64 is always computed; MD5 is optional and comes from the same read.

use std::fmt;
use std::io;
use std::path::Path;

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize, Serializer};
use xxhash_rust::xxh64::Xxh64;

use crate::pipeline::{self, BufferPool, ChunkSink};

/// Which checksums to compute. xxHash64 is always on (it's what ASC MHL uses and it's
/// fast enough to never be the bottleneck); MD5 is for productions/insurers that ask.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashConfig {
    pub md5: bool,
}

/// The checksums of one file's contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hashes {
    pub xxh64: u64,
    pub md5: Option<[u8; 16]>,
}

impl Hashes {
    /// xxHash64 as 16 lowercase hex digits (big-endian), the form ASC MHL and `xxhsum` use.
    pub fn xxh64_hex(&self) -> String {
        format!("{:016x}", self.xxh64)
    }

    /// Parses the hex forms produced by [`Self::xxh64_hex`] and [`Self::md5_hex`].
    pub fn from_hex(xxh64: &str, md5: Option<&str>) -> Option<Hashes> {
        let xxh64 = u64::from_str_radix(xxh64, 16).ok()?;
        let md5 = match md5 {
            Some(m) if m.len() == 32 => {
                let mut out = [0u8; 16];
                for (i, b) in out.iter_mut().enumerate() {
                    *b = u8::from_str_radix(&m[i * 2..i * 2 + 2], 16).ok()?;
                }
                Some(out)
            }
            Some(_) => return None,
            None => None,
        };
        Some(Hashes { xxh64, md5 })
    }

    pub fn md5_hex(&self) -> Option<String> {
        self.md5.map(|d| d.iter().map(|b| format!("{b:02x}")).collect())
    }

    /// True when every checksum present on both sides agrees. xxHash64 is always present.
    pub fn matches(&self, other: &Hashes) -> bool {
        self.xxh64 == other.xxh64
            && match (self.md5, other.md5) {
                (Some(a), Some(b)) => a == b,
                _ => true,
            }
    }
}

impl fmt::Display for Hashes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "xxh64 {}", self.xxh64_hex())?;
        if let Some(md5) = self.md5_hex() {
            write!(f, " md5 {md5}")?;
        }
        Ok(())
    }
}

impl Serialize for Hashes {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Repr {
            xxh64: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            md5: Option<String>,
        }
        Repr { xxh64: self.xxh64_hex(), md5: self.md5_hex() }.serialize(s)
    }
}

/// Incremental hasher for callers that feed data themselves.
pub struct Hasher {
    xxh: Xxh64Sink,
    md5: Option<Md5Sink>,
}

impl Hasher {
    pub fn new(cfg: HashConfig) -> Self {
        Self { xxh: Xxh64Sink::new(), md5: cfg.md5.then(Md5Sink::new) }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.xxh.0.update(data);
        if let Some(m) = &mut self.md5 {
            m.0.update(data);
        }
    }

    pub fn finish(self) -> Hashes {
        Hashes { xxh64: self.xxh.finish(), md5: self.md5.map(Md5Sink::finish) }
    }
}

/// Pipeline sink computing xxHash64.
pub(crate) struct Xxh64Sink(Xxh64);

impl Xxh64Sink {
    pub fn new() -> Self {
        Self(Xxh64::new(0))
    }
    pub fn finish(&self) -> u64 {
        self.0.digest()
    }
}

impl ChunkSink for Xxh64Sink {
    fn consume(&mut self, data: &[u8]) -> io::Result<()> {
        self.0.update(data);
        Ok(())
    }
}

/// Pipeline sink computing MD5. Runs on its own thread so MD5 (the slow one) overlaps
/// with reading and with xxHash64.
pub(crate) struct Md5Sink(Md5);

impl Md5Sink {
    pub fn new() -> Self {
        Self(Md5::new())
    }
    pub fn finish(self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out.copy_from_slice(&self.0.finalize());
        out
    }
}

impl ChunkSink for Md5Sink {
    fn consume(&mut self, data: &[u8]) -> io::Result<()> {
        self.0.update(data);
        Ok(())
    }
}

/// Hashes a file. With `uncached`, reads bypass the OS cache (see
/// [`crate::platform::UncachedReader`]); the returned flag says whether that worked.
pub fn hash_file(path: &Path, cfg: HashConfig, uncached: bool) -> io::Result<(Hashes, bool)> {
    let pool = BufferPool::new();
    let never = std::sync::atomic::AtomicBool::new(false);
    let (len, mut reader, bypassed): (u64, Box<dyn io::Read>, bool) = if uncached {
        let r = crate::platform::UncachedReader::open(path)?;
        let bypassed = r.cache_bypassed();
        (r.file_len(), Box::new(r), bypassed)
    } else {
        let f = std::fs::File::open(path)?;
        (f.metadata()?.len(), Box::new(f), false)
    };
    let (hashes, _) =
        pipeline::hash_stream(&mut *reader, len, cfg, &pool, &never, &mut |_| {}).map_err(|e| match e {
            pipeline::PumpError::Read(e) => e,
            pipeline::PumpError::Cancelled => io::Error::new(io::ErrorKind::Interrupted, "cancelled"),
        })?;
    // An uncached reader may have fallen back to cached reads part-way.
    Ok((hashes, bypassed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        let mut h = Hasher::new(HashConfig { md5: true });
        h.update(b"");
        let empty = h.finish();
        assert_eq!(empty.xxh64_hex(), "ef46db3751d8e999");
        assert_eq!(empty.md5_hex().unwrap(), "d41d8cd98f00b204e9800998ecf8427e");

        let mut h = Hasher::new(HashConfig { md5: true });
        h.update(b"a");
        h.update(b"bc");
        let abc = h.finish();
        assert_eq!(abc.xxh64_hex(), "44bc2cf5ad770999");
        assert_eq!(abc.md5_hex().unwrap(), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn matches_ignores_missing_md5() {
        let a = Hashes { xxh64: 1, md5: Some([1; 16]) };
        let b = Hashes { xxh64: 1, md5: None };
        assert!(a.matches(&b));
        assert!(!a.matches(&Hashes { xxh64: 1, md5: Some([2; 16]) }));
        assert!(!a.matches(&Hashes { xxh64: 2, md5: None }));
    }
}
