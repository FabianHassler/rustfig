//! fig2dev's `latex` driver: the figure as a LaTeX `picture` environment.
//!
//! LaTeX draws lines only at slopes dx/dy with |dx|, |dy| ≤ 6 (≤ 4 for
//! vectors), circles up to a fixed size, and ovals; so lines snap to the
//! nearest LaTeX slope, arcs become quarter ovals, ellipses ovals, and
//! splines are left out. Dashed and dotted lines are built from short
//! lines and dots. Texts go into `\makebox`es, rotated with `\rotatebox`
//! (fig2dev is built with `LATEX2E_GRAPHICS`), coloured with `\color`.
//!
//! Options: `-b` border margin (points), `-d` dash magnification, `-F`
//! don't select font names, `-f` the default font, `-l` the thickness from
//! which lines are thick, `-v` verbose (object comments), `-a` obsolete.
//
// Ported from fig2dev dev/genlatex.c (xfig licence, see LICENSE-xfig).

use std::fmt;

use crate::gendev::{print_comments, strtol, Ctx, DEFAULT_FONT_SIZE};
use crate::object::{
    Arc, Compound, Ellipse, Line, Point, Spline, Text, BLACK_FILL, DASH_LINE, DEFAULT, DOTTED_LINE, MAX_FONT,
    SOLID_LINE, T_ARC_BOX, T_BOX, T_CENTER_JUSTIFIED, T_CIRCLE_BY_DIA, T_CIRCLE_BY_RAD, T_LEFT_JUSTIFIED, T_OPEN_ARC,
    T_RIGHT_JUSTIFIED, UNFILLED,
};
use crate::util::latex::LatexColor;
use crate::util::texfonts::{put_string, TexFonts};
use crate::{fprintf, round, Driver, Out};

/// A dot for lines at `\thinlines` and `\thicklines`.
const THINDOT: &str = "\\small.";
const THICKDOT: &str = "\\normalsize.";
/// Dots for slanted lines too short for LaTeX.
const THIN_LDOT: &str = "\\tiny.";
const THICK_LDOT: &str = "\\small.";
const THIN_XOFF: f64 = 0.1 / 72.0;
const THIN_YOFF: f64 = 0.7 / 72.0;
const THICK_XOFF: f64 = 0.4 / 72.0;
const THICK_YOFF: f64 = 0.6 / 72.0;
const THIN_LXOFF: f64 = 0.1 / 72.0;
const THIN_LYOFF: f64 = 0.7 / 72.0;
const THICK_LXOFF: f64 = 0.4 / 72.0;
const THICK_LYOFF: f64 = 0.6 / 72.0;
/// The shortest slanted line LaTeX draws, 13 points.
const MIN_LEN: f64 = 13.0 / 72.27;
const TOP: i32 = 840;
const THINLINES: i32 = 1;
const THICKLINES: i32 = 2;
const MAXCIRCLEDIA: f64 = 80.0;
const MAXCIRCLERAD: f64 = (MAXCIRCLEDIA - 0.5) / (2.0 * 72.27);
#[allow(clippy::excessive_precision)]
const RAD2DEG: f64 = 57.295_779_513_082_320_877;
const TOLERANCE: f64 = 2.0;
const ARC_TOLERANCE: f64 = 1.0;
const LDOT_DIAMETER: f64 = 1.0 / 72.0;

/// The line slopes LaTeX draws: (x, y, angle in degrees), with C's
/// digits.
#[allow(clippy::excessive_precision)]
const LINE_ANGLES: [(i32, i32, f64); 25] = [
    (0, 1, 90.0),
    (1, 0, 0.0),
    (1, 1, 45.0),
    (1, 2, 63.434948822922010648),
    (1, 3, 71.565051177077989351),
    (1, 4, 75.963756532073521417),
    (1, 5, 78.690067525979786913),
    (1, 6, 80.537677791974382609),
    (2, 1, 26.565051177077989351),
    (2, 3, 56.309932474020213086),
    (2, 5, 68.198590513648188229),
    (3, 1, 18.434948822922010648),
    (3, 2, 33.690067525979786913),
    (3, 4, 53.130102354155978703),
    (3, 5, 59.036243467926478582),
    (4, 1, 14.036243467926478588),
    (4, 3, 36.869897645844021297),
    (4, 5, 51.340191745909909396),
    (5, 1, 11.309932474020213086),
    (5, 2, 21.801409486351811770),
    (5, 3, 30.963756532073521417),
    (5, 4, 38.659808254090090604),
    (5, 6, 50.194428907734805993),
    (6, 1, 9.4623222080256173906),
    (6, 5, 39.805571092265194006),
];

