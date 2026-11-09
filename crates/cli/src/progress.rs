//! The progress line shown during a copy.

use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant};

use offloader_core::{Event, FileStatus, Job, Pass, Stage, format};

/// Renders a single updating progress line on stderr.
pub struct Progress {
    names: Vec<String>,
    tty: bool,
    pass: Option<Pass>,
    file: usize,
    stage: Stage,
    done: u64,
    total: u64,
    samples: Vec<(Instant, u64)>,
    last_draw: Instant,
}

impl Progress {
    pub fn new(job: &Job) -> Self {
        Self {
            names: job.plan().files.iter().map(|f| format::rel_path(&f.dest_rel)).collect(),
            tty: io::stderr().is_terminal(),
            pass: None,
            file: 0,
            stage: Stage::Copy,
            done: 0,
            total: 0,
            samples: Vec::new(),
            last_draw: Instant::now(),
        }
    }

    fn clear(&self) {
        if self.tty {
            eprint!("\r{:<110}\r", "");
        }
    }

    pub fn on_event(&mut self, e: &Event) {
        match e {
            Event::PassStarted { pass, files, bytes, .. } => {
                self.pass = Some(*pass);
                self.samples.clear();
                let what = match pass {
                    Pass::Copy => "Copying",
                    Pass::Backup => "Backing up from the verified copy",
                    Pass::FanOut => "Copying to every destination at once",
                };
                eprintln!("{what}: {files} files, {}", format::size(*bytes));
            }
            Event::Progress { file, stage, done, total, .. } => {
                (self.file, self.stage, self.done, self.total) = (*file, *stage, *done, *total);
                let now = Instant::now();
                self.samples.push((now, *done));
                self.samples.retain(|(t, _)| now.duration_since(*t) < Duration::from_secs(4));
                if now.duration_since(self.last_draw) >= Duration::from_millis(120) {
                    self.last_draw = now;
                    self.draw();
                }
            }
            Event::FileFinished { file, results, .. } => {
                for (_, status) in results {
                    if let FileStatus::Failed { failure } = status {
                        self.clear();
                        eprintln!("FAIL  {}: {failure}", self.names[*file]);
                    }
                }
            }
            Event::SourceReleased => {
                self.clear();
                eprintln!("✓ Copy verified. The source can be removed.");
            }
            Event::PassFinished { verified, failed, .. } => {
                self.clear();
                eprintln!("  {verified} verified, {failed} failed");
            }
            Event::Warning { message } => {
                self.clear();
                eprintln!("warning: {message}");
            }
            Event::FileStarted { .. } | Event::Finished { .. } => {}
        }
    }

    fn draw(&self) {
        if !self.tty || self.total == 0 {
            return;
        }
        let pct = self.done as f64 * 100.0 / self.total as f64;
        let (rate, eta) = match (self.samples.first(), self.samples.last()) {
            (Some((t0, d0)), Some((t1, d1))) if t1 > t0 && d1 > d0 => {
                let r = (d1 - d0) as f64 / t1.duration_since(*t0).as_secs_f64();
                let left = Duration::from_secs_f64((self.total - self.done) as f64 / r);
                (format::rate(r), format!("{} left", format::duration(left)))
            }
            _ => (String::new(), String::new()),
        };
        let stage = match self.stage {
            Stage::Copy => "copying  ",
            Stage::Verify => "verifying",
        };
        let mut name = self.names.get(self.file).cloned().unwrap_or_default();
        if name.chars().count() > 44 {
            name =
                format!("…{}", name.chars().rev().take(43).collect::<Vec<_>>().into_iter().rev().collect::<String>());
        }
        eprint!("\r{pct:5.1}%  {stage}  {name:<45} {rate:>10}  {eta:<12}");
        let _ = io::stderr().flush();
    }
}
