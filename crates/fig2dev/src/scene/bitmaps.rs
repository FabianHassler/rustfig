//! fig2dev's bitmap languages (`-L gif|jpeg|pcx|ppm|tiff|xbm|xpm`) from
//! RustFig's own raster renderer, without ghostscript or netpbm.
//!
//! fig2dev's `genbitmaps.c` wrote EPS, rendered it with ghostscript at
//! 80 dpi times the magnification and piped the pixmap through netpbm
//! (`ppmtogif`, `pbmtoxbm`, `ppmtoxpm`, `ppmtopcx`, ...). Here the scene is
//! rasterised by [`super::raster`] (the size, border and smoothing follow
//! genbitmaps; see [`super::output`]) and encoded below: GIF, JPEG and TIFF
//! with the `image` crate, PPM, PCX, XBM and XPM by hand. Images without
//! an alpha channel are composed over white, as ghostscript's are.
//
// Ported from fig2dev dev/genbitmaps.c (xfig licence, see LICENSE-xfig).

use super::raster::Pixmap;
use super::ExportFormat;
use figbase::color::Rgb;
use std::collections::HashMap;
use std::io::Cursor;

/// fig2dev's JPEG quality without `-q`.
pub const DEFAULT_JPEG_QUALITY: u8 = 75;

/// Settings of the encoders.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Encoding {
    /// JPEG quality, 1 to 100.
    pub quality: u8,
    /// Pixels of exactly this colour are transparent in a GIF
    /// (`ppmtogif -transparent=`).
    pub transparent: Option<Rgb>,
    /// Resolution recorded in the file where the format has a field for it
    /// (PCX).
    pub dpi: f64,
}

impl Default for Encoding {
    fn default() -> Self {
        Encoding {
            quality: DEFAULT_JPEG_QUALITY,
            transparent: None,
            dpi: 80.0,
        }
    }
}

/// Encode a rendered pixmap in one of genbitmaps' languages.
pub fn encode(format: ExportFormat, pm: &Pixmap, enc: &Encoding) -> Result<Vec<u8>, String> {
    let (w, h) = (pm.width(), pm.height());
    let rgb = rgb_over_white(pm);
    match format {
        ExportFormat::Gif => gif(&rgb, w, h, enc.transparent),
        ExportFormat::Jpeg => {
            let mut out = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, enc.quality.clamp(1, 100))
                .encode(&rgb, w, h, image::ExtendedColorType::Rgb8)
                .map_err(|e| format!("JPEG: {e}"))?;
            Ok(out)
        }
        ExportFormat::Tiff => {
            let mut out = Cursor::new(Vec::new());
            image::ImageEncoder::write_image(
                image::codecs::tiff::TiffEncoder::new(&mut out),
                &rgb,
                w,
                h,
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| format!("TIFF: {e}"))?;
            Ok(out.into_inner())
        }
        ExportFormat::Ppm => Ok(ppm(&rgb, w, h)),
        ExportFormat::Pcx => pcx(&rgb, w, h, enc.dpi),
        ExportFormat::Xbm => Ok(xbm(&rgb, w, h).into_bytes()),
        ExportFormat::Xpm => Ok(xpm(&rgb, w, h).into_bytes()),
        other => Err(format!(
            "{} is not one of fig2dev's bitmap languages",
            other.extension()
        )),
    }
}

/// The pixmap's RGB bytes, composed over white.
pub fn rgb_over_white(pm: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pm.data().len() / 4 * 3);
    for &[r, g, b, a] in pm.data().as_chunks::<4>().0 {
        // Premultiplied colour over white: c + (1 − a) · 255.
        let rest = 255 - a;
        out.extend_from_slice(&[r.saturating_add(rest), g.saturating_add(rest), b.saturating_add(rest)]);
    }
    out
}

fn gif(rgb: &[u8], w: u32, h: u32, transparent: Option<Rgb>) -> Result<Vec<u8>, String> {
    let mut rgba = Vec::with_capacity(rgb.len() / 3 * 4);
    for p in rgb.as_chunks::<3>().0 {
        let clear = transparent.is_some_and(|t| *p == [t.r, t.g, t.b]);
        rgba.extend_from_slice(p);
        rgba.push(if clear { 0 } else { 255 });
    }
    let mut out = Vec::new();
    {
        // Speed 10: the gif crate's quantiser is slow at its best quality;
        // figures with at most 256 colours keep them exactly anyway.
        let mut e = image::codecs::gif::GifEncoder::new_with_speed(&mut out, 10);
        e.encode(&rgba, w, h, image::ExtendedColorType::Rgba8)
            .map_err(|err| format!("GIF: {err}"))?;
    }
    Ok(out)
}

