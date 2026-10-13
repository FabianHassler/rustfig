//! Distances, containment and measurement.

use crate::Point;

/// Distance from `p` to the segment `a`–`b`.
pub fn dist_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let l2 = ab.dot(ab);
    if l2 <= 1e-18 {
        return p.dist(a);
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    p.dist(a + ab * t)
}

/// Distance from `p` to a polyline (closing it back to the start if `closed`).
pub fn dist_to_polyline(p: Point, pts: &[Point], closed: bool) -> f64 {
    let mut best = match pts.first() {
        Some(f) => p.dist(*f),
        None => return f64::INFINITY,
    };
    for w in pts.windows(2) {
        if let [a, b] = w {
            best = best.min(dist_to_segment(p, *a, *b));
        }
    }
    if closed {
        if let (Some(f), Some(l)) = (pts.first(), pts.last()) {
            best = best.min(dist_to_segment(p, *l, *f));
        }
    }
    best
}

/// Even-odd point-in-polygon test.
pub fn point_in_polygon(p: Point, poly: &[Point]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (Some(a), Some(b)) = (poly.get(i), poly.get(j)) else {
            break;
        };
        if (a.y > p.y) != (b.y > p.y) {
            let x = (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x;
            if p.x < x {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Unsigned area of a simple polygon (shoelace formula).
pub fn polygon_area(poly: &[Point]) -> f64 {
    let n = poly.len();
    if n < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..n {
        if let (Some(a), Some(b)) = (poly.get(i), poly.get((i + 1) % n)) {
            s += a.cross(*b);
        }
    }
    (s / 2.0).abs()
}

/// Total length of a polyline.
pub fn polyline_length(pts: &[Point], closed: bool) -> f64 {
    let mut l: f64 = pts
        .windows(2)
        .map(|w| match w {
            [a, b] => a.dist(*b),
            _ => 0.0,
        })
        .sum();
    if closed {
        if let (Some(f), Some(e)) = (pts.first(), pts.last()) {
            l += f.dist(*e);
        }
    }
    l
}

/// Whether segments `a1`–`a2` and `b1`–`b2` intersect.
pub fn segments_intersect(a1: Point, a2: Point, b1: Point, b2: Point) -> bool {
    let d1 = (a2 - a1).cross(b1 - a1);
    let d2 = (a2 - a1).cross(b2 - a1);
    let d3 = (b2 - b1).cross(a1 - b1);
    let d4 = (b2 - b1).cross(a2 - b1);
    ((d1 > 0.0) != (d2 > 0.0)) && ((d3 > 0.0) != (d4 > 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_distance() {
        let d = dist_to_segment(Point::new(5.0, 5.0), Point::ZERO, Point::new(10.0, 0.0));
        assert!((d - 5.0).abs() < 1e-12);
        let d = dist_to_segment(Point::new(-3.0, 4.0), Point::ZERO, Point::new(10.0, 0.0));
        assert!((d - 5.0).abs() < 1e-12);
    }

    #[test]
    fn polygon_tests() {
        let sq = [
            Point::ZERO,
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        assert!(point_in_polygon(Point::new(5.0, 5.0), &sq));
        assert!(!point_in_polygon(Point::new(15.0, 5.0), &sq));
        assert!((polygon_area(&sq) - 100.0).abs() < 1e-12);
        assert!((polyline_length(&sq, true) - 40.0).abs() < 1e-12);
        assert!(segments_intersect(
            Point::ZERO,
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
            Point::new(10.0, 0.0)
        ));
    }
}
