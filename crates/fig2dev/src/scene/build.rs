//! fig2dev's objects as scene primitives: each object as RustFig draws
//! it, from the numbers `read.c` returns.
//!
//! The reader's numbers go back to the file's: coordinates to 1200 per
//! inch (the file's `ppi` scaled), thicknesses and arc-box radii to
//! 1/80 inch (`read.c` multiplied them by `THICK_SCALE`), user colours to
//! their RGB. Then the objects are drawn as RustFig's own document draws
//! them (`rustfig-render`): X-splines, arcs and ellipses flattened by
//! `figbase`, arrowheads as fig2dev shapes them, line widths as fig2dev
//! draws them.
//
// Ported from fig2dev dev/genps.c (`genps_line`: boxes and pictures,
// `PSFONTMAG` text sizes, `-f`) and read.c (closed polylines, old splines)
// (xfig licence, see LICENSE-xfig).

use super::{fill, picture_prims, shape_prims, stroke, EmbeddedEps, Prim, Rgba, Scene, Shape, TextPrim};
use crate::gendev::{drawing_order, Obj, FIG_COLOR_NAMES, NUM_STD_COLS};
use crate::object::{self as f, Compound};
use crate::util::readpics::{load_picture, LoadedPicture};
use crate::Ctx;
use figbase::color::{Rgb, STANDARD_COLORS};
use figbase::geom::{
    arc_from_three_points, arc_path, ellipse_path, flatten_arc, flatten_ellipse, flatten_round_rect, flatten_xspline,
    round_rect_path, ArrowSpec, Point, Rect, FIG_PER_STYLE_UNIT, FLATTEN_TOL,
};
use figbase::style::{CapStyle, JoinStyle, Justify, LineKind};
use std::collections::BTreeMap;
use std::sync::Arc as Shared;

/// How texts are drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextOptions {
    /// Factor on font sizes: 72/80 without `-F` (fig2dev reads them as
    /// 1/80 inch, `PSFONTMAG`), 76.2/80 with `-F` in metric figures (their
    /// text keeps its point size while the drawing is magnified by
    /// 80/76.2), else 1.
    pub scale: f64,
    /// `-f`: the PostScript font that replaces the default font and
    /// Times-Roman.
    pub font: Option<i32>,
    /// Leave special (LaTeX) texts out, as the graphics half of `pstex`
    /// and `pdftex` does.
    pub skip_special: bool,
}

impl Default for TextOptions {
    fn default() -> Self {
        TextOptions {
            scale: 1.0,
            font: None,
            skip_special: false,
        }
    }
}

/// Draws fig2dev's objects one at a time, as a driver receives them.
#[derive(Debug, Default)]
pub struct Builder {
    /// Fig units (1200 per inch) per file unit.
    scale: f64,
    /// `round(THICK_SCALE)`, by which `read.c` multiplied thicknesses and
    /// radii; 0 for Fig 1.3, which keeps them.
    thick_factor: i32,
    thick_scale: f64,
    text: TextOptions,
    pub prims: Vec<Prim>,
    /// The EPS pictures read, by file name, for the EPS writer to embed.
    pub embedded_eps: BTreeMap<String, EmbeddedEps>,
}

impl Builder {
    pub fn new(ctx: &Ctx, text: TextOptions) -> Builder {
        Builder {
            scale: file_scale(ctx),
            thick_factor: crate::round(f64::from(ctx.thick_scale)),
            thick_scale: f64::from(ctx.thick_scale),
            text,
            prims: Vec::new(),
            embedded_eps: BTreeMap::new(),
        }
    }

    /// The scene drawn so far.
    pub fn scene(&mut self) -> Scene {
        Scene {
            prims: std::mem::take(&mut self.prims),
        }
    }

    fn coord(&self, v: f64) -> f64 {
        let v = v * self.scale;
        if v.is_finite() {
            v
        } else {
            0.0
        }
    }

    fn point(&self, p: f::Point) -> Point {
        Point::new(self.coord(f64::from(p.x)), self.coord(f64::from(p.y)))
    }

