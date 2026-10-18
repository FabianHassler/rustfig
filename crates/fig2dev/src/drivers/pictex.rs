//! fig2dev's `pictex` driver: the figure as PiCTeX macros, in centimetres
//! with y up from 10.5 inches.
//!
//! Lines become `\putrule` (horizontal and vertical solid lines) or
//! `\plot`, arcs `\circulararc`, ellipses `\ellipticalarc`, splines plotted
//! polylines, arrowheads open `\plot`s; boxes may be shaded. Texts are
//! set with LaTeX fonts (PostScript fonts are replaced) and rotated with
//! PostScript `\special`s for dvips.
//!
//! Options: `-f` the default font, `-l` the line thickness, `-p` the plot
//! symbol, `-r` no rotation `\special`s.
//
// Ported from fig2dev dev/genpictex.c and localmath.c (xfig licence, see
// LICENSE-xfig).

use std::f64::consts::PI;

use super::latex::{default_font_option, set_default_size, Cf};
use crate::gendev::{print_comments, Ctx, PACKAGE_VERSION, PROG};
use crate::object::{
    Arc, Compound, Ellipse, Line, Spline, Text, BLACK_FILL, DASH_LINE, DEFAULT, DOTTED_LINE, SOLID_LINE, T_ARC_BOX,
    T_BOX, T_CENTER_JUSTIFIED, T_LEFT_JUSTIFIED, T_OPEN_ARC, T_RIGHT_JUSTIFIED, UNFILLED, WHITE_FILL,
};
use crate::util::creationdate::creation_date;
use crate::util::latex::LatexColor;
use crate::util::texfonts::{put_string, TexFonts};
use crate::{fprintf, round, Driver, Out};

const UNIT: &str = "cm";
const CONVUNIT: f64 = 2.54;
/// The top of the page, inches.
const TOP: f64 = 10.5;
/// Black filled circles larger than this (points) are too big.
const MAXBLACKDIAM: i32 = 15;
/// Splines are plotted with points closer than this, inches.
const THRESHOLD: f64 = 0.05;
/// Splines are subdivided at most this deep (not in C, which recurses
/// until the threshold is met; this is reached only by segments over
/// 50,000 inches long).
const MAX_DEPTH: u32 = 20;

fn convy(a: f64) -> f64 {
    TOP - a
}

/// `rtop`: rectangular to polar.
pub(super) fn rtop(x: f64, y: f64) -> (f64, f64) {
    let r = (x * x + y * y).sqrt();
    let th = (x / r).acos();
    (r, if y < 0.0 { 2.0 * PI - th } else { th })
}

/// `arc_tangent` (localmath.c): a point on the tangent of the arc at
/// (x2, y2) around (x1, y1).
pub(super) fn arc_tangent(x1: f64, y1: f64, x2: f64, y2: f64, direction: bool) -> (f64, f64) {
    if direction {
        (x2 - (y2 - y1), y2 + (x2 - x1))
    } else {
        (x2 + (y2 - y1), y2 - (x2 - x1))
    }
}

/// The `pictex` driver's state (genpictex.c's statics).
#[derive(Debug)]
pub struct Pictex {
    fonts: TexFonts,
    color: LatexColor,
    dash_length: f64,
    line_style: i32,
    linethick: String,
    plotsymbol: String,
    cur_thickness: i32,
    rotate: bool,
}

impl Default for Pictex {
    fn default() -> Pictex {
        Pictex {
            fonts: TexFonts::new(),
            color: LatexColor::default(),
            dash_length: -1.0,
            line_style: SOLID_LINE,
            linethick: "1pt".into(),
            plotsymbol: "\\makebox(0,0)[l]{\\tencirc\\symbol{'160}}".into(),
            cur_thickness: -1,
            rotate: true,
        }
    }
}

/// The driver for `lang`.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    (lang == "pictex").then(|| Box::new(Pictex::default()) as Box<dyn Driver>)
}

/// Fig units to output units, x.
fn cx(v: f64, ppi: f64) -> f64 {
    (v / ppi) * CONVUNIT
}

/// Fig units to output units, y.
fn cy(v: f64, ppi: f64) -> f64 {
    convy(v / ppi) * CONVUNIT
}

