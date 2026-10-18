//! fig2dev's reader (`read.c`, `read1_3.c`): Fig 1.3 to 3.2 into the
//! object model, with fig2dev's checks, repairs and messages.
//!
//! [`readfp_fig`] reads a figure from bytes and sets what `read.c` sets in
//! fig2dev's globals ([`Ctx`]): the resolution, orientation, centering,
//! units, paper size, magnification, multiple-page flag, transparent GIF
//! colour, the user colours, the format flags and the colours, patterns
//! and arrowheads used. Messages fig2dev prints on stderr go to
//! `ctx.messages`, one entry per line; where fig2dev gives up (a bad file,
//! or a call of `exit()`), the reader returns fig2dev's status.
//!
//! Differences from the C, all for robustness: integers that overflow
//! saturate; a `nan` or infinite value where `read.c` compares a double
//! against a range (ppi, arc centres, angles, text sizes, style values,
//! arrowheads, spline controls) counts as out of range; compounds nest at
//! most [`MAX_NESTING`] deep; text strings are decoded to Unicode
//! ([`Ctx::input_encoding`], else the file's `#encoding: UTF-8` line,
//! else UTF-8 if the whole file is valid UTF-8 and ISO-8859-1 otherwise).
//! Fig 3.2 splines become polylines here, as in fig2dev
//! (`create_line_with_spline`).
//
// Ported from fig2dev read.c, read1_3.c, free.c and messages.c (xfig
// licence, see LICENSE-xfig).

mod read1_3;
mod scan;

use crate::gendev::{UserColor, DEFAULT_FONT_SIZE, FIG_COLOR_NAMES, MAX_USR_COLS, NUMARROWS, NUM_STD_COLS};
use crate::object::*;
use crate::Ctx;
pub use scan::Encoding;
use scan::{atof, atoi, fmt_g, sscanf, starts_ci, Scan};

/// fig2dev's version as `read.c` compares it with the file's.
const PACKAGE_VERSION: &[u8] = b"3.2";
/// At most this many comment lines are kept for one object (`MAXCOMMENTS`).
pub const MAXCOMMENTS: usize = 100;
/// Compounds nest at most this deep (fig2dev has no limit; deeper nesting
/// would overflow the stack here and in the drivers).
pub const MAX_NESTING: usize = 200;
/// Largest figure file read, in bytes (RustFig's limit; fig2dev has none).
pub const MAX_FILE_BYTES: usize = 512 << 20;
/// The longest paper size name (`PAPERSZ_LEN` − 1).
const PAPERSZ_MAX: usize = 15;
/// Points of a line before Fig 3.0, which ends at `9999` (`MAX_V2POINTS`).
const MAX_V2POINTS: i64 = 1_000_000;

/// The result of the reader: fig2dev's status on failure (-1 bad file,
/// -2 empty file, -3 input error).
type R<T> = Result<T, i32>;

/// `read_fig`: read the Fig file at `path`. Statuses as [`readfp_fig`];
/// -3 if the file cannot be read (fig2dev then reports `errno`).
pub fn read_fig(ctx: &mut Ctx, path: &std::path::Path) -> Result<Compound, i32> {
    let Ok(meta) = std::fs::metadata(path) else {
        return Err(-3);
    };
    if !meta.is_file() {
        ctx.warn(format!("Not a regular file: {}", path.display()));
        return Err(-1);
    }
    let Ok(src) = std::fs::read(path) else {
        return Err(-3);
    };
    readfp_fig(ctx, &src)
}

/// `readfp_fig`: read a Fig file from `src`. On failure the messages are
/// in `ctx.messages` and the error is fig2dev's status (-1 bad file, -2
/// empty file, -3 input error).
pub fn readfp_fig(ctx: &mut Ctx, src: &[u8]) -> Result<Compound, i32> {
    ctx.user_colors.clear();
    ctx.user_col_indx.clear();
    ctx.pats_used = false;
    ctx.pattern_used = [false; NUMPATTERNS as usize];
    ctx.std_color_used = [false; NUM_STD_COLS];
    ctx.arrows_used = false;
    ctx.arrow_used = [false; NUMARROWS];

    let Some(&first) = src.first() else {
        return Err(-2);
    };
    let mut r = Reader {
        ctx,
        inp: Scan::new(src),
        line_no: 0,
        comments: Vec::new(),
        num_usr_cols: 0,
        enc: Encoding::Latin1,
        file_is_utf8: std::str::from_utf8(src).is_ok(),
        nesting: 0,
    };
    if first == b'#' {
        r.read_objects()
    } else {
        r.enc = Encoding::resolve(r.ctx.input_encoding.as_deref(), r.file_is_utf8);
        r.read_1_3_objects()
    }
}

/// The reader's state: fig2dev's globals and `read.c`'s statics.
struct Reader<'a, 'c> {
    ctx: &'c mut Ctx,
    inp: Scan<'a>,
    line_no: i32,
    /// Comments saved for the next object.
    comments: Vec<Comment>,
    /// The user colours defined, counting one past the maximum to report
    /// it once (`num_usr_cols`).
    num_usr_cols: usize,
    enc: Encoding,
    file_is_utf8: bool,
    /// How deep compounds nest at the moment.
    nesting: usize,
}

/// The names of closed line types, from `T_BOX` (`obj_name[]`).
fn obj_name(type_: i32) -> &'static str {
    match type_ {
        T_BOX => "rectangle",
        T_POLYGON => "polygon",
        T_ARC_BOX => "rectangle with rounded corners",
        _ => "picture box",
    }
}

/// `COMMON_PROPERTIES`, true if invalid; a non-finite style value too.
fn common_invalid(style: i32, thickness: i32, depth: i32, fill_style: i32, style_val: f64) -> bool {
    !(SOLID_LINE..=DASH_3_DOTS_LINE).contains(&style)
        || thickness < 0
        || !(0..=999).contains(&depth)
        || !(UNFILLED..NUMSHADES + NUMTINTS + NUMPATTERNS).contains(&fill_style)
        || style_val.is_nan()
        || style_val < 0.0
        || style_val.is_infinite()
}

/// Whether the four points from `p` describe a rectangle with its edges
/// parallel to the axes (`IS_RECTANGLE`).
fn is_rectangle(p: &[Point]) -> bool {
    let (Some(p0), Some(p1), Some(p2), Some(p3)) = (p.first(), p.get(1), p.get(2), p.get(3)) else {
        return false;
    };
    (p0.x == p1.x && p2.x == p3.x && p0.y == p3.y && p1.y == p2.y)
        || (p0.y == p1.y && p2.y == p3.y && p0.x == p3.x && p1.x == p2.x)
}

