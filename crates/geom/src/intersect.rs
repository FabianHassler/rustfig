//! Intersections used by the Chop mode.
//!
//! The line/line, line/circle and circle/circle cases are the same as xfig's
//! `w_intersect.c`; line/ellipse is the same quadratic solved by
//! `do_intersect_ellipse_polyline`. Circle/ellipse and ellipse/ellipse are
//! found on the flattened ellipse outline (as `snap.rs` already does for the
//! Snap menu) instead of xfig's quartic solver, within the flattening
//! tolerance.
//
// Ported from xfig src/w_intersect.c (xfig licence, see LICENSE-xfig).

use crate::{arc_from_three_points, flatten_ellipse, Affine, ArcGeom, Point, FLATTEN_TOL};
use std::f64::consts::TAU;

const EPS: f64 = 1e-9;
/// Intersections are rounded to whole Fig units by the caller; when several
/// points represent the same cut (e.g. an exact tangency) this is the
/// distance under which they are merged.
const DUP_EPS: f64 = 1e-6;

fn sorted_roots(a: f64, b: f64, c: f64) -> Vec<f64> {
    if !a.is_finite() || !b.is_finite() || !c.is_finite() {
        return Vec::new();
    }
    if a.abs() <= EPS {
        if b.abs() <= EPS {
            return Vec::new();
        }
        let t = -c / b;
        return if (-EPS..=1.0 + EPS).contains(&t) {
            vec![t.clamp(0.0, 1.0)]
        } else {
            Vec::new()
        };
    }
    let disc = b * b - 4.0 * a * c;
    if disc < -EPS {
        return Vec::new();
    }
    let disc = disc.max(0.0).sqrt();
    let q = -0.5 * (b + b.signum() * disc);
    let (r1, r2) = if q.abs() > EPS {
        (q / a, c / q)
    } else {
        let r = -b / (2.0 * a);
        (r, r)
    };
    let mut out = Vec::new();
    for t in [r1, r2] {
        if t.is_finite() && (-EPS..=1.0 + EPS).contains(&t) {
            out.push(t.clamp(0.0, 1.0));
        }
    }
    out.sort_by(f64::total_cmp);
    out.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    out
}

/// The intersection point of two line segments, if any.
pub fn line_segment_intersection(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    let r = b - a;
    let s = d - c;
    let den = r.x * s.y - r.y * s.x;
    if den.abs() <= EPS {
        return None;
    }
    let t = ((c.x - a.x) * s.y - (c.y - a.y) * s.x) / den;
    let u = ((c.x - a.x) * r.y - (c.y - a.y) * r.x) / den;
    if (-EPS..=1.0 + EPS).contains(&t) && (-EPS..=1.0 + EPS).contains(&u) {
        Some(a + r * t.clamp(0.0, 1.0))
    } else {
        None
    }
}

/// Points where the segment `a`–`b` meets the circle, ordered from `a` to `b`.
pub fn line_circle_intersections(a: Point, b: Point, center: Point, r: f64) -> Vec<Point> {
    if !a.is_finite() || !b.is_finite() || !center.is_finite() || !r.is_finite() || r <= 0.0 {
        return Vec::new();
    }
    let d = b - a;
    let u = a - center;
    let roots = sorted_roots(d.dot(d), 2.0 * u.dot(d), u.dot(u) - r * r);
    roots.iter().map(|t| a + d * *t).collect()
}

/// Points where the segment `a`–`b` meets the ellipse, ordered from `a` to `b`.
pub fn line_ellipse_intersections(a: Point, b: Point, center: Point, radii: Point, angle: f64) -> Vec<Point> {
    if !a.is_finite() || !b.is_finite() || !center.is_finite() || !radii.is_finite() {
        return Vec::new();
    }
    let (rx, ry) = (radii.x.abs(), radii.y.abs());
    if rx <= EPS || ry <= EPS {
        return Vec::new();
    }
    let rot = Affine::rotate(if angle.is_finite() { -angle } else { 0.0 });
    let u = rot.apply_vec(a - center);
    let d = rot.apply_vec(b - a);
    let inv = |v: f64, k: f64| v / k;
    let roots = sorted_roots(
        inv(d.x, rx).powi(2) + inv(d.y, ry).powi(2),
        2.0 * (u.x * d.x / (rx * rx) + u.y * d.y / (ry * ry)),
        (u.x / rx).powi(2) + (u.y / ry).powi(2) - 1.0,
    );
    roots.iter().map(|t| a + (b - a) * *t).collect()
}

fn dedup(points: &mut Vec<Point>) {
    points.sort_by(|p, q| p.x.total_cmp(&q.x).then(p.y.total_cmp(&q.y)));
    points.dedup_by(|a, b| a.dist(*b) <= DUP_EPS);
}

/// Points where two circles meet. Coincident circles yield nothing (xfig
/// reports them as congruent).
pub fn circle_circle_intersections(c1: Point, r1: f64, c2: Point, r2: f64) -> Vec<Point> {
    if !c1.is_finite() || !c2.is_finite() || !r1.is_finite() || !r2.is_finite() || r1 <= 0.0 || r2 <= 0.0 {
        return Vec::new();
    }
    let d = c2 - c1;
    let d2 = d.dot(d);
    if d2 <= EPS {
        return Vec::new();
    }
    let a = (r1 * r1 - r2 * r2 + d2) / (2.0 * d2);
    let h2 = r1 * r1 - a * a * d2;
    if h2 < -1e-6 {
        return Vec::new();
    }
    let p = c1 + d * a;
    if h2.abs() <= 1e-6 {
        return vec![p];
    }
    let h = h2.max(0.0).sqrt();
    let perp = Point::new(-d.y, d.x) * (1.0 / d2.sqrt());
    vec![p + perp * h, p - perp * h]
}

