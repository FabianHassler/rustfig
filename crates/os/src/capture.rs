//! Screen capture (D11, replaces xfig's `w_capture.c`): the system's own way to grab a window
//! or an area of the screen, as PNG bytes.
//!
//! - macOS: `/usr/sbin/screencapture -i -x` (the user picks a window or drags an area; Escape
//!   cancels). No unsafe.
//! - Linux/BSD: the xdg-desktop-portal Screenshot request, interactive (the desktop's own
//!   selection). No unsafe.
//! - Windows: the whole virtual screen through GDI (`BitBlt`); the caller hides its window first
//!   ([`needs_window_hidden`]) and the grab waits a moment for it to disappear.
//! - Web: not available.
//!
//! [`capture_region`] blocks until the user is done: call it on a thread.

/// What a capture needs from the caller.
pub const fn available() -> bool {
    !cfg!(target_arch = "wasm32")
}

/// Why the command is not available (empty when it works).
pub const fn unavailable_reason() -> &'static str {
    if available() {
        ""
    } else {
        "The browser cannot capture the screen from here; use the system's screenshot tool and Import Picture"
    }
}

/// The caller's own window has to step aside before the grab: the system selector does not
/// hide it (macOS, Linux) or there is none (Windows grabs the whole screen).
pub const fn needs_window_hidden() -> bool {
    available()
}

/// Largest image taken (pixels per side), so a bogus screen size cannot allocate without limit.
const MAX_EDGE: usize = 16_384;

/// Let the user pick a window or an area of the screen; the PNG bytes, or `None` when the user
/// cancelled. Blocks (macOS and Linux wait for the user, Windows for the window to hide).
pub fn capture_region() -> Result<Option<Vec<u8>>, String> {
    platform_capture()
}

#[cfg(target_arch = "wasm32")]
fn platform_capture() -> Result<Option<Vec<u8>>, String> {
    Err(unavailable_reason().to_string())
}

/// A file name in the temporary folder that no other capture of this process uses.
#[cfg(target_os = "macos")]
fn temp_png() -> std::path::PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    std::env::temp_dir().join(format!("rustfig-capture-{}-{stamp}.png", std::process::id()))
}

/// The PNG the capture program wrote; `None` when there is none or it is empty (cancelled).
/// The file is removed.
#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn take_file(path: &std::path::Path) -> Result<Option<Vec<u8>>, String> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    // The copy is ours now; a leftover temporary file is harmless if it cannot be removed.
    let _ = std::fs::remove_file(path);
    if bytes.is_empty() {
        return Ok(None);
    }
    if !is_png(&bytes) {
        return Err("The capture program did not write a PNG image".into());
    }
    Ok(Some(bytes))
}

/// Whether the bytes start with the PNG signature.
pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])
}

#[cfg(target_os = "macos")]
fn platform_capture() -> Result<Option<Vec<u8>>, String> {
    let path = temp_png();
    let out = std::process::Command::new("/usr/sbin/screencapture")
        // -i: the user picks an area (Space switches to a window); -x: no shutter sound.
        .args(["-i", "-x"])
        .arg(&path)
        .output()
        .map_err(|e| format!("Could not start screencapture: {e}"))?;
    if !out.status.success() {
        let _ = std::fs::remove_file(&path);
        let why = String::from_utf8_lossy(&out.stderr);
        let why = why.trim();
        return Err(if why.is_empty() {
            "screencapture failed (does RustFig have the Screen Recording permission?)".into()
        } else {
            format!("screencapture failed: {why}")
        });
    }
    take_file(&path)
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn platform_capture() -> Result<Option<Vec<u8>>, String> {
    let r = pollster::block_on(async {
        let req = ashpd::desktop::screenshot::Screenshot::request()
            .interactive(true)
            .modal(true)
            .send()
            .await?;
        req.response()
    });
    match r {
        Ok(shot) => {
            let path = shot.uri().to_file_path().map_err(|()| {
                format!(
                    "The portal gave a screenshot address that is not a file: {}",
                    shot.uri()
                )
            })?;
            take_file(&path)
        }
        Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)) => Ok(None),
        Err(ashpd::Error::PortalNotFound(_)) => {
            Err("Capturing needs xdg-desktop-portal with a Screenshot backend, which was not found".into())
        }
        Err(e) => Err(format!("The screenshot failed: {e}")),
    }
}

#[cfg(target_os = "windows")]
fn platform_capture() -> Result<Option<Vec<u8>>, String> {
    // The caller's window is minimising; its animation takes a moment.
    std::thread::sleep(std::time::Duration::from_millis(500));
    let (w, h, rgb) = win::grab_screen()?;
    encode_png_rgb(w, h, &rgb).map(Some)
}

