//! One module per `offload` command.

pub mod copy;
pub mod eject;
pub mod hash;
pub mod history;
pub mod plan;
pub mod resume;
pub mod verify;

use std::process::ExitCode;

pub fn fail(e: &dyn std::fmt::Display) -> ExitCode {
    eprintln!("error: {e}");
    ExitCode::from(2)
}