/// netpbm's raw PPM (`P6`).
fn ppm(rgb: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    out.extend_from_slice(rgb);
    out
}

/// PCX version 5, 24 bits in three planes, run-length encoded (as
/// `ppmtopcx` writes images of more than 256 colours).
fn pcx(rgb: &[u8], w: u32, h: u32, dpi: f64) -> Result<Vec<u8>, String> {
    let (Ok(w16), Ok(h16)) = (u16::try_from(w), u16::try_from(h)) else {
        return Err("PCX images are at most 65535 pixels wide and high".into());
    };
    if w16 == 0 || h16 == 0 {
        return Err("image would be empty".into());
    }
    // Scan lines hold an even number of bytes.
    let bytes_per_line = w16.checked_add(w16 % 2).ok_or("PCX image too wide")?;
    let res = if dpi.is_finite() {
        dpi.round().clamp(1.0, 65535.0) as u16
    } else {
        80
    };
    let mut out = Vec::with_capacity(128 + rgb.len());
    out.extend_from_slice(&[0x0a, 5, 1, 8]); // manufacturer, version, RLE, bits per pixel
    for v in [0, 0, w16 - 1, h16 - 1, res, res] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&[0; 48]); // 16-colour palette, unused
    out.push(0); // reserved
    out.push(3); // planes
    out.extend_from_slice(&bytes_per_line.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // colour palette
    out.extend_from_slice(&[0; 58]); // screen size and filler
    let (w, bpl) = (w as usize, usize::from(bytes_per_line));
    let mut plane = Vec::with_capacity(bpl);
    for row in rgb.chunks_exact(w * 3) {
        for c in 0..3 {
            plane.clear();
            plane.extend(row.iter().skip(c).step_by(3));
            plane.resize(bpl, 0);
            pcx_rle(&plane, &mut out);
        }
    }
    Ok(out)
}

/// PCX run-length encoding of one plane's scan line: runs of up to 63
/// bytes as `0xC0 | n, byte`; single bytes below 0xC0 as themselves.
fn pcx_rle(line: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while let Some(&b) = line.get(i) {
        let mut n = 1;
        while n < 63 && line.get(i + n) == Some(&b) {
            n += 1;
        }
        if n > 1 || b >= 0xc0 {
            out.push(0xc0 | n as u8);
        }
        out.push(b);
        i += n;
    }
}

/// Whether a pixel is black in a 1-bit image: darker than mid-grey
/// (luminance as fig2dev's `rgb2luminance`).
fn is_dark(p: &[u8; 3]) -> bool {
    let [r, g, b] = p.map(f64::from);
    0.3 * r + 0.59 * g + 0.11 * b < 127.5
}

/// X11 bitmap: black pixels are set bits, least significant bit first.
fn xbm(rgb: &[u8], w: u32, h: u32) -> String {
    use std::fmt::Write as _;
    let mut s = format!("#define noname_width {w}\n#define noname_height {h}\nstatic char noname_bits[] = {{\n");
    let w = w as usize;
    let mut n = 0usize;
    for row in rgb.chunks_exact(w * 3) {
        for byte in row.chunks(8 * 3) {
            let mut v = 0u8;
            for (bit, p) in byte.as_chunks::<3>().0.iter().enumerate() {
                if is_dark(p) {
                    v |= 1 << bit;
                }
            }
            if n > 0 {
                s.push(',');
                s.push(if n.is_multiple_of(12) { '\n' } else { ' ' });
            }
            if n.is_multiple_of(12) {
                s.push(' ');
            }
            let _ = write!(s, "0x{v:02x}"); // writing to a String cannot fail
            n += 1;
        }
    }
    s.push_str("};\n");
    s
}

/// Characters naming XPM colours: printable ASCII without `"` and `\`.
const XPM_CHARS: &[u8] =
    b"!#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~";

