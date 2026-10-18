//! fig2dev's `ibmgl` driver: HP-GL/2 for the IBM 6180 colour plotter, with
//! the IBM Graphics Enhancement Cartridge's arcs (on by default; `-c`
//! toggles), optionally wrapped in PCL job control (`-k`). Coordinates are
//! inches, scaled onto the plotter's window by `IP`/`SC`; texts use the
//! HP stick font sized with `SI`, or the PostScript font's HP-GL/2
//! counterpart with `SD` (`-F`).
//!
//! The pen, pattern and font tables can be replaced from files (`-p`, `-l`,
//! `-f`), as in fig2dev; the dxf driver, adapted from this one, shares
//! them, the line style state and the spline flattening.
//
// Ported from fig2dev dev/genibmgl.c (xfig licence, see LICENSE-xfig).

use super::cgm::fma;
use crate::gendev::{strtod_prefix, Ctx, Driver, PAPERDEF};
use crate::object::{
    Arc, Arrow, Compound, Ellipse, Line, Spline, Text, DASH_LINE, DEFAULT, DOTTED_LINE, SOLID_LINE, T_ARC_BOX, T_BOX,
    T_CENTER_JUSTIFIED, T_LEFT_JUSTIFIED, T_OPEN_ARC, T_PIC_BOX, T_POLYGON, T_POLYLINE, T_RIGHT_JUSTIFIED,
};
use crate::{fprintf, Out};
use std::f64::consts::PI;

/// The driver for `lang`.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    let _ = lang;
    Some(Box::new(Ibmgl::default()))
}

pub(crate) const FONTS: usize = 35;
pub(crate) const COLORS: usize = 8;
pub(crate) const PATTERNS: i32 = 21;
/// Radians per step of a flattened arc (`DELTA`).
const DELTA: f64 = PI / 36.0;
/// Plotter units per inch (1 unit = 0.025 mm).
const UNITS_PER_INCH: f64 = 1016.0;
const ISO_A4_HEIGHT: f64 = 842.0;
const ISO_A4_WIDTH: f64 = 595.0;
const ANSI_A_HEIGHT: f64 = 792.0;
const ANSI_A_WIDTH: f64 = 612.0;
/// cm/s.
const SPEED_LIMIT: f64 = 128.0;
/// Splines are subdivided down to this size, inches.
const THRESHOLD: f64 = 0.05;
/// Subdivision depth beyond which a spline segment is drawn straight
/// (fig2dev has no limit; it only matters for non-finite coordinates).
const MAX_SUBDIVISION: u32 = 64;

/// PostScript font → HP-GL/2 typeface, italic, bold (`psfont2hpgl`).
const PSFONT2HPGL: [(i32, i32, i32); FONTS] = [
    (5, 0, 0),
    (5, 1, 0),
    (5, 0, 3),
    (5, 1, 3),
    (31, 0, 0),
    (31, 1, 0),
    (31, 0, 3),
    (31, 1, 3),
    (47, 0, 0),
    (47, 1, 0),
    (47, 0, 3),
    (47, 1, 3),
    (3, 0, 0),
    (3, 1, 0),
    (3, 0, 3),
    (3, 1, 3),
    (4, 0, 0),
    (4, 1, 0),
    (4, 0, 3),
    (4, 1, 3),
    (4, 0, 0),
    (4, 1, 0),
    (4, 0, 3),
    (4, 1, 3),
    (23, 0, 0),
    (23, 1, 0),
    (23, 0, 3),
    (23, 1, 3),
    (15, 0, 0),
    (15, 1, 0),
    (15, 0, 3),
    (15, 1, 3),
    (52, 0, 0),
    (43, 0, 0),
    (45, 0, 0),
];

/// C's `%.Nf`, with C's spelling of infinities and NaN.
pub(crate) fn cf(x: f64, prec: usize) -> String {
    if x.is_nan() {
        // macOS prints NaN without its sign.
        "nan".to_string()
    } else if x.is_infinite() {
        if x < 0.0 { "-inf" } else { "inf" }.to_string()
    } else {
        format!("{x:.prec$}")
    }
}

/// A `scanf` over the bytes of a file or an option argument.
pub(crate) struct Scan<'a> {
    s: &'a [u8],
    pos: usize,
    /// A conversion failed: `fscanf` stops there, and later calls fail at
    /// the same place.
    pub failed: bool,
}

impl<'a> Scan<'a> {
    pub fn new(s: &'a [u8]) -> Scan<'a> {
        Scan {
            s,
            pos: 0,
            failed: false,
        }
    }

    fn rest(&self) -> &'a [u8] {
        self.s.get(self.pos..).unwrap_or(&[])
    }

