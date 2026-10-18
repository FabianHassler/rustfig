//! fig2dev's `tikz` driver: the figure as a TikZ picture (PGF macros).
//!
//! Coordinates are Fig units with y negated (`XCOORD`, `YCOORD`), shifted
//! when TeX would see numbers below -16383; the unit is one Fig unit times
//! the magnification, in scaled points. Polylines, polygons, boxes (with
//! rounded corners), circles, ellipses and arcs are `\draw`, `\fill` or
//! `\filldraw` paths; patterns are `\pgfdeclarepatternformonly`
//! definitions; arrowheads are declared with `\pgfdeclarearrow` (only the
//! ones used) and set with `\pgfsetarrows`; texts are `\pgftext`. Splines
//! are not supported (3.2 files have none: the reader approximates them).
//
// Ported from fig2dev dev/gentikz.c and gentikz.h (xfig licence, see LICENSE-xfig).

use std::f64::consts::PI;

use crate::bound::ARROW_SHAPES;
use crate::drivers::dxf::fmt_g;
use crate::drivers::latex::{set_default_size, Cf};
use crate::drivers::pict2e::round_mul;
use crate::gendev::{print_comments, strtol, Ctx, NUMARROWS, NUM_STD_COLS};
use crate::object::{
    Arc, Arrow, Compound, Ellipse, Line, Point, Spline, Text, BLACK_COLOR, DASH_2_DOTS_LINE, DASH_3_DOTS_LINE,
    DASH_DOT_LINE, DASH_LINE, DEFAULT, DOTTED_LINE, MAX_FONT, MAX_PSFONT, NUMPATTERNS, NUMSHADES, NUMTINTS, SOLID_LINE,
    T_ARC_BOX, T_BOX, T_LEFT_JUSTIFIED, T_OPEN_ARC, T_PIC_BOX, T_PIE_WEDGE_ARC, T_POLYGON, T_POLYLINE,
    T_RIGHT_JUSTIFIED, UNFILLED, WHITE_COLOR,
};
use crate::util::psfonts::PS_FONT_NAMES;
use crate::util::texfonts::{put_string, TexFonts};
use crate::{fprintf, round, Driver, Out};

/// The standard colours: name and rgb (`std_color[]`, entry 0 is DEFAULT).
const STD_COLOR: [(&str, f32, f32, f32); NUM_STD_COLS + 1] = [
    (".", 0., 0., 0.),
    ("black", 0., 0., 0.),
    ("blue", 0., 0., 1.),
    ("green", 0., 1., 0.),
    ("cyan", 0., 1., 1.),
    ("red", 1., 0., 0.),
    ("magenta", 1., 0., 1.),
    ("yellow", 1., 1., 0.),
    ("white", 1., 1., 1.),
    ("blue1", 0., 0., 0.56),
    ("blue2", 0., 0., 0.69),
    ("blue3", 0., 0., 0.82),
    ("blue4", 0.53, 0.81, 1.),
    ("green1", 0., 0.56, 0.),
    ("green2", 0., 0.69, 0.),
    ("green3", 0., 0.82, 0.),
    ("cyan1", 0., 0.56, 0.56),
    ("cyan2", 0., 0.69, 0.69),
    ("cyan3", 0., 0.82, 0.82),
    ("red1", 0.56, 0., 0.),
    ("red2", 0.69, 0., 0.),
    ("red3", 0.82, 0., 0.),
    ("magenta1", 0.56, 0., 0.56),
    ("magenta2", 0.69, 0., 0.69),
    ("magenta3", 0.82, 0., 0.82),
    ("brown1", 0.50, 0.19, 0.),
    ("brown2", 0.63, 0.25, 0.),
    ("brown3", 0.75, 0.38, 0.),
    ("pink1", 1., 0.50, 0.50),
    ("pink2", 1., 0.63, 0.63),
    ("pink3", 1., 0.75, 0.75),
    ("pink4", 1., 0.88, 0.88),
    ("gold", 1., 0.84, 0.),
];

// pgfarrow property flags
const HAS_FILL: i32 = 1;
const HAS_SWAP: i32 = 2;
const HAS_WIDTH: i32 = 4;
/// HAS_WIDTH is currently equivalent to not-a-circle.
const NOT_CIRCLE: i32 = HAS_WIDTH;
const HAS_LENGTH: i32 = 8;
/// Needs to be shifted to be flush with the line.
const IS_HOOK: i32 = 16;
const ARROWFLAG: [&str; 3] = ["open", "fill=.", "swap"];

/// What the PGF arrows need to know about each tikz arrow type
/// (`struct pgfarrow`).
struct PgfArrow {
    props: i32,
    backl: f64,
    lendl: f64,
    lendt: f64,
    tiphull: bool,
    nhull: i32,
    hullp: [(f64, f64); 3],
    endhull: bool,
}

#[allow(clippy::too_many_arguments)]
const fn pa(
    props: i32,
    backl: f64,
    lendl: f64,
    lendt: f64,
    tiphull: bool,
    nhull: i32,
    hullp: [(f64, f64); 3],
    endhull: bool,
) -> PgfArrow {
    PgfArrow {
        props,
        backl,
        lendl,
        lendt,
        tiphull,
        nhull,
        hullp,
        endhull,
    }
}

const Z: (f64, f64) = (0., 0.);

const PGFARROW: [PgfArrow; 17] = [
    pa(12, -1., 0., -0.5, true, 1, [(-1., 0.5), Z, Z], false),
    pa(13, -1., -1., -0.45, true, 1, [(-1., 0.5), Z, Z], false),
    pa(13, -1.25, -1., -0.5, true, 1, [(-1.25, 0.5), Z, Z], false),
    pa(13, -1., -1., 0.5, true, 1, [(-0.75, 0.5), Z, Z], true),
    pa(13, -1., -1., 0.5, true, 1, [(-0.5, 0.5), Z, Z], true),
    pa(
        9,
        -1.,
        -1.,
        0.45,
        true,
        3,
        [(-0.25, 0.35), (-0.5, 0.5), (-0.75, 0.35)],
        true,
    ),
    pa(9, -0.5, -0.5, 0.45, false, 2, [(0., 0.5), (-0.25, 0.35), Z], true),
    pa(13, -1., -1., -0.45, false, 2, [(0., 0.5), (-1., 0.5), Z], false),
    pa(13, -1., -1., 0.5, false, 1, [(0., 0.5), Z, Z], true),
    pa(14, -1.25, -1., -0.5, true, 1, [(-1.25, 0.5), Z, Z], false),
    pa(31, -1., -1., -0.45, true, -1, [(-1., 0.5), Z, Z], false),
    pa(31, -1.25, -1., -0.5, true, -1, [(-1.25, 0.5), Z, Z], false),
    pa(31, -1., -1., 0.5, true, -1, [(-0.75, 0.5), Z, Z], true),
    pa(12, -1., -1., 0.5, false, 1, [(0., 0.5), Z, Z], true),
    pa(4, 0., 0., -0.45, false, 1, [(0., 0.5), Z, Z], false),
    pa(12, -1., -1., -0.45, false, 2, [(0., 0.5), (-1., 0.5), Z], false),
    pa(12, -1., 0., -0.45, false, 2, [(0., 0.5), (-1., 0.5), Z], false),
];

/// Arrow options (`struct options`).
#[derive(Clone, Copy, Debug, Default)]
struct Options {
    thickness: f64,
    wid: f64,
    ht: f64,
    flags: i32,
}

/// The changes needed to get from some options to those an arrow demands
/// (`struct settings`).
#[derive(Clone, Copy, Debug, Default)]
struct Settings {
    has_options: bool,
    thickness: f64,
    wid: f64,
    ht: f64,
    /// Which flags must be set, and to what value.
    setflags: i32,
    flags: i32,
}

#[derive(Clone, Copy, Debug)]
struct TikzArrow {
    type_: i32,
    props: i32,
    flags: i32,
    s: Settings,
}

const UNSET: f64 = -1.0;
const NOARROW: i32 = -1;
const NO_ARROW: TikzArrow = TikzArrow {
    type_: NOARROW,
    props: 0,
    flags: 0,
    s: Settings {
        has_options: false,
        thickness: 0.0,
        wid: 0.0,
        ht: 0.0,
        setflags: 0,
        flags: 0,
    },
};

/// Lines of points are broken after this many characters.
const MINLINELENGTH: usize = 75;
const MITERJOIN: i32 = 0;
const BUTTCAP: i32 = 0;
const ROUNDCAP: i32 = 1;
#[allow(clippy::excessive_precision)]
const DEG_RAD: f64 = 57.295779513082322865;
/// `for (d = 1.0; da < 1.0/d; d *= 10.)` ends by itself; this is a cap.
const MAX_PRECISION: usize = 400;

/// The `tikz` driver's state (gentikz.c's statics).
#[derive(Debug)]
pub struct Tikz {
    fonts: TexFonts,
    select_fontsize: bool,
    select_fontname: bool,
    removesuffix: bool,
    only_texfonts: bool,
    /// `-O`: read, but fig2dev doesn't act on it.
    allspecial: bool,
    pagemode: bool,
    figscaling: bool,
    prepend: Option<String>,
    xshift: i32,
    yshift: i32,
    verbose: bool,
    default_color: i32,
    cur_pencolor: i32,
    cur_patcolor: i32,
    cur_fillcolor: i32,
    cur_fillstyle: i32,
    border_margin: i32,
    default_options: Options,
    cur_thickness: i32,
    cur_style: i32,
    cur_styleval: f64,
    cur_capstyle: i32,
    cur_joinstyle: i32,
    cur_b: TikzArrow,
    cur_f: TikzArrow,
    thick_scale: f32,
    ppi: f64,
}

