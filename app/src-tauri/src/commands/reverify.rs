//! Re-checking a folder against its recorded checksums.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use offloader_core::{log, reverify};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::state::Engine;

/// Re-checks a folder against its MHL history (or .md5 list) on a worker thread.
/// Emits `reverify` events and finally `reverify-report` (or `reverify-error`).
#[tauri::command]
pub fn reverify_folder(app: AppHandle, engine: State<'_, Engine>, path: PathBuf) -> Result<(), String> {
    let mut slot = engine.reverifying.lock().unwrap();
    if slot.is_some() {
        return Err("A re-verify is already running.".into());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    *slot = Some(cancel.clone());
    std::thread::spawn(move || {
        let t0 = Instant::now();
        let last = AtomicU64::new(0);
        let result = reverify::reverify(&path, log::default_log_dir().as_deref(), &cancel, &|e| {
            if matches!(e, reverify::ReverifyEvent::Progress { .. }) {
                let now = t0.elapsed().as_millis() as u64;
                let prev = last.load(Ordering::Relaxed);
                if now < prev + 50 || last.compare_exchange(prev, now, Ordering::Relaxed, Ordering::Relaxed).is_err() {
                    return;
                }
            }
            let _ = app.emit("reverify", e);
        });
        if let Some(engine) = app.try_state::<Engine>() {
            *engine.reverifying.lock().unwrap() = None;
        }
        match result {
            Ok(report) => {
                let _ = app.emit("reverify-report", &report);
            }
            Err(e) => {
                let _ = app.emit("reverify-error", e.to_string());
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub fn cancel_reverify(engine: State<'_, Engine>) {
    if let Some(flag) = engine.reverifying.lock().unwrap().as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
}
