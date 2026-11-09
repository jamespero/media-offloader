//! Small requests: reading a log, the log folder, thumbnails.

use std::path::PathBuf;

use offloader_core::log;
use tauri::State;

use super::blocking;
use crate::thumbs;

#[tauri::command]
pub async fn read_text(path: PathBuf) -> Result<String, String> {
    blocking(move || std::fs::read_to_string(&path).map_err(|e| e.to_string())).await
}

#[tauri::command]
pub fn log_dir() -> Option<PathBuf> {
    log::default_log_dir()
}

/// Queues thumbnails for these files, replacing any earlier request. Results arrive as
/// `thumb` events tagged with `generation`.
#[tauri::command]
pub fn thumbs_request(thumbs: State<'_, thumbs::Thumbs>, generation: u64, items: Vec<thumbs::ThumbRequest>) {
    thumbs.request(generation, items);
}
