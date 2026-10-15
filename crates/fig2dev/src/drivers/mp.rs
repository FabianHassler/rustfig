//! fig2dev's `mp` driver: MetaPost. Every object becomes a MetaPost path
//! drawn with `pencircle`; texts are typeset by LaTeX (the default) or TeX
//! (`-o`) in `btex … etex` labels, or set in a PostScript font with
//! `infont`. With `-M` (or the old language name `mmp`), objects of
//! non-adjacent depths go into separate figures (`beginfig(n)`) that share
//! one bounding box, for building up a picture in layers.
//!
//! Options: `-o` TeX instead of LaTeX, `-d file` a whole LaTeX header,
//! `-i file` a file to `\input`, `-p n` a `prologues:=n;` line, `-M` layers.
//! Other options are accepted and only listed in the output's comment.
//
// Ported from fig2dev dev/genmp.c (xfig licence, see LICENSE-xfig).

use super::cgm::{text_bytes, text_encoding, TextEnc};
use crate::bound::{calc_arrow, compute_arcarrow_angle};
use crate::gendev::{print_comments, Ctx, Driver, NUM_STD_COLS, PACKAGE_VERSION};
use crate::object::{Arc, Arrow, Compound, Ellipse, Line, Point, Spline, Text};
use crate::util::setfigfont::define_setfigfont;
use crate::util::texfonts::TexFonts;
use crate::{fprintf, round, Out};

/// The driver for `lang`; `mmp` (fig2dev before 3.2.7) is `mp -M`.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    Some(Box::new(Mp {
        split: lang == "mmp",
        ..Mp::default()
    }))
}

const GENMP_VERSION: f64 = 0.05;
const BP_PER_INCH: f64 = 72.0;
const MAXY_BP: f64 = BP_PER_INCH * 11.0;
const PIX_PER_INCH: f64 = 1200.0;
const FIGS_PER_INCH: f64 = 80.0;
/// Size of fig2dev's buffer for the options comment.
const OPTIONS_SIZE: usize = 1024;
const MATHFONTMACRO: &str = "\\figtodev@genmp@mathfonts";

/// TeX names of the PostScript fonts (`xfigpsfonts`).
const XFIGPSFONTS: [&str; 35] = [
    "ptmr", "ptmri", "ptmb", "ptmbi", "pagk", "pagko", "pagd", "pagdo", "pbkl", "pbkli", "pbkd", "pbkdi", "pcrr",
    "pcrro", "pcrb", "pcrbo", "phvr", "phvro", "phvb", "phvbo", "phvrrn", "phvron", "phvbrn", "phvbon", "pncr",
    "pncri", "pncb", "pncbi", "pplr", "pplri", "pplb", "pplbi", "psyr", "pzcmi", "pzdr",
];

/// TeX names of the LaTeX fonts (`xfiglatexfonts`).
const XFIGLATEXFONTS: [&str; 6] = ["cmr10", "cmr10", "cmbx10", "cmti10", "cmss10", "cmtt10"];

/// The standard colours as MetaPost colours (`xfigcolors`).
const XFIGCOLORS: [&str; NUM_STD_COLS] = [
    "(0.00,0.00,0.00)",
    "(0.00,0.00,1.00)",
    "(0.00,1.00,0.00)",
    "(0.00,1.00,1.00)",
    "(1.00,0.00,0.00)",
    "(1.00,0.00,1.00)",
    "(1.00,1.00,0.00)",
    "(1.00,1.00,1.00)",
    "(0.00,0.00,0.56)",
    "(0.00,0.00,0.69)",
    "(0.00,0.00,0.82)",
    "(0.53,0.81,1.00)",
    "(0.00,0.56,0.00)",
    "(0.00,0.69,0.00)",
    "(0.00,0.82,0.00)",
    "(0.00,0.56,0.56)",
    "(0.00,0.69,0.69)",
    "(0.00,0.82,0.82)",
    "(0.56,0.00,0.00)",
    "(0.69,0.00,0.00)",
    "(0.82,0.00,0.00)",
    "(0.56,0.00,0.56)",
    "(0.69,0.00,0.69)",
    "(0.82,0.00,0.82)",
    "(0.50,0.19,0.00)",
    "(0.63,0.25,0.00)",
    "(0.75,0.38,0.00)",
    "(1.00,0.50,0.50)",
    "(1.00,0.63,0.63)",
    "(1.00,0.75,0.75)",
    "(1.00,0.88,0.88)",
    "(1.00,0.84,0.00)",
];

/// Characters special to (La)TeX and what normal texts write instead
/// (`tex_text_specials`, `tex_text_mappings`).
const TEX_TEXT_MAPPINGS: [(u8, &str); 13] = [
    (b'\\', "$\\backslash$"),
    (b'{', "$\\{$"),
    (b'}', "$\\}$"),
    (b'>', "$>$"),
    (b'<', "$<$"),
    (b'^', "\\^{}"),
    (b'~', "\\~{}"),
    (b'$', "\\$"),
    (b'&', "\\&"),
    (b'#', "\\#"),
    (b'_', "\\_"),
    (b'%', "\\%"),
    (b' ', "~"),
];