/// The vector slopes LaTeX draws.
#[allow(clippy::excessive_precision)]
const ARROW_ANGLES: [(i32, i32, f64); 13] = [
    (0, 1, 90.0),
    (1, 0, 0.0),
    (1, 1, 45.0),
    (1, 2, 63.434948822922010648),
    (1, 3, 71.565051177077989351),
    (1, 4, 75.963756532073521417),
    (2, 1, 26.565051177077989351),
    (2, 3, 56.309932474020213086),
    (3, 1, 18.434948822922010648),
    (3, 2, 33.690067525979786913),
    (3, 4, 53.130102354155978703),
    (4, 1, 14.036243467926478588),
    (4, 3, 36.869897645844021297),
];

/// A double printed as C's `printf` prints it: as Rust does, but "nan"
/// for NaN.
pub(super) struct Cf(pub f64);

impl fmt::Display for Cf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_nan() {
            let w = f.width().unwrap_or(0);
            if f.align() == Some(fmt::Alignment::Left) {
                write!(f, "{:<w$}", "nan")
            } else {
                write!(f, "{:>w$}", "nan")
            }
        } else {
            fmt::Display::fmt(&self.0, f)
        }
    }
}

/// `round4`: rounded to 4 decimals.
fn round4(x: f64) -> f64 {
    f64::from(round(10000.0 * x)) / 10000.0
}

/// `translate2`: Fig coordinates to the picture's (y up, from `TOP`).
fn trans(x: i32, y: i32) -> (i32, i32) {
    (x.wrapping_add(1), TOP.wrapping_sub(y).wrapping_sub(1))
}

/// `get_slope`: the LaTeX line (or, with `arrow`, vector) slope closest
/// to (dx, dy).
pub fn get_slope(dx: i32, dy: i32, arrow: bool) -> (i32, i32) {
    if dx == 0 {
        return (0, if dy < 0 { -1 } else { 1 });
    }
    let angle = (f64::from(dy).abs() / f64::from(dx).abs()).atan() * RAD2DEG;
    let table: &[(i32, i32, f64)] = if arrow { &ARROW_ANGLES } else { &LINE_ANGLES };
    let mut best = (0, 1);
    let mut d = 9.9e9;
    for &(x, y, a) in table {
        let d1 = (angle - a).abs();
        if d1 < d {
            best = (x, y);
            d = d1;
        }
    }
    let sx = if dx < 0 { -best.0 } else { best.0 };
    let sy = if dy < 0 { -best.1 } else { best.1 };
    (sx, sy)
}

/// The end of a line of x-length `l` from (x, y) at slope (sx, sy), and
/// the cosine of its angle.
fn line_end(x: i32, y: i32, sx: i32, sy: i32, l: f64) -> (i32, i32, f64) {
    if sx != 0 {
        let cosine =
            f64::from(sx.wrapping_abs()) / (f64::from(sx) * f64::from(sx) + f64::from(sy) * f64::from(sy)).sqrt();
        let x2 = if sx >= 0 {
            x.wrapping_add(round(l))
        } else {
            x.wrapping_sub(round(l))
        };
        let y2 = y.wrapping_add(round(if sx >= 0 { l } else { -l } * f64::from(sy) / f64::from(sx)));
        (x2, y2, cosine)
    } else {
        let y2 = if sy >= 0 {
            y.wrapping_add(round(l))
        } else {
            y.wrapping_sub(round(l))
        };
        (x, y2, 1.0)
    }
}

/// The `latex` driver's state (genlatex.c's statics).
#[derive(Debug)]
pub struct Latex {
    fonts: TexFonts,
    color: LatexColor,
    select_fontname: bool,
    verbose: bool,
    dash_mag: f64,
    thick_width: i32,
    unitlength: f64,
    cur_thickness: i32,
    dot_cmd: &'static str,
    ldot_cmd: &'static str,
    dot_xoffset: f64,
    dot_yoffset: f64,
    ldot_xoffset: f64,
    ldot_yoffset: f64,
    border_margin: i32,
}

impl Default for Latex {
    fn default() -> Latex {
        Latex {
            fonts: TexFonts::new(),
            color: LatexColor::default(),
            select_fontname: true,
            verbose: false,
            dash_mag: 1.0,
            thick_width: 2,
            unitlength: 0.0,
            cur_thickness: -1,
            dot_cmd: THINDOT,
            ldot_cmd: THIN_LDOT,
            dot_xoffset: 0.0,
            dot_yoffset: 0.0,
            ldot_xoffset: 0.0,
            ldot_yoffset: 0.0,
            border_margin: 0,
        }
    }
}

/// The driver for `lang`.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    (lang == "latex").then(|| Box::new(Latex::default()) as Box<dyn Driver>)
}