    fn points(&self, ps: &[f::Point]) -> Vec<Point> {
        ps.iter().map(|&p| self.point(p)).collect()
    }

    /// A thickness or radius back in 1/80 inch.
    fn thickness(&self, t: i32) -> i32 {
        if self.thick_factor > 0 {
            (f64::from(t) / f64::from(self.thick_factor)).round() as i32
        } else {
            t
        }
    }

    fn arrow(&self, a: &Option<f::Arrow>) -> Option<ArrowSpec> {
        let a = a.as_ref()?;
        let ts = self.thick_scale;
        let thickness = if ts > 0.0 { a.thickness / ts } else { 1.0 };
        let finite = |v: f64| if v.is_finite() { v } else { 0.0 };
        Some(ArrowSpec {
            kind: a.type_.clamp(0, 30),
            style: a.style.clamp(0, 1),
            thickness: finite(thickness).clamp(0.0, 10_000.0),
            width: finite(a.wid * self.scale).abs().min(1.0e7),
            height: finite(a.ht * self.scale).abs().min(1.0e7),
        })
    }

    /// The line style of an object.
    #[allow(clippy::too_many_arguments)]
    fn style(
        &self,
        ctx: &Ctx,
        line: i32,
        thickness: i32,
        pen: i32,
        fill_color: i32,
        area_fill: i32,
        style_val: f64,
        cap: i32,
        join: i32,
    ) -> Style {
        Style {
            line: LineKind::from_fig(line),
            thickness: self.thickness(thickness).clamp(0, 10_000),
            pen: Rgba::opaque(color(ctx, pen)),
            fill: color(ctx, fill_color),
            fill_default_or_black: color_number(ctx, fill_color) <= 0,
            fill_white: color_number(ctx, fill_color) == 7,
            area_fill: area_fill.clamp(-1, 62),
            style_val: if style_val.is_finite() { style_val } else { 0.0 },
            cap: CapStyle::from_fig(cap),
            join: JoinStyle::from_fig(join),
        }
    }

    fn shape(&mut self, st: &Style, round_dots: bool, s: ShapeGeom) {
        shape_prims(
            Shape {
                paths: s.paths,
                exact: s.exact,
                fill: fill(st.area_fill, st.pen, st.fill, st.fill_default_or_black, st.fill_white),
                stroke: st.stroke(round_dots),
                pen: st.pen,
                thickness: f64::from(st.thickness),
                forward_arrow: s.forward,
                backward_arrow: s.backward,
                poly: s.poly,
                pie_arc: s.pie_arc,
            },
            &mut self.prims,
        );
    }

    pub fn arc(&mut self, ctx: &Ctx, a: &f::Arc) {
        let st = self.style(
            ctx,
            a.style,
            a.thickness,
            a.pen_color,
            a.fill_color,
            a.fill_style,
            a.style_val,
            a.cap_style,
            0,
        );
        let pts = a.point.map(|p| self.point(p));
        let pie = a.type_ == f::T_PIE_WEDGE_ARC;
        let (fwd, back) = (self.arrow(&a.for_arrow), self.arrow(&a.back_arrow));
        // RustFig reads the direction from the three points.
        let geom = arc_from_three_points(pts[0], pts[1], pts[2]);
        let (paths, exact, pie_arc) = match geom {
            Some(g) => {
                let open = flatten_arc(&g, FLATTEN_TOL);
                let mut outline = open.clone();
                if pie {
                    outline.push(g.center);
                }
                // Pie wedges carry their arrows on the arc (fig2dev draws them).
                let pie_arc = (pie && (fwd.is_some() || back.is_some())).then_some(open);
                (vec![(outline, pie)], Some(arc_path(&g, pie)), pie_arc)
            }
            None => {
                let pie_arc = (pie && (fwd.is_some() || back.is_some())).then(|| pts.to_vec());
                (vec![(pts.to_vec(), false)], None, pie_arc)
            }
        };
        self.shape(
            &st,
            false,
            ShapeGeom {
                paths,
                exact,
                forward: fwd,
                backward: back,
                poly: false,
                pie_arc,
            },
        );
    }

