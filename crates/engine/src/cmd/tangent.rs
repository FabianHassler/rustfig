//! Tangent/normal mode (plan D4): a line through a point of a curve, along
//! its tangent or its normal there, as one undo step.
//
// Ported from xfig src/e_tangent.c (`tangent_or_normal`,
// `tangent_normal_line`), src/u_smartsearch.c (`smart_next_*_found`) and
// src/u_geom.c (`close_to_ellipse`, `close_to_arc`, `close_to_polyline`,
// `close_to_spline`, `close_to_vector`) (xfig licence, see LICENSE-xfig).

use super::shape::{added, arrows_from, style_from};
use super::{opt_bool, opt_num, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_geom::{Point, FIG_PER_CM, FIG_PER_INCH};
use rustfig_model::{flatten_object, Arc, Ellipse, Object, ObjectKind, PolyKind, Polyline, Units};
use serde_json::{json, Value};

/// A tangent shorter than this (Fig units) is a singularity
/// (`ZERO_TOLERANCE`).
const ZERO_TOLERANCE: f64 = 2.0;
/// How close to a vertex counts as the vertex itself (`SING_TOLERANCE`).
const SING_TOLERANCE: f64 = 1.0;
/// xfig's `MAX_TANGNORMLEN`: the length setting's upper limit.
pub const MAX_LENGTH: f64 = 20.0;

pub(super) fn commands() -> Vec<Command> {
    vec![Command {
        id: "object.tangent_line",
        label: "Add tangent/normal line",
        menu: "",
        shortcut: "",
        params: "{\"id\": circle, ellipse, arc, line, polygon, box or spline, \"x\", \"y\": the point on it (Fig units; the curve point nearest to it within \"tolerance\" is used), \"normal\"?: bool (default false: the tangent), \"length\"?: in inches or cm as the units box shows (default: the indicator's tangent_length, xfig's Tangent/Norm Length), \"tolerance\"?: Fig units (default: the pick tolerance), plus the style and arrow parameters of shape.polyline}  — add a line through the curve point, along its tangent or normal, centred there (xfig's Tangent mode: left button tangent, middle button normal). Fails at a singularity (a vertex or a point where the curve has no direction). Returns {\"id\", \"x\", \"y\" (the curve point), \"points\"}",
        mutates: true,
        enabled: super::always,
        run: tangent_line,
    }]
}

/// Where a click met an object: the point on the curve and the direction
/// of the tangent there (xfig's `smart_point1`, `smart_point2`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CurvePoint {
    pub x: f64,
    pub y: f64,
    /// Tangent vector (`smart_point2 - smart_point1`), rounded to whole
    /// Fig units as xfig's integer points are.
    pub vx: f64,
    pub vy: f64,
}

/// Whether the mode accepts this kind of object at all (`M_TANGENT_OBJECT`).
pub(crate) fn accepts(o: &Object) -> bool {
    matches!(
        &o.kind,
        ObjectKind::Ellipse(_) | ObjectKind::Arc(_) | ObjectKind::Polyline(_) | ObjectKind::Spline(_)
    )
}

/// The curve point and tangent of `o` near (`x`, `y`) within `tol`, or
/// `None` when the click is not on the curve (`smart_next_*_found`).
pub(crate) fn curve_point(o: &Object, x: f64, y: f64, tol: f64) -> Option<CurvePoint> {
    if !(x.is_finite() && y.is_finite() && tol.is_finite()) {
        return None;
    }
    match &o.kind {
        ObjectKind::Ellipse(e) => close_to_ellipse(e, x, y, tol),
        ObjectKind::Arc(a) => close_to_arc(a, x, y, tol),
        ObjectKind::Polyline(l) => close_to_polyline(l, x, y, tol),
        ObjectKind::Spline(_) => close_to_spline(o, x, y, tol),
        ObjectKind::Text(_) | ObjectKind::Compound(_) => None,
    }
}

fn rnd(v: f64) -> f64 {
    v.round()
}

fn rotate(x: f64, y: f64, c: f64, s: f64) -> (f64, f64) {
    (c * x - s * y, s * x + c * y)
}

