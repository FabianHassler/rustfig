//! fig2dev's `ge` driver: the Graphical Editor format of the SPARK
//! (Simulation Problem Analysis Research Kernel) tools. Polylines (`p`),
//! arcs (`A`), ellipses (`E`) and texts become one record each, with
//! attributes for width, style, colour, fill and arrowheads; coordinates
//! are in 0.1 mm. Splines are written as PostScript-like path code, as
//! fig2dev writes them. No options besides fig2dev's general ones.
//
// Ported from fig2dev dev/genge.c (xfig licence, see LICENSE-xfig).

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
}

impl Default for Ge {
    fn default() -> Ge {
        Ge { scale: 254.0 / 1200.0 }
    }
}

/// What the reference binary (fig2dev 3.2.9a, arm64 macOS) holds right after
/// C's `GE_COLORS[33]`: user colours 32.. (and fill colours above 31) index
/// past the table, and C prints whatever data follows it (further statics and
/// halves of double constants, hence the odd numbers). Recorded from the
/// reference for colours 32..543 so the output matches it; beyond, black.
const GE_COLORS_PAST_END: [i32; 512] = [
    3,
    2,
    2,
    0,
    1083162624,
    0,
    1080459264,
    1413754136,
    -1072094725,
    1256670343,
    -1078569192,
    1413754136,
    -1074191877,
    5,
    0,
    0,
    5,
    1,
    0,
    5,
    0,
    3,
    5,
    1,
    3,
    31,
    0,
    0,
    31,
    1,
    0,
    31,
    0,
    3,
    31,
    1,
    3,
    47,
    0,
    0,
    47,
    1,
    0,
    47,
    0,
    3,
    47,
    1,
    3,
    3,
    0,
    0,
    3,
    1,
    0,
    3,
    0,
    3,
    3,
    1,
    3,
    4,
    0,
    0,
    4,
    1,
    0,
    4,
    0,
    3,
    4,
    1,
    3,
    4,
    0,
    0,
    4,
    1,
    0,
    4,
    0,
    3,
    4,
    1,
    3,
    23,
    0,
    0,
    23,
    1,
    0,
    23,
    0,
    3,
    23,
    1,
    3,
    15,
    0,
    0,
    15,
    1,
    0,
    15,
    0,
    3,
    15,
    1,
    3,
    52,
    0,
    0,
    43,
    0,
    0,
    45,
    0,
    0,
    0,
    -1374389535,
    1095897415,
    402653184,
    1107456176,
    0,
    1086556160,
    905110723,
    1070007895,
    547345355,
    1071749573,
    0,
    1081507840,
    0,
    1079410688,
    0,
    -1068072960,
    0,
    1,
    0,
    1079410688,
    1,
    0,
    0,
    0,
    1,
    1,
    0,
    1078362112,
    1,
    2,
    1730998558,
    1078966188,
    1,
    3,
    -865499279,
    1079108649,
    1,
    4,
    803251194,
    1079180718,
    2,
    1,
    832970180,
    1077579943,
    2,
    3,
    -569904818,
    1078732715,
    3,
    1,
    -832970180,
    1077047128,
    3,
    2,
    569904818,
    1077991508,
    3,
    4,
    832970180,
    1078628519,
    4,
    1,
    -2131042258,
    1076630158,
    4,
    3,
    -832970180,
    1078095704,
    0,
    1,
    0,
    1079410688,
    1,
    0,
    0,
    0,
    1,
    1,
    0,
    1078362112,
    1,
    2,
    1730998558,
    1078966188,
    1,
    3,
    -865499279,
    1079108649,
    1,
    4,
    803251194,
    1079180718,
    1,
    5,
    284952409,
    1079225386,
    1,
    6,
    1344082993,
    1079255657,
    2,
    1,
    832970180,
    1077579943,
    2,
    3,
    -569904818,
    1078732715,
    2,
    5,
    -1258530164,
    1079053493,
    3,
    1,
    -832970180,
    1077047128,
    3,
    2,
    569904818,
    1077991508,
    3,
    4,
    832970180,
    1078628519,
    3,
    5,
    -1606502389,
    1078822051,
    4,
    1,
    -2131042258,
    1076630158,
    4,
    3,
    -832970180,
    1078095704,
    4,
    5,
    1731430063,
    1078569867,
    5,
    1,
    2015348023,
    1076272815,
    5,
    2,
    739153359,
    1077267753,
    5,
    3,
    -1081962519,
    1077868216,
    5,
    4,
    -1731430063,
    1078154356,
    5,
    6,
    199495450,
    1078532323,
    6,
    1,
    2132237947,
    1076030645,
    6,
    5,
    -199495450,
    1078191900,
    -2061584302,
    1072714219,
    1256670343,
    1071011608,
    0,
    1082703872,
    0,
    -1064124416,
    1717986918,
    1074423398,
    1414677840,
    1074340347,
    381774871,
    1063698796,
    0,
    1085267968,
    -1717986918,
    1071225241,
    -858993459,
    1071434956,
    996432413,
    1070944223,
    0,
    1087373184,
    1413754136,
    -1073143301,
    0,
    -1065975808,
    1413754136,
    1072243195,
    1413754136,
    -1075240453,
    2134057426,
    -1073555076,
    693450846,
    -1072731526,
    0,
    1079656448,
    -1717986918,
    1074370969,
    -2061584302,
    1073238507,
    0,
    1083129856,
    0,
    0,
    2006013112,
    -1076150471,
    -1611224339,
    1072722411,
    2006013112,
    1071333177,
    -1611224339,
    1072722411,
    -1611224339,
    1072722411,
    2006013112,
    1071333177,
    -1611224339,
    1072722411,
    2006013112,
    -1076150471,
    2006013112,
    1071333177,
    -1611224339,
    -1074761237,
    2006013112,
    -1076150471,
    -1611224339,
    -1074761237,
    -1611224339,
    -1074761237,
    2006013112,
    -1076150471,
    -1611224339,
    -1074761237,
    2006013112,
    1071333177,
    1655769888,
    -1076153931,
    1206439134,
    1072720323,
    1655769888,
    1071329717,
    1206439134,
    1072720323,
    1206439134,
    1072720323,
    1655769888,
    1071329717,
    1206439134,
    1072720323,
    1655769888,
    -1076153931,
    1655769888,
    -1076153931,
    1206439134,
    -1074763325,
    1206439134,
    -1074763325,
    1655769888,
    -1076153931,
    1206439134,
    -1074763325,
    1655769888,
    1071329717,
    1655769888,
    1071329717,
    1206439134,
    -1074763325,
    0,
    0,
    0,
    0,
    0,
    -1074790400,
    -347170796,
    1071753270,
    0,
    -1074790400,
    0,
    1072693248,
    -347170796,
    -1075730378,
    0,
    1072693248,
    0,
    0,
    0,
    1072693248,
    -347170796,
    1071753270,
    -347170796,
    1071753270,
    0,
    1072693248,
    0,
    0,
    0,
    1072693248,
    -347170796,
    -1075730378,
    0,
    1072693248,
    0,
    -1074790400,
    -347170796,
    1071753270,
    0,
    -1074790400,
    0,
    0,
    0,
    -1074790400,
    -347170796,
    -1075730378,
    -347170796,
    -1075730378,
    0,
    -1074790400,
    0,
    0,
    0,
    -1074790400,
    442745336,
    -1068718628,
    -953655296,
    -944603961,
    -1295530041,
    -1299220113,
    315065543,
    -943208505,
    -943208526,
    -943208505,
    -952690489,
    -954187433,
    147292359,
    -1792146233,
    -943208505,
    -1211643961,
    1229563550,
    1291845674,
    2752589,
    0,
    -1717986918,
    -1078355559,
    1199842064,
    1072809755,
    -858993459,
    1079184588,
    0,
    1089470432,
    -1717986918,
    1069128089,
    -755914244,
    1062232653,
    0,
    -1067024384,
    0,
    1081262080,
    -1758569287,
    1079174603,
    0,
    -1068072960,
    0,
    1079410688,
    473460962,
    1081968398,
    1889785610,
    1077086781,
    171798691,
    1077502935,
    927712936,
    1067467145,
    1431655765,
    1071994197,
    -660764200,
    1072209289,
    858993459,
    -1076677837,
    230,
    17170619,
    21758044,
    19661194,
    9044424,
];

/// The ge colour of a Fig colour. fig2dev indexes its table unchecked
/// (`GE_COLORS[col + 1]`); see `GE_COLORS_PAST_END` for colours beyond it.
fn ge_color(col: i32) -> i32 {
    let Ok(i) = usize::try_from(col.saturating_add(1)) else {
        return 1;
    };
    match GE_COLORS.get(i) {
        Some(&c) => c,
        None => i
            .checked_sub(GE_COLORS.len())
            .and_then(|j| GE_COLORS_PAST_END.get(j))
            .copied()
            .unwrap_or(1),
    }
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
        out.bytes(&t.raw);
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
