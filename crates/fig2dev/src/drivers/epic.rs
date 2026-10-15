//! fig2dev's `epic`, `eepic` and `eepicemu` drivers (`dev/genepic.c`):
//! LaTeX picture environments drawn with the epic and eepic packages.
//!
//! Coordinates are Fig units with y turned upwards (`convertCS`); the unit
//! length is one Fig unit times the magnification. Polylines are written
//! with `\drawline` (epic) or `\path` (eepic), splines as `\spline`
//! (eepic) or as subdivided curves, arcs with `\arc` (eepic) or as point
//! lists, arrowheads as small polylines, fills as `\shade`, `\whiten` or
//! `\blacken` with `\texture` patterns.
//!
//! genepic.c's settings that no option changes (`UseBox`, `DashType`,
//! `LowerLeftX`, …) keep their only value here, so the code they guard
//! (`\framebox` boxes, economic dashes) is left out.
//
// Ported from fig2dev dev/genepic.c (xfig licence, see LICENSE-xfig).

use std::f64::consts::PI;

use crate::gendev::{atof, print_comments, strtol, Ctx, DEFAULT_FONT_SIZE};
use crate::object::{
    Arc, Arrow, Compound, Ellipse, Line, Point, Spline, Text, BLACK_COLOR, BLACK_FILL, DASH_LINE, DEFAULT, DOTTED_LINE,
    NUMPATTERNS, NUMSHADES, NUMTINTS, SOLID_LINE, T_ARC_BOX, T_BOX, T_CENTER_JUSTIFIED, T_LEFT_JUSTIFIED, T_OPEN_ARC,
    T_RIGHT_JUSTIFIED, UNFILLED, WHITE_COLOR, WHITE_FILL,
};
use crate::util::texfonts::{put_string, TexFonts, NUM_TEXFONTS};
use crate::{fprintf, round, Driver, Out};

/// 10.5 in × 80 dpi; any value would do (`TopCoord`).
const TOP_COORD: i32 = 840;
/// Points per output line.
const PT_PER_LINE: usize = 3;
const THIN_LINES: i32 = 0;
const THICK_LINES: i32 = 1;
/// The shading used instead of hatch patterns.
const DEFAULT_SHADING: usize = 8;
/// Segment length of curves drawn as point lists, inches (`SegLen`).
const SEG_LEN: f64 = 0.0625;
const TOP_MARGIN: i32 = 5;
const BOTTOM_MARGIN: i32 = 10;
/// At most this many points of an arc drawn as a point list (C writes as
/// many as the arithmetic asks for).
const MAX_ARC_POINTS: i64 = 1 << 20;
/// Deepest subdivision of a curve; finite coordinates never get there.
const MAX_DEPTH: u32 = 80;

const PREAMBLE: &str =
    "\\documentclass{minimal}\n\\usepackage{epic,eepic}\\pagestyle{empty}\n\\begin{document}\\begin{center}\n";
const POSTAMBLE: &str = "\\end{center}\n\\end{document}\n";

/// `-S 8..=12`: magnification and font size (`ScaleTbl`).
const SCALE_TBL: [(f64, f64); 5] = [(0.6667, 8.0), (0.75, 9.0), (0.8333, 10.0), (0.9167, 11.0), (1.0, 12.0)];

/// The dialects, in the order of `Tlangkw`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TexLang {
    Epic,
    EEpicEmu,
    EEpic,
    /// None of the three names (C: `TeXLang = 3`).
    Unknown,
}

const TLANGKW: [&str; 3] = ["Epic", "EEpicemu", "EEpic"];

impl TexLang {
    /// `strcasecmp` against `Tlangkw`.
    fn from_name(name: &str) -> TexLang {
        match TLANGKW.iter().position(|k| k.eq_ignore_ascii_case(name)) {
            Some(0) => TexLang::Epic,
            Some(1) => TexLang::EEpicEmu,
            Some(2) => TexLang::EEpic,
            _ => TexLang::Unknown,
        }
    }

    fn name(self) -> &'static str {
        match self {
            TexLang::Epic => TLANGKW[0],
            TexLang::EEpicEmu => TLANGKW[1],
            TexLang::EEpic => TLANGKW[2],
            TexLang::Unknown => "(null)",
        }
    }
}

/// The driver for `lang` (`epic`, `eepic` or `eepicemu`).
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    Some(Box::new(Epic::new(lang)))
}

/// A point with `double` coordinates (`FPoint`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct FPoint {
    x: f64,
    y: f64,
}

impl FPoint {
    fn of(p: Point) -> FPoint {
        FPoint {
            x: f64::from(p.x),
            y: f64::from(p.y),
        }
    }
}

/// genepic.c's file-level state.
#[derive(Clone, Debug)]
pub struct Epic {
    tex_lang: TexLang,
    fonts: TexFonts,
    threshold: f64,
    linew_spec: bool,
    select_fontname: bool,
    cur_width: i32,
    line_style: i32,
    llx: i32,
    lly: i32,
    ln_cmd: &'static str,
    max_circle_radius: i32,
    dash_len: f64,
    dot_dist: i32,
    page_mode: bool,
    pattern_type: i32,
    pattern_color: i32,
    verbose: bool,
    line_thick: i32,
    /// `\ellipse` (eepic) or `\oval` (epic).
    oval: bool,
    var_width: bool,
    dash_stretch: i32,
    arrow_scale: f64,
    allow_rotated_text: bool,
    dash_scale: f64,
    out_line: bool,
    /// fig2dev exited (`exit(1)`): the output so far stands, nothing more
    /// is written, and the exit status is 1.
    failed: bool,
}

impl Epic {
    pub fn new(lang: &str) -> Epic {
        Epic {
            tex_lang: TexLang::from_name(lang),
            fonts: TexFonts::new(),
            threshold: 0.0,
            linew_spec: false,
            select_fontname: true,
            cur_width: 0,
            line_style: SOLID_LINE,
            llx: 0,
            lly: 0,
            ln_cmd: "path",
            max_circle_radius: 0,
            dash_len: 0.0,
            dot_dist: 5,
            page_mode: false,
            pattern_type: UNFILLED,
            pattern_color: WHITE_COLOR,
            verbose: false,
            line_thick: 0,
            oval: false,
            var_width: false,
            dash_stretch: 30,
            arrow_scale: 1.0,
            allow_rotated_text: false,
            dash_scale: 0.0,
            out_line: false,
            failed: false,
        }
    }

    /// `convertCS`: y upwards, relative to the lower left corner.
    fn convert(&self, p: Point) -> Point {
        Point::new(
            p.x.wrapping_sub(self.llx),
            TOP_COORD.wrapping_sub(p.y).wrapping_sub(self.lly),
        )
    }

