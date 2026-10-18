//! The driver interface (`struct driver`), fig2dev's global state (`Ctx`),
//! its general options (`get_args`), and its main flow after reading:
//! bounds, `-Z`, the metric correction, and the objects handed to the
//! driver deepest first (`gendev_objects`).
//
// Ported from fig2dev fig2dev.c, fig2dev.h and drivers.h (xfig licence, see
// LICENSE-xfig).

use crate::object::{Arc, Compound, Ellipse, Line, Spline, Text, NUMPATTERNS};
use crate::Out;

pub const DEFAULT_FONT_SIZE: f64 = 11.0;
pub const NUM_STD_COLS: usize = 32;
pub const MAX_USR_COLS: usize = 512;
/// Synchronised with `arrow_shapes[]` in bound.c.
pub const NUMARROWS: usize = 30;
pub const PROG: &str = "fig2dev";
/// fig2dev's version, which some drivers write into their output.
pub const PACKAGE_VERSION: &str = "3.2.9a";
/// At most this many `-D` ranges (`NUMDEPTHS`).
const NUMDEPTHS: usize = 100;

/// A colour with 16-bit channels (`RGB`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
}

/// A user colour from the file (`User_color`): number `c`, 8-bit channels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UserColor {
    pub c: i32,
    pub r: i32,
    pub g: i32,
    pub b: i32,
}

/// Paper sizes in points (`paperdef[]`).
pub const PAPERDEF: &[(&str, i32, i32)] = &[
    ("Letter", 612, 792),
    ("Legal", 612, 1008),
    ("Tabloid", 792, 1224),
    ("A", 612, 792),
    ("B", 792, 1224),
    ("C", 1224, 1584),
    ("D", 1584, 2448),
    ("E", 2448, 3168),
    ("A9", 105, 148),
    ("A8", 148, 210),
    ("A7", 210, 297),
    ("A6", 297, 420),
    ("A5", 420, 595),
    ("A4", 595, 842),
    ("A3", 842, 1190),
    ("A2", 1190, 1684),
    ("A1", 1684, 2383),
    ("A0", 2383, 3370),
    ("B10", 91, 127),
    ("B9", 127, 181),
    ("B8", 181, 258),
    ("B7", 258, 363),
    ("B6", 363, 516),
    ("B5", 516, 729),
    ("B4", 729, 1032),
    ("B3", 1032, 1460),
    ("B2", 1460, 2064),
    ("B1", 2064, 2920),
    ("B0", 2920, 4127),
];

/// Hex names of the 32 standard Fig colours (`Fig_color_names`).
pub const FIG_COLOR_NAMES: [&str; NUM_STD_COLS] = [
    "#000000", "#0000ff", "#00ff00", "#00ffff", "#ff0000", "#ff00ff", "#ffff00", "#ffffff", "#000090", "#0000b0",
    "#0000d0", "#87ceff", "#009000", "#00b000", "#00d000", "#009090", "#00b0b0", "#00d0d0", "#900000", "#b00000",
    "#d00000", "#900090", "#b000b0", "#d000d0", "#803000", "#a04000", "#c06000", "#ff8080", "#ffa0a0", "#ffc0c0",
    "#ffe0e0", "#ffd700",
];

/// `-D`: keep (`+`) or drop (`-`) the listed depths and ranges.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DepthOpts {
    pub op: char,
    /// `(d1, d2)`; `d2 < 0` for a single depth.
    pub ranges: Vec<(i32, i32)>,
    /// The values given by every `-D` so far (`depth_index`).
    pub index: usize,
}