impl Pictex {
    /// `set_linewidth`.
    fn set_linewidth(&mut self, out: &mut Out, ctx: &Ctx, w: i32) {
        if w == 0 {
            out.puts("\\linethickness=0pt\n");
            self.cur_thickness = 0;
        } else {
            self.cur_thickness = round(f64::from(w) * ctx.mag * 80.0 / ctx.ppi);
            if self.cur_thickness == 0 || self.cur_thickness == 1 {
                self.cur_thickness = 1;
                fprintf!(out, "\\linethickness={:6.3}pt\n", 0.5 * f64::from(self.cur_thickness));
            } else {
                fprintf!(out, "\\linethickness={}pt\n", self.cur_thickness.wrapping_sub(1));
            }
        }
        match self.cur_thickness {
            0 => out.puts("\\setplotsymbol ({\\thinlinefont \\ })\n"),
            1 => out.puts("\\setplotsymbol ({\\thinlinefont .})\n"),
            // \tencirc's characters '160 to '176.
            c @ 2..=16 => fprintf!(
                out,
                "\\setplotsymbol ({{\\makebox(0,0)[l]{{\\tencirc\\symbol{{'{:o}}}}}}})\n",
                0o160 + c - 2
            ),
            _ => out.puts("\\setplotsymbol ({\\makebox(0,0)[l]{\\tencirc\\symbol{'176}}})\n"),
        }
    }

    /// `set_style`: issue style commands as needed.
    fn set_style(&mut self, out: &mut Out, style: i32, dash_len: f64) {
        let dash = (0.5 * dash_len * f64::from(self.cur_thickness.wrapping_add(1)) / 80.0) * CONVUNIT;
        // Nothing when the style is already set.
        let same = self.line_style == style && (style != DASH_LINE || self.dash_length == dash_len);
        match style {
            SOLID_LINE if !same => out.puts("\\setsolid\n"),
            DASH_LINE if !same => fprintf!(out, "\\setdashes <{:7.4}{}>\n", Cf(dash), UNIT),
            DOTTED_LINE if !same => fprintf!(out, "\\setdots <{:7.4}{}>\n", Cf(dash), UNIT),
            _ => {}
        }
        self.line_style = style;
        self.dash_length = dash_len;
    }

    /// `putline`: a rule where possible, lengthened at corners by half the
    /// line width; `next` is the following point (none for the last
    /// segment), `first` the first segment of the line.
    #[allow(clippy::too_many_arguments)]
    fn putline(
        &self,
        out: &mut Out,
        ctx: &Ctx,
        start: (i32, i32),
        end: (f64, f64),
        next: (i32, i32),
        first_start: (i32, i32),
        first_end: (i32, i32),
    ) {
        let (start_x, start_y) = (f64::from(start.0), f64::from(start.1));
        let (mut end_x, mut end_y) = end;
        let ppi = ctx.ppi;
        if self.line_style == SOLID_LINE && (start_x == end_x || start_y == end_y) {
            let ext = 0.5 * ppi * f64::from(self.cur_thickness.wrapping_sub(1)) / 72.0 / ctx.mag;
            let (next_x, next_y) = (f64::from(next.0), f64::from(next.1));
            if next.0 != -1 {
                if start_x == end_x && end_y == next_y {
                    end_y += if end_y > start_y { ext } else { -ext };
                } else if start_y == end_y && end_x == next_x {
                    end_x += if end_x > start_x { ext } else { -ext };
                }
            } else if end_x == f64::from(first_start.0) && end_y == f64::from(first_start.1) {
                if start_x == end_x && first_start.1 == first_end.1 {
                    end_y += if end_y > start_y { ext } else { -ext };
                } else if start_y == end_y && first_start.0 == first_end.0 {
                    end_x += if end_x > start_x { ext } else { -ext };
                }
            }
            fprintf!(
                out,
                "\\putrule from {:6.3} {:6.3} to {:6.3} {:6.3}\n",
                Cf(cx(start_x, ppi)),
                Cf(cy(start_y, ppi)),
                Cf(cx(end_x, ppi)),
                Cf(cy(end_y, ppi))
            );
        } else {
            fprintf!(
                out,
                "\\plot {:6.3} {:6.3} {:6.3} {:6.3} /\n",
                Cf(cx(start_x, ppi)),
                Cf(cy(start_y, ppi)),
                Cf(cx(end_x, ppi)),
                Cf(cy(end_y, ppi))
            );
        }
    }

