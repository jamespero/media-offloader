//! The commands the UI can call.

pub mod history;
pub mod links;
pub mod offload;
pub mod reverify;
pub mod sources;
pub mod system;

/// Runs blocking work (scans, prepares) off the async runtime's worker threads.
pub async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}
