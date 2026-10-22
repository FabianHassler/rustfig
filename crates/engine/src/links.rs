//! Smart links (xfig's linked move): when a shape is moved, the end points
//! of lines attached to it follow. Ported from xfig src/u_list.c
//! (`get_links`, `point_on_perim`, `get_interior_links`, `point_on_inside`,
//! `adjust_links`) and src/u_elastic.c (`elastic_links`; xfig licence, see
//! LICENSE-xfig).
//!
//! xfig links the lines of a moved box, arc-box or picture (end within
//! `LINK_TOL` of the perimeter) and of a moved compound (end inside its
//! box), for Move and Copy. D14 extends the same tolerance rule: the targets
//! are also circles, ellipses, arcs, polygons, closed splines and text boxes
//! (end within `LINK_TOL` of the outline); the followers are open polylines
//! and open splines, and a line attached to moved shapes at both ends moves
//! both ends; the operations are every geometric command (move, rotate,
//! flip, scale, align, nudge, copy, move point on a box corner): the
//! attached end point goes through the same transform as the shape. Nothing
//! is stored in the file.

use crate::{Engine, LinkMode};
use rustfig_geom::{Affine, Point, Rect};
use rustfig_model::{flatten_object, Object, ObjectId, ObjectKind, PolyKind};
use std::collections::BTreeSet;

/// How close a line end must be to a shape's outline to count as attached:
/// xfig's `LINK_TOL = 3 * PIX_PER_INCH / DISPLAY_PIX_PER_INCH`
/// = 3 × 1200 / 80 = 45 Fig units.
const LINK_TOL: f64 = 45.0;

/// One end point of a top-level open polyline or spline attached to a moved
/// shape.
#[derive(Debug, Clone)]
pub(crate) struct Link {
    line_id: ObjectId,
    /// Index of the end point in `points` (0 or `points.len() - 1`).
    end: usize,
    /// The point next to the end point, moved by [`LinkMode::Slide`]. `None`
    /// for a single-point line and for a line attached at both ends (both
    /// ends follow their shapes, so no neighbour has to slide).
    prev: Option<usize>,
}

/// What a line end can be attached to.
enum Target {
    /// A box, arc-box or picture: within `LINK_TOL` of the edges of its box
    /// (xfig's `point_on_perim`).
    Perimeter(Rect),
    /// A compound: anywhere inside its box (`point_on_inside`).
    Interior(Rect),
    /// Any other shape: within `LINK_TOL` of its outline.
    Outline {
        reach: Rect,
        paths: Vec<(Vec<Point>, bool)>,
    },
}

fn target_of(o: &Object) -> Option<Target> {
    match &o.kind {
        ObjectKind::Polyline(p) if p.kind.is_rect() => {
            let r = Rect::from_points(p.points.iter().copied());
            (!r.is_empty()).then_some(Target::Perimeter(r))
        }
        ObjectKind::Compound(_) => {
            let r = o.geometry_bbox();
            (!r.is_empty()).then_some(Target::Interior(r))
        }
        ObjectKind::Text(t) => outline(vec![(t.corners().to_vec(), true)]),
        ObjectKind::Polyline(p) if p.kind == PolyKind::Polygon => outline(paths_of(o)),
        ObjectKind::Spline(s) if s.closed => outline(paths_of(o)),
        ObjectKind::Ellipse(_) | ObjectKind::Arc(_) => outline(paths_of(o)),
        _ => None,
    }
}

fn paths_of(o: &Object) -> Vec<(Vec<Point>, bool)> {
    flatten_object(o).into_iter().map(|f| (f.pts, f.closed)).collect()
}

fn outline(paths: Vec<(Vec<Point>, bool)>) -> Option<Target> {
    let mut r = Rect::EMPTY;
    for (pts, _) in &paths {
        for p in pts {
            r.add_point(*p);
        }
    }
    (!r.is_empty()).then(|| Target::Outline {
        reach: r.expand(LINK_TOL),
        paths,
    })
}