/// `close_to_ellipse` + `smart_next_ellipse_found`.
fn close_to_ellipse(e: &Ellipse, xp: f64, yp: f64, d: f64) -> Option<CurvePoint> {
    let (a, b) = (e.radii.x.abs(), e.radii.y.abs());
    let (mut dx, mut dy) = (xp - e.center.x, yp - e.center.y);
    let phi = e.angle;
    if !phi.is_finite() {
        return None;
    }
    let (cphi, sphi) = (phi.cos(), phi.sin());
    // Into the ellipse's own axes (Fig angles turn counter-clockwise on
    // screen, y down: xfig's rotate_vector with the angle's sine).
    if phi != 0.0 {
        (dx, dy) = rotate(dx, dy, cphi, sphi);
    }
    let dis = dx.hypot(dy);
    let r = if a * dy == 0.0 && b * dx == 0.0 {
        0.0
    } else {
        a * b * dis / (b * b * dx * dx + a * a * dy * dy).sqrt()
    };
    if (dis - r).abs() > d {
        return None;
    }
    // On the centre of a (tiny) ellipse there is no direction.
    let (qx, qy) = if dis == 0.0 {
        (0.0, 0.0)
    } else {
        (r * dx / dis, r * dy / dis)
    };
    let (qx, qy) = if phi == 0.0 { (rnd(qx), rnd(qy)) } else { (qx, qy) };
    let (mut vx, mut vy) = if a == 0.0 {
        (0.0, b)
    } else if b == 0.0 {
        (a, 0.0)
    } else {
        (-qy * a / b, qx * b / a)
    };
    let (mut qx, mut qy) = (qx, qy);
    if phi != 0.0 {
        (qx, qy) = rotate(qx, qy, cphi, -sphi);
        (vx, vy) = rotate(vx, vy, cphi, -sphi);
    }
    let (px, py) = (rnd(e.center.x + qx), rnd(e.center.y + qy));
    if !(px.is_finite() && py.is_finite() && vx.is_finite() && vy.is_finite()) {
        return None;
    }
    // A very small ellipse has no usable tangent.
    if (px - e.center.x).abs() <= SING_TOLERANCE && (py - e.center.y).abs() <= SING_TOLERANCE {
        return Some(CurvePoint {
            x: px,
            y: py,
            vx: 0.0,
            vy: 0.0,
        });
    }
    Some(CurvePoint {
        x: px,
        y: py,
        vx: rnd(vx),
        vy: rnd(vy),
    })
}

/// `close_to_arc` + `smart_next_arc_found`: the point on the arc's circle
/// in the direction of the click, the tangent perpendicular to the radius.
fn close_to_arc(a: &Arc, xp: f64, yp: f64, d: f64) -> Option<CurvePoint> {
    let c = a.center;
    let [p0, p1, p2] = a.points;
    let (u, v, w) = (p0 - c, p1 - c, p2 - c);
    if (u.x == 0.0 && u.y == 0.0) || (v.x == 0.0 && v.y == 0.0) || (w.x == 0.0 && w.y == 0.0) {
        return None;
    }
    let rarc = u.x.hypot(u.y);
    let (dx, dy) = (xp - c.x, yp - c.y);
    if dx == 0.0 && dy == 0.0 {
        return None;
    }
    let rp = dx.hypot(dy);
    if (rarc - rp).abs() > d {
        return None;
    }
    let tangent = |px: f64, py: f64| CurvePoint {
        x: rnd(px),
        y: rnd(py),
        vx: rnd(py - c.y),
        vy: -rnd(px - c.x),
    };
    // The three defining points snap first. (xfig writes
    // `abs(yp - y <= d)` here, a typo for the intended test.)
    for p in [p0, p1, p2] {
        if (xp - p.x).abs() <= d && (yp - p.y).abs() <= d {
            return Some(tangent(p.x, p.y));
        }
    }
    let (uang, vang) = (u.y.atan2(u.x), v.y.atan2(v.x));
    let (mut wang, mut pang) = (w.y.atan2(w.x), dy.atan2(dx));
    let tau = std::f64::consts::TAU;
    let ok = if uang >= wang {
        if uang >= vang && vang >= wang {
            uang >= pang && pang >= wang
        } else {
            wang += tau;
            if uang >= pang {
                pang += tau;
            }
            pang <= wang
        }
    } else if uang <= vang && vang <= wang {
        uang <= pang && pang <= wang
    } else {
        wang -= tau;
        if uang <= pang {
            pang -= tau;
        }
        pang >= wang
    };
    if !ok {
        return None;
    }
    let (ex, ey) = rotate(rarc, 0.0, pang.cos(), pang.sin());
    Some(tangent(c.x + ex, c.y + ey))
}

