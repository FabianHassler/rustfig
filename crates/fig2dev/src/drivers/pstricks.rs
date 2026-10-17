//! fig2dev's `pstricks` driver: the figure as PSTricks macros (`\psline`,
//! `\psframe`, `\pscircle`, `\psarc`, `\rput` for texts, ...), with
//! coordinates in centimetres relative to the figure's bounding box.
//! Arrowheads are drawn like Fig does (`-R 0`) or with PSTricks' own arrow
//! ends; shades and tints are declared as `\newrgbcolor`s on first use.
//! With `-p`, pictures that are not EPS are converted with `bmeps`.
//
// Ported from fig2dev dev/genpstricks.c (xfig licence, see LICENSE-xfig).

use std::f64::consts::PI;

use crate::drivers::cgm::{text_bytes, text_encoding, TextEnc};
use crate::drivers::latex::{c_string, default_font_option, set_default_size, Cf};
use crate::gendev::{print_comments, strtod_prefix, strtol, Ctx, NUM_STD_COLS};
use crate::object::{
    Arc, Arrow, Compound, Ellipse, Line, Point, Spline, Text, DASH_2_DOTS_LINE, DASH_3_DOTS_LINE, DASH_DOT_LINE,
    DASH_LINE, DEFAULT, DOTTED_LINE, SOLID_LINE, T_ARC_BOX, T_BOX, T_CENTER_JUSTIFIED, T_LEFT_JUSTIFIED, T_OPEN_ARC,
    T_PIC_BOX, T_PIE_WEDGE_ARC, T_POLYGON, T_POLYLINE, T_RIGHT_JUSTIFIED,
};
use crate::util::psfonts::{ps_font_name, ps_index, PS_FONT_MAP, PS_MAP_WARN};
use crate::util::texfonts::TexFonts;
use crate::{fprintf, Driver, Out};

// Warnings, printed once at the end of the run (`warn_flags`).
const W_DASH_DOT: u32 = 0;
const W_DASH_DOTS: u32 = 1;
const W_HOLLOW_ARROW: u32 = 2;
const W_HATCH_PATTERN: u32 = 3;
const W_OLD_SPLNE: u32 = 4;
const W_PIC: u32 = 5;
const W_GRID: u32 = 6;
const W_ELLIPSE_HATCH: u32 = 7;
const W_FN_CHARS: u32 = 8;
const W_SLASH_CONVERT: u32 = 9;
const W_UNK_PIC_TYPE: u32 = 10;
const W_SYSTEM_EXEC: u32 = 11;
const W_BMEPS_ERR: u32 = 12;
const W_EPS_NEEDED: u32 = 13;
const W_PIC_CONVERT: u32 = 14;
const W_PS_FONT: u32 = 15;
const W_PAGESCALE: u32 = 16;
const W_UNIT: u32 = 17;
const W_LINEJOIN: u32 = 18;
const W_UNKNOWN_ARROW: u32 = 19;
const W_NEW_ARROW: u32 = 20;
const W_TERMINATOR_ARROW: u32 = 21;

const W_RTN_ERROR_FLAGS: u32 = (1 << W_UNK_PIC_TYPE) | (1 << W_SYSTEM_EXEC) | (1 << W_BMEPS_ERR);

const WARNINGS: [&str; 22] = [
    "Dash-dot lines require pstricks patchlevel 15 of June 2004 or later.",
    "Dash-dot-dot lines require \\usepackage{pstricks-add}.",
    "Hollow arrows required \\usepackage{pstricks-add}.",
    "A hatch pattern not supported by pstricks has been converted to crosshatch.",
    "Driver never tested with old splines. Send examples to ressler@usma.edu.",
    "Picture requires \\usepackage{graphicx}",
    "Grid sizes are ignored by the pstricks driver. Default \\psgrid issued.",
    "Hatch pattern in rotated ellipse may be imprecisely converted.",
    "Spaces and special chars in picture file path are likely to cause problems.",
    "Backslashes were converted to slashes in picture file path.",
    "Couldn't convert unknown picture type to eps. Known: eps, png, jpg, pnm, tif (exit code 17).",
    "Could not execute bmeps to convert picture to eps (exit code 17).",
    "BMEPS returned non-0 exit code; picture conversion to eps probably failed (exit code 17).",
    "Non-EPS picture paths were converted to the eps image directory.",
    "Pictures were converted and placed in the eps image directory.",
    "Substituted LaTeX font for PS.  Use -v to see details.",
    "Image scaled to page.",
    "Setting unit requires \\psset{unit=X}\\newlength\\unit\\setlength\\unit{X}.",
    "Linejoins depend on PSTricks version. Use -t option to ensure compatibility.",
    "Unknown arrow type. Simple (stick) arrow substituted.",
    "New arrow type requires xfig 3.2.5a or newer.",
    "One or more arrows have been approximated with a PSTricks style. Try -R 0.",
];

// Page modes
const PM_PICTURE: i32 = 0;
const PM_BARE: i32 = 1;
const PM_PAGE: i32 = 2;
const PM_SCALED_PAGE: i32 = 3;

// Arrow styles
const A_XFIG: i32 = 0;
const A_PSTRICKS: i32 = 1;

// Font handling
const FH_FULL: i32 = 0;
const FH_SIZE_ONLY: i32 = 1;

/// `-S`: magnification and font size.
const SCALE_TBL: [(f64, i32); 5] = [(0.6667, 8), (0.75, 9), (0.8333, 10), (0.9167, 11), (1.0, 12)];

/// Extra packages the figure needs: `graphicx`, `pstricks-add`.
const EP_GRAPHICX: u32 = 0;
const EP_PSTRICKS_ADD: u32 = 1;
const EXTRA_PACKAGE_NAMES: [&str; 2] = ["graphicx", "pstricks-add"];

/// The version of PSTricks assumed (`ASSUMED_PST_VERSION`) and the first
/// one with the linejoin option.
const ASSUMED_PST_VERSION: &str = "1.20";
const PST_LINEJOIN_VERSION: &str = "1.20";
/// Half the size of a version key.
const KEY_HALF: usize = 10;

// Arrow types
const STICK_ARROW: i32 = 0;
const TRIANGLE_ARROW: i32 = 1;
const INDENTED_BUTT_ARROW: i32 = 2;
const POINTED_BUTT_ARROW: i32 = 3;
const LAST_OLD_ARROW: i32 = POINTED_BUTT_ARROW;
const DIAMOND_ARROW: i32 = 4;
const CIRCLE_ARROW: i32 = 5;
const SEMI_CIRCLE_ARROW: i32 = 6;
const RECTANGLE_ARROW: i32 = 7;
const REVERSE_TRIANGLE_ARROW: i32 = 8;
const BISHADE_INDENTED_BUTT_ARROW: i32 = 9;
const TRIANGLE_HALF_ARROW: i32 = 10;
const INDENTED_BUTT_HALF_ARROW: i32 = 11;
const POINTED_BUTT_HALF_ARROW: i32 = 12;
const PARTIAL_REVERSE_TRIANGLE_ARROW: i32 = 13;
const PARTIAL_RECTANGLE_ARROW: i32 = 14;

const HOLLOW_ARROW: i32 = 0;

// format_options flags
const FO_LINE_STYLE: u32 = 1;
const FO_PEN_COLOR: u32 = 2;
const FO_LINE_TERM: u32 = 4;
const FO_ARROWS: u32 = 8;
const FO_ARROW_SIZE: u32 = 16;
const FO_FILL: u32 = 32;
const FO_JOIN_STYLE: u32 = 64;

const CT_BLACK: i32 = 0;
const CT_WHITE: i32 = 7;

/// Size of C's option buffers: longer strings make fig2dev exit.
const BUF: usize = 256;
/// Characters of a text fig2dev formats (`64*1024`).
const TEXT_BUF: usize = 64 * 1024;
/// `sizeof "$\\backslash$"`.
const LONGEST_TRANSLATION: usize = 13;
/// `Pic_convert_dir[245]`, less the room `-p` leaves.
const MAX_CONVERT_DIR: usize = 243;

/// The standard colours: PSTricks name, declared already, rgb.
const STD_COLORS: [(&str, bool, f64, f64, f64); NUM_STD_COLS] = [
    ("black", true, 0.00, 0.00, 0.00),
    ("blue", true, 0.00, 0.00, 1.00),
    ("green", true, 0.00, 1.00, 0.00),
    ("cyan", true, 0.00, 1.00, 1.00),
    ("red", true, 1.00, 0.00, 0.00),
    ("magenta", true, 1.00, 0.00, 1.00),
    ("yellow", true, 1.00, 1.00, 0.00),
    ("white", true, 1.00, 1.00, 1.00),
    ("bluei", false, 0.00, 0.00, 0.56),
    ("blueii", false, 0.00, 0.00, 0.69),
    ("blueiii", false, 0.00, 0.00, 0.82),
    ("blueiv", false, 0.53, 0.81, 1.00),
    ("greeni", false, 0.00, 0.56, 0.00),
    ("greenii", false, 0.00, 0.69, 0.00),
    ("greeniii", false, 0.00, 0.82, 0.00),
    ("cyani", false, 0.00, 0.56, 0.56),
    ("cyanii", false, 0.00, 0.69, 0.69),
    ("cyaniii", false, 0.00, 0.82, 0.82),
    ("redi", false, 0.56, 0.00, 0.00),
    ("redii", false, 0.69, 0.00, 0.00),
    ("rediii", false, 0.82, 0.00, 0.00),
    ("magentai", false, 0.56, 0.00, 0.56),
    ("magentaii", false, 0.69, 0.00, 0.69),
    ("magentaiii", false, 0.82, 0.00, 0.82),
    ("browni", false, 0.50, 0.19, 0.00),
    ("brownii", false, 0.63, 0.25, 0.00),
    ("browniii", false, 0.75, 0.38, 0.00),
    ("pinki", false, 1.00, 0.50, 0.50),
    ("pinkii", false, 1.00, 0.63, 0.63),
    ("pinkiii", false, 1.00, 0.75, 0.75),
    ("pinkiv", false, 1.00, 0.88, 0.88),
    ("gold", false, 1.00, 0.84, 0.00),
];

/// An entry of `color_table`: shades and tints are bit sets of the
/// declarations already emitted.
#[derive(Clone, Debug)]
struct ColorEntry {
    name: String,
    defined: bool,
    r: f64,
    g: f64,
    b: f64,
    shades: u32,
    tints: u32,
}

/// What `format_options` needs to know about an object.
#[derive(Clone, Copy, Default)]
struct FoArgs<'a> {
    flags: u32,
    thickness: i32,
    style: i32,
    style_val: f64,
    pen_color: i32,
    join_style: i32,
    cap_style: i32,
    fill_style: i32,
    fill_color: i32,
    back_arrow: Option<&'a Arrow>,
    fore_arrow: Option<&'a Arrow>,
    hatch_angle_offset: f64,
}

