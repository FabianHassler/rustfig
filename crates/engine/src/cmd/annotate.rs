//! Angle annotation (plan Q9): draw the angle `object.measure_angle`
//! measures: an arc between the two sides with its label, or a right-angle
//! mark. Plain Fig objects (an arc and a text glued into a compound, or a
//! three-point polyline), drawn in the indicator's attributes.

use super::{num, opt_num, opt_str, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_geom::Point;
use rustfig_model::{Arc, Compound, Object, ObjectKind, PolyKind, Polyline};
use serde_json::{json, Value};
use std::f64::consts::{FRAC_PI_2, PI};

pub(super) fn commands() -> Vec<Command> {
    vec![Command {
        id: "object.annotate_angle",
        label: "Draw measured angle",
        menu: "",
        shortcut: "",
        params: "{\"points\": [[x, y], [x, y], [x, y]]} (the angle at the second point) or {\"id\": polyline or arc, \"x\", \"y\": the corner clicked, as object.measure_angle}; \"mark\"?: \"arc\" (default: an arc between the two sides with the angle as label, glued into a compound) | \"right\" (a square corner mark; the angle must be 90 degrees within half a degree), \"radius\"?: Fig units (the arc's radius or the mark's side; default a third of the shorter side for the arc, a seventh for the mark), \"label\"?: string (default the angle in degrees, e.g. \"42.5°\"; \"\" for none; a right-angle mark has none unless given)  — the arc is drawn the short way (up to 180 degrees) in the indicator's pen color, thickness and depth; Returns {\"id\", \"degrees\"}. In Angle measure mode: Shift+middle (arc) and Shift+right (right angle) at the last point",
        mutates: true,
        enabled: super::always,
        run: annotate,
    }]
}

fn unit(v: Point) -> Option<Point> {
    let l = v.x.hypot(v.y);
    (l.is_finite() && l > 1e-9).then(|| Point::new(v.x / l, v.y / l))
}

/// The three points of the angle: the points given, or the corner of the
/// line or the arc's two ends and center.
fn three_points(e: &Engine, p: &Value) -> Result<[Point; 3], EngineError> {
    if super::param(p, "points").is_some() {
        let pts = super::points(p, "points")?;
        let [a, b, c] = pts.as_slice() else {
            return Err(EngineError::bad_params("\"points\" must be three points"));
        };
        return Ok([*a, *b, *c]);
    }
    let id = super::chop::id_param(p, "id")?;
    let o = super::chop::object(e, id)?;
    match &o.kind {
        ObjectKind::Polyline(l) if matches!(l.kind, PolyKind::Polyline | PolyKind::Polygon) => {
            let (x, y) = (num(p, "x")?, num(p, "y")?);
            let tol = opt_num(p, "tolerance")?.map_or_else(|| e.pick_tolerance(), |t| t.clamp(0.0, 1.0e6));
            let n = l.points.len();
            let i = l
                .points
                .iter()
                .position(|q| (q.x - x).abs() <= tol && (q.y - y).abs() <= tol)
                .ok_or_else(|| EngineError::bad_params("no corner of the line at that point"))?;
            let closed = l.kind == PolyKind::Polygon;
            let prev = if i > 0 {
                l.points.get(i - 1)
            } else if closed {
                l.points.last()
            } else {
                None
            };
            let next = if i + 1 < n {
                l.points.get(i + 1)
            } else if closed {
                l.points.first()
            } else {
                None
            };
            match (prev, l.points.get(i), next) {
                (Some(a), Some(b), Some(c)) => Ok([*a, *b, *c]),
                _ => Err(EngineError::bad_params("a line end has no angle")),
            }
        }
        ObjectKind::Arc(a) => Ok([a.points[0], a.center, a.points[2]]),
        _ => Err(EngineError::bad_params("the object has no angle to draw")),
    }
}

fn degrees_label(deg: f64) -> String {
    let s = format!("{deg:.1}");
    let s = s.strip_suffix(".0").unwrap_or(&s);
    format!("{s}°")
}

fn annotate(e: &mut Engine, p: &Value) -> CmdResult {
    let [a, v, c] = three_points(e, p)?;
    let (Some(ua), Some(uc)) = (unit(a - v), unit(c - v)) else {
        return Err(EngineError::bad_params("a side of the angle has no length"));
    };
    let shorter = v.dist(a).min(v.dist(c));
    let theta = (ua.x * uc.x + ua.y * uc.y).clamp(-1.0, 1.0).acos();
    let degrees = theta * 180.0 / PI;
    let radius = opt_num(p, "radius")?;
    if radius.is_some_and(|r| r <= 0.0 || r > 1.0e7) {
        return Err(EngineError::bad_params("radius must be a positive length"));
    }
    let label = match super::param(p, "label").filter(|v| !v.is_null()) {
        Some(Value::String(s)) if s.len() <= rustfig_format::MAX_TEXT_BYTES => Some(s.clone()),
        Some(_) => return Err(EngineError::bad_params("label must be a text")),
        None => None,
    };
    let style = e.indicator.style();
    match opt_str(p, "mark")?.unwrap_or("arc") {
        "right" => {
            if (theta - FRAC_PI_2).abs() > 0.5_f64.to_radians() {
                let msg = format!("the angle is {degrees:.1} degrees, not a right angle");
                e.set_status(msg.clone());
                return Err(EngineError::bad_params(msg));
            }
            let s = radius.unwrap_or(shorter / 7.0).max(1.0);
            let r = |q: Point| Point::new(q.x.round(), q.y.round());
            let pts = vec![r(v + ua * s), r(v + ua * s + uc * s), r(v + uc * s)];
            let mut st = style;
            st.area_fill = -1;
            let mark = Object::new(ObjectKind::Polyline(Polyline::new(PolyKind::Polyline, st, pts)));
            let out = match label.filter(|l| !l.is_empty()) {
                Some(l) => {
                    let at = v + (ua + uc) * (s * 1.4 + 60.0);
                    let t = label_text(e, at, l);
                    glue(vec![mark, t])
                }
                None => mark,
            };
            finish(e, out, degrees)
        }
        "arc" => {
            let r = radius.unwrap_or(shorter / 3.0).max(1.0);
            // The middle of the arc: the bisector of the sides, or at a
            // straight angle the perpendicular.
            let mid = unit(ua + uc).unwrap_or(Point::new(-ua.y, ua.x));
            let on = |u: Point| Point::new((v.x + u.x * r).round(), (v.y + u.y * r).round());
            let mut st = style;
            st.area_fill = -1;
            let arc = Arc::through(st, on(ua), on(mid), on(uc), false)
                .ok_or_else(|| EngineError::bad_params("the angle is too small or too flat for an arc"))?;
            let arc = Object::new(ObjectKind::Arc(arc));
            let text = label.unwrap_or_else(|| degrees_label(degrees));
            let out = if text.is_empty() {
                arc
            } else {
                let at = v + mid * (r + 90.0 + 25.0 * text.chars().count() as f64);
                glue(vec![arc, label_text(e, at, text)])
            };
            finish(e, out, degrees)
        }
        other => Err(EngineError::bad_params(format!("unknown mark {other:?} (arc, right)"))),
    }
}

/// A centered text with its middle at `at`.
fn label_text(e: &Engine, at: Point, text: String) -> Object {
    let mut t = super::shape::make_text(e, at, text);
    t.justify = rustfig_model::Justify::Center;
    crate::fix_text_metrics(&mut t);
    t.origin = Point::new(at.x.round(), (at.y + t.height / 2.0).round());
    Object::new(ObjectKind::Text(t))
}

fn glue(objects: Vec<Object>) -> Object {
    Object::new(ObjectKind::Compound(Compound { objects }))
}

fn finish(e: &mut Engine, o: Object, degrees: f64) -> CmdResult {
    let id = e.add_object(o);
    e.set_selection(vec![id]);
    Ok(json!({"id": id.0, "degrees": degrees}))
}
