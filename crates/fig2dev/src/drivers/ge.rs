//! fig2dev's `ge` driver: the Graphical Editor format of the SPARK
//! (Simulation Problem Analysis Research Kernel) tools. Polylines (`p`),
//! arcs (`A`), ellipses (`E`) and texts become one record each, with
//! attributes for width, style, colour, fill and arrowheads; coordinates
//! are in 0.1 mm. Splines are written as PostScript-like path code, as
//! fig2dev writes them. No options besides fig2dev's general ones.
//
// Ported from fig2dev dev/genge.c (xfig licence, see LICENSE-xfig).

use super::cgm::{text_bytes, text_encoding, TextEnc};
use crate::gendev::{print_comments, Ctx, Driver};
use crate::object::{
    Arc, Arrow, Compound, Ellipse, Line, Spline, Text, DASH_2_DOTS_LINE, DASH_3_DOTS_LINE, DASH_DOT_LINE, DASH_LINE,
    DOTTED_LINE, T_OPEN_ARC, T_PIC_BOX, UNFILLED,
};
use crate::{fprintf, round, Out};

/// The driver for `lang`.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    let _ = lang;
    Some(Box::new(Ge::default()))
}

const MAX_FONT_SIZE: i32 = 31;

/// Fig colour (from DEFAULT, −1) → ge colour (`GE_COLORS`).
const GE_COLORS: [i32; 33] = [
    1, 1, 8, 7, 6, 5, 4, 3, 2, 14, 21, 20, 12, 22, 16, 13, 23, 23, 18, 9, 15, 24, 25, 26, 27, 28, 29, 30, 10, 17, 31,
    11, 19,
];

#[derive(Debug)]
struct Ge {
    /// Fig units → 0.1 mm.
    scale: f32,
    enc: TextEnc,
}

impl Default for Ge {
    fn default() -> Ge {
        Ge {
            scale: 254.0 / 1200.0,
            enc: TextEnc::Utf8,
        }
    }
}

/// The ge colour of a Fig colour. fig2dev indexes its table unchecked;
/// user colours, beyond it, are black here.
fn ge_color(col: i32) -> i32 {
    usize::try_from(col.saturating_add(1))
        .ok()
        .and_then(|i| GE_COLORS.get(i))
        .copied()
        .unwrap_or(1)
}

fn set_color(out: &mut Out, col: i32) {
    fprintf!(out, "c{:02} ", ge_color(col));
}

fn set_fill(out: &mut Out, style: i32, color: i32) {
    if style != UNFILLED {
        fprintf!(out, "C{:02} ", ge_color(color));
    }
}

fn set_style(out: &mut Out, s: i32) {
    if s == DASH_LINE {
        out.puts("y02 ");
    } else if s == DOTTED_LINE || s == DASH_DOT_LINE || s == DASH_2_DOTS_LINE || s == DASH_3_DOTS_LINE {
        out.puts("y03 ");
    } else {
        out.puts("y01 ");
    }
}

fn set_linewidth(out: &mut Out, w: i32) {
    fprintf!(out, "s{:02} ", w);
}

/// `(int)(v/15)`, saturating.
fn arrow_size(v: f64) -> i32 {
    let q = v / 15.0;
    if q.is_finite() {
        q as i32
    } else {
        0
    }
}

fn for_arrow(out: &mut Out, a: &Arrow) {
    fprintf!(out, "w{}{} ", arrow_size(a.wid), arrow_size(a.ht));
}

fn back_arrow(out: &mut Out, a: &Arrow) {
    fprintf!(out, "v{}{} ", arrow_size(a.wid), arrow_size(a.ht));
}

impl Ge {
    /// `SCALE(v)`: `round(v*scale)`, in single precision as fig2dev.
    fn sc(&self, v: i32) -> i32 {
        round(f64::from(v as f32 * self.scale))
    }

    /// `genge_itp_spline`. fig2dev first walks its control point pointer
    /// to the last control point and then reads past it, so it crashes on
    /// every interpolated spline; here the controls are taken in order.
    fn itp_spline(out: &mut Out, s: &Spline) {
        let Some(p0) = s.points.first() else { return };
        fprintf!(out, "n {} {} m\n", p0.x, p0.y);
        for (k, q) in s.points.iter().enumerate().skip(1) {
            let a = s.controls.get(k - 1).copied().unwrap_or_default();
            let b = s.controls.get(k).copied().unwrap_or_default();
            fprintf!(
                out,
                "\t{:.1} {:.1} {:.1} {:.1} {} {} curveto\n",
                a.rx,
                a.ry,
                b.lx,
                b.ly,
                q.x,
                q.y
            );
        }
    }

    /// `genge_ctl_spline`.
    fn ctl_spline(out: &mut Out, s: &Spline) {
        let closed = s.closed_spline();
        if closed {
            out.puts("% Closed spline\n");
        } else {
            out.puts("% Open spline\n");
        }
        let (Some(p0), Some(p1)) = (s.points.first(), s.points.get(1)) else {
            return;
        };
        set_style(out, s.style);
        let (x1, y1) = (f64::from(p0.x), f64::from(p0.y));
        let (mut c, mut d) = (f64::from(p1.x), f64::from(p1.y));
        let (a, b) = ((x1 + c) / 2.0, (y1 + d) / 2.0);
        let (mut x3, mut y3) = (a, b);
        if closed {
            fprintf!(out, "n {:.1} {:.1} m\n", a, b);
        } else {
            fprintf!(out, "n {:.1} {:.1} m {:.1} {:.1} l\n", x1, y1, x3, y3);
        }
        for q in s.points.iter().skip(2) {
            let (x1, y1) = (x3, y3);
            let (x2, y2) = (c, d);
            c = f64::from(q.x);
            d = f64::from(q.y);
            x3 = (x2 + c) / 2.0;
            y3 = (y2 + d) / 2.0;
            fprintf!(
                out,
                "\t{:.1} {:.1} {:.1} {:.1} {:.1} {:.1} DrawSplineSection\n",
                x1,
                y1,
                x2,
                y2,
                x3,
                y3
            );
        }
        if closed {
            fprintf!(
                out,
                "\t{:.1} {:.1} {:.1} {:.1} {:.1} {:.1} DrawSplineSection closepath ",
                x3,
                y3,
                c,
                d,
                a,
                b
            );
        } else {
            fprintf!(out, "\t{:.1} {:.1} l ", c, d);
        }
    }
}

