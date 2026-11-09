//! The plain-text log of a re-verify.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::Local;

use super::{CheckStatus, ManifestSource, ReverifyOutcome, ReverifyReport};
use crate::format;
use crate::{APP_NAME, VERSION};

pub(super) fn write_log(
    dir: &Path,
    r: &ReverifyReport,
    started: chrono::DateTime<Local>,
    elapsed: std::time::Duration,
) -> io::Result<PathBuf> {
    use std::io::Write;
    fs::create_dir_all(dir)?;
    let name = r.root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "folder".into());
    let safe: String = name.chars().map(|c| if c.is_alphanumeric() || " -_.".contains(c) { c } else { '_' }).collect();
    let mut path = dir.join(format!("{} re-verify {safe}.log", started.format("%Y-%m-%d %H%M%S")));
    let mut n = 1;
    while path.exists() {
        n += 1;
        path = dir.join(format!("{} re-verify {safe} ({n}).log", started.format("%Y-%m-%d %H%M%S")));
    }
    let mut f = io::BufWriter::new(fs::OpenOptions::new().write(true).create_new(true).open(&path)?);
    writeln!(f, "{APP_NAME} {VERSION} — re-verify log")?;
    writeln!(f, "Started      {}", started.format("%Y-%m-%d %H:%M:%S %:z"))?;
    writeln!(f, "Folder       {}", r.root.display())?;
    match &r.source {
        ManifestSource::Mhl { generations } => {
            writeln!(f, "Checked by   ASC MHL history ({generations} generation(s))")?
        }
        ManifestSource::Md5List { file } => writeln!(f, "Checked by   {file}")?,
    }
    writeln!(f, "Reads        bypass the OS cache")?;
    for p in &r.manifest_problems {
        writeln!(f, "MANIFEST     {p}")?;
    }
    writeln!(f)?;
    for file in &r.files {
        let size = file.size.map(format::size).unwrap_or_default();
        match &file.status {
            CheckStatus::Verified => writeln!(f, "OK    {}  {size}", file.path)?,
            CheckStatus::Pending => writeln!(f, "--    {}  not checked", file.path)?,
            CheckStatus::Missing => writeln!(f, "FAIL  {}  — missing", file.path)?,
            CheckStatus::SizeDiffers { expected, actual } => {
                writeln!(f, "FAIL  {}  — size is {actual} bytes, manifest says {expected}", file.path)?
            }
            CheckStatus::Mismatch { format, expected, actual } => {
                writeln!(f, "FAIL  {}  — {} is {actual}, manifest says {expected}", file.path, format.name())?
            }
            CheckStatus::ReadError { message } => writeln!(f, "FAIL  {}  — couldn't read: {message}", file.path)?,
        }
    }
    for p in &r.not_listed {
        writeln!(f, "NEW   {p}  — not in any manifest")?;
    }
    writeln!(f)?;
    writeln!(f, "Finished     {}  ({})", Local::now().format("%Y-%m-%d %H:%M:%S %:z"), format::duration(elapsed))?;
    let result = match &r.outcome {
        ReverifyOutcome::Verified => format!("All {} files match their recorded checksums.", r.files.len()),
        ReverifyOutcome::Failed { files } => format!("{files} file(s) do NOT match their recorded checksums."),
        ReverifyOutcome::Cancelled => "Stopped before every file was checked.".to_string(),
    };
    writeln!(f, "Result       {result}")?;
    f.flush()?;
    Ok(path)
}
