//! fig2dev's `emf` driver: the figure as an Enhanced MetaFile (little-endian
//! binary records: pens, brushes and fonts are created as GDI objects and
//! selected, shapes are drawn with polygon, rectangle, ellipse, arc and pie
//! records). Old-style splines are not supported (3.2 splines arrive as
//! polylines). Texts are written as UTF-16 of the input bytes; only PNG
//! pictures are supported (as in fig2dev). The CJK locales (`$LANG` zh, ja, ko:
//! iconv, locale fonts) are not ported.
//
// Ported from fig2dev dev/genemf.c and genemf.h (xfig licence, see LICENSE-xfig).

use crate::gendev::{print_comments, Ctx, NUM_STD_COLS, PACKAGE_VERSION};
use crate::object::{
    Arc, Arrow, Ellipse, Line, Pic, Point, Spline, Text, DEFAULT, P_PNG, T_ARC_BOX, T_BOX, T_CENTER_JUSTIFIED,
    T_CIRCLE_BY_DIA, T_CIRCLE_BY_RAD, T_ELLIPSE_BY_DIA, T_ELLIPSE_BY_RAD, T_LEFT_JUSTIFIED, T_OPEN_ARC, T_PIC_BOX,
    T_PIE_WEDGE_ARC, T_POLYGON, T_POLYLINE, T_RIGHT_JUSTIFIED,
};
use crate::round;
use crate::util::readpics::{open_stream_pic, read_png};
use crate::{Driver, Out};

const UNDEFVALUE: i32 = -100;
const EPSILON: f64 = 1e-4;
/// Limit on the number of handles (the least recently used one is reused).
const MAX_HANDLES: u32 = 50;
/// `sizeof(ENHMETAHEADER)`.
const HEADER_SIZE: usize = 88;
const ARROW_INDENT_DIST: f64 = 0.8;
const ARROW_POINT_DIST: f64 = 1.2;
const ELLIPSE_NPOINT: usize = 72;
/// `PIXEL_01MM`: pixels to 0.01 mm.
const PIXEL_01MM: f64 = 2.11666666667;

// Record types
const EMR_HEADER: u32 = 1;
const EMR_POLYGON: u32 = 3;
const EMR_POLYLINE: u32 = 4;
const EMR_EOF: u32 = 14;
const EMR_SETBKMODE: u32 = 18;
const EMR_SETMAPMODE: u32 = 17;
const EMR_SETBKCOLOR: u32 = 25;
const EMR_SETTEXTALIGN: u32 = 22;
const EMR_SETTEXTCOLOR: u32 = 24;
const EMR_EXTTEXTOUTW: u32 = 84;
const EMR_STRETCHDIBITS: u32 = 81;
const EMR_EXTCREATEFONTINDIRECTW: u32 = 82;
const EMR_SELECTOBJECT: u32 = 37;
const EMR_CREATEBRUSHINDIRECT: u32 = 39;
const EMR_DELETEOBJECT: u32 = 40;
const EMR_ELLIPSE: u32 = 42;
const EMR_RECTANGLE: u32 = 43;
const EMR_ROUNDRECT: u32 = 44;
const EMR_ARC: u32 = 45;
const EMR_CHORD: u32 = 46;
const EMR_PIE: u32 = 47;
const EMR_CREATEDIBPATTERNBRUSHPT: u32 = 94;
const EMR_EXTCREATEPEN: u32 = 95;
const EMR_POLYGON16: u32 = 86;
const EMR_POLYLINE16: u32 = 87;

const ENHMETA_SIGNATURE: u32 = 0x464D_4520;
const ENHMETA_VERSION: u32 = 0x10000;
const ENHMETA_STOCK_OBJECT: u32 = 0x8000_0000;

// Stock objects
const WHITE_BRUSH: u32 = 0;
const LTGRAY_BRUSH: u32 = 1;
const GRAY_BRUSH: u32 = 2;
const DKGRAY_BRUSH: u32 = 3;
const BLACK_BRUSH: u32 = 4;
const NULL_BRUSH: u32 = 5;
const BLACK_PEN: u32 = 7;
const NULL_PEN: u32 = 8;
const SYSTEM_FONT: u32 = 13;

const OPAQUE: i32 = 2;
const TRANSPARENT: i32 = 1;
const TA_LEFT: i32 = 0;
const TA_RIGHT: i32 = 2;
const TA_CENTER: i32 = 6;
const TA_BASELINE: i32 = 24;
const RAD_01DEG: f64 = 572.95780;
const SRCCOPY: u32 = 0x00CC_0020;
const NUM_PS_FONTS: i32 = 35;
const NUM_LATEX_FONTS: i32 = 5;
const MM_TEXT: u32 = 1;

// Brush styles
const BS_SOLID: i32 = 0;
const BS_HOLLOW: i32 = 1;
const BS_HATCHED: i32 = 2;

// Hatch styles
const HS_HORIZONTAL: i32 = 0;
const HS_VERTICAL: i32 = 1;
const HS_FDIAGONAL: i32 = 2;
const HS_BDIAGONAL: i32 = 3;
const HS_CROSS: i32 = 4;
const HS_DIAGCROSS: i32 = 5;

// Pen styles
const PS_USERSTYLE: i32 = 7;
const PS_ENDCAP_ROUND: i32 = 0;
const PS_ENDCAP_SQUARE: i32 = 0x100;
const PS_ENDCAP_FLAT: i32 = 0x200;
const PS_JOIN_ROUND: i32 = 0;
const PS_JOIN_BEVEL: i32 = 0x1000;
const PS_JOIN_MITER: i32 = 0x2000;
const PS_COSMETIC: i32 = 0;
const PS_GEOMETRIC: i32 = 0x10000;

const PAT0_BITS: &[u8] = &[0x03, 0x0C, 0x30, 0xC0];
const PAT1_BITS: &[u8] = &[0xC0, 0x30, 0x0C, 0x03];
const PAT2_BITS: &[u8] = &[0x81, 0x66, 0x18, 0x66];
const PAT6_BITS: &[u8] = &[
    0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0xFF, 0xFF, 0x80, 0x00, 0x80,
    0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0xFF, 0xFF,
];
const PAT7_BITS: &[u8] = &[
    0xFF, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0xFF, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
];
const PAT11_BITS: &[u8] = &[
    0x00, 0x00, 0x80, 0x00, 0x00, 0x80, 0x00, 0x00, 0x40, 0x00, 0x00, 0x40, 0x00, 0x00, 0x20, 0x00, 0x00, 0x20, 0x00,
    0x00, 0x10, 0xFF, 0xFF, 0xFF, 0x80, 0x00, 0x00, 0x80, 0x00, 0x00, 0x40, 0x00, 0x00, 0x40, 0x00, 0x00, 0x20, 0x00,
    0x00, 0x20, 0x00, 0x00, 0x10, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0x80, 0x00, 0x00, 0x80, 0x00, 0x00, 0x40, 0x00,
    0x00, 0x40, 0x00, 0x00, 0x20, 0x00, 0x00, 0x20, 0x00, 0x00, 0x10, 0x00, 0xFF, 0xFF, 0xFF,
];
const PAT12_BITS: &[u8] = &[
    0x00, 0x00, 0x10, 0x00, 0x00, 0x10, 0x00, 0x00, 0x20, 0x00, 0x00, 0x20, 0x00, 0x00, 0x40, 0x00, 0x00, 0x40, 0x00,
    0x00, 0x80, 0xFF, 0xFF, 0xFF, 0x00, 0x10, 0x00, 0x00, 0x10, 0x00, 0x00, 0x20, 0x00, 0x00, 0x20, 0x00, 0x00, 0x40,
    0x00, 0x00, 0x40, 0x00, 0x00, 0x80, 0x00, 0xFF, 0xFF, 0xFF, 0x10, 0x00, 0x00, 0x10, 0x00, 0x00, 0x20, 0x00, 0x00,
    0x20, 0x00, 0x00, 0x40, 0x00, 0x00, 0x40, 0x00, 0x00, 0x80, 0x00, 0x00, 0xFF, 0xFF, 0xFF,
];
const PAT13_BITS: &[u8] = &[
    0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x81, 0x80, 0x80, 0x86, 0x80,
    0x80, 0x98, 0x80, 0x80, 0xE0, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x81,
    0x80, 0x80, 0x86, 0x80, 0x80, 0x98, 0x80, 0x80, 0xE0, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
    0x80, 0x80, 0x80, 0x81, 0x80, 0x80, 0x86, 0x80, 0x80, 0x98, 0x80, 0x80, 0xE0, 0x80, 0x80,
];
const PAT14_BITS: &[u8] = &[
    0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0xC0, 0x80, 0x80, 0xB0, 0x80, 0x80,
    0x8C, 0x80, 0x80, 0x83, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
    0xC0, 0x80, 0x80, 0xB0, 0x80, 0x80, 0x8C, 0x80, 0x80, 0x83, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
    0x80, 0x80, 0x80, 0xC0, 0x80, 0x80, 0xB0, 0x80, 0x80, 0x8C, 0x80, 0x80, 0x83, 0x80, 0x80,
];
const PAT15_BITS: &[u8] = &[
    0x40, 0x02, 0x30, 0x0C, 0x0E, 0x70, 0x01, 0x80, 0x02, 0x40, 0x0C, 0x30, 0x70, 0x0E, 0x80, 0x01,
];
const PAT16_BITS: &[u8] = &[0x01, 0x01, 0x82, 0x6C, 0x10, 0x10, 0x28, 0xC6];
const PAT17_BITS: &[u8] = &[
    0xE0, 0x0F, 0x18, 0x30, 0x04, 0x40, 0x02, 0x80, 0x02, 0x80, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01,
    0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x80, 0x02, 0x80, 0x04, 0x40, 0x18, 0x30,
];
const PAT18_BITS: &[u8] = &[
    0x08, 0x80, 0x00, 0x00, 0x08, 0x80, 0x00, 0x00, 0x04, 0x00, 0x01, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02, 0x00, 0x02,
    0x00, 0x02, 0x00, 0x02, 0x00, 0x01, 0x00, 0x04, 0x00, 0x01, 0x00, 0x04, 0x00, 0x00, 0x00, 0xF8, 0x3F, 0x01, 0x00,
    0x04, 0x00, 0x01, 0x00, 0x04, 0x00, 0x02, 0x00, 0x02, 0x00, 0x02, 0x00, 0x02, 0x00, 0x04, 0x00, 0x01, 0x00, 0x04,
    0x00, 0x01, 0x00, 0x08, 0x80, 0x00, 0x00, 0x08, 0x80, 0x00, 0x00, 0xF0, 0x7F, 0x00, 0x00,
];
const PAT19_BITS: &[u8] = &[
    0xE0, 0x0F, 0x10, 0x10, 0x08, 0x20, 0x04, 0x40, 0x02, 0x80, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01,
    0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x80, 0x04, 0x40, 0x08, 0x20, 0x10, 0x10,
];
const PAT20_BITS: &[u8] = &[0x00, 0x00, 0x00, 0x08, 0x14, 0x22, 0x41, 0x80];
const PAT21_BITS: &[u8] = &[0x02, 0x04, 0x08, 0x10, 0x08, 0x04, 0x02, 0x01];