    /// `put_box`: a box, perhaps shaded.
    fn put_box(&mut self, out: &mut Out, ctx: &mut Ctx, b: [i32; 4], l: &Line, type_: i32) {
        let [llx, lly, urx, ury] = b.map(f64::from);
        let ppi = ctx.ppi;
        self.color.set_color(out, ctx, l.pen_color);
        if type_ == T_BOX {
            let fill = l.fill_style;
            if fill == BLACK_FILL {
                fprintf!(
                    out,
                    "\\linethickness={:6.3}{}\n",
                    Cf((convy(lly / ppi) - convy(ury / ppi)) * CONVUNIT * ctx.mag),
                    UNIT
                );
                out.puts("{\\setsolid");
                let ym = ((convy(lly / ppi) + convy(ury / ppi)) / 2.0) * CONVUNIT;
                fprintf!(
                    out,
                    "\\putrule from {:6.3} {:6.3} to {:6.3} {:6.3} }}%\n",
                    Cf(cx(llx, ppi)),
                    Cf(ym),
                    Cf(cx(urx, ppi)),
                    Cf(ym)
                );
                fprintf!(out, "\\linethickness={}pt\n", l.thickness);
            } else if fill > BLACK_FILL * 3 / 4 {
                out.puts("\\setshadegrid span <1pt>\n\\shaderectangleson\n");
            } else if fill > BLACK_FILL * 2 / 4 {
                out.puts("\\setshadegrid span <2pt>\n\\shaderectangleson\n");
            } else if fill > BLACK_FILL / 4 {
                out.puts("\\setshadegrid span <4pt>\n\\shaderectangleson\n");
            } else if fill == WHITE_FILL {
                ctx.warn("WHITE_FILL not implemented for boxes");
            }
            fprintf!(
                out,
                "\\putrectangle corners at {:6.3} {:6.3} and {:6.3} {:6.3}\n",
                Cf(cx(llx, ppi)),
                Cf(cy(lly, ppi)),
                Cf(cx(urx, ppi)),
                Cf(cy(ury, ppi))
            );
            if fill != UNFILLED && fill != WHITE_FILL && fill != BLACK_FILL {
                out.puts("\\setshadegrid span <5pt>\n\\shaderectanglesoff\n");
            }
        } else if type_ == T_ARC_BOX {
            // Not reached: genpictex_line draws rounded boxes as boxes.
            let r = f64::from(l.radius);
            let rules = [
                (llx + r, lly, urx - r, lly),
                (urx, lly + r, urx, ury - r),
                (urx - r, ury, llx + r, ury),
                (llx, ury - r, llx, lly + r),
            ];
            for (x1, y1, x2, y2) in rules {
                fprintf!(
                    out,
                    "\\putrule from {:6.3} {:6.3} to {:6.3} {:6.3}\n",
                    Cf(cx(x1, ppi)),
                    Cf(cy(y1, ppi)),
                    Cf(cx(x2, ppi)),
                    Cf(cy(y2, ppi))
                );
            }
            let arcs = [
                (urx - r, lly, urx - r, lly + r),
                (urx, ury - r, urx - r, ury - r),
                (llx + r, ury, llx + r, ury - r),
                (llx, lly + r, llx + r, lly + r),
            ];
            for (x1, y1, x2, y2) in arcs {
                fprintf!(
                    out,
                    "\\circulararc -90 degrees from {:6.3} {:6.3} center at {:6.3} {:6.3}\n",
                    Cf(cx(x1, ppi)),
                    Cf(cy(y1, ppi)),
                    Cf(cx(x2, ppi)),
                    Cf(cy(y2, ppi))
                );
            }
        }
        self.color.reset_color(out, l.pen_color);
    }

