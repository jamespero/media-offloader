//! A whole offload: one source, a copy destination and optional backups.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::iter;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Instant, SystemTime};

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::error::{FileFailure, PrepareError};
use crate::event::{Event, FileStatus, Outcome, Pass, Stage};
use crate::format;
use crate::hash::{HashConfig, Hashes};
use crate::log::{self, TransferLog};
use crate::pipeline::BufferPool;
use crate::plan::{self, Plan};
use crate::platform;
use crate::scan;
use crate::serde_util;
use crate::transfer::{self, CopyOutcome, SourceExpect, VerifyOutcome};

/// How copy and backup are scheduled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CopyMode {
    /// Copy and verify, release the card, then back up from the verified copy (classic).
    #[default]
    Sequential,
    /// Read the card once and write every destination at the same time; the card is
    /// released when all of them verify.
    FanOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DestRole {
    Copy,
    Backup,
}

/// Test hooks for simulating failures. Not part of the stable API.
#[doc(hidden)]
pub trait Faults: Send + Sync {
    /// Called after each read from a source file, with the bytes read so far.
    fn after_source_read(&self, _src: &Path, _offset: u64) {}
    /// Return an error to make the next read from `src` fail.
    fn source_read_error(&self, _src: &Path, _offset: u64) -> Option<io::Error> {
        None
    }
    /// Called with a fully written `.partial` right before it is verified.
    fn before_verify(&self, _partial: &Path) {}
    /// Override the free space reported for a destination.
    fn available_space(&self, _dest: &Path) -> Option<u64> {
        None
    }
}

/// What to offload, and where.
#[derive(Clone, Default)]
pub struct JobSpec {
    /// Card, drive or folder to copy from.
    pub source: PathBuf,
    /// Folder to copy into: the chosen destination plus any subfolders.
    pub copy_to: PathBuf,
    /// Backup folders (usually zero or one).
    pub backups: Vec<PathBuf>,
    pub mode: CopyMode,
    pub hash: HashConfig,
    /// Copy OS housekeeping (.Spotlight-V100, System Volume Information…) too.
    pub include_system_files: bool,
    /// Where to write the transfer log; `None` for no log.
    pub log_dir: Option<PathBuf>,
    /// Keep the computer from sleeping while the job runs.
    pub keep_awake: bool,
    #[doc(hidden)]
    pub faults: Option<Arc<dyn Faults>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    pub role: DestRole,
    /// The folder that was asked for.
    pub requested: PathBuf,
    /// Where files actually go: `requested`, or `requested/copy-conflicted[-N]` when
    /// `requested` already holds any of this source's files.
    pub root: PathBuf,
    /// `Some(0)` for `copy-conflicted`, `Some(n)` for `copy-conflicted-n`.
    pub conflict: Option<u32>,
    pub fs_type: String,
    pub available: u64,
    /// Space this transfer needs on this destination (rounded up to whole clusters).
    pub required: u64,
    #[serde(skip)]
    volume_id: String,
}

/// The state of one file at the end of a run.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileReport {
    /// Checksums of the source, once it has been read.
    pub hashes: Option<Hashes>,
    /// One status per destination, indexed like [`Job::destinations`].
    pub dests: Vec<FileStatus>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobReport {
    pub outcome: Outcome,
    /// True once every file was verified somewhere other than the source.
    pub source_released: bool,
    #[serde(serialize_with = "serde_util::opt_time_ms")]
    pub started: Option<SystemTime>,
    #[serde(serialize_with = "serde_util::opt_time_ms")]
    pub finished: Option<SystemTime>,
    /// Indexed like [`Plan::files`].
    pub files: Vec<FileReport>,
    pub log_path: Option<PathBuf>,
    pub warnings: Vec<String>,
}

impl JobReport {
    pub fn verified_count(&self, dest: usize) -> usize {
        self.files.iter().filter(|f| f.dests[dest].is_verified()).count()
    }
    pub fn failed_count(&self, dest: usize) -> usize {
        self.files.iter().filter(|f| f.dests[dest].is_failed()).count()
    }
}

/// A planned offload. [`Job::prepare`] checks everything up front and writes nothing;
/// [`Job::run`] does the work.
pub struct Job {
    spec: JobSpec,
    plan: Plan,
    dests: Vec<Destination>,
}

/// Slack on top of the cluster-rounded total, for file system metadata.
const SPACE_MARGIN: u64 = 16 << 20;