    fn skip_ws(&mut self) {
        while self.rest().first().is_some_and(|b| b.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    /// `%d`.
    pub fn int(&mut self) -> Option<i32> {
        if self.failed {
            return None;
        }
        self.skip_ws();
        let r = self.rest();
        let mut i = usize::from(matches!(r.first(), Some(b'+' | b'-')));
        let ds = i;
        while r.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == ds {
            self.failed = true;
            return None;
        }
        let v = std::str::from_utf8(r.get(..i).unwrap_or(&[]))
            .ok()
            .and_then(|t| t.parse::<i64>().ok())
            .unwrap_or(0);
        self.pos += i;
        Some(v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32)
    }

    /// `%lf`.
    pub fn float(&mut self) -> Option<f64> {
        if self.failed {
            return None;
        }
        self.skip_ws();
        let r = self.rest();
        let text = match std::str::from_utf8(r) {
            Ok(t) => t,
            Err(e) => std::str::from_utf8(r.get(..e.valid_up_to()).unwrap_or(&[])).unwrap_or(""),
        };
        match strtod_prefix(text) {
            Some((v, n)) => {
                self.pos += n;
                Some(v)
            }
            None => {
                self.failed = true;
                None
            }
        }
    }

    /// A literal character of the format.
    pub fn lit(&mut self, c: u8) -> bool {
        if !self.failed && self.rest().first() == Some(&c) {
            self.pos += 1;
            return true;
        }
        self.failed = true;
        false
    }
}

/// Assign `v` if the conversion succeeded (`fscanf` leaves the variable
/// alone otherwise).
fn set<T>(slot: Option<&mut T>, v: Option<T>) {
    if let (Some(slot), Some(v)) = (slot, v) {
        *slot = v;
    }
}

/// The pen, pattern and font tables of genibmgl.c and gendxf.c.
#[derive(Clone, Debug)]
pub(crate) struct Tables {
    pub pen_number: [i32; COLORS + 1],
    pub pen_thickness: [f64; COLORS + 1],
    pub line_type: [i32; PATTERNS as usize],
    pub line_space: [f64; PATTERNS as usize],
    pub fill_type: [i32; PATTERNS as usize],
    pub fill_space: [f64; PATTERNS as usize],
    pub fill_angle: [f64; PATTERNS as usize],
    pub standard: [i32; FONTS + 1],
    pub alternate: [i32; FONTS + 1],
    pub slant: [f64; FONTS + 1],
    pub wide: [f64; FONTS + 1],
    pub high: [f64; FONTS + 1],
}

impl Tables {
    /// The tables with the character sizes `wide` and `high`.
    pub fn new(wide: f64, high: f64) -> Tables {
        let mut slant = [0.0; FONTS + 1];
        for (i, s) in slant.iter_mut().enumerate().take(FONTS - 1) {
            if i % 2 == 1 {
                *s = 10.0;
            }
        }
        Tables {
            pen_number: [1, 2, 3, 4, 5, 6, 7, 8, 1],
            pen_thickness: [0.3; COLORS + 1],
            line_type: [1, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, -1, -1, -1, -1, -1, -1, -1, -1],
            line_space: [
                0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.3, 0.3,
            ],
            fill_type: [3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 1, 2],
            fill_space: [0.1; PATTERNS as usize],
            fill_angle: [
                0.0, -45.0, 0.0, 45.0, 90.0, -45.0, 0.0, 45.0, 90.0, -45.0, 0.0, 45.0, 90.0, -45.0, 0.0, 45.0, 90.0,
                0.0, 0.0, 0.0, 0.0,
            ],
            standard: [0; FONTS + 1],
            alternate: [1; FONTS + 1],
            slant,
            wide: [wide; FONTS + 1],
            high: [high; FONTS + 1],
        }
    }

    /// The options that read the tables from files: `-f` (fonts), `-l`
    /// (fill patterns), `-p` (pens). `false` for other options.
    pub fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> bool {
        if !matches!(opt, 'f' | 'l' | 'p') {
            return false;
        }
        let name = arg.unwrap_or("");
        let data = match std::fs::read(name) {
            Ok(d) => d,
            Err(_) => {
                // fig2dev then calls fclose(NULL), and crashes.
                ctx.warn(format!("Couldn't open {name}"));
                return true;
            }
        };
        let mut sc = Scan::new(&data);
        match opt {
            'f' => {
                for i in 0..=FONTS {
                    set(self.standard.get_mut(i), sc.int());
                    set(self.alternate.get_mut(i), sc.int());
                    set(self.slant.get_mut(i), sc.float());
                    set(self.wide.get_mut(i), sc.float());
                    set(self.high.get_mut(i), sc.float());
                }
            }
            'l' => {
                for i in 0..PATTERNS as usize {
                    set(self.line_type.get_mut(i), sc.int());
                    set(self.line_space.get_mut(i), sc.float());
                    set(self.fill_type.get_mut(i), sc.int());
                    set(self.fill_space.get_mut(i), sc.float());
                    set(self.fill_angle.get_mut(i), sc.float());
                }
            }
            _ => {
                for i in 0..=COLORS {
                    set(self.pen_number.get_mut(i), sc.int());
                    set(self.pen_thickness.get_mut(i), sc.float());
                }
            }
        }
        true
    }

    /// `FONT(T)`: the font's table index, `FONTS` for others.
    pub fn font(font: i32) -> usize {
        usize::try_from(font).ok().filter(|&f| f < FONTS).unwrap_or(FONTS)
    }
}

/// The current line style (`line_style`, `dash_length`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct LineStyle {
    pub style: i32,
    pub dash_length: f64,
}

impl Default for LineStyle {
    fn default() -> LineStyle {
        LineStyle {
            style: SOLID_LINE,
            dash_length: f64::from(DEFAULT),
        }
    }
}

impl LineStyle {
    /// `set_style`: the `LT` command for `style`, dashes `length` long;
    /// `scale` converts the length to the output's unit.
    pub fn set(&mut self, out: &mut Out, style: i32, length: f64, scale: f64) {
        if style == self.style {
            if (style == DASH_LINE || style == DOTTED_LINE) && self.dash_length != length && length > 0.0 {
                self.dash_length = length;
                let n = if style == DASH_LINE { 2 } else { 1 };
                fprintf!(out, "LT{},{};\n", n, cf(self.dash_length * 2.0 * scale, 4));
            }
            return;
        }
        self.style = style;
        match style {
            SOLID_LINE => out.puts("LT;\n"),
            DASH_LINE | DOTTED_LINE => {
                let n = if style == DASH_LINE { 2 } else { 1 };
                if self.dash_length != length && length > 0.0 {
                    self.dash_length = length;
                }
                if self.dash_length > 0.0 {
                    fprintf!(out, "LT{},{};\n", n, cf(self.dash_length * 2.0 * scale, 4));
                } else {
                    fprintf!(out, "LT{},-1.0;\n", n);
                }
            }
            _ => {}
        }
    }
}

/// How a driver draws the arrowheads of old-style splines.
pub(crate) trait SplineArrow {
    /// `draw_arrow_head` from `(x1, y1)` to `(x2, y2)`, output units.
    fn spline_arrow(&mut self, out: &mut Out, x1: f64, y1: f64, x2: f64, y2: f64, a: &Arrow);
}

fn pa(out: &mut Out, x: f64, y: f64) {
    fprintf!(out, "PA{},{};\n", cf(x, 4), cf(y, 4));
}

/// `bezier_spline`: subdivide a cubic Bézier segment.
#[allow(clippy::too_many_arguments)]
fn bezier_spline(out: &mut Out, a0: f64, b0: f64, a1: f64, b1: f64, a2: f64, b2: f64, a3: f64, b3: f64, depth: u32) {
    let (x0, y0, x3, y3) = (a0, b0, a3, b3);
    let near = (x0 - x3).abs() < THRESHOLD && (y0 - y3).abs() < THRESHOLD;
    if near || depth >= MAX_SUBDIVISION || !(x0 - x3).is_finite() || !(y0 - y3).is_finite() {
        pa(out, x3, y3);
        return;
    }
    let (tx, ty) = ((a1 + a2) / 2.0, (b1 + b2) / 2.0);
    let (sx1, sy1) = ((x0 + a1) / 2.0, (y0 + b1) / 2.0);
    let (sx2, sy2) = ((sx1 + tx) / 2.0, (sy1 + ty) / 2.0);
    let (tx2, ty2) = ((a2 + x3) / 2.0, (b2 + y3) / 2.0);
    let (tx1, ty1) = ((tx2 + tx) / 2.0, (ty2 + ty) / 2.0);
    let (xmid, ymid) = ((sx2 + tx1) / 2.0, (sy2 + ty1) / 2.0);
    bezier_spline(out, x0, y0, sx1, sy1, sx2, sy2, xmid, ymid, depth + 1);
    bezier_spline(out, xmid, ymid, tx1, ty1, tx2, ty2, x3, y3, depth + 1);
}

/// `quadratic_spline`: subdivide a quadratic B-spline segment.
#[allow(clippy::too_many_arguments)]
fn quadratic_spline(out: &mut Out, a1: f64, b1: f64, a2: f64, b2: f64, a3: f64, b3: f64, a4: f64, b4: f64, depth: u32) {
    let (x1, y1, x4, y4) = (a1, b1, a4, b4);
    let (xmid, ymid) = ((a2 + a3) / 2.0, (b2 + b3) / 2.0);
    let stop = depth >= MAX_SUBDIVISION || !(x1 - x4).is_finite() || !(y1 - y4).is_finite();
    if stop || ((x1 - xmid).abs() < THRESHOLD && (y1 - ymid).abs() < THRESHOLD) {
        pa(out, xmid, ymid);
    } else {
        quadratic_spline(
            out,
            x1,
            y1,
            (x1 + a2) / 2.0,
            (y1 + b2) / 2.0,
            fma(3.0, a2, a3) / 4.0,
            fma(3.0, b2, b3) / 4.0,
            xmid,
            ymid,
            depth + 1,
        );
    }
    if stop || ((xmid - x4).abs() < THRESHOLD && (ymid - y4).abs() < THRESHOLD) {
        pa(out, x4, y4);
    } else {
        quadratic_spline(
            out,
            xmid,
            ymid,
            fma(3.0, a3, a2) / 4.0,
            fma(3.0, b3, b2) / 4.0,
            (a3 + x4) / 2.0,
            (b3 + y4) / 2.0,
            x4,
            y4,
            depth + 1,
        );
    }
}

/// `gen*_itp_spline`: an interpolated spline as Bézier segments between
/// its points; `sx`, `sy` convert Fig coordinates to output units.
pub(crate) fn itp_spline(
    dev: &mut dyn SplineArrow,
    out: &mut Out,
    s: &Spline,
    sx: &dyn Fn(f64) -> f64,
    sy: &dyn Fn(f64) -> f64,
) {
    let (Some(p1), Some(c1)) = (s.points.first(), s.controls.first()) else {
        return;
    };
    let (mut x2, mut y2) = (sx(f64::from(p1.x)), sy(f64::from(p1.y)));
    if s.thickness != 0 {
        if let Some(a) = &s.back_arrow {
            dev.spline_arrow(out, sx(c1.rx), sy(c1.ry), x2, y2, a);
        }
    }
    fprintf!(out, "PA{},{};PD;\n", cf(x2, 4), cf(y2, 4));
    let mut cp1 = *c1;
    for (p2, cp2) in s.points.iter().zip(s.controls.iter()).skip(1) {
        let (x1, y1) = (x2, y2);
        x2 = sx(f64::from(p2.x));
        y2 = sy(f64::from(p2.y));
        bezier_spline(out, x1, y1, sx(cp1.rx), sy(cp1.ry), sx(cp2.lx), sy(cp2.ly), x2, y2, 0);
        cp1 = *cp2;
    }
    out.puts("PU;\n");
    if s.thickness != 0 {
        if let Some(a) = &s.for_arrow {
            dev.spline_arrow(out, sx(cp1.lx), sy(cp1.ly), x2, y2, a);
        }
    }
}

/// `gen*_ctl_spline`: an approximated (B-)spline.
pub(crate) fn ctl_spline(
    dev: &mut dyn SplineArrow,
    out: &mut Out,
    s: &Spline,
    sx: &dyn Fn(f64) -> f64,
    sy: &dyn Fn(f64) -> f64,
) {
    let (Some(p0), Some(p1)) = (s.points.first(), s.points.get(1)) else {
        return;
    };
    let (mut x1, mut y1) = (sx(f64::from(p0.x)), sy(f64::from(p0.y)));
    let (mut x2, mut y2) = (sx(f64::from(p1.x)), sy(f64::from(p1.y)));
    let (mut cx1, mut cy1) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
    let (mut cx2, mut cy2) = (fma(3.0, x2, x1) / 4.0, fma(3.0, y2, y1) / 4.0);
    if s.closed_spline() {
        fprintf!(out, "PA{},{};PD;\n ", cf(cx1, 4), cf(cy1, 4));
    } else {
        if s.thickness != 0 {
            if let Some(a) = &s.back_arrow {
                dev.spline_arrow(out, cx1, cy1, x1, y1, a);
            }
        }
        fprintf!(out, "PA{},{};PD{},{};\n", cf(x1, 4), cf(y1, 4), cf(cx1, 4), cf(cy1, 4));
    }
    for p in s.points.iter().skip(2) {
        (x1, y1) = (x2, y2);
        x2 = sx(f64::from(p.x));
        y2 = sy(f64::from(p.y));
        let (cx3, cy3) = (fma(3.0, x1, x2) / 4.0, fma(3.0, y1, y2) / 4.0);
        let (cx4, cy4) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
        quadratic_spline(out, cx1, cy1, cx2, cy2, cx3, cy3, cx4, cy4, 0);
        (cx1, cy1) = (cx4, cy4);
        (cx2, cy2) = (fma(3.0, x2, x1) / 4.0, fma(3.0, y2, y1) / 4.0);
    }
    (x1, y1) = (x2, y2);
    x2 = sx(f64::from(p1.x));
    y2 = sy(f64::from(p1.y));
    let (cx3, cy3) = (fma(3.0, x1, x2) / 4.0, fma(3.0, y1, y2) / 4.0);
    let (cx4, cy4) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
    if s.closed_spline() {
        quadratic_spline(out, cx1, cy1, cx2, cy2, cx3, cy3, cx4, cy4, 0);
        out.puts("PU;\n");
    } else {
        fprintf!(out, "PA{},{};PU;\n", cf(x1, 4), cf(y1, 4));
        if s.thickness != 0 {
            if let Some(a) = &s.for_arrow {
                dev.spline_arrow(out, cx1, cy1, x1, y1, a);
            }
        }
    }
}

/// fig2dev's `static` state of genibmgl.c.
#[derive(Debug)]
pub struct Ibmgl {
    ibmgec: bool,
    pcljcl: bool,
    reflected: bool,
    correct_font_size: bool,
    tables: Tables,
    line_color: i32,
    line: LineStyle,
    fill_pattern: i32,
    pageheight: f64,
    pagewidth: f64,
    pen_speed: f64,
    xz: f64,
    yz: f64,
    xl: f64,
    yl: f64,
    xu: f64,
    yu: f64,
    /// cent/inch, cent/pixel, cm/point (width, height).
    cpi: f64,
    cpp: f64,
    wcmpp: f64,
    hcmpp: f64,
    ppi: f64,
    current_width: i32,
    pen: i32,
    pen_thick: f64,
    font: i32,
    size: i32,
    theta: f64,
    angle: f64,
}

impl Default for Ibmgl {
    fn default() -> Ibmgl {
        Ibmgl {
            // fig2dev is configured with IBMGEC and A4 by default.
            ibmgec: true,
            pcljcl: false,
            reflected: false,
            correct_font_size: false,
            tables: Tables::new(0.401, 0.561),
            line_color: DEFAULT,
            line: LineStyle::default(),
            fill_pattern: DEFAULT,
            pageheight: ISO_A4_HEIGHT,
            pagewidth: ISO_A4_WIDTH,
            pen_speed: SPEED_LIMIT,
            xz: 0.0,
            yz: 0.0,
            xl: 0.0,
            yl: 0.0,
            xu: 1456.0 / 25.4,
            yu: 1030.0 / 25.4,
            cpi: 0.0,
            cpp: 0.0,
            wcmpp: 0.0,
            hcmpp: 0.0,
            ppi: 1200.0,
            current_width: -1,
            pen: 0,
            pen_thick: 0.3,
            font: DEFAULT,
            size: DEFAULT,
            theta: 0.0,
            angle: 0.0,
        }
    }
}

/// `arc_tangent`: a point on the tangent at `(x2, y2)` of the circle
/// around `(x1, y1)`.
fn arc_tangent(x1: f64, y1: f64, x2: f64, y2: f64, ccw: bool) -> (f64, f64) {
    if ccw {
        (x2 - (y2 - y1), y2 + (x2 - x1))
    } else {
        (x2 + (y2 - y1), y2 - (x2 - x1))
    }
}

impl Ibmgl {
    fn set_style(&mut self, out: &mut Out, style: i32, length: f64) {
        let cpp = self.cpp;
        self.line.set(out, style, length, cpp);
    }

