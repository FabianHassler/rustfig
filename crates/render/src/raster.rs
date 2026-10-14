//! Raster backend (tiny-skia): canvas drawing and bitmap export.

use crate::pattern::{coverage, PatternFrame};
use crate::{textcache, Fill, Prim, Rgba, Scene, Stroke, SubPath, TextPrim};
use rustfig_geom::{Affine, Point, Rect};
use rustfig_model::{CapStyle, JoinStyle, Justify, RasterImage};
use rustfig_text::PathOp;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

pub use tiny_skia::Pixmap;
use tiny_skia::{
    Color, FillRule, IntSize, LineCap, LineJoin, Mask, Paint, Path, PathBuilder, PixmapPaint, Shader, StrokeDash,
    Transform,
};

/// Largest bitmap edge produced by exports, in pixels.
pub const MAX_EDGE: u32 = 20_000;
/// Largest bitmap area produced by exports, in pixels.
pub const MAX_PIXELS: u64 = 200_000_000;

/// Rasterisation parameters.
#[derive(Debug, Clone, Copy)]
pub struct RasterOptions {
    /// Fig units → device pixels.
    pub transform: Affine,
    /// Thinnest stroke drawn, in pixels (1.0 on screen so hairlines show).
    pub min_line_px: f64,
    /// Where fill patterns are anchored and how large their tiles are.
    pub pattern: PatternFrame,
}

fn color(c: Rgba) -> Color {
    Color::from_rgba8(c.r, c.g, c.b, c.a)
}

fn paint(c: Rgba) -> Paint<'static> {
    Paint {
        shader: Shader::SolidColor(color(c)),
        anti_alias: true,
        ..Paint::default()
    }
}

fn f(v: f64) -> f32 {
    if v.is_finite() {
        v.clamp(-1.0e7, 1.0e7) as f32
    } else {
        0.0
    }
}

fn build_path(subpaths: &[SubPath], t: &Affine) -> Option<Path> {
    let mut pb = PathBuilder::new();
    for s in subpaths {
        let mut it = s.pts.iter().map(|p| t.apply(*p));
        let Some(first) = it.next() else { continue };
        pb.move_to(f(first.x), f(first.y));
        for p in it {
            pb.line_to(f(p.x), f(p.y));
        }
        if s.closed {
            pb.close();
        }
    }
    pb.finish()
}

fn device_bbox(subpaths: &[SubPath], t: &Affine) -> Rect {
    let mut r = Rect::EMPTY;
    for s in subpaths {
        for p in &s.pts {
            r.add_point(t.apply(*p));
        }
    }
    r
}

fn sk_stroke(s: &Stroke, scale: f64, min_px: f64) -> tiny_skia::Stroke {
    let width = (s.width * scale).max(min_px);
    let dash = if s.dash.is_empty() {
        None
    } else {
        let d: Vec<f32> = s.dash.iter().map(|v| f((v * scale).max(0.01))).collect();
        StrokeDash::new(d, 0.0)
    };
    tiny_skia::Stroke {
        width: f(width),
        miter_limit: 4.0,
        line_cap: match s.cap {
            CapStyle::Butt => LineCap::Butt,
            CapStyle::Round => LineCap::Round,
            CapStyle::Projecting => LineCap::Square,
        },
        line_join: match s.join {
            JoinStyle::Miter => LineJoin::Miter,
            JoinStyle::Round => LineJoin::Round,
            JoinStyle::Bevel => LineJoin::Bevel,
        },
        dash,
    }
}