impl Job {
    pub fn prepare(spec: JobSpec) -> Result<Job, PrepareError> {
        let faults = spec.faults.as_deref();
        let scan = scan::scan(&spec.source, spec.include_system_files)?;
        let src_canon = fs::canonicalize(&scan.info.root)
            .map_err(|e| PrepareError::Io { path: scan.info.root.clone(), source: e })?;

        let requested: Vec<(DestRole, PathBuf)> = iter::once((DestRole::Copy, &spec.copy_to))
            .chain(spec.backups.iter().map(|b| (DestRole::Backup, b)))
            .map(|(role, p)| {
                std::path::absolute(p)
                    .map(|abs| (role, abs))
                    .map_err(|e| PrepareError::DestinationUnavailable { dest: p.clone(), source: e })
            })
            .collect::<Result<_, _>>()?;

        let mut canon = Vec::with_capacity(requested.len());
        let mut vols = Vec::with_capacity(requested.len());
        for (_, dest) in &requested {
            let unavailable = |e| PrepareError::DestinationUnavailable { dest: dest.clone(), source: e };
            let c = platform::canonical_lenient(dest).map_err(unavailable)?;
            if c.starts_with(&src_canon) {
                return Err(PrepareError::DestinationInsideSource { dest: dest.clone() });
            }
            vols.push(platform::volume_info(dest).map_err(unavailable)?);
            canon.push(c);
        }
        for i in 0..canon.len() {
            for j in i + 1..canon.len() {
                if canon[i].starts_with(&canon[j]) || canon[j].starts_with(&canon[i]) {
                    return Err(PrepareError::OverlappingDestinations {
                        a: requested[i].1.clone(),
                        b: requested[j].1.clone(),
                    });
                }
            }
        }

        // One naming scheme for every destination keeps the copies identical.
        let windows_rules = vols.iter().any(|v| v.windows_name_rules());
        let plan = plan::build(scan, windows_rules);

        let mut dests = Vec::with_capacity(requested.len());
        let mut clusters = Vec::with_capacity(requested.len());
        for ((role, req), vol) in requested.into_iter().zip(vols) {
            let (root, conflict) = resolve_root(&req, &plan)?;
            let available = faults.and_then(|f| f.available_space(&req)).unwrap_or(vol.available);
            clusters.push(vol.cluster_size);
            dests.push(Destination {
                role,
                required: 0,
                requested: req,
                root,
                conflict,
                fs_type: vol.fs_type,
                available,
                volume_id: vol.id,
            });
        }
        for (d, dest) in dests.iter_mut().enumerate() {
            dest.required = required_bytes(&plan, clusters[d]);
        }

        // Destinations sharing a volume need room for all of them.
        let mut by_volume: HashMap<&str, (u64, &Destination)> = HashMap::new();
        for d in &dests {
            by_volume.entry(&d.volume_id).or_insert((0, d)).0 += d.required;
        }
        for (needed, d) in by_volume.into_values() {
            if needed > d.available {
                return Err(PrepareError::InsufficientSpace {
                    dest: d.requested.clone(),
                    needed,
                    available: d.available,
                });
            }
        }

        Ok(Job { spec, plan, dests })
    }

    pub fn spec(&self) -> &JobSpec {
        &self.spec
    }

    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    /// Copy destination first, then backups.
    pub fn destinations(&self) -> &[Destination] {
        &self.dests
    }

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
            files: (0..self.plan.files.len())
                .map(|_| FileReport { hashes: None, dests: vec![FileStatus::Pending; self.dests.len()] })
                .collect(),
            warnings: Vec::new(),
            copy_pool: BufferPool::new(),
            verify_pools: self.dests.iter().map(|_| BufferPool::new()).collect(),
            cache_warned: false,
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

        let all: Vec<usize> = (0..self.dests.len()).collect();
        let mut released = false;
        let end = match self.spec.mode {
            CopyMode::Sequential => {
                let end = r.pass(Pass::Copy, None, &[0]);
                if matches!(end, PassEnd::Done) && r.all_verified(&[0]) && self.plan.problems.is_empty() {
                    released = true;
                    r.release();
                    if self.dests.len() > 1 { r.pass(Pass::Backup, Some(0), &all[1..]) } else { end }
                } else {
                    if matches!(end, PassEnd::Done) && self.dests.len() > 1 {
                        r.note("Backup on hold: the copy didn't fully verify.");
                    }
                    end
                }
            }
            CopyMode::FanOut => {
                let end = r.pass(Pass::FanOut, None, &all);
                if matches!(end, PassEnd::Done) && r.all_verified(&all) && self.plan.problems.is_empty() {
                    released = true;
                    r.release();
                }
                end
            }
        };

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
        (r.emit)(&Event::Finished { outcome: outcome.clone() });

