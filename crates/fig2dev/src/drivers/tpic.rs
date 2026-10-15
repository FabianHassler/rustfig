//! fig2dev's `tpic` driver (`dev/gentpic.c`): PIC for TeX's tpic, in
//! inches with y upwards from 10.5 in. Line thickness goes into `.ps`
//! (milli-inches), shading into `shaded`, bitmap patterns into `.tx`;
//! texts are TeX boxes with fonts and sizes from tpic's own tables, fonts
//! beyond the six LaTeX ones as virtual (PostScript) fonts.
//!
//! fig2dev is configured with `TPIC_ARC_BOX` by default (configure's
//! `--enable-round-box`), so boxes are `box … radius …`; that variant
//! writes no comments of polylines.
//
// Ported from fig2dev dev/gentpic.c (xfig licence, see LICENSE-xfig).

use std::borrow::Cow;

use crate::gendev::{print_comments, Ctx};
use crate::object::{
    Arc, Compound, Ellipse, Line, Spline, Text, BLACK_FILL, DASH_LINE, DEFAULT, DOTTED_LINE, MAX_FONT, SOLID_LINE,
    T_ARC_BOX, T_BOX, T_CENTER_JUSTIFIED, T_LEFT_JUSTIFIED, T_OPEN_ARC, T_POLYGON, T_RIGHT_JUSTIFIED, WHITE_FILL,
};
use crate::util::psfonts::unpsfont;
use crate::{fprintf, round, Driver, Out};

/// The top of the page, inches.
const TOP: f64 = 10.5;
/// Default line thickness, milli-inches.
const LINE_WIDTH: i32 = 8;
const MAX_TPICFONT: usize = 48;
const MAXFONTSIZE: f64 = 25.0;
const MAXPATTERNS: i32 = 27;
/// Curves are subdivided down to this, inches.
const THRESHOLD: f64 = 0.05;
/// Text baseline correction, inches per point (`HT_OFFSET`).
const HT_OFFSET: f64 = 0.25 / 72.0;
/// Deepest subdivision of a curve; finite coordinates never get there.
const MAX_DEPTH: u32 = 80;

/// TeX font names: the six LaTeX fonts, then virtual PostScript fonts
/// (`texfontnames`).
/// C indexes one past its end with font 48 (`MAX_TPICFONT`); here that
/// gives an empty name.
const TEXFONTNAMES: [&str; MAX_TPICFONT] = [
    "rm", // default
    "rm", // roman
    "bf", // bold
    "it", // italic
    "sf", // sans serif
    "tt", // typewriter
    // Virtual fonts from here on.
    "avant", "avantcsc", "avantd", "avantdi", "avanti", "bookd", "bookdi", "bookl", "booklcsc", "bookli", "chanc",
    "cour", "courb", "courbi", "couri", "helv", "helvb", "helvbi", "helvc", "helvcb", "helvcbi", "helvci", "helvcsc",
    "helvi", "pal", "palb", "palbi", "palbu", "palc", "palcsc", "pali", "palsl", "palu", "palx", "times", "timesb",
    "timesbi", "timesc", "timescsc", "timesi", "timessl", "timesx",
];

/// Font size names for point sizes 0..=25 (`texfontsizes`).
const TEXFONTSIZES: [&str; 26] = [
    "ten", // default
    "fiv", "fiv", "fiv", "fiv", // small fonts
    "fiv", // five point font
    "six", "sev", "egt", // etc
    "nin", "ten", "elv", "twl", "twl", "frtn", "frtn", "frtn", "svtn", "svtn", "svtn", "twty", "twty", "twty", "twty",
    "twty", "twfv",
];

/// The point sizes of those fonts (`TeXfontsizes`).
const TEX_FONT_SIZES_INT: [i32; 26] = [
    10, // default
    5, 5, 5, 5, // small fonts
    5, // five point font
    6, 7, 8, // etc
    9, 10, 11, 12, 12, 14, 14, 14, 17, 17, 17, 20, 20, 20, 20, 20, 25,
];

