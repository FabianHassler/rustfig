//! fig2dev's `pict2e` driver: the figure as a LaTeX `picture` environment
//! drawn with the macros of the pict2e package.
//!
//! Coordinates are Fig units with y turned upwards (`XCOORD`, `YCOORD`);
//! the unit length is one Fig unit times the magnification, written in
//! scaled points. Lines are `\polyline`s, `\polygon`s and `\Line`s, with
//! dashes built from `\multiput` lines; arc boxes, circles and arcs are
//! `\circlearc` paths (`\oval` and `\circle` with `-e`), ellipses Bézier
//! curves (`\curveto`). Dashed ellipses, arcs and arc boxes are drawn as
//! polylines approximating an X-spline through an octagon. Arrowheads
//! are polygons, or LaTeX vectors with `-R`/`-r`. Fills set a `\color`
//! in a scope of their own; patterns are not supported (25 % of the pen
//! colour instead).
//!
//! fig2dev computes many coordinates as `round(a*b)` or `a*b + c`, which
//! clang contracts into fused multiply-adds; the port uses `mul_add` in
//! the same places, so that the output matches fig2dev's byte for byte.
//
// Ported from fig2dev dev/genpict2e.c (xfig licence, see LICENSE-xfig).

use std::f64::consts::PI;

use crate::bound::{arrow_shape, calc_arrow, compute_arcarrow_angle};
use crate::drivers::cgm::{text_bytes, text_encoding, TextEnc};
use crate::drivers::dxf::fmt_g;
use crate::drivers::latex::{c_string, set_default_size, Cf};
use crate::gendev::{print_comments, strtol, Ctx, FIG_COLOR_NAMES, NUM_STD_COLS};
use crate::object::{
    Arc, Arrow, Compound, Control, Ellipse, Line, Point, Spline, Text, BLACK_COLOR, DASH_2_DOTS_LINE, DASH_3_DOTS_LINE,
    DASH_DOT_LINE, DASH_LINE, DEFAULT, DOTTED_LINE, MAX_FONT, MAX_PSFONT, NUMSHADES, NUMTINTS, SOLID_LINE,
    S_SPLINE_APPROX, T_ARC_BOX, T_BOX, T_CENTER_JUSTIFIED, T_CLOSED_XSPLINE, T_LEFT_JUSTIFIED, T_OPEN_ARC, T_PIC_BOX,
    T_PIE_WEDGE_ARC, T_POLYGON, T_POLYLINE, T_RIGHT_JUSTIFIED, UNFILLED, WHITE_COLOR,
};
use crate::trans_spline::create_line_with_spline;
use crate::util::psfonts::PS_FONT_NAMES;
use crate::util::texfonts::{put_string_bytes, TexFonts};
use crate::{fprintf, round, Driver, Out};

const M_PI_2: f64 = std::f64::consts::FRAC_PI_2;
const M_PI_4: f64 = std::f64::consts::FRAC_PI_4;

const MITERJOIN: i32 = 0;
const BUTTCAP: i32 = 0;
const ROUNDCAP: i32 = 1;
const SQUARECAP: i32 = 2;
/// Points are written on lines up to about this many characters.
const LINEBREAK: usize = 76;
/// `insert_endpoint` looks back at most this many points.
const MAXREVERSE: usize = 100;
/// Twice the most dashes of a pattern (`PAT_ND_2`).
const PAT_ND_2: usize = 8;
/// At most this many dashes of one line segment beyond the full patterns
/// (C writes as many as the arithmetic asks for; only an overflowing
/// pattern count gets near it).
const MAX_DASHES: i32 = 1 << 20;
/// At most this many steps reduce an ellipse's angle to ±π/2 (C loops
/// forever on huge angles).
const MAX_ANGLE_STEPS: u32 = 1 << 20;

/// Where a line ends at an arrowhead (`struct pict2earrow`): at `hfac`
/// times the arrow's height plus `tfac` times its line thickness from the
/// tip; index `2*type + style`.
const PICT2EARROW: [(f64, f64); 30] = [
    (0., 0.5),  // type 0
    (0., 0.),   // place holder for what would be type 0 filled
    (1., 0.5),  // type 1a simple triangle
    (1., 0.5),  // type 1b filled simple triangle
    (1., 0.5),  // type 2a concave spearhead
    (1., 0.5),  // type 2b filled concave spearhead
    (1., -0.5), // type 3a convex spearhead
    (1., -0.5), // type 3b filled convex spearhead
    (1., -0.5), // type 4a diamond
    (1., -0.5), // type 4b filled diamond
    (1., 0.5),  // type 5a/b circle
    (1., 0.5),
    (0.5, -0.5), // type 6a/b half circle
    (0.5, -0.5),
    (1., 0.5),  // type 7a square
    (1., 0.5),  // type 7b filled square
    (1., -0.5), // type 8a reverse triangle
    (1., -0.5), // type 8b filled reverse triangle
    (1., 0.5),  // type 9a top-half filled concave spearhead
    (1., 0.5),  // type 9b bottom-half filled concave spearhead
    (1., 0.5),  // type 10o top-half simple triangle
    (1., 0.5),  // type 10f top-half filled simple triangle
    (1., 0.5),  // type 11o top-half concave spearhead
    (1., 0.5),  // type 11f top-half filled concave spearhead
    (1., -0.5), // type 12o top-half convex spearhead
    (1., -0.5), // type 12f top-half filled convex spearhead
    (1., -0.5), // type 13a "wye"
    (0., 0.5),  // type 13b bar
    (1., 0.5),  // type 14a two-prong fork
    (0., 0.5),  // type 14b backward two-prong fork
];

/// `round(a*b)` as clang compiles fig2dev's `round()` macro: the product
/// and the ±0.5 fused, the sign taken from the rounded product.
fn round_mul(a: f64, b: f64) -> i32 {
    let half = if a * b >= 0.0 { 0.5 } else { -0.5 };
    a.mul_add(b, half) as i32
}

/// `sqrt(x*x + y*y)`, contracted.
fn hypot_fma(x: f64, y: f64) -> f64 {
    x.mul_add(x, y * y).sqrt()
}

/// The distance between two points (`DISTANCE`).
fn distance(p: Point, q: Point) -> f64 {
    hypot_fma(f64::from(q.x.wrapping_sub(p.x)), f64::from(q.y.wrapping_sub(p.y)))
}

/// `gcd`, assuming 0 < a <= b (in absolute value).
fn gcd(mut a: i32, mut b: i32) -> i32 {
    loop {
        b = b.wrapping_rem(a);
        if b == 0 {
            return a;
        }
        std::mem::swap(&mut a, &mut b);
    }
}

/// `get_slope`: the slope (sx, sy) and the length lx of the line from `p`
/// to `q`.
fn get_slope(p: Point, q: Point) -> (i32, i32, i32) {
    let sx = q.x.wrapping_sub(p.x);
    let sy = q.y.wrapping_sub(p.y);
    if sx == 0 {
        return if sy > 0 {
            (sx, 1, sy)
        } else {
            (sx, -1, sy.wrapping_neg())
        };
    }
    if sy == 0 {
        return if sx > 0 {
            (1, sy, sx)
        } else {
            (-1, sy, sx.wrapping_neg())
        };
    }
    let ax = sx.wrapping_abs();
    let ay = sy.wrapping_abs();
    let div = if ax >= ay { gcd(ay, ax) } else { gcd(ax, ay) };
    (sx.wrapping_div(div), sy.wrapping_div(div), ax)
}

/// `get_lineslope`: the slope (sx, sy), with arguments below 16384, of
/// the line from `p` to `q`, the precision to print its length, and the
/// length.
fn get_lineslope(p: Point, q: Point) -> (i32, i32, usize, f64) {
    let (mut sx, mut sy, ax) = get_slope(p, q);
    let mut lx = f64::from(ax);
    let mut precl = 0;
    let ax = sx.wrapping_abs();
    let ay = sy.wrapping_abs();
    if ax < 16384 && ay < 16384 {
        return (sx, sy, precl, lx);
    }
    if ax > ay {
        let fac = 16383. / f64::from(ax);
        sx = if sx > 0 { 16383 } else { -16383 };
        sy = round_mul(f64::from(sy), fac);
    } else {
        let fac = 16383. / f64::from(ay);
        sy = if sy > 0 { 16383 } else { -16383 };
        sx = round_mul(f64::from(sx), fac);
        if sx.wrapping_abs() < 3 {
            sy = if sy > 0 { 1 } else { -1 };
            sx = 0;
        }
        // For a steep line, the length in y-direction stays the same, the
        // length in x-direction may be changed by rounding.
        let fac = (f64::from(sy) / f64::from(sx)).abs();
        lx = f64::from(q.y.wrapping_sub(p.y).wrapping_abs()) / fac;
        let ax = fac as i32;
        let mut digits: i32 = 1;
        while precl < 5 && digits < ax {
            precl += 1;
            digits = digits.wrapping_mul(10);
        }
    }
    (sx, sy, precl, lx)
}

/// `get_vectorslope`: the slope (sx, sy) and length of a vector from `p`
/// to `q`; `true` if the arguments had to be scaled to 1000.
fn get_vectorslope(p: Point, q: Point) -> (i32, i32, i32, bool) {
    let (mut sx, mut sy, lx) = get_slope(p, q);
    let ax = sx.wrapping_abs();
    let ay = sy.wrapping_abs();
    if ax > 1000 || ay > 1000 {
        if ax > ay {
            sy = round(f64::from(sy) * 1000.0 / f64::from(ax));
            sx = if sx > 0 { 1000 } else { -1000 };
        } else {
            sx = round(f64::from(sx) * 1000.0 / f64::from(ay));
            sy = if sy > 0 { 1000 } else { -1000 };
        }
        return (sx, sy, lx, true);
    }
    (sx, sy, lx, false)
}

/// `get_midpoint`: the point a distance `h` from `p` towards `q`, `None`
/// if the line is too short.
fn get_midpoint(p: Point, q: Point, h: f64) -> Option<Point> {
    let sx = f64::from(q.x.wrapping_sub(p.x));
    let sy = f64::from(q.y.wrapping_sub(p.y));
    let s = hypot_fma(sx, sy);
    (s > h).then(|| Point::new(p.x.wrapping_add(round(sx * h / s)), p.y.wrapping_add(round(sy * h / s))))
}

