//! Past offloads, read from their journals.

use std::path::PathBuf;

use offloader_core::journal;
use serde::Serialize;

use super::blocking;

/// A past offload, for the history tabs.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    path: PathBuf,
    started: u64,
    finished: Option<u64>,
    label: String,
    source: PathBuf,
    source_kind: offloader_core::SourceKind,
    source_present: bool,
    roots: Vec<PathBuf>,
    files: usize,
    bytes: u64,
    verified: Vec<usize>,
    failed: Vec<usize>,
    released: bool,
    /// `None`: the app stopped before the offload finished (crash, power loss, quit).
    outcome: Option<offloader_core::Outcome>,
    resumable: bool,
    log: Option<PathBuf>,
    sidecars: Vec<PathBuf>,
    runs: usize,
}

impl HistoryItem {
    fn from(r: &journal::Record) -> Self {
        let h = &r.header;
        Self {
            path: r.path.clone(),
            started: r.first_started,
            finished: r.finished,
            label: h.source_label.clone(),
            source: h.source.clone(),
            source_kind: h.source_kind,
            source_present: h.source.is_dir(),
            roots: h.roots.clone(),
            files: h.files.len(),
            bytes: h.total_bytes,
            verified: (0..h.roots.len()).map(|d| r.verified_count(d)).collect(),
            failed: r.failed.iter().map(|m| m.len()).collect(),
            released: r.released,
            outcome: r.outcome.clone(),
            resumable: r.resumable(),
            log: h.log.clone(),
            sidecars: r.sidecars.clone(),
            runs: r.runs,
        }
    }
}

/// One past offload in full: its files and what happened to each on each drive.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDetail {
    item: HistoryItem,
    files: Vec<journal::JournalFile>,
    /// Per destination: the paths verified there.
    verified: Vec<Vec<String>>,
    /// Per destination: path → reason.
    failed: Vec<std::collections::HashMap<String, String>>,
}

#[tauri::command]
pub async fn history_list(limit: usize) -> Result<Vec<HistoryItem>, String> {
    blocking(move || {
        let Some(dir) = journal::default_journal_dir() else { return Ok(Vec::new()) };
        Ok(journal::list(&dir).iter().take(limit).map(HistoryItem::from).collect())
    })
    .await
}

#[tauri::command]
pub async fn history_detail(path: PathBuf) -> Result<HistoryDetail, String> {
    blocking(move || {
        let r = journal::Record::read(&path).map_err(|e| e.to_string())?;
        Ok(HistoryDetail {
            item: HistoryItem::from(&r),
            files: r.header.files.clone(),
            verified: r.verified_paths(),
            failed: r.failed.clone(),
        })
    })
    .await
}

/// Removes a past offload from the history (its journal only; logs and footage stay).
#[tauri::command]
pub fn forget_history(path: PathBuf) -> Result<(), String> {
    let dir = journal::default_journal_dir().ok_or("no history folder")?;
    let inside = path.parent().is_some_and(|p| p == dir) && path.extension().is_some_and(|x| x == "jsonl");
    if !inside {
        return Err("not a history entry".into());
    }
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}
