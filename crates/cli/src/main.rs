//! `offload`: drive the offload engine from a terminal. Handy for testing against real
//! cards before the app exists, and for scripting.

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clap::{Args, Parser, Subcommand};
use offloader_core::{
    CopyMode, Event, FileStatus, HashConfig, Job, JobSpec, Outcome, Pass, Sidecars, Stage, format, hash, journal, log,
    reverify,
};

#[derive(Parser)]
#[command(name = "offload", version, about = "Copy a camera card with checksum verification (Media Offloader engine)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Copy SOURCE into DEST and verify every file; optionally back it up too.
    Copy(CopyArgs),
    /// Show what `copy` would do, without writing anything.
    Plan(CopyArgs),
    /// Continue an interrupted offload, or retry its failed files, from its journal.
    Resume {
        /// The journal (.jsonl) of that offload; see `offload history`.
        journal: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// List recent offloads (from their journals).
    History,
    /// Make a card or drive safe to remove (dismount and eject).
    Eject { volume: PathBuf },
    /// Re-check a folder against its ASC MHL history (or .md5 list), reading every file from disk.
    Verify {
        folder: PathBuf,
        /// Where to write the re-verify log [default: the app's log folder].
        #[arg(long)]
        log_dir: Option<PathBuf>,
    },
    /// Print checksums, reading from disk with the OS cache bypassed.
    Hash {
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Also compute MD5.
        #[arg(long)]
        md5: bool,
        /// Read through the OS cache (normal reads).
        #[arg(long)]
        cached: bool,
    },
}

#[derive(Args)]
struct CopyArgs {
    /// Card, drive or folder to copy from.
    source: PathBuf,
    /// Folder to copy into (destination plus any subfolders).
    dest: PathBuf,
    /// Also back up to this folder. Repeat for more than one.
    #[arg(long, short)]
    backup: Vec<PathBuf>,
    /// Compute MD5 as well as xxHash64.
    #[arg(long)]
    md5: bool,
    /// Read the card once and write every destination at the same time.
    #[arg(long)]
    fan_out: bool,
    /// Also copy OS housekeeping (.Spotlight-V100, System Volume Information, …).
    #[arg(long)]
    include_system_files: bool,
    /// Where to write the transfer log [default: the app's log folder].
    #[arg(long)]
    log_dir: Option<PathBuf>,
    /// Don't write a transfer log.
    #[arg(long, conflicts_with = "log_dir")]
    no_log: bool,
    /// Don't write an ASC MHL manifest next to the footage.
    #[arg(long)]
    no_mhl: bool,
    /// Also write checksums.md5 next to the footage (needs --md5).
    #[arg(long, requires = "md5")]
    md5_list: bool,
    /// Print events as JSON lines on stdout instead of a progress line.
    #[arg(long)]
    json: bool,
}

impl CopyArgs {
    fn spec(&self) -> JobSpec {
        JobSpec {
            source: self.source.clone(),
            copy_to: self.dest.clone(),
            backups: self.backup.clone(),
            mode: if self.fan_out { CopyMode::FanOut } else { CopyMode::Sequential },
            hash: HashConfig { md5: self.md5 },
            include_system_files: self.include_system_files,
            log_dir: if self.no_log { None } else { self.log_dir.clone().or_else(log::default_log_dir) },
            sidecars: Sidecars { mhl: !self.no_mhl, md5_list: self.md5_list },
            resume: None,
            journal: if self.no_log { None } else { new_journal(&self.source) },
            keep_awake: true,
            faults: None,
        }
    }
}

/// A journal path for a new offload, named after the source folder or drive.
fn new_journal(source: &std::path::Path) -> Option<PathBuf> {
    let dir = journal::default_journal_dir()?;
    let label = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| source.to_string_lossy().trim_end_matches(['\\', '/', ':']).to_string());
    Some(journal::new_journal_path(&dir, &label))
}

fn main() -> ExitCode {
    match Cli::parse().cmd {
        Cmd::Hash { files, md5, cached } => hash_files(&files, HashConfig { md5 }, !cached),
        Cmd::Plan(args) => match Job::prepare(args.spec()) {
            Ok(job) => {
                print_plan(&job, true);
                ExitCode::SUCCESS
            }
            Err(e) => fail(&e),
        },
        Cmd::Copy(args) => match Job::prepare(args.spec()) {
            Ok(job) => run_job(&job, args.json),
            Err(e) => fail(&e),
        },
        Cmd::Resume { journal: path, json } => resume(&path, json),
        Cmd::History => history(),
        Cmd::Eject { volume } => match offloader_core::platform::eject(&volume) {
            Ok(()) => {
                eprintln!("Ejected {}; it can be removed.", volume.display());
                ExitCode::SUCCESS
            }
            Err(e) => fail(&format!("couldn't eject {}: {e}", volume.display())),
        },
        Cmd::Verify { folder, log_dir } => verify(&folder, log_dir.or_else(log::default_log_dir)),
    }
}

