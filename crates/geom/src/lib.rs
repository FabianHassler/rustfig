//! RustFig geometry (layer L0).
//!
//! Points, rectangles, affine transforms, curve flattening and arrowheads
//! come from [`figbase::geom`]; this crate adds hit testing.
//! Coordinates are Fig units (1200 per inch), y grows downwards.

mod hit;

pub use figbase::geom::*;
pub use hit::{
    dist_to_polyline, dist_to_segment, point_in_polygon, polygon_area, polygon_contains_rect, polyline_length,
    segments_intersect,
};
