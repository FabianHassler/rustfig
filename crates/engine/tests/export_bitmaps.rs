//! B6: fig2dev's bitmap languages from RustFig's renderer, exported as
//! `file.export` exports (Fig text through RustFig's fig2dev, plan B12).
//! Every bitmap format carries the pixels of RustFig's PNG of the same
//! figure, and the PNG matches RustFig's EPS rasterised by ghostscript
//! (when installed): RustFig's own EPS is the reference, not fig2dev's
//! ghostscript output.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use image::{ImageFormat, RgbImage, RgbaImage};
use rustfig_engine::fig2dev::languages::language;
use rustfig_engine::fig2dev::options::{parse_options, OptionValue};
use rustfig_engine::model::Document;
use rustfig_engine::Engine;
use serde_json::json;
use std::path::{Path, PathBuf};
use support::{compare, fig2dev};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(name: &str) -> Document {
    let fig = std::fs::read(fixtures().join(format!("{name}.fig"))).unwrap();
    rustfig_engine::format::read_fig(&fig).unwrap()
}

const FIXTURES: &[&str] = &["lines", "fills", "arrows", "colours", "text", "metric"];

/// `doc` exported to `lang` by fig2dev with options `items` (letter and
/// argument; "" for a flag), at fig2dev's 80 dpi.
fn render(doc: &Document, lang: &str, items: &[(char, &str)]) -> Vec<u8> {
    let args: Vec<(char, Option<String>)> = items
        .iter()
        .map(|(c, a)| (*c, (!a.is_empty()).then(|| a.to_string())))
        .collect();
    rustfig_engine::fig2dev::export(doc, lang, &args, None).unwrap().0
}

fn decode(bytes: &[u8], f: ImageFormat) -> RgbaImage {
    image::load_from_memory_with_format(bytes, f).unwrap().to_rgba8()
}

/// RGBA over white.
fn on_white(img: &RgbaImage) -> RgbImage {
    RgbImage::from_fn(img.width(), img.height(), |x, y| {
        let [r, g, b, a] = img.get_pixel(x, y).0;
        let over = |c: u8| ((u32::from(c) * u32::from(a) + 255 * (255 - u32::from(a)) + 127) / 255) as u8;
        image::Rgb([over(r), over(g), over(b)])
    })
}

/// Mean absolute channel difference, 0–255.
fn mean_diff(a: &RgbImage, b: &RgbImage) -> f64 {
    assert_eq!(a.dimensions(), b.dimensions());
    let sum: u64 = a
        .as_raw()
        .iter()
        .zip(b.as_raw())
        .map(|(x, y)| u64::from(x.abs_diff(*y)))
        .sum();
    sum as f64 / a.as_raw().len() as f64
}

fn colours(img: &RgbImage) -> usize {
    img.pixels()
        .map(|p| p.0)
        .collect::<std::collections::HashSet<_>>()
        .len()
}