/// The `pstricks` driver's state (genpstricks.c's statics).
#[derive(Debug)]
pub struct Pstricks {
    fonts: TexFonts,
    verbose: bool,
    page_mode: i32,
    arrows: i32,
    font_handling: i32,
    pic_convert_p: bool,
    pic_convert_dir: String,
    x_margin: f64,
    y_margin: f64,
    line_weight: f64,
    /// `Pst_version`: its string, key and the number of updates.
    pst_str: String,
    pst_key: [u8; 2 * KEY_HALF],
    pst_updates: u32,
    /// `Linejoin == LJ_PSTVERB`.
    linejoin_pstverb: bool,
    extra_package_mask: u32,
    has_text: bool,
    warn_flags: u32,
    rtn_val: i32,
    colors: Vec<ColorEntry>,
    grays: [Option<String>; 20],
    /// Source path and the eps path made for it.
    converted: Vec<(String, String)>,
    /// Base names used for converted pictures and their counters.
    base_names: Vec<(String, i32)>,
    ppi: f64,
    enc: TextEnc,
    /// `min_int(llx, urx)` and `max_int(lly, ury)`.
    x_origin: i32,
    y_origin: i32,
}

impl Default for Pstricks {
    fn default() -> Pstricks {
        let (pst_str, pst_key) = parse_pst_version(ASSUMED_PST_VERSION).unwrap_or_default();
        Pstricks {
            fonts: TexFonts::new(),
            verbose: false,
            page_mode: PM_PICTURE,
            arrows: A_XFIG,
            font_handling: FH_FULL,
            pic_convert_p: false,
            pic_convert_dir: String::new(),
            x_margin: 0.0,
            y_margin: 0.0,
            line_weight: 0.5,
            pst_str,
            pst_key,
            pst_updates: 0,
            linejoin_pstverb: false,
            extra_package_mask: 0,
            has_text: false,
            warn_flags: 0,
            rtn_val: 0,
            colors: Vec::new(),
            grays: Default::default(),
            converted: Vec::new(),
            base_names: Vec::new(),
            ppi: 1200.0,
            enc: TextEnc::Utf8,
            x_origin: 0,
            y_origin: 0,
        }
    }
}

/// The driver for `lang`.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    (lang == "pstricks").then(|| Box::new(Pstricks::default()) as Box<dyn Driver>)
}

/// A roman numeral for `n` (`roman_numeral_from_int`).
fn roman_numeral(mut n: u32) -> String {
    const DIGITS: [&str; 25] = [
        "M", "CM", "D", "CD", "C", "XC", "L", "XL", "X", "IX", "V", "IV", "m", "cm", "d", "cd", "c", "xc", "l", "xl",
        "x", "ix", "v", "iv", "i",
    ];
    const DIVS: [u32; 25] = [
        1000000, 900000, 500000, 400000, 100000, 90000, 50000, 40000, 10000, 9000, 5000, 4000, 1000, 900, 500, 400,
        100, 90, 50, 40, 10, 9, 5, 4, 1,
    ];
    let mut s = String::new();
    for (d, &div) in DIGITS.iter().zip(DIVS.iter()) {
        while n >= div {
            n -= div;
            s.push_str(d);
        }
    }
    s
}

/// `cm_from_80ths`.
fn cm_80(v: f64) -> f64 {
    v * (2.54 / 80.0)
}

/// `DBL`: a coordinate with four digits.
fn f4(x: f64) -> String {
    format!("{:.4}", Cf(x))
}

fn max_dbl(a: f64, b: f64) -> f64 {
    if a > b {
        a
    } else {
        b
    }
}

/// `SNPRINTF_EXIT`: a string that does not fit C's buffer ends the run.
fn chk(s: String) -> Result<String, String> {
    if s.len() >= BUF {
        let cut: String = s.chars().take(BUF - 1).collect();
        return Err(format!("Unable to print:\n{cut}"));
    }
    Ok(s)
}

/// `append_to_comma_list`, for a buffer of `BUF` bytes.
fn append_to_comma_list(list: &mut String, val: &str) -> Result<(), String> {
    if val.is_empty() {
        return Ok(());
    }
    if !list.is_empty() {
        if val.len() + list.len() + 2 >= BUF {
            return Err(format!("Unable to print:\n{list},{val}"));
        }
        list.push(',');
        list.push_str(val);
    } else {
        if val.len() + 1 >= BUF {
            return Err(format!("Unable to print:\n{val}"));
        }
        list.push_str(val);
    }
    Ok(())
}

/// Fills a version string and key from `s` (`parse_pst_version`): digits,
/// a dot, digits, and optionally one letter. The error is what C writes to
/// stderr.
fn parse_pst_version(s: &str) -> Result<(String, [u8; 2 * KEY_HALF]), String> {
    let b = s.as_bytes();
    let at = |i: usize| b.get(i).copied().unwrap_or(0);
    let mut key = [b'0'; 2 * KEY_HALF];
    let mut v: Vec<u8> = Vec::new();
    if b.len() > KEY_HALF - 1 {
        return Err("PSTricks version string too long".to_string());
    }
    if !at(0).is_ascii_digit() {
        return Err(format!("bad character '{}' in PSTricks version", char::from(at(0))));
    }
    v.push(at(0));
    let mut i = 1;
    while at(i).is_ascii_digit() {
        v.push(at(i));
        i += 1;
    }
    if at(i) != b'.' {
        return Err("expected dot in PSTricks version".to_string());
    }
    // save the major version in the key
    if let Some(dst) = key.get_mut(KEY_HALF - i..KEY_HALF) {
        dst.copy_from_slice(&v);
    }
    v.push(b'.');
    i += 1;
    let i_minor = v.len();
    if !at(i).is_ascii_digit() {
        return Err("expected digit after dot in PSTricks version".to_string());
    }
    v.push(at(i));
    i += 1;
    while at(i).is_ascii_digit() {
        v.push(at(i));
        i += 1;
    }
    if at(i).is_ascii_alphabetic() {
        v.push(at(i).to_ascii_lowercase());
        i += 1;
        if at(i) != 0 {
            return Err(format!(
                "expected end of PSTricks version, found '{}'",
                char::from(at(i))
            ));
        }
    } else if at(i) != 0 {
        return Err("expected digit or subversion letter in PSTricks version".to_string());
    }
    let minor = v.get(i_minor..).unwrap_or(&[]);
    if let Some(dst) = key.get_mut(KEY_HALF..KEY_HALF + minor.len()) {
        dst.copy_from_slice(minor);
    }
    Ok((String::from_utf8_lossy(&v).into_owned(), key))
}

/// `%d` of sscanf: the number, if the string starts with one.
fn scan_int(s: &str) -> Option<i32> {
    let (v, used) = strtol(s);
    (used > 0).then_some(v)
}

/// `%lf` of sscanf.
fn scan_double(s: &str) -> Option<f64> {
    strtod_prefix(s).map(|(v, _)| v)
}

/// Whether the arrow is hollow (`pp_check_arrow_style`).
fn hollow(a: &Option<Arrow>) -> bool {
    a.as_ref().is_some_and(|a| a.style == 0)
}

/// `pp_find_pstricks_extras`: the packages the figure needs.
fn find_extras(c: &Compound, mask: &mut u32) {
    let line_style = |style: i32, mask: &mut u32| {
        if style == DASH_2_DOTS_LINE || style == DASH_3_DOTS_LINE {
            *mask |= 1 << EP_PSTRICKS_ADD;
        }
    };
    for l in &c.lines {
        line_style(l.style, mask);
        if hollow(&l.for_arrow) || hollow(&l.back_arrow) {
            *mask |= 1 << EP_PSTRICKS_ADD;
        }
        if l.type_ == T_PIC_BOX {
            *mask |= 1 << EP_GRAPHICX;
        }
    }
    for e in &c.ellipses {
        line_style(e.style, mask);
    }
    for s in &c.splines {
        line_style(s.style, mask);
        if hollow(&s.for_arrow) || hollow(&s.back_arrow) {
            *mask |= 1 << EP_PSTRICKS_ADD;
        }
    }
    for a in &c.arcs {
        line_style(a.style, mask);
        if hollow(&a.for_arrow) || hollow(&a.back_arrow) {
            *mask |= 1 << EP_PSTRICKS_ADD;
        }
    }
    for sub in &c.compounds {
        find_extras(sub, mask);
    }
}

/// `pp_find_text`.
fn find_text(c: &Compound) -> bool {
    !c.texts.is_empty() || c.compounds.iter().any(find_text)
}

/// The bounding box of points: (llx, lly, urx, ury).
fn bounding_box(pts: &[Point]) -> (i32, i32, i32, i32) {
    let Some(first) = pts.first() else {
        return (0, 0, 0, 0);
    };
    let (mut llx, mut lly, mut urx, mut ury) = (first.x, first.y, first.x, first.y);
    for p in pts {
        llx = llx.min(p.x);
        lly = lly.min(p.y);
        urx = urx.max(p.x);
        ury = ury.max(p.y);
    }
    (llx, lly, urx, ury)
}

/// `(x,y)` with four digits each.
fn pt(x: f64, y: f64) -> String {
    format!("({},{})", f4(x), f4(y))
}

/// `translate_tex_specials`: what a character becomes.
fn translate_tex_special(ch: u8) -> Option<&'static str> {
    Some(match ch {
        b'\\' => "$\\backslash$",
        b'{' => "$\\{$",
        b'}' => "$\\}$",
        b'>' => "$>$",
        b'<' => "$<$",
        b'^' => "\\^{}",
        b'~' => "\\~{}",
        b'$' => "\\$",
        b'&' => "\\&",
        b'#' => "\\#",
        b'_' => "\\_",
        b'%' => "\\%",
        b'\n' => "\\\\\n",
        _ => return None,
    })
}

fn ref_pt_str(type_: i32) -> Result<&'static str, String> {
    match type_ {
        T_LEFT_JUSTIFIED | DEFAULT => Ok("lb"),
        T_CENTER_JUSTIFIED => Ok("b"),
        T_RIGHT_JUSTIFIED => Ok("rb"),
        _ => Err(format!("unknown text position ({type_})")),
    }
}

fn just_str(type_: i32) -> Result<&'static str, String> {
    match type_ {
        T_LEFT_JUSTIFIED | DEFAULT => Ok("l"),
        T_CENTER_JUSTIFIED => Ok("c"),
        T_RIGHT_JUSTIFIED => Ok("r"),
        _ => Err(format!("unknown text position ({type_})")),
    }
}

