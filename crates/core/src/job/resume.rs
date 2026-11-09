//! Picking up an earlier run of the same offload.

use std::fs;
use std::time::SystemTime;

use super::{Destination, FileReport, Job, Resume};
use crate::error::FileFailure;
use crate::event::FileStatus;
use crate::format;
use crate::hash::Hashes;
use crate::journal;
use crate::plan::Plan;

/// A file's state on a destination before a resumed run starts.
#[derive(Clone, Debug)]
pub(super) enum Pre {
    /// Still to copy.
    Todo,
    /// Verified by the earlier run and still there.
    Done(Hashes, SystemTime),
    /// Something else has that name now; never overwritten.
    Blocked(FileFailure),
}

impl Job {
    /// A file's status before this run: pending, or what a resumed run already knows.
    pub(super) fn initial_report(&self, i: usize) -> FileReport {
        let mut r = FileReport { hashes: None, dests: vec![FileStatus::Pending; self.dests.len()] };
        for (d, pre) in self.pre.get(i).into_iter().flatten().enumerate() {
            match pre {
                Pre::Done(h, _) => {
                    r.hashes = Some(*h);
                    r.dests[d] = FileStatus::Verified;
                }
                Pre::Blocked(failure) => r.dests[d] = FileStatus::Failed { failure: failure.clone() },
                Pre::Todo => {}
            }
        }
        r
    }
}

/// For a resumed run: which files are already done on each destination. A file counts
/// as done only if the earlier run verified it, it is still there at the right size,
/// and the source file still has the size and date it had then. Anything else already
/// using the name is left alone and reported.
pub(super) fn resume_state(plan: &Plan, dests: &[Destination], r: &Resume) -> Vec<Vec<Pre>> {
    let same_time = |a: Option<SystemTime>, b: Option<SystemTime>| match (a, b) {
        (Some(a), Some(b)) => journal::ms(a) == journal::ms(b),
        _ => true,
    };
    plan.files
        .iter()
        .map(|f| {
            let rel = format::rel_path(&f.dest_rel);
            dests
                .iter()
                .enumerate()
                .map(|(d, dest)| {
                    let final_path = dest.root.join(&f.dest_rel);
                    let on_disk = fs::symlink_metadata(&final_path).ok();
                    let recorded = r.verified.get(d).and_then(|m| m.get(&rel));
                    match (recorded, on_disk) {
                        (Some(v), Some(m))
                            if m.is_file()
                                && m.len() == f.size
                                && v.size == f.size
                                && same_time(v.modified, f.modified) =>
                        {
                            Pre::Done(v.hashes, v.at)
                        }
                        (_, Some(_)) => {
                            Pre::Blocked(FileFailure::AlreadyExists { path: final_path.display().to_string() })
                        }
                        (_, None) => Pre::Todo,
                    }
                })
                .collect()
        })
        .collect()
}
