//! File-manager thumbnails: a figure fitted into N x N pixels as a PNG.
//!
//! Used by `rustfig-cli thumbnail` and `rustfig thumbnail`, which the
//! freedesktop thumbnailer entry (`rustfig.thumbnailer`) runs. Reads what
//! `file.open` reads (`.fig`, compressed `.fig.gz`/`.Z`/`.bz2`/`.xz`,
//! `.rustfig`).

use crate::Engine;
use serde_json::json;

/// Largest thumbnail edge accepted, in pixels.
pub const MAX_SIZE: u32 = 4096;
/// Default thumbnail edge, in pixels.
pub const DEFAULT_SIZE: u32 = 256;

/// Width and height from a PNG's IHDR chunk.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.get(..8)? != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let w = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let h = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    Some((w, h))
}

/// Render `input` to `output` (PNG, white background) so that its larger
/// side is `size` pixels (the figure is never cropped).
pub fn make(input: &str, output: &str, size: u32) -> Result<(), String> {
    if !(1..=MAX_SIZE).contains(&size) {
        return Err(format!("size must be 1 to {MAX_SIZE}"));
    }
    let mut e = Engine::new();
    e.execute("file.open", &json!({ "path": input }))
        .map_err(|e| e.message)?;
    let params = |dpi: f64| json!({ "path": output, "format": "png", "dpi": dpi, "background": "#ffffff" });
    // First pass at 80 dpi (one pixel per 1/80 inch) to learn the size.
    e.execute("file.export", &params(80.0)).map_err(|e| e.message)?;
    let bytes = std::fs::read(output).map_err(|e| format!("{output}: {e}"))?;
    let (w, h) = png_size(&bytes).ok_or("the export is not a PNG")?;
    let big = w.max(h);
    if big == 0 {
        return Err("empty drawing".into());
    }
    if big != size {
        let dpi = (80.0 * f64::from(size) / f64::from(big)).clamp(0.5, 100_000.0);
        e.execute("file.export", &params(dpi)).map_err(|e| e.message)?;
        // Rounding may overshoot by a pixel: retry once, slightly smaller.
        let bytes = std::fs::read(output).map_err(|e| format!("{output}: {e}"))?;
        if let Some((w2, h2)) = png_size(&bytes) {
            if w2.max(h2) > size {
                let d2 = dpi * f64::from(size) / f64::from(w2.max(h2)) * 0.99;
                e.execute("file.export", &params(d2.max(0.5))).map_err(|e| e.message)?;
            }
        }
    }
    Ok(())
}

/// [`make`] returning the PNG bytes (for the macOS Quick Look extensions, which hand the
/// image to the system). The temporary file is removed.
pub fn make_png(input: &std::path::Path, size: u32) -> Result<Vec<u8>, String> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let tmp = std::env::temp_dir().join(format!(
        "rustfig-thumb-{}-{}.png",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let (Some(i), Some(o)) = (input.to_str(), tmp.to_str()) else {
        return Err("the path is not valid text".into());
    };
    let r = make(i, o, size).and_then(|()| std::fs::read(&tmp).map_err(|e| e.to_string()));
    let _ = std::fs::remove_file(&tmp); // best effort: a stray temp file is harmless
    r
}

/// `thumbnail INPUT OUTPUT.png [--size N]` (the arguments after `thumbnail`).
pub fn run(args: &[String]) -> Result<(), String> {
    const USAGE: &str = "usage: thumbnail INPUT OUTPUT.png [--size N]";
    let mut files = Vec::new();
    let mut size = DEFAULT_SIZE;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--size" || a == "-s" {
            size = it.next().and_then(|v| v.parse().ok()).ok_or("--size needs a number")?;
        } else {
            files.push(a.as_str());
        }
    }
    let [input, output] = files[..] else {
        return Err(USAGE.into());
    };
    make(input, output, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_fits_size() {
        let dir = std::env::temp_dir().join(format!("rf-thumb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fig = format!("{}/tests/fixtures/boxes.fig", env!("CARGO_MANIFEST_DIR"));
        for size in [64u32, 256] {
            let out = dir.join(format!("t{size}.png"));
            make(&fig, out.to_str().unwrap(), size).unwrap();
            let (w, h) = png_size(&std::fs::read(&out).unwrap()).unwrap();
            assert!(w.max(h) <= size && w.max(h) >= size - 2, "{w}x{h} for {size}");
        }
        assert!(make(&fig, "x.png", 0).is_err());
        assert!(run(&["only-one".into()]).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
