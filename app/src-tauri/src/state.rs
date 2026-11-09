//! What the shell keeps between commands.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use offloader_core::Job;

/// The job waiting to start (after `prepare`) and the cancel flag of the one running.
#[derive(Default)]
pub struct Engine {
    pub prepared: Mutex<Option<Job>>,
    pub running: Mutex<Option<Arc<AtomicBool>>>,
    pub reverifying: Mutex<Option<Arc<AtomicBool>>>,
}