impl Driver for Ge {
    fn option(&mut self, _ctx: &mut Ctx, opt: char, _arg: Option<&str>) -> Result<(), String> {
        match opt {
            'G' | 'L' => Ok(()),
            _ => Err(format!("Argument -{opt} unknown to ge driver.")),
        }
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        self.scale = (254.0 / ctx.ppi) as f32;
        self.enc = text_encoding(ctx);
        print_comments(out, "# ", &objects.comments, "");
        Ok(())
    }

    fn arc(&mut self, _ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        print_comments(out, "# ", &a.comments, "");
        out.puts("A ");
        set_linewidth(out, a.thickness);
        set_style(out, a.style);
        set_color(out, a.pen_color);
        set_fill(out, a.fill_style, a.fill_color);
        let arrows = a.type_ == T_OPEN_ARC && a.thickness > 0;
        if let Some(b) = a.back_arrow.as_ref().filter(|_| arrows) {
            back_arrow(out, b);
        }
        for p in &a.point {
            fprintf!(out, "({},{}) ", self.sc(p.x), self.sc(p.y));
        }
        if let Some(f) = a.for_arrow.as_ref().filter(|_| arrows) {
            for_arrow(out, f);
        }
        out.puts(" ;\n");
        Ok(())
    }

    fn ellipse(&mut self, _ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        print_comments(out, "# ", &e.comments, "");
        out.puts("E ");
        set_linewidth(out, e.thickness);
        set_style(out, e.style);
        set_color(out, e.pen_color);
        set_fill(out, e.fill_style, e.fill_color);
        let (c, r) = (e.center, e.radiuses);
        fprintf!(
            out,
            "({},{}) ({},{})\n",
            self.sc(c.x.saturating_sub(r.x)),
            self.sc(c.y.saturating_sub(r.y)),
            self.sc(c.x.saturating_add(r.x)),
            self.sc(c.y.saturating_add(r.y))
        );
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        if l.type_ == T_PIC_BOX {
            ctx.warn("Warning: Pictures not supported in GE language");
            return Ok(());
        }
        print_comments(out, "# ", &l.comments, "");
        out.puts("p ");
        set_linewidth(out, l.thickness);
        set_style(out, l.style);
        set_color(out, l.pen_color);
        set_fill(out, l.fill_style, l.fill_color);
        let arrows = l.thickness > 0;
        if let Some(b) = l.back_arrow.as_ref().filter(|_| arrows) {
            back_arrow(out, b);
        }
        let Some((last, rest)) = l.points.split_last() else {
            return Ok(());
        };
        if rest.is_empty() {
            fprintf!(out, "({},{});\n", self.sc(last.x), self.sc(last.y));
            return Ok(());
        }
        for (i, p) in rest.iter().enumerate() {
            fprintf!(out, "({},{}) ", self.sc(p.x), self.sc(p.y));
            if i > 0 && i % 5 == 0 {
                out.puts("\n  ");
            }
        }
        if let Some(f) = l.for_arrow.as_ref().filter(|_| arrows) {
            for_arrow(out, f);
        }
        fprintf!(out, "({},{});\n", self.sc(last.x), self.sc(last.y));
        Ok(())
    }

    fn spline(&mut self, _ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        print_comments(out, "# ", &s.comments, "");
        set_linewidth(out, s.thickness);
        set_style(out, s.style);
        set_color(out, s.pen_color);
        set_fill(out, s.fill_style, s.fill_color);
        let arrows = s.thickness > 0;
        if let Some(b) = s.back_arrow.as_ref().filter(|_| arrows) {
            back_arrow(out, b);
        }
        if s.int_spline() {
            Ge::itp_spline(out, s);
        } else {
            Ge::ctl_spline(out, s);
        }
        if let Some(f) = s.for_arrow.as_ref().filter(|_| arrows) {
            for_arrow(out, f);
        }
        Ok(())
    }

    fn text(&mut self, _ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String> {
        print_comments(out, "# ", &t.comments, "");
        out.puts("\"");
        out.bytes(&text_bytes(self.enc, &t.cstring));
        out.puts("\" ");
        set_color(out, t.color);
        // C's `%` keeps the sign: font −1 (default PostScript font) gives
        // −1, for which fig2dev leaves the style uninitialised; normal here.
        let style = match t.font % 4 {
            1 => 3,
            2 | 3 => 2,
            _ => 1,
        };
        let size = if t.size.is_finite() { t.size as i32 } else { 0 };
        fprintf!(out, "f{:02} z{:02} y{} ", t.font, MAX_FONT_SIZE.min(size), style);
        fprintf!(out, "({},{}) ", self.sc(t.base_x), self.sc(t.base_y));
        out.puts("l0 ;\n");
        Ok(())
    }

    fn end(&mut self, _ctx: &mut Ctx, _out: &mut Out) -> Result<i32, String> {
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}