    /// `draw_arrow_head` from `(x1, y1)` to `(x2, y2)`, inches.
    #[allow(clippy::too_many_arguments)]
    fn arrow_head(&mut self, out: &mut Out, x1: f64, y1: f64, x2: f64, y2: f64, ht: f64, wid: f64) {
        let (dx, dy) = (x2 - x1, y1 - y2);
        let l = fma(dx, dx, dy * dy).sqrt();
        let (sina, cosa) = (dy / l, dx / l);
        let xb = fma(x2, cosa, -(y2 * sina));
        let yb = fma(x2, sina, y2 * cosa);
        let x = xb - ht;
        let y = yb - wid / 2.0;
        let xc = fma(x, cosa, y * sina);
        let yc = fma(-x, sina, y * cosa);
        let y = yb + wid / 2.0;
        let xd = fma(x, cosa, y * sina);
        let yd = fma(-x, sina, y * cosa);
        // Solid, then the line style back.
        let saved = self.line;
        self.set_style(out, SOLID_LINE, 0.0);
        fprintf!(
            out,
            "PA{},{};PD{},{},{},{};PU\n",
            cf(xc, 4),
            cf(yc, 4),
            cf(x2, 4),
            cf(y2, 4),
            cf(xd, 4),
            cf(yd, 4)
        );
        self.set_style(out, saved.style, saved.dash_length);
    }

