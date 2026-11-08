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

#[cfg(windows)]
fn init_thread() {
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
    // Shell thumbnail providers expect a single-threaded apartment.
    // SAFETY: called once on this dedicated thread before any COM use.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
}

#[cfg(not(windows))]
fn init_thread() {}

/// Windows: ask the Shell for the file's thumbnail (the same one Explorer shows).
#[cfg(windows)]
fn extract(path: &Path) -> Option<(&'static str, Vec<u8>)> {
    use windows::Win32::Foundation::SIZE as WSIZE;
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject,
        GetDIBits, GetObjectW,
    };
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_THUMBNAILONLY,
    };
    use windows::core::HSTRING;

    // SAFETY: COM was initialized on this thread; every handle is released below.
    unsafe {
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None).ok()?;
        let size = WSIZE { cx: SIZE as i32, cy: SIZE as i32 };
        let hbmp = factory.GetImage(size, SIIGBF_THUMBNAILONLY | SIIGBF_BIGGERSIZEOK).ok()?;

        let mut bm = BITMAP::default();
        let got = GetObjectW(hbmp.into(), std::mem::size_of::<BITMAP>() as i32, Some((&mut bm as *mut BITMAP).cast()));
        let (w, h) = (bm.bmWidth, bm.bmHeight);
        let mut bgra = vec![0u8; (w.max(0) * h.max(0) * 4) as usize];
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let dc = CreateCompatibleDC(None);
        let lines = if got != 0 && w > 0 && h > 0 {
            GetDIBits(dc, hbmp, 0, h as u32, Some(bgra.as_mut_ptr().cast()), &mut info, DIB_RGB_COLORS)
        } else {
            0
        };
        let _ = DeleteDC(dc);
        let _ = DeleteObject(hbmp.into());
        if lines == 0 {
            return None;
        }

        // Premultiplied BGRA (or alpha all zero, meaning opaque) → RGB over the tile colour.
        let opaque = bgra.as_chunks::<4>().0.iter().all(|p| p[3] == 0);
        let bg = [0x1a_u32, 0x19, 0x20];
        let mut rgb = Vec::with_capacity((w * h * 3) as usize);
        for p in bgra.as_chunks::<4>().0 {
            let a = if opaque { 255 } else { u32::from(p[3]) };
            for (c, b) in [p[2], p[1], p[0]].into_iter().zip(bg) {
                rgb.push((u32::from(c) + b * (255 - a) / 255).min(255) as u8);
            }
        }
        encode_jpeg(w as u16, h as u16, &rgb)
    }
}

/// How long QuickLook gets per file. Some formats (e.g. R3D with no QuickLook plugin)
/// make `qlmanage` hang forever, which would stall every thumbnail queued behind it.
#[cfg(target_os = "macos")]
const QL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// macOS: QuickLook's command-line thumbnailer (same previews as Finder).
#[cfg(target_os = "macos")]
fn extract(path: &Path) -> Option<(&'static str, Vec<u8>)> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let dir = std::env::temp_dir().join(format!("media-offloader-thumbs-{}", std::process::id()));
    fs::create_dir_all(&dir).ok()?;
    let mut child = Command::new("qlmanage")
        .args(["-t", "-s", &SIZE.to_string(), "-o"])
        .arg(&dir)
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + QL_TIMEOUT;
    let ok = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            _ => {
                // Hung or unwaitable: give up on this file (it's remembered as "none").
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let out = dir.join(format!("{}.png", path.file_name()?.to_string_lossy()));
    let bytes = fs::read(&out).ok();
    let _ = fs::remove_file(&out);
    bytes.filter(|_| ok).map(|b| ("image/png", b))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn extract(_path: &Path) -> Option<(&'static str, Vec<u8>)> {
    None
}

#[cfg(windows)]
fn encode_jpeg(w: u16, h: u16, rgb: &[u8]) -> Option<(&'static str, Vec<u8>)> {
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, 82).encode(rgb, w, h, jpeg_encoder::ColorType::Rgb).ok()?;
    Some(("image/jpeg", out))
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn encodes_rgb_as_a_jpeg_of_the_same_size() {
        let (mime, bytes) = super::encode_jpeg(16, 8, &[128; 16 * 8 * 3]).unwrap();
        assert_eq!(mime, "image/jpeg");
        assert_eq!(bytes[..2], [0xFF, 0xD8], "start of image");
        assert_eq!(bytes[bytes.len() - 2..], [0xFF, 0xD9], "end of image");
    }

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