/// Bitmap texture patterns for fills beyond black (`texture_patterns`).
const TEXTURE_PATTERNS: [&str; MAXPATTERNS as usize] = [
    "8 0 8 0 4 1 3 e 0 8 0 8 1 4 e 3", // scales
    "f f 8 0 8 0 8 0 f f 0 8 0 8 0 8", // bricks
    "8 1 4 2 2 4 1 8 8 1 4 2 2 4 1 8", // waves
    "8 0 4 0 2 0 1 0 0 8 0 4 0 2 0 1", // light backslash alternating
    "e 0 7 0 3 8 1 c 0 e 0 7 8 3 c 1", // heavy backslash alternating
    "7 7 b b d d e e 7 7 b b d d e e", // heavy backslash
    "8 8 4 4 2 2 1 1 8 8 4 4 2 2 1 1", // light backslash
    "9 9 c c 6 6 3 3 9 9 c c 6 6 3 3", // medium backslash
    "2 0 4 0 8 0 0 0 0 8 0 4 0 2 0 0", // light hash
    "f f 0 0 f f 0 0 f f 0 0 f f 0 0", // horizontal lines
    "f f 0 0 0 0 0 0 f f 0 0 0 0 0 0", // spaced horizontal lines
    "c c 0 0 0 0 0 0 3 3 0 0 0 0 0 0", // spaced horizontal dashed lines
    "f 0 f 0 f 0 f 0 0 f 0 f 0 f 0 f", // chessboard
    "f f 8 8 8 8 8 8 f f 8 8 8 8 8 8", // light meshed lines
    "a a 4 4 a a 1 1 a a 4 4 a a 1 1", // hashed dotted lines
    "0 1 0 2 0 4 0 8 1 0 2 0 4 0 8 0", // spaced light frontslash
    "8 3 0 7 0 e 1 c 3 8 7 0 e 0 c 1", // spaced heavy frontslash
    "e e d d b b 7 7 e e d d b b 7 7", // heavy frontslash
    "1 1 2 2 4 4 8 8 1 1 2 2 4 4 8 8", // light frontslash
    "3 3 6 6 c c 9 9 3 3 6 6 c c 9 9", // medium frontslash
    "4 0 a 0 0 0 0 0 0 4 0 a 0 0 0 0", // wallpaper - birds
    "a a a a a a a a a a a a a a a a", // vertical lines
    "8 8 8 8 8 8 8 8 8 8 8 8 8 8 8 8", // spaced vertical lines
    "0 1 0 1 1 0 1 0 0 1 0 1 1 0 1 0", // vertical dashed lines
    "0 0 0 8 1 4 2 a 5 5 2 a 1 4 0 8", // hashed diamonds
    "f f 8 0 8 0 8 0 8 0 8 0 8 0 8 0", // spaced meshed lines
    "8 2 4 4 2 8 1 0 2 8 4 4 8 2 0 1", // hashed dotted lines
];

/// The driver for `tpic`.
pub fn new(_lang: &str) -> Option<Box<dyn Driver>> {
    Some(Box::new(Tpic::default()))
}

/// gentpic.c's file-level and function-static state.
#[derive(Clone, Debug)]
pub struct Tpic {
    /// `texfontnames[0]`, which `-f` sets.
    default_font: Cow<'static, str>,
    /// `vfont`: the default font is a virtual font.
    vfont: bool,
    /// fig2dev's `ppi`, divided by the magnification in `start`.
    ppi: f64,
    cur_thickness: i32,
    style_val: f32,
    cur_baseline: i32,
    /// fig2dev exited (`exit(1)`): the output so far stands, nothing more
    /// is written, and the exit status is 1.
    failed: bool,
}

impl Default for Tpic {
    fn default() -> Tpic {
        Tpic {
            default_font: Cow::Borrowed(TEXFONTNAMES[0]),
            vfont: false,
            ppi: 1200.0,
            cur_thickness: -1,
            style_val: -1.0,
            cur_baseline: -1,
            failed: false,
        }
    }
}

fn convy(a: f64) -> f64 {
    TOP - a
}

/// `" dashed"` or `" dotted"` for dashed and dotted lines with a dash
/// length.
fn dash_attr(style: i32, style_val: f64) -> &'static str {
    if style == DASH_LINE && style_val > 0.0 {
        " dashed"
    } else if style == DOTTED_LINE && style_val > 0.0 {
        " dotted"
    } else {
        ""
    }
}

