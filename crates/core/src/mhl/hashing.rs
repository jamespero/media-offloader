//! The hash formats MHL allows, computing them, and C4 IDs.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use md5::Md5;
use serde::Serialize;
use sha1::Sha1;
use sha2::{Digest, Sha512};
use xxhash_rust::xxh3::Xxh3;
use xxhash_rust::xxh64::Xxh64;

/// Hash formats ASC MHL allows, in the order the schema lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HashFormat {
    C4,
    Md5,
    Sha1,
    Xxh128,
    Xxh3,
    Xxh64,
}

impl HashFormat {
    pub const ALL: [HashFormat; 6] =
        [HashFormat::C4, HashFormat::Md5, HashFormat::Sha1, HashFormat::Xxh128, HashFormat::Xxh3, HashFormat::Xxh64];

    pub fn name(self) -> &'static str {
        match self {
            HashFormat::C4 => "c4",
            HashFormat::Md5 => "md5",
            HashFormat::Sha1 => "sha1",
            HashFormat::Xxh128 => "xxh128",
            HashFormat::Xxh3 => "xxh3",
            HashFormat::Xxh64 => "xxh64",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name() == s)
    }
}

/// Computes any set of MHL hash formats in one pass.
pub struct MultiHasher {
    c4: Option<Sha512>,
    md5: Option<Md5>,
    sha1: Option<Sha1>,
    xxh3: Option<Box<Xxh3>>,
    xxh64: Option<Xxh64>,
    want_xxh3: bool,
    want_xxh128: bool,
}

impl MultiHasher {
    pub fn new(formats: &[HashFormat]) -> Self {
        let has = |f| formats.contains(&f);
        Self {
            c4: has(HashFormat::C4).then(Sha512::new),
            md5: has(HashFormat::Md5).then(Md5::new),
            sha1: has(HashFormat::Sha1).then(Sha1::new),
            xxh3: (has(HashFormat::Xxh3) || has(HashFormat::Xxh128)).then(|| Box::new(Xxh3::new())),
            xxh64: has(HashFormat::Xxh64).then(|| Xxh64::new(0)),
            want_xxh3: has(HashFormat::Xxh3),
            want_xxh128: has(HashFormat::Xxh128),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        if let Some(h) = &mut self.c4 {
            h.update(data);
        }
        if let Some(h) = &mut self.md5 {
            h.update(data);
        }
        if let Some(h) = &mut self.sha1 {
            h.update(data);
        }
        if let Some(h) = &mut self.xxh3 {
            h.update(data);
        }
        if let Some(h) = &mut self.xxh64 {
            h.update(data);
        }
    }

    /// Each requested format with its value as MHL writes it.
    pub fn finish(self) -> Vec<(HashFormat, String)> {
        let mut out = Vec::new();
        if let Some(h) = self.c4 {
            out.push((HashFormat::C4, c4_id(&h.finalize())));
        }
        if let Some(h) = self.md5 {
            out.push((HashFormat::Md5, hex(&h.finalize())));
        }
        if let Some(h) = self.sha1 {
            out.push((HashFormat::Sha1, hex(&h.finalize())));
        }
        if let Some(h) = &self.xxh3 {
            if self.want_xxh128 {
                out.push((HashFormat::Xxh128, format!("{:032x}", h.digest128())));
            }
            if self.want_xxh3 {
                out.push((HashFormat::Xxh3, format!("{:016x}", h.digest())));
            }
        }
        if let Some(h) = self.xxh64 {
            out.push((HashFormat::Xxh64, format!("{:016x}", h.digest())));
        }
        out
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

const C4_ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// C4 ID of a SHA-512 digest: the digest as a base-58 number, left-padded with '1'
/// (the alphabet's zero) to 88 characters, prefixed with "c4".
pub fn c4_id(sha512: &[u8]) -> String {
    let mut num = sha512.to_vec();
    let mut digits = Vec::with_capacity(88);
    while num.iter().any(|&b| b != 0) {
        let mut rem = 0u32;
        for byte in num.iter_mut() {
            let cur = (rem << 8) | u32::from(*byte);
            *byte = (cur / 58) as u8;
            rem = cur % 58;
        }
        digits.push(C4_ALPHABET[rem as usize]);
    }
    while digits.len() < 88 {
        digits.push(b'1');
    }
    digits.reverse();
    format!("c4{}", String::from_utf8(digits).expect("ascii"))
}

pub fn c4_of_file(path: &Path) -> io::Result<String> {
    let mut f = File::open(path)?;
    let mut h = Sha512::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(c4_id(&h.finalize()))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn c4_matches_the_reference_vector() {
        // C4 ID of the empty input, from the C4 spec / reference implementation.
        assert_eq!(
            c4_id(&Sha512::digest(b"")),
            "c459dsjfscH38cYeXXYogktxf4Cd9ibshE3BHUo6a58hBXmRQdZrAkZzsWcbWtDg5oQstpDuni4Hirj75GEmTc1sFT"
        );
        assert_eq!(c4_id(&[0u8; 64]).len(), 90);
        assert!(c4_id(&[0u8; 64]).chars().skip(2).all(|c| c == '1'));
    }

    #[test]
    fn multi_hasher_formats() {
        let mut h = MultiHasher::new(&HashFormat::ALL);
        h.update(b"abc");
        let got: BTreeMap<HashFormat, String> = h.finish().into_iter().collect();
        assert_eq!(got[&HashFormat::Md5], "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(got[&HashFormat::Sha1], "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(got[&HashFormat::Xxh64], "44bc2cf5ad770999");
        assert_eq!(got[&HashFormat::Xxh3], "78af5f94892f3950");
        assert_eq!(got[&HashFormat::Xxh128], "06b05ab6733a618578af5f94892f3950");
    }
}
