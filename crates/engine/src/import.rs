//! Loading imported pictures (PNG, JPEG, GIF, BMP, PNM, TIFF; through
//! fig2dev's `readpics.c` also XBM, XPM, PCX and compressed files).

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

/// Read and decode a picture file as fig2dev's `readpics` does (PNG
/// gamma and transparency, GIF transparent colour, XBM, XPM, PCX, `name.gz`
/// for `name`), so the canvas shows the pixels the exports show; what
/// `readpics` refuses (BMP, palette TIFF) goes to the `image` crate.
/// EPS and PDF pictures have no pixels: they are shown as a box (and
/// embedded in EPS exports).
pub fn load_picture(path: &Path) -> Result<RasterImage, String> {
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > MAX_PICTURE_BYTES {
            return Err(format!("{}: file too large", path.display()));
        }
    }
    match crate::fig2dev::load_picture(path) {
        Ok(crate::fig2dev::LoadedPicture::Raster(img))
        | Ok(crate::fig2dev::LoadedPicture::Jpeg { raster: Some(img), .. }) => {
            if img.width > MAX_PICTURE_EDGE || img.height > MAX_PICTURE_EDGE {
                return Err(format!(
                    "{}: picture size {}x{} is not supported",
                    path.display(),
                    img.width,
                    img.height
                ));
            }
            Ok(img)
        }
        Ok(crate::fig2dev::LoadedPicture::Eps { .. }) => Err(format!(
            "{}: EPS picture shown as a box (embedded in EPS exports)",
            path.display()
        )),
        Ok(crate::fig2dev::LoadedPicture::Pdf) => Err(format!("{}: PDF picture shown as a box", path.display())),
        Ok(crate::fig2dev::LoadedPicture::Jpeg { raster: None, .. }) | Err(_) => load_picture_direct(path),
    }
}

fn load_picture_direct(path: &Path) -> Result<RasterImage, String> {
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

    fn crc32(data: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for &b in data {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
        }
        !c
    }

    fn encode(img: &image::RgbaImage, f: image::ImageFormat) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, f).unwrap();
        out.into_inner()
    }

    /// The picture `readpics` reads, as the exports see it.
    fn via_readpics(path: &Path) -> RasterImage {
        match crate::fig2dev::load_picture(path).unwrap() {
            crate::fig2dev::LoadedPicture::Raster(r) => r,
            _ => panic!("not a raster"),
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rustfig-d5-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d.join(name)
    }

    #[test]
    fn canvas_pixels_are_readpics_pixels() {
        let img = image::RgbaImage::from_fn(6, 4, |x, y| image::Rgba([(x * 40) as u8, (y * 60) as u8, 128, 255]));
        // PNG with a gAMA chunk after IHDR.
        let png = encode(&img, image::ImageFormat::Png);
        let mut chunk = Vec::from(*b"gAMA");
        chunk.extend_from_slice(&100_000u32.to_be_bytes());
        let mut g = Vec::new();
        g.extend_from_slice(&4u32.to_be_bytes());
        g.extend_from_slice(&chunk);
        g.extend_from_slice(&crc32(&chunk).to_be_bytes());
        let mut with_gamma = png[..33].to_vec();
        with_gamma.extend_from_slice(&g);
        with_gamma.extend_from_slice(&png[33..]);
        let p = scratch("gamma.png");
        std::fs::write(&p, &with_gamma).unwrap();
        assert_eq!(load_picture(&p).unwrap(), via_readpics(&p));
        let image_crate = decode_picture(&with_gamma).unwrap();
        eprintln!("gAMA png: image crate == readpics: {}", image_crate == via_readpics(&p));

        // GIF with a transparent colour.
        let gif = encode(
            &image::RgbaImage::from_fn(5, 5, |x, _| {
                image::Rgba([x as u8 * 50, 0, 0, if x == 0 { 0 } else { 255 }])
            }),
            image::ImageFormat::Gif,
        );
        let p = scratch("t.gif");
        std::fs::write(&p, &gif).unwrap();
        assert_eq!(load_picture(&p).unwrap(), via_readpics(&p));

        // XBM: only readpics reads it.
        let xbm = scratch("b.xbm");
        std::fs::write(
            &xbm,
            b"#define b_width 8\n#define b_height 2\nstatic unsigned char b_bits[] = {\n 0x0f, 0xf0 };\n",
        )
        .unwrap();
        let r = load_picture(&xbm).unwrap();
        assert_eq!((r.width, r.height), (8, 2));
        assert_eq!(r, via_readpics(&xbm));
    }

    #[test]
    fn bmp_falls_back_to_image() {
        let img = image::RgbaImage::from_pixel(3, 3, image::Rgba([1, 2, 3, 255]));
        let p = scratch("x.bmp");
        std::fs::write(&p, encode(&img, image::ImageFormat::Bmp)).unwrap();
        let r = load_picture(&p).unwrap();
        assert_eq!((r.width, r.height), (3, 3));
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(decode_picture(b"not an image").is_err());
        assert!(decode_picture(&[]).is_err());
    }
}