/// `get_startpoint`: the line from a point a distance `h1` along `pts`
/// (`Ok`), or, if the line is shorter, its final two points (`Err`).
fn get_startpoint(pts: &[Point], mut h1: f64) -> Result<Vec<Point>, (Point, Point)> {
    let mut f = (Point::default(), Point::default());
    let mut k = 0;
    loop {
        let (Some(&p), Some(&n)) = (pts.get(k), pts.get(k + 1)) else {
            // the end is reached, the line is shorter than h1
            return Err(f);
        };
        let sx = f64::from(n.x.wrapping_sub(p.x));
        let sy = f64::from(n.y.wrapping_sub(p.y));
        let s = hypot_fma(sx, sy);
        if s > h1 {
            let a = Point::new(
                p.x.wrapping_add(round(h1 * sx / s)),
                p.y.wrapping_add(round(h1 * sy / s)),
            );
            let mut line = vec![a];
            line.extend_from_slice(pts.get(k + 1..).unwrap_or(&[]));
            return Ok(line);
        }
        h1 -= s;
        f = (p, n);
        k += 1;
    }
}

/// `insert_endpoint`: the line `pts` ending a distance `h2` before its
/// end, and its original final two points. A line shorter than `h2` is
/// kept; if more than `MAXREVERSE` points lie within `h2`, the line ends
/// at the point that far back.
fn insert_endpoint(pts: &[Point], mut h2: f64) -> (Vec<Point>, (Point, Point)) {
    let n = pts.len();
    let (Some(&f0), Some(&f1)) = (pts.get(n.wrapping_sub(2)), pts.get(n.wrapping_sub(1))) else {
        let p = pts.first().copied().unwrap_or_default();
        return (pts.to_vec(), (p, p));
    };
    // fig2dev keeps the last MAXREVERSE + 1 points in a ring and walks
    // back through at most n - 1 segments, or MAXREVERSE - 1 segments
    // once the ring has wrapped around.
    let wrapped = n > MAXREVERSE + 1;
    let maxseg = if wrapped { MAXREVERSE - 1 } else { n - 1 };
    let mut k = n - 1;
    let mut seg = 0;
    while seg < maxseg {
        let (Some(&r0), Some(&r1)) = (pts.get(k - 1), pts.get(k)) else {
            break;
        };
        let sx = f64::from(r0.x.wrapping_sub(r1.x));
        let sy = f64::from(r0.y.wrapping_sub(r1.y));
        let s = hypot_fma(sx, sy);
        if s > h2 {
            let o = Point::new(
                r1.x.wrapping_add(round(h2 * sx / s)),
                r1.y.wrapping_add(round(h2 * sy / s)),
            );
            let mut line = pts.get(..k).unwrap_or(&[]).to_vec();
            line.push(o);
            return (line, (f0, f1));
        }
        h2 -= s;
        k -= 1;
        seg += 1;
    }
    if wrapped {
        // more than MAXREVERSE points within h2
        (pts.get(..=k).unwrap_or(pts).to_vec(), (f0, f1))
    } else {
        // the line is too short
        (pts.to_vec(), (f0, f1))
    }
}

/// The octagon on the unit circle, scaled by `s`, through whose vertices
/// a closed X-spline approximates a circle (`get_xsplinepoints`); and the
/// spline's controls.
fn get_xsplinepoints(s: f64) -> ([(f64, f64); 8], Vec<Control>) {
    // y = tan(pi/8) = sqrt(2) - 1
    let x = s;
    let y = 0.41421356 * s;
    let controls = vec![
        Control {
            s: S_SPLINE_APPROX,
            ..Control::default()
        };
        8
    ];
    (
        [(-y, x), (y, x), (x, y), (x, -y), (y, -x), (-y, -x), (-x, -y), (-x, y)],
        controls,
    )
}

/// `THICKNESS(T)` of an integer thickness: half of it up to
/// `THICK_SCALE` (integer division), else less `THICK_SCALE`; a float.
fn thickness_i(t: i32, ts: f32) -> f64 {
    if t as f32 <= ts {
        f64::from((t / 2) as f32)
    } else {
        f64::from(t as f32 - ts)
    }
}

/// `THICKNESS(T)` of a double.
fn thickness_d(t: f64, ts: f32) -> f64 {
    if t <= f64::from(ts) {
        t / 2.0
    } else {
        t - f64::from(ts)
    }
}

/// The dash pattern of a non-solid line (`struct pattern`): lengths of
/// dashes (even) and spaces (odd).
#[derive(Clone, Copy, Debug, Default)]
struct Pattern {
    d: [i32; 2 * PAT_ND_2],
    nd: usize,
    length: i32,
}

impl Pattern {
    fn d(&self, i: usize) -> i32 {
        self.d.get(i).copied().unwrap_or(0)
    }
    /// The index of the dash after dash `i`.
    fn next(&self, i: usize) -> usize {
        if i + 2 == self.nd {
            0
        } else {
            i + 2
        }
    }
}

/// The `pict2e` driver's state (genpict2e.c's statics).
#[derive(Debug)]
pub struct Pict2e {
    fonts: TexFonts,
    select_fontsize: bool,
    select_fontname: bool,
    epiccompatible: bool,
    removesuffix: bool,
    only_texfonts: bool,
    /// `-O`: read, but fig2dev doesn't act on it.
    allspecial: bool,
    pagemode: bool,
    allarrowsltx: bool,
    prepend: Option<String>,
    /// The arrow index (`2*type + style`) drawn as a LaTeX vector.
    ltxarrow: i32,
    verbose: bool,
    unitlength: f64,
    cur_thickness: i32,
    saved_thickness: i32,
    cur_joinstyle: i32,
    cur_capstyle: i32,
    default_color: i32,
    cur_color: i32,
    cur_shade: i32,
    border_margin: i32,
    /// fig2dev's `llx`, `ury` (after the border margin), `THICK_SCALE`
    /// and `ppi`, copied at the start.
    llx: i32,
    ury: i32,
    thick_scale: f32,
    ppi: f64,
    /// The input's text encoding: texts are written in the input's bytes.
    enc: TextEnc,
}

impl Default for Pict2e {
    fn default() -> Pict2e {
        Pict2e {
            fonts: TexFonts::new(),
            select_fontsize: true,
            select_fontname: true,
            epiccompatible: true,
            removesuffix: false,
            only_texfonts: false,
            allspecial: false,
            pagemode: false,
            allarrowsltx: false,
            prepend: None,
            ltxarrow: -1,
            verbose: false,
            unitlength: 0.0,
            cur_thickness: 0,
            saved_thickness: 0,
            cur_joinstyle: 0,
            cur_capstyle: 0,
            default_color: DEFAULT,
            cur_color: DEFAULT,
            cur_shade: NUMSHADES - 1,
            border_margin: 0,
            llx: 0,
            ury: 0,
            thick_scale: 1.0,
            ppi: 1200.0,
            enc: TextEnc::Utf8,
        }
    }
}

/// The driver for `lang`.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    (lang == "pict2e").then(|| Box::new(Pict2e::default()) as Box<dyn Driver>)
}

/// `atoi`.
fn atoi(s: &str) -> i32 {
    strtol(s).0
}

impl Pict2e {
    fn xc(&self, x: i32) -> i32 {
        x.wrapping_sub(self.llx)
    }
    fn yc(&self, y: i32) -> i32 {
        self.ury.wrapping_sub(y)
    }
    /// `(x,y)` of a point, the number of characters written.
    fn put_point(&self, out: &mut Out, p: Point) -> usize {
        let s = format!("({},{})", self.xc(p.x), self.yc(p.y));
        out.puts(&s);
        s.len()
    }

    /// `ISLTXARROW`: whether arrowhead `a` is drawn as a LaTeX vector.
    fn is_ltx_arrow(&self, a: &Arrow) -> bool {
        self.allarrowsltx || a.type_.wrapping_mul(2).wrapping_add(a.style) == self.ltxarrow
    }

    fn set_linejoin(&mut self, out: &mut Out, ctx: &mut Ctx, j: i32) {
        if j != self.cur_joinstyle {
            self.cur_joinstyle = j;
            match j {
                MITERJOIN => out.puts("\\miterjoin%\n"),
                1 => out.puts("\\roundjoin%\n"),
                2 => out.puts("\\beveljoin%\n"),
                _ => ctx.warn(format!("Undefined line join style {j}.")),
            }
        }
    }

    fn set_linecap(&mut self, out: &mut Out, ctx: &mut Ctx, j: i32) {
        if j != self.cur_capstyle {
            self.cur_capstyle = j;
            match j {
                BUTTCAP => out.puts("\\buttcap%\n"),
                ROUNDCAP => out.puts("\\roundcap%\n"),
                SQUARECAP => out.puts("\\squarecap%\n"),
                _ => ctx.warn(format!("Undefined line cap style {j}.")),
            }
        }
    }

    /// `set_linewidth`, compatible with genps.c.
    fn set_linewidth(&mut self, out: &mut Out, w: i32) {
        if w == self.cur_thickness {
            return;
        }
        self.cur_thickness = w;
        let t = fmt_g(thickness_i(w, self.thick_scale), 6);
        fprintf!(out, "\\linethickness{{{t}\\unitlength}}");
        if self.epiccompatible {
            fprintf!(out, "\\XFigeepicthickness{{{t}\\unitlength}}");
        }
        // The % at the end is necessary, otherwise objects are shifted by
        // a space.
        if w as f32 == self.thick_scale {
            out.puts("%\\thinlines\n");
        } else if w as f32 == 2.0 * self.thick_scale {
            out.puts("%\\thicklines\n");
        } else {
            out.puts("%\n");
        }
    }

    /// `get_rgbcolor`: 8-bit channels of colour `c`.
    fn get_rgbcolor(ctx: &Ctx, c: i32) -> (i32, i32, i32) {
        if c < NUM_STD_COLS as i32 {
            let v = usize::try_from(c)
                .ok()
                .and_then(|i| FIG_COLOR_NAMES.get(i))
                .and_then(|h| h.get(1..))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .unwrap_or(0);
            return ((v >> 16) as i32 & 0xff, (v >> 8) as i32 & 0xff, v as i32 & 0xff);
        }
        let u = usize::try_from(c - NUM_STD_COLS as i32)
            .ok()
            .and_then(|i| ctx.user_colors.get(i))
            .copied()
            .unwrap_or_default();
        (i32::from(u.r as u16), i32::from(u.g as u16), i32::from(u.b as u16))
    }

