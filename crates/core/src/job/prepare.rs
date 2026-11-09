//! Checking everything up front: resolving destinations, applying the never-overwrite
//! rule and checking free space, without writing anything.

use std::collections::HashMap;
use std::fs;
use std::iter;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::resume::{Pre, resume_state};
use super::{DestRole, Job, JobSpec};
use crate::error::PrepareError;
use crate::plan::{self, Plan};
use crate::platform;
use crate::scan;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    pub role: DestRole,
    /// The folder that was asked for.
    pub requested: PathBuf,
    /// Where files actually go: `requested`, or `requested/copy-conflicted[-N]` when
    /// `requested` already holds any of this source's files.
    pub root: PathBuf,
    /// `Some(0)` for `copy-conflicted`, `Some(n)` for `copy-conflicted-n`.
    pub conflict: Option<u32>,
    pub fs_type: String,
    pub available: u64,
    /// Space this transfer needs on this destination (rounded up to whole clusters).
    pub required: u64,
    #[serde(skip)]
    volume_id: String,
}

/// Slack on top of the cluster-rounded total, for file system metadata.
const SPACE_MARGIN: u64 = 16 << 20;

impl Job {
    pub fn prepare(spec: JobSpec) -> Result<Job, PrepareError> {
        let faults = spec.faults.as_deref();
        let scan = scan::scan(&spec.source, spec.include_system_files)?;
        let src_canon = fs::canonicalize(&scan.info.root)
            .map_err(|e| PrepareError::Io { path: scan.info.root.clone(), source: e })?;

        let requested: Vec<(DestRole, PathBuf)> = iter::once((DestRole::Copy, &spec.copy_to))
            .chain(spec.backups.iter().map(|b| (DestRole::Backup, b)))
            .map(|(role, p)| {
                std::path::absolute(p)
                    .map(|abs| (role, abs))
                    .map_err(|e| PrepareError::DestinationUnavailable { dest: p.clone(), source: e })
            })
            .collect::<Result<_, _>>()?;

        let mut canon = Vec::with_capacity(requested.len());
        let mut vols = Vec::with_capacity(requested.len());
        for (_, dest) in &requested {
            let unavailable = |e| PrepareError::DestinationUnavailable { dest: dest.clone(), source: e };
            let c = platform::canonical_lenient(dest).map_err(unavailable)?;
            if c.starts_with(&src_canon) {
                return Err(PrepareError::DestinationInsideSource { dest: dest.clone() });
            }
            vols.push(platform::volume_info(dest).map_err(unavailable)?);
            canon.push(c);
        }
        for i in 0..canon.len() {
            for j in i + 1..canon.len() {
                if canon[i].starts_with(&canon[j]) || canon[j].starts_with(&canon[i]) {
                    return Err(PrepareError::OverlappingDestinations {
                        a: requested[i].1.clone(),
                        b: requested[j].1.clone(),
                    });
                }
            }
        }

        // One naming scheme for every destination keeps the copies identical.
        let windows_rules = vols.iter().any(|v| v.windows_name_rules());
        let plan = plan::build(scan, windows_rules);

        let mut dests = Vec::with_capacity(requested.len());
        let mut clusters = Vec::with_capacity(requested.len());
        for (idx, ((role, req), vol)) in requested.into_iter().zip(vols).enumerate() {
            // A resumed offload continues in the folders it already wrote to.
            let (root, conflict) = match spec.resume.as_ref().and_then(|r| r.roots.get(idx)) {
                Some(root) => (root.clone(), None),
                None => resolve_root(&req, &plan)?,
            };
            let available = faults.and_then(|f| f.available_space(&req)).unwrap_or(vol.available);
            clusters.push(vol.cluster_size);
            dests.push(Destination {
                role,
                required: 0,
                requested: req,
                root,
                conflict,
                fs_type: vol.fs_type,
                available,
                volume_id: vol.id,
            });
        }
        let pre = match &spec.resume {
            Some(r) => resume_state(&plan, &dests, r),
            None => Vec::new(),
        };
        for (d, dest) in dests.iter_mut().enumerate() {
            let todo = |i: usize| pre.get(i).is_none_or(|row| matches!(row[d], Pre::Todo));
            dest.required = required_bytes(&plan, clusters[d], todo);
        }

        // Destinations sharing a volume need room for all of them.
        let mut by_volume: HashMap<&str, (u64, &Destination)> = HashMap::new();
        for d in &dests {
            by_volume.entry(&d.volume_id).or_insert((0, d)).0 += d.required;
        }
        for (needed, d) in by_volume.into_values() {
            if needed > d.available {
                return Err(PrepareError::InsufficientSpace {
                    dest: d.requested.clone(),
                    needed,
                    available: d.available,
                });
            }
        }

        Ok(Job { spec, plan, dests, pre })
    }
}

/// Where a destination's files go, applying the never-overwrite rule.
fn resolve_root(requested: &Path, plan: &Plan) -> Result<(PathBuf, Option<u32>), PrepareError> {
    if !occupied(requested, plan) {
        return Ok((requested.to_path_buf(), None));
    }
    for n in 0..10_000u32 {
        let name = if n == 0 { "copy-conflicted".to_string() } else { format!("copy-conflicted-{n}") };
        let candidate = requested.join(name);
        if !occupied(&candidate, plan) {
            return Ok((candidate, Some(n)));
        }
    }
    Err(PrepareError::NoConflictFolder(requested.to_path_buf()))
}

/// Would writing this plan under `root` touch anything that already exists?
fn occupied(root: &Path, plan: &Plan) -> bool {
    match fs::symlink_metadata(root) {
        Err(_) => return false,
        Ok(m) if !m.is_dir() => return true,
        Ok(_) => {}
    }
    let exists = |p: PathBuf| fs::symlink_metadata(p).is_ok();
    plan.files.iter().any(|f| exists(root.join(&f.dest_rel)) || exists(root.join(&f.partial_rel)))
        || plan.dirs.iter().any(|d| matches!(fs::symlink_metadata(root.join(d)), Ok(m) if !m.is_dir()))
}

fn required_bytes(plan: &Plan, cluster: u64, todo: impl Fn(usize) -> bool) -> u64 {
    let files: u64 =
        plan.files.iter().enumerate().filter(|(i, _)| todo(*i)).map(|(_, f)| f.size.div_ceil(cluster) * cluster).sum();
    files + (plan.dirs.len() as u64 + 1) * cluster + SPACE_MARGIN
}