/// `fmt_text_str`: the text with TeX's special characters quoted; texts
/// with several lines become a `\shortstack`.
fn fmt_text_str(ctx: &mut Ctx, s: &[u8], type_: i32) -> Result<Vec<u8>, String> {
    let multiline = s.contains(&b'\n');
    let mut buf: Vec<u8> = Vec::new();
    if multiline {
        buf.extend_from_slice(format!("\\shortstack[{}]{{", just_str(type_)?).as_bytes());
    }
    for (i, &ch) in s.iter().enumerate() {
        if buf.len() >= TEXT_BUF - LONGEST_TRANSLATION {
            let rest = String::from_utf8_lossy(s.get(i..).unwrap_or(&[])).into_owned();
            ctx.warn(format!("string too long; truncated ...{rest}"));
            break;
        }
        match translate_tex_special(ch) {
            Some(t) => buf.extend_from_slice(t.as_bytes()),
            None => buf.push(ch),
        }
    }
    if multiline {
        buf.push(b'}');
    }
    Ok(buf)
}

/// `check_tex_format`: a brace and dollar balance check of a special text.
fn check_tex_format(ctx: &mut Ctx, s: &[u8]) {
    let (mut braces, mut dollars) = (0i32, 0i32);
    let mut i = 0;
    while let Some(&ch) = s.get(i) {
        i += 1;
        match ch {
            b'{' => braces += 1,
            b'}' => braces -= 1,
            b'\\' => {
                if i < s.len() {
                    i += 1;
                }
            }
            b'$' => dollars += 1,
            _ => {}
        }
    }
    let text = String::from_utf8_lossy(s);
    if braces != 0 {
        ctx.warn(format!("warning: unbalanced braces in special text '{text}'"));
    }
    if dollars & 1 != 0 {
        ctx.warn(format!("warning: unbalanced $'s in special text '{text}'"));
    }
}

impl Pstricks {
    fn warn(&mut self, flag: u32) {
        self.warn_flags |= 1 << flag;
        if (1 << flag) & W_RTN_ERROR_FLAGS != 0 {
            self.rtn_val = 17;
        }
    }

    /// `cm_from_1200ths` of an integer.
    fn cm(&self, v: i32) -> f64 {
        f64::from(v) / self.ppi * 2.54
    }

    /// `cm_from_1200ths` of a double.
    fn cm_d(&self, v: f64) -> f64 {
        v / self.ppi * 2.54
    }

    /// `convert_pt_to_image_bb`: relative to the bounding box, y up.
    fn to_bb(&self, p: Point) -> Point {
        Point {
            x: p.x.wrapping_sub(self.x_origin),
            y: self.y_origin.wrapping_sub(p.y),
        }
    }

    // ----- colours -----

    /// `setup_color_table`.
    fn setup_colors(&mut self, ctx: &Ctx) {
        if !self.colors.is_empty() {
            return;
        }
        for &(name, defined, r, g, b) in &STD_COLORS {
            self.colors.push(ColorEntry {
                name: name.to_string(),
                defined,
                r,
                g,
                b,
                shades: 0,
                tints: 0,
            });
        }
        for (i, u) in ctx.user_colors.iter().enumerate() {
            self.colors.push(ColorEntry {
                name: format!("usrclr{}", roman_numeral(i as u32)),
                defined: false,
                r: f64::from(u.r) / 256.0,
                g: f64::from(u.g) / 256.0,
                b: f64::from(u.b) / 256.0,
                shades: 0,
                tints: 0,
            });
        }
    }

    /// `check_color_index`.
    fn check_color_index(&self, ic: i32) -> Result<usize, String> {
        match usize::try_from(ic) {
            Ok(i) if i < self.colors.len() => Ok(i),
            _ => Err(format!(
                "bad color index ({}; max={})",
                ic,
                self.colors.len() as i64 - 1
            )),
        }
    }

    /// `interpolate_colors`.
    fn interpolate(&self, ic0: i32, ic1: i32, t: f64) -> Result<(f64, f64, f64), String> {
        if !(0.0..=1.0).contains(&t) {
            return Err(format!("bad color interpolation parameter ({})", f4(t)));
        }
        let a = self.colors.get(self.check_color_index(ic0)?);
        let b = self.colors.get(self.check_color_index(ic1)?);
        let (Some(a), Some(b)) = (a, b) else {
            return Err("bad color index".to_string());
        };
        Ok((
            t.mul_add(b.r - a.r, a.r),
            t.mul_add(b.g - a.g, a.g),
            t.mul_add(b.b - a.b, a.b),
        ))
    }

    /// `color_name_after_declare_color`.
    fn color_name(&mut self, out: &mut Out, ctx: &Ctx, ic: i32) -> Result<String, String> {
        self.setup_colors(ctx);
        let i = self.check_color_index(ic)?;
        let verbose = self.verbose;
        let Some(c) = self.colors.get_mut(i) else {
            return Err("bad color index".to_string());
        };
        if !c.defined {
            if verbose {
                fprintf!(out, "% declare color {}\n", ic);
            }
            fprintf!(
                out,
                "\\newrgbcolor{{{}}}{{{} {} {}}}%\n",
                c.name,
                f4(c.r),
                f4(c.g),
                f4(c.b)
            );
            c.defined = true;
        }
        Ok(c.name.clone())
    }

    /// `shade_or_tint_name_after_declare_color`.
    fn shade_or_tint_name(&mut self, out: &mut Out, ctx: &Ctx, ist: i32, ic: i32) -> Result<String, String> {
        if !(0..=40).contains(&ist) {
            return Err(format!("bad shade/tint index ({ist})"));
        }
        match ist {
            0 => return self.color_name(out, ctx, CT_BLACK),
            20 => return self.color_name(out, ctx, ic),
            40 => return self.color_name(out, ctx, CT_WHITE),
            _ => {}
        }
        self.setup_colors(ctx);
        let i = self.check_color_index(ic)?;
        let base = self.colors.get(i).map(|c| c.name.clone()).unwrap_or_default();
        let name = format!("{}st{}", base, roman_numeral(ist as u32));
        let (shades, tints) = self.colors.get(i).map_or((0, 0), |c| (c.shades, c.tints));
        if ist < 20 && shades & (1 << ist) == 0 {
            if self.verbose {
                fprintf!(out, "% declare shade {} of {}\n", ist, base);
            }
            let (r, g, b) = self.interpolate(CT_BLACK, ic, f64::from(ist) / 20.0)?;
            fprintf!(out, "\\newrgbcolor{{{}}}{{{} {} {}}}%\n", name, f4(r), f4(g), f4(b));
            if let Some(c) = self.colors.get_mut(i) {
                c.shades |= 1 << ist;
            }
        } else if ist > 20 && tints & (1 << (ist - 20)) == 0 {
            if self.verbose {
                fprintf!(out, "% declare tint {} of {}\n", ist, base);
            }
            let (r, g, b) = self.interpolate(ic, CT_WHITE, f64::from(ist - 20) / 20.0)?;
            fprintf!(out, "\\newrgbcolor{{{}}}{{{} {} {}}}%\n", name, f4(r), f4(g), f4(b));
            if let Some(c) = self.colors.get_mut(i) {
                c.tints |= 1 << (ist - 20);
            }
        }
        Ok(name)
    }

    /// `gray_name_after_declare_gray`.
    fn gray_name(&mut self, out: &mut Out, ctx: &Ctx, ig: i32) -> Result<String, String> {
        if !(0..=20).contains(&ig) {
            return Err(format!("bad gray value ({ig})"));
        }
        if ig == 0 {
            return self.color_name(out, ctx, CT_BLACK);
        }
        if ig == 20 {
            return self.color_name(out, ctx, CT_WHITE);
        }
        let idx = ig as usize;
        if let Some(Some(n)) = self.grays.get(idx) {
            return Ok(n.clone());
        }
        if self.verbose {
            fprintf!(out, "% declare gray {}\n", ig);
        }
        let name = format!("gray{}", roman_numeral(ig as u32));
        fprintf!(out, "\\newgray{{{}}}{{{}}}%\n", name, f4(f64::from(ig) / 20.0));
        if let Some(slot) = self.grays.get_mut(idx) {
            *slot = Some(name.clone());
        }
        Ok(name)
    }

    // ----- arrows -----

