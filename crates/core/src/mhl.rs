//! ASC MHL v2 manifests (`ascmhl/` folders) and classic `.md5` lists: written next to
//! the footage so any copy can be re-verified later, by this app or any MHL tool.
//!
//! Follows the ASC reference implementation (github.com/ascmitc/mhl): generations are
//! `ascmhl/NNNN_<folder>_<YYYY-MM-DD_HHMMSS>Z.mhl`, listed in `ascmhl/ascmhl_chain.xml`
//! with the C4 ID of each manifest file. Directory and root hashes are optional in the
//! schema and not written.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Local, Utc};
use md5::Md5;
use quick_xml::escape::escape;
use quick_xml::events::Event as Xml;
use quick_xml::reader::Reader;
use serde::Serialize;
use sha1::Sha1;
use sha2::{Digest, Sha512};
use xxhash_rust::xxh3::Xxh3;
use xxhash_rust::xxh64::Xxh64;

use crate::hash::Hashes;
use crate::{APP_NAME, VERSION};

pub const FOLDER: &str = "ascmhl";
pub const CHAIN_FILE: &str = "ascmhl_chain.xml";
/// Name of the `.md5` list written at a destination root.
pub const MD5_LIST: &str = "checksums.md5";

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

/* ------------------------------------------------------------------ reading */

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashValue {
    pub format: HashFormat,
    pub value: String,
    pub action: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// `/`-separated, relative to the history root.
    pub path: String,
    pub size: Option<u64>,
    pub hashes: Vec<HashValue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Generation {
    pub number: u32,
    pub file_name: String,
    /// Whether the manifest file still matches the C4 recorded in the chain
    /// (`None` when the chain doesn't list it).
    pub c4_ok: Option<bool>,
    pub process: Option<String>,
    pub tool: Option<String>,
    pub creation_date: Option<String>,
    pub ignore: Vec<String>,
    pub entries: Vec<Entry>,
}

/// An `ascmhl` history: every generation, oldest first.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub root: PathBuf,
    pub generations: Vec<Generation>,
    /// Damaged or missing manifests, chain problems.
    pub problems: Vec<String>,
}

/// What a file should hash to, according to a history or a `.md5` list.
#[derive(Clone, Debug)]
pub struct Expected {
    pub size: Option<u64>,
    pub hashes: Vec<(HashFormat, String)>,
}

