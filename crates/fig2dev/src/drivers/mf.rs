//! fig2dev's `mf` driver: METAFONT code for the mfpic macros (`grafbase.mf`).
//! The figure becomes one character; lines, splines, ellipses and arcs
//! become mfpic paths (in inches, y upwards from `-t`), shaded, filled or
//! drawn with a round pen. Texts are not supported (a warning each).
//!
//! Options: `-C code` the character code, `-n name` the name in the
//! comment, `-p scale` the pen scale, `-t top` the top y, and the bounds
//! `-x`, `-X`, `-y`, `-Y` (inches).
//
// Ported from fig2dev dev/genmf.c (xfig licence, see LICENSE-xfig).

use crate::gendev::{atof, print_comments, Ctx, Driver, PACKAGE_VERSION};
use crate::object::{Arc, Compound, Ellipse, Line, Point, Spline, Text, BLACK_FILL};
use crate::{fprintf, Out};

/// The driver for `lang`.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    let _ = lang;
    Some(Box::new(Mf::default()))
}

const INFTY: f64 = 1e5;

#[derive(Debug)]
struct Mf {
    code: f64,
    /// The thickness of the current pen.
    oldpen: i32,
    penscale: f64,
    xscale: f64,
    yscale: f64,
    xl: f64,
    xu: f64,
    yl: f64,
    yu: f64,
    maxy: f64,
    ppi: f64,
}

impl Default for Mf {
    fn default() -> Mf {
        Mf {
            code: 32.0,
            oldpen: 0,
            penscale: 1.0,
            xscale: 0.125,
            yscale: 0.125,
            xl: 0.0,
            xu: 8.0,
            yl: 0.0,
            yu: 8.0,
            maxy: INFTY,
            ppi: 1200.0,
        }
    }
}

/// `%g`: six significant digits, trailing zeros removed, exponent form
/// outside 1e-4..1e6.
fn fmt_g(x: f64) -> String {
    if !x.is_finite() {
        return if x.is_nan() {
            "nan".into()
        } else if x > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        };
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0".into() } else { "0".into() };
    }
    let e = format!("{x:.5e}");
    let (mant, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let trim = |s: &str| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if !(-4..6).contains(&exp) {
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{}e{}{:02}", trim(mant), sign, exp.abs())
    } else {
        let prec = usize::try_from(5 - exp).unwrap_or(0);
        trim(&format!("{x:.prec$}"))
    }
}

/// The fill gray of mfpic's `shade` (`dofill`).
fn dofill(fill_style: i32) -> f64 {
    1.2 - f64::from(fill_style) / f64::from(BLACK_FILL)
}

impl Mf {
    /// `setpen`: a new pen when the thickness changes.
    fn setpen(&mut self, ctx: &Ctx, out: &mut Out, thickness: i32) {
        if thickness != self.oldpen {
            let w = f64::from(thickness as f32 / ctx.thick_scale) * self.penscale * self.yscale * ctx.mag;
            fprintf!(out, "penwd := {:.2}pt;\n", w);
            out.puts("drawpen := pencircle scaled penwd yscaled aspect_ratio;\n");
            self.oldpen = thickness;
        }
    }

    /// x in inches (`(float) x/ppi`).
    fn x(&self, v: i32) -> f64 {
        f64::from(v as f32) / self.ppi
    }
    /// y in inches, upwards from `maxy`.
    fn y(&self, v: i32) -> f64 {
        self.maxy - f64::from(v as f32) / self.ppi
    }

    fn points(&self, out: &mut Out, pts: &[Point]) {
        for (i, p) in pts.iter().enumerate() {
            let lead = if i == 0 { "\t   ((" } else { ",\n\t   (" };
            fprintf!(out, "{}{:.6}, {:.6})", lead, self.x(p.x), self.y(p.y));
        }
        out.puts(");\n");
    }

    /// The fill of a closed or filled path.
    fn fill(out: &mut Out, fill_style: i32, shade_space: bool) {
        if fill_style == BLACK_FILL {
            out.puts("  filled ");
        } else if fill_style < BLACK_FILL && fill_style > 0 {
            fprintf!(
                out,
                "  shade({:.6}pt) unfilled{}",
                dofill(fill_style),
                if shade_space { " " } else { "" }
            );
        } else if fill_style == 0 {
            out.puts("  drawn unfilled ");
        } else {
            out.puts("  drawn ");
        }
    }
}