impl Default for Tikz {
    fn default() -> Tikz {
        Tikz {
            fonts: TexFonts::new(),
            select_fontsize: true,
            select_fontname: true,
            removesuffix: false,
            only_texfonts: false,
            allspecial: false,
            pagemode: false,
            figscaling: true,
            prepend: None,
            xshift: 0,
            yshift: 0,
            verbose: false,
            default_color: DEFAULT,
            cur_pencolor: DEFAULT,
            cur_patcolor: DEFAULT,
            cur_fillcolor: DEFAULT,
            cur_fillstyle: NUMSHADES - 1,
            border_margin: 0,
            default_options: Options::default(),
            cur_thickness: 0,
            cur_style: SOLID_LINE,
            cur_styleval: 0.0,
            cur_capstyle: 0,
            cur_joinstyle: 0,
            cur_b: NO_ARROW,
            cur_f: NO_ARROW,
            thick_scale: 1.0,
            ppi: 1200.0,
        }
    }
}

/// The driver for `lang`.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    (lang == "tikz").then(|| Box::new(Tikz::default()) as Box<dyn Driver>)
}

fn atoi(s: &str) -> i32 {
    strtol(s).0
}

/// `PREC1(f)`: 0 digits for a whole number, else 1.
fn prec1(f: f64) -> usize {
    usize::from(f.floor() != f)
}

/// The number of digits behind the comma needed, up to `digits`.
fn precision(digits: usize, mut d: f64) -> usize {
    let mut i = 0;
    while i <= digits && d.floor() != d {
        d *= 10.;
        i += 1;
    }
    i
}

/// `put_texdimen`.
fn texdimen(out: &mut Out, d: f64, dimen: &str) {
    if d == 0. {
        out.puts("0pt");
    } else if d == 1. {
        out.puts(dimen);
    } else if d == -1. {
        out.puts("-");
        out.puts(dimen);
    } else {
        fprintf!(out, "{:.*}{}", precision(3, d), Cf(d), dimen);
    }
}

/// The tikz arrow type of a Fig arrow type and style.
fn arrowtype(type_: i32, style: i32) -> i32 {
    if type_ < 13 {
        type_
    } else {
        type_.wrapping_mul(2).wrapping_add(style).wrapping_sub(13)
    }
}

/// Shift to translate coordinates to values larger than -16383.
fn shift_coordinate(ll: i32, ppi: f64) -> i32 {
    let ippi = ppi as i32;
    let min_number = -16383;
    if ll >= min_number {
        return 0;
    }
    let shift = min_number.wrapping_sub(ll);
    match shift.checked_div(ippi) {
        Some(q) => q.wrapping_add(1).wrapping_mul(ippi),
        None => 0,
    }
}

/// The name of colour `c` (`put_colorname`).
fn colorname(c: i32) -> String {
    if c < NUM_STD_COLS as i32 {
        let name = usize::try_from(c.wrapping_add(1))
            .ok()
            .and_then(|i| STD_COLOR.get(i))
            .map_or(".", |e| e.0);
        name.to_string()
    } else {
        format!("xfigc{c}")
    }
}

/// `THICKNESS(T)` for an int `T`.
fn thickness_i(t: i32, ts: f32) -> f64 {
    if (t as f32) <= ts {
        0.5 * f64::from(t)
    } else {
        f64::from(t as f32 - ts)
    }
}

/// `THICKNESS(T)` for a double `T`.
fn thickness_d(t: f64, ts: f32) -> f64 {
    if t <= f64::from(ts) {
        0.5 * t
    } else {
        t - f64::from(ts)
    }
}

/// Fully shaded colours are black, fully tinted ones white (`normalize`).
fn normalize(fill_color: &mut i32, fill_style: &mut i32) {
    if *fill_style >= NUMSHADES + NUMTINTS || *fill_style == NUMSHADES - 1 {
        return;
    }
    if *fill_style == NUMSHADES + NUMTINTS - 1 || (*fill_color == WHITE_COLOR && *fill_style >= NUMSHADES - 1) {
        *fill_color = WHITE_COLOR;
        *fill_style = NUMSHADES - 1;
    } else if *fill_color == WHITE_COLOR {
        *fill_color = BLACK_COLOR;
        *fill_style += NUMSHADES - 1;
    } else if (*fill_color == BLACK_COLOR || *fill_color == DEFAULT) && *fill_style < NUMSHADES - 1 {
        if *fill_style == 0 {
            *fill_color = WHITE_COLOR;
            *fill_style = NUMSHADES - 1;
        } else {
            *fill_style = NUMSHADES + NUMTINTS - 1 - *fill_style;
        }
    } else if *fill_style == 0 {
        *fill_color = BLACK_COLOR;
        *fill_style = NUMSHADES - 1;
    }
}

/// The settings necessary to change the arrow options from `o` to those
/// demanded by `a`, `a_flags` and `a_props`.
fn get_settings(a: &Arrow, a_flags: i32, a_props: i32, o: Options) -> Settings {
    let mut s = Settings {
        thickness: if a.thickness != o.thickness { a.thickness } else { UNSET },
        wid: if a_props & HAS_WIDTH != 0 && a.wid != o.wid {
            a.wid
        } else {
            UNSET
        },
        ht: if a_props & HAS_LENGTH != 0 && a.ht != o.ht {
            a.ht
        } else {
            UNSET
        },
        ..Settings::default()
    };
    if a_props & HAS_FILL != 0 && (a_flags & HAS_FILL) != (o.flags & HAS_FILL) {
        s.setflags = HAS_FILL;
        s.flags = a_flags & HAS_FILL;
    }
    if a_props & HAS_SWAP != 0 && (a_flags & HAS_SWAP) != (o.flags & HAS_SWAP) {
        s.setflags += HAS_SWAP;
        s.flags += a_flags & HAS_SWAP;
    }
    s.has_options = !(s.thickness == UNSET && s.wid == UNSET && s.ht == UNSET && s.setflags == 0);
    s
}

fn combine(s: &Settings, o: &Options) -> Options {
    if !s.has_options {
        return *o;
    }
    Options {
        thickness: if s.thickness == UNSET { o.thickness } else { s.thickness },
        wid: if s.wid == UNSET { o.wid } else { s.wid },
        ht: if s.ht == UNSET { o.ht } else { s.ht },
        flags: if s.setflags != 0 {
            s.flags | (o.flags & !s.setflags)
        } else {
            o.flags
        },
    }
}

/// The arrow of tikz type `arrowtype(a.type_, a.style)`, or `None` if
/// there is no such type.
fn create_arrow(a: &Arrow, o: &Options) -> Option<TikzArrow> {
    let type_ = arrowtype(a.type_, a.style);
    let props = PGFARROW.get(usize::try_from(type_).ok()?)?.props;
    let flags = if a.style != 0 && props & (HAS_FILL + HAS_SWAP) != 0 {
        if type_ == 9 {
            HAS_SWAP
        } else {
            HAS_FILL
        }
    } else {
        0
    };
    Some(TikzArrow {
        type_,
        props,
        flags,
        s: get_settings(a, flags, props, *o),
    })
}

/// The number of changed options.
fn changes(s: &Settings) -> i32 {
    i32::from(s.thickness != UNSET)
        + i32::from(s.wid != UNSET)
        + i32::from(s.ht != UNSET)
        + i32::from(s.setflags & HAS_FILL != 0)
        + i32::from(s.setflags & HAS_SWAP != 0)
}

/// Which of two settings is the default: `Some(true)` the back one,
/// `Some(false)` the front one, `None` if the defaults stay unchanged.
fn choose_default(b_type: i32, b_s: &Settings, f_type: i32, f_s: &Settings) -> Option<bool> {
    if !b_s.has_options {
        return if !f_s.has_options { None } else { Some(true) };
    } else if !f_s.has_options {
        return Some(false);
    }
    let nb = changes(b_s);
    let nf = changes(f_s);
    if nb == nf && b_s.setflags == f_s.setflags && b_type == f_type {
        return Some(true);
    }
    if nb < nf {
        return Some(true);
    }
    if nf < nb {
        return Some(false);
    }
    if b_s.setflags < f_s.setflags {
        return Some(true);
    }
    if f_s.setflags < b_s.setflags {
        return Some(false);
    }
    if b_s.flags < f_s.flags {
        return Some(true);
    }
    if f_s.flags < b_s.flags {
        return Some(false);
    }
    if f_type < b_type {
        return Some(false);
    }
    Some(true)
}

/// The options string with its square brackets; `the` selects `*\the\XFigu`.
fn put_options(out: &mut Out, s: &Settings, the: bool, ts: f32) {
    let xfigu = if the { "*\\the\\XFigu" } else { "\\XFigu" };
    out.puts("[");
    let mut sep = false;
    if s.thickness != UNSET {
        let thk = thickness_d(s.thickness, ts);
        fprintf!(out, "line width={:.*}", prec1(thk), Cf(thk));
        out.puts(xfigu);
        sep = true;
    }
    if s.wid != UNSET {
        if sep {
            out.puts(", ");
        }
        sep = true;
        fprintf!(out, "width={:.0}", Cf(s.wid));
        out.puts(xfigu);
    }
    if s.ht != UNSET {
        if sep {
            out.puts(", ");
        }
        sep = true;
        fprintf!(out, "length={:.0}", Cf(s.ht));
        out.puts(xfigu);
    }
    if s.setflags != 0 {
        if s.setflags & HAS_FILL != 0 {
            if sep {
                out.puts(", ");
            }
            sep = true;
            out.puts(ARROWFLAG[(s.flags & HAS_FILL) as usize]);
        }
        if s.setflags & HAS_SWAP != 0 {
            if sep {
                out.puts(", ");
            }
            // swap is a toggle
            out.puts(ARROWFLAG[HAS_SWAP as usize]);
        }
    }
    out.puts("]");
}