/// `emf_bm_pattern`: width, height and bits of the bitmap fill patterns
/// (0 x 0: mapped to an EMF hatch).
const BM_PATTERN: [(u32, u32, &[u8]); 22] = [
    (8, 4, PAT0_BITS),
    (8, 4, PAT1_BITS),
    (8, 4, PAT2_BITS),
    (0, 0, &[]),
    (0, 0, &[]),
    (0, 0, &[]),
    (16, 16, PAT6_BITS),
    (16, 16, PAT7_BITS),
    (0, 0, &[]),
    (0, 0, &[]),
    (0, 0, &[]),
    (24, 24, PAT11_BITS),
    (24, 24, PAT12_BITS),
    (24, 24, PAT13_BITS),
    (24, 24, PAT14_BITS),
    (16, 8, PAT15_BITS),
    (8, 8, PAT16_BITS),
    (16, 16, PAT17_BITS),
    (30, 18, PAT18_BITS),
    (16, 16, PAT19_BITS),
    (8, 8, PAT20_BITS),
    (8, 8, PAT21_BITS),
];

/// `emf_map_pattern`: EMF hatch of a fill pattern, -1 for a bitmap pattern.
const MAP_PATTERN: [i32; 22] = [
    -1,
    -1,
    -1,
    HS_FDIAGONAL,
    HS_BDIAGONAL,
    HS_DIAGCROSS,
    -1,
    -1,
    HS_HORIZONTAL,
    HS_VERTICAL,
    HS_CROSS,
    -1,
    -1,
    -1,
    -1,
    -1,
    -1,
    -1,
    -1,
    -1,
    -1,
    -1,
];

/// `rgbColors` of `conv_color`.
const RGB_COLORS: [u32; NUM_STD_COLS] = [
    0x000000, 0x0000FF, 0x00FF00, 0x00FFFF, 0xFF0000, 0xFF00FF, 0xFFFF00, 0xFFFFFF, 0x000090, 0x0000B0, 0x0000D0,
    0x87CEFF, 0x009000, 0x00B000, 0x00D000, 0x009090, 0x00B0B0, 0x00D0D0, 0x900000, 0xB00000, 0xD00000, 0x900090,
    0xB000B0, 0xD000D0, 0x803000, 0xA04000, 0xC06000, 0xFF8080, 0xFFA0A0, 0xFFC0C0, 0xFFE0E0, 0xFFD700,
];

/// EMF compatibility level (`-l`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Level {
    Win95,
    Win98,
    #[default]
    WinNt,
}

impl Level {
    fn name(self) -> &'static str {
        match self {
            Level::Win95 => "win95",
            Level::Win98 => "win98",
            Level::WinNt => "winnt",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HType {
    Pen = 0,
    Brush = 1,
    Font = 2,
}

/// `struct emfhandle`: pen (style, width, rgb), brush (style, rgb, pattern,
/// hatch background rgb) or font (font, size, angle) in `v`.
#[derive(Clone, Debug)]
struct Handle {
    ty: HType,
    is_current: bool,
    handle: u32,
    v: [i32; 4],
}

/// The direction of a line segment (`Dir`).
#[derive(Clone, Copy, Debug, Default)]
struct Dir {
    x: f64,
    y: f64,
}

/// `bbx_left` ... `bbx_bottom` with `UPDATE_BBX_X/Y`.
#[derive(Clone, Copy, Debug)]
struct Bbx {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Bbx {
    fn new(p: Point) -> Bbx {
        Bbx {
            left: p.x,
            top: p.y,
            right: p.x,
            bottom: p.y,
        }
    }
    fn update(&mut self, p: Point) {
        if self.left > p.x {
            self.left = p.x;
        } else if self.right < p.x {
            self.right = p.x;
        }
        if self.top > p.y {
            self.top = p.y;
        } else if self.bottom < p.y {
            self.bottom = p.y;
        }
    }
    fn fits16(&self) -> bool {
        self.left >= -32768 && self.right <= 32767 && self.top >= -32768 && self.bottom <= 32767
    }
}

/// The attributes `shape` reads from a line, an ellipse or an arc.
#[derive(Clone, Copy, Debug)]
struct Sh {
    style: i32,
    thickness: i32,
    pen_color: i32,
    fill_color: i32,
    fill_style: i32,
}

/// The `emf` driver's state (genemf.c's statics).
#[derive(Debug)]
pub struct Emf {
    /// Bytes and messages of the current call, handed to `out` and `ctx`
    /// when it ends.
    buf: Vec<u8>,
    msgs: Vec<String>,
    /// Where the header starts in `out`.
    base: usize,
    header: [u32; HEADER_SIZE / 4],
    n_bytes: u32,
    n_records: u32,
    n_handles: u32,
    hs: Vec<Handle>,
    /// The handle list, oldest first (`handles` .. `latesthandle`).
    order: Vec<usize>,
    last_handle: [u32; 3],
    rounded_arrows: bool,
    level: Level,
    old_bkmode: i32,
    old_bkcolor: i32,
    warned_32bit: bool,
    warned_spline: bool,
    user_colors: Vec<[u8; 3]>,
    /// `oldcolor` of `textcolr` and `oldalign` of `textalign`.
    text_color: i32,
    text_align: i32,
    /// `coltab` of `picbox`: static, so entries survive from picture to picture.
    coltab: [[u8; 4]; 256],
}

impl Default for Emf {
    fn default() -> Emf {
        Emf {
            buf: Vec::new(),
            msgs: Vec::new(),
            base: 0,
            header: [0; HEADER_SIZE / 4],
            n_bytes: 0,
            n_records: 0,
            n_handles: 0,
            hs: Vec::new(),
            order: Vec::new(),
            last_handle: [0; 3],
            rounded_arrows: false,
            level: Level::WinNt,
            old_bkmode: 0,
            old_bkcolor: UNDEFVALUE,
            warned_32bit: false,
            warned_spline: false,
            user_colors: Vec::new(),
            text_color: UNDEFVALUE,
            text_align: 0,
            coltab: [[0; 4]; 256],
        }
    }
}

/// The driver for `lang`.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    (lang == "emf").then(|| Box::new(Emf::default()) as Box<dyn Driver>)
}

/// `RGB(r, g, b)`.
fn rgb_of(r: u8, g: u8, b: u8) -> i32 {
    (u32::from(r) | u32::from(g) << 8 | u32::from(b) << 16) as i32
}

fn get_r(rgb: i32) -> u8 {
    rgb as u8
}

fn get_g(rgb: i32) -> u8 {
    (rgb >> 8) as u8
}

fn get_b(rgb: i32) -> u8 {
    (rgb >> 16) as u8
}

/// `ARROW_EXTRA_LEN(a)`: arrows appear this much longer with projected caps.
fn extra_len(a: &Arrow) -> f64 {
    a.ht / a.wid * a.thickness
}

/// `distance()`: `sqrt(dx*dx + dy*dy)`, fused as clang does.
fn hypot_fma(dx: f64, dy: f64) -> f64 {
    dx.mul_add(dx, dy * dy).sqrt()
}

fn distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    hypot_fma(x2 - x1, y2 - y1)
}

/// `direction`: the distance and normalised direction vector from `q` to
/// `p`; false when the points coincide.
fn direction(p: Point, q: Point) -> (bool, Dir, f64) {
    let mut dir = Dir {
        x: f64::from(p.x.wrapping_sub(q.x)),
        y: f64::from(p.y.wrapping_sub(q.y)),
    };
    let dist = hypot_fma(dir.x, dir.y);
    if dist < EPSILON {
        return (false, dir, dist);
    }
    dir.x /= dist;
    dir.y /= dist;
    (true, dir, dist)
}

/// `arc_rotate`: rotates `p` counter clockwise along the arc with centre c.
fn arc_rotate(p: &mut Point, cx: f64, cy: f64, angle: f64) {
    let mut x = f64::from(p.x);
    let mut y = f64::from(p.y);
    x += -cx;
    y += -cy;
    let (s, c) = (angle.sin(), angle.cos());
    let (xt, yt) = (x, y);
    x = c.mul_add(xt, s * yt);
    y = (-s).mul_add(xt, c * yt);
    x += cx;
    y += cy;
    p.x = round(x);
    p.y = round(y);
}

fn arc_radius(a: &Arc) -> f64 {
    (distance(f64::from(a.point[0].x), f64::from(a.point[0].y), a.center.x, a.center.y)
        + distance(f64::from(a.point[1].x), f64::from(a.point[1].y), a.center.x, a.center.y)
        + distance(f64::from(a.point[2].x), f64::from(a.point[2].y), a.center.x, a.center.y))
        / 3.
}

/// `conv_linetype`.
fn conv_linetype(t: i32) -> i32 {
    if t < 0 {
        -1
    } else if t > 4 {
        t % 5
    } else {
        t
    }
}

/// `ltFonts`: LaTeX font to PostScript font.
const LT_FONTS: [i32; 6] = [0, 0, 2, 1, 4, 12];
/// `lfFaceName`.
const LF_FACE_NAME: [&str; 35] = [
    "Times",
    "Times",
    "Times",
    "Times",
    "AvantGarde",
    "AvantGarde",
    "AvantGarde",
    "AvantGarde",
    "Bookman",
    "Bookman",
    "Bookman",
    "Bookman",
    "Courier",
    "Courier",
    "Courier",
    "Courier",
    "Helvetica",
    "Helvetica",
    "Helvetica",
    "Helvetica",
    "Helvetica-Narrow",
    "Helvetica-Narrow",
    "Helvetica-Narrow",
    "Helvetica-Narrow",
    "NewCenturySchlbk",
    "NewCenturySchlbk",
    "NewCenturySchlbk",
    "NewCenturySchlbk",
    "Palatino",
    "Palatino",
    "Palatino",
    "Palatino",
    "Symbol",
    "ZapfChancery",
    "ZapfDingbats",
];
/// `lfWeight` (FW_NORMAL 400, FW_BOLD 700, FW_DEMIBOLD 600, FW_LIGHT 300).
const LF_WEIGHT: [u32; 35] = [
    400, 400, 700, 700, 400, 400, 600, 600, 300, 300, 600, 600, 400, 400, 700, 700, 400, 400, 700, 700, 400, 400, 700,
    700, 400, 400, 700, 700, 400, 400, 700, 700, 400, 400, 400,
];
/// `lfItalic`: odd entries (the last three are not italic).
const LF_ITALIC: [u8; 35] = [
    0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0, 0,
];

/// `is_flip`.
fn is_flip(rot: i32, flip: i32) -> bool {
    if rot == 90 || rot == 270 {
        flip == 0
    } else {
        flip != 0
    }
}

/// `double` to `EMFulong` as the reference build does it (saturating).
fn sat_u32(x: f64) -> u32 {
    x as u32
}

impl Emf {
    // ---- the record writer ----