/// Whether `p` lies on the circular arc through `points` (tolerance one Fig
/// unit, matching xfig's rounded intersection points).
pub fn point_on_arc(points: [Point; 3], ccw: bool, p: Point) -> bool {
    let Some(g) = arc_from_three_points(points[0], points[1], points[2]) else {
        return false;
    };
    point_on_arc_geom(&g.in_direction(ccw), p)
}

/// Whether `p` lies on an arc given as geometry.
pub fn point_on_arc_geom(g: &ArcGeom, p: Point) -> bool {
    if !p.is_finite() {
        return false;
    }
    if (p.dist(g.center) - g.radius).abs() > 1.0 {
        return false;
    }
    let ang = (p.y - g.center.y).atan2(p.x - g.center.x);
    g.contains_angle(ang)
}

/// Points where a circle meets a (possibly rotated) ellipse, found on the
/// ellipse's flattened outline. Points are not returned in any particular
/// order; duplicates closer than [`DUP_EPS`] are merged.
pub fn circle_ellipse_intersections(
    circle_center: Point,
    r: f64,
    e_center: Point,
    radii: Point,
    angle: f64,
) -> Vec<Point> {
    let mut pts = flatten_ellipse(e_center, radii.x, radii.y, angle, FLATTEN_TOL);
    if pts.len() < 3 {
        return Vec::new();
    }
    // flatten_ellipse is closed (last == first).
    if pts.len() > 1 {
        pts.pop();
    }
    let n = pts.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (Some(a), Some(b)) = (pts.get(i), pts.get((i + 1) % n)) else {
            continue;
        };
        out.extend(line_circle_intersections(*a, *b, circle_center, r));
    }
    dedup(&mut out);
    out
}

/// Points where two ellipses meet, found on their flattened outlines.
pub fn ellipse_ellipse_intersections(c1: Point, r1: Point, a1: f64, c2: Point, r2: Point, a2: f64) -> Vec<Point> {
    let mut p1 = flatten_ellipse(c1, r1.x, r1.y, a1, FLATTEN_TOL);
    let mut p2 = flatten_ellipse(c2, r2.x, r2.y, a2, FLATTEN_TOL);
    if p1.len() < 3 || p2.len() < 3 {
        return Vec::new();
    }
    p1.pop();
    p2.pop();
    let mut out = Vec::new();
    for (i, q) in p1.iter().enumerate() {
        let Some(qn) = p1.get((i + 1) % p1.len()) else {
            continue;
        };
        for (j, r) in p2.iter().enumerate() {
            let Some(rn) = p2.get((j + 1) % p2.len()) else {
                continue;
            };
            if let Some(p) = line_segment_intersection(*q, *qn, *r, *rn) {
                out.push(p);
            }
        }
    }
    dedup(&mut out);
    out
}

/// The signed angular distance from the start of `g` along its sweep,
/// positive in the arc's direction (0 at the start, |sweep| at the end).
pub fn arc_position(g: &ArcGeom, p: Point) -> f64 {
    let raw = (p.y - g.center.y).atan2(p.x - g.center.x);
    let rel = (raw - g.start).rem_euclid(TAU);
    if g.sweep >= 0.0 {
        rel
    } else if rel == 0.0 {
        0.0
    } else {
        rel - TAU
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Point, b: Point) -> bool {
        a.dist(b) < 1e-6
    }

    #[test]
    fn segment_crossings() {
        let p = line_segment_intersection(
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
            Point::new(10.0, 0.0),
        )
        .unwrap();
        assert!(close(p, Point::new(5.0, 5.0)));
        assert!(line_segment_intersection(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(0.0, 5.0),
            Point::new(10.0, 5.0)
        )
        .is_none());
    }

    #[test]
    fn line_circle_hits() {
        let h = line_circle_intersections(Point::new(-20.0, 0.0), Point::new(20.0, 0.0), Point::ZERO, 10.0);
        assert_eq!(h.len(), 2);
        assert!(close(h[0], Point::new(-10.0, 0.0)));
        assert!(close(h[1], Point::new(10.0, 0.0)));
        assert!(line_circle_intersections(Point::new(0.0, 20.0), Point::new(20.0, 20.0), Point::ZERO, 10.0).is_empty());
    }

    #[test]
    fn line_ellipse_hits() {
        let h = line_ellipse_intersections(
            Point::new(-30.0, 0.0),
            Point::new(30.0, 0.0),
            Point::ZERO,
            Point::new(20.0, 10.0),
            0.0,
        );
        assert_eq!(h.len(), 2);
        assert!(close(h[0], Point::new(-20.0, 0.0)));
        assert!(close(h[1], Point::new(20.0, 0.0)));
    }

    #[test]
    fn circle_circle() {
        let h = circle_circle_intersections(Point::ZERO, 10.0, Point::new(10.0, 0.0), 10.0);
        assert_eq!(h.len(), 2);
        assert!(h.iter().any(|p| close(*p, Point::new(5.0, 8.660254037844386))));
        assert!(circle_circle_intersections(Point::ZERO, 5.0, Point::ZERO, 5.0).is_empty());
        assert!(circle_circle_intersections(Point::ZERO, 5.0, Point::new(30.0, 0.0), 5.0).is_empty());
    }

    #[test]
    fn arc_points() {
        let pts = [Point::new(10.0, 0.0), Point::new(0.0, 10.0), Point::new(-10.0, 0.0)];
        let g = arc_from_three_points(pts[0], pts[1], pts[2]).unwrap();
        assert!(point_on_arc_geom(&g, Point::new(0.0, 10.0)));
        assert!(!point_on_arc_geom(&g, Point::new(0.0, -10.0)));
    }
}
