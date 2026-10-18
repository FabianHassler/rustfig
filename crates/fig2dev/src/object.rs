//! fig2dev's object model: the figure as `read.c` hands it to the drivers.
//! Coordinates are in the file's resolution (`Ctx::ppi`, usually 1200 per
//! inch), y grows downwards. C's linked lists are `Vec`s in file order.
//
// Ported from fig2dev object.h (xfig licence, see LICENSE-xfig).

pub const BLACK_COLOR: i32 = 0;
pub const WHITE_COLOR: i32 = 7;
pub const DEFAULT: i32 = -1;
pub const MAXCOLORMAPSIZE: usize = 256;

pub const NUMSHADES: i32 = 21;
pub const NUMTINTS: i32 = 20;
pub const NUMPATTERNS: i32 = 22;
/// NUMSHADES + NUMTINTS − 1.
pub const NUMFILLS: i32 = 40;

pub const UNFILLED: i32 = -1;
pub const WHITE_FILL: i32 = 0;
pub const BLACK_FILL: i32 = 20;

pub const SOLID_LINE: i32 = 0;
pub const DASH_LINE: i32 = 1;
pub const DOTTED_LINE: i32 = 2;
pub const DASH_DOT_LINE: i32 = 3;
pub const DASH_2_DOTS_LINE: i32 = 4;
pub const DASH_3_DOTS_LINE: i32 = 5;

pub const CLOSED_PATH: i32 = 0;
pub const OPEN_PATH: i32 = 1;

/// Object codes in the Fig file.
pub const OBJ_COLOR_DEF: i32 = 0;
pub const OBJ_ELLIPSE: i32 = 1;
pub const OBJ_POLYLINE: i32 = 2;
pub const OBJ_SPLINE: i32 = 3;
pub const OBJ_TEXT: i32 = 4;
pub const OBJ_ARC: i32 = 5;
pub const OBJ_COMPOUND: i32 = 6;
pub const OBJ_END_COMPOUND: i32 = -OBJ_COMPOUND;

/// A point (`F_point`, `F_pos`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Point {
        Point { x, y }
    }
}

/// An arrowhead (`F_arrow`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Arrow {
    pub type_: i32,
    pub style: i32,
    pub thickness: f64,
    pub wid: f64,
    pub ht: f64,
}

/// Ellipse types.
pub const T_ELLIPSE_BY_RAD: i32 = 1;
pub const T_ELLIPSE_BY_DIA: i32 = 2;
pub const T_CIRCLE_BY_RAD: i32 = 3;
pub const T_CIRCLE_BY_DIA: i32 = 4;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ellipse {
    pub type_: i32,
    pub style: i32,
    pub thickness: i32,
    pub pen_color: i32,
    pub fill_color: i32,
    pub depth: i32,
    pub pen: i32,
    pub fill_style: i32,
    pub style_val: f64,
    pub direction: i32,
    /// Radians.
    pub angle: f64,
    pub center: Point,
    pub radiuses: Point,
    pub start: Point,
    pub end: Point,
    pub comments: Vec<Comment>,
}

/// Arc types.
pub const T_OPEN_ARC: i32 = 1;
pub const T_PIE_WEDGE_ARC: i32 = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Arc {
    pub type_: i32,
    pub style: i32,
    pub thickness: i32,
    pub pen_color: i32,
    pub fill_color: i32,
    pub depth: i32,
    pub pen: i32,
    pub fill_style: i32,
    pub style_val: f64,
    pub for_arrow: Option<Arrow>,
    pub back_arrow: Option<Arrow>,
    pub cap_style: i32,
    /// 0 clockwise, 1 counterclockwise.
    pub direction: i32,
    pub center: DPoint,
    pub point: [Point; 3],
    pub comments: Vec<Comment>,
}

