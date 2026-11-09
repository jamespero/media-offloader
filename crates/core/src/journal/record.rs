//! Reading a journal back, and resuming from it.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{Header, Line, from_ms};
use crate::event::Outcome;
use crate::hash::Hashes;
use crate::job::{JobSpec, Resume, VerifiedFile};

/// A journal as read back.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub path: PathBuf,
    /// The latest run's header.
    pub header: Header,
    /// When the first run started.
    pub first_started: u64,
    pub runs: usize,
    /// Per destination: path → verified file (from every run).
    #[serde(skip)]
    pub verified: Vec<HashMap<String, VerifiedFile>>,
    /// Per destination: path → why it failed (latest run only).
    pub failed: Vec<HashMap<String, String>>,
    pub released: bool,
    /// `None` while running, or if the app stopped without finishing (crash, power loss).
    pub outcome: Option<Outcome>,
    pub finished: Option<u64>,
    pub sidecars: Vec<PathBuf>,
}

impl Record {
    pub fn read(path: &Path) -> io::Result<Record> {
        let reader = BufReader::new(File::open(path)?);
        let mut header: Option<Header> = None;
        let mut first_started = 0;
        let mut runs = 0;
        let mut verified: Vec<HashMap<String, VerifiedFile>> = Vec::new();
        let mut failed: Vec<HashMap<String, String>> = Vec::new();
        let (mut released, mut outcome, mut finished, mut sidecars) = (false, None, None, Vec::new());
        for line in reader.lines() {
            let Ok(line) = line else { break };
            // A torn last line (power loss mid-write) is simply ignored.
            let Ok(parsed) = serde_json::from_str::<Line>(&line) else { continue };
            match parsed {
                Line::Start(h) => {
                    if runs == 0 {
                        first_started = h.started;
                    }
                    runs += 1;
                    let n = h.roots.len();
                    verified.resize_with(n, HashMap::new);
                    failed = vec![HashMap::new(); n];
                    (released, outcome, finished) = (false, None, None);
                    header = Some(*h);
                }
                Line::Verified { dest, path, size, modified, xxh64, md5, at } => {
                    if let (Some(map), Some(hashes)) =
                        (verified.get_mut(dest), Hashes::from_hex(&xxh64, md5.as_deref()))
                    {
                        if let Some(f) = failed.get_mut(dest) {
                            f.remove(&path);
                        }
                        map.insert(
                            path,
                            VerifiedFile { size, modified: modified.map(from_ms), hashes, at: from_ms(at) },
                        );
                    }
                }
                Line::Failed { dest, path, reason } => {
                    if let Some(f) = failed.get_mut(dest) {
                        f.insert(path, reason);
                    }
                }
                Line::Released => released = true,
                Line::End { outcome: o, sidecars: s, at } => {
                    outcome = Some(o);
                    finished = Some(at);
                    sidecars.extend(s);
                }
            }
        }
        let header = header.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "not a journal"))?;
        Ok(Record {
            path: path.to_path_buf(),
            header,
            first_started,
            runs,
            verified,
            failed,
            released,
            outcome,
            finished,
            sidecars,
        })
    }

    pub fn verified_count(&self, dest: usize) -> usize {
        self.verified.get(dest).map_or(0, HashMap::len)
    }

    /// Paths verified on every destination, for the history view.
    pub fn verified_paths(&self) -> Vec<Vec<String>> {
        self.verified.iter().map(|m| m.keys().cloned().collect()).collect()
    }

    /// Whether there's anything left to do: interrupted, stopped, or files failed.
    pub fn resumable(&self) -> bool {
        !matches!(self.outcome, Some(Outcome::Verified))
    }

    /// A spec that continues this offload into the same folders: already verified files
    /// are kept, failed and missing ones are copied again, and the journal continues.
    pub fn resume_spec(&self, log_dir: Option<PathBuf>) -> JobSpec {
        let h = &self.header;
        JobSpec {
            source: h.source.clone(),
            copy_to: h.roots[0].clone(),
            backups: h.roots[1..].to_vec(),
            mode: h.mode,
            hash: h.hash,
            include_system_files: h.include_system_files,
            log_dir,
            sidecars: h.sidecars,
            resume: Some(Resume { roots: h.roots.clone(), verified: self.verified.clone() }),
            journal: Some(self.path.clone()),
            keep_awake: true,
            faults: None,
        }
    }
}

/// Every readable journal in `dir`, newest first.
pub fn list(dir: &Path) -> Vec<Record> {
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Record> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .filter_map(|p| Record::read(&p).ok())
        .collect();
    out.sort_by_key(|r| std::cmp::Reverse(r.first_started));
    out
}