/// A line end counts as attached to `t` under the rules above.
fn attached(p: Point, t: &Target) -> bool {
    match t {
        Target::Interior(r) => r.contains(p),
        Target::Perimeter(r) => {
            if p.x < r.min.x - LINK_TOL
                || p.x > r.max.x + LINK_TOL
                || p.y < r.min.y - LINK_TOL
                || p.y > r.max.y + LINK_TOL
            {
                return false;
            }
            (p.x - r.min.x).abs() <= LINK_TOL
                || (p.x - r.max.x).abs() <= LINK_TOL
                || (p.y - r.min.y).abs() <= LINK_TOL
                || (p.y - r.max.y).abs() <= LINK_TOL
        }
        Target::Outline { reach, paths } => {
            reach.contains(p)
                && paths
                    .iter()
                    .any(|(pts, closed)| rustfig_geom::dist_to_polyline(p, pts, *closed) <= LINK_TOL)
        }
    }
}

/// The points of an open polyline or open spline: the objects whose ends
/// follow.
fn follower_points(o: &Object) -> Option<&[Point]> {
    match &o.kind {
        ObjectKind::Polyline(p) if p.kind == PolyKind::Polyline => Some(&p.points),
        ObjectKind::Spline(s) if !s.closed => Some(&s.points),
        _ => None,
    }
}

fn follower_points_mut(o: &mut Object) -> Option<&mut Vec<Point>> {
    match &mut o.kind {
        ObjectKind::Polyline(p) if p.kind == PolyKind::Polyline => Some(&mut p.points),
        ObjectKind::Spline(s) if !s.closed => Some(&mut s.points),
        _ => None,
    }
}

/// Collect the links of the shapes in `ids` (xfig's `get_links` and
/// `get_interior_links`, for every target kind). A line that is itself
/// among the moved objects is skipped, so a shape and its line selected
/// together move as one instead of the line's end moving twice.
pub(crate) fn collect(e: &Engine, ids: &[ObjectId]) -> Vec<Link> {
    if e.indicator.linkmode == LinkMode::Off {
        return Vec::new();
    }
    let targets: Vec<Target> = ids
        .iter()
        .filter_map(|id| e.doc().get(*id))
        .filter_map(target_of)
        .collect();
    collect_for(e, ids, &targets)
}

fn collect_for(e: &Engine, moved: &[ObjectId], targets: &[Target]) -> Vec<Link> {
    let mut links = Vec::new();
    if targets.is_empty() {
        return links;
    }
    let moved: BTreeSet<u64> = moved.iter().map(|i| i.0).collect();
    let hit = |p: Point| targets.iter().any(|t| attached(p, t));
    for o in &e.doc().objects {
        if moved.contains(&o.id.0) {
            continue;
        }
        let Some(pts) = follower_points(o) else { continue };
        let (Some(first), Some(last)) = (pts.first(), pts.last()) else {
            continue;
        };
        let n = pts.len();
        // xfig: a single-point line has no second end.
        let a = hit(*first);
        let b = n > 1 && hit(*last);
        if a {
            links.push(Link {
                line_id: o.id,
                end: 0,
                prev: (n > 1 && !b).then_some(1),
            });
        }
        if b {
            links.push(Link {
                line_id: o.id,
                end: n - 1,
                prev: (!a).then(|| n - 2),
            });
        }
    }
    links
}

/// Move the linked end points through `f` (xfig's `adjust_links`). In slide
/// mode the point next to each end point also moves, along the axis the last
/// segment keeps, by the end point's displacement, so the segment stays
/// axis-aligned (for a rotation, flip or scale that displacement is the end
/// point's own).
pub(crate) fn adjust(e: &mut Engine, mode: LinkMode, links: &[Link], f: &dyn Fn(Point) -> Point) {
    if mode == LinkMode::Off || links.is_empty() {
        return;
    }
    let doc = e.doc_mut();
    for link in links {
        if let Some(points) = doc.get_mut(link.line_id).and_then(follower_points_mut) {
            shift(points, link, mode, f);
        }
    }
}

/// Copy with links (xfig: `adjust_links` with `copying`): every attached
/// line is copied where it is, and the original's end follows the new shape,
/// so each of the two shapes has a line. Then as [`adjust`].
pub(crate) fn copy_adjust(e: &mut Engine, mode: LinkMode, links: &[Link], f: &dyn Fn(Point) -> Point) {
    if mode == LinkMode::Off || links.is_empty() {
        return;
    }
    let mut copied = BTreeSet::new();
    for link in links {
        if copied.insert(link.line_id.0) {
            if let Some(o) = e.doc().get(link.line_id).cloned() {
                e.add_object(o);
            }
        }
    }
    adjust(e, mode, links, f);
}

