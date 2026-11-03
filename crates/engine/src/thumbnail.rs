//! File-manager thumbnails: a figure fitted into N x N pixels as a PNG.
//!
//! Used by `rustfig-cli thumbnail` and `rustfig thumbnail`, which the
//! freedesktop thumbnailer entry (`rustfig.thumbnailer`) runs. Reads what
//! `file.open` reads (`.fig`, compressed `.fig.gz`/`.Z`/`.bz2`/`.xz`,
//! `.rustfig`).

use rustfig_geom::{Affine, Rect};
use rustfig_render::raster::{self, RasterOptions};
use rustfig_render::SceneOptions;
use std::path::Path;
use std::sync::Arc;

/// Largest thumbnail edge accepted, in pixels.
pub const MAX_SIZE: u32 = 4096;
/// Default thumbnail edge, in pixels.
pub const DEFAULT_SIZE: u32 = 256;

/// Width and height from a PNG's IHDR chunk.
#[cfg(test)]
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
    let png = make_png(Path::new(input), size)?;
    std::fs::write(output, png).map_err(|e| format!("{output}: {e}"))
}

/// The thumbnail as PNG bytes (the Quick Look extensions and the thumbnailer).
///
/// Drawn directly, without an [`Engine`](crate::Engine): opening a figure and
/// exporting it by command would keep every command and export driver in the
/// Quick Look extensions (13 MB each instead of a few). Reads what `file.open`
/// reads (pictures included) and draws as the canvas does, once, at the size
/// asked for.
pub fn make_png(input: &Path, size: u32) -> Result<Vec<u8>, String> {
    // A bug in the reader or the renderer must not take the Quick Look
    // extension or the thumbnailer down: an error instead (as Engine::execute).
    crate::guard::catch(|| draw_png(input, size)).and_then(|r| r)
}

/// Texts with the hidden flag are left out, as fig2dev's exports leave them out.
fn drop_hidden_texts(objects: &mut Vec<Arc<rustfig_model::Object>>) {
    use rustfig_model::ObjectKind;
    fn keep(o: &rustfig_model::Object) -> bool {
        !matches!(&o.kind, ObjectKind::Text(t) if t.is_hidden())
    }
    fn walk(o: &mut rustfig_model::Object) {
        if let ObjectKind::Compound(c) = &mut o.kind {
            c.objects.retain(keep);
            c.objects.iter_mut().for_each(walk);
        }
    }
    objects.retain(|o| keep(o));
    for o in objects.iter_mut() {
        walk(Arc::make_mut(o));
    }
}

fn draw_png(input: &Path, size: u32) -> Result<Vec<u8>, String> {
    if !(1..=MAX_SIZE).contains(&size) {
        return Err(format!("size must be 1 to {MAX_SIZE}"));
    }
    let name = input.display().to_string();
    let bytes = std::fs::read(input).map_err(|e| format!("{name}: {e}"))?;
    let bytes = rustfig_format::unpack_file(&bytes, &name).map_err(|e| format!("{name}: {e}"))?;
    let (mut doc, _) = rustfig_format::read_any_messages(&bytes).map_err(|e| e.to_string())?;
    for o in &mut doc.objects {
        crate::fix_texts(Arc::make_mut(o));
    }
    let _ = crate::import::load_all_pictures(&mut doc, input.parent()); // a missing picture draws as its box
                                                                        // The drawing's box as fig2dev frames it (what the exports did before).
    let mut ctx = rustfig_format::bridge::from_model::context(&doc);
    let r = doc.objects.iter().fold(Rect::EMPTY, |r, o| {
        r.union(&rustfig_format::bridge::object_bounds(o, &mut ctx))
    });
    let (fw, fh) = (r.width(), r.height());
    if r.is_empty() || !(fw.is_finite() && fh.is_finite()) || fw.max(fh) <= 0.0 {
        return Err("empty drawing".into());
    }
    let scale = f64::from(size) / fw.max(fh);
    let w = ((fw * scale).ceil() as u32).clamp(1, size);
    let h = ((fh * scale).ceil() as u32).clamp(1, size);
    // After the bounds: fig2dev frames hidden texts but does not draw them.
    drop_hidden_texts(&mut doc.objects);
    let scene = rustfig_render::build_scene(
        &doc,
        &SceneOptions {
            // LaTeX text as a readable approximation: no TeX in a thumbnail.
            approx_latex: true,
            ..SceneOptions::default()
        },
    );
    let opts = RasterOptions {
        transform: Affine::translate(-r.min.x, -r.min.y).then(&Affine::scale(scale, scale)),
        min_line_px: 1.0,
        adjust_px: 0.0,
        pattern_k: 1,
        pattern: rustfig_render::pattern::PatternFrame::screen(),
    };
    let pm = rustfig_render::rasterize_banded(&scene, w, h, &opts).ok_or("the thumbnail is too large")?;
    // Premultiplied RGBA over white.
    let rgba: Vec<u8> = pm
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, a]| {
            let back = 255 - a;
            [
                r.saturating_add(back),
                g.saturating_add(back),
                b.saturating_add(back),
                255,
            ]
        })
        .collect();
    raster::write_png(w, h, &rgba)
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

    /// A hidden text is framed (fig2dev's bounds count it) but not drawn, as
    /// in fig2dev's exports; the canvas would show it.
    #[test]
    fn hidden_texts_are_not_drawn() {
        crate::text::set_builtin_fonts(true);
        let dir = std::env::temp_dir().join(format!("rf-thumb-hidden-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fig = |flags: i32| {
            format!(
                "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
                 2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 600 0 600 600 0 600 0 0\n\
                 4 0 0 50 -1 0 40 0.0000 {flags} 400 3000 3000 600 HIDDENTEXT\\001\n"
            )
        };
        let ink = |flags: i32| {
            let path = dir.join(format!("f{flags}.fig"));
            std::fs::write(&path, fig(flags)).unwrap();
            let png = make_png(&path, 128).unwrap();
            let img = image::load_from_memory(&png).unwrap().to_luma8();
            (img.width(), img.pixels().filter(|p| p.0[0] < 128).count())
        };
        let (w_shown, dark_shown) = ink(4); // a plain text
        let (w_hidden, dark_hidden) = ink(4 | 8); // the hidden flag (8)
        assert_eq!(w_shown, w_hidden, "the same framing");
        assert!(
            dark_hidden < dark_shown,
            "the hidden text is not drawn: {dark_hidden} vs {dark_shown}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

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