#[derive(Debug)]
struct Mp {
    /// `-M`: a figure per group of adjacent depths.
    split: bool,
    /// The number of the next figure, and the last depth drawn (`-M`).
    fig_number: i32,
    last_depth: i32,
    /// LaTeX (default) or TeX (`-o`).
    latexmode: bool,
    /// `-p`.
    prologues: Option<i64>,
    /// The options as given, for the comment in the output.
    options: String,
    /// `-d`, `-i`.
    immediate_insert_filename: Option<String>,
    include_filename: Option<String>,
    tex: TexFonts,
    enc: TextEnc,
    mag: f64,
}

impl Default for Mp {
    fn default() -> Mp {
        Mp {
            split: false,
            fig_number: 0,
            last_depth: 1001,
            latexmode: true,
            prologues: None,
            options: String::new(),
            immediate_insert_filename: None,
            include_filename: None,
            tex: TexFonts::new(),
            enc: TextEnc::Utf8,
            mag: 1.0,
        }
    }
}

/// fig2dev's `rad2deg`, with its approximation of pi.
#[allow(clippy::approx_constant)]
fn rad2deg(x: f64) -> f64 {
    x * 180.0 / 3.141592654
}

fn y_off(x: f64) -> f64 {
    MAXY_BP - x
}

/// `atoi`: the leading integer, 0 if none.
fn atoi(s: &str) -> i64 {
    let t = s.trim_start();
    let b = t.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    t.get(..i).and_then(|n| n.parse::<i64>().ok()).unwrap_or(0)
}

/// The pen colour (`genmp_pencolor`).
fn pencolor(ctx: &Ctx, c: i32) -> String {
    if c > 0 && c < NUM_STD_COLS as i32 {
        XFIGCOLORS
            .get(c as usize)
            .copied()
            .unwrap_or("(0.00,0.00,0.00)")
            .to_string()
    } else if c >= NUM_STD_COLS as i32 {
        let u = usize::try_from(c - NUM_STD_COLS as i32)
            .ok()
            .and_then(|i| ctx.user_colors.get(i))
            .copied()
            .unwrap_or_default();
        format!(
            "({:.2},{:.2},{:.2})",
            f64::from(u.r) / 255.0,
            f64::from(u.g) / 255.0,
            f64::from(u.b) / 255.0
        )
    } else {
        "(0.00,0.00,0.00)".to_string()
    }
}

/// The fill colour (`genmp_fillcolor`): a shade of black or a tint of
/// the colour, by mixing in white.
fn fillcolor(ctx: &Ctx, c: i32, s: i32) -> String {
    if c == 0 {
        format!("withcolor (black + {:.2}white)", f64::from(20 - s) / 20.0)
    } else {
        format!(
            "withcolor ({} + {:.2}white)",
            pencolor(ctx, c),
            f64::from(s - 20) / 20.0
        )
    }
}

/// The dash pattern and the end of a `draw` statement.
fn draw_style(out: &mut Out, style: i32, style_val: f64) {
    match style {
        1 => fprintf!(out, " dashed evenly scaled {:.2};\n", style_val / 3.3),
        2 => fprintf!(out, " dashed withdots scaled {:.2};\n", style_val / 4.75),
        _ => out.puts(";\n"),
    }
}

/// The properties `genmp_arrowheads` reads from a line, spline or arc.
struct ArrowOwner<'a> {
    thickness: i32,
    pen_color: i32,
    for_arrow: Option<&'a Arrow>,
    back_arrow: Option<&'a Arrow>,
}

impl Mp {
    fn fig2bp(&self, x: f64) -> f64 {
        x * self.mag * BP_PER_INCH / PIX_PER_INCH
    }
    fn bpi(&self, x: i32) -> f64 {
        self.fig2bp(f64::from(x))
    }
    fn ybpi(&self, y: i32) -> f64 {
        y_off(self.bpi(y))
    }

    /// `do_split`: with `-M`, a new figure when the depth jumps by more
    /// than one.
    fn do_split(&mut self, out: &mut Out, depth: i32) {
        if !self.split {
            return;
        }
        if depth.saturating_add(1) < self.last_depth {
            if self.fig_number > 0 {
                out.puts("setbounds currentpicture to allbounds;\n");
                out.puts("endfig;\n");
            }
            fprintf!(out, "% Now draw objects of depth: {}\n", depth);
            fprintf!(out, "beginfig({})\n", self.fig_number);
            self.fig_number = self.fig_number.saturating_add(1);
        }
        self.last_depth = depth;
    }