/// `close_to_vector`: the foot of (`xp`, `yp`) on the segment when it is
/// within `d`; near an end the end itself.
fn close_to_vector(a: Point, b: Point, xp: f64, yp: f64, d: f64) -> Option<(f64, f64)> {
    if (xp - a.x).abs() <= d && (yp - a.y).abs() <= d {
        return Some((a.x, a.y));
    }
    if (xp - b.x).abs() <= d && (yp - b.y).abs() <= d {
        return Some((b.x, b.y));
    }
    if xp < a.x.min(b.x) - d || xp > a.x.max(b.x) + d || yp < a.y.min(b.y) - d || yp > a.y.max(b.y) + d {
        return None;
    }
    let (x, y) = if a.x == b.x {
        (a.x, yp)
    } else if a.y == b.y {
        (xp, a.y)
    } else {
        let slope = (b.x - a.x) / (b.y - a.y);
        let y = (slope * (xp - a.x + slope * a.y) + yp) / (1.0 + slope * slope);
        (a.x + slope * (y - a.y), y)
    };
    let (dx, dy) = (xp - x, yp - y);
    (dx * dx + dy * dy < d * d).then(|| (rnd(x), rnd(y)))
}

/// `close_to_polyline`: lines, polygons, boxes (the corners; a closed one
/// also has its closing side). A click on a vertex has no direction.
fn close_to_polyline(l: &Polyline, xp: f64, yp: f64, d: f64) -> Option<CurvePoint> {
    let mut pts: Vec<Point> = l.points.iter().copied().filter(|p| p.is_finite()).collect();
    if l.kind != PolyKind::Polyline && pts.len() > 2 && pts.first() != pts.last() {
        pts.push(*pts.first()?);
    }
    let first = *pts.first()?;
    if (xp - first.x).abs() <= SING_TOLERANCE && (yp - first.y).abs() <= SING_TOLERANCE {
        return Some(CurvePoint {
            x: first.x,
            y: first.y,
            vx: 0.0,
            vy: 0.0,
        });
    }
    for w in pts.windows(2) {
        let (a, b) = (*w.first()?, *w.get(1)?);
        if (xp - b.x).abs() <= SING_TOLERANCE && (yp - b.y).abs() <= SING_TOLERANCE {
            return Some(CurvePoint {
                x: b.x,
                y: b.y,
                vx: 0.0,
                vy: 0.0,
            });
        }
        if let Some((x, y)) = close_to_vector(a, b, xp, yp, d) {
            return Some(CurvePoint {
                x,
                y,
                vx: b.x - a.x,
                vy: b.y - a.y,
            });
        }
    }
    None
}

/// The spline's points as xfig computes them for hit tests (integer points,
/// closed splines end on their first point), through fig2dev's port.
fn xfig_spline_points(o: &Object) -> Option<Vec<Point>> {
    use fig2dev::trans_spline::{compute_closed_spline, compute_open_spline, HIGH_PRECISION};
    use rustfig_format::bridge::{object_to_fig2dev, FigObject};
    let mut ctx = fig2dev::Ctx::default();
    let FigObject::Spline(sp) = object_to_fig2dev(o, &ctx) else {
        return None;
    };
    let pts = if sp.open_spline() {
        compute_open_spline(&mut ctx, &sp, HIGH_PRECISION)?
    } else {
        compute_closed_spline(&mut ctx, &sp, HIGH_PRECISION)?
    };
    Some(pts.iter().map(|q| Point::new(f64::from(q.x), f64::from(q.y))).collect())
}