/// A minimal reader for RustFig's PCX files (bit planes with the palette in
/// the header, 8 bits with the palette behind the data, or 24 bits).
fn read_pcx(b: &[u8]) -> RgbImage {
    let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let bits = b[3];
    let (w, h) = (u32::from(u16_at(8)) + 1, u32::from(u16_at(10)) + 1);
    let planes = usize::from(b[65]);
    let bpl = usize::from(u16_at(66));
    let end = if bits == 8 && planes == 1 {
        b.len() - 769
    } else {
        b.len()
    };
    let mut data = Vec::new();
    let mut i = 128;
    while i < end {
        if b[i] >= 0xc0 {
            data.extend(std::iter::repeat_n(b[i + 1], usize::from(b[i] & 0x3f)));
            i += 2;
        } else {
            data.push(b[i]);
            i += 1;
        }
    }
    RgbImage::from_fn(w, h, |x, y| {
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

/// A minimal reader for RustFig's XPM files.
fn read_xpm(b: &[u8]) -> RgbImage {
    let s = std::str::from_utf8(b).unwrap();
    let strings: Vec<&str> = s.lines().filter_map(|l| l.split('"').nth(1)).collect();
    let head: Vec<usize> = strings[0].split(' ').map(|n| n.parse().unwrap()).collect();
    let (w, h, n, cpp) = (head[0], head[1], head[2], head[3]);
    let mut map = std::collections::HashMap::new();
    for line in &strings[1..=n] {
        let (name, rest) = line.split_at(cpp);
        let hex = rest.trim().strip_prefix("c #").unwrap();
        let v = u32::from_str_radix(hex, 16).unwrap();
        map.insert(name.to_string(), [(v >> 16) as u8, (v >> 8) as u8, v as u8]);
    }
    let rows = &strings[1 + n..];
    assert_eq!(rows.len(), h);
    RgbImage::from_fn(w as u32, h as u32, |x, y| {
        let row = rows[y as usize];
        let k = x as usize * cpp;
        image::Rgb(map[&row[k..k + cpp]])
    })
}

/// A minimal reader for RustFig's XBM files (black and white).
fn read_xbm(b: &[u8]) -> RgbImage {
    let s = std::str::from_utf8(b).unwrap();
    let num = |key: &str| -> u32 {
        let l = s.lines().find(|l| l.contains(key)).unwrap();
        l.split_whitespace().last().unwrap().parse().unwrap()
    };
    let (w, h) = (num("_width"), num("_height"));
    let body = &s[s.find('{').unwrap() + 1..];
    let bytes: Vec<u8> = body
        .split(|c: char| c == ',' || c.is_whitespace() || c == '}' || c == ';')
        .filter_map(|t| t.strip_prefix("0x"))
        .map(|t| u8::from_str_radix(t, 16).unwrap())
        .collect();
    let bpl = w.div_ceil(8) as usize;
    assert_eq!(bytes.len(), bpl * h as usize);
    RgbImage::from_fn(w, h, |x, y| {
        let set = bytes[y as usize * bpl + x as usize / 8] >> (x % 8) & 1 == 1;
        image::Rgb(if set { [0; 3] } else { [255; 3] })
    })
}

#[test]
fn every_bitmap_carries_the_pngs_pixels() {
    for name in FIXTURES {
        let doc = fixture(name);
        for smoothing in [None, Some("1"), Some("4")] {
            let items: Vec<(char, &str)> = smoothing.map(|s| ('S', s)).into_iter().collect();
            let png = on_white(&decode(&render(&doc, "png", &items), ImageFormat::Png));
            let get = |lang: &str| render(&doc, lang, &items);
            let ctx = format!("{name} smoothing {smoothing:?}");
            // Lossless formats: the same pixels.
            for (lang, fmt) in [("ppm", ImageFormat::Pnm), ("tiff", ImageFormat::Tiff)] {
                let img = decode(&get(lang), fmt);
                assert_eq!(on_white(&img), png, "{lang} {ctx}");
            }
            assert_eq!(read_pcx(&get("pcx")), png, "pcx {ctx}");
            assert_eq!(read_xpm(&get("xpm")), png, "xpm {ctx}");
            // GIF: exact up to 256 colours, quantised beyond.
            let gif = on_white(&decode(&get("gif"), ImageFormat::Gif));
            if colours(&png) <= 256 {
                assert_eq!(gif, png, "gif {ctx}");
            } else {
                let d = mean_diff(&gif, &png);
                assert!(d < 4.0, "gif {ctx}: mean difference {d}");
            }
            // JPEG: lossy.
            let jpeg = on_white(&decode(&get("jpeg"), ImageFormat::Jpeg));
            let d = mean_diff(&jpeg, &png);
            assert!(d < 8.0, "jpeg {ctx}: mean difference {d}");
            // XBM: ppmtopgm and pgmtopbm's Floyd-Steinberg dithering: black
            // and white stay, greys come out as the matching share of black.
            let xbm = read_xbm(&get("xbm"));
            assert_eq!(xbm.dimensions(), png.dimensions(), "xbm {ctx}");
            let (mut want, mut got) = (0.0, 0.0);
            for (a, b) in xbm.pixels().zip(png.pixels()) {
                let [r, g, bl] = b.0.map(i64::from);
                let gray = (77 * r + 150 * g + 29 * bl + 128) >> 8;
                if gray == 0 {
                    assert_eq!(a.0[0], 0, "xbm {ctx}");
                } else if gray == 255 {
                    assert_eq!(a.0[0], 255, "xbm {ctx}");
                }
                want += 1.0 - gray as f64 / 255.0;
                got += if a.0[0] == 0 { 1.0 } else { 0.0 };
            }
            let n = (xbm.width() * xbm.height()) as f64;
            assert!(((want - got) / n).abs() < 0.01, "xbm {ctx}: {want} {got}");
        }
    }
}

#[test]
fn bitmaps_are_sized_as_genbitmaps_sizes_them() {
    // fig2dev: 80 dpi times the magnification, (int)(size + 0.9) pixels.
    let doc = fixture("lines");
    let size = |items: &[(char, &str)]| decode(&render(&doc, "ppm", items), ImageFormat::Pnm).dimensions();
    let (w, h) = size(&[]);
    // fig2dev's bounds of the figure (1200 units per inch).
    let mut ctx = ::fig2dev::Ctx::default();
    let fig = rustfig_engine::format::write_fig(&doc);
    let objects = ::fig2dev::read::readfp_fig(&mut ctx, fig.as_bytes()).unwrap();
    let (llx, lly, urx, ury) = ::fig2dev::bound::compound_bound(&ctx, &objects, true);
    assert_eq!(w, (f64::from(urx - llx) / 15.0 + 0.9).floor() as u32);
    assert_eq!(h, (f64::from(ury - lly) / 15.0 + 0.9).floor() as u32);
    // -m 2 doubles it; -b 5 adds five pixels on each side.
    let (w2, _) = size(&[('m', "2")]);
    assert!(w2.abs_diff(2 * w) <= 1, "{w2} {w}");
    let (wb, hb) = size(&[('b', "5")]);
    assert_eq!((wb, hb), (w + 10, h + 10));
}

#[test]
fn smoothing_changes_the_edges() {
    let doc = fixture("arrows");
    let img = |s: &str| on_white(&decode(&render(&doc, "ppm", &[('S', s)]), ImageFormat::Pnm));
    let (hard, some, more) = (img("1"), img("2"), img("4"));
    assert_eq!(img("0"), hard, "0 and 1 both mean none");
    // Hard edges keep the figure's few colours; smoothing adds grey levels.
    assert!(colours(&hard) < colours(&some), "{} {}", colours(&hard), colours(&some));
    assert!(colours(&some) < colours(&more), "{} {}", colours(&some), colours(&more));
    assert_ne!(some, more);
    // fig2dev's default (no -S) is hard edges.
    let plain = on_white(&decode(&render(&doc, "ppm", &[]), ImageFormat::Pnm));
    assert_eq!(plain, hard);
}

#[test]
fn background_border_and_grayscale() {
    let doc = fixture("colours");
    let img = |items: &[(char, &str)]| on_white(&decode(&render(&doc, "ppm", items), ImageFormat::Pnm));
    let bg = img(&[('g', "red"), ('b', "4")]);
    assert_eq!(bg.get_pixel(0, 0).0, [255, 0, 0]);
    let plain = img(&[('b', "4")]);
    assert_eq!(plain.get_pixel(0, 0).0, [255, 255, 255], "white without -g");
    let gray = img(&[('N', "")]);
    assert!(gray.pixels().all(|p| p.0[0] == p.0[1] && p.0[1] == p.0[2]));
}

#[test]
fn gif_transparent_colour_from_the_option_or_the_figure() {
    let mut doc = fixture("lines");
    let corner_alpha = |doc: &Document, items: &[(char, &str)]| {
        let gif = decode(&render(doc, "gif", items), ImageFormat::Gif);
        gif.get_pixel(0, 0).0[3]
    };
    assert_eq!(corner_alpha(&doc, &[]), 255);
    assert_eq!(corner_alpha(&doc, &[('t', "#ffffff")]), 0);
    assert_eq!(corner_alpha(&doc, &[('t', "#ff0000")]), 255);
    // The figure's own transparent colour (header line 9): 7 is white.
    doc.settings.transparent_color = 7;
    assert_eq!(corner_alpha(&doc, &[]), 0);
    // -t overrides it.
    assert_eq!(corner_alpha(&doc, &[('t', "black")]), 255);
    // None (-2), default (-1) and an undefined user colour give none.
    for n in [-2, -1, 40] {
        doc.settings.transparent_color = n;
        assert_eq!(corner_alpha(&doc, &[]), 255, "{n}");
    }
}

#[test]
fn jpeg_quality_option() {
    let doc = fixture("fills");
    let size = |q: &str| render(&doc, "jpeg", &[('q', q)]).len();
    assert!(size("20") < size("95"));
    // fig2dev clamps the quality; -q has no effect outside JPEG.
    assert_eq!(size("500"), size("100"));
    let png = |items: &[(char, &str)]| render(&doc, "png", items);
    assert_eq!(png(&[('q', "10")]), png(&[]));
    assert!(parse_options(language("gif").unwrap(), &[("S", OptionValue::Text("3"))]).is_err());
}

#[test]
fn bitmaps_flow_through_file_export() {
    let dir = std::env::temp_dir().join(format!("rustfig-export-bitmaps-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 2400, "height": 1200}),
    )
    .unwrap();
    for (name, lang, magic) in [
        ("a.gif", "gif", &b"GIF8"[..]),
        ("a.jpg", "jpeg", &[0xff, 0xd8][..]),
        ("a.jpeg", "jpeg", &[0xff, 0xd8][..]),
        ("a.pcx", "pcx", &[0x0a, 5][..]),
        ("a.ppm", "ppm", &b"P6"[..]),
        ("a.tif", "tiff", &b"II*\0"[..]),
        ("a.tiff", "tiff", &b"II*\0"[..]),
        ("a.xbm", "xbm", &b"#define"[..]),
        ("a.xpm", "xpm", &b"/* XPM */"[..]),
    ] {
        let path = dir.join(name).to_string_lossy().into_owned();
        let r = e
            .execute(
                "file.export",
                &json!({"path": path, "dpi": 80, "options": {"smoothing": 2, "border": 3}}),
            )
            .unwrap();
        assert_eq!(r["format"], lang);
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(magic), "{name}");
    }
    // Text formats come back as text, the others as base64.
    let r = e.execute("file.export_data", &json!({"format": "xpm"})).unwrap();
    assert!(r["data"].as_str().unwrap().starts_with("/* XPM */"));
    let r = e.execute("file.export_data", &json!({"format": "gif"})).unwrap();
    assert!(r["base64"].as_str().is_some());
    let err = e
        .execute(
            "file.export_data",
            &json!({"format": "gif", "options": {"transparent": "nosuch"}}),
        )
        .unwrap_err();
    assert!(err.message.contains("color"), "{}", err.message);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// RustFig's PNG against RustFig's EPS rasterised by ghostscript at the
/// same resolution (skipped without ghostscript).
#[test]
fn png_matches_rustfigs_eps() {
    if fig2dev::ghostscript().is_none() {
        eprintln!("ghostscript not found; skipping");
        return;
    }
    let dpi = 80;
    for name in FIXTURES {
        let doc = fixture(name);
        // At 80 dpi (fig2dev's), on white, smoothed as the export panel does.
        let png = render(&doc, "png", &[('F', ""), ('g', "white"), ('S', "4")]);
        let eps = render(&doc, "eps", &[('F', "")]);
        let gs = fig2dev::rasterise(&eps, false, dpi, true).unwrap();
        let c = compare::compare(&gs, &png).unwrap();
        eprintln!("{name}: mean {:.4} strong {:.4}", c.score.mean, c.score.strong);
        assert!(c.score.mean < 0.04 && c.score.strong < 0.04, "{name}: {:?}", c.score);
    }
}
