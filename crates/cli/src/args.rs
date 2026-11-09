//! The command line: what `offload` accepts.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use offloader_core::{CopyMode, HashConfig, JobSpec, Sidecars, journal, log};

#[derive(Parser)]
#[command(name = "offload", version, about = "Copy a camera card with checksum verification (Media Offloader engine)")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
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
pub struct CopyArgs {
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
    pub json: bool,
}

impl CopyArgs {
    pub fn spec(&self) -> JobSpec {
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