    /// `fconvertCS`.
    fn fconvert(&self, x: f64, y: f64) -> FPoint {
        FPoint {
            x: x - f64::from(self.llx),
            y: f64::from(TOP_COORD) - y - f64::from(self.lly),
        }
    }

    fn set_linewidth(&mut self, out: &mut Out, ctx: &Ctx, w: i32) {
        if w < 0 {
            return;
        }
        let old_width = self.cur_width;
        self.cur_width = if self.var_width {
            w
        } else if w >= self.line_thick {
            THICK_LINES
        } else {
            THIN_LINES
        };
        if old_width != self.cur_width {
            if self.cur_width == THIN_LINES {
                out.puts("\\thinlines\n");
            } else if self.var_width {
                fprintf!(
                    out,
                    "\\allinethickness{{{}pt}}%\n",
                    cf(f64::from(w) * 80.0 / ctx.ppi, 4, 3)
                );
            } else {
                out.puts("\\thicklines\n");
            }
        }
    }

    fn set_style(&mut self, style: i32, dash_len: f64) {
        self.line_style = style;
        if style == DASH_LINE {
            self.dash_len = f64::from(round(dash_len * self.dash_scale));
        } else if style == DOTTED_LINE {
            self.dot_dist = round(dash_len * self.dash_scale);
        }
    }

    /// `set_pattern`: `\texture` for the shades of a colour and the
    /// patterns.
    fn set_pattern(&mut self, out: &mut Out, type_: i32, color: i32) {
        if type_ <= WHITE_FILL
            || (BLACK_FILL..NUMSHADES + NUMTINTS).contains(&type_)
            || type_ >= NUMSHADES + NUMTINTS + NUMPATTERNS
        {
            return;
        }
        if type_ == self.pattern_type && color == self.pattern_color {
            return;
        }
        self.pattern_type = type_;
        self.pattern_color = color;
        let i = if type_ >= NUMSHADES + NUMTINTS {
            DEFAULT_SHADING
        } else if color == BLACK_COLOR || color == DEFAULT {
            usize::try_from(type_).unwrap_or(0)
        } else {
            usize::try_from(NUMSHADES - type_ - 1).unwrap_or(0)
        };
        let Some(pat) = PATTERNS.get(i) else {
            return;
        };
        out.puts("\\texture{");
        for (row, chunk) in pat.chunks(8).enumerate() {
            for v in chunk {
                fprintf!(out, "{:x} ", v);
            }
            out.puts(if row < 3 { "\n\t" } else { "}\n" });
        }
    }

    /// The arrowhead of `a` with its own height and width, from `p1` to
    /// `p2` (`draw_arrow_head`).
    fn draw_arrow_head(&mut self, out: &mut Out, ctx: &Ctx, p1: Point, p2: Point, a: &Arrow) {
        self.fdraw_arrow_head(out, ctx, FPoint::of(p1), FPoint::of(p2), (a.ht, a.wid), a);
    }

    /// `fdraw_arrow_head`: the arrowhead of `a` (type, style, thickness)
    /// with height and width `size`, pointing from `p1` to `p2`.
    fn fdraw_arrow_head(&mut self, out: &mut Out, ctx: &Ctx, p1: FPoint, p2: FPoint, size: (f64, f64), a: &Arrow) {
        let arrowht = size.0 / self.arrow_scale;
        let arrowwid = size.1 / self.arrow_scale;
        let (x1, y1, x2, y2) = (p1.x, p1.y, p2.x, p2.y);
        let dx = x2 - x1;
        let dy = y1 - y2;
        let l = (dx * dx + dy * dy).sqrt();
        if l == 0.0 {
            return;
        }
        let sina = dy / l;
        let cosa = dx / l;
        let xb = x2 * cosa - y2 * sina;
        let yb = x2 * sina + y2 * cosa;
        let x = xb - arrowht;
        let y = yb - arrowwid / 2.0;
        let xc = x * cosa + y * sina;
        let yc = -x * sina + y * cosa;
        let y = yb + arrowwid / 2.0;
        let xd = x * cosa + y * sina;
        let yd = -x * sina + y * cosa;

        if self.verbose {
            out.puts("%\n% arrow head\n%\n");
        }
        if a.type_ != 0 {
            out.puts(if a.style == 1 { "\\blacken" } else { "\\whiten" });
        }
        self.set_linewidth(out, ctx, a.thickness as i32);
        let mut pts = vec![(xc, yc), (x2, y2), (xd, yd)];
        match a.type_ {
            0 => {}
            1 => pts.push((xc, yc)),
            2 | 3 => {
                let k = if a.type_ == 2 { 0.7 } else { 1.3 };
                pts.push((x2 - (x2 - x1) / l * arrowht * k, y2 - (y2 - y1) / l * arrowht * k));
                pts.push((xc, yc));
            }
            _ => return,
        }
        fprintf!(out, "\\{}", self.ln_cmd);
        for (x, y) in pts {
            fprintf!(out, "({},{})", cf(x, 4, 3), cf(y, 4, 3));
        }
        out.puts("\n");
    }

    /// `chaikin_curve`.
    fn chaikin_curve(&self, out: &mut Out, c: [f64; 6], depth: u32) {
        let [a1, b1, a2, b2, a3, b3] = c;
        if ((a1 - a3).abs() < self.threshold && (b1 - b3).abs() < self.threshold) || depth > MAX_DEPTH {
            fprintf!(out, "\t({},{})\n", cf(a3, 0, 3), cf(b3, 0, 3));
        } else {
            let xm1 = (a1 + a2) / 2.0;
            let ym1 = (b1 + b2) / 2.0;
            let xm2 = (a2 + a3) / 2.0;
            let ym2 = (b2 + b3) / 2.0;
            let xmid = (xm1 + xm2) / 2.0;
            let ymid = (ym1 + ym2) / 2.0;
            self.chaikin_curve(out, [a1, b1, xm1, ym1, xmid, ymid], depth + 1);
            self.chaikin_curve(out, [xmid, ymid, xm2, ym2, a3, b3], depth + 1);
        }
    }

