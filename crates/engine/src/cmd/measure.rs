//! The measure modes (plan D5): the angle between three points or at a
//! corner of a line or an arc, the length of lines, boxes, arcs and
//! ellipses with a running total, and their area with a running total
//! (signed or not). Nothing here changes the drawing: no undo step, no
//! change to the selection; the result goes to the message panel.
//
// Ported from xfig src/e_measure.c (`init_anglemeas_object`,
// `anglemeas_line`, `anglemeas_arc`, `anglemeas_third`,
// `init_lenmeas_object`, `clear_lenmeas_memory`, `init_areameas_object`,
// `clear_areameas_memory`), src/u_geom.c (`compute_3p_angle`,
// `compute_line_angle`, `compute_arc_angle`, `compute_poly_length`,
// `compute_arc_length`, `compute_poly_area`, `compute_arc_area`,
// `compute_ellipse_area`) and src/w_msgpanel.c (`lenmeas_msg`,
// `areameas_msg`, `make_dimension_string`) (xfig licence, see
// LICENSE-xfig).

use super::{opt_bool, opt_num, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_geom::{Point, FIG_PER_CM, FIG_PER_INCH, FIG_PER_STYLE_UNIT};
use rustfig_model::{Arc, Ellipse, Object, ObjectId, ObjectKind, PolyKind, Polyline, Units};
use serde_json::{json, Value};
use std::f64::consts::{PI, TAU};

/// What the modes remember between clicks (xfig's `total_len`,
/// `total_area` and `signed_area`).
#[derive(Debug, Clone, Default)]
pub(crate) struct MeasureState {
    /// The running total of lengths, Fig units.
    pub total_len: f64,
    /// The running total of areas, square Fig units.
    pub total_area: f64,
    /// Whether areas keep their sign (set by the area mode's Shift+right
    /// button, "reset to +-0").
    pub signed_area: bool,
}

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "object.measure_angle",
            label: "Measure angle",
            menu: "",
            shortcut: "",
            params: "{\"points\": [[x, y], [x, y], [x, y]]} the angle at the second point between the lines to the first and the third, or {\"id\": line, polygon or arc, \"x\", \"y\": the corner clicked (a vertex within \"tolerance\", default the pick tolerance; not needed for an arc: its angle at the centre), \"tolerance\"?}; \"save\"?: bool also stores it as the indicator's rotation angle (xfig's middle button). Shows \"Angle defined by three points: 45.00 Degrees\" (or at a polyline corner, at the arc centre) in the message panel; nothing is changed. Returns {\"degrees\", \"kind\", \"text\"}",
            mutates: false,
            enabled: super::always,
            run: measure_angle,
        },
        Command {
            id: "object.measure_length",
            label: "Measure length",
            menu: "",
            shortcut: "",
            params: "{\"id\": line, polygon, box, arc box, arc, circle or ellipse, \"add\"?: bool (add to the running total, xfig's middle button)}  — shows \"Length of polygon is 3.0 in, accumulated 5.0 in\" in the drawing's units; nothing is changed. Returns {\"kind\", \"length\" (Fig units), \"accumulated\"? (Fig units), \"text\"}",
            mutates: false,
            enabled: super::always,
            run: measure_length,
        },
        Command {
            id: "object.measure_area",
            label: "Measure area",
            menu: "",
            shortcut: "",
            params: "{\"id\": polygon, box, arc box, arc, circle or ellipse, \"add\"?: bool (add to the running total, xfig's middle button)}  — shows \"Area of box is 2.0 square in, accumulated 4.0 square in\"; the area is absolute unless the total was reset with \"signed\"; nothing is changed. Returns {\"kind\", \"area\" (square Fig units), \"accumulated\"?, \"signed\", \"text\"}",
            mutates: false,
            enabled: super::always,
            run: measure_area,
        },
        Command {
            id: "object.measure_reset",
            label: "Reset measured total",
            menu: "",
            shortcut: "",
            params: "{\"what\": \"length\"|\"area\", \"signed\"?: bool (area only: from now on areas keep their sign, xfig's Shift+right button \"reset to +-0\")}  — sets the running total to 0",
            mutates: false,
            enabled: super::always,
            run: measure_reset,
        },
    ]
}

// ---- geometry (u_geom.c) -------------------------------------------------

