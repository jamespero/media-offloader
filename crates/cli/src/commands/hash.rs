//! `offload hash`.

use std::path::PathBuf;
use std::process::ExitCode;

use offloader_core::{HashConfig, hash};

pub fn hash_files(files: &[PathBuf], cfg: HashConfig, uncached: bool) -> ExitCode {
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
