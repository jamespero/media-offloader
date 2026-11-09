//! `offload eject`.

use std::path::Path;
use std::process::ExitCode;

use super::fail;

pub fn eject(volume: &Path) -> ExitCode {
    match offloader_core::platform::eject(volume) {
        Ok(()) => {
            eprintln!("Ejected {}; it can be removed.", volume.display());
            ExitCode::SUCCESS
        }
        Err(e) => fail(&format!("couldn't eject {}: {e}", volume.display())),
    }
}