/// `-f`: make font `name` (rm, bf, it, sf, tt) the default.
pub(super) fn default_font_option(fonts: &mut TexFonts, ctx: &mut Ctx, name: &str) {
    let found = (1..=MAX_FONT as usize).find(|&i| fonts.names.get(i).is_some_and(|n| n == name));
    match found {
        None => ctx.warn(format!("warning: non-standard font name {name} ignored")),
        Some(i) => {
            if let Some(v) = fonts.names.get(i).cloned() {
                fonts.names[0] = v;
            }
            if let Some(v) = fonts.family.get(i).cloned() {
                fonts.family[0] = v;
            }
            if let Some(v) = fonts.series.get(i).cloned() {
                fonts.series[0] = v;
            }
            if let Some(v) = fonts.shape.get(i).cloned() {
                fonts.shape[0] = v;
            }
        }
    }
}

/// `texfontsizes[0] = texfontsizes[1] = TEXFONTSIZE(font_size or 11)`.
pub(super) fn set_default_size(fonts: &mut TexFonts, ctx: &Ctx) {
    let s = if ctx.font_size != 0.0 {
        ctx.font_size
    } else {
        DEFAULT_FONT_SIZE
    };
    let v = fonts.texfontsize(s) as i32;
    fonts.sizes[0] = v;
    fonts.sizes[1] = v;
}

impl Latex {
    /// `set_linewidth`: LaTeX knows thin and thick lines only.
    fn set_linewidth(&mut self, out: &mut Out, ctx: &Ctx, w: i32) {
        if w == 0 {
            return;
        }
        let latex_w = if w as f32 >= self.thick_width as f32 * ctx.thick_scale {
            THICKLINES
        } else {
            THINLINES
        };
        if latex_w == self.cur_thickness {
            return;
        }
        self.cur_thickness = latex_w;
        let u = self.unitlength;
        if latex_w == THICKLINES {
            out.puts("\\thicklines\n");
            self.dot_cmd = THICKDOT;
            self.dot_xoffset = round4(THICK_XOFF / u);
            self.dot_yoffset = round4(THICK_YOFF / u);
            self.ldot_cmd = THICK_LDOT;
            self.ldot_xoffset = round4(THICK_LXOFF / u);
            self.ldot_yoffset = round4(THICK_LYOFF / u);
        } else {
            out.puts("\\thinlines\n");
            self.dot_cmd = THINDOT;
            self.dot_xoffset = round4(THIN_XOFF / u);
            self.dot_yoffset = round4(THIN_YOFF / u);
            self.ldot_cmd = THIN_LDOT;
            self.ldot_xoffset = round4(THIN_LXOFF / u);
            self.ldot_yoffset = round4(THIN_LYOFF / u);
        }
    }

    /// The forward and backward arrowheads (`\vector…{0}`) of a line
    /// from (x, y) to (x2, y2) at slope (sx, sy).
    #[allow(clippy::too_many_arguments)]
    fn arrowheads(out: &mut Out, x: i32, y: i32, x2: i32, y2: i32, sx: i32, sy: i32, arrow: i32) {
        if arrow == 1 || arrow == 2 {
            fprintf!(out, "\\put({:3},{:3}){{\\vector({:2},{:2}){{0}}}}\n", x2, y2, sx, sy);
        }
        if arrow == -1 || arrow == 2 {
            fprintf!(out, "\\put({:3},{:3}){{\\vector({:2},{:2}){{0}}}}\n", x, y, -sx, -sy);
        }
    }