/// `" <->"`, `" <-"`, `" ->"` or nothing.
fn arrows<A>(for_arrow: &Option<A>, back_arrow: &Option<A>) -> &'static str {
    match (for_arrow.is_some(), back_arrow.is_some()) {
        (true, true) => " <->",
        (false, true) => " <-",
        (true, false) => " ->",
        (false, false) => "",
    }
}

impl Tpic {
    /// x (or a length) in inches.
    fn inch(&self, v: i32) -> f64 {
        f64::from(v) / self.ppi
    }

    fn texfont(&self, font: i32) -> &str {
        let i = usize::try_from(font).unwrap_or(0).min(MAX_TPICFONT);
        if i == 0 {
            &self.default_font
        } else {
            TEXFONTNAMES.get(i).copied().unwrap_or("")
        }
    }

    fn set_linewidth(&mut self, out: &mut Out, w: i32) {
        if w == 0 {
            return;
        }
        if w != self.cur_thickness {
            self.cur_thickness = w;
            let n = w.wrapping_mul(LINE_WIDTH).wrapping_mul(80);
            fprintf!(out, ".ps {}\n", round(f64::from(n) / self.ppi));
        }
    }

    fn set_style(&mut self, out: &mut Out, s: i32, v: f32) {
        if s == DASH_LINE || s == DOTTED_LINE {
            if v == self.style_val || v == 0.0 {
                return;
            }
            self.style_val = v;
            fprintf!(out, "dashwid = {:.3}i\n", f64::from(v) / self.ppi);
        }
    }

    fn set_baseline(&mut self, out: &mut Out, b: i32) {
        if b != self.cur_baseline {
            fprintf!(out, ".baseline {}\n", b);
            self.cur_baseline = b;
        }
    }

    /// `set_texture`: the bitmap patterns.
    fn set_texture(out: &mut Out, t: i32) {
        if t <= BLACK_FILL || t > MAXPATTERNS + BLACK_FILL {
            return;
        }
        if let Some(p) = usize::try_from(t - BLACK_FILL - 1)
            .ok()
            .and_then(|i| TEXTURE_PATTERNS.get(i))
        {
            fprintf!(out, ".tx 16 {}\n", p);
        }
    }

    /// `set_fill`: the fill attribute.
    fn set_fill(out: &mut Out, f: i32) {
        if !(WHITE_FILL..=BLACK_FILL + MAXPATTERNS).contains(&f) {
            return;
        }
        match f {
            BLACK_FILL => out.puts(" black"),
            WHITE_FILL => out.puts(" white"),
            _ if f > BLACK_FILL => out.puts(" shaded"),
            _ => fprintf!(out, " shaded {:.3}", f64::from(f - 1) * 0.05),
        }
    }

    fn quadratic_spline(out: &mut Out, c: [f64; 8], depth: u32) {
        let [x1, y1, a2, b2, a3, b3, x4, y4] = c;
        let xmid = (a2 + a3) / 2.0;
        let ymid = (b2 + b3) / 2.0;
        if ((x1 - xmid).abs() < THRESHOLD && (y1 - ymid).abs() < THRESHOLD) || depth > MAX_DEPTH {
            fprintf!(out, "\tto {:.3},{:.3}\\\n", xmid, ymid);
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
            Tpic::quadratic_spline(out, c, depth + 1);
        }
        if ((xmid - x4).abs() < THRESHOLD && (ymid - y4).abs() < THRESHOLD) || depth > MAX_DEPTH {
            fprintf!(out, "\tto {:.3},{:.3}\\\n", x4, y4);
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
            Tpic::quadratic_spline(out, c, depth + 1);
        }
    }

    fn bezier_spline(out: &mut Out, c: [f64; 8], depth: u32) {
        let [x0, y0, a1, b1, a2, b2, x3, y3] = c;
        if ((x0 - x3).abs() < THRESHOLD && (y0 - y3).abs() < THRESHOLD) || depth > MAX_DEPTH {
            fprintf!(out, "\tto {:.3},{:.3}\\\n", x3, y3);
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
            Tpic::bezier_spline(out, [x0, y0, sx1, sy1, sx2, sy2, xmid, ymid], depth + 1);
            Tpic::bezier_spline(out, [xmid, ymid, tx1, ty1, tx2, ty2, x3, y3], depth + 1);
        }
    }