fn assign_settings(s: &Settings, o: &mut Options) {
    if s.thickness != UNSET {
        o.thickness = s.thickness;
    }
    if s.wid != UNSET {
        o.wid = s.wid;
    }
    if s.ht != UNSET {
        o.ht = s.ht;
    }
    if s.setflags != 0 {
        o.flags = s.flags | (o.flags & !s.setflags);
    }
}

fn assign_arrow(a: &TikzArrow, o: &mut TikzArrow) {
    o.type_ = a.type_;
    if a.type_ == NOARROW {
        return;
    }
    o.props = a.props;
    o.flags = a.flags;
    o.s.has_options = a.s.has_options;
    if !a.s.has_options {
        return;
    }
    o.s.thickness = a.s.thickness;
    o.s.wid = a.s.wid;
    o.s.ht = a.s.ht;
    o.s.setflags = a.s.setflags;
    if a.s.setflags != 0 {
        o.s.flags = a.s.flags;
    }
}

/// `OPTIONS_EQUAL`.
fn options_equal(a: &Settings, b: &Settings) -> bool {
    a.has_options == b.has_options
        && (!a.has_options
            || (a.thickness == b.thickness
                && a.wid == b.wid
                && a.ht == b.ht
                && a.setflags == b.setflags
                && a.flags == b.flags))
}

/// Whether the arrow `b` differs from the current arrow `cur`.
fn arrow_changed(b: &TikzArrow, cur: &TikzArrow) -> bool {
    !(b.type_ == NOARROW && cur.type_ == NOARROW)
        && (b.type_ != cur.type_
            || b.s.has_options != cur.s.has_options
            || (b.s.has_options && cur.s.has_options && !options_equal(&b.s, &cur.s)))
}

/// `PUT_POINT`.
fn put_point(out: &mut Out, xs: i32, ys: i32, p: Point) {
    fprintf!(
        out,
        "({},{})",
        p.x.wrapping_add(xs),
        p.y.wrapping_neg().wrapping_add(ys)
    );
}

/// A point of an arrow outline or of a hull.
type DP = crate::object::DPoint;

impl Tikz {
    fn xc(&self, x: i32) -> i32 {
        x.wrapping_add(self.xshift)
    }
    fn yc(&self, y: i32) -> i32 {
        y.wrapping_neg().wrapping_add(self.yshift)
    }

    /// `\pgfdeclarearrow` of tikz type `ttype`, drawn like arrow shape `indx`.
    fn define_arrow(&self, out: &mut Out, ctx: &mut Ctx, ttype: i32, indx: usize) {
        let Some(pg) = usize::try_from(ttype).ok().and_then(|i| PGFARROW.get(i)) else {
            return;
        };
        let Some(shape) = ARROW_SHAPES.get(indx) else {
            return;
        };
        let dopt = self.default_options;
        fprintf!(out, "\\pgfdeclarearrow{{\n  name = xfiga{},\n", ttype);
        out.puts("  parameters = {\n    \\the\\pgfarrowlinewidth");
        if HAS_LENGTH & pg.props != 0 {
            out.puts(" \\the\\pgfarrowlength");
        }
        if HAS_WIDTH & pg.props != 0 {
            out.puts(" \\the\\pgfarrowwidth");
        }
        if HAS_FILL & pg.props != 0 {
            out.puts("\\ifpgfarrowopen o\\fi");
        }
        out.puts("},\n  defaults = {\n");
        fprintf!(
            out,
            "\t  line width=+{:.*}\\XFigu",
            prec1(dopt.thickness),
            Cf(dopt.thickness)
        );
        if HAS_LENGTH & pg.props != 0 {
            fprintf!(out, ", length=+{:.*}\\XFigu", prec1(dopt.ht), Cf(dopt.ht));
        }
        if NOT_CIRCLE & pg.props != 0 {
            fprintf!(out, ", width=+{:.*}\\XFigu", prec1(dopt.wid), Cf(dopt.wid));
        }
        out.puts("},\n  setup code = {\n");

        if shape.tipmv > 0. {
            out.puts("    % miter protrusion = thk * sqrt(wd^2 + (tipmv*len)^2) / (2 * wd)\n");
            // CHANGE the 2 below if any .tipmv has more than 2 digits precision!
            fprintf!(
                out,
                "    \\dimen7 {:.*}\\pgfarrowlength",
                precision(2, shape.tipmv),
                Cf(shape.tipmv)
            );
            out.puts("\\pgfmathveclen{\\the\\dimen7}{\\the\\pgfarrowwidth}\n");
            out.puts("    \\dimen7 2\\pgfarrowwidth\\pgfmathdivide{\\pgfmathresult}{\\the\\dimen7}\n");
            out.puts("    \\dimen7 \\pgfmathresult\\pgfarrowlinewidth\n");
        } else if shape.tipmv == 0.0 {
            out.puts("    \\dimen7 0.5\\pgfarrowlinewidth\n");
        } else {
            out.puts("    \\dimen7 0pt\n");
        }
        out.puts("    \\pgfarrowssettipend{+\\dimen7}\n");

        out.puts("    \\pgfarrowssetbackend{+");
        texdimen(out, pg.backl, "\\pgfarrowlength");
        out.puts("}\n");

        if pg.lendl == 0. {
            out.puts("    \\dimen9 -0.5\\pgfarrowlinewidth\n");
            out.puts("    \\pgfarrowssetvisualbackend{+\\dimen9}\n");
            out.puts("    \\pgfarrowssetlineend{+");
            texdimen(out, pg.lendt, "\\pgfarrowlinewidth");
            out.puts("}\n");
        } else {
            out.puts("    \\dimen9 ");
            texdimen(out, pg.lendl, "\\pgfarrowlength");
            out.puts("\\advance\\dimen9 by");
            texdimen(out, pg.lendt, "\\pgfarrowlinewidth");
            out.puts("\n");
            out.puts("    \\pgfarrowssetlineend{+\\dimen9}\n");
            out.puts("    \\dimen9 ");
            texdimen(out, pg.lendl, "\\pgfarrowlength");
            out.puts("\\advance\\dimen9 by-0.5\\pgfarrowlinewidth\n");
            out.puts("    \\pgfarrowssetvisualbackend{+\\dimen9}\n");
        }

        if pg.tiphull {
            out.puts("    \\pgfarrowshullpoint{+\\dimen7}{+0pt}\n");
        }
        out.puts("    ");
        if pg.nhull > 0 {
            for h in pg.hullp.iter().take(pg.nhull as usize) {
                out.puts("\\pgfarrowsupperhullpoint{+");
                texdimen(out, h.0, "\\pgfarrowlength");
                out.puts("}{+");
                texdimen(out, h.1, "\\pgfarrowwidth");
                out.puts("}");
            }
        } else {
            // nhull == -1
            let h = pg.hullp[0];
            fprintf!(
                out,
                "\\pgfarrowshullpoint{{+{}\\pgfarrowlength}}{{+{:.1}\\pgfarrowwidth}}",
                fmt_g(h.0, 3),
                Cf(-h.1)
            );
        }
        out.puts("\n");
        if pg.endhull {
            out.puts("    \\pgfarrowshullpoint{+\\dimen9}{+0pt}\n");
        }

        if IS_HOOK & pg.props != 0 {
            // abuse \pgfarrowinset to hold the y-shift
            out.puts("    \\pgfarrowinset 0.5\\pgfarrowlinewidth");
            out.puts("\\advance\\pgfarrowinset by -0.5\\pgflinewidth\n");
            out.puts("    \\pgfarrowssavethe\\pgfarrowinset\n");
        }

        out.puts("    \\pgfarrowssavethe\\pgfarrowlinewidth\n");
        if HAS_LENGTH & pg.props != 0 {
            out.puts("    \\pgfarrowssavethe\\pgfarrowlength\n");
        }
        if HAS_WIDTH & pg.props != 0 {
            out.puts("    \\pgfarrowssavethe\\pgfarrowwidth\n");
        }

        out.puts("  },\n  drawing code = {\\pgfsetdash{}{+0pt}\n");
        out.puts("    \\ifdim\\pgfarrowlinewidth=\\pgflinewidth");
        out.puts("\\else\\pgfsetlinewidth{+\\pgfarrowlinewidth}\\fi\n");
        let pts: &[DP] = shape.points;
        let path_point = |out: &mut Out, cmd: &str, p: &DP| {
            out.puts(cmd);
            texdimen(out, p.x, "\\pgfarrowlength");
            out.puts("}{");
            texdimen(out, -p.y, "\\pgfarrowwidth");
            out.puts("}}\n");
        };
        if NOT_CIRCLE & pg.props != 0 {
            // neither circle nor half-circle
            if IS_HOOK & pg.props != 0 {
                out.puts("    \\pgftransformyshift{-\\pgfarrowinset}\n");
            }
            if let Some(first) = pts.first() {
                path_point(out, "    \\pgfpathmoveto{\\pgfqpoint{", first);
            }
            let n = pts.len();
            for p in pts.iter().take(n.saturating_sub(1)).skip(1) {
                path_point(out, "    \\pgfpathlineto{\\pgfqpoint{", p);
            }
            if let (Some(first), Some(last)) = (pts.first(), pts.last()) {
                if last.x == first.x && last.y == first.y {
                    out.puts("    \\pgfpathclose\n");
                } else {
                    path_point(out, "    \\pgfpathlineto{\\pgfqpoint{", last);
                }
            }
        } else if ttype == 5 {
            // circle
            out.puts("    \\dimen3 0.5\\pgfarrowlength\n");
            out.puts("    \\pgfpathcircle{\\pgfqpoint{-\\dimen3}{0pt}}{+\\dimen3}\n");
        } else if ttype == 6 {
            // half-circle
            out.puts("    \\dimen3 0.5\\pgfarrowlength\n");
            out.puts("    \\pgfpathmoveto{\\pgfqpoint{0pt}{\\dimen3}}\n");
            out.puts("    \\pgfpatharctoprecomputed{\\pgfpointorigin}{90}{270}");
            out.puts("{\\pgfqpoint{0pt}{-\\dimen3}}{\\the\\dimen3}{\\the\\dimen3}{1}{1}\n");
        } else {
            ctx.warn(format!(
                "The tikz-arrowtype {ttype} does not exist. Please report this bug."
            ));
        }

        if !shape.fillpoints.is_empty() {
            // not a simple fill
            out.puts("%%\n    \\pgfusepathqstroke\n  \\pgfpathmoveto{\\pgfqpoint{");
            for (i, p) in shape.fillpoints.iter().enumerate() {
                if i == 0 {
                    texdimen(out, p.x, "\\pgfarrowlength");
                    out.puts("}{");
                    texdimen(out, -p.y, "\\pgfarrowwidth");
                    out.puts("}}\n");
                } else {
                    path_point(out, "    \\pgfpathlineto{\\pgfqpoint{", p);
                }
            }
            out.puts("    \\pgfsetfillcolor{pgfstrokecolor}\\pgfusepathqfill\n");
        } else if HAS_FILL & pg.props != 0 {
            out.puts("    \\ifpgfarrowopen\\pgfusepathqstroke\\else\\pgfsetfillcolor{pgfstrokecolor}\n");
            out.puts("\t\\ifdim\\pgfarrowlinewidth>0pt\\pgfusepathqfillstroke");
            out.puts("\\else\\pgfusepathqfill\\fi\\fi\n");
        } else {
            out.puts("    \\pgfusepathqstroke\n");
        }
        out.puts("  }\n}\n");
    }