    /// `quadratic_spline`.
    fn quadratic_spline(&self, out: &mut Out, c: [f64; 8], depth: u32) {
        let [x1, y1, a2, b2, a3, b3, x4, y4] = c;
        let xmid = (a2 + a3) / 2.0;
        let ymid = (b2 + b3) / 2.0;
        let t = self.threshold;
        if ((x1 - xmid).abs() < t && (y1 - ymid).abs() < t) || depth > MAX_DEPTH {
            fprintf!(out, "\t({},{})\n", cf(xmid, 0, 3), cf(ymid, 0, 3));
        } else {
            let c = [
                x1,
                y1,
                (x1 + a2) / 2.0,
                (y1 + b2) / 2.0,
                (3.0 * a2 + a3) / 4.0,
                (3.0 * b2 + b3) / 4.0,
                xmid,
                ymid,
            ];
            self.quadratic_spline(out, c, depth + 1);
        }
        if ((xmid - x4).abs() < t && (ymid - y4).abs() < t) || depth > MAX_DEPTH {
            fprintf!(out, "\t({},{})\n", cf(x4, 0, 3), cf(y4, 0, 3));
        } else {
            let c = [
                xmid,
                ymid,
                (a2 + 3.0 * a3) / 4.0,
                (b2 + 3.0 * b3) / 4.0,
                (a3 + x4) / 2.0,
                (b3 + y4) / 2.0,
                x4,
                y4,
            ];
            self.quadratic_spline(out, c, depth + 1);
        }
    }

    /// `bezier_spline`.
    fn bezier_spline(&self, out: &mut Out, c: [f64; 8], depth: u32) {
        let [x0, y0, a1, b1, a2, b2, x3, y3] = c;
        if ((x0 - x3).abs() < self.threshold && (y0 - y3).abs() < self.threshold) || depth > MAX_DEPTH {
            fprintf!(out, "\t({},{})\n", cf(x3, 0, 3), cf(y3, 0, 3));
        } else {
            let tx = (a1 + a2) / 2.0;
            let ty = (b1 + b2) / 2.0;
            let sx1 = (x0 + a1) / 2.0;
            let sy1 = (y0 + b1) / 2.0;
            let sx2 = (sx1 + tx) / 2.0;
            let sy2 = (sy1 + ty) / 2.0;
            let tx2 = (a2 + x3) / 2.0;
            let ty2 = (b2 + y3) / 2.0;
            let tx1 = (tx2 + tx) / 2.0;
            let ty1 = (ty2 + ty) / 2.0;
            let xmid = (sx2 + tx1) / 2.0;
            let ymid = (sy2 + ty1) / 2.0;
            self.bezier_spline(out, [x0, y0, sx1, sy1, sx2, sy2, xmid, ymid], depth + 1);
            self.bezier_spline(out, [xmid, ymid, tx1, ty1, tx2, ty2, x3, y3], depth + 1);
        }
    }

    /// `drawarc`: an arc as a list of points.
    fn drawarc(&self, out: &mut Out, ctr: FPoint, r: f64, th1: f64, angle: f64) {
        let d = angle * r / self.threshold;
        let division: i64 = if d.is_finite() { d as i64 } else { 0 };
        let delta = angle / division as f64;
        let n = division.clamp(-1, MAX_ARC_POINTS) + 1;
        let mut th = th1;
        let mut pt_count = 0;
        for _ in 0..n {
            pt_count += 1;
            if pt_count > PT_PER_LINE {
                out.puts("\n\t");
                pt_count = 1;
            }
            fprintf!(
                out,
                "({},{})",
                cf(ctr.x + th.cos() * r, 0, 3),
                cf(ctr.y + th.sin() * r, 0, 3)
            );
            th += delta;
        }
        out.puts("\n");
    }

    /// `genepic_open_spline`: an approximated open spline.
    fn open_spline(&mut self, out: &mut Out, ctx: &Ctx, s: &Spline) {
        let pts: Vec<Point> = s.points.iter().map(|&p| self.convert(p)).collect();
        let (Some(&p), Some(&q)) = (pts.first(), pts.get(1)) else {
            return;
        };
        if let Some(a) = &s.back_arrow {
            self.draw_arrow_head(out, ctx, q, p, a);
            if self.verbose {
                out.puts("%\n");
            }
        }
        if pts.len() == 2 {
            // No forward arrowhead here, as in C.
            fprintf!(out, "\\{}({},{})({},{})\n", self.ln_cmd, p.x, p.y, q.x, q.y);
            return;
        }
        if matches!(self.tex_lang, TexLang::EEpic | TexLang::EEpicEmu) {
            fprintf!(out, "\\spline({},{})\n", p.x, p.y);
            put_points(out, &pts[1..], 1);
        } else {
            fprintf!(out, "\\{}({},{})\n", self.ln_cmd, p.x, p.y);
            let f: Vec<FPoint> = pts.iter().map(|&p| FPoint::of(p)).collect();
            let mut first = FPoint::of(p);
            for (i, w) in f.windows(2).enumerate().skip(1) {
                let (q, r) = (w[0], w[1]);
                if i + 2 == f.len() {
                    self.chaikin_curve(out, [first.x, first.y, q.x, q.y, r.x, r.y], 0);
                } else {
                    let mid = FPoint {
                        x: (q.x + r.x) / 2.0,
                        y: (q.y + r.y) / 2.0,
                    };
                    self.chaikin_curve(out, [first.x, first.y, q.x, q.y, mid.x, mid.y], 0);
                    first = mid;
                }
            }
            out.puts("\n");
        }
        if let (Some(a), [.., p, q]) = (&s.for_arrow, pts.as_slice()) {
            self.draw_arrow_head(out, ctx, *p, *q, a);
            if self.verbose {
                out.puts("%\n");
            }
        }
    }

    /// `genepic_closed_spline`: an approximated closed spline.
    fn closed_spline(&mut self, out: &mut Out, s: &Spline) {
        let pts: Vec<FPoint> = s.points.iter().map(|&p| FPoint::of(self.convert(p))).collect();
        let (Some(&a), Some(&b)) = (pts.first(), pts.get(1)) else {
            return;
        };
        let (mut x1, mut y1, mut x2, mut y2) = (a.x, a.y, b.x, b.y);
        let mut cx1 = (x1 + x2) / 2.0;
        let mut cy1 = (y1 + y2) / 2.0;
        let mut cx2 = (x1 + 3.0 * x2) / 4.0;
        let mut cy2 = (y1 + 3.0 * y2) / 4.0;
        for p in pts.iter().skip(2).chain(std::iter::once(&b)) {
            fprintf!(out, "\\{}({},{})", self.ln_cmd, cf(cx1, 0, 3), cf(cy1, 0, 3));
            x1 = x2;
            y1 = y2;
            x2 = p.x;
            y2 = p.y;
            let cx3 = (3.0 * x1 + x2) / 4.0;
            let cy3 = (3.0 * y1 + y2) / 4.0;
            let cx4 = (x1 + x2) / 2.0;
            let cy4 = (y1 + y2) / 2.0;
            self.quadratic_spline(out, [cx1, cy1, cx2, cy2, cx3, cy3, cx4, cy4], 0);
            out.puts("\n");
            cx1 = cx4;
            cy1 = cy4;
            cx2 = (x1 + 3.0 * x2) / 4.0;
            cy2 = (y1 + 3.0 * y2) / 4.0;
        }
    }

