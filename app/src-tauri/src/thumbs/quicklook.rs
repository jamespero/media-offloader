//! Thumbnails from macOS QuickLook.

use std::fs;
use std::path::Path;

use super::SIZE;

/// How long QuickLook gets per file. Some formats (e.g. R3D with no QuickLook plugin)
/// make `qlmanage` hang forever, which would stall every thumbnail queued behind it.
const QL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// macOS: QuickLook's command-line thumbnailer (same previews as Finder).
pub(super) fn extract(path: &Path) -> Option<(&'static str, Vec<u8>)> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let dir = std::env::temp_dir().join(format!("media-offloader-thumbs-{}", std::process::id()));
    fs::create_dir_all(&dir).ok()?;
    let mut child = Command::new("qlmanage")
        .args(["-t", "-s", &SIZE.to_string(), "-o"])
        .arg(&dir)
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + QL_TIMEOUT;
    let ok = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            _ => {
                // Hung or unwaitable: give up on this file (it's remembered as "none").
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let out = dir.join(format!("{}.png", path.file_name()?.to_string_lossy()));
    let bytes = fs::read(&out).ok();
    let _ = fs::remove_file(&out);
    bytes.filter(|_| ok).map(|b| ("image/png", b))
}