impl History {
    /// Loads `root/ascmhl`, or `None` if there's no history there.
    pub fn load(root: &Path) -> io::Result<Option<History>> {
        let dir = root.join(FOLDER);
        if !dir.is_dir() {
            return Ok(None);
        }
        let mut problems = Vec::new();
        let chain_path = dir.join(CHAIN_FILE);
        let mut listed: Vec<(u32, String, Option<String>)> = match fs::read_to_string(&chain_path) {
            Ok(text) => parse_chain(&text).unwrap_or_else(|e| {
                problems.push(format!("{CHAIN_FILE} can't be read: {e}"));
                Vec::new()
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                problems.push(format!("{CHAIN_FILE} is missing"));
                Vec::new()
            }
            Err(e) => return Err(e),
        };
        // Manifests on disk that the chain doesn't mention still count. OS housekeeping
        // isn't a manifest: macOS leaves `._<name>.mhl` metadata files on exFAT/FAT drives.
        for e in fs::read_dir(&dir)?.flatten() {
            if crate::scan::is_system_name(&e.file_name(), false) {
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            if name.to_ascii_lowercase().ends_with(".mhl") && !listed.iter().any(|(_, n, _)| n == &name) {
                let number = name.split('_').next().and_then(|n| n.parse().ok()).unwrap_or(0);
                problems.push(format!("{name} isn't listed in {CHAIN_FILE}"));
                listed.push((number, name, None));
            }
        }
        listed.sort_by_key(|(n, _, _)| *n);

        let mut generations = Vec::new();
        for (number, file_name, chain_c4) in listed {
            let path = dir.join(&file_name);
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(e) => {
                    problems.push(format!("{file_name} can't be read: {e}"));
                    continue;
                }
            };
            let c4_ok = chain_c4.map(|c4| {
                let ok = c4_id(&Sha512::digest(&bytes)) == c4;
                if !ok {
                    problems.push(format!("{file_name} doesn't match the C4 recorded in {CHAIN_FILE}"));
                }
                ok
            });
            match parse_manifest(&String::from_utf8_lossy(&bytes)) {
                Ok(mut g) => {
                    g.number = number;
                    g.file_name = file_name;
                    g.c4_ok = c4_ok;
                    generations.push(g);
                }
                Err(e) => problems.push(format!("{file_name} can't be parsed: {e}")),
            }
        }
        Ok(Some(History { root: root.to_path_buf(), generations, problems }))
    }

    pub fn next_number(&self) -> u32 {
        self.generations.iter().map(|g| g.number).max().unwrap_or(0) + 1
    }

    /// For each path, the first recorded ("original") value of each format: later
    /// generations can't quietly replace what was recorded when the file was new.
    pub fn expected(&self) -> BTreeMap<String, Expected> {
        let mut out: BTreeMap<String, Expected> = BTreeMap::new();
        for g in &self.generations {
            for e in &g.entries {
                let exp = out.entry(e.path.clone()).or_insert(Expected { size: e.size, hashes: Vec::new() });
                for h in &e.hashes {
                    if h.action.as_deref() == Some("failed") {
                        continue;
                    }
                    if !exp.hashes.iter().any(|(f, _)| *f == h.format) {
                        // Hex formats compare case-insensitively; C4 (base 58) is case-sensitive.
                        let value =
                            if h.format == HashFormat::C4 { h.value.clone() } else { h.value.to_ascii_lowercase() };
                        exp.hashes.push((h.format, value));
                    }
                }
            }
        }
        out.retain(|_, e| !e.hashes.is_empty());
        out
    }

    pub fn ignore_patterns(&self) -> Vec<String> {
        self.generations.last().map(|g| g.ignore.clone()).unwrap_or_default()
    }
}

/// Resolves `&amp;`, `&#38;`, … in raw XML text.
fn unescape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find(';') {
            Some(j) => {
                let name = &after[..j];
                let ch = match name {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    n if n.starts_with("#x") || n.starts_with("#X") => {
                        u32::from_str_radix(&n[2..], 16).ok().and_then(char::from_u32)
                    }
                    n if n.starts_with('#') => n[1..].parse().ok().and_then(char::from_u32),
                    _ => None,
                };
                match ch {
                    Some(c) => out.push(c),
                    None => out.push_str(&rest[i..i + 2 + j]),
                }
                rest = &after[j + 1..];
            }
            None => {
                out.push_str(&rest[i..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Walks an XML document, calling `f(path_of_local_names, attributes, text)` at each
/// element end. Namespaces are ignored (MHL uses one default namespace).
fn walk_xml(text: &str, mut f: impl FnMut(&[String], &[(String, String)], &str)) -> Result<(), String> {
    let mut reader = Reader::from_str(text);
    // (element name, attributes, text) for each open element
    type Open = (String, Vec<(String, String)>, String);
    let mut stack: Vec<Open> = Vec::new();
    let attrs_of = |e: &quick_xml::events::BytesStart| -> Vec<(String, String)> {
        e.attributes()
            .flatten()
            .map(|a| {
                let key = String::from_utf8_lossy(a.key.local_name().as_ref()).into_owned();
                (key, unescape(&String::from_utf8_lossy(&a.value)))
            })
            .collect()
    };
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Xml::Start(e) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                stack.push((name, attrs_of(&e), String::new()));
            }
            Xml::Empty(e) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                let attrs = attrs_of(&e);
                let mut names: Vec<String> = stack.iter().map(|s| s.0.clone()).collect();
                names.push(name);
                f(&names, &attrs, "");
            }
            Xml::Text(t) => {
                if let Some(top) = stack.last_mut() {
                    top.2.push_str(&t.decode().map_err(|e| e.to_string())?);
                }
            }
            Xml::CData(t) => {
                if let Some(top) = stack.last_mut() {
                    top.2.push_str(&String::from_utf8_lossy(&t));
                }
            }
            Xml::GeneralRef(r) => {
                if let Some(top) = stack.last_mut() {
                    let name = r.decode().map_err(|e| e.to_string())?;
                    top.2.push_str(&unescape(&format!("&{name};")));
                }
            }
            Xml::End(_) => {
                let names: Vec<String> = stack.iter().map(|s| s.0.clone()).collect();
                if let Some((_, attrs, text)) = stack.pop() {
                    f(&names, &attrs, text.trim());
                }
            }
            Xml::Eof => break,
            _ => {}
        }
    }
    Ok(())
}

fn parse_chain(text: &str) -> Result<Vec<(u32, String, Option<String>)>, String> {
    let mut out = Vec::new();
    let mut cur: (Option<String>, Option<String>) = (None, None);
    walk_xml(text, |names, attrs, text| {
        let n: Vec<&str> = names.iter().map(String::as_str).collect();
        match n.as_slice() {
            [.., "hashlist", "path"] => cur.0 = Some(text.to_string()),
            [.., "hashlist", "c4"] => cur.1 = Some(text.to_string()),
            [.., "hashlist"] => {
                let number =
                    attrs.iter().find(|(k, _)| k == "sequencenr").and_then(|(_, v)| v.parse().ok()).unwrap_or(0);
                if let Some(path) = cur.0.take() {
                    out.push((number, path, cur.1.take()));
                }
                cur = (None, None);
            }
            _ => {}
        }
    })?;
    Ok(out)
}

fn parse_manifest(text: &str) -> Result<Generation, String> {
    let mut g = Generation {
        number: 0,
        file_name: String::new(),
        c4_ok: None,
        process: None,
        tool: None,
        creation_date: None,
        ignore: Vec::new(),
        entries: Vec::new(),
    };
    let mut cur: Option<Entry> = None;
    let mut saw_root = false;
    walk_xml(text, |names, attrs, text| {
        let n: Vec<&str> = names.iter().map(String::as_str).collect();
        match n.as_slice() {
            ["hashlist"] => saw_root = true,
            ["hashlist", "creatorinfo", "tool"] => g.tool = Some(text.to_string()),
            ["hashlist", "creatorinfo", "creationdate"] => g.creation_date = Some(text.to_string()),
            ["hashlist", "processinfo", "process"] => g.process = Some(text.to_string()),
            ["hashlist", "processinfo", "ignore", "pattern"] => g.ignore.push(text.to_string()),
            ["hashlist", "hashes", "hash", "path"] => {
                let size = attrs.iter().find(|(k, _)| k == "size").and_then(|(_, v)| v.parse().ok());
                cur = Some(Entry { path: text.to_string(), size, hashes: Vec::new() });
            }
            ["hashlist", "hashes", "hash", fmt] => {
                if let (Some(format), Some(e)) = (HashFormat::parse(fmt), cur.as_mut()) {
                    let action = attrs.iter().find(|(k, _)| k == "action").map(|(_, v)| v.clone());
                    e.hashes.push(HashValue { format, value: text.to_string(), action });
                }
            }
            ["hashlist", "hashes", "hash"] => {
                if let Some(e) = cur.take() {
                    g.entries.push(e);
                }
            }
            _ => {}
        }
    })?;
    if !saw_root {
        return Err("not an ASC MHL hashlist".into());
    }
    Ok(g)
}

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

/* ------------------------------------------------------------------ writing */

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

fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

/// Simple `.gitignore`-style matching for MHL ignore patterns: `*` and `?` wildcards,
/// a trailing `/` matches folders only, and a pattern without `/` matches a name at
/// any depth.
pub fn ignored(rel: &str, is_dir: bool, patterns: &[String]) -> bool {
    patterns.iter().any(|p| {
        let (p, dir_only) = match p.strip_suffix('/') {
            Some(s) => (s, true),
            None => (p.as_str(), false),
        };
        if dir_only && !is_dir {
            return false;
        }
        if p.contains('/') {
            wildcard(p.trim_start_matches('/'), rel)
        } else {
            rel.rsplit('/').next().is_some_and(|name| wildcard(p, name))
        }
    })
}

fn wildcard(p: &str, s: &str) -> bool {
    let (p, s): (Vec<char>, Vec<char>) = (p.chars().collect(), s.chars().collect());
    let (mut pi, mut si, mut star, mut mark) = (0, 0, None, 0);
    while si < s.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = si;
            pi += 1;
        } else if let Some(st) = star {
            pi = st + 1;
            mark += 1;
            si = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn unescapes_entities() {
        assert_eq!(unescape("a &amp; b &lt;c&gt; &#233;&#x41; &bogus;"), "a & b <c> éA &bogus;");
    }

    #[test]
    fn ignore_patterns() {
        let p: Vec<String> =
            [".DS_Store", "ascmhl", "ascmhl/", "._*", "logs/*.txt"].iter().map(|s| s.to_string()).collect();
        assert!(ignored("DCIM/.DS_Store", false, &p));
        assert!(ignored("ascmhl", true, &p));
        assert!(ignored("DCIM/._DSC_0001.NEF", false, &p));
        assert!(ignored("logs/a.txt", false, &p));
        assert!(!ignored("DCIM/DSC_0001.NEF", false, &p));
        assert!(!ignored("other/logs/a.txt", false, &p));
    }

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
