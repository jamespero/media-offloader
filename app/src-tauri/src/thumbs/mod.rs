//! Thumbnails from the OS's own thumbnailers (Windows Shell, macOS QuickLook): the
//! broadest camera-format support with no codec work. Cached on disk by file name, size
//! and date (so a card file and its verified copy share an entry) and handed to the UI
//! as small data URLs. One worker, one file at a time; the UI stops it while the card is
//! being read so thumbnails never slow a copy.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[cfg(target_os = "macos")]
mod quicklook;
#[cfg(windows)]
mod windows_shell;

#[cfg(target_os = "macos")]
use quicklook::extract;
#[cfg(windows)]
use windows_shell::{extract, init_thread};

#[cfg(not(windows))]
fn init_thread() {}

#[cfg(not(any(windows, target_os = "macos")))]
fn extract(_path: &Path) -> Option<(&'static str, Vec<u8>)> {
    None
}

/// Longest side of a generated thumbnail, in pixels (tiles are ~170 px wide at 2×).
const SIZE: u32 = 320;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbRequest {
    /// Echoed back so the UI knows which tile it's for.
    pub key: String,
    pub path: PathBuf,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThumbReady {
    generation: u64,
    key: String,
    url: Option<String>,
}

pub struct Thumbs {
    tx: Mutex<Sender<(u64, Vec<ThumbRequest>)>>,
    generation: Arc<AtomicU64>,
}

impl Thumbs {
    pub fn start(app: AppHandle, cache: PathBuf) -> Self {
        let (tx, rx) = channel::<(u64, Vec<ThumbRequest>)>();
        let generation = Arc::new(AtomicU64::new(0));
        let current = generation.clone();
        std::thread::spawn(move || {
            init_thread();
            let _ = fs::create_dir_all(&cache);
            while let Ok((generation, items)) = rx.recv() {
                for item in items {
                    if current.load(Ordering::Relaxed) != generation {
                        break; // superseded by a newer request
                    }
                    let url = thumbnail(&cache, &item.path);
                    let _ = app.emit("thumb", ThumbReady { generation, key: item.key, url });
                }
            }
        });
        Self { tx: Mutex::new(tx), generation }
    }

    /// Replaces whatever is queued with `items`; an empty list just stops the worker.
    pub fn request(&self, generation: u64, items: Vec<ThumbRequest>) {
        self.generation.store(generation, Ordering::SeqCst);
        let _ = self.tx.lock().unwrap().send((generation, items));
    }
}

fn thumbnail(cache: &Path, path: &Path) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_millis();
    let name = path.file_name()?.to_string_lossy();
    let key = format!("{:016x}", xxhash_rust::xxh64::xxh64(format!("{name}|{}|{mtime}", meta.len()).as_bytes(), 0));
    for (ext, mime) in [("jpg", "image/jpeg"), ("png", "image/png")] {
        if let Ok(bytes) = fs::read(cache.join(format!("{key}.{ext}"))) {
            return Some(data_url(mime, &bytes));
        }
    }
    let none = cache.join(format!("{key}.none"));
    if none.exists() {
        return None; // known to have no thumbnail
    }
    match extract(path) {
        Some((mime, bytes)) => {
            let ext = if mime == "image/png" { "png" } else { "jpg" };
            let _ = fs::write(cache.join(format!("{key}.{ext}")), &bytes);
            Some(data_url(mime, &bytes))
        }
        None => {
            let _ = fs::write(none, b"");
            None
        }
    }
}

fn data_url(mime: &str, bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(bytes.len() * 4 / 3 + 32);
    s.push_str("data:");
    s.push_str(mime);
    s.push_str(";base64,");
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                s.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_known_vectors() {
        let enc = |s: &str| super::data_url("x", s.as_bytes()).trim_start_matches("data:x;base64,").to_string();
        assert_eq!(enc(""), "");
        assert_eq!(enc("f"), "Zg==");
        assert_eq!(enc("fo"), "Zm8=");
        assert_eq!(enc("foo"), "Zm9v");
        assert_eq!(enc("foobar"), "Zm9vYmFy");
    }
}
