//! Keeping the computer awake during a transfer.

/// Keeps the Mac from idle-sleeping while held, via `caffeinate` tied to our process
/// (so it ends even if we crash). No-op elsewhere.
pub struct KeepAwake(Option<std::process::Child>);

#[allow(clippy::new_without_default)] // acquiring it has an effect; no silent defaults
impl KeepAwake {
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            let child = std::process::Command::new("caffeinate")
                .args(["-i", "-w", &std::process::id().to_string()])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .ok();
            KeepAwake(child)
        }
        #[cfg(not(target_os = "macos"))]
        KeepAwake(None)
    }
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        if let Some(mut c) = self.0.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}
