//! Reading an `ascmhl` history from disk.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use quick_xml::events::Event as Xml;
use quick_xml::reader::Reader;
use sha2::{Digest, Sha512};

use super::{CHAIN_FILE, Entry, Expected, FOLDER, Generation, HashFormat, HashValue, History, c4_id};

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
                let key = a.key.local_name().into_inner().to_string();
                (key, unescape(&a.value))
            })
            .collect()
    };
    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Xml::Start(e) => {
                let name = e.local_name().into_inner().to_string();
                stack.push((name, attrs_of(&e), String::new()));
            }
            Xml::Empty(e) => {
                let name = e.local_name().into_inner().to_string();
                let attrs = attrs_of(&e);
                let mut names: Vec<String> = stack.iter().map(|s| s.0.clone()).collect();
                names.push(name);
                f(&names, &attrs, "");
            }
            Xml::Text(t) => {
                if let Some(top) = stack.last_mut() {
                    top.2.push_str(&t.xml10_content());
                }
            }
            Xml::CData(t) => {
                if let Some(top) = stack.last_mut() {
                    top.2.push_str(&t);
                }
            }
            Xml::GeneralRef(r) => {
                if let Some(top) = stack.last_mut() {
                    top.2.push_str(&unescape(&format!("&{};", &*r)));
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

pub(super) fn parse_chain(text: &str) -> Result<Vec<(u32, String, Option<String>)>, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescapes_entities() {
        assert_eq!(unescape("a &amp; b &lt;c&gt; &#233;&#x41; &bogus;"), "a & b <c> éA &bogus;");
    }
}

#[cfg(test)]
mod xml_tests {
    use super::walk_xml;

    /// (element path, attributes, text)
    type Element = (String, Vec<(String, String)>, String);

    fn walk(xml: &str) -> Vec<Element> {
        let mut out = Vec::new();
        walk_xml(xml, |names, attrs, text| out.push((names.join("/"), attrs.to_vec(), text.to_string()))).unwrap();
        out
    }

    fn text_of(xml: &str, element: &str) -> String {
        walk(xml).into_iter().find(|(n, _, _)| n == element).map(|(_, _, t)| t).unwrap()
    }

    #[test]
    fn entities_in_text_are_decoded() {
        let xml = "<r><path>A &amp; B &lt;1&gt; &quot;x&quot; &apos;y&apos;.mov</path></r>";
        assert_eq!(text_of(xml, "r/path"), r#"A & B <1> "x" 'y'.mov"#);
    }

    #[test]
    fn numeric_character_references_in_text_are_decoded() {
        assert_eq!(text_of("<r><path>caf&#233;_&#xE9;.mov</path></r>", "r/path"), "café_é.mov");
    }

    #[test]
    fn non_ascii_text_passes_through() {
        assert_eq!(text_of("<r><path>日本語/café 🎬.mov</path></r>", "r/path"), "日本語/café 🎬.mov");
    }

    #[test]
    fn cdata_is_taken_literally() {
        assert_eq!(text_of("<r><path><![CDATA[a &amp; <b>.mov]]></path></r>", "r/path"), "a &amp; <b>.mov");
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_but_inner_spaces_are_kept() {
        assert_eq!(
            text_of(
                "<r><path>
   A  B.mov 
 </path></r>",
                "r/path"
            ),
            "A  B.mov"
        );
    }

    #[test]
    fn attributes_are_read_and_their_entities_decoded() {
        let found = walk(r#"<r><path size="12" note="a &amp; b &#233;">x</path></r>"#);
        let (_, attrs, _) = found.iter().find(|(n, _, _)| n == "r/path").unwrap();
        assert_eq!(attrs, &vec![("size".to_string(), "12".to_string()), ("note".to_string(), "a & b é".to_string())]);
    }

    #[test]
    fn namespace_prefixes_are_ignored() {
        let found = walk(r#"<m:r xmlns:m="urn:x"><m:path m:size="3">x</m:path></m:r>"#);
        let (_, attrs, text) = found.iter().find(|(n, _, _)| n == "r/path").unwrap();
        assert_eq!(text, "x");
        assert!(attrs.iter().any(|(k, v)| k == "size" && v == "3"));
    }

    #[test]
    fn empty_elements_are_reported_with_their_attributes() {
        let found = walk(r#"<r><hash algo="xx"/></r>"#);
        assert_eq!(found[0], ("r/hash".to_string(), vec![("algo".to_string(), "xx".to_string())], String::new()));
    }

    #[test]
    fn mismatched_tags_are_an_error() {
        assert!(walk_xml("<r><a></b></r>", |_, _, _| {}).is_err());
    }

    #[test]
    fn line_endings_inside_text_are_normalized() {
        assert_eq!(text_of("<r><path>a\r\nb</path></r>", "r/path"), "a\nb");
    }
}