/// Translate each object by its own (dx, dy) and let the ends attached to it
/// follow (align and align-to move every object by a different amount). The
/// links are collected before anything moves; an end follows the first
/// shape that claims it.
pub(crate) fn translate_each(e: &mut Engine, moves: &[(ObjectId, f64, f64)]) {
    let mode = e.indicator.linkmode;
    let ids: Vec<ObjectId> = moves.iter().map(|m| m.0).collect();
    let mut seen = BTreeSet::new();
    let mut per_shape = Vec::new();
    for (id, dx, dy) in moves {
        let links = if mode == LinkMode::Off {
            Vec::new()
        } else {
            let targets: Vec<Target> = e.doc().get(*id).and_then(target_of).into_iter().collect();
            let mut l = collect_for(e, &ids, &targets);
            l.retain(|k| seen.insert((k.line_id.0, k.end)));
            l
        };
        per_shape.push((*id, *dx, *dy, links));
    }
    for (id, dx, dy, links) in per_shape {
        if dx == 0.0 && dy == 0.0 {
            continue;
        }
        if let Some(o) = e.doc_mut().get_mut(id) {
            o.transform(&Affine::translate(dx, dy), false);
        }
        adjust(e, mode, &links, &|p| Point::new(p.x + dx, p.y + dy));
    }
}

/// The map that takes the points of the box `old` to the same relative
/// places of the box `new` (a box corner moved: the attached ends stay on
/// their edges).
pub(crate) fn rect_map(old: Rect, new: Rect) -> impl Fn(Point) -> Point {
    let axis = |o0: f64, o1: f64, n0: f64, n1: f64| {
        let span = o1 - o0;
        let k = if span.abs() > 1e-9 { (n1 - n0) / span } else { 1.0 };
        move |v: f64| n0 + (v - o0) * k
    };
    let fx = axis(old.min.x, old.max.x, new.min.x, new.max.x);
    let fy = axis(old.min.y, old.max.y, new.min.y, new.max.y);
    move |p: Point| Point::new(fx(p.x), fy(p.y))
}

/// The attached lines as they will be after moving the objects `ids` by
/// (dx, dy), for drawing while the objects are dragged (xfig rubber-bands
/// them, `u_elastic.c`); empty when smart links are off.
pub(crate) fn preview(e: &Engine, ids: &[ObjectId], dx: f64, dy: f64) -> Vec<Object> {
    preview_with(e, &collect(e, ids), &|p| Point::new(p.x + dx, p.y + dy))
}

/// The lines of `links` with their ends moved through `f`.
pub(crate) fn preview_with(e: &Engine, links: &[Link], f: &dyn Fn(Point) -> Point) -> Vec<Object> {
    let mode = e.indicator.linkmode;
    if mode == LinkMode::Off {
        return Vec::new();
    }
    let mut out: Vec<Object> = Vec::new();
    for link in links {
        let pos = match out.iter().position(|o| o.id == link.line_id) {
            Some(i) => i,
            None => {
                let Some(o) = e.doc().get(link.line_id) else { continue };
                out.push(o.clone());
                out.len() - 1
            }
        };
        if let Some(points) = out.get_mut(pos).and_then(follower_points_mut) {
            shift(points, link, mode, f);
        }
    }
    out
}

/// Move one linked end point (and in slide mode its neighbour).
fn shift(points: &mut [Point], link: &Link, mode: LinkMode, f: &dyn Fn(Point) -> Point) {
    let Some(old) = points.get(link.end).copied() else {
        return;
    };
    let new = f(old);
    if !new.x.is_finite() || !new.y.is_finite() {
        return;
    }
    let (dx, dy) = (new.x - old.x, new.y - old.y);
    if mode == LinkMode::Slide {
        if let Some(prev) = link.prev {
            // xfig: a vertical last segment keeps its x, any other keeps
            // its y (`endpt->x == prevpt->x`).
            let vertical = points.get(prev).is_some_and(|q| (old.x - q.x).abs() < 1e-9);
            if let Some(prevpt) = points.get_mut(prev) {
                if vertical {
                    prevpt.x += dx;
                } else {
                    prevpt.y += dy;
                }
            }
        }
    }
    if let Some(endpt) = points.get_mut(link.end) {
        *endpt = new;
    }
}
