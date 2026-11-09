//! `offload resume`.

use std::process::ExitCode;

use offloader_core::{Job, journal, log};

use super::copy::run_job;
use super::fail;

/// Continues an interrupted offload (or retries its failed files) from its journal.
pub fn resume(path: &std::path::Path, json: bool) -> ExitCode {
    let record = match journal::Record::read(path) {
        Ok(r) => r,
        Err(e) => return fail(&format!("{}: {e}", path.display())),
    };
    if !record.resumable() {
        eprintln!("That offload already finished with every file verified; nothing to resume.");
        return ExitCode::SUCCESS;
    }
    let done = record.verified_count(0);
    eprintln!(
        "Resuming {}: {done} of {} files already verified on the copy.",
        record.header.source_label,
        record.header.files.len()
    );
    match Job::prepare(record.resume_spec(log::default_log_dir())) {
        Ok(job) => run_job(&job, json),
        Err(e) => fail(&e),
    }
}