    /// `close_scope`: closes a colour scope, which also scopes the line
    /// thickness.
    fn close_scope(&mut self, out: &mut Out) {
        if self.verbose {
            out.puts("}% close color scope, return to color of surrounding text\n");
        } else {
            // a %, otherwise pict2e puts a space
            out.puts("}%\n");
        }
        self.cur_thickness = self.saved_thickness;
    }

    fn open_scope(&mut self, out: &mut Out) {
        if self.verbose {
            out.puts("% open color scope\n{");
        } else {
            out.puts("{");
        }
        self.saved_thickness = self.cur_thickness;
    }

    fn is_default(&self, c: i32) -> bool {
        c == DEFAULT || c == self.default_color
    }

    /// `set_color`. Always call `set_color` before `set_linewidth`.
    fn set_color(&mut self, out: &mut Out, ctx: &Ctx, col: i32) {
        if self.cur_shade == NUMSHADES - 1
            && (col == self.cur_color || (self.is_default(col) && self.is_default(self.cur_color)))
        {
            return;
        }
        if self.is_default(col) {
            self.close_scope(out);
            self.cur_color = DEFAULT;
            self.cur_shade = NUMSHADES - 1;
            return;
        } else if self.is_default(self.cur_color) {
            self.open_scope(out);
        }
        self.cur_color = col;
        self.cur_shade = NUMSHADES - 1;
        let c = if col == DEFAULT { self.default_color } else { col };
        match c {
            BLACK_COLOR => out.puts("\\color{black}\n"),
            1 => out.puts("\\color{blue}\n"),
            2 => out.puts("\\color{green}\n"),
            3 => out.puts("\\color{cyan}\n"),
            4 => out.puts("\\color{red}\n"),
            5 => out.puts("\\color{magenta}\n"),
            6 => out.puts("\\color{yellow}\n"),
            WHITE_COLOR => out.puts("\\color{white}\n"),
            _ => {
                let (r, g, b) = Self::get_rgbcolor(ctx, c);
                fprintf!(
                    out,
                    "\\color[rgb]{{{},{},{}}}\n",
                    fmt_g(f64::from(r) / 255., 3),
                    fmt_g(f64::from(g) / 255., 3),
                    fmt_g(f64::from(b) / 255., 3)
                );
            }
        }
    }

    /// `set_fillcolor`: a shade between 0 and NUMSHADES - 1 mixes the
    /// colour with black, above with white (a tint); black and white have
    /// their shades and tints swapped.
    fn set_fillcolor(&mut self, out: &mut Out, ctx: &mut Ctx, mut col: i32, mut shade: i32, pen_color: i32) {
        if shade < 0 {
            ctx.warn("A color error occurred. Please report this bug.");
        }
        if shade >= NUMSHADES + NUMTINTS {
            if col == pen_color {
                // solid fill
                shade = NUMSHADES - 1;
            } else {
                ctx.warn("Patterns not supported. Filling with 25% pen color instead.");
                col = pen_color;
                shade = NUMSHADES - 1 + 3 * NUMTINTS / 4;
            }
        }
        // pure colours
        if shade == NUMSHADES - 1 {
            self.set_color(out, ctx, col);
            self.cur_shade = shade;
            return;
        }
        // an unknown colour can not be shaded or tinted
        if col == DEFAULT {
            col = if self.default_color == DEFAULT {
                BLACK_COLOR
            } else {
                self.default_color
            };
        }
        // black
        if (shade == 0 && col != BLACK_COLOR) || (col == WHITE_COLOR && shade == NUMSHADES + NUMTINTS - 1) {
            self.set_color(out, ctx, BLACK_COLOR);
            self.cur_shade = NUMSHADES - 1;
            return;
        }
        // white
        if (shade == NUMSHADES + NUMTINTS - 1 && col != WHITE_COLOR) || (col == BLACK_COLOR && shade == 0) {
            self.set_color(out, ctx, WHITE_COLOR);
            self.cur_shade = NUMSHADES - 1;
            return;
        }
        // map shades to tints for black and tints to shades for white
        if col == BLACK_COLOR {
            col = WHITE_COLOR;
            if shade > NUMSHADES - 1 {
                shade -= NUMSHADES - 1;
            } else {
                shade = NUMSHADES - 1 - shade;
            }
        }
        if self.cur_color == col && self.cur_shade == shade {
            return;
        }
        if self.is_default(self.cur_color) && self.cur_shade == NUMSHADES - 1 {
            self.open_scope(out);
        }
        // grays
        if col == WHITE_COLOR {
            self.cur_color = col;
            self.cur_shade = shade;
            fprintf!(
                out,
                "\\color[gray]{{{}}}\n",
                fmt_g(f64::from(shade) / f64::from(NUMSHADES - 1), 3)
            );
            return;
        }
        // shaded and tinted colours
        self.cur_color = col;
        self.cur_shade = shade;
        let (r, g, b) = Self::get_rgbcolor(ctx, col);
        // 4080 = 255 << 4
        let mix = |c: i32| {
            let t = f64::from(shade);
            if shade < NUMSHADES {
                f64::from(c << 4) * t / f64::from(NUMSHADES - 1) / 4080.
            } else {
                (f64::from(c << 4) + f64::from((255 - c) << 4) * f64::from(shade - NUMSHADES + 1) / f64::from(NUMTINTS))
                    / 4080.
            }
        };
        fprintf!(
            out,
            "\\color[rgb]{{{},{},{}}}\n",
            fmt_g(mix(r), 3),
            fmt_g(mix(g), 3),
            fmt_g(mix(b), 3)
        );
    }

    /// `set_linestyle`: the dash pattern of a non-solid line; `None`
    /// (after fig2dev's message) for a zero frequency or an unknown style.
    fn set_linestyle(&self, out: &mut Out, style: i32, style_val: f64) -> Option<Pattern> {
        let v = style_val / (80.0 / self.ppi);
        if v <= 0. {
            out.puts("Error: Set_linestyle with zero frequency called. Please report this bug\n");
            return None;
        }
        let dot = round(self.ppi / 80.);
        let mut p = Pattern::default();
        let d: &[i32] = match style {
            DASH_LINE => &[round(v), round(v)],
            DOTTED_LINE => &[dot, round(v)],
            DASH_DOT_LINE => {
                let d1 = round_mul(v, 0.5);
                &[round(v), d1, dot, d1]
            }
            DASH_2_DOTS_LINE => {
                let d1 = round_mul(v, 0.45);
                &[round(v), d1, dot, round_mul(v, 0.333), dot, d1]
            }
            DASH_3_DOTS_LINE => {
                let d1 = round_mul(v, 0.4);
                let d3 = round_mul(v, 0.3);
                &[round(v), d1, dot, d3, dot, d3, dot, d1]
            }
            // fig2dev goes on with an unset pattern
            _ => return None,
        };
        for (i, &x) in d.iter().enumerate() {
            if let Some(slot) = p.d.get_mut(i) {
                *slot = x;
            }
        }
        p.nd = d.len();
        p.length = d.iter().fold(0i32, |a, &x| a.wrapping_add(x));
        Some(p)
    }

    /// `put_points`: all points but the last after `cmd`, without the
    /// newline; returns the last point.
    fn put_points(&self, out: &mut Out, cmd: &str, pts: &[Point]) -> Point {
        out.puts(cmd);
        let mut chars = cmd.len();
        let Some((&last, init)) = pts.split_last() else {
            return Point::default();
        };
        for &p in init {
            if chars >= LINEBREAK {
                out.puts("\n");
                chars = 0;
            }
            chars += self.put_point(out, p);
        }
        last
    }

    /// `put_polyline`.
    fn put_polyline(&self, out: &mut Out, ctx: &mut Ctx, pts: &[Point]) {
        if pts.len() < 2 {
            ctx.warn("Not enough points in the line, strange...");
            return;
        }
        let last = self.put_points(out, "\\polyline", pts);
        fprintf!(out, "({},{})\n", self.xc(last.x), self.yc(last.y));
    }

    /// `put_pointarray`: points after `\polygon`, `\polygon*` or
    /// `\polyline`.
    fn put_pointarray(&self, out: &mut Out, pts: &[Point]) {
        let mut chars = 9;
        for &p in pts {
            if chars >= LINEBREAK {
                out.puts("\n");
                chars = 0;
            }
            chars += self.put_point(out, p);
        }
        out.puts("\n");
    }

    fn put_line_cmd(&self, out: &mut Out, a: Point, o: Point) {
        fprintf!(
            out,
            "\\Line({},{})({},{})\n",
            self.xc(a.x),
            self.yc(a.y),
            self.xc(o.x),
            self.yc(o.y)
        );
    }