    /// `gentpic_open_spline`.
    fn open_spline(&mut self, out: &mut Out, s: &Spline) {
        let (Some(p0), Some(p1)) = (s.points.first(), s.points.get(1)) else {
            return;
        };
        let (x1, y1) = (self.inch(p0.x), convy(self.inch(p0.y)));
        let (x2, y2) = (self.inch(p1.x), convy(self.inch(p1.y)));
        self.set_style(out, s.style, s.style_val as f32);
        self.set_linewidth(out, s.thickness);
        let attrs = format!(
            "{}{} from {:.3},{:.3} to {:.3},{:.3}",
            dash_attr(s.style, s.style_val),
            arrows(&s.for_arrow, &s.back_arrow),
            x1,
            y1,
            x2,
            y2
        );
        if s.points.len() == 2 {
            fprintf!(out, "line{attrs}\n");
            return;
        }
        fprintf!(out, "spline{attrs}");
        for q in s.points.iter().skip(2) {
            fprintf!(out, " to {:.3},{:.3}", self.inch(q.x), convy(self.inch(q.y)));
        }
        out.puts("\n");
    }

    /// `gentpic_closed_spline`.
    fn closed_spline(&mut self, out: &mut Out, s: &Spline) {
        let pts: Vec<(f64, f64)> = s
            .points
            .iter()
            .map(|p| (self.inch(p.x), convy(self.inch(p.y))))
            .collect();
        let (Some(&(ax, ay)), Some(&b)) = (pts.first(), pts.get(1)) else {
            return;
        };
        let (mut x1, mut y1, mut x2, mut y2) = (ax, ay, b.0, b.1);
        let mut cx1 = (x1 + x2) / 2.0;
        let mut cy1 = (y1 + y2) / 2.0;
        let mut cx2 = (x1 + 3.0 * x2) / 4.0;
        let mut cy2 = (y1 + 3.0 * y2) / 4.0;

        self.set_linewidth(out, s.thickness);
        Tpic::set_texture(out, s.fill_style); // probably won't work!
        self.set_style(out, s.style, s.style_val as f32);

        let n = pts.len();
        for (i, p) in pts.iter().skip(2).chain(std::iter::once(&b)).enumerate() {
            fprintf!(out, "line from {:.3},{:.3} ", cx1, cy1);
            x1 = x2;
            y1 = y2;
            x2 = p.0;
            y2 = p.1;
            let cx3 = (3.0 * x1 + x2) / 4.0;
            let cy3 = (3.0 * y1 + y2) / 4.0;
            let cx4 = (x1 + x2) / 2.0;
            let cy4 = (y1 + y2) / 2.0;
            Tpic::quadratic_spline(out, [cx1, cy1, cx2, cy2, cx3, cy3, cx4, cy4], 0);
            if i + 2 == n {
                // The closing segment.
                if s.thickness == 0 {
                    out.puts(" invis");
                }
                Tpic::set_fill(out, s.fill_style);
            }
            out.puts("\n");
            cx1 = cx4;
            cy1 = cy4;
            cx2 = (x1 + 3.0 * x2) / 4.0;
            cy2 = (y1 + 3.0 * y2) / 4.0;
        }
    }

    /// `gentpic_itp_spline`: interpolated splines of files before 3.2.
    fn itp_spline(&mut self, out: &mut Out, s: &Spline) {
        self.set_style(out, s.style, s.style_val as f32);
        Tpic::set_texture(out, s.fill_style); // probably won't work!
        self.set_linewidth(out, s.thickness);

        let control = |i: usize| s.controls.get(i).copied().unwrap_or_default();
        let Some(p1) = s.points.first() else {
            return;
        };
        let (mut x2, mut y2) = (self.inch(p1.x), convy(self.inch(p1.y)));
        let n = s.points.len();
        for (i, p2) in s.points.iter().enumerate().skip(1) {
            out.puts("line ");
            if s.back_arrow.is_some() && i == 1 {
                out.puts(" <- ");
            } else if s.for_arrow.is_some() && i + 1 == n {
                out.puts(" -> ");
            }
            fprintf!(out, " from {:.3},{:.3} ", x2, y2);
            let (x1, y1) = (x2, y2);
            x2 = self.inch(p2.x);
            y2 = convy(self.inch(p2.y));
            let (cp1, cp2) = (control(i - 1), control(i));
            let c = [
                x1,
                y1,
                cp1.rx / self.ppi,
                convy(cp1.ry / self.ppi),
                cp2.lx / self.ppi,
                convy(cp2.ly / self.ppi),
                x2,
                y2,
            ];
            Tpic::bezier_spline(out, c, 0);
            out.puts("\n");
        }
    }
}

