//! Chop mode: cut objects at their intersections (plan D3).
//!
//! xfig's `e_chop.c`: the left button collects the axe (cutting) objects,
//! the middle button picks the log object and chops it with every axe, and
//! the right button clears the axe list. Only polylines, polygons, arcs and
//! circles can be chopped; elliptical arcs are not supported by xfig. The
//! pieces keep the attributes of the object they came from, and one chop is
//! one undo step.
//!
//! RustFig extends this (plan D15, `outline.rs` and `chop_ext.rs`): boxes,
//! arc-boxes, ellipses, splines and compounds can be chopped, splines and
//! compounds can be axes, arrowheads stay only at the original ends, and
//! the pieces replace the log object at its place in the drawing order.
//
// Ported from xfig src/e_chop.c and src/w_intersect.c (xfig licence, see
// LICENSE-xfig).

use super::outline::{self, Mode};
use super::{CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_geom::{
    arc_from_three_points, arc_position, circle_circle_intersections, circle_ellipse_intersections,
    line_circle_intersections, line_ellipse_intersections, line_segment_intersection, point_on_arc, point_on_arc_geom,
    Point,
};
use rustfig_model::{arc_geom, flatten_object, Arc, Ellipse, Object, ObjectId, ObjectKind, PolyKind, Polyline};
use serde_json::{json, Value};
use std::f64::consts::TAU;

pub(super) fn commands() -> Vec<Command> {
    let mut v = vec![Command {
        id: "object.chop",
        label: "Chop objects",
        menu: "",
        shortcut: "",
        params: "{\"id\", \"axes\": [object ids]}  — cut the log object (polyline, polygon, box, arc-box, arc, circle, ellipse, spline or compound; a compound chops every member) at every intersection with the axe objects (any of those kinds; a compound contributes its members). Closed figures become wedges (polygons, closed splines, ellipses) or pie arcs (circles); the pieces replace the log object and keep arrowheads only at the original ends",
        mutates: true,
        enabled: super::always,
        run: chop,
    }];
    v.extend(super::chop_ext::commands());
    v
}

pub(super) fn bad(m: impl Into<String>) -> EngineError {
    EngineError::bad_params(m)
}

pub(super) fn object(e: &Engine, id: ObjectId) -> Result<Object, EngineError> {
    e.doc()
        .get(id)
        .cloned()
        .ok_or_else(|| EngineError::not_found(format!("no top-level object with id {}", id.0)))
}

pub(super) fn id_param(p: &Value, key: &str) -> Result<ObjectId, EngineError> {
    match p.get(key) {
        Some(v) => v
            .as_u64()
            .map(ObjectId)
            .ok_or_else(|| bad(format!("{key:?} must be an object id"))),
        None => Err(bad(format!("missing object id {key:?}"))),
    }
}

/// Most cutting objects (after compounds are expanded) one command takes.
const MAX_AXES: usize = 20_000;

/// The cutting leaves of an object: a compound contributes its members
/// (recursively), texts nothing.
pub(super) fn leaf_objects(o: &Object, out: &mut Vec<Object>, depth: usize) {
    match &o.kind {
        ObjectKind::Compound(c) => {
            if depth < 64 {
                for m in &c.objects {
                    leaf_objects(m, out, depth + 1);
                }
            }
        }
        ObjectKind::Text(_) => {}
        _ => out.push(o.clone()),
    }
}

/// Parse an id list of cutting objects into leaves. `skip` is dropped
/// silently (Trim and Extend ignore the object itself); otherwise an axe
/// equal to `log_id` is an error, as in xfig.
pub(super) fn axe_leaves(
    e: &Engine,
    ids: &[Value],
    log_id: ObjectId,
    skip_log: bool,
) -> Result<Vec<Object>, EngineError> {
    let mut out = Vec::new();
    for v in ids {
        let id = v
            .as_u64()
            .map(ObjectId)
            .ok_or_else(|| bad("axes must be an array of object ids"))?;
        let o = object(e, id)?;
        if matches!(o.kind, ObjectKind::Text(_)) {
            return Err(bad("an axe must be a polyline, arc, ellipse, spline or compound"));
        }
        if id == log_id {
            if skip_log {
                continue;
            }
            return Err(bad("An axe cannot chop itself."));
        }
        leaf_objects(&o, &mut out, 0);
        if out.len() > MAX_AXES {
            return Err(bad("too many axe objects"));
        }
    }
    Ok(out)
}

fn axes_param(e: &Engine, p: &Value, log_id: ObjectId) -> Result<Vec<Object>, EngineError> {
    let Some(Value::Array(arr)) = p.get("axes") else {
        return Err(bad("missing object id list \"axes\""));
    };
    if arr.is_empty() {
        return Err(bad("\"axes\" must name at least one object"));
    }
    axe_leaves(e, arr, log_id, false)
}

fn chop(e: &mut Engine, p: &Value) -> CmdResult {
    let log_id = id_param(p, "id")?;
    let log = object(e, log_id)?;
    let axes = axes_param(e, p, log_id)?;
    let pie_arc = e.indicator.pie_arc;
    let cut = outline::cut_object(e.doc_mut(), &log, &axes, Mode::Chop, pie_arc)?;
    // Pieces made (for a compound: members cut).
    let pieces = match &cut {
        outline::Cut::Unchanged => 0,
        outline::Cut::Pieces(k) => k.len(),
        outline::Cut::Compound(_, n) => *n,
    };
    let ids = super::chop_ext::replace_with(e, &log, cut);
    if ids.is_empty() {
        return Ok(json!({"ids": [], "pieces": 0}));
    }
    e.set_selection(ids.clone());
    Ok(json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>(), "pieces": pieces}))
}