impl Driver for Mf {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        let a = arg.unwrap_or("");
        match opt {
            'C' => self.code = atof(a),
            'n' => ctx.name = arg.map(String::from),
            'p' => self.penscale = atof(a),
            't' => self.maxy = atof(a),
            'x' => self.xl = atof(a),
            'y' => self.yl = atof(a),
            'X' => self.xu = atof(a),
            'Y' => self.yu = atof(a),
            'G' | 'L' => {}
            _ => return Err(format!("Argument -{opt} unknown to mf driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        self.ppi = ctx.ppi;
        if self.maxy == INFTY {
            self.maxy = self.yu;
        }
        // `(int)code`, saturating.
        let curchar = if self.code.is_finite() { self.code as i32 } else { 0 };
        fprintf!(out, "%\n% fig2dev -L mf (Version {})\n", PACKAGE_VERSION);
        out.puts("%\n");
        if !objects.comments.is_empty() {
            out.puts("%\n");
            print_comments(out, "% ", &objects.comments, "");
            out.puts("%\n");
        }
        out.puts("%\n% Preamble\n%\n");
        fprintf!(out, "mag:=1; input grafbase.mf; code:={};\n", fmt_g(self.code));
        out.puts("interim hdwdr:=1; interim hdten:=1;\n");
        let name = ctx.name.as_deref().or(ctx.from.as_deref()).unwrap_or("stdin");
        fprintf!(out, "%\n% {} (char {})\n%\n", name, curchar.wrapping_add(1));
        fprintf!(
            out,
            "xscale:={:.3}; yscale:={:.3};\n",
            self.xscale * ctx.mag,
            self.yscale * ctx.mag
        );
        fprintf!(
            out,
            "bounds({:.3},{:.3},{:.3},{:.3});\n",
            self.xl,
            self.xu,
            self.yl,
            self.yu
        );
        out.puts("unitlen:=1.0in#;\n");
        out.puts("beginmfpic(incr code);\n");
        self.setpen(ctx, out, 1);
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        print_comments(out, "% ", &a.comments, "");
        self.setpen(ctx, out, a.thickness);
        out.puts("\tstore (curpath)\n");
        let [p0, p1, p2] = a.point;
        fprintf!(
            out,
            "  drawn arcppp(({:.6},{:.6}), ({:.6},{:.6}), ({:.6},{:.6}));\n",
            self.x(p0.x),
            self.y(p0.y),
            self.x(p1.x),
            self.y(p1.y),
            self.x(p2.x),
            self.y(p2.y)
        );
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        print_comments(out, "% ", &e.comments, "");
        self.setpen(ctx, out, e.thickness);
        out.puts("\tstore (curpath)\n");
        if e.fill_style == BLACK_FILL {
            out.puts("  filled ");
        } else if e.fill_style < BLACK_FILL && e.fill_style > 0 {
            fprintf!(out, "  shade({:.6}pt) ", dofill(e.fill_style));
        } else {
            out.puts("  drawn ");
        }
        match e.type_ {
            3 | 4 => fprintf!(
                out,
                "circle(({:.6},{:.6}),{:.6});\n",
                self.x(e.center.x),
                self.y(e.center.y),
                self.x(e.radiuses.x)
            ),
            1 | 2 => fprintf!(
                out,
                "ellipse(({:.6},{:.6}),{:.6},{:.6},0);\n",
                self.x(e.center.x),
                self.y(e.center.y),
                self.x(e.radiuses.x),
                self.x(e.radiuses.y)
            ),
            _ => {}
        }
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        print_comments(out, "% ", &l.comments, "");
        self.setpen(ctx, out, l.thickness);
        out.puts("\tstore (curpath)\n");
        if l.type_ == 1 && l.fill_style < 0 {
            out.puts("  drawn polyline(false)\n");
        } else {
            Mf::fill(out, l.fill_style, false);
            out.puts("polyline(true)\n");
        }
        self.points(out, &l.points);
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        print_comments(out, "% ", &s.comments, "");
        self.setpen(ctx, out, s.thickness);
        out.puts("\tstore (curpath)\n");
        if s.type_ == 0 || (s.type_ == 2 && s.fill_style < 0) {
            out.puts("  drawn curve(false)\n");
        } else {
            Mf::fill(out, s.fill_style, true);
            out.puts("curve(true)\n");
        }
        self.points(out, &s.points);
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, _out: &mut Out, t: &Text) -> Result<(), String> {
        ctx.warn(format!(
            "warning: text ignored in MF output: `{}'",
            String::from_utf8_lossy(&t.raw)
        ));
        Ok(())
    }

    fn end(&mut self, _ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        out.puts("endmfpic;\nend.\n");
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::fmt_g;

    #[test]
    fn g_format() {
        assert_eq!(fmt_g(32.0), "32");
        assert_eq!(fmt_g(65.5), "65.5");
        assert_eq!(fmt_g(1234567.0), "1.23457e+06");
        assert_eq!(fmt_g(0.0001), "0.0001");
        assert_eq!(fmt_g(0.00001), "1e-05");
        assert_eq!(fmt_g(-2.5), "-2.5");
    }
}
