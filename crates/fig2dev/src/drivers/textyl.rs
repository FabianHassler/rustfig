//! fig2dev's `textyl` driver: the figure as TeXtyl `\special{tyl …}`
//! commands, in scaled points measured from the top of the figure.
//!
//! Lines, arcs, circles and ellipses become `tyl line` and `tyl arc`
//! specials, splines are approximated by lines, arrowheads are two lines,
//! texts `tyl label`s. Line widths and fills are not written.
//!
//! Options: `-f` the default font, `-l` the line thickness (1 to 12), `-a`
//! (ignored).
//
// Ported from fig2dev dev/gentextyl.c and localmath.c (xfig licence, see
// LICENSE-xfig).

use std::f64::consts::PI;

use super::latex::{c_string, default_font_option};
use super::pictex::{arc_tangent, rtop};
use crate::gendev::{print_comments, strtol, Ctx, DEFAULT_FONT_SIZE};
use crate::object::{
    Arc, Compound, Ellipse, Line, Spline, Text, DASH_LINE, DEFAULT, DOTTED_LINE, SOLID_LINE, T_LEFT_JUSTIFIED,
    T_OPEN_ARC,
};
use crate::util::texfonts::TexFonts;
use crate::{fprintf, Driver, Out};

/// Scaled points per inch.
const SCALE: f64 = 65536.0 * 72.27;
/// The unit of the specials: scaled points.
const MEASURE: char = 'S';
/// Splines are drawn with lines shorter than this, Fig units.
const THRESHOLD: f64 = 10.0;
/// Splines are subdivided at most this deep (not in C; reached only by
/// segments over 8,000 inches long).
const MAX_DEPTH: u32 = 20;

/// The `textyl` driver's state (gentextyl.c's statics).
#[derive(Debug)]
pub struct Textyl {
    fonts: TexFonts,
    /// TeXtyl's line style: 0 solid, 1 dotted, 2 dashed.
    line_style: i32,
    /// 1 to 12 "pixels".
    linethick: i32,
    last_x: f64,
    last_y: f64,
    ury: i32,
    ppi: f64,
}

impl Default for Textyl {
    fn default() -> Textyl {
        Textyl {
            fonts: TexFonts::new(),
            line_style: 0,
            linethick: 2,
            last_x: 0.0,
            last_y: 0.0,
            ury: 0,
            ppi: 1200.0,
        }
    }
}

/// The driver for `lang`.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    (lang == "textyl").then(|| Box::new(Textyl::default()) as Box<dyn Driver>)
}

impl Textyl {
    /// `convx`: Fig units to scaled points (through a `float`, as in C),
    /// written as C's `%u` writes an `int`.
    fn convx(&self, a: f64) -> u32 {
        let f = (a * SCALE) as f32;
        (f64::from(f) / self.ppi) as i32 as u32
    }

    /// `convy`: from the top of the figure.
    fn convy(&self, a: f64) -> u32 {
        (((f64::from(self.ury) - a) * SCALE) / self.ppi) as i32 as u32
    }

    /// `set_style`.
    fn set_style(&mut self, style: i32) {
        match style {
            SOLID_LINE => self.line_style = 0,
            DASH_LINE => self.line_style = 2,
            DOTTED_LINE => self.line_style = 1,
            _ => {}
        }
    }

    /// `putline`.
    fn putline(&self, out: &mut Out, x1: i32, y1: i32, x2: i32, y2: i32) {
        fprintf!(
            out,
            "\\special{{tyl line {} {} L {} {} {}; {} {}}}\n",
            MEASURE,
            self.linethick,
            self.line_style,
            self.convx(f64::from(x1)),
            self.convy(f64::from(y1)),
            self.convx(f64::from(x2)),
            self.convy(f64::from(y2))
        );
    }

    /// A line without style from the last point to (x, y), which becomes
    /// the last point; `sep` separates the two points.
    fn line_from_last(&mut self, out: &mut Out, x: f64, y: f64, sep: &str) {
        fprintf!(
            out,
            "\\special{{tyl line {} {} {} {}{}{} {}}}\n",
            MEASURE,
            self.linethick,
            self.convx(self.last_x),
            self.convy(self.last_y),
            sep,
            self.convx(x),
            self.convy(y)
        );
        self.last_x = x;
        self.last_y = y;
    }

