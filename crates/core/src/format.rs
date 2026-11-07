//! Human-readable sizes, rates and durations, shared by the log, the CLI and the app.
//! Sizes use decimal units (1 GB = 10^9 bytes), the way cameras and drive labels do.

use std::path::Path;
use std::time::Duration;

const KB: f64 = 1e3;
const MB: f64 = 1e6;
const GB: f64 = 1e9;
const TB: f64 = 1e12;

/// "45.1 GB", "454 MB", "12 KB", "7 B".
pub fn size(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= TB {
        format!("{:.2} TB", b / TB)
    } else if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.0} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// "1.18 GB/s", "85 MB/s".
pub fn rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= GB {
        format!("{:.2} GB/s", bytes_per_sec / GB)
    } else if bytes_per_sec >= MB {
        format!("{:.0} MB/s", bytes_per_sec / MB)
    } else {
        format!("{:.0} KB/s", bytes_per_sec / KB)
    }
}

/// "1h 02m", "2m 10s", "8s".
pub fn duration(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{}h {:02}m", s / 3600, (s % 3600) / 60)
    } else if s >= 60 {
        format!("{}m {:02}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}

/// A relative path with `/` separators on every platform (logs, MHL, the UI).
pub fn rel_path(p: &Path) -> String {
    let mut out = String::new();
    for c in p.components() {
        if !out.is_empty() {
            out.push('/');
        }
        out.push_str(&c.as_os_str().to_string_lossy());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(999), "999 B");
        assert_eq!(size(12_400), "12 KB");
        assert_eq!(size(454_100_000), "454 MB");
        assert_eq!(size(45_120_000_000), "45.1 GB");
    }

    #[test]
    fn durations() {
        assert_eq!(duration(Duration::from_secs(8)), "8s");
        assert_eq!(duration(Duration::from_secs(130)), "2m 10s");
        assert_eq!(duration(Duration::from_secs(3720)), "1h 02m");
    }

    #[test]
    fn rel_paths_use_forward_slashes() {
        let p: std::path::PathBuf = ["DCIM", "100NCZ_R", "DSC_0001.NEF"].iter().collect();
        assert_eq!(rel_path(&p), "DCIM/100NCZ_R/DSC_0001.NEF");
    }
}