    /// `put_solidline`: a line of x-length `l` at LaTeX slope (sx, sy);
    /// `arrow` 0 none, 1 forward, −1 backward, 2 both.
    #[allow(clippy::too_many_arguments)]
    fn put_solidline(&self, out: &mut Out, ctx: &mut Ctx, x: i32, y: i32, sx: i32, sy: i32, l: f64, arrow: i32) {
        let (x2, y2, cosine) = line_end(x, y, sx, sy, l);
        if sx == 0 || sy == 0 || l * self.unitlength >= MIN_LEN {
            match arrow {
                0 => fprintf!(out, "\\put({:3},{:3}){{\\line({:2},{:2})", x, y, sx, sy),
                1 => fprintf!(out, "\\put({:3},{:3}){{\\vector({:2},{:2})", x, y, sx, sy),
                -1 => fprintf!(out, "\\put({:3},{:3}){{\\vector({:2},{:2})", x2, y2, -sx, -sy),
                2 => {
                    fprintf!(out, "\\put({:3},{:3}){{\\vector({:2},{:2}){{  0}}}}\n", x, y, -sx, -sy);
                    fprintf!(out, "\\put({:3},{:3}){{\\vector({:2},{:2})", x, y, sx, sy);
                }
                _ => {}
            }
            if l == l.floor() {
                fprintf!(out, "{{{:3.0}}}}}\n", Cf(l));
            } else {
                fprintf!(out, "{{{:7.3}}}}}\n", Cf(l));
            }
        } else {
            let n = (2.0 * (l / cosine) / (LDOT_DIAMETER / self.unitlength)) as i32;
            if n == 0 {
                return;
            }
            ctx.warn(format!("Line too short; will do {n} dots"));
            let mut dx = l / f64::from(n);
            if sx < 0 {
                dx = -dx;
            }
            let dy = dx * f64::from(sy) / f64::from(sx);
            fprintf!(
                out,
                "\\multiput({:3},{:3})({:.5},{:.5}){{{}}}{{\\makebox({:.4},{:.4}){{{}}}}}\n",
                x,
                y,
                Cf(dx),
                Cf(dy),
                n.wrapping_add(1),
                Cf(self.ldot_xoffset),
                Cf(self.ldot_yoffset),
                self.ldot_cmd
            );
            Self::arrowheads(out, x, y, x2, y2, sx, sy, arrow);
        }
    }

    /// `put_dashline`.
    #[allow(clippy::too_many_arguments)]
    fn put_dashline(
        &self,
        out: &mut Out,
        ctx: &mut Ctx,
        x: i32,
        y: i32,
        sx: i32,
        sy: i32,
        l: f64,
        arrow: i32,
        val: f64,
    ) {
        let (x2, y2, cosine) = line_end(x, y, sx, sy, l);
        // The number of dashes and gaps on the line, made odd.
        let mut nd = l / (val * self.dash_mag * cosine);
        let mut n = round((nd + 1.0) / 2.0).wrapping_mul(2).wrapping_sub(1);
        let mut dl = l / f64::from(n);
        if sx != 0 && sy != 0 && dl * self.unitlength < MIN_LEN {
            ctx.warn("Dash too small; using larger dash");
            dl = MIN_LEN / self.unitlength * cosine;
            nd = l / dl;
            n = round((nd + 1.0) / 2.0).wrapping_mul(2).wrapping_sub(1);
        }
        if 2.0 * dl >= l || (sx != 0 && sy != 0 && l * self.unitlength < MIN_LEN) {
            ctx.warn("Dashed line too short; drawing solid line");
            self.put_solidline(out, ctx, x, y, sx, sy, l, arrow);
            return;
        }
        let half = n / 2;
        let dg = (l - f64::from(half.wrapping_add(1)) * dl) / f64::from(half);
        let (dx, dy) = if sx != 0 {
            let dx = if sx < 0 { -(dl + dg) } else { dl + dg };
            (dx, dx * f64::from(sy) / f64::from(sx))
        } else {
            (0.0, if sy < 0 { -(dl + dg) } else { dl + dg })
        };
        fprintf!(
            out,
            "\\multiput({:3},{:3})({:.5},{:.5}){{{}}}{{\\line({:2},{:2}){{{:7.3}}}}}\n",
            x,
            y,
            Cf(dx),
            Cf(dy),
            half.wrapping_add(1),
            sx,
            sy,
            Cf(dl)
        );
        Self::arrowheads(out, x, y, x2, y2, sx, sy, arrow);
    }

    /// `put_dotline`.
    #[allow(clippy::too_many_arguments)]
    fn put_dotline(&self, out: &mut Out, mut x: i32, mut y: i32, sx: i32, sy: i32, l: f64, arrow: i32, val: f64) {
        let cosine = if sx != 0 {
            f64::from(sx.wrapping_abs()) / (f64::from(sx) * f64::from(sx) + f64::from(sy) * f64::from(sy)).sqrt()
        } else {
            1.0
        };
        let nd = l / (3.0 * val * cosine);
        let mut n = round(nd);
        if n == 0 {
            n = 1;
        }
        let (dx, dy) = if sx != 0 {
            let dx = if sx < 0 { -(l / f64::from(n)) } else { l / f64::from(n) };
            (dx, dx * f64::from(sy) / f64::from(sx))
        } else {
            let dy = l / f64::from(n);
            (0.0, if sy < 0 { -dy } else { dy })
        };
        if arrow == 1 || arrow == 2 {
            let (x2, y2, _) = line_end(x, y, sx, sy, l);
            fprintf!(out, "\\put({:3},{:3}){{\\vector({:2},{:2}){{0}}}}\n", x2, y2, sx, sy);
            n = n.wrapping_sub(1);
        }
        if arrow == -1 || arrow == 2 {
            fprintf!(out, "\\put({:3},{:3}){{\\vector({:2},{:2}){{0}}}}\n", x, y, -sx, -sy);
            x = round(f64::from(x) + dx);
            y = round(f64::from(y) + dy);
            n = n.wrapping_sub(1);
        }
        fprintf!(
            out,
            "\\multiput({:3},{:3})({:.5},{:.5}){{{}}}{{\\makebox({:.4},{:.4}){{{}}}}}\n",
            x,
            y,
            Cf(dx),
            Cf(dy),
            n.wrapping_add(1),
            Cf(self.dot_xoffset),
            Cf(self.dot_yoffset),
            self.dot_cmd
        );
    }

