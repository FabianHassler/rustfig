//! RustFig rendering (layer L3).
//!
//! A [`Scene`] is a flat, depth-sorted list of drawing primitives built from
//! a document. Backends turn it into pixels ([`raster`]), SVG, EPS or PDF.
//! The same scene drives the canvas and every export, so what you see is
//! what you export.

pub mod eps;
pub mod export;
pub mod pattern;
pub mod pdf;
pub mod raster;
pub mod svg;
mod textcache;

pub use export::{export, export_bounds, ExportFormat, ExportOptions};

use rustfig_geom::{arrowhead, ArrowShape, Point, Rect, FIG_PER_POINT};
use rustfig_model::{
    area_fill_color, flatten_object, Arrow, CapStyle, ColorRef, Document, JoinStyle, Justify, LineKind, Object,
    ObjectKind, PolyKind, RasterImage, Rgb, Style,
};
use std::collections::BTreeSet;
use std::sync::Arc;

/// A colour with alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn opaque(c: Rgb) -> Rgba {
        Rgba {
            r: c.r,
            g: c.g,
            b: c.b,
            a: 255,
        }
    }
    pub fn rgb(self) -> Rgb {
        Rgb::new(self.r, self.g, self.b)
    }
}

/// How a closed path is filled.
#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    Solid(Rgba),
    /// Fig pattern 41–62: `fg` lines over a `bg` background.
    Pattern {
        kind: i32,
        fg: Rgba,
        bg: Rgba,
    },
}

/// How a path is stroked. Lengths are Fig units.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub width: f64,
    pub color: Rgba,
    pub dash: Vec<f64>,
    pub cap: CapStyle,
    pub join: JoinStyle,
}

/// One polyline of a path.
#[derive(Debug, Clone, PartialEq)]
pub struct SubPath {
    pub pts: Vec<Point>,
    pub closed: bool,
}

/// A drawing primitive in Fig coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum Prim {
    Path {
        subpaths: Vec<SubPath>,
        fill: Option<Fill>,
        stroke: Option<Stroke>,
    },
    Text(TextPrim),
    Image {
        /// Top-left, top-right, bottom-right, bottom-left of the picture box.
        corners: [Point; 4],
        image: Arc<RasterImage>,
        /// Transposed (Fig "flipped" pictures).
        flipped: bool,
    },
}

/// A text run.
#[derive(Debug, Clone, PartialEq)]
pub struct TextPrim {
    pub origin: Point,
    /// Radians, counter-clockwise.
    pub angle: f64,
    /// Points.
    pub size: f64,
    pub justify: Justify,
    pub font: i32,
    pub postscript: bool,
    pub text: String,
    pub color: Rgba,
}

impl TextPrim {
    pub fn request(&self) -> rustfig_text::FontRequest {
        rustfig_text::FontRequest::for_fig(self.font, self.postscript)
    }
}

impl Rgba {
    /// The light gray xfig uses for depths that are off: darker colours
    /// stay a little darker, so shapes remain readable.
    pub fn grayed(self) -> Rgba {
        let lum = (u32::from(self.r) * 299 + u32::from(self.g) * 587 + u32::from(self.b) * 114) / 1000;
        let v = (170 + lum * 70 / 255).min(255) as u8;
        Rgba {
            r: v,
            g: v,
            b: v,
            a: self.a,
        }
    }
}

impl Prim {
    /// Recolour in gray (objects on depths that are off).
    pub fn gray_out(&mut self) {
        match self {
            Prim::Path { fill, stroke, .. } => {
                match fill {
                    Some(Fill::Solid(c)) => *c = c.grayed(),
                    Some(Fill::Pattern { fg, bg, .. }) => {
                        *fg = fg.grayed();
                        *bg = bg.grayed();
                    }
                    None => {}
                }
                if let Some(s) = stroke {
                    s.color = s.color.grayed();
                }
            }
            Prim::Text(t) => t.color = t.color.grayed(),
            // Pictures keep their pixels; they can't be picked either way.
            Prim::Image { .. } => {}
        }
    }

