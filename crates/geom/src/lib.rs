//! RustFig geometry (layer L0).
//!
//! Points, rectangles, affine transforms, curve flattening and arrowheads
//! come from [`figbase::geom`]; this crate adds hit testing and the
//! intersections used by the Chop mode.
//! Coordinates are Fig units (1200 per inch), y grows downwards.

mod hit;
mod intersect;

pub use figbase::geom::*;
pub use hit::{
    dist_to_polyline, dist_to_segment, point_in_polygon, polygon_area, polygon_contains_rect, polyline_length,
    segments_intersect,
};
pub use intersect::{
    arc_position, circle_circle_intersections, circle_ellipse_intersections, ellipse_ellipse_intersections,
    line_circle_intersections, line_ellipse_intersections, line_segment_intersection, point_on_arc, point_on_arc_geom,
};
