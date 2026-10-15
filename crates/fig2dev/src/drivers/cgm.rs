//! fig2dev's `cgm` driver: a clear-text, version-1 Computer Graphics
//! Metafile. Coordinates are Fig units with the origin at the figure's
//! lower left corner; colours are indices into a table of the 32 standard
//! and the user colours, plus one entry redefined for computed fill shades.
//!
//! As in fig2dev: old-style (pre-3.2) splines are not drawn (3.2 splines
//! arrive as polylines), pictures are drawn as their outline, the
//! dash-triple-dotted style is drawn solid, and only 6 hatch patterns
//! exist. `-a` asked fig2dev to pipe the output through `ralcgm -b` for
//! binary CGM; RustFig runs no external programs, so it says so and writes
//! clear text, as fig2dev does when it can't open the pipe.
//
// Ported from fig2dev dev/gencgm.c (xfig licence, see LICENSE-xfig).

use crate::gendev::{Ctx, Driver, NUM_STD_COLS};
use crate::object::{
    Arc, Arrow, Compound, Ellipse, Line, Point, Spline, Text, T_ARC_BOX, T_BOX, T_CENTER_JUSTIFIED, T_CIRCLE_BY_DIA,
    T_CIRCLE_BY_RAD, T_ELLIPSE_BY_DIA, T_ELLIPSE_BY_RAD, T_LEFT_JUSTIFIED, T_OPEN_ARC, T_PIC_BOX, T_PIE_WEDGE_ARC,
    T_POLYGON, T_POLYLINE, T_RIGHT_JUSTIFIED,
};
use crate::{fprintf, round, Out};
use std::f64::consts::PI;

/// The driver for `lang`.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    let _ = lang;
    Some(Box::new(Cgm::default()))
}

/// An undefined attribute value (`UNDEFVALUE`): nothing is cached yet.
const UNDEFVALUE: i32 = -100;
/// The colour index used for solid fill shades (`FILL_COLOR_INDEX`).
const FILL_COLOR_INDEX: i32 = 999;
const EPSILON: f64 = 1e-4;
/// Index of the first LaTeX-like font in the font list.
const LATEX_FONT_BASE: i32 = 2;
const NUM_LATEX_FONTS: i32 = 5;
/// Index of the first PostScript font in the font list.
const PS_FONT_BASE: i32 = 7;
const NUM_PS_FONTS: i32 = 35;
const ARROW_INDENT_DIST: f64 = 0.8;
const ARROW_POINT_DIST: f64 = 1.2;

/// The standard colours, as fig2dev's `float` table (`stdcols`).
const STDCOLS: [[f32; 3]; NUM_STD_COLS] = [
    [0.0, 0.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 1.0, 0.0],
    [0.0, 1.0, 1.0],
    [1.0, 0.0, 0.0],
    [1.0, 0.0, 1.0],
    [1.0, 1.0, 0.0],
    [1.0, 1.0, 1.0],
    [0.0, 0.0, 0.56],
    [0.0, 0.0, 0.69],
    [0.0, 0.0, 0.82],
    [0.53, 0.81, 1.0],
    [0.0, 0.56, 0.0],
    [0.0, 0.69, 0.0],
    [0.0, 0.82, 0.0],
    [0.0, 0.56, 0.56],
    [0.0, 0.69, 0.69],
    [0.0, 0.82, 0.82],
    [0.56, 0.0, 0.0],
    [0.69, 0.0, 0.0],
    [0.82, 0.0, 0.0],
    [0.56, 0.0, 0.56],
    [0.69, 0.0, 0.69],
    [0.82, 0.0, 0.82],
    [0.5, 0.19, 0.0],
    [0.63, 0.25, 0.0],
    [0.75, 0.38, 0.0],
    [1.0, 0.5, 0.5],
    [1.0, 0.63, 0.63],
    [1.0, 0.75, 0.75],
    [1.0, 0.88, 0.88],
    [1.0, 0.84, 0.0],
];

/// Fig pattern → CGM hatch index 1–6 (0: none) (`map_pattern`).
const MAP_PATTERN: [i32; 22] = [0, 0, 0, 4, 3, 6, 0, 0, 1, 2, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

/// The font list written into the header.
const FONTLIST: &str = "fontlist 'Hardware',
  'Times New Roman', 'Times New Roman Bold', 'Times New Roman Italic',
  'Helvetica', 'Courier',
  'Times-Roman', 'Times-Italic',
  'Times-Bold', 'Times-BoldItalic',
  'AvantGarde-Book', 'AvantGarde-BookOblique',
  'AvantGarde-Demi', 'AvantGarde-DemiOblique',
  'Bookman-light', 'Bookman-lightItalic',
  'Bookman-Demi', 'Bookman-DemiItalic',
  'Courier', 'Courier-Oblique',
  'Courier-Bold', 'Courier-BoldOblique',
  'Helvetica', 'Helvetica-Oblique',
  'Helvetica-Bold', 'Helvetica-BoldOblique',
  'Helvetica-Narrow', 'Helvetica-Narrow-Oblique',
  'Helvetica-Narrow-Bold', 'Helvetica-Narrow-BoldOblique',
  'NewCenturySchlbk-Roman', 'NewCenturySchlbk-Italic',
  'NewCenturySchlbk-Bold', 'NewCenturySchlbk-BoldItalic',
  'Palatino-Roman', 'Palatino-Italic',
  'Palatino-Bold', 'Palatino-BoldItalic',
  'Symbol', 'ZapfChancery-MediumItalic', 'ZapfDingbats';
";

/// CGM interior styles (`INTSTYLE`; fig2dev never uses `HOLLOW`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IntStyle {
    Solid,
    Hatch,
    Empty,
}

/// The attributes `shape` and `shape_interior` read: the common part of
/// fig2dev's `F_line`, `F_ellipse` and `F_arc`.
#[derive(Clone, Copy, Debug)]
struct Attrs {
    style: i32,
    thickness: i32,
    pen_color: i32,
    fill_color: i32,
    fill_style: i32,
}

/// A normalised direction (`Dir`).
#[derive(Clone, Copy, Debug, Default)]
struct Dir {
    x: f64,
    y: f64,
}