    /// `draw_arrow_head`: an open arrowhead from (x1, y1) to (x2, y2), in
    /// inches (y up).
    #[allow(clippy::too_many_arguments)]
    fn draw_arrow_head(&mut self, out: &mut Out, x1: f64, y1: f64, x2: f64, y2: f64, arrowht: f64, arrowwid: f64) {
        let dx = x2 - x1;
        let dy = y1 - y2;
        if dx == 0.0 && dy == 0.0 {
            return;
        }
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

        // Draw it solid, then restore the line style.
        let (style, dash) = (self.line_style, self.dash_length);
        self.set_style(out, SOLID_LINE, 0.0);
        out.puts("%\n% arrow head\n%\n");
        fprintf!(
            out,
            "\\plot {:6.3} {:6.3} {:6.3} {:6.3} {:6.3} {:6.3} /\n%\n",
            Cf(xc * CONVUNIT),
            Cf(yc * CONVUNIT),
            Cf(x2 * CONVUNIT),
            Cf(y2 * CONVUNIT),
            Cf(xd * CONVUNIT),
            Cf(yd * CONVUNIT)
        );
        self.set_style(out, style, dash);
    }

    /// `genpictex_ctl_spline`: an approximated spline.
    fn ctl_spline(&mut self, out: &mut Out, ctx: &Ctx, s: &Spline) {
        let ppi = ctx.ppi;
        out.puts("%\n% Fig CONTROL PT SPLINE\n%\n");
        let pt = |i: usize| {
            s.points
                .get(i)
                .map(|p| (f64::from(p.x) / ppi, convy(f64::from(p.y) / ppi)))
        };
        let (Some((mut x1, mut y1)), Some((mut x2, mut y2))) = (pt(0), pt(1)) else {
            return;
        };
        let mut cx1 = (x1 + x2) / 2.0;
        let mut cy1 = (y1 + y2) / 2.0;
        let mut cx2 = (x1 + 3.0 * x2) / 4.0;
        let mut cy2 = (y1 + 3.0 * y2) / 4.0;

        if s.closed_spline() {
            out.puts("% closed spline\n%\n");
            fprintf!(out, "\\plot\t{:6.3} {:6.3} \n ", Cf(cx1 * CONVUNIT), Cf(cy1 * CONVUNIT));
        } else {
            out.puts("% open spline\n%\n");
            if let Some(a) = &s.back_arrow {
                self.draw_arrow_head(out, cx1, cy1, x1, y1, a.ht / ppi, a.wid / ppi);
            }
            fprintf!(
                out,
                "\\plot\t{:6.3} {:6.3} {:6.3} {:6.3}\n ",
                Cf(x1 * CONVUNIT),
                Cf(y1 * CONVUNIT),
                Cf(cx1 * CONVUNIT),
                Cf(cy1 * CONVUNIT)
            );
        }

        for i in 2..s.points.len() {
            let Some((nx, ny)) = pt(i) else { break };
            x1 = x2;
            y1 = y2;
            x2 = nx;
            y2 = ny;
            let cx3 = (3.0 * x1 + x2) / 4.0;
            let cy3 = (3.0 * y1 + y2) / 4.0;
            let cx4 = (x1 + x2) / 2.0;
            let cy4 = (y1 + y2) / 2.0;
            quadratic_spline(out, [cx1, cy1, cx2, cy2, cx3, cy3, cx4, cy4], 0);
            cx1 = cx4;
            cy1 = cy4;
            cx2 = (x1 + 3.0 * x2) / 4.0;
            cy2 = (y1 + 3.0 * y2) / 4.0;
        }
        x1 = x2;
        y1 = y2;
        let Some((x2, y2)) = pt(1) else { return };
        let cx3 = (3.0 * x1 + x2) / 4.0;
        let cy3 = (3.0 * y1 + y2) / 4.0;
        let cx4 = (x1 + x2) / 2.0;
        let cy4 = (y1 + y2) / 2.0;
        if s.closed_spline() {
            quadratic_spline(out, [cx1, cy1, cx2, cy2, cx3, cy3, cx4, cy4], 0);
            out.puts("\t/\n");
        } else {
            fprintf!(
                out,
                "\t /\n\\plot {:6.3} {:6.3} {:6.3} {:6.3} /\n",
                Cf(cx1 * CONVUNIT),
                Cf(cy1 * CONVUNIT),
                Cf(x1 * CONVUNIT),
                Cf(y1 * CONVUNIT)
            );
            if let Some(a) = &s.for_arrow {
                self.draw_arrow_head(out, cx1, cy1, x1, y1, a.ht / ppi, a.wid / ppi);
            }
        }
    }

