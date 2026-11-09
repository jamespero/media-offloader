//! A whole offload: one source, a copy destination and optional backups.

mod finalize;
mod pass;
mod prepare;
mod report;
mod resume;
mod run;
mod runner;
mod sidecars;
mod spec;

use std::path::PathBuf;

use crate::plan::Plan;

pub use crate::faults::Faults;
pub use prepare::Destination;
pub use report::{FileReport, JobReport};
pub use spec::{CopyMode, DestRole, JobSpec, Resume, Sidecars, VerifiedFile};

use resume::Pre;

/// A planned offload. [`Job::prepare`] checks everything up front and writes nothing;
/// [`Job::run`] does the work.
pub struct Job {
    spec: JobSpec,
    plan: Plan,
    dests: Vec<Destination>,
    /// File × destination, for a resumed run; empty otherwise.
    pre: Vec<Vec<Pre>>,
}

impl Job {
    pub fn spec(&self) -> &JobSpec {
        &self.spec
    }

    /// Records progress in this journal file (see [`crate::journal`]). Lets callers name
    /// it after the scanned source label.
    pub fn set_journal(&mut self, path: Option<PathBuf>) {
        self.spec.journal = path;
    }

    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    /// Copy destination first, then backups.
    pub fn destinations(&self) -> &[Destination] {
        &self.dests
    }
}