    pub fn ellipse(&mut self, ctx: &Ctx, e: &f::Ellipse) {
        let st = self.style(
            ctx,
            e.style,
            e.thickness,
            e.pen_color,
            e.fill_color,
            e.fill_style,
            e.style_val,
            0,
            0,
        );
        let center = self.point(e.center);
        let radii = self.point(e.radiuses);
        let (rx, ry) = (radii.x.abs(), radii.y.abs());
        let angle = if e.angle.is_finite() { e.angle } else { 0.0 };
        self.shape(
            &st,
            true,
            ShapeGeom {
                paths: vec![(flatten_ellipse(center, rx, ry, angle, FLATTEN_TOL), true)],
                exact: Some(ellipse_path(center, rx, ry, angle)),
                ..ShapeGeom::default()
            },
        );
    }

    pub fn line(&mut self, ctx: &mut Ctx, l: &f::Line) {
        let st = self.style(
            ctx,
            l.style,
            l.thickness,
            l.pen_color,
            l.fill_color,
            l.fill_style,
            l.style_val,
            l.cap_style,
            l.join_style,
        );
        let kind = l.type_;
        // Unknown types draw as polylines.
        let closed = matches!(kind, f::T_BOX | f::T_POLYGON | f::T_ARC_BOX | f::T_PIC_BOX);
        let rect = matches!(kind, f::T_BOX | f::T_ARC_BOX | f::T_PIC_BOX);
        let mut points = self.points(&l.points);
        // Closed polylines repeat their first point in the file.
        if closed && points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        if rect && !points.is_empty() {
            let mut lo = Point::new(f64::INFINITY, f64::INFINITY);
            let mut hi = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
            for p in &points {
                lo = Point::new(lo.x.min(p.x), lo.y.min(p.y));
                hi = Point::new(hi.x.max(p.x), hi.y.max(p.y));
            }
            points = vec![lo, Point::new(hi.x, lo.y), hi, Point::new(lo.x, hi.y)];
        }
        if kind == f::T_PIC_BOX {
            self.picture(ctx, l, &st, &points);
            return;
        }
        let (paths, exact) = if kind == f::T_ARC_BOX {
            let r = Rect::from_points(points.iter().copied());
            let radius = f64::from(self.thickness(l.radius).clamp(0, 100_000)) * FIG_PER_STYLE_UNIT;
            (
                vec![(flatten_round_rect(&r, radius), true)],
                Some(round_rect_path(&r, radius)),
            )
        } else {
            (vec![(points, closed)], None)
        };
        let (forward, backward) = if !closed {
            (self.arrow(&l.for_arrow), self.arrow(&l.back_arrow))
        } else {
            (None, None)
        };
        self.shape(
            &st,
            false,
            ShapeGeom {
                paths,
                exact,
                forward,
                backward,
                poly: true,
                pie_arc: None,
            },
        );
    }

    /// A picture: its pixels, or (EPS, PDF, unreadable files) a reference
    /// and a crossed box; EPS files are kept for the EPS writer to embed.
    fn picture(&mut self, ctx: &mut Ctx, l: &f::Line, st: &Style, points: &[Point]) {
        let r = Rect::from_points(points.iter().copied());
        if r.is_empty() {
            return;
        }
        let pic = l.pic.as_deref();
        let file = pic.map(|p| p.file.as_str()).unwrap_or_default();
        let mut image = None;
        if !file.is_empty() {
            match load_picture(ctx, file) {
                Ok(LoadedPicture::Raster(img)) => image = Some(Shared::new(img)),
                Ok(LoadedPicture::Eps { llx, lly, urx, ury, ps }) => {
                    self.embedded_eps.insert(
                        file.to_string(),
                        EmbeddedEps {
                            title: file.to_string(),
                            llx,
                            lly,
                            urx,
                            ury,
                            ps,
                        },
                    );
                }
                Ok(LoadedPicture::Pdf) => {}
                // fig2dev's message; the picture becomes a placeholder.
                Err(e) => ctx.warn(e),
            }
        }
        let frame = if st.thickness > 0 { st.stroke(false) } else { None };
        picture_prims(
            r.corners(),
            image,
            pic.is_some_and(|p| p.flipped != 0),
            file,
            frame,
            &mut self.prims,
        );
    }

