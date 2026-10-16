//! Circular arcs and ellipses.

use crate::geom::{Affine, Point};
use std::f64::consts::TAU;

/// A circular arc through three points, from `p1` via `p2` to `p3`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcGeom {
    pub center: Point,
    pub radius: f64,
    /// Start angle (radians, screen coordinates: atan2(dy, dx) with y down).
    pub start: f64,
    /// Signed sweep from `start`. Positive sweeps turn clockwise on screen.
    pub sweep: f64,
}

impl ArcGeom {
    /// xfig's direction flag: true when the arc runs counter-clockwise on screen.
    pub fn counter_clockwise(&self) -> bool {
        self.sweep < 0.0
    }

    /// The arc as fig2dev draws it: around the same circle from the same
    /// start, in the direction `ccw` (the Fig direction flag), even when the
    /// middle point lies on the other side (as in files not written by xfig).
    pub fn in_direction(self, ccw: bool) -> ArcGeom {
        if self.counter_clockwise() == ccw {
            return self;
        }
        let sweep = if self.sweep > 0.0 {
            self.sweep - TAU
        } else {
            self.sweep + TAU
        };
        ArcGeom { sweep, ..self }
    }

    pub fn point_at(&self, angle: f64) -> Point {
        Point::new(
            self.center.x + self.radius * angle.cos(),
            self.center.y + self.radius * angle.sin(),
        )
    }

    /// Whether `angle` lies within the swept range.
    pub fn contains_angle(&self, angle: f64) -> bool {
        let rel = (angle - self.start).rem_euclid(TAU);
        if self.sweep >= 0.0 {
            rel <= self.sweep + 1e-12
        } else {
            rel == 0.0 || rel >= TAU + self.sweep - 1e-12
        }
    }
}

/// The circle through three points, oriented to pass from `p1` through `p2`
/// to `p3`. Returns `None` for collinear or non-finite points.
pub fn arc_from_three_points(p1: Point, p2: Point, p3: Point) -> Option<ArcGeom> {
    let d = 2.0 * (p1.x * (p2.y - p3.y) + p2.x * (p3.y - p1.y) + p3.x * (p1.y - p2.y));
    if d.abs() < 1e-9 || !d.is_finite() {
        return None;
    }
    let s1 = p1.x * p1.x + p1.y * p1.y;
    let s2 = p2.x * p2.x + p2.y * p2.y;
    let s3 = p3.x * p3.x + p3.y * p3.y;
    let cx = (s1 * (p2.y - p3.y) + s2 * (p3.y - p1.y) + s3 * (p1.y - p2.y)) / d;
    let cy = (s1 * (p3.x - p2.x) + s2 * (p1.x - p3.x) + s3 * (p2.x - p1.x)) / d;
    let center = Point::new(cx, cy);
    if !center.is_finite() {
        return None;
    }
    Some(arc_with_center(center, p1, p2, p3))
}

/// Arc around a given centre from the direction of `p1` via `p2` to `p3`.
pub(crate) fn arc_with_center(center: Point, p1: Point, p2: Point, p3: Point) -> ArcGeom {
    let ang = |p: Point| (p.y - center.y).atan2(p.x - center.x);
    let (a1, a2, a3) = (ang(p1), ang(p2), ang(p3));
    let d13 = (a3 - a1).rem_euclid(TAU);
    let d12 = (a2 - a1).rem_euclid(TAU);
    let sweep = if d12 <= d13 { d13 } else { d13 - TAU };
    ArcGeom {
        center,
        radius: center.dist(p1),
        start: a1,
        sweep,
    }
}

/// Flatten an arc into points from start to end with chord length about `tol`.
pub fn flatten_arc(arc: &ArcGeom, tol: f64) -> Vec<Point> {
    let tol = if tol.is_finite() && tol > 0.0 { tol } else { 10.0 };
    let len = (arc.radius * arc.sweep).abs();
    if !len.is_finite() {
        return Vec::new();
    }
    let steps = ((len / tol).ceil() as usize).clamp(4, 4096);
    (0..=steps)
        .map(|i| arc.point_at(arc.start + arc.sweep * i as f64 / steps as f64))
        .collect()
}

/// Flatten an ellipse with radii `rx`, `ry`, rotated by `angle` (radians,
/// counter-clockwise on screen). The result is closed (last == first).
pub fn flatten_ellipse(center: Point, rx: f64, ry: f64, angle: f64, tol: f64) -> Vec<Point> {
    let tol = if tol.is_finite() && tol > 0.0 { tol } else { 10.0 };
    let (rx, ry) = (rx.abs(), ry.abs());
    if !(rx.is_finite() && ry.is_finite() && center.is_finite()) {
        return Vec::new();
    }
    let circumference = TAU * rx.max(ry);
    let steps = ((circumference / tol).ceil() as usize).clamp(12, 4096);
    let rot = Affine::rotate(if angle.is_finite() { angle } else { 0.0 });
    (0..=steps)
        .map(|i| {
            let t = TAU * i as f64 / steps as f64;
            center + rot.apply_vec(Point::new(rx * t.cos(), ry * t.sin()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_point_arc() {
        let a = arc_from_three_points(Point::new(100.0, 0.0), Point::new(0.0, 100.0), Point::new(-100.0, 0.0)).unwrap();
        assert!(a.center.dist(Point::ZERO) < 1e-9);
        assert!((a.radius - 100.0).abs() < 1e-9);
        // From +x through +y (down on screen) to -x: increasing screen angle → clockwise.
        assert!(a.sweep > 0.0);
        assert!(!a.counter_clockwise());
        let pts = flatten_arc(&a, 5.0);
        assert!(pts.first().unwrap().dist(Point::new(100.0, 0.0)) < 1e-9);
        assert!(pts.last().unwrap().dist(Point::new(-100.0, 0.0)) < 1e-6);
        assert!(pts.iter().all(|p| p.y >= -1e-9));
    }

    #[test]
    fn reverse_arc_is_ccw() {
        let a = arc_from_three_points(Point::new(-100.0, 0.0), Point::new(0.0, 100.0), Point::new(100.0, 0.0)).unwrap();
        assert!(a.counter_clockwise());
        assert!(a.contains_angle(std::f64::consts::FRAC_PI_2));
        assert!(!a.contains_angle(-std::f64::consts::FRAC_PI_2));
    }

    #[test]
    fn collinear_is_none() {
        assert!(arc_from_three_points(Point::ZERO, Point::new(1.0, 1.0), Point::new(2.0, 2.0)).is_none());
    }

    #[test]
    fn ellipse_extent() {
        let pts = flatten_ellipse(Point::ZERO, 200.0, 100.0, 0.0, 5.0);
        let r = crate::geom::Rect::from_points(pts);
        assert!((r.width() - 400.0).abs() < 1.0);
        assert!((r.height() - 200.0).abs() < 1.0);
    }
}