    fn msg(&mut self, m: impl Into<String>) {
        self.msgs.push(m.into());
    }

    /// `emh_write(.., EMH_RECORD)` of a record made of little-endian words.
    fn rec(&mut self, words: &[u32]) {
        self.n_records = self.n_records.wrapping_add(1);
        for w in words {
            self.buf.extend_from_slice(&w.to_le_bytes());
        }
        self.n_bytes = self.n_bytes.wrapping_add((words.len() * 4) as u32);
    }

    /// `emh_write(.., EMH_DATA)`.
    fn data(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
        self.n_bytes = self.n_bytes.wrapping_add(b.len() as u32);
    }

    /// Hands the bytes and messages of this call on.
    fn flush(&mut self, ctx: &mut Ctx, out: &mut Out) {
        out.bytes(&self.buf);
        self.buf.clear();
        for m in self.msgs.drain(..) {
            ctx.warn(m);
        }
    }

    fn warn_32bit_pos(&mut self) {
        if self.level != Level::WinNt && !self.warned_32bit {
            self.msg("Warning: Coordinates exceed 16bit value.");
            self.msg("Some figures will be invisible on Windows 95/98/Me.");
            self.warned_32bit = true;
        }
    }

    // ---- handles ----

    fn delete_handle(&mut self, i: usize) {
        let Some(h) = self.hs.get(i).cloned() else {
            return;
        };
        if h.is_current {
            let (stock, slot) = match h.ty {
                HType::Pen => (ENHMETA_STOCK_OBJECT | BLACK_PEN, 0),
                HType::Brush => (ENHMETA_STOCK_OBJECT | WHITE_BRUSH, 1),
                HType::Font => (ENHMETA_STOCK_OBJECT | SYSTEM_FONT, 2),
            };
            self.last_handle[slot] = stock;
            self.rec(&[EMR_SELECTOBJECT, 12, stock]);
            if let Some(h) = self.hs.get_mut(i) {
                h.is_current = false;
            }
        }
        self.rec(&[EMR_DELETEOBJECT, 12, h.handle]);
    }

    /// Clears the current mark of the first object of `ty` that has it.
    fn clear_current_handle(&mut self, ty: HType) {
        for &i in &self.order {
            if let Some(h) = self.hs.get_mut(i) {
                if h.is_current && h.ty == ty {
                    h.is_current = false;
                    break;
                }
            }
        }
    }

    fn select_object(&mut self, handle: u32) {
        self.rec(&[EMR_SELECTOBJECT, 12, handle]);
    }

    /// Moves a handle to the newest position of the list.
    fn use_handle(&mut self, i: usize) {
        if self.order.last() != Some(&i) {
            self.order.retain(|&j| j != i);
            self.order.push(i);
        }
    }

    /// Allocates a handle; beyond the limit the least recently used one that
    /// isn't selected is reused.
    fn get_handle(&mut self, ty: HType) -> usize {
        if self.n_handles >= MAX_HANDLES {
            let reuse = self
                .order
                .iter()
                .copied()
                .find(|&i| self.hs.get(i).is_some_and(|h| !h.is_current));
            if let Some(i) = reuse {
                self.delete_handle(i);
                if let Some(h) = self.hs.get_mut(i) {
                    h.ty = ty;
                }
                return i;
            }
        }
        self.n_handles = self.n_handles.wrapping_add(1);
        self.hs.push(Handle {
            ty,
            is_current: false,
            handle: self.n_handles,
            v: [0; 4],
        });
        self.hs.len() - 1
    }

    /// The most recent handle of `ty` with attributes `v`.
    fn find_handle(&self, ty: HType, v: [i32; 4], n: usize) -> Option<usize> {
        self.order
            .iter()
            .rev()
            .copied()
            .find(|&i| self.hs.get(i).is_some_and(|h| h.ty == ty && h.v[..n] == v[..n]))
    }

    // ---- colours and styles ----

    /// `conv_color`: the RGB value of a Fig colour.
    fn conv_color(&self, color: i32) -> i32 {
        let idx = if color == DEFAULT { 0 } else { color };
        let Ok(i) = usize::try_from(idx) else {
            return 0;
        };
        if let Some(&c) = RGB_COLORS.get(i) {
            rgb_of((c >> 16) as u8, (c >> 8) as u8, c as u8)
        } else {
            match self.user_colors.get(i - NUM_STD_COLS) {
                Some(&[r, g, b]) => rgb_of(r, g, b),
                None => 0,
            }
        }
    }

    /// `conv_fill_color`: the fill intensity is `style` (0 to 40).
    fn conv_fill_color(&self, style: i32, color: i32) -> i32 {
        let rgb = self.conv_color(color);
        let (mut b, mut g, mut r) = (i32::from(get_b(rgb)), i32::from(get_g(rgb)), i32::from(get_r(rgb)));
        if style <= 0 {
            b = 0;
            g = 0;
            r = 0;
        } else if style < 20 {
            let f = (f64::from(style as f32) / 20.0) as f32;
            let dark = |c: i32| round(f64::from(c as f32 * f));
            b = dark(b);
            g = dark(g);
            r = dark(r);
        } else if style == 20 {
        } else if style < 40 {
            let f = (f64::from((style - 20) as f32) / 20.0) as f32;
            let light = |c: i32| round(f64::from(f.mul_add((255 - c) as f32, c as f32)));
            b = light(b);
            g = light(g);
            r = light(r);
        } else {
            b = 0xFF;
            g = 0xFF;
            r = 0xFF;
        }
        rgb_of(r as u8, g as u8, b as u8)
    }

    fn conv_capstyle(&mut self, cap: i32) -> i32 {
        match cap {
            0 => PS_ENDCAP_FLAT,
            1 => PS_ENDCAP_ROUND,
            2 => PS_ENDCAP_SQUARE,
            _ => {
                self.msg(format!("genemf: unknown cap style {cap}."));
                PS_ENDCAP_FLAT
            }
        }
    }

    fn conv_joinstyle(&mut self, join: i32) -> i32 {
        match join {
            0 => PS_JOIN_MITER,
            1 => PS_JOIN_ROUND,
            2 => PS_JOIN_BEVEL,
            _ => {
                self.msg(format!("genemf: unknown join style {join}."));
                PS_JOIN_MITER
            }
        }
    }

    /// Finds or creates the pen (the stock null pen when invisible).
    fn pen_handle(
        &mut self,
        vis: i32,
        mut typ: i32,
        width: i32,
        color: i32,
        join: i32,
        cap: i32,
    ) -> (u32, Option<usize>) {
        if vis == 0 {
            return (ENHMETA_STOCK_OBJECT | NULL_PEN, None);
        }
        let rgb = self.conv_color(color);
        let mut style = PS_GEOMETRIC;
        match self.level {
            Level::Win98 => {
                if (1..=4).contains(&typ) {
                    self.msg("Warning: line style may be ignored");
                }
            }
            Level::Win95 => {
                if (1..=4).contains(&typ) {
                    if width < 20 {
                        self.msg("Warning: this style of line is converted to thin line");
                        style = PS_COSMETIC;
                    } else {
                        self.msg("Warning: line style may be ignored");
                    }
                }
            }
            Level::WinNt => {}
        }
        let mut user_style = false;
        if self.level == Level::WinNt {
            if typ == 5 {
                style |= PS_USERSTYLE;
                user_style = true;
            } else {
                style |= conv_linetype(typ);
            }
        } else {
            if typ == 5 {
                self.msg("Warning: dash-triple-dotted style is converted to solid style");
                typ = 0;
            }
            style |= conv_linetype(typ);
        }
        let cap = self.conv_capstyle(cap);
        let join = self.conv_joinstyle(join);
        style |= cap | join;

        let key = [style, width, rgb, 0];
        if let Some(i) = self.find_handle(HType::Pen, key, 3) {
            self.use_handle(i);
            return (self.hs.get(i).map_or(0, |h| h.handle), Some(i));
        }
        let i = self.get_handle(HType::Pen);
        let handle = self.hs.get(i).map_or(0, |h| h.handle);
        self.use_handle(i);
        if let Some(h) = self.hs.get_mut(i) {
            h.v = key;
        }
        // EMREXTCREATEPEN (52 bytes) and the user style entries.
        let size: u32 = if user_style { 52 + 32 } else { 52 };
        self.rec(&[
            EMR_EXTCREATEPEN,
            size,
            handle,
            52,
            0,
            52,
            0,
            style as u32,
            width as u32,
            0,
            rgb as u32,
            0,
            if user_style { 8 } else { 0 },
        ]);
        if user_style {
            let w = width as u32;
            let entries = [w.wrapping_mul(4), w, w, w, w, w, w, w];
            let mut bytes = Vec::with_capacity(32);
            for e in entries {
                bytes.extend_from_slice(&e.to_le_bytes());
            }
            self.data(&bytes);
        }
        (handle, Some(i))
    }

