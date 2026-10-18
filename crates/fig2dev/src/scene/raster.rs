//! Raster backend (tiny-skia): canvas drawing and bitmap export.

use super::pattern::{coverage, PatternFrame};
use super::{textcache, Fill, Prim, Rgba, Scene, Stroke, SubPath, TextPrim};
use figbase::geom::{Affine, Point, Rect};
use figbase::image::RasterImage;
use figbase::style::{CapStyle, JoinStyle, Justify};
use figbase::text::PathOp;
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

fn paint(c: Rgba, anti_alias: bool) -> Paint<'static> {
    Paint {
        shader: Shader::SolidColor(color(c)),
        anti_alias,
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
    aa: bool,
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
        Some(Fill::Solid(c)) => pm.fill_path(&path, &paint(*c, aa), FillRule::EvenOdd, Transform::identity(), None),
        Some(Fill::Pattern { kind, fg, bg, nonzero }) => {
            let rule = if *nonzero { FillRule::Winding } else { FillRule::EvenOdd };
            pm.fill_path(&path, &paint(*bg, aa), rule, Transform::identity(), None);
            if let Some(mut mask) = Mask::new(pm.width(), pm.height()) {
                mask.fill_path(&path, rule, aa, Transform::identity());
                // Pattern lines over the visible part of the shape, in Fig units.
                if let Some(inv) = t.invert() {
                    let vis = Rect::from_points(bb.intersection_or_empty(&view).corners().map(|p| inv.apply(p)));
                    let (lines, line_width) = coverage(*kind, &vis, &opts.pattern);
                    let subs: Vec<SubPath> = lines
                        .into_iter()
                        .map(|pts| SubPath {
                            pts,
                            closed: false,
                            curve: None,
                        })
                        .collect();
                    if let Some(lp) = build_path(&subs, t) {
                        let st = tiny_skia::Stroke {
                            width: f((line_width * scale).max(0.5)),
                            ..tiny_skia::Stroke::default()
                        };
                        pm.stroke_path(&lp, &paint(*fg, aa), &st, Transform::identity(), Some(&mask));
                    }
                }
            }
        }
        None => {}
    }
    if let Some(s) = stroke {
        let st = sk_stroke(s, scale, opts.min_line_px);
        pm.stroke_path(&path, &paint(s.color, aa), &st, Transform::identity(), None);
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
    (ops, shaped.synthetic_bold, super::font_px(t.size) * 0.04)
}