/// Line types.
pub const T_POLYLINE: i32 = 1;
pub const T_BOX: i32 = 2;
pub const T_POLYGON: i32 = 3;
pub const T_ARC_BOX: i32 = 4;
pub const T_PIC_BOX: i32 = 5;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line {
    pub type_: i32,
    pub style: i32,
    pub thickness: i32,
    pub pen_color: i32,
    pub fill_color: i32,
    pub depth: i32,
    pub pen: i32,
    pub fill_style: i32,
    pub style_val: f64,
    pub for_arrow: Option<Arrow>,
    pub back_arrow: Option<Arrow>,
    pub cap_style: i32,
    pub join_style: i32,
    /// Corner radius of a `T_ARC_BOX`.
    pub radius: i32,
    pub points: Vec<Point>,
    pub pic: Option<Box<Pic>>,
    pub comments: Vec<Comment>,
}

impl Line {
    /// `num_points` of the C structure.
    pub fn num_points(&self) -> usize {
        self.points.len()
    }
}

/// Picture subtypes (`F_pic.subtype`).
pub const P_EPS: i32 = 0;
pub const P_XBM: i32 = 1;
pub const P_XPM: i32 = 2;
pub const P_GIF: i32 = 3;
pub const P_JPEG: i32 = 4;
pub const P_PCX: i32 = 5;
pub const P_PPM: i32 = 6;
pub const P_TIF: i32 = 7;
pub const P_PNG: i32 = 8;

/// An imported picture (`F_pic`). The drivers that embed pictures read
/// the file themselves (`readpics.c`); `read.c` fills only `file` and
/// `flipped`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pic {
    pub subtype: i32,
    /// The picture's file name (relative to the Fig file's directory),
    /// decoded to Unicode.
    pub file: String,
    /// The same name as C holds it: the raw bytes of the Fig file, unless
    /// `-E` names an encoding other than UTF-8 (`conv_filename` converts
    /// then). Empty: `file`'s bytes.
    pub file_raw: Vec<u8>,
    pub flipped: i32,
    pub bitmap: Vec<u8>,
    /// Colour map, `cmap[RED|GREEN|BLUE][i]`, `numcols` entries used.
    pub cmap: Vec<[u8; 3]>,
    pub numcols: i32,
    pub num_transp: i32,
    pub transp_col: [u8; 3],
    pub hw_ratio: f32,
    pub bit_size: Point,
}

/// Text justification.
pub const T_LEFT_JUSTIFIED: i32 = 0;
pub const T_CENTER_JUSTIFIED: i32 = 1;
pub const T_RIGHT_JUSTIFIED: i32 = 2;

/// LaTeX fonts (without `PSFONT_TEXT`).
pub const DEFAULT_FONT: i32 = 0;
pub const ROMAN_FONT: i32 = 1;
pub const BOLD_FONT: i32 = 2;
pub const ITALIC_FONT: i32 = 3;
pub const MODERN_FONT: i32 = 4;
pub const TYPEWRITER_FONT: i32 = 5;
pub const MAX_FONT: i32 = 5;
pub const MAX_PSFONT: i32 = 34;

/// Text flags.
pub const RIGID_TEXT: i32 = 1;
pub const SPECIAL_TEXT: i32 = 2;
pub const PSFONT_TEXT: i32 = 4;
pub const HIDDEN_TEXT: i32 = 8;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Text {
    pub type_: i32,
    pub font: i32,
    /// Points.
    pub size: f64,
    pub color: i32,
    pub depth: i32,
    /// Radians.
    pub angle: f64,
    pub flags: i32,
    /// Fig units, as written by xfig.
    pub height: f64,
    pub length: f64,
    pub base_x: i32,
    pub base_y: i32,
    pub pen: i32,
    /// The string decoded to Unicode by the reader, for the consumers that
    /// need characters (bounds, the scene and its EPS/PDF/SVG writers, the
    /// app's document).
    pub cstring: String,
    /// The string as C's `char *cstring` holds it: the input's bytes after
    /// the `\ooo` escapes, up to the first NUL. The text drivers write
    /// these (see `util::textbytes`).
    pub raw: Vec<u8>,
    pub comments: Vec<Comment>,
}

/// A comment line (`# ...`) of the Fig file: the bytes as C keeps them and
/// the same decoded to Unicode (by the encoding the reader resolved).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Comment {
    pub raw: Vec<u8>,
    pub text: String,
}

impl From<&str> for Comment {
    fn from(s: &str) -> Comment {
        Comment {
            raw: s.as_bytes().to_vec(),
            text: s.to_string(),
        }
    }
}

