//! Walks the source and lists everything to copy, in a stable order.

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;

use crate::error::PrepareError;
use crate::platform;
use crate::serde_util;

/// A card or drive (can be ejected, auto-selected) versus any other folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    Volume,
    Folder,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub root: PathBuf,
    /// Volume label for a card or drive ("ZR_CARD_A"), folder name otherwise. This is
    /// what the Source Name subfolder token uses.
    pub label: String,
    pub kind: SourceKind,
}

/// Something in the source that won't be copied, and why.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    #[serde(serialize_with = "serde_util::rel_path")]
    pub rel: PathBuf,
    pub reason: SkipReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SkipReason {
    /// OS housekeeping (.Spotlight-V100, System Volume Information, ._ files…).
    SystemFile,
    /// Symbolic links and junctions are not followed.
    Symlink,
    /// Sockets, devices and other non-files.
    NotARegularFile,
    /// An OS housekeeping folder we were asked to include but can't read.
    UnreadableSystemFolder,
}

/// Something in the source that should be copied but can't be read. A job with
/// problems never reports success: the card must not be treated as safe to wipe.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    #[serde(serialize_with = "serde_util::rel_path")]
    pub rel: PathBuf,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryKind {
    File,
    Dir,
}

#[derive(Clone, Debug)]
pub(crate) struct ScanEntry {
    pub rel: PathBuf,
    pub kind: EntryKind,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
}

pub(crate) struct SourceScan {
    pub info: SourceInfo,
    /// Folders come before their contents.
    pub entries: Vec<ScanEntry>,
    pub skipped: Vec<Skipped>,
    pub problems: Vec<Problem>,
}

const SYSTEM_DIRS: &[&str] = &[
    ".Trashes",
    ".Spotlight-V100",
    ".fseventsd",
    ".TemporaryItems",
    ".DocumentRevisions-V100",
    "System Volume Information",
    "$RECYCLE.BIN",
];
const SYSTEM_FILES: &[&str] = &[".DS_Store", ".apdisk", "Thumbs.db", "desktop.ini"];

/// Whether a name is OS housekeeping rather than camera data. These are skipped unless
/// "copy hidden system files" is on. Other hidden files (some cameras write dotfiles)
/// are always copied.
pub fn is_system_name(name: &OsStr, is_dir: bool) -> bool {
    let name = name.to_string_lossy();
    let list = if is_dir { SYSTEM_DIRS } else { SYSTEM_FILES };
    list.iter().any(|s| s.eq_ignore_ascii_case(&name)) || (!is_dir && name.starts_with("._"))
}

/// The OS housekeeping names as `.gitignore`-style patterns (for MHL ignore lists).
pub fn system_patterns() -> Vec<String> {
    SYSTEM_DIRS
        .iter()
        .map(|d| format!("{d}/"))
        .chain(SYSTEM_FILES.iter().map(|f| f.to_string()))
        .chain(std::iter::once("._*".to_string()))
        .collect()
}

pub(crate) fn scan(root: &Path, include_system: bool) -> Result<SourceScan, PrepareError> {
    let meta = match fs::metadata(root) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(PrepareError::SourceMissing(root.into())),
        Err(e) => return Err(PrepareError::Io { path: root.into(), source: e }),
    };
    if !meta.is_dir() {
        return Err(PrepareError::SourceNotFolder(root.into()));
    }
    let root = std::path::absolute(root).map_err(|e| PrepareError::Io { path: root.into(), source: e })?;
    let info = source_info(&root);

    let mut out = SourceScan { info, entries: Vec::new(), skipped: Vec::new(), problems: Vec::new() };
    // The top level must be readable; deeper failures are recorded as problems.
    let top = read_sorted(&root).map_err(|e| PrepareError::Io { path: root.clone(), source: e })?;
    walk(&root, Path::new(""), top, include_system, false, &mut out);
    Ok(out)
}