    /// `set_width`: `PW`, from Fig thickness, 0.3 mm per 1/80 inch.
    fn set_width(&mut self, out: &mut Out, w: i32) {
        if w == self.current_width {
            return;
        }
        fprintf!(out, "PW{};\n", cf(f64::from(w.wrapping_mul(80)) / self.ppi * 0.3, 1));
        self.current_width = w;
    }

    /// `set_color`: the pen of a colour.
    fn set_color(&mut self, out: &mut Out, color: i32) {
        if self.line_color == color {
            return;
        }
        self.line_color = color;
        let c = (COLORS as i32).wrapping_add(color).rem_euclid(COLORS as i32) as usize;
        let number = self.tables.pen_number.get(c).copied().unwrap_or(1);
        if self.pen != number {
            self.pen = number;
            fprintf!(out, "SP{};\n", number);
        }
        let thick = self.tables.pen_thickness.get(c).copied().unwrap_or(0.3);
        if self.pen_thick != thick {
            self.pen_thick = thick;
            fprintf!(out, "PW{};\n", cf(thick, 4));
        }
    }

    fn fill_polygon(&mut self, out: &mut Out, pattern: i32, color: i32) {
        if !(0 < pattern && pattern < PATTERNS) {
            return;
        }
        let i = pattern as usize;
        self.set_color(out, color);
        if self.fill_pattern != pattern {
            self.fill_pattern = pattern;
            let angle = self.tables.fill_angle.get(i).copied().unwrap_or(0.0);
            fprintf!(
                out,
                "FT{},{},{};",
                self.tables.fill_type.get(i).copied().unwrap_or(0),
                cf(self.tables.fill_space.get(i).copied().unwrap_or(0.0), 4),
                cf(if self.reflected { -angle } else { angle }, 4)
            );
        }
        let saved = self.line;
        fprintf!(
            out,
            "LT{},{};FP;\n",
            self.tables.line_type.get(i).copied().unwrap_or(0),
            cf(self.tables.line_space.get(i).copied().unwrap_or(0.0) * self.cpi, 4)
        );
        self.line = LineStyle {
            style: DEFAULT,
            dash_length: f64::from(DEFAULT),
        };
        self.set_style(out, saved.style, saved.dash_length);
    }