        JobReport {
            outcome,
            source_released: released,
            started: Some(started.into()),
            finished: Some(finished.into()),
            log_path: r.log.as_ref().map(|l| l.path().to_path_buf()),
            files: r.files,
            warnings: r.warnings,
        }
    }
}

/// Where a destination's files go, applying the never-overwrite rule.
fn resolve_root(requested: &Path, plan: &Plan) -> Result<(PathBuf, Option<u32>), PrepareError> {
    if !occupied(requested, plan) {
        return Ok((requested.to_path_buf(), None));
    }
    for n in 0..10_000u32 {
        let name = if n == 0 { "copy-conflicted".to_string() } else { format!("copy-conflicted-{n}") };
        let candidate = requested.join(name);
        if !occupied(&candidate, plan) {
            return Ok((candidate, Some(n)));
        }
    }
    Err(PrepareError::NoConflictFolder(requested.to_path_buf()))
}

/// Would writing this plan under `root` touch anything that already exists?
fn occupied(root: &Path, plan: &Plan) -> bool {
    match fs::symlink_metadata(root) {
        Err(_) => return false,
        Ok(m) if !m.is_dir() => return true,
        Ok(_) => {}
    }
    let exists = |p: PathBuf| fs::symlink_metadata(p).is_ok();
    plan.files.iter().any(|f| exists(root.join(&f.dest_rel)) || exists(root.join(&f.partial_rel)))
        || plan.dirs.iter().any(|d| matches!(fs::symlink_metadata(root.join(d)), Ok(m) if !m.is_dir()))
}

fn required_bytes(plan: &Plan, cluster: u64) -> u64 {
    let files: u64 = plan.files.iter().map(|f| f.size.div_ceil(cluster) * cluster).sum();
    files + (plan.dirs.len() as u64 + 1) * cluster + SPACE_MARGIN
}

enum PassEnd {
    Done,
    Cancelled,
    Aborted(String),
}

struct Runner<'a> {
    job: &'a Job,
    cancel: &'a AtomicBool,
    emit: &'a (dyn Fn(&Event) + Sync),
    faults: Option<&'a dyn Faults>,
    log: Option<TransferLog>,
    files: Vec<FileReport>,
    warnings: Vec<String>,
    copy_pool: BufferPool,
    verify_pools: Vec<BufferPool>,
    cache_warned: bool,
}

impl Runner<'_> {
    /// Every file is verified off the source: tell the UI.
    fn release(&mut self) {
        (self.emit)(&Event::SourceReleased);
    }

    fn warn(&mut self, message: String) {
        self.note(&message);
        (self.emit)(&Event::Warning { message: message.clone() });
        self.warnings.push(message);
    }

    fn note(&mut self, s: &str) {
        if let Some(l) = self.log.as_mut() {
            l.line(s);
        }
    }

    fn all_verified(&self, dests: &[usize]) -> bool {
        self.files.iter().all(|f| dests.iter().all(|&d| f.dests[d].is_verified()))
    }

    /// Copies every file from `read_from` (the source, or a verified destination) to
    /// `targets`, verifying each before it gets its final name.
    fn pass(&mut self, pass: Pass, read_from: Option<usize>, targets: &[usize]) -> PassEnd {
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
                                self.files[i].dests[t] = status;
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
                self.files[i].dests[*t] = status.clone();
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

/// Gives a verified temp file its real name, without ever replacing anything.
fn finalize(partial: &Path, final_path: &Path) -> FileStatus {
    match platform::rename_no_replace(partial, final_path) {
        Ok(()) => {
            if let Some(dir) = final_path.parent() {
                let _ = platform::sync_dir(dir);
            }
            FileStatus::Verified
        }
        Err(e) => {
            transfer::discard(partial);
            let failure = if e.kind() == io::ErrorKind::AlreadyExists {
                FileFailure::AlreadyExists { path: final_path.display().to_string() }
            } else {
                FileFailure::write(&e)
            };
            FileStatus::Failed { failure }
        }
    }
}
