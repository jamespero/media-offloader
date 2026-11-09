//! Adding a generation to an `ascmhl` history.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Local, Utc};
use quick_xml::escape::escape;
use sha2::{Digest, Sha512};

use super::reader::parse_chain;
use super::{CHAIN_FILE, FOLDER, HashFormat, History, c4_id};
use crate::hash::Hashes;
use crate::{APP_NAME, VERSION};

/// One verified file for a new generation.
pub(crate) struct NewEntry<'a> {
    pub path: String,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub hashes: &'a Hashes,
    pub hashed_at: SystemTime,
    /// Verified by an earlier run of a resumed offload (not re-hashed in this one).
    pub from_earlier_run: bool,
}

fn iso(t: SystemTime, micros: bool) -> String {
    let d: DateTime<Local> = t.into();
    if micros { d.format("%Y-%m-%dT%H:%M:%S%.6f%:z").to_string() } else { d.format("%Y-%m-%dT%H:%M:%S%:z").to_string() }
}

pub fn hostname() -> String {
    #[cfg(windows)]
    {
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".into())
    }
    #[cfg(unix)]
    {
        let mut buf = [0u8; 256];
        // SAFETY: buf is valid for its length; gethostname NUL-terminates on success.
        if unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) } == 0 {
            let n = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            return String::from_utf8_lossy(&buf[..n]).into_owned();
        }
        "unknown".into()
    }
}

/// Adds a generation to `root/ascmhl` (creating the history if needed) and returns the
/// manifest's path, or `None` if there was nothing new to record (a resumed run whose
/// earlier files are already in the history). The manifest is a new file; the chain
/// index is rewritten as a superset of itself (write-to-temp, then replace). That index
/// is the only file this app ever replaces, and only inside `ascmhl/`.
pub(crate) fn write_generation(root: &Path, entries: &[NewEntry], ignore: &[String]) -> io::Result<Option<PathBuf>> {
    let dir = root.join(FOLDER);
    let existing = History::load(root)?;
    let previous = existing.as_ref().map(History::expected).unwrap_or_default();
    let entries: Vec<&NewEntry> =
        entries.iter().filter(|e| !(e.from_earlier_run && previous.contains_key(&e.path))).collect();
    if entries.is_empty() {
        return Ok(None);
    }
    fs::create_dir_all(&dir)?;
    let chain_path = dir.join(CHAIN_FILE);
    let mut chain = match fs::read_to_string(&chain_path) {
        Ok(text) => Some(parse_chain(&text).map_err(io::Error::other)?),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    let number = existing.as_ref().map_or(1, History::next_number);

    let now = SystemTime::now();
    let folder = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "root".into());
    let stamp = DateTime::<Utc>::from(now).format("%Y-%m-%d_%H%M%SZ");
    let file_name = format!("{number:04}_{folder}_{stamp}.mhl");

    let mut x = String::new();
    x.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<hashlist version=\"2.0\" xmlns=\"urn:ASC:MHL:v2.0\">\n");
    x.push_str("  <creatorinfo>\n");
    x.push_str(&format!("    <creationdate>{}</creationdate>\n", iso(now, false)));
    x.push_str(&format!("    <hostname>{}</hostname>\n", escape(hostname())));
    x.push_str(&format!("    <tool version=\"{}\">{}</tool>\n", escape(VERSION), escape(APP_NAME)));
    x.push_str("  </creatorinfo>\n  <processinfo>\n    <process>transfer</process>\n    <ignore>\n");
    for p in ignore {
        x.push_str(&format!("      <pattern>{}</pattern>\n", escape(p.as_str())));
    }
    x.push_str("    </ignore>\n  </processinfo>\n  <hashes>\n");
    for e in entries {
        x.push_str("    <hash>\n");
        let modified = e.modified.map(|m| format!(" lastmodificationdate=\"{}\"", iso(m, false))).unwrap_or_default();
        x.push_str(&format!("      <path size=\"{}\"{modified}>{}</path>\n", e.size, escape(e.path.as_str())));
        let mut values: Vec<(HashFormat, String)> = vec![(HashFormat::Xxh64, e.hashes.xxh64_hex())];
        if let Some(m) = e.hashes.md5_hex() {
            values.push((HashFormat::Md5, m));
        }
        values.sort_by_key(|(f, _)| *f);
        let hashdate = iso(e.hashed_at, true);
        for (format, value) in values {
            // "original" the first time a file is recorded; checked against the history otherwise.
            let action = match previous.get(&e.path).and_then(|p| p.hashes.iter().find(|(f, _)| *f == format)) {
                Some((_, v)) if *v == value => "verified",
                Some(_) => "failed",
                None => "original",
            };
            let tag = format.name();
            x.push_str(&format!("      <{tag} action=\"{action}\" hashdate=\"{hashdate}\">{value}</{tag}>\n"));
        }
        x.push_str("    </hash>\n");
    }
    x.push_str("  </hashes>\n</hashlist>\n");

    let manifest = dir.join(&file_name);
    write_new(&manifest, x.as_bytes())?;
    let c4 = c4_id(&Sha512::digest(x.as_bytes()));

    let mut generations = chain.take().unwrap_or_default();
    generations.push((number, file_name, Some(c4)));
    let mut c = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<ascmhldirectory xmlns=\"urn:ASC:MHL:DIRECTORY:v2.0\">\n",
    );
    for (n, name, c4) in &generations {
        c.push_str(&format!("  <hashlist sequencenr=\"{n}\">\n    <path>{}</path>\n", escape(name.as_str())));
        if let Some(c4) = c4 {
            c.push_str(&format!("    <c4>{c4}</c4>\n"));
        }
        c.push_str("  </hashlist>\n");
    }
    c.push_str("</ascmhldirectory>\n");
    let tmp = dir.join(format!("{CHAIN_FILE}.partial"));
    let _ = fs::remove_file(&tmp);
    write_new(&tmp, c.as_bytes())?;
    fs::rename(&tmp, &chain_path)?;
    let _ = crate::platform::sync_dir(&dir);
    Ok(Some(manifest))
}

pub(super) fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}