    /// `put_patternline`: the line `pts` from point `start` with a dash
    /// pattern, beginning an offset `h1` into it; a negative `h1` is the
    /// offset into dash `dstart`.
    #[allow(clippy::too_many_arguments)]
    fn put_patternline(
        &self,
        out: &mut Out,
        ctx: &mut Ctx,
        pts: &[Point],
        start: usize,
        pattern: &Pattern,
        mut h1: f64,
        mut dstart: usize,
    ) {
        if pattern.length <= 0 || pattern.nd < 2 {
            // fig2dev loops forever on an empty pattern
            ctx.warn("Dash pattern of zero length; omitting line.");
            return;
        }
        let pt = |i: usize| pts.get(i).copied().unwrap_or_default();
        let n = pts.len();
        let mut k = start;
        loop {
            let mut pi = k;
            let mut qi = k + 1;
            if qi >= n {
                return;
            }
            // put a remaining dash
            if h1 < 0. {
                let mut len = f64::from(pattern.d(dstart)) + h1;
                let mut lenpq;
                loop {
                    lenpq = distance(pt(pi), pt(qi));
                    if lenpq > len {
                        break;
                    }
                    if qi + 1 >= n {
                        return;
                    }
                    len -= lenpq;
                    pi = qi;
                    qi += 1;
                    self.put_point(out, pt(pi));
                }
                let (p, q) = (pt(pi), pt(qi));
                let cosl = f64::from(q.x.wrapping_sub(p.x)) / lenpq;
                let sinl = f64::from(q.y.wrapping_sub(p.y)) / lenpq;
                fprintf!(
                    out,
                    "({},{})\n",
                    self.xc(p.x.wrapping_add(round_mul(cosl, len))),
                    self.yc(p.y.wrapping_add(round_mul(sinl, len)))
                );
                h1 = len + f64::from(pattern.d(dstart + 1));
                dstart = pattern.next(dstart);
            }
            // wind forward through the points within the offset
            let mut lenpq;
            loop {
                lenpq = distance(pt(pi), pt(qi));
                if lenpq > h1 {
                    break;
                }
                if qi + 1 >= n {
                    return;
                }
                h1 -= lenpq;
                pi = qi;
                qi += 1;
            }
            let (p, q) = (pt(pi), pt(qi));

            // the number of full dash patterns in this segment
            let numpatterns = ((lenpq - h1) / f64::from(pattern.length)).floor() as i32;
            let cosl = f64::from(q.x.wrapping_sub(p.x)) / lenpq;
            let sinl = f64::from(q.y.wrapping_sub(p.y)) / lenpq;
            let (mut dx, mut dy, mut precx, mut precy) = (0., 0., 0, 0);
            let mut slope = [(0i32, 0i32, 0usize, 0f64); PAT_ND_2];
            if numpatterns > 0 {
                dx = f64::from(pattern.length) * cosl;
                dy = f64::from(pattern.length) * sinl;
                // the precision for dx*numpatterns to be off by less than 0.5
                let prec = |len: f64| {
                    let mut prec = 0;
                    let mut digits = 1.;
                    while prec < 5
                        && (len - len.mul_add(digits, 0.5).floor() / digits).abs() >= 0.5 / f64::from(numpatterns)
                    {
                        prec += 1;
                        digits *= 10.;
                    }
                    prec
                };
                precx = prec(dx.abs());
                precy = prec(dy.abs());
                for (i, s) in slope.iter_mut().enumerate().take(pattern.nd / 2) {
                    let d = f64::from(pattern.d(2 * i));
                    let a = Point::new(round_mul(cosl, d), round_mul(sinl, d));
                    *s = get_lineslope(Point::default(), a);
                }
            }
            let pstart = Point::new(
                p.x.wrapping_add(round_mul(cosl, h1)),
                p.y.wrapping_add(round_mul(sinl, h1)),
            );
            // check whether single dashes fit into the remaining line
            let mut len = lenpq - f64::from(numpatterns.wrapping_mul(pattern.length)) - h1;
            let mut dadd = 0;
            let mut i = dstart;
            while len >= f64::from(pattern.d(i)) && dadd < MAX_DASHES {
                len -= f64::from(pattern.d(i).wrapping_add(pattern.d(i + 1)));
                i = pattern.next(i);
                dadd += 1;
            }

            let mut din: i32 = 0;
            let mut j = 1;
            let dash = |out: &mut Out, din: i32, i: usize| {
                let e = din.wrapping_add(pattern.d(i));
                fprintf!(
                    out,
                    "\\Line({},{})({},{})\n",
                    self.xc(pstart.x.wrapping_add(round_mul(f64::from(din), cosl))),
                    self.yc(pstart.y.wrapping_add(round_mul(f64::from(din), sinl))),
                    self.xc(pstart.x.wrapping_add(round_mul(f64::from(e), cosl))),
                    self.yc(pstart.y.wrapping_add(round_mul(f64::from(e), sinl)))
                );
            };
            let multi = |out: &mut Out, din: i32, i: usize, count: i32| {
                let (sx, sy, dpl, dlx) = slope.get(i / 2).copied().unwrap_or_default();
                fprintf!(
                    out,
                    "\\multiput({},{})({:.*},{:.*}){{{}}}{{\\line({},{}){{{:.*}}}}}\n",
                    self.xc(pstart.x.wrapping_add(round_mul(f64::from(din), cosl))),
                    self.yc(pstart.y.wrapping_add(round_mul(f64::from(din), sinl))),
                    precx,
                    dx,
                    precy,
                    -dy,
                    count,
                    sx,
                    sy.wrapping_neg(),
                    dpl,
                    dlx
                );
            };
            let mut i = dstart;
            if numpatterns == 0 {
                while j <= dadd {
                    dash(out, din, i);
                    din = din.wrapping_add(pattern.d(i).wrapping_add(pattern.d(i + 1)));
                    i = pattern.next(i);
                    j += 1;
                }
                dstart = i;
            } else {
                while j <= dadd {
                    multi(out, din, i, numpatterns.wrapping_add(1));
                    din = din.wrapping_add(pattern.d(i).wrapping_add(pattern.d(i + 1)));
                    i = pattern.next(i);
                    j += 1;
                }
                dstart = i;
                // the remaining dashes of the pattern, numpatterns times
                while j <= (pattern.nd / 2) as i32 {
                    if numpatterns == 1 {
                        dash(out, din, i);
                    } else {
                        multi(out, din, i, numpatterns);
                    }
                    din = din.wrapping_add(pattern.d(i).wrapping_add(pattern.d(i + 1)));
                    i = pattern.next(i);
                    j += 1;
                }
            }

            // len > 0: the first part of the last, incomplete dash; len <= 0:
            // -len is the offset for the next segment.
            let tail = Point::new(
                q.x.wrapping_sub(round_mul(cosl, len)),
                q.y.wrapping_sub(round_mul(sinl, len)),
            );
            if qi + 1 >= n {
                if len > 0. {
                    self.put_line_cmd(out, tail, q);
                }
                return;
            }
            if len > 0. {
                fprintf!(
                    out,
                    "\\polyline({},{})({},{})",
                    self.xc(tail.x),
                    self.yc(tail.y),
                    self.xc(q.x),
                    self.yc(q.y)
                );
            }
            k = qi;
            h1 = -len;
        }
    }

    /// `put_arrow`: the arrowhead from `p` to the tip `q`.
    fn put_arrow(&mut self, out: &mut Out, ctx: &mut Ctx, p: Point, q: Point, a: &Arrow, linethick: i32) {
        if self.is_ltx_arrow(a) {
            // the size of LaTeX vectors follows the arrow's thickness
            if a.thickness <= 0. {
                return;
            }
            self.set_linewidth(out, round(a.thickness));
            let (sx, sy, _, _) = get_vectorslope(p, q);
            fprintf!(
                out,
                "\\put({},{}){{\\vector({},{}){{0}}}}%\n",
                self.xc(q.x),
                self.yc(q.y),
                sx,
                sy.wrapping_neg()
            );
            return;
        }
        let ap = calc_arrow(ctx, p.x, p.y, q.x, q.y, linethick, a);
        let points = &ap.points;
        let mut npoints = points.len();
        if npoints < 2 {
            return;
        }
        // fill filled arrows
        if (a.style != 0 || !ap.fillpoints.is_empty()) && a.type_ < 13 {
            out.puts("\\polygon*");
            if ap.fillpoints.is_empty() {
                // the first and the last point coincide, except for a half
                // circle
                let n = if a.type_ == 6 { npoints } else { npoints - 1 };
                self.put_pointarray(out, points.get(..n).unwrap_or(points));
            } else {
                self.put_pointarray(out, &ap.fillpoints);
            }
        }
        if round_mul(0.5, a.thickness) == 0 {
            return;
        }
        self.set_linewidth(out, round(a.thickness));
        self.set_linejoin(out, ctx, MITERJOIN);
        self.set_linecap(out, ctx, BUTTCAP);
        if points.first() == points.last() {
            npoints -= 1;
            out.puts("\\polygon");
        } else {
            out.puts("\\polyline");
        }
        self.put_pointarray(out, points.get(..npoints).unwrap_or(points));
    }

    /// `put_oval`: a filled arc box (`c` = "fillpath"), or one stroked with
    /// a solid line ("strokepath"), between the diagonal points `p`, `q`.
    fn put_oval(&self, out: &mut Out, p: Point, q: Point, mut rad: i32, c: &str) {
        let dx = q.x.wrapping_sub(p.x);
        let dy = q.y.wrapping_sub(p.y);
        let m = dx.wrapping_abs().min(dy.wrapping_abs());
        if m < rad.wrapping_mul(2) {
            rad = m / 2;
        }
        if !self.epiccompatible && c.starts_with('s') {
            let cx = f64::from(p.x.wrapping_add(q.x)) / 2.;
            let cy = f64::from(p.y.wrapping_add(q.y)) / 2.;
            fprintf!(
                out,
                "\\put({:.*},{:.*}){{\\oval[{}]({},{})}}\n",
                usize::from(cx != cx.floor()),
                cx - f64::from(self.llx),
                usize::from(cy != cy.floor()),
                f64::from(self.ury) - cy,
                rad,
                dx.wrapping_abs(),
                dy.wrapping_abs()
            );
            return;
        }
        // epic compatible
        let dx: i32 = if dx > 0 { 1 } else { -1 };
        let dy: i32 = -(if dy > 0 { 1 } else { -1 });
        let dir = dx * dy;
        let x1 = self.xc(p.x).wrapping_add(dx.wrapping_mul(rad));
        let y1 = self.yc(p.y).wrapping_add(dy.wrapping_mul(rad));
        let x2 = self.xc(q.x).wrapping_sub(dx.wrapping_mul(rad));
        let y2 = self.yc(q.y).wrapping_sub(dy.wrapping_mul(rad));
        let mut a = if dx < 0 { 0 } else { 2 };
        fprintf!(
            out,
            "\\circlearc[1]{{{}}}{{{}}}{{{}}}{{{}}}{{{}}}",
            x1,
            y1,
            rad,
            a * 90,
            (a + dir) * 90
        );
        a += dir;
        fprintf!(
            out,
            "\\circlearc{{{}}}{{{}}}{{{}}}{{{}}}{{{}}}%\n",
            x2,
            y1,
            rad,
            a * 90,
            (a + dir) * 90
        );
        a += dir;
        fprintf!(
            out,
            "\\circlearc{{{}}}{{{}}}{{{}}}{{{}}}{{{}}}",
            x2,
            y2,
            rad,
            a * 90,
            (a + dir) * 90
        );
        a += dir;
        fprintf!(
            out,
            "\\circlearc{{{}}}{{{}}}{{{}}}{{{}}}{{{}}}\\closepath\\{}\n",
            x1,
            y2,
            rad,
            a * 90,
            (a + dir) * 90,
            c
        );
    }

    /// `ltx_offset`: the length of a LaTeX arrow of thickness `t` (pict2e
    /// manual: width 3.2 × line thickness + 1.5 pt, height 1.52 × width).
    fn ltx_offset(&self, t: f64) -> f64 {
        let t = thickness_d(t, self.thick_scale);
        1.52 * 3.2f64.mul_add(t, 105. * self.unitlength)
    }