    /// `genmp_drawarrow`.
    fn drawarrow(&self, ctx: &Ctx, out: &mut Out, from: Point, to: Point, obj: &ArrowOwner, arr: &Arrow) {
        let ap = calc_arrow(ctx, from.x, from.y, to.x, to.y, obj.thickness, arr);
        let points = &ap.points;
        let Some(first) = points.first() else { return };
        if points.len() < 3 {
            return;
        }
        let ty = arr.type_;
        fprintf!(out, "% Draw arrowhead type {}\n", ty);
        out.puts("  linecap:=0;\n");
        out.puts("  linejoin:=0;\n");
        fprintf!(out, "  pickup pencircle scaled {:.2};\n", self.fig2bp(arr.thickness));
        out.puts("  path arr;\n");
        fprintf!(out, "  arr = ({:.2}, {:.2})", self.bpi(first.x), self.ybpi(first.y));
        for p in points.iter().skip(1) {
            fprintf!(out, "\n      --({:.2}, {:.2})", self.bpi(p.x), self.ybpi(p.y));
        }
        if ty != 0 && ty != 6 && ty < 9 {
            out.puts(" -- cycle");
        }
        out.puts(";\n");
        if ty != 0 {
            if arr.style == 0 {
                out.puts("  fill arr withcolor white;\n");
            } else if ty < 9 {
                fprintf!(out, "  fill arr withcolor {};\n", pencolor(ctx, obj.pen_color));
            }
        }
        fprintf!(out, "  draw arr withcolor {};\n", pencolor(ctx, obj.pen_color));
    }

    /// `genmp_arrowheads` for lines and splines, whose ends are known.
    fn arrowheads(&self, ctx: &Ctx, out: &mut Out, obj: &ArrowOwner, fwd: (Point, Point), back: (Point, Point)) {
        if let Some(a) = obj.for_arrow {
            self.drawarrow(ctx, out, fwd.0, fwd.1, obj, a);
        }
        if let Some(a) = obj.back_arrow {
            self.drawarrow(ctx, out, back.0, back.1, obj, a);
        }
    }

    /// The ends of a spline for its arrowheads (the C code is marked
    /// "UNTESTED", and the forward arrow of an approximated spline points
    /// along its penultimate segment; kept as in fig2dev).
    fn spline_ends(s: &Spline) -> ((Point, Point), (Point, Point)) {
        let pts = &s.points;
        let first = pts.first().copied().unwrap_or_default();
        let second = pts.get(1).copied().unwrap_or(first);
        let n = pts.len();
        let last = pts.last().copied().unwrap_or_default();
        let ctl = |i: usize| s.controls.get(i).copied().unwrap_or_default();
        if s.int_spline() {
            let c = ctl(n.saturating_sub(2));
            let c0 = ctl(0);
            (
                (Point::new(round(c.lx), round(c.ly)), last),
                (Point::new(round(c0.rx), round(c0.ry)), first),
            )
        } else {
            let (mut x2, mut y2) = (f64::from(first.x), f64::from(first.y));
            let (mut c, mut d) = (f64::from(second.x), f64::from(second.y));
            for p in pts.iter().take(n.saturating_sub(1)).skip(1) {
                x2 = c;
                y2 = d;
                c = f64::from(p.x);
                d = f64::from(p.y);
            }
            let bx = round((f64::from(first.x) + f64::from(second.x)) / 2.0);
            let by = round((f64::from(first.y) + f64::from(second.y)) / 2.0);
            (
                (Point::new(round(x2), round(y2)), Point::new(round(c), round(d))),
                (Point::new(bx, by), first),
            )
        }
    }

    fn spline_arrows(&self, ctx: &Ctx, out: &mut Out, s: &Spline) {
        if s.for_arrow.is_none() && s.back_arrow.is_none() {
            return;
        }
        let owner = ArrowOwner {
            thickness: s.thickness,
            pen_color: s.pen_color,
            for_arrow: s.for_arrow.as_ref(),
            back_arrow: s.back_arrow.as_ref(),
        };
        let (fwd, back) = Mp::spline_ends(s);
        self.arrowheads(ctx, out, &owner, fwd, back);
    }

    /// `genmp_writefontmacro_latex`.
    fn writefontmacro_latex(&self, ctx: &Ctx, out: &mut Out, t: &Text, name: &str) {
        let mag = self.tex.texfontmag(t, ctx.fontmag);
        let mut texsize = if mag.is_finite() { mag as i32 } else { 0 };
        let mut baselineskip = f64::from(texsize) * 1.2;
        texsize = (f64::from(texsize) * (BP_PER_INCH / FIGS_PER_INCH)) as i32;
        baselineskip *= BP_PER_INCH / FIGS_PER_INCH;
        out.puts("  verbatimtex\n");
        fprintf!(out, "   \\def\\{}{{%\n", name);
        let ps = t.flags & 0x4 != 0;
        let isdefault = (!ps && t.font == 0) || (ps && t.font == -1);
        if !isdefault {
            fprintf!(
                out,
                "\t    \\SetFigFont{{{}}}{{{:.1}}}{{{}}}{{{}}}{{{}}}%\n",
                texsize,
                baselineskip,
                self.tex.texfamily(t.font),
                self.tex.texseries(t.font),
                self.tex.texshape(t.font)
            );
        } else {
            fprintf!(out, "\t    \\SetFigFontSize{{{}}}{{{:.1}}}%\n", texsize, baselineskip);
        }
        out.puts("    }%\n");
        out.puts("  etex;\n");
    }

