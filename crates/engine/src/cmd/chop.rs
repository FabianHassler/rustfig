//! Chop mode: cut objects at their intersections (plan D3).
//!
//! xfig's `e_chop.c`: the left button collects the axe (cutting) objects,
//! the middle button picks the log object and chops it with every axe, and
//! the right button clears the axe list. Only polylines, polygons, arcs and
//! circles can be chopped; elliptical arcs are not supported by xfig. The
//! pieces keep the attributes of the object they came from, and one chop is
//! one undo step.
//
// Ported from xfig src/e_chop.c and src/w_intersect.c (xfig licence, see
// LICENSE-xfig).

use super::{CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_geom::{
    arc_from_three_points, arc_position, circle_circle_intersections, circle_ellipse_intersections,
    line_circle_intersections, line_ellipse_intersections, line_segment_intersection, point_on_arc, point_on_arc_geom,
    Point,
};
use rustfig_model::{arc_geom, Arc, Ellipse, Object, ObjectId, ObjectKind, PolyKind, Polyline};
use serde_json::{json, Value};
use std::f64::consts::TAU;
use std::sync::Arc as Shared;

pub(super) fn commands() -> Vec<Command> {
    vec![Command {
        id: "object.chop",
        label: "Chop objects",
        menu: "",
        shortcut: "",
        params: "{\"id\", \"axes\": [object ids]}  — cut the log object (polyline, polygon, arc or circle) at every intersection with the axe objects (polylines, polygons, arcs, circles or ellipses)",
        mutates: true,
        enabled: super::always,
        run: chop,
    }]
}

fn bad(m: impl Into<String>) -> EngineError {
    EngineError::bad_params(m)
}

fn object(e: &Engine, id: ObjectId) -> Result<Object, EngineError> {
    e.doc()
        .get(id)
        .cloned()
        .ok_or_else(|| EngineError::not_found(format!("no top-level object with id {}", id.0)))
}

fn id_param(p: &Value, key: &str) -> Result<ObjectId, EngineError> {
    match p.get(key) {
        Some(v) => v
            .as_u64()
            .map(ObjectId)
            .ok_or_else(|| bad(format!("{key:?} must be an object id"))),
        None => Err(bad(format!("missing object id {key:?}"))),
    }
}

fn axes_param(e: &Engine, p: &Value) -> Result<Vec<Object>, EngineError> {
    let Some(Value::Array(arr)) = p.get("axes") else {
        return Err(bad("missing object id list \"axes\""));
    };
    if arr.is_empty() {
        return Err(bad("\"axes\" must name at least one object"));
    }
    let mut out = Vec::with_capacity(arr.len());
    for v in arr {
        let id = v
            .as_u64()
            .map(ObjectId)
            .ok_or_else(|| bad("axes must be an array of object ids"))?;
        let o = object(e, id)?;
        if !matches!(
            o.kind,
            ObjectKind::Polyline(_) | ObjectKind::Arc(_) | ObjectKind::Ellipse(_)
        ) {
            return Err(bad("an axe must be a polyline, arc or ellipse"));
        }
        out.push(o);
    }
    Ok(out)
}

fn chop(e: &mut Engine, p: &Value) -> CmdResult {
    let log_id = id_param(p, "id")?;
    let log = object(e, log_id)?;
    let axes = axes_param(e, p)?;
    let pieces = match &log.kind {
        ObjectKind::Polyline(l) if matches!(l.kind, PolyKind::Polyline | PolyKind::Polygon) => {
            chop_polyline(l, &axes, log_id, e.indicator.pie_arc)?
        }
        ObjectKind::Arc(a) => chop_arc(a, &axes, log_id)?,
        ObjectKind::Ellipse(el) => chop_ellipse(el, &axes, log_id)?,
        _ => return Err(bad("only polylines, polygons, arcs and circles can be chopped")),
    };
    if pieces.is_empty() {
        return Ok(json!({"ids": []}));
    }
    let doc = e.doc_mut();
    doc.objects.retain(|o| o.id != log_id);
    let mut ids = Vec::with_capacity(pieces.len());
    for kind in pieces {
        let mut o = Object::new(kind);
        o.comments = log.comments.clone();
        o.id = doc.alloc_id();
        ids.push(o.id);
        doc.objects.push(Shared::new(o));
    }
    e.set_selection(ids.clone());
    Ok(json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>()}))
}