    /// `put_arrowhead`: the arrowhead as Fig draws it, with its head at
    /// (`x_hd`, `y_hd`) and pointing along (`x_dir`, `y_dir`).
    #[allow(clippy::too_many_arguments)]
    fn put_arrowhead(
        &mut self,
        out: &mut Out,
        ctx: &Ctx,
        x_hd: f64,
        y_hd: f64,
        mut x_dir: f64,
        mut y_dir: f64,
        wid: f64,
        len: f64,
        thickness: f64,
        type_: i32,
        style: i32,
        pen_color: i32,
    ) -> Result<(), String> {
        let blank_fill = ",fillcolor=white".to_string();
        let (fill_fill, line_opt) = if pen_color == -1 {
            (String::new(), String::new())
        } else {
            let n = self.color_name(out, ctx, pen_color)?;
            (format!(",fillcolor={n}"), format!(",linecolor={n}"))
        };
        let fill_opt = if style == HOLLOW_ARROW {
            blank_fill.clone()
        } else {
            fill_fill.clone()
        };

        // convert to centimeters
        let x_hd = self.cm_d(x_hd);
        let y_hd = self.cm_d(y_hd);
        let wid = self.cm_d(wid);
        let len = self.cm_d(len);
        let th = f4(self.cm_d(thickness) * self.line_weight);

        // make direction vector unit length
        let dir_len = x_dir.mul_add(x_dir, y_dir * y_dir).sqrt();
        x_dir /= dir_len;
        y_dir /= dir_len;

        // find base of arrowhead
        let mut x_bas = (-x_dir).mul_add(len, x_hd);
        let mut y_bas = (-y_dir).mul_add(len, y_hd);
        // find unit perp from direction
        let x_prp = -y_dir;
        let y_prp = x_dir;
        // normal baseline endpoints
        let mut x_lft = (x_prp * 0.5).mul_add(wid, x_bas);
        let mut y_lft = (y_prp * 0.5).mul_add(wid, y_bas);
        let mut x_rgt = (-x_prp).mul_add(wid, x_lft);
        let mut y_rgt = (-y_prp).mul_add(wid, y_lft);
        // alternate baseline endpoints
        let x_lfa = (x_prp * 0.5).mul_add(wid, x_hd);
        let y_lfa = (y_prp * 0.5).mul_add(wid, y_hd);
        let x_rga = (-x_prp).mul_add(wid, x_lfa);
        let y_rga = (-y_prp).mul_add(wid, y_lfa);

        if type_ > LAST_OLD_ARROW {
            self.warn(W_NEW_ARROW);
        }
        let verbose = self.verbose;
        let note = |out: &mut Out, s: &str| {
            if verbose {
                fprintf!(out, "% {}\n", s);
            }
        };
        let pts = |v: &[(f64, f64)]| -> String { v.iter().map(|&(x, y)| pt(x, y)).collect() };
        // a polygon outlined and filled
        let poly = |out: &mut Out, v: &[(f64, f64)]| {
            fprintf!(
                out,
                "\\pspolygon[linewidth={th}{line_opt},fillstyle=solid{fill_opt}]{}\n",
                pts(v)
            );
        };
        match type_ {
            TRIANGLE_ARROW => {
                note(out, "triangle arrow");
                poly(out, &[(x_hd, y_hd), (x_lft, y_lft), (x_rgt, y_rgt)]);
            }
            INDENTED_BUTT_ARROW => {
                x_bas = (x_dir * len).mul_add(0.3, x_bas);
                y_bas = (y_dir * len).mul_add(0.3, y_bas);
                note(out, "indented butt arrow");
                poly(out, &[(x_hd, y_hd), (x_lft, y_lft), (x_bas, y_bas), (x_rgt, y_rgt)]);
            }
            POINTED_BUTT_ARROW => {
                x_lft = (x_dir * len).mul_add(0.3, x_lft);
                y_lft = (y_dir * len).mul_add(0.3, y_lft);
                x_rgt = (x_dir * len).mul_add(0.3, x_rgt);
                y_rgt = (y_dir * len).mul_add(0.3, y_rgt);
                note(out, "pointed butt arrow");
                poly(out, &[(x_hd, y_hd), (x_lft, y_lft), (x_bas, y_bas), (x_rgt, y_rgt)]);
            }
            DIAMOND_ARROW => {
                x_lft = (x_dir * len).mul_add(0.5, x_lft);
                y_lft = (y_dir * len).mul_add(0.5, y_lft);
                x_rgt = (x_dir * len).mul_add(0.5, x_rgt);
                y_rgt = (y_dir * len).mul_add(0.5, y_rgt);
                note(out, "diamond arrow");
                poly(out, &[(x_hd, y_hd), (x_lft, y_lft), (x_bas, y_bas), (x_rgt, y_rgt)]);
            }
            CIRCLE_ARROW => {
                note(out, "circle arrow");
                fprintf!(
                    out,
                    "\\pscircle[linewidth={th}{line_opt},fillstyle=solid{fill_opt}]{}{{{}}}%\n",
                    pt(x_hd - x_dir * len / 2.0, y_hd - y_dir * len / 2.0),
                    f4(len / 2.0)
                );
            }
            SEMI_CIRCLE_ARROW => {
                let t_lft = y_prp.atan2(x_prp) * (180.0 / PI);
                let t_rgt = t_lft + 180.0;
                note(out, "semicircle arrow");
                fprintf!(
                    out,
                    "\\psarc[linewidth={th}{line_opt},fillstyle=solid{fill_opt}]{}{{{}}}{{{}}}{{{}}}%\n",
                    pt(x_hd, y_hd),
                    f4(len / 2.0),
                    f4(t_lft),
                    f4(t_rgt)
                );
            }
            RECTANGLE_ARROW => {
                note(out, "rectangle arrow");
                poly(out, &[(x_lfa, y_lfa), (x_lft, y_lft), (x_rgt, y_rgt), (x_rga, y_rga)]);
            }
            REVERSE_TRIANGLE_ARROW => {
                note(out, "reverse triangle arrow");
                poly(out, &[(x_bas, y_bas), (x_rga, y_rga), (x_lfa, y_lfa)]);
            }
            BISHADE_INDENTED_BUTT_ARROW => {
                x_bas = (x_dir * len).mul_add(0.3, x_bas);
                y_bas = (y_dir * len).mul_add(0.3, y_bas);
                note(out, "bishade indented butt arrow");
                let quad = pts(&[(x_hd, y_hd), (x_lft, y_lft), (x_bas, y_bas), (x_rgt, y_rgt)]);
                // clear the arrowhead area
                fprintf!(out, "\\pspolygon[linestyle=none,fillstyle=solid{blank_fill}]{quad}\n");
                // fill the half that's filled
                let half = if style == HOLLOW_ARROW {
                    pts(&[(x_hd, y_hd), (x_lft, y_lft), (x_bas, y_bas)])
                } else {
                    pts(&[(x_bas, y_bas), (x_rgt, y_rgt), (x_hd, y_hd)])
                };
                fprintf!(out, "\\pspolygon[linestyle=none,fillstyle=solid{fill_fill}]{half}\n");
                // rule the outline
                fprintf!(out, "\\pspolygon[linewidth={th}{line_opt}]{quad}\n");
            }
            TRIANGLE_HALF_ARROW => {
                note(out, "triangle half-arrow");
                poly(out, &[(x_bas, y_bas), (x_rgt, y_rgt), (x_hd, y_hd)]);
            }
            INDENTED_BUTT_HALF_ARROW => {
                x_bas = (x_dir * len).mul_add(0.3, x_bas);
                y_bas = (y_dir * len).mul_add(0.3, y_bas);
                note(out, "indented butt half-arrow");
                poly(out, &[(x_bas, y_bas), (x_rgt, y_rgt), (x_hd, y_hd)]);
            }
            POINTED_BUTT_HALF_ARROW => {
                x_rgt = (x_dir * len).mul_add(0.3, x_rgt);
                y_rgt = (y_dir * len).mul_add(0.3, y_rgt);
                note(out, "pointed butt half-arrow");
                poly(out, &[(x_hd, y_hd), (x_bas, y_bas), (x_rgt, y_rgt)]);
            }
            PARTIAL_REVERSE_TRIANGLE_ARROW => {
                if style == HOLLOW_ARROW {
                    note(out, "reverse stick arrow");
                    fprintf!(
                        out,
                        "\\psline[linewidth={th}{line_opt},fillstyle=solid{blank_fill}]{{c-c}}{}\n",
                        pts(&[(x_rga, y_rga), (x_bas, y_bas), (x_lfa, y_lfa)])
                    );
                } else {
                    note(out, "T-bar arrow");
                    fprintf!(
                        out,
                        "\\psline[linewidth={th}{line_opt}]{{c-c}}{}\n",
                        pts(&[(x_rga, y_rga), (x_lfa, y_lfa)])
                    );
                }
            }
            PARTIAL_RECTANGLE_ARROW => {
                if style == HOLLOW_ARROW {
                    note(out, "cup arrow");
                    fprintf!(
                        out,
                        "\\psline[linewidth={th}{line_opt},fillstyle=solid{blank_fill}]{{c-c}}{}\n",
                        pts(&[(x_lfa, y_lfa), (x_lft, y_lft), (x_rgt, y_rgt), (x_rga, y_rga)])
                    );
                } else {
                    note(out, "cap arrow");
                    fprintf!(
                        out,
                        "\\psline[linewidth={th}{line_opt}]{{c-c}}{}\n",
                        pts(&[(x_rgt, y_rgt), (x_rga, y_rga), (x_lfa, y_lfa), (x_lft, y_lft)])
                    );
                }
            }
            t => {
                if t != STICK_ARROW {
                    self.warn(W_UNKNOWN_ARROW);
                }
                if self.verbose {
                    out.puts("% stick arrow\n");
                }
                fprintf!(
                    out,
                    "\\psline[linewidth={th}{line_opt}]{{c-c}}{}\n",
                    pts(&[(x_lft, y_lft), (x_hd, y_hd), (x_rgt, y_rgt)])
                );
            }
        }
        Ok(())
    }

    /// `put_arrowhead_on_seg`: the arrowhead of the segment `tl` to `hd`.
    #[allow(clippy::too_many_arguments)]
    fn put_arrowhead_on_seg(
        &mut self,
        out: &mut Out,
        ctx: &Ctx,
        tl: Point,
        hd: Point,
        a: &Arrow,
        thickness: i32,
        pen_color: i32,
    ) -> Result<(), String> {
        self.put_arrowhead(
            out,
            ctx,
            f64::from(hd.x),
            f64::from(hd.y),
            f64::from(hd.x.wrapping_sub(tl.x)),
            f64::from(hd.y.wrapping_sub(tl.y)),
            a.wid,
            a.ht,
            max_dbl(a.thickness, f64::from(thickness)),
            a.type_,
            a.style,
            pen_color,
        )
    }

    /// `format_terminators`: the square-bracket options and the curly
    /// bracket options of one end of a line.
    fn format_terminators(
        &mut self,
        flags: u32,
        ch_arrow: char,
        arrow: Option<&Arrow>,
        cap_style: i32,
    ) -> Result<(String, String), String> {
        let mut sqrb = String::new();
        let other = if ch_arrow == '<' { '>' } else { '<' };
        let Some(arrow) = arrow.filter(|_| flags & FO_ARROWS != 0) else {
            let curb = match cap_style {
                1 => "c",
                2 => "C",
                _ => "",
            };
            return Ok((sqrb, curb.to_string()));
        };
        let mut curb = ch_arrow.to_string();
        let mut inset = 0.0;
        let mut length = arrow.ht / arrow.wid;
        let size = flags & FO_ARROW_SIZE != 0;
        let hollow = arrow.style == HOLLOW_ARROW;
        match arrow.type_ {
            TRIANGLE_HALF_ARROW => self.warn(W_TERMINATOR_ARROW),
            TRIANGLE_ARROW => {}
            BISHADE_INDENTED_BUTT_ARROW | INDENTED_BUTT_HALF_ARROW => {
                self.warn(W_TERMINATOR_ARROW);
                inset = 0.3;
            }
            INDENTED_BUTT_ARROW => inset = 0.3,
            POINTED_BUTT_HALF_ARROW | POINTED_BUTT_ARROW => {
                if arrow.type_ == POINTED_BUTT_HALF_ARROW {
                    self.warn(W_TERMINATOR_ARROW);
                }
                inset = -0.3;
                length *= 1.0 / (1.0 - inset);
            }
            RECTANGLE_ARROW | DIAMOND_ARROW => {
                if arrow.type_ == RECTANGLE_ARROW {
                    self.warn(W_TERMINATOR_ARROW);
                }
                inset = -1.0;
                length *= 1.0 / (1.0 - inset);
            }
            SEMI_CIRCLE_ARROW => {
                self.warn(W_TERMINATOR_ARROW);
                // closest we can get to semicircle is centered disk
                if size {
                    sqrb = chk(format!("dotsize={} 2", f4(self.cm_d(arrow.ht))))?;
                }
                return Ok((sqrb, if hollow { "o" } else { "*" }.to_string()));
            }
            CIRCLE_ARROW => {
                if size {
                    sqrb = chk(format!("dotsize={} 2", f4(self.cm_d(arrow.ht))))?;
                }
                return Ok((sqrb, if hollow { "oo" } else { "**" }.to_string()));
            }
            PARTIAL_REVERSE_TRIANGLE_ARROW => {
                if hollow {
                    // reverse stick arrow
                    inset = 1.0;
                    curb = other.to_string();
                } else {
                    // t-bar centered on end
                    if size {
                        sqrb = chk(format!("tbarsize={} 2", f4(self.cm_d(arrow.wid))))?;
                    }
                    return Ok((sqrb, "|".to_string()));
                }
            }
            REVERSE_TRIANGLE_ARROW => {
                inset = 0.0;
                curb = other.to_string();
            }
            PARTIAL_RECTANGLE_ARROW => {
                // closest we can get to a cup is a cap
                if hollow {
                    self.warn(W_TERMINATOR_ARROW);
                }
                if size {
                    sqrb = chk(format!(
                        "tbarsize={} 2,bracketlength={}",
                        f4(self.cm_d(arrow.wid)),
                        f4(arrow.ht / arrow.wid)
                    ))?;
                }
                return Ok((sqrb, if ch_arrow == '<' { "[" } else { "]" }.to_string()));
            }
            t => {
                if t != STICK_ARROW {
                    self.warn(W_UNKNOWN_ARROW);
                }
                inset = 1.0;
            }
        }
        if size {
            sqrb = chk(format!(
                "arrowsize={} 2,arrowlength={:.5},arrowinset={:.5}",
                f4(self.cm_d(arrow.wid)),
                Cf(length),
                Cf(inset)
            ))?;
            if hollow {
                // warn user hollow arrows require additional package pstricks-add
                self.warn(W_HOLLOW_ARROW);
                append_to_comma_list(&mut sqrb, "ArrowFill=false")?;
            }
        }
        Ok((sqrb, curb))
    }

