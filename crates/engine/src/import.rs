//! Loading imported pictures (PNG, JPEG, GIF, BMP, PNM, TIFF).

use rustfig_model::{Document, Object, ObjectKind, RasterImage};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Largest picture edge accepted, in pixels.
pub const MAX_PICTURE_EDGE: u32 = 16_384;
/// Largest picture file read, in bytes.
pub const MAX_PICTURE_BYTES: u64 = 256 << 20;

/// Decode image bytes.
pub fn decode_picture(bytes: &[u8]) -> Result<RasterImage, String> {
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
    if w == 0 || h == 0 || w > MAX_PICTURE_EDGE || h > MAX_PICTURE_EDGE {
        return Err(format!("picture size {w}x{h} is not supported"));
    }
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    Ok(RasterImage {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

/// Resolve a picture path relative to the document's directory.
pub fn resolve(file: &str, base: Option<&Path>) -> PathBuf {
    let p = PathBuf::from(file);
    match base {
        Some(b) if p.is_relative() => b.join(p),
        _ => p,
    }
}

/// Read and decode a picture file.
pub fn load_picture(path: &Path) -> Result<RasterImage, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.len() > MAX_PICTURE_BYTES {
        return Err(format!("{}: file too large", path.display()));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    decode_picture(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Load every picture in the document; returns messages for failures.
pub fn load_all_pictures(doc: &mut Document, base: Option<&Path>) -> Vec<String> {
    let mut errors = Vec::new();
    fn walk(objs: &mut [Object], base: Option<&Path>, errors: &mut Vec<String>) {
        for o in objs {
            match &mut o.kind {
                ObjectKind::Polyline(p) => {
                    if let Some(pic) = &mut p.picture {
                        if pic.image.is_none() && !pic.file.is_empty() {
                            match load_picture(&resolve(&pic.file, base)) {
                                Ok(img) => pic.image = Some(Arc::new(img)),
                                Err(e) => errors.push(e),
                            }
                        }
                    }
                }
                ObjectKind::Compound(c) => walk(&mut c.objects, base, errors),
                _ => {}
            }
        }
    }
    for o in &mut doc.objects {
        walk(std::slice::from_mut(std::sync::Arc::make_mut(o)), base, &mut errors);
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_is_an_error() {
        assert!(decode_picture(b"not an image").is_err());
        assert!(decode_picture(&[]).is_err());
    }
}