    /// `edgeattr`: sets the pen attributes and selects the pen.
    fn edgeattr(&mut self, mut vis: i32, typ: i32, width: i32, color: i32, join: i32, cap: i32) {
        if width == 0 {
            vis = 0;
        }
        let (handle, h) = self.pen_handle(vis, typ, width, color, join, cap);
        if handle == self.last_handle[HType::Pen as usize] {
            return;
        }
        self.last_handle[HType::Pen as usize] = handle;
        self.clear_current_handle(HType::Pen);
        if let Some(h) = h.and_then(|i| self.hs.get_mut(i)) {
            h.is_current = true;
        }
        self.select_object(handle);
    }

    fn edgevis(&mut self, onoff: i32) {
        self.edgeattr(onoff, 0, 0, 0, 0, 0);
    }

    fn bkmode(&mut self, mode: i32) {
        if mode == self.old_bkmode {
            return;
        }
        self.old_bkmode = mode;
        self.rec(&[EMR_SETBKMODE, 12, mode as u32]);
    }

    fn bkcolor(&mut self, rgb: i32) {
        if rgb == self.old_bkcolor {
            return;
        }
        self.old_bkcolor = rgb;
        self.rec(&[EMR_SETBKCOLOR, 12, rgb as u32]);
    }

    /// `create_brush_pattern`: the pattern's bits, bottom to top, each line
    /// padded to 4 bytes, ORed into `bits`.
    fn create_brush_pattern(bits: &mut [u8], pattern: usize) {
        let Some(&(width, height, b)) = BM_PATTERN.get(pattern) else {
            return;
        };
        let xbmhbytecnt = (width as usize).div_ceil(8);
        let mut byte = 0usize;
        for y in (0..height as usize).rev() {
            let mut bit: i32 = 7;
            for x in 0..width as usize {
                if b.get(xbmhbytecnt * y + x / 8).is_some_and(|v| v & (1 << (x % 8)) != 0) {
                    if let Some(o) = bits.get_mut(byte) {
                        *o |= 1 << bit;
                    }
                }
                bit -= 1;
                if bit < 0 {
                    byte += 1;
                    bit = 7;
                }
            }
            if bit != 7 {
                byte += 1;
            }
            byte = (byte + 3) & !3;
        }
    }

    /// `fillstyle`: sets the brush for fill styles 0 to 40 (solid) and the
    /// patterns above, and selects it.
    fn fillstyle(&mut self, fill_color: i32, fill_style: i32, pen_color: i32) {
        let (style, rgb, mut hatchbkrgb, pattern);
        if fill_style < 0 {
            style = BS_HOLLOW;
            rgb = 0;
            hatchbkrgb = 0;
            pattern = 0;
        } else if fill_style <= 40 {
            style = BS_SOLID;
            rgb = self.conv_fill_color(fill_style, fill_color);
            hatchbkrgb = 0;
            pattern = 0;
        } else {
            style = BS_HATCHED;
            rgb = self.conv_color(pen_color);
            hatchbkrgb = self.conv_color(fill_color);
            pattern = fill_style - 41;
        }
        // C tests `> 22` and then reads one entry past the tables for 22.
        let pat = match usize::try_from(pattern) {
            Ok(p) if p < BM_PATTERN.len() => p,
            _ => {
                self.msg(format!("fig2dev: emf: fill pattern {pattern} is not supported"));
                return;
            }
        };
        let map = MAP_PATTERN.get(pat).copied().unwrap_or(-1);
        if style == BS_HATCHED && map != -1 {
            // EMF hatch pattern; the background colour is not in the brush.
            self.bkmode(OPAQUE);
            self.bkcolor(hatchbkrgb);
            hatchbkrgb = 0;
        }
        let stock = match style {
            BS_HOLLOW => Some(NULL_BRUSH),
            BS_SOLID => match rgb {
                0xffffff => Some(WHITE_BRUSH),
                0xc0c0c0 => Some(LTGRAY_BRUSH),
                0x808080 => Some(GRAY_BRUSH),
                0x404040 => Some(DKGRAY_BRUSH),
                0x000000 => Some(BLACK_BRUSH),
                _ => None,
            },
            _ => None,
        };
        let (handle, h) = if let Some(s) = stock {
            (ENHMETA_STOCK_OBJECT | s, None)
        } else {
            let key = [style, rgb, pattern, hatchbkrgb];
            if let Some(i) = self.find_handle(HType::Brush, key, 4) {
                self.use_handle(i);
                (self.hs.get(i).map_or(0, |h| h.handle), Some(i))
            } else {
                let i = self.get_handle(HType::Brush);
                let handle = self.hs.get(i).map_or(0, |h| h.handle);
                self.use_handle(i);
                if let Some(h) = self.hs.get_mut(i) {
                    h.v = key;
                }
                if style == BS_HATCHED && map == -1 {
                    self.create_pattern_brush(handle, pat, rgb, hatchbkrgb);
                } else {
                    let hatch = if style == BS_HATCHED { map } else { 0 };
                    self.rec(&[
                        EMR_CREATEBRUSHINDIRECT,
                        24,
                        handle,
                        style as u32,
                        rgb as u32,
                        hatch as u32,
                    ]);
                }
                (handle, Some(i))
            }
        };
        if handle == self.last_handle[HType::Brush as usize] {
            return;
        }
        self.last_handle[HType::Brush as usize] = handle;
        self.clear_current_handle(HType::Brush);
        if let Some(h) = h.and_then(|i| self.hs.get_mut(i)) {
            h.is_current = true;
        }
        self.select_object(handle);
    }

    /// The bitmap pattern brush (`EMR_CREATEDIBPATTERNBRUSHPT`).
    fn create_pattern_brush(&mut self, handle: u32, pattern: usize, rgb: i32, hatchbkrgb: i32) {
        let Some(&(width, orig_height, _)) = BM_PATTERN.get(pattern) else {
            return;
        };
        let mut height = orig_height;
        if self.level != Level::WinNt && (width > 8 || height > 8) {
            self.msg(format!(
                "Warning: fill pattern {pattern} will not appear properly on Windows 95/98/Me"
            ));
        }
        // Windows 95/98/Me doesn't show 8x4 pat properly. Convert to 8x8.
        if height == 4 {
            height = 8;
        }
        let bsize = (width.div_ceil(32) * height * 4) as usize;
        self.rec(&[
            EMR_CREATEDIBPATTERNBRUSHPT,
            (32 + 40 + 8 + bsize) as u32,
            handle,
            0,
            32,
            (40 + 2 * 4) as u32,
            (32 + 40 + 8) as u32,
            bsize as u32,
        ]);
        let mut bmi = Vec::with_capacity(40);
        for w in [40, width, height, 1 | 1 << 16, 0, bsize as u32, 2953, 2953, 2, 0] {
            bmi.extend_from_slice(&w.to_le_bytes());
        }
        self.data(&bmi);
        self.data(&[
            get_b(hatchbkrgb),
            get_g(hatchbkrgb),
            get_r(hatchbkrgb),
            0,
            get_b(rgb),
            get_g(rgb),
            get_r(rgb),
            0,
        ]);
        let mut bits = vec![0u8; bsize];
        Emf::create_brush_pattern(&mut bits, pattern);
        if orig_height == 4 {
            // the latter half
            if let Some(second) = bits.get_mut(bsize / 2..) {
                Emf::create_brush_pattern(second, pattern);
            }
        }
        self.data(&bits);
    }

    // ---- arrows ----

    /// The length by which an arrow shortens its line or arc.
    fn arrow_length(&self, a: &Arrow) -> f64 {
        let mut len = match a.type_ {
            0 => extra_len(a),
            1 => a.ht,
            2 => ARROW_INDENT_DIST * a.ht,
            3 => ARROW_POINT_DIST * a.ht,
            _ => 0.,
        };
        if self.rounded_arrows {
            len = (-(a.ht / a.wid)).mul_add(a.thickness, len);
        }
        len
    }

    /// Writes a polyline or polygon record (16 bit points when they fit).
    fn poly_record(&mut self, t16: u32, t32: u32, bbx: Bbx, pts: &[Point]) {
        let cpt = pts.len() as u32;
        let bounds = [bbx.left as u32, bbx.top as u32, bbx.right as u32, bbx.bottom as u32];
        if bbx.fits16() {
            let mut w = vec![t16, 28 + 4 * cpt];
            w.extend_from_slice(&bounds);
            w.push(cpt);
            self.rec(&w);
            let mut b = Vec::with_capacity(pts.len() * 4);
            for p in pts {
                b.extend_from_slice(&(p.x as i16).to_le_bytes());
                b.extend_from_slice(&(p.y as i16).to_le_bytes());
            }
            self.data(&b);
        } else {
            self.warn_32bit_pos();
            let mut w = vec![t32, 28 + 8 * cpt];
            w.extend_from_slice(&bounds);
            w.push(cpt);
            self.rec(&w);
            let mut b = Vec::with_capacity(pts.len() * 8);
            for p in pts {
                b.extend_from_slice(&p.x.to_le_bytes());
                b.extend_from_slice(&p.y.to_le_bytes());
            }
            self.data(&b);
        }
    }

