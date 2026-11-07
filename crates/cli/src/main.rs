//! `offload`: drive the offload engine from a terminal. Handy for testing against real
//! cards before the app exists, and for scripting.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use offloader_core::{HashConfig, hash};

#[derive(Parser)]
#[command(name = "offload", version, about = "Copy a camera card with checksum verification (Media Offloader engine)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Make a card or drive safe to remove (dismount and eject).
    Eject { volume: PathBuf },
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

fn main() -> ExitCode {
    match Cli::parse().cmd {
        Cmd::Hash { files, md5, cached } => hash_files(&files, HashConfig { md5 }, !cached),
        Cmd::Eject { volume } => match offloader_core::platform::eject(&volume) {
            Ok(()) => {
                eprintln!("Ejected {}; it can be removed.", volume.display());
                ExitCode::SUCCESS
            }
            Err(e) => fail(&format!("couldn't eject {}: {e}", volume.display())),
        },
    }
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
