//! Running a prepared job from start to finish.

use std::sync::atomic::AtomicBool;
use std::time::Instant;

use chrono::Local;

use super::resume::Pre;
use super::runner::{PassEnd, Runner};
use super::{CopyMode, Job, JobReport};
use crate::event::{Event, FileStatus, Outcome, Pass};
use crate::journal::JournalWriter;
use crate::log::TransferLog;
use crate::pipeline::BufferPool;
use crate::platform;

impl Job {
    /// Runs the offload. Blocks until done; `cancel` stops it between chunks.
    /// `on_event` is called from engine threads and must be cheap.
    pub fn run(&self, cancel: &AtomicBool, on_event: &(dyn Fn(&Event) + Sync)) -> JobReport {
        let _awake = self.spec.keep_awake.then(platform::KeepAwake::new);
        let started = Local::now();
        let t0 = Instant::now();
        let mut r = Runner {
            job: self,
            cancel,
            emit: on_event,
            faults: self.spec.faults.as_deref(),
            log: None,
            files: (0..self.plan.files.len()).map(|i| self.initial_report(i)).collect(),
            warnings: Vec::new(),
            copy_pool: BufferPool::new(),
            verify_pools: self.dests.iter().map(|_| BufferPool::new()).collect(),
            cache_warned: false,
            stamps: (0..self.plan.files.len())
                .map(|i| {
                    (0..self.dests.len())
                        .map(|d| match self.pre.get(i).map(|row| &row[d]) {
                            Some(Pre::Done(_, at)) => Some(*at),
                            _ => None,
                        })
                        .collect()
                })
                .collect(),
            sidecars: Vec::new(),
            journal: None,
        };
        if let Some(dir) = &self.spec.log_dir {
            match TransferLog::create(dir, started, &self.plan.source.label) {
                Ok(mut l) => {
                    l.header(self, started);
                    r.log = Some(l);
                }
                Err(e) => r.warn(format!("Couldn't create the transfer log in {}: {e}", dir.display())),
            }
        }
        if let Some(path) = &self.spec.journal {
            match JournalWriter::open(path) {
                Ok(mut j) => {
                    j.start(self, r.log.as_ref().map(|l| l.path()));
                    r.journal = Some(j);
                }
                Err(e) => r.warn(format!("Couldn't open the journal {}: {e}", path.display())),
            }
        }

        let all: Vec<usize> = (0..self.dests.len()).collect();
        let mut released = false;
        let mut completed = vec![false; self.dests.len()];
        let end = match self.spec.mode {
            CopyMode::Sequential => {
                let end = r.pass(Pass::Copy, None, &[0]);
                completed[0] = matches!(end, PassEnd::Done);
                if matches!(end, PassEnd::Done) && r.all_verified(&[0]) && self.plan.problems.is_empty() {
                    released = true;
                    r.release();
                    if self.dests.len() > 1 {
                        let end = r.pass(Pass::Backup, Some(0), &all[1..]);
                        let done = matches!(end, PassEnd::Done);
                        completed[1..].iter_mut().for_each(|c| *c = done);
                        end
                    } else {
                        end
                    }
                } else {
                    if matches!(end, PassEnd::Done) && self.dests.len() > 1 {
                        r.note("Backup on hold: the copy didn't fully verify.");
                    }
                    end
                }
            }
            CopyMode::FanOut => {
                let end = r.pass(Pass::FanOut, None, &all);
                completed.iter_mut().for_each(|c| *c = matches!(end, PassEnd::Done));
                if matches!(end, PassEnd::Done) && r.all_verified(&all) && self.plan.problems.is_empty() {
                    released = true;
                    r.release();
                }
                end
            }
        };

        // Sidecars go last: in sequential mode the backup re-reads the copy, including any
        // ascmhl history that came off the card, so the copy must not change before then.
        for (d, done) in completed.iter().enumerate() {
            if *done {
                r.write_sidecars(d);
            }
        }

        let outcome = match end {
            PassEnd::Cancelled => Outcome::Cancelled,
            PassEnd::Aborted(reason) => Outcome::Aborted { reason },
            PassEnd::Done => {
                let failed = r.files.iter().filter(|f| f.dests.iter().any(FileStatus::is_failed)).count()
                    + self.plan.problems.len();
                let pending = r.files.iter().any(|f| f.dests.iter().any(|d| matches!(d, FileStatus::Pending)));
                if failed == 0 && !pending { Outcome::Verified } else { Outcome::Failed { files: failed.max(1) } }
            }
        };
        let finished = Local::now();
        if let Some(l) = r.log.as_mut() {
            l.footer(finished, t0.elapsed(), &outcome, released);
        }
        if let Some(j) = r.journal.as_mut() {
            j.end(&outcome, &r.sidecars);
        }
        (r.emit)(&Event::Finished { outcome: outcome.clone() });

        JobReport {
            outcome,
            source_released: released,
            started: Some(started.into()),
            finished: Some(finished.into()),
            log_path: r.log.as_ref().map(|l| l.path().to_path_buf()),
            files: r.files,
            sidecars: r.sidecars,
            warnings: r.warnings,
        }
    }
}