impl Driver for Tpic {
    fn option(&mut self, _ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        match opt {
            'f' => {
                // Set the default text font.
                let a = arg.unwrap_or("");
                let i = (1..=MAX_TPICFONT)
                    .find(|&i| TEXFONTNAMES.get(i) == Some(&a))
                    .unwrap_or(MAX_TPICFONT + 1);
                if i > MAX_FONT as usize {
                    // Assume a virtual font.
                    self.vfont = true;
                }
                self.default_font = Cow::Owned(a.to_string());
            }
            'G' | 'L' => {}
            _ => return Err(format!("Argument -{opt} unknown to tpic driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        self.ppi = ctx.ppi / ctx.mag;
        if !objects.comments.is_empty() {
            out.puts(".\\\"\n");
            print_comments(out, ".\\\" ", &objects.comments, "");
            out.puts(".\\\"\n");
        }
        out.puts(".PS\n");
        Ok(())
    }

    fn arc(&mut self, _ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        if self.failed {
            return Ok(());
        }
        print_comments(out, ".\\\" ", &a.comments, "");

        let cx = a.center.x / self.ppi;
        let cy = convy(a.center.y / self.ppi);
        let sx = self.inch(a.point[0].x);
        let sy = convy(self.inch(a.point[0].y));
        let ex = self.inch(a.point[2].x);
        let ey = convy(self.inch(a.point[2].y));

        Tpic::set_texture(out, a.fill_style);
        self.set_linewidth(out, a.thickness);
        self.set_style(out, a.style, a.style_val as f32);

        out.puts("arc");
        out.puts(dash_attr(a.style, a.style_val));
        if a.type_ == T_OPEN_ARC && a.thickness != 0 {
            out.puts(arrows(&a.for_arrow, &a.back_arrow));
        }
        fprintf!(out, " at {cx:.3},{cy:.3} from {sx:.3},{sy:.3} to {ex:.3},{ey:.3}");
        if a.direction == 0 {
            out.puts(" cw");
        }
        if a.thickness == 0 {
            out.puts(" invis");
        }
        Tpic::set_fill(out, a.fill_style);
        out.puts("\n");
        Ok(())
    }

    fn ellipse(&mut self, _ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        if self.failed {
            return Ok(());
        }
        print_comments(out, ".\\\" ", &e.comments, "");

        self.set_linewidth(out, e.thickness);
        Tpic::set_texture(out, e.fill_style);
        self.set_style(out, e.style, e.style_val as f32);

        out.puts("ellipse");
        out.puts(dash_attr(e.style, e.style_val));
        fprintf!(
            out,
            " at {:.3},{:.3} wid {:.3} ht {:.3}",
            self.inch(e.center.x),
            convy(self.inch(e.center.y)),
            self.inch(e.radiuses.x.wrapping_mul(2)),
            self.inch(e.radiuses.y.wrapping_mul(2))
        );
        if e.thickness == 0 {
            out.puts(" invis");
        }
        Tpic::set_fill(out, e.fill_style);
        out.puts("\n");
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        if self.failed {
            return Ok(());
        }
        self.set_linewidth(out, l.thickness);
        self.set_style(out, l.style, l.style_val as f32);
        Tpic::set_texture(out, l.fill_style);
        let Some(p) = l.points.first() else {
            return Ok(());
        };
        if l.points.len() == 1 {
            // A single point.
            let (x, y) = (self.inch(p.x), convy(self.inch(p.y)));
            fprintf!(out, "line from {x:.3},{y:.3} to {x:.3},{y:.3}\n");
            return Ok(());
        }

        if l.type_ == T_BOX || l.type_ == T_ARC_BOX {
            let (mut llx, mut lly, mut urx, mut ury) = (p.x, p.y, p.x, p.y);
            for q in &l.points {
                llx = llx.min(q.x);
                urx = urx.max(q.x);
                lly = lly.min(q.y);
                ury = ury.max(q.y);
            }
            fprintf!(
                out,
                "box height {:.3} width {:.3}",
                self.inch(ury.wrapping_sub(lly)),
                self.inch(urx.wrapping_sub(llx))
            );
            match l.style {
                SOLID_LINE => {}
                DASH_LINE => out.puts(" dashed"),
                DOTTED_LINE => out.puts(" dotted"),
                _ => {
                    // C leaves the line unfinished.
                    ctx.warn("Program error! No other line styles allowed.\n");
                    return Ok(());
                }
            }
            if l.thickness == 0 {
                out.puts(" invis");
            }
            fprintf!(out, " radius {:.3}", self.inch(l.radius));
            Tpic::set_fill(out, l.fill_style);
            fprintf!(out, " with .nw at {:.3},{:.3}\n", self.inch(llx), convy(self.inch(lly)));
            return Ok(());
        }

        out.puts("line");
        out.puts(dash_attr(l.style, l.style_val));
        let arr = arrows(&l.for_arrow, &l.back_arrow);
        if arr.is_empty() {
            out.puts(" from ");
        } else {
            fprintf!(out, "{arr} from");
        }
        let n = l.points.len();
        for (i, q) in l.points.iter().enumerate() {
            fprintf!(out, " {:.3},{:.3}", self.inch(q.x), convy(self.inch(q.y)));
            if i + 1 < n {
                out.puts(" to");
            }
        }
        if l.thickness == 0 {
            out.puts(" invis");
        }
        if l.type_ == T_POLYGON {
            Tpic::set_fill(out, l.fill_style);
        }
        out.puts("\n");
        Ok(())
    }

    fn spline(&mut self, _ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        if self.failed {
            return Ok(());
        }
        print_comments(out, ".\\\" ", &s.comments, "");
        if s.int_spline() {
            self.itp_spline(out, s);
        } else if s.closed_spline() {
            self.closed_spline(out, s);
        } else {
            self.open_spline(out, s);
        }
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, text: &Text) -> Result<(), String> {
        if self.failed {
            return Ok(());
        }
        print_comments(out, ".\\\" ", &text.comments, "");

        // tpic learns the baseline spacing here; not used much.
        self.set_baseline(out, text.size as i32);

        let mut t = text.clone();
        unpsfont(ctx, &mut t);
        let size = t.size * if t.rigid_text() { 1.0 } else { ctx.fontmag };
        if t.font > MAX_FONT || self.vfont {
            // Virtual fonts are defined with TeX's font machinery.
            let f = self.texfont(t.font);
            fprintf!(out, "\"\\font\\fig{f}={f} at {}pt \\fig{f} ", t.size as i32);
        } else {
            // `TEXFONTMAG`.
            let i = if size <= MAXFONTSIZE {
                usize::try_from(round(size)).unwrap_or(0)
            } else {
                MAXFONTSIZE as usize
            };
            let s = TEXFONTSIZES.get(i).copied().unwrap_or(TEXFONTSIZES[0]);
            fprintf!(out, "\"\\{}{} ", s, self.texfont(t.font));
        }
        let tpos = match t.type_ {
            T_LEFT_JUSTIFIED | DEFAULT => "ljust",
            T_CENTER_JUSTIFIED => "",
            T_RIGHT_JUSTIFIED => "rjust",
            _ => {
                ctx.warn("unknown text position type");
                self.failed = true;
                return Ok(());
            }
        };
        // `TEXFONTMAGINT`.
        let i = if size <= MAXFONTSIZE { size } else { MAXFONTSIZE };
        let pts = TEX_FONT_SIZES_INT
            .get(if i.is_finite() && i >= 0.0 { i as usize } else { 0 })
            .copied()
            .unwrap_or(TEX_FONT_SIZES_INT[0]);
        let y = (convy(self.inch(t.base_y)) + f64::from(pts) * HT_OFFSET) as f32;
        fprintf!(
            out,
            "{}\" at {:.3},{:.3} {}\n",
            t.cstring,
            self.inch(t.base_x),
            f64::from(y),
            tpos
        );
        Ok(())
    }

    fn end(&mut self, _ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        if self.failed {
            return Ok(1);
        }
        out.puts(".PE\n");
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}
