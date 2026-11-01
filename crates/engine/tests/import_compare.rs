#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Manual check of the SVG importer: renders the original SVGs with resvg and
//! puts them beside RustFig's PNG of the imported figure, with a diff.
//!
//! `IMPORT_CMP_DIR=<dir> cargo test -p rustfig-engine --test import_compare -- --ignored --nocapture`
//! where `<dir>` holds `NAME.svg` and `NAME.png` (made by `rustfig-cli convert NAME.fig NAME.png`);
//! writes `NAME-cmp.png` (resvg | Inkscape | librsvg | RustFig | diff) there; the Inkscape and librsvg
//! renderings `NAME-ink.png` and `NAME-rsvg.png` are optional.

use image::{imageops, Rgba, RgbaImage};

#[test]
#[ignore = "manual: needs IMPORT_CMP_DIR"]
fn page_figures_for_comparison() {
    // NAME-page.fig is NAME.fig plus a white page-sized box at the back, so that RustFig's PNG
    // covers the whole SVG page (it is otherwise cropped to the drawing).
    let Ok(dir) = std::env::var("IMPORT_CMP_DIR") else {
        return;
    };
    for e in std::fs::read_dir(&dir).unwrap().filter_map(Result::ok) {
        let Some(n) = e
            .file_name()
            .to_str()
            .and_then(|f| f.strip_suffix(".svg"))
            .map(String::from)
        else {
            continue;
        };
        let (Ok(fig), Ok(svg)) = (
            std::fs::read_to_string(format!("{dir}/{n}.fig")),
            std::fs::read(e.path()),
        ) else {
            continue;
        };
        let Ok(tree) = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default()) else {
            continue;
        };
        let (pw, ph) = (
            (tree.size().width() * 12.5).round(),
            (tree.size().height() * 12.5).round(),
        );
        // Two tiny white circles in the page corners: they widen the bounds to the page and sit
        // below everything (ellipses come first at a depth, in file order).
        let dot = |x: f32, y: f32| format!("1 3 0 0 7 7 999 -1 20 0.000 1 0.0000 {x} {y} 3 3 {x} {y} {x} {y}\n");
        let boxed = format!("{}{}", dot(3.0, 3.0), dot(pw - 3.0, ph - 3.0));
        // First object in the file, so that same-depth objects draw above it.
        let at = fig.lines().scan(0usize, |pos, l| {
            let start = *pos;
            *pos += l.len() + 1;
            Some((start, l))
        });
        let first = at
            .into_iter()
            .skip_while(|(_, l)| !l.starts_with("1200 "))
            .skip(1)
            .find(|(_, l)| !l.starts_with("0 "))
            .map_or(fig.len(), |(i, _)| i);
        let page = format!("{}{boxed}{}", &fig[..first], &fig[first..]);
        std::fs::write(format!("{dir}/{n}-page.fig"), page).unwrap();
    }
}

#[test]
#[ignore = "manual: needs IMPORT_CMP_DIR"]
fn compare_imports_with_resvg() {
    let Ok(dir) = std::env::var("IMPORT_CMP_DIR") else {
        return;
    };
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().to_str()?.strip_suffix(".svg").map(String::from))
        .collect();
    names.sort();
    for n in names {
        let Ok(ours) = image::open(format!("{dir}/{n}.png")) else {
            continue;
        };
        let ours = ours.to_rgba8();
        let svg = std::fs::read(format!("{dir}/{n}.svg")).unwrap();
        let mut opt = resvg::usvg::Options::default();
        opt.resources_dir = Some(std::path::PathBuf::from(&dir));
        let Ok(tree) = resvg::usvg::Tree::from_data(&svg, &opt) else {
            continue;
        };
        let (w, h) = (ours.width(), ours.height());
        let s = tree.size();
        // The page, scaled to RustFig's width (the heights agree within a pixel or two).
        let k = w as f32 / s.width();
        let ph = (s.height() * k).round().max(1.0) as u32;
        let mut pix = resvg::tiny_skia::Pixmap::new(w, ph).unwrap();
        pix.fill(resvg::tiny_skia::Color::WHITE);
        resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(k, k), &mut pix.as_mut());
        let r = imageops::resize(
            &RgbaImage::from_raw(w, ph, pix.data().to_vec()).unwrap(),
            w,
            h,
            imageops::FilterType::Triangle,
        );
        // Inkscape's and librsvg's renderings, when present (NAME-ink.png, NAME-rsvg.png).
        let load = |suffix: &str| {
            image::open(format!("{dir}/{n}-{suffix}.png"))
                .ok()
                .map(|i| imageops::resize(&i.to_rgba8(), w, h, imageops::FilterType::Triangle))
        };
        let (ink, rsvg) = (load("ink"), load("rsvg"));
        let stats = |a: &RgbaImage, b: &RgbaImage| {
            let (mut sum, mut max, mut bad) = (0u64, 0i32, 0u64);
            for (p, q) in a.pixels().zip(b.pixels()) {
                let d: i32 = (0..3).map(|i| (i32::from(p[i]) - i32::from(q[i])).abs()).sum::<i32>() / 3;
                sum += d as u64;
                max = max.max(d);
                bad += u64::from(d > 30);
            }
            let px = f64::from(w) * f64::from(h);
            format!(
                "mean {:.2} max {max} >30: {:.2}%",
                sum as f64 / px,
                bad as f64 * 100.0 / px
            )
        };
        let diff_of = |a: &RgbaImage, b: &RgbaImage| {
            let mut d = RgbaImage::new(w, h);
            for (x, y, p) in d.enumerate_pixels_mut() {
                let (u, v) = (a.get_pixel(x, y), b.get_pixel(x, y));
                let e: i32 = (0..3).map(|i| (i32::from(u[i]) - i32::from(v[i])).abs()).sum();
                let c = 255 - e.min(255) as u8;
                *p = Rgba([255, c, c, 255]);
            }
            d
        };
        println!("{n}: RustFig vs resvg: {}", stats(&ours, &r));
        if let (Some(i), Some(g)) = (&ink, &rsvg) {
            println!("{n}: resvg vs inkscape: {}", stats(&r, i));
            println!("{n}: resvg vs librsvg: {}", stats(&r, g));
            println!("{n}: inkscape vs librsvg: {}", stats(i, g));
        }
        // Tiles: resvg | inkscape | librsvg | RustFig | RustFig-vs-resvg diff, each at most 700 wide.
        let mut tiles: Vec<RgbaImage> = vec![r.clone()];
        tiles.extend(ink.clone());
        tiles.extend(rsvg.clone());
        tiles.push(ours.clone());
        tiles.push(diff_of(&ours, &r));
        let tw = 700.min(w);
        let th = (u64::from(tw) * u64::from(h) / u64::from(w)).max(1) as u32;
        let mut out = RgbaImage::from_pixel((tw + 6) * tiles.len() as u32, th, Rgba([128, 128, 128, 255]));
        for (k, t) in tiles.iter().enumerate() {
            let t = imageops::resize(t, tw, th, imageops::FilterType::Triangle);
            imageops::overlay(&mut out, &t, i64::from((tw + 6) * k as u32), 0);
        }
        out.save(format!("{dir}/{n}-cmp.png")).unwrap();
    }
}
