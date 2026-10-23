//! RustFig rendering (layer L3): the scene of a document.
//!
//! [`build_scene`] turns a document into fig2dev's display list
//! ([`Scene`], from `fig2dev`), which the canvas rasterises
//! ([`raster`], the same rasteriser fig2dev's bitmap languages use). The
//! exports don't come from here: the engine writes the document as Fig
//! text and runs fig2dev (plan B12), whose scene builder draws fig2dev's
//! objects with the same primitives.
//
// Ported from xfig src/u_redraw.c (drawing order, gray depths) and fig2dev
// dev/genps.c (`PSFONTMAG` text sizes, arrows on pie arcs) (xfig licence,
// see LICENSE-xfig).

pub use fig2dev::scene::{
    font_px, head_at_end, line_width, pattern, raster, Fill, Prim, Rgba, Scene, Stroke, SubPath, TextPrim,
};

use fig2dev::scene::{fill, picture_prims, shape_prims, stroke, Shape};
use rustfig_geom::{ArrowSpec, Point, Rect, Seg};
use rustfig_model::{flatten_object, Arrow, ColorRef, Document, Object, ObjectKind, PolyKind, Style};
use std::collections::BTreeSet;
use std::sync::Arc;

/// What a typesetter is asked for: one special (LaTeX) text.
#[derive(Debug, Clone, PartialEq)]
pub struct TypesetRequest<'a> {
    pub text: &'a str,
    pub font: i32,
    /// Fig fonts of the PostScript set rather than the LaTeX set.
    pub postscript: bool,
    /// Points.
    pub size: f64,
    pub color: Rgba,
}

/// A special text typeset by TeX: the picture and its size (F8).
#[derive(Debug, Clone)]
pub struct Typeset {
    /// TeX's page: the text's box with `margin_pt` on every side (ink may
    /// reach outside the box); vector, so it stays sharp at every zoom (the
    /// rasteriser renders it at the size shown).
    pub pdf: Arc<fig2dev::util::pdfpic::PdfPicture>,
    /// Width, height (including depth) and depth below the baseline, in
    /// PostScript points.
    pub width_pt: f64,
    pub height_pt: f64,
    pub depth_pt: f64,
    /// Room around the box on the page, in PostScript points.
    pub margin_pt: f64,
}

