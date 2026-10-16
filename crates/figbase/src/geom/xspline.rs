//! X-spline evaluation, after C. Blanc and C. Schlick, "X-Splines: A Spline
//! Model Designed for the End User", SIGGRAPH '95.
//!
//! Each control point carries a shape factor `s` in [-1, 1]: positive values
//! approximate the point (B-spline like), negative values interpolate it, and 0
//! gives a sharp corner through the point.

use crate::geom::Point;

/// Upper bound on the number of points produced by one flattening.
pub const XSPLINE_MAX_POINTS: usize = 2_000_000;

/// The general blending function F(u, p) with p = 2 d^2 (paper §4).
fn f_blend(num: f64, den: f64) -> f64 {
    if den == 0.0 {
        return 0.0;
    }
    let p = 2.0 * den * den;
    let u = num / den;
    let u2 = u * u;
    u * u2 * (10.0 - p + (2.0 * p - 15.0) * u + (6.0 - p) * u2)
}

/// Interpolating blending function G (paper §5), with p = 2.
fn g_blend(u: f64, q: f64) -> f64 {
    let p = 2.0;
    u * (q + u * (2.0 * q + u * (10.0 - 12.0 * q - p + u * (2.0 * p + 14.0 * q - 15.0 + u * (6.0 - 5.0 * q - p)))))
}

/// Interpolating blending function H (paper §5).
fn h_blend(u: f64, q: f64) -> f64 {
    let u2 = u * u;
    u * (q + u * (2.0 * q + u2 * (-2.0 * q - u * q)))
}

/// Blending weights for the segment between p1 and p2 at parameter t.
fn weights(t: f64, s1: f64, s2: f64) -> [f64; 4] {
    let (a0, a2) = if s1 < 0.0 {
        let q = -s1;
        (h_blend(-t, q), g_blend(t, q))
    } else {
        let a0 = if t < s1 { f_blend(t - s1, -1.0 - s1) } else { 0.0 };
        (a0, f_blend(t + s1, 1.0 + s1))
    };
    let (a1, a3) = if s2 < 0.0 {
        let q = -s2;
        (g_blend(1.0 - t, q), h_blend(t - 1.0, q))
    } else {
        let a1 = f_blend(t - 1.0 - s2, -1.0 - s2);
        let a3 = if t > 1.0 - s2 {
            f_blend(t - 1.0 + s2, 1.0 + s2)
        } else {
            0.0
        };
        (a1, a3)
    };
    [a0, a1, a2, a3]
}

fn eval(p: [Point; 4], w: [f64; 4]) -> Option<Point> {
    let sum: f64 = w.iter().sum();
    if sum.abs() < 1e-12 || !sum.is_finite() {
        return None;
    }
    let x = (w[0] * p[0].x + w[1] * p[1].x + w[2] * p[2].x + w[3] * p[3].x) / sum;
    let y = (w[0] * p[0].y + w[1] * p[1].y + w[2] * p[2].y + w[3] * p[3].y) / sum;
    Some(Point::new(x, y))
}