impl std::ops::Deref for Comment {
    type Target = str;
    fn deref(&self) -> &str {
        &self.text
    }
}

impl std::fmt::Display for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl PartialEq<String> for Comment {
    fn eq(&self, o: &String) -> bool {
        self.text == *o
    }
}

impl PartialEq<&str> for Comment {
    fn eq(&self, o: &&str) -> bool {
        self.text == *o
    }
}

impl Pic {
    /// The file name's bytes (`file` in C).
    pub fn name_bytes(&self) -> &[u8] {
        if self.file_raw.is_empty() {
            self.file.as_bytes()
        } else {
            &self.file_raw
        }
    }

    /// The file name as the operating system takes it: on Unix the raw
    /// bytes, elsewhere the Unicode name.
    pub fn os_name(&self) -> std::ffi::OsString {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            std::ffi::OsString::from_vec(self.name_bytes().to_vec())
        }
        #[cfg(not(unix))]
        {
            std::ffi::OsString::from(self.file.clone())
        }
    }
}

impl Text {
    pub fn rigid_text(&self) -> bool {
        self.flags == DEFAULT || self.flags & RIGID_TEXT != 0
    }
    pub fn special_text(&self) -> bool {
        self.flags != DEFAULT && self.flags & SPECIAL_TEXT != 0
    }
    pub fn psfont_text(&self) -> bool {
        self.flags != DEFAULT && self.flags & PSFONT_TEXT != 0
    }
    pub fn hidden_text(&self) -> bool {
        self.flags != DEFAULT && self.flags & HIDDEN_TEXT != 0
    }
    /// `MAXFONT(t)`.
    pub fn max_font(&self) -> i32 {
        if self.psfont_text() {
            MAX_PSFONT
        } else {
            MAX_FONT
        }
    }
}

/// A spline control point (`F_control`): `lx..ry` for files before 3.2,
/// `s` (the X-spline shape factor) for 3.2.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Control {
    pub lx: f64,
    pub ly: f64,
    pub rx: f64,
    pub ry: f64,
    pub s: f64,
}

/// Spline types.
pub const T_OPEN_APPROX: i32 = 0;
pub const T_CLOSED_APPROX: i32 = 1;
pub const T_OPEN_INTERPOLATED: i32 = 2;
pub const T_CLOSED_INTERPOLATED: i32 = 3;
pub const T_OPEN_XSPLINE: i32 = 4;
pub const T_CLOSED_XSPLINE: i32 = 5;

pub const S_SPLINE_ANGULAR: f64 = 0.0;
pub const S_SPLINE_APPROX: f64 = 1.0;
pub const S_SPLINE_INTERP: f64 = -1.0;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spline {
    pub type_: i32,
    pub style: i32,
    pub thickness: i32,
    pub pen_color: i32,
    pub fill_color: i32,
    pub depth: i32,
    pub pen: i32,
    pub fill_style: i32,
    pub style_val: f64,
    pub for_arrow: Option<Arrow>,
    pub back_arrow: Option<Arrow>,
    pub cap_style: i32,
    pub points: Vec<Point>,
    /// One per point; empty if the file had none.
    pub controls: Vec<Control>,
    pub comments: Vec<Comment>,
}

impl Spline {
    pub fn int_spline(&self) -> bool {
        self.type_ & 0x2 != 0
    }
    pub fn x_spline(&self) -> bool {
        self.type_ & 0x4 != 0
    }
    pub fn approx_spline(&self) -> bool {
        !(self.int_spline() || self.x_spline())
    }
    pub fn closed_spline(&self) -> bool {
        self.type_ & 0x1 != 0
    }
    pub fn open_spline(&self) -> bool {
        !self.closed_spline()
    }
}

/// A compound, and the whole figure (`F_compound`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Compound {
    pub nwcorner: Point,
    pub secorner: Point,
    pub lines: Vec<Line>,
    pub ellipses: Vec<Ellipse>,
    pub splines: Vec<Spline>,
    pub texts: Vec<Text>,
    pub arcs: Vec<Arc>,
    pub compounds: Vec<Compound>,
    pub comments: Vec<Comment>,
}
