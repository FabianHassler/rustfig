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
    /// The output resolution in pixels per inch (no format records it now:
    /// netpbm's PCX header holds the image size instead).
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
        ExportFormat::Pcx => pcx(&rgb, w, h),
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

/// PCX version 5 as `ppmtopcx` writes it: images of at most 16 colours in
/// bit planes with the palette in the header (one to four planes), at most
/// 256 colours as 8-bit indices with the palette after the data, else 24
/// bits in three planes. Scan lines are not padded to an even length, and
/// the header's resolution is the image size, as in netpbm. (The palette is
/// in order of first use; netpbm's follows its hash table.)
fn pcx(rgb: &[u8], w: u32, h: u32) -> Result<Vec<u8>, String> {
    let (Ok(w16), Ok(h16)) = (u16::try_from(w), u16::try_from(h)) else {
        return Err("PCX images are at most 65535 pixels wide and high".into());
    };
    if w16 == 0 || h16 == 0 {
        return Err("image would be empty".into());
    }
    // The distinct colours, in order of first use, up to 257.
    let mut index: HashMap<[u8; 3], u8> = HashMap::new();
    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut many = false;
    for &c in rgb.as_chunks::<3>().0 {
        if index.contains_key(&c) {
            continue;
        }
        if palette.len() == 256 {
            many = true;
            break;
        }
        index.insert(c, palette.len() as u8);
        palette.push(c);
    }
    let w = w as usize;
    let (bits, planes, bpl) = if many {
        (8u8, 3u8, w)
    } else if palette.len() > 16 {
        (8, 1, w)
    } else {
        // 1 or 2 colours: one plane; 3 or 4: two; up to 16: four at most.
        let planes = match palette.len() {
            0..=2 => 1,
            3..=4 => 2,
            5..=8 => 3,
            _ => 4,
        };
        (1, planes, w.div_ceil(8))
    };
    let bpl16 = u16::try_from(bpl).map_err(|_| "PCX image too wide")?;
    let mut out = Vec::with_capacity(128 + rgb.len() / 2);
    out.extend_from_slice(&[0x0a, 5, 1, bits]); // manufacturer, version, RLE, bits per pixel
    for v in [0, 0, w16 - 1, h16 - 1, w16, h16] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    let mut header_palette = [0u8; 48];
    if bits == 1 {
        for (dst, c) in header_palette.as_chunks_mut::<3>().0.iter_mut().zip(&palette) {
            dst.copy_from_slice(c);
        }
    }
    out.extend_from_slice(&header_palette);
    out.push(0); // reserved
    out.push(planes);
    out.extend_from_slice(&bpl16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // colour palette
    out.extend_from_slice(&[0; 58]); // screen size and filler
    let mut plane = Vec::with_capacity(bpl);
    for row in rgb.chunks_exact((w * 3).max(1)) {
        if many {
            for c in 0..3 {
                plane.clear();
                plane.extend(row.iter().skip(c).step_by(3));
                pcx_rle(&plane, &mut out);
            }
        } else {
            let idx = |p: &[u8; 3]| -> u8 { index.get(p).copied().unwrap_or(0) };
            if bits == 8 {
                plane.clear();
                plane.extend(row.as_chunks::<3>().0.iter().map(idx));
                pcx_rle(&plane, &mut out);
            } else {
                for bit in 0..planes {
                    plane.clear();
                    plane.resize(bpl, 0);
                    for (x, p) in row.as_chunks::<3>().0.iter().enumerate() {
                        if let Some(byte) = plane.get_mut(x / 8) {
                            *byte |= ((idx(p) >> bit) & 1) << (7 - x % 8);
                        }
                    }
                    pcx_rle(&plane, &mut out);
                }
            }
        }
    }
    if bits == 8 && planes == 1 {
        out.push(0x0c);
        for i in 0..256 {
            out.extend_from_slice(palette.get(i).unwrap_or(&[0, 0, 0]));
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

/// `ppmtopgm`'s grey (`(77 r + 150 g + 29 b + 128) / 256`).
fn gray(p: &[u8; 3]) -> i64 {
    let [r, g, b] = p.map(i64::from);
    (77 * r + 150 * g + 29 * b + 128) >> 8
}

/// `pgmtopbm`'s Floyd-Steinberg dithering (its default): the rows run
/// alternately left to right and back, a pixel is white when the grey plus
/// the error carried to it reaches the middle, and the error goes 7/16 to
/// the next pixel, 3/16, 5/16 and 1/16 to the row below. (netpbm starts
/// from random errors, so its pixels in grey areas differ from run to run;
/// here the errors start at zero.) Black is true.
fn dither(rgb: &[u8], w: usize, h: usize) -> Vec<bool> {
    const SCALE: i64 = 1024;
    let mut black = vec![false; w * h];
    let mut this = vec![0i64; w + 2];
    let mut next = vec![0i64; w + 2];
    for (y, row) in rgb.chunks_exact((w * 3).max(1)).take(h).enumerate() {
        let forward = y % 2 == 0;
        next.iter_mut().for_each(|e| *e = 0);
        for step in 0..w {
            let x = if forward { step } else { w - 1 - step };
            let g = row.get(x * 3..x * 3 + 3).and_then(|p| <&[u8; 3]>::try_from(p).ok());
            let g = g.map_or(255, gray);
            let mut sum = g * SCALE / 255 + this.get(x + 1).copied().unwrap_or(0);
            if sum >= SCALE / 2 {
                sum -= SCALE;
            } else if let Some(b) = black.get_mut(y * w + x) {
                *b = true;
            }
            // `sum` is the error now.
            let (ahead, behind) = if forward { (x + 2, x) } else { (x, x + 2) };
            if let Some(e) = this.get_mut(ahead) {
                *e += sum * 7 / 16;
            }
            if let Some(e) = next.get_mut(behind) {
                *e += sum * 3 / 16;
            }
            if let Some(e) = next.get_mut(x + 1) {
                *e += sum * 5 / 16;
            }
            if let Some(e) = next.get_mut(ahead) {
                *e += sum / 16;
            }
        }
        std::mem::swap(&mut this, &mut next);
    }
    black
}

/// X11 bitmap as `pbmtoxbm` writes it after `ppmtopgm | pgmtopbm`: black
/// (after dithering) pixels are set bits, least significant bit first.
fn xbm(rgb: &[u8], w: u32, h: u32) -> String {
    use std::fmt::Write as _;
    let mut s = format!("#define noname_width {w}\n#define noname_height {h}\nstatic char noname_bits[] = {{\n");
    let (wu, hu) = (w as usize, h as usize);
    let black = dither(rgb, wu, hu);
    let mut n = 0usize;
    for row in black.chunks(wu.max(1)) {
        for byte in row.chunks(8) {
            let mut v = 0u8;
            for (bit, &b) in byte.iter().enumerate() {
                if b {
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

    /// A minimal PCX reader for the files written above.
    fn read_pcx(b: &[u8]) -> image::RgbImage {
        let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        let bits = b[3];
        let (w, h) = (u32::from(u16_at(8)) + 1, u32::from(u16_at(10)) + 1);
        let planes = usize::from(b[65]);
        let bpl = usize::from(u16_at(66));
        let mut data = Vec::new();
        let mut i = 128;
        let end = if bits == 8 && planes == 1 {
            b.len() - 769
        } else {
            b.len()
        };
        while i < end {
            if b[i] >= 0xc0 {
                data.extend(std::iter::repeat_n(b[i + 1], usize::from(b[i] & 0x3f)));
                i += 2;
            } else {
                data.push(b[i]);
                i += 1;
            }
        }
        assert_eq!(data.len(), bpl * planes * h as usize);
        image::RgbImage::from_fn(w, h, |x, y| {
            let row = &data[y as usize * bpl * planes..];
            let x = x as usize;
            match (bits, planes) {
                (8, 3) => image::Rgb([0, 1, 2].map(|c| row[c * bpl + x])),
                (8, 1) => {
                    let k = usize::from(row[x]);
                    image::Rgb([b[end + 1 + 3 * k], b[end + 2 + 3 * k], b[end + 3 + 3 * k]])
                }
                _ => {
                    let k: usize = (0..planes)
                        .map(|p| usize::from(row[p * bpl + x / 8] >> (7 - x % 8) & 1) << p)
                        .sum();
                    image::Rgb([b[16 + 3 * k], b[17 + 3 * k], b[18 + 3 * k]])
                }
            }
        })
    }

    #[test]
    fn pcx_round_trips() {
        // Three colours: two bit planes, the palette in the header.
        let b = enc(ExportFormat::Pcx);
        assert_eq!(&b[..4], &[0x0a, 5, 1, 1]);
        assert_eq!(b[65], 2);
        check_exact(&read_pcx(&b));
        // 20 colours: 8 bits with the palette behind the data; many more:
        // 24 bits in three planes. Lines are not padded.
        let mut pm = Pixmap::new(37, 20).unwrap();
        for (i, p) in pm.data_mut().as_chunks_mut::<4>().0.iter_mut().enumerate() {
            p.copy_from_slice(&[(i % 20) as u8, 1, 2, 255]);
        }
        let b = encode(ExportFormat::Pcx, &pm, &Encoding::default()).unwrap();
        assert_eq!((b[3], b[65], u16::from_le_bytes([b[66], b[67]])), (8, 1, 37));
        assert_eq!(b[b.len() - 769], 0x0c);
        assert_eq!(read_pcx(&b).get_pixel(5, 0).0, [5, 1, 2]);
        for (i, p) in pm.data_mut().as_chunks_mut::<4>().0.iter_mut().enumerate() {
            p.copy_from_slice(&[(i % 256) as u8, (i / 3) as u8, 2, 255]);
        }
        let b = encode(ExportFormat::Pcx, &pm, &Encoding::default()).unwrap();
        assert_eq!((b[3], b[65]), (8, 3));
        assert_eq!(read_pcx(&b).get_pixel(5, 0).0, [5, 1, 2]);
        // Long runs and bytes from 0xC0 up are encoded as runs.
        let mut out = Vec::new();
        pcx_rle(&[0xc5, 1, 1, 1], &mut out);
        assert_eq!(out, vec![0xc1, 0xc5, 0xc3, 1]);
        out.clear();
        pcx_rle(&[7; 70], &mut out);
        assert_eq!(out, vec![0xc0 | 63, 7, 0xc7, 7]);
    }

    fn xbm_bytes(pm: &Pixmap) -> Vec<u8> {
        let s = String::from_utf8(encode(ExportFormat::Xbm, pm, &Encoding::default()).unwrap()).unwrap();
        s.split(|c: char| c == ',' || c.is_whitespace() || c == '}')
            .filter_map(|t| t.strip_prefix("0x"))
            .map(|t| u8::from_str_radix(t, 16).unwrap())
            .collect()
    }

    #[test]
    fn xbm_sets_the_black_pixels() {
        let s = String::from_utf8(enc(ExportFormat::Xbm)).unwrap();
        assert!(s.starts_with("#define noname_width 11\n#define noname_height 5\nstatic char noname_bits[] = {\n"));
        // Black and white only: no dithering. Two bytes per row, the dark
        // (ppmtopgm grey 77 of red, so dithered to about a third) pixels
        // aside, the blue pixel (grey 29) is black.
        let bytes = xbm_bytes(&sample());
        assert_eq!(bytes.len(), 10);
        let mut black = Pixmap::new(11, 2).unwrap();
        black.fill(Color::WHITE);
        let paint = Paint {
            shader: tiny_skia::Shader::SolidColor(Color::BLACK),
            ..Paint::default()
        };
        black.fill_rect(
            SkRect::from_xywh(0.0, 0.0, 4.0, 1.0).unwrap(),
            &paint,
            Transform::identity(),
            None,
        );
        assert_eq!(xbm_bytes(&black), vec![0x0f, 0, 0, 0]);
    }

    #[test]
    fn xbm_dithers_grey_like_pgmtopbm() {
        // Mid grey (128 of 255) comes out about half black.
        let mut pm = Pixmap::new(64, 8).unwrap();
        pm.fill(Color::from_rgba8(128, 128, 128, 255));
        let ones: u32 = xbm_bytes(&pm).iter().map(|b| b.count_ones()).sum();
        assert!((200..=312).contains(&ones), "{ones} of 512");
        // Light grey is mostly white, dark grey mostly black.
        pm.fill(Color::from_rgba8(230, 230, 230, 255));
        let light: u32 = xbm_bytes(&pm).iter().map(|b| b.count_ones()).sum();
        pm.fill(Color::from_rgba8(30, 30, 30, 255));
        let dark: u32 = xbm_bytes(&pm).iter().map(|b| b.count_ones()).sum();
        assert!(light < 100 && dark > 400, "{light} {dark}");
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