/// Flatten an X-spline. `shapes` holds one shape factor per control point
/// (missing entries count as 0). `tolerance` is the target chord length in Fig
/// units. Open splines pass through their end points.
pub fn flatten_xspline(points: &[Point], shapes: &[f64], closed: bool, tolerance: f64) -> Vec<Point> {
    let n = points.len();
    if n < 2 {
        return points.to_vec();
    }
    let shape = |i: usize| -> f64 {
        let s = shapes.get(i).copied().unwrap_or(0.0);
        if s.is_finite() {
            s.clamp(-1.0, 1.0)
        } else {
            0.0
        }
    };
    let tol = if tolerance.is_finite() && tolerance > 0.0 {
        tolerance
    } else {
        20.0
    };

    // Control polygon with its shape factors, extended so that every segment
    // has four neighbours.
    let mut cp: Vec<(Point, f64)> = Vec::with_capacity(n + 3);
    if closed {
        for i in 0..n + 3 {
            let j = (i + n - 1) % n;
            if let Some(p) = points.get(j) {
                cp.push((*p, shape(j)));
            }
        }
    } else {
        let first = points.first().copied().unwrap_or_default();
        let last = points.last().copied().unwrap_or_default();
        cp.push((first, 0.0));
        for (i, p) in points.iter().enumerate() {
            let s = if i == 0 || i == n - 1 { 0.0 } else { shape(i) };
            cp.push((*p, s));
        }
        cp.push((last, 0.0));
    }

    let mut out = Vec::new();
    let segs = if closed { n } else { n - 1 };
    for k in 0..segs {
        let (Some(c0), Some(c1), Some(c2), Some(c3)) = (cp.get(k), cp.get(k + 1), cp.get(k + 2), cp.get(k + 3)) else {
            break;
        };
        let p = [c0.0, c1.0, c2.0, c3.0];
        let hull = c0.0.dist(c1.0) + c1.0.dist(c2.0) + c2.0.dist(c3.0);
        let steps = ((hull / tol).ceil() as usize).clamp(2, 256);
        for i in 0..steps {
            if out.len() >= XSPLINE_MAX_POINTS {
                return out;
            }
            let t = i as f64 / steps as f64;
            if let Some(q) = eval(p, weights(t, c1.1, c2.1)) {
                out.push(q);
            }
        }
    }
    if closed {
        if let Some(f) = out.first().copied() {
            out.push(f);
        }
    } else if let Some(l) = points.last() {
        out.push(*l);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts() -> Vec<Point> {
        vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 100.0),
            Point::new(0.0, 100.0),
        ]
    }

    #[test]
    fn open_spline_hits_endpoints() {
        let p = pts();
        let out = flatten_xspline(&p, &[0.0, 1.0, 1.0, 0.0], false, 5.0);
        assert!(out.first().unwrap().dist(p[0]) < 1e-9);
        assert!(out.last().unwrap().dist(p[3]) < 1e-9);
    }

    #[test]
    fn zero_shape_is_polyline() {
        let p = pts();
        let out = flatten_xspline(&p, &[0.0; 4], false, 5.0);
        // Every point lies on the control polygon.
        for q in out {
            let d = p
                .windows(2)
                .map(|w| {
                    let (a, b) = (w[0], w[1]);
                    let ab = b - a;
                    let t = ((q - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
                    q.dist(a + ab * t)
                })
                .fold(f64::INFINITY, f64::min);
            assert!(d < 1e-6, "{q:?} {d}");
        }
    }

    #[test]
    fn interpolating_passes_through_points() {
        let p = pts();
        let out = flatten_xspline(&p, &[0.0, -1.0, -1.0, 0.0], false, 1.0);
        for cp in &p {
            let best = out.iter().map(|q| q.dist(*cp)).fold(f64::INFINITY, f64::min);
            assert!(best < 1e-6, "{cp:?} {best}");
        }
    }

    #[test]
    fn approximating_stays_inside_hull() {
        let p = pts();
        let out = flatten_xspline(&p, &[1.0; 4], true, 5.0);
        assert!(out.len() > 8);
        for q in out {
            assert!(q.x >= -1e-9 && q.x <= 100.0 + 1e-9 && q.y >= -1e-9 && q.y <= 100.0 + 1e-9);
        }
    }

    #[test]
    fn degenerate_inputs() {
        assert!(flatten_xspline(&[], &[], false, 5.0).is_empty());
        assert_eq!(flatten_xspline(&[Point::ZERO], &[], true, 5.0).len(), 1);
        let p = vec![Point::new(f64::NAN, 0.0), Point::new(1.0, 1.0)];
        let _ = flatten_xspline(&p, &[f64::NAN], true, f64::NAN);
    }
}
