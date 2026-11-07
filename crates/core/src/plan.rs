//! Turns a source scan into a plan: destination-relative paths for every file and
//! folder, adjusted only where a destination can't hold the original name.

use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;

use crate::scan::{EntryKind, Problem, Skipped, SourceInfo, SourceScan};
use crate::serde_util;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedFile {
    /// Path relative to the source root.
    #[serde(serialize_with = "serde_util::rel_path")]
    pub src_rel: PathBuf,
    /// Path relative to each destination root. Same as `src_rel` unless renamed.
    #[serde(serialize_with = "serde_util::rel_path")]
    pub dest_rel: PathBuf,
    pub size: u64,
    /// Last-modified time on the card (≈ capture time). Preserved on every copy.
    #[serde(serialize_with = "serde_util::opt_time_ms")]
    pub modified: Option<SystemTime>,
    #[serde(skip)]
    pub(crate) partial_rel: PathBuf,
    #[serde(skip)]
    pub(crate) accessed: Option<SystemTime>,
    #[serde(skip)]
    pub(crate) created: Option<SystemTime>,
}

/// A name that had to change to fit the destination.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Renamed {
    #[serde(serialize_with = "serde_util::rel_path")]
    pub from: PathBuf,
    #[serde(serialize_with = "serde_util::rel_path")]
    pub to: PathBuf,
    pub reason: RenameReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RenameReason {
    /// Characters or names Windows/FAT/exFAT don't allow (e.g. `:` from a Mac folder).
    IllegalName,
    /// Differs from another name only by case, and the destination ignores case.
    CaseCollision,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub source: SourceInfo,
    /// Every file to copy, in a stable order (folders first-to-last, names A→Z).
    pub files: Vec<PlannedFile>,
    /// Every folder (destination-relative), parents first, empty ones included.
    #[serde(skip)]
    pub dirs: Vec<PathBuf>,
    pub skipped: Vec<Skipped>,
    pub problems: Vec<Problem>,
    pub renamed: Vec<Renamed>,
    pub total_bytes: u64,
}

/// Scans a source and plans it with no destination yet, so the app can show its files
/// as soon as it is selected. Names follow this computer's rules; [`crate::Job::prepare`]
/// plans again against the real destinations.
pub fn preview(source: &Path, include_system_files: bool) -> Result<Plan, crate::PrepareError> {
    Ok(build(crate::scan::scan(source, include_system_files)?, cfg!(windows)))
}

pub(crate) fn build(scan: SourceScan, windows_rules: bool) -> Plan {
    let mut dir_map: HashMap<PathBuf, PathBuf> = HashMap::from([(PathBuf::new(), PathBuf::new())]);
    let mut taken: HashMap<PathBuf, HashSet<String>> = HashMap::new();
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    let mut renamed = Vec::new();

    for e in scan.entries {
        let parent_src = e.rel.parent().unwrap_or(Path::new(""));
        let parent_dest = dir_map.get(parent_src).cloned().unwrap_or_else(|| parent_src.to_path_buf());
        let name = e.rel.file_name().unwrap_or_default();

        let (mut new_name, mut reason) = match legal_name(name, windows_rules) {
            Some(n) => (n, Some(RenameReason::IllegalName)),
            None => (name.to_os_string(), None),
        };
        // Destinations are treated as case-insensitive (NTFS, exFAT, APFS by default).
        let used = taken.entry(parent_dest.clone()).or_default();
        if used.contains(&fold(&new_name)) {
            new_name = dedupe(&new_name, used);
            reason = reason.or(Some(RenameReason::CaseCollision));
        }
        used.insert(fold(&new_name));

        let dest_rel = parent_dest.join(&new_name);
        if let Some(reason) = reason {
            renamed.push(Renamed { from: e.rel.clone(), to: dest_rel.clone(), reason });
        }
        match e.kind {
            EntryKind::Dir => {
                dir_map.insert(e.rel, dest_rel.clone());
                dirs.push(dest_rel);
            }
            EntryKind::File => files.push(PlannedFile {
                src_rel: e.rel,
                dest_rel,
                size: e.size,
                modified: e.modified,
                partial_rel: PathBuf::new(),
                accessed: e.accessed,
                created: e.created,
            }),
        }
    }

    // Temp names are `<name>.partial`, unless the card itself has a file by that name.
    let all: HashSet<String> =
        files.iter().map(|f| fold_path(&f.dest_rel)).chain(dirs.iter().map(|d| fold_path(d))).collect();
    for f in &mut files {
        let mut n = 0;
        f.partial_rel = loop {
            let suffix = if n == 0 { ".partial".to_string() } else { format!(".partial{n}") };
            let mut s = f.dest_rel.as_os_str().to_os_string();
            s.push(&suffix);
            let candidate = PathBuf::from(s);
            if !all.contains(&fold_path(&candidate)) {
                break candidate;
            }
            n += 1;
        };
    }

    let total_bytes = files.iter().map(|f| f.size).sum();
    Plan { source: scan.info, files, dirs, skipped: scan.skipped, problems: scan.problems, renamed, total_bytes }
}