/// `close_to_spline`: the first side of the flattened curve within `d`
/// (`close_to_float_vector`).
fn close_to_spline(o: &Object, xp: f64, yp: f64, d: f64) -> Option<CurvePoint> {
    let ObjectKind::Spline(s) = &o.kind else { return None };
    // xfig walks the points of its own spline computation at HIGH_PRECISION
    // (`compute_open_spline` / `compute_closed_spline`), whose steps are
    // fine enough for the tangent: use fig2dev's port of it (W22-25).
    let mut pts: Vec<Point> = xfig_spline_points(o).unwrap_or_else(|| {
        let flat = flatten_object(o);
        flat.first().map(|p| p.pts.clone()).unwrap_or_default()
    });
    if s.closed && pts.len() > 2 && pts.first() != pts.last() {
        if let Some(f) = pts.first().copied() {
            pts.push(f);
        }
    }
    for w in pts.windows(2) {
        let (a, b) = (*w.first()?, *w.get(1)?);
        let (rx, ry) = (b.x - a.x, b.y - a.y);
        let radius2 = rx * rx + ry * ry;
        if radius2 <= 0.0 {
            continue;
        }
        let radius = radius2.sqrt();
        let (nx, ny) = (-ry / radius, rx / radius);
        let dist = (xp - a.x) * nx + (yp - a.y) * ny;
        let (fx, fy) = (xp - dist * nx, yp - dist * ny);
        let near = |p: Point| (xp - p.x) * (xp - p.x) + (yp - p.y) * (yp - p.y) <= d * d;
        let ok = if dist.abs() > d {
            false
        } else if near(a) || near(b) {
            true
        } else {
            let lambda = ((xp - a.x) * rx + (yp - a.y) * ry) / radius2;
            (0.0..=1.0).contains(&lambda)
        };
        if ok {
            return Some(CurvePoint {
                x: rnd(fx),
                y: rnd(fy),
                vx: rnd(b.x) - rnd(a.x),
                vy: rnd(b.y) - rnd(a.y),
            });
        }
    }
    None
}

/// The line's end points: the curve point +- the tangent (or normal)
/// scaled to half the length (`tangent_or_normal`, `tangent_normal_line`).
/// `length` is in inches or cm.
pub(crate) fn line_points(hit: CurvePoint, normal: bool, length: f64, units: Units) -> Result<[Point; 3], EngineError> {
    let len = hit.vx.hypot(hit.vy);
    if len < ZERO_TOLERANCE {
        return Err(EngineError::bad_params("singularity, can't draw tangent/normal"));
    }
    let per_unit = match units {
        Units::Inches => FIG_PER_INCH,
        Units::Metric => FIG_PER_CM,
    };
    let tanlen = length * per_unit / 2.0;
    let (sx, sy) = (hit.vx * tanlen / len, hit.vy * tanlen / len);
    let (vx, vy) = if normal { (-sy, sx) } else { (sx, sy) };
    let (dx, dy) = (rnd(vx), rnd(vy));
    Ok([
        Point::new(hit.x - dx, hit.y - dy),
        Point::new(hit.x, hit.y),
        Point::new(hit.x + dx, hit.y + dy),
    ])
}

fn tangent_line(e: &mut Engine, p: &Value) -> CmdResult {
    let id = super::chop::id_param(p, "id")?;
    let o = super::chop::object(e, id)?;
    if !accepts(&o) {
        return Err(EngineError::bad_params(
            "tangent and normal work on a circle, ellipse, arc, line, polygon, box or spline",
        ));
    }
    let (x, y) = (super::num(p, "x")?, super::num(p, "y")?);
    let tol = match opt_num(p, "tolerance")? {
        Some(t) => t.clamp(0.0, 1.0e6),
        None => e.pick_tolerance(),
    };
    let normal = opt_bool(p, "normal")?.unwrap_or(false);
    let length = match opt_num(p, "length")? {
        Some(l) if (0.0..=MAX_LENGTH).contains(&l) => l,
        Some(_) => return Err(EngineError::bad_params(format!("length must be 0 to {MAX_LENGTH}"))),
        None => e.indicator.tangent_length.clamp(0.0, MAX_LENGTH),
    };
    let hit = curve_point(&o, x, y, tol).ok_or_else(|| EngineError::bad_params("the point is not on the curve"))?;
    let pts = line_points(hit, normal, length, e.doc().settings.units)?;
    if pts[0] == pts[2] {
        return Err(EngineError::bad_params("the tangent/normal length is 0"));
    }
    let mut pl = Polyline::new(PolyKind::Polyline, style_from(e, p)?, pts.to_vec());
    (pl.forward_arrow, pl.backward_arrow) = arrows_from(e, p)?;
    let mut r = added(e, Object::new(ObjectKind::Polyline(pl)))?;
    if let Some(m) = r.as_object_mut() {
        m.insert("x".into(), json!(hit.x));
        m.insert("y".into(), json!(hit.y));
        m.insert(
            "points".into(),
            json!(pts.iter().map(|q| json!([q.x, q.y])).collect::<Vec<_>>()),
        );
    }
    Ok(r)
}
