//! `offload verify`.

use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use offloader_core::{format, reverify};

use super::fail;

pub fn verify(folder: &std::path::Path, log_dir: Option<PathBuf>) -> ExitCode {
    use reverify::{CheckStatus, ManifestSource, ReverifyEvent, ReverifyOutcome};
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let cancel = cancel.clone();
        let _ = ctrlc::set_handler(move || cancel.store(true, Ordering::SeqCst));
    }
    let tty = io::stderr().is_terminal();
    let last = Mutex::new(Instant::now());
    let result = reverify::reverify(folder, log_dir.as_deref(), &cancel, &|e| match e {
        ReverifyEvent::Started { source, files, bytes } => {
            let by = match source {
                ManifestSource::Mhl { generations } => format!("ASC MHL history ({generations} generation(s))"),
                ManifestSource::Md5List { file } => file.clone(),
            };
            eprintln!("Checking {files} files ({}) against {by}", format::size(*bytes));
        }
        ReverifyEvent::Progress { done, total, .. } if tty => {
            let mut l = last.lock().unwrap();
            if l.elapsed() >= Duration::from_millis(150) {
                *l = Instant::now();
                eprint!("\r{:5.1}%", *done as f64 * 100.0 / (*total).max(1) as f64);
            }
        }
        _ => {}
    });
    if tty {
        eprint!("\r      \r");
    }
    let report = match result {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    for p in &report.manifest_problems {
        eprintln!("manifest: {p}");
    }
    for f in report.files.iter().filter(|f| !f.status.is_ok()) {
        let why = match &f.status {
            CheckStatus::Missing => "missing".to_string(),
            CheckStatus::SizeDiffers { expected, actual } => format!("size {actual}, expected {expected}"),
            CheckStatus::Mismatch { format, expected, actual } => {
                format!("{} {actual}, expected {expected}", format.name())
            }
            CheckStatus::ReadError { message } => format!("can't read: {message}"),
            CheckStatus::Pending => "not checked".to_string(),
            CheckStatus::Verified => unreachable!(),
        };
        eprintln!("FAIL  {}  {why}", f.path);
    }
    for p in &report.not_listed {
        eprintln!("new   {p}  (not in any manifest)");
    }
    if let Some(p) = &report.log_path {
        eprintln!("log: {}", p.display());
    }
    match report.outcome {
        ReverifyOutcome::Verified => {
            eprintln!("✓ All {} files match their recorded checksums.", report.files.len());
            ExitCode::SUCCESS
        }
        ReverifyOutcome::Failed { files } => {
            eprintln!("✗ {files} file(s) do NOT match.");
            ExitCode::from(1)
        }
        ReverifyOutcome::Cancelled => ExitCode::from(130),
    }
}
