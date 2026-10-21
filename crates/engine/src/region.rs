//! The region of xfig's region selections: the rectangle of two clicks, or
//! (plan W15) the freehand lasso drawn by pressing and dragging.

use rustfig_geom::{point_in_polygon, polygon_contains_rect, Point, Rect};

/// Where objects or points are taken from.
#[derive(Debug, Clone, PartialEq)]
pub enum Region {
    Rect(Rect),
    /// A closed freehand path (the last point joins the first).
    Lasso(Vec<Point>),
}

impl Region {
    /// Whether `r` lies completely inside.
    pub fn contains_rect(&self, r: &Rect) -> bool {
        match self {
            Region::Rect(q) => q.contains_rect(r),
            Region::Lasso(poly) => polygon_contains_rect(poly, r),
        }
    }

    /// Whether the point lies inside.
    pub fn contains(&self, p: Point) -> bool {
        match self {
            Region::Rect(q) => q.contains(p),
            Region::Lasso(poly) => point_in_polygon(p, poly),
        }
    }
}