    /// Draws an arrow ending at `pp` pointing into direction `dir`.
    fn arrow(&mut self, pp: Point, a: &Arrow, pen_color: i32, dir: Dir) {
        let mut t = Point::new(0, 0);
        let mut p = pp;
        if !self.rounded_arrows {
            // Move the arrow backwards in order to let it end at the correct spot.
            let f = extra_len(a);
            p.x = round((-f).mul_add(dir.x, f64::from(p.x)));
            p.y = round((-f).mul_add(dir.y, f64::from(p.y)));
        }
        self.edgeattr(1, 0, a.thickness as i32, pen_color, 0, 0);
        if a.type_ != 0 {
            match a.style {
                0 => self.fillstyle(7, 20, 0),
                1 => self.fillstyle(pen_color, 20, 0),
                s => self.msg(format!("Unsupported fig arrow style {s} !!")),
            }
        }
        // the bounding box uses the original point
        let mut bbx = Bbx::new(pp);
        let (px, py) = (f64::from(p.x), f64::from(p.y));
        let w5 = a.wid * 0.5;
        // distance of the arrow's base (or of the hat's corners) from the tip
        let (k, cpt, polyline) = match a.type_ {
            0 => (a.ht, 3, true),
            1 => (a.ht, 3, false),
            2 => (a.ht * ARROW_POINT_DIST, 4, false),
            3 => (a.ht * ARROW_INDENT_DIST, 4, false),
            ty => {
                self.msg(format!("Unsupported fig arrow type {ty}."));
                return;
            }
        };
        let bx = (-k).mul_add(dir.x, px);
        let by = (-k).mul_add(dir.y, py);
        let s1 = Point::new(round(w5.mul_add(dir.y, bx)), round((-w5).mul_add(dir.x, by)));
        let s2 = Point::new(round((-w5).mul_add(dir.y, bx)), round(w5.mul_add(dir.x, by)));
        if cpt == 4 {
            t.x = round((-a.ht).mul_add(dir.x, px));
            t.y = round((-a.ht).mul_add(dir.y, py));
            bbx.update(t);
        }
        bbx.update(s1);
        bbx.update(s2);
        let all = [s1, p, s2, t];
        let pts = all.get(..cpt).unwrap_or(&all);
        if polyline {
            self.poly_record(EMR_POLYLINE16, EMR_POLYLINE, bbx, pts);
        } else {
            self.poly_record(EMR_POLYGON16, EMR_POLYGON, bbx, pts);
        }
    }

    fn arc_arrow(&mut self, p: Point, q: Point, arw: Option<&Arrow>, arc: &Arc) {
        let Some(arw) = arw else {
            return;
        };
        let (_, dir, _) = direction(p, q);
        self.arrow(p, arw, arc.pen_color, dir);
    }

    /// Replaces `p` by the starting point of the arc arrow ending at `p`;
    /// `arwpos` is the origin of the arrow.
    fn arc_arrow_adjust(&self, p: &mut Point, arwpos: &mut Point, arc: &Arc, r: f64, arw: Option<&Arrow>, adir: f64) {
        let Some(arw) = arw else {
            return;
        };
        let (cx, cy) = (arc.center.x, arc.center.y);
        let origpos = *p;
        let l = if arw.type_ != 0 { self.arrow_length(arw) } else { arw.ht };
        arc_rotate(p, cx, cy, adir * l / 1.8f64.mul_add(r, l));

        // length of arrow head
        let mut alen = arw.ht;
        if !self.rounded_arrows {
            alen = (arw.ht / arw.wid).mul_add(arw.thickness, alen);
        }
        'fix: {
            // if the arrow is longer than the diameter, no way to fix its root
            if alen >= 2. * r {
                *arwpos = *p;
                break 'fix;
            }
            // make a chord with the arc and the center line of the arrow head
            let th = 2. * (1. / (4. * r * r / (alen * alen) - 1.)).sqrt().atan();
            *arwpos = origpos;
            arc_rotate(arwpos, cx, cy, adir * th);

            // direction from arrow root to arrow top
            let (ok, dir, _) = direction(origpos, *arwpos);
            if !ok {
                break 'fix;
            }
            // the centre of the line of the arrow top
            let mut p1 = origpos;
            if !self.rounded_arrows {
                let d = extra_len(arw);
                p1.x = round((-d).mul_add(dir.x, f64::from(p1.x)));
                p1.y = round((-d).mul_add(dir.y, f64::from(p1.y)));
            }
            // the other points of the arrow head
            let w5 = arw.wid * 0.5;
            let (x1, y1) = (f64::from(p1.x), f64::from(p1.y));
            let bx = (-arw.ht).mul_add(dir.x, x1);
            let by = (-arw.ht).mul_add(dir.y, y1);
            let mut p2 = Point::new(round(w5.mul_add(dir.y, bx)), round((-w5).mul_add(dir.x, by)));
            let p3 = Point::new(round((-w5).mul_add(dir.y, bx)), round(w5.mul_add(dir.x, by)));

            // the distance from the centre of the arc to the points
            let mut d2 = distance(cx, cy, f64::from(p2.x), f64::from(p2.y));
            let d3 = distance(cx, cy, f64::from(p3.x), f64::from(p3.y));
            // use the farther point
            if d3 > d2 {
                p2 = p3;
                d2 = d3;
            }
            // the farther point shall be out of the arc
            if d2 < r {
                break 'fix;
            }
            // direction from p1 to p2
            let (ok, dir, _) = direction(p2, p1);
            if !ok {
                break 'fix;
            }
            // drop a perpendicular line from the arc centre to the line on
            // which p1 and p2 are
            let xx = f64::from(p2.x) - cx;
            let yy = f64::from(p2.y) - cy;
            // the pedal point of the perpendicular line
            let d = xx.mul_add(dir.x, yy * dir.y);
            let mut p4x = (-d).mul_add(dir.x, f64::from(p2.x));
            let mut p4y = (-d).mul_add(dir.y, f64::from(p2.y));
            // the length^2 of the perpendicular line segment
            let d = xx.mul_add(dir.y, -(yy * dir.x));
            let d = d * d;
            // length from p4 to the target point
            let d = r.mul_add(r, -d).sqrt();
            p4x = d.mul_add(dir.x, p4x);
            p4y = d.mul_add(dir.y, p4y);
            p.x = round(p4x);
            p.y = round(p4y);
        }
        if arc.cap_style == 2 {
            // Projecting cap style is a little longer. Shorten the arc.
            arc_rotate(p, cx, cy, adir * f64::from(arc.thickness) / (2.0 * r));
        }
    }

    // ---- arcs ----

    fn arc_box(a: &Arc, r: f64) -> [u32; 4] {
        [
            round(a.center.x - r) as u32,
            round(a.center.y - r) as u32,
            round(a.center.x + r) as u32,
            round(a.center.y + r) as u32,
        ]
    }

    fn arcoutline(&mut self, a: &Arc) {
        let r = arc_radius(a);
        let mut p0 = a.point[0];
        let mut p2 = a.point[2];
        let mut arwp0 = Point::default();
        let mut arwp2 = Point::default();
        let mut skip = false;
        if a.type_ == T_OPEN_ARC && a.thickness != 0 {
            // adjust for arrow heads
            self.arc_arrow_adjust(&mut p0, &mut arwp0, a, r, a.back_arrow.as_ref(), 1.);
            self.arc_arrow_adjust(&mut p2, &mut arwp2, a, r, a.for_arrow.as_ref(), -1.);
            // the inner product: if negative the arc is probably very short
            // and hidden in the arrow heads
            let ip = a.point[0]
                .x
                .wrapping_sub(a.point[2].x)
                .wrapping_mul(p0.x.wrapping_sub(p2.x))
                .wrapping_add(
                    a.point[0]
                        .y
                        .wrapping_sub(a.point[2].y)
                        .wrapping_mul(p0.y.wrapping_sub(p2.y)),
                );
            if ip < 0 {
                skip = true;
            }
        }
        if !skip {
            let t = if a.type_ == T_OPEN_ARC { EMR_ARC } else { EMR_PIE };
            let b = Emf::arc_box(a, r);
            self.rec(&[
                t,
                40,
                b[0],
                b[1],
                b[2],
                b[3],
                p0.x as u32,
                p0.y as u32,
                p2.x as u32,
                p2.y as u32,
            ]);
        }
        if a.type_ == T_OPEN_ARC && a.thickness != 0 {
            // draw arrow heads
            self.arc_arrow(a.point[0], arwp0, a.back_arrow.as_ref(), a);
            self.arc_arrow(a.point[2], arwp2, a.for_arrow.as_ref(), a);
        }
    }

    fn arcinterior(&mut self, a: &Arc) {
        let r = arc_radius(a);
        let b = Emf::arc_box(a, r);
        self.rec(&[
            EMR_CHORD,
            40,
            b[0],
            b[1],
            b[2],
            b[3],
            a.point[0].x as u32,
            a.point[0].y as u32,
            a.point[2].x as u32,
            a.point[2].y as u32,
        ]);
    }

    /// `cwarc`: whether the arc is a clockwise arc.
    fn cwarc(a: &Arc) -> bool {
        let x1 = a.point[1].x.wrapping_sub(a.point[0].x);
        let y1 = a.point[1].y.wrapping_sub(a.point[0].y);
        let x2 = a.point[2].x.wrapping_sub(a.point[1].x);
        let y2 = a.point[2].y.wrapping_sub(a.point[1].y);
        x1.wrapping_mul(y2).wrapping_sub(y1.wrapping_mul(x2)) > 0
    }

    // ---- shapes ----

    /// `shape`: edge and interior attributes of a simple closed shape.
    fn shape(&mut self, s: Sh, join: i32, cap: i32) {
        self.edgeattr(1, s.style, s.thickness, s.pen_color, join, cap);
        self.fillstyle(s.fill_color, s.fill_style, s.pen_color);
    }

    fn polygon(&mut self, pts: &[Point]) {
        let Some(&first) = pts.first() else {
            return;
        };
        let mut bbx = Bbx::new(first);
        for &p in pts {
            bbx.update(p);
        }
        // Windows 95/98/Me: maximum points allowed is approx. 1360
        if pts.len() > 1360 && self.level != Level::WinNt {
            self.msg("Warning: polygon has too many points -- may be partially invisible");
        }
        self.poly_record(EMR_POLYGON16, EMR_POLYGON, bbx, pts);
    }