fn draw_path(
    pm: &mut Pixmap,
    subpaths: &[SubPath],
    fill: Option<&Fill>,
    stroke: Option<&Stroke>,
    opts: &RasterOptions,
) {
    let t = &opts.transform;
    let scale = t.mean_scale();
    let bb = device_bbox(subpaths, t);
    let sw = stroke.map(|s| (s.width * scale).max(opts.min_line_px)).unwrap_or(0.0);
    let view = Rect::new(Point::ZERO, Point::new(f64::from(pm.width()), f64::from(pm.height())));
    if !bb.expand(sw + 2.0).intersects(&view) {
        return;
    }
    let Some(path) = build_path(subpaths, t) else { return };
    match fill {
        Some(Fill::Solid(c)) => pm.fill_path(&path, &paint(*c), FillRule::EvenOdd, Transform::identity(), None),
        Some(Fill::Pattern { kind, fg, bg }) => {
            pm.fill_path(&path, &paint(*bg), FillRule::EvenOdd, Transform::identity(), None);
            if let Some(mut mask) = Mask::new(pm.width(), pm.height()) {
                mask.fill_path(&path, FillRule::EvenOdd, true, Transform::identity());
                // Pattern lines over the visible part of the shape, in Fig units.
                if let Some(inv) = t.invert() {
                    let vis = Rect::from_points(bb.intersection_or_empty(&view).corners().map(|p| inv.apply(p)));
                    let (lines, line_width) = coverage(*kind, &vis, &opts.pattern);
                    let subs: Vec<SubPath> = lines.into_iter().map(|pts| SubPath { pts, closed: false }).collect();
                    if let Some(lp) = build_path(&subs, t) {
                        let st = tiny_skia::Stroke {
                            width: f((line_width * scale).max(0.5)),
                            ..tiny_skia::Stroke::default()
                        };
                        pm.stroke_path(&lp, &paint(*fg), &st, Transform::identity(), Some(&mask));
                    }
                }
            }
        }
        None => {}
    }
    if let Some(s) = stroke {
        let st = sk_stroke(s, scale, opts.min_line_px);
        pm.stroke_path(&path, &paint(s.color), &st, Transform::identity(), None);
    }
}

/// Text outlines in Fig units, justified, rotated and placed at the origin.
pub(crate) fn text_ops_fig(t: &TextPrim) -> (Vec<PathOp>, bool, f64) {
    let shaped = textcache::shaped(t);
    let rot = Affine::rotate(t.angle);
    let mut ops = Vec::new();
    let map = |x: f64, y: f64, dx: f64| {
        let p = t.origin + rot.apply_vec(Point::new(x + dx, y));
        (p.x, p.y)
    };
    for (line, w) in shaped.lines.iter().zip(shaped.line_widths.iter()) {
        let dx = match t.justify {
            Justify::Left => 0.0,
            Justify::Center => -w / 2.0,
            Justify::Right => -w,
        };
        for op in line {
            ops.push(match *op {
                PathOp::Move(x, y) => {
                    let (x, y) = map(x, y, dx);
                    PathOp::Move(x, y)
                }
                PathOp::Line(x, y) => {
                    let (x, y) = map(x, y, dx);
                    PathOp::Line(x, y)
                }
                PathOp::Quad(a, b, x, y) => {
                    let (a, b) = map(a, b, dx);
                    let (x, y) = map(x, y, dx);
                    PathOp::Quad(a, b, x, y)
                }
                PathOp::Cubic(a, b, c, d, x, y) => {
                    let (a, b) = map(a, b, dx);
                    let (c, d) = map(c, d, dx);
                    let (x, y) = map(x, y, dx);
                    PathOp::Cubic(a, b, c, d, x, y)
                }
                PathOp::Close => PathOp::Close,
            });
        }
    }
    (ops, shaped.synthetic_bold, crate::font_px(t.size) * 0.04)
}