    /// `genepic_itp_spline`: an interpolated spline (files before 3.2)
    /// as Bézier segments.
    fn itp_spline(&mut self, out: &mut Out, ctx: &Ctx, s: &Spline) {
        let pts: Vec<Point> = s.points.iter().map(|&p| self.convert(p)).collect();
        let control = |i: usize| s.controls.get(i).copied().unwrap_or_default();
        let Some(&first) = pts.first() else {
            return;
        };
        let cp1 = control(0);
        let mut pt1r = self.fconvert(cp1.rx, cp1.ry);
        if let Some(a) = &s.back_arrow {
            self.fdraw_arrow_head(out, ctx, pt1r, FPoint::of(first), (a.ht, a.wid), a);
            if self.verbose {
                out.puts("%\n");
            }
        }
        let mut p1 = first;
        let mut pt2l = FPoint::default();
        for (i, &p2) in pts.iter().enumerate().skip(1) {
            fprintf!(out, "\\{}({},{})", self.ln_cmd, p1.x, p1.y);
            let cp2 = control(i);
            pt2l = self.fconvert(cp2.lx, cp2.ly);
            let pt2r = self.fconvert(cp2.rx, cp2.ry);
            let c = [
                f64::from(p1.x),
                f64::from(p1.y),
                pt1r.x,
                pt1r.y,
                pt2l.x,
                pt2l.y,
                f64::from(p2.x),
                f64::from(p2.y),
            ];
            self.bezier_spline(out, c, 0);
            out.puts("\n");
            p1 = p2;
            pt1r = pt2r;
        }
        if let Some(a) = &s.for_arrow {
            self.fdraw_arrow_head(out, ctx, pt2l, FPoint::of(p1), (a.ht, a.wid), a);
            if self.verbose {
                out.puts("%\n");
            }
        }
    }
}

/// Writes `(x,y)` points, three per output line, and a newline; `count`
/// points are already on the line.
fn put_points(out: &mut Out, pts: &[Point], count: usize) {
    let mut pt_count = count;
    let Some((last, rest)) = pts.split_last() else {
        return;
    };
    for p in rest {
        pt_count += 1;
        if pt_count > PT_PER_LINE {
            pt_count = 1;
            out.puts("\n\t");
        }
        fprintf!(out, "({},{})", p.x, p.y);
    }
    fprintf!(out, "({},{})\n", last.x, last.y);
}

/// `arc_arrow`: the point on the chord of the arc around `pt1` that meets
/// the arc the arrowhead's height back from `pt2`; the tangent where the
/// radius is too small. (Copied in C from xfig's `compute_arcarrow_angle`.)
fn arc_arrow(pt1: FPoint, pt2: FPoint, direction: bool, arrow: &Arrow) -> FPoint {
    let (x1, y1, x2, y2) = (pt1.x, pt1.y, pt2.x, pt2.y);
    let dy = y2 - y1;
    let dx = x2 - x1;
    let r = (dx * dx + dy * dy).sqrt();
    // How far the arrowhead extends beyond the end of the line.
    let lpt = arrow.thickness / 30.0 / (arrow.wid / arrow.ht / 2.0);
    let h = arrow.ht + lpt;
    if h > 2.0 * r {
        // `arc_tangent`.
        return if direction {
            FPoint {
                x: x2 - (y2 - y1),
                y: y2 + (x2 - x1),
            }
        } else {
            FPoint {
                x: x2 + (y2 - y1),
                y: y2 - (x2 - x1),
            }
        };
    }
    let beta = dx.atan2(dy);
    let alpha = 2.0 * (h / 2.0 / r).asin();
    let alpha = if direction { alpha } else { -alpha };
    FPoint {
        x: f64::from(round(x1 + r * (beta + alpha).sin())),
        y: y1 + r * (beta + alpha).cos(),
    }
}

/// `rtop`: polar coordinates.
fn rtop(x: f64, y: f64) -> (f64, f64) {
    let r = (x * x + y * y).sqrt();
    let mut th = (x / r).acos();
    if th < 0.0 {
        th += PI;
    }
    if y < 0.0 {
        th = 2.0 * PI - th;
    }
    (r, th)
}

/// `FillCommands`.
fn fill_commands(style: i32, color: i32) -> &'static str {
    let black = color == BLACK_COLOR || color == DEFAULT;
    match style {
        UNFILLED => "",
        WHITE_FILL if black => "\\whiten",
        WHITE_FILL => "\\blacken",
        BLACK_FILL if black => "\\blacken",
        BLACK_FILL => "\\whiten",
        _ => "\\shade",
    }
}

/// C's `%W.Pf`: Rust's `{:W.P}`, with `nan` and `inf` as C writes them.
fn cf(v: f64, width: usize, prec: usize) -> String {
    let s = if v.is_nan() {
        if v.is_sign_negative() { "-nan" } else { "nan" }.to_string()
    } else if v.is_infinite() {
        if v < 0.0 { "-inf" } else { "inf" }.to_string()
    } else {
        format!("{v:.prec$}")
    };
    format!("{s:>width$}")
}

