//! fig2dev's `gbx` driver: Gerber RS-274X, the CAD format of printed
//! circuit boards (by Edward Grace).
//!
//! Only black and default-coloured objects are written. Polylines,
//! polygons and boxes become drawn or area-filled paths, with apertures
//! chosen from the join and cap styles; arcs and circles become circular
//! interpolations; ellipses, splines, texts and arrowheads are not written
//! (a warning says so, once). Like fig2dev, the driver writes absolute
//! coordinates in the figure's own orientation and keeps its quirks: the
//! `%SF…%` parameter lacks its `*`, circular aperture definitions end in
//! `%*`, and an exposure-off code at the end of an object waits on the
//! line for the next block.
// Ported from fig2dev dev/gengbx.c (xfig licence, see LICENSE-xfig).

use crate::fprintf;
use crate::gendev::{strtod_prefix, PACKAGE_VERSION, PROG};
use crate::object::{
    Arc, DPoint, Ellipse, Line, Point, Spline, Text, T_ARC_BOX, T_BOX, T_CIRCLE_BY_DIA, T_CIRCLE_BY_RAD, T_POLYGON,
};
use crate::util::creationdate::creation_date;
use crate::{round, Ctx, Driver, Out};

const GBX_DRIVER_VERSION: &str = "0.1.1";

/// D codes from 10 on select apertures; the largest D code is 999.
const D_SELECT_APERTURE: i64 = 10;
const MAX_D_CODE: i64 = 999;
const MAX_APERTURE_INDEX: i64 = MAX_D_CODE - D_SELECT_APERTURE;

/// G codes (`enum g_codes`).
const G_LINE_INTERP_1X: u32 = 1;
const G_CIRC_INTERP_CLOCK: u32 = 2;
const G_CIRC_INTERP_ANTI: u32 = 3;
const G_IGNORE_DATA_BLOCK: u32 = 4;
const G_POLY_AREA_FILL_ON: u32 = 36;
const G_POLY_AREA_FILL_OFF: u32 = 37;
const G_TOOL_PREPARE: u32 = 54;
const G_CIRC_INTERP_DISABLE: u32 = 74;
const G_CIRC_INTERP_ENABLE: u32 = 75;

/// D codes (`enum d_codes`).
const D_EXPOSURE_ON: i64 = 1;
const D_EXPOSURE_OFF: i64 = 2;
const D_FLASH: i64 = 3;

/// M code `end_of_program`.
const M_END_OF_PROGRAM: u32 = 2;

/// Cap styles and the "no fill" fill style.
const CAP_STYLE_BUTT: i32 = 0;
const CAP_STYLE_SQUARE: i32 = 2;
const FILL_STYLE_NONE: i32 = -1;

/// The pen colours written: default and black.
const P_COLOUR_DEFAULT: i32 = -1;
const P_COLOUR_BLACK: i32 = 0;

const XY_FIG2IN: f64 = 1.0 / 1200.0;
const XY_FIG2MM: f64 = XY_FIG2IN * 25.4;

/// The warnings given at most once (`enum warnings`).
#[derive(Clone, Copy)]
enum Warn {
    SquareJoin,
    FilledPolyWithLine,
    OpenFilledPolygon,
    OpenFilledArc,
    TextUnsupported,
    EllipseUnsupported,
    SplineNotImplemented,
    ArrowNotImplemented,
    LineColour,
    LineZeroWidth,
}

const NUM_WARNINGS: usize = 10;

/// The state of fig2dev's gengbx.c: its options and what it has written.
struct Gbx {
    /// Digits before and after the decimal point (`-f`).
    before: u32,
    after: u32,
    /// Millimetres (`-d mm`, the default) or inches (`-d in`).
    mm: bool,
    /// `-g AxB+C+D`.
    scale_a: f32,
    scale_b: f32,
    offset_a: f32,
    offset_b: f32,
    /// `-p`: negative (the default) or positive.
    negative: bool,
    /// `-v`: trace comments.
    debug_comments: bool,
    warned: [bool; NUM_WARNINGS],
    is_exposure: bool,
    /// 10^after.
    scale_factor: u64,
    count_apertures: i64,
    /// Diameters and widths stored at their aperture index; `-1` unset.
    circ_list: Vec<f64>,
    square_list: Vec<f64>,
    count_circ: usize,
    count_square: usize,
}