    /// `genmp_writefontmacro_tex`.
    fn writefontmacro_tex(&self, out: &mut Out, psize: f64, font: &str, name: &str) {
        let psize = psize * (self.mag * BP_PER_INCH / FIGS_PER_INCH);
        let (ten, seven, five) = (psize, psize * 0.7, psize * 0.5);
        out.puts("  verbatimtex\n");
        fprintf!(out, "\t \\font\\{}={} at {:.2}pt\n", name, font, psize);
        fprintf!(
            out,
            "\t \\font\\tenrm=cmr10 at {:.2}pt\\font\\sevenrm=cmr7 at {:.2}pt\n",
            ten,
            seven
        );
        fprintf!(
            out,
            "\t \\font\\fiverm=cmr5 at {:.2}pt\\font\\teni=cmmi10 at {:.2}pt\n",
            five,
            ten
        );
        fprintf!(
            out,
            "\t \\font\\seveni=cmmi7 at {:.2}pt\\font\\fivei=cmmi5 at {:.2}pt\n",
            seven,
            five
        );
        fprintf!(
            out,
            "\t \\font\\tensy=cmsy10 at {:.2}pt\\font\\sevensy=cmsy7 at {:.2}pt\n",
            ten,
            seven
        );
        fprintf!(out, "\t \\font\\fivesy=cmsy5 at {:.2}pt\n", five);
        fprintf!(out, "\t \\def{}{{%\n", MATHFONTMACRO);
        out.puts("\t   \\textfont0\\tenrm\\scriptfont0\\sevenrm\\scriptscriptfont0\\fiverm\n");
        out.puts("\t   \\textfont1\\teni\\scriptfont1\\seveni\\scriptscriptfont1\\fivei\n");
        out.puts("\t   \\textfont2\\tensy\\scriptfont2\\sevensy\\scriptscriptfont2\\fivesy}\n");
        out.puts("  etex;\n");
    }
}

/// `getfont(flags, font)`: the TeX name of a text's font. fig2dev indexes
/// its tables with the font number unchecked. The LaTeX table follows the
/// PostScript one in memory, so LaTeX font −1 (the reader allows it) gives
/// the last PostScript font, as there; PostScript font −1 reads whatever
/// precedes the table in fig2dev, an empty name here.
fn getfont(flags: i32, font: i32) -> &'static str {
    let i = usize::try_from(font).ok();
    if flags & 0x4 != 0 {
        i.and_then(|i| XFIGPSFONTS.get(i)).copied().unwrap_or("")
    } else if font == -1 {
        XFIGPSFONTS[34]
    } else {
        i.and_then(|i| XFIGLATEXFONTS.get(i)).copied().unwrap_or("cmr10")
    }
}

/// A special text in TeX mode: the math font macro after each opening
/// math delimiter.
fn put_tex_math(out: &mut Out, s: &[u8]) {
    let mut is_mathmode = false;
    let mut i = 0;
    while i < s.len() {
        let rest = s.get(i..).unwrap_or_default();
        let (len, math) = if rest.first() == Some(&b'$') {
            (1, true)
        } else if rest.starts_with(b"\\begin{math}") {
            (12, true)
        } else if rest.starts_with(b"\\end{math}") {
            (10, true)
        } else if rest.starts_with(b"\\(") || rest.starts_with(b"\\)") {
            (2, true)
        } else if rest.starts_with(b"\\$") {
            (2, false)
        } else {
            (1, false)
        };
        out.bytes(rest.get(..len).unwrap_or(rest));
        if math {
            if !is_mathmode {
                fprintf!(out, "{} ", MATHFONTMACRO);
            }
            is_mathmode = !is_mathmode;
        }
        i += len;
    }
}

