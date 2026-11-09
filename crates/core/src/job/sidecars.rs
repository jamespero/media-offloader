//! Writing the MHL generation and `.md5` list at a destination.

use std::fs;
use std::path::Path;
use std::time::SystemTime;

use super::JobSpec;
use super::runner::Runner;
use crate::format;
use crate::mhl;
use crate::scan;

impl Runner<'_> {
    /// Writes the MHL generation (and `.md5` list) for destination `d`. A failure here
    /// is a warning: the footage itself is verified either way.
    pub(super) fn write_sidecars(&mut self, d: usize) {
        let job = self.job;
        let spec = &job.spec;
        let root = &job.dests[d].root;
        let verified: Vec<usize> = (0..job.plan.files.len())
            .filter(|&i| self.files[i].dests[d].is_verified() && self.files[i].hashes.is_some())
            .collect();
        if verified.is_empty() {
            return;
        }
        if spec.sidecars.mhl {
            let written = {
                let entries: Vec<mhl::NewEntry> = verified
                    .iter()
                    .filter(|&&i| !in_mhl_folder(&job.plan.files[i].dest_rel))
                    .map(|&i| {
                        let f = &job.plan.files[i];
                        mhl::NewEntry {
                            path: format::rel_path(&f.dest_rel),
                            size: f.size,
                            modified: f.modified,
                            hashes: self.files[i].hashes.as_ref().expect("filtered above"),
                            hashed_at: self.stamps[i][d].unwrap_or_else(SystemTime::now),
                            from_earlier_run: self.pre_verified(i, d),
                        }
                    })
                    .collect();
                mhl::write_generation(root, &entries, &ignore_patterns(spec))
            };
            match written {
                Ok(Some(p)) => {
                    self.note(&format!("Sidecar      {}", p.display()));
                    self.sidecars.push(p);
                }
                Ok(None) => {}
                Err(e) => self.warn(format!("Couldn't write the ASC MHL manifest in {}: {e}", root.display())),
            }
        }
        let complete = verified.len() == job.plan.files.len() && job.plan.problems.is_empty();
        let has_list = fs::read_dir(root).into_iter().flatten().flatten().any(|e| {
            let n = e.file_name().to_string_lossy().to_ascii_lowercase();
            n.starts_with("checksums") && n.ends_with(".md5")
        });
        if spec.sidecars.md5_list && spec.hash.md5 && complete && !has_list {
            let entries: Vec<(String, String)> = verified
                .iter()
                .filter_map(|&i| {
                    let md5 = self.files[i].hashes.and_then(|h| h.md5_hex())?;
                    Some((format::rel_path(&job.plan.files[i].dest_rel), md5))
                })
                .collect();
            match mhl::write_md5_list(root, &entries) {
                Ok(p) => {
                    self.note(&format!("Sidecar      {}", p.display()));
                    self.sidecars.push(p);
                }
                Err(e) => self.warn(format!("Couldn't write the .md5 list in {}: {e}", root.display())),
            }
        }
    }
}

/// A file inside an `ascmhl` history that came off the source: copied and verified like
/// any file, but not listed in our own manifest (MHL ignores its own folder).
fn in_mhl_folder(rel: &Path) -> bool {
    rel.components().next().is_some_and(|c| c.as_os_str().eq_ignore_ascii_case(mhl::FOLDER))
}

/// What MHL tools should ignore at a destination: MHL's defaults, our `.md5` lists, and
/// the OS housekeeping we skipped.
fn ignore_patterns(spec: &JobSpec) -> Vec<String> {
    let mut p: Vec<String> =
        [".DS_Store", "ascmhl", "ascmhl/", "checksums*.md5"].iter().map(|s| s.to_string()).collect();
    if !spec.include_system_files {
        for s in scan::system_patterns() {
            if !p.contains(&s) {
                p.push(s);
            }
        }
    }
    p
}
