//! Showing things outside the app: the file manager and the browser.

use std::path::PathBuf;

use tauri::AppHandle;

#[tauri::command]
pub fn reveal(path: PathBuf) -> Result<(), String> {
    tauri_plugin_opener::reveal_item_in_dir(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_folder(app: AppHandle, path: PathBuf) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

/// Opens the project's repository (the `repository` field in Cargo.toml) in the browser.
/// The URL is fixed here rather than passed in, so the UI can't open arbitrary links.
#[tauri::command]
pub fn open_repo(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(env!("CARGO_PKG_REPOSITORY"), None::<&str>).map_err(|e| e.to_string())
}

/// Opens a release's notes on GitHub. Only version strings are accepted, so the UI
/// can't use this to open arbitrary links.
#[tauri::command]
pub fn open_release(app: AppHandle, version: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let valid =
        !version.is_empty() && version.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'));
    if !valid {
        return Err(format!("not a version: {version}"));
    }
    let url = format!("{}/releases/tag/v{version}", env!("CARGO_PKG_REPOSITORY"));
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}