/// The driver for `lang` (`gbx`).
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    (lang == "gbx").then(|| {
        Box::new(Gbx {
            before: 3,
            after: 5,
            mm: true,
            scale_a: 1.0,
            scale_b: 1.0,
            offset_a: 0.0,
            offset_b: 0.0,
            negative: true,
            debug_comments: false,
            warned: [false; NUM_WARNINGS],
            is_exposure: true,
            scale_factor: 0,
            count_apertures: 0,
            circ_list: vec![-1.0; MAX_APERTURE_INDEX as usize],
            square_list: vec![-1.0; MAX_APERTURE_INDEX as usize],
            count_circ: 0,
            count_square: 0,
        }) as Box<dyn Driver>
    })
}

/// `length`: the length of the vector from (x1, y1) to (x2, y2), computed
/// as fig2dev does.
fn length(x1: i64, y1: i64, x2: i64, y2: i64) -> f64 {
    let dx = x2.wrapping_sub(x1) as f64;
    let dy = y2.wrapping_sub(y1) as f64;
    if dx.abs() > dy.abs() {
        dx.abs() * (1.0 + (dy / dx) * (dy / dx)).sqrt()
    } else {
        dy.abs() * (1.0 + (dx / dy) * (dx / dy)).sqrt()
    }
}

/// `get_dx_dy`: the step of length `dist` from point 1 towards point 2.
fn get_dx_dy(x1: i64, y1: i64, x2: i64, y2: i64, dist: i64) -> (f64, f64) {
    let l = length(x1, y1, x2, y2);
    (
        dist.wrapping_mul(x2.wrapping_sub(x1)) as f64 / l,
        dist.wrapping_mul(y2.wrapping_sub(y1)) as f64 / l,
    )
}

/// `is_double_equal`.
fn is_double_equal(a: f64, b: f64) -> bool {
    const DIFF: f64 = 1e-6;
    a - DIFF < b && a + DIFF > b
}

/// `sscanf`'s `%f` into a float: the value and the bytes used.
fn scan_f32(s: &str) -> Option<(f32, usize)> {
    let (_, n) = strtod_prefix(s)?;
    let v = s.get(..n)?.trim_start().parse::<f32>().ok()?;
    Some((v, n))
}

/// `sscanf`'s `%u`: an optionally signed decimal number, negative values
/// wrapping around; the value and the bytes used.
fn scan_u32(s: &str) -> Option<(u32, usize)> {
    let t = s.trim_start();
    let skip = s.len() - t.len();
    let b = t.as_bytes();
    let mut i = 0;
    let neg = match b.first() {
        Some(b'-') => {
            i = 1;
            true
        }
        Some(b'+') => {
            i = 1;
            false
        }
        _ => false,
    };
    let start = i;
    let mut v: u64 = 0;
    while let Some(d) = b.get(i).filter(|c| c.is_ascii_digit()) {
        v = v.saturating_mul(10).saturating_add(u64::from(d - b'0'));
        i += 1;
    }
    if i == start {
        return None;
    }
    let v = if neg { v.wrapping_neg() } else { v };
    Some((v as u32, skip + i))
}

/// `sscanf(s, "%fx%f+%f+%f", …)`: the values converted until the first
/// mismatch; `None` (`EOF`) for an empty input.
fn scan_geometry(s: &str) -> Option<Vec<f32>> {
    if s.trim_start().is_empty() {
        return None;
    }
    let mut vals = Vec::new();
    let mut rest = s;
    for sep in ["x", "+", "+", ""] {
        let Some((v, n)) = scan_f32(rest) else {
            break;
        };
        vals.push(v);
        rest = rest.get(n..).unwrap_or("");
        match rest.strip_prefix(sep) {
            Some(r) if !sep.is_empty() => rest = r,
            _ => break,
        }
    }
    Some(vals)
}