    /// `get_offset`: how much shorter a line with arrowhead `a` is drawn.
    fn get_offset(&self, a: Option<&Arrow>, thickness: i32, capstyle: i32) -> f64 {
        let Some(a) = a else {
            return 0.;
        };
        let mut h = if self.is_ltx_arrow(a) {
            self.ltx_offset(a.thickness)
        } else {
            let indx = a.type_.wrapping_mul(2).wrapping_add(a.style);
            let (hfac, tfac) = usize::try_from(indx)
                .ok()
                .and_then(|i| PICT2EARROW.get(i))
                .copied()
                .unwrap_or((0., 0.));
            let mut h = arrow_shape(a).map_or(0., |s| s.tipmv);
            let thk = thickness_d(a.thickness, self.thick_scale);
            if h > 0.0 {
                h = thk * a.wid.mul_add(a.wid, h * h * a.ht * a.ht).sqrt() / 2.0 / a.wid;
            } else if h == 0.0 {
                // types which have a blunt end
                h = thk / 2.0;
            } else {
                // the tip is not moved
                h = 0.0;
            }
            // the additional line move
            h += a.ht.mul_add(hfac, thk * tfac);
            if tfac > 0.0 {
                h -= 0.5;
            } else if tfac < 0.0 {
                h += 0.5;
            }
            h
        };
        if capstyle == SQUARECAP {
            h = thickness_i(thickness, self.thick_scale).mul_add(0.5, h);
        }
        h
    }

    /// `put_vector`: a LaTeX vector from `p` to `q`, or a LaTeX arrowhead
    /// and a line. Returns `true` and the slope from `p` to `q` if an
    /// arrowhead is drawn.
    fn put_vector(
        &self,
        out: &mut Out,
        p: Point,
        q: Point,
        backarrow: Option<&Arrow>,
        thickness: i32,
        capstyle: i32,
    ) -> (bool, i32, i32) {
        let mut too_short = false;
        let mut a = p;
        if backarrow.is_some() {
            let h = self.get_offset(backarrow, thickness, capstyle);
            if h > 0.5 {
                match get_midpoint(p, q, h) {
                    Some(m) => a = m,
                    None => too_short = true,
                }
            }
        }
        if !too_short {
            let (sx, sy, lx, scaled) = get_vectorslope(a, q);
            if !scaled {
                fprintf!(
                    out,
                    "\\put({},{}){{\\vector({},{}){{{}}}}}\n",
                    self.xc(a.x),
                    self.yc(a.y),
                    sx,
                    sy.wrapping_neg(),
                    lx
                );
                return (false, sx, sy);
            }
            let h = self.ltx_offset(f64::from(thickness));
            if h != 0. {
                if let Some(o) = get_midpoint(q, a, h) {
                    self.put_line_cmd(out, a, o);
                }
            }
        }
        let (sx, sy, _, _) = get_vectorslope(p, q);
        self.put_vector_cmd(out, q, sx, sy);
        (true, sx, sy)
    }

    /// `PUTVECTOR(p, sx, sy, 0)`.
    fn put_vector_cmd(&self, out: &mut Out, p: Point, sx: i32, sy: i32) {
        fprintf!(
            out,
            "\\put({},{}){{\\vector({},{}){{0}}}}\n",
            self.xc(p.x),
            self.yc(p.y),
            sx,
            sy.wrapping_neg()
        );
    }

    /// `put_poly_or_patternline`: a polyline of more than two points or a
    /// non-solid one, shortened at its arrowheads, and the arrowheads.
    fn put_poly_or_patternline(&mut self, out: &mut Out, ctx: &mut Ctx, l: &Line) {
        let pts = &l.points;
        let (Some(&p), Some(&q)) = (pts.first(), pts.get(1)) else {
            return;
        };
        // the start point, if there is a line
        let h = if l.thickness > 0 && l.back_arrow.is_some() {
            self.get_offset(l.back_arrow.as_ref(), l.thickness, l.cap_style)
        } else {
            0.
        };
        let start = if l.thickness > 0 && l.back_arrow.is_some() && h > 0.5 {
            get_startpoint(pts, h)
        } else {
            Ok(pts.clone())
        };
        // the final two points, to draw the forward arrow
        let mut f = (p, q);
        match start {
            Err(last) => f = last,
            Ok(mut line) => {
                if l.for_arrow.is_some() {
                    let h = self.get_offset(l.for_arrow.as_ref(), l.thickness, l.cap_style);
                    if l.thickness == 0 || h <= 0.5 {
                        if let [.., f0, f1] = pts.as_slice() {
                            f = (*f0, *f1);
                        }
                    } else {
                        let (shorter, last) = insert_endpoint(&line, h);
                        line = shorter;
                        f = last;
                    }
                }
                // draw the line
                self.set_linewidth(out, l.thickness);
                self.set_linecap(out, ctx, l.cap_style);
                self.set_linejoin(out, ctx, l.join_style);
                if l.style == SOLID_LINE || l.style_val <= 0. {
                    self.put_polyline(out, ctx, &line);
                } else if let Some(pattern) = self.set_linestyle(out, l.style, l.style_val) {
                    self.put_patternline(out, ctx, &line, 0, &pattern, 0., 0);
                }
            }
        }
        if let Some(fa) = &l.for_arrow {
            if self.verbose {
                out.puts("% Forward Arrow\n");
            }
            self.put_arrow(out, ctx, f.0, f.1, fa, l.thickness);
        }
        if let Some(ba) = &l.back_arrow {
            if self.verbose {
                out.puts("% Backward Arrow\n");
            }
            self.put_arrow(out, ctx, q, p, ba, l.thickness);
        }
    }

    /// `put_line_or_vector`: a solid two-point line, as a `\Line`, or as a
    /// LaTeX vector.
    fn put_line_or_vector(&mut self, out: &mut Out, ctx: &mut Ctx, l: &Line) {
        let (Some(&p), Some(&q)) = (l.points.first(), l.points.get(1)) else {
            return;
        };
        let fa = l.for_arrow.as_ref();
        let ba = l.back_arrow.as_ref();
        if fa.is_some_and(|f| self.is_ltx_arrow(f) && l.thickness == round(f.thickness)) {
            // At least an arrow is drawn. The line cap might be set uselessly.
            self.set_linewidth(out, l.thickness);
            self.set_linecap(out, ctx, l.cap_style);
            let (drawn, sx, sy) = self.put_vector(out, p, q, ba, l.thickness, l.cap_style);
            match ba {
                Some(b) if drawn && self.is_ltx_arrow(b) && b.thickness > 0.5 => {
                    self.set_linewidth(out, round(b.thickness));
                    self.put_vector_cmd(out, p, sx.wrapping_neg(), sy.wrapping_neg());
                }
                Some(b) => self.put_arrow(out, ctx, q, p, b, l.thickness),
                None => {}
            }
        } else if ba.is_some_and(|b| self.is_ltx_arrow(b) && l.thickness == round(b.thickness)) {
            self.set_linewidth(out, l.thickness);
            self.set_linecap(out, ctx, l.cap_style);
            self.put_vector(out, q, p, fa, l.thickness, l.cap_style);
            if let Some(f) = fa {
                self.put_arrow(out, ctx, p, q, f, l.thickness);
            }
        } else {
            let mut too_short = false;
            let mut a = p;
            if ba.is_some() {
                let h = self.get_offset(ba, l.thickness, l.cap_style);
                if h > 0.5 {
                    match get_midpoint(p, q, h) {
                        Some(m) => a = m,
                        None => too_short = true,
                    }
                }
            }
            let mut o = q;
            if !too_short && fa.is_some() {
                let h = self.get_offset(fa, l.thickness, l.cap_style);
                if h > 0.5 {
                    match get_midpoint(q, a, h) {
                        Some(m) => o = m,
                        None => too_short = true,
                    }
                }
            }
            if !too_short {
                self.set_linewidth(out, l.thickness);
                self.set_linecap(out, ctx, l.cap_style);
                self.put_line_cmd(out, a, o);
            }
            if let Some(f) = fa {
                self.put_arrow(out, ctx, p, q, f, l.thickness);
            }
            if let Some(b) = ba {
                self.put_arrow(out, ctx, q, p, b, l.thickness);
            }
        }
    }

    /// `put_picture`: an included graphics file in the box with corners
    /// `p`, `q`, `r`, `s` (in the file's order).
    fn put_picture(&self, out: &mut Out, p: Point, q: Point, r: Point, s: Point, l: &Line) {
        let Some(pic) = &l.pic else {
            return;
        };
        let file = pic.file.as_bytes();
        let n = if self.removesuffix {
            file.iter().rposition(|&c| c == b'.').unwrap_or(file.len())
        } else {
            file.len()
        };
        let file = file.get(..n).unwrap_or(file);
        let mut dx = r.x.wrapping_sub(p.x);
        let mut dy = r.y.wrapping_sub(p.y);
        // the rotation; dx and dy become the width and the height
        let mut rot = 0;
        let mut ll = s;
        if dx < 0 && dy < 0 {
            rot = 180;
            ll = s;
            dx = dx.wrapping_neg();
            dy = dy.wrapping_neg();
        } else if dx < 0 && dy >= 0 {
            let t = dy;
            dy = dx.wrapping_neg();
            dx = t;
            rot = 270;
            ll = q;
        } else if dy < 0 && dx >= 0 {
            let t = dx;
            dx = dy.wrapping_neg();
            dy = t;
            rot = 90;
            ll = q;
        }
        let prepend = self.prepend.as_deref().unwrap_or("");
        if pic.flipped == 0 {
            if rot == 0 {
                fprintf!(
                    out,
                    "\\put({},{}){{\\resizebox{{{}\\unitlength}}{{{}\\unitlength}}{{\\includegraphics{{{}",
                    self.xc(ll.x),
                    self.yc(ll.y),
                    dx,
                    dy,
                    prepend
                );
            } else {
                fprintf!(
                    out,
                    "\\put({},{}){{\\rotatebox{{{}}}{{\\smash{{\\rlap{{\\resizebox*{{{}\\unitlength}}{{{}\\unitlength}}{{\\includegraphics{{{}",
                    self.xc(ll.x),
                    self.yc(ll.y),
                    rot,
                    dx,
                    dy,
                    prepend
                );
            }
            out.bytes(file);
            out.puts(if rot == 0 { "}}}\n" } else { "}}}}}}\n" });
            return;
        }
        // flipped
        rot += 90;
        if rot == 360 {
            fprintf!(
                out,
                "\\put({},{}){{\\resizebox*{{-{}\\unitlength}}{{{}\\unitlength}}{{\\includegraphics{{{}",
                self.xc(r.x),
                self.yc(r.y),
                dy,
                dx,
                prepend
            );
            out.bytes(file);
            out.puts("}}}\n");
        } else {
            fprintf!(
                out,
                "\\put({},{}){{\\rotatebox{{{}}}{{\\smash{{\\rlap{{\\resizebox*{{-{}\\unitlength}}{{{}\\unitlength}}{{\\includegraphics{{{}",
                self.xc(r.x),
                self.yc(r.y),
                rot,
                dy,
                dx,
                prepend
            );
            out.bytes(file);
            out.puts("}}}}}}\n");
        }
    }

