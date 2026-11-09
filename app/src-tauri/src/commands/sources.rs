//! Finding sources, reading them, and making them safe to remove.

use std::path::PathBuf;

use offloader_core::{Plan, plan, platform};

use super::blocking;

#[tauri::command]
pub async fn list_sources() -> Result<Vec<platform::VolumeEntry>, String> {
    blocking(|| Ok(platform::list_volumes())).await
}

#[tauri::command]
pub async fn preview_source(path: PathBuf, include_system_files: bool) -> Result<Plan, String> {
    blocking(move || plan::preview(&path, include_system_files).map_err(|e| e.to_string())).await
}

/// Makes a card or drive safe to pull.
#[tauri::command]
pub async fn eject(path: PathBuf) -> Result<(), String> {
    blocking(move || platform::eject(&path).map_err(|e| e.to_string())).await
}