    pub fn spline(&mut self, ctx: &Ctx, s: &f::Spline) {
        let st = self.style(
            ctx,
            s.style,
            s.thickness,
            s.pen_color,
            s.fill_color,
            s.fill_style,
            s.style_val,
            s.cap_style,
            0,
        );
        let closed = s.closed_spline();
        // Shape factors: the file's for Fig 3.2 X-splines (fig2dev turned
        // those into polylines already), else those of the spline's kind.
        let default_shape = if s.x_spline() {
            0.5
        } else if s.int_spline() {
            -1.0
        } else {
            1.0
        };
        let mut points = self.points(&s.points);
        let mut shapes: Vec<f64> = if s.controls.len() == s.points.len() && ctx.v32_flag {
            s.controls.iter().map(|c| c.s.clamp(-1.0, 1.0)).collect()
        } else {
            let mut v = vec![default_shape; points.len()];
            if !closed {
                if let Some(first) = v.first_mut() {
                    *first = 0.0;
                }
                if let Some(last) = v.last_mut() {
                    *last = 0.0;
                }
            }
            v
        };
        if closed && points.len() > 2 && points.first() == points.last() && !ctx.v32_flag {
            points.pop();
        }
        shapes.truncate(points.len());
        let (forward, backward) = if closed {
            (None, None)
        } else {
            (self.arrow(&s.for_arrow), self.arrow(&s.back_arrow))
        };
        self.shape(
            &st,
            closed,
            ShapeGeom {
                paths: vec![(flatten_xspline(&points, &shapes, closed, FLATTEN_TOL * 2.0), closed)],
                exact: None,
                forward,
                backward,
                poly: false,
                pie_arc: None,
            },
        );
    }

    pub fn text(&mut self, ctx: &Ctx, t: &f::Text) {
        // RustFig's flags: bits, even for −1.
        let special = t.flags & f::SPECIAL_TEXT != 0;
        if self.text.skip_special && special {
            return;
        }
        let mut postscript = t.flags & f::PSFONT_TEXT != 0;
        let mut font = t.font;
        if let Some(fnum) = self.text.font {
            // PostScript fonts −1 and 0, LaTeX fonts 0 and 1.
            let replaced = if postscript { font <= 0 } else { font <= 1 };
            if replaced {
                font = fnum;
                postscript = true;
            }
        }
        let finite = |v: f64, d: f64| if v.is_finite() { v } else { d };
        let scale = Some(self.text.scale)
            .filter(|s| s.is_finite() && *s > 0.0)
            .unwrap_or(1.0);
        self.prims.push(Prim::Text(TextPrim {
            origin: Point::new(self.coord(f64::from(t.base_x)), self.coord(f64::from(t.base_y))),
            angle: finite(t.angle, 0.0),
            size: finite(t.size, 12.0).clamp(0.1, 5000.0) * scale,
            justify: Justify::from_fig(t.type_),
            font,
            postscript,
            text: t.cstring.clone(),
            color: Rgba::opaque(color(ctx, t.color)),
        }));
    }

    /// One object, as `convert` hands it to a driver.
    pub fn object(&mut self, ctx: &mut Ctx, o: Obj<'_>) {
        match o {
            Obj::Arc(a) => self.arc(ctx, a),
            Obj::Ellipse(e) => self.ellipse(ctx, e),
            Obj::Line(l) => self.line(ctx, l),
            Obj::Spline(s) => self.spline(ctx, s),
            Obj::Text(t) => self.text(ctx, t),
        }
    }
}

