//! One pass over the files: copy, verify, and only then give each its real name.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::finalize::finalize;
use super::runner::{PassEnd, Runner};
use crate::error::FileFailure;
use crate::event::{Event, FileStatus, Pass, Stage};
use crate::format;
use crate::hash::Hashes;
use crate::log;
use crate::transfer::{self, CopyOutcome, SourceExpect, VerifyOutcome};

impl Runner<'_> {
    /// Copies every file from `read_from` (the source, or a verified destination) to
    /// `targets`, verifying each before it gets its final name.
    pub(super) fn pass(&mut self, pass: Pass, read_from: Option<usize>, targets: &[usize]) -> PassEnd {
        let job = self.job;
        let plan = &job.plan;
        let cfg = job.spec.hash;
        let total = plan.total_bytes * 2;
        (self.emit)(&Event::PassStarted {
            pass,
            targets: targets.to_vec(),
            files: plan.files.len(),
            bytes: plan.total_bytes,
        });

        let names: Vec<String> = targets.iter().map(|&t| job.dests[t].root.display().to_string()).collect();
        let title = match pass {
            Pass::Copy => format!("copy → {}", names.join(", ")),
            Pass::Backup => format!("backup (from the verified copy) → {}", names.join(", ")),
            Pass::FanOut => format!("fan-out → {}", names.join(", ")),
        };
        if let Some(l) = self.log.as_mut() {
            l.section(&title);
        }

        for &t in targets {
            let root = &job.dests[t].root;
            let made = fs::create_dir_all(root)
                .and_then(|()| plan.dirs.iter().try_for_each(|d| fs::create_dir_all(root.join(d))));
            if let Err(e) = made {
                let reason = format!("couldn't create folders in {}: {e}", root.display());
                self.note(&format!("Stopped: {reason}"));
                return PassEnd::Aborted(reason);
            }
        }

        let (mut verified, mut failed) = (0, 0);
        let mut done = 0u64;
        for (i, f) in plan.files.iter().enumerate() {
            if self.cancel.load(Ordering::Relaxed) {
                return PassEnd::Cancelled;
            }
            // In a resumed run some destinations may already have this file (or be
            // blocked by something else with its name): only the rest are worked on.
            let todo: Vec<usize> =
                targets.iter().copied().filter(|&t| matches!(self.files[i].dests[t], FileStatus::Pending)).collect();
            if todo.is_empty() {
                let results: Vec<(usize, FileStatus)> =
                    targets.iter().map(|&t| (t, self.files[i].dests[t].clone())).collect();
                for (t, status) in &results {
                    match status {
                        FileStatus::Verified => verified += 1,
                        FileStatus::Failed { .. } => failed += 1,
                        FileStatus::Pending => {}
                    }
                    if let Some(l) = self.log.as_mut() {
                        l.file(
                            &log::role_title(&job.dests[*t]).to_lowercase(),
                            f,
                            self.files[i].hashes.as_ref(),
                            status,
                        );
                    }
                }
                (self.emit)(&Event::FileFinished { pass, file: i, hashes: self.files[i].hashes, results });
                done += f.size * 2;
                continue;
            }
            let targets: &[usize] = &todo;
            (self.emit)(&Event::FileStarted { pass, file: i });

            let src = match read_from {
                None => plan.source.root.join(&f.src_rel),
                Some(d) => job.dests[d].root.join(&f.dest_rel),
            };
            let partials: Vec<PathBuf> = targets.iter().map(|&t| job.dests[t].root.join(&f.partial_rel)).collect();
            if job.spec.resume.is_some() {
                // Temp files left by the interrupted run are ours; start them over.
                for p in &partials {
                    transfer::discard(p);
                }
            }
            let expect = SourceExpect { size: f.size, modified: f.modified, accessed: f.accessed, created: f.created };
            let emit = self.emit;
            let copied = transfer::copy_to_partials(
                &src,
                &expect,
                &partials,
                cfg,
                &self.copy_pool,
                self.cancel,
                self.faults,
                &mut |b| {
                    emit(&Event::Progress { pass, file: i, stage: Stage::Copy, file_bytes: b, done: done + b, total })
                },
            );

            let mut results: Vec<(usize, FileStatus)> = Vec::with_capacity(targets.len());
            let mut abort: Option<String> = None;
            let fail_all = |failure: FileFailure| -> Vec<(usize, FileStatus)> {
                targets.iter().map(|&t| (t, FileStatus::Failed { failure: failure.clone() })).collect()
            };
            match copied {
                CopyOutcome::Cancelled => return PassEnd::Cancelled,
                CopyOutcome::SourceError(e) => {
                    results = fail_all(FileFailure::source_read(&e));
                    let root = read_from.map_or(&plan.source.root, |d| &job.dests[d].root);
                    if !root.exists() {
                        abort = Some(match read_from {
                            None => "the source was disconnected".to_string(),
                            Some(d) => format!("{} was disconnected", job.dests[d].root.display()),
                        });
                    }
                }
                CopyOutcome::SourceChanged(when) => {
                    results = fail_all(FileFailure::SourceChanged { when: when.to_string() });
                }
                CopyOutcome::Copied { hashes, targets: writes } => {
                    // A backup must reproduce the hash verified during the copy pass; in a
                    // resumed run, the card must still match what the earlier run verified.
                    let expected = self.files[i].hashes.unwrap_or(hashes);
                    self.files[i].hashes = Some(expected);
                    if !expected.matches(&hashes) {
                        for (p, w) in partials.iter().zip(&writes) {
                            if w.is_ok() {
                                transfer::discard(p);
                            }
                        }
                        results = fail_all(match read_from {
                            Some(_) => FileFailure::CopyChanged { expected, actual: hashes },
                            None => FileFailure::SourceChanged { when: "since the earlier run verified it".into() },
                        });
                    } else {
                        let mut to_verify = Vec::new();
                        for (slot, w) in writes.into_iter().enumerate() {
                            let t = targets[slot];
                            match w {
                                Ok(()) => to_verify.push(slot),
                                Err(e) => {
                                    if e.kind() == io::ErrorKind::StorageFull {
                                        abort = Some(format!("{} is full", job.dests[t].root.display()));
                                    } else if !job.dests[t].root.exists() {
                                        abort = Some(format!("{} was disconnected", job.dests[t].root.display()));
                                    }
                                    results.push((t, FileStatus::Failed { failure: FileFailure::write(&e) }));
                                }
                            }
                        }
                        let outcomes = self.verify_all(
                            pass,
                            i,
                            &to_verify,
                            targets,
                            &partials,
                            &expected,
                            f.size,
                            done + f.size,
                            total,
                        );
                        let mut cancelled = false;
                        for (slot, outcome) in to_verify.iter().copied().zip(outcomes) {
                            let t = targets[slot];
                            let partial = &partials[slot];
                            let status = match outcome {
                                VerifyOutcome::Match { cache_bypassed } => {
                                    if !cache_bypassed && !self.cache_warned {
                                        self.cache_warned = true;
                                        self.warn(format!(
                                            "Verification on {} couldn't bypass the OS cache; it read through the cache instead.",
                                            job.dests[t].root.display()
                                        ));
                                    }
                                    finalize(partial, &job.dests[t].root.join(&f.dest_rel))
                                }
                                VerifyOutcome::Mismatch { actual } => {
                                    transfer::discard(partial);
                                    FileStatus::Failed { failure: FileFailure::Mismatch { expected, actual } }
                                }
                                VerifyOutcome::ReadError(e) => {
                                    transfer::discard(partial);
                                    FileStatus::Failed { failure: FileFailure::verify_read(&e) }
                                }
                                VerifyOutcome::Cancelled => {
                                    transfer::discard(partial);
                                    cancelled = true;
                                    FileStatus::Pending
                                }
                            };
                            results.push((t, status));
                        }
                        if cancelled {
                            // Verified siblings keep their final names; nothing partial remains.
                            for (t, status) in results {
                                self.set_status(i, t, &status);
                            }
                            return PassEnd::Cancelled;
                        }
                        results.sort_by_key(|(t, _)| *t);
                    }
                }
            }

            for (t, status) in &results {
                match status {
                    FileStatus::Verified => verified += 1,
                    FileStatus::Failed { .. } => failed += 1,
                    FileStatus::Pending => {}
                }
                if let Some(l) = self.log.as_mut() {
                    l.file(&log::role_title(&job.dests[*t]).to_lowercase(), f, self.files[i].hashes.as_ref(), status);
                }
                self.set_status(i, *t, status);
            }
            (self.emit)(&Event::FileFinished { pass, file: i, hashes: self.files[i].hashes, results });
            done += f.size * 2;

            if let Some(reason) = abort {
                self.note(&format!("Stopped: {reason}"));
                return PassEnd::Aborted(reason);
            }
        }
        (self.emit)(&Event::PassFinished { pass, verified, failed });
        if let Some(l) = self.log.as_mut() {
            l.line(&format!("{verified} verified, {failed} failed ({})", format::size(plan.total_bytes)));
        }
        PassEnd::Done
    }

    /// Verifies the written targets; several targets are read in parallel (they're
    /// normally on different drives). Progress reports the average across them.
    #[allow(clippy::too_many_arguments)]
    fn verify_all(
        &self,
        pass: Pass,
        file: usize,
        slots: &[usize],
        targets: &[usize],
        partials: &[PathBuf],
        expected: &Hashes,
        size: u64,
        base: u64,
        total: u64,
    ) -> Vec<VerifyOutcome> {
        let cfg = self.job.spec.hash;
        let emit = self.emit;
        let counters: Vec<AtomicU64> = slots.iter().map(|_| AtomicU64::new(0)).collect();
        let report = |k: usize, b: u64| {
            counters[k].store(b, Ordering::Relaxed);
            let avg = counters.iter().map(|c| c.load(Ordering::Relaxed)).sum::<u64>() / counters.len() as u64;
            emit(&Event::Progress { pass, file, stage: Stage::Verify, file_bytes: avg, done: base + avg, total });
        };
        let run_one = |k: usize, slot: usize| {
            transfer::verify(
                &partials[slot],
                expected,
                size,
                cfg,
                &self.verify_pools[targets[slot]],
                self.cancel,
                self.faults,
                &mut |b| report(k, b),
            )
        };
        if slots.len() <= 1 {
            return slots.iter().enumerate().map(|(k, &s)| run_one(k, s)).collect();
        }
        std::thread::scope(|scope| {
            let handles: Vec<_> = slots.iter().enumerate().map(|(k, &s)| scope.spawn(move || run_one(k, s))).collect();
            handles.into_iter().map(|h| h.join().expect("verify thread panicked")).collect()
        })
    }
}