/// `sscanf(s, "%u.%u", …)`.
fn scan_digits(s: &str) -> Option<Vec<u32>> {
    if s.trim_start().is_empty() {
        return None;
    }
    let mut vals = Vec::new();
    if let Some((v, n)) = scan_u32(s) {
        vals.push(v);
        if let Some(r) = s.get(n..).and_then(|r| r.strip_prefix('.')) {
            if let Some((v, _)) = scan_u32(r) {
                vals.push(v);
            }
        }
    }
    Some(vals)
}

impl Gbx {
    /// `warn_once`.
    fn warn_once(&mut self, ctx: &mut Ctx, w: Warn, message: &str) {
        if let Some(done) = self.warned.get_mut(w as usize) {
            if !*done {
                ctx.warn(format!("Warning: {message}"));
                *done = true;
            }
        }
    }

    /// `is_pen_colour_ok`: only default and black objects are written.
    fn is_pen_colour_ok(&mut self, ctx: &mut Ctx, colour: i32) -> bool {
        if colour == P_COLOUR_BLACK || colour == P_COLOUR_DEFAULT {
            return true;
        }
        self.warn_once(
            ctx,
            Warn::LineColour,
            "By default only objects with line colour of \"default\" or  \"black\" are output.",
        );
        false
    }

    fn conversion_factor(&self) -> f64 {
        if self.mm {
            XY_FIG2MM
        } else {
            XY_FIG2IN
        }
    }

    /// `width_fig2gbx`.
    fn width_fig2gbx(&self, ctx: &Ctx, f: i64) -> f64 {
        f as f64 * ctx.mag * self.conversion_factor()
    }

    /// `x_fig2gbx`, `y_fig2gbx`.
    fn pos_fig2gbx(&self, ctx: &Ctx, f: i64) -> i32 {
        (f as f64 * ctx.mag * self.conversion_factor() * self.scale_factor as f64) as i32
    }

    fn write_end_block(out: &mut Out) {
        out.puts("*\n");
    }

    fn write_g_code(out: &mut Out, g: u32) {
        fprintf!(out, "G{g:02}");
    }

    fn write_m_code(out: &mut Out, m: u32) {
        fprintf!(out, "M{m:02}");
        Self::write_end_block(out);
    }

    fn write_d_code(out: &mut Out, d: i64) {
        if d < 10 {
            fprintf!(out, "D{d:02}");
        } else {
            fprintf!(out, "D{d}");
        }
    }

    fn exposure_off(&mut self, out: &mut Out) {
        Self::write_d_code(out, D_EXPOSURE_OFF);
        self.is_exposure = false;
    }

    fn exposure_on(&mut self, out: &mut Out) {
        Self::write_d_code(out, D_EXPOSURE_ON);
        self.is_exposure = true;
    }

    fn use_aperture(out: &mut Out, index: i64) {
        Self::write_g_code(out, G_TOOL_PREPARE);
        Self::write_d_code(out, D_SELECT_APERTURE.saturating_add(index));
        Self::write_end_block(out);
    }

    fn write_comment(out: &mut Out, c: &str) {
        fprintf!(out, "G{:02} {c}*\n", G_IGNORE_DATA_BLOCK);
    }

    fn write_trace(&self, out: &mut Out, c: &str) {
        if self.debug_comments {
            Self::write_comment(out, c);
        }
    }

    fn xy(&self, ctx: &Ctx, out: &mut Out, x: i64, y: i64) {
        fprintf!(out, "X{}Y{}", self.pos_fig2gbx(ctx, x), self.pos_fig2gbx(ctx, y));
    }

    fn draw_xy(&mut self, ctx: &Ctx, out: &mut Out, p: Point) {
        self.xy(ctx, out, p.x.into(), p.y.into());
        if !self.is_exposure {
            self.exposure_on(out);
        }
        Self::write_end_block(out);
    }

    fn move_xy(&mut self, ctx: &Ctx, out: &mut Out, p: Point) {
        self.xy(ctx, out, p.x.into(), p.y.into());
        self.exposure_off(out);
        Self::write_end_block(out);
    }

    fn flash_xy(&mut self, ctx: &Ctx, out: &mut Out, p: Point, aperture: i64) {
        if self.is_exposure {
            self.exposure_off(out);
        }
        Self::write_end_block(out);
        Self::write_g_code(out, G_TOOL_PREPARE);
        Self::write_d_code(out, D_SELECT_APERTURE.saturating_add(aperture));
        Self::write_end_block(out);
        self.xy(ctx, out, p.x.into(), p.y.into());
        Self::write_d_code(out, D_FLASH);
        Self::write_end_block(out);
    }