/// The segments of a polyline. Closed kinds (xfig repeats the first point at
/// the end; the Rust model does not) include the closing segment.
pub(super) fn poly_segments(l: &Polyline) -> Vec<(Point, Point)> {
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

/// The segments of any object's drawn outline (the flattened outline the
/// renderer uses: arc-boxes with their round corners, splines as drawn).
pub(super) fn object_segments(o: &Object) -> Vec<(Point, Point)> {
    let mut out = Vec::new();
    for path in flatten_object(o) {
        out.extend(path.pts.windows(2).filter_map(|w| Some((*w.first()?, *w.get(1)?))));
        if let (true, Some(f), Some(l)) = (path.closed, path.pts.first(), path.pts.last()) {
            if f.dist(*l) > 1e-9 {
                out.push((*l, *f));
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Isect {
    pub point: Point,
    pub seg_idx: usize,
}

pub(super) fn round(p: Point) -> Point {
    Point::new(p.x.round(), p.y.round())
}

/// Most intersections one cut collects.
const MAX_CUTS: usize = 200_000;
/// Most segment pairs one cut examines (a hostile figure cannot stall it).
const MAX_PAIRS: usize = 100_000_000;

/// All intersections of the log's segments with the axe objects.
pub(super) fn outline_cuts(segs: &[(Point, Point)], axes: &[Object]) -> Result<Vec<Isect>, EngineError> {
    let mut out = Vec::new();
    let mut pairs = 0usize;
    for axe in axes {
        pairs = pairs.saturating_add(segs.len().saturating_mul(match &axe.kind {
            ObjectKind::Polyline(_) | ObjectKind::Spline(_) => object_segments(axe).len().max(1),
            _ => 1,
        }));
        if pairs > MAX_PAIRS {
            return Err(bad("too many segments to intersect"));
        }
        out.extend(axe_intersections(segs, axe)?);
        if out.len() > MAX_CUTS {
            return Err(bad("too many intersections"));
        }
    }
    Ok(out)
}

/// All intersections of the log segments with one axe object.
fn axe_intersections(segs: &[(Point, Point)], axe: &Object) -> Result<Vec<Isect>, EngineError> {
    let mut out = Vec::new();
    match &axe.kind {
        ObjectKind::Polyline(_) | ObjectKind::Spline(_) => {
            let axe_segs = object_segments(axe);
            for (i, (a, b)) in segs.iter().enumerate() {
                for (c, d) in &axe_segs {
                    if let Some(p) = line_segment_intersection(*a, *b, *c, *d) {
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
        _ => return Err(bad("an axe must be a polyline, arc, ellipse or spline")),
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
pub(super) fn valid_piece(points: &mut Vec<Point>, polygon: bool) -> bool {
    points.dedup_by(|a, b| a.dist(*b) < 1e-6);
    points.len() >= if polygon { 3 } else { 2 }
}

/// xfig's chop of an unconstrained polyline or polygon.
pub(super) fn chop_polyline(l: &Polyline, axes: &[Object], pie_arc: bool) -> Result<Vec<ObjectKind>, EngineError> {
    if !matches!(l.kind, PolyKind::Polyline | PolyKind::Polygon) {
        return Err(bad("Only unconstrained polylines and polygons may be chopped."));
    }
    let isects = outline_cuts(&poly_segments(l), axes)?;
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

pub(super) fn centroid(pts: &[Point]) -> Point {
    let n = pts.len();
    if n == 0 {
        return Point::ZERO;
    }
    let s = pts.iter().fold(Point::ZERO, |s, p| s + *p);
    s * (1.0 / n as f64)
}

/// `dice_polygons`: split a closed figure into polygons at every cut. The
/// pieces do not repeat their first point (xfig's do; the Rust model does
/// not).
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
                    if valid_piece(&mut cur.points, true) && rustfig_geom::polygon_area(&cur.points) > 1e-6 {
                        pieces.push(ObjectKind::Polyline(cur));
                    }
                }
                if i < nr_points.saturating_sub(1) {
                    current = Some(new_line(l, sp.point));
                }
            }
        }
    }
    pieces
}

pub(super) fn arc_geometry(a: &Arc) -> Option<rustfig_geom::ArcGeom> {
    let g = arc_from_three_points(a.points[0], a.points[1], a.points[2])?;
    Some(arc_geom(a, g))
}

/// Intersections of the log arc with one axe object.
fn arc_axe_intersections(g: &rustfig_geom::ArcGeom, axe: &Object) -> Result<Vec<Point>, EngineError> {
    let mut out = Vec::new();
    match &axe.kind {
        ObjectKind::Polyline(_) | ObjectKind::Spline(_) => {
            for (p, q) in object_segments(axe) {
                for h in line_circle_intersections(p, q, g.center, g.radius) {
                    if point_on_arc_geom(g, h) {
                        out.push(round(h));
                    }
                }
            }
        }
        ObjectKind::Arc(ax) => {
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
        _ => return Err(bad("an axe must be a polyline, arc, ellipse or spline")),
    }
    Ok(out)
}

/// The pieces of an arc cut at its intersections with the axe objects.
pub(super) fn chop_arc(a: &Arc, axes: &[Object]) -> Result<Vec<ObjectKind>, EngineError> {
    let Some(g) = arc_geometry(a) else {
        return Ok(Vec::new());
    };
    let mut cuts = Vec::new();
    for axe in axes {
        cuts.extend(arc_axe_intersections(&g, axe)?);
        if cuts.len() > MAX_CUTS {
            return Err(bad("too many intersections"));
        }
    }
    Ok(arc_pieces_at(a, &g, cuts))
}

/// The pieces of an arc cut at the given points (on the arc).
pub(super) fn arc_pieces_at(a: &Arc, g: &rustfig_geom::ArcGeom, cuts: Vec<Point>) -> Vec<ObjectKind> {
    let mut points = vec![a.points[0], a.points[2]];
    points.extend(cuts);
    if points.len() <= 2 {
        return Vec::new();
    }
    points.sort_by(|p, q| {
        let pk = arc_position(g, *p) * g.sweep.signum();
        let qk = arc_position(g, *q) * g.sweep.signum();
        pk.total_cmp(&qk)
    });
    points.dedup_by(|p, q| p.dist(*q) < 1e-6);
    let mut pieces = Vec::new();
    for w in points.windows(2) {
        let (Some(&p0), Some(&p1)) = (w.first(), w.get(1)) else {
            continue;
        };
        let k0 = arc_position(g, p0) * g.sweep.signum();
        let k1 = arc_position(g, p1) * g.sweep.signum();
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
        // Arrowheads only at the original ends.
        piece.forward_arrow = if p1.dist(a.points[2]) < 1e-6 {
            a.forward_arrow
        } else {
            None
        };
        piece.backward_arrow = if p0.dist(a.points[0]) < 1e-6 {
            a.backward_arrow
        } else {
            None
        };
        pieces.push(ObjectKind::Arc(piece));
    }
    pieces
}

/// Intersections of the log circle with one axe object.
fn circle_axe_intersections(center: Point, r: f64, axe: &Object) -> Result<Vec<Point>, EngineError> {
    let mut out = Vec::new();
    match &axe.kind {
        ObjectKind::Polyline(_) | ObjectKind::Spline(_) => {
            for (p, q) in object_segments(axe) {
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
        _ => return Err(bad("an axe must be a polyline, arc, ellipse or spline")),
    }
    Ok(out)
}

/// A circle cut at its intersections with the axe objects: pie wedges
/// (`pie`, Chop) or open arcs (Trim and Cut all).
pub(super) fn chop_circle(e: &Ellipse, axes: &[Object], pie: bool) -> Result<Vec<ObjectKind>, EngineError> {
    let r = e.radii.x.abs();
    if !r.is_finite() || r <= 0.0 {
        return Err(bad("the circle has no radius"));
    }
    let mut points = Vec::new();
    for axe in axes {
        points.extend(circle_axe_intersections(e.center, r, axe)?);
        if points.len() > MAX_CUTS {
            return Err(bad("too many intersections"));
        }
    }
    if points.len() < 2 {
        return if pie {
            Err(bad("Closed figures require two or more intersects."))
        } else {
            Ok(Vec::new())
        };
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
        pieces.push(ObjectKind::Arc(Arc {
            pie,
            style: e.style,
            ccw: false,
            forward_arrow: None,
            backward_arrow: None,
            center: e.center,
            points: [p0, mid, p1],
        }));
    }
    Ok(pieces)
}