    /// `format_options`: the `[...]` and `{...}` options of a command, with
    /// the line join commands that go before and after it (the last two
    /// only if `with_prefix`).
    fn format_options(
        &mut self,
        out: &mut Out,
        ctx: &mut Ctx,
        a: &FoArgs,
        sqrb_init: Option<&str>,
        with_prefix: bool,
    ) -> Result<(String, String, String), String> {
        let mut sqrb = sqrb_init.unwrap_or("").to_string();
        let mut curb = String::new();
        let (mut prefix, mut postfix) = (String::new(), String::new());
        let lw = self.line_weight;
        let c_thick = self.cm(a.thickness);

        if a.flags & FO_LINE_STYLE != 0 {
            let mut style = a.style;
            // fig allows filled lines where the line has zero width; the
            // intent is to draw the filling with no line at all
            if a.thickness == 0 {
                style = -2;
            } else {
                let t = chk(format!("linewidth={}", f4(c_thick * lw)))?;
                append_to_comma_list(&mut sqrb, &t)?;
            }
            let sv = cm_80(a.style_val);
            let g = 2.0 * c_thick * lw;
            let tmp = match style {
                -2 => Some("linestyle=none".to_string()),
                -1 | SOLID_LINE => None,
                DOTTED_LINE => Some(format!("linestyle=dotted,dotsep={}", f4(sv))),
                DASH_LINE => Some(format!("linestyle=dashed,dash={} {}", f4(sv), f4(sv))),
                DASH_DOT_LINE => {
                    self.warn(W_DASH_DOT);
                    Some(format!(
                        "linestyle=dashed,dash={} {} {} {}",
                        f4(sv),
                        f4(sv / 2.0),
                        f4(g),
                        f4(sv / 2.0)
                    ))
                }
                DASH_2_DOTS_LINE => {
                    self.warn(W_DASH_DOTS);
                    Some(format!(
                        "linestyle=dashed,dash={} {} {} {} {} {}",
                        f4(sv),
                        f4(sv / 3.0),
                        f4(g),
                        f4(sv / 3.0),
                        f4(g),
                        f4(sv / 3.0)
                    ))
                }
                DASH_3_DOTS_LINE => {
                    self.warn(W_DASH_DOTS);
                    Some(format!(
                        "linestyle=dashed,dash={} {} {} {} {} {} {} {}",
                        f4(sv),
                        f4(sv / 4.0),
                        f4(g),
                        f4(sv / 4.0),
                        f4(g),
                        f4(sv / 4.0),
                        f4(g),
                        f4(sv / 4.0)
                    ))
                }
                _ => return Err(format!("bad line style ({style})")),
            };
            if let Some(t) = tmp {
                append_to_comma_list(&mut sqrb, &chk(t)?)?;
            }
        }

        // add linejoin in one of two ways, which depend on the PSTricks
        // version: \pstVerb or [linejoin=N]
        if with_prefix && a.flags & FO_JOIN_STYLE != 0 && a.join_style > 0 {
            if self.pst_updates == 0 {
                self.warn(W_LINEJOIN);
            }
            if self.linejoin_pstverb {
                prefix = format!("\\pstVerb{{{} setlinejoin}}%\n", a.join_style);
                postfix = "\\pstVerb{0 setlinejoin}%\n".to_string();
            } else {
                append_to_comma_list(&mut sqrb, &format!("linejoin={}", a.join_style))?;
            }
        }

        // add cap style and optionally the arrows
        if a.flags & FO_LINE_TERM != 0 {
            let (tmps, c1) = self.format_terminators(a.flags, '<', a.back_arrow, a.cap_style)?;
            curb.push_str(&c1);
            curb.push('-');
            let (tmps_alt, c2) = self.format_terminators(a.flags, '>', a.fore_arrow, a.cap_style)?;
            curb.push_str(&c2);
            // forward arrow parameters take precedence over back
            append_to_comma_list(&mut sqrb, if tmps_alt.is_empty() { &tmps } else { &tmps_alt })?;
        }

        // deal with pen color
        if a.flags & FO_PEN_COLOR != 0 && a.pen_color != -1 {
            let n = self.color_name(out, ctx, a.pen_color)?;
            append_to_comma_list(&mut sqrb, &format!("linecolor={n}"))?;
        }

        if a.flags & FO_FILL != 0 && a.fill_style != -1 {
            let fs = a.fill_style;
            let tmps = if a.fill_color == CT_WHITE && fs <= 20 {
                // black to white gray fill
                format!("fillstyle=solid,fillcolor={}", self.gray_name(out, ctx, fs)?)
            } else if (a.fill_color == CT_BLACK || a.fill_color == -1) && fs <= 20 {
                // white to black gray fill
                format!("fillstyle=solid,fillcolor={}", self.gray_name(out, ctx, 20 - fs)?)
            } else if fs <= 40 {
                // shade or tint fill
                let ic = if a.fill_color == DEFAULT {
                    CT_BLACK
                } else {
                    a.fill_color
                };
                format!(
                    "fillstyle=solid,fillcolor={}",
                    self.shade_or_tint_name(out, ctx, fs, ic)?
                )
            } else {
                // hatch pattern
                let (type_, angle) = match fs {
                    41 => ("hlines", 30),
                    42 => ("hlines", -30),
                    43 => ("crosshatch", 30),
                    44 => ("vlines", 45),
                    45 => ("hlines", -45),
                    46 => ("crosshatch", 45),
                    49 => ("hlines", 0),
                    50 => ("vlines", 0),
                    51 => ("crosshatch", 0),
                    _ => {
                        self.warn(W_HATCH_PATTERN);
                        ("crosshatch", 10)
                    }
                };
                // build up fill and hatch color strings, accounting for default colors
                let mut ps = String::new();
                if a.fill_color != -1 {
                    let n = self.color_name(out, ctx, a.fill_color)?;
                    append_to_comma_list(&mut ps, &format!("fillcolor={n}"))?;
                }
                if a.pen_color != -1 {
                    let n = self.color_name(out, ctx, a.pen_color)?;
                    append_to_comma_list(&mut ps, &format!("hatchcolor={n}"))?;
                }
                // negative sign on angle for fig's strange clockwise system
                let ha = -(f64::from(angle) + a.hatch_angle_offset);
                append_to_comma_list(&mut ps, &format!("fillstyle={type_}*,hatchangle={:.2}", Cf(ha)))?;
                ps
            };
            append_to_comma_list(&mut sqrb, &tmps)?;
        }

        let options = if !curb.is_empty() && !sqrb.is_empty() {
            format!("[{sqrb}]{{{curb}}}")
        } else if !sqrb.is_empty() {
            format!("[{sqrb}]")
        } else if !curb.is_empty() {
            format!("{{{curb}}}")
        } else {
            String::new()
        };
        Ok((options, prefix, postfix))
    }

    /// `put_points`: the points, four to a line. `flag`: 0 all, 1 without
    /// the last one, 2 all and then the first two again.
    fn put_points(&self, out: &mut Out, list: &[Point], flag: i32) {
        let mut n_points = 0;
        let mut countdown = -1;
        let mut idx = 0;
        loop {
            let Some(p) = list.get(idx) else {
                out.puts("\n");
                return;
            };
            fprintf!(out, "({},{})", f4(self.cm(p.x)), f4(self.cm(p.y)));
            n_points += 1;
            countdown -= 1;
            idx += 1;
            let mut has_next = idx < list.len();
            if !has_next && flag == 2 && n_points > 2 {
                idx = 0;
                has_next = true;
                countdown = 2;
            }
            if !has_next || (flag == 1 && idx + 1 >= list.len()) || countdown == 0 {
                out.puts("\n");
                return;
            }
            if n_points % 4 == 0 {
                out.puts("\n\t");
            }
        }
    }

    /// `do_eps_conversion`: the path of the eps file for a picture; may run
    /// `bmeps` if the user asked for it with `-p`.
    fn do_eps_conversion(&mut self, ctx: &mut Ctx, src: &str) -> String {
        if let Some((_, eps)) = self.converted.iter().find(|(s, _)| s == src) {
            return eps.clone();
        }
        let mut buf: Vec<u8> = Vec::with_capacity(src.len());
        let mut islash: isize = -1;
        for (i, &c) in src.as_bytes().iter().enumerate() {
            match c {
                b'\\' => {
                    self.warn(W_SLASH_CONVERT);
                    islash = i as isize;
                    buf.push(b'/');
                }
                b'/' => {
                    islash = i as isize;
                    buf.push(c);
                }
                b' ' | b'>' | b'<' | b'|' | b'$' | b'%' | b'*' | b'?' => {
                    self.warn(W_FN_CHARS);
                    buf.push(c);
                }
                _ => buf.push(c),
            }
        }
        // get the base file name from the slash location, cut off the extension
        let start = (islash + 1) as usize;
        let mut base_end = buf.len();
        let mut ext_start = buf.len();
        for i in (start..buf.len()).rev() {
            if buf.get(i) == Some(&b'.') {
                base_end = i;
                ext_start = i + 1;
                break;
            }
        }
        let base = String::from_utf8_lossy(buf.get(start..base_end).unwrap_or(&[])).into_owned();
        let ext = String::from_utf8_lossy(buf.get(ext_start..).unwrap_or(&[])).to_ascii_lowercase();

        // if the base is already in the table, the name is used for some
        // other converted path with the same base, so make it unique
        let uniq = if let Some(e) = self.base_names.iter_mut().find(|(n, _)| *n == base) {
            e.1 = e.1.wrapping_add(1);
            format!("{}-{:03}", base, e.1)
        } else {
            base
        };
        let eps_path = format!("{}{}", self.pic_convert_dir, uniq);

        // we act as if conversion succeeded even if it doesn't, because the
        // LaTeX file is useless unless the user gets the conversion done
        self.converted.push((src.to_string(), eps_path.clone()));
        if !self.base_names.iter().any(|(n, _)| *n == uniq) {
            self.base_names.push((uniq, 0));
        }

        if self.pic_convert_p {
            let kind = match ext.as_str() {
                "png" => Some("png"),
                "jpg" | "jpeg" => Some("jpg"),
                "pnm" => Some("pnm"),
                "tif" | "tiff" => Some("tif"),
                _ => None,
            };
            match kind {
                None => self.warn(W_UNK_PIC_TYPE),
                Some(kind) => {
                    let dest = format!("{eps_path}.eps");
                    let errval = run_bmeps(kind, src, &dest);
                    if self.verbose {
                        ctx.warn(format!(
                            "running system(bmeps -c -t {kind} \"{src}\" \"{dest}\"): returned {errval}"
                        ));
                    }
                    if errval == -1 {
                        self.warn(W_SYSTEM_EXEC);
                    } else if errval != 0 {
                        self.warn(W_BMEPS_ERR);
                    } else {
                        self.warn(W_PIC_CONVERT);
                    }
                }
            }
        } else {
            self.warn(W_EPS_NEEDED);
        }
        eps_path
    }