    /// `genpictex_itp_spline`: an interpolated spline, as Bézier curves
    /// through the points with the file's control points.
    fn itp_spline(&mut self, out: &mut Out, ctx: &Ctx, s: &Spline) {
        let ppi = ctx.ppi;
        out.puts("%\n% Fig INTERPOLATED PT SPLINE\n%\n");
        let (Some(p1), Some(cp1)) = (s.points.first(), s.controls.first()) else {
            return;
        };
        let mut x2 = f64::from(p1.x) / ppi;
        let mut y2 = convy(f64::from(p1.y) / ppi);
        if let Some(a) = &s.back_arrow {
            self.draw_arrow_head(out, cp1.rx / ppi, convy(cp1.ry / ppi), x2, y2, a.ht / ppi, a.wid / ppi);
        }
        fprintf!(out, "\\plot {:6.3} {:6.3} ", Cf(x2 * CONVUNIT), Cf(y2 * CONVUNIT));
        let mut cp1 = *cp1;
        for (p2, cp2) in s.points.iter().skip(1).zip(s.controls.iter().skip(1)) {
            let (x1, y1) = (x2, y2);
            x2 = f64::from(p2.x) / ppi;
            y2 = convy(f64::from(p2.y) / ppi);
            bezier_spline(
                out,
                [
                    x1,
                    y1,
                    cp1.rx / ppi,
                    convy(cp1.ry / ppi),
                    cp2.lx / ppi,
                    convy(cp2.ly / ppi),
                    x2,
                    y2,
                ],
                0,
            );
            cp1 = *cp2;
        }
        out.puts("\t/\n");
        if let Some(a) = &s.for_arrow {
            self.draw_arrow_head(out, cp1.lx / ppi, convy(cp1.ly / ppi), x2, y2, a.ht / ppi, a.wid / ppi);
        }
    }
}

/// `quadratic_spline`: subdivides until the points are closer than
/// `THRESHOLD`, writing each point.
fn quadratic_spline(out: &mut Out, v: [f64; 8], depth: u32) {
    let [x1, y1, a2, b2, a3, b3, x4, y4] = v;
    let xmid = (a2 + a3) / 2.0;
    let ymid = (b2 + b3) / 2.0;
    let deep = depth >= MAX_DEPTH;
    if deep || ((x1 - xmid).abs() < THRESHOLD && (y1 - ymid).abs() < THRESHOLD) {
        fprintf!(out, "\t{:6.3} {:6.3}\n", Cf(xmid * CONVUNIT), Cf(ymid * CONVUNIT));
    } else {
        quadratic_spline(
            out,
            [
                x1,
                y1,
                (x1 + a2) / 2.0,
                (y1 + b2) / 2.0,
                (3.0 * a2 + a3) / 4.0,
                (3.0 * b2 + b3) / 4.0,
                xmid,
                ymid,
            ],
            depth + 1,
        );
    }
    if deep || ((xmid - x4).abs() < THRESHOLD && (ymid - y4).abs() < THRESHOLD) {
        fprintf!(out, "\t{:6.3} {:6.3}\n", Cf(x4 * CONVUNIT), Cf(y4 * CONVUNIT));
    } else {
        quadratic_spline(
            out,
            [
                xmid,
                ymid,
                (a2 + 3.0 * a3) / 4.0,
                (b2 + 3.0 * b3) / 4.0,
                (a3 + x4) / 2.0,
                (b3 + y4) / 2.0,
                x4,
                y4,
            ],
            depth + 1,
        );
    }
}