    fn set_width(&mut self, out: &mut Out, w: i32) {
        if w == self.cur_thickness {
            return;
        }
        self.cur_thickness = w;
        let v = thickness_i(w, self.thick_scale);
        let l = usize::from(v != f64::from(v as i32));
        fprintf!(out, "\\pgfsetlinewidth{{+{:.*}\\XFigu}}\n", l, Cf(v));
    }

    fn set_stipple(&mut self, out: &mut Out, ctx: &mut Ctx, s: i32, mut v: f64) {
        if s == self.cur_style && v == self.cur_styleval {
            return;
        }
        self.cur_style = s;
        self.cur_styleval = v;
        if s == SOLID_LINE || v <= 0. {
            out.puts("\\pgfsetdash{}{+0pt}\n");
            if v <= 0. {
                self.cur_styleval = 0.;
            }
            return;
        }
        v *= self.ppi / 80.;
        let dot = round(self.ppi / 80.);
        match s {
            DASH_LINE => {
                let l = round(v);
                fprintf!(out, "\\pgfsetdash{{{{+{l}\\XFigu}}{{+{l}\\XFigu}}}}{{++0pt}}\n");
            }
            DOTTED_LINE => {
                fprintf!(
                    out,
                    "\\pgfsetdash{{{{+{dot}\\XFigu}}{{+{}\\XFigu}}}}{{+{dot}\\XFigu}}\n",
                    round(v)
                );
            }
            DASH_DOT_LINE => {
                let l = round_mul(v, 0.5);
                fprintf!(
                    out,
                    "\\pgfsetdash{{{{+{}\\XFigu}}{{+{l}\\XFigu}}{{+{dot}\\XFigu}}{{+{l}\\XFigu}}}}{{+0pt}}\n",
                    round(v)
                );
            }
            DASH_2_DOTS_LINE => {
                fprintf!(
                    out,
                    "\\pgfsetdash{{{{+{}\\XFigu}}{{+{}\\XFigu}}{{+{dot}\\XFigu}}{{+{}\\XFigu}}{{+{dot}\\XFigu}}{{+{}\\XFigu}}}}{{+0pt}}\n",
                    round(v),
                    round_mul(v, 0.45),
                    round_mul(v, 0.333),
                    round_mul(v, 0.45)
                );
            }
            DASH_3_DOTS_LINE => {
                fprintf!(
                    out,
                    "\\pgfsetdash{{{{+{}\\XFigu}}{{+{}\\XFigu}}{{+{dot}\\XFigu}}{{+{}\\XFigu}}{{+{dot}\\XFigu}}{{+{}\\XFigu}}{{+{dot}\\XFigu}}{{+{}\\XFigu}}}}{{+0pt}}\n",
                    round(v),
                    round_mul(v, 0.4),
                    round_mul(v, 0.3),
                    round_mul(v, 0.3),
                    round_mul(v, 0.4)
                );
            }
            _ => ctx.warn(format!("Undefined line style {s}.")),
        }
    }

    fn set_capstyle(&mut self, out: &mut Out, c: i32) {
        const CAPCMD: [&str; 3] = ["\\pgfsetbuttcap\n", "\\pgfsetroundcap\n", "\\pgfsetrectcap\n"];
        if c == self.cur_capstyle {
            return;
        }
        self.cur_capstyle = c;
        if let Some(cmd) = usize::try_from(c).ok().and_then(|i| CAPCMD.get(i)) {
            out.puts(cmd);
        }
    }

    fn set_joinstyle(&mut self, out: &mut Out, j: i32) {
        const JOINCMD: [&str; 3] = ["\\pgfsetmiterjoin\n", "\\pgfsetroundjoin\n", "\\pgfsetbeveljoin\n"];
        if j == self.cur_joinstyle {
            return;
        }
        self.cur_joinstyle = j;
        if let Some(cmd) = usize::try_from(j).ok().and_then(|i| JOINCMD.get(i)) {
            out.puts(cmd);
        }
    }

    fn set_linecolor(&mut self, out: &mut Out, mut c: i32) {
        if c == self.default_color {
            c = DEFAULT;
        }
        if c == self.cur_pencolor {
            return;
        }
        self.cur_pencolor = c;
        fprintf!(out, "\\pgfsetstrokecolor{{{}}}\n", colorname(c));
    }

    fn set_linefillcolor(&mut self, out: &mut Out, mut c: i32) {
        if c == self.default_color {
            c = DEFAULT;
        }
        if c == self.cur_pencolor && c == self.cur_fillcolor && self.cur_fillstyle == NUMSHADES - 1 {
            return;
        }
        self.cur_pencolor = c;
        self.cur_fillcolor = c;
        self.cur_fillstyle = NUMSHADES - 1;
        fprintf!(out, "\\pgfsetcolor{{{}}}\n", colorname(c));
    }

    fn set_fillcolor(&mut self, out: &mut Out, mut c: i32, s: i32) {
        if c == self.default_color {
            c = DEFAULT;
        }
        if c != self.cur_fillcolor || s != self.cur_fillstyle {
            self.cur_fillcolor = c;
            self.cur_fillstyle = s;
            fprintf!(out, "\\pgfsetfillcolor{{{}", colorname(c));
            // A shade between 0 and NUMSHADES - 1 mixes the color with
            // black, shades > NUMSHADES - 1 mix it with white.
            if s == NUMSHADES - 1 {
                out.puts("}\n");
            } else if s < NUMSHADES {
                fprintf!(out, "!{}!black}}\n", 100i32.wrapping_mul(s) / (NUMSHADES - 1));
            } else {
                fprintf!(
                    out,
                    "!{}}}\n",
                    100i32.wrapping_mul(NUMTINTS.wrapping_sub(s).wrapping_add(NUMSHADES - 1)) / NUMTINTS
                );
            }
        }
    }