    /// `arc`: from `(sx, sy)` around `(cx, cy)` by `theta`.
    fn arc_to(&self, out: &mut Out, sx: f64, sy: f64, cx: f64, cy: f64, theta: f64) {
        if self.ibmgec {
            // Always called with DELTA, so the default chord angle.
            fprintf!(out, "AA{},{},{};", cf(cx, 4), cf(cy, 4), cf(theta * 180.0 / PI, 4));
            return;
        }
        let delta = if theta < 0.0 { -DELTA } else { DELTA };
        let at = |alpha: f64| {
            (
                fma(-(sy - cy), alpha.sin(), fma(sx - cx, alpha.cos(), cx)),
                fma(sx - cx, alpha.sin(), fma(sy - cy, alpha.cos(), cy)),
            )
        };
        let mut alpha = delta;
        // |theta| is at most 2π: at most 72 steps.
        let mut steps = 0;
        while alpha.abs() < theta.abs() && steps < 1000 {
            let (x, y) = at(alpha);
            pa(out, x, y);
            alpha += delta;
            steps += 1;
        }
        let (x, y) = at(theta);
        pa(out, x, y);
    }

    fn fill_shown(&self, fill_style: i32) -> bool {
        self.ibmgec && (0..PATTERNS).contains(&fill_style)
    }
}

impl SplineArrow for Ibmgl {
    fn spline_arrow(&mut self, out: &mut Out, x1: f64, y1: f64, x2: f64, y2: f64, a: &Arrow) {
        let (ht, wid) = (a.ht / self.ppi, a.wid / self.ppi);
        self.arrow_head(out, x1, y1, x2, y2, ht, wid);
    }
}

impl Driver for Ibmgl {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        if self.tables.option(ctx, opt, arg) {
            return Ok(());
        }
        let a = arg.unwrap_or("");
        match opt {
            'a' => {
                // The other paper size than the default A4.
                self.pageheight = ANSI_A_HEIGHT;
                self.pagewidth = ANSI_A_WIDTH;
            }
            'c' => self.ibmgec = !self.ibmgec,
            'd' => {
                let mut sc = Scan::new(a.as_bytes());
                set(Some(&mut self.xl), sc.float());
                for slot in [&mut self.yl, &mut self.xu, &mut self.yu] {
                    if sc.lit(b',') {
                        set(Some(slot), sc.float());
                    }
                }
            }
            'k' => self.pcljcl = true,
            'F' => self.correct_font_size = true,
            'G' | 'L' => {}
            'P' => {
                ctx.landscape = false;
                ctx.orientspec = true;
            }
            'S' => self.pen_speed = crate::gendev::atof(a),
            'v' => self.reflected = true,
            'x' => self.xz = crate::gendev::atof(a),
            'y' => self.yz = crate::gendev::atof(a),
            'z' => {
                ctx.papersize = a.to_string();
                ctx.paperspec = true;
            }
            _ => return Err(format!("Argument -{opt} unknown to ibmgl driver.")),
        }
        Ok(())
    }