fn source_info(root: &Path) -> SourceInfo {
    let folder_name =
        || root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| root.display().to_string());
    match platform::volume_info(root) {
        Ok(v) if same_path(&v.mount_root, root) => {
            let label = v.label.clone().unwrap_or_else(|| {
                // Unlabeled volume: use the drive letter / mount name.
                let s = v.mount_root.to_string_lossy();
                let s = s.trim_end_matches(['\\', '/']).trim_end_matches(':');
                if s.is_empty() { "Volume".into() } else { s.rsplit(['\\', '/']).next().unwrap_or(s).into() }
            });
            SourceInfo { root: root.into(), label, kind: SourceKind::Volume }
        }
        _ => SourceInfo { root: root.into(), label: folder_name(), kind: SourceKind::Folder },
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        let s = p.to_string_lossy().trim_end_matches(['\\', '/']).to_string();
        if cfg!(windows) { s.to_lowercase() } else { s }
    };
    norm(a) == norm(b)
}

fn read_sorted(dir: &Path) -> io::Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(dir)?.collect::<io::Result<Vec<_>>>()?;
    entries.sort_by(|a, b| name_order(&a.file_name(), &b.file_name()));
    Ok(entries)
}

/// Case-insensitive, then byte order as a tiebreak: stable and natural for camera names.
fn name_order(a: &OsStr, b: &OsStr) -> Ordering {
    let (la, lb) = (a.to_string_lossy().to_lowercase(), b.to_string_lossy().to_lowercase());
    la.cmp(&lb).then_with(|| a.cmp(b))
}

fn walk(
    abs: &Path,
    rel: &Path,
    entries: Vec<fs::DirEntry>,
    include_system: bool,
    in_system: bool,
    out: &mut SourceScan,
) {
    for entry in entries {
        let name = entry.file_name();
        let rel_child = rel.join(&name);
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(e) => {
                out.problems.push(Problem { rel: rel_child, message: e.to_string() });
                continue;
            }
        };
        if ft.is_symlink() {
            out.skipped.push(Skipped { rel: rel_child, reason: SkipReason::Symlink });
            continue;
        }
        let is_dir = ft.is_dir();
        let system = in_system || is_system_name(&name, is_dir);
        if system && !include_system {
            out.skipped.push(Skipped { rel: rel_child, reason: SkipReason::SystemFile });
            continue;
        }
        if !is_dir && !ft.is_file() {
            out.skipped.push(Skipped { rel: rel_child, reason: SkipReason::NotARegularFile });
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                record_unreadable(out, rel_child, e, system);
                continue;
            }
        };
        let kind = if is_dir { EntryKind::Dir } else { EntryKind::File };
        let abs_child = abs.join(&name);
        // Read a folder's listing before recording it, so an unreadable system folder
        // can be skipped as a whole.
        let children = if is_dir {
            match read_sorted(&abs_child) {
                Ok(c) => Some(c),
                Err(e) => {
                    record_unreadable(out, rel_child, e, system);
                    continue;
                }
            }
        } else {
            None
        };
        out.entries.push(ScanEntry {
            rel: rel_child.clone(),
            kind,
            size: if is_dir { 0 } else { meta.len() },
            modified: meta.modified().ok(),
            accessed: meta.accessed().ok(),
            created: meta.created().ok(),
        });
        if let Some(children) = children {
            walk(&abs_child, &rel_child, children, include_system, system, out);
        }
    }
}

fn record_unreadable(out: &mut SourceScan, rel: PathBuf, e: io::Error, system: bool) {
    if system {
        out.skipped.push(Skipped { rel, reason: SkipReason::UnreadableSystemFolder });
    } else {
        out.problems.push(Problem { rel, message: e.to_string() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_names() {
        assert!(is_system_name(OsStr::new(".Spotlight-V100"), true));
        assert!(is_system_name(OsStr::new("system volume information"), true));
        assert!(is_system_name(OsStr::new(".DS_Store"), false));
        assert!(is_system_name(OsStr::new("._DSC_0001.NEF"), false));
        assert!(!is_system_name(OsStr::new(".DS_Store"), true));
        assert!(!is_system_name(OsStr::new(".hidden_camera_file"), false));
        assert!(!is_system_name(OsStr::new("DCIM"), true));
    }

    #[test]
    fn ordering_is_case_insensitive_with_stable_tiebreak() {
        let mut v = vec!["b.mov", "A.MOV", "a.mov", "C001.MP4"];
        v.sort_by(|a, b| name_order(OsStr::new(a), OsStr::new(b)));
        assert_eq!(v, ["A.MOV", "a.mov", "b.mov", "C001.MP4"]);
    }
}
