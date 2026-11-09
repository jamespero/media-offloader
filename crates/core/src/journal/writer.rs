//! Recording an offload as it runs.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{Header, JournalFile, Line, ms};
use crate::event::Outcome;
use crate::format;
use crate::hash::Hashes;
use crate::job::Job;

pub(crate) struct JournalWriter {
    file: File,
}

impl JournalWriter {
    pub fn open(path: &Path) -> io::Result<Self> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        Ok(Self { file: OpenOptions::new().create(true).append(true).open(path)? })
    }

    /// Best effort: a journal problem must never fail the offload itself.
    fn write(&mut self, line: &Line) {
        if let Ok(mut s) = serde_json::to_string(line) {
            s.push('\n');
            let _ = self.file.write_all(s.as_bytes()).and_then(|()| self.file.sync_data());
        }
    }

    pub fn start(&mut self, job: &Job, log: Option<&Path>) {
        let plan = job.plan();
        let spec = job.spec();
        let header = Header {
            started: ms(SystemTime::now()),
            source: plan.source.root.clone(),
            source_label: plan.source.label.clone(),
            source_kind: plan.source.kind,
            roles: job.destinations().iter().map(|d| d.role).collect(),
            requested: job.destinations().iter().map(|d| d.requested.clone()).collect(),
            roots: job.destinations().iter().map(|d| d.root.clone()).collect(),
            mode: spec.mode,
            hash: spec.hash,
            sidecars: spec.sidecars,
            include_system_files: spec.include_system_files,
            log: log.map(Path::to_path_buf),
            total_bytes: plan.total_bytes,
            files: plan
                .files
                .iter()
                .map(|f| JournalFile {
                    src: format::rel_path(&f.src_rel),
                    dest: format::rel_path(&f.dest_rel),
                    size: f.size,
                    modified: f.modified.map(ms),
                })
                .collect(),
        };
        self.write(&Line::Start(Box::new(header)));
    }

    pub fn verified(
        &mut self,
        dest: usize,
        path: String,
        size: u64,
        modified: Option<SystemTime>,
        hashes: &Hashes,
        at: SystemTime,
    ) {
        self.write(&Line::Verified {
            dest,
            path,
            size,
            modified: modified.map(ms),
            xxh64: hashes.xxh64_hex(),
            md5: hashes.md5_hex(),
            at: ms(at),
        });
    }

    pub fn failed(&mut self, dest: usize, path: String, reason: String) {
        self.write(&Line::Failed { dest, path, reason });
    }

    pub fn released(&mut self) {
        self.write(&Line::Released);
    }

    pub fn end(&mut self, outcome: &Outcome, sidecars: &[PathBuf]) {
        self.write(&Line::End { outcome: outcome.clone(), sidecars: sidecars.to_vec(), at: ms(SystemTime::now()) });
    }
}