    /// `put_bezier`: a quadratic bezier curve as a cubic one.
    fn put_bezier(&self, out: &mut Out, options: &str, p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) {
        let k: f64 = 2.0 / 3.0;
        fprintf!(
            out,
            "\\psbezier{}{}{}{}{}\n",
            options,
            pt(p1.0, p1.1),
            pt(k.mul_add(p2.0 - p1.0, p1.0), k.mul_add(p2.1 - p1.1, p1.1)),
            pt(k.mul_add(p2.0 - p3.0, p3.0), k.mul_add(p2.1 - p3.1, p3.1)),
            pt(p3.0, p3.1)
        );
    }

    /// The flags of an arrow-bearing open object (`FO_*_STD`).
    fn arrow_flags(&self, std: u32) -> u32 {
        match self.arrows {
            A_XFIG => std,
            A_PSTRICKS => std | FO_ARROWS | FO_ARROW_SIZE,
            _ => std | FO_ARROWS,
        }
    }

    /// `begin_set_font`; the text's font may be replaced by a LaTeX font.
    fn begin_set_font(&mut self, out: &mut Out, ctx: &mut Ctx, t: &mut Text) {
        // substitute latex for postscript fonts and warn the user
        if t.psfont_text() {
            let i = ps_index(t.font);
            let map = PS_FONT_MAP.get(i).copied().unwrap_or(0);
            if PS_MAP_WARN.get(i).copied().unwrap_or(false) {
                self.warn(W_PS_FONT);
                if self.verbose {
                    const FIG_FONT_NAMES: [&str; 7] =
                        ["Roman", "Roman", "Roman", "Bold", "Italic", "Modern", "Typewriter"];
                    let sub = usize::try_from(map + 1)
                        .ok()
                        .and_then(|j| FIG_FONT_NAMES.get(j))
                        .copied()
                        .unwrap_or("Roman");
                    ctx.warn(format!(
                        "Substituting LaTeX {} for Postscript {} ('{}')",
                        sub,
                        ps_font_name(t.font),
                        c_string(&t.cstring)
                    ));
                }
            }
            // leave default to be default, but no-ps
            t.font = if t.font == -1 { 0 } else { map };
        }
        let texsize = self.fonts.texfontmag(t, ctx.fontmag) as i32;
        let baselineskip = f64::from(texsize) * 1.2;
        if self.font_handling == FH_SIZE_ONLY {
            fprintf!(
                out,
                "\\begingroup\\SetFigFontSizeOnly{{{}}}{{{:.1}}}%\n",
                texsize,
                Cf(baselineskip)
            );
        } else if self.font_handling == FH_FULL {
            fprintf!(
                out,
                "\\begingroup\\SetFigFont{{{}}}{{{:.1}}}{{{}}}{{{}}}{{{}}}%\n",
                texsize,
                Cf(baselineskip),
                self.fonts.texfamily(t.font),
                self.fonts.texseries(t.font),
                self.fonts.texshape(t.font)
            );
        }
    }

    /// `define_setfont`.
    fn define_setfont(&self, out: &mut Out) {
        if !self.has_text {
            return;
        }
        if self.font_handling == FH_SIZE_ONLY {
            out.puts(
                "%\n\\begingroup\\makeatletter%\n\\ifx\\SetFigFontSizeOnly\\undefined%\n  \\gdef\\SetFigFont#1#2{%\n\t \\fontsize{#1}{#2pt}%\n\t \\selectfont}%\n\\fi\n\\endgroup%\n",
            );
        } else if self.font_handling == FH_FULL {
            out.puts(
                "%\n\\begingroup\\makeatletter%\n\\ifx\\SetFigFont\\undefined%\n  \\gdef\\SetFigFont#1#2#3#4#5{%\n\t \\reset@font\\fontsize{#1}{#2pt}%\n\t \\fontfamily{#3}\\fontseries{#4}\\fontshape{#5}%\n\t \\selectfont}%\n\\fi\n\\endgroup%\n",
            );
        }
    }
}

/// Runs `bmeps` on `src` (C's `system`): the wait status, -1 if the
/// program could not be run at all.
#[cfg(not(target_arch = "wasm32"))]
fn run_bmeps(kind: &str, src: &str, dest: &str) -> i32 {
    match std::process::Command::new("bmeps")
        .args(["-c", "-t", kind, src, dest])
        .status()
    {
        // `system` returns the wait status: the exit code in the high byte
        Ok(s) => s.code().map_or(-1, |c| c.wrapping_shl(8)),
        // the shell reports a missing program as exit code 127
        Err(_) => 127 << 8,
    }
}

#[cfg(target_arch = "wasm32")]
fn run_bmeps(_kind: &str, _src: &str, _dest: &str) -> i32 {
    -1
}

/// `-p`: writes the readme into the conversion directory.
#[cfg(not(target_arch = "wasm32"))]
fn write_readme(dir: &str) -> bool {
    std::fs::write(
        format!("{dir}readme.txt"),
        "This directory has been used by the fig2dev pstricks driver.\nAny '.eps' file here may be overwritten.\n",
    )
    .is_ok()
}

#[cfg(target_arch = "wasm32")]
fn write_readme(_dir: &str) -> bool {
    false
}

