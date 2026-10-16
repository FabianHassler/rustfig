//! Flattening objects into polylines (for bounds, hit testing and rendering).

use crate::{Object, ObjectKind, PolyKind};
use rustfig_geom::{
    arc_from_three_points, flatten_arc, flatten_ellipse, flatten_round_rect, flatten_xspline, Point, Rect,
    FIG_PER_STYLE_UNIT,
};

pub use rustfig_geom::FLATTEN_TOL;

/// A flattened outline.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatPath {
    pub pts: Vec<Point>,
    pub closed: bool,
}

/// Flatten an object's outline(s). Texts and compounds yield nothing.
pub fn flatten_object(o: &Object) -> Vec<FlatPath> {
    match &o.kind {
        ObjectKind::Ellipse(e) => vec![FlatPath {
            pts: flatten_ellipse(e.center, e.radii.x, e.radii.y, e.angle, FLATTEN_TOL),
            closed: true,
        }],
        ObjectKind::Polyline(p) => {
            let pts = match p.kind {
                PolyKind::ArcBox => {
                    let r = Rect::from_points(p.points.iter().copied());
                    flatten_round_rect(&r, p.radius.max(0) as f64 * FIG_PER_STYLE_UNIT)
                }
                _ => p.points.clone(),
            };
            vec![FlatPath {
                pts,
                closed: p.kind.is_closed(),
            }]
        }
        ObjectKind::Spline(s) => vec![FlatPath {
            pts: flatten_xspline(&s.points, &s.shapes, s.closed, FLATTEN_TOL * 2.0),
            closed: s.closed,
        }],
        ObjectKind::Arc(a) => {
            let Some(g) = arc_from_three_points(a.points[0], a.points[1], a.points[2]) else {
                return vec![FlatPath {
                    pts: a.points.to_vec(),
                    closed: false,
                }];
            };
            let mut pts = flatten_arc(&arc_geom(a, g), FLATTEN_TOL);
            if a.pie {
                pts.push(g.center);
            }
            vec![FlatPath { pts, closed: a.pie }]
        }
        ObjectKind::Text(_) | ObjectKind::Compound(_) => Vec::new(),
    }
}

/// The arc as fig2dev draws it: around the circle through the three points,
/// from the first to the last point in the direction of the arc's direction
/// flag ([`rustfig_geom::ArcGeom::in_direction`]).
pub fn arc_geom(a: &crate::Arc, g: rustfig_geom::ArcGeom) -> rustfig_geom::ArcGeom {
    g.in_direction(a.ccw)
}