fn draw_text(pm: &mut Pixmap, t: &TextPrim, opts: &RasterOptions, aa: bool) {
    let tr = &opts.transform;
    let view = Rect::new(Point::ZERO, Point::new(f64::from(pm.width()), f64::from(pm.height())));
    let bb = Rect::from_points(super::Prim::Text(t.clone()).bbox().corners().map(|p| tr.apply(p)));
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
    pm.fill_path(
        &path,
        &paint(t.color, aa),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    if bold {
        let st = tiny_skia::Stroke {
            width: f(bold_w * tr.mean_scale()),
            line_join: LineJoin::Round,
            ..tiny_skia::Stroke::default()
        };
        pm.stroke_path(&path, &paint(t.color, aa), &st, Transform::identity(), None);
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
        for [r, g, b, a] in data.as_chunks_mut::<4>().0.iter_mut() {
            let al = u16::from(*a);
            *r = ((u16::from(*r) * al + 127) / 255) as u8;
            *g = ((u16::from(*g) * al + 127) / 255) as u8;
            *b = ((u16::from(*b) * al + 127) / 255) as u8;
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

/// Draw a scene onto a pixmap, antialiased.
pub fn rasterize(scene: &Scene, pm: &mut Pixmap, opts: &RasterOptions) {
    rasterize_aa(scene, pm, opts, true);
}

/// Draw a scene onto a pixmap, antialiased or not (fig2dev's bitmaps
/// without smoothing are not).
pub fn rasterize_aa(scene: &Scene, pm: &mut Pixmap, opts: &RasterOptions, aa: bool) {
    for p in &scene.prims {
        match p {
            Prim::Path { subpaths, fill, stroke } => draw_path(pm, subpaths, fill.as_ref(), stroke.as_ref(), opts, aa),
            Prim::Text(t) => draw_text(pm, t, opts, aa),
            Prim::PictureRef { .. } => {}
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
    let (w, h) = checked_size(w, h)?;
    render_sized(
        scene,
        bounds,
        px_per_fig,
        (w, h),
        background,
        pattern,
        Smoothing::Antialias,
    )
}

/// A bitmap size in pixels, checked against the export limits.
pub fn checked_size(w: f64, h: f64) -> Result<(u32, u32), String> {
    if !(w.is_finite() && h.is_finite()) || w < 1.0 || h < 1.0 {
        return Err("image would be empty".into());
    }
    if w > f64::from(MAX_EDGE) || h > f64::from(MAX_EDGE) || w * h > MAX_PIXELS as f64 {
        return Err(format!("image too large ({w}x{h} pixels); lower the resolution"));
    }
    Ok((w as u32, h as u32))
}

/// How a bitmap's edges are smoothed (fig2dev's `-S`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Smoothing {
    /// tiny-skia's antialiasing (RustFig's default).
    Antialias,
    /// Hard edges (fig2dev without `-S`, or `-S 1`).
    None,
    /// Hard edges drawn at this many times the resolution, then averaged
    /// (`-S 2`, `-S 4`: ghostscript's `TextAlphaBits`/`GraphicsAlphaBits`).
    Supersample(u32),
}

/// Render the region `bounds` (Fig units) at `px_per_fig` into a pixmap of
/// `size` pixels, smoothed as asked.
pub fn render_sized(
    scene: &Scene,
    bounds: &Rect,
    px_per_fig: f64,
    size: (u32, u32),
    background: Option<Rgba>,
    pattern: PatternFrame,
    smoothing: Smoothing,
) -> Result<Pixmap, String> {
    if bounds.is_empty() || !(px_per_fig.is_finite() && px_per_fig > 0.0) {
        return Err("nothing to render".into());
    }
    let (w, h) = checked_size(f64::from(size.0), f64::from(size.1))?;
    let (k, aa) = match smoothing {
        Smoothing::Antialias => (1, true),
        Smoothing::None => (1, false),
        Smoothing::Supersample(k) => (k.clamp(1, 4), false),
    };
    let (kw, kh) = (w.saturating_mul(k), h.saturating_mul(k));
    if u64::from(kw) * u64::from(kh) > MAX_PIXELS {
        return Err(format!(
            "image too large to smooth ({w}x{h} pixels at {k}x); lower the resolution or the smoothing"
        ));
    }
    let mut pm = Pixmap::new(kw, kh).ok_or("cannot allocate image")?;
    if let Some(bg) = background {
        pm.fill(color(bg));
    }
    let ks = f64::from(k);
    let transform =
        Affine::translate(-bounds.min.x, -bounds.min.y).then(&Affine::scale(px_per_fig * ks, px_per_fig * ks));
    rasterize_aa(
        scene,
        &mut pm,
        &RasterOptions {
            transform,
            min_line_px: 0.5 * ks,
            pattern,
        },
        aa,
    );
    if k == 1 {
        Ok(pm)
    } else {
        downsample(&pm, k, w, h)
    }
}

/// Average `k`×`k` blocks of premultiplied pixels into a `w`×`h` pixmap.
fn downsample(src: &Pixmap, k: u32, w: u32, h: u32) -> Result<Pixmap, String> {
    let mut out = Pixmap::new(w, h).ok_or("cannot allocate image")?;
    let sw = src.width() as usize;
    let data = src.data();
    let (k, w) = (k as usize, w as usize);
    let n = (k * k) as u32;
    let dst = out.data_mut();
    for (y, row) in dst.chunks_exact_mut(w * 4).enumerate() {
        for (x, px) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let mut sum = [0u32; 4];
            for sy in y * k..(y + 1) * k {
                let start = (sy * sw + x * k) * 4;
                let Some(block) = data.get(start..start + k * 4) else {
                    continue;
                };
                for p in block.as_chunks::<4>().0 {
                    for (s, v) in sum.iter_mut().zip(p) {
                        *s += u32::from(*v);
                    }
                }
            }
            for (d, s) in px.iter_mut().zip(sum) {
                // Averages of premultiplied values stay premultiplied.
                *d = ((s + n / 2) / n) as u8;
            }
        }
    }
    Ok(out)
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
    for [r, g, b, a] in data.as_chunks_mut::<4>().0.iter_mut() {
        let al = u16::from(*a);
        *r = ((u16::from(*r) * al + 127) / 255) as u8;
        *g = ((u16::from(*g) * al + 127) / 255) as u8;
        *b = ((u16::from(*b) * al + 127) / 255) as u8;
    }
    let pm = Pixmap::from_vec(data, size).ok_or("cannot build image")?;
    encode_png(&pm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use figbase::color::Rgb;

    #[test]
    fn draws_a_filled_box() {
        let corners = [
            Point::new(0.0, 0.0),
            Point::new(1200.0, 0.0),
            Point::new(1200.0, 1200.0),
            Point::new(0.0, 1200.0),
        ];
        let scene = Scene {
            prims: vec![Prim::Path {
                subpaths: vec![SubPath {
                    pts: corners.to_vec(),
                    closed: true,
                    curve: None,
                }],
                fill: Some(Fill::Solid(Rgba::opaque(Rgb::new(255, 0, 0)))),
                stroke: None,
            }],
        };
        let pm = render_region(
            &scene,
            &Rect::from_points(corners),
            0.05,
            Some(Rgba::opaque(Rgb::WHITE)),
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
