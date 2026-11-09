//! Classic `.md5` lists.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::writer::write_new;
use super::{Expected, HashFormat};

/// Name of the `.md5` list written at a destination root.
pub const MD5_LIST: &str = "checksums.md5";

/// Reads a classic md5 list: `hash  path`, `hash *path`, or BSD `MD5 (path) = hash`.
pub fn read_md5_list(path: &Path) -> io::Result<BTreeMap<String, Expected>> {
    let text = fs::read_to_string(path)?;
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        let (hash, file) = if let Some(rest) = line.strip_prefix("MD5 (") {
            match rest.rsplit_once(") = ") {
                Some((f, h)) => (h.trim(), f),
                None => continue,
            }
        } else {
            match line.split_once(' ') {
                Some((h, f)) => (h, f.trim_start_matches([' ', '*'])),
                None => continue,
            }
        };
        if hash.len() == 32 && hash.chars().all(|c| c.is_ascii_hexdigit()) && !file.is_empty() {
            out.insert(
                file.replace('\\', "/"),
                Expected { size: None, hashes: vec![(HashFormat::Md5, hash.to_ascii_lowercase())] },
            );
        }
    }
    Ok(out)
}

/// Writes `checksums.md5` (md5sum format) at `root`, never replacing anything.
pub(crate) fn write_md5_list(root: &Path, entries: &[(String, String)]) -> io::Result<PathBuf> {
    let mut text = String::new();
    for (path, md5) in entries {
        text.push_str(&format!("{md5}  {path}\n"));
    }
    for n in 0..1000 {
        let name = if n == 0 { MD5_LIST.to_string() } else { format!("checksums-{}.md5", n + 1) };
        let path = root.join(name);
        match write_new(&path, text.as_bytes()) {
            Ok(()) => return Ok(path),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(io::ErrorKind::AlreadyExists, "no free name for the .md5 list"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_list_formats() {
        let dir = std::env::temp_dir().join(format!("offloader-md5-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("x.md5");
        fs::write(&f, "d41d8cd98f00b204e9800998ecf8427e  a/empty.dat\r\n900150983CD24FB0D6963F7D28E17F72 *b.txt\nMD5 (c d.txt) = 900150983cd24fb0d6963f7d28e17f72\n# junk\n").unwrap();
        let m = read_md5_list(&f).unwrap();
        assert_eq!(m.len(), 3);
        assert_eq!(m["b.txt"].hashes[0].1, "900150983cd24fb0d6963f7d28e17f72");
        assert!(m.contains_key("c d.txt") && m.contains_key("a/empty.dat"));
        let _ = fs::remove_dir_all(&dir);
    }
}