    /// `single_line`: a line between two Fig points at the nearest LaTeX
    /// slope.
    #[allow(clippy::too_many_arguments)]
    fn single_line(&self, out: &mut Out, ctx: &mut Ctx, p: Point, q: Point, arrow: i32, style: i32, val: f64) {
        let (x1, y1) = trans(p.x, p.y);
        let (x2, y2) = trans(q.x, q.y);
        let dx = x2.wrapping_sub(x1);
        let dy = y2.wrapping_sub(y1);
        let (sx, sy) = get_slope(dx, dy, arrow != 0);
        let adx = f64::from(dx).abs();
        let ady = f64::from(dy).abs();
        let l = if sx == 0 {
            ady
        } else {
            let m = f64::from(sy).abs() / f64::from(sx).abs();
            let l = (adx + m * ady) / (1.0 + m * m);
            let deviation = (l - adx).abs() + (m * l - ady).abs();
            if deviation > TOLERANCE {
                ctx.warn(format!(
                    "Not a LaTeX slope ({dx}, {dy}), deviation {deviation:.1} pixels"
                ));
            }
            l
        };
        let l = round4(l);
        match style {
            SOLID_LINE => self.put_solidline(out, ctx, x1, y1, sx, sy, l, arrow),
            DASH_LINE => self.put_dashline(out, ctx, x1, y1, sx, sy, l, arrow, val),
            DOTTED_LINE => self.put_dotline(out, x1, y1, sx, sy, l, arrow, val),
            _ => ctx.warn("Unknown line style -- approximating with solid lines"),
        }
    }

    /// `put_arc_box`: a box with rounded corners, always solid.
    #[allow(clippy::too_many_arguments)]
    fn put_arc_box(&self, out: &mut Out, ctx: &mut Ctx, b: [i32; 4], radius: i32, style: i32) {
        let [llx, lly, urx, ury] = b;
        let radius2 = radius.wrapping_mul(2);
        let swidth = f64::from(urx.wrapping_sub(llx).wrapping_sub(radius2));
        let sheight = f64::from(ury.wrapping_sub(lly).wrapping_sub(radius2));
        match style {
            SOLID_LINE => {}
            DASH_LINE => ctx.warn("Dashed arc boxes approximated with solid arcs"),
            DOTTED_LINE => ctx.warn("Dotted arc boxes approximated with solid arcs"),
            _ => ctx.warn("Unknown line style -- approximating with solid lines"),
        }
        let corners = [
            (llx.wrapping_add(radius), lly.wrapping_add(radius), "bl"),
            (llx.wrapping_add(radius), ury.wrapping_sub(radius), "tl"),
            (urx.wrapping_sub(radius), lly.wrapping_add(radius), "br"),
            (urx.wrapping_sub(radius), ury.wrapping_sub(radius), "tr"),
        ];
        for (x, y, q) in corners {
            fprintf!(
                out,
                "\\put({:3},{:3}){{\\oval({:3},{:3})[{}]}}\n",
                x,
                y,
                radius2,
                radius2,
                q
            );
        }
        self.put_solidline(out, ctx, llx.wrapping_add(radius), lly, 1, 0, swidth, 0);
        self.put_solidline(out, ctx, llx.wrapping_add(radius), ury, 1, 0, swidth, 0);
        self.put_solidline(out, ctx, llx, lly.wrapping_add(radius), 0, 1, sheight, 0);
        self.put_solidline(out, ctx, urx, lly.wrapping_add(radius), 0, 1, sheight, 0);
    }