/// Source of typeset special texts. L3 never starts a process: the engine
/// supplies one that runs TeX in the background and answers `None` until
/// the picture is ready (or when TeX is missing or failed).
pub trait Typesetter: Send + Sync + std::fmt::Debug {
    fn typeset(&self, req: &TypesetRequest<'_>) -> Option<Typeset>;
}

/// Options for building a scene.
#[derive(Debug, Clone, Default)]
pub struct SceneOptions {
    /// Draw special texts as TeX typeset them where the typesetter has them
    /// (canvas only; exports never set it).
    pub typeset: Option<Arc<dyn Typesetter>>,
    /// Depths that are off in the depth manager.
    pub hidden_depths: BTreeSet<i32>,
    /// Draw objects on hidden depths in gray instead of leaving them out
    /// (xfig's depth panel "Gray" setting).
    pub gray_hidden: bool,
    /// Draw for the screen: hidden texts appear as a placeholder.
    pub screen: bool,
    /// Factor on font sizes; `None` is 1. Exports of metric drawings use
    /// 76.2/80 so text keeps its point size while the drawing is magnified
    /// by 80/76.2 (fig2dev with xfig's `-F`, `genps.c` `PSFONTMAG`).
    pub text_scale: Option<f64>,
}

/// The scene of a document: its objects back to front, as xfig draws them.
///
/// With compounds open (`Document::open`) the scene is the open compound;
/// where it was opened with "keep visible" the rest of the figure is drawn
/// behind it, all gray (`redisplay_objects` in xfig's `u_redraw.c`).
pub fn build_scene(doc: &Document, opts: &SceneOptions) -> Scene {
    let mut backdrop: Vec<std::sync::Arc<Object>> = Vec::new();
    for level in doc.open.iter().rev() {
        if !level.keep_visible {
            break;
        }
        backdrop.extend(level.parent.iter().cloned());
    }
    let mut prims = Vec::new();
    if !backdrop.is_empty() {
        let behind = Document {
            settings: doc.settings.clone(),
            colors: doc.colors.clone(),
            objects: backdrop,
            ..Document::default()
        };
        prims = scene_prims(&behind, opts);
        prims.iter_mut().for_each(Prim::gray_out);
    }
    prims.extend(scene_prims(doc, opts));
    Scene { prims }
}

fn scene_prims(doc: &Document, opts: &SceneOptions) -> Vec<Prim> {
    let mut leaves: Vec<(&Object, bool)> = Vec::with_capacity(doc.leaf_count());
    doc.for_each_leaf_xfig_order(&mut |o| {
        let hidden = opts.hidden_depths.contains(&o.depth());
        if !hidden || opts.gray_hidden {
            leaves.push((o, hidden));
        }
    });
    // xfig's order: depths that are off (gray) behind everything, then
    // back to front; within a depth, the stable sort keeps xfig's type
    // order from the walk above (`u_redraw.c` `redisplay_objects`).
    leaves.sort_by_key(|(o, gray)| (!*gray, std::cmp::Reverse(o.depth())));
    let mut prims = Vec::with_capacity(leaves.len());
    for (o, gray) in leaves {
        let first = prims.len();
        object_prims(doc, o, opts, &mut prims);
        if gray {
            if let Some(new) = prims.get_mut(first..) {
                new.iter_mut().for_each(Prim::gray_out);
            }
        }
    }
    prims
}

fn color(doc: &Document, c: ColorRef) -> Rgba {
    Rgba::opaque(doc.rgb(c))
}

fn fill_for(doc: &Document, s: &Style) -> Option<Fill> {
    fill(
        s.area_fill,
        color(doc, s.pen_color),
        doc.rgb(s.fill_color),
        s.fill_color.is_default_or_black(),
        s.fill_color == ColorRef::WHITE,
    )
}

/// The stroke for an object's style. `round_dots`: fig2dev draws dotted
/// ellipses and closed splines with round caps (dots); other objects use
/// their cap style.
fn stroke_for(doc: &Document, s: &Style, round_dots: bool) -> Option<Stroke> {
    stroke(
        s.thickness,
        s.line,
        s.style_val,
        color(doc, s.pen_color),
        s.cap,
        s.join,
        round_dots,
    )
}

/// An arrowhead as the geometry draws it.
pub fn arrow_spec(a: &Arrow) -> ArrowSpec {
    ArrowSpec {
        kind: a.kind,
        style: a.style,
        thickness: a.thickness,
        width: a.width,
        height: a.height,
    }
}

/// The corners of a typeset text's box (top-left, top-right, bottom-right,
/// bottom-left of the unrotated text) at the text's anchor, justification
/// and angle; `pad` widens it on every side (Fig units).
pub fn typeset_corners(
    origin: Point,
    angle: f64,
    justify: rustfig_model::Justify,
    ts: &Typeset,
    pad: f64,
) -> [Point; 4] {
    const FIG_PER_PT: f64 = 1200.0 / 72.0;
    let angle = if angle.is_finite() { angle } else { 0.0 };
    let (w, h, d) = (
        ts.width_pt * FIG_PER_PT,
        ts.height_pt * FIG_PER_PT,
        ts.depth_pt * FIG_PER_PT,
    );
    let u0 = match justify {
        rustfig_model::Justify::Left => 0.0,
        rustfig_model::Justify::Center => -w / 2.0,
        rustfig_model::Justify::Right => -w,
    };
    let (c, s) = (angle.cos(), angle.sin());
    // u along the baseline, v up (Fig's y grows downwards).
    let at = |u: f64, v: f64| Point::new(origin.x + u * c - v * s, origin.y - u * s - v * c);
    let pad = if pad.is_finite() { pad.max(0.0) } else { 0.0 };
    let (u0, u1) = (u0 - pad, u0 + w + pad);
    let (v0, v1) = (-d - pad, h - d + pad);
    [at(u0, v1), at(u1, v1), at(u1, v0), at(u0, v0)]
}

/// The typeset text as a vector picture at the text's anchor, justification
/// and angle.
fn typeset_prim(origin: Point, angle: f64, justify: rustfig_model::Justify, ts: &Typeset) -> Prim {
    Prim::PdfPicture {
        corners: typeset_corners(origin, angle, justify, ts, ts.margin_pt * 1200.0 / 72.0),
        flipped: false,
        pdf: ts.pdf.clone(),
    }
}

/// A text in an installed font (F7) as glyph outlines, or `None` when it has
/// none or the font is missing (the Fig font is drawn then).
fn local_text_prim(doc: &Document, t: &rustfig_model::Text, text: &str, opts: &SceneOptions) -> Option<Prim> {
    use rustfig_text::PathOp;
    let name = t.local_font.as_deref()?;
    let size = t.size * opts.text_scale.filter(|s| s.is_finite() && *s > 0.0).unwrap_or(1.0);
    let shaped = rustfig_text::shape_local(name, size, text)?;
    let angle = if t.angle.is_finite() { t.angle } else { 0.0 };
    let rot = rustfig_geom::Affine::rotate(angle);
    let mut subpaths: Vec<SubPath> = Vec::new();
    for (line, w) in shaped.lines.iter().zip(shaped.line_widths.iter()) {
        let dx = match t.justify {
            rustfig_model::Justify::Left => 0.0,
            rustfig_model::Justify::Center => -w / 2.0,
            rustfig_model::Justify::Right => -w,
        };
        let map = |x: f64, y: f64| t.origin + rot.apply_vec(Point::new(x + dx, y));
        let mut cur: Option<(Vec<Point>, Vec<Seg>)> = None;
        let mut last = Point::new(0.0, 0.0);
        let mut finish = |c: &mut Option<(Vec<Point>, Vec<Seg>)>| {
            if let Some((pts, curve)) = c.take() {
                if pts.len() > 2 {
                    subpaths.push(SubPath {
                        pts,
                        closed: true,
                        curve: Some(curve),
                    });
                }
            }
        };
        for op in line {
            match *op {
                PathOp::Move(x, y) => {
                    finish(&mut cur);
                    last = map(x, y);
                    cur = Some((vec![last], vec![Seg::Move(last)]));
                }
                PathOp::Line(x, y) => {
                    if let Some((pts, curve)) = cur.as_mut() {
                        last = map(x, y);
                        pts.push(last);
                        curve.push(Seg::Line(last));
                    }
                }
                PathOp::Quad(a, b, x, y) => {
                    if let Some((pts, curve)) = cur.as_mut() {
                        let (q, e) = (map(a, b), map(x, y));
                        // Elevate to a cubic.
                        let c1 = last + (q - last) * (2.0 / 3.0);
                        let c2 = e + (q - e) * (2.0 / 3.0);
                        flatten_cubic(last, c1, c2, e, pts);
                        curve.push(Seg::Cubic(c1, c2, e));
                        last = e;
                    }
                }
                PathOp::Cubic(a, b, c, d, x, y) => {
                    if let Some((pts, curve)) = cur.as_mut() {
                        let (c1, c2, e) = (map(a, b), map(c, d), map(x, y));
                        flatten_cubic(last, c1, c2, e, pts);
                        curve.push(Seg::Cubic(c1, c2, e));
                        last = e;
                    }
                }
                PathOp::Close => finish(&mut cur),
            }
        }
        finish(&mut cur);
    }
    if subpaths.is_empty() {
        return None;
    }
    Some(Prim::Path {
        subpaths,
        fill: Some(Fill::Solid(color(doc, t.color))),
        stroke: None,
    })
}

fn flatten_cubic(p0: Point, p1: Point, p2: Point, p3: Point, out: &mut Vec<Point>) {
    const STEPS: usize = 8;
    for i in 1..=STEPS {
        let t = i as f64 / STEPS as f64;
        let u = 1.0 - t;
        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        out.push(Point::new(
            a * p0.x + b * p1.x + c * p2.x + d * p3.x,
            a * p0.y + b * p1.y + c * p2.y + d * p3.y,
        ));
    }
}

fn object_prims(doc: &Document, o: &Object, opts: &SceneOptions, out: &mut Vec<Prim>) {
    match &o.kind {
        ObjectKind::Text(t) => {
            let text = if opts.screen && t.is_hidden() {
                "<<>>".to_string()
            } else {
                t.text.clone()
            };
            if opts.screen && t.is_special() && !t.is_hidden() {
                if let Some(ts) = opts.typeset.as_ref().and_then(|ty| {
                    ty.typeset(&TypesetRequest {
                        text: &t.text,
                        font: t.font,
                        postscript: t.is_postscript(),
                        size: t.size,
                        color: color(doc, t.color),
                    })
                }) {
                    out.push(typeset_prim(t.origin, t.angle, t.justify, &ts));
                    return;
                }
            }
            if let Some(prim) = local_text_prim(doc, t, &text, opts) {
                out.push(prim);
                return;
            }
            out.push(Prim::Text(TextPrim {
                origin: t.origin,
                angle: if t.angle.is_finite() { t.angle } else { 0.0 },
                size: t.size * opts.text_scale.filter(|s| s.is_finite() && *s > 0.0).unwrap_or(1.0),
                justify: t.justify,
                font: t.font,
                postscript: t.is_postscript(),
                text,
                color: color(doc, t.color),
                special: t.is_special(),
            }));
        }
        ObjectKind::Polyline(p) if p.kind == PolyKind::Picture => {
            let r = Rect::from_points(p.points.iter().copied());
            if r.is_empty() {
                return;
            }
            let pic = p.picture.as_ref();
            let frame = if p.style.thickness > 0 {
                stroke_for(doc, &p.style, false)
            } else {
                None
            };
            picture_prims(
                r.corners(),
                pic.and_then(|pic| pic.image.clone()),
                pic.is_some_and(|pic| pic.flipped),
                pic.map(|pic| pic.file.as_str()).unwrap_or_default(),
                frame,
                out,
            );
        }
        ObjectKind::Compound(_) => {}
        _ => {
            let Some(style) = o.style() else { return };
            let round_dots = match &o.kind {
                ObjectKind::Ellipse(_) => true,
                ObjectKind::Spline(s) => s.closed,
                _ => false,
            };
            let (fwd, back) = match &o.kind {
                ObjectKind::Polyline(p) if p.kind == PolyKind::Polyline => (p.forward_arrow, p.backward_arrow),
                ObjectKind::Spline(s) if !s.closed => (s.forward_arrow, s.backward_arrow),
                ObjectKind::Arc(a) => (a.forward_arrow, a.backward_arrow),
                _ => (None, None),
            };
            // Pie wedges carry their arrows on the arc (fig2dev draws them).
            let pie_arc = match &o.kind {
                ObjectKind::Arc(a) if a.pie && (fwd.is_some() || back.is_some()) => {
                    let mut open = o.clone();
                    if let ObjectKind::Arc(oa) = &mut open.kind {
                        oa.pie = false;
                    }
                    flatten_object(&open).into_iter().next().map(|p| p.pts)
                }
                _ => None,
            };
            shape_prims(
                Shape {
                    paths: flatten_object(o).into_iter().map(|p| (p.pts, p.closed)).collect(),
                    exact: exact_outline(o),
                    fill: fill_for(doc, style),
                    stroke: stroke_for(doc, style, round_dots),
                    pen: color(doc, style.pen_color),
                    thickness: f64::from(style.thickness),
                    forward_arrow: fwd.as_ref().map(arrow_spec),
                    backward_arrow: back.as_ref().map(arrow_spec),
                    poly: matches!(&o.kind, ObjectKind::Polyline(_)),
                    pie_arc,
                },
                out,
            );
        }
    }
}

/// An object's outline as lines and curves, where it has curves (ellipses,
/// arcs, rounded boxes); others are exact as their points.
fn exact_outline(o: &Object) -> Option<Vec<Seg>> {
    match &o.kind {
        ObjectKind::Ellipse(e) => Some(rustfig_geom::ellipse_path(e.center, e.radii.x, e.radii.y, e.angle)),
        ObjectKind::Arc(a) => {
            let g = rustfig_geom::arc_from_three_points(a.points[0], a.points[1], a.points[2])?;
            Some(rustfig_geom::arc_path(&rustfig_model::arc_geom(a, g), a.pie))
        }
        ObjectKind::Polyline(p) if p.kind == PolyKind::ArcBox => {
            let r = Rect::from_points(p.points.iter().copied());
            Some(rustfig_geom::round_rect_path(
                &r,
                f64::from(p.radius.max(0)) * rustfig_geom::FIG_PER_STYLE_UNIT,
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustfig_geom::Point;
    use rustfig_model::{Polyline, Style};

    #[test]
    fn depth_order() {
        let mut doc = Document::new();
        let mut front = Style::default();
        front.depth = 10;
        let mut back = Style::default();
        back.depth = 90;
        back.pen_color = ColorRef(4);
        doc.objects
            .push(std::sync::Arc::new(Object::new(ObjectKind::Polyline(Polyline::rect(
                front,
                Point::ZERO,
                Point::new(10.0, 10.0),
            )))));
        doc.objects
            .push(std::sync::Arc::new(Object::new(ObjectKind::Polyline(Polyline::rect(
                back,
                Point::ZERO,
                Point::new(10.0, 10.0),
            )))));
        let s = build_scene(&doc, &SceneOptions::default());
        assert_eq!(s.prims.len(), 2);
        let Prim::Path { stroke: Some(st), .. } = &s.prims[0] else {
            panic!()
        };
        assert_eq!(st.color.r, 255);
    }

    fn rect_at(depth: i32, pen: i32, x: f64) -> Object {
        let mut st = Style::default();
        st.depth = depth;
        st.pen_color = ColorRef(pen);
        Object::new(ObjectKind::Polyline(Polyline::rect(
            st,
            Point::new(x, 0.0),
            Point::new(x + 10.0, 10.0),
        )))
    }

    fn stroke_pens(s: &Scene) -> Vec<u8> {
        s.prims
            .iter()
            .filter_map(|p| match p {
                Prim::Path { stroke: Some(st), .. } => Some(st.color.r),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn xfig_draw_order_within_a_depth() {
        use rustfig_model::{Arc as FigArc, Compound};
        // Same depth: a line listed before an arc, and a compound holding a
        // line. xfig draws arcs, then compounds, then lines.
        let mut doc = Document::new();
        doc.objects.push(std::sync::Arc::new(rect_at(50, 4, 0.0))); // red line
        let mut st = Style::default();
        st.depth = 50;
        st.pen_color = ColorRef(1); // blue arc
        let arc = FigArc::through(
            st,
            Point::new(0.0, 0.0),
            Point::new(5.0, 5.0),
            Point::new(10.0, 0.0),
            false,
        )
        .unwrap();
        doc.objects.push(std::sync::Arc::new(Object::new(ObjectKind::Arc(arc))));
        doc.objects
            .push(std::sync::Arc::new(Object::new(ObjectKind::Compound(Compound {
                objects: vec![rect_at(50, 2, 20.0)], // green line in a compound
            }))));
        let s = build_scene(&doc, &SceneOptions::default());
        // Red channel of each stroke: blue arc 0, green compound member 0, red line 255.
        let order: Vec<u8> = s
            .prims
            .iter()
            .filter_map(|p| match p {
                Prim::Path { stroke: Some(st), .. } => Some(if st.color.b == 255 {
                    1
                } else if st.color.g == 255 {
                    2
                } else {
                    4
                }),
                _ => None,
            })
            .collect();
        assert_eq!(order, vec![1, 2, 4]);
    }

    #[test]
    fn gray_depths_are_drawn_behind_active_ones() {
        let mut doc = Document::new();
        doc.objects.push(std::sync::Arc::new(rect_at(90, 4, 0.0))); // active, at the back
        doc.objects.push(std::sync::Arc::new(rect_at(10, 4, 0.0))); // off, in front
        let opts = SceneOptions {
            hidden_depths: [10].into_iter().collect(),
            gray_hidden: true,
            ..SceneOptions::default()
        };
        let pens = stroke_pens(&build_scene(&doc, &opts));
        // The gray (r == g == b, light) one comes first.
        assert!(pens[0] >= 170 && pens[0] < 255, "{pens:?}");
        assert_eq!(pens[1], 255);
    }

    #[test]
    fn hidden_depths_blank_or_gray() {
        let mut doc = Document::new();
        let mut st = Style::default();
        st.depth = 30;
        st.pen_color = ColorRef(4);
        doc.objects
            .push(std::sync::Arc::new(Object::new(ObjectKind::Polyline(Polyline::rect(
                st,
                Point::ZERO,
                Point::new(10.0, 10.0),
            )))));
        let mut opts = SceneOptions {
            hidden_depths: [30].into_iter().collect(),
            ..SceneOptions::default()
        };
        assert!(build_scene(&doc, &opts).prims.is_empty());
        opts.gray_hidden = true;
        let s = build_scene(&doc, &opts);
        let Prim::Path { stroke: Some(st), .. } = &s.prims[0] else {
            panic!()
        };
        let c = st.color;
        assert!(c.r == c.g && c.g == c.b && c.r >= 170, "{c:?} is not a light gray");
    }

    #[test]
    fn arrows_shorten_line() {
        let mut doc = Document::new();
        let mut p = Polyline::new(
            PolyKind::Polyline,
            Style::default(),
            vec![Point::ZERO, Point::new(1000.0, 0.0)],
        );
        p.forward_arrow = Some(Arrow::default());
        doc.objects
            .push(std::sync::Arc::new(Object::new(ObjectKind::Polyline(p))));
        let s = build_scene(&doc, &SceneOptions::default());
        assert_eq!(s.prims.len(), 2);
        let Prim::Path { subpaths, .. } = &s.prims[0] else {
            panic!()
        };
        assert!(subpaths[0].pts[1].x < 1000.0);
    }
}