    /// `genpict2e_line`.
    fn put_line(&mut self, out: &mut Out, ctx: &mut Ctx, l: &Line) {
        if self.verbose {
            out.puts("%\n% Fig POLYLINE object\n%\n");
        }
        print_comments(out, "% ", &l.comments, "");
        let pts = &l.points;
        let Some(&p0) = pts.first() else {
            return;
        };
        // the number of points, at most five
        let npts = pts.len().min(5);
        let pt = |i: usize| pts.get(i).copied().unwrap_or_default();

        if l.type_ == T_PIC_BOX {
            if npts < 4 {
                ctx.warn("A picture box with less than four unique points. Omitting.");
            } else {
                self.put_picture(out, p0, pt(1), pt(2), pt(3), l);
            }
            return;
        }

        // fill
        if l.fill_style != UNFILLED && npts > 2 {
            if self.verbose {
                out.puts("% Fill\n");
            }
            self.set_fillcolor(out, ctx, l.fill_color, l.fill_style, l.pen_color);
            if l.type_ == T_POLYLINE {
                let last = self.put_points(out, "\\polygon*", pts);
                if p0 != last {
                    self.put_point(out, last);
                }
                out.puts("\n");
            } else if l.type_ == T_POLYGON || l.type_ == T_BOX {
                self.put_points(out, "\\polygon*", pts);
                out.puts("\n");
            } else if l.type_ == T_ARC_BOX {
                self.put_oval(out, p0, pt(2), l.radius, "fillpath");
            }
        }

        if l.thickness == 0 && l.for_arrow.is_none() && l.back_arrow.is_none() {
            return;
        }

        // a single point
        if npts == 1 {
            if l.thickness == 0 {
                return;
            }
            self.set_color(out, ctx, l.pen_color);
            self.set_linewidth(out, l.thickness);
            let (ox, oy) = (self.xc(p0.x), self.yc(p0.y));
            if l.cap_style == ROUNDCAP {
                self.set_linecap(out, ctx, ROUNDCAP);
                fprintf!(out, "\\Line({},{})({},{})%\n", ox, oy, ox, oy);
            } else {
                self.set_linecap(out, ctx, BUTTCAP);
                let h1 = if l.thickness as f32 <= self.thick_scale {
                    f64::from(l.thickness) / 4.0
                } else {
                    f64::from(l.thickness as f32 - self.thick_scale) / 2.0
                };
                fprintf!(
                    out,
                    "\\Line({},{})({},{})%\n",
                    round(f64::from(ox) - h1),
                    oy,
                    round(f64::from(ox) + h1),
                    oy
                );
            }
            return;
        }

        // two or more points; the colour might be set uselessly for a too
        // short line with arrowheads and zero line thickness
        self.set_color(out, ctx, l.pen_color);

        if l.type_ == T_POLYGON || l.type_ == T_BOX {
            self.set_linewidth(out, l.thickness);
            if l.style == SOLID_LINE || l.style_val <= 0. {
                self.set_linejoin(out, ctx, l.join_style);
                self.put_points(out, "\\polygon", pts);
                out.puts("\n");
            } else {
                let pattern = self.set_linestyle(out, l.style, l.style_val);
                self.set_linecap(out, ctx, l.cap_style);
                self.set_linejoin(out, ctx, l.join_style);
                if let Some(pattern) = pattern {
                    self.put_patternline(out, ctx, pts, 0, &pattern, 0., 0);
                }
            }
            return;
        }

        if l.type_ == T_ARC_BOX {
            if npts < 3 || p0 == pt(2) {
                return;
            }
            if l.style == SOLID_LINE {
                self.set_linewidth(out, l.thickness);
                self.put_oval(out, p0, pt(2), l.radius, "strokepath");
            } else {
                self.put_patternoval(out, ctx, p0, pt(2), l);
            }
            return;
        }

        if l.type_ != T_POLYLINE {
            ctx.warn(format!("Unknown line style {}. Please report this bug.", l.type_));
            return;
        }
        if npts > 2 || l.style != SOLID_LINE {
            self.put_poly_or_patternline(out, ctx, l);
        } else {
            // two points, solid line
            self.put_line_or_vector(out, ctx, l);
        }
    }

    /// The closed X-spline through `points` as a polyline, with the
    /// attributes of a non-solid object.
    #[allow(clippy::too_many_arguments)]
    fn octagon_line(
        ctx: &mut Ctx,
        points: Vec<Point>,
        controls: Vec<Control>,
        style: i32,
        thickness: i32,
        pen_color: i32,
        depth: i32,
        style_val: f64,
        cap_style: i32,
        arrows: (Option<Arrow>, Option<Arrow>),
    ) -> Option<Line> {
        let s = Spline {
            type_: T_CLOSED_XSPLINE,
            style,
            thickness,
            pen_color,
            depth,
            fill_style: UNFILLED,
            style_val,
            for_arrow: arrows.0,
            back_arrow: arrows.1,
            cap_style,
            points,
            controls,
            ..Spline::default()
        };
        create_line_with_spline(ctx, &s)
    }

    /// `put_patternellipse`: a non-solid ellipse, as a polyline that
    /// approximates an X-spline through an octagon.
    fn put_patternellipse(&mut self, out: &mut Out, ctx: &mut Ctx, e: &Ellipse) {
        // The stretch for the polyline to oscillate around the circle.
        let (exi, controls) = get_xsplinepoints(1.0258210);
        let angle = -reduce_angle(e.angle);
        let (cosa, sina) = (angle.cos(), angle.sin());
        let (rx, ry) = (f64::from(e.radiuses.x), f64::from(e.radiuses.y));
        // scale, rotate and move the unit circle
        let points = exi
            .iter()
            .map(|&(x, y)| {
                Point::new(
                    e.center.x.wrapping_add(round((cosa * rx).mul_add(x, -(sina * ry * y)))),
                    e.center.y.wrapping_add(round((sina * rx).mul_add(x, cosa * ry * y))),
                )
            })
            .collect();
        let Some(mut l) = Self::octagon_line(
            ctx,
            points,
            controls,
            e.style,
            e.thickness,
            e.pen_color,
            e.depth,
            e.style_val,
            BUTTCAP,
            (None, None),
        ) else {
            return;
        };
        l.join_style = 1; // round join
        let verbose = std::mem::replace(&mut self.verbose, false);
        self.put_line(out, ctx, &l);
        self.verbose = verbose;
    }

    /// `put_patternoval`: a non-solid arc box, as a polyline approximating
    /// a circle whose four quadrants are moved into the box's corners.
    fn put_patternoval(&mut self, out: &mut Out, ctx: &mut Ctx, p: Point, q: Point, l: &Line) {
        let dx = q.x.wrapping_sub(p.x);
        let dy = q.y.wrapping_sub(p.y);
        let mut rad = l.radius;
        let m = dx.wrapping_abs().min(dy.wrapping_abs());
        if m < rad.wrapping_mul(2) {
            rad = m / 2;
        }
        let dx: i32 = if dx > 0 { 1 } else { -1 };
        let dy: i32 = if dy > 0 { 1 } else { -1 };
        let x1 = p.x.wrapping_add(dx.wrapping_mul(rad));
        let y1 = p.y.wrapping_add(dy.wrapping_mul(rad));
        let x2 = q.x.wrapping_sub(dx.wrapping_mul(rad));
        let y2 = q.y.wrapping_sub(dy.wrapping_mul(rad));

        let (u, controls) = get_xsplinepoints(1.0278126);
        let r = f64::from(rad);
        let octagon = u
            .iter()
            .map(|&(x, y)| Point::new(round_mul(r, x), round_mul(r, y)))
            .collect();
        let Some(mut b) = Self::octagon_line(
            ctx,
            octagon,
            controls,
            l.style,
            l.thickness,
            l.pen_color,
            l.depth,
            l.style_val,
            l.cap_style,
            (None, None),
        ) else {
            return;
        };
        b.join_style = l.join_style;
        if b.points.is_empty() {
            return;
        }
        // The octagon is drawn in TeX's coordinate direction, hence -dy.
        // Points on the axes are doubled, such that the lines connecting
        // the quadrants are exactly horizontal or vertical.
        let src = std::mem::take(&mut b.points);
        let mut pts = Vec::with_capacity(src.len() + 4);
        let mv = |p: Point, x: i32, y: i32| {
            Point::new(
                x.wrapping_add(dx.wrapping_mul(p.x)),
                y.wrapping_sub(dy.wrapping_mul(p.y)),
            )
        };
        let mut k = 0;
        let at = |k: usize| src.get(k).copied();
        while let Some(p) = at(k).filter(|p| p.y > 0) {
            pts.push(mv(p, x2, y1));
            k += 1;
        }
        let corners = [
            ((x2, y1), (x2, y2)),
            ((x2, y2), (x1, y2)),
            ((x1, y2), (x1, y1)),
            ((x1, y1), (x2, y1)),
        ];
        for (side, &(c1, c2)) in corners.iter().enumerate() {
            let on_axis = |p: Point| if side % 2 == 0 { p.y == 0 } else { p.x == 0 };
            if let Some(p) = at(k).filter(|&p| on_axis(p)) {
                pts.push(mv(p, c1.0, c1.1));
                pts.push(mv(p, c2.0, c2.1));
                k += 1;
            }
            let inside = |p: Point| match side {
                0 => p.x > 0,
                1 => p.y < 0,
                2 => p.x < 0,
                _ => false,
            };
            if side < 3 {
                while let Some(p) = at(k).filter(|&p| inside(p)) {
                    pts.push(mv(p, c2.0, c2.1));
                    k += 1;
                }
            }
        }
        while let Some(p) = at(k) {
            pts.push(mv(p, x2, y1));
            k += 1;
        }
        b.points = pts;
        let verbose = std::mem::replace(&mut self.verbose, false);
        self.put_line(out, ctx, &b);
        self.verbose = verbose;
    }

