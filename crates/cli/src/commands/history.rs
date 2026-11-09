//! `offload history`.

use std::process::ExitCode;
use std::time::Duration;

use offloader_core::{Outcome, format, journal};

use super::fail;

/// Lists recent offloads from their journals.
pub fn history() -> ExitCode {
    let Some(dir) = journal::default_journal_dir() else { return fail(&"no app data folder") };
    let records = journal::list(&dir);
    if records.is_empty() {
        eprintln!("No offloads yet.");
    }
    for r in records.iter().take(30) {
        let when = chrono_like(r.first_started);
        let state = match &r.outcome {
            Some(Outcome::Verified) => "verified".to_string(),
            Some(Outcome::Failed { files }) => format!("{files} failed"),
            Some(Outcome::Cancelled) => "stopped".to_string(),
            Some(Outcome::Aborted { reason }) => format!("stopped: {reason}"),
            None => "interrupted".to_string(),
        };
        println!(
            "{when}  {:<16} {:>5} files  {:>9}  {state:<12} {}",
            r.header.source_label,
            r.header.files.len(),
            format::size(r.header.total_bytes),
            r.path.display()
        );
    }
    ExitCode::SUCCESS
}

/// "YYYY-MM-DD HH:MM" in local time from ms since the epoch.
fn chrono_like(ms: u64) -> String {
    let t = std::time::UNIX_EPOCH + Duration::from_millis(ms);
    let d: chrono::DateTime<chrono::Local> = t.into();
    d.format("%Y-%m-%d %H:%M").to_string()
}
