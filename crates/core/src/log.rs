//! The plain-text log kept for every transfer: one line per file per destination with
//! its checksums, plus what was skipped or renamed and how it ended. Written as the job
//! goes, so a crash still leaves a useful log.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Local};

use crate::event::{FileStatus, Outcome};
use crate::format;
use crate::hash::Hashes;
use crate::job::{CopyMode, Destination, Job};
use crate::plan::{PlannedFile, RenameReason};
use crate::scan::SkipReason;
use crate::{APP_NAME, VERSION};

/// Where transfer logs go by default:
/// `%LOCALAPPDATA%\Media Offloader\Logs` on Windows, `~/Library/Logs/Media Offloader` on macOS.
pub fn default_log_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join(APP_NAME).join("Logs"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Logs").join(APP_NAME))
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
            .map(|d| d.join("media-offloader").join("logs"))
    }
}

pub(crate) struct TransferLog {
    out: BufWriter<File>,
    path: PathBuf,
}

impl TransferLog {
    /// Creates `<dir>/<YYYY-MM-DD HHMMSS> <source label>.log` (never overwriting).
    pub fn create(dir: &Path, started: DateTime<Local>, label: &str) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        let safe: String = label
            .chars()
            .map(|c| if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.') { c } else { '_' })
            .collect();
        let stem = format!("{} {}", started.format("%Y-%m-%d %H%M%S"), safe.trim());
        for n in 0.. {
            let name = if n == 0 { format!("{stem}.log") } else { format!("{stem} ({n}).log") };
            let path = dir.join(name);
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(f) => return Ok(Self { out: BufWriter::new(f), path }),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Logging is best effort: a full log drive must not fail the offload.
    pub fn line(&mut self, s: &str) {
        let _ = writeln!(self.out, "{s}").and_then(|()| self.out.flush());
    }

    pub fn header(&mut self, job: &Job, started: DateTime<Local>) {
        let plan = job.plan();
        let spec = job.spec();
        self.line(&format!("{APP_NAME} {VERSION} — transfer log"));
        self.line(&format!("Started      {}", started.format("%Y-%m-%d %H:%M:%S %:z")));
        self.line(&format!("Source       {}  ({})", plan.source.label, plan.source.root.display()));
        for d in job.destinations() {
            self.line(&format!("{:<13}{}", role_title(d), dest_desc(d)));
        }
        let mode = match spec.mode {
            CopyMode::Sequential => "sequential (copy, then back up from the verified copy)",
            CopyMode::FanOut => "fan-out (one card read, every drive written at once)",
        };
        let hash = if spec.hash.md5 { "xxHash64 + MD5" } else { "xxHash64" };
        self.line(&format!("Mode         {mode} · {hash} · verify reads bypass the OS cache"));
        let skipped_system = plan.skipped.iter().filter(|s| s.reason == SkipReason::SystemFile).count();
        let mut files = format!("Files        {} · {}", plan.files.len(), format::size(plan.total_bytes));
        if skipped_system > 0 {
            let s = if skipped_system == 1 { "" } else { "s" };
            files.push_str(&format!("  ({skipped_system} system item{s} skipped)"));
        }
        self.line(&files);

        if !plan.skipped.is_empty() || !plan.renamed.is_empty() || !plan.problems.is_empty() {
            self.line("");
        }
        for s in &plan.skipped {
            let why = match s.reason {
                SkipReason::SystemFile => "OS system file",
                SkipReason::Symlink => "symbolic link, not followed",
                SkipReason::NotARegularFile => "not a regular file",
                SkipReason::UnreadableSystemFolder => "unreadable system folder",
            };
            self.line(&format!("Skipped      {}  ({why})", format::rel_path(&s.rel)));
        }
        for r in &plan.renamed {
            let why = match r.reason {
                RenameReason::IllegalName => "name not allowed on the destination",
                RenameReason::CaseCollision => "differs from another name only by case",
            };
            self.line(&format!("Renamed      {} → {}  ({why})", format::rel_path(&r.from), format::rel_path(&r.to)));
        }
        for p in &plan.problems {
            self.line(&format!("UNREADABLE   {}  ({})", format::rel_path(&p.rel), p.message));
        }
    }

    pub fn section(&mut self, title: &str) {
        self.line("");
        self.line(&format!("== {title} =="));
    }

    pub fn file(&mut self, role: &str, f: &PlannedFile, hashes: Option<&Hashes>, status: &FileStatus) {
        let path = format::rel_path(&f.dest_rel);
        let size = format::size(f.size);
        let sums = hashes
            .map(|h| match h.md5_hex() {
                Some(m) => format!("{}  {m}", h.xxh64_hex()),
                None => h.xxh64_hex(),
            })
            .unwrap_or_default();
        match status {
            FileStatus::Verified => self.line(&format!("OK    {role:<7}{sums}  {path}  {size}")),
            FileStatus::Failed { failure } => self.line(&format!("FAIL  {role:<7}{path}  {size}  — {failure}")),
            FileStatus::Pending => self.line(&format!("--    {role:<7}{path}  {size}  — not copied")),
        }
    }

    pub fn footer(&mut self, finished: DateTime<Local>, elapsed: Duration, outcome: &Outcome, released: bool) {
        self.line("");
        self.line(&format!(
            "Finished     {}  ({})",
            finished.format("%Y-%m-%d %H:%M:%S %:z"),
            format::duration(elapsed)
        ));
        let result = match outcome {
            Outcome::Verified => "All files verified.".to_string(),
            Outcome::Failed { files } => {
                format!("{files} file(s) not verified. This offload is NOT complete; keep the card.")
            }
            Outcome::Cancelled => "Stopped. Verified files were kept; the file in progress was removed.".to_string(),
            Outcome::Aborted { reason } => format!("Stopped: {reason}. Verified files were kept."),
        };
        self.line(&format!("Result       {result}"));
        if released {
            self.line("Source       Every file is verified on the copy drive; the source can be removed.");
        }
    }
}

pub(crate) fn role_title(d: &Destination) -> &'static str {
    match d.role {
        crate::job::DestRole::Copy => "Copy",
        crate::job::DestRole::Backup => "Backup",
    }
}

fn dest_desc(d: &Destination) -> String {
    match d.conflict {
        None => d.root.display().to_string(),
        Some(_) => format!("{}  (already held files from this source, so nothing there was touched)", d.root.display()),
    }
}