/// X11 pixmap (XPM 3), colours in order of first use.
fn xpm(rgb: &[u8], w: u32, h: u32) -> String {
    use std::fmt::Write as _;
    let mut index: HashMap<[u8; 3], u32> = HashMap::new();
    let mut colors: Vec<[u8; 3]> = Vec::new();
    let pixels: Vec<u32> = rgb
        .as_chunks::<3>()
        .0
        .iter()
        .map(|&c| {
            *index.entry(c).or_insert_with(|| {
                colors.push(c);
                u32::try_from(colors.len() - 1).unwrap_or(u32::MAX)
            })
        })
        .collect();
    let base = XPM_CHARS.len();
    let mut cpp = 1;
    let mut capacity = base;
    while capacity < colors.len() {
        cpp += 1;
        capacity = capacity.saturating_mul(base);
    }
    let name = |mut i: usize| -> String {
        let mut s = String::with_capacity(cpp);
        for _ in 0..cpp {
            s.push(char::from(XPM_CHARS.get(i % base).copied().unwrap_or(b'!')));
            i /= base;
        }
        s
    };
    let names: Vec<String> = (0..colors.len()).map(name).collect();
    let mut s = String::from("/* XPM */\nstatic char *noname[] = {\n/* width height ncolors chars_per_pixel */\n");
    let _ = writeln!(s, "\"{w} {h} {} {cpp}\",", colors.len()); // writing to a String cannot fail
    s.push_str("/* colors */\n");
    for (c, n) in colors.iter().zip(&names) {
        let _ = writeln!(s, "\"{n} c #{:02X}{:02X}{:02X}\",", c[0], c[1], c[2]);
    }
    s.push_str("/* pixels */\n");
    let w = w as usize;
    let rows = pixels.chunks(w.max(1));
    let last = rows.len();
    for (i, row) in rows.enumerate() {
        s.push('"');
        for p in row {
            s.push_str(names.get(*p as usize).map_or("!", String::as_str));
        }
        s.push('"');
        if i + 1 < last {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("};\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::{Color, Paint, Rect as SkRect, Transform};

    /// 11×5: red on the left (x < 4), transparent elsewhere, one blue pixel at (8, 2).
    fn sample() -> Pixmap {
        let mut pm = Pixmap::new(11, 5).unwrap();
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(255, 0, 0, 255));
        pm.fill_rect(
            SkRect::from_xywh(0.0, 0.0, 4.0, 5.0).unwrap(),
            &paint,
            Transform::identity(),
            None,
        );
        paint.set_color(Color::from_rgba8(0, 0, 255, 255));
        pm.fill_rect(
            SkRect::from_xywh(8.0, 2.0, 1.0, 1.0).unwrap(),
            &paint,
            Transform::identity(),
            None,
        );
        pm
    }

    fn expected(x: u32, y: u32) -> [u8; 3] {
        if x < 4 {
            [255, 0, 0]
        } else if (x, y) == (8, 2) {
            [0, 0, 255]
        } else {
            [255, 255, 255]
        }
    }

    fn enc(format: ExportFormat) -> Vec<u8> {
        encode(format, &sample(), &Encoding::default()).unwrap()
    }

    fn check_exact(img: &image::RgbImage) {
        assert_eq!(img.dimensions(), (11, 5));
        for (x, y, p) in img.enumerate_pixels() {
            assert_eq!(p.0, expected(x, y), "({x}, {y})");
        }
    }

    #[test]
    fn transparent_pixels_become_white() {
        let rgb = rgb_over_white(&sample());
        assert_eq!(rgb.len(), 11 * 5 * 3);
        assert_eq!(rgb.get(..3), Some(&[255, 0, 0][..]));
        assert_eq!(rgb.get(3 * 10..3 * 11), Some(&[255, 255, 255][..]));
    }

    #[test]
    fn gif_ppm_tiff_round_trip_exactly() {
        for (f, fmt) in [
            (ExportFormat::Gif, image::ImageFormat::Gif),
            (ExportFormat::Ppm, image::ImageFormat::Pnm),
            (ExportFormat::Tiff, image::ImageFormat::Tiff),
        ] {
            let img = image::load_from_memory_with_format(&enc(f), fmt).unwrap();
            check_exact(&img.to_rgb8());
        }
        assert!(enc(ExportFormat::Ppm).starts_with(b"P6\n11 5\n255\n"));
    }

    #[test]
    fn gif_transparent_colour() {
        let e = Encoding {
            transparent: Some(Rgb::WHITE),
            ..Encoding::default()
        };
        let bytes = encode(ExportFormat::Gif, &sample(), &e).unwrap();
        let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Gif)
            .unwrap()
            .to_rgba8();
        assert_eq!(img.get_pixel(0, 0).0[3], 255);
        assert_eq!(img.get_pixel(8, 2).0, [0, 0, 255, 255]);
        assert_eq!(img.get_pixel(10, 4).0[3], 0, "white is transparent");
        let plain = image::load_from_memory_with_format(&enc(ExportFormat::Gif), image::ImageFormat::Gif)
            .unwrap()
            .to_rgba8();
        assert!(plain.pixels().all(|p| p.0[3] == 255));
    }

    #[test]
    fn jpeg_quality_changes_the_size() {
        let mut pm = Pixmap::new(64, 64).unwrap();
        for (i, p) in pm.data_mut().as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let v = (i * 37 % 251) as u8;
            p.copy_from_slice(&[v, v / 2, 255 - v, 255]);
        }
        let jpeg = |q| {
            encode(
                ExportFormat::Jpeg,
                &pm,
                &Encoding {
                    quality: q,
                    ..Encoding::default()
                },
            )
            .unwrap()
        };
        let (lo, hi) = (jpeg(10), jpeg(95));
        assert!(lo.len() < hi.len(), "{} {}", lo.len(), hi.len());
        let img = image::load_from_memory_with_format(&jpeg(75), image::ImageFormat::Jpeg).unwrap();
        assert_eq!((img.width(), img.height()), (64, 64));
        // The sample's red stays red.
        let img = image::load_from_memory_with_format(&enc(ExportFormat::Jpeg), image::ImageFormat::Jpeg)
            .unwrap()
            .to_rgb8();
        let p = img.get_pixel(1, 1).0;
        assert!(p[0] > 200 && p[1] < 60 && p[2] < 60, "{p:?}");
    }

    /// A minimal PCX reader for the 24-bit files written above.
    fn read_pcx(b: &[u8]) -> image::RgbImage {
        assert_eq!(&b[..4], &[0x0a, 5, 1, 8]);
        let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        let (w, h) = (u32::from(u16_at(8)) + 1, u32::from(u16_at(10)) + 1);
        assert_eq!(b[65], 3);
        let bpl = usize::from(u16_at(66));
        let mut data = Vec::new();
        let mut i = 128;
        while i < b.len() {
            if b[i] >= 0xc0 {
                data.extend(std::iter::repeat_n(b[i + 1], usize::from(b[i] & 0x3f)));
                i += 2;
            } else {
                data.push(b[i]);
                i += 1;
            }
        }
        assert_eq!(data.len(), bpl * 3 * h as usize);
        image::RgbImage::from_fn(w, h, |x, y| {
            let row = &data[y as usize * bpl * 3..];
            image::Rgb([0, 1, 2].map(|c| row[c * bpl + x as usize]))
        })
    }

    #[test]
    fn pcx_round_trips() {
        let b = enc(ExportFormat::Pcx);
        assert!(b.len() > 128);
        check_exact(&read_pcx(&b));
        // Long runs and bytes from 0xC0 up are encoded as runs.
        let mut out = Vec::new();
        pcx_rle(&[0xc5, 1, 1, 1], &mut out);
        assert_eq!(out, vec![0xc1, 0xc5, 0xc3, 1]);
        out.clear();
        pcx_rle(&[7; 70], &mut out);
        assert_eq!(out, vec![0xc0 | 63, 7, 0xc7, 7]);
    }

    #[test]
    fn xbm_sets_the_dark_pixels() {
        let s = String::from_utf8(enc(ExportFormat::Xbm)).unwrap();
        assert!(s.starts_with("#define noname_width 11\n#define noname_height 5\nstatic char noname_bits[] = {\n"));
        let bytes: Vec<u8> = s
            .split(|c: char| c == ',' || c.is_whitespace() || c == '}')
            .filter_map(|t| t.strip_prefix("0x"))
            .map(|t| u8::from_str_radix(t, 16).unwrap())
            .collect();
        // Two bytes per row; red's luminance (0.3 · 255) is dark, so the
        // red columns are set, as is the blue pixel.
        assert_eq!(bytes.len(), 10);
        for (y, row) in bytes.chunks(2).enumerate() {
            let blue = if y == 2 { 1 } else { 0 };
            assert_eq!(row, [0x0f, blue], "row {y}");
        }
    }

    #[test]
    fn xpm_lists_colours_and_pixels() {
        let s = String::from_utf8(enc(ExportFormat::Xpm)).unwrap();
        let strings: Vec<&str> = s.lines().filter_map(|l| l.split('"').nth(1)).collect();
        assert_eq!(strings[0], "11 5 3 1");
        assert_eq!(&strings[1..4], ["! c #FF0000", "# c #FFFFFF", "$ c #0000FF"]);
        assert_eq!(strings.len(), 4 + 5);
        assert_eq!(strings[4 + 2], "!!!!####$##");
        assert!(s.trim_end().ends_with("};"));
    }

    #[test]
    fn xpm_uses_more_characters_for_many_colours() {
        let mut pm = Pixmap::new(200, 1).unwrap();
        for (i, p) in pm.data_mut().as_chunks_mut::<4>().0.iter_mut().enumerate() {
            p.copy_from_slice(&[i as u8, 0, 0, 255]);
        }
        let s = xpm(&rgb_over_white(&pm), 200, 1);
        let header = s.lines().find_map(|l| l.strip_prefix('"')).unwrap();
        assert_eq!(header, "200 1 200 2\",");
    }

    #[test]
    fn vector_formats_are_refused() {
        assert!(encode(ExportFormat::Svg, &sample(), &Encoding::default()).is_err());
    }
}