    /// `abort_too_many_apertures`.
    fn check_aperture_count(&self) -> Result<(), String> {
        if self.count_apertures < MAX_APERTURE_INDEX {
            return Ok(());
        }
        Err(format!(
            "Error: Maximum number of possible apertures ({MAX_APERTURE_INDEX}) exceeded."
        ))
    }

    /// `ad_aperture_define_circ`: the index of a circular aperture of this
    /// diameter, defined on first use. Like fig2dev, the lookup scans only
    /// the first `count_circ` entries of a list indexed by aperture number.
    fn define_circ(&mut self, out: &mut Out, dia: f64) -> Result<i64, String> {
        if dia.is_nan() || dia <= 0.0 {
            return Err("Error: Something tried to define a circular aperture of zero size.".into());
        }
        let found = self
            .circ_list
            .iter()
            .take(self.count_circ)
            .position(|&d| is_double_equal(d, dia));
        if let Some(i) = found {
            return Ok(i as i64);
        }
        self.check_aperture_count()?;
        let index = self.count_apertures;
        fprintf!(out, "%ADD{}C,{dia:.6}%*\n", D_SELECT_APERTURE + index);
        if let Some(slot) = self.circ_list.get_mut(index as usize) {
            *slot = dia;
        }
        self.count_circ += 1;
        self.count_apertures += 1;
        Ok(index)
    }

    /// `ad_aperture_define_square`.
    fn define_square(&mut self, out: &mut Out, width: f64) -> Result<i64, String> {
        if width.is_nan() || width <= 0.0 {
            return Err("Error: Something tried to define a square aperture of zero size.".into());
        }
        let found = self
            .square_list
            .iter()
            .take(self.count_square)
            .position(|&w| is_double_equal(w, width));
        if let Some(i) = found {
            return Ok(i as i64);
        }
        self.check_aperture_count()?;
        let index = self.count_apertures;
        fprintf!(out, "%ADD{}R,{width:.6}X{width:.6}*%\n", D_SELECT_APERTURE + index);
        if let Some(slot) = self.square_list.get_mut(index as usize) {
            *slot = width;
        }
        self.count_square += 1;
        self.count_apertures += 1;
        Ok(index)
    }

    /// `define_aperture_for_line`: round for round joins, else square.
    fn aperture_for_line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<i64, String> {
        let width = self.width_fig2gbx(ctx, l.thickness.into());
        if l.join_style == 1 {
            self.define_circ(out, width)
        } else {
            let index = self.define_square(out, width)?;
            if l.points.len() > 3 {
                self.warn_once(
                    ctx,
                    Warn::SquareJoin,
                    "Join style of square does not translate well to intermediate points in Gerber format.",
                );
            }
            Ok(index)
        }
    }

    /// `define_aperture_for_line_end`: round for round caps, else square.
    fn aperture_for_line_end(&mut self, ctx: &Ctx, out: &mut Out, l: &Line) -> Result<i64, String> {
        let width = self.width_fig2gbx(ctx, l.thickness.into());
        if l.cap_style == 1 {
            self.define_circ(out, width)
        } else {
            self.define_square(out, width)
        }
    }

    /// `define_aperture_for_arc`.
    fn aperture_for_arc(&mut self, ctx: &Ctx, out: &mut Out, a: &Arc) -> Result<i64, String> {
        let width = self.width_fig2gbx(ctx, a.thickness.into());
        if a.cap_style == 1 {
            self.define_circ(out, width)
        } else {
            self.define_square(out, width)
        }
    }