    /// Bounding box in Fig units (including half the stroke width).
    pub fn bbox(&self) -> Rect {
        match self {
            Prim::Path { subpaths, stroke, .. } => {
                let mut r = Rect::EMPTY;
                for s in subpaths {
                    for p in &s.pts {
                        r.add_point(*p);
                    }
                }
                r.expand(stroke.as_ref().map(|s| s.width / 2.0).unwrap_or(0.0))
            }
            Prim::Text(t) => {
                let m = textcache::metrics(t);
                let x0 = match t.justify {
                    Justify::Left => 0.0,
                    Justify::Center => -m.width / 2.0,
                    Justify::Right => -m.width,
                };
                let rot = rustfig_geom::Affine::rotate(t.angle);
                Rect::from_points(
                    [
                        Point::new(x0, -m.ascent),
                        Point::new(x0 + m.width, -m.ascent),
                        Point::new(x0 + m.width, m.descent),
                        Point::new(x0, m.descent),
                    ]
                    .map(|p| t.origin + rot.apply_vec(p)),
                )
            }
            Prim::Image { corners, .. } => Rect::from_points(corners.iter().copied()),
        }
    }
}

/// Options for building a scene.
#[derive(Debug, Clone, Default)]
pub struct SceneOptions {
    /// Depths that are off in the depth manager.
    pub hidden_depths: BTreeSet<i32>,
    /// Draw objects on hidden depths in gray instead of leaving them out
    /// (xfig's depth panel "Gray" setting).
    pub gray_hidden: bool,
    /// Draw for the screen: hidden texts appear as a placeholder.
    pub screen: bool,
}

/// A depth-sorted list of primitives.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub prims: Vec<Prim>,
}

impl Scene {
    pub fn build(doc: &Document, opts: &SceneOptions) -> Scene {
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
        Scene { prims }
    }

    pub fn bbox(&self) -> Rect {
        self.prims.iter().fold(Rect::EMPTY, |r, p| r.union(&p.bbox()))
    }
}

fn color(doc: &Document, c: ColorRef) -> Rgba {
    Rgba::opaque(doc.rgb(c))
}

fn fill_for(doc: &Document, s: &Style) -> Option<Fill> {
    if s.area_fill < 0 {
        return None;
    }
    let fc = doc.rgb(s.fill_color);
    if s.area_fill > 40 {
        return Some(Fill::Pattern {
            kind: s.area_fill,
            fg: color(doc, s.pen_color),
            bg: Rgba::opaque(fc),
        });
    }
    let is_white = s.fill_color == ColorRef::WHITE;
    area_fill_color(fc, s.fill_color.is_default_or_black(), is_white, s.area_fill).map(|c| Fill::Solid(Rgba::opaque(c)))
}

/// Stroke width in Fig units for a thickness in 1/80 inch (fig2dev's rule,
/// [`rustfig_geom::stroke_width`]).
pub fn line_width(thickness: f64) -> f64 {
    rustfig_geom::stroke_width(thickness)
}

fn stroke_for(doc: &Document, s: &Style) -> Option<Stroke> {
    if s.thickness <= 0 {
        return None;
    }
    let width = line_width(f64::from(s.thickness));
    let dash = s.line.dash_pattern(s.style_val, width);
    let dotted = matches!(
        s.line,
        LineKind::Dotted | LineKind::DashDotted | LineKind::DashDoubleDotted | LineKind::DashTripleDotted
    );
    Some(Stroke {
        width,
        color: color(doc, s.pen_color),
        dash,
        cap: if dotted { CapStyle::Round } else { s.cap },
        join: s.join,
    })
}

/// A point at least `min` away from the end of `pts`, walking backwards, to
/// orient arrowheads.
fn arrow_from(pts: &[Point], tip: Point, min: f64) -> Option<Point> {
    let mut last = None;
    for p in pts.iter().rev() {
        if p.dist(tip) > 1e-6 {
            last = Some(*p);
            if p.dist(tip) >= min {
                break;
            }
        }
    }
    last
}

