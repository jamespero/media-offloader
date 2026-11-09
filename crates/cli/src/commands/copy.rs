//! `offload copy`: runs a prepared job to the end, with progress and a summary.

use std::io::{self, Write};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use offloader_core::{Job, Outcome};

use super::plan::print_plan;
use crate::progress::Progress;

pub fn run_job(job: &Job, json: bool) -> ExitCode {
    print_plan(job, false);
    eprintln!();

    let cancel = Arc::new(AtomicBool::new(false));
    let presses = Arc::new(AtomicUsize::new(0));
    {
        let (cancel, presses) = (cancel.clone(), presses.clone());
        let _ = ctrlc::set_handler(move || {
            if presses.fetch_add(1, Ordering::SeqCst) == 0 {
                cancel.store(true, Ordering::SeqCst);
                eprintln!("\nStopping… verified files are kept. Press Ctrl-C again to quit immediately.");
            } else {
                std::process::exit(130);
            }
        });
    }

    let progress = Mutex::new(Progress::new(job));
    let report = job.run(&cancel, &|e| {
        if json {
            let mut out = io::stdout().lock();
            let _ = serde_json::to_writer(&mut out, e);
            let _ = writeln!(out);
        } else {
            progress.lock().unwrap().on_event(e);
        }
    });

    eprintln!();
    for (i, d) in job.destinations().iter().enumerate() {
        eprintln!(
            "{}: {} verified, {} failed  →  {}",
            if i == 0 { "copy" } else { "backup" },
            report.verified_count(i),
            report.failed_count(i),
            d.root.display()
        );
    }
    if let Some(p) = &report.log_path {
        eprintln!("log: {}", p.display());
    }
    match &report.outcome {
        Outcome::Verified => {
            eprintln!(
                "✓ All {} files verified on {} destination(s).",
                job.plan().files.len(),
                job.destinations().len()
            );
            ExitCode::SUCCESS
        }
        Outcome::Failed { files } => {
            eprintln!("✗ {files} file(s) NOT verified. Keep the card.");
            ExitCode::from(1)
        }
        Outcome::Cancelled => {
            eprintln!("Stopped. Verified files were kept; the file in progress was removed.");
            ExitCode::from(130)
        }
        Outcome::Aborted { reason } => {
            eprintln!("✗ Stopped: {reason}.");
            ExitCode::from(3)
        }
    }
}