    /// `gengbx_line` on a copy of the line it may change: butt caps and
    /// round joins move and add points, and a filled polygon with an edge
    /// is drawn as its outline first.
    fn draw_line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &mut Line) -> Result<(), String> {
        if !self.is_pen_colour_ok(ctx, l.pen_color) {
            return Ok(());
        }
        if l.thickness <= 0 && l.fill_style == FILL_STYLE_NONE {
            self.warn_once(ctx, Warn::LineZeroWidth, "Unfilled line of zero width, ignoring.");
            return Ok(());
        }
        let Some(&first) = l.points.first() else {
            return Ok(());
        };

        // All points at one place: flash the aperture there.
        let mut same = first;
        let mut is_same_place = true;
        for &p in l.points.iter().skip(1) {
            is_same_place = p == same;
            if !is_same_place {
                break;
            }
            same = p;
        }
        if is_same_place {
            let ap = self.aperture_for_line(ctx, out, l)?;
            self.flash_xy(ctx, out, same, ap);
            return Ok(());
        }

        if l.fill_style == FILL_STYLE_NONE {
            self.write_trace(out, "## START:POLYGON:OPEN");
        } else {
            self.write_trace(out, "## START:POLYGON:CLOSED");
        }

        let n_points_original = l.points.len();
        // The penultimate point (the first, for one point).
        let mut penult = n_points_original.saturating_sub(2);
        let half = i64::from(l.thickness / 2);

        // Square ends with round joins, long enough to mean something.
        let mut circsquare = false;
        if l.join_style == 1 && (l.cap_style == CAP_STYLE_BUTT || l.cap_style == CAP_STYLE_SQUARE) {
            if n_points_original > 2 {
                circsquare = true;
            } else {
                l.join_style = 0;
            }
        }

        if l.fill_style == FILL_STYLE_NONE {
            if l.type_ != T_BOX && l.type_ != T_ARC_BOX && l.type_ != T_POLYGON {
                // Butt caps: pull the ends in by half the thickness.
                if l.cap_style == CAP_STYLE_BUTT && n_points_original > 1 {
                    shift(&mut l.points, 0, 1, half);
                    shift(&mut l.points, penult + 1, penult, half);
                }
                if circsquare {
                    if n_points_original >= 3 {
                        // Make way for a round join after the second point:
                        // two copies of it, the first copy flashed.
                        let Some(&p2) = l.points.get(1) else {
                            return Ok(());
                        };
                        l.points.insert(2, p2);
                        l.points.insert(2, p2);
                        if penult > 1 {
                            penult += 2;
                        }
                        shift(&mut l.points, 1, 0, half);
                        // The second copy moves towards the next point.
                        shift(&mut l.points, 3, 4, half);
                    }
                    if n_points_original > 3 {
                        // A new penultimate point, moved forward.
                        let Some(&pp) = l.points.get(penult) else {
                            return Ok(());
                        };
                        l.points.insert(penult + 1, pp);
                        shift(&mut l.points, penult + 1, penult + 2, half);
                    }
                }
            }

            let pts = l.points.clone();
            let at = |i: usize| pts.get(i).copied();
            if circsquare {
                if n_points_original == 3 {
                    // Five points: square segments at both ends, the round
                    // join flashed at the third.
                    let (Some(p0), Some(p1), Some(p2), Some(p3), Some(p4)) = (at(0), at(1), at(2), at(3), at(4)) else {
                        return Ok(());
                    };
                    let ap = self.aperture_for_line_end(ctx, out, l)?;
                    Self::use_aperture(out, ap);
                    self.move_xy(ctx, out, p0);
                    self.draw_xy(ctx, out, p1);
                    let ap = self.aperture_for_line(ctx, out, l)?;
                    self.flash_xy(ctx, out, p2, ap);
                    let ap = self.aperture_for_line_end(ctx, out, l)?;
                    Self::use_aperture(out, ap);
                    self.move_xy(ctx, out, p3);
                    self.draw_xy(ctx, out, p4);
                } else if n_points_original > 3 {
                    // The ends with the cap's aperture, the rest with the
                    // join's.
                    let (Some(p0), Some(p1), Some(p2), Some(p3)) = (at(0), at(1), at(2), at(3)) else {
                        return Ok(());
                    };
                    let ap = self.aperture_for_line_end(ctx, out, l)?;
                    Self::use_aperture(out, ap);
                    self.move_xy(ctx, out, p0);
                    self.draw_xy(ctx, out, p1);
                    let ap = self.aperture_for_line(ctx, out, l)?;
                    Self::use_aperture(out, ap);
                    self.move_xy(ctx, out, p1);
                    self.draw_xy(ctx, out, p2);
                    self.draw_xy(ctx, out, p3);
                    if pts.len() > 4 {
                        self.move_xy(ctx, out, p3);
                        // Up to, not including, the penultimate point.
                        let mut i = 3;
                        while i + 2 < pts.len() {
                            if let Some(p) = at(i) {
                                self.draw_xy(ctx, out, p);
                            }
                            i += 1;
                        }
                        let ap = self.aperture_for_line_end(ctx, out, l)?;
                        Self::use_aperture(out, ap);
                        if let (Some(p), Some(q)) = (at(i), at(i + 1)) {
                            self.move_xy(ctx, out, p);
                            self.draw_xy(ctx, out, q);
                        }
                    }
                }
            } else {
                let ap = self.aperture_for_line(ctx, out, l)?;
                Self::use_aperture(out, ap);
                let mut it = pts.iter();
                if let Some(&p) = it.next() {
                    self.move_xy(ctx, out, p);
                }
                for &p in it {
                    self.draw_xy(ctx, out, p);
                }
            }
            self.write_trace(out, "## END:POLYGON:OPEN");
        } else {
            // A filled polygon with an edge: draw the edge as an unfilled
            // line first (which may move its points), then the area.
            if l.thickness > 0 {
                self.warn_once(
                    ctx,
                    Warn::FilledPolyWithLine,
                    "You have specified a filled polygon with a finite edge thickness. Check if this is what you want",
                );
                l.fill_style = FILL_STYLE_NONE;
                self.draw_line(ctx, out, l)?;
            }
            let pts = l.points.clone();
            let Some(&p1) = pts.first() else {
                return Ok(());
            };
            let mut p2 = Point::new(0, 0);
            self.move_xy(ctx, out, p1);
            Self::write_g_code(out, G_POLY_AREA_FILL_ON);
            Self::write_end_block(out);
            for &p in pts.iter().skip(1) {
                self.draw_xy(ctx, out, p);
                p2 = p;
            }
            Self::write_g_code(out, G_POLY_AREA_FILL_OFF);
            Self::write_end_block(out);
            self.exposure_off(out);
            if p1 != p2 {
                self.warn_once(
                    ctx,
                    Warn::OpenFilledPolygon,
                    "A filled polygon is not closed. This can appear different in the Gerber file.",
                );
            }
            self.write_trace(out, "## END:POLYGON:FILLED");
        }
        if l.for_arrow.is_some() || l.back_arrow.is_some() {
            self.warn_once(
                ctx,
                Warn::ArrowNotImplemented,
                "Arrows on lines are not yet implemented.",
            );
        }
        Ok(())
    }

    /// `gengbx_arc` on a copy: a filled arc with an edge is drawn filled,
    /// then as its outline.
    fn draw_arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &mut Arc) -> Result<(), String> {
        if !self.is_pen_colour_ok(ctx, a.pen_color) {
            return Ok(());
        }
        if a.thickness <= 0 && a.fill_style == FILL_STYLE_NONE {
            self.warn_once(ctx, Warn::LineZeroWidth, "Unfilled arc of zero width, ignoring.");
            return Ok(());
        }
        let filled = a.fill_style != FILL_STYLE_NONE;
        self.write_trace(
            out,
            if filled {
                "## START:ARC:FILLED"
            } else {
                "## START:ARC:OPEN"
            },
        );
        if !filled {
            let ap = self.aperture_for_arc(ctx, out, a)?;
            Self::use_aperture(out, ap);
        }
        let [p0, _, p2] = a.point;
        self.move_xy(ctx, out, p0);
        Self::write_g_code(out, G_CIRC_INTERP_ENABLE);
        Self::write_end_block(out);
        if filled {
            Self::write_g_code(out, G_POLY_AREA_FILL_ON);
            Self::write_end_block(out);
        }
        Self::write_g_code(
            out,
            if a.direction == 1 {
                G_CIRC_INTERP_CLOCK
            } else {
                G_CIRC_INTERP_ANTI
            },
        );
        Self::write_end_block(out);
        // The centre is a double; fig2dev truncates the offsets to long.
        let off = |c: f64, p: i32| (c - f64::from(p)) as i64;
        fprintf!(
            out,
            "X{}Y{}I{}J{}D01*\n",
            self.pos_fig2gbx(ctx, p2.x.into()),
            self.pos_fig2gbx(ctx, p2.y.into()),
            self.pos_fig2gbx(ctx, off(a.center.x, p0.x)),
            self.pos_fig2gbx(ctx, off(a.center.y, p0.y))
        );
        if filled {
            Self::write_g_code(out, G_POLY_AREA_FILL_OFF);
            Self::write_end_block(out);
        }
        Self::write_g_code(out, G_CIRC_INTERP_DISABLE);
        Self::write_end_block(out);
        Self::write_g_code(out, G_LINE_INTERP_1X);
        Self::write_end_block(out);
        self.exposure_off(out);

        if a.thickness > 0 && filled {
            self.write_trace(out, "## START:ARC:OUTLINE:CURVE");
            a.fill_style = FILL_STYLE_NONE;
            self.draw_arc(ctx, out, a)?;
            self.warn_once(
                ctx,
                Warn::OpenFilledArc,
                "Polygon fill on an arc with finite line thickness.  Check this is what you want.",
            );
            self.write_trace(out, "## START:ARC:OUTLINE:LINE");
            self.move_xy(ctx, out, p0);
            self.draw_xy(ctx, out, p2);
            self.write_trace(out, "## END:ARC:OUTLINE:LINE");
            self.write_trace(out, "## END:ARC:OUTLINE:CURVE");
        }
        if a.fill_style == FILL_STYLE_NONE {
            self.write_trace(out, "## END:ARC:OPEN");
        } else {
            self.write_trace(out, "## END:ARC:FILLED");
        }
        Ok(())
    }
}