    // The negated comparisons are fig2dev's else branches: NaN fails them.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, _objects: &Compound) -> Result<(), String> {
        let mag = ctx.mag;
        if mag.abs() < 1.0 / 2048.0 || !mag.is_finite() {
            return Err("|mag| < 1/2048".into());
        }
        self.ppi = ctx.ppi;
        if ctx.paperspec {
            if ctx.papersize.eq_ignore_ascii_case("ledger") {
                ctx.papersize = "tabloid".into();
            }
            if let Some(&(name, w, h)) = PAPERDEF.iter().find(|p| p.0.eq_ignore_ascii_case(&ctx.papersize)) {
                self.pagewidth = f64::from(w);
                self.pageheight = f64::from(h);
                ctx.papersize = name.to_string();
            }
            // fig2dev's "Unknown paper size" check can't trigger: the
            // page size starts out positive.
        }
        let points_per_inch = 72.0;
        self.pagewidth /= points_per_inch;
        self.pageheight /= points_per_inch;
        self.wcmpp = 2.54 / points_per_inch;
        self.hcmpp = self.wcmpp;

        let (ph, pw) = (self.pageheight, self.pagewidth);
        if !(self.xl < self.xu) {
            return Err("xur <= xll".into());
        }
        if !(0.0 < self.xu) {
            return Err("xur <= 0.0".into());
        }
        if !(self.xl < ph) {
            return Err(format!("xll >= {}", cf(ph, 2)));
        }
        self.xl = if 0.0 < self.xl { self.xl } else { 0.0 };
        self.xu = if self.xu < ph { self.xu } else { ph };
        if !(self.yl < self.yu) {
            return Err("yur <= yll".into());
        }
        if !(0.0 < self.yu) {
            return Err("yur <= 0.0".into());
        }
        if !(self.yl < pw) {
            return Err(format!("yll >= {}", cf(pw, 2)));
        }
        self.yl = if 0.0 < self.yl { self.yl } else { 0.0 };
        self.yu = if self.yu < pw { self.yu } else { pw };
        let (xl, yl, xu, yu) = (self.xl, self.yl, self.xu, self.yu);

        self.cpi = mag * 100.0 / fma(xu - xl, xu - xl, (yu - yl) * (yu - yl)).sqrt();
        self.cpp = self.cpi / ctx.ppi;

        if self.pcljcl {
            out.puts("\x1bE");
            if ctx.landscape {
                out.puts("\x1b&l1O");
            }
            out.puts("\x1b%0B");
        }
        out.puts("BP;IN;\n");