/// Count consecutive backslashes backwards from `start` (`backslash_count`).
fn backslash_count(s: &[u8], start: usize) -> usize {
    s.get(..=start)
        .map_or(0, |h| h.iter().rev().take_while(|&&c| c == b'\\').count())
}

/// Where the text in `s` ends (`find_end`): at `\001` (not escaped) from
/// Fig 3.0, at a ^A before.
fn find_end(s: &[u8], v30: bool) -> Option<usize> {
    if !v30 {
        return s.iter().position(|&c| c == 1);
    }
    const ENDMARK: &[u8] = b"\\001";
    let find = |from: usize| -> Option<usize> {
        s.get(from..)?
            .windows(ENDMARK.len())
            .position(|w| w == ENDMARK)
            .map(|i| i + from)
    };
    let mut end = find(0);
    while let Some(e) = end {
        if !backslash_count(s, e).is_multiple_of(2) {
            break;
        }
        end = find(e + 3);
    }
    end
}

impl<'a> Reader<'a, '_> {
    /// `put_msg`: a message, one entry per line.
    fn msg(&mut self, m: impl AsRef<str>) {
        for l in m.as_ref().split('\n') {
            self.ctx.warn(l);
        }
    }

    fn incomplete(&mut self, what: &str) {
        let line_no = self.line_no;
        self.msg(format!("Incomplete {what} object at line {line_no}."));
    }

    fn invalid(&mut self, what: &str) {
        let line_no = self.line_no;
        self.msg(format!("Invalid {what} object at line {line_no}."));
    }

    fn decode(&self, b: &[u8]) -> String {
        self.enc.decode(b)
    }

    /// `round(THICK_SCALE)`.
    fn thick_factor(&self) -> i32 {
        crate::round(f64::from(self.ctx.thick_scale))
    }

    /// `FILL_CONVERT`: the fill style of older formats.
    fn fill_convert(&self, f: i32) -> i32 {
        let f = i64::from(f);
        let v = if self.ctx.v2_flag || f < i64::from(WHITE_FILL) {
            if self.ctx.v30_flag {
                f
            } else {
                f - 1
            }
        } else {
            20 - (f - 1) * 5
        };
        v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }

    /// `save_comment`: keep a comment line (without `#` and one blank)
    /// for the next object.
    fn save_comment(&mut self, line: &[u8]) {
        if self.comments.len() >= MAXCOMMENTS {
            return;
        }
        let body = line.get(1..).unwrap_or_default();
        let body = body.strip_prefix(b" ").unwrap_or(body);
        let text = self.decode(body);
        self.comments.push(Comment {
            raw: body.to_vec(),
            text,
        });
    }

    /// `attach_comments`.
    fn attach_comments(&mut self) -> Vec<Comment> {
        std::mem::take(&mut self.comments)
    }

    /// A NUL in a line: fig2dev exits.
    fn nul_in_line(&mut self) -> i32 {
        let line_no = self.line_no;
        self.msg(format!("ASCII NUL ('\\0') in line {line_no}."));
        -1
    }

    /// `get_line`: the next line that is neither empty nor a comment,
    /// without its newline (and a carriage return before it); comments
    /// are saved. `None` at the end of the file.
    fn get_line(&mut self) -> R<Option<&'a [u8]>> {
        while let Some(raw) = self.inp.getline() {
            self.line_no = self.line_no.saturating_add(1);
            if raw.first() == Some(&b'\n') || raw == b"\r\n" {
                continue;
            }
            if raw.contains(&0) {
                return Err(self.nul_in_line());
            }
            let line = match raw.strip_suffix(b"\n") {
                Some(l) => l.strip_suffix(b"\r").unwrap_or(l),
                None => raw,
            };
            if line.first() == Some(&b'#') {
                self.save_comment(line);
                continue;
            }
            return Ok(Some(line));
        }
        Ok(None)
    }

    /// A header line, or the "File is truncated" message.
    fn header_line(&mut self, what: &str) -> R<&'a [u8]> {
        match self.get_line()? {
            Some(l) => Ok(l),
            None => {
                self.msg(format!("File is truncated at {what} specification."));
                Err(-1)
            }
        }
    }

    /// `count_lines_correctly`: skip blanks and tabs between points,
    /// counting a newline.
    fn count_lines_correctly(&mut self) {
        loop {
            let mut c = self.inp.getc();
            if c == Some(b'\n') {
                self.line_no = self.line_no.saturating_add(1);
                c = self.inp.getc();
            }
            match c {
                Some(b' ') | Some(b'\t') => continue,
                Some(_) => {
                    self.inp.unget();
                    return;
                }
                None => return,
            }
        }
    }

    /// `fscanf(fp, "%d%d")`.
    fn read_xy(&mut self) -> Option<(i32, i32)> {
        let x = self.inp.int()?;
        let y = self.inp.int()?;
        Some((x, y))
    }

    /// `read_objects`: Fig 1.4 to 3.2.
    fn read_objects(&mut self) -> R<Compound> {
        let mut obj = Compound::default();

        // The first line, by fgets(): a 16-byte buffer.
        let Some(buf) = self.inp.fgets16() else {
            self.msg("Could not read input file.");
            return Err(-1);
        };
        let slen = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let buf = buf.get(..slen).unwrap_or_default();
        let ends_nl = buf.last() == Some(&b'\n');
        if slen < 16 && !ends_nl {
            self.msg("ASCII NUL ('\\0') character within the first line.");
            return Err(-1);
        } else if !ends_nl {
            // Seek to the end of the first line (where NULs are tolerated).
            self.inp.skip_line();
        }
        if !buf.starts_with(b"#FIG ") {
            self.msg("Incorrect format string in first line of input file.");
            return Err(-1);
        }
        let v3 = buf.starts_with(b"#FIG 3");
        self.ctx.v2_flag = buf.starts_with(b"#FIG 2") || v3;
        self.ctx.v21_flag = buf.starts_with(b"#FIG 2.1") || v3;
        self.ctx.v30_flag = v3 || buf.starts_with(b"#FIG 2.2");
        self.ctx.v32_flag = buf.starts_with(b"#FIG 3.2");
        let version = buf.get(5..).unwrap_or_default();
        // strncmp(&buf[5], PACKAGE_VERSION, 3) > 0
        let newer = (0..3)
            .map(|i| {
                (
                    version.get(i).copied().unwrap_or(0),
                    PACKAGE_VERSION.get(i).copied().unwrap_or(0),
                )
            })
            .find(|(a, b)| a != b)
            .is_some_and(|(a, b)| a > b);
        if newer {
            let v = self.decode(version);
            self.msg(format!(
                "Fig file format ({v}) newer than this version of fig2dev ({}), exiting",
                crate::gendev::PACKAGE_VERSION
            ));
            return Err(-1);
        }
        let c = &*self.ctx;
        if !(c.v2_flag || c.v21_flag || c.v30_flag || c.v32_flag) && !buf.starts_with(b"#FIG 1.4") {
            let v = self.decode(version);
            self.msg(format!("Cannot determine fig file format from string '{v}'."));
            return Err(-1);
        }

        // An optional "#encoding: UTF-8" line.
        match self.inp.getc() {
            Some(b'#') => {
                let Some(buf) = self.inp.fgets16() else {
                    self.msg("Could not read input file.");
                    return Err(-1);
                };
                let slen = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                let buf = buf.get(..slen).unwrap_or_default();
                if self.ctx.input_encoding.is_none() && buf == b"encoding: UTF-8\n" {
                    self.ctx.input_encoding = Some("UTF-8".into());
                } else if buf.last() != Some(&b'\n') {
                    self.inp.skip_line();
                }
            }
            Some(_) => self.inp.unget(),
            None => {
                self.msg("Could not read input file.");
                return Err(-1);
            }
        }
        self.enc = Encoding::resolve(self.ctx.input_encoding.as_deref(), self.file_is_utf8);

        self.line_no = 1;
        let mut gif_colnum = 0;
        if self.ctx.v30_flag {
            let line = self.header_line("landscape/portrait")?;
            if !self.ctx.orientspec {
                self.ctx.landscape = starts_ci(line, b"land");
            }
            let mut line = self.header_line("metric/inches or centering")?;
            if starts_ci(line, b"center") || starts_ci(line, b"flush") {
                if !self.ctx.centerspec {
                    self.ctx.center = !starts_ci(line, b"flush");
                }
                line = self.header_line("metric/inches")?;
            }
            self.ctx.metric = starts_ci(line, b"metric");

            if self.ctx.v32_flag {
                let line = self.header_line("paper size")?;
                if !self.ctx.paperspec {
                    let name = line.split(|&c| c == b' ').next().unwrap_or_default();
                    if name.len() > PAPERSZ_MAX {
                        let (line_no, name) = (self.line_no, self.decode(name));
                        self.msg(format!("Invalid paper size specification at line {line_no}: {name}"));
                        return Err(-1);
                    }
                    self.ctx.papersize = self.decode(name);
                }

                let line = self.header_line("magnification")?;
                if !self.ctx.magspec {
                    let mut mag = atof(line) / 100.0;
                    if !(mag > 0.0 && mag.is_finite()) {
                        mag = 1.0;
                    }
                    self.ctx.mag = mag;
                    self.ctx.fontmag = mag;
                }

                let line = self.header_line("multiple page")?;
                if !self.ctx.multispec {
                    self.ctx.multi_page = starts_ci(line, b"multiple");
                }

                let line = self.header_line("transparent color")?;
                if !self.ctx.transspec {
                    gif_colnum = atoi(line);
                    if gif_colnum < -3 {
                        self.msg("Invalid color number for transparent color.");
                        return Err(-1);
                    }
                    if let Some(name) = usize::try_from(gif_colnum).ok().and_then(|i| FIG_COLOR_NAMES.get(i)) {
                        self.ctx.gif_transparent = (*name).to_string();
                    }
                }
            }
        } else {
            self.ctx.metric = false;
        }
        if !self.ctx.v32_flag && !self.ctx.paperspec {
            self.ctx.papersize = if self.ctx.metric { "A4" } else { "Letter" }.into();
        }

        // The resolution and coordinate system (the latter is not used).
        let Some(line) = self.get_line()? else {
            self.msg("File is truncated at resolution specification.");
            return Err(-1);
        };
        let f = sscanf(line, "fd");
        if f.n() != 2 {
            let line_no = self.line_no;
            self.msg(format!("Incomplete resolution information at line {line_no}."));
            return Err(-1);
        }
        let ppi = f.f(0);
        if !(ppi > 1.0 && ppi.is_finite()) {
            let line_no = self.line_no;
            self.msg(format!(
                "Invalid resolution information ({}) at line {line_no}.",
                fmt_g(ppi)
            ));
            return Err(-1);
        }
        self.ctx.ppi = ppi;
        self.ctx.thick_scale = (ppi / 80.0) as f32;

        obj.comments = self.attach_comments();

        let mut objects = false;
        while let Some(line) = self.get_line()? {
            let Some(object) = Scan::new(line).int() else {
                let line_no = self.line_no;
                self.msg(format!("Incorrect format at line {line_no}."));
                return Err(-1);
            };
            if object == OBJ_COLOR_DEF {
                if objects {
                    let line_no = self.line_no;
                    self.msg(format!(
                        "Color definitions must come before other objects (line {line_no})."
                    ));
                    return Err(-1);
                }
                self.read_colordef(line);
                continue;
            }
            if !self.read_object(object, line, &mut obj)? {
                let line_no = self.line_no;
                self.msg(format!("Incorrect object code at line {line_no}."));
                return Err(-1);
            }
            objects = true;
        }

        // A user colour as the transparent GIF colour.
        if gif_colnum >= NUM_STD_COLS as i32 {
            if let Some(i) = self.ctx.user_col_indx.iter().position(|&c| c == gif_colnum) {
                if let Some(u) = self.ctx.user_colors.get(i) {
                    self.ctx.gif_transparent = format!("#{:2x}{:2x}{:2x}", u.r, u.g, u.b);
                }
            }
        }
        Ok(obj)
    }

    /// Read an object of code `object` (its first line is `line`) into
    /// `com`. `Ok(false)` for an unknown code.
    fn read_object(&mut self, object: i32, line: &'a [u8], com: &mut Compound) -> R<bool> {
        match object {
            OBJ_POLYLINE => {
                let l = self.read_lineobject(line)?;
                com.lines.push(l);
            }
            OBJ_SPLINE => match self.read_splineobject(line)? {
                SplineOrLine::Line(l) => com.lines.push(l),
                SplineOrLine::Spline(s) => com.splines.push(s),
            },
            OBJ_ELLIPSE => {
                let e = self.read_ellipseobject(line)?;
                com.ellipses.push(e);
            }
            OBJ_ARC => {
                let a = self.read_arcobject(line)?;
                com.arcs.push(a);
            }
            OBJ_TEXT => {
                let t = self.read_textobject(line)?;
                com.texts.push(t);
            }
            OBJ_COMPOUND => {
                let c = self.read_compoundobject(line)?;
                com.compounds.push(c);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// `read_colordef`: `0 number #rrggbb`.
    fn read_colordef(&mut self, line: &[u8]) {
        let line_no = self.line_no;
        if self.num_usr_cols >= MAX_USR_COLS {
            if self.num_usr_cols == MAX_USR_COLS {
                self.msg(format!(
                    "Maximum number of color definitions ({MAX_USR_COLS}) exceeded at line {line_no}."
                ));
                self.num_usr_cols += 1;
            }
            return;
        }
        let mut s = Scan::new(line);
        let parsed = (|| {
            s.int()?;
            let c = s.int()?;
            s.skip_ws();
            if !s.lit(b'#') {
                return None;
            }
            Some((c, s.hex2()?, s.hex2()?, s.hex2()?))
        })();
        let Some((c, r, g, b)) = parsed else {
            let l = self.decode(line);
            self.msg(format!("Invalid color definition at line {line_no}: {l}."));
            return;
        };
        let lo = NUM_STD_COLS as i32;
        let hi = (NUM_STD_COLS + MAX_USR_COLS) as i32;
        let byte = |v: i64| (0..256).contains(&v);
        if (lo..hi).contains(&c) && byte(r) && byte(g) && byte(b) {
            let idx = self.ctx.user_colors.len();
            self.ctx.user_colors.push(UserColor {
                c: NUM_STD_COLS as i32 + idx as i32,
                r: r as i32,
                g: g as i32,
                b: b as i32,
            });
            self.ctx.user_col_indx.push(c);
            self.num_usr_cols += 1;
        } else if !(lo..hi).contains(&c) {
            self.msg(format!(
                "User color number at line {line_no} out of range ({c}), should be between {lo} and {}.",
                hi - 1
            ));
        } else {
            let l = self.decode(line);
            self.msg(format!(
                "Invalid color definition at line {line_no}: {l}, color values must be between 0 through 255."
            ));
        }
    }

    /// `fix_and_note_color`: note a standard colour as used, turn a user
    /// colour into `NUM_STD_COLS` + its index in `ctx.user_colors`, and an
    /// unknown colour into the default.
    fn fix_and_note_color(&mut self, color: &mut i32) {
        let line_no = self.line_no;
        if *color < DEFAULT {
            self.msg(format!(
                "Invalid color number {} at line {line_no}, using default color.",
                *color
            ));
            *color = DEFAULT;
            return;
        }
        if *color < NUM_STD_COLS as i32 {
            if let Some(u) = usize::try_from(*color)
                .ok()
                .and_then(|i| self.ctx.std_color_used.get_mut(i))
            {
                *u = true;
            }
            return;
        }
        if let Some(i) = self.ctx.user_col_indx.iter().position(|&c| c == *color) {
            *color = NUM_STD_COLS as i32 + i as i32;
            return;
        }
        self.msg(format!(
            "Cannot locate user color {}, using default color at line {line_no}.",
            *color
        ));
        *color = DEFAULT;
    }

    /// `note_fill`.
    fn note_fill(&mut self, fill: i32, color: &mut i32) {
        if fill != UNFILLED {
            self.fix_and_note_color(color);
            if fill >= NUMSHADES + NUMTINTS {
                if let Some(p) = usize::try_from(fill - NUMSHADES - NUMTINTS)
                    .ok()
                    .and_then(|i| self.ctx.pattern_used.get_mut(i))
                {
                    *p = true;
                }
                self.ctx.pats_used = true;
            }
        }
    }

    /// `make_arrow`: `None` (with a message) for an invalid arrowhead.
    fn make_arrow(&mut self, type_: i32, style: i32, thickness: f64, wid: f64, ht: f64) -> Option<Arrow> {
        let line_no = self.line_no;
        let narrows = NUMARROWS as i32;
        if !(0..=1).contains(&style)
            || type_ < 0
            || type_ > narrows
            || (type_ + 1) * 2 > narrows
            || !(thickness.is_finite() && wid.is_finite() && ht.is_finite())
        {
            self.msg(format!("Invalid arrow at line {line_no}."));
            return None;
        }
        if wid < 1e-2 && type_ != 5 && type_ != 6 {
            self.msg(format!("Zero-width arrow at line {line_no} ignored."));
            return None;
        }
        self.ctx.arrows_used = true;
        if let Some(u) = usize::try_from(2 * type_ + style)
            .ok()
            .and_then(|i| self.ctx.arrow_used.get_mut(i))
        {
            *u = true;
        }
        Some(Arrow {
            type_,
            style,
            thickness: thickness * f64::from(self.ctx.thick_scale),
            wid,
            ht,
        })
    }

    /// Read an arrowhead line of a `what` object.
    fn read_arrow(&mut self, what: &str) -> R<Option<Arrow>> {
        let Some(line) = self.get_line()? else {
            self.incomplete(what);
            return Err(-1);
        };
        let f = sscanf(line, "ddfff");
        if f.n() != 5 {
            self.incomplete(what);
            return Err(-1);
        }
        Ok(self.make_arrow(f.i(0), f.i(1), f.f(2), f.f(3), f.f(4)))
    }

    /// `sanitize_arc`: reject an arc of radius zero; move the last point
    /// onto the circle through the first.
    fn sanitize_arc(&mut self, a: &mut Arc) -> R<()> {
        let line_no = self.line_no;
        let p0 = a.point[0];
        let (cx, cy) = (a.center.x, a.center.y);
        let (x0, y0) = (f64::from(p0.x), f64::from(p0.y));
        if x0 - 1.0 < cx && x0 + 1.0 > cx && y0 - 1.0 < cy && y0 + 1.0 > cy {
            self.msg(format!(
                "Invalid arc object at line {line_no}: arc radius smaller than one Fig unit."
            ));
            return Err(-1);
        }
        let p2 = a.point[2];
        if p2.x == crate::round(cx) && p2.y == crate::round(cy) {
            self.msg(format!(
                "Invalid arc object at line {line_no}: the end point co-incides with the center of the arc."
            ));
            return Err(-1);
        }
        let (x2, y2) = (f64::from(p2.x), f64::from(p2.y));
        let r = (x0 - cx).hypot(y0 - cy);
        let l = (x2 - cx).hypot(y2 - cy);
        if l < r - 1.0 || l > r + 1.0 {
            let fac = r / l;
            a.point[2] = Point::new(crate::round(cx + fac * (x2 - cx)), crate::round(cy + fac * (y2 - cy)));
        }
        Ok(())
    }

    /// `read_arcobject`.
    fn read_arcobject(&mut self, line: &[u8]) -> R<Arc> {
        let mut a = Arc::default();
        let (fa, ba);
        if self.ctx.v30_flag {
            let f = sscanf(line, "*ddddddddfddddffdddddd");
            if f.n() != 21 {
                self.incomplete("arc");
                return Err(-1);
            }
            (a.type_, a.style, a.thickness, a.pen_color) = (f.i(0), f.i(1), f.i(2), f.i(3));
            (a.fill_color, a.depth, a.pen, a.fill_style) = (f.i(4), f.i(5), f.i(6), f.i(7));
            (a.style_val, a.cap_style, a.direction) = (f.f(8), f.i(9), f.i(10));
            (fa, ba) = (f.i(11), f.i(12));
            a.center = DPoint { x: f.f(13), y: f.f(14) };
            for (k, p) in a.point.iter_mut().enumerate() {
                *p = Point::new(f.i(15 + 2 * k), f.i(16 + 2 * k));
            }
        } else {
            let f = sscanf(line, "*dddddddfdddffdddddd");
            if f.n() != 19 {
                self.incomplete("arc");
                return Err(-1);
            }
            (a.type_, a.style, a.thickness, a.pen_color) = (f.i(0), f.i(1), f.i(2), f.i(3));
            (a.depth, a.pen, a.fill_style) = (f.i(4), f.i(5), f.i(6));
            (a.style_val, a.direction) = (f.f(7), f.i(8));
            (fa, ba) = (f.i(9), f.i(10));
            a.center = DPoint { x: f.f(11), y: f.f(12) };
            for (k, p) in a.point.iter_mut().enumerate() {
                *p = Point::new(f.i(13 + 2 * k), f.i(14 + 2 * k));
            }
            a.fill_color = a.pen_color;
            a.cap_style = 0;
        }
        a.thickness = a.thickness.saturating_mul(self.thick_factor());
        a.fill_style = self.fill_convert(a.fill_style);
        let coord = |v: f64| v.is_finite() && v >= f64::from(i32::MIN) && v <= f64::from(i32::MAX);
        if !(T_OPEN_ARC..=T_PIE_WEDGE_ARC).contains(&a.type_)
            || common_invalid(a.style, a.thickness, a.depth, a.fill_style, a.style_val)
            || !(0..=2).contains(&a.cap_style)
            || !coord(a.center.x)
            || !coord(a.center.y)
            || (a.direction != 0 && a.direction != 1)
        {
            self.invalid("arc");
            return Err(-1);
        }
        self.sanitize_arc(&mut a)?;
        let mut pen = a.pen_color;
        self.fix_and_note_color(&mut pen);
        a.pen_color = pen;
        let mut fill = a.fill_color;
        self.note_fill(a.fill_style, &mut fill);
        a.fill_color = fill;
        if fa != 0 {
            a.for_arrow = self.read_arrow("arc")?;
        }
        if ba != 0 {
            a.back_arrow = self.read_arrow("arc")?;
        }
        a.comments = self.attach_comments();
        Ok(a)
    }

    /// `read_compoundobject`: objects up to `-6`, or the end of the file.
    fn read_compoundobject(&mut self, line: &[u8]) -> R<Compound> {
        let mut com = Compound {
            comments: self.attach_comments(),
            ..Compound::default()
        };
        let f = sscanf(line, "*dddd");
        if f.n() != 4 {
            self.incomplete("compound");
            return Err(-1);
        }
        com.nwcorner = Point::new(f.i(0), f.i(1));
        com.secorner = Point::new(f.i(2), f.i(3));
        if self.nesting >= MAX_NESTING {
            let line_no = self.line_no;
            self.msg(format!(
                "Compound objects nested deeper than {MAX_NESTING} levels at line {line_no}."
            ));
            return Err(-1);
        }
        self.nesting += 1;
        let r = self.read_compound_body(&mut com);
        self.nesting -= 1;
        r.map(|()| com)
    }

    fn read_compound_body(&mut self, com: &mut Compound) -> R<()> {
        while let Some(line) = self.get_line()? {
            let Some(object) = Scan::new(line).int() else {
                self.incomplete("compound");
                return Err(-1);
            };
            if object == OBJ_END_COMPOUND {
                return Ok(());
            }
            if !self.read_object(object, line, com)? {
                let line_no = self.line_no;
                self.msg(format!("Wrong object code at line {line_no}"));
                return Err(-1);
            }
        }
        Ok(())
    }

    /// `read_ellipseobject`.
    fn read_ellipseobject(&mut self, line: &[u8]) -> R<Ellipse> {
        let mut e = Ellipse::default();
        let (f, i) = if self.ctx.v30_flag {
            let f = sscanf(line, "*ddddddddfdfdddddddd");
            if f.n() != 19 {
                self.incomplete("ellipse");
                return Err(-1);
            }
            (e.type_, e.style, e.thickness, e.pen_color) = (f.i(0), f.i(1), f.i(2), f.i(3));
            (e.fill_color, e.depth, e.pen, e.fill_style) = (f.i(4), f.i(5), f.i(6), f.i(7));
            (e.style_val, e.direction, e.angle) = (f.f(8), f.i(9), f.f(10));
            (f, 11)
        } else {
            let f = sscanf(line, "*dddddddfdfdddddddd");
            if f.n() != 18 {
                self.incomplete("ellipse");
                return Err(-1);
            }
            (e.type_, e.style, e.thickness, e.pen_color) = (f.i(0), f.i(1), f.i(2), f.i(3));
            (e.depth, e.pen, e.fill_style) = (f.i(4), f.i(5), f.i(6));
            (e.style_val, e.direction, e.angle) = (f.f(7), f.i(8), f.f(9));
            e.fill_color = e.pen_color;
            (f, 10)
        };
        e.center = Point::new(f.i(i), f.i(i + 1));
        e.radiuses = Point::new(f.i(i + 2), f.i(i + 3));
        e.start = Point::new(f.i(i + 4), f.i(i + 5));
        e.end = Point::new(f.i(i + 6), f.i(i + 7));

        let mut pen = e.pen_color;
        self.fix_and_note_color(&mut pen);
        e.pen_color = pen;
        e.thickness = e.thickness.saturating_mul(self.thick_factor());
        e.fill_style = self.fill_convert(e.fill_style);
        e.radiuses.x = e.radiuses.x.saturating_abs();
        e.radiuses.y = e.radiuses.y.saturating_abs();
        if ellipse_invalid(&e) {
            self.invalid("ellipse");
            return Err(-1);
        }
        let mut fill = e.fill_color;
        self.note_fill(e.fill_style, &mut fill);
        e.fill_color = fill;
        e.comments = self.attach_comments();
        Ok(e)
    }

    /// `sanitize_lineobject`: close open boxes and polygons, reject
    /// malformed arc-boxes and picture boxes, turn degenerate boxes and
    /// polygons into polylines or polygons, unfill short polylines and
    /// take the arrowheads off a single point. `num_points` is the C
    /// counter (which the messages print).
    fn sanitize_lineobject(&mut self, l: &mut Line, mut num_points: i64) -> R<()> {
        let line_no = self.line_no;
        if matches!(l.type_, T_BOX | T_POLYGON | T_ARC_BOX | T_PIC_BOX) {
            let name = obj_name(l.type_);
            if let (Some(&first), Some(&last)) = (l.points.first(), l.points.last()) {
                if first != last {
                    self.msg(format!("An open {name} at line {line_no} - close it."));
                    l.points.insert(0, last);
                    num_points += 1;
                }
            }
            if l.type_ == T_ARC_BOX || l.type_ == T_PIC_BOX {
                if num_points != 5 {
                    self.msg(format!(
                        "A malformed {name} at line {line_no} - it has {} corners.",
                        num_points - 1
                    ));
                    return Err(-1);
                }
                if !is_rectangle(&l.points) {
                    self.msg(format!("A distorted or inclined {name} at line {line_no}."));
                    return Err(-1);
                }
            } else if num_points < 4 {
                self.msg(format!(
                    "A {name} with {num_points} points at line {line_no} - convert to a polyline."
                ));
                l.type_ = T_POLYLINE;
                if num_points == 2 {
                    l.points.truncate(1);
                } else if num_points == 3 {
                    l.points.truncate(2);
                }
            } else if l.type_ == T_BOX {
                if num_points != 5 {
                    self.msg(format!(
                        "A rectangle with {} corners at line {line_no} - convert to a polygon.",
                        num_points - 1
                    ));
                    l.type_ = T_POLYGON;
                } else if !is_rectangle(&l.points) {
                    self.msg(format!(
                        "A distorted retangle at line {line_no} - convert to a polygon."
                    ));
                    l.type_ = T_POLYGON;
                }
            }
        }
        if l.type_ == T_POLYLINE && l.points.len() < 3 {
            l.fill_style = UNFILLED;
            if l.points.len() < 2 {
                if l.for_arrow.take().is_some() {
                    self.msg("A single point with a forward arrow - remove the arrow.");
                }
                if l.back_arrow.take().is_some() {
                    self.msg("A single point with a backward arrow - remove the arrow.");
                }
            }
        }
        Ok(())
    }

    /// The name of a picture file: relative to the Fig file's directory
    /// when that is given as a path (fig2dev prepends the path of `from`).
    /// Returns the name decoded to Unicode and as bytes: the file's bytes,
    /// or the Unicode name when `conv_filename` converts from `-E`.
    fn picture_path(&self, file: &[u8]) -> (String, Vec<u8>) {
        let name = self.decode(file);
        let converted = self
            .ctx
            .input_encoding
            .as_deref()
            .is_some_and(|n| !n.eq_ignore_ascii_case("UTF-8") && Encoding::from_name(n).is_some());
        let bytes: Vec<u8> = if converted {
            name.as_bytes().to_vec()
        } else {
            file.to_vec()
        };
        let Some(from) = self.ctx.from.as_deref() else {
            return (name, bytes);
        };
        let sep = |c: char| c == '/' || (cfg!(windows) && c == '\\');
        match from.rfind(sep) {
            Some(i) if !name.starts_with('/') && !(cfg!(windows) && is_windows_absolute(&name)) => {
                let prefix = from.get(..=i).unwrap_or_default();
                let mut raw = prefix.as_bytes().to_vec();
                raw.extend_from_slice(&bytes);
                (format!("{prefix}{name}"), raw)
            }
            _ => (name, bytes),
        }
    }

    /// `read_lineobject`.
    fn read_lineobject(&mut self, line: &[u8]) -> R<Line> {
        let mut l = Line::default();
        let mut sc = Scan::new(line);
        let _ = sc.int(); // the object code, already known
        l.type_ = sc.int().unwrap_or(0);
        let (v2, v21, v30) = (self.ctx.v2_flag, self.ctx.v21_flag, self.ctx.v30_flag);
        let radius_flag = v30 || v21 || (v2 && l.type_ == T_ARC_BOX);
        let (fa, ba);
        let mut npts = 0;
        let ok = if radius_flag {
            if v30 {
                let f = sscanf(line, "*ddddddddfdddddd");
                (l.type_, l.style, l.thickness, l.pen_color) = (f.i(0), f.i(1), f.i(2), f.i(3));
                (l.fill_color, l.depth, l.pen, l.fill_style) = (f.i(4), f.i(5), f.i(6), f.i(7));
                (l.style_val, l.join_style, l.cap_style, l.radius) = (f.f(8), f.i(9), f.i(10), f.i(11));
                (fa, ba, npts) = (f.i(12), f.i(13), f.i(14));
                f.n() == 15
            } else {
                let f = sscanf(line, "*dddddddfddd");
                (l.type_, l.style, l.thickness, l.pen_color) = (f.i(0), f.i(1), f.i(2), f.i(3));
                (l.depth, l.pen, l.fill_style, l.style_val) = (f.i(4), f.i(5), f.i(6), f.f(7));
                (l.radius, fa, ba) = (f.i(8), f.i(9), f.i(10));
                l.fill_color = l.pen_color;
                f.n() == 11
            }
        } else {
            // The old format keeps the corner radius of arc-boxes in pen.
            let f = sscanf(line, "*dddddddfdd");
            (l.type_, l.style, l.thickness, l.pen_color) = (f.i(0), f.i(1), f.i(2), f.i(3));
            (l.depth, l.pen, l.fill_style, l.style_val) = (f.i(4), f.i(5), f.i(6), f.f(7));
            (fa, ba) = (f.i(8), f.i(9));
            l.fill_color = l.pen_color;
            if l.type_ == T_ARC_BOX {
                l.radius = l.pen;
                l.pen = 0;
            } else {
                l.radius = 0;
            }
            f.n() == 10
        };
        if !ok {
            self.incomplete("line");
            return Err(-1);
        }
        let tf = self.thick_factor();
        l.radius = l.radius.saturating_mul(tf);
        l.thickness = l.thickness.saturating_mul(tf);
        l.fill_style = self.fill_convert(l.fill_style);
        if !(T_POLYLINE..=T_PIC_BOX).contains(&l.type_)
            || common_invalid(l.style, l.thickness, l.depth, l.fill_style, l.style_val)
            || !(0..=2).contains(&l.cap_style)
            || !(0..=2).contains(&l.join_style)
            || (l.type_ == T_ARC_BOX && l.radius < 0)
        {
            self.invalid("line");
            return Err(-1);
        }
        let mut fill = l.fill_color;
        self.note_fill(l.fill_style, &mut fill);
        l.fill_color = fill;
        let mut pen = l.pen_color;
        self.fix_and_note_color(&mut pen);
        l.pen_color = pen;
        if fa != 0 {
            l.for_arrow = self.read_arrow("line")?;
        }
        if ba != 0 {
            l.back_arrow = self.read_arrow("line")?;
        }
        if l.type_ == T_PIC_BOX {
            let Some(pline) = self.get_line()? else {
                self.incomplete("picture");
                return Err(-1);
            };
            let mut sc = Scan::new(pline);
            let Some(flipped) = sc.int() else {
                self.incomplete("picture");
                return Err(-1);
            };
            sc.skip_ws();
            let file = sc.rest();
            let (file, file_raw) = self.picture_path(file);
            l.pic = Some(Box::new(Pic {
                flipped,
                file,
                file_raw,
                ..Pic::default()
            }));
        }

        // The points.
        self.line_no = self.line_no.saturating_add(1);
        let Some((x, y)) = self.read_xy() else {
            self.incomplete("line");
            return Err(-1);
        };
        l.points.push(Point::new(x, y));
        let mut num_ident: i64 = 0;
        let mut npts = if v30 { i64::from(npts) } else { MAX_V2POINTS };
        let total = npts;
        npts -= 1;
        while npts > 0 {
            self.count_lines_correctly();
            let Some((x, y)) = self.read_xy() else {
                self.incomplete("line");
                return Err(-1);
            };
            if !v30 && x == 9999 {
                break;
            }
            let p = Point::new(x, y);
            if l.points.last() == Some(&p) {
                num_ident += 1;
            } else {
                l.points.push(p);
            }
            npts -= 1;
        }
        let num_points = if v30 {
            total - num_ident
        } else {
            MAX_V2POINTS - npts - num_ident
        };

        self.sanitize_lineobject(&mut l, num_points)?;
        l.comments = self.attach_comments();
        self.inp.skip_line();
        Ok(l)
    }

    /// `read_splineobject`. Fig 3.2 splines come back as lines.
    fn read_splineobject(&mut self, line: &[u8]) -> R<SplineOrLine> {
        let mut s = Spline::default();
        let (v30, v32) = (self.ctx.v30_flag, self.ctx.v32_flag);
        let (fa, ba);
        let mut npts = 0;
        if v30 {
            let f = sscanf(line, "*ddddddddfdddd");
            if f.n() != 13 {
                self.incomplete("spline");
                return Err(-1);
            }
            (s.type_, s.style, s.thickness, s.pen_color) = (f.i(0), f.i(1), f.i(2), f.i(3));
            (s.fill_color, s.depth, s.pen, s.fill_style) = (f.i(4), f.i(5), f.i(6), f.i(7));
            (s.style_val, s.cap_style) = (f.f(8), f.i(9));
            (fa, ba, npts) = (f.i(10), f.i(11), f.i(12));
        } else {
            let f = sscanf(line, "*dddddddfdd");
            if f.n() != 10 {
                self.incomplete("spline");
                return Err(-1);
            }
            (s.type_, s.style, s.thickness, s.pen_color) = (f.i(0), f.i(1), f.i(2), f.i(3));
            (s.depth, s.pen, s.fill_style, s.style_val) = (f.i(4), f.i(5), f.i(6), f.f(7));
            (fa, ba) = (f.i(8), f.i(9));
            s.fill_color = s.pen_color;
            s.cap_style = 0;
        }
        s.thickness = s.thickness.saturating_mul(self.thick_factor());
        s.fill_style = self.fill_convert(s.fill_style);
        if !(T_OPEN_APPROX..=T_CLOSED_XSPLINE).contains(&s.type_)
            || common_invalid(s.style, s.thickness, s.depth, s.fill_style, s.style_val)
            || !(0..=2).contains(&s.cap_style)
        {
            self.invalid("spline");
            return Err(-1);
        }
        let mut fill = s.fill_color;
        self.note_fill(s.fill_style, &mut fill);
        s.fill_color = fill;
        let mut pen = s.pen_color;
        self.fix_and_note_color(&mut pen);
        s.pen_color = pen;
        if fa != 0 {
            s.for_arrow = self.read_arrow("spline")?;
        }
        if ba != 0 {
            s.back_arrow = self.read_arrow("spline")?;
        }

        // The points.
        self.line_no = self.line_no.saturating_add(1);
        let Some((x, y)) = self.read_xy() else {
            self.incomplete("spline");
            return Err(-1);
        };
        s.points.push(Point::new(x, y));
        let mut npts = if v30 { i64::from(npts) } else { MAX_V2POINTS };
        if npts < 2 {
            self.incomplete("spline");
            return Err(-1);
        }
        npts -= 1;
        while npts > 0 {
            self.count_lines_correctly();
            let Some((x, y)) = self.read_xy() else {
                self.incomplete("spline");
                return Err(-1);
            };
            if !v30 && x == 9999 {
                break;
            }
            s.points.push(Point::new(x, y));
            npts -= 1;
        }
        if s.points.len() < 2 {
            self.incomplete("spline");
            return Err(-1);
        }
        s.comments = self.attach_comments();

        if v32 {
            // An X-spline: a shape factor per point.
            let mut controls = Vec::with_capacity(s.points.len());
            for _ in 0..s.points.len() {
                self.count_lines_correctly();
                let Some(v) = self.inp.float() else {
                    self.incomplete("spline");
                    return Err(-1);
                };
                controls.push(Control {
                    s: v,
                    ..Control::default()
                });
            }
            s.controls = controls;
            // Splines of only co-incident points (graphviz 2.43 wrote such
            // splines) are dropped.
            if s.points.iter().all(|p| Some(p) == s.points.first()) {
                let line_no = self.line_no;
                self.msg(format!(
                    "Invalid spline with only co-incident points at or before line {line_no}."
                ));
                return Err(-1);
            }
            if s.controls.iter().any(|c| !c.s.is_finite()) {
                self.incomplete("spline");
                return Err(-1);
            }
            let line = spline_to_line(self.ctx, &s);
            self.inp.skip_line();
            return match line {
                Some(l) => Ok(SplineOrLine::Line(l)),
                None => {
                    let line_no = self.line_no;
                    self.msg(format!("Unable to convert spline to line at line {line_no}."));
                    Err(-1)
                }
            };
        }

        if s.approx_spline() {
            self.inp.skip_line();
            return Ok(SplineOrLine::Spline(s));
        }
        // The control points of older interpolated splines.
        let mut controls = Vec::with_capacity(s.points.len());
        for _ in 0..s.points.len() {
            self.count_lines_correctly();
            let vals = (|| {
                Some((
                    self.inp.float()?,
                    self.inp.float()?,
                    self.inp.float()?,
                    self.inp.float()?,
                ))
            })();
            let Some((lx, ly, rx, ry)) = vals else {
                self.incomplete("spline");
                return Err(-1);
            };
            let in_range = |v: f64| v.is_finite() && v >= f64::from(i32::MIN) && v <= f64::from(i32::MAX);
            if !(in_range(lx) && in_range(ly) && in_range(rx) && in_range(ry)) {
                let line_no = self.line_no;
                self.msg(format!("Spline control points out of range at line {line_no}."));
                return Err(-1);
            }
            controls.push(Control { lx, ly, rx, ry, s: 0.0 });
        }
        s.controls = controls;
        self.inp.skip_line();
        Ok(SplineOrLine::Spline(s))
    }

    /// `read_textobject`.
    fn read_textobject(&mut self, line: &'a [u8]) -> R<Text> {
        let mut t = Text::default();
        let mut sc = Scan::new(line);
        let f = sc.fields("*dddddffdffdd");
        if f.n() != 12 {
            self.incomplete("text");
            return Err(-1);
        }
        (t.type_, t.color, t.depth, t.pen, t.font) = (f.i(0), f.i(1), f.i(2), f.i(3), f.i(4));
        (t.size, t.angle, t.flags, t.height, t.length) = (f.f(5), f.f(6), f.i(7), f.f(8), f.f(9));
        (t.base_x, t.base_y) = (f.i(10), f.i(11));
        sc.skip_ws();
        let start = sc.rest();
        let v30 = self.ctx.v30_flag;

        let mut text: Vec<u8> = match find_end(start, v30) {
            Some(end) => start.get(..end).unwrap_or_default().to_vec(),
            None => {
                // A text over several lines: the newline goes back in.
                let mut next = start.to_vec();
                next.push(b'\n');
                while let Some(raw) = self.inp.getline() {
                    self.line_no = self.line_no.saturating_add(1);
                    if raw.contains(&0) {
                        return Err(self.nul_in_line());
                    }
                    if let Some(end) = find_end(raw, v30) {
                        next.extend_from_slice(raw.get(..end).unwrap_or_default());
                        break;
                    }
                    next.extend_from_slice(raw);
                }
                next
            }
        };

        // Convert \ooo and \c.
        if v30 && text.contains(&b'\\') {
            let c = text;
            let at = |i: usize| c.get(i).copied().unwrap_or(0);
            let mut out = Vec::with_capacity(c.len());
            let mut l = 0;
            while l < c.len() {
                if at(l) == b'\\' && at(l + 1) != 0 {
                    if at(l + 1).is_ascii_digit() && at(l + 2) != 0 && at(l + 3) != 0 {
                        // sscanf("%3hho"): up to three octal digits.
                        let digits: Vec<u8> = (1..=3)
                            .map(|k| at(l + k))
                            .take_while(|d| (b'0'..=b'7').contains(d))
                            .collect();
                        if digits.is_empty() {
                            let line_no = self.line_no;
                            self.msg(format!("Error in parsing text string on line {line_no}"));
                            return Err(-1);
                        }
                        let num = digits.iter().fold(0u32, |v, d| v * 8 + u32::from(d - b'0'));
                        out.push((num & 0xff) as u8);
                        l += 3;
                    } else {
                        l += 1;
                        out.push(at(l));
                    }
                } else {
                    out.push(at(l));
                }
                l += 1;
            }
            text = out;
        }
        // A \000 ends the C string.
        if let Some(nul) = text.iter().position(|&b| b == 0) {
            text.truncate(nul);
        }
        t.cstring = self.decode(&text);
        t.raw = text;

        if self.ctx.font_size != 0.0 {
            // Scale length and height to the requested font size.
            if t.size > 0.0 {
                t.length = t.length * self.ctx.font_size / t.size;
                t.height = t.height * self.ctx.font_size / t.size;
            }
            t.size = self.ctx.font_size;
        }
        if t.size <= 0.0 {
            t.size = DEFAULT_FONT_SIZE;
        }
        let (v2, v21) = (self.ctx.v2_flag, self.ctx.v21_flag);
        let base = |flags: i32| if flags != DEFAULT { flags } else { 0 };
        if !v21 && (t.font == 0 || t.font == DEFAULT) {
            t.flags = base(t.flags) | SPECIAL_TEXT;
        }
        if v2 && !v21 && !t.special_text() {
            t.flags = base(t.flags) | PSFONT_TEXT;
        }
        if text_invalid(&t) {
            self.invalid("text");
            return Err(-1);
        }
        let mut color = t.color;
        self.fix_and_note_color(&mut color);
        t.color = color;
        t.comments = self.attach_comments();
        Ok(t)
    }
}

/// A spline as read: Fig 3.2 splines are converted to lines.
enum SplineOrLine {
    Spline(Spline),
    Line(Line),
}

/// `create_line_with_spline` (trans_spline.c).
fn spline_to_line(ctx: &mut Ctx, s: &Spline) -> Option<Line> {
    crate::trans_spline::create_line_with_spline(ctx, s)
}

/// Whether a picture name is absolute on Windows (`C:\…`, `\\server`).
fn is_windows_absolute(name: &str) -> bool {
    let b = name.as_bytes();
    name.starts_with('\\') || (b.get(1) == Some(&b':') && b.first().is_some_and(u8::is_ascii_alphabetic))
}

/// `INVALID_ELLIPSE` (a non-finite angle too).
fn ellipse_invalid(e: &Ellipse) -> bool {
    !(T_ELLIPSE_BY_RAD..=T_CIRCLE_BY_DIA).contains(&e.type_)
        || common_invalid(e.style, e.thickness, e.depth, e.fill_style, e.style_val)
        || (e.direction != 1 && e.direction != 0)
        || e.radiuses.x == 0
        || e.radiuses.y == 0
        || !(-7.0..=7.0).contains(&e.angle)
}

/// `INVALID_TEXT` (non-finite sizes and angles too).
fn text_invalid(t: &Text) -> bool {
    !(T_LEFT_JUSTIFIED..=T_RIGHT_JUSTIFIED).contains(&t.type_)
        || t.font < DEFAULT
        || t.font > MAX_PSFONT
        || t.size.is_nan()
        || t.size < 0.0
        || t.size.is_infinite()
        || t.flags < DEFAULT
        || t.flags >= 2 * HIDDEN_TEXT
        || !(t.height >= 0.0 && t.height.is_finite())
        || !(t.length >= 0.0 && t.length.is_finite())
        || !(-7.0..=7.0).contains(&t.angle)
}

#[cfg(test)]
mod tests;