/// fig2dev's global state (fig2dev.h, read.c): set by the options and the
/// reader, read and partly changed by the drivers.
#[derive(Clone, Debug)]
pub struct Ctx {
    /// Input and output file names (`from`, `to`); `None` for stdin/stdout.
    pub from: Option<String>,
    pub to: Option<String>,
    /// `-n`: the name some drivers put into their output.
    pub name: Option<String>,
    pub font_size: f64,
    pub mag: f64,
    pub fontmag: f64,
    /// The file's resolution (e.g. 1200).
    pub ppi: f64,
    /// Bounds of the figure, set before `start` (`llx`, `lly`, `urx`, `ury`).
    pub llx: i32,
    pub lly: i32,
    pub urx: i32,
    pub ury: i32,
    pub landscape: bool,
    pub center: bool,
    pub multi_page: bool,
    pub overlap: bool,
    pub orientspec: bool,
    pub centerspec: bool,
    pub magspec: bool,
    pub transspec: bool,
    pub paperspec: bool,
    pub boundingboxspec: bool,
    /// `-K`.
    pub adjust_boundingbox: bool,
    pub multispec: bool,
    pub metric: bool,
    /// `-N`.
    pub grayonly: bool,
    /// `-g` given.
    pub bgspec: bool,
    pub gif_transparent: String,
    pub papersize: String,
    pub boundingbox: String,
    /// The language (`-L`).
    pub lang: String,
    /// `-E`: `None`, or "UTF-8", "ISO-8859-1", "ISO-8859-2", or another name.
    pub input_encoding: Option<String>,
    /// Whether texts need a conversion to UTF-8 (`util::textconvert`).
    pub text_conv: crate::util::textconvert::ConvState,
    /// `-g`.
    pub background: Rgb,
    /// Grid spacings: as given (in `mult` units), then in Fig units.
    pub grid_minor_spacing: f32,
    pub grid_major_spacing: f32,
    /// Multiplier of the `-G` unit to inches.
    pub grid_mult: f32,
    /// Converts line thickness from screen resolution (set by the reader).
    pub thick_scale: f32,
    pub user_colors: Vec<UserColor>,
    /// `user_col_indx[i]`: the file's colour number of `user_colors[i]`
    /// (objects use `NUM_STD_COLS + i`, as in fig2dev).
    pub user_col_indx: Vec<i32>,
    pub v2_flag: bool,
    pub v21_flag: bool,
    pub v30_flag: bool,
    pub v32_flag: bool,
    pub pats_used: bool,
    pub pattern_used: [bool; NUMPATTERNS as usize],
    pub std_color_used: [bool; NUM_STD_COLS],
    pub arrows_used: bool,
    pub arrow_used: [bool; NUMARROWS],
    pub depths: Option<DepthOpts>,
    /// `-Z`: 0 none, 1 the larger dimension, 2 width, 3 height.
    pub maxdimspec: i32,
    pub max_dimension: f64,
    /// Lines fig2dev writes on stderr, in order.
    pub messages: Vec<String>,
    /// The directory relative picture names are resolved against when
    /// the figure doesn't come from a file. The reader already prefixes
    /// relative names with the directory of `from` (as read.c does), so
    /// leave this `None` when `from` is set.
    pub dir: Option<std::path::PathBuf>,
}

impl Default for Ctx {
    fn default() -> Ctx {
        Ctx {
            from: None,
            to: None,
            name: None,
            font_size: 0.0,
            mag: 1.0,
            fontmag: 1.0,
            ppi: 1200.0,
            llx: 0,
            lly: 0,
            urx: 0,
            ury: 0,
            landscape: false,
            center: false,
            multi_page: false,
            overlap: false,
            orientspec: false,
            centerspec: false,
            magspec: false,
            transspec: false,
            paperspec: false,
            boundingboxspec: false,
            adjust_boundingbox: false,
            multispec: false,
            metric: false,
            grayonly: false,
            bgspec: false,
            gif_transparent: String::new(),
            papersize: String::new(),
            boundingbox: String::new(),
            lang: String::new(),
            input_encoding: None,
            text_conv: Default::default(),
            background: Rgb::default(),
            grid_minor_spacing: 0.0,
            grid_major_spacing: 0.0,
            grid_mult: 1.0,
            thick_scale: 1.0,
            user_colors: Vec::new(),
            user_col_indx: Vec::new(),
            v2_flag: false,
            v21_flag: false,
            v30_flag: false,
            v32_flag: false,
            pats_used: false,
            pattern_used: [false; NUMPATTERNS as usize],
            std_color_used: [false; NUM_STD_COLS],
            arrows_used: false,
            arrow_used: [false; NUMARROWS],
            depths: None,
            maxdimspec: 0,
            max_dimension: 0.0,
            messages: Vec::new(),
            dir: None,
        }
    }
}

