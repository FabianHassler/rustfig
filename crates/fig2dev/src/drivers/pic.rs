//! fig2dev's `pic` driver (`dev/genpic.c`): troff's PIC language, in
//! inches with y upwards from 10.5 in. `-p` switches on GNU pic's
//! extensions: rounded boxes (`arc`), line thickness (`line`), filled
//! ellipses (`fill`), PostScript fonts (`psfont`), or all of them (`all`,
//! `allps`).
//
// Ported from fig2dev dev/genpic.c, dev/picfonts.h and dev/picpsfonts.h
// (xfig licence, see LICENSE-xfig).

use std::borrow::Cow;

use crate::gendev::{print_comments, Ctx, DEFAULT_FONT_SIZE};
use crate::object::{
    Arc, Compound, Ellipse, Line, Spline, Text, BLACK_FILL, DASH_LINE, DEFAULT, DOTTED_LINE, MAX_FONT, T_ARC_BOX,
    T_BOX, T_CENTER_JUSTIFIED, T_LEFT_JUSTIFIED, T_OPEN_ARC, T_RIGHT_JUSTIFIED, UNFILLED,
};
use crate::util::psfonts::unpsfont;
use crate::{fprintf, Driver, Out};

/// The top of the page, inches.
const TOP: f64 = 10.5;
/// Curves are subdivided down to this, inches.
const THRESHOLD: f64 = 0.05;
/// Text baseline correction, inches per point (`HT_OFFSET`).
const HT_OFFSET: f64 = 0.2 / 72.0;
/// Deepest subdivision of a curve; finite coordinates never get there.
const MAX_DEPTH: u32 = 80;

/// troff font names of the LaTeX fonts, from index 1 (`picfontnames`).
const PICFONTNAMES: [&str; 7] = [
    "R", "R", // default
    "R", // roman
    "B", // bold
    "I", // italic
    "H", // sans serif
    "C", // typewriter
];

/// The largest font size (`MAXFONTSIZE` of picfonts.h).
const MAXFONTSIZE: i32 = 108;
/// The font of PostScript font numbers out of range (`DEFAULT_PICFONT`).
const DEFAULT_PICFONT: usize = 13;

/// troff fonts of the PostScript fonts, index `font + 1`
/// (`PICPSfontmap`, values index [`PIC_FONTNAMES`]).
const PICPS_FONTMAP: [usize; 36] = [
    0, 1,  // default, Times-Roman
    2,  // Times-Italic
    3,  // Times-Bold
    4,  // Times-BoldItalic
    5,  // AvantGarde-Book
    7,  // AvantGarde-BookOblique
    6,  // AvantGarde-Demi
    8,  // AvantGarde-DemiOblique
    1,  // Bookman-Light
    2,  // Bookman-LightItalic
    3,  // Bookman-Demi
    4,  // Bookman-DemiItalic
    9,  // Courier
    11, // Courier-Oblique
    10, // Courier-Bold
    12, // Courier-BoldOblique
    5,  // Helvetica
    7,  // Helvetica-Oblique
    6,  // Helvetica-Bold
    8,  // Helvetica-BoldOblique
    5,  // Helvetica-Narrow
    7,  // Helvetica-Narrow-Oblique
    6,  // Helvetica-Narrow-Bold
    8,  // Helvetica-Narrow-BoldOblique
    1,  // NewCenturySchlbk-Roman
    2,  // NewCenturySchlbk-Italic
    3,  // NewCenturySchlbk-Bold
    4,  // NewCenturySchlbk-BoldItalic
    1,  // Palatino-Roman
    2,  // Palatino-Italic
    3,  // Palatino-Bold
    4,  // Palatino-BoldItalic
    13, // Symbol
    2,  // ZapfChancery-MediumItalic
    14, // ZapfDingbats
];

/// troff names of the PostScript fonts (`PICfontnames`).
const PIC_FONTNAMES: [&str; 15] = [
    "R", "R", // default
    "I", "B", "BI", "H", "HB", "HO", "HX", "C", "CB", "CO", "CX", "S", "S2",
];

/// The driver for `pic`.
pub fn new(_lang: &str) -> Option<Box<dyn Driver>> {
    Some(Box::new(Pic::default()))
}

/// genpic.c's file-level and function-static state.
#[derive(Clone, Debug)]
pub struct Pic {
    /// `picfontnames[0]` and `[1]`, which `-f` sets.
    default_font: Cow<'static, str>,
    /// fig2dev's `ppi`, divided by the magnification in `start`.
    ppi: f64,
    line_thickness: i32,
    opt_arc_box: bool,
    opt_line_thick: bool,
    opt_ellipse_fill: bool,
    /// Keep PostScript fonts (no `unpsfont`).
    opt_no_unps: bool,
    style_val: f32,
    /// fig2dev exited (`exit(1)`): the output so far stands, nothing more
    /// is written, and the exit status is 1.
    failed: bool,
}