/// The segments of a polyline. Closed kinds (xfig repeats the first point at
/// the end; the Rust model does not) include the closing segment.
fn poly_segments(l: &Polyline) -> Vec<(Point, Point)> {
    if l.points.len() < 2 {
        return Vec::new();
    }
    if l.kind == PolyKind::Polyline {
        return l
            .points
            .windows(2)
            .filter_map(|w| Some((*w.first()?, *w.get(1)?)))
            .collect();
    }
    let n = l.points.len();
    (0..n)
        .filter_map(|i| Some((*l.points.get(i)?, *l.points.get((i + 1) % n)?)))
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct Isect {
    point: Point,
    seg_idx: usize,
}

fn round(p: Point) -> Point {
    Point::new(p.x.round(), p.y.round())
}

/// All intersections of the log polyline with one axe object.
fn polyline_axe_intersections(l: &Polyline, axe: &Object, log_id: ObjectId) -> Result<Vec<Isect>, EngineError> {
    let segs = poly_segments(l);
    let mut out = Vec::new();
    match &axe.kind {
        ObjectKind::Polyline(ap) => {
            if axe.id == log_id {
                return Err(bad("An axe cannot chop itself."));
            }
            for (i, (a, b)) in segs.iter().enumerate() {
                for (c, d) in poly_segments(ap) {
                    if let Some(p) = line_segment_intersection(*a, *b, c, d) {
                        out.push(Isect {
                            point: round(p),
                            seg_idx: i,
                        });
                    }
                }
            }
        }
        ObjectKind::Arc(a) => {
            let r = a.center.dist(a.points[1]);
            if !r.is_finite() || r <= 0.0 {
                return Ok(out);
            }
            for (i, (p, q)) in segs.iter().enumerate() {
                for h in line_circle_intersections(*p, *q, a.center, r) {
                    if point_on_arc(a.points, a.ccw, h) {
                        out.push(Isect {
                            point: round(h),
                            seg_idx: i,
                        });
                    }
                }
            }
        }
        ObjectKind::Ellipse(e) => {
            for (i, (p, q)) in segs.iter().enumerate() {
                for h in line_ellipse_intersections(*p, *q, e.center, e.radii, e.angle) {
                    out.push(Isect {
                        point: round(h),
                        seg_idx: i,
                    });
                }
            }
        }
        _ => return Err(bad("an axe must be a polyline, arc or ellipse")),
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PType {
    StartPline,
    StartVertex,
    EndVertex,
    EndPline,
    Cut,
}

#[derive(Debug, Clone, Copy)]
struct SPoint {
    point: Point,
    dist: f64,
    ptype: PType,
}

fn build_seg_points(l: &Polyline, isects: &[Isect]) -> Vec<Vec<SPoint>> {
    let segs = poly_segments(l);
    let mut top = Vec::with_capacity(segs.len());
    for (i, (a, b)) in segs.iter().enumerate() {
        let end = i + 1 == segs.len();
        top.push(vec![
            SPoint {
                point: *a,
                dist: 0.0,
                ptype: if i == 0 { PType::StartPline } else { PType::StartVertex },
            },
            SPoint {
                point: *b,
                dist: a.dist(*b),
                ptype: if end { PType::EndPline } else { PType::EndVertex },
            },
        ]);
    }
    for is in isects {
        let Some(pts) = top.get_mut(is.seg_idx) else {
            continue;
        };
        let Some(first) = pts.first() else { continue };
        pts.push(SPoint {
            point: is.point,
            dist: first.point.dist(is.point),
            ptype: PType::Cut,
        });
    }
    for pts in &mut top {
        pts.sort_by(|a, b| a.dist.total_cmp(&b.dist));
    }
    top
}

fn new_line(l: &Polyline, first: Point) -> Polyline {
    let mut p = l.clone();
    p.points = vec![first];
    p
}

/// `check_poly`: remove duplicate adjacent points, then require enough
/// points for the kind.
fn valid_piece(points: &mut Vec<Point>, polygon: bool) -> bool {
    points.dedup_by(|a, b| a.dist(*b) < 1e-6);
    points.len() >= if polygon { 3 } else { 2 }
}

fn chop_polyline(
    l: &Polyline,
    axes: &[Object],
    log_id: ObjectId,
    pie_arc: bool,
) -> Result<Vec<ObjectKind>, EngineError> {
    if !matches!(l.kind, PolyKind::Polyline | PolyKind::Polygon) {
        return Err(bad("Only unconstrained polylines and polygons may be chopped."));
    }
    let mut isects = Vec::new();
    for axe in axes {
        isects.extend(polyline_axe_intersections(l, axe, log_id)?);
    }
    if isects.is_empty() {
        return Ok(Vec::new());
    }
    if l.kind == PolyKind::Polygon && isects.len() < 2 {
        return Err(bad("Closed figures require two or more intersects."));
    }
    let top = build_seg_points(l, &isects);
    Ok(if l.kind == PolyKind::Polyline {
        dice_polylines(&top, l)
    } else {
        dice_polygons(&top, l, pie_arc)
    })
}

/// `dice_polylines`: split an open polyline into pieces at every cut.
fn dice_polylines(top: &[Vec<SPoint>], l: &Polyline) -> Vec<ObjectKind> {
    let mut pieces = Vec::new();
    let mut current: Option<Polyline> = None;
    let mut completed = false;
    for seg in top {
        for sp in seg {
            if completed {
                break;
            }
            match sp.ptype {
                PType::EndVertex => {}
                PType::StartPline => current = Some(new_line(l, sp.point)),
                PType::StartVertex => {
                    if let Some(cur) = current.as_mut() {
                        cur.points.push(sp.point);
                    }
                }
                PType::Cut => {
                    if let Some(mut cur) = current.take() {
                        cur.points.push(sp.point);
                        if valid_piece(&mut cur.points, false) {
                            pieces.push(ObjectKind::Polyline(cur));
                        }
                    }
                    current = Some(new_line(l, sp.point));
                }
                PType::EndPline => {
                    if let Some(mut cur) = current.take() {
                        cur.points.push(sp.point);
                        if valid_piece(&mut cur.points, false) {
                            pieces.push(ObjectKind::Polyline(cur));
                        }
                    }
                    completed = true;
                }
            }
        }
    }
    pieces
}

fn centroid(pts: &[Point]) -> Point {
    let n = pts.len();
    if n == 0 {
        return Point::ZERO;
    }
    let s = pts.iter().fold(Point::ZERO, |s, p| s + *p);
    s * (1.0 / n as f64)
}

/// `dice_polygons`: split a closed figure into polygons at every cut.
fn dice_polygons(top: &[Vec<SPoint>], l: &Polyline, pie_arc: bool) -> Vec<ObjectKind> {
    let mut all = Vec::new();
    for seg in top {
        all.extend_from_slice(seg);
    }
    let nr_points = all.len();
    let Some(start) = all.iter().position(|sp| sp.ptype == PType::Cut) else {
        return Vec::new();
    };
    let mut pieces = Vec::new();
    let mut current: Option<Polyline> = None;
    let mut start_point = Point::ZERO;
    let focus = centroid(&l.points);
    for i in 0..=nr_points {
        let Some(sp) = all.get((start + i) % nr_points) else {
            continue;
        };
        match sp.ptype {
            PType::EndVertex | PType::EndPline => {}
            PType::StartVertex | PType::StartPline => {
                if let Some(cur) = current.as_mut() {
                    cur.points.push(sp.point);
                }
            }
            PType::Cut => {
                if let Some(mut cur) = current.take() {
                    cur.points.push(sp.point);
                    if pie_arc {
                        cur.points.push(focus);
                    }
                    cur.points.push(start_point);
                    if valid_piece(&mut cur.points, true) && rustfig_geom::polygon_area(&cur.points) > 1e-6 {
                        pieces.push(ObjectKind::Polyline(cur));
                    }
                }
                if i < nr_points.saturating_sub(1) {
                    current = Some(new_line(l, sp.point));
                    start_point = sp.point;
                }
            }
        }
    }
    pieces
}

fn arc_geometry(a: &Arc) -> Option<rustfig_geom::ArcGeom> {
    let g = arc_from_three_points(a.points[0], a.points[1], a.points[2])?;
    Some(arc_geom(a, g))
}

/// Intersections of the log arc with one axe object.
fn arc_axe_intersections(g: &rustfig_geom::ArcGeom, axe: &Object, log_id: ObjectId) -> Result<Vec<Point>, EngineError> {
    let mut out = Vec::new();
    match &axe.kind {
        ObjectKind::Polyline(ap) => {
            for (p, q) in poly_segments(ap) {
                for h in line_circle_intersections(p, q, g.center, g.radius) {
                    if point_on_arc_geom(g, h) {
                        out.push(round(h));
                    }
                }
            }
        }
        ObjectKind::Arc(ax) => {
            if axe.id == log_id {
                return Err(bad("An axe cannot chop itself."));
            }
            let gr = ax.center.dist(ax.points[1]);
            for h in circle_circle_intersections(g.center, g.radius, ax.center, gr) {
                if point_on_arc_geom(g, h) && point_on_arc(ax.points, ax.ccw, h) {
                    out.push(round(h));
                }
            }
        }
        ObjectKind::Ellipse(e) => {
            for h in circle_ellipse_intersections(g.center, g.radius, e.center, e.radii, e.angle) {
                if point_on_arc_geom(g, h) {
                    out.push(round(h));
                }
            }
        }
        _ => return Err(bad("an axe must be a polyline, arc or ellipse")),
    }
    Ok(out)
}

fn chop_arc(a: &Arc, axes: &[Object], log_id: ObjectId) -> Result<Vec<ObjectKind>, EngineError> {
    let Some(g) = arc_geometry(a) else {
        return Ok(Vec::new());
    };
    let mut points = vec![a.points[0], a.points[2]];
    for axe in axes {
        points.extend(arc_axe_intersections(&g, axe, log_id)?);
    }
    if points.len() <= 2 {
        return Ok(Vec::new());
    }
    points.sort_by(|p, q| {
        let pk = arc_position(&g, *p) * g.sweep.signum();
        let qk = arc_position(&g, *q) * g.sweep.signum();
        pk.total_cmp(&qk)
    });
    points.dedup_by(|p, q| p.dist(*q) < 1e-6);
    let mut pieces = Vec::new();
    for w in points.windows(2) {
        let (Some(&p0), Some(&p1)) = (w.first(), w.get(1)) else {
            continue;
        };
        let k0 = arc_position(&g, p0) * g.sweep.signum();
        let k1 = arc_position(&g, p1) * g.sweep.signum();
        let delta = (k1 - k0).abs();
        let chord = 2.0 * g.radius * (delta / 2.0).sin().abs();
        if delta < 1e-6 || chord <= 5.0 {
            continue;
        }
        let mid_k = (k0 + k1) / 2.0;
        let angle = if g.sweep >= 0.0 {
            g.start + mid_k
        } else {
            g.start - mid_k
        };
        let mid = Point::new(g.center.x + g.radius * angle.cos(), g.center.y + g.radius * angle.sin());
        let mut piece = a.clone();
        piece.points = [p0, mid, p1];
        piece.center = g.center;
        piece.ccw = a.ccw;
        pieces.push(ObjectKind::Arc(piece));
    }
    Ok(pieces)
}

/// Intersections of the log circle with one axe object.
fn ellipse_axe_intersections(center: Point, r: f64, axe: &Object, log_id: ObjectId) -> Result<Vec<Point>, EngineError> {
    let mut out = Vec::new();
    match &axe.kind {
        ObjectKind::Polyline(ap) => {
            for (p, q) in poly_segments(ap) {
                out.extend(line_circle_intersections(p, q, center, r).into_iter().map(round));
            }
        }
        ObjectKind::Arc(ax) => {
            let gr = ax.center.dist(ax.points[1]);
            for h in circle_circle_intersections(center, r, ax.center, gr) {
                if point_on_arc(ax.points, ax.ccw, h) {
                    out.push(round(h));
                }
            }
        }
        ObjectKind::Ellipse(e) => {
            if axe.id == log_id {
                return Err(bad("An axe cannot chop itself."));
            }
            if e.kind.is_circle() {
                out.extend(
                    circle_circle_intersections(center, r, e.center, e.radii.x.abs())
                        .into_iter()
                        .map(round),
                );
            } else {
                out.extend(
                    circle_ellipse_intersections(center, r, e.center, e.radii, e.angle)
                        .into_iter()
                        .map(round),
                );
            }
        }
        _ => return Err(bad("an axe must be a polyline, arc or ellipse")),
    }
    Ok(out)
}

fn chop_ellipse(e: &Ellipse, axes: &[Object], log_id: ObjectId) -> Result<Vec<ObjectKind>, EngineError> {
    if !e.kind.is_circle() {
        return Err(bad("Elliptical arcs not (yet) supported."));
    }
    let r = e.radii.x.abs();
    if !r.is_finite() || r <= 0.0 {
        return Err(bad("the circle has no radius"));
    }
    let mut points = Vec::new();
    for axe in axes {
        points.extend(ellipse_axe_intersections(e.center, r, axe, log_id)?);
    }
    if points.len() < 2 {
        return Err(bad("Closed figures require two or more intersects."));
    }
    let mut angles: Vec<(f64, Point)> = points
        .into_iter()
        .map(|p| ((p.y - e.center.y).atan2(p.x - e.center.x), p))
        .collect();
    angles.sort_by(|a, b| a.0.total_cmp(&b.0));
    angles.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-9);
    if let Some(first) = angles.first().copied() {
        angles.push((first.0 + TAU, first.1));
    }
    let mut pieces = Vec::new();
    for w in angles.windows(2) {
        let (Some(&(a0, p0)), Some(&(a1, p1))) = (w.first(), w.get(1)) else {
            continue;
        };
        let delta = a1 - a0;
        if delta < 1e-6 {
            continue;
        }
        let mid_angle = a0 + delta / 2.0;
        let mid = Point::new(e.center.x + r * mid_angle.cos(), e.center.y + r * mid_angle.sin());
        let mut arc = Arc {
            pie: true,
            style: e.style,
            ccw: false,
            forward_arrow: None,
            backward_arrow: None,
            center: e.center,
            points: [p0, mid, p1],
        };
        arc.ccw = false;
        pieces.push(ObjectKind::Arc(arc));
    }
    Ok(pieces)
}
