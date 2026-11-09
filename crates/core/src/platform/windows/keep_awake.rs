//! Keeping the computer awake during a transfer.

/// Keeps the computer from sleeping while held (system sleep only; the display may
/// still turn off). Must be dropped on the thread that created it.
pub struct KeepAwake(());

#[allow(clippy::new_without_default)] // acquiring it has an effect; no silent defaults
impl KeepAwake {
    pub fn new() -> Self {
        use windows_sys::Win32::System::Power::{ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetThreadExecutionState};
        // SAFETY: plain flag call.
        unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) };
        KeepAwake(())
    }
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        use windows_sys::Win32::System::Power::{ES_CONTINUOUS, SetThreadExecutionState};
        // SAFETY: plain flag call; clears this thread's requirement.
        unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
    }
}