/// `bezier_spline`: subdivides a cubic Bézier curve until its ends are
/// closer than `THRESHOLD`, writing each end point.
fn bezier_spline(out: &mut Out, v: [f64; 8], depth: u32) {
    let [x0, y0, a1, b1, a2, b2, x3, y3] = v;
    if depth >= MAX_DEPTH || ((x0 - x3).abs() < THRESHOLD && (y0 - y3).abs() < THRESHOLD) {
        fprintf!(out, "\t{:6.3} {:6.3}\n", Cf(x3 * CONVUNIT), Cf(y3 * CONVUNIT));
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
        bezier_spline(out, [x0, y0, sx1, sy1, sx2, sy2, xmid, ymid], depth + 1);
        bezier_spline(out, [xmid, ymid, tx1, ty1, tx2, ty2, x3, y3], depth + 1);
    }
}

impl Driver for Pictex {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        let a = arg.unwrap_or("");
        match opt {
            'f' => default_font_option(&mut self.fonts, ctx, a),
            'l' => self.linethick = a.to_string(),
            'p' => self.plotsymbol = a.to_string(),
            'r' => self.rotate = false,
            'G' | 'L' => {}
            _ => return Err(format!("Argument -{opt} unknown to pictex driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        set_default_size(&mut self.fonts, ctx);
        let title = ctx.name.as_deref().or(ctx.from.as_deref()).unwrap_or("stdin");
        fprintf!(out, "%Title: {}\n", title);
        fprintf!(out, "%%Created by: {} Version {}\n", PROG, PACKAGE_VERSION);
        if let Some(date) = creation_date(ctx) {
            fprintf!(out, "%%CreationDate: {}\n", date);
        }
        if !objects.comments.is_empty() {
            out.puts("%\n");
            print_comments(out, "% ", &objects.comments, "");
            out.puts("%\n");
        }
        out.puts("\\font\\thinlinefont=cmr5\n\\mbox{\\beginpicture\n");
        fprintf!(
            out,
            "\\setcoordinatesystem units <{:6.5}{},{:6.5}{}>\n",
            Cf(ctx.mag),
            UNIT,
            Cf(ctx.mag),
            UNIT
        );
        fprintf!(out, "\\unitlength={:6.5}{}\n", Cf(ctx.mag), UNIT);
        fprintf!(out, "\\linethickness={}\n", self.linethick);
        fprintf!(out, "\\setplotsymbol ({{{}}})\n", self.plotsymbol);
        out.puts("\\setshadesymbol ({\\thinlinefont .})\n\\setlinear\n");
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        let ppi = ctx.ppi;
        print_comments(out, "% ", &a.comments, "");
        out.puts("%\n% Fig CIRCULAR ARC object\n%\n");
        self.set_linewidth(out, ctx, a.thickness);
        self.set_style(out, a.style, a.style_val);
        self.color.set_color(out, ctx, a.pen_color);

        let cx = a.center.x / ppi;
        let cy = convy(a.center.y / ppi);
        let sx = f64::from(a.point[0].x) / ppi;
        let sy = convy(f64::from(a.point[0].y) / ppi);
        let ex = f64::from(a.point[2].x) / ppi;
        let ey = convy(f64::from(a.point[2].y) / ppi);

        if a.type_ == T_OPEN_ARC && a.thickness != 0 {
            if let Some(ar) = &a.for_arrow {
                let (x, y) = arc_tangent(cx, cy, ex, ey, a.direction != 0);
                self.draw_arrow_head(out, x, y, ex, ey, ar.ht / ppi, ar.wid / ppi);
            }
            if let Some(ar) = &a.back_arrow {
                let (x, y) = arc_tangent(cx, cy, sx, sy, a.direction == 0);
                self.draw_arrow_head(out, x, y, sx, sy, ar.ht / ppi, ar.wid / ppi);
            }
        }

        let (_, th1) = rtop(sx - cx, sy - cy);
        let (_, th2) = rtop(ex - cx, ey - cy);
        let mut theta = th2 - th1;
        if theta > 0.0 {
            theta -= 2.0 * PI;
        }
        let (deg, x, y) = if a.direction != 0 {
            (360.0 + (180.0 / PI * theta), sx, sy)
        } else {
            (-180.0 / PI * theta, ex, ey)
        };
        fprintf!(
            out,
            "\\circulararc {:6.3} degrees from {:6.3} {:6.3} center at {:6.3} {:6.3}\n",
            Cf(deg),
            Cf(x * CONVUNIT),
            Cf(y * CONVUNIT),
            Cf(cx * CONVUNIT),
            Cf(cy * CONVUNIT)
        );
        if a.fill_style != UNFILLED {
            ctx.warn("Arc area fill not implemented");
        }
        self.color.reset_color(out, a.pen_color);
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        let ppi = ctx.ppi;
        print_comments(out, "% ", &e.comments, "");
        out.puts("%\n% Fig ELLIPSE\n%\n");
        self.set_linewidth(out, ctx, e.thickness);
        self.set_style(out, e.style, e.style_val);
        self.color.set_color(out, ctx, e.pen_color);

        let (rx, ry) = (f64::from(e.radiuses.x), f64::from(e.radiuses.y));
        let (ccx, ccy) = (f64::from(e.center.x), f64::from(e.center.y));
        if e.fill_style == BLACK_FILL && e.radiuses.x == e.radiuses.y {
            if ctx.mag * rx > 0.5 * ppi / 72.0 * f64::from(MAXBLACKDIAM) {
                ctx.warn(format!(
                    "Too big black filled circle substituted by a diameter of {MAXBLACKDIAM}pt"
                ));
            }
            fprintf!(
                out,
                "\\put{{\\makebox(0,0)[l]{{\\circle*{{{:6.3}}}}}}} at {:6.3} {:6.3}\n",
                Cf((2.0 * rx / ppi) * CONVUNIT),
                Cf(cx(ccx, ppi)),
                Cf(cy(ccy, ppi))
            );
        } else {
            fprintf!(
                out,
                "\\ellipticalarc axes ratio {:6.3}:{:<6.3} 360 degrees \n",
                Cf((rx / ppi) * CONVUNIT),
                Cf((ry / ppi) * CONVUNIT)
            );
            fprintf!(
                out,
                "\tfrom {:6.3} {:6.3} center at {:6.3} {:6.3}\n",
                Cf(cx(ccx + rx, ppi)),
                Cf(cy(ccy, ppi)),
                Cf(cx(ccx, ppi)),
                Cf(cy(ccy, ppi))
            );
            if e.fill_style != UNFILLED {
                ctx.warn("Ellipse area fill not implemented");
            }
        }
        self.color.reset_color(out, e.pen_color);
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        let ppi = ctx.ppi;
        print_comments(out, "% ", &l.comments, "");
        out.puts("%\n% Fig POLYLINE object\n%\n");
        let mut type_ = l.type_;
        if type_ == T_ARC_BOX {
            ctx.warn("Arc box not implemented; substituting box.");
            type_ = T_BOX;
        }
        self.set_linewidth(out, ctx, l.thickness);
        self.set_style(out, l.style, l.style_val);
        self.color.set_color(out, ctx, l.pen_color);

        let pts = &l.points;
        let (Some(&p0), Some(&p1)) = (pts.first(), pts.get(1)) else {
            if let Some(p) = pts.first() {
                // A single point; fig2dev leaves the colour group open.
                let (x, y) = (cx(f64::from(p.x), ppi), cy(f64::from(p.y), ppi));
                fprintf!(
                    out,
                    "\\plot {:6.3} {:6.3} {:6.3} {:6.3} /\n",
                    Cf(x),
                    Cf(y),
                    Cf(x),
                    Cf(y)
                );
            }
            return Ok(());
        };

        if type_ == T_BOX {
            let (mut llx, mut lly, mut urx, mut ury) = (p0.x, p0.y, p0.x, p0.y);
            for q in pts.iter().skip(1) {
                llx = llx.min(q.x);
                lly = lly.min(q.y);
                urx = urx.max(q.x);
                ury = ury.max(q.y);
            }
            self.put_box(out, ctx, [llx, lly, urx, ury], l, type_);
            return Ok(());
        }

        let inch = |p: crate::object::Point| (f64::from(p.x) / ppi, convy(f64::from(p.y) / ppi));
        if let Some(a) = &l.back_arrow {
            let (x1, y1) = inch(p1);
            let (x2, y2) = inch(p0);
            self.draw_arrow_head(out, x1, y1, x2, y2, a.ht / ppi, a.wid / ppi);
        }
        self.set_style(out, l.style, l.style_val);

        let n = pts.len();
        for i in 0..n.saturating_sub(2) {
            let (Some(p), Some(q), Some(r)) = (pts.get(i), pts.get(i + 1), pts.get(i + 2)) else {
                break;
            };
            self.putline(
                out,
                ctx,
                (p.x, p.y),
                (f64::from(q.x), f64::from(q.y)),
                (r.x, r.y),
                (-1, -1),
                (-1, -1),
            );
        }
        let (Some(&p), Some(&q)) = (pts.get(n - 2), pts.get(n - 1)) else {
            return Ok(());
        };
        self.putline(
            out,
            ctx,
            (p.x, p.y),
            (f64::from(q.x), f64::from(q.y)),
            (-1, -1),
            (p0.x, p0.y),
            (p1.x, p1.y),
        );
        if let Some(a) = &l.for_arrow {
            let (x1, y1) = inch(p);
            let (x2, y2) = inch(q);
            self.draw_arrow_head(out, x1, y1, x2, y2, a.ht / ppi, a.wid / ppi);
        }
        if l.fill_style != UNFILLED {
            ctx.warn("Line area fill not implemented");
        }
        self.color.reset_color(out, l.pen_color);
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        print_comments(out, "% ", &s.comments, "");
        out.puts("%\n% Fig SPLINE\n%\n");
        self.set_linewidth(out, ctx, s.thickness);
        self.set_style(out, s.style, s.style_val);
        self.color.set_color(out, ctx, s.pen_color);
        if s.int_spline() {
            self.itp_spline(out, ctx, s);
        } else {
            self.ctl_spline(out, ctx, s);
        }
        if s.fill_style != UNFILLED {
            ctx.warn("Spline area fill not implemented");
        }
        self.color.reset_color(out, s.pen_color);
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String> {
        print_comments(out, "% ", &t.comments, "");
        out.puts("%\n% Fig TEXT object\n%\n");
        let x = f64::from(t.base_x) / ctx.ppi;
        let y = convy(f64::from(t.base_y) / ctx.ppi);
        let tpos = match t.type_ {
            T_LEFT_JUSTIFIED | DEFAULT => "[lB]",
            T_CENTER_JUSTIFIED => "[B]",
            T_RIGHT_JUSTIFIED => "[rB]",
            _ => {
                ctx.warn("Text incorrectly positioned");
                return Ok(());
            }
        };
        let mut t = t.clone();
        self.fonts.select_font(out, ctx, &mut t, true, true, true);
        if self.rotate && t.angle != 0.0 && t.type_ == T_LEFT_JUSTIFIED {
            fprintf!(
                out,
                "\\special{{ps:gsave currentpoint currentpoint translate\n-{:.1} rotate neg exch neg exch translate}}",
                Cf(t.angle * 180.0 / PI)
            );
        }
        self.color.set_color(out, ctx, t.color);
        put_string(out, ctx, &t.raw, t.font, t.special_text());
        self.color.reset_color(out, t.color);
        if self.rotate && t.angle != 0.0 {
            if t.type_ == T_LEFT_JUSTIFIED {
                out.puts("\\special{ps:currentpoint grestore moveto}");
            } else {
                ctx.warn("Rotated Text only for left justified text");
            }
        }
        fprintf!(
            out,
            "}} {} at {:6.3} {:6.3}\n",
            tpos,
            Cf(x * CONVUNIT),
            Cf(y * CONVUNIT)
        );
        Ok(())
    }

    fn end(&mut self, ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        let ppi = ctx.ppi;
        out.puts("\\linethickness=0pt\n");
        fprintf!(
            out,
            "\\putrectangle corners at {:6.3} {:6.3} and {:6.3} {:6.3}\n",
            Cf(cx(f64::from(ctx.llx), ppi)),
            Cf(cy(f64::from(ctx.lly), ppi)),
            Cf(cx(f64::from(ctx.urx), ppi)),
            Cf(cy(f64::from(ctx.ury), ppi))
        );
        out.puts("\\endpicture}\n");
        Ok(0)
    }

    fn text_include(&self) -> bool {
        false
    }
}