    fn set_pattern(&mut self, out: &mut Out, pen_color: i32, fill_style: i32) {
        if pen_color == self.cur_patcolor && fill_style == self.cur_fillstyle {
            return;
        }
        self.cur_patcolor = pen_color;
        self.cur_fillstyle = fill_style;
        fprintf!(
            out,
            "\\pgfsetfillpattern{{xfigp{}}}{{{}}}\n",
            fill_style.wrapping_sub(NUMSHADES).wrapping_sub(NUMTINTS),
            colorname(pen_color)
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn put_drawcmd(
        &mut self,
        out: &mut Out,
        ctx: &mut Ctx,
        style: i32,
        thickness: i32,
        pen_color: i32,
        mut fill_color: i32,
        mut fill_style: i32,
        style_val: f64,
        has_endcaps: bool,
        cap_style: i32,
    ) {
        let filled = fill_style > UNFILLED && fill_style < NUMSHADES + NUMTINTS;
        let pattern = fill_style >= NUMSHADES + NUMTINTS;
        if thickness > 0 {
            if has_endcaps || (style != SOLID_LINE && style_val > 0.) {
                self.set_capstyle(out, cap_style);
            }
            self.set_width(out, thickness);
            self.set_stipple(out, ctx, style, style_val);
            if filled {
                normalize(&mut fill_color, &mut fill_style);
                if fill_color == pen_color && fill_style == NUMSHADES - 1 {
                    self.set_linefillcolor(out, pen_color);
                } else {
                    self.set_linecolor(out, pen_color);
                    self.set_fillcolor(out, fill_color, fill_style);
                }
                out.puts("\\filldraw ");
            } else if pattern {
                self.set_linecolor(out, pen_color);
                self.set_pattern(out, pen_color, fill_style);
                out.puts("\\draw[pattern,preaction={fill=");
                out.puts(&colorname(fill_color));
                out.puts("}] ");
            } else {
                self.set_linecolor(out, pen_color);
                out.puts("\\draw ");
            }
        } else if filled {
            normalize(&mut fill_color, &mut fill_style);
            self.set_fillcolor(out, fill_color, fill_style);
            out.puts("\\fill ");
        } else if pattern {
            self.set_pattern(out, pen_color, fill_style);
            out.puts("\\pattern[preaction={fill=");
            out.puts(&colorname(fill_color));
            out.puts("}] ");
        } else {
            ctx.warn("A non-existing object was processed. Please report this bug.");
        }
    }

    /// Set the default arrow options and the arrows for the next path.
    fn set_arrows(&mut self, out: &mut Out, back: Option<&Arrow>, forw: Option<&Arrow>) {
        let dopt = self.default_options;
        // arrows with a type outside the table don't exist
        fn mk<'a>(a: Option<&'a Arrow>, o: &Options) -> Option<(&'a Arrow, TikzArrow)> {
            a.and_then(|a| create_arrow(a, o).map(|t| (a, t)))
        }
        let back = mk(back, &dopt);
        let forw = mk(forw, &dopt);
        let mut b = NO_ARROW;
        let mut f = NO_ARROW;
        // which settings become the default options: Some(true) b's
        let mut d: Option<bool> = None;
        match (back, forw) {
            (Some((ba, bt)), Some((fa, ft))) => {
                b = bt;
                // a shortcut for back == forw
                if ba.type_ == fa.type_
                    && ba.style == fa.style
                    && ba.thickness == fa.thickness
                    && ba.wid == fa.wid
                    && ba.ht == fa.ht
                {
                    d = Some(true);
                    f.type_ = b.type_;
                    f.props = b.props;
                    f.flags = b.flags;
                    f.s.has_options = false;
                } else {
                    f = ft;
                    d = choose_default(b.type_, &b.s, f.type_, &f.s);
                    match d {
                        Some(true) => {
                            f.s = get_settings(fa, f.flags, f.props, combine(&b.s, &dopt));
                        }
                        Some(false) => {
                            b.s = get_settings(ba, b.flags, b.props, combine(&f.s, &dopt));
                        }
                        None => {}
                    }
                }
            }
            (Some((_, bt)), None) => {
                b = bt;
                d = Some(true);
            }
            (None, Some((_, ft))) => {
                f = ft;
                d = Some(false);
            }
            (None, None) => {}
        }

        // Output the command to set the default options.
        if let Some(is_b) = d {
            let s = if is_b { b.s } else { f.s };
            if s.has_options {
                out.puts("\\pgfsetarrows{");
                put_options(out, &s, false, self.thick_scale);
                out.puts("}\n");
                assign_settings(&s, &mut self.default_options);
                if is_b {
                    b.s.has_options = false;
                } else {
                    f.s.has_options = false;
                }
            }
        }

        // Which arrows must be set
        let set_back = arrow_changed(&b, &self.cur_b);
        if set_back {
            assign_arrow(&b, &mut self.cur_b);
        }
        let set_forw = arrow_changed(&f, &self.cur_f);
        if set_forw {
            assign_arrow(&f, &mut self.cur_f);
        }

        if set_back || set_forw {
            out.puts("\\pgfsetarrows");
            if !set_forw {
                out.puts("start{");
            } else if !set_back {
                out.puts("end{");
            } else {
                out.puts("{");
            }
        }
        if set_back && b.type_ != NOARROW {
            fprintf!(out, "xfiga{}", b.type_);
            if b.s.has_options {
                put_options(out, &b.s, true, self.thick_scale);
            }
        }
        if set_back && set_forw {
            out.puts("-");
        }
        if set_forw && f.type_ != NOARROW {
            fprintf!(out, "xfiga{}", f.type_);
            if f.s.has_options {
                put_options(out, &f.s, true, self.thick_scale);
            }
        }
        if set_back || set_forw {
            out.puts("}\n");
        }
    }

    /// The points of `pts`, each as `--(x,y)`, lines broken when long.
    fn points_all(&self, out: &mut Out, pts: &[Point]) {
        let mut len = 6; // "\draw " is 6 chars
        for p in pts {
            if len > MINLINELENGTH {
                out.puts("\n  ");
                len = 2;
            }
            let s = format!("--({},{})", self.xc(p.x), self.yc(p.y));
            len += s.len();
            out.puts(&s);
        }
        out.puts(";\n");
    }

    fn put_picture(&self, out: &mut Out, p: Point, r: Point, pic: &crate::object::Pic) {
        let mut dx = r.x.wrapping_sub(p.x);
        let mut dy = r.y.wrapping_sub(p.y);
        let mut rot = 0;
        // get the rotation, and write the height and width to dx and dy
        if dx < 0 && dy < 0 {
            dx = dx.wrapping_neg();
            dy = dy.wrapping_neg();
            rot = 180;
        } else if dx < 0 && dy >= 0 {
            rot = dy;
            dy = dx.wrapping_neg();
            dx = rot;
            rot = 270;
        } else if dy < 0 && dx >= 0 {
            rot = dx;
            dx = dy.wrapping_neg();
            dy = rot;
            rot = 90;
        }
        if pic.flipped != 0 {
            rot += 90;
            std::mem::swap(&mut dx, &mut dy);
        }
        // The name's bytes up to a NUL, as C holds them.
        let file = pic.name_bytes().split(|&b| b == 0).next().unwrap_or_default();
        let file = if self.removesuffix {
            file.iter()
                .rposition(|&c| c == b'.')
                .map_or(file, |i| file.get(..i).unwrap_or(file))
        } else {
            file
        };
        out.puts("\\node[below right");
        if rot != 0 && rot != 360 {
            fprintf!(out, ",rotate={rot}");
        }
        if pic.flipped != 0 {
            out.puts(",xscale=-1");
        }
        fprintf!(
            out,
            "] at ({},{}) {{\\pgfimage[width=+{dx}\\XFigu,height=+{dy}\\XFigu]{{{}",
            self.xc(p.x),
            self.yc(p.y),
            self.prepend.as_deref().unwrap_or("")
        );
        out.bytes(file);
        out.puts("}};\n");
    }
}

impl Driver for Tikz {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        let a = arg.unwrap_or("");
        match opt {
            // border margin around the figure
            'b' => self.border_margin = atoi(a),
            // default colour
            'C' => self.default_color = atoi(a),
            'F' => self.select_fontname = false,
            // prepend this string to included graphics files
            'i' => self.prepend = Some(a.to_string()),
            // the default text font
            'f' => {
                let tex = (1..=MAX_FONT as usize).find(|&i| {
                    self.fonts.names.get(i).is_some_and(|n| n == a) || self.fonts.fonts.get(i).is_some_and(|n| n == a)
                });
                if let Some(i) = tex {
                    if let Some(v) = self.fonts.fonts.get(i).cloned() {
                        self.fonts.fonts[0] = v;
                    }
                } else if PS_FONT_NAMES
                    .get(1..=MAX_PSFONT as usize)
                    .is_some_and(|names| names.contains(&a))
                {
                    self.fonts.psfonts[0] = a.to_string().into();
                } else {
                    ctx.warn(format!("warning: non-standard font name {a} ignored"));
                }
            }
            'o' => self.select_fontsize = false,
            'T' => self.only_texfonts = true,
            'O' => self.allspecial = true,
            'P' => self.pagemode = true,
            'v' => self.verbose = true,
            'W' => self.figscaling = false,
            // remove the suffix from included graphics files
            'w' => self.removesuffix = true,
            'G' | 'L' => {}
            _ => return Err(format!("Argument -{opt} unknown to tikz driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        set_default_size(&mut self.fonts, ctx);
        let unitlength = ctx.mag / ctx.ppi;
        self.border_margin = (f64::from(self.border_margin) / (unitlength * 72.0)) as i32;
        // 1 in = 72.27 x 65536 sp
        let splength = 4736286.72 * unitlength;
        let bm = self.border_margin;
        ctx.llx = ctx.llx.wrapping_sub(bm);
        ctx.lly = ctx.lly.wrapping_sub(bm);
        ctx.urx = ctx.urx.wrapping_add(bm);
        ctx.ury = ctx.ury.wrapping_add(bm);
        self.thick_scale = ctx.thick_scale;
        self.ppi = ctx.ppi;
        self.xshift = shift_coordinate(ctx.llx, ctx.ppi);
        self.yshift = shift_coordinate(ctx.ury.wrapping_neg(), ctx.ppi);
        let sp = splength as i64;

        if !objects.comments.is_empty() {
            out.puts("%\n");
            print_comments(out, "% ", &objects.comments, "");
            out.puts("%\n");
        }
        if self.pagemode {
            // add a tiny extra margin ( + 1), otherwise 2 pages result
            let mut width = f64::from(ctx.urx.wrapping_sub(ctx.llx).wrapping_add(1)) * unitlength;
            let mut height = f64::from(ctx.ury.wrapping_sub(ctx.lly).wrapping_add(1)) * unitlength;
            let mut unit = "in";
            if ctx.metric {
                width *= 2.54;
                height *= 2.54;
                unit = "cm";
            }
            out.puts("\\documentclass{minimal}\n");
            fprintf!(
                out,
                "\\usepackage[papersize={{{}{unit},{}{unit}}},margin=0pt]{{geometry}}\n",
                fmt_g(width, 4),
                fmt_g(height, 4)
            );
            out.puts("\\usepackage{tikz}\n");
            if ctx.arrows_used {
                out.puts("\\usetikzlibrary{arrows.meta,bending}\n");
            }
            if ctx.pats_used {
                out.puts("\\usetikzlibrary{patterns}\n");
            }
            out.puts("\\parindent0pt\n\\begin{document}\n");
        }
        if !self.pagemode {
            out.puts("{\\pgfkeys{/pgf/fpu/.try=false}%\n");
        }
        if self.pagemode || !self.figscaling {
            fprintf!(out, "\\tikzpicture[x=+{sp}sp, y=+{sp}sp]\n");
            fprintf!(out, "\\newdimen\\XFigu\\XFigu{sp}sp\n");
        } else {
            out.puts("\\ifx\\XFigwidth\\undefined\\dimen1=0pt");
            out.puts("\\else\\dimen1\\XFigwidth\\fi\n");
            fprintf!(out, "\\divide\\dimen1 by {}\n", ctx.urx.wrapping_sub(ctx.llx));
            out.puts("\\ifx\\XFigheight\\undefined\\dimen3=0pt");
            out.puts("\\else\\dimen3\\XFigheight\\fi\n");
            fprintf!(out, "\\divide\\dimen3 by {}\n", ctx.ury.wrapping_sub(ctx.lly));
            out.puts("\\ifdim\\dimen1=0pt\\ifdim\\dimen3=0pt");
            fprintf!(out, "\\dimen1={sp}sp\\dimen3\\dimen1\n");
            out.puts("  \\else\\dimen1\\dimen3\\fi");
            out.puts("\\else\\ifdim\\dimen3=0pt\\dimen3\\dimen1\\fi\\fi\n");
            out.puts("\\tikzpicture[x=+\\dimen1, y=+\\dimen3]\n");
        }
        if self.pagemode {
            fprintf!(out, "\\newdimen\\XFigu\\XFigu{sp}sp\n");
        } else {
            out.puts("{\\ifx\\XFigu\\undefined\\catcode`\\@11\n");
            // \newdimen can only be used in \outer mode: define \temp like
            // \newdimen in plain.tex, without \outer
            out.puts("\\def\\temp{\\alloc@1\\dimen\\dimendef\\insc@unt}");
            out.puts("\\temp\\XFigu\\catcode`\\@12\\fi}\n");
            fprintf!(out, "\\XFigu{sp}sp\n");
            if self.figscaling {
                out.puts("% Uncomment to scale line thicknesses with the same\n");
                out.puts("% factor as width of the drawing.\n");
                out.puts("%\\pgfextractx\\XFigu{\\pgfqpointxy{1}{1}}\n");
                out.puts("\\ifdim\\XFigu<0pt\\XFigu-\\XFigu\\fi\n");
            }
        }

        if ctx.pats_used {
            if self.pagemode {
                for (i, def) in PATTERNS.iter().enumerate().take(NUMPATTERNS as usize) {
                    if ctx.pattern_used[i] {
                        out.puts(def);
                    }
                }
            } else {
                out.puts("\\catcode`\\@11\n");
                for (i, def) in PATTERNS.iter().enumerate().take(NUMPATTERNS as usize) {
                    if ctx.pattern_used[i] {
                        fprintf!(out, "\\pgfutil@ifundefined{{pgf@pattern@name@xfigp{i}}}{{\n");
                        out.puts(def);
                        out.puts("}{}\n");
                    }
                }
                out.puts("\\catcode`\\@12\n");
            }
        }
        let ts = f64::from(ctx.thick_scale);
        self.default_options = Options {
            thickness: 0.5 * ts,
            wid: f64::from(4.0f32 * ctx.thick_scale),
            ht: f64::from(8.0f32 * ctx.thick_scale),
            flags: HAS_FILL,
        };
        if ctx.arrows_used {
            for i in (0..NUMARROWS).step_by(2) {
                let t = (i / 2) as i32;
                if ctx.arrow_used[i] {
                    let mut ttype = arrowtype(t, 0);
                    self.define_arrow(out, ctx, ttype, i);
                    ttype += 1;
                    if ctx.arrow_used[i + 1] && ttype == arrowtype(t, 1) {
                        self.define_arrow(out, ctx, ttype, i + 1);
                    }
                } else if ctx.arrow_used[i + 1] {
                    self.define_arrow(out, ctx, arrowtype(t, 1), i + 1);
                }
            }
        }

        for (i, &(name, r, g, b)) in STD_COLOR.iter().enumerate().skip(9) {
            // i - 1: no default color in std_color_used
            if ctx.std_color_used.get(i - 1).copied().unwrap_or(false) {
                fprintf!(
                    out,
                    "\\definecolor{{{name}}}{{rgb}}{{{},{},{}}}\n",
                    fmt_g(f64::from(r), 2),
                    fmt_g(f64::from(g), 2),
                    fmt_g(f64::from(b), 2)
                );
            }
        }
        for (i, c) in ctx.user_colors.iter().enumerate() {
            fprintf!(
                out,
                "\\definecolor{{xfigc{}}}{{rgb}}{{{:.3},{:.3},{:.3}}}\n",
                i + NUM_STD_COLS,
                f64::from(c.r) / 255.,
                f64::from(c.g) / 255.,
                f64::from(c.b) / 255.
            );
        }
        fprintf!(
            out,
            "\\clip({},{}) rectangle ({},{});\n",
            self.xc(ctx.llx),
            self.yc(ctx.ury),
            self.xc(ctx.urx),
            self.yc(ctx.lly)
        );
        out.puts("\\tikzset{inner sep=+0pt, outer sep=+0pt}\n");
        Ok(())
    }

    fn grid(&mut self, ctx: &mut Ctx, out: &mut Out, mut major: f32, mut minor: f32) -> Result<(), String> {
        if minor == 0.0 && major == 0.0 {
            return Ok(());
        }
        if ctx.metric {
            // Cancel out the factor by which the figure is multiplied
            minor = (f64::from(minor) * (3.81 / 4.)) as f32;
            major = (f64::from(major) * (3.81 / 4.)) as f32;
        }
        let ts = f64::from(ctx.thick_scale);
        let (llx, lly, urx, ury) = (ctx.llx, ctx.lly, ctx.urx, ctx.ury);
        let line = |out: &mut Out, w: f64, step: f32| {
            fprintf!(
                out,
                "\\draw[black!30, line width={:.*}\\XFigu] ({},{}) grid[step={:.2}\\XFigu] ({},{});\n",
                prec1(w),
                Cf(w),
                self.xc(llx),
                self.yc(ury),
                Cf(f64::from(step)),
                self.xc(urx),
                self.yc(lly)
            );
        };
        // first the minor grid
        if minor != 0.0 {
            line(out, 0.5 * ts, minor);
        }
        if major != 0.0 {
            line(out, 1.25 * ts, major);
        }
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        if self.verbose {
            out.puts("%\n% Fig ARC object\n%\n");
        }
        print_comments(out, "% ", &a.comments, "");
        if a.thickness <= 0 && a.fill_style == UNFILLED && a.for_arrow.is_none() && a.back_arrow.is_none() {
            return Ok(());
        }
        // compute the angles in the tikz coordinate system
        let d1x = f64::from(a.point[0].x) - a.center.x;
        let d1y = f64::from(a.point[0].y) - a.center.y;
        let d2x = f64::from(a.point[2].x) - a.center.x;
        let d2y = f64::from(a.point[2].y) - a.center.y;
        let mut angle1 = (-d1y).atan2(d1x) * 180. / PI;
        let mut angle2 = (-d2y).atan2(d2x) * 180. / PI;
        let rad = 0.5 * (d1x.mul_add(d1x, d1y * d1y).sqrt() + d2x.mul_add(d2x, d2y * d2y).sqrt());
        // how precise must the angle be given? 1/rad is the view angle of
        // one pixel
        let da = 180.0 / PI / rad;
        let mut preca = 0usize;
        let mut d = 1.0;
        while da < 1.0 / d && preca < MAX_PRECISION {
            preca += 1;
            d *= 10.;
        }

        if a.direction != 0 {
            // counter-clockwise
            if angle1 > angle2 {
                if angle1 > 180. {
                    angle1 -= 360.;
                } else {
                    angle2 += 360.;
                }
            }
        } else if angle1 < angle2 {
            // clockwise
            if angle1 > 180. {
                angle2 -= 360.;
            } else {
                angle1 += 360.;
            }
        }

        if a.thickness > 0 && a.type_ == T_PIE_WEDGE_ARC {
            self.set_joinstyle(out, MITERJOIN);
        }
        self.set_arrows(out, a.back_arrow.as_ref(), a.for_arrow.as_ref());
        self.put_drawcmd(
            out,
            ctx,
            a.style,
            a.thickness,
            a.pen_color,
            a.fill_color,
            a.fill_style,
            a.style_val,
            a.type_ != T_OPEN_ARC && !(a.for_arrow.is_some() && a.back_arrow.is_some()),
            a.cap_style,
        );
        let radprec = usize::from(f64::from(round(rad)) != f64::from(round_mul(10., rad)) / 10.);
        fprintf!(
            out,
            "({},{}) arc[start angle=+{:.*}, end angle=+{:.*}, radius=+{:.*}]",
            self.xc(a.point[0].x),
            self.yc(a.point[0].y),
            preca,
            Cf(angle1),
            preca,
            Cf(angle2),
            radprec,
            Cf(rad)
        );
        if a.type_ == T_PIE_WEDGE_ARC {
            fprintf!(
                out,
                "--({:.0},{:.0})--cycle;\n",
                Cf(a.center.x + f64::from(self.xshift)),
                Cf(-a.center.y + f64::from(self.yshift))
            );
        } else {
            // T_OPEN_ARC or unknown
            out.puts(";\n");
        }
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        if self.verbose {
            out.puts("%%\n%% Fig ELLIPSE object\n%%\n");
        }
        print_comments(out, "% ", &e.comments, "");
        if e.thickness <= 0 && e.fill_style == UNFILLED {
            return Ok(());
        }
        self.put_drawcmd(
            out,
            ctx,
            e.style,
            e.thickness,
            e.pen_color,
            e.fill_color,
            e.fill_style,
            e.style_val,
            false,
            ROUNDCAP,
        );
        if e.radiuses.x == e.radiuses.y {
            fprintf!(
                out,
                " ({},{}) circle [radius=+{}];\n",
                self.xc(e.center.x),
                self.yc(e.center.y),
                e.radiuses.x
            );
        } else {
            fprintf!(
                out,
                " ({},{}) ellipse [x radius=+{},y radius=+{}",
                self.xc(e.center.x),
                self.yc(e.center.y),
                e.radiuses.x,
                e.radiuses.y
            );
            if e.angle != 0.0 {
                fprintf!(out, ",rotate=+{:.0}];\n", Cf(e.angle * DEG_RAD));
            } else {
                out.puts("];\n");
            }
        }
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        if self.verbose {
            out.puts("%\n% Fig POLYLINE object\n%\n");
        }
        print_comments(out, "% ", &l.comments, "");
        let Some(&p) = l.points.first() else {
            return Ok(());
        };
        let q = l.points.get(1).copied();
        let r = l.points.get(2).copied();

        // One or two points; be strict, only draw a line if it is a POLYLINE.
        if q.is_none() || r.is_none() {
            if l.thickness <= 0 || l.type_ != T_POLYLINE {
                return Ok(());
            }
            self.set_width(out, l.thickness);
            self.set_linecolor(out, l.pen_color);
            if let Some(q) = q {
                // two points
                self.set_capstyle(out, l.cap_style);
                self.set_stipple(out, ctx, l.style, l.style_val);
                self.set_arrows(out, l.back_arrow.as_ref(), l.for_arrow.as_ref());
                fprintf!(
                    out,
                    "\\draw ({},{})--({},{});\n",
                    self.xc(p.x),
                    self.yc(p.y),
                    self.xc(q.x),
                    self.yc(q.y)
                );
            } else {
                // a single point
                self.set_arrows(out, None, None);
                if l.cap_style == ROUNDCAP {
                    self.set_capstyle(out, l.cap_style);
                    // a zero-length line is only displayed with round caps
                    fprintf!(
                        out,
                        "\\draw ({},{})--({},{});\n",
                        self.xc(p.x),
                        self.yc(p.y),
                        self.xc(p.x),
                        self.yc(p.y)
                    );
                } else {
                    self.set_capstyle(out, BUTTCAP);
                    let h = thickness_i(l.thickness, self.thick_scale) / 2.;
                    fprintf!(
                        out,
                        "\\draw ({},{})--({},{});\n",
                        self.xc(round(f64::from(p.x) - h)),
                        self.yc(p.y),
                        self.xc(round(f64::from(p.x) + h)),
                        self.yc(p.y)
                    );
                }
            }
            return Ok(());
        }
        let (Some(_), Some(r)) = (q, r) else {
            return Ok(());
        };
        // three or more points, from here on below
        if l.type_ == T_PIC_BOX {
            if let Some(pic) = &l.pic {
                self.put_picture(out, p, r, pic);
            }
            return Ok(());
        }

        // T_POLYLINE, T_BOX, T_POLYGON or T_ARC_BOX from here on below
        if l.thickness <= 0 && l.fill_style == UNFILLED && l.for_arrow.is_none() && l.back_arrow.is_none() {
            return Ok(());
        }
        if l.type_ == T_BOX || l.type_ == T_POLYGON || l.type_ == T_POLYLINE {
            self.set_joinstyle(out, l.join_style);
        }
        let (xs, ys) = (self.xshift, self.yshift);
        if l.type_ == T_BOX || l.type_ == T_POLYGON || l.type_ == T_ARC_BOX {
            self.put_drawcmd(
                out,
                ctx,
                l.style,
                l.thickness,
                l.pen_color,
                l.fill_color,
                l.fill_style,
                l.style_val,
                false,
                l.cap_style,
            );
            put_point(out, xs, ys, p);
            if l.type_ == T_POLYGON {
                // all but the last point of the rest
                let rest = l.points.get(1..l.points.len().saturating_sub(1)).unwrap_or(&[]);
                let mut len = 6;
                for pt in rest {
                    if len > MINLINELENGTH {
                        out.puts("\n  ");
                        len = 2;
                    }
                    let s = format!("--({},{})", self.xc(pt.x), self.yc(pt.y));
                    len += s.len();
                    out.puts(&s);
                }
                out.puts("--cycle;\n");
            } else {
                if l.type_ == T_ARC_BOX {
                    fprintf!(out, " [rounded corners=+{}\\XFigu]", l.radius);
                }
                fprintf!(out, " rectangle ({},{});\n", self.xc(r.x), self.yc(r.y));
            }
        } else {
            // T_POLYLINE
            self.set_arrows(out, l.back_arrow.as_ref(), l.for_arrow.as_ref());
            self.put_drawcmd(
                out,
                ctx,
                l.style,
                l.thickness,
                l.pen_color,
                l.fill_color,
                l.fill_style,
                l.style_val,
                true,
                l.cap_style,
            );
            put_point(out, xs, ys, p);
            self.points_all(out, l.points.get(1..).unwrap_or(&[]));
        }
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        print_comments(out, "% ", &s.comments, "");
        ctx.warn("Can't generate spline; omitting object");
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String> {
        if self.verbose {
            out.puts("%\n% Fig TEXT object\n%\n");
        }
        print_comments(out, "% ", &t.comments, "");
        self.set_fillcolor(out, t.color, NUMSHADES - 1);
        out.puts("\\pgftext[base");
        match t.type_ {
            T_LEFT_JUSTIFIED => out.puts(",left"),
            T_RIGHT_JUSTIFIED => out.puts(",right"),
            _ => {}
        }
        fprintf!(
            out,
            ",at=\\pgfqpointxy{{{}}}{{{}}}",
            self.xc(t.base_x),
            self.yc(t.base_y)
        );
        if t.angle != 0.0 {
            let a = t.angle * 180. / PI;
            let aprec = usize::from(round_mul(a, 10.) != round(a).wrapping_mul(10));
            fprintf!(out, ",rotate=+{:.*}", aprec, Cf(a));
        }
        out.puts("] {");
        let mut t = t.clone();
        self.fonts.select_font(
            out,
            ctx,
            &mut t,
            self.select_fontsize,
            self.select_fontname,
            self.only_texfonts,
        );
        put_string(out, ctx, &t.raw, t.font, t.special_text());
        out.puts("}\n");
        Ok(())
    }

    fn end(&mut self, _ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        out.puts("\\endtikzpicture");
        if self.pagemode {
            out.puts("%\n\\end{document}");
        } else {
            out.puts("}%\n");
        }
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}

/// The 22 pattern definitions (`PATTERNS` of gentikz.h).
const PATTERNS: [&str; 22] = [
    "% lines 30 degree left\n\\pgfdeclarepatternformonly{xfigp0}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{9bp}{5bp}}{\\pgfqpoint{8bp}{4bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{-1bp}{4.5bp}\\pgfpathqlineto{9bp}{-0.5bp}\n\t\\pgfusepathqstroke\n}\n",
    "% lines 30 degree right\n\\pgfdeclarepatternformonly{xfigp1}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{9bp}{5bp}}{\\pgfqpoint{8bp}{4bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{-1bp}{-0.5bp}\\pgfpathqlineto{9bp}{4.5bp}\n\t\\pgfusepathqstroke\n}\n",
    "% crosshatch 30 degree\n\\pgfdeclarepatternformonly{xfigp2}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{9bp}{5bp}}{\\pgfqpoint{8bp}{4bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{-1bp}{-0.5bp}\\pgfpathqlineto{9bp}{4.5bp}\n\t\\pgfpathqmoveto{-1bp}{4.5bp}\\pgfpathqlineto{9bp}{-0.5bp}\n\t\\pgfusepathqstroke\n}\n",
    "% lines 45 degree left\n\\pgfdeclarepatternformonly{xfigp3}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{9bp}{9bp}}{\\pgfqpoint{8bp}{8bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{-1bp}{9bp}\\pgfpathqlineto{9bp}{-1bp}\n\t\\pgfusepathqstroke\n}\n",
    "% lines 45 degree right\n\\pgfdeclarepatternformonly{xfigp4}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{9bp}{9bp}}{\\pgfqpoint{8bp}{8bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{-1bp}{-1bp}\\pgfpathqlineto{9bp}{9bp}\n\t\\pgfusepathqstroke\n}\n",
    "% crosshatch 45 degree\n\\pgfdeclarepatternformonly{xfigp5}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{9bp}{9bp}}{\\pgfqpoint{8bp}{8bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{-1bp}{-1bp}\\pgfpathqlineto{9bp}{9bp}\n\t\\pgfpathqmoveto{-1bp}{9bp}\\pgfpathqlineto{9bp}{-1bp}\n\t\\pgfusepathqstroke\n}\n",
    "% horizontal bricks\n\\pgfdeclarepatternformonly{xfigp6}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{17bp}{17bp}}{\\pgfqpoint{16bp}{16bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetbuttcap\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{-1bp}{4bp}\\pgfpathqlineto{17bp}{4bp}\n\t\\pgfpathqmoveto{-1bp}{12bp}\\pgfpathqlineto{17bp}{12bp}\n\t\\pgfpathqmoveto{4bp}{-1bp}\\pgfpathqlineto{4bp}{4bp}\n\t\\pgfpathqmoveto{4bp}{12bp}\\pgfpathqlineto{4bp}{17bp}\n\t\\pgfpathqmoveto{12bp}{4bp}\\pgfpathqlineto{12bp}{12bp}\n\t\\pgfusepathqstroke\n}\n",
    "% vertical bricks\n\\pgfdeclarepatternformonly{xfigp7}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{17bp}{17bp}}{\\pgfqpoint{16bp}{16bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetbuttcap\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{4bp}{-1bp}\\pgfpathqlineto{4bp}{17bp}\n\t\\pgfpathqmoveto{12bp}{-1bp}\\pgfpathqlineto{12bp}{17bp}\n\t\\pgfpathqmoveto{-1bp}{4bp}\\pgfpathqlineto{4bp}{4bp}\n\t\\pgfpathqmoveto{12bp}{4bp}\\pgfpathqlineto{17bp}{4bp}\n\t\\pgfpathqmoveto{4bp}{12bp}\\pgfpathqlineto{12bp}{12bp}\n\t\\pgfusepathqstroke\n}\n",
    "% horizontal lines\n\\pgfdeclarepatternformonly{xfigp8}\n{\\pgfpointorigin}{\\pgfqpoint{101bp}{1bp}}{\\pgfqpoint{100bp}{4bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{0bp}{0.5bp}\\pgfpathqlineto{101bp}{0.5bp}\n\t\\pgfusepathqstroke\n}\n",
    "% vertical lines\n\\pgfdeclarepatternformonly{xfigp9}\n{\\pgfpointorigin}{\\pgfqpoint{1bp}{101bp}}{\\pgfqpoint{4bp}{100bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{0.5bp}{0bp}\\pgfpathqlineto{0.5bp}{101bp}\n\t\\pgfusepathqstroke\n}\n",
    "% crosshatch\n\\pgfdeclarepatternformonly{xfigp10}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{5bp}{5bp}}{\\pgfqpoint{4bp}{4bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{-1bp}{2bp}\\pgfpathqlineto{5bp}{2bp}\n\t\\pgfpathqmoveto{2bp}{-1bp}\\pgfpathqlineto{2bp}{5bp}\n\t\\pgfusepathqstroke\n}\n",
    "% left shingles\n\\pgfdeclarepatternformonly{xfigp11}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{25bp}{25bp}}{\\pgfqpoint{24bp}{24bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetbuttcap\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{-1bp}{0bp}\\pgfpathqlineto{25bp}{0bp}\n\t\\pgfpathqmoveto{-1bp}{8bp}\\pgfpathqlineto{25bp}{8bp}\n\t\\pgfpathqmoveto{-1bp}{16bp}\\pgfpathqlineto{25bp}{16bp}\n\t\\pgfpathqmoveto{10bp}{0bp}\\pgfpathqlineto{14bp}{8bp}\n\t\\pgfpathqmoveto{2bp}{8bp}\\pgfpathqlineto{6bp}{16bp}\n\t\\pgfpathqmoveto{18bp}{16bp}\\pgfpathqlineto{22bp}{24bp}\n\t\\pgfusepathqstroke\n}\n",
    "% right shingles\n\\pgfdeclarepatternformonly{xfigp12}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{25bp}{25bp}}{\\pgfqpoint{24bp}{24bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetbuttcap\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{-1bp}{0bp}\\pgfpathqlineto{25bp}{0bp}\n\t\\pgfpathqmoveto{-1bp}{8bp}\\pgfpathqlineto{25bp}{8bp}\n\t\\pgfpathqmoveto{-1bp}{16bp}\\pgfpathqlineto{25bp}{16bp}\n\t\\pgfpathqmoveto{14bp}{0bp}\\pgfpathqlineto{10bp}{8bp}\n\t\\pgfpathqmoveto{22bp}{8bp}\\pgfpathqlineto{18bp}{16bp}\n\t\\pgfpathqmoveto{6bp}{16bp}\\pgfpathqlineto{2bp}{24bp}\n\t\\pgfusepathqstroke\n}\n",
    "% vertical left shingles\n\\pgfdeclarepatternformonly{xfigp13}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{25bp}{25bp}}{\\pgfqpoint{24bp}{24bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetbuttcap\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{0bp}{-1bp}\\pgfpathqlineto{0bp}{25bp}\n\t\\pgfpathqmoveto{8bp}{-1bp}\\pgfpathqlineto{8bp}{25bp}\n\t\\pgfpathqmoveto{16bp}{-1bp}\\pgfpathqlineto{16bp}{25bp}\n\t\\pgfpathqmoveto{0bp}{14bp}\\pgfpathqlineto{8bp}{10bp}\n\t\\pgfpathqmoveto{8bp}{22bp}\\pgfpathqlineto{16bp}{18bp}\n\t\\pgfpathqmoveto{16bp}{6bp}\\pgfpathqlineto{24bp}{2bp}\n\t\\pgfusepathqstroke\n}\n",
    "% vertical right shingles\n\\pgfdeclarepatternformonly{xfigp14}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{25bp}{25bp}}{\\pgfqpoint{24bp}{24bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetbuttcap\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{0bp}{-1bp}\\pgfpathqlineto{0bp}{25bp}\n\t\\pgfpathqmoveto{8bp}{-1bp}\\pgfpathqlineto{8bp}{25bp}\n\t\\pgfpathqmoveto{16bp}{-1bp}\\pgfpathqlineto{16bp}{25bp}\n\t\\pgfpathqmoveto{0bp}{10bp}\\pgfpathqlineto{8bp}{14bp}\n\t\\pgfpathqmoveto{8bp}{2bp}\\pgfpathqlineto{16bp}{6bp}\n\t\\pgfpathqmoveto{16bp}{18bp}\\pgfpathqlineto{24bp}{22bp}\n\t\\pgfusepathqstroke\n}\n",
    "% fishscales\n\\pgfdeclarepatternformonly{xfigp15}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{17bp}{9bp}}{\\pgfqpoint{16bp}{8bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetbuttcap\\pgfsetlinewidth{0.45bp}\n\t\\draw(8bp,0bp) arc [radius=10bp,start angle=270,end angle=216.87]\n\t\t(8bp,0bp) arc [radius=10bp,start angle=270,end angle=323.13]\n\t\t(0bp,4bp) arc [radius=10bp,start angle=270,end angle=323.13]\n\t\t(16bp,4bp) arc [radius=10bp,start angle=270,end angle=216.87];\n}\n",
    "% small fishscales\n\\pgfdeclarepatternformonly{xfigp16}\n{\\pgfqpoint{-1bp}{-1bp}}{\\pgfqpoint{9bp}{9bp}}{\\pgfqpoint{8bp}{8bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetbuttcap\\pgfsetlinewidth{0.45bp}\n\t\\draw(0bp,4bp) arc [radius=4bp,start angle=180,end angle=360]\n\t\t(4bp,8bp) arc [radius=4bp,start angle=360,end angle=270]\n\t\t(4bp,8bp) arc [radius=4bp,start angle=180,end angle=270];\n}\n",
    "% circles\n\\pgfdeclarepatternformonly{xfigp17}\n{\\pgfqpoint{-9bp}{-9bp}}{\\pgfqpoint{8bp}{8bp}}{\\pgfqpoint{16bp}{16bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqcircle{8bp}\\pgfusepathqstroke\n}\n",
    "% hexagons\n\\pgfdeclarepatternformonly{xfigp18}\n{\\pgfqpoint{-5bp}{-1bp}}{\\pgfqpoint{23bp}{16bp}}{\\pgfqpoint{27bp}{15.6bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetbuttcap\\pgfsetmiterjoin\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathmoveto{0bp}{15.6bp}\\pgfpathqlineto{-4.5bp}{7.8bp}\n\t\\pgfpathqlineto{0bp}{0bp}\\pgfpathqlineto{9bp}{0bp}\n\t\\pgfpathqlineto{13.5bp}{7.8bp}\\pgfpathqlineto{9bp}{15.6bp}\n\t\\pgfpathqmoveto{13.5bp}{7.8bp}\\pgfpathqlineto{22.5bp}{7.8bp}\n\t\\pgfusepathqstroke\n}\n",
    "% octagons\n\\pgfdeclarepatternformonly{xfigp19}\n{\\pgfqpoint{-8bp}{-8bp}}{\\pgfqpoint{8bp}{8bp}}{\\pgfqpoint{15.84bp}{15.84bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetbuttcap\\pgfsetmiterjoin\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{-8bp}{0bp}\\pgfpathqlineto{-4.64bp}{0bp}\n\t\\pgfpathqlineto{0bp}{-4.64bp}\\pgfpathqlineto{4.64bp}{0bp}\n\t\\pgfpathqlineto{0bp}{4.64bp}\\pgfpathqlineto{-4.64bp}{0bp}\n\t\\pgfpathqmoveto{0bp}{-8bp}\\pgfpathqlineto{0bp}{-4.64bp}\n\t\\pgfpathqmoveto{0bp}{8bp}\\pgfpathqlineto{0bp}{4.64bp}\n\t\\pgfpathqmoveto{8bp}{0bp}\\pgfpathqlineto{4.64bp}{0bp}\n\t\\pgfusepathqstroke\n}\n",
    "% horizontal sawtooth lines / tire threads\n\\pgfdeclarepatternformonly{xfigp20}\n{\\pgfpointorigin}{\\pgfqpoint{9bp}{6bp}}{\\pgfqpoint{8bp}{8bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetmiterjoin\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{0bp}{3bp}\\pgfpathqlineto{2bp}{5bp}\n\t\\pgfpathqlineto{6bp}{1bp}\\pgfpathqlineto{9bp}{4bp}\n\t\\pgfusepathqstroke\n}\n",
    "% vertical sawtooth lines / tire threads\n\\pgfdeclarepatternformonly{xfigp21}\n{\\pgfpointorigin}{\\pgfqpoint{6bp}{9bp}}{\\pgfqpoint{8bp}{8bp}}\n{\t\\pgfsetdash{}{0pt}\\pgfsetmiterjoin\\pgfsetlinewidth{0.45bp}\n\t\\pgfpathqmoveto{3bp}{0bp}\\pgfpathqlineto{5bp}{2bp}\n\t\\pgfpathqlineto{1bp}{6bp}\\pgfpathqlineto{4bp}{9bp}\n\t\\pgfusepathqstroke\n}\n",
];