fn draw_text(pm: &mut Pixmap, t: &TextPrim, opts: &RasterOptions) {
    let tr = &opts.transform;
    let view = Rect::new(Point::ZERO, Point::new(f64::from(pm.width()), f64::from(pm.height())));
    let bb = Rect::from_points(crate::Prim::Text(t.clone()).bbox().corners().map(|p| tr.apply(p)));
    if !bb.intersects(&view) {
        return;
    }
    let (ops, bold, bold_w) = text_ops_fig(t);
    let mut pb = PathBuilder::new();
    let m = |x: f64, y: f64| {
        let p = tr.apply(Point::new(x, y));
        (f(p.x), f(p.y))
    };
    for op in ops {
        match op {
            PathOp::Move(x, y) => {
                let (x, y) = m(x, y);
                pb.move_to(x, y);
            }
            PathOp::Line(x, y) => {
                let (x, y) = m(x, y);
                pb.line_to(x, y);
            }
            PathOp::Quad(a, b, x, y) => {
                let (a, b) = m(a, b);
                let (x, y) = m(x, y);
                pb.quad_to(a, b, x, y);
            }
            PathOp::Cubic(a, b, c, d, x, y) => {
                let (a, b) = m(a, b);
                let (c, d) = m(c, d);
                let (x, y) = m(x, y);
                pb.cubic_to(a, b, c, d, x, y);
            }
            PathOp::Close => pb.close(),
        }
    }
    let Some(path) = pb.finish() else { return };
    pm.fill_path(&path, &paint(t.color), FillRule::Winding, Transform::identity(), None);
    if bold {
        let st = tiny_skia::Stroke {
            width: f(bold_w * tr.mean_scale()),
            line_join: LineJoin::Round,
            ..tiny_skia::Stroke::default()
        };
        pm.stroke_path(&path, &paint(t.color), &st, Transform::identity(), None);
    }
}

thread_local! {
    static IMAGES: RefCell<HashMap<usize, (Arc<RasterImage>, Pixmap)>> = RefCell::new(HashMap::new());
}

/// Premultiplied pixmap for an image (cached by identity).
fn image_pixmap(img: &Arc<RasterImage>) -> Option<Pixmap> {
    let key = Arc::as_ptr(img) as usize;
    IMAGES.with(|c| {
        let mut c = c.borrow_mut();
        if let Some((_, pm)) = c.get(&key) {
            return Some(pm.clone());
        }
        let size = IntSize::from_wh(img.width, img.height)?;
        let expected = (img.width as usize).checked_mul(img.height as usize)?.checked_mul(4)?;
        if img.rgba.len() != expected {
            return None;
        }
        let mut data = img.rgba.clone();
        for px in data.chunks_exact_mut(4) {
            if let [r, g, b, a] = px {
                let al = u16::from(*a);
                *r = ((u16::from(*r) * al + 127) / 255) as u8;
                *g = ((u16::from(*g) * al + 127) / 255) as u8;
                *b = ((u16::from(*b) * al + 127) / 255) as u8;
            }
        }
        let pm = Pixmap::from_vec(data, size)?;
        if c.len() > 64 {
            c.clear();
        }
        c.insert(key, (img.clone(), pm.clone()));
        Some(pm)
    })
}

fn draw_image(pm: &mut Pixmap, corners: &[Point; 4], img: &Arc<RasterImage>, flipped: bool, opts: &RasterOptions) {
    let t = &opts.transform;
    let view = Rect::new(Point::ZERO, Point::new(f64::from(pm.width()), f64::from(pm.height())));
    let bb = Rect::from_points(corners.iter().map(|p| t.apply(*p)));
    if !bb.intersects(&view) || img.width == 0 || img.height == 0 {
        return;
    }
    let Some(src) = image_pixmap(img) else { return };
    let [tl, tr, _br, bl] = corners.map(|p| t.apply(p));
    // Image x axis → top edge, y axis → left edge (swapped when flipped).
    let (ex, ey) = if flipped {
        (bl - tl, tr - tl)
    } else {
        (tr - tl, bl - tl)
    };
    let sx = ex * (1.0 / f64::from(img.width));
    let sy = ey * (1.0 / f64::from(img.height));
    let ts = Transform::from_row(f(sx.x), f(sx.y), f(sy.x), f(sy.y), f(tl.x), f(tl.y));
    let paint = PixmapPaint {
        quality: tiny_skia::FilterQuality::Bilinear,
        ..PixmapPaint::default()
    };
    pm.draw_pixmap(0, 0, src.as_ref(), &paint, ts, None);
}

/// Draw a scene onto a pixmap.
pub fn rasterize(scene: &Scene, pm: &mut Pixmap, opts: &RasterOptions) {
    for p in &scene.prims {
        match p {
            Prim::Path { subpaths, fill, stroke } => draw_path(pm, subpaths, fill.as_ref(), stroke.as_ref(), opts),
            Prim::Text(t) => draw_text(pm, t, opts),
            Prim::Image {
                corners,
                image,
                flipped,
            } => draw_image(pm, corners, image, *flipped, opts),
        }
    }
}