        let (llx, lly, urx, ury) = (
            f64::from(ctx.llx),
            f64::from(ctx.lly),
            f64::from(ctx.urx),
            f64::from(ctx.ury),
        );
        let ppi = ctx.ppi;
        let (mut xoff, mut yoff) = (0.0, 0.0);
        let (p1x, p1y, p2x, p2y, xll, yll, xur, yur, height, width);
        if !ctx.landscape {
            out.puts("RO90;\n");
            xll = (yl * UNITS_PER_INCH) as i32;
            xur = (yu * UNITS_PER_INCH) as i32;
            yll = ((ph - xu) * UNITS_PER_INCH) as i32;
            yur = ((ph - xl) * UNITS_PER_INCH) as i32;
            height = yu - yl;
            width = xu - xl;
            p1x = xll;
            p2x = xur;
            if self.reflected {
                self.hcmpp = -self.hcmpp;
                p1y = yll;
                p2y = yur;
            } else {
                p1y = yur;
                p2y = yll;
            }
            if ctx.center {
                yoff = (ph - (urx - llx) * mag / ppi) / 2.0 - llx * mag / ppi;
                xoff = (pw - (ury - lly) * mag / ppi) / 2.0 - lly * mag / ppi;
            }
        } else {
            xll = (xl * UNITS_PER_INCH) as i32;
            yll = (yl * UNITS_PER_INCH) as i32;
            yur = (yu * UNITS_PER_INCH) as i32;
            xur = (xu * UNITS_PER_INCH) as i32;
            height = xu - xl;
            width = yu - yl;
            if self.reflected {
                self.wcmpp = -self.wcmpp;
                p1x = xur;
                p2x = xll;
            } else {
                p1x = xll;
                p2x = xur;
            }
            p1y = yur;
            p2y = yll;
            if ctx.center {
                xoff = (ph - (urx - llx) * mag / ppi) / 2.0 - llx * mag / ppi;
                yoff = (pw - (ury - lly) * mag / ppi) / 2.0 - lly * mag / ppi;
            }
        }
        if xoff < 0.0 {
            xoff = 0.0;
        }
        if yoff < 0.0 {
            yoff = 0.0;
        }
        let xmin = -self.xz - xoff;
        let ymin = self.yz - yoff;
        let xmax = -self.xz - xoff + height / mag;
        let ymax = self.yz - yoff + width / mag;
        fprintf!(out, "IP{},{},{},{};\n", p1x, p1y, p2x, p2y);
        fprintf!(out, "IW{},{},{},{};\n", xll, yll, xur, yur);
        fprintf!(
            out,
            "SC{},{},{},{};\n",
            cf(xmin, 4),
            cf(xmax, 4),
            cf(ymin, 4),
            cf(ymax, 4)
        );
        if 0.0 < self.pen_speed && self.pen_speed < SPEED_LIMIT {
            fprintf!(out, "VS{};\n", cf(self.pen_speed, 2));
        }
        Ok(())
    }

    fn arc(&mut self, _ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        if !(a.thickness != 0 || self.fill_shown(a.fill_style)) {
            return Ok(());
        }
        self.set_style(out, a.style, a.style_val);
        self.set_width(out, a.thickness);
        self.set_color(out, a.pen_color);
        let ppi = self.ppi;
        let (cx, cy) = (a.center.x / ppi, a.center.y / ppi);
        let [p0, _, p2] = a.point;
        let (sx, sy) = (f64::from(p0.x) / ppi, f64::from(p0.y) / ppi);
        let (ex, ey) = (f64::from(p2.x) / ppi, f64::from(p2.y) / ppi);
        let mut theta = (ey - cy).atan2(ex - cx) - (sy - cy).atan2(sx - cx);
        if a.direction != 0 {
            if theta > 0.0 {
                theta -= 2.0 * PI;
            }
        } else if theta < 0.0 {
            theta += 2.0 * PI;
        }
        if a.type_ == T_OPEN_ARC && a.thickness != 0 {
            if let Some(ar) = &a.back_arrow {
                let (x, y) = arc_tangent(cx, cy, sx, sy, a.direction == 0);
                self.arrow_head(out, x, y, sx, sy, ar.ht / ppi, ar.wid / ppi);
            }
        }
        fprintf!(out, "PA{},{};PM;PD;", cf(sx, 4), cf(sy, 4));
        self.arc_to(out, sx, sy, cx, cy, theta);
        out.puts("PU;PM2;\n");
        if a.thickness != 0 {
            out.puts("EP;\n");
        }
        if a.type_ == T_OPEN_ARC && a.thickness != 0 {
            if let Some(ar) = &a.for_arrow {
                let (x, y) = arc_tangent(cx, cy, ex, ey, a.direction != 0);
                self.arrow_head(out, x, y, ex, ey, ar.ht / ppi, ar.wid / ppi);
            }
        }
        if 0 < a.fill_style && a.fill_style < PATTERNS {
            self.fill_polygon(out, a.fill_style, a.fill_color);
        }
        Ok(())
    }

    fn ellipse(&mut self, _ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        if !(e.thickness != 0 || self.fill_shown(e.fill_style)) {
            return Ok(());
        }
        self.set_style(out, e.style, e.style_val);
        self.set_width(out, e.thickness);
        self.set_color(out, e.pen_color);
        let ppi = self.ppi;
        let (a, b) = (f64::from(e.radiuses.x) / ppi, f64::from(e.radiuses.y) / ppi);
        let (x0, y0) = (f64::from(e.center.x) / ppi, f64::from(e.center.y) / ppi);
        let angle = -e.angle;
        let delta = -DELTA;
        let (x, y) = (fma(angle.cos(), a, x0), fma(angle.sin(), a, y0));
        fprintf!(out, "PA{},{};PM;PD;\n", cf(x, 4), cf(y, 4));
        for j in 1..=72 {
            let alpha = f64::from(j) * delta;
            let x = fma(-(angle.sin() * b), alpha.sin(), fma(angle.cos() * a, alpha.cos(), x0));
            let y = fma(angle.cos() * b, alpha.sin(), fma(angle.sin() * a, alpha.cos(), y0));
            pa(out, x, y);
        }
        out.puts("PU;PM2;\n");
        if e.thickness != 0 {
            out.puts("EP;\n");
        }
        if 0 < e.fill_style && e.fill_style < PATTERNS {
            self.fill_polygon(out, e.fill_style, e.fill_color);
        }
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        if !(l.thickness != 0 || self.fill_shown(l.fill_style)) {
            return Ok(());
        }
        self.set_style(out, l.style, l.style_val);
        self.set_width(out, l.thickness);
        self.set_color(out, l.pen_color);
        let ppi = self.ppi;
        let Some(&p) = l.points.first() else {
            return Ok(());
        };
        let fx = |v: i32| f64::from(v) / ppi;
        match l.type_ {
            T_POLYLINE | T_BOX | T_POLYGON => {
                let Some(&q) = l.points.get(1) else {
                    fprintf!(out, "PA{},{};PD;PU;\n", cf(fx(p.x), 4), cf(fx(p.y), 4));
                    return Ok(());
                };
                if l.thickness != 0 {
                    if let Some(a) = &l.back_arrow {
                        self.arrow_head(out, fx(q.x), fx(q.y), fx(p.x), fx(p.y), a.ht / ppi, a.wid / ppi);
                    }
                }
                fprintf!(
                    out,
                    "PA{},{};PM;PD{},{};\n",
                    cf(fx(p.x), 4),
                    cf(fx(p.y), 4),
                    cf(fx(q.x), 4),
                    cf(fx(q.y), 4)
                );
                for r in l.points.iter().skip(2) {
                    pa(out, fx(r.x), fx(r.y));
                }
                out.puts("PU;PM2;\n");
                if l.thickness != 0 {
                    out.puts("EP;\n");
                }
                let n = l.points.len();
                if let (true, Some(a), Some(p), Some(q)) =
                    (l.thickness != 0, &l.for_arrow, l.points.get(n - 2), l.points.get(n - 1))
                {
                    self.arrow_head(out, fx(p.x), fx(p.y), fx(q.x), fx(q.y), a.ht / ppi, a.wid / ppi);
                }
                if 0 < l.fill_style && l.fill_style < PATTERNS {
                    self.fill_polygon(out, l.fill_style, l.fill_color);
                }
            }
            T_ARC_BOX => {
                let (mut llx, mut lly, mut urx, mut ury) = (p.x, p.y, p.x, p.y);
                for r in l.points.iter().skip(1) {
                    llx = llx.min(r.x);
                    urx = urx.max(r.x);
                    lly = lly.min(r.y);
                    ury = ury.max(r.y);
                }
                let (x0, x1) = (fx(llx), fx(urx));
                let dx = fx(l.radius);
                let (y0, y1) = (fx(ury), fx(lly));
                let dy = -dx;
                let angle = -PI / 2.0;
                fprintf!(out, "PA{},{};PM;PD;\n", cf(x0, 4), cf(y0 + dy, 4));
                self.arc_to(out, x0, y0 + dy, x0 + dx, y0 + dy, angle);
                pa(out, x1 - dx, y0);
                self.arc_to(out, x1 - dx, y0, x1 - dx, y0 + dy, angle);
                pa(out, x1, y1 - dy);
                self.arc_to(out, x1, y1 - dy, x1 - dx, y1 - dy, angle);
                pa(out, x0 + dx, y1);
                self.arc_to(out, x0 + dx, y1, x0 + dx, y1 - dy, angle);
                fprintf!(out, "PA{},{};PU;PM2;\n", cf(x0, 4), cf(y0 + dy, 4));
                if l.thickness != 0 {
                    out.puts("EP;\n");
                }
                if 0 < l.fill_style && l.fill_style < PATTERNS {
                    self.fill_polygon(out, l.fill_style, l.fill_color);
                }
            }
            T_PIC_BOX => ctx.warn("Warning: Pictures not supported in IBMGL language"),
            _ => {}
        }
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        if s.thickness != 0 {
            self.set_style(out, s.style, s.style_val);
            self.set_width(out, s.thickness);
            self.set_color(out, s.pen_color);
            let ppi = self.ppi;
            let sc = move |v: f64| v / ppi;
            if s.int_spline() {
                itp_spline(self, out, s, &sc, &sc);
            } else {
                ctl_spline(self, out, s, &sc, &sc);
            }
        }
        if 0 < s.fill_style && s.fill_style < PATTERNS {
            ctx.warn("Spline area fill not implemented");
        }
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String> {
        let font = Tables::font(t.font);
        let mut newfont = false;
        let mut newsize = false;
        if self.font != font as i32 {
            self.font = font as i32;
            // Italic fonts are simulated with a 10 degree slant.
            let slant = self.tables.slant.get(font).copied().unwrap_or(0.0);
            if self.theta != slant {
                self.theta = slant;
                fprintf!(out, "SL{};", cf((self.theta * PI / 180.0).tan(), 4));
            }
            newfont = true;
        }
        if f64::from(self.size) != t.size {
            // fig2dev keeps the size in an int.
            self.size = t.size as i32;
            newsize = true;
            if !self.correct_font_size {
                // The HP stick font: cap height and pitch with SI.
                let size = f64::from(self.size);
                let width = size * self.wcmpp * self.tables.wide.get(font).copied().unwrap_or(0.0);
                let height = size * self.hcmpp * self.tables.high.get(font).copied().unwrap_or(0.0);
                fprintf!(out, "SI{},{};", cf(width * ctx.mag, 4), cf(height * ctx.mag, 4));
            }
        }
        if self.correct_font_size && (newfont || newsize) {
            // fig2dev reads one entry beyond its table for unknown fonts.
            let (face, italic, bold) = PSFONT2HPGL.get(font).copied().unwrap_or((0, 0, 0));
            fprintf!(
                out,
                "SD2,1,4,{},5,{},6,{},7,{};SS;\n",
                fma(f64::from(self.size), ctx.mag, 0.5) as i32,
                italic,
                bold,
                face
            );
        }
        if self.angle != t.angle {
            self.angle = t.angle;
            let s = if self.reflected { -self.angle } else { self.angle };
            fprintf!(out, "DI{},{};", cf(self.angle.cos(), 4), cf(s.sin(), 4));
        }
        self.set_color(out, t.color);
        let ppi = self.ppi;
        fprintf!(
            out,
            "PA{},{};\n",
            cf(f64::from(t.base_x) / ppi, 4),
            cf(f64::from(t.base_y) / ppi, 4)
        );
        let bytes = t.raw.clone();
        let len = bytes.len() as f64;
        match t.type_ {
            DEFAULT | T_LEFT_JUSTIFIED => {}
            T_CENTER_JUSTIFIED => fprintf!(out, "CP{},0.0;", cf(-(len / 2.0), 4)),
            T_RIGHT_JUSTIFIED => fprintf!(out, "CP{},0.0;", cf(-len, 4)),
            _ => return Err("unknown text position type".into()),
        }
        out.puts("LB");
        out.bytes(&bytes);
        out.puts("\x03\n");
        Ok(())
    }

    fn end(&mut self, _ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        out.puts("PU;SP;IN;\n");
        if self.pcljcl {
            // End the job and eject the page.
            out.puts("\x1b%0A\x1bE");
        }
        Ok(0)
    }

    fn text_include(&self) -> bool {
        false
    }
}
