//! Preparing, starting and stopping an offload. The engine runs on a worker thread and
//! its events are forwarded to the window as `engine` events (progress throttled), with
//! the final report as `engine-report`.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use offloader_core::{CopyMode, Destination, Event, HashConfig, Job, JobSpec, Plan, Sidecars, journal, log};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use super::blocking;
use crate::state::Engine;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OffloadArgs {
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

/// Prepares to continue an earlier offload (or retry its failed files) in place.
#[tauri::command]
pub async fn prepare_resume(engine: State<'_, Engine>, journal_path: PathBuf) -> Result<serde_json::Value, String> {
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

/// Plans the offload (resolving `copy-conflicted`, checking space) without writing.
/// The job is kept so `start` runs exactly what was previewed.
#[tauri::command]
pub async fn prepare(engine: State<'_, Engine>, args: OffloadArgs) -> Result<serde_json::Value, String> {
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
pub fn start(app: AppHandle, engine: State<'_, Engine>) -> Result<(), String> {
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
pub fn cancel(engine: State<'_, Engine>) {
    if let Some(flag) = engine.running.lock().unwrap().as_ref() {
        flag.store(true, Ordering::SeqCst);
    }
}

#[tauri::command]
pub fn is_running(engine: State<'_, Engine>) -> bool {
    engine.running.lock().unwrap().is_some()
}