fn arrow_prims(doc: &Document, style: &Style, a: &Arrow, shape: ArrowShape, out: &mut Vec<Prim>) {
    let pen = color(doc, style.pen_color);
    let fill = match shape.fill {
        rustfig_geom::ArrowFill::None => None,
        rustfig_geom::ArrowFill::White => Some(Fill::Solid(Rgba::opaque(Rgb::WHITE))),
        rustfig_geom::ArrowFill::Pen => Some(Fill::Solid(pen)),
    };
    out.push(Prim::Path {
        subpaths: vec![SubPath {
            pts: shape.outline,
            closed: shape.closed,
        }],
        fill,
        stroke: Some(Stroke {
            width: line_width(a.thickness).max(1.0),
            color: pen,
            dash: Vec::new(),
            cap: CapStyle::Butt,
            join: JoinStyle::Miter,
        }),
    });
}

fn object_prims(doc: &Document, o: &Object, opts: &SceneOptions, out: &mut Vec<Prim>) {
    match &o.kind {
        ObjectKind::Text(t) => {
            let text = if opts.screen && t.is_hidden() {
                "<<>>".to_string()
            } else {
                t.text.clone()
            };
            out.push(Prim::Text(TextPrim {
                origin: t.origin,
                angle: if t.angle.is_finite() { t.angle } else { 0.0 },
                size: t.size,
                justify: t.justify,
                font: t.font,
                postscript: t.is_postscript(),
                text,
                color: color(doc, t.color),
            }));
        }
        ObjectKind::Polyline(p) if p.kind == PolyKind::Picture => {
            let r = Rect::from_points(p.points.iter().copied());
            if r.is_empty() {
                return;
            }
            let corners = r.corners();
            match p
                .picture
                .as_ref()
                .and_then(|pic| pic.image.clone().map(|i| (i, pic.flipped)))
            {
                Some((image, flipped)) => out.push(Prim::Image {
                    corners,
                    image,
                    flipped,
                }),
                None => {
                    // Missing picture: box with a cross.
                    let [a, b, c, d] = corners;
                    out.push(Prim::Path {
                        subpaths: vec![
                            SubPath {
                                pts: corners.to_vec(),
                                closed: true,
                            },
                            SubPath {
                                pts: vec![a, c],
                                closed: false,
                            },
                            SubPath {
                                pts: vec![b, d],
                                closed: false,
                            },
                        ],
                        fill: None,
                        stroke: Some(Stroke {
                            width: line_width(1.0),
                            color: Rgba::opaque(Rgb::new(128, 128, 128)),
                            dash: Vec::new(),
                            cap: CapStyle::Butt,
                            join: JoinStyle::Miter,
                        }),
                    });
                }
            }
            if p.style.thickness > 0 {
                if let Some(stroke) = stroke_for(doc, &p.style) {
                    out.push(Prim::Path {
                        subpaths: vec![SubPath {
                            pts: corners.to_vec(),
                            closed: true,
                        }],
                        fill: None,
                        stroke: Some(stroke),
                    });
                }
            }
        }
        ObjectKind::Compound(_) => {}
        _ => {
            let Some(style) = o.style() else { return };
            let mut paths = flatten_object(o);
            let fill = fill_for(doc, style);
            let stroke = stroke_for(doc, style);
            let (fwd, back) = match &o.kind {
                ObjectKind::Polyline(p) if p.kind == PolyKind::Polyline => (p.forward_arrow, p.backward_arrow),
                ObjectKind::Spline(s) if !s.closed => (s.forward_arrow, s.backward_arrow),
                ObjectKind::Arc(a) if !a.pie => (a.forward_arrow, a.backward_arrow),
                _ => (None, None),
            };
            let mut heads = Vec::new();
            if let Some(path) = paths.first_mut() {
                if !path.closed && path.pts.len() >= 2 {
                    if let Some(a) = fwd {
                        if let Some(tip) = path.pts.last().copied() {
                            if let Some(from) = arrow_from(&path.pts, tip, a.height) {
                                if let Some(sh) = arrowhead(tip, from, a.kind, a.style, a.width, a.height) {
                                    if let Some(l) = path.pts.last_mut() {
                                        *l = sh.line_end;
                                    }
                                    heads.push((a, sh));
                                }
                            }
                        }
                    }
                    if let Some(a) = back {
                        if let Some(tip) = path.pts.first().copied() {
                            let rev: Vec<Point> = path.pts.iter().rev().copied().collect();
                            if let Some(from) = arrow_from(&rev, tip, a.height) {
                                if let Some(sh) = arrowhead(tip, from, a.kind, a.style, a.width, a.height) {
                                    if let Some(f) = path.pts.first_mut() {
                                        *f = sh.line_end;
                                    }
                                    heads.push((a, sh));
                                }
                            }
                        }
                    }
                }
            }
            let closed_any = paths.iter().any(|p| p.closed);
            let subpaths: Vec<SubPath> = paths
                .into_iter()
                .filter(|p| !p.pts.is_empty())
                .map(|p| {
                    let mut pts = p.pts;
                    if pts.len() == 1 {
                        // A single point: a dot.
                        if let Some(q) = pts.first().copied() {
                            pts.push(q + Point::new(0.01, 0.0));
                        }
                    }
                    SubPath { pts, closed: p.closed }
                })
                .collect();
            let single_dot = subpaths
                .iter()
                .all(|s| s.pts.len() == 2 && s.pts[0].dist(s.pts[1]) < 0.1);
            let stroke = stroke.map(|mut s| {
                if single_dot {
                    s.cap = CapStyle::Round;
                }
                s
            });
            // Open shapes can still be filled (xfig fills open polylines and splines).
            let fill = if closed_any || subpaths.iter().any(|s| s.pts.len() > 2) {
                fill
            } else {
                None
            };
            out.push(Prim::Path { subpaths, fill, stroke });
            for (a, sh) in heads {
                arrow_prims(doc, style, &a, sh, out);
            }
        }
    }
}