    /// `polyline_adjust`: replaces `p` by the shortened starting point of the
    /// line segment towards `q`.
    fn polyline_adjust(&mut self, p: &mut Point, q: Point, l: f64) {
        let (ok, dir, d) = direction(*p, q);
        if ok {
            p.x = round((-l).mul_add(dir.x, f64::from(p.x)));
            p.y = round((-l).mul_add(dir.y, f64::from(p.y)));
            let _ = l < d;
            return;
        }
        self.msg("Warning: Arrow at zero-length line segment omitted.");
    }

    /// Draws the polyline boundary (with arrows if needed).
    fn polyline(&mut self, l: &Line) {
        let pts = &l.points;
        let Some(&first) = pts.first() else {
            return;
        };
        let n = pts.len();
        let mut bbx = Bbx::new(first);
        for &p in pts {
            bbx.update(p);
        }
        let mut p0 = first;
        let mut pn = pts.get(n - 1).copied().unwrap_or(first);
        let mut cpt = n;
        // s: index in `pts` of the node `p0` was copied from
        let mut s = 0usize;
        let at = |i: usize, s: usize, p0: Point| -> Point {
            if i == 0 {
                p0
            } else {
                pts.get(s + i).copied().unwrap_or(p0)
            }
        };
        let mut draw = true;
        if cpt == 1 {
            // Draw single point as a short line.
            cpt = 2;
            pn.y = pn.y.wrapping_add(1);
        } else {
            // Delete line segments which may overlap arrow heads.
            if let Some(ba) = &l.back_arrow {
                let mut alen = self.arrow_length(ba);
                while cpt > 1 {
                    let q = at(1, s, p0);
                    let seglen = distance(f64::from(p0.x), f64::from(p0.y), f64::from(q.x), f64::from(q.y));
                    if seglen > alen {
                        break;
                    }
                    cpt -= 1;
                    p0 = q;
                    s += 1;
                    alen -= seglen;
                }
                if cpt > 1 {
                    // shorten line segment
                    let q = at(1, s, p0);
                    self.polyline_adjust(&mut p0, q, alen);
                }
            }
            if let Some(fa) = &l.for_arrow {
                let mut alen = self.arrow_length(fa);
                let mut q = p0;
                while cpt > 1 {
                    q = at(cpt - 2, s, p0);
                    let p = at(cpt - 1, s, p0);
                    let seglen = distance(f64::from(p.x), f64::from(p.y), f64::from(q.x), f64::from(q.y));
                    if seglen > alen {
                        break;
                    }
                    cpt -= 1;
                    pn = q;
                    alen -= seglen;
                }
                if cpt > 1 {
                    self.polyline_adjust(&mut pn, q, alen);
                }
            }
            if cpt <= 1 {
                // all line segments are removed: skip drawing them
                draw = false;
            }
        }
        if draw {
            // Windows 95/98/Me: maximum points allowed is approx. 1360
            if cpt > 1360 && self.level != Level::WinNt {
                self.msg("Warning: polyline has too many points -- may be partially invisible");
            }
            let mut out_pts: Vec<Point> = (0..cpt.saturating_sub(1)).map(|u| at(u, s, p0)).collect();
            out_pts.push(pn);
            self.poly_record(EMR_POLYLINE16, EMR_POLYLINE, bbx, &out_pts);
        }
        // draw_arrows:
        if n == 1 {
            if l.for_arrow.is_some() || l.back_arrow.is_some() {
                self.msg("Warning: Arrow at zero-length line segment omitted.");
            }
            return;
        }
        // At least two different points now
        if let (Some(ba), Some(&p), Some(&q)) = (&l.back_arrow, pts.first(), pts.get(1)) {
            let (ok, dir, _) = direction(p, q);
            if ok {
                self.arrow(p, ba, l.pen_color, dir);
            }
        }
        if let Some(fa) = &l.for_arrow {
            if let (Some(&p), Some(&q)) = (pts.last(), n.checked_sub(2).and_then(|i| pts.get(i))) {
                let (ok, dir, _) = direction(p, q);
                if ok {
                    self.arrow(p, fa, l.pen_color, dir);
                }
            }
        }
    }

    /// The first and third point of a box, or a warning.
    fn box_corners(&mut self, l: &Line) -> Option<(Point, Point)> {
        if let (Some(&a), Some(&c)) = (l.points.first(), l.points.get(2)) {
            Some((a, c))
        } else {
            self.msg("Warning: Invalid fig box omitted.");
            None
        }
    }

    fn rect(&mut self, l: &Line) {
        if let Some((a, c)) = self.box_corners(l) {
            self.rec(&[EMR_RECTANGLE, 24, a.x as u32, a.y as u32, c.x as u32, c.y as u32]);
        }
    }

    fn roundrect(&mut self, l: &Line) {
        if let Some((a, c)) = self.box_corners(l) {
            let r = l.radius.wrapping_mul(2) as u32;
            self.rec(&[EMR_ROUNDRECT, 32, a.x as u32, a.y as u32, c.x as u32, c.y as u32, r, r]);
        }
    }

    fn ellipse_record(&mut self, e: &Ellipse, rx: i32, ry: i32) {
        self.rec(&[
            EMR_ELLIPSE,
            24,
            e.center.x.wrapping_sub(rx) as u32,
            e.center.y.wrapping_sub(ry) as u32,
            e.center.x.wrapping_add(rx) as u32,
            e.center.y.wrapping_add(ry) as u32,
        ]);
    }

    /// Draws a rotated ellipse as a polygon.
    fn rotated_ellipse(&mut self, e: &Ellipse) {
        let delta = 2. * std::f64::consts::PI / ELLIPSE_NPOINT as f64;
        let (sina, cosa) = (e.angle.sin(), e.angle.cos());
        let mut pnt = Vec::with_capacity(ELLIPSE_NPOINT);
        for i in 0..ELLIPSE_NPOINT {
            let th = delta * i as f64;
            let ex = f64::from(e.radiuses.x) * th.cos();
            let ey = f64::from(e.radiuses.y) * th.sin();
            pnt.push(Point::new(
                e.center.x.wrapping_add(cosa.mul_add(ex, sina * ey) as i32),
                e.center.y.wrapping_add((-sina).mul_add(ex, cosa * ey) as i32),
            ));
        }
        let s = Sh {
            style: e.style,
            thickness: e.thickness,
            pen_color: e.pen_color,
            fill_color: e.fill_color,
            fill_style: e.fill_style,
        };
        self.shape(s, 0, 0);
        self.polygon(&pnt);
    }

    fn do_arc(&mut self, a0: &Arc) {
        let mut a = a0.clone();
        if Emf::cwarc(&a) {
            // make counter clockwise arc: swap endpoints and arrows
            a.point.swap(0, 2);
            std::mem::swap(&mut a.for_arrow, &mut a.back_arrow);
        }
        let s = Sh {
            style: a.style,
            thickness: a.thickness,
            pen_color: a.pen_color,
            fill_color: a.fill_color,
            fill_style: a.fill_style,
        };
        match a.type_ {
            T_OPEN_ARC => {
                // open arcs should be drawn separately
                if a.fill_style >= 0 {
                    self.edgevis(0);
                    self.fillstyle(a.fill_color, a.fill_style, a.pen_color);
                    self.arcinterior(&a);
                }
                if a.thickness > 0 {
                    self.edgeattr(1, a.style, a.thickness, a.pen_color, 0, a.cap_style);
                    self.arcoutline(&a);
                }
            }
            T_PIE_WEDGE_ARC => {
                self.shape(s, a.cap_style, 0);
                self.arcoutline(&a);
            }
            t => self.msg(format!("Unsupported fig arc type {t}.")),
        }
    }

    fn do_ellipse(&mut self, e: &Ellipse) {
        let s = Sh {
            style: e.style,
            thickness: e.thickness,
            pen_color: e.pen_color,
            fill_color: e.fill_color,
            fill_style: e.fill_style,
        };
        match e.type_ {
            T_ELLIPSE_BY_RAD | T_ELLIPSE_BY_DIA => {
                if e.angle.abs() > EPSILON && e.angle.sin().abs() > EPSILON {
                    self.rotated_ellipse(e);
                } else {
                    self.shape(s, 0, 0);
                    self.ellipse_record(e, e.radiuses.x, e.radiuses.y);
                }
            }
            T_CIRCLE_BY_RAD | T_CIRCLE_BY_DIA => {
                self.shape(s, 0, 0);
                self.ellipse_record(e, e.radiuses.x, e.radiuses.x);
            }
            t => self.msg(format!("Unsupported FIG ellipse type {t}.")),
        }
    }

    // ---- text ----

    /// `emh_write(.., EMH_RECORD)` of a record given as bytes.
    fn rec_bytes(&mut self, b: &[u8]) {
        self.n_records = self.n_records.wrapping_add(1);
        self.data(b);
    }

    /// `conv_fontindex`: the index into the font tables.
    fn conv_fontindex(&mut self, font: i32, flags: i32) -> i32 {
        if flags & 4 != 0 {
            // PostScript fonts
            if font == -1 {
                return 0;
            }
            if !(0..NUM_PS_FONTS).contains(&font) {
                self.msg(format!("Unsupported Fig PostScript font index {font}."));
                return 0;
            }
            font
        } else {
            if !(0..=NUM_LATEX_FONTS).contains(&font) {
                self.msg(format!("Unsupported Fig LaTeX font index {font}."));
                return 0;
            }
            LT_FONTS.get(font as usize).copied().unwrap_or(0)
        }
    }

    /// `textunicode`: the input bytes of `s` as UTF-16LE code units, each
    /// byte sign-extended, `\n` as 0.
    fn textunicode(&self, s: &[u8]) -> Vec<u8> {
        let cut = s.split(|&b| b == 0).next().unwrap_or_default();
        let mut v = Vec::new();
        for &b in cut {
            let u: u16 = if b == b'\n' { 0 } else { b as i8 as i16 as u16 };
            v.extend_from_slice(&u.to_le_bytes());
        }
        v
    }