    /// `put_box`.
    fn put_box(&self, out: &mut Out, ctx: &mut Ctx, b: [i32; 4], style: i32, val: f64) {
        let [llx, lly, urx, ury] = b;
        let (w, h) = (urx.wrapping_sub(llx), ury.wrapping_sub(lly));
        match style {
            SOLID_LINE => fprintf!(out, "\\put({:3},{:3}){{\\framebox({},{}){{}}}}\n", llx, lly, w, h),
            DASH_LINE => {
                let dlen = round(val * self.dash_mag);
                fprintf!(
                    out,
                    "\\put({:3},{:3}){{\\dashbox{{{}}}({},{}){{}}}}\n",
                    llx,
                    lly,
                    dlen,
                    w,
                    h
                );
            }
            DOTTED_LINE => {
                self.put_dotline(out, llx, lly, 1, 0, f64::from(w), 0, val);
                self.put_dotline(out, llx, ury, 1, 0, f64::from(w), 0, val);
                self.put_dotline(out, llx, lly, 0, 1, f64::from(h), 0, val);
                self.put_dotline(out, urx, lly, 0, 1, f64::from(h), 0, val);
            }
            _ => ctx.warn("Unknown line style -- approximating with solid lines"),
        }
    }

    /// `put_quarter`: a quarter oval from p1 to p2 in quadrant q.
    fn put_quarter(out: &mut Out, p1: Point, p2: Point, q: usize) {
        let dx = p1.x.wrapping_sub(p2.x).wrapping_abs().wrapping_mul(2);
        let dy = p1.y.wrapping_sub(p2.y).wrapping_abs().wrapping_mul(2);
        if dx == 0 && dy == 0 {
            return;
        }
        let (px, py, opt) = match q {
            0 => (p1.x.min(p2.x), p1.y.min(p2.y), "tr"),
            1 => (p1.x.max(p2.x), p1.y.min(p2.y), "tl"),
            2 => (p1.x.max(p2.x), p1.y.max(p2.y), "bl"),
            _ => (p1.x.min(p2.x), p1.y.max(p2.y), "br"),
        };
        fprintf!(out, "\\put({:3},{:3}){{\\oval({:3},{:3})[{}]}}\n", px, py, dx, dy, opt);
    }
}