/// Move point `i` by `dist` towards point `other` (fig2dev's `get_dx_dy`
/// and `p += round(d)`; a step away from the previous point, `p -=
/// round(d)`, is the same as a step towards it). Points that don't exist
/// are left alone.
fn shift(points: &mut [Point], i: usize, other: usize, dist: i64) {
    let (Some(&p), Some(&q)) = (points.get(i), points.get(other)) else {
        return;
    };
    let (dx, dy) = get_dx_dy(p.x.into(), p.y.into(), q.x.into(), q.y.into(), dist);
    if let Some(pt) = points.get_mut(i) {
        pt.x = pt.x.wrapping_add(round(dx));
        pt.y = pt.y.wrapping_add(round(dy));
    }
}

impl Driver for Gbx {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        let a = arg.unwrap_or("");
        match opt {
            'G' | 'L' => {}
            'd' => match a {
                "mm" => self.mm = true,
                "in" => self.mm = false,
                _ => ctx.warn("Dimensions should be given in 'mm' for millimeters or 'in' for inches, respectively"),
            },
            'p' => match a {
                "pos" | "+" => self.negative = false,
                "neg" | "-" => self.negative = true,
                _ => ctx.warn(
                    "Polarity option should be [+,pos] for positive images (marks add material making it opaque) or [-,neg] for negative images (marks remove material, making it transparent)",
                ),
            },
            'g' => {
                if let Some(vals) = scan_geometry(a) {
                    if vals.is_empty() {
                        ctx.warn(
                            "Error in scaling format, expect <double>x<double>+<double>+<double> for X and Y scale and x, y offset, respectively.",
                        );
                    }
                    let fields = [&mut self.scale_a, &mut self.scale_b, &mut self.offset_a, &mut self.offset_b];
                    for (f, v) in fields.into_iter().zip(vals) {
                        *f = v;
                    }
                }
            }
            'f' => {
                if let Some(vals) = scan_digits(a) {
                    if vals.is_empty() {
                        ctx.warn(
                            "Error in numeric format, expect form <unsigned int>.<unsigned int> for degree of precision.",
                        );
                    }
                    let fields = [&mut self.before, &mut self.after];
                    for (f, v) in fields.into_iter().zip(vals) {
                        *f = v;
                    }
                }
                if self.before > 4 {
                    ctx.warn("Warning: Having more than 4 digits before the decimal place is unusual");
                }
                if self.after > 5 {
                    ctx.warn("Warning: Having more than 5 digits after the decimal place is unusual");
                }
                if self.before.wrapping_add(self.after) > 9 {
                    ctx.warn("Warning: Having more than nine significant figures is usually unrealistic");
                }
            }
            'i' => {
                ctx.warn("Warning: Option -i obsolete, use -v (verbose) instead.");
                match a {
                    "off" => self.debug_comments = false,
                    "on" => self.debug_comments = true,
                    _ => ctx.warn("Error: Debug comments option should be 'on' or 'off'"),
                }
            }
            'v' => self.debug_comments = true,
            _ => return Err(format!("Argument -{opt} unknown to gbx driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &crate::object::Compound) -> Result<(), String> {
        self.scale_factor = 10f64.powf(f64::from(self.after)) as u64;
        Self::write_comment(out, "Gerber RS-274x file");
        Self::write_comment(out, &format!("Creator: {PROG}"));
        Self::write_comment(out, &format!("Version: {PACKAGE_VERSION}"));
        Self::write_comment(out, &format!("Driver version: {GBX_DRIVER_VERSION}"));
        Self::write_comment(out, "Author: Edward Grace <edward.grace@gmail.com>");
        if let Some(date) = creation_date(ctx) {
            Self::write_comment(out, &format!("Creation date: {date}"));
        }
        for c in &objects.comments {
            Self::write_comment(out, c);
        }
        out.puts(if self.mm { "%MOMM*%\n" } else { "%MOIN*%\n" });
        fprintf!(
            out,
            "%OFA{:.6}B{:.6}*%\n",
            f64::from(self.offset_a),
            f64::from(self.offset_b)
        );
        fprintf!(out, "%FSLAX{0}{1}Y{0}{1}*%\n", self.before, self.after);
        fprintf!(
            out,
            "%SFA{:.6}B{:.6}%\n",
            f64::from(self.scale_a),
            f64::from(self.scale_b)
        );
        out.puts(if self.negative { "%IPNEG*%\n" } else { "%IPPOS*%\n" });
        out.puts("%LPD*%\n");
        Self::write_comment(
            out,
            "The following is an aperture definition of width pi/10. It should never be used.",
        );
        self.define_square(out, 0.314159)?;
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        let mut a = a.clone();
        self.draw_arc(ctx, out, &mut a)
    }

    /// Circles become full-circle arcs; other ellipses are not written.
    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        if e.type_ == T_CIRCLE_BY_RAD || e.type_ == T_CIRCLE_BY_DIA {
            if !self.is_pen_colour_ok(ctx, e.pen_color) {
                return Ok(());
            }
            self.write_trace(out, "## START:CIRCLE");
            // fig2dev's arc comes from malloc with the remaining fields
            // unset; a fresh allocation is zeroed: black, clockwise, butt.
            let s = std::f64::consts::SQRT_2;
            let px = (f64::from(e.center.x) + f64::from(e.radiuses.x) / s) as i32;
            let py = (f64::from(e.center.y) + f64::from(e.radiuses.y) / s) as i32;
            let mut a = Arc {
                pen: e.pen,
                fill_style: e.fill_style,
                thickness: e.thickness,
                center: DPoint {
                    x: e.center.x.into(),
                    y: e.center.y.into(),
                },
                point: [Point::new(px, py), Point::default(), Point::new(px, py)],
                ..Arc::default()
            };
            self.draw_arc(ctx, out, &mut a)?;
        } else {
            self.warn_once(
                ctx,
                Warn::EllipseUnsupported,
                "Ellipse output is currently unsupported.",
            );
        }
        self.write_trace(out, "## END:CIRCLE");
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        let mut l = l.clone();
        self.draw_line(ctx, out, &mut l)
    }

    fn spline(&mut self, ctx: &mut Ctx, _out: &mut Out, _s: &Spline) -> Result<(), String> {
        self.warn_once(ctx, Warn::SplineNotImplemented, "Splines are not directly implemented.");
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, _out: &mut Out, _t: &Text) -> Result<(), String> {
        self.warn_once(ctx, Warn::TextUnsupported, "Text output is currently unsupported.");
        Ok(())
    }

    fn end(&mut self, _ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        Self::write_m_code(out, M_END_OF_PROGRAM);
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}
