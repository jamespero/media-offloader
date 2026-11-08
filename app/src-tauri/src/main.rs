// Release builds on Windows shouldn't open a console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Tauri shell around `offloader-core`. The engine runs on a worker thread and its
//! events are forwarded to the window as `engine` events (progress throttled), with the
//! final report as `engine-report`.

mod thumbs;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use offloader_core::{
    CopyMode, Destination, Event, HashConfig, Job, JobSpec, Plan, Sidecars, journal, log, plan, platform, reverify,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

/// The job waiting to start (after `prepare`) and the cancel flag of the one running.
#[derive(Default)]
struct Engine {
    prepared: Mutex<Option<Job>>,
    running: Mutex<Option<Arc<AtomicBool>>>,
    reverifying: Mutex<Option<Arc<AtomicBool>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OffloadArgs {
    source: PathBuf,
    copy_to: PathBuf,
    backup_to: Option<PathBuf>,
    md5: bool,
    fan_out: bool,
    include_system_files: bool,
    mhl: bool,
    md5_list: bool,
    keep_awake: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Prepared<'a> {
    plan: &'a Plan,
    destinations: &'a [Destination],
    journal: Option<&'a std::path::Path>,
}

/// Keeps a prepared job for `start` and returns what the UI shows before Start.
fn keep_prepared(engine: &Engine, job: Job) -> Result<serde_json::Value, String> {
    let view = serde_json::to_value(Prepared {
        plan: job.plan(),
        destinations: job.destinations(),
        journal: job.spec().journal.as_deref(),
    })
    .map_err(|e| e.to_string())?;
    *engine.prepared.lock().unwrap() = Some(job);
    Ok(view)
}

/// A past offload, for the history tabs.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HistoryItem {
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
struct HistoryDetail {
    item: HistoryItem,
    files: Vec<journal::JournalFile>,
    /// Per destination: the paths verified there.
    verified: Vec<Vec<String>>,
    /// Per destination: path → reason.
    failed: Vec<std::collections::HashMap<String, String>>,
}

#[tauri::command]
async fn history_list(limit: usize) -> Result<Vec<HistoryItem>, String> {
    blocking(move || {
        let Some(dir) = journal::default_journal_dir() else { return Ok(Vec::new()) };
        Ok(journal::list(&dir).iter().take(limit).map(HistoryItem::from).collect())
    })
    .await
}

#[tauri::command]
async fn history_detail(path: PathBuf) -> Result<HistoryDetail, String> {
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
fn forget_history(path: PathBuf) -> Result<(), String> {
    let dir = journal::default_journal_dir().ok_or("no history folder")?;
    let inside = path.parent().is_some_and(|p| p == dir) && path.extension().is_some_and(|x| x == "jsonl");
    if !inside {
        return Err("not a history entry".into());
    }
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

/// Prepares to continue an earlier offload (or retry its failed files) in place.
#[tauri::command]
async fn prepare_resume(engine: State<'_, Engine>, journal_path: PathBuf) -> Result<serde_json::Value, String> {
    if engine.running.lock().unwrap().is_some() {
        return Err("An offload is already running.".into());
    }
    let job = blocking(move || {
        let r = journal::Record::read(&journal_path).map_err(|e| e.to_string())?;
        Job::prepare(r.resume_spec(log::default_log_dir())).map_err(|e| e.to_string())
    })
    .await?;
    keep_prepared(&engine, job)
}

/// Runs blocking work (scans, prepares) off the async runtime's worker threads.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn list_sources() -> Result<Vec<platform::VolumeEntry>, String> {
    blocking(|| Ok(platform::list_volumes())).await
}

#[tauri::command]
async fn preview_source(path: PathBuf, include_system_files: bool) -> Result<Plan, String> {
    blocking(move || plan::preview(&path, include_system_files).map_err(|e| e.to_string())).await
}

/// Plans the offload (resolving `copy-conflicted`, checking space) without writing.
/// The job is kept so `start` runs exactly what was previewed.
#[tauri::command]
async fn prepare(engine: State<'_, Engine>, args: OffloadArgs) -> Result<serde_json::Value, String> {
    if engine.running.lock().unwrap().is_some() {
        return Err("An offload is already running.".into());
    }
    let spec = JobSpec {
        source: args.source,
        copy_to: args.copy_to,
        backups: args.backup_to.into_iter().collect(),
        mode: if args.fan_out { CopyMode::FanOut } else { CopyMode::Sequential },
        hash: HashConfig { md5: args.md5 },
        include_system_files: args.include_system_files,
        log_dir: log::default_log_dir(),
        sidecars: Sidecars { mhl: args.mhl, md5_list: args.md5_list },
        resume: None,
        journal: None,
        keep_awake: args.keep_awake,
        faults: None,
    };
    let mut job = blocking(move || Job::prepare(spec).map_err(|e| e.to_string())).await?;
    let label = job.plan().source.label.clone();
    job.set_journal(journal::default_journal_dir().map(|d| journal::new_journal_path(&d, &label)));
    keep_prepared(&engine, job)
}

#[tauri::command]
fn start(app: AppHandle, engine: State<'_, Engine>) -> Result<(), String> {
    let mut running = engine.running.lock().unwrap();
    if running.is_some() {
        return Err("An offload is already running.".into());
    }
    let job = engine.prepared.lock().unwrap().take().ok_or("Nothing to start.")?;
    let cancel = Arc::new(AtomicBool::new(false));
    *running = Some(cancel.clone());

    std::thread::spawn(move || {
        let t0 = Instant::now();
        let last_progress = AtomicU64::new(0);
        let report = job.run(&cancel, &|e| {
            // ~30 progress updates a second is plenty for the panels and thumbnails.
            if matches!(e, Event::Progress { .. }) {
                let now = t0.elapsed().as_millis() as u64;
                let last = last_progress.load(Ordering::Relaxed);
                if now < last + 33
                    || last_progress.compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed).is_err()
                {
                    return;
                }
            }
            let _ = app.emit("engine", e);
        });
        if let Some(engine) = app.try_state::<Engine>() {
            *engine.running.lock().unwrap() = None;
        }
        let _ = app.emit("engine-report", &report);
    });
    Ok(())
}

#[tauri::command]
fn cancel(engine: State<'_, Engine>) {
    if let Some(flag) = engine.running.lock().unwrap().as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
}

#[tauri::command]
fn is_running(engine: State<'_, Engine>) -> bool {
    engine.running.lock().unwrap().is_some()
}

/// Re-checks a folder against its MHL history (or .md5 list) on a worker thread.
/// Emits `reverify` events and finally `reverify-report` (or `reverify-error`).
#[tauri::command]
fn reverify_folder(app: AppHandle, engine: State<'_, Engine>, path: PathBuf) -> Result<(), String> {
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
fn cancel_reverify(engine: State<'_, Engine>) {
    if let Some(flag) = engine.reverifying.lock().unwrap().as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
}

#[tauri::command]
async fn read_text(path: PathBuf) -> Result<String, String> {
    blocking(move || std::fs::read_to_string(&path).map_err(|e| e.to_string())).await
}

/// Queues thumbnails for these files, replacing any earlier request. Results arrive as
/// `thumb` events tagged with `generation`.
#[tauri::command]
fn thumbs_request(thumbs: State<'_, thumbs::Thumbs>, generation: u64, items: Vec<thumbs::ThumbRequest>) {
    thumbs.request(generation, items);
}

/// Makes a card or drive safe to pull.
#[tauri::command]
async fn eject(path: PathBuf) -> Result<(), String> {
    blocking(move || platform::eject(&path).map_err(|e| e.to_string())).await
}

#[tauri::command]
fn log_dir() -> Option<PathBuf> {
    log::default_log_dir()
}

#[tauri::command]
fn reveal(path: PathBuf) -> Result<(), String> {
    tauri_plugin_opener::reveal_item_in_dir(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_folder(app: AppHandle, path: PathBuf) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

/// Opens the project's repository (the `repository` field in Cargo.toml) in the browser.
/// The URL is fixed here rather than passed in, so the UI can't open arbitrary links.
#[tauri::command]
fn open_repo(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(env!("CARGO_PKG_REPOSITORY"), None::<&str>).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(Engine::default())
        .setup(|app| {
            let cache = app.path().app_cache_dir()?.join("thumbs");
            app.manage(thumbs::Thumbs::start(app.handle().clone(), cache));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_sources,
            preview_source,
            prepare,
            start,
            cancel,
            is_running,
            reverify_folder,
            history_list,
            history_detail,
            forget_history,
            prepare_resume,
            cancel_reverify,
            read_text,
            log_dir,
            eject,
            thumbs_request,
            reveal,
            open_folder,
            open_repo
        ])
        .run(tauri::generate_context!())
        .expect("error while running Media Offloader");
}