impl Ctx {
    /// A message on stderr (`fprintf(stderr, …)`, `put_msg`).
    pub fn warn(&mut self, msg: impl Into<String>) {
        self.messages.push(msg.into());
    }

    /// `depth_filter`: whether objects at `depth` are drawn.
    pub fn depth_filter(&self, depth: i32) -> bool {
        let Some(d) = &self.depths else {
            return true;
        };
        if d.ranges.is_empty() {
            return true;
        }
        for &(d1, d2) in &d.ranges {
            let hit = if d2 >= 0 {
                depth >= d1 && depth <= d2
            } else {
                d1 == depth
            };
            if hit {
                return d.op == '+';
            }
        }
        d.op == '-'
    }

    /// fig2dev's general options (the first switch of `get_args`). Returns
    /// `Ok(true)` when the option is fully handled, `Ok(false)` when it
    /// goes on to the driver too (`-G`, `-L` and the driver's own).
    /// `-h`, `-V` and `-L`'s language lookup are the caller's.
    pub fn general_option(&mut self, c: char, arg: Option<&str>) -> Result<bool, String> {
        let a = arg.unwrap_or("");
        match c {
            'D' => {
                self.depths.get_or_insert_with(DepthOpts::default).add(a)?;
                Ok(true)
            }
            'E' => {
                self.input_encoding = Some(
                    match a {
                        "0" | "utf8" | "UTF8" | "utf-8" => "UTF-8",
                        "1" => "ISO-8859-1",
                        "2" => "ISO-8859-2",
                        other => other,
                    }
                    .to_string(),
                );
                Ok(true)
            }
            'K' => {
                self.adjust_boundingbox = true;
                Ok(true)
            }
            'G' => {
                self.grid_option(a);
                Ok(false)
            }
            'L' => {
                self.lang = a.chars().take(11).collect();
                Ok(false)
            }
            'm' => {
                self.mag = atof(a);
                self.fontmag = self.mag;
                self.magspec = true;
                Ok(true)
            }
            's' => {
                self.font_size = atof(a);
                if self.font_size <= 0.0 || !self.font_size.is_finite() {
                    self.font_size = DEFAULT_FONT_SIZE;
                }
                Ok(true)
            }
            'Z' => {
                let height = a.ends_with('h') || a.starts_with(':');
                let num = a.strip_prefix(':').unwrap_or(a);
                if height {
                    self.maxdimspec = 3;
                } else {
                    self.maxdimspec = if a.ends_with('w') || a.ends_with(':') { 2 } else { 1 };
                }
                // strtod: no number is 0 without an error; infinity, NaN
                // and overflow (ERANGE) are errors. The message has no
                // newline in fig2dev.
                let t = num.trim_start();
                let t = t.strip_prefix(['+', '-']).unwrap_or(t);
                let inf_nan = t
                    .get(..3)
                    .is_some_and(|w| w.eq_ignore_ascii_case("inf") || w.eq_ignore_ascii_case("nan"));
                let v = atof(num);
                if inf_nan || !v.is_finite() {
                    self.warn(format!("Unable to parse dimension specification (-Z {a})"));
                    self.maxdimspec = 0;
                } else {
                    self.max_dimension = v;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// `-G minor[:major][unit]`.
    fn grid_option(&mut self, a: &str) {
        let usage = "Can't parse grid spec. Format is -G [minor_tick][:major_tick]unit e.g. -G .5:2cm or -G 1/16:1/2in\nAllowable units are: 'i', 'in', 'ft', 'feet', 'c', 'cm', 'mm', 'm'.  Ignoring grid.";
        let rest = if let Some((minor, major)) = a.split_once(':') {
            if !minor.is_empty() {
                match parse_gridspec(minor) {
                    Some((v, _)) => self.grid_minor_spacing = v,
                    None => self.warn(usage),
                }
            }
            match parse_gridspec(major) {
                Some((v, n)) => {
                    self.grid_major_spacing = v;
                    major.get(n..).unwrap_or("")
                }
                None => {
                    self.warn(usage);
                    ""
                }
            }
        } else {
            match parse_gridspec(a) {
                Some((v, n)) => {
                    self.grid_minor_spacing = v;
                    a.get(n..).unwrap_or("")
                }
                None => {
                    self.warn(usage);
                    ""
                }
            }
        };
        self.grid_mult = match rest {
            "" => 1.0,
            "i" | "in" | "inch" => 1.0,
            "f" | "ft" | "feet" => 12.0,
            "c" | "cm" => 1.0 / 2.54,
            "mm" => 1.0 / 25.4,
            "m" => 1.0 / 0.0254,
            _ => {
                self.warn(usage);
                self.grid_major_spacing = 0.0;
                self.grid_minor_spacing = 0.0;
                1.0
            }
        };
    }

    /// The RGB of colour number `c` (0..31 standard, then user colours),
    /// 8-bit channels; `None` for DEFAULT and unknown numbers.
    pub fn color_rgb(&self, c: i32) -> Option<(u8, u8, u8)> {
        if (0..NUM_STD_COLS as i32).contains(&c) {
            let hex = FIG_COLOR_NAMES.get(c as usize)?;
            let v = u32::from_str_radix(hex.get(1..)?, 16).ok()?;
            return Some(((v >> 16) as u8, (v >> 8) as u8, v as u8));
        }
        let u = self.user_colors.iter().find(|u| u.c == c)?;
        Some((
            u.r.clamp(0, 255) as u8,
            u.g.clamp(0, 255) as u8,
            u.b.clamp(0, 255) as u8,
        ))
    }
}

/// `atof`: the longest numeric prefix, 0 if none.
pub fn atof(s: &str) -> f64 {
    strtod(s).unwrap_or(0.0)
}

/// `strtod` without the end pointer: the longest prefix that is a number.
pub fn strtod(s: &str) -> Option<f64> {
    strtod_prefix(s).map(|(v, _)| v)
}

/// `strtod`: the value and the number of bytes it used.
pub fn strtod_prefix(s: &str) -> Option<(f64, usize)> {
    let t = s.trim_start();
    let skip = s.len() - t.len();
    let b = t.as_bytes();
    let mut i = 0;
    if matches!(b.first(), Some(b'+') | Some(b'-')) {
        i += 1;
    }
    let digits_start = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    if b.get(i) == Some(&b'.') {
        i += 1;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
    }
    if i == digits_start || (i == digits_start + 1 && b.get(digits_start) == Some(&b'.')) {
        return None;
    }
    if matches!(b.get(i), Some(b'e') | Some(b'E')) {
        let mut j = i + 1;
        if matches!(b.get(j), Some(b'+') | Some(b'-')) {
            j += 1;
        }
        if b.get(j).is_some_and(u8::is_ascii_digit) {
            while b.get(j).is_some_and(u8::is_ascii_digit) {
                j += 1;
            }
            i = j;
        }
    }
    t.get(..i)?.parse::<f64>().ok().map(|v| (v, skip + i))
}

/// `parse_gridspec`: `numer/denom` or a number; the spacing and the bytes
/// used. Zero or negative spacings are rejected.
fn parse_gridspec(s: &str) -> Option<(f32, usize)> {
    let (v, n) = strtod_prefix(s)?;
    if let Some(after) = s.get(n..).and_then(|r| r.strip_prefix('/')) {
        if let Some((d, m)) = strtod_prefix(after) {
            if v < 0.0 || d <= 0.0 {
                return None;
            }
            return Some(((v / d) as f32, n + 1 + m));
        }
    }
    (v > 0.0).then_some((v as f32, n))
}

const DEPTH_USAGE: &str = "fig2dev: help for -D option:
  -D +rangelist  means keep only depths in rangelist.
  -D -rangelist  means keep all depths but those in rangelist.
  Rangelist can be a list of numbers or ranges of numbers, e.g.:
    10,40,55,60:70,99";

/// `depth_option`: `+list` or `-list` of depths and `d1:d2` ranges.
pub fn depth_option(s: &str) -> Result<DepthOpts, String> {
    let mut d = DepthOpts::default();
    d.add(s)?;
    Ok(d)
}

impl DepthOpts {
    /// `depth_option` for one more `-D`: as in fig2dev, its values fill
    /// the slots from the first one again while the count (`depth_index`)
    /// goes on, and all of them together may not reach `NUMDEPTHS`.
    /// Something that isn't a number or a delimiter makes no progress, so
    /// fig2dev fills every slot with it and stops with "Too many -D
    /// values!".
    pub fn add(&mut self, s: &str) -> Result<(), String> {
        let mut chars = s.chars();
        self.op = match chars.next() {
            Some(c @ ('+' | '-')) => c,
            _ => return Err(DEPTH_USAGE.into()),
        };
        let mut rest = chars.as_str();
        let mut slot = 0;
        while self.index < NUMDEPTHS && !rest.is_empty() {
            rest = rest.strip_prefix(',').unwrap_or(rest);
            let (d1, n) = strtol(rest);
            if d1 < 0 {
                return Err(DEPTH_USAGE.into());
            }
            rest = rest.get(n..).unwrap_or("");
            let mut d2 = -1;
            if let Some(r) = rest.strip_prefix(':') {
                let (v, n) = strtol(r);
                if v < d1 {
                    return Err(DEPTH_USAGE.into());
                }
                d2 = v;
                rest = r.get(n..).unwrap_or("");
            } else if let Some(r) = rest.strip_prefix(',') {
                rest = r;
            }
            match self.ranges.get_mut(slot) {
                Some(r) => *r = (d1, d2),
                None => self.ranges.push((d1, d2)),
            }
            slot += 1;
            self.index += 1;
        }
        if self.index >= NUMDEPTHS {
            return Err(format!("{PROG}: Too many -D values!"));
        }
        Ok(())
    }
}

/// `strtol(s, &end, 10)`: the value (0 if none) and the bytes used.
pub fn strtol(s: &str) -> (i32, usize) {
    let b = s.as_bytes();
    let mut i = 0;
    while b.get(i).is_some_and(|c| c.is_ascii_whitespace()) {
        i += 1;
    }
    let start = i;
    if matches!(b.get(i), Some(b'+') | Some(b'-')) {
        i += 1;
    }
    let ds = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    if i == ds {
        return (0, 0);
    }
    let v = s.get(start..i).and_then(|t| t.parse::<i64>().ok()).unwrap_or(0);
    (v.clamp(i32::MIN as i64, i32::MAX as i64) as i32, i)
}

/// A fig2dev driver (`struct driver`). Each method gets fig2dev's global
/// state and the output; `Err` is fig2dev's `exit(1)` with that message.
pub trait Driver {
    /// A driver-specific option (`-L` and `-G` come here too).
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String>;
    /// The header.
    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String>;
    /// The grid (`-G`), spacings in Fig units; most drivers draw none.
    fn grid(&mut self, _ctx: &mut Ctx, _out: &mut Out, _major: f32, _minor: f32) -> Result<(), String> {
        Ok(())
    }
    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String>;
    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String>;
    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String>;
    fn spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String>;
    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String>;
    /// The trailer; returns the exit status.
    fn end(&mut self, ctx: &mut Ctx, out: &mut Out) -> Result<i32, String>;
    /// Whether texts count in the bounding box (`INCLUDE_TEXT`).
    fn text_include(&self) -> bool;
}

/// The result of a conversion: the output and fig2dev's exit status (its
/// messages are in `Ctx::messages`).
#[derive(Debug)]
pub struct Status {
    pub output: Vec<u8>,
    pub exit: i32,
}

/// One object of the figure, as the drivers receive it.
#[derive(Clone, Copy, Debug)]
pub enum Obj<'a> {
    Arc(&'a Arc),
    Ellipse(&'a Ellipse),
    Line(&'a Line),
    Spline(&'a Spline),
    Text(&'a Text),
}

/// Whether `com` holds any object (`compound_dump` finds one).
pub fn has_objects(com: &Compound) -> bool {
    !(com.arcs.is_empty()
        && com.ellipses.is_empty()
        && com.lines.is_empty()
        && com.splines.is_empty()
        && com.texts.is_empty())
        || com.compounds.iter().any(has_objects)
}

/// `compound_dump`: every object, compounds first, then arcs, ellipses,
/// lines, splines, texts, with its depth.
fn compound_dump<'a>(com: &'a Compound, out: &mut Vec<(i32, Obj<'a>)>) {
    for c in &com.compounds {
        compound_dump(c, out);
    }
    out.extend(com.arcs.iter().map(|a| (a.depth, Obj::Arc(a))));
    out.extend(com.ellipses.iter().map(|e| (e.depth, Obj::Ellipse(e))));
    out.extend(com.lines.iter().map(|l| (l.depth, Obj::Line(l))));
    out.extend(com.splines.iter().map(|s| (s.depth, Obj::Spline(s))));
    out.extend(com.texts.iter().map(|t| (t.depth, Obj::Text(t))));
}

/// Every object with its depth in the order the drivers draw them
/// (`gendev_objects`): deepest first; objects of one depth where the macOS
/// C library's `qsort` puts `compound_dump`'s order, as in the reference
/// fig2dev.
pub fn drawing_order(objects: &Compound) -> Vec<(i32, Obj<'_>)> {
    let mut recs = Vec::new();
    compound_dump(objects, &mut recs);
    // Deepest first, with the C library's qsort as macOS has it (not
    // stable), so that objects of one depth come in fig2dev's order.
    crate::drivers::shape::qsort(&mut recs, &mut |a: &(i32, Obj<'_>), b: &(i32, Obj<'_>)| {
        b.0.wrapping_sub(a.0)
    });
    recs
}

/// fig2dev's `main` after reading the figure, and `gendev_objects`: the
/// bounds, `-Z`, the metric correction, then the driver's header, grid,
/// objects deepest first and trailer.
pub fn convert(ctx: &mut Ctx, dev: &mut dyn Driver, objects: &Compound) -> Result<Status, String> {
    // Grid spacing in Fig units.
    ctx.grid_minor_spacing *= ctx.grid_mult * ctx.ppi as f32;
    ctx.grid_major_spacing *= ctx.grid_mult * ctx.ppi as f32;

    let (llx, lly, urx, ury) = crate::bound::compound_bound(ctx, objects, dev.text_include());
    ctx.llx = llx;
    ctx.lly = lly;
    ctx.urx = urx;
    ctx.ury = ury;
    // Give the box width and height (a figure of only LaTeX special text
    // may have none).
    if ctx.urx.saturating_sub(ctx.llx) == 0 {
        ctx.urx = ctx.llx.saturating_add(100);
    }
    if ctx.ury.saturating_sub(ctx.lly) == 0 {
        ctx.ury = ctx.lly.saturating_add(100);
    }
    if ctx.maxdimspec != 0 {
        let figdim = match ctx.maxdimspec {
            2 => ctx.urx as f64 - ctx.llx as f64,
            3 => ctx.ury as f64 - ctx.lly as f64,
            _ => (ctx.urx as f64 - ctx.llx as f64).max(ctx.ury as f64 - ctx.lly as f64),
        };
        let mut max_dimension = ctx.max_dimension;
        if ctx.metric {
            max_dimension /= 2.54;
        }
        if figdim != 0.0 {
            ctx.mag = max_dimension * ctx.ppi / figdim;
        }
    }
    // Metric figures: 450 Fig units per cm, but 472.44 really; font sizes
    // are in points and stay.
    if ctx.metric {
        ctx.mag *= 80.0 / 76.2;
    }

    let recs = drawing_order(objects);
    let mut out = Out::new();
    if recs.is_empty() {
        ctx.warn("fig2dev: No objects in Fig file");
        return Ok(Status {
            output: out.buf,
            exit: -1,
        });
    }
    dev.start(ctx, &mut out, objects)?;
    let (major, minor) = (ctx.grid_major_spacing, ctx.grid_minor_spacing);
    dev.grid(ctx, &mut out, major, minor)?;
    for (depth, obj) in &recs {
        if !ctx.depth_filter(*depth) {
            continue;
        }
        match obj {
            Obj::Arc(a) => dev.arc(ctx, &mut out, a)?,
            Obj::Ellipse(e) => dev.ellipse(ctx, &mut out, e)?,
            Obj::Line(l) => dev.line(ctx, &mut out, l)?,
            Obj::Spline(s) => dev.spline(ctx, &mut out, s)?,
            Obj::Text(t) => dev.text(ctx, &mut out, t)?,
        }
    }
    let exit = dev.end(ctx, &mut out)?;
    Ok(Status { output: out.buf, exit })
}

/// Print the comments (`print_comments`): `"{pre} {comment} {post}"` per
/// comment.
pub fn print_comments(out: &mut Out, pre: &str, comments: &[crate::object::Comment], post: &str) {
    for c in comments {
        crate::fprintf!(out, "{pre} ");
        out.bytes(&c.raw);
        crate::fprintf!(out, " {post}\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_options_as_fig2dev() {
        let d = depth_option("+10,40:50,99").unwrap();
        assert_eq!(d.ranges, vec![(10, -1), (40, 50), (99, -1)]);
        let ctx = Ctx {
            depths: Some(d),
            ..Ctx::default()
        };
        assert!(ctx.depth_filter(45) && ctx.depth_filter(99) && !ctx.depth_filter(51));
        let ctx = Ctx {
            depths: Some(depth_option("-0:10").unwrap()),
            ..Ctx::default()
        };
        assert!(!ctx.depth_filter(5) && ctx.depth_filter(11));
        assert!(depth_option("x").is_err());
        assert!(depth_option("+5:2").is_err());
        // Junk makes no progress: fig2dev fills every slot.
        for junk in ["+a", "+10x", "+5;6"] {
            assert_eq!(
                depth_option(junk).unwrap_err(),
                "fig2dev: Too many -D values!",
                "{junk}"
            );
        }
        let ninety_nine = format!("+{}", vec!["1"; 99].join(","));
        assert!(depth_option(&ninety_nine).is_ok());
        let hundred = format!("+{}", vec!["1"; 100].join(","));
        assert!(depth_option(&hundred).is_err(), "NUMDEPTHS values are too many");
        // "-D +" sets no filter.
        let ctx = Ctx {
            depths: Some(depth_option("+").unwrap()),
            ..Ctx::default()
        };
        assert!(ctx.depth_filter(7));
        // A second -D refills the slots from the first one.
        let mut ctx = Ctx::default();
        ctx.general_option('D', Some("+1,2")).unwrap();
        ctx.general_option('D', Some("-3")).unwrap();
        let d = ctx.depths.clone().unwrap();
        assert_eq!((d.op, d.ranges, d.index), ('-', vec![(3, -1), (2, -1)], 3));
    }

    #[test]
    fn grid_and_numbers() {
        let mut ctx = Ctx::default();
        ctx.general_option('G', Some(".25:1cm")).unwrap();
        assert_eq!((ctx.grid_minor_spacing, ctx.grid_major_spacing), (0.25, 1.0));
        assert!((ctx.grid_mult - 1.0 / 2.54).abs() < 1e-6);
        let mut ctx = Ctx::default();
        ctx.general_option('G', Some("1/16:1/2in")).unwrap();
        assert_eq!((ctx.grid_minor_spacing, ctx.grid_major_spacing), (1.0 / 16.0, 0.5));
        assert_eq!(strtod_prefix("2.5e1x"), Some((25.0, 5)));
        assert_eq!(strtod("."), None);
        assert_eq!(strtol("-12:3"), (-12, 3));
        let mut ctx = Ctx::default();
        ctx.general_option('Z', Some("4w")).unwrap();
        assert_eq!((ctx.maxdimspec, ctx.max_dimension), (2, 4.0));
        ctx.general_option('Z', Some(":3")).unwrap();
        assert_eq!((ctx.maxdimspec, ctx.max_dimension), (3, 3.0));
        // strtod: a prefix counts, no number is 0 without a message.
        ctx.general_option('Z', Some("4in")).unwrap();
        assert_eq!((ctx.maxdimspec, ctx.max_dimension), (1, 4.0));
        ctx.general_option('Z', Some("abc")).unwrap();
        assert_eq!((ctx.maxdimspec, ctx.max_dimension), (1, 0.0));
        assert!(ctx.messages.is_empty());
        for bad in ["inf", "-NaN", "1e999"] {
            ctx.general_option('Z', Some(bad)).unwrap();
            assert_eq!(ctx.maxdimspec, 0, "{bad}");
        }
        assert_eq!(ctx.messages.len(), 3);
        ctx.general_option('s', Some("0")).unwrap();
        assert_eq!(ctx.font_size, DEFAULT_FONT_SIZE);
        ctx.general_option('s', Some("12pt")).unwrap();
        assert_eq!(ctx.font_size, 12.0);
        ctx.general_option('m', Some("x")).unwrap();
        assert_eq!((ctx.mag, ctx.magspec), (0.0, true));
    }
}