impl Driver for Pstricks {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        // every option call resets the PSTricks version to the default
        (self.pst_str, self.pst_key) = parse_pst_version(ASSUMED_PST_VERSION).unwrap_or_default();
        self.pst_updates = 0;
        let a = arg.unwrap_or("");
        match opt {
            'f' => default_font_option(&mut self.fonts, ctx, a),
            // grid spec; just warn that we don't pay attention to spacing
            'G' => self.warn(W_GRID),
            'L' => {}
            'l' => match scan_double(a) {
                Some(v) if (0.0..=2.0).contains(&v) => self.line_weight = v,
                _ => ctx.warn(format!("warning: bad line weight {a}, expected 0 to 2.0")),
            },
            'n' => match scan_int(a) {
                Some(v) if (0..=3).contains(&v) => self.page_mode = v,
                _ => ctx.warn("warning: bad page mode (0, 1, 2, or 3 expected)"),
            },
            'P' => self.page_mode = PM_SCALED_PAGE,
            'p' => {
                self.pic_convert_p = true;
                if a != "-" {
                    // copy while converting slashes
                    let mut dir: String = a
                        .chars()
                        .take(MAX_CONVERT_DIR)
                        .map(|c| if c == '\\' { '/' } else { c })
                        .collect();
                    // append slash if not there already
                    if !dir.is_empty() && !dir.ends_with('/') {
                        dir.push('/');
                    }
                    self.pic_convert_dir = dir;
                }
                if !write_readme(&self.pic_convert_dir) {
                    return Err(format!(
                        "can't write the eps conversion directory {}.",
                        self.pic_convert_dir
                    ));
                }
            }
            'R' => match scan_int(a) {
                Some(v) if (0..=2).contains(&v) => self.arrows = v,
                _ => ctx.warn("warning: bad arrow spec (0, 1, or 2 expected)"),
            },
            'S' => {
                let Some(arg) = arg else {
                    return Err("Integer argument to -S between 8 and 12 expected.".to_string());
                };
                match scan_int(arg) {
                    Some(v) if (8..=12).contains(&v) => {
                        let (m, s) = SCALE_TBL[(v - 8) as usize];
                        ctx.mag = m;
                        ctx.font_size = f64::from(s);
                    }
                    // C reads an uninitialised value if there is no number
                    _ => return Err("Scale must be between 8 and 12 inclusively.".to_string()),
                }
            }
            't' => match parse_pst_version(a) {
                Ok((s, key)) => {
                    self.pst_str = s;
                    self.pst_key = key;
                    self.pst_updates = 1;
                    // compare with the first version that had linejoin to see
                    // which style of linejoin command to emit
                    let join = parse_pst_version(PST_LINEJOIN_VERSION)
                        .map(|(_, k)| k)
                        .unwrap_or_default();
                    self.linejoin_pstverb = self.pst_key < join;
                }
                Err(e) => ctx.warn(format!("{e}warning: bad PSTricks version '{a}' was ignored")),
            },
            'v' => self.verbose = true,
            'x' => match scan_double(a) {
                Some(v) if (0.0..=100.0).contains(&v) => self.x_margin = 1200.0 / 2.54 * v,
                _ => ctx.warn("warning: bad x margin setting ignored"),
            },
            'y' => match scan_double(a) {
                Some(v) if (0.0..=100.0).contains(&v) => self.y_margin = 1200.0 / 2.54 * v,
                _ => ctx.warn("warning: bad y margin setting ignored"),
            },
            'z' => match scan_int(a) {
                Some(v) if (0..=2).contains(&v) => self.font_handling = v,
                _ => ctx.warn("warning: bad font spec (0, 1, or 2 expected)"),
            },
            _ => return Err(format!("Argument -{opt} unknown to pstricks driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        self.ppi = ctx.ppi;
        self.enc = text_encoding(ctx);
        self.x_origin = ctx.llx.min(ctx.urx);
        self.y_origin = ctx.lly.max(ctx.ury);
        self.setup_colors(ctx);

        // run the preprocessor
        find_extras(objects, &mut self.extra_package_mask);
        self.has_text = find_text(objects);

        // if the user gave a font size, set up the tables
        set_default_size(&mut self.fonts, ctx);
        self.define_setfont(out);

        // print header information
        if self.verbose {
            out.puts("% PSTricks driver for Fig Version 1.2 by Gene Ressler.\n");
            fprintf!(
                out,
                "% PSTricks version {} is assumed in Sketch output (change with -t).\n",
                self.pst_str
            );
        }

        // print any whole-figure comments prefixed with "%"
        if !objects.comments.is_empty() {
            out.puts("%\n");
            print_comments(out, "% ", &objects.comments, "");
            out.puts("%\n");
        }

        // print preamble if user asked for complete tex file
        if self.page_mode >= PM_PAGE {
            out.puts(
                "\\documentclass{article}\n\\pagestyle{empty}\n\\setlength\\topmargin{-.5in}\n\\setlength\\textheight{10in}\n\\setlength\\oddsidemargin{-.5in}\n\\setlength\\evensidemargin\\oddsidemargin\n\\setlength\\textwidth{7.5in}\n\\usepackage{pstricks",
            );
            // mask was set up by the preprocessor
            for (i, name) in EXTRA_PACKAGE_NAMES.iter().enumerate() {
                if self.extra_package_mask & (1 << i) != 0 {
                    fprintf!(out, ",{}", name);
                }
            }
            out.puts("}\n\\begin{document}\n\\begin{center}\n");
        }

        let pllx = self.cm_d(-self.x_margin);
        let plly = self.cm_d(-self.y_margin);
        let dx = ctx.urx.wrapping_sub(ctx.llx).wrapping_abs();
        let dy = ctx.ury.wrapping_sub(ctx.lly).wrapping_abs();
        let purx = self.cm_d(f64::from(1i32.wrapping_add(dx)) + self.x_margin);
        let pury = self.cm_d(f64::from(1i32.wrapping_add(dy)) + self.y_margin);

        // default unit is 1cm
        let mut unit = 1.0f64;
        if self.page_mode == PM_SCALED_PAGE {
            // allow a small epsilon for page size to avoid spurious blank page
            const PAGE_EPS: f64 = 0.01;
            let a = (7.5 * 2.54 - PAGE_EPS) / (purx - pllx);
            unit = if a < unit { a } else { unit };
            let b = (10.0 * 2.54 - PAGE_EPS) / (pury - plly);
            unit = if b < unit { b } else { unit };
            if unit < 1.0 {
                fprintf!(out, "\\psset{{unit={}}}\n", f4(unit));
                self.warn(W_PAGESCALE);
            }
            // this works only roughly
            ctx.fontmag *= unit;
        }
        // set up unit length for pictures if there are any
        if self.extra_package_mask & (1 << EP_GRAPHICX) != 0 {
            fprintf!(
                out,
                "% Following sets up unit length for picture dimensions. If you \n% do \\psset{{unit=LEN}} to scale the picture, you must also\n% \\newlength\\unit\\setlength\\unit{{LEN}} for the bitmaps to match.\n\\makeatletter%\n\\@ifundefined{{unit}}{{\\newlength\\unit\\setlength\\unit{{{:.3}cm}}}}%\n\\makeatother%\n",
                Cf(unit)
            );
            self.warn(W_UNIT);
        }
        if self.page_mode != PM_BARE {
            fprintf!(
                out,
                "\\begin{{pspicture}}({},{})({},{})\n",
                f4(pllx),
                f4(plly),
                f4(purx),
                f4(pury)
            );
        }
        Ok(())
    }

    fn grid(&mut self, _ctx: &mut Ctx, out: &mut Out, major: f32, minor: f32) -> Result<(), String> {
        if minor == 0.0 && major == 0.0 {
            return Ok(());
        }
        out.puts("\\psgrid[gridcolor=lightgray,subgridcolor=lightgray]\n");
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        print_comments(out, "% ", &a.comments, "");
        if self.verbose {
            out.puts("% arc\n");
        }
        let x = a.center.x - f64::from(self.x_origin);
        let y = f64::from(self.y_origin) - a.center.y;
        let (p0, p1) = if a.direction == 1 {
            (a.point[0], a.point[2])
        } else {
            (a.point[2], a.point[0])
        };
        let b0 = self.to_bb(p0);
        let b1 = self.to_bb(p1);
        let (x_0, y_0, x_1, y_1) = (b0.x, b0.y, b1.x, b1.y);
        let t_0 = (f64::from(y_0) - y).atan2(f64::from(x_0) - x) * (180.0 / PI);
        let t_1 = (f64::from(y_1) - y).atan2(f64::from(x_1) - x) * (180.0 / PI);
        let sq = |v: f64| v * v;
        let r = (sq(f64::from(x_0) - x) + sq(f64::from(y_0) - y)).sqrt();

        match a.type_ {
            T_OPEN_ARC => {
                let (a_0, a_1) = if a.direction == 1 {
                    (a.back_arrow.as_ref(), a.for_arrow.as_ref())
                } else {
                    (a.for_arrow.as_ref(), a.back_arrow.as_ref())
                };
                let fo = FoArgs {
                    flags: self.arrow_flags(FO_LINE_STYLE | FO_PEN_COLOR | FO_LINE_TERM | FO_FILL),
                    thickness: a.thickness,
                    style: a.style,
                    style_val: a.style_val,
                    pen_color: a.pen_color,
                    cap_style: a.cap_style,
                    fill_style: a.fill_style,
                    fill_color: a.fill_color,
                    back_arrow: a_0,
                    fore_arrow: a_1,
                    ..FoArgs::default()
                };
                let (options, _, _) = self.format_options(out, ctx, &fo, None, false)?;
                fprintf!(
                    out,
                    "\\psarc{}{}{{{}}}{{{}}}{{{}}}%\n",
                    options,
                    pt(self.cm_d(x), self.cm_d(y)),
                    f4(self.cm_d(r)),
                    f4(t_0),
                    f4(t_1)
                );
                if self.arrows == A_XFIG {
                    if let Some(a0) = a_0 {
                        self.put_arrowhead(
                            out,
                            ctx,
                            f64::from(x_0),
                            f64::from(y_0),
                            f64::from(y_0) - y,
                            x - f64::from(x_0),
                            a0.wid,
                            a0.ht,
                            max_dbl(a0.thickness, f64::from(a.thickness)),
                            a0.type_,
                            a0.style,
                            a.pen_color,
                        )?;
                    }
                    if let Some(a1) = a_1 {
                        self.put_arrowhead(
                            out,
                            ctx,
                            f64::from(x_1),
                            f64::from(y_1),
                            y - f64::from(y_1),
                            f64::from(x_1) - x,
                            a1.wid,
                            a1.ht,
                            max_dbl(a1.thickness, f64::from(a.thickness)),
                            a1.type_,
                            a1.style,
                            a.pen_color,
                        )?;
                    }
                }
            }
            T_PIE_WEDGE_ARC => {
                let fo = FoArgs {
                    flags: FO_LINE_STYLE | FO_PEN_COLOR | FO_FILL,
                    thickness: a.thickness,
                    style: a.style,
                    style_val: a.style_val,
                    pen_color: a.pen_color,
                    cap_style: a.cap_style,
                    fill_style: a.fill_style,
                    fill_color: a.fill_color,
                    ..FoArgs::default()
                };
                let (options, _, _) = self.format_options(out, ctx, &fo, None, false)?;
                fprintf!(
                    out,
                    "\\pswedge{}{}{{{}}}{{{}}}{{{}}}%\n",
                    options,
                    pt(self.cm_d(x), self.cm_d(y)),
                    f4(self.cm_d(r)),
                    f4(t_0),
                    f4(t_1)
                );
            }
            t => ctx.warn(format!("unknown arc type ({t})")),
        }
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        let c = self.to_bb(e.center);
        let (x, y) = (c.x, c.y);
        // note the kludge to compensate for rotation in hatch pattern angle
        let fo = FoArgs {
            flags: FO_LINE_STYLE | FO_PEN_COLOR | FO_FILL,
            thickness: e.thickness,
            style: e.style,
            style_val: e.style_val,
            pen_color: e.pen_color,
            fill_style: e.fill_style,
            fill_color: e.fill_color,
            hatch_angle_offset: e.angle * (180.0 / PI),
            ..FoArgs::default()
        };
        let (options, _, _) = self.format_options(out, ctx, &fo, None, false)?;
        let (cx, cy) = (self.cm(x), self.cm(y));
        let (rx, ry) = (self.cm(e.radiuses.x), self.cm(e.radiuses.y));
        // cull out circles as a special case
        if e.radiuses.x == e.radiuses.y {
            fprintf!(out, "\\pscircle{}{}{{{}}}%\n", options, pt(cx, cy), f4(rx));
        } else if e.angle == 0.0 {
            fprintf!(out, "\\psellipse{}{}{}\n", options, pt(cx, cy), pt(rx, ry));
        } else {
            if e.fill_style > 40 {
                self.warn(W_ELLIPSE_HATCH);
            }
            fprintf!(
                out,
                "\\rput{{{}}}{}{{\\psellipse{}(0,0){}}}%\n",
                f4(e.angle * (180.0 / PI)),
                pt(cx, cy),
                options,
                pt(rx, ry)
            );
        }
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        print_comments(out, "%% ", &l.comments, "");
        if l.points.is_empty() {
            return Ok(());
        }
        // adjust for bounding box and top<->bottom flip
        let pts: Vec<Point> = l.points.iter().map(|&p| self.to_bb(p)).collect();
        let n = pts.len();
        let (Some(&first), Some(&last)) = (pts.first(), pts.last()) else {
            return Ok(());
        };

        // if only one point, draw a dot; polygons always have the first
        // point repeated
        if n == 1 || (l.type_ == T_POLYGON && n == 2) {
            if self.verbose {
                out.puts("% line, length zero\n");
            }
            fprintf!(
                out,
                "\\psdots[dotsize={}]{}\n",
                f4(self.cm(l.thickness) * self.line_weight / 2.0),
                pt(self.cm(first.x), self.cm(first.y))
            );
            return Ok(());
        }

        let base = FoArgs {
            thickness: l.thickness,
            style: l.style,
            style_val: l.style_val,
            pen_color: l.pen_color,
            join_style: l.join_style,
            cap_style: l.cap_style,
            fill_style: l.fill_style,
            fill_color: l.fill_color,
            ..FoArgs::default()
        };
        match l.type_ {
            T_ARC_BOX => {
                if self.verbose {
                    out.puts("% arc box\n");
                }
                let (llx, lly, urx, ury) = bounding_box(&pts);
                // radius of the corner
                let radius = l.radius.min(urx.wrapping_sub(llx) / 2).min(ury.wrapping_sub(lly) / 2);
                let init = format!("cornersize=absolute,linearc={}", f4(self.cm(radius)));
                let fo = FoArgs {
                    flags: FO_LINE_STYLE | FO_PEN_COLOR | FO_FILL,
                    ..base
                };
                let (options, _, _) = self.format_options(out, ctx, &fo, Some(&init), false)?;
                fprintf!(
                    out,
                    "\\psframe{}{}{}\n",
                    options,
                    pt(self.cm(llx), self.cm(lly)),
                    pt(self.cm(urx), self.cm(ury))
                );
            }
            T_PIC_BOX => {
                // the rotation angle ends up encoded in the diagonal's x and y order
                const ROT_FROM_SIGNS: [[i32; 2]; 2] = [[90, 0], [180, -90]];
                const FLIPPED_ROT_FROM_SIGNS: [[i32; 2]; 2] = [[0, 90], [-90, 180]];
                if self.verbose {
                    out.puts("% pic box\n");
                }
                self.warn(W_PIC);
                let (llx, lly, urx, ury) = bounding_box(&pts);
                let (x0, y0) = (llx, lly);
                let wd = urx.wrapping_sub(llx);
                let mut ht = ury.wrapping_sub(lly);
                let corner = pts.get(2).copied().unwrap_or(last);
                let sx = usize::from(corner.x < first.x);
                let sy = usize::from(corner.y < first.y);
                let flipped = l.pic.as_ref().is_some_and(|p| p.flipped != 0);
                let table = if flipped {
                    // a flip is equivalent to a 90d rotation and a mirror about the x-axis
                    ht = ht.wrapping_neg();
                    &FLIPPED_ROT_FROM_SIGNS
                } else {
                    &ROT_FROM_SIGNS
                };
                let rot = table.get(sx).and_then(|r| r.get(sy)).copied().unwrap_or(0);
                // format the pic file name, but this may also do a conversion to eps
                let file = l.pic.as_ref().map_or("", |p| c_string(&p.file)).to_string();
                let file_name = self.do_eps_conversion(ctx, &file);
                fprintf!(
                    out,
                    "\\rput[bl]{}{{\\includegraphics[origin=c,angle={},width={}\\unit,totalheight={}\\unit]{{{}}}}}%\n",
                    pt(self.cm(x0), self.cm(y0)),
                    rot,
                    f4(self.cm(wd)),
                    f4(self.cm(ht)),
                    file_name
                );
            }
            T_BOX => {
                if self.verbose {
                    out.puts("% box\n");
                }
                let (llx, lly, urx, ury) = bounding_box(&pts);
                let fo = FoArgs {
                    flags: FO_LINE_STYLE | FO_JOIN_STYLE | FO_PEN_COLOR | FO_FILL,
                    ..base
                };
                let (options, prefix, postfix) = self.format_options(out, ctx, &fo, None, true)?;
                fprintf!(
                    out,
                    "{}\\psframe{}{}{}\n{}",
                    prefix,
                    options,
                    pt(self.cm(llx), self.cm(lly)),
                    pt(self.cm(urx), self.cm(ury)),
                    postfix
                );
            }
            T_POLYLINE => {
                if self.verbose {
                    out.puts("% polyline\n");
                }
                let fo = FoArgs {
                    flags: self.arrow_flags(FO_LINE_STYLE | FO_JOIN_STYLE | FO_PEN_COLOR | FO_FILL | FO_LINE_TERM),
                    back_arrow: l.back_arrow.as_ref(),
                    fore_arrow: l.for_arrow.as_ref(),
                    ..base
                };
                let (options, prefix, postfix) = self.format_options(out, ctx, &fo, None, true)?;
                fprintf!(out, "{}\\psline{}", prefix, options);
                self.put_points(out, &pts, 0);
                out.puts(&postfix);
                if self.arrows == A_XFIG {
                    if let (Some(b), Some(&second)) = (l.back_arrow.as_ref(), pts.get(1)) {
                        self.put_arrowhead_on_seg(out, ctx, second, first, b, l.thickness, l.pen_color)?;
                    }
                    if let (Some(f), Some(&tl)) = (l.for_arrow.as_ref(), pts.get(n - 2)) {
                        self.put_arrowhead_on_seg(out, ctx, tl, last, f, l.thickness, l.pen_color)?;
                    }
                }
            }
            T_POLYGON => {
                if self.verbose {
                    out.puts("% polygon\n");
                }
                // if >2 points (3 with repeat at end), draw a polygon, else a line
                let postfix;
                if n > 3 {
                    let fo = FoArgs {
                        flags: FO_LINE_STYLE | FO_JOIN_STYLE | FO_PEN_COLOR | FO_FILL,
                        ..base
                    };
                    let (options, prefix, post) = self.format_options(out, ctx, &fo, None, true)?;
                    postfix = post;
                    fprintf!(out, "{}\\pspolygon{}", prefix, options);
                } else {
                    // no JOIN_STYLE clears postfix
                    let fo = FoArgs {
                        flags: FO_LINE_STYLE | FO_PEN_COLOR,
                        ..base
                    };
                    let (options, _, post) = self.format_options(out, ctx, &fo, None, true)?;
                    postfix = post;
                    fprintf!(out, "\\psline{}", options);
                }
                self.put_points(out, &pts, 1);
                out.puts(&postfix);
            }
            _ => {}
        }
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        if s.points.is_empty() {
            return Ok(());
        }
        // warn user this code is completely untested
        self.warn(W_OLD_SPLNE);
        let pts: Vec<Point> = s.points.iter().map(|&p| self.to_bb(p)).collect();
        let n = pts.len();
        let (Some(&first), Some(&last)) = (pts.first(), pts.last()) else {
            return Ok(());
        };

        // if only one point, draw a dot
        if n == 1 {
            if self.verbose {
                out.puts("% spline, length zero\n");
            }
            fprintf!(
                out,
                "\\psdots[dotsize={}]{}\n",
                f4(self.cm(s.thickness) * self.line_weight / 2.0),
                pt(self.cm(first.x), self.cm(first.y))
            );
            return Ok(());
        }

        let base = FoArgs {
            thickness: s.thickness,
            style: s.style,
            style_val: s.style_val,
            pen_color: s.pen_color,
            cap_style: s.cap_style,
            fill_style: s.fill_style,
            fill_color: s.fill_color,
            back_arrow: s.back_arrow.as_ref(),
            fore_arrow: s.for_arrow.as_ref(),
            ..FoArgs::default()
        };
        let std_flags = FO_LINE_STYLE | FO_PEN_COLOR | FO_LINE_TERM;

        // if two points, draw a line
        if n == 2 {
            let fo = FoArgs {
                flags: self.arrow_flags(std_flags),
                ..base
            };
            let (options, _, _) = self.format_options(out, ctx, &fo, None, false)?;
            fprintf!(out, "\\psline{}", options);
            self.put_points(out, &pts, 0);
            if self.arrows == A_XFIG {
                if let Some(b) = s.back_arrow.as_ref() {
                    self.put_arrowhead_on_seg(out, ctx, last, first, b, s.thickness, s.pen_color)?;
                }
                if let Some(f) = s.for_arrow.as_ref() {
                    self.put_arrowhead_on_seg(out, ctx, first, last, f, s.thickness, s.pen_color)?;
                }
            }
            return Ok(());
        }

        // general case
        let closed = s.closed_spline();
        let flags = if closed {
            FO_LINE_STYLE | FO_PEN_COLOR | FO_FILL
        } else {
            self.arrow_flags(std_flags)
        };
        let fo = FoArgs { flags, ..base };
        let (options, _, _) = self.format_options(out, ctx, &fo, None, false)?;

        if s.int_spline() {
            // interpolating spline; just use pscurve
            if closed {
                fprintf!(out, "\\psecurve{}", options);
                self.put_points(out, &pts, 2);
            } else {
                fprintf!(out, "\\pscurve{}", options);
                self.put_points(out, &pts, 0);
            }
            return Ok(());
        }
        // approximating spline: a set of end-to-end joined bezier curves.
        // Control polygons are computed between midpoints of control
        // polygon segments; the inner vertices are 2/3 of the way to the
        // user control vertex.
        let c = |v: i32| self.cm(v);
        let half = |a: i32, b: i32| 1.0 / 2.0 * self.cm(a.wrapping_add(b));
        let mid = |r: Point, q: Point, p: Point| {
            (
                (half(r.x, q.x), half(r.y, q.y)),
                (c(q.x), c(q.y)),
                (half(q.x, p.x), half(q.y, p.y)),
            )
        };
        if closed {
            // the last two points are covered twice, which closes the curve
            let (Some(&r0), Some(&q0)) = (pts.get(n - 2), pts.get(n - 1)) else {
                return Ok(());
            };
            let (mut r, mut q) = (r0, q0);
            for &p in &pts {
                let (a, b, d) = mid(r, q, p);
                self.put_bezier(out, &options, a, b, d);
                r = q;
                q = p;
            }
        } else {
            if n < 4 {
                // C dereferences a null pointer after the first curve
                return Err("a spline with three points is not supported".to_string());
            }
            // first point and last points handled specially so curve hits them
            let (r, q, p) = (pts[0], pts[1], pts[2]);
            self.put_bezier(
                out,
                &options,
                (c(r.x), c(r.y)),
                (c(q.x), c(q.y)),
                (half(q.x, p.x), half(q.y, p.y)),
            );
            let (mut r, mut q) = (pts[1], pts[2]);
            let mut i = 3;
            while i + 1 < n {
                let p = pts[i];
                let (a, b, d) = mid(r, q, p);
                self.put_bezier(out, &options, a, b, d);
                r = q;
                q = p;
                i += 1;
            }
            let p = pts[i];
            // the second coordinate of the first point is not halved in C
            self.put_bezier(
                out,
                &options,
                (half(r.x, q.x), c(r.y.wrapping_add(q.y))),
                (c(q.x), c(q.y)),
                (c(p.x), c(p.y)),
            );
        }
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String> {
        print_comments(out, "% ", &t.comments, "");
        if self.verbose {
            fprintf!(
                out,
                "% text - {}\n",
                if t.special_text() { "special" } else { "no flags" }
            );
        }
        let mut t = t.clone();
        self.begin_set_font(out, ctx, &mut t);

        let p = self.to_bb(Point::new(t.base_x, t.base_y));
        let angle = if t.angle == 0.0 {
            String::new()
        } else {
            format!("{{{:.2}}}", Cf(t.angle * (180.0 / PI)))
        };
        let bytes = text_bytes(self.enc, c_string(&t.cstring));
        let body = if t.special_text() {
            check_tex_format(ctx, &bytes);
            bytes
        } else {
            fmt_text_str(ctx, &bytes, t.type_)?
        };
        fprintf!(
            out,
            "\\rput[{}]{}{}{{",
            ref_pt_str(t.type_)?,
            angle,
            pt(self.cm(p.x), self.cm(p.y))
        );
        out.bytes(&body);
        out.puts("}%\n");
        if self.font_handling == FH_SIZE_ONLY || self.font_handling == FH_FULL {
            out.puts("\\endgroup%\n");
        }
        Ok(())
    }

    fn end(&mut self, ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        if self.page_mode != PM_BARE {
            out.puts("\\end{pspicture}\n");
        }
        if self.page_mode >= PM_PAGE {
            out.puts("\\end{center}\n\\end{document}\n");
        }
        // issue_warnings
        if self.warn_flags != 0 {
            ctx.warn("IMPORTANT notes from pstricks driver:");
            for (i, w) in WARNINGS.iter().enumerate() {
                if self.warn_flags & (1 << i) != 0 {
                    ctx.warn(format!("  {w}"));
                }
            }
        }
        // the return value is determined by the warning mechanism
        Ok(self.rtn_val)
    }

    fn text_include(&self) -> bool {
        true
    }
}