/// Font size in Fig units for a size in points.
pub fn font_px(size: f64) -> f64 {
    size * FIG_PER_POINT
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustfig_model::{Polyline, Style};

    #[test]
    fn depth_order() {
        let mut doc = Document::new();
        let mut front = Style::default();
        front.depth = 10;
        let mut back = Style::default();
        back.depth = 90;
        back.pen_color = ColorRef(4);
        doc.objects.push(Object::new(ObjectKind::Polyline(Polyline::rect(
            front,
            Point::ZERO,
            Point::new(10.0, 10.0),
        ))));
        doc.objects.push(Object::new(ObjectKind::Polyline(Polyline::rect(
            back,
            Point::ZERO,
            Point::new(10.0, 10.0),
        ))));
        let s = Scene::build(&doc, &SceneOptions::default());
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
        doc.objects.push(rect_at(50, 4, 0.0)); // red line
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
        doc.objects.push(Object::new(ObjectKind::Arc(arc)));
        doc.objects.push(Object::new(ObjectKind::Compound(Compound {
            objects: vec![rect_at(50, 2, 20.0)], // green line in a compound
        })));
        let s = Scene::build(&doc, &SceneOptions::default());
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
        doc.objects.push(rect_at(90, 4, 0.0)); // active, at the back
        doc.objects.push(rect_at(10, 4, 0.0)); // off, in front
        let opts = SceneOptions {
            hidden_depths: [10].into_iter().collect(),
            gray_hidden: true,
            ..SceneOptions::default()
        };
        let pens = stroke_pens(&Scene::build(&doc, &opts));
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
        doc.objects.push(Object::new(ObjectKind::Polyline(Polyline::rect(
            st,
            Point::ZERO,
            Point::new(10.0, 10.0),
        ))));
        let mut opts = SceneOptions {
            hidden_depths: [30].into_iter().collect(),
            ..SceneOptions::default()
        };
        assert!(Scene::build(&doc, &opts).prims.is_empty());
        opts.gray_hidden = true;
        let s = Scene::build(&doc, &opts);
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
        doc.objects.push(Object::new(ObjectKind::Polyline(p)));
        let s = Scene::build(&doc, &SceneOptions::default());
        assert_eq!(s.prims.len(), 2);
        let Prim::Path { subpaths, .. } = &s.prims[0] else {
            panic!()
        };
        assert!(subpaths[0].pts[1].x < 1000.0);
    }
}
