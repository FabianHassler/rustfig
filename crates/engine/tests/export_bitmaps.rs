//! B6: fig2dev's bitmap languages from RustFig's renderer. Every bitmap
//! format carries the pixels of RustFig's PNG of the same figure, and the
//! PNG matches RustFig's EPS rasterised by ghostscript (when installed):
//! RustFig's own EPS is the reference, not fig2dev's ghostscript output.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use image::{ImageFormat, RgbImage, RgbaImage};
use rustfig_engine::model::{Document, Rgb};
use rustfig_engine::render::options::{parse_options, OptionValue};
use rustfig_engine::render::{export, languages::language, ExportFormat, ExportOptions};
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

/// Export options for `lang` with fig2dev options, at 80 dpi as fig2dev.
fn options(lang: &str, items: &[(&str, OptionValue<'_>)]) -> ExportOptions {
    ExportOptions {
        dpi: 80.0,
        fig2dev: parse_options(language(lang).unwrap(), items).unwrap(),
        ..ExportOptions::default()
    }
}

fn render(doc: &Document, format: ExportFormat, opts: &ExportOptions) -> Vec<u8> {
    export(doc, format, opts).unwrap()
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

/// A minimal reader for RustFig's 24-bit PCX files.
fn read_pcx(b: &[u8]) -> RgbImage {
    assert_eq!(&b[..4], &[0x0a, 5, 1, 8]);
    let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let (w, h) = (u32::from(u16_at(8)) + 1, u32::from(u16_at(10)) + 1);
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
    RgbImage::from_fn(w, h, |x, y| {
        let row = &data[y as usize * bpl * 3..];
        image::Rgb([0, 1, 2].map(|c| row[c * bpl + x as usize]))
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
            let items: Vec<(&str, OptionValue<'_>)> = smoothing
                .map(|s| ("smoothing", OptionValue::Text(s)))
                .into_iter()
                .collect();
            let png = on_white(&decode(
                &render(&doc, ExportFormat::Png, &options("png", &items)),
                ImageFormat::Png,
            ));
            let get = |lang: &str, f: ExportFormat| render(&doc, f, &options(lang, &items));
            let ctx = format!("{name} smoothing {smoothing:?}");
            // Lossless formats: the same pixels.
            for (lang, f, fmt) in [
                ("ppm", ExportFormat::Ppm, ImageFormat::Pnm),
                ("tiff", ExportFormat::Tiff, ImageFormat::Tiff),
            ] {
                let img = decode(&get(lang, f), fmt);
                assert_eq!(on_white(&img), png, "{lang} {ctx}");
            }
            assert_eq!(read_pcx(&get("pcx", ExportFormat::Pcx)), png, "pcx {ctx}");
            assert_eq!(read_xpm(&get("xpm", ExportFormat::Xpm)), png, "xpm {ctx}");
            // GIF: exact up to 256 colours, quantised beyond.
            let gif = on_white(&decode(&get("gif", ExportFormat::Gif), ImageFormat::Gif));
            if colours(&png) <= 256 {
                assert_eq!(gif, png, "gif {ctx}");
            } else {
                let d = mean_diff(&gif, &png);
                assert!(d < 4.0, "gif {ctx}: mean difference {d}");
            }
            // JPEG: lossy.
            let jpeg = on_white(&decode(&get("jpeg", ExportFormat::Jpeg), ImageFormat::Jpeg));
            let d = mean_diff(&jpeg, &png);
            assert!(d < 8.0, "jpeg {ctx}: mean difference {d}");
            // XBM: the PNG thresholded.
            let xbm = read_xbm(&get("xbm", ExportFormat::Xbm));
            assert_eq!(xbm.dimensions(), png.dimensions(), "xbm {ctx}");
            for (a, b) in xbm.pixels().zip(png.pixels()) {
                let [r, g, bl] = b.0.map(f64::from);
                let dark = 0.3 * r + 0.59 * g + 0.11 * bl < 127.5;
                assert_eq!(a.0[0] == 0, dark, "xbm {ctx}");
            }
        }
    }
}

#[test]
fn bitmaps_are_sized_as_genbitmaps_sizes_them() {
    // fig2dev: 80 dpi times the magnification, (int)(size + 0.9) pixels.
    let doc = fixture("lines");
    let size = |items: &[(&str, OptionValue<'_>)]| {
        let img = decode(
            &render(&doc, ExportFormat::Ppm, &options("ppm", items)),
            ImageFormat::Pnm,
        );
        img.dimensions()
    };
    let (w, h) = size(&[]);
    let b = rustfig_engine::render::bound::fig2dev_bounds(&doc, &Default::default(), true);
    assert_eq!(w, (b.width() / 15.0 + 0.9).floor() as u32);
    assert_eq!(h, (b.height() / 15.0 + 0.9).floor() as u32);
    // -m 2 doubles it; -b 5 adds five pixels on each side.
    let (w2, _) = size(&[("m", OptionValue::Text("2"))]);
    assert!(w2.abs_diff(2 * w) <= 1, "{w2} {w}");
    let (wb, hb) = size(&[("b", OptionValue::Text("5"))]);
    assert_eq!((wb, hb), (w + 10, h + 10));
}

#[test]
fn smoothing_changes_the_edges() {
    let doc = fixture("arrows");
    let img = |s: &str| {
        on_white(&decode(
            &render(&doc, ExportFormat::Ppm, &options("ppm", &[("S", OptionValue::Text(s))])),
            ImageFormat::Pnm,
        ))
    };
    let (hard, some, more) = (img("1"), img("2"), img("4"));
    assert_eq!(img("0"), hard, "0 and 1 both mean none");
    // Hard edges keep the figure's few colours; smoothing adds grey levels.
    assert!(colours(&hard) < colours(&some), "{} {}", colours(&hard), colours(&some));
    assert!(colours(&some) < colours(&more), "{} {}", colours(&some), colours(&more));
    assert_ne!(some, more);
    // And RustFig's default (no -S) antialiases.
    let aa = on_white(&decode(
        &render(&doc, ExportFormat::Ppm, &options("ppm", &[])),
        ImageFormat::Pnm,
    ));
    assert!(colours(&aa) > colours(&hard));
}

#[test]
fn background_border_and_grayscale() {
    let doc = fixture("colours");
    let img = |items: &[(&str, OptionValue<'_>)]| {
        on_white(&decode(
            &render(&doc, ExportFormat::Ppm, &options("ppm", items)),
            ImageFormat::Pnm,
        ))
    };
    let bg = img(&[("g", OptionValue::Text("red")), ("b", OptionValue::Text("4"))]);
    assert_eq!(bg.get_pixel(0, 0).0, [255, 0, 0]);
    let plain = img(&[("b", OptionValue::Text("4"))]);
    assert_eq!(plain.get_pixel(0, 0).0, [255, 255, 255], "white without -g");
    let gray = img(&[("N", OptionValue::Bool(true))]);
    assert!(gray.pixels().all(|p| p.0[0] == p.0[1] && p.0[1] == p.0[2]));
}

#[test]
fn gif_transparent_colour_from_the_option_or_the_figure() {
    let mut doc = fixture("lines");
    let corner_alpha = |doc: &Document, items: &[(&str, OptionValue<'_>)]| {
        let gif = decode(
            &render(doc, ExportFormat::Gif, &options("gif", items)),
            ImageFormat::Gif,
        );
        gif.get_pixel(0, 0).0[3]
    };
    assert_eq!(corner_alpha(&doc, &[]), 255);
    assert_eq!(corner_alpha(&doc, &[("t", OptionValue::Text("#ffffff"))]), 0);
    assert_eq!(corner_alpha(&doc, &[("t", OptionValue::Text("#ff0000"))]), 255);
    // The figure's own transparent colour (header line 9): 7 is white.
    doc.settings.transparent_color = 7;
    assert_eq!(corner_alpha(&doc, &[]), 0);
    // -t overrides it.
    assert_eq!(corner_alpha(&doc, &[("t", OptionValue::Text("black"))]), 255);
    // None (-2), default (-1) and an undefined user colour give none.
    for n in [-2, -1, 40] {
        doc.settings.transparent_color = n;
        assert_eq!(corner_alpha(&doc, &[]), 255, "{n}");
    }
}

#[test]
fn jpeg_quality_option() {
    let doc = fixture("fills");
    let size = |q: &str| {
        render(
            &doc,
            ExportFormat::Jpeg,
            &options("jpeg", &[("q", OptionValue::Text(q))]),
        )
        .len()
    };
    assert!(size("20") < size("95"));
    // fig2dev clamps the quality; -q has no effect outside JPEG.
    assert_eq!(size("500"), size("100"));
    let png = |items: &[(&str, OptionValue<'_>)]| render(&doc, ExportFormat::Png, &options("png", items));
    assert_eq!(png(&[("q", OptionValue::Text("10"))]), png(&[]));
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
        let opts = ExportOptions {
            dpi: f64::from(dpi),
            background: Some(Rgb::WHITE),
            ..ExportOptions::default()
        };
        let png = render(&doc, ExportFormat::Png, &opts);
        let eps = render(&doc, ExportFormat::Eps, &ExportOptions::default());
        let gs = fig2dev::rasterise(&eps, false, dpi, true).unwrap();
        let c = compare::compare(&gs, &png).unwrap();
        eprintln!("{name}: mean {:.4} strong {:.4}", c.score.mean, c.score.strong);
        assert!(c.score.mean < 0.04 && c.score.strong < 0.04, "{name}: {:?}", c.score);
    }
}