impl Driver for Mp {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        match opt {
            'M' => self.split = true,
            'o' => self.latexmode = false,
            'd' => match arg {
                Some(a) => self.immediate_insert_filename = Some(a.to_string()),
                None => ctx.warn("Warning: missing argument for '-d'. Ignored."),
            },
            'i' => match arg {
                Some(a) => self.include_filename = Some(a.to_string()),
                None => ctx.warn("Warning: missing argument for '-i'. Ignored."),
            },
            'p' => match arg {
                Some(a) => self.prologues = Some(atoi(a)),
                None => ctx.warn("Warning: missing argument for '-p'. Ignored."),
            },
            // Other options are silently ignored.
            _ => {}
        }
        // The option and its argument, for the comment in the output.
        let mut cllen = self.options.len();
        if cllen + 4 < OPTIONS_SIZE {
            self.options.push(' ');
            self.options.push('-');
            self.options.push(opt);
            cllen += 3;
        }
        if let Some(a) = arg {
            if cllen + a.len().saturating_sub(2) < OPTIONS_SIZE {
                self.options.push(' ');
                self.options.push_str(a);
            }
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        self.mag = ctx.mag;
        self.enc = text_encoding(ctx);
        fprintf!(
            out,
            "%\n% fig2dev (version {}) -L (m)mp version {:.2} --- Preamble\n%\n",
            PACKAGE_VERSION,
            GENMP_VERSION
        );
        out.puts("\n");
        fprintf!(out, "%\n% mp output driver options:\n% {}\n%\n\n", self.options);
        if !objects.comments.is_empty() {
            out.puts("%\n");
            print_comments(out, "% ", &objects.comments, "");
            out.puts("%\n");
        }
        if let Some(n) = self.prologues {
            fprintf!(out, "prologues:={};\n\n", n);
        }

        out.puts("% +MP-ADDITIONAL-HEADER\n");
        if self.latexmode || self.immediate_insert_filename.is_some() || self.include_filename.is_some() {
            out.puts("verbatimtex\n");
            if self.latexmode {
                out.puts("%&latex\n");
            }
            let inserted = self
                .immediate_insert_filename
                .as_deref()
                .and_then(|f| std::fs::read(f).ok().map(|d| (f, d)));
            if let Some((f, data)) = inserted {
                fprintf!(out, "% start of included material from {}\n", f);
                out.bytes(&data);
                out.puts("% end of included material\n");
            } else if self.latexmode {
                out.puts("\\documentclass{article}\n");
                out.puts("\\begin{document}\n");
            }
            if let Some(f) = &self.include_filename {
                fprintf!(out, "\\input {}\n", f);
            }
            out.puts("etex\n");
        }
        out.puts("% -MP-ADDITIONAL-HEADER\n\n");

        if self.latexmode {
            out.puts("\n%SetFigFont macros for latex\n");
            out.puts("verbatimtex\n");
            define_setfigfont(out, false);
            out.puts("\\ifx\\SetFigFontSize\\undefined%\n");
            out.puts("\\gdef\\SetFigFontSize#1#2{%\n");
            out.puts("\t\\fontsize{#1}{#2pt}%\n");
            out.puts("\t\\selectfont}%\n");
            out.puts("\\fi%\n");
            out.puts("etex\n\n");
        }

        if self.split {
            let (llx, lly, urx, ury) = (
                self.bpi(ctx.llx),
                self.ybpi(ctx.lly),
                self.bpi(ctx.urx),
                self.ybpi(ctx.ury),
            );
            out.puts("path allbounds;\n");
            fprintf!(out, "allbounds = ({:.2},{:.2})--({:.2},{:.2})", llx, lly, urx, lly);
            fprintf!(out, "--({:.2},{:.2})--({:.2},{:.2})--cycle;\n", urx, ury, llx, ury);
        } else {
            out.puts("% Now draw the figure\n");
            out.puts("beginfig(0)\n");
        }
        out.puts("% Some reasonable defaults\n");
        out.puts("  labeloffset:=0;\n");
        fprintf!(out, "  truecorners:={};\n", i32::from(!self.split));
        out.puts("  bboxmargin:=0;\n");
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        self.do_split(out, a.depth);
        print_comments(out, "% ", &a.comments, "");
        out.puts("% Begin arc object\n");
        match a.type_ {
            0..=2 => {
                let [p0, _, p2] = a.point;
                fprintf!(out, "  linecap:={};\n", a.cap_style);
                out.puts("  linejoin:=0;\n");
                fprintf!(out, "  pickup pencircle scaled {:.2};\n", self.bpi(a.thickness));
                out.puts("  path a,p,ls,le;\n");
                out.puts("  pair s,e,c;\n");
                fprintf!(
                    out,
                    "  c = ({:.2},{:.2});\n",
                    self.fig2bp(a.center.x),
                    y_off(self.fig2bp(a.center.y))
                );
                fprintf!(out, "  s = ({:.2},{:.2});\n", self.bpi(p0.x), self.ybpi(p0.y));
                fprintf!(out, "  e = ({:.2},{:.2});\n", self.bpi(p2.x), self.ybpi(p2.y));
                fprintf!(
                    out,
                    "  d := ({:.2} ++ {:.2})*2.0;\n",
                    self.bpi(p0.x) - self.fig2bp(a.center.x),
                    self.bpi(p0.y) - self.fig2bp(a.center.y)
                );
                out.puts("  ls = (0,0)--(d,0) rotated angle (s-c);\n");
                out.puts("  le = (0,0)--(d,0) rotated angle (e-c);\n");
                out.puts("  p = ");
                if a.direction != 1 {
                    out.puts("reverse ");
                }
                out.puts("fullcircle scaled d rotated angle (s-c) ");
                out.puts("cutafter le;\n");
                if a.type_ == 2 {
                    out.puts("\ta = buildcycle(ls,p,le) shifted c;\n");
                } else {
                    out.puts("\ta = p shifted c;\n");
                }
                if a.fill_style != -1 {
                    out.puts("\tpath f;\n");
                    out.puts("\tf = a--cycle;\n");
                    fprintf!(out, "\tfill f {};\n", fillcolor(ctx, a.fill_color, a.fill_style));
                }
                if a.thickness != 0 {
                    out.puts("\tdraw a ");
                    fprintf!(out, "withcolor {}", pencolor(ctx, a.pen_color));
                    draw_style(out, a.style, a.style_val);
                    let owner = ArrowOwner {
                        thickness: a.thickness,
                        pen_color: a.pen_color,
                        for_arrow: a.for_arrow.as_ref(),
                        back_arrow: a.back_arrow.as_ref(),
                    };
                    let (cx, cy) = (a.center.x, a.center.y);
                    if let Some(arr) = owner.for_arrow {
                        let (fx, fy) =
                            compute_arcarrow_angle(ctx, cx, cy, f64::from(p2.x), f64::from(p2.y), a.direction, arr);
                        self.drawarrow(ctx, out, Point::new(fx, fy), p2, &owner, arr);
                    }
                    if let Some(arr) = owner.back_arrow {
                        let (fx, fy) =
                            compute_arcarrow_angle(ctx, cx, cy, f64::from(p0.x), f64::from(p0.y), a.direction ^ 1, arr);
                        self.drawarrow(ctx, out, Point::new(fx, fy), p0, &owner, arr);
                    }
                }
            }
            t => fprintf!(out, "  show \"This Arc object is not supported! (type={})\"\n", t),
        }
        out.puts("% End arc object\n");
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        self.do_split(out, e.depth);
        print_comments(out, "% ", &e.comments, "");
        out.puts("% Begin ellipse object\n");
        match e.type_ {
            1..=4 => {
                fprintf!(out, "  pickup pencircle scaled {:.2};\n", self.bpi(e.thickness));
                out.puts("  path c;\n");
                fprintf!(
                    out,
                    "  c = fullcircle scaled {:.2} yscaled {:.2}\n",
                    self.bpi(e.radiuses.x.saturating_mul(2)),
                    self.bpi(e.radiuses.y) / self.bpi(e.radiuses.x)
                );
                fprintf!(
                    out,
                    "\t    rotated {:.2} shifted ({:.2},{:.2});\n",
                    rad2deg(e.angle),
                    self.bpi(e.center.x),
                    self.ybpi(e.center.y)
                );
                if e.fill_style != -1 {
                    fprintf!(out, " fill c {};\n", fillcolor(ctx, e.fill_color, e.fill_style));
                }
                if e.thickness != 0 {
                    fprintf!(out, "\tdraw c withcolor {}", pencolor(ctx, e.pen_color));
                    draw_style(out, e.style, e.style_val);
                }
            }
            _ => out.puts("  show \"This Ellipse object is not supported!\"\n"),
        }
        out.puts("% End ellipse object\n");
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        self.do_split(out, l.depth);
        print_comments(out, "% ", &l.comments, "");
        out.puts("% Begin polyline object\n");
        let Some(&first) = l.points.first() else {
            out.puts("% End polyline object\n");
            return Ok(());
        };
        match l.type_ {
            1..=3 => {
                fprintf!(out, "  linecap:={};\n", l.cap_style);
                fprintf!(out, "  linejoin:={};\n", l.join_style);
                fprintf!(out, "  pickup pencircle scaled {:.2};\n", self.bpi(l.thickness));
                out.puts("  path p;\n");
                fprintf!(out, "  p = ({:.2}, {:.2})", self.bpi(first.x), self.ybpi(first.y));
                for p in l.points.iter().skip(1) {
                    fprintf!(out, "\n    --({:.2}, {:.2})", self.bpi(p.x), self.ybpi(p.y));
                }
                if l.type_ != 1 {
                    out.puts("--cycle;\n");
                } else {
                    out.puts(";\n");
                }
                if l.fill_style != -1 {
                    out.puts("\tpath f;\n");
                    out.puts("\tf = p--cycle;\n");
                    fprintf!(out, "\tfill f {};\n", fillcolor(ctx, l.fill_color, l.fill_style));
                }
                if l.thickness != 0 {
                    out.puts("\tdraw p ");
                    fprintf!(out, "withcolor {}", pencolor(ctx, l.pen_color));
                    draw_style(out, l.style, l.style_val);
                    let n = l.points.len();
                    let last = l.points.last().copied().unwrap_or(first);
                    let penult = n.checked_sub(2).and_then(|i| l.points.get(i)).copied().unwrap_or(last);
                    let second = l.points.get(1).copied().unwrap_or(first);
                    let owner = ArrowOwner {
                        thickness: l.thickness,
                        pen_color: l.pen_color,
                        for_arrow: l.for_arrow.as_ref(),
                        back_arrow: l.back_arrow.as_ref(),
                    };
                    self.arrowheads(ctx, out, &owner, (penult, last), (second, first));
                }
            }
            4 => {
                let r = l.radius;
                let p2 = l.points.get(2).copied().unwrap_or(first);
                fprintf!(out, "  linecap:={};\n", l.cap_style);
                out.puts("  linejoin:=1;\n");
                fprintf!(out, "  pickup pencircle scaled {:.2};\n", self.bpi(l.thickness));
                out.puts("  path p,pb,sw,nw,ne,se;\n");
                out.puts("  pair ll,ul,ur,lr;\n");
                fprintf!(out, "  p = ({:.2},{:.2})--", self.bpi(first.x), self.ybpi(first.y));
                fprintf!(out, "({:.2},{:.2});\n", self.bpi(p2.x), self.ybpi(p2.y));
                out.puts("  ur = urcorner p; ll = llcorner p;\n");
                out.puts("  ul = ulcorner p; lr = lrcorner p;\n");
                let d = self.bpi(r.saturating_mul(2));
                let rb = self.bpi(r);
                fprintf!(
                    out,
                    "  sw = fullcircle scaled {:.2} shifted (ll + ({:.2},{:.2}));\n",
                    d,
                    rb,
                    rb
                );
                fprintf!(
                    out,
                    "  nw = fullcircle scaled {:.2} shifted (ul + ({:.2},{:.2}));\n",
                    d,
                    rb,
                    -rb
                );
                fprintf!(
                    out,
                    "  ne = fullcircle rotated 180 scaled {:.2} shifted (ur + ({:.2},{:.2}));\n",
                    d,
                    -rb,
                    -rb
                );
                fprintf!(
                    out,
                    "  se = fullcircle rotated 180 scaled {:.2} shifted (lr + ({:.2},{:.2}));\n",
                    d,
                    -rb,
                    rb
                );
                out.puts("  pb = buildcycle(sw,ll--ul,nw,ul--ur,ne,ur--lr,se,lr--ll);\n");
                if l.fill_style != -1 {
                    fprintf!(out, "\tfill pb {};\n", fillcolor(ctx, l.fill_color, l.fill_style));
                }
                if l.thickness != 0 {
                    fprintf!(out, "\tdraw pb withcolor {}", pencolor(ctx, l.pen_color));
                    draw_style(out, l.style, l.style_val);
                }
            }
            5 => out.puts("  show \"Picture objects are not supported!\"\n"),
            _ => out.puts("  show \"This Polyline object is not supported!\"\n"),
        }
        out.puts("% End polyline object\n");
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        self.do_split(out, s.depth);
        print_comments(out, "% ", &s.comments, "");
        out.puts("% Begin spline object\n");
        let pts = &s.points;
        let (Some(&p0), Some(&p1)) = (pts.first(), pts.get(1)) else {
            out.puts("% End spline object\n");
            return Ok(());
        };
        let n = pts.len();
        let last = pts.last().copied().unwrap_or(p1);
        match s.type_ {
            0 | 1 => {
                let closed = s.type_ == 1;
                fprintf!(out, "  linecap:={};\n", s.cap_style);
                out.puts("  pair p[],c[];\n");
                out.puts("  path s;\n");
                fprintf!(out, "  pickup pencircle scaled {:.2};\n", self.bpi(s.thickness));
                // "Curve points" halfway between the given points.
                for (i, (a, b)) in pts.iter().zip(pts.iter().skip(1)).enumerate() {
                    let lead = if i == 0 { "  " } else { "\t" };
                    fprintf!(
                        out,
                        "{}p{} = .5[({:.2},{:.2}),({:.2},{:.2})];\n",
                        lead,
                        i,
                        self.bpi(a.x),
                        self.ybpi(a.y),
                        self.bpi(b.x),
                        self.ybpi(b.y)
                    );
                }
                // "Control points" 2/3 of the way from the curve points to
                // the given points.
                let mut i = 0;
                let mut j = 0;
                for p in pts.iter().take(n - 1).skip(1) {
                    let (x, y) = (self.bpi(p.x), self.ybpi(p.y));
                    fprintf!(out, "\tc{} = .666667[p{},({:.2},{:.2})];\n", j, i, x, y);
                    fprintf!(out, "\tc{} = .666667[p{},({:.2},{:.2})];\n", j + 1, i + 1, x, y);
                    j += 2;
                    i += 1;
                }
                if closed {
                    let (x, y) = (self.bpi(last.x), self.ybpi(last.y));
                    fprintf!(out, "\tc{} = .666667[p{},({:.2},{:.2})];\n", j, i, x, y);
                    fprintf!(out, "\tc{} = .666667[p0,({:.2},{:.2})];\n", j + 1, x, y);
                }
                // The spline.
                if closed {
                    out.puts("\ts =\n");
                } else {
                    fprintf!(out, "\ts = ({:.2},{:.2})..\n", self.bpi(p0.x), self.ybpi(p0.y));
                }
                let mut i = 0;
                let mut j = 0;
                for _ in 2..n {
                    fprintf!(out, "\t  p{}..controls c{} and c{}..\n", i, j, j + 1);
                    i += 1;
                    j += 2;
                }
                if closed {
                    fprintf!(out, "\t  p{}..controls c{} and c{}..cycle;\n", i, j, j + 1);
                } else {
                    fprintf!(out, "\t  p{}..({:.2},{:.2});\n", i, self.bpi(last.x), self.ybpi(last.y));
                }
            }
            2 | 3 => {
                fprintf!(out, "  linecap:={};\n", s.cap_style);
                out.puts("  path s;\n");
                fprintf!(out, "  pickup pencircle scaled {:.2};\n", self.bpi(s.thickness));
                fprintf!(out, "  s = ({:.2}, {:.2})", self.bpi(p0.x), self.ybpi(p0.y));
                for (k, p) in pts.iter().enumerate().skip(1) {
                    let c = s.controls.get(k - 1).copied().unwrap_or_default();
                    let cn = s.controls.get(k).copied().unwrap_or_default();
                    fprintf!(
                        out,
                        "..controls ({:.2}, {:.2}) and ({:.2}, {:.2})\n",
                        self.fig2bp(c.rx),
                        y_off(self.fig2bp(c.ry)),
                        self.fig2bp(cn.lx),
                        y_off(self.fig2bp(cn.ly))
                    );
                    fprintf!(out, "\t..({:.2},{:.2})", self.bpi(p.x), self.ybpi(p.y));
                }
                if s.type_ == 3 {
                    out.puts("..cycle;\n");
                } else {
                    out.puts(";\n");
                }
            }
            _ => {
                out.puts("  show \"This Spline object is not supported!\"\n");
                out.puts("% End spline object\n");
                return Ok(());
            }
        }
        if s.fill_style != -1 {
            out.puts("\tpath f;\n");
            out.puts("\tf = s--cycle;\n");
            fprintf!(out, "\tfill f {};\n", fillcolor(ctx, s.fill_color, s.fill_style));
        }
        if s.thickness != 0 {
            out.puts("\tdraw s ");
            fprintf!(out, "withcolor {}", pencolor(ctx, s.pen_color));
            draw_style(out, s.style, s.style_val);
            self.spline_arrows(ctx, out, s);
        }
        out.puts("% End spline object\n");
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String> {
        self.do_split(out, t.depth);
        print_comments(out, "% ", &t.comments, "");
        out.puts("% Begin text object\n");
        let s = text_bytes(self.enc, &t.cstring);
        let ps = t.flags & 0x4 != 0;
        if !ps || t.special_text() {
            out.puts("  picture q;\n");
            if self.latexmode {
                self.writefontmacro_latex(ctx, out, t, "mpsetfnt");
            } else {
                self.writefontmacro_tex(out, t.size, getfont(t.flags, t.font), "mpsetfnt");
            }
            out.puts("  q = btex \\mpsetfnt ");
            if !t.special_text() {
                for &b in &s {
                    match TEX_TEXT_MAPPINGS.iter().find(|m| m.0 == b) {
                        Some(m) => out.puts(m.1),
                        None => out.bytes(&[b]),
                    }
                }
            } else if !self.latexmode {
                put_tex_math(out, &s);
            } else {
                out.bytes(&s);
            }
            out.puts(" etex;\n");
        } else {
            out.puts("  picture q;\n");
            out.puts("  q=thelabel.urt(\"");
            out.bytes(&s);
            out.puts("\" infont ");
            if t.font < 0 {
                out.puts("defaultfont");
            } else {
                fprintf!(out, "\"{}\"", getfont(t.flags, t.font));
            }
            out.puts(" scaled ");
            let size = t.size * self.mag * BP_PER_INCH / FIGS_PER_INCH;
            fprintf!(
                out,
                "({}pt/fontsize defaultfont)",
                if size.is_finite() { size as i32 } else { 0 }
            );
            out.puts(",(0,0));\n");
        }

        out.puts("  picture p;\n");
        let deg = rad2deg(t.angle);
        fprintf!(out, "  p = q rotated {:.2};\n", deg);
        let (x, y) = (self.bpi(t.base_x), self.ybpi(t.base_y));
        let color = pencolor(ctx, t.color);
        match t.type_ {
            0 => fprintf!(out, "  label.urt(p,(({:.2},{:.2}))+llcorner p) withcolor {};\n", x, y, color),
            1 => fprintf!(
                out,
                "  label.urt(p,(({:.2},{:.2}))+xpart (lrcorner q - llcorner q)*(-cosd {:.2},-sind {:.2})/2+llcorner p) withcolor {};\n",
                x,
                y,
                deg,
                deg,
                color
            ),
            2 => fprintf!(
                out,
                "  label.urt(p,(({:.2},{:.2}))+xpart (lrcorner q - llcorner q)*(-cosd {:.2},-sind {:.2})+llcorner p) withcolor {};\n",
                x,
                y,
                deg,
                deg,
                color
            ),
            _ => out.puts("  show \"This Text object is not supported!\"\n"),
        }
        out.puts("% End text object\n");
        Ok(())
    }

    fn end(&mut self, _ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        if self.split {
            out.puts("setbounds currentpicture to allbounds;\n");
        }
        out.puts("endfig;\nend\n");
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}