/// The geometry of a line object.
#[derive(Debug, Default)]
struct ShapeGeom {
    paths: Vec<(Vec<Point>, bool)>,
    exact: Option<Vec<figbase::geom::Seg>>,
    forward: Option<ArrowSpec>,
    backward: Option<ArrowSpec>,
    poly: bool,
    pie_arc: Option<Vec<Point>>,
}

/// An object's line and fill style, resolved.
#[derive(Debug, Clone, Copy)]
struct Style {
    line: LineKind,
    /// 1/80 inch.
    thickness: i32,
    pen: Rgba,
    fill: Rgb,
    fill_default_or_black: bool,
    fill_white: bool,
    area_fill: i32,
    style_val: f64,
    cap: CapStyle,
    join: JoinStyle,
}

impl Style {
    fn stroke(&self, round_dots: bool) -> Option<super::Stroke> {
        stroke(
            self.thickness,
            self.line,
            self.style_val,
            self.pen,
            self.cap,
            self.join,
            round_dots,
        )
    }
}

/// Fig units (1200 per inch) per unit of the file.
pub fn file_scale(ctx: &Ctx) -> f64 {
    if ctx.ppi.is_finite() && ctx.ppi > 0.0 {
        1200.0 / ctx.ppi
    } else {
        1.0
    }
}

/// The colour number as the file gives it: −1 default, 0–31 standard, the
/// file's number for user colours (−1 if unknown).
fn color_number(ctx: &Ctx, c: i32) -> i32 {
    if c < NUM_STD_COLS as i32 {
        return c.max(-1);
    }
    let idx = usize::try_from(c - NUM_STD_COLS as i32).ok();
    idx.and_then(|i| ctx.user_col_indx.get(i)).copied().unwrap_or(-1)
}

/// The RGB of colour `c` as the reader numbers it (user colours after the
/// standard ones); default and unknown colours are black.
pub fn color(ctx: &Ctx, c: i32) -> Rgb {
    if (0..NUM_STD_COLS as i32).contains(&c) {
        return STANDARD_COLORS.get(c as usize).copied().unwrap_or(Rgb::BLACK);
    }
    let Some(i) = usize::try_from(c - NUM_STD_COLS as i32).ok() else {
        return Rgb::BLACK;
    };
    // The file's number; the first colour defined with it counts.
    let Some(&n) = ctx.user_col_indx.get(i) else {
        return Rgb::BLACK;
    };
    if !(32..=543).contains(&n) {
        return Rgb::BLACK;
    }
    ctx.user_col_indx
        .iter()
        .position(|&m| m == n)
        .and_then(|j| ctx.user_colors.get(j))
        .map_or(Rgb::BLACK, |u| Rgb::new(byte(u.r), byte(u.g), byte(u.b)))
}

fn byte(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// The figure's GIF transparent colour (`read.c` keeps its name in
/// `gif_transparent`): a standard colour, or a user colour the figure
/// defines; none for "none", "default" and "background".
pub fn transparent_color(ctx: &Ctx) -> Option<Rgb> {
    let name = ctx.gif_transparent.as_str();
    if name.is_empty() {
        return None;
    }
    if let Some(i) = FIG_COLOR_NAMES.iter().position(|n| *n == name) {
        return STANDARD_COLORS.get(i).copied();
    }
    ctx.user_colors
        .iter()
        .find(|u| format!("#{:2x}{:2x}{:2x}", u.r, u.g, u.b) == name)
        .map(|u| Rgb::new(byte(u.r), byte(u.g), byte(u.b)))
}

/// The scene of a whole figure as fig2dev reads it: the objects `-D`
/// keeps, deepest first ([`drawing_order`]), and the EPS pictures to embed.
pub fn build(ctx: &mut Ctx, objects: &Compound, text: TextOptions) -> (Scene, BTreeMap<String, EmbeddedEps>) {
    let mut b = Builder::new(ctx, text);
    for (depth, o) in drawing_order(objects) {
        if ctx.depth_filter(depth) {
            b.object(ctx, o);
        }
    }
    (b.scene(), std::mem::take(&mut b.embedded_eps))
}