/// Continues an interrupted offload (or retries its failed files) from its journal.
fn resume(path: &std::path::Path, json: bool) -> ExitCode {
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

/// Lists recent offloads from their journals.
fn history() -> ExitCode {
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

fn fail(e: &dyn std::fmt::Display) -> ExitCode {
    eprintln!("error: {e}");
    ExitCode::from(2)
}

fn hash_files(files: &[PathBuf], cfg: HashConfig, uncached: bool) -> ExitCode {
    let mut code = ExitCode::SUCCESS;
    for f in files {
        match hash::hash_file(f, cfg, uncached) {
            Ok((h, bypassed)) => {
                let md5 = h.md5_hex().map(|m| format!("  {m}")).unwrap_or_default();
                println!("{}{md5}  {}", h.xxh64_hex(), f.display());
                if uncached && !bypassed {
                    eprintln!("note: couldn't bypass the OS cache for {}", f.display());
                }
            }
            Err(e) => {
                eprintln!("error: {}: {e}", f.display());
                code = ExitCode::from(1);
            }
        }
    }
    code
}

fn print_plan(job: &Job, verbose: bool) {
    let plan = job.plan();
    let kind = match plan.source.kind {
        offloader_core::SourceKind::Volume => "drive",
        offloader_core::SourceKind::Folder => "folder",
    };
    eprintln!("Source   {} ({kind})  {}", plan.source.label, plan.source.root.display());
    eprintln!(
        "Files    {} · {}  ({} skipped, {} renamed, {} unreadable)",
        plan.files.len(),
        format::size(plan.total_bytes),
        plan.skipped.len(),
        plan.renamed.len(),
        plan.problems.len()
    );
    for d in job.destinations() {
        let role = match d.role {
            offloader_core::DestRole::Copy => "Copy",
            offloader_core::DestRole::Backup => "Backup",
        };
        eprintln!(
            "{role:<8} {}  ({}, {} free, needs {})",
            d.root.display(),
            if d.fs_type.is_empty() { "?" } else { &d.fs_type },
            format::size(d.available),
            format::size(d.required)
        );
        if d.conflict.is_some() {
            eprintln!(
                "         {} already holds files from this source; nothing there will be touched.",
                d.requested.display()
            );
        }
    }
    if verbose {
        for s in &plan.skipped {
            eprintln!("  skip     {}  ({:?})", format::rel_path(&s.rel), s.reason);
        }
        for r in &plan.renamed {
            eprintln!("  rename   {} → {}", format::rel_path(&r.from), format::rel_path(&r.to));
        }
    }
    for p in &plan.problems {
        eprintln!("  UNREADABLE  {}  ({})", format::rel_path(&p.rel), p.message);
    }
}

fn run_job(job: &Job, json: bool) -> ExitCode {
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

/// Renders a single updating progress line on stderr.
struct Progress {
    names: Vec<String>,
    tty: bool,
    pass: Option<Pass>,
    file: usize,
    stage: Stage,
    done: u64,
    total: u64,
    samples: Vec<(Instant, u64)>,
    last_draw: Instant,
}

impl Progress {
    fn new(job: &Job) -> Self {
        Self {
            names: job.plan().files.iter().map(|f| format::rel_path(&f.dest_rel)).collect(),
            tty: io::stderr().is_terminal(),
            pass: None,
            file: 0,
            stage: Stage::Copy,
            done: 0,
            total: 0,
            samples: Vec::new(),
            last_draw: Instant::now(),
        }
    }

    fn clear(&self) {
        if self.tty {
            eprint!("\r{:<110}\r", "");
        }
    }

    fn on_event(&mut self, e: &Event) {
        match e {
            Event::PassStarted { pass, files, bytes, .. } => {
                self.pass = Some(*pass);
                self.samples.clear();
                let what = match pass {
                    Pass::Copy => "Copying",
                    Pass::Backup => "Backing up from the verified copy",
                    Pass::FanOut => "Copying to every destination at once",
                };
                eprintln!("{what}: {files} files, {}", format::size(*bytes));
            }
            Event::Progress { file, stage, done, total, .. } => {
                (self.file, self.stage, self.done, self.total) = (*file, *stage, *done, *total);
                let now = Instant::now();
                self.samples.push((now, *done));
                self.samples.retain(|(t, _)| now.duration_since(*t) < Duration::from_secs(4));
                if now.duration_since(self.last_draw) >= Duration::from_millis(120) {
                    self.last_draw = now;
                    self.draw();
                }
            }
            Event::FileFinished { file, results, .. } => {
                for (_, status) in results {
                    if let FileStatus::Failed { failure } = status {
                        self.clear();
                        eprintln!("FAIL  {}: {failure}", self.names[*file]);
                    }
                }
            }
            Event::SourceReleased => {
                self.clear();
                eprintln!("✓ Copy verified. The source can be removed.");
            }
            Event::PassFinished { verified, failed, .. } => {
                self.clear();
                eprintln!("  {verified} verified, {failed} failed");
            }
            Event::Warning { message } => {
                self.clear();
                eprintln!("warning: {message}");
            }
            Event::FileStarted { .. } | Event::Finished { .. } => {}
        }
    }

    fn draw(&self) {
        if !self.tty || self.total == 0 {
            return;
        }
        let pct = self.done as f64 * 100.0 / self.total as f64;
        let (rate, eta) = match (self.samples.first(), self.samples.last()) {
            (Some((t0, d0)), Some((t1, d1))) if t1 > t0 && d1 > d0 => {
                let r = (d1 - d0) as f64 / t1.duration_since(*t0).as_secs_f64();
                let left = Duration::from_secs_f64((self.total - self.done) as f64 / r);
                (format::rate(r), format!("{} left", format::duration(left)))
            }
            _ => (String::new(), String::new()),
        };
        let stage = match self.stage {
            Stage::Copy => "copying  ",
            Stage::Verify => "verifying",
        };
        let mut name = self.names.get(self.file).cloned().unwrap_or_default();
        if name.chars().count() > 44 {
            name =
                format!("…{}", name.chars().rev().take(43).collect::<Vec<_>>().into_iter().rev().collect::<String>());
        }
        eprint!("\r{pct:5.1}%  {stage}  {name:<45} {rate:>10}  {eta:<12}");
        let _ = io::stderr().flush();
    }
}

fn verify(folder: &std::path::Path, log_dir: Option<PathBuf>) -> ExitCode {
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