#[cfg(target_os = "windows")]
mod win {
    use super::MAX_EDGE;
    use std::ffi::c_void;
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, SRCCOPY,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    };

    /// The whole virtual screen as width, height and RGB bytes.
    #[allow(unsafe_code)]
    pub fn grab_screen() -> Result<(usize, usize, Vec<u8>), String> {
        // SAFETY: plain Win32 calls; every handle is checked and released, the bitmap info and the
        // pixel buffer are valid for the sizes passed (buffer = width * height * 4 bytes).
        unsafe {
            let (x, y) = (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN));
            let (w, h) = (
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            );
            if w <= 0 || h <= 0 || w as usize > MAX_EDGE || h as usize > MAX_EDGE {
                return Err("The screen size is not usable".into());
            }
            let screen = GetDC(std::ptr::null_mut());
            if screen.is_null() {
                return Err("Could not read the screen".into());
            }
            let mem = CreateCompatibleDC(screen);
            let bmp = CreateCompatibleBitmap(screen, w, h);
            let result = if mem.is_null() || bmp.is_null() {
                Err("Could not make a bitmap for the screen".to_string())
            } else {
                let old = SelectObject(mem, bmp);
                let mut out = Err("Could not copy the screen".to_string());
                if BitBlt(mem, 0, 0, w, h, screen, x, y, SRCCOPY | CAPTUREBLT) != 0 {
                    let mut info: BITMAPINFO = std::mem::zeroed();
                    info.bmiHeader = BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: w,
                        biHeight: -h, // top-down
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB,
                        biSizeImage: 0,
                        biXPelsPerMeter: 0,
                        biYPelsPerMeter: 0,
                        biClrUsed: 0,
                        biClrImportant: 0,
                    };
                    let (wu, hu) = (w as usize, h as usize);
                    let mut bgra = vec![0u8; wu * hu * 4];
                    let lines = GetDIBits(
                        mem,
                        bmp,
                        0,
                        h as u32,
                        bgra.as_mut_ptr().cast::<c_void>(),
                        &mut info,
                        DIB_RGB_COLORS,
                    );
                    if lines != 0 {
                        let rgb = bgra.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0]]).collect();
                        out = Ok((wu, hu, rgb));
                    }
                }
                SelectObject(mem, old);
                out
            };
            if !bmp.is_null() {
                DeleteObject(bmp);
            }
            if !mem.is_null() {
                DeleteDC(mem);
            }
            ReleaseDC(std::ptr::null_mut(), screen);
            result
        }
    }
}

/// An 8-bit RGB image as a PNG (uncompressed deflate blocks: no compression library needed; the
/// file is only an intermediate the user's figure links to). Errors for sizes that do not match.
#[cfg_attr(not(any(target_os = "windows", test)), allow(dead_code))]
pub fn encode_png_rgb(width: usize, height: usize, rgb: &[u8]) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 || width > MAX_EDGE || height > MAX_EDGE || rgb.len() != width * height * 3 {
        return Err("The image size is not usable".into());
    }
    // Scanlines: filter byte 0 + the row.
    let row = width * 3;
    let mut raw = Vec::with_capacity(height * (row + 1));
    for line in rgb.chunks_exact(row) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    // zlib: header, stored blocks of at most 65535 bytes, Adler-32.
    let mut z = vec![0x78, 0x01];
    let mut blocks = raw.chunks(65_535).peekable();
    while let Some(b) = blocks.next() {
        z.push(u8::from(blocks.peek().is_none()));
        let n = b.len() as u16;
        z.extend_from_slice(&n.to_le_bytes());
        z.extend_from_slice(&(!n).to_le_bytes());
        z.extend_from_slice(b);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 bit, truecolour, deflate, no filter, no interlace
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &z);
    chunk(&mut png, b"IEND", &[]);
    Ok(png)
}

#[cfg_attr(not(any(target_os = "windows", test)), allow(dead_code))]
fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

#[cfg_attr(not(any(target_os = "windows", test)), allow(dead_code))]
fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &d in data {
        a = (a + u32::from(d)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

#[cfg_attr(not(any(target_os = "windows", test)), allow(dead_code))]
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for &d in data {
        crc ^= u32::from(d);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_and_adler_known_values() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn encoded_png_has_the_signature_and_chunks() {
        let png = encode_png_rgb(2, 1, &[255, 0, 0, 0, 255, 0]).unwrap();
        assert!(is_png(&png));
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }

    #[test]
    fn encoder_rejects_bad_sizes() {
        assert!(encode_png_rgb(0, 1, &[]).is_err());
        assert!(encode_png_rgb(2, 2, &[0; 5]).is_err());
        assert!(encode_png_rgb(MAX_EDGE + 1, 1, &[]).is_err());
    }

    #[test]
    fn large_images_use_several_blocks() {
        let png = encode_png_rgb(300, 300, &vec![7u8; 300 * 300 * 3]).unwrap();
        assert!(is_png(&png));
    }

    #[test]
    fn availability_and_reason_agree() {
        assert_eq!(available(), unavailable_reason().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn missing_or_empty_file_means_cancelled() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let dir = std::env::temp_dir();
            let p = dir.join(format!("rustfig-capture-test-{}-none.png", std::process::id()));
            assert_eq!(take_file(&p).unwrap(), None);
            std::fs::write(&p, b"").unwrap();
            assert_eq!(take_file(&p).unwrap(), None);
            std::fs::write(&p, b"not a png").unwrap();
            assert!(take_file(&p).is_err());
            let _ = std::fs::remove_file(&p);
        }
    }
}