impl Driver for Latex {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        let a = arg.unwrap_or("");
        match opt {
            'a' => ctx.warn("warning: latex option -a obsolete"),
            'b' => {
                // sscanf("%d"): unchanged when there is no number.
                let (v, n) = strtol(a);
                if n > 0 {
                    self.border_margin = v;
                }
            }
            'd' => self.dash_mag = crate::gendev::atof(a),
            'F' => self.select_fontname = false,
            'f' => default_font_option(&mut self.fonts, ctx, a),
            'l' => self.thick_width = strtol(a).0,
            'v' => self.verbose = true,
            'G' | 'L' => {}
            _ => return Err(format!("Argument -{opt} unknown to latex driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        set_default_size(&mut self.fonts, ctx);
        self.unitlength = ctx.mag / ctx.ppi;
        self.dash_mag /= self.unitlength * 80.0;
        self.border_margin = (f64::from(self.border_margin) / (self.unitlength * 72.0)) as i32;

        // The border margin around the figure.
        let bm = self.border_margin;
        let (llx, lly) = trans(ctx.llx.wrapping_sub(bm), ctx.lly.wrapping_sub(bm));
        let (urx, ury) = trans(ctx.urx.wrapping_add(bm), ctx.ury.wrapping_add(bm));
        ctx.llx = llx.min(urx);
        ctx.urx = llx.max(urx);
        ctx.lly = lly.min(ury);
        ctx.ury = lly.max(ury);

        if !objects.comments.is_empty() {
            out.puts("%\n");
            print_comments(out, "% ", &objects.comments, "");
            out.puts("%\n");
        }
        fprintf!(
            out,
            "\\setlength{{\\unitlength}}{{{}sp}}%\n",
            round(4736286.72 * self.unitlength)
        );
        fprintf!(
            out,
            "\\begin{{picture}}({},{})({},{})\n",
            ctx.urx.wrapping_sub(ctx.llx),
            ctx.ury.wrapping_sub(ctx.lly),
            ctx.llx,
            ctx.lly
        );
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        const AD1: [&str; 4] = [" 0,-1", " 1, 0", " 0, 1", "-1, 0"];
        const AD2: [&str; 4] = ["-1, 0", " 0,-1", " 1, 0", " 0, 1"];
        if self.verbose {
            out.puts("%\n% Fig ARC object\n%\n");
        }
        print_comments(out, "% ", &a.comments, "");
        self.set_linewidth(out, ctx, a.thickness);
        self.color.set_color(out, ctx, a.pen_color);
        match a.style {
            SOLID_LINE => {}
            DASH_LINE => ctx.warn("Dashed arcs not supported"),
            DOTTED_LINE => ctx.warn("Dotted arcs not supported"),
            _ => ctx.warn("Unknown line style -- approximating with solid lines"),
        }
        let (p1, p2, p1_arrow, p2_arrow) = if a.direction == 1 {
            (a.point[0], a.point[2], a.back_arrow.is_some(), a.for_arrow.is_some())
        } else {
            (a.point[2], a.point[0], a.for_arrow.is_some(), a.back_arrow.is_some())
        };
        let (x, y) = trans(p1.x, p1.y);
        let mut p1 = Point::new(x, y);
        let (x, y) = trans(p2.x, p2.y);
        let p2 = Point::new(x, y);
        let cx = a.center.x + 1.0;
        let cy = f64::from(TOP) - a.center.y - 1.0;
        // Vectors and angles from the centre to p1, p2.
        let v1x = f64::from(p1.x) - cx;
        let v1y = f64::from(p1.y) - cy;
        let v2x = f64::from(p2.x) - cx;
        let v2y = f64::from(p2.y) - cy;
        let mut angle1 = v1y.atan2(v1x) * RAD2DEG;
        let mut angle2 = v2y.atan2(v2x) * RAD2DEG;
        if angle1 < 0.0 {
            angle1 += 360.0;
        }
        if angle2 < 0.0 {
            angle2 += 360.0;
        }
        let r = (v1x * v1x + v1y * v1y).sqrt();
        // Where the arc meets the axes through the centre.
        let pq = [
            Point::new(round(cx), round(cy + r)),
            Point::new(round(cx - r), round(cy)),
            Point::new(round(cx), round(cy - r)),
            Point::new(round(cx + r), round(cy)),
        ];
        // The quadrants of p1 and p2 (an angle of 360 is quadrant 0; C
        // indexes past its tables there).
        let q1 = (angle1 / 90.0) as i32;
        let q2 = (angle2 / 90.0) as i32;
        if (angle1 - 90.0 * f64::from(q1)).abs() > ARC_TOLERANCE
            || (angle2 - 90.0 * f64::from(q2)).abs() > ARC_TOLERANCE
        {
            ctx.warn("Approximating arc by ovals");
        }
        let mut q1 = q1.rem_euclid(4) as usize;
        let q2 = q2.rem_euclid(4) as usize;
        if a.type_ == T_OPEN_ARC && a.thickness != 0 && p1_arrow {
            fprintf!(
                out,
                "\\put({:3},{:3}){{\\vector({}){{0}}}}\n",
                p1.x,
                p1.y,
                AD1.get(q1).copied().unwrap_or("")
            );
        }
        while q1 != q2 {
            let pq1 = pq.get(q1).copied().unwrap_or_default();
            Self::put_quarter(out, p1, pq1, q1);
            p1 = pq1;
            q1 = (q1 + 1) % 4;
        }
        Self::put_quarter(out, p1, p2, q1);
        if a.type_ == T_OPEN_ARC && a.thickness != 0 && p2_arrow {
            fprintf!(
                out,
                "\\put({:3},{:3}){{\\vector({}){{0}}}}\n",
                p2.x,
                p2.y,
                AD2.get(q2).copied().unwrap_or("")
            );
        }
        if a.fill_style != UNFILLED {
            ctx.warn("Arc area fill not implemented");
        }
        self.color.reset_color(out, a.pen_color);
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        if self.verbose {
            out.puts("%\n% Fig ELLIPSE\n%\n");
        }
        print_comments(out, "% ", &e.comments, "");
        self.color.set_color(out, ctx, e.pen_color);
        self.set_linewidth(out, ctx, e.thickness);
        match e.style {
            SOLID_LINE => {}
            DASH_LINE => ctx.warn("Dashed circles and elipses not supported"),
            DOTTED_LINE => ctx.warn("Dotted circles and elipses not supported"),
            _ => ctx.warn("Unknown line style -- approximating with solid lines"),
        }
        let (x, y) = trans(e.center.x, e.center.y);
        if (e.type_ == T_CIRCLE_BY_RAD || e.type_ == T_CIRCLE_BY_DIA)
            && f64::from(e.radiuses.x) * self.unitlength <= MAXCIRCLERAD
        {
            let d = e.radiuses.x.wrapping_mul(2);
            if e.fill_style == BLACK_FILL {
                fprintf!(out, "\\put({:3},{:3}){{\\circle*{{{}}}}}\n", x, y, d);
            } else {
                fprintf!(out, "\\put({:3},{:3}){{\\circle{{{}}}}}\n", x, y, d);
                if e.fill_style != UNFILLED {
                    ctx.warn("Circle area fill not implemented");
                }
            }
        } else {
            let dx = e.radiuses.x.wrapping_mul(2);
            let dy = e.radiuses.y.wrapping_mul(2);
            fprintf!(out, "\\put({:3},{:3}){{\\oval({},{})}}\n", x, y, dx, dy);
            if e.fill_style != UNFILLED {
                ctx.warn("Ellipse area fill not implemented");
            }
        }
        self.color.reset_color(out, e.pen_color);
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        if self.verbose {
            out.puts("%\n% Fig POLYLINE object\n%\n");
        }
        print_comments(out, "% ", &l.comments, "");
        self.set_linewidth(out, ctx, l.thickness);
        self.color.set_color(out, ctx, l.pen_color);

        let Some((&first, rest)) = l.points.split_first() else {
            // The reader gives every line a point; C would crash here.
            self.color.reset_color(out, l.pen_color);
            return Ok(());
        };
        if rest.is_empty() {
            let (x, y) = trans(first.x, first.y);
            fprintf!(
                out,
                "\\put({:3},{:3}){{\\makebox({:.4},{:.4}){{{}}}}}\n",
                x,
                y,
                Cf(self.dot_xoffset),
                Cf(self.dot_yoffset),
                self.dot_cmd
            );
            self.color.reset_color(out, l.pen_color);
            return Ok(());
        }

        // A box, perhaps with rounded corners.
        if l.type_ == T_BOX || l.type_ == T_ARC_BOX {
            let (x, y) = trans(first.x, first.y);
            let (mut llx, mut lly, mut urx, mut ury) = (x, y, x, y);
            for q in rest {
                let (x, y) = trans(q.x, q.y);
                llx = llx.min(x);
                lly = lly.min(y);
                urx = urx.max(x);
                ury = ury.max(y);
            }
            if l.type_ == T_ARC_BOX {
                self.put_arc_box(out, ctx, [llx, lly, urx, ury], l.radius, l.style);
            } else {
                self.put_box(out, ctx, [llx, lly, urx, ury], l.style, l.style_val);
            }
            self.color.reset_color(out, l.pen_color);
            return Ok(());
        }

        let n = l.points.len();
        for (i, seg) in l.points.windows(2).enumerate() {
            let (Some(&p), Some(&q)) = (seg.first(), seg.get(1)) else {
                continue;
            };
            let mut arrow = 0;
            if l.for_arrow.is_some() && i + 2 == n {
                arrow = 1;
            }
            if l.back_arrow.is_some() && i == 0 {
                arrow = if arrow != 0 { 2 } else { -1 };
            }
            self.single_line(out, ctx, p, q, arrow, l.style, l.style_val);
        }
        if l.fill_style != UNFILLED {
            ctx.warn("Line area fill not implemented");
        }
        self.color.reset_color(out, l.pen_color);
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, _out: &mut Out, _s: &Spline) -> Result<(), String> {
        ctx.warn("Can't generate spline; omitting object");
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String> {
        if self.verbose {
            out.puts("%\n% Fig TEXT object\n%\n");
        }
        print_comments(out, "% ", &t.comments, "");
        let (x, y) = trans(t.base_x, t.base_y);
        let tpos = match t.type_ {
            T_LEFT_JUSTIFIED | DEFAULT => "[lb]",
            T_CENTER_JUSTIFIED => "[b]",
            T_RIGHT_JUSTIFIED => "[rb]",
            _ => {
                ctx.warn("Text incorrectly positioned");
                "[lb]"
            }
        };
        fprintf!(out, "\\put({:3},{:3}){{", x, y);
        if t.angle != 0.0 {
            fprintf!(
                out,
                "\\rotatebox{{{:.1}}}{{",
                Cf(t.angle * 180.0 / std::f64::consts::PI)
            );
        }
        // \smash puts the text on the baseline.
        fprintf!(out, "\\makebox(0,0){}{{\\smash{{", tpos);
        let mut t = t.clone();
        self.fonts
            .select_font(out, ctx, &mut t, true, self.select_fontname, false);
        self.color.set_color(out, ctx, t.color);
        put_string(out, c_string(&t.cstring), t.font, t.special_text());
        self.color.reset_color(out, t.color);
        if t.angle != 0.0 {
            out.puts("}");
        }
        out.puts("}}}\n");
        Ok(())
    }

    fn end(&mut self, _ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        out.puts("\\end{picture}%\n");
        Ok(0)
    }

    fn text_include(&self) -> bool {
        false
    }
}

/// The string as C sees it: up to the first NUL.
pub(super) fn c_string(s: &str) -> &str {
    s.split('\0').next().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slopes() {
        assert_eq!(get_slope(0, -5, false), (0, -1));
        assert_eq!(get_slope(300, -525, true), (1, -2));
        assert_eq!(get_slope(-600, 100, false), (-6, 1));
        assert_eq!(format!("{:7.3}|{:.1}", Cf(f64::NAN), Cf(1.25)), "    nan|1.2");
    }
}
