//! `offload`: drive the offload engine from a terminal. Handy for testing against real
//! cards before the app exists, and for scripting.

mod args;
mod commands;
mod progress;

use std::process::ExitCode;

use args::{Cli, Cmd};
use clap::Parser;
use offloader_core::{HashConfig, Job, log};

fn main() -> ExitCode {
    match Cli::parse().cmd {
        Cmd::Hash { files, md5, cached } => commands::hash::hash_files(&files, HashConfig { md5 }, !cached),
        Cmd::Plan(args) => match Job::prepare(args.spec()) {
            Ok(job) => {
                commands::plan::print_plan(&job, true);
                ExitCode::SUCCESS
            }
            Err(e) => commands::fail(&e),
        },
        Cmd::Copy(args) => match Job::prepare(args.spec()) {
            Ok(job) => commands::copy::run_job(&job, args.json),
            Err(e) => commands::fail(&e),
        },
        Cmd::Resume { journal: path, json } => commands::resume::resume(&path, json),
        Cmd::History => commands::history::history(),
        Cmd::Eject { volume } => commands::eject::eject(&volume),
        Cmd::Verify { folder, log_dir } => commands::verify::verify(&folder, log_dir.or_else(log::default_log_dir)),
    }
}