    /// `textfont`: creates (or finds) the font and selects it.
    fn textfont(&mut self, font: i32, size: f64, angle: i32) {
        let fontsz = (size.mul_add(1200.0 / 72.27 * 0.88, 0.5) as i32).wrapping_neg();
        let key = [font, fontsz, angle, 0];
        let handle;
        let hi;
        if let Some(i) = self.find_handle(HType::Font, key, 3) {
            self.use_handle(i);
            handle = self.hs.get(i).map_or(0, |h| h.handle);
            hi = Some(i);
        } else {
            let i = self.get_handle(HType::Font);
            handle = self.hs.get(i).map_or(0, |h| h.handle);
            self.use_handle(i);
            if let Some(h) = self.hs.get_mut(i) {
                h.v = key;
            }
            hi = Some(i);
            let f = usize::try_from(font).unwrap_or(0);
            let mut r = vec![0u8; 332];
            let mut put = |off: usize, v: u32| {
                if let Some(d) = r.get_mut(off..off + 4) {
                    d.copy_from_slice(&v.to_le_bytes());
                }
            };
            put(0, EMR_EXTCREATEFONTINDIRECTW);
            put(4, 332);
            put(8, handle);
            put(12, fontsz as u32);
            put(20, angle as u32);
            put(24, angle as u32);
            put(28, LF_WEIGHT.get(f).copied().unwrap_or(0));
            let italic = LF_ITALIC.get(f).copied().unwrap_or(0);
            let charset = if f >= 32 { 2 } else { 0 };
            let pitch = if (12..16).contains(&f) { 0x31 } else { 0x12 };
            if let Some(d) = r.get_mut(32..36) {
                d.copy_from_slice(&[italic, 0, 0, charset]);
            }
            if let Some(d) = r.get_mut(36..40) {
                d.copy_from_slice(&[0, 0, 0, pitch]);
            }
            let name = self.textunicode(LF_FACE_NAME.get(f).copied().unwrap_or("").as_bytes());
            if let Some(d) = r.get_mut(40..40 + name.len().min(64)) {
                d.copy_from_slice(&name[..d.len()]);
            }
            self.rec_bytes(&r);
        }
        // chkcache(handle, lasthandle[EMFH_FONT])
        if handle == self.last_handle[HType::Font as usize] {
            return;
        }
        self.last_handle[HType::Font as usize] = handle;
        self.clear_current_handle(HType::Font);
        if let Some(h) = hi.and_then(|i| self.hs.get_mut(i)) {
            h.is_current = true;
        }
        self.select_object(handle);
    }

    /// `text`: EMR_EXTTEXTOUTW.
    fn text_out(&mut self, x: i32, y: i32, bbx: [i32; 4], s: &[u8]) {
        let u = self.textunicode(s);
        let n_unicode = u.len();
        let padded = (n_unicode + 4) & !3;
        let mut r: Vec<u32> = vec![
            EMR_EXTTEXTOUTW,
            (76 + padded) as u32,
            bbx[0] as u32,
            bbx[1] as u32,
            bbx[2] as u32,
            bbx[3] as u32,
            1, // GM_COMPATIBLE
            0x4007_7777,
            0x4007_7777,
            x as u32,
            y as u32,
            (n_unicode / 2) as u32,
            76,
        ];
        r.resize(19, 0);
        self.rec(&r);
        let mut d = u;
        d.resize(padded, 0);
        self.data(&d);
    }

    fn textcolr(&mut self, color: i32) {
        self.bkmode(TRANSPARENT);
        if color == self.text_color {
            return;
        }
        self.text_color = color;
        let rgb = self.conv_color(color);
        self.rec(&[EMR_SETTEXTCOLOR, 12, rgb as u32]);
    }

    fn textalign(&mut self, align: i32) {
        if align == self.text_align {
            return;
        }
        self.text_align = align;
        self.rec(&[EMR_SETTEXTALIGN, 12, align as u32]);
    }

    /// `genemf_text`.
    fn genemf_text(&mut self, t: &Text) {
        self.textcolr(t.color);
        let font = self.conv_fontindex(t.font, t.flags);
        let (sin_t, cos_t) = (t.angle.sin(), t.angle.cos());
        let (mut x, mut y) = (t.base_x, t.base_y);
        match t.type_ {
            T_LEFT_JUSTIFIED => {}
            T_CENTER_JUSTIFIED => {
                x = (-(t.length / 2.0)).mul_add(cos_t, f64::from(x)) as i32;
                y = (t.length / 2.0).mul_add(sin_t, f64::from(y)) as i32;
            }
            T_RIGHT_JUSTIFIED => {
                x = (-t.length).mul_add(cos_t, f64::from(x)) as i32;
                y = t.length.mul_add(sin_t, f64::from(y)) as i32;
            }
            ty => self.msg(format!("Unsupported fig text type {ty}.")),
        }
        let (xf, yf) = (f64::from(x), f64::from(y));
        let rnd = |v: f64| round(v);
        let bbx = [
            rnd((-t.size).mul_add(sin_t, xf)),
            rnd((-t.size).mul_add(cos_t, yf)),
            rnd(t.length.mul_add(cos_t, xf)),
            rnd(t.length.mul_add(sin_t, yf)),
        ];
        match t.type_ {
            T_LEFT_JUSTIFIED => self.textalign(TA_LEFT | TA_BASELINE),
            T_CENTER_JUSTIFIED => self.textalign(TA_CENTER | TA_BASELINE),
            T_RIGHT_JUSTIFIED => self.textalign(TA_RIGHT | TA_BASELINE),
            _ => {}
        }
        let a = t.angle;
        let half = if a * RAD_01DEG >= 0.0 { 0.5 } else { -0.5 };
        self.textfont(font, t.size, a.mul_add(RAD_01DEG, half) as i32);
        self.text_out(t.base_x, t.base_y, bbx, &t.raw);
    }

    // ---- pictures ----

    /// `encode_bitmap`: the pixels bottom to top, each line padded to 4 bytes.
    fn encode_bitmap(&mut self, pic: &Pic, bpp: i32, rot: i32) {
        let (img_w, img_h) = (pic.bit_size.x, pic.bit_size.y);
        let (mut diri, mut dirj) = match rot {
            0 => (1, -1),
            90 => (1, 1),
            180 => (-1, 1),
            270 => (-1, -1),
            _ => {
                self.msg(format!("fig2dev: emf: pic rotation {rot} is not supported"));
                return;
            }
        };
        if pic.flipped != 0 && (rot == 0 || rot == 180) {
            diri = -diri;
            dirj = -dirj;
        }
        let flip = is_flip(rot, pic.flipped);
        let (mut initi, mut endi) = if diri < 0 { (img_w - 1, 0) } else { (0, img_w - 1) };
        let (mut initj, mut endj) = if dirj < 0 { (img_h - 1, 0) } else { (0, img_h - 1) };
        if flip {
            std::mem::swap(&mut initi, &mut initj);
            std::mem::swap(&mut endi, &mut endj);
            std::mem::swap(&mut diri, &mut dirj);
        }
        if img_w <= 0 || img_h <= 0 || !matches!(bpp, 1 | 4 | 8 | 24) {
            return;
        }
        endi += diri;
        endj += dirj;
        let mut out: Vec<u8> = Vec::new();
        let mut freebits: i32 = 32;
        let mut bits: u32 = 0;
        let px = |k: i64| {
            pic.bitmap
                .get(usize::try_from(k).unwrap_or(usize::MAX))
                .copied()
                .unwrap_or(0)
        };
        let mut j = initj;
        while j != endj {
            let mut i = initi;
            while i != endi {
                let pos = if flip {
                    i64::from(i) * i64::from(img_w) + i64::from(j)
                } else {
                    i64::from(j) * i64::from(img_w) + i64::from(i)
                };
                let n = if bpp == 24 { 3 } else { 1 };
                for u in 0..n {
                    let (v, w) = if bpp == 24 {
                        (px(pos * 3 + u), 8)
                    } else {
                        (px(pos), bpp)
                    };
                    freebits -= w;
                    bits |= u32::from(v) << freebits;
                    if freebits == 0 {
                        out.extend_from_slice(&bits.to_be_bytes());
                        freebits = 32;
                        bits = 0;
                    }
                }
                i += diri;
            }
            if freebits != 32 {
                out.extend_from_slice(&bits.to_be_bytes());
                freebits = 32;
                bits = 0;
            }
            j += dirj;
        }
        self.data(&out);
    }