/// The shapes `shape` and `shape_interior` draw (fig2dev passes a
/// function pointer).
enum Draw<'a> {
    Rect(&'a [Point]),
    Polygon(&'a [Point]),
    ArcBoxInterior(&'a Line),
    Ellipse(&'a Ellipse),
    Circle(&'a Ellipse),
    ArcInterior(&'a ArcCopy),
}

/// The arc as `gencgm_arc` works on it: a copy, made counterclockwise.
struct ArcCopy {
    type_: i32,
    thickness: i32,
    pen_color: i32,
    for_arrow: Option<Arrow>,
    back_arrow: Option<Arrow>,
    center: (f64, f64),
    point: [Point; 3],
}

/// fig2dev's `static` state of gencgm.c: options and attribute caches.
#[derive(Debug)]
pub struct Cgm {
    rounded_arrows: bool,
    binary_output: bool,
    linetype: i32,
    edgetype: i32,
    linewidth: i32,
    edgewidth: i32,
    linecolr: i32,
    edgecolr: i32,
    edgevis: i32,
    intstyle: Option<IntStyle>,
    fillcolor: i32,
    fillrgb: i32,
    hatchindex: i32,
    texttype: i32,
    textfont: i32,
    textcolr: i32,
    textsize: f64,
    textangle: f64,
    pic_warned: bool,
    spline_warned: bool,
    enc: TextEnc,
}

impl Default for Cgm {
    fn default() -> Cgm {
        let u = UNDEFVALUE;
        Cgm {
            rounded_arrows: false,
            binary_output: false,
            linetype: u,
            edgetype: u,
            linewidth: u,
            edgewidth: u,
            linecolr: u,
            edgecolr: u,
            edgevis: u,
            intstyle: None,
            fillcolor: u,
            fillrgb: u,
            hatchindex: u,
            texttype: u,
            textfont: u,
            textcolr: u,
            textsize: f64::from(u),
            textangle: f64::from(u),
            pic_warned: false,
            spline_warned: false,
            enc: TextEnc::Utf8,
        }
    }
}

/// `_pos`: Fig coordinates relative to the lower left corner, y up.
fn pos(ctx: &Ctx, out: &mut Out, x: i32, y: i32) {
    fprintf!(out, "({},{})", x.wrapping_sub(ctx.llx), ctx.ury.wrapping_sub(y));
}

fn point(ctx: &Ctx, out: &mut Out, p: Point) {
    pos(ctx, out, p.x, p.y);
}

/// `_relpos`: a relative position, y reversed.
fn relpos(out: &mut Out, x: i32, y: i32) {
    fprintf!(out, "({},{})", x, y.wrapping_neg());
}

/// `conv_linetype`: CGM knows the first five Fig styles; the
/// dash-triple-dotted style becomes solid.
fn conv_linetype(t: i32) -> i32 {
    if t < 0 {
        -1
    } else if t > 4 {
        t % 5
    } else {
        t
    }
}

/// `conv_color`: a Fig colour → an index into the CGM colour table.
fn conv_color(ctx: &Ctx, color: i32) -> i32 {
    let c = if color < 0 {
        0
    } else if color == FILL_COLOR_INDEX {
        (NUM_STD_COLS + ctx.user_colors.len()) as i32
    } else {
        color
    };
    c.wrapping_add(1)
}

/// `getrgb`: the 8-bit RGB of a colour (DEFAULT is black).
fn getrgb(ctx: &mut Ctx, color: i32) -> (i32, i32, i32) {
    let color = color.max(0);
    if let Some(c) = STDCOLS.get(color as usize).filter(|_| color < NUM_STD_COLS as i32) {
        return (
            (f64::from(c[0]) * 255.0) as i32,
            (f64::from(c[1]) * 255.0) as i32,
            (f64::from(c[2]) * 255.0) as i32,
        );
    }
    let found = ctx
        .user_col_indx
        .iter()
        .position(|&i| i == color)
        .and_then(|i| ctx.user_colors.get(i));
    match found {
        Some(u) => (u.r, u.g, u.b),
        None => {
            ctx.warn(format!("getrgb: color {color} is undefined (program error)."));
            // fig2dev leaves the values unset.
            (0, 0, 0)
        }
    }
}

/// `direction`: the distance from `q` to `p` and the normalised direction;
/// `false` if they coincide (the direction is then not normalised).
fn direction(p: Point, q: Point) -> (Dir, f64, bool) {
    let mut dir = Dir {
        x: f64::from(p.x) - f64::from(q.x),
        y: f64::from(p.y) - f64::from(q.y),
    };
    let dist = fma(dir.x, dir.x, dir.y * dir.y).sqrt();
    if dist < EPSILON {
        return (dir, dist, false);
    }
    dir.x /= dist;
    dir.y /= dist;
    (dir, dist, true)
}

/// `ARROW_EXTRA_LEN`: arrows appear this much longer with projected caps.
fn arrow_extra_len(a: &Arrow) -> f64 {
    a.ht / a.wid * a.thickness
}

/// `rotate`: counterclockwise around the origin.
fn rotate(x: f64, y: f64, angle: f64) -> (f64, f64) {
    let (s, c) = (angle.sin(), angle.cos());
    (fma(c, x, s * y), fma(-s, x, c * y))
}

/// `arc_rotate`: rotate `p` around `(cx, cy)`.
fn arc_rotate(p: &mut Point, cx: f64, cy: f64, angle: f64) {
    let (x, y) = rotate(f64::from(p.x) - cx, f64::from(p.y) - cy, angle);
    p.x = round(x + cx);
    p.y = round(y + cy);
}

/// `a * b + c` as fig2dev's build computes it: Apple clang contracts it
/// into a fused multiply-add (`-ffp-contract=on`).
pub(crate) fn fma(a: f64, b: f64, c: f64) -> f64 {
    a.mul_add(b, c)
}

fn distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let (dx, dy) = (x2 - x1, y2 - y1);
    fma(dx, dx, dy * dy).sqrt()
}

impl Cgm {
    fn linetype(&mut self, out: &mut Out, t: i32) {
        if t == self.linetype {
            return;
        }
        self.linetype = t;
        fprintf!(out, "linetype {};\n", conv_linetype(t) + 1);
    }

    fn edgetype(&mut self, out: &mut Out, t: i32) {
        if t == self.edgetype {
            return;
        }
        self.edgetype = t;
        fprintf!(out, "edgetype {};\n", conv_linetype(t) + 1);
    }

    fn linewidth(&mut self, out: &mut Out, w: i32) {
        if w == self.linewidth {
            return;
        }
        self.linewidth = w;
        fprintf!(out, "linewidth {};\n", w);
    }

    fn edgewidth(&mut self, out: &mut Out, w: i32) {
        if w == self.edgewidth {
            return;
        }
        self.edgewidth = w;
        fprintf!(out, "edgewidth {};\n", w);
    }

    fn linecolr(&mut self, ctx: &Ctx, out: &mut Out, c: i32) {
        if c == self.linecolr {
            return;
        }
        self.linecolr = c;
        fprintf!(out, "linecolr {};\n", conv_color(ctx, c));
    }

    fn edgecolr(&mut self, ctx: &Ctx, out: &mut Out, c: i32) {
        if c == self.edgecolr {
            return;
        }
        self.edgecolr = c;
        fprintf!(out, "edgecolr {};\n", conv_color(ctx, c));
    }

    fn edgevis(&mut self, out: &mut Out, on: i32) {
        if on == self.edgevis {
            return;
        }
        self.edgevis = on;
        fprintf!(out, "edgevis {};\n", if on != 0 { "ON" } else { "OFF" });
    }

    fn lineattr(&mut self, ctx: &Ctx, out: &mut Out, t: i32, w: i32, c: i32) {
        self.linetype(out, t);
        self.linewidth(out, w);
        self.linecolr(ctx, out, c);
    }

    fn edgeattr(&mut self, ctx: &Ctx, out: &mut Out, vis: i32, t: i32, w: i32, c: i32) {
        self.edgevis(out, vis);
        self.edgetype(out, t);
        self.edgewidth(out, w);
        self.edgecolr(ctx, out, c);
    }

    fn intstyle(&mut self, out: &mut Out, s: IntStyle) {
        if Some(s) == self.intstyle {
            return;
        }
        self.intstyle = Some(s);
        out.puts(match s {
            IntStyle::Solid => "intstyle SOLID;\n",
            IntStyle::Hatch => "intstyle HATCH;\n",
            IntStyle::Empty => "intstyle EMPTY;\n",
        });
    }

    /// `_fillcolr`: unconditionally.
    fn fillcolr_now(&mut self, ctx: &Ctx, out: &mut Out, c: i32) {
        self.fillcolor = c;
        fprintf!(out, "fillcolr {};\n", conv_color(ctx, c));
    }

    fn fillcolr(&mut self, ctx: &Ctx, out: &mut Out, c: i32) {
        if c == self.fillcolor {
            return;
        }
        self.fillcolr_now(ctx, out, c);
    }

    /// `fillcolrgb`: a computed fill colour, through the extra table entry.
    fn fillcolrgb(&mut self, ctx: &Ctx, out: &mut Out, r: i32, g: i32, b: i32) {
        let rgb = r.wrapping_mul(256).wrapping_add(g).wrapping_mul(256).wrapping_add(b);
        if rgb != self.fillrgb {
            self.fillrgb = rgb;
            fprintf!(
                out,
                "colrtable {} {} {} {};\n",
                conv_color(ctx, FILL_COLOR_INDEX),
                r,
                g,
                b
            );
            self.fillcolr_now(ctx, out, FILL_COLOR_INDEX);
        } else {
            self.fillcolr(ctx, out, FILL_COLOR_INDEX);
        }
    }

    fn hatchindex(&mut self, out: &mut Out, index: i32) {
        if index == self.hatchindex {
            return;
        }
        self.hatchindex = index;
        let h = usize::try_from(index)
            .ok()
            .and_then(|i| MAP_PATTERN.get(i))
            .copied()
            .unwrap_or(0);
        fprintf!(out, "hatchindex {};\n", h);
    }

    /// `fillshade`: the fill colour of fill styles 0 to 40.
    fn fillshade(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Attrs) {
        match l.fill_color {
            -1 | 0 => {
                let shade = round(f64::from((20 - l.fill_style) as f32) * 255.0 / 20.0);
                self.fillcolrgb(ctx, out, shade, shade, shade);
            }
            7 => {
                let shade = round(f64::from(l.fill_style as f32) * 255.0 / 20.0);
                self.fillcolrgb(ctx, out, shade, shade, shade);
            }
            _ => {
                // fig2dev computes the tints in `float`.
                if l.fill_style == 0 {
                    self.fillcolr(ctx, out, 0);
                } else if l.fill_style < 20 {
                    let (r, g, b) = getrgb(ctx, l.fill_color);
                    let f = (f64::from(l.fill_style) / 20.0) as f32;
                    let m = |v: i32| round(f64::from(v as f32 * f));
                    self.fillcolrgb(ctx, out, m(r), m(g), m(b));
                } else if l.fill_style == 20 {
                    self.fillcolr(ctx, out, l.fill_color);
                } else if l.fill_style < 40 {
                    let (r, g, b) = getrgb(ctx, l.fill_color);
                    let f = (f64::from(l.fill_style - 20) / 20.0) as f32;
                    let m = |v: i32| round(f64::from(f.mul_add((255 - v) as f32, v as f32)));
                    self.fillcolrgb(ctx, out, m(r), m(g), m(b));
                } else if l.fill_style == 40 {
                    self.fillcolr(ctx, out, 7);
                }
            }
        }
    }

    /// `shape`: the interior and outline of a simple closed shape.
    fn shape(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Attrs, draw: &Draw<'_>) {
        let style = l.fill_style;
        if style < 0 {
            self.intstyle(out, IntStyle::Empty);
            self.edgeattr(ctx, out, 1, l.style, l.thickness, l.pen_color);
            draw_shape(ctx, out, draw);
            return;
        }
        self.intstyle(out, IntStyle::Solid);
        if style <= 40 {
            self.fillshade(ctx, out, l);
            self.edgeattr(ctx, out, 1, l.style, l.thickness, l.pen_color);
        } else {
            // A pattern: first the background colour.
            self.fillcolr(ctx, out, l.fill_color);
            self.edgevis(out, 0);
        }
        draw_shape(ctx, out, draw);
        if style > 40 {
            self.intstyle(out, IntStyle::Hatch);
            self.hatchindex(out, style - 41);
            self.fillcolr(ctx, out, l.pen_color);
            self.edgeattr(ctx, out, 1, l.style, l.thickness, l.pen_color);
            draw_shape(ctx, out, draw);
        }
    }

    /// `shape_interior`: the interior of a shape whose outline is drawn
    /// apart (polylines, arc boxes, arcs).
    fn shape_interior(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Attrs, draw: &Draw<'_>) {
        let style = l.fill_style;
        if style < 0 {
            return;
        }
        self.edgevis(out, 0);
        self.intstyle(out, IntStyle::Solid);
        if style <= 40 {
            self.fillshade(ctx, out, l);
        } else {
            self.fillcolr(ctx, out, l.fill_color);
        }
        draw_shape(ctx, out, draw);
        if style > 40 {
            self.intstyle(out, IntStyle::Hatch);
            self.hatchindex(out, style - 41);
            self.fillcolr(ctx, out, l.pen_color);
            draw_shape(ctx, out, draw);
        }
    }

    /// `arrow_length`: how much a line is shortened for an arrow.
    fn arrow_length(&self, a: &Arrow) -> f64 {
        let mut len = match a.type_ {
            0 => arrow_extra_len(a),
            1 => a.ht,
            2 => ARROW_INDENT_DIST * a.ht,
            3 => ARROW_POINT_DIST * a.ht,
            _ => 0.0,
        };
        if self.rounded_arrows {
            len -= arrow_extra_len(a);
        }
        len
    }

    /// `cgm_arrow`: the arrow ending at `tip`, pointing along `dir`.
    fn arrow(&mut self, ctx: &mut Ctx, out: &mut Out, tip: Point, a: &Arrow, pen_color: i32, dir: Dir) {
        let mut p = tip;
        if !self.rounded_arrows {
            // Move the arrow back to let it end at the right spot.
            let f = arrow_extra_len(a);
            p.x = round(fma(-f, dir.x, f64::from(p.x)));
            p.y = round(fma(-f, dir.y, f64::from(p.y)));
        }
        let (px, py) = (f64::from(p.x), f64::from(p.y));
        let s1 = Point::new(
            round(fma(a.wid * 0.5, dir.y, fma(-a.ht, dir.x, px))),
            round(fma(-(a.wid * 0.5), dir.x, fma(-a.ht, dir.y, py))),
        );
        let s2 = Point::new(
            round(fma(-(a.wid * 0.5), dir.y, fma(-a.ht, dir.x, px))),
            round(fma(a.wid * 0.5, dir.x, fma(-a.ht, dir.y, py))),
        );
        if a.type_ == 0 {
            self.lineattr(ctx, out, 0, a.thickness as i32, pen_color);
        } else {
            self.intstyle(out, IntStyle::Solid);
            match a.style {
                0 => self.fillcolr(ctx, out, 7),
                1 => self.fillcolr(ctx, out, pen_color),
                s => ctx.warn(format!("Unsupported FIG arrow style {s} !!")),
            }
            self.edgeattr(ctx, out, 1, 0, a.thickness as i32, pen_color);
        }
        let back = |dist: f64| {
            Point::new(
                round(fma(-(a.ht * dist), dir.x, px)),
                round(fma(-(a.ht * dist), dir.y, py)),
            )
        };
        let (kind, t) = match a.type_ {
            0 => ("line ", None),
            1 => ("polygon ", None),
            2 => ("polygon ", Some(back(ARROW_INDENT_DIST))),
            3 => ("polygon ", Some(back(ARROW_POINT_DIST))),
            other => {
                ctx.warn(format!("Unsupported FIG arrow type {other}."));
                return;
            }
        };
        out.puts(kind);
        point(ctx, out, s1);
        point(ctx, out, p);
        point(ctx, out, s2);
        if let Some(t) = t {
            point(ctx, out, t);
        }
        out.puts(";\n");
    }

    /// `polyline_arrow_adjust`: move `p` back to where the arrow from `q`
    /// to `p` starts. `true` if the arrow is shorter than the segment;
    /// otherwise the line under the arrow is erased.
    fn polyline_arrow_adjust(&self, ctx: &mut Ctx, p: &mut Point, q: Point, a: &Arrow) -> bool {
        let (dir, d, ok) = direction(*p, q);
        if ok {
            let l = self.arrow_length(a);
            p.x = round(fma(-l, dir.x, f64::from(p.x)));
            p.y = round(fma(-l, dir.y, f64::from(p.y)));
            return l < d;
        }
        ctx.warn("Warning: arrow at zero-length line segment omitted.");
        true
    }

    /// `polyline`: the outline of a polyline, with its arrows.
    fn polyline(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) {
        let pts = &l.points;
        let Some(&first) = pts.first() else {
            return;
        };
        if pts.len() == 1 {
            out.puts("line ");
            point(ctx, out, first);
            point(ctx, out, first);
            out.puts(";\n");
            if l.for_arrow.is_some() || l.back_arrow.is_some() {
                ctx.warn("Warning: arrow at zero-length line segment omitted.");
            }
            return;
        }
        let (mut erase_head, mut erase_tail) = (false, false);
        let (mut p0, mut pn) = (first, first);
        out.puts("line");
        let last = pts.len() - 1;
        for (count, &p) in pts.iter().enumerate() {
            if count != 0 && count % 5 == 0 {
                out.puts("\n\t  ");
            }
            out.puts(" ");
            match (&l.back_arrow, &l.for_arrow) {
                (Some(a), _) if count == 0 => {
                    p0 = p;
                    let q = pts.get(1).copied().unwrap_or(p);
                    erase_head = !self.polyline_arrow_adjust(ctx, &mut p0, q, a);
                    point(ctx, out, if erase_head { p } else { p0 });
                }
                (_, Some(a)) if count == last => {
                    pn = p;
                    let q = pts.get(count.wrapping_sub(1)).copied().unwrap_or(p);
                    erase_tail = !self.polyline_arrow_adjust(ctx, &mut pn, q, a);
                    point(ctx, out, if erase_tail { p } else { pn });
                }
                _ => point(ctx, out, p),
            }
        }
        out.puts(";\n");

        let half = f64::from(l.thickness) / 2.0;
        if let (Some(a), Some(&p), Some(&q)) = (&l.back_arrow, pts.first(), pts.get(1)) {
            let (dir, _, ok) = direction(p, q);
            if ok {
                if erase_head {
                    let qq = Point::new(
                        round(f64::from(p.x) + dir.x * half),
                        round(f64::from(p.y) + dir.y * half),
                    );
                    self.lineattr(ctx, out, 0, l.thickness.wrapping_mul(3) / 2, 7);
                    line(ctx, out, qq, p0);
                }
                self.arrow(ctx, out, p, a, l.pen_color, dir);
            }
        }
        if let (Some(a), Some(&p), Some(&q)) = (&l.for_arrow, pts.get(last), pts.get(last.wrapping_sub(1))) {
            let (dir, _, ok) = direction(p, q);
            if ok {
                if erase_tail {
                    let qq = Point::new(
                        round(f64::from(p.x) + dir.x * half),
                        round(f64::from(p.y) + dir.y * half),
                    );
                    self.lineattr(ctx, out, 0, l.thickness.wrapping_mul(3) / 2, 7);
                    line(ctx, out, pn, qq);
                }
                self.arrow(ctx, out, p, a, l.pen_color, dir);
            }
        }
    }

    /// `arc_arrow_adjust`: move `p` back along the arc to where the arrow
    /// ending at `p` starts.
    fn arc_arrow_adjust(&self, p: &mut Point, cx: f64, cy: f64, r: f64, arw: Option<&Arrow>, dir: f64) {
        let Some(arw) = arw else {
            return;
        };
        let l = if arw.type_ != 0 { self.arrow_length(arw) } else { arw.ht };
        arc_rotate(p, cx, cy, dir * PI * l / fma(2.0, r, l));
    }

    /// `arc_arrow`: the arc's arrow ending at `p` and starting at `q`.
    fn arc_arrow(&mut self, ctx: &mut Ctx, out: &mut Out, p: Point, q: Point, arw: Option<&Arrow>, pen_color: i32) {
        let Some(arw) = arw else {
            return;
        };
        let (dir, _, _) = direction(p, q);
        self.arrow(ctx, out, p, arw, pen_color, dir);
        if arw.type_ == 0 {
            // The middle leg of an old-style stick arrow.
            let f = self.arrow_length(arw);
            let tip = Point::new(
                round(fma(-f, dir.x, f64::from(p.x))),
                round(fma(-f, dir.y, f64::from(p.y))),
            );
            line(ctx, out, q, tip);
        }
    }

    fn arc_with_arrows(&mut self, ctx: &mut Ctx, out: &mut Out, a: &ArcCopy) {
        let (cx, cy) = a.center;
        let [a0, a1, a2] = a.point;
        let r = (distance(f64::from(a0.x), f64::from(a0.y), cx, cy)
            + distance(f64::from(a1.x), f64::from(a1.y), cx, cy)
            + distance(f64::from(a2.x), f64::from(a2.y), cx, cy))
            / 3.0;
        let mut p0 = a0;
        self.arc_arrow_adjust(&mut p0, cx, cy, r, a.back_arrow.as_ref(), 1.0);
        let mut p2 = a2;
        self.arc_arrow_adjust(&mut p2, cx, cy, r, a.for_arrow.as_ref(), -1.0);
        // arc_midpoint: P1 between P0 and P2.
        let (_, d, _) = direction(p0, p2);
        let mut p1 = p0;
        arc_rotate(&mut p1, cx, cy, PI * d / fma(2.0, r, d) / 2.0);

        out.puts("arc3pt ");
        point(ctx, out, p0);
        out.puts(" ");
        point(ctx, out, p1);
        out.puts(" ");
        point(ctx, out, p2);
        out.puts(";\n");

        self.arc_arrow(ctx, out, a0, p0, a.back_arrow.as_ref(), a.pen_color);
        self.arc_arrow(ctx, out, a2, p2, a.for_arrow.as_ref(), a.pen_color);
    }

    fn arc_outline(&mut self, ctx: &mut Ctx, out: &mut Out, a: &ArcCopy) {
        if a.type_ == T_OPEN_ARC && a.thickness != 0 && (a.back_arrow.is_some() || a.for_arrow.is_some()) {
            self.arc_with_arrows(ctx, out, a);
            return;
        }
        let [p0, p1, p2] = a.point;
        out.puts("arc3pt ");
        point(ctx, out, p0);
        out.puts(" ");
        point(ctx, out, p1);
        out.puts(" ");
        point(ctx, out, p2);
        out.puts(";\n");
        if a.type_ == T_PIE_WEDGE_ARC {
            // Close the wedge.
            out.puts("line ");
            point(ctx, out, p2);
            out.puts(" ");
            pos(ctx, out, round(a.center.0), round(a.center.1));
            out.puts(" ");
            point(ctx, out, p0);
            out.puts(";\n");
        }
    }

    fn texttype(&mut self, ctx: &mut Ctx, out: &mut Out, t: i32) {
        if t == self.texttype {
            return;
        }
        self.texttype = t;
        match t {
            T_LEFT_JUSTIFIED => out.puts("textalign left base 0.0 0.0;\n"),
            T_CENTER_JUSTIFIED => out.puts("textalign ctr base 0.0 0.0;\n"),
            T_RIGHT_JUSTIFIED => out.puts("textalign right base 0.0 0.0;\n"),
            _ => ctx.warn(format!("Unsupported FIG text type {t}.")),
        }
    }

    fn textfont(&mut self, ctx: &mut Ctx, out: &mut Out, font: i32, flags: i32) {
        let font = conv_fontindex(ctx, font, flags);
        if font == self.textfont {
            return;
        }
        self.textfont = font;
        fprintf!(out, "textfontindex {};\n", font);
    }

    fn textcolr(&mut self, ctx: &Ctx, out: &mut Out, c: i32) {
        if c == self.textcolr {
            return;
        }
        self.textcolr = c;
        fprintf!(out, "textcolr {};\n", conv_color(ctx, c));
    }

    fn textsize(&mut self, ctx: &Ctx, out: &mut Out, size: f64) {
        if size == self.textsize {
            return;
        }
        self.textsize = size;
        // Adjust for differences in ppi (Fig 2.x vs 3.x).
        fprintf!(
            out,
            "charheight {};\n",
            round(10.0 * size * ctx.ppi / 1200.0 / ctx.fontmag)
        );
    }

    fn textangle(&mut self, out: &mut Out, angle: f64) {
        if angle == self.textangle {
            return;
        }
        self.textangle = angle;
        let c = round(1200.0 * angle.cos());
        let s = round(1200.0 * angle.sin());
        fprintf!(out, "charori ({},{}) ({},{});\n", s.wrapping_neg(), c, c, s);
    }
}

/// `conv_fontindex`: a Fig font → an index into the header's font list.
fn conv_fontindex(ctx: &mut Ctx, font: i32, flags: i32) -> i32 {
    if flags & 4 != 0 {
        let font = font.max(0);
        if font >= NUM_PS_FONTS {
            ctx.warn(format!("Unsupported FIG PostScript font index {font}."));
            1
        } else {
            PS_FONT_BASE + font
        }
    } else {
        // The default font is Roman.
        let font = if font <= 0 { 1 } else { font };
        if font >= NUM_LATEX_FONTS {
            ctx.warn(format!("Unsupported FIG LaTeX font index {font}."));
            1
        } else {
            LATEX_FONT_BASE + font - 1
        }
    }
}

/// `line`: a line between two points.
fn line(ctx: &Ctx, out: &mut Out, p: Point, q: Point) {
    out.puts("line ");
    point(ctx, out, p);
    out.puts(" ");
    point(ctx, out, q);
    out.puts(";\n");
}

fn circle(ctx: &Ctx, out: &mut Out, cx: i32, cy: i32, r: i32) {
    out.puts("circle ");
    pos(ctx, out, cx, cy);
    fprintf!(out, " {};\n", r);
}

/// `arcboxsetup`: the corners, ordered, and the radius.
fn arcbox_setup(l: &Line) -> Option<(i32, i32, i32, i32, i32)> {
    let (p0, p2) = (l.points.first()?, l.points.get(2)?);
    let (llx, urx) = (p0.x.min(p2.x), p0.x.max(p2.x));
    let (lly, ury) = (p0.y.min(p2.y), p0.y.max(p2.y));
    Some((llx, lly, urx, ury, l.radius))
}

fn arcctr(ctx: &Ctx, out: &mut Out, c: (i32, i32), p1: (i32, i32), p2: (i32, i32), r: i32) {
    out.puts("arcctr ");
    pos(ctx, out, c.0, c.1);
    out.puts(" ");
    relpos(out, p2.0, p2.1);
    out.puts(" ");
    relpos(out, p1.0, p1.1);
    fprintf!(out, " {};\n", r);
}

fn arcbox_outline(ctx: &Ctx, out: &mut Out, l: &Line) {
    let Some((llx, lly, urx, ury, r)) = arcbox_setup(l) else {
        return;
    };
    let seg =
        |out: &mut Out, x1: i32, y1: i32, x2: i32, y2: i32| line(ctx, out, Point::new(x1, y1), Point::new(x2, y2));
    let nr = r.wrapping_neg();
    seg(out, llx, lly.wrapping_add(r), llx, ury.wrapping_sub(r));
    seg(out, llx.wrapping_add(r), ury, urx.wrapping_sub(r), ury);
    seg(out, urx, ury.wrapping_sub(r), urx, lly.wrapping_add(r));
    seg(out, urx.wrapping_sub(r), lly, llx.wrapping_add(r), lly);
    arcctr(ctx, out, (llx.wrapping_add(r), ury.wrapping_sub(r)), (0, r), (nr, 0), r);
    arcctr(ctx, out, (urx.wrapping_sub(r), ury.wrapping_sub(r)), (r, 0), (0, r), r);
    arcctr(ctx, out, (urx.wrapping_sub(r), lly.wrapping_add(r)), (0, nr), (r, 0), r);
    arcctr(
        ctx,
        out,
        (llx.wrapping_add(r), lly.wrapping_add(r)),
        (nr, 0),
        (0, nr),
        r,
    );
}

fn arcbox_interior(ctx: &Ctx, out: &mut Out, l: &Line) {
    let Some((llx, lly, urx, ury, r)) = arcbox_setup(l) else {
        return;
    };
    let (llxr, llyr) = (llx.wrapping_add(r), lly.wrapping_add(r));
    let (urxr, uryr) = (urx.wrapping_sub(r), ury.wrapping_sub(r));
    let pts = [
        (llx, llyr, " "),
        (llx, uryr, " "),
        (llxr, uryr, " "),
        (llxr, ury, "\n\t  "),
        (urxr, ury, " "),
        (urxr, uryr, " "),
        (urx, uryr, " "),
        (urx, llyr, "\n\t  "),
        (urxr, llyr, " "),
        (urxr, lly, " "),
        (llxr, lly, " "),
        (llxr, llyr, ";\n"),
    ];
    out.puts("polygon ");
    for (x, y, sep) in pts {
        pos(ctx, out, x, y);
        out.puts(sep);
    }
    circle(ctx, out, llxr, uryr, r);
    circle(ctx, out, urxr, uryr, r);
    circle(ctx, out, urxr, llyr, r);
    circle(ctx, out, llxr, llyr, r);
}

/// Draw one of the shapes `shape` and `shape_interior` take.
fn draw_shape(ctx: &mut Ctx, out: &mut Out, draw: &Draw<'_>) {
    match draw {
        Draw::Rect(pts) => {
            let (Some(&p0), Some(&p2)) = (pts.first(), pts.get(2)) else {
                ctx.warn("Warning: Invalid FIG box omitted.");
                return;
            };
            out.puts("rect ");
            point(ctx, out, p0);
            out.puts(" ");
            point(ctx, out, p2);
            out.puts(";\n");
        }
        Draw::Polygon(pts) => {
            out.puts("polygon");
            for (count, &p) in pts.iter().enumerate() {
                if count != 0 && count % 5 == 0 {
                    out.puts("\n\t  ");
                }
                out.puts(" ");
                point(ctx, out, p);
            }
            out.puts(";\n");
        }
        Draw::ArcBoxInterior(l) => arcbox_interior(ctx, out, l),
        Draw::Circle(e) => circle(ctx, out, e.center.x, e.center.y, e.radiuses.x),
        Draw::Ellipse(e) => {
            // ellipsetup: the ends of the two axes.
            let (cx, cy) = (f64::from(e.center.x), f64::from(e.center.y));
            let (x1, y1) = rotate(f64::from(e.radiuses.x), 0.0, e.angle);
            let (x2, y2) = rotate(0.0, f64::from(e.radiuses.y), e.angle);
            out.puts("ellipse ");
            pos(ctx, out, e.center.x, e.center.y);
            out.puts(" ");
            pos(ctx, out, (x1 + cx) as i32, (y1 + cy) as i32);
            out.puts(" ");
            pos(ctx, out, (x2 + cx) as i32, (y2 + cy) as i32);
            out.puts(";\n");
        }
        Draw::ArcInterior(a) => {
            let [p0, p1, p2] = a.point;
            out.puts("arc3ptclose ");
            point(ctx, out, p0);
            out.puts(" ");
            point(ctx, out, p1);
            out.puts(" ");
            point(ctx, out, p2);
            let kind = match a.type_ {
                T_OPEN_ARC => "CHORD",
                T_PIE_WEDGE_ARC => "PIE",
                t => {
                    ctx.warn(format!("Unsupported FIG arc type {t}."));
                    "UNKNOWN"
                }
            };
            fprintf!(out, " {};\n", kind);
        }
    }
}

/// The name of the figure for the header: the input's file name without
/// directory and extension.
fn figname(from: Option<&str>) -> String {
    let Some(from) = from else {
        return "(stdin)".into();
    };
    let base = from.rsplit('/').next().unwrap_or(from);
    base.split('.').next().unwrap_or(base).to_string()
}

/// The encoding fig2dev's texts are in: the reader decoded them to
/// Unicode, fig2dev writes the input's bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TextEnc {
    #[default]
    Utf8,
    Latin1,
    Latin2,
}

/// The encoding the reader decoded the texts from: `-E` (or the file's
/// `#encoding: UTF-8` line), else UTF-8 if the input file is valid UTF-8
/// and ISO-8859-1 otherwise (figures from stdin count as UTF-8).
pub(crate) fn text_encoding(ctx: &Ctx) -> TextEnc {
    let Some(name) = ctx.input_encoding.as_deref() else {
        let latin1 = ctx
            .from
            .as_deref()
            .and_then(|f| std::fs::read(f).ok())
            .is_some_and(|d| std::str::from_utf8(&d).is_err());
        return if latin1 { TextEnc::Latin1 } else { TextEnc::Utf8 };
    };
    let key: String = name
        .chars()
        .filter(|c| !matches!(c, '-' | '_' | ' '))
        .map(|c| c.to_ascii_lowercase())
        .collect();
    match key.as_str() {
        "utf8" | "0" => TextEnc::Utf8,
        "iso88592" | "latin2" | "l2" | "2" => TextEnc::Latin2,
        _ => TextEnc::Latin1,
    }
}

/// The bytes of a text in `enc`.
pub(crate) fn text_bytes(enc: TextEnc, s: &str) -> Vec<u8> {
    match enc {
        TextEnc::Utf8 => s.as_bytes().to_vec(),
        TextEnc::Latin2 => s
            .chars()
            .map(|c| match u8::try_from(u32::from(c)) {
                Ok(b) if b < 0xA0 => b,
                _ => crate::util::iso2tex::unicode_to_iso2(c).unwrap_or(b'?'),
            })
            .collect(),
        TextEnc::Latin1 => s.chars().map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?')).collect(),
    }
}

impl Driver for Cgm {
    fn option(&mut self, _ctx: &mut Ctx, opt: char, _arg: Option<&str>) -> Result<(), String> {
        // fig2dev resets -r with every option.
        self.rounded_arrows = false;
        match opt {
            'a' => self.binary_output = true,
            'r' => self.rounded_arrows = true,
            'G' | 'L' => {}
            _ => return Err(format!("Argument -{opt} unknown to cgm driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        let figname = figname(ctx.from.as_deref());
        self.enc = text_encoding(ctx);
        if self.binary_output {
            // fig2dev pipes the output through ralcgm; RustFig runs no
            // external programs, as when fig2dev can't open the pipe.
            let cmd = format!("ralcgm -b - {}", ctx.to.as_deref().unwrap_or("-"));
            ctx.warn("fig2dev: Can't open pipe to ralcgm, producing ASCII CGM instead.");
            ctx.warn(format!("Command was: {cmd}"));
            self.binary_output = false;
        }
        let from = ctx.from.as_deref().unwrap_or("(stdin)");
        fprintf!(out, "BEGMF '{}';\n", figname);
        out.puts("mfversion 1;\n");
        fprintf!(out, "mfdesc 'Converted from {} using fig2dev -Lcgm';\n", from);
        out.puts("mfelemlist 'DRAWINGPLUS';\n");
        out.puts("vdctype integer;\n");
        out.puts(FONTLIST);
        out.puts("BEGMFDEFAULTS;\n");
        fprintf!(
            out,
            "  vdcext (0,0) ({},{});\n",
            ctx.urx.wrapping_sub(ctx.llx),
            ctx.ury.wrapping_sub(ctx.lly)
        );
        out.puts("  clip off;\n");
        out.puts("  colrmode indexed;\n");
        out.puts("  colrtable 1");
        for c in STDCOLS {
            let v = |x: f32| (f64::from(x) * 255.0) as i32;
            fprintf!(out, "\n\t{} {} {}", v(c[0]), v(c[1]), v(c[2]));
        }
        for u in &ctx.user_colors {
            fprintf!(out, "\n\t{} {} {}", u.r, u.g, u.b);
        }
        out.puts(";\n");
        out.puts("  linewidthmode abs;\n");
        out.puts("  edgewidthmode abs;\n");
        out.puts("  backcolr 255 255 255;\n");
        out.puts("  textprec stroke;\n");
        out.puts("  transparency ON;\n");
        out.puts("ENDMFDEFAULTS;\n");
        fprintf!(out, "BEGPIC '{}';\n", figname);
        out.puts("BEGPICBODY;\n");
        if !objects.comments.is_empty() {
            out.puts("% %\n");
            crate::gendev::print_comments(out, "% ", &objects.comments, " %");
            out.puts("% %\n");
        }
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, arc: &Arc) -> Result<(), String> {
        crate::gendev::print_comments(out, "% ", &arc.comments, " %");
        out.puts("% Arc %\n");
        let mut a = ArcCopy {
            type_: arc.type_,
            thickness: arc.thickness,
            pen_color: arc.pen_color,
            for_arrow: arc.for_arrow,
            back_arrow: arc.back_arrow,
            center: (arc.center.x, arc.center.y),
            point: arc.point,
        };
        // cwarc: make the arc counterclockwise.
        let [p0, p1, p2] = a.point;
        let (x1, y1) = (p1.x.wrapping_sub(p0.x), p1.y.wrapping_sub(p0.y));
        let (x2, y2) = (p2.x.wrapping_sub(p1.x), p2.y.wrapping_sub(p1.y));
        if x1.wrapping_mul(y2).wrapping_sub(y1.wrapping_mul(x2)) > 0 {
            a.point.swap(0, 2);
            std::mem::swap(&mut a.for_arrow, &mut a.back_arrow);
        }
        let attrs = Attrs {
            style: arc.style,
            thickness: arc.thickness,
            pen_color: arc.pen_color,
            fill_color: arc.fill_color,
            fill_style: arc.fill_style,
        };
        self.shape_interior(ctx, out, &attrs, &Draw::ArcInterior(&a));
        if a.thickness > 0 {
            self.lineattr(ctx, out, arc.style, arc.thickness, arc.pen_color);
            self.arc_outline(ctx, out, &a);
        }
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        crate::gendev::print_comments(out, "% ", &e.comments, " %");
        let attrs = Attrs {
            style: e.style,
            thickness: e.thickness,
            pen_color: e.pen_color,
            fill_color: e.fill_color,
            fill_style: e.fill_style,
        };
        match e.type_ {
            T_ELLIPSE_BY_RAD | T_ELLIPSE_BY_DIA => {
                out.puts("% Ellipse %\n");
                self.shape(ctx, out, &attrs, &Draw::Ellipse(e));
            }
            T_CIRCLE_BY_RAD | T_CIRCLE_BY_DIA => {
                out.puts("% Circle %\n");
                self.shape(ctx, out, &attrs, &Draw::Circle(e));
            }
            t => ctx.warn(format!("Unsupported FIG ellipse type {t}.")),
        }
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        crate::gendev::print_comments(out, "% ", &l.comments, " %");
        let attrs = Attrs {
            style: l.style,
            thickness: l.thickness,
            pen_color: l.pen_color,
            fill_color: l.fill_color,
            fill_style: l.fill_style,
        };
        match l.type_ {
            T_POLYLINE => {
                out.puts("% Polyline %\n");
                self.shape_interior(ctx, out, &attrs, &Draw::Polygon(&l.points));
                self.lineattr(ctx, out, l.style, l.thickness, l.pen_color);
                self.polyline(ctx, out, l);
            }
            T_BOX => {
                out.puts("% Box %\n");
                self.shape(ctx, out, &attrs, &Draw::Rect(&l.points));
            }
            T_POLYGON => {
                out.puts("% Polygon %\n");
                self.shape(ctx, out, &attrs, &Draw::Polygon(&l.points));
            }
            T_ARC_BOX => {
                out.puts("% Arc Box %\n");
                self.shape_interior(ctx, out, &attrs, &Draw::ArcBoxInterior(l));
                self.lineattr(ctx, out, l.style, l.thickness, l.pen_color);
                arcbox_outline(ctx, out, l);
            }
            T_PIC_BOX => {
                out.puts("% Imported Picture %\n");
                if !self.pic_warned {
                    ctx.warn("Warning: Pictures not supported in CGM language");
                    self.pic_warned = true;
                }
                self.polyline(ctx, out, l);
            }
            t => ctx.warn(format!("Unsupported FIG polyline type {t}.")),
        }
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        crate::gendev::print_comments(out, "% ", &s.comments, "");
        if !self.spline_warned {
            ctx.warn("Warning: the CGM driver doesn't support (old style) FIG splines.");
            ctx.warn("Suggestion: convert your (old?) FIG image by loading it into xfig v3.2");
            ctx.warn("or higher and saving again.");
            self.spline_warned = true;
        }
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String> {
        crate::gendev::print_comments(out, "% ", &t.comments, " %");
        out.puts("% Text %\n");
        self.textfont(ctx, out, t.font, t.flags);
        self.texttype(ctx, out, t.type_);
        self.textcolr(ctx, out, t.color);
        self.textsize(ctx, out, t.size);
        self.textangle(out, t.angle);
        out.puts("text ");
        pos(ctx, out, t.base_x, t.base_y);
        out.puts(" final '");
        // Double any "'".
        for b in text_bytes(self.enc, &t.cstring) {
            out.bytes(&[b]);
            if b == b'\'' {
                out.bytes(b"'");
            }
        }
        out.puts("';\n");
        Ok(())
    }

    fn end(&mut self, _ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        out.puts("% End of Picture %\n");
        out.puts("ENDPIC;\n");
        out.puts("ENDMF;\n");
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}