/// Render the region `bounds` (Fig units) at `px_per_fig` into a pixmap,
/// with fill patterns in `pattern`.
pub fn render_region(
    scene: &Scene,
    bounds: &Rect,
    px_per_fig: f64,
    background: Option<Rgba>,
    pattern: PatternFrame,
) -> Result<Pixmap, String> {
    if bounds.is_empty() || !(px_per_fig.is_finite() && px_per_fig > 0.0) {
        return Err("nothing to render".into());
    }
    let w = (bounds.width() * px_per_fig).ceil();
    let h = (bounds.height() * px_per_fig).ceil();
    if !(w.is_finite() && h.is_finite()) || w < 1.0 || h < 1.0 {
        return Err("image would be empty".into());
    }
    if w > f64::from(MAX_EDGE) || h > f64::from(MAX_EDGE) || w * h > MAX_PIXELS as f64 {
        return Err(format!("image too large ({w}x{h} pixels); lower the resolution"));
    }
    let mut pm = Pixmap::new(w as u32, h as u32).ok_or("cannot allocate image")?;
    if let Some(bg) = background {
        pm.fill(color(bg));
    }
    let transform = Affine::translate(-bounds.min.x, -bounds.min.y).then(&Affine::scale(px_per_fig, px_per_fig));
    rasterize(
        scene,
        &mut pm,
        &RasterOptions {
            transform,
            min_line_px: 0.5,
            pattern,
        },
    );
    Ok(pm)
}

/// Un-premultiplied RGBA bytes of a pixmap.
pub fn pixmap_rgba(pm: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pm.data().len());
    for p in pm.pixels() {
        let c = p.demultiply();
        out.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    out
}

/// Encode a pixmap as PNG.
pub fn encode_png(pm: &Pixmap) -> Result<Vec<u8>, String> {
    pm.encode_png().map_err(|e| e.to_string())
}

/// Encode straight (un-premultiplied) RGBA pixels as PNG.
pub fn encode_rgba_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let size = IntSize::from_wh(width, height).ok_or("empty image")?;
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or("image too large")?;
    if rgba.len() != expected {
        return Err("pixel buffer has the wrong size".into());
    }
    let mut data = rgba.to_vec();
    for px in data.chunks_exact_mut(4) {
        if let [r, g, b, a] = px {
            let al = u16::from(*a);
            *r = ((u16::from(*r) * al + 127) / 255) as u8;
            *g = ((u16::from(*g) * al + 127) / 255) as u8;
            *b = ((u16::from(*b) * al + 127) / 255) as u8;
        }
    }
    let pm = Pixmap::from_vec(data, size).ok_or("cannot build image")?;
    encode_png(&pm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SceneOptions;
    use rustfig_model::{Document, Object, ObjectKind, Polyline, Style};

    #[test]
    fn draws_a_filled_box() {
        let mut doc = Document::new();
        let mut s = Style::default();
        s.area_fill = 20;
        s.fill_color = rustfig_model::ColorRef(4);
        doc.objects.push(Object::new(ObjectKind::Polyline(Polyline::rect(
            s,
            Point::new(0.0, 0.0),
            Point::new(1200.0, 1200.0),
        ))));
        let scene = Scene::build(&doc, &SceneOptions::default());
        let pm = render_region(
            &scene,
            &doc.bbox(),
            0.05,
            Some(Rgba::opaque(rustfig_model::Rgb::WHITE)),
            PatternFrame::screen(),
        )
        .unwrap();
        let mid = pm.pixel(pm.width() / 2, pm.height() / 2).unwrap();
        assert_eq!((mid.red(), mid.green(), mid.blue()), (255, 0, 0));
        assert!(!encode_png(&pm).unwrap().is_empty());
    }

    #[test]
    fn refuses_huge_images() {
        let scene = Scene::default();
        let r = Rect::new(Point::ZERO, Point::new(1e9, 1e9));
        assert!(render_region(&scene, &r, 1.0, None, PatternFrame::screen()).is_err());
    }
}