    /// `put_ellipse`: the path of an ellipse, four cubic Bézier curves;
    /// `\fillpath` or `\strokepath` follows.
    fn put_ellipse(&self, out: &mut Out, e: &Ellipse) {
        // On the unit circle, the control points lie at (1, MAGIC) or
        // (MAGIC, 1); the curves then fit the circle every 30 degrees
        // (http://www.tinaja.com/glib/ellipse4.pdf).
        const MAGIC: f64 = 0.551784;
        // Fig and pict2e's y differ: start at the top, going clockwise, which
        // ends up starting at the bottom, counter-clockwise.
        const EXI: [(f64, f64); 13] = [
            (0., -1.),
            (MAGIC, -1.),
            (1., -MAGIC),
            (1., 0.),
            (1., MAGIC),
            (MAGIC, 1.),
            (0., 1.),
            (-MAGIC, 1.),
            (-1., MAGIC),
            (-1., 0.),
            (-1., -MAGIC),
            (-MAGIC, -1.),
            (0., -1.),
        ];
        // correct for the coordinate transform
        let angle = -reduce_angle(e.angle);
        let (cosa, sina) = (angle.cos(), angle.sin());
        let (cx, cy) = (f64::from(e.center.x), f64::from(e.center.y));
        let (rx, ry) = (f64::from(e.radiuses.x), f64::from(e.radiuses.y));
        let ey0 = EXI[0].1;
        fprintf!(
            out,
            "\\moveto({},{})",
            self.xc(round((-(sina * ry)).mul_add(ey0, cx))),
            self.yc(round((cosa * ry).mul_add(ey0, cy)))
        );
        for (n, chunk) in EXI.get(1..).unwrap_or(&[]).chunks(3).enumerate() {
            out.puts("\\curveto");
            for &(x, y) in chunk {
                let px = (-(sina * ry)).mul_add(y, (cosa * rx).mul_add(x, cx));
                let py = (cosa * ry).mul_add(y, (sina * rx).mul_add(x, cy));
                fprintf!(out, "({},{})", self.xc(round(px)), self.yc(round(py)));
            }
            if n == 1 {
                out.puts("\n");
            }
        }
        out.puts("\\closepath");
    }

    /// `put_patternarc`: a non-solid arc, as a polyline that approximates
    /// a circle, cut at the arc's angle `da` (radians, positive), rotated
    /// to start at `angle1` and moved to the centre `c`.
    #[allow(clippy::too_many_arguments)]
    fn put_patternarc(&mut self, out: &mut Out, ctx: &mut Ctx, c: Point, rad: f64, da: f64, angle1: f64, a: &Arc) {
        let (u, controls) = get_xsplinepoints(1.025821);
        let o: Vec<Point> = u
            .iter()
            .map(|&(x, y)| Point::new(round_mul(rad, x), round_mul(rad, y)))
            .collect();
        let o4 = o.get(4).copied().unwrap_or_default();
        let Some(mut l) = Self::octagon_line(
            ctx,
            o,
            controls,
            a.style,
            a.thickness,
            a.pen_color,
            a.depth,
            a.style_val,
            a.cap_style,
            (a.for_arrow, a.back_arrow),
        ) else {
            return;
        };
        l.join_style = MITERJOIN;
        let pts = std::mem::take(&mut l.points);
        let Some(&first) = pts.first() else {
            return;
        };
        // Walk along the spline until the arc's angle is covered. The
        // octagon runs clockwise; a1 is about 3π/8.
        let a1 = f64::from(first.y).atan2(f64::from(first.x));
        let a2 = a1 - da;
        let (sinr, cosr) = (a2.sin(), a2.cos());
        let sina2 = round_mul(rad, sinr);
        let cosa2 = round_mul(rad, cosr);
        // Compare on the short cathetus: the distance of the spline's
        // points wobbles around the radius.
        let mut k = 0;
        let mut prev: Option<usize> = None;
        let mut wind = |cond: &dyn Fn(Point) -> bool| {
            while pts.get(k).is_some_and(|&p| cond(p)) {
                prev = Some(k);
                k += 1;
            }
        };
        if a2 < M_PI_4 {
            wind(&|p| p.y > p.x);
            if a2 < -M_PI_4 {
                wind(&|p| p.x > p.y.wrapping_neg());
                if a2 < -3. * M_PI_4 {
                    wind(&|p| p.y < p.x);
                    if a2 < -5. * M_PI_4 {
                        wind(&|p| p.x < p.y.wrapping_neg());
                        wind(&|p| p.x < cosa2);
                    } else {
                        wind(&|p| p.y < sina2);
                    }
                } else {
                    wind(&|p| p.x > cosa2);
                }
            } else {
                wind(&|p| p.y > sina2);
            }
        } else {
            wind(&|p| p.x < cosa2);
        }

        // The polyline wobbles around the radius: take the current one for
        // the point on the arc at angle a2.
        let last = prev.and_then(|j| pts.get(j)).copied().unwrap_or(o4);
        let rot = hypot_fma(f64::from(last.x), f64::from(last.y));
        let end = Point::new(round_mul(rot, cosr), round_mul(rot, sinr));
        let mut arc: Vec<Point> = match prev {
            Some(j) => {
                let mut v = pts.get(..=j).unwrap_or(&pts).to_vec();
                v.push(end);
                v
            }
            None => pts.clone(),
        };

        // rotate and move the arc into place (direction 0 clockwise, 1
        // counter-clockwise)
        let (i, rot) = if a.direction != 0 {
            (-1, -(angle1 - PI + a1))
        } else {
            (1, -(angle1 - a1))
        };
        let (cosr, sinr) = (rot.cos(), rot.sin());
        for p in &mut arc {
            let px = f64::from(i * p.x);
            let py = f64::from(p.y);
            *p = Point::new(
                c.x.wrapping_add(round(cosr.mul_add(px, sinr * py))),
                c.y.wrapping_add(round(sinr.mul_add(px, -(cosr * py)))),
            );
        }

        if a.type_ == T_PIE_WEDGE_ARC {
            // xfig closes polygons
            l.type_ = T_POLYGON;
            let mut v = Vec::with_capacity(arc.len() + 2);
            v.push(c);
            v.extend_from_slice(&arc);
            if prev.is_some() {
                v.push(c);
            }
            l.points = v;
        } else {
            l.type_ = T_POLYLINE;
            l.points = arc;
        }
        self.put_line(out, ctx, &l);
    }
}

/// The angle reduced to −π/2 … π/2 by steps of π, as fig2dev does.
fn reduce_angle(mut angle: f64) -> f64 {
    let mut steps = 0;
    while angle > M_PI_2 && steps < MAX_ANGLE_STEPS {
        angle -= PI;
        steps += 1;
    }
    while angle < -M_PI_2 && steps < MAX_ANGLE_STEPS {
        angle += PI;
        steps += 1;
    }
    angle
}

