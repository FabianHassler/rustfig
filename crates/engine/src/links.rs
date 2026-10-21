//! Smart links (xfig's linked move): when a box or compound is moved, the
//! end points of lines attached to it follow. Ported from xfig src/u_list.c
//! (`get_links`, `point_on_perim`, `get_interior_links`, `point_on_inside`,
//! `adjust_links`; xfig licence, see LICENSE-xfig).

use crate::{Engine, LinkMode};
use rustfig_geom::{Point, Rect};
use rustfig_model::{ObjectId, ObjectKind, PolyKind};

/// How close a line end must be to a box's perimeter to count as attached:
/// xfig's `LINK_TOL = 3 * PIX_PER_INCH / DISPLAY_PIX_PER_INCH`
/// = 3 × 1200 / 80 = 45 Fig units.
const LINK_TOL: f64 = 45.0;

/// One end point of a top-level polyline attached to a moved box.
pub(crate) struct Link {
    line_id: ObjectId,
    /// Index of the end point in `points` (0 or `points.len() - 1`).
    end: usize,
    /// The point next to the end point, moved by [`LinkMode::Slide`].
    prev: Option<usize>,
}

/// Collect the links of the boxes and compounds in `ids` (xfig's `get_links`
/// for boxes, `get_interior_links` for compounds). A line that is itself
/// among the moved objects is skipped, so a box and its line selected
/// together move as one instead of the line's end moving twice.
pub(crate) fn collect(e: &Engine, ids: &[ObjectId]) -> Vec<Link> {
    let mut links = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for id in ids {
        let Some(o) = e.doc().get(*id) else { continue };
        let (rect, interior) = match &o.kind {
            ObjectKind::Polyline(p) if p.kind.is_rect() => (Rect::from_points(p.points.iter().copied()), false),
            ObjectKind::Compound(_) => (o.geometry_bbox(), true),
            _ => continue,
        };
        if rect.is_empty() {
            continue;
        }
        collect_perimeter(e, ids, &rect, interior, &mut links, &mut seen);
    }
    links
}

/// Add a link for every top-level polyline whose first or last point sits on
/// the moved box's perimeter (or inside it, for a compound).
fn collect_perimeter(
    e: &Engine,
    moved: &[ObjectId],
    rect: &Rect,
    interior: bool,
    links: &mut Vec<Link>,
    seen: &mut std::collections::BTreeSet<(u64, usize)>,
) {
    for o in &e.doc().objects {
        if moved.contains(&o.id) {
            continue;
        }
        let ObjectKind::Polyline(p) = &o.kind else {
            continue;
        };
        // Only open polylines link; a box, polygon, arc-box or picture is a
        // moved target, not a follower.
        if p.kind != PolyKind::Polyline {
            continue;
        }
        let n = p.points.len();
        let Some(first) = p.points.first() else {
            continue;
        };
        if attached(*first, rect, interior) {
            push(
                links,
                seen,
                Link {
                    line_id: o.id,
                    end: 0,
                    prev: (n > 1).then_some(1),
                },
            );
            continue;
        }
        // A second, distinct end point (xfig: a single-point line has none).
        if let Some((last, _)) = p.points.split_last().filter(|(_, rest)| !rest.is_empty()) {
            if attached(*last, rect, interior) {
                push(
                    links,
                    seen,
                    Link {
                        line_id: o.id,
                        end: n - 1,
                        prev: Some(n - 2),
                    },
                );
            }
        }
    }
}

/// Push a link unless this end point is already linked (two overlapping
/// boxes sharing a perimeter could otherwise move it twice).
fn push(links: &mut Vec<Link>, seen: &mut std::collections::BTreeSet<(u64, usize)>, link: Link) {
    if seen.insert((link.line_id.0, link.end)) {
        links.push(link);
    }
}

/// A line end point counts as attached when it is within [`LINK_TOL`] of one
/// of the box's four edges (xfig's `point_on_perim`), or anywhere inside a
/// compound's box (xfig's `point_on_inside`).
fn attached(p: Point, r: &Rect, interior: bool) -> bool {
    if interior {
        return r.contains(p);
    }
    if p.x < r.min.x - LINK_TOL || p.x > r.max.x + LINK_TOL || p.y < r.min.y - LINK_TOL || p.y > r.max.y + LINK_TOL {
        return false;
    }
    (p.x - r.min.x).abs() <= LINK_TOL
        || (p.x - r.max.x).abs() <= LINK_TOL
        || (p.y - r.min.y).abs() <= LINK_TOL
        || (p.y - r.max.y).abs() <= LINK_TOL
}

/// Move the linked end points by (dx, dy) (xfig's `adjust_links`). In slide
/// mode the point next to each end point also moves, along the axis the last
/// segment keeps, so the segment stays axis-aligned.
pub(crate) fn adjust(e: &mut Engine, mode: LinkMode, links: &[Link], dx: f64, dy: f64) {
    if mode == LinkMode::Off || links.is_empty() {
        return;
    }
    let doc = e.doc_mut();
    for link in links {
        let Some(o) = doc.get_mut(link.line_id) else {
            continue;
        };
        let ObjectKind::Polyline(p) = &mut o.kind else {
            continue;
        };
        if mode == LinkMode::Slide {
            if let Some(prev) = link.prev {
                // xfig: a vertical last segment keeps its x, any other keeps
                // its y (`endpt->x == prevpt->x`).
                let vertical = match (p.points.get(link.end), p.points.get(prev)) {
                    (Some(endpt), Some(prevpt)) => (endpt.x - prevpt.x).abs() < 1e-9,
                    _ => false,
                };
                if let Some(prevpt) = p.points.get_mut(prev) {
                    if vertical {
                        prevpt.x += dx;
                    } else {
                        prevpt.y += dy;
                    }
                }
            }
        }
        if let Some(endpt) = p.points.get_mut(link.end) {
            endpt.x += dx;
            endpt.y += dy;
        }
    }
}