    /// `draw_arrow_head`: an open arrowhead from (x1, y1) to (x2, y2).
    #[allow(clippy::too_many_arguments)]
    fn draw_arrow_head(&self, out: &mut Out, x1: f64, y1: f64, x2: f64, y2: f64, arrowht: f64, arrowwid: f64) {
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
        out.puts("%\n% arrow head\n%\n");
        for (a, b, c, d) in [(xc, yc, x2, y2), (x2, y2, xd, yd)] {
            fprintf!(
                out,
                "\\special{{tyl line {} {} {} {}; {} {}}}\n",
                MEASURE,
                self.linethick,
                self.convx(a),
                self.convy(b),
                self.convx(c),
                self.convy(d)
            );
        }
    }

    /// `quadratic_spline`.
    fn quadratic_spline(&mut self, out: &mut Out, v: [f64; 8], depth: u32) {
        let [x1, y1, a2, b2, a3, b3, x4, y4] = v;
        let xmid = (a2 + a3) / 2.0;
        let ymid = (b2 + b3) / 2.0;
        let deep = depth >= MAX_DEPTH;
        if deep || ((x1 - xmid).abs() < THRESHOLD && (y1 - ymid).abs() < THRESHOLD) {
            self.line_from_last(out, xmid, ymid, "; ");
        } else {
            self.quadratic_spline(
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
            self.line_from_last(out, x4, y4, "; ");
        } else {
            self.quadratic_spline(
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

    /// `bezier_spline`.
    fn bezier_spline(&mut self, out: &mut Out, v: [f64; 8], depth: u32) {
        let [x0, y0, a1, b1, a2, b2, x3, y3] = v;
        if depth >= MAX_DEPTH || ((x0 - x3).abs() < THRESHOLD && (y0 - y3).abs() < THRESHOLD) {
            self.line_from_last(out, x3, y3, " ; ");
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

    /// `gentextyl_ctl_spline`.
    fn ctl_spline(&mut self, out: &mut Out, s: &Spline) {
        out.puts("%\n% Fig CONTROL PT SPLINE\n%\n");
        let pt = |i: usize| s.points.get(i).map(|p| (f64::from(p.x), f64::from(p.y)));
        let (Some((mut x1, mut y1)), Some((mut x2, mut y2))) = (pt(0), pt(1)) else {
            return;
        };
        let mut cx1 = (x1 + x2) / 2.0;
        let mut cy1 = (y1 + y2) / 2.0;
        let mut cx2 = (x1 + 3.0 * x2) / 4.0;
        let mut cy2 = (y1 + 3.0 * y2) / 4.0;
        if s.closed_spline() {
            out.puts("% closed spline\n%\n");
        } else {
            out.puts("% open spline\n%\n");
            if let Some(a) = &s.back_arrow {
                self.draw_arrow_head(out, cx1, cy1, x1, y1, a.ht, a.wid);
            }
            fprintf!(
                out,
                "\\special{{tyl line {} {} {} {};{} {}}}\n",
                MEASURE,
                self.linethick,
                self.convx(x1),
                self.convy(y1),
                self.convx(cx1),
                self.convy(cy1)
            );
        }
        self.last_x = cx1;
        self.last_y = cy1;

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
            self.quadratic_spline(out, [cx1, cy1, cx2, cy2, cx3, cy3, cx4, cy4], 0);
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
            self.quadratic_spline(out, [cx1, cy1, cx2, cy2, cx3, cy3, cx4, cy4], 0);
        } else {
            fprintf!(
                out,
                "\\special{{tyl line {} {} {} {};{} {}}}\n",
                MEASURE,
                self.linethick,
                self.convx(cx1),
                self.convy(cy1),
                self.convx(x1),
                self.convy(y1)
            );
            if let Some(a) = &s.for_arrow {
                self.draw_arrow_head(out, cx1, cy1, x1, y1, a.ht, a.wid);
            }
        }
    }

    /// `gentextyl_itp_spline`.
    fn itp_spline(&mut self, out: &mut Out, s: &Spline) {
        let (Some(p1), Some(cp1)) = (s.points.first(), s.controls.first()) else {
            return;
        };
        let mut x2 = f64::from(p1.x);
        let mut y2 = f64::from(p1.y);
        if let Some(a) = &s.back_arrow {
            self.draw_arrow_head(out, cp1.rx, cp1.ry, x2, y2, a.ht, a.wid);
        }
        self.last_x = x2;
        self.last_y = y2;
        out.puts("%\n% Fig INTERPOLATED SPLINE\n%\n");
        let mut cp1 = *cp1;
        for (p2, cp2) in s.points.iter().skip(1).zip(s.controls.iter().skip(1)) {
            let (x1, y1) = (x2, y2);
            x2 = f64::from(p2.x);
            y2 = f64::from(p2.y);
            self.bezier_spline(out, [x1, y1, cp1.rx, cp1.ry, cp2.lx, cp2.ly, x2, y2], 0);
            cp1 = *cp2;
        }
        if let Some(a) = &s.for_arrow {
            self.draw_arrow_head(out, cp1.lx, cp1.ly, x2, y2, a.ht, a.wid);
        }
    }
}

/// `fill_style && fill_style != DEFAULT`.
fn filled(fill_style: i32) -> bool {
    fill_style != 0 && fill_style != DEFAULT
}

impl Driver for Textyl {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        let a = arg.unwrap_or("");
        match opt {
            'a' => {}
            'f' => default_font_option(&mut self.fonts, ctx, a),
            'l' => {
                self.linethick = strtol(a).0;
                if !(1..=12).contains(&self.linethick) {
                    return Err("Line thickness must be between 1 and 12.".into());
                }
            }
            'G' | 'L' => {}
            _ => return Err(format!("Argument -{opt} unknown to textyl driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        self.ury = ctx.ury;
        self.ppi = ctx.ppi;
        // texfontsizes[(font_size or 11) + 1]; textyl writes no fonts.
        let fs = if ctx.font_size != 0.0 {
            ctx.font_size as i32
        } else {
            DEFAULT_FONT_SIZE as i32
        };
        let s = usize::try_from(fs.saturating_add(1))
            .ok()
            .and_then(|i| self.fonts.sizes.get(i).copied())
            .unwrap_or(self.fonts.sizes[0]);
        self.fonts.sizes[0] = s;
        self.fonts.sizes[1] = s;

        if !objects.comments.is_empty() {
            out.puts("%\n");
            print_comments(out, "% ", &objects.comments, "");
            out.puts("%\n");
        }
        fprintf!(out, "\\begintyl{{{:.6}sp}}\n", f64::from(ctx.ury) / ctx.ppi * SCALE);
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        print_comments(out, "% ", &a.comments, "");
        self.set_style(a.style);
        let (cx, cy) = (a.center.x, a.center.y);
        let (sx, sy) = (f64::from(a.point[0].x), f64::from(a.point[0].y));
        let (ex, ey) = (f64::from(a.point[2].x), f64::from(a.point[2].y));
        if a.type_ == T_OPEN_ARC && a.thickness != 0 {
            if let Some(ar) = &a.for_arrow {
                let (x, y) = arc_tangent(cx, cy, ex, ey, a.direction == 0);
                self.draw_arrow_head(out, x, y, ex, ey, ar.ht, ar.wid);
            }
            if let Some(ar) = &a.back_arrow {
                let (x, y) = arc_tangent(cx, cy, sx, sy, a.direction != 0);
                self.draw_arrow_head(out, x, y, sx, sy, ar.ht, ar.wid);
            }
        }
        // Angles with y up.
        let ury = f64::from(ctx.ury);
        let (r1, th1) = rtop(sx - cx, (ury - sy) - (ury - cy));
        let (_, th2) = rtop(ex - cx, (ury - ey) - (ury - cy));
        let (t1, t2) = if a.direction != 0 { (th1, th2) } else { (th2, th1) };
        fprintf!(
            out,
            "\\special{{tyl arc {} {} L 0 {} @ {},{} {} {}}}\n",
            MEASURE,
            self.linethick,
            self.convx(r1),
            self.convx(cx),
            self.convy(cy),
            (180.0 / PI * t1) as i32,
            (180.0 / PI * t2) as i32
        );
        if filled(a.fill_style) {
            ctx.warn("Arc area fill not implemented");
        }
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        print_comments(out, "% ", &e.comments, "");
        out.puts("%\n% Fig ELLIPSE\n%\n");
        self.set_style(e.style);
        let (rx, ry) = (e.radiuses.x, e.radiuses.y);
        let (cx, cy) = (f64::from(e.center.x), f64::from(e.center.y));
        if rx == ry {
            fprintf!(
                out,
                "\\special{{tyl arc {} {} L 0 {} @ {},{} 0 360}}\n",
                MEASURE,
                self.linethick,
                self.convx(f64::from(rx)),
                self.convx(cx),
                self.convy(cy)
            );
        } else {
            let ratio = |a: i32, b: i32| (f64::from(a as f32 / b as f32) * 100.0) as i32 as u32;
            let (sx, sy, radius) = if rx > ry {
                (ratio(rx, ry), 100, ry)
            } else {
                (100, ratio(ry, rx), rx)
            };
            fprintf!(
                out,
                "\\special{{tyl arc {} T {} {} 0 0 0 {} L 0 {} @ {},{} 0 360}}\n",
                MEASURE,
                sx,
                sy,
                self.linethick,
                self.convx(f64::from(radius)),
                self.convx(cx),
                self.convy(cy)
            );
            if filled(e.fill_style) {
                ctx.warn("Ellipse area fill not implemented");
            }
        }
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        print_comments(out, "% ", &l.comments, "");
        out.puts("%\n% Fig POLYLINE object\n%\n");
        self.set_style(l.style);
        let pts = &l.points;
        let (Some(&p0), Some(&p1)) = (pts.first(), pts.get(1)) else {
            if let Some(p) = pts.first() {
                self.putline(out, p.x, p.y, p.x, p.y);
            }
            return Ok(());
        };
        if let Some(a) = &l.back_arrow {
            self.draw_arrow_head(
                out,
                f64::from(p1.x),
                f64::from(p1.y),
                f64::from(p0.x),
                f64::from(p0.y),
                a.ht,
                a.wid,
            );
        }
        for seg in pts.windows(2) {
            if let (Some(p), Some(q)) = (seg.first(), seg.get(1)) {
                self.putline(out, p.x, p.y, q.x, q.y);
            }
        }
        if let (Some(a), [.., p, q]) = (&l.for_arrow, pts.as_slice()) {
            self.draw_arrow_head(
                out,
                f64::from(p.x),
                f64::from(p.y),
                f64::from(q.x),
                f64::from(q.y),
                a.ht,
                a.wid,
            );
        }
        if filled(l.fill_style) {
            ctx.warn("Line area fill not implemented");
        }
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        print_comments(out, "% ", &s.comments, "");
        self.set_style(s.style);
        if s.int_spline() {
            self.itp_spline(out, s);
        } else {
            self.ctl_spline(out, s);
        }
        if filled(s.fill_style) {
            ctx.warn("Spline area fill not implemented");
        }
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String> {
        print_comments(out, "% ", &t.comments, "");
        out.puts("%\n% Fig TEXT object\n%\n");
        if t.type_ != T_LEFT_JUSTIFIED && t.type_ != DEFAULT {
            ctx.warn("Warning: Text incorrectly positioned");
        }
        fprintf!(
            out,
            "\\special{{tyl label {} 1 {} {} \"{}\"}}\n",
            MEASURE,
            self.convx(f64::from(t.base_x)),
            self.convy(f64::from(t.base_y)),
            c_string(&t.cstring)
        );
        Ok(())
    }

    fn end(&mut self, _ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        out.puts("\\endtyl\n");
        Ok(0)
    }

    fn text_include(&self) -> bool {
        false
    }
}