impl Driver for Pict2e {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        let a = arg.unwrap_or("");
        match opt {
            // border margin around the figure
            'b' => self.border_margin = atoi(a),
            // default colour
            'C' => self.default_color = atoi(a),
            'e' => self.epiccompatible = false,
            'F' => self.select_fontname = false,
            // prepend this string to included graphics files
            'i' => self.prepend = Some(a.to_string()),
            // the default text font
            'f' => {
                let tex = (1..=MAX_FONT as usize).find(|&i| {
                    self.fonts.names.get(i).is_some_and(|n| n == a) || self.fonts.fonts.get(i).is_some_and(|n| n == a)
                });
                if let Some(i) = tex {
                    if let Some(v) = self.fonts.fonts.get(i).cloned() {
                        self.fonts.fonts[0] = v;
                    }
                } else if PS_FONT_NAMES
                    .get(1..=MAX_PSFONT as usize)
                    .is_some_and(|names| names.contains(&a))
                {
                    self.fonts.psfonts[0] = a.to_string().into();
                } else {
                    ctx.warn(format!("warning: non-standard font name {a} ignored"));
                }
            }
            'o' => self.select_fontsize = false,
            'T' => self.only_texfonts = true,
            'O' => self.allspecial = true,
            'P' => {
                self.pagemode = true;
                self.epiccompatible = false;
            }
            'R' => {
                self.ltxarrow = atoi(a);
                // types 0, 2, 3, etc. exist
                if self.ltxarrow <= 1 {
                    self.ltxarrow -= 1;
                }
            }
            'r' => self.allarrowsltx = true,
            'v' => self.verbose = true,
            // remove the suffix from included graphics files
            'w' => self.removesuffix = true,
            'G' | 'L' => {}
            _ => return Err(format!("Argument -{opt} unknown to pict2e driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        set_default_size(&mut self.fonts, ctx);
        self.unitlength = ctx.mag / ctx.ppi;
        self.border_margin = (f64::from(self.border_margin) / (self.unitlength * 72.0)) as i32;
        let bm = self.border_margin;
        ctx.llx = ctx.llx.wrapping_sub(bm);
        ctx.lly = ctx.lly.wrapping_sub(bm);
        ctx.urx = ctx.urx.wrapping_add(bm);
        ctx.ury = ctx.ury.wrapping_add(bm);
        self.llx = ctx.llx;
        self.ury = ctx.ury;
        self.thick_scale = ctx.thick_scale;
        self.ppi = ctx.ppi;
        self.enc = text_encoding(ctx);

        if !objects.comments.is_empty() {
            out.puts("%\n");
            print_comments(out, "% ", &objects.comments, "");
            out.puts("%\n");
        }
        if self.pagemode {
            let mut width = f64::from(ctx.urx.wrapping_sub(ctx.llx)) * self.unitlength;
            let mut height = f64::from(ctx.ury.wrapping_sub(ctx.lly)) * self.unitlength;
            let mut unit = "in";
            if ctx.metric {
                width *= 2.54;
                height *= 2.54;
                unit = "cm";
            }
            out.puts("\\documentclass{minimal}\n");
            fprintf!(
                out,
                "\\usepackage[papersize={{{}{unit},{}{unit}}},margin=0pt]{{geometry}}\n",
                fmt_g(width, 4),
                fmt_g(height, 4)
            );
            // color.sty always: shaded black or tinted white needs it
            out.puts("\\usepackage{pict2e,graphics,color}\n");
            out.puts("\\parindent0pt\n\\begin{document}\n");
        }
        fprintf!(
            out,
            "\\unitlength{}sp",
            4736286.72f64.mul_add(self.unitlength, 0.5) as i64
        );
        if ctx.metric {
            out.puts("% 4143.7 sp = (1/472.44) cm\n");
        } else {
            out.puts("% 3946.9 sp = (1/1200) in\n");
        }
        fprintf!(
            out,
            "\\begin{{picture}}({},{})%(0,0)\n",
            ctx.urx.wrapping_sub(ctx.llx),
            ctx.ury.wrapping_sub(ctx.lly)
        );
        if self.epiccompatible {
            // \allinethickness is defined in eepic.sty, not in epic.sty.
            out.puts(
                "\\ifx\\allinethickness\\undefined\n  \\def\\XFigeepicthickness#1{\\relax}\n\\else\n  \\let\\XFigeepicthickness\\allinethickness\n\\fi\n",
            );
        }
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        if self.verbose {
            out.puts("%\n% Fig ARC object\n%\n");
        }
        print_comments(out, "% ", &a.comments, "");
        if a.fill_style == UNFILLED && a.thickness == 0 && a.for_arrow.is_none() && a.back_arrow.is_none() {
            return Ok(());
        }
        let c = Point::new(round(a.center.x), round(a.center.y));
        // the angles in pict2e's direction, in degrees
        let d1x = a.point[0].x.wrapping_sub(c.x);
        let d1y = a.point[0].y.wrapping_sub(c.y);
        let d2x = a.point[2].x.wrapping_sub(c.x);
        let d2y = a.point[2].y.wrapping_sub(c.y);
        let mut angle1 = f64::from(d1y.wrapping_neg()).atan2(f64::from(d1x)) * 180. / PI;
        let mut angle2 = f64::from(d2y.wrapping_neg()).atan2(f64::from(d2x)) * 180. / PI;
        // The radius is the mean of the two radii, which can not differ by
        // more than one pixel, rounded to tenths.
        let rad = 0.5 * (hypot_fma(f64::from(d1x), f64::from(d1y)) + hypot_fma(f64::from(d2x), f64::from(d2y)));
        let rad = f64::from(round_mul(rad, 10.0)) / 10.0;
        // the precision of the angles: 1/rad is the view angle of a pixel
        let mut da = 180.0 / PI / rad;
        let mut preca = 0usize;
        let mut d = 1.0;
        while da < 1.0 / d {
            preca += 1;
            d *= 10.;
        }
        if a.direction != 0 {
            // counter-clockwise
            if angle1 > angle2 {
                angle2 += 360.;
            }
            da = angle2 - angle1;
        } else {
            if angle1 < angle2 {
                angle1 += 360.;
            }
            da = angle1 - angle2;
        }
        let mut steps = 0;
        while da > 360. && steps < MAX_ANGLE_STEPS {
            da -= 360.;
            steps += 1;
        }
        let rads = fmt_g(rad, 6);

        // fill the arc
        if a.fill_style != UNFILLED && da > 0. {
            self.set_fillcolor(out, ctx, a.fill_color, a.fill_style, a.pen_color);
            fprintf!(
                out,
                "\\circlearc[1]{{{}}}{{{}}}{{{}}}{{{:.*}}}{{{:.*}}}",
                self.xc(c.x),
                self.yc(c.y),
                rads,
                preca,
                Cf(angle1),
                preca,
                Cf(angle2)
            );
            if a.type_ == T_PIE_WEDGE_ARC {
                fprintf!(
                    out,
                    "\\lineto({},{})\\closepath\\fillpath\n",
                    self.xc(c.x),
                    self.yc(c.y)
                );
            } else {
                if a.type_ != T_OPEN_ARC {
                    ctx.warn("Unknown arc type - please report this bug.");
                }
                out.puts("\\closepath\\fillpath\n");
            }
        }

        if a.style != SOLID_LINE && a.thickness != 0 {
            self.put_patternarc(out, ctx, c, rad, da * PI / 180., angle1 * PI / 180., a);
            return Ok(());
        }

        self.set_color(out, ctx, a.pen_color);

        // the arrowheads, and the new start and end angles
        let (mut offa1, mut offa2) = (0., 0.);
        if let Some(ba) = &a.back_arrow {
            let p = a.point[0];
            let q = compute_arcarrow_angle(
                ctx,
                f64::from(c.x),
                f64::from(c.y),
                f64::from(p.x),
                f64::from(p.y),
                i32::from(a.direction == 0),
                ba,
            );
            self.put_arrow(out, ctx, Point::new(q.0, q.1), p, ba, a.thickness);
            let d = self.get_offset(Some(ba), a.thickness, a.cap_style);
            if d != 0. {
                offa1 = (d / rad).atan() * 180. / PI;
                angle1 += if a.direction != 0 { offa1 } else { -offa1 };
                if offa1 >= da {
                    return Ok(());
                }
            }
        }
        if let Some(fa) = &a.for_arrow {
            let p = a.point[2];
            let q = compute_arcarrow_angle(
                ctx,
                f64::from(c.x),
                f64::from(c.y),
                f64::from(p.x),
                f64::from(p.y),
                a.direction,
                fa,
            );
            self.put_arrow(out, ctx, Point::new(q.0, q.1), p, fa, a.thickness);
            let d = self.get_offset(Some(fa), a.thickness, a.cap_style);
            if d != 0. {
                offa2 = (d / rad).atan() * 180. / PI;
                angle2 -= if a.direction != 0 { offa2 } else { -offa2 };
                if offa2 >= da {
                    return Ok(());
                }
            }
        }
        if (a.for_arrow.is_some() || a.back_arrow.is_some()) && offa1 + offa2 >= da {
            return Ok(());
        }
        if a.thickness == 0 {
            return Ok(());
        }
        self.set_linewidth(out, a.thickness);
        // Arcs have no join style, though pie-wedge arcs should have one.
        fprintf!(
            out,
            "\\circlearc[1]{{{}}}{{{}}}{{{}}}{{{:.*}}}{{{:.*}}}",
            self.xc(c.x),
            self.yc(c.y),
            rads,
            preca,
            Cf(angle1),
            preca,
            Cf(angle2)
        );
        match a.type_ {
            T_PIE_WEDGE_ARC => fprintf!(
                out,
                "\\lineto({},{})\\closepath\\strokepath\n",
                self.xc(c.x),
                self.yc(c.y)
            ),
            T_OPEN_ARC => out.puts("\\strokepath\n"),
            _ => {
                out.puts("\\strokepath\n");
                ctx.warn("Unknown arc type - please report this bug.");
            }
        }
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        // For compatibility with eepic (\usepackage{pict2e,epic,eepic}),
        // avoid \circle*: pict2e ignores the line thickness and just fills
        // the circle, eepic fills it and strokes the outline. XeTeX seems
        // to have problems setting the colour of the eepic macros, so avoid
        // \circle too.
        if self.verbose {
            out.puts("%\n% Fig ELLIPSE object\n%\n");
        }
        print_comments(out, "% ", &e.comments, "");
        let circle = e.radiuses.x == e.radiuses.y;
        let (cx, cy) = (self.xc(e.center.x), self.yc(e.center.y));
        if e.fill_style != UNFILLED {
            self.set_fillcolor(out, ctx, e.fill_color, e.fill_style, e.pen_color);
            if circle {
                if self.epiccompatible {
                    fprintf!(
                        out,
                        "\\circlearc[1]{{{}}}{{{}}}{{{}}}{{0}}{{360}}\\closepath\\fillpath\n",
                        cx,
                        cy,
                        e.radiuses.x
                    );
                } else {
                    fprintf!(
                        out,
                        "\\put({},{}){{\\circle*{{{}}}}}\n",
                        cx,
                        cy,
                        e.radiuses.x.wrapping_mul(2)
                    );
                }
            } else {
                self.put_ellipse(out, e);
                out.puts("\\fillpath\n");
            }
        }
        if e.thickness == 0 {
            return Ok(());
        }
        self.set_color(out, ctx, e.pen_color);
        self.set_linewidth(out, e.thickness);
        if e.style == SOLID_LINE {
            if circle {
                if self.epiccompatible {
                    fprintf!(
                        out,
                        "\\circlearc[1]{{{}}}{{{}}}{{{}}}{{0}}{{360}}\\closepath\\strokepath\n",
                        cx,
                        cy,
                        e.radiuses.x
                    );
                } else {
                    fprintf!(
                        out,
                        "\\put({},{}){{\\circle{{{}}}}}\n",
                        cx,
                        cy,
                        e.radiuses.x.wrapping_mul(2)
                    );
                }
            } else {
                self.put_ellipse(out, e);
                out.puts("\\strokepath\n");
            }
        } else {
            self.put_patternellipse(out, ctx, e);
        }
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        self.put_line(out, ctx, l);
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        print_comments(out, "% ", &s.comments, "");
        ctx.warn("Can't generate spline; omitting object");
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String> {
        if self.verbose {
            out.puts("%\n% Fig TEXT object\n%\n");
        }
        print_comments(out, "% ", &t.comments, "");
        let x = self.xc(t.base_x);
        let y = self.yc(t.base_y);
        let tpos = match t.type_ {
            T_LEFT_JUSTIFIED | DEFAULT => "[lb]",
            T_CENTER_JUSTIFIED => "[b]",
            T_RIGHT_JUSTIFIED => "[rb]",
            _ => {
                ctx.warn("Text incorrectly positioned");
                "[lb]"
            }
        };
        self.set_color(out, ctx, t.color);
        fprintf!(out, "\\put({},{}){{", x, y);
        if t.angle != 0.0 {
            fprintf!(out, "\\rotatebox{{{:.1}}}{{", Cf(t.angle * 180. / PI));
        }
        // \smash puts the text on the baseline
        fprintf!(out, "\\makebox(0,0){}{{\\smash{{", tpos);
        let mut t = t.clone();
        self.fonts.select_font(
            out,
            ctx,
            &mut t,
            self.select_fontsize,
            self.select_fontname,
            self.only_texfonts,
        );
        let s = c_string(&t.cstring);
        put_string_bytes(out, s, &text_bytes(self.enc, s), t.font, t.special_text());
        if t.angle != 0.0 {
            out.puts("}");
        }
        out.puts("}}}\n");
        Ok(())
    }

    fn end(&mut self, ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        // close a colour scope that may be open
        self.set_color(out, ctx, DEFAULT);
        // line caps and joins reach beyond } and \end{picture}: reset them
        self.set_linecap(out, ctx, BUTTCAP);
        self.set_linejoin(out, ctx, MITERJOIN);
        out.puts("\\end{picture}%\n");
        if self.pagemode {
            out.puts("\\end{document}");
        }
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}
