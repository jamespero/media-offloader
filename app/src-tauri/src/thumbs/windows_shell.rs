//! Thumbnails from the Windows Shell.

use std::path::Path;

use super::SIZE;

pub(super) fn init_thread() {
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
    // Shell thumbnail providers expect a single-threaded apartment.
    // SAFETY: called once on this dedicated thread before any COM use.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
}

/// Windows: ask the Shell for the file's thumbnail (the same one Explorer shows).
pub(super) fn extract(path: &Path) -> Option<(&'static str, Vec<u8>)> {
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

fn encode_jpeg(w: u16, h: u16, rgb: &[u8]) -> Option<(&'static str, Vec<u8>)> {
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, 82).encode(rgb, w, h, jpeg_encoder::ColorType::Rgb).ok()?;
    Some(("image/jpeg", out))
}

#[cfg(test)]
mod tests {
    #[test]
    fn encodes_rgb_as_a_jpeg_of_the_same_size() {
        let (mime, bytes) = super::encode_jpeg(16, 8, &[128; 16 * 8 * 3]).unwrap();
        assert_eq!(mime, "image/jpeg");
        assert_eq!(bytes[..2], [0xFF, 0xD8], "start of image");
        assert_eq!(bytes[bytes.len() - 2..], [0xFF, 0xD9], "end of image");
    }
}