    /// `picbox`: a PNG as EMR_STRETCHDIBITS (other formats are unsupported).
    fn picbox(&mut self, ctx: &Ctx, l: &Line) {
        let (Some(pic0), Some(&p0), Some(&p2)) = (l.pic.as_deref(), l.points.first(), l.points.get(2)) else {
            return;
        };
        let (dx, dy) = (p2.x.wrapping_sub(p0.x), p2.y.wrapping_sub(p0.y));
        let rotation = if dx < 0 && dy < 0 {
            180
        } else if dx < 0 && dy >= 0 {
            270
        } else if dy < 0 && dx >= 0 {
            90
        } else {
            0
        };
        let Ok(stream) = open_stream_pic(ctx, pic0) else {
            self.msg(format!("fig2dev: {}: No such picture file", pic0.file));
            return;
        };
        if stream.data.len() < 16 {
            self.msg(format!("fig2dev: {}: short read\n", pic0.file));
            return;
        }
        let mut pic = Pic {
            file: pic0.file.clone(),
            flipped: pic0.flipped,
            ..Pic::default()
        };
        if stream.data.starts_with(b"\x89PNG\r\n\x1a\n") && read_png(ctx, &stream.data, &mut pic).is_err() {
            self.msg(format!("fig2dev: {}: illegal format", pic0.file));
            return;
        }
        if pic.subtype != P_PNG {
            self.msg(format!("fig2dev: {}: emf: unsupported picture format", pic0.file));
            return;
        }
        let (img_w, img_h) = (pic.bit_size.x, pic.bit_size.y);
        let ncol = pic.numcols as u32;
        let flip = is_flip(rotation, pic.flipped);
        // Sizes as C computes them in `int` (wrapping); the image is capped.
        let (w, h) = (i64::from(img_w), i64::from(img_h));
        let (bpp, per, coltabsize): (i32, i64, i32) = if ncol <= 2 {
            (1, 32, 8)
        } else if ncol <= 16 {
            (4, 8, ncol as i32 * 4)
        } else if ncol <= 256 {
            (8, 4, ncol as i32 * 4)
        } else {
            (24, 4, 0)
        };
        let (long, short) = if bpp == 24 { (h * 3, w * 3) } else { (h, w) };
        let rows = (if flip {
            (long + per - 1) / per * w
        } else {
            (short + per - 1) / per * h
        }) as i32;
        let bsize = rows.wrapping_mul(4);
        let (x0, x1) = if dx >= 0 { (p0.x, p2.x) } else { (p2.x, p0.x) };
        let (y0, y1) = if dy >= 0 { (p0.y, p2.y) } else { (p2.y, p0.y) };
        let (cxd, cyd) = (dx.wrapping_abs(), dy.wrapping_abs());
        let (cxs, cys) = if flip { (img_h, img_w) } else { (img_w, img_h) };
        let total = 80i32.wrapping_add(40).wrapping_add(coltabsize).wrapping_add(bsize);
        self.rec(&[
            EMR_STRETCHDIBITS,
            total as u32,
            x0 as u32,
            y0 as u32,
            x1 as u32,
            y1 as u32,
            x0 as u32,
            y0 as u32,
            0,
            0,
            cxs as u32,
            cys as u32,
            80,
            (40 + coltabsize) as u32,
            (80 + 40 + coltabsize) as u32,
            bsize as u32,
            0,
            SRCCOPY,
            cxd as u32,
            cyd as u32,
        ]);
        // the BITMAPINFO
        let mut b = Vec::with_capacity(40);
        for w in [40u32, cxs as u32] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        b.extend_from_slice(&(cys as u32).to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&(bpp as u16).to_le_bytes());
        for w in [0u32, bsize as u32, 2953, 2953, if bpp <= 8 { ncol } else { 0 }, 0] {
            b.extend_from_slice(&w.to_le_bytes());
        }
        self.data(&b);
        if coltabsize != 0 {
            self.coltab[0] = [0; 4];
            self.coltab[1] = [0; 4];
            for u in 0..(ncol as usize).min(256) {
                let c = pic.cmap.get(u).copied().unwrap_or([0; 3]);
                if let Some(e) = self.coltab.get_mut(u) {
                    *e = [c[2], c[1], c[0], 0];
                }
            }
            let n = (coltabsize as usize / 4).min(256);
            let bytes: Vec<u8> = self.coltab.iter().take(n).flatten().copied().collect();
            self.data(&bytes);
        }
        self.encode_bitmap(&pic, bpp, rotation);
    }

    fn do_line(&mut self, ctx: &Ctx, l: &Line) {
        let s = Sh {
            style: l.style,
            thickness: l.thickness,
            pen_color: l.pen_color,
            fill_color: l.fill_color,
            fill_style: l.fill_style,
        };
        match l.type_ {
            T_POLYLINE => {
                // draw the interior
                if l.fill_style >= 0 {
                    self.edgevis(0);
                    self.fillstyle(l.fill_color, l.fill_style, l.pen_color);
                    self.polygon(&l.points);
                }
                if l.points.is_empty() {
                    // C dereferences a null pointer here
                    return;
                }
                if l.points.len() == 1 && l.cap_style == 0 {
                    // single point line, butt style: draw as projecting style
                    // but in smaller size
                    self.edgeattr(
                        1,
                        l.style,
                        l.thickness.wrapping_add(1) / 2,
                        l.pen_color,
                        l.join_style,
                        2,
                    );
                } else {
                    self.edgeattr(1, l.style, l.thickness, l.pen_color, l.join_style, l.cap_style);
                }
                self.polyline(l);
            }
            T_BOX => {
                self.shape(s, l.join_style, l.cap_style);
                self.rect(l);
            }
            T_POLYGON => {
                self.shape(s, l.join_style, l.cap_style);
                self.polygon(&l.points);
            }
            T_ARC_BOX => {
                self.shape(s, 0, 0);
                self.roundrect(l);
            }
            T_PIC_BOX => self.picbox(ctx, l),
            t => self.msg(format!("Unsupported FIG polyline type {t}.")),
        }
    }
}

impl Driver for Emf {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        // C resets this on every option, also on `-l`.
        self.rounded_arrows = false;
        match opt {
            'r' => self.rounded_arrows = true,
            'l' => {
                let a = arg.unwrap_or("");
                if a.eq_ignore_ascii_case("win95") {
                    self.level = Level::Win95;
                } else if a.eq_ignore_ascii_case("win98") {
                    self.level = Level::Win98;
                } else if a.eq_ignore_ascii_case("winnt") {
                    self.level = Level::WinNt;
                } else {
                    ctx.warn(format!("warning: unknown level {a} ignored"));
                }
            }
            'G' | 'L' => {}
            _ => return Err(format!("Argument -{opt} unknown to emf driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &crate::object::Compound) -> Result<(), String> {
        // C reads `$LANG` for `figLanguage` (zh_CN, zh_TW, ja, ko: texts through
        // iconv from EUC and locale fonts); only the default language is
        // ported, those locales are treated as the default one.
        #[cfg(not(target_arch = "wasm32"))]
        if std::env::var("LANG").is_err() {
            self.msg("fig2dev: LANG not defined; assuming C locale");
        }
        self.user_colors = ctx
            .user_colors
            .iter()
            .map(|c| [c.r as u8, c.g as u8, c.b as u8])
            .collect();
        self.base = out.buf.len();

        let mut h = [0u32; HEADER_SIZE / 4];
        h[0] = EMR_HEADER;
        h[10] = ENHMETA_SIGNATURE;
        h[11] = ENHMETA_VERSION;

        // The description string: where the figure came from, then the
        // figure's comments, as UTF-16 (each byte a character).
        let figname = ctx.from.clone().unwrap_or_else(|| "stdin".to_string());
        let mut texts = vec![format!(
            "Converted from {figname} using fig2dev {PACKAGE_VERSION} for {}",
            self.level.name()
        )
        .into_bytes()];
        texts.extend(objects.comments.iter().map(|c| c.raw.clone()));
        let mut desc: Vec<u8> = Vec::new();
        for t in &texts {
            for &b in t {
                let u: u16 = if b == b'\n' { 0 } else { b as i8 as i16 as u16 };
                desc.extend_from_slice(&u.to_le_bytes());
            }
            desc.extend_from_slice(&[0, 0]);
        }
        let n_unicode = desc.len();
        h[15] = (n_unicode / 2) as u32;
        h[16] = HEADER_SIZE as u32;

        // The size of the reference device: 260 x 215 mm at 1200 dpi.
        h[20] = 260;
        h[21] = 215;
        h[18] = round(260. * 47.244094) as u32;
        h[19] = round(215. * 47.244094) as u32;

        // Bounds in device units (1200 dpi) and 0.01 mm units, 1/20 inch
        // added all the way around.
        h[2] = ctx.llx.wrapping_sub(48) as u32;
        h[3] = ctx.lly.wrapping_sub(48) as u32;
        h[4] = ctx.urx.wrapping_add(48) as u32;
        h[5] = ctx.ury.wrapping_add(48) as u32;
        h[6] = sat_u32((f64::from(ctx.llx.wrapping_sub(48)) * PIXEL_01MM).floor());
        h[7] = sat_u32((f64::from(ctx.lly.wrapping_sub(48)) * PIXEL_01MM).floor());
        h[8] = sat_u32((f64::from(ctx.urx.wrapping_add(48)) * PIXEL_01MM).ceil());
        h[9] = sat_u32((f64::from(ctx.ury.wrapping_add(48)) * PIXEL_01MM).ceil());

        let padded = (n_unicode + 3) & !3;
        desc.resize(padded, 0);
        h[1] = (HEADER_SIZE + padded) as u32;
        self.header = h;
        self.rec(&h);
        self.data(&desc);

        // MM_TEXT: each logical unit is mapped to one device pixel.
        self.rec(&[EMR_SETMAPMODE, 12, MM_TEXT]);
        self.flush(ctx, out);
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        self.do_arc(a);
        self.flush(ctx, out);
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        self.do_ellipse(e);
        self.flush(ctx, out);
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        self.do_line(ctx, l);
        self.flush(ctx, out);
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        // The comments go into the file, as in C.
        print_comments(out, "% ", &s.comments, "");
        if !self.warned_spline {
            self.msg("Warning: the EMF driver does not support (old style) FIG splines.");
            self.msg("Suggestion: convert your (old?) FIG image by loading it into xfig v3.2 or");
            self.msg("higher and saving again.");
            self.warned_spline = true;
        }
        self.flush(ctx, out);
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String> {
        self.genemf_text(t);
        self.flush(ctx, out);
        Ok(())
    }

    fn end(&mut self, ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        // delete all created handles
        for i in self.order.clone() {
            self.delete_handle(i);
        }
        self.rec(&[EMR_EOF, 20, 0, 16, 20]);
        self.flush(ctx, out);

        // Rewrite the updated header record at the beginning of the file.
        self.header[12] = self.n_bytes;
        self.header[13] = self.n_records;
        // the size of the handle table is the last handle number + 1
        self.header[14] = self.n_handles.wrapping_add(1) & 0xFFFF;
        let mut bytes = Vec::with_capacity(HEADER_SIZE);
        for w in self.header {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        let Some(dst) = out.buf.get_mut(self.base..self.base + HEADER_SIZE) else {
            ctx.warn("fig2dev: error: failed to write EMF header");
            return Ok(-1);
        };
        dst.copy_from_slice(&bytes);
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_and_fills() {
        let e = Emf::default();
        assert_eq!(e.conv_color(DEFAULT), 0);
        assert_eq!(e.conv_color(4), 0x0000FF);
        assert_eq!(e.conv_fill_color(20, 1), 0xFF0000);
        assert_eq!(e.conv_fill_color(40, 1), 0xFFFFFF);
        assert_eq!(e.conv_fill_color(0, 1), 0);
    }

    #[test]
    fn pattern_bits_are_flipped_and_padded() {
        let mut bits = vec![0u8; 32];
        Emf::create_brush_pattern(&mut bits, 16);
        assert_eq!(bits.len(), 32);
        assert!(bits.iter().any(|&b| b != 0));
    }
}