fn pt_ok(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// `compute_3p_angle`: the angle at `p2` from the line to `p1` to the line
/// to `p3`, in [0, 2 pi); `None` when a line has no length.
pub(crate) fn angle_3p(p1: Point, p2: Point, p3: Point) -> Option<f64> {
    let (vx, vy) = (p1.x - p2.x, p1.y - p2.y);
    let (wx, wy) = (p3.x - p2.x, p3.y - p2.y);
    if (vx == 0.0 && vy == 0.0) || (wx == 0.0 && wy == 0.0) {
        return None;
    }
    let mut dang = -wy.atan2(wx) + vy.atan2(vx);
    if dang < 0.0 {
        dang += TAU;
    }
    dang.is_finite().then_some(dang)
}

/// The vertices of a line as xfig stores them: a closed one repeats its
/// first point at the end.
fn xfig_points(l: &Polyline) -> Vec<Point> {
    let mut pts: Vec<Point> = l.points.clone();
    if l.kind != PolyKind::Polyline && pts.len() > 1 && pts.first() != pts.last() {
        if let Some(f) = pts.first().copied() {
            pts.push(f);
        }
    }
    pts
}

/// `compute_line_angle`: the angle at the vertex `p` between the sides
/// meeting there (in [0, 2 pi)). The first point of an open line has no
/// side before it, the last none after it; a polygon's closing point is its
/// first.
pub(crate) fn line_angle(l: &Polyline, p: Point) -> Option<f64> {
    let pts = xfig_points(l);
    for (i, q) in pts.iter().enumerate() {
        if q.x != p.x || q.y != p.y || i == 0 {
            continue;
        }
        let Some(vor) = pts.get(i - 1) else { continue };
        let nach = match pts.get(i + 1) {
            Some(n) => Some(n),
            None if l.kind == PolyKind::Polygon => pts.get(1),
            None => None,
        };
        if let Some(a) = nach.and_then(|n| angle_3p(*vor, *q, *n)) {
            return Some(a);
        }
    }
    None
}

/// `compute_arc_angle`: the oriented angle swept by the arc, in (-2 pi, 2 pi).
pub(crate) fn arc_angle(a: &Arc) -> Option<f64> {
    let [p0, p1, p2] = a.points;
    let (u, v, w) = (p0 - a.center, p1 - a.center, p2 - a.center);
    if (u.x == 0.0 && u.y == 0.0) || (v.x == 0.0 && v.y == 0.0) || (w.x == 0.0 && w.y == 0.0) {
        return None;
    }
    let (uang, vang, wang) = (u.y.atan2(u.x), v.y.atan2(v.x), w.y.atan2(w.x));
    let dang = if uang >= wang {
        if uang >= vang && vang >= wang {
            uang - wang
        } else {
            -TAU + uang - wang
        }
    } else if uang <= vang && vang <= wang {
        uang - wang
    } else {
        TAU + uang - wang
    };
    dang.is_finite().then_some(dang)
}

fn arc_radius(a: &Arc) -> f64 {
    let u = a.points[0] - a.center;
    if u.x == 0.0 && u.y == 0.0 {
        0.0
    } else {
        u.x.hypot(u.y)
    }
}

/// `compute_poly_length`: the sides added up (a closed line closes).
fn poly_length(l: &Polyline) -> f64 {
    xfig_points(l)
        .windows(2)
        .map(|w| match (w.first(), w.get(1)) {
            (Some(a), Some(b)) => (b.x - a.x).hypot(b.y - a.y),
            _ => 0.0,
        })
        .sum()
}

/// `compute_arc_length`.
fn arc_length(a: &Arc) -> Option<f64> {
    arc_angle(a).map(|alpha| arc_radius(a) * alpha.abs())
}

/// `compute_poly_area`: the signed area by the shoelace formula (negative
/// where the points turn the other way).
fn poly_area(l: &Polyline) -> f64 {
    xfig_points(l)
        .windows(2)
        .map(|w| match (w.first(), w.get(1)) {
            (Some(p), Some(q)) => -p.x * q.y + p.y * q.x,
            _ => 0.0,
        })
        .sum::<f64>()
        / 2.0
}

/// `compute_arc_area`: the sector, less the triangle for an open arc.
fn arc_area(a: &Arc) -> Option<f64> {
    let alpha = arc_angle(a)?;
    let r = arc_radius(a);
    let mut area = 0.5 * alpha * r * r;
    if !a.pie {
        let tri = 0.5 * r * r * alpha.sin().abs();
        if area > 0.0 {
            area -= tri;
        } else {
            area += tri;
        }
    }
    Some(area)
}

/// The length of an ellipse's outline: 2 pi r for a circle, else the
/// approximation from The Math Forum that xfig uses.
fn ellipse_length(e: &Ellipse) -> (&'static str, f64) {
    let (a, b) = (e.radii.x, e.radii.y);
    if a == b {
        return ("circle", PI * a * 2.0);
    }
    let z = (a - b) / (a + b);
    (
        "ellipse",
        PI * (a + b) * (1.0 + 3.0 * z * z / (10.0 + (4.0 - 3.0 * z * z).sqrt())),
    )
}

/// An arc box's corner radius in Fig units.
fn corner_radius(l: &Polyline) -> f64 {
    l.radius.max(0) as f64 * FIG_PER_STYLE_UNIT
}

/// What kind of thing a length or an area was measured of, and the value
/// in Fig units (`init_lenmeas_object`).
pub(crate) fn length_of(o: &Object) -> Option<(&'static str, f64)> {
    match &o.kind {
        ObjectKind::Polyline(l) => {
            let len = poly_length(l);
            Some(match l.kind {
                PolyKind::Box | PolyKind::Picture => ("box", len),
                // xfig takes off one circle's circumference for the corners.
                PolyKind::ArcBox => ("arcbox", len - PI * corner_radius(l) * 2.0),
                PolyKind::Polygon => ("polygon", len),
                PolyKind::Polyline => ("polyline", len),
            })
        }
        ObjectKind::Arc(a) => arc_length(a).map(|l| ("arc", l)),
        ObjectKind::Ellipse(e) => Some(ellipse_length(e)),
        _ => None,
    }
}

/// The signed area (`init_areameas_object`).
pub(crate) fn area_of(o: &Object) -> Option<(&'static str, f64)> {
    match &o.kind {
        ObjectKind::Polyline(l) if l.kind != PolyKind::Polyline => {
            let area = poly_area(l);
            Some(match l.kind {
                PolyKind::Box => ("box", area),
                PolyKind::ArcBox => {
                    let r = corner_radius(l);
                    // xfig subtracts the corners from the signed area, which
                    // adds them for a box whose points turn the other way;
                    // here they always come off the magnitude.
                    ("arcbox", area - area.signum() * PI * r * r)
                }
                _ => ("polygon", area),
            })
        }
        ObjectKind::Arc(a) => arc_area(a).map(|v| (if a.pie { "sector" } else { "open arc" }, v)),
        ObjectKind::Ellipse(e) => Some(("ellipse", PI * e.radii.x * e.radii.y)),
        _ => None,
    }
}

// ---- the message panel (w_msgpanel.c) ------------------------------------

/// Decimals after the point of a length (xfig's `cur_dimline_prec`).
const DIMLINE_PREC: usize = 1;

/// A length (or, with `square`, an area) in the drawing's units, scaled by
/// the unit/scale setting, as xfig's `make_dimension_string` writes it:
/// "3.5 cm", "1-3/4 in", "2.000 square cm".
pub(crate) fn dimension_string(e: &Engine, value: f64, square: bool) -> String {
    let scale = e.unit_scale.factor;
    let unit = if e.unit_scale.unit.is_empty() {
        e.doc().unit_name()
    } else {
        e.unit_scale.unit.as_str()
    };
    let prec = DIMLINE_PREC;
    match e.doc().settings.units {
        Units::Metric => {
            if square {
                format!("{:.3} square {unit}", value / (FIG_PER_CM * FIG_PER_CM) * scale * scale)
            } else {
                format!("{:.prec$} {unit}", value / FIG_PER_CM * scale)
            }
        }
        Units::Inches if square => format!(
            "{:.prec$} square {unit}",
            value / (FIG_PER_INCH * FIG_PER_INCH) * scale * scale
        ),
        Units::Inches => {
            let ulen = value / FIG_PER_INCH * scale;
            if e.unit_scale.decimal_inches {
                format!("{ulen:.prec$} {unit}")
            } else {
                inch_fraction(ulen, unit, prec)
            }
        }
    }
}

/// Inches as a whole number and a fraction of 1/64, with feet first when
/// the unit is feet (the fractional branch of `make_dimension_string`).
fn inch_fraction(mut ulen: f64, unit: &str, prec: usize) -> String {
    let mut ilen = ulen as i64;
    if ilen as f64 == ulen {
        return format!("{ilen} {unit}");
    }
    let mut tmp = String::new();
    let mut units = unit;
    let mut ifeet = 0_i64;
    if matches!(unit, "f" | "ft" | "feet") {
        ifeet = ulen as i64;
        if ifeet.abs() >= 1 {
            tmp = format!("{ifeet} {unit} ");
        }
        ulen = (ulen - ifeet as f64) * 12.0;
        ilen = ulen as i64;
        units = "in";
    }
    let mut ifract = ((ulen - ilen as f64) * 64.0) as i64;
    if ifract == 0 {
        if ifeet < 0 {
            ilen = ilen.abs();
        }
        return format!("{tmp}{ilen} {units}");
    }
    if ilen as f64 + ifract as f64 / 64.0 == ulen {
        let mut iexp = 64_i64;
        while iexp >= 2 && ifract % 2 == 0 {
            ifract /= 2;
            iexp /= 2;
        }
        format!("{tmp}{}-{}/{iexp} {units}", ilen.abs(), ifract.abs())
    } else {
        format!("{tmp}{ulen:.prec$} {units}")
    }
}

/// `lenmeas_msg`.
pub(crate) fn length_message(e: &Engine, kind: &str, len: f64, total: Option<f64>) -> String {
    let l = dimension_string(e, len, false);
    match total {
        Some(t) => format!("Length of {kind} is {l}, accumulated {}", dimension_string(e, t, false)),
        None => format!("Length of {kind} is {l}"),
    }
}

/// `areameas_msg`.
pub(crate) fn area_message(e: &Engine, kind: &str, area: f64, total: Option<f64>) -> String {
    let a = dimension_string(e, area, true);
    match total {
        Some(t) => format!("Area of {kind} is {a}, accumulated {}", dimension_string(e, t, true)),
        None => format!("Area of {kind} is {a}"),
    }
}

/// `angle_msg`: "<what><degrees, two decimals> Degrees".
fn angle_message(what: &str, radians: f64) -> String {
    format!("{what}{:.2} Degrees", radians * 180.0 / PI)
}

// ---- the commands ----------------------------------------------------------

fn target(e: &Engine, p: &Value) -> Result<Object, EngineError> {
    let id = super::chop::id_param(p, "id")?;
    super::chop::object(e, id)
}

/// Fail with xfig's message in the message panel too.
fn refuse<T>(e: &mut Engine, msg: &str) -> Result<T, EngineError> {
    e.set_status(msg);
    Err(EngineError::bad_params(msg))
}

fn measure_length(e: &mut Engine, p: &Value) -> CmdResult {
    let o = target(e, p)?;
    let add = opt_bool(p, "add")?.unwrap_or(false);
    let Some((kind, len)) = length_of(&o).filter(|(_, l)| l.is_finite()) else {
        return refuse(e, "Sorry, can't measure length of this object");
    };
    let total = if add {
        e.measure.total_len += len;
        Some(e.measure.total_len)
    } else {
        None
    };
    let text = length_message(e, kind, len, total);
    e.set_status(text.clone());
    Ok(json!({"kind": kind, "length": len, "accumulated": total, "text": text}))
}

fn measure_area(e: &mut Engine, p: &Value) -> CmdResult {
    let o = target(e, p)?;
    let add = opt_bool(p, "add")?.unwrap_or(false);
    let Some((kind, mut area)) = area_of(&o).filter(|(_, a)| a.is_finite()) else {
        return refuse(e, "Sorry, can't measure area of this object");
    };
    let signed = e.measure.signed_area;
    if !signed {
        area = area.abs();
    }
    let total = if add {
        e.measure.total_area += area;
        Some(e.measure.total_area)
    } else {
        None
    };
    let text = area_message(e, kind, area, total);
    e.set_status(text.clone());
    Ok(json!({"kind": kind, "area": area, "accumulated": total, "signed": signed, "text": text}))
}

fn measure_reset(e: &mut Engine, p: &Value) -> CmdResult {
    let what = super::string(p, "what")?;
    match what.as_str() {
        "length" => {
            e.measure.total_len = 0.0;
            e.set_status("length reset to 0");
        }
        "area" => {
            let signed = opt_bool(p, "signed")?.unwrap_or(false);
            e.measure.total_area = 0.0;
            e.measure.signed_area = signed;
            e.set_status(if signed {
                "signed area reset to 0"
            } else {
                "area reset to 0"
            });
        }
        _ => return Err(EngineError::bad_params("\"what\" must be \"length\" or \"area\"")),
    }
    Ok(
        json!({"what": what, "length": e.measure.total_len, "area": e.measure.total_area, "signed": e.measure.signed_area}),
    )
}

/// `angle_save` (`set_and_show_rotnangle`): the angle, 0 to 180 degrees,
/// becomes the rotation angle of the indicator.
fn save_angle(e: &mut Engine, radians: f64) {
    let mut degr = (radians * 180.0 / PI).abs();
    if degr > 180.0 {
        degr = 360.0 - degr;
    }
    if degr.is_finite() {
        e.indicator.rotate_angle = degr;
    }
}

/// The vertex of a line within `tol` of (`x`, `y`) that xfig's point search
/// finds first (`next_line_point_found`).
pub(crate) fn vertex_near(l: &Polyline, x: f64, y: f64, tol: f64) -> Option<Point> {
    l.points
        .iter()
        .copied()
        .find(|q| pt_ok(*q) && (q.x - x).abs() <= tol && (q.y - y).abs() <= tol)
}

fn measure_angle(e: &mut Engine, p: &Value) -> CmdResult {
    let save = opt_bool(p, "save")?.unwrap_or(false);
    let (what, kind, ang) = if p.get("points").is_some() {
        let pts = super::points(p, "points")?;
        let [a, b, c] = pts.as_slice() else {
            return Err(EngineError::bad_params("\"points\" must be three points"));
        };
        let Some(v) = angle_3p(*a, *b, *c) else {
            return refuse(e, "Can't compute angle");
        };
        // `anglemeas_third`: the angle is shown between -180 and 180.
        let v = if v > PI { v - TAU } else { v };
        ("Angle defined by three points: ", "three points", v)
    } else {
        let o = target(e, p)?;
        match &o.kind {
            ObjectKind::Polyline(l) if matches!(l.kind, PolyKind::Polyline | PolyKind::Polygon) => {
                let (x, y) = (super::num(p, "x")?, super::num(p, "y")?);
                let tol = match opt_num(p, "tolerance")? {
                    Some(t) => t.clamp(0.0, 1.0e6),
                    None => e.pick_tolerance(),
                };
                let Some(q) = vertex_near(l, x, y, tol) else {
                    return Err(EngineError::bad_params("no corner of the line at that point"));
                };
                let Some(v) = line_angle(l, q) else {
                    return refuse(e, "Can't compute angle at endpoint");
                };
                let v = if v > PI { v - TAU } else { v };
                ("Angle at polyline corner: ", "corner", v)
            }
            ObjectKind::Arc(a) => {
                let Some(v) = arc_angle(a) else {
                    return refuse(e, "Can't compute angle");
                };
                ("Angle at arc center: ", "arc", v)
            }
            _ => return refuse(e, "Sorry, can't measure the angle of this object"),
        }
    };
    if save {
        save_angle(e, ang);
    }
    let text = angle_message(what, ang);
    e.set_status(text.clone());
    Ok(json!({"degrees": ang * 180.0 / PI, "kind": kind, "text": text, "saved": save}))
}

/// The object a click at (`x`, `y`) picks in the angle mode's point
/// search: the first line or polygon with a vertex there, then the first
/// arc with one of its three points there (`do_point_search`: xfig's
/// ellipses and splines are not in this mode's mask).
pub(crate) fn angle_target(e: &Engine, x: f64, y: f64, tol: f64) -> Option<ObjectId> {
    let doc = e.doc();
    let first_line = doc.objects.iter().find(|o| {
        matches!(&o.kind, ObjectKind::Polyline(l)
            if matches!(l.kind, PolyKind::Polyline | PolyKind::Polygon) && vertex_near(l, x, y, tol).is_some())
            && e.is_visible(o)
    });
    if let Some(o) = first_line {
        return Some(o.id);
    }
    doc.objects
        .iter()
        .find(|o| {
            matches!(&o.kind, ObjectKind::Arc(a)
                if a.points.iter().any(|q| pt_ok(*q) && (q.x - x).abs() <= tol && (q.y - y).abs() <= tol))
                && e.is_visible(o)
        })
        .map(|o| o.id)
}