fn fold(name: &OsStr) -> String {
    name.to_string_lossy().to_lowercase()
}

fn fold_path(p: &Path) -> String {
    crate::format::rel_path(p).to_lowercase()
}

/// `name (2).ext`, `name (3).ext`, … — the first one not already used.
fn dedupe(name: &OsStr, used: &HashSet<String>) -> OsString {
    let s = name.to_string_lossy();
    let (stem, ext) = match s.rfind('.') {
        Some(i) if i > 0 => (&s[..i], &s[i..]),
        _ => (&s[..], ""),
    };
    (2..).map(|n| OsString::from(format!("{stem} ({n}){ext}"))).find(|c| !used.contains(&fold(c))).expect("unbounded")
}

const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
    "COM8", "COM9", "COM¹", "COM²", "COM³", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
    "LPT9", "LPT¹", "LPT²", "LPT³",
];

/// A version of `name` the destination accepts, or `None` if it's fine as is.
/// Under Windows rules: `<>:"/\|?*` and control characters become `_`, trailing dots
/// and spaces become `_`, and reserved device names get a `_` prefix.
pub(crate) fn legal_name(name: &OsStr, windows_rules: bool) -> Option<OsString> {
    if !windows_rules {
        return None;
    }
    let original = name.to_str();
    let s = name.to_string_lossy();
    let mut out: String = s
        .chars()
        .map(|c| if c < ' ' || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') { '_' } else { c })
        .collect();
    let kept = out.trim_end_matches(['.', ' ']).len();
    if kept < out.len() {
        let trailing = out.len() - kept;
        out.truncate(kept);
        out.push_str(&"_".repeat(trailing));
    }
    let stem = out.split('.').next().unwrap_or("").trim_end();
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(stem)) {
        out.insert(0, '_');
    }
    (original != Some(out.as_str())).then(|| out.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legal(s: &str) -> String {
        legal_name(OsStr::new(s), true).map(|o| o.to_string_lossy().into_owned()).unwrap_or_else(|| s.to_string())
    }

    #[test]
    fn windows_name_rules() {
        assert_eq!(legal("DSC_0001.NEF"), "DSC_0001.NEF");
        assert_eq!(legal("Scene 1: take 2?.mov"), "Scene 1_ take 2_.mov");
        assert_eq!(legal("trailing."), "trailing_");
        assert_eq!(legal("trailing. "), "trailing__");
        assert_eq!(legal("CON"), "_CON");
        assert_eq!(legal("con.txt"), "_con.txt");
        assert_eq!(legal("LPT1.log"), "_LPT1.log");
        assert_eq!(legal("CONSOLE.txt"), "CONSOLE.txt");
        assert_eq!(legal("tab\there"), "tab_here");
        assert!(legal_name(OsStr::new("a:b"), false).is_none());
    }

    fn scan_of(entries: &[(&str, EntryKind)]) -> SourceScan {
        use crate::scan::{ScanEntry, SourceKind};
        SourceScan {
            info: SourceInfo { root: PathBuf::from("card"), label: "CARD".into(), kind: SourceKind::Folder },
            entries: entries
                .iter()
                .map(|(rel, kind)| ScanEntry {
                    rel: rel.split('/').collect(),
                    kind: *kind,
                    size: 1,
                    modified: None,
                    accessed: None,
                    created: None,
                })
                .collect(),
            skipped: Vec::new(),
            problems: Vec::new(),
        }
    }

    #[test]
    fn temp_names_avoid_real_files() {
        use EntryKind::File;
        let plan = build(scan_of(&[("A.MOV", File), ("A.MOV.partial", File), ("B.MOV", File)]), false);
        let partial = |i: usize| crate::format::rel_path(&plan.files[i].partial_rel);
        assert_eq!(partial(0), "A.MOV.partial1");
        assert_eq!(partial(1), "A.MOV.partial.partial");
        assert_eq!(partial(2), "B.MOV.partial");
    }

    #[test]
    fn renames_follow_their_folder() {
        use EntryKind::{Dir, File};
        let plan = build(
            scan_of(&[("Day 1: A", Dir), ("Day 1: A/clip.mov", File), ("day 1_ a", Dir), ("day 1_ a/x.mov", File)]),
            true,
        );
        let dest: Vec<String> = plan.files.iter().map(|f| crate::format::rel_path(&f.dest_rel)).collect();
        assert_eq!(dest, ["Day 1_ A/clip.mov", "day 1_ a (2)/x.mov"]);
        assert_eq!(plan.renamed.len(), 2);
        assert_eq!(plan.renamed[1].reason, RenameReason::CaseCollision);
    }

    #[test]
    fn dedupe_finds_free_name() {
        let used: HashSet<String> = ["a.mov", "a (2).mov"].iter().map(|s| s.to_string()).collect();
        assert_eq!(dedupe(OsStr::new("A.MOV"), &used), "A (3).MOV");
        assert_eq!(dedupe(OsStr::new(".hidden"), &HashSet::new()), ".hidden (2)");
    }
}