impl Driver for Epic {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        let a = arg.unwrap_or("");
        match opt {
            // Processed in fig2dev.c.
            'G' => {}
            'd' => self.arrow_scale = atof(a),
            'a' => ctx.warn("warning: genepic option -a obsolete"),
            'f' => match (1..NUM_TEXFONTS).find(|&i| self.fonts.names.get(i).is_some_and(|n| n == a)) {
                None => ctx.warn(format!("warning: non-standard font name {a} ignored")),
                Some(i) => {
                    self.fonts.names[0] = self.fonts.names[i].clone();
                    self.fonts.family[0] = self.fonts.family[i].clone();
                    self.fonts.series[0] = self.fonts.series[i].clone();
                    self.fonts.shape[0] = self.fonts.shape[i].clone();
                }
            },
            'l' => {
                self.linew_spec = true;
                self.line_thick = strtol(a).0;
            }
            'L' => self.tex_lang = TexLang::from_name(a),
            'P' => self.page_mode = true,
            'S' => {
                let Some(&(mag, size)) = strtol(a)
                    .0
                    .checked_sub(8)
                    .and_then(|i| usize::try_from(i).ok())
                    .and_then(|i| SCALE_TBL.get(i))
                else {
                    return Err("Scale must be between 8 and 12 inclusively".into());
                };
                ctx.mag = mag;
                ctx.font_size = size;
            }
            'v' => self.verbose = true,
            'W' | 'w' => self.var_width = opt == 'W',
            't' => self.dash_stretch = strtol(a).0.max(-100),
            'F' => self.select_fontname = false,
            'R' => self.allow_rotated_text = true,
            _ => return Err(format!("Argument -{opt} unknown to epic driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        let fs = if ctx.font_size != 0.0 {
            ctx.font_size
        } else {
            DEFAULT_FONT_SIZE
        };
        let size = self.fonts.texfontsize(fs) as i32;
        self.fonts.sizes[0] = size;
        self.fonts.sizes[1] = size;

        // Whole-figure comments, prefixed with "%".
        if !objects.comments.is_empty() {
            out.puts("%\n");
            print_comments(out, "% ", &objects.comments, "");
            out.puts("%\n");
        }

        match self.tex_lang {
            TexLang::Epic => {
                self.oval = true;
                self.ln_cmd = "drawline";
            }
            TexLang::EEpicEmu | TexLang::EEpic => self.ln_cmd = "path",
            TexLang::Unknown => {
                ctx.warn("Program error in main");
                // C leaves LnCmd NULL, which printf writes as "(null)".
                self.ln_cmd = "(null)";
            }
        }
        if self.page_mode {
            out.puts(PREAMBLE);
        }

        let ppi = ctx.ppi;
        if self.linew_spec {
            self.line_thick = (f64::from(self.line_thick) * ppi / 80.0) as i32;
        }
        if self.line_thick == 0 {
            self.line_thick = (2.0 * ppi / 80.0) as i32;
        }
        self.dash_scale = ppi / 80.0;
        let mut pt1 = self.convert(Point::new(ctx.llx, ctx.lly));
        let mut pt2 = self.convert(Point::new(ctx.urx, ctx.ury));
        if pt1.x > pt2.x {
            std::mem::swap(&mut pt1.x, &mut pt2.x);
        }
        if pt1.y > pt2.y {
            std::mem::swap(&mut pt1.y, &mut pt2.y);
        }
        self.llx = pt1.x;
        self.lly = pt1.y;
        if self.verbose {
            fprintf!(out, "%\n% Language in use is {}\n%\n", self.tex_lang.name());
        }
        self.threshold = 1.0 / ppi * ctx.mag;
        fprintf!(out, "\\setlength{{\\unitlength}}{{{}in}}\n", cf(self.threshold, 0, 8));
        self.max_circle_radius = (40.0 / 72.27 / self.threshold) as i32;
        self.threshold = SEG_LEN / self.threshold;
        if self.dash_stretch != 0 {
            fprintf!(out, "{{\\renewcommand{{\\dashlinestretch}}{{{}}}\n", self.dash_stretch);
        }
        fprintf!(
            out,
            "\\begin{{picture}}({},{})(0,{})\n",
            pt2.x.wrapping_sub(pt1.x),
            pt2.y.wrapping_sub(pt1.y).wrapping_add(TOP_MARGIN + BOTTOM_MARGIN),
            -BOTTOM_MARGIN
        );
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, arc: &Arc) -> Result<(), String> {
        if self.failed {
            return Ok(());
        }
        print_comments(out, "% ", &arc.comments, "");

        let ctr = self.fconvert(arc.center.x, arc.center.y);
        let p0 = FPoint::of(arc.point[0]);
        let p2 = FPoint::of(arc.point[2]);
        let pt1 = self.fconvert(p0.x, p0.y);
        let pt2 = self.fconvert(p2.x, p2.y);
        let (r1, th1) = rtop(pt1.x - ctr.x, pt1.y - ctr.y);
        let (r2, th2) = rtop(pt2.x - ctr.x, pt2.y - ctr.y);
        let mut arrowfactor = (r1 + r2) / 30.0;
        if arrowfactor > 1.0 {
            arrowfactor = 1.0;
        }
        self.set_linewidth(out, ctx, arc.thickness);
        let eepic = self.tex_lang == TexLang::EEpic;
        if eepic {
            self.set_pattern(out, arc.fill_style, arc.fill_color);
            fprintf!(out, "\\put({},{}){{", cf(ctr.x, 4, 3), cf(ctr.y, 4, 3));
            let fill_style = arc.fill_style.max(UNFILLED);
            out.puts(fill_commands(fill_style, arc.fill_color));
            if fill_style != UNFILLED {
                self.out_line = true;
            }
        } else {
            out.puts("\\drawline");
        }
        // `\arc` measures its angles clockwise from the x axis.
        let (r, th_from, theta) = if arc.direction != 0 {
            let mut theta = th2 - th1;
            if theta < 0.0 {
                theta += 2.0 * PI;
            }
            (r1, 2.0 * PI - th2, theta)
        } else {
            let mut theta = th1 - th2;
            if theta < 0.0 {
                theta += 2.0 * PI;
            }
            (r2, 2.0 * PI - th1, theta)
        };
        if eepic {
            let cmd = format!(
                "\\arc{{{}}}{{{}}}{{{}}}}}\n",
                cf(2.0 * r, 4, 3),
                cf(th_from, 2, 4),
                cf(th_from + theta, 2, 4)
            );
            out.puts(&cmd);
            if self.out_line {
                self.out_line = false;
                fprintf!(out, "\\put({},{}){{", cf(ctr.x, 4, 3), cf(ctr.y, 4, 3));
                out.puts(&cmd);
            }
        } else {
            self.drawarc(out, ctr, r, 2.0 * PI - th_from - theta, theta);
        }
        if arc.type_ == T_OPEN_ARC && arc.thickness != 0 {
            let size = |a: &Arrow| (a.ht * arrowfactor, a.wid * arrowfactor);
            if let Some(a) = &arc.for_arrow {
                let tmp = arc_arrow(ctr, pt2, arc.direction != 0, a);
                self.fdraw_arrow_head(out, ctx, tmp, pt2, size(a), a);
                if self.verbose {
                    out.puts("%\n");
                }
            }
            if let Some(a) = &arc.back_arrow {
                let tmp = arc_arrow(ctr, pt1, arc.direction == 0, a);
                self.fdraw_arrow_head(out, ctx, tmp, pt1, size(a), a);
                if self.verbose {
                    out.puts("%\n");
                }
            }
        }
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, ell: &Ellipse) -> Result<(), String> {
        if self.failed {
            return Ok(());
        }
        print_comments(out, "% ", &ell.comments, "");

        self.set_linewidth(out, ctx, ell.thickness);
        let pt = self.convert(ell.center);
        let (rx, ry) = (ell.radiuses.x, ell.radiuses.y);
        let (w, h) = (rx.wrapping_mul(2), ry.wrapping_mul(2));
        if self.tex_lang == TexLang::EEpic
            || self.tex_lang == TexLang::EEpicEmu
            || rx != ry
            || rx > self.max_circle_radius
        {
            self.set_pattern(out, ell.fill_style, ell.fill_color);
            fprintf!(out, "\\put({},{}){{", pt.x, pt.y);
            if !self.oval {
                let fill_style = ell.fill_style.max(UNFILLED);
                out.puts(fill_commands(fill_style, ell.fill_color));
                if fill_style != UNFILLED {
                    self.out_line = true;
                }
            }
            let cmd = if self.oval {
                format!("\\oval({w},{h})}}\n")
            } else {
                format!("\\ellipse{{{w}}}{{{h}}}}}\n")
            };
            out.puts(&cmd);
            if self.out_line {
                self.out_line = false;
                fprintf!(out, "\\put({},{}){{", pt.x, pt.y);
                out.puts(&cmd);
            }
        } else {
            fprintf!(out, "\\put({},{}){{\\circle", pt.x, pt.y);
            if ell.fill_style == BLACK_FILL {
                out.puts("*");
            }
            fprintf!(out, "{{{w}}}}}\n");
        }
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, line: &Line) -> Result<(), String> {
        if self.failed {
            return Ok(());
        }
        print_comments(out, "% ", &line.comments, "");

        let pts: Vec<Point> = line.points.iter().map(|&p| self.convert(p)).collect();
        let (Some(&p0), Some(&p1)) = (pts.first(), pts.get(1)) else {
            if let Some(p) = pts.first() {
                fprintf!(out, "\\drawline({},{})({},{})\n", p.x, p.y, p.x, p.y);
            }
            return Ok(());
        };
        // First any backward arrowhead.
        if let Some(a) = &line.back_arrow {
            self.draw_arrow_head(out, ctx, p1, p0, a);
            if self.verbose {
                out.puts("%\n");
            }
        }
        // Now the attributes.
        self.set_linewidth(out, ctx, line.thickness);
        self.set_style(line.style, line.style_val);
        let mut type_ = line.type_;
        let mut fill_style = line.fill_style;
        if type_ == T_ARC_BOX {
            // A box with rounded corners.
            if self.tex_lang == TexLang::Epic || self.line_style != SOLID_LINE || fill_style != UNFILLED {
                ctx.warn("Arc box not implemented; substituting box.");
                type_ = T_BOX;
            } else {
                if self.verbose {
                    out.puts("%\n% An arcbox\n%\n");
                }
                let (mut llx, mut lly, mut urx, mut ury) = (p0.x, p0.y, p0.x, p0.y);
                for q in &pts {
                    llx = llx.min(q.x);
                    urx = urx.max(q.x);
                    lly = lly.min(q.y);
                    ury = ury.max(q.y);
                }
                let r = line.radius;
                let d = r.wrapping_mul(2);
                let (l, b) = (llx.wrapping_add(r), lly.wrapping_add(r));
                let (rt, t) = (urx.wrapping_sub(r), ury.wrapping_sub(r));
                fprintf!(out, "\\put({l},{b}){{\\arc{{{d}}}{{1.5708}}{{3.1416}}}}\n");
                fprintf!(out, "\\put({l},{t}){{\\arc{{{d}}}{{3.1416}}{{4.7124}}}}\n");
                fprintf!(out, "\\put({rt},{t}){{\\arc{{{d}}}{{4.7124}}{{6.2832}}}}\n");
                fprintf!(out, "\\put({rt},{b}){{\\arc{{{d}}}{{0}}{{1.5708}}}}\n");
                let ln = self.ln_cmd;
                fprintf!(out, "\\{ln}({llx},{b})({llx},{t})\n");
                fprintf!(out, "\\{ln}({l},{ury})({rt},{ury})\n");
                fprintf!(out, "\\{ln}({urx},{t})({urx},{b})\n");
                fprintf!(out, "\\{ln}({rt},{lly})({l},{lly})\n");
                return Ok(());
            }
        }
        if self.verbose {
            if type_ == T_BOX {
                out.puts("%\n% A box\n%\n");
            } else if type_ != T_ARC_BOX {
                out.puts("%\n% A polyline\n%\n");
            }
        }
        self.set_pattern(out, fill_style, line.fill_color);
        match self.line_style {
            SOLID_LINE => {
                if pts.len() > 2 && self.ln_cmd == "path" {
                    fill_style = fill_style.max(UNFILLED);
                    out.puts(fill_commands(fill_style, line.fill_color));
                }
                fprintf!(out, "\\{}", self.ln_cmd);
                if fill_style != UNFILLED {
                    self.out_line = true;
                }
            }
            DASH_LINE => fprintf!(out, "\\dashline{{{}}}", cf(self.dash_len, 4, 3)),
            DOTTED_LINE => fprintf!(out, "\\dottedline{{{}}}", self.dot_dist),
            _ => {
                ctx.warn("Only solid, dashed, and dotted line styles supported by epic(eepic)");
                self.failed = true;
                return Ok(());
            }
        }
        put_points(out, &pts, 0);
        if self.out_line {
            self.out_line = false;
            fprintf!(out, "\\{}", self.ln_cmd);
            put_points(out, &pts, 0);
        }
        // Finally any forward arrowhead.
        if let (Some(a), [.., p, q]) = (&line.for_arrow, pts.as_slice()) {
            self.draw_arrow_head(out, ctx, *p, *q, a);
            if self.verbose {
                out.puts("%\n");
            }
        }
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        if self.failed {
            return Ok(());
        }
        print_comments(out, "% ", &s.comments, "");

        self.set_linewidth(out, ctx, s.thickness);
        self.set_style(SOLID_LINE, 0.0);
        if s.int_spline() {
            self.itp_spline(out, ctx, s);
        } else if s.closed_spline() {
            self.closed_spline(out, s);
        } else {
            self.open_spline(out, ctx, s);
        }
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, text: &Text) -> Result<(), String> {
        if self.failed {
            return Ok(());
        }
        print_comments(out, "% ", &text.comments, "");

        let pt = self.convert(Point::new(text.base_x, text.base_y));
        let tpos = match text.type_ {
            T_LEFT_JUSTIFIED | DEFAULT => "[lb]",
            T_CENTER_JUSTIFIED => "[b]",
            T_RIGHT_JUSTIFIED => "[rb]",
            _ => {
                ctx.warn("unknown text position type");
                self.failed = true;
                return Ok(());
            }
        };
        fprintf!(out, "\\put({},{}){{", pt.x, pt.y);
        let rot_angle = (text.angle * (180.0 / PI)) as i32;
        let rotated = self.allow_rotated_text && rot_angle != 0;
        if rotated {
            fprintf!(out, "\\rotatebox[origin=l]{{{rot_angle}}}{{");
        } else {
            fprintf!(out, "\\makebox(0,0){tpos}{{");
        }
        out.puts("\\smash{");
        let mut t = text.clone();
        self.fonts
            .select_font(out, ctx, &mut t, true, self.select_fontname, false);
        put_string(out, &t.cstring, t.font, t.special_text());
        if rotated {
            out.puts("}");
        }
        out.puts("}}}\n");
        Ok(())
    }

    fn end(&mut self, _ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        if self.failed {
            return Ok(1);
        }
        out.puts("\\end{picture}\n");
        if self.dash_stretch != 0 {
            out.puts("}\n");
        }
        if self.page_mode {
            out.puts(POSTAMBLE);
        }
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}

/// Shading patterns of 32 × 32 bits (`patterns` in `set_pattern`).
#[rustfmt::skip]
const PATTERNS: [[u32; 32]; 21] = [
    [
        0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    ],
    [
        0x00000000, 0x00000000, 0x00000000, 0x00888888, 0x88000000, 0x00000000, 0x00000000, 0x00080808,
        0x08000000, 0x00000000, 0x00000000, 0x00888888, 0x88000000, 0x00000000, 0x00000000, 0x00080808,
        0x08000000, 0x00000000, 0x00000000, 0x00888888, 0x88000000, 0x00000000, 0x00000000, 0x00080808,
        0x08000000, 0x00000000, 0x00000000, 0x00888888, 0x88000000, 0x00000000, 0x00000000, 0x00080808,
    ],
    [
        0x08101010, 0x10000000, 0x00444444, 0x44000000, 0x00011101, 0x11000000, 0x00444444, 0x44000000,
        0x00101010, 0x10000000, 0x00444444, 0x44000000, 0x00010101, 0x01000000, 0x00444444, 0x44000000,
        0x00101010, 0x10000000, 0x00444444, 0x44000000, 0x00011101, 0x11000000, 0x00444444, 0x44000000,
        0x00101010, 0x10000000, 0x00444444, 0x44000000, 0x00010101, 0x01000000, 0x00444444, 0x44000000,
    ],
    [
        0x00000000, 0x00115111, 0x51000000, 0x00444444, 0x44000000, 0x00151515, 0x15000000, 0x00444444,
        0x44000000, 0x00511151, 0x11000000, 0x00444444, 0x44000000, 0x00151515, 0x15000000, 0x00444444,
        0x44000000, 0x00115111, 0x51000000, 0x00444444, 0x44000000, 0x00151515, 0x15000000, 0x00444444,
        0x44000000, 0x00511151, 0x11000000, 0x00444444, 0x44000000, 0x00151515, 0x15000000, 0x00444444,
    ],
    [
        0x44000000, 0x00aaaaaa, 0xaa000000, 0x008a888a, 0x88000000, 0x00aaaaaa, 0xaa000000, 0x00888888,
        0x88000000, 0x00aaaaaa, 0xaa000000, 0x008a8a8a, 0x8a000000, 0x00aaaaaa, 0xaa000000, 0x00888888,
        0x88000000, 0x00aaaaaa, 0xaa000000, 0x008a888a, 0x88000000, 0x00aaaaaa, 0xaa000000, 0x00888888,
        0x88000000, 0x00aaaaaa, 0xaa000000, 0x008a8a8a, 0x8a000000, 0x00aaaaaa, 0xaa000000, 0x00888888,
    ],
    [
        0x88555555, 0x55000000, 0x00555555, 0x55000000, 0x00555555, 0x55000000, 0x00555555, 0x55000000,
        0x00555555, 0x55000000, 0x00555555, 0x55000000, 0x00555555, 0x55000000, 0x00555555, 0x55000000,
        0x00555555, 0x55000000, 0x00555555, 0x55000000, 0x00555555, 0x55000000, 0x00555555, 0x55000000,
        0x00555555, 0x55000000, 0x00555555, 0x55000000, 0x00555555, 0x55000000, 0x00555555, 0x55000000,
    ],
    [
        0x00555555, 0x55000000, 0x00555555, 0x55888888, 0x88555555, 0x55000000, 0x00555555, 0x55808080,
        0x80555555, 0x55000000, 0x00555555, 0x55888888, 0x88555555, 0x55000000, 0x00555555, 0x55888088,
        0x80555555, 0x55000000, 0x00555555, 0x55888888, 0x88555555, 0x55000000, 0x00555555, 0x55808080,
        0x80555555, 0x55000000, 0x00555555, 0x55888888, 0x88555555, 0x55000000, 0x00555555, 0x55888088,
    ],
    [
        0x80222222, 0x22555555, 0x55808080, 0x80555555, 0x55222222, 0x22555555, 0x55880888, 0x08555555,
        0x55222222, 0x22555555, 0x55808080, 0x80555555, 0x55222222, 0x22555555, 0x55080808, 0x08555555,
        0x55222222, 0x22555555, 0x55808080, 0x80555555, 0x55222222, 0x22555555, 0x55880888, 0x08555555,
        0x55222222, 0x22555555, 0x55808080, 0x80555555, 0x55222222, 0x22555555, 0x55080808, 0x08555555,
    ],
    [
        0x55888888, 0x88555555, 0x5522a222, 0xa2555555, 0x55888888, 0x88555555, 0x552a2a2a, 0x2a555555,
        0x55888888, 0x88555555, 0x55a222a2, 0x22555555, 0x55888888, 0x88555555, 0x552a2a2a, 0x2a555555,
        0x55888888, 0x88555555, 0x5522a222, 0xa2555555, 0x55888888, 0x88555555, 0x552a2a2a, 0x2a555555,
        0x55888888, 0x88555555, 0x55a222a2, 0x22555555, 0x55888888, 0x88555555, 0x552a2a2a, 0x2a555555,
    ],
    [
        0x55aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa545454, 0x54aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa444444,
        0x44aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa445444, 0x54aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa444444,
        0x44aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa545454, 0x54aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa444444,
        0x44aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa445444, 0x54aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa444444,
    ],
    [
        0x44555555, 0x55aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa555555, 0x55aaaaaa,
        0xaa555555, 0x55aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa555555, 0x55aaaaaa,
        0xaa555555, 0x55aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa555555, 0x55aaaaaa,
        0xaa555555, 0x55aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa555555, 0x55aaaaaa, 0xaa555555, 0x55aaaaaa,
    ],
    [
        0xaadddddd, 0xddaaaaaa, 0xaa555555, 0x55aaaaaa, 0xaad5d5d5, 0xd5aaaaaa, 0xaa555555, 0x55aaaaaa,
        0xaadddddd, 0xddaaaaaa, 0xaa555555, 0x55aaaaaa, 0xaaddd5dd, 0xd5aaaaaa, 0xaa555555, 0x55aaaaaa,
        0xaadddddd, 0xddaaaaaa, 0xaa555555, 0x55aaaaaa, 0xaad5d5d5, 0xd5aaaaaa, 0xaa555555, 0x55aaaaaa,
        0xaadddddd, 0xddaaaaaa, 0xaa555555, 0x55aaaaaa, 0xaaddd5dd, 0xd5aaaaaa, 0xaa555555, 0x55aaaaaa,
    ],
    [
        0xaa777777, 0x77aaaaaa, 0xaad5d5d5, 0xd5aaaaaa, 0xaa777777, 0x77aaaaaa, 0xaadd5ddd, 0x5daaaaaa,
        0xaa777777, 0x77aaaaaa, 0xaad5d5d5, 0xd5aaaaaa, 0xaa777777, 0x77aaaaaa, 0xaa5ddd5d, 0xddaaaaaa,
        0xaa777777, 0x77aaaaaa, 0xaad5d5d5, 0xd5aaaaaa, 0xaa777777, 0x77aaaaaa, 0xaadd5ddd, 0x5daaaaaa,
        0xaa777777, 0x77aaaaaa, 0xaad5d5d5, 0xd5aaaaaa, 0xaa777777, 0x77aaaaaa, 0xaa5ddd5d, 0xddaaaaaa,
    ],
    [
        0xaa555555, 0x55bbbbbb, 0xbb555555, 0x55fefefe, 0xfe555555, 0x55bbbbbb, 0xbb555555, 0x55eeefee,
        0xef555555, 0x55bbbbbb, 0xbb555555, 0x55fefefe, 0xfe555555, 0x55bbbbbb, 0xbb555555, 0x55efefef,
        0xef555555, 0x55bbbbbb, 0xbb555555, 0x55fefefe, 0xfe555555, 0x55bbbbbb, 0xbb555555, 0x55eeefee,
        0xef555555, 0x55bbbbbb, 0xbb555555, 0x55fefefe, 0xfe555555, 0x55bbbbbb, 0xbb555555, 0x55efefef,
    ],
    [
        0xefffffff, 0xffaaaaaa, 0xaa777777, 0x77aaaaaa, 0xaaffffff, 0xffaaaaaa, 0xaa777f77, 0x7faaaaaa,
        0xaaffffff, 0xffaaaaaa, 0xaa777777, 0x77aaaaaa, 0xaaffffff, 0xffaaaaaa, 0xaa7f7f7f, 0x7faaaaaa,
        0xaaffffff, 0xffaaaaaa, 0xaa777777, 0x77aaaaaa, 0xaaffffff, 0xffaaaaaa, 0xaa777f77, 0x7faaaaaa,
        0xaaffffff, 0xffaaaaaa, 0xaa777777, 0x77aaaaaa, 0xaaffffff, 0xffaaaaaa, 0xaa7f7f7f, 0x7faaaaaa,
    ],
    [
        0xaaffffff, 0xffaaaaaa, 0xaaffffff, 0xffaaaaaa, 0xaaffffff, 0xffaaaaaa, 0xaaffffff, 0xffaaaaaa,
        0xaaffffff, 0xffaaaaaa, 0xaaffffff, 0xffaaaaaa, 0xaaffffff, 0xffaaaaaa, 0xaaffffff, 0xffaaaaaa,
        0xaaffffff, 0xffaaaaaa, 0xaaffffff, 0xffaaaaaa, 0xaaffffff, 0xffaaaaaa, 0xaaffffff, 0xffaaaaaa,
        0xaaffffff, 0xffaaaaaa, 0xaaffffff, 0xffaaaaaa, 0xaaffffff, 0xffaaaaaa, 0xaaffffff, 0xffaaaaaa,
    ],
    [
        0xaa555555, 0x55ffffff, 0xffdddddd, 0xddffffff, 0xff555555, 0x55ffffff, 0xff5ddd5d, 0xddffffff,
        0xff555555, 0x55ffffff, 0xffdddddd, 0xddffffff, 0xff555555, 0x55ffffff, 0xff5d5d5d, 0x5dffffff,
        0xff555555, 0x55ffffff, 0xffdddddd, 0xddffffff, 0xff555555, 0x55ffffff, 0xff5ddd5d, 0xddffffff,
        0xff555555, 0x55ffffff, 0xffdddddd, 0xddffffff, 0xff555555, 0x55ffffff, 0xff5d5d5d, 0x5dffffff,
    ],
    [
        0xffeeeeee, 0xeeffffff, 0xffbbbaba, 0xbaffffff, 0xffeeeeee, 0xeeffffff, 0xffabbbab, 0xbbffffff,
        0xffeeeeee, 0xeeffffff, 0xffbbbaba, 0xbaffffff, 0xffeeeeee, 0xeeffffff, 0xffbbabbb, 0xabffffff,
        0xffeeeeee, 0xeeffffff, 0xffbbbaba, 0xbaffffff, 0xffeeeeee, 0xeeffffff, 0xffabbbab, 0xbbffffff,
        0xffeeeeee, 0xeeffffff, 0xffbbbaba, 0xbaffffff, 0xffeeeeee, 0xeeffffff, 0xffbbabbb, 0xabffffff,
    ],
    [
        0xffffffff, 0xffeeeeee, 0xeeffffff, 0xfffbfbfb, 0xfbffffff, 0xffeeeeee, 0xeeffffff, 0xffbfbbbf,
        0xbbffffff, 0xffeeeeee, 0xeeffffff, 0xfffbfbfb, 0xfbffffff, 0xffeeeeee, 0xeeffffff, 0xffbfbfbf,
        0xbfffffff, 0xffeeeeee, 0xeeffffff, 0xfffbfbfb, 0xfbffffff, 0xffeeeeee, 0xeeffffff, 0xffbfbbbf,
        0xbbffffff, 0xffeeeeee, 0xeeffffff, 0xfffbfbfb, 0xfbffffff, 0xffeeeeee, 0xeeffffff, 0xffbfbfbf,
    ],
    [
        0xbfffffff, 0xffffffff, 0xffffffff, 0xffbbbbbb, 0xbbffffff, 0xffffffff, 0xffffffff, 0xfffbfbfb,
        0xfbffffff, 0xffffffff, 0xffffffff, 0xffbbbbbb, 0xbbffffff, 0xffffffff, 0xffffffff, 0xfffbfbfb,
        0xfbffffff, 0xffffffff, 0xffffffff, 0xffbbbbbb, 0xbbffffff, 0xffffffff, 0xffffffff, 0xfffbfbfb,
        0xfbffffff, 0xffffffff, 0xffffffff, 0xffbbbbbb, 0xbbffffff, 0xffffffff, 0xffffffff, 0xfffbfbfb,
    ],
    [
        0xfbffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff,
        0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff,
        0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff,
        0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff,
    ],
];
