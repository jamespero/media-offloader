//! The state of a job while it runs.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::SystemTime;

use super::resume::Pre;
use super::{FileReport, Job};
use crate::event::{Event, FileStatus};
use crate::faults::Faults;
use crate::format;
use crate::journal::JournalWriter;
use crate::log::TransferLog;
use crate::pipeline::BufferPool;

pub(super) enum PassEnd {
    Done,
    Cancelled,
    Aborted(String),
}

pub(super) struct Runner<'a> {
    pub(super) job: &'a Job,
    pub(super) cancel: &'a AtomicBool,
    pub(super) emit: &'a (dyn Fn(&Event) + Sync),
    pub(super) faults: Option<&'a dyn Faults>,
    pub(super) log: Option<TransferLog>,
    pub(super) files: Vec<FileReport>,
    pub(super) warnings: Vec<String>,
    pub(super) copy_pool: BufferPool,
    pub(super) verify_pools: Vec<BufferPool>,
    pub(super) cache_warned: bool,
    /// When each file was verified on each destination (for MHL hash dates).
    pub(super) stamps: Vec<Vec<Option<SystemTime>>>,
    pub(super) sidecars: Vec<PathBuf>,
    pub(super) journal: Option<JournalWriter>,
}

impl Runner<'_> {
    /// Every file is verified off the source: tell the UI, and remember it.
    pub(super) fn release(&mut self) {
        if let Some(j) = self.journal.as_mut() {
            j.released();
        }
        (self.emit)(&Event::SourceReleased);
    }

    /// Whether an earlier run had already verified file `i` on destination `d`.
    pub(super) fn pre_verified(&self, i: usize, d: usize) -> bool {
        matches!(self.job.pre.get(i).map(|row| &row[d]), Some(Pre::Done(..)))
    }

    /// Records a new status in the report and the journal.
    pub(super) fn set_status(&mut self, i: usize, d: usize, status: &FileStatus) {
        let f = &self.job.plan.files[i];
        if let Some(j) = self.journal.as_mut() {
            let rel = format::rel_path(&f.dest_rel);
            match status {
                FileStatus::Verified => {
                    if let Some(h) = &self.files[i].hashes {
                        j.verified(d, rel, f.size, f.modified, h, SystemTime::now());
                    }
                }
                FileStatus::Failed { failure } => j.failed(d, rel, failure.to_string()),
                FileStatus::Pending => {}
            }
        }
        if status.is_verified() {
            self.stamps[i][d] = Some(SystemTime::now());
        }
        self.files[i].dests[d] = status.clone();
    }
    pub(super) fn warn(&mut self, message: String) {
        self.note(&message);
        (self.emit)(&Event::Warning { message: message.clone() });
        self.warnings.push(message);
    }

    pub(super) fn note(&mut self, s: &str) {
        if let Some(l) = self.log.as_mut() {
            l.line(s);
        }
    }

    pub(super) fn all_verified(&self, dests: &[usize]) -> bool {
        self.files.iter().all(|f| dests.iter().all(|&d| f.dests[d].is_verified()))
    }
}