impl Default for Pic {
    fn default() -> Pic {
        Pic {
            default_font: Cow::Borrowed(PICFONTNAMES[0]),
            ppi: 1200.0,
            line_thickness: 0,
            opt_arc_box: false,
            opt_line_thick: false,
            opt_ellipse_fill: false,
            opt_no_unps: false,
            style_val: -1.0,
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
    if style_val > 0.0 {
        match style {
            DASH_LINE => " dashed",
            DOTTED_LINE => " dotted",
            _ => "",
        }
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

impl Pic {
    fn inch(&self, v: i32) -> f64 {
        f64::from(v) / self.ppi
    }

    /// `PICFONT(F)`.
    fn picfont(&self, font: i32) -> &str {
        let i = if font <= MAX_FONT {
            usize::try_from(font.saturating_add(1)).unwrap_or(0)
        } else {
            MAX_FONT as usize
        };
        if i <= 1 {
            &self.default_font
        } else {
            PICFONTNAMES.get(i).copied().unwrap_or("R")
        }
    }

    fn set_linewidth(&mut self, w: i32) {
        let v = f64::from(w.wrapping_mul(80)) / self.ppi;
        self.line_thickness = if v.is_finite() { v as i32 } else { 0 };
    }

    /// `AddThickness`.
    fn add_thickness(&self, out: &mut Out) {
        if self.opt_line_thick && self.line_thickness != 0 {
            fprintf!(out, " thickness {}", self.line_thickness);
        }
    }

    fn set_style(&mut self, out: &mut Out, s: i32, v: f64) {
        if s == DASH_LINE || s == DOTTED_LINE {
            if v == f64::from(self.style_val) || v == 0.0 {
                return;
            }
            self.style_val = v as f32;
            fprintf!(
                out,
                "dashwid = {:.3}\n",
                f64::from((f64::from(self.style_val) / 80.0) as f32)
            );
        }
    }

    /// `genpic_box`: a box with the `box` command; `false` if the line
    /// hasn't five points.
    fn put_box(&self, out: &mut Out, l: &Line) -> bool {
        let Some(p) = l.points.first() else {
            return false;
        };
        if l.points.len() != 5 {
            return false;
        }
        let (mut minx, mut miny, mut maxx, mut maxy) = (p.x, p.y, p.x, p.y);
        for q in &l.points {
            minx = minx.min(q.x);
            maxx = maxx.max(q.x);
            miny = miny.min(q.y);
            maxy = maxy.max(q.y);
        }
        out.puts("box");
        if l.thickness == 0 {
            out.puts(" invis");
        } else {
            out.puts(dash_attr(l.style, l.style_val));
        }
        if l.fill_style != UNFILLED {
            fprintf!(out, " fill {:.3}", f64::from(l.fill_style) / f64::from(BLACK_FILL));
        }
        fprintf!(
            out,
            " with .sw at ({:.3},{:.3}) ",
            self.inch(minx),
            convy(self.inch(maxy))
        );
        let width = self.inch(maxx.wrapping_sub(minx)).abs();
        let height = (convy(self.inch(maxy)) - convy(self.inch(miny))).abs();
        fprintf!(out, "width {width:.3} height {height:.3}");
        if self.opt_arc_box && l.type_ == T_ARC_BOX {
            fprintf!(out, " rad {:.3}", self.inch(l.radius));
        }
        self.add_thickness(out);
        out.puts("\n");
        true
    }

    fn quadratic_spline(out: &mut Out, c: [f64; 8], depth: u32) {
        let [x1, y1, a2, b2, a3, b3, x4, y4] = c;
        let xmid = (a2 + a3) / 2.0;
        let ymid = (b2 + b3) / 2.0;
        if ((x1 - xmid).abs() < THRESHOLD && (y1 - ymid).abs() < THRESHOLD) || depth > MAX_DEPTH {
            fprintf!(out, " to {:.3},{:.3}", xmid, ymid);
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
            Pic::quadratic_spline(out, c, depth + 1);
        }
        if ((xmid - x4).abs() < THRESHOLD && (ymid - y4).abs() < THRESHOLD) || depth > MAX_DEPTH {
            fprintf!(out, " to {:.3},{:.3}", x4, y4);
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
            Pic::quadratic_spline(out, c, depth + 1);
        }
    }

    fn bezier_spline(out: &mut Out, c: [f64; 8], depth: u32) {
        let [x0, y0, a1, b1, a2, b2, x3, y3] = c;
        if ((x0 - x3).abs() < THRESHOLD && (y0 - y3).abs() < THRESHOLD) || depth > MAX_DEPTH {
            fprintf!(out, " to {:.3},{:.3}", x3, y3);
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
            Pic::bezier_spline(out, [x0, y0, sx1, sy1, sx2, sy2, xmid, ymid], depth + 1);
            Pic::bezier_spline(out, [xmid, ymid, tx1, ty1, tx2, ty2, x3, y3], depth + 1);
        }
    }

    /// `genpic_open_spline`. Pic's spline supports only solid lines; the
    /// thickness is the last one set.
    fn open_spline(&self, out: &mut Out, s: &Spline) {
        let (Some(p0), Some(p1)) = (s.points.first(), s.points.get(1)) else {
            return;
        };
        let attrs = format!(
            "{} from {:.3},{:.3} to {:.3},{:.3}",
            arrows(&s.for_arrow, &s.back_arrow),
            self.inch(p0.x),
            convy(self.inch(p0.y)),
            self.inch(p1.x),
            convy(self.inch(p1.y))
        );
        fprintf!(out, "{}{attrs}", if s.points.len() == 2 { "line" } else { "spline" });
        for q in s.points.iter().skip(2) {
            fprintf!(out, " to {:.3},{:.3}", self.inch(q.x), convy(self.inch(q.y)));
        }
        self.add_thickness(out);
        out.puts("\n");
    }

    /// `genpic_closed_spline`.
    fn closed_spline(&self, out: &mut Out, s: &Spline) {
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
        for p in pts.iter().skip(2).chain(std::iter::once(&b)) {
            fprintf!(out, "line from {:.3},{:.3} ", cx1, cy1);
            x1 = x2;
            y1 = y2;
            x2 = p.0;
            y2 = p.1;
            let cx3 = (3.0 * x1 + x2) / 4.0;
            let cy3 = (3.0 * y1 + y2) / 4.0;
            let cx4 = (x1 + x2) / 2.0;
            let cy4 = (y1 + y2) / 2.0;
            Pic::quadratic_spline(out, [cx1, cy1, cx2, cy2, cx3, cy3, cx4, cy4], 0);
            self.add_thickness(out);
            out.puts("\n");
            cx1 = cx4;
            cy1 = cy4;
            cx2 = (x1 + 3.0 * x2) / 4.0;
            cy2 = (y1 + 3.0 * y2) / 4.0;
        }
    }

    /// `genpic_itp_spline`: interpolated splines of files before 3.2.
    fn itp_spline(&self, out: &mut Out, s: &Spline) {
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
            fprintf!(out, " from {:.3},{:.3}", x2, y2);
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
            Pic::bezier_spline(out, c, 0);
            self.add_thickness(out);
            out.puts("\n");
        }
    }
}

impl Driver for Pic {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        let a = arg.unwrap_or("");
        match opt {
            'f' => {
                // Set the default text font.
                if !(1..=MAX_FONT as usize).any(|i| PICFONTNAMES.get(i) == Some(&a)) {
                    ctx.warn(format!("warning: non-standard font name {a}"));
                }
                self.default_font = Cow::Owned(a.to_string());
            }
            'G' | 'L' => {}
            'p' => match a {
                "all" => {
                    self.opt_arc_box = true;
                    self.opt_line_thick = true;
                    self.opt_ellipse_fill = true;
                }
                "arc" => self.opt_arc_box = true,
                "line" => self.opt_line_thick = true,
                "fill" => self.opt_ellipse_fill = true,
                "psfont" => self.opt_no_unps = true,
                "allps" => {
                    self.opt_arc_box = true;
                    self.opt_line_thick = true;
                    self.opt_ellipse_fill = true;
                    self.opt_no_unps = true;
                }
                _ => return Err(format!("Invalid option: {a}")),
            },
            _ => return Err(format!("Argument -{opt} unknown to pic driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        self.ppi = ctx.ppi / ctx.mag;
        // Whole-figure comments, prefixed with '.\" '.
        if !objects.comments.is_empty() {
            out.puts(".\\\"\n");
            print_comments(out, ".\\\" ", &objects.comments, "");
            out.puts(".\\\"\n");
        }
        let size = if ctx.font_size != 0.0 {
            ctx.font_size as i32
        } else {
            DEFAULT_FONT_SIZE as i32
        };
        fprintf!(out, ".PS\n.ps {size}\n");
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
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

        self.set_linewidth(a.thickness);
        out.puts("arc ");
        out.puts(dash_attr(a.style, a.style_val));
        if a.type_ == T_OPEN_ARC && a.thickness != 0 {
            out.puts(arrows(&a.for_arrow, &a.back_arrow));
        }
        fprintf!(out, " at {cx:.3},{cy:.3} from {sx:.3},{sy:.3} to {ex:.3},{ey:.3}");
        if a.direction == 0 {
            out.puts(" cw");
        }
        if a.fill_style != UNFILLED {
            ctx.warn("PIC does not support filled arcs ... ignoring 'fill' directive");
        }
        self.add_thickness(out);
        out.puts("\n");
        Ok(())
    }

    fn ellipse(&mut self, _ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        if self.failed {
            return Ok(());
        }
        print_comments(out, ".\\\" ", &e.comments, "");

        self.set_linewidth(e.thickness);
        let (x, y) = (self.inch(e.center.x), convy(self.inch(e.center.y)));
        if e.type_ == 3 || e.type_ == 4 {
            fprintf!(out, "circle at {x:.3},{y:.3} rad {:.3}", self.inch(e.radiuses.x));
        } else {
            fprintf!(
                out,
                "ellipse at {x:.3},{y:.3} wid {:.3} ht {:.3}",
                self.inch(e.radiuses.x.wrapping_mul(2)),
                self.inch(e.radiuses.y.wrapping_mul(2))
            );
        }
        if self.opt_ellipse_fill && e.fill_style != UNFILLED {
            fprintf!(out, " fill {:.3}", f64::from(e.fill_style) / f64::from(BLACK_FILL));
        }
        self.add_thickness(out);
        out.puts("\n");
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, line: &Line) -> Result<(), String> {
        if self.failed {
            return Ok(());
        }
        print_comments(out, ".\\\" ", &line.comments, "");

        let mut l = Cow::Borrowed(line);
        if l.type_ == T_ARC_BOX && !self.opt_arc_box {
            ctx.warn("Arc box not implemented; substituting box.");
            l.to_mut().type_ = T_BOX;
        }
        self.set_linewidth(l.thickness);
        self.set_style(out, l.style, l.style_val);
        let Some(p) = l.points.first() else {
            return Ok(());
        };
        if l.points.len() == 1 {
            // A single point.
            let (x, y) = (self.inch(p.x), convy(self.inch(p.y)));
            fprintf!(out, "line from {x:.3},{y:.3} to {x:.3},{y:.3}");
            self.add_thickness(out);
            out.puts("\n");
            return Ok(());
        }
        if l.type_ == T_BOX || l.type_ == T_ARC_BOX {
            if self.put_box(out, &l) {
                return Ok(());
            }
            ctx.warn("Invalid T_BOX or T_ARC_BOX in fig file");
            ctx.warn("  Using 'line' instead");
        }
        out.puts("line");
        out.puts(dash_attr(l.style, l.style_val));
        out.puts(arrows(&l.for_arrow, &l.back_arrow));
        fprintf!(out, " from {:.3},{:.3}", self.inch(p.x), convy(self.inch(p.y)));
        for q in l.points.iter().skip(1) {
            fprintf!(out, " to {:.3},{:.3}", self.inch(q.x), convy(self.inch(q.y)));
        }
        self.add_thickness(out);
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

        // `PICFONTMAG`.
        let s = text.size * if text.rigid_text() { 1.0 } else { ctx.fontmag };
        let s = if s.is_finite() { s as i32 } else { 0 };
        let size = if s > 0 {
            s.min(MAXFONTSIZE)
        } else {
            ctx.font_size as i32
        };
        let mut t = text.clone();
        if self.opt_no_unps {
            // `PICPSFONT`.
            let i = if t.font <= t.max_font() {
                usize::try_from(t.font.saturating_add(1)).unwrap_or(0)
            } else {
                DEFAULT_PICFONT
            };
            let f = PICPS_FONTMAP
                .get(i)
                .and_then(|&j| PIC_FONTNAMES.get(j))
                .copied()
                .unwrap_or("R");
            fprintf!(out, ".ps\n.ps {size}\n.ft\n.ft {f}\n");
        } else {
            unpsfont(ctx, &mut t);
            fprintf!(out, "\"\\s{size}\\f{}", self.picfont(t.font));
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
        let y = f64::from((convy(self.inch(t.base_y)) + f64::from(size) * HT_OFFSET) as f32);
        let x = self.inch(t.base_x);
        if self.opt_no_unps {
            out.puts("\"");
            out.bytes(&t.raw);
            fprintf!(out, "\" at {x:.3},{y:.3} {tpos}\n.ft \n.ps \n");
        } else {
            out.bytes(&t.raw);
            fprintf!(out, "\\fP\" at {x:.3},{y:.3} {tpos}\n");
        }
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
