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

/// Longest side of a rendered PDF page on the canvas, in pixels.
const CANVAS_PDF_EDGE: f64 = 2400.0;

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
/// A PDF picture is its first page rendered (B15); an EPS picture is
/// ghostscript's rendering when `gs` is installed, else its preview image,
/// else a box. Exports use the files themselves.
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
        // An EPS file: ghostscript's rendering where it is installed (B15),
        // else the preview the file carries; exports embed the file itself.
        Ok(crate::fig2dev::LoadedPicture::Eps { ps, preview, .. }) => canvas_eps(path, &ps, preview),
        // A PDF file: its first page, rendered (exports keep it a vector).
        Ok(crate::fig2dev::LoadedPicture::Pdf(Some(pdf))) => canvas_pdf(path, &pdf),
        Ok(crate::fig2dev::LoadedPicture::Pdf(None)) => Err(format!("{}: PDF picture can't be read", path.display())),
        Ok(crate::fig2dev::LoadedPicture::Jpeg { raster: None, .. }) | Err(_) => load_picture_direct(path),
    }
}

/// The canvas's pixels for a PDF page: up to 3 pixels per point and
/// `CANVAS_PDF_EDGE` on the longer side.
fn canvas_pdf(path: &Path, pdf: &crate::fig2dev::PdfPicture) -> Result<RasterImage, String> {
    let (w, h) = pdf.size();
    let s = 3.0f64.min(CANVAS_PDF_EDGE / w.max(h));
    pdf.render((w * s).ceil() as u32, (h * s).ceil() as u32)
        .ok_or_else(|| format!("{}: PDF picture can't be drawn", path.display()))
}

/// The canvas's pixels for an EPS file (native: ghostscript's, when it is
/// installed; else the file's preview image).
fn canvas_eps(path: &Path, ps: &[u8], preview: Option<RasterImage>) -> Result<RasterImage, String> {
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(pdf) = crate::fig2dev::gs_eps_to_pdf(ps) {
        if let Some(p) = crate::fig2dev::PdfPicture::open(pdf.to_vec()) {
            return canvas_pdf(path, &p);
        }
    }
    let _ = ps;
    preview.ok_or_else(|| {
        format!(
            "{}: EPS picture without preview shown as a box (embedded in exports)",
            path.display()
        )
    })
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

    /// A one-page PDF (100 x 50 pt) with a red rectangle over the left half.
    fn tiny_pdf() -> Vec<u8> {
        let content = b"1 0 0 rg 0 0 50 50 re f";
        let objs: Vec<Vec<u8>> = vec![
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] /Contents 4 0 R /Resources << >> >>".to_vec(),
            [
                format!("<< /Length {} >>\nstream\n", content.len()).as_bytes(),
                content,
                b"\nendstream",
            ]
            .concat(),
        ];
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offs = Vec::new();
        for (i, o) in objs.iter().enumerate() {
            offs.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            out.extend_from_slice(o);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        for o in &offs {
            out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
        out
    }

    #[test]
    fn pdf_picture_is_its_page_on_the_canvas() {
        let path = scratch("page.pdf");
        std::fs::write(&path, tiny_pdf()).unwrap();
        let img = load_picture(&path).unwrap();
        assert!(img.width >= 300 && img.height >= 150, "{}x{}", img.width, img.height);
        let at = |x: u32, y: u32| {
            let i = ((y * img.width + x) * 4) as usize;
            [img.rgba[i], img.rgba[i + 1], img.rgba[i + 2], img.rgba[i + 3]]
        };
        assert_eq!(at(img.width / 4, img.height / 2), [255, 0, 0, 255]);
        assert_eq!(at(img.width * 3 / 4, img.height / 2)[3], 0);
        // a file that is only a PDF's start is an error, not a panic
        let bad = scratch("bad.pdf");
        std::fs::write(&bad, b"%PDF-1.4\n/MediaBox [0 0 5 5]\n").unwrap();
        assert!(load_picture(&bad).is_err());
    }

    #[test]
    fn eps_picture_is_drawn_through_ghostscript_or_its_preview() {
        let path = scratch("box.eps");
        std::fs::write(
            &path,
            "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 40 20\n0 0 1 setrgbcolor 0 0 40 20 rectfill\nshowpage\n",
        )
        .unwrap();
        match load_picture(&path) {
            Ok(img) => {
                let i = (((img.height / 2) * img.width + img.width / 2) * 4) as usize;
                assert_eq!(&img.rgba[i..i + 3], &[0, 0, 255]);
            }
            // without ghostscript (and a preview) it is a box, with a reason
            Err(e) => assert!(e.contains("EPS picture"), "{e}"),
        }
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
