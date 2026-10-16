//! Exact outlines as lines and cubic Bézier curves, for the vector exports
//! (plan A5): an ellipse is four curves instead of hundreds of points. The
//! raster backend keeps the flattened outlines.

use crate::geom::{Affine, ArcGeom, Point, Rect};
use std::f64::consts::{FRAC_PI_2, TAU};

/// One step of a path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seg {
    /// Start a subpath here.
    Move(Point),
    Line(Point),
    /// A cubic Bézier: two control points, then the end point.
    Cubic(Point, Point, Point),
}

/// Curves along the ellipse `center + rot(rx cos t, ry sin t)` from `t0`
/// over `sweep` (radians), at most a quarter turn each; the start point is
/// not included.
fn elliptic_arc(center: Point, rx: f64, ry: f64, rot: f64, t0: f64, sweep: f64) -> Vec<Seg> {
    if ![center.x, center.y, rx, ry, rot, t0, sweep]
        .iter()
        .all(|v| v.is_finite())
        || sweep == 0.0
    {
        return Vec::new();
    }
    let r = Affine::rotate(rot);
    let at = |t: f64| center + r.apply_vec(Point::new(rx * t.cos(), ry * t.sin()));
    let tangent = |t: f64| r.apply_vec(Point::new(-rx * t.sin(), ry * t.cos()));
    let n = ((sweep.abs() / FRAC_PI_2).ceil() as usize).clamp(1, 16);
    let dt = sweep / n as f64;
    // Control length of a cubic approximating a circular arc of angle dt.
    let k = 4.0 / 3.0 * (dt / 4.0).tan();
    (0..n)
        .map(|i| {
            let a = t0 + dt * i as f64;
            let b = a + dt;
            let (p0, p3) = (at(a), at(b));
            Seg::Cubic(p0 + tangent(a) * k, p3 - tangent(b) * k, p3)
        })
        .collect()
}

/// A closed ellipse (the outline `flatten_ellipse` approximates).
pub fn ellipse_path(center: Point, rx: f64, ry: f64, angle: f64) -> Vec<Seg> {
    let angle = if angle.is_finite() { angle } else { 0.0 };
    let (rx, ry) = (rx.abs(), ry.abs());
    let start = center + Affine::rotate(angle).apply_vec(Point::new(rx, 0.0));
    let mut out = vec![Seg::Move(start)];
    out.extend(elliptic_arc(center, rx, ry, angle, 0.0, TAU));
    out
}

/// A circular arc (as `flatten_arc` approximates it), closed through the
/// centre for a pie wedge.
pub fn arc_path(arc: &ArcGeom, pie: bool) -> Vec<Seg> {
    let mut out = vec![Seg::Move(arc.point_at(arc.start))];
    out.extend(elliptic_arc(
        arc.center, arc.radius, arc.radius, 0.0, arc.start, arc.sweep,
    ));
    if pie {
        out.push(Seg::Line(arc.center));
    }
    out
}

/// A box with rounded corners (as `flatten_round_rect` approximates it).
pub fn round_rect_path(r: &Rect, radius: f64) -> Vec<Seg> {
    if r.is_empty() {
        return Vec::new();
    }
    let rad = radius.max(0.0).min(r.width() / 2.0).min(r.height() / 2.0);
    let corners = [
        (Point::new(r.max.x - rad, r.min.y + rad), -FRAC_PI_2),
        (Point::new(r.max.x - rad, r.max.y - rad), 0.0),
        (Point::new(r.min.x + rad, r.max.y - rad), FRAC_PI_2),
        (Point::new(r.min.x + rad, r.min.y + rad), std::f64::consts::PI),
    ];
    let mut out = Vec::new();
    for (i, (c, start)) in corners.iter().enumerate() {
        let p = Point::new(c.x + rad * start.cos(), c.y + rad * start.sin());
        out.push(if i == 0 { Seg::Move(p) } else { Seg::Line(p) });
        if rad > 0.0 && rad.is_finite() {
            out.extend(elliptic_arc(*c, rad, rad, 0.0, *start, FRAC_PI_2));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{flatten_arc, flatten_ellipse};

    /// Points along a cubic.
    fn sample(segs: &[Seg]) -> Vec<Point> {
        let mut out = Vec::new();
        let mut cur = Point::ZERO;
        for s in segs {
            match *s {
                Seg::Move(p) | Seg::Line(p) => {
                    out.push(p);
                    cur = p;
                }
                Seg::Cubic(a, b, p) => {
                    for i in 1..=16 {
                        let t = f64::from(i) / 16.0;
                        let u = 1.0 - t;
                        out.push(cur * (u * u * u) + a * (3.0 * u * u * t) + b * (3.0 * u * t * t) + p * (t * t * t));
                    }
                    cur = p;
                }
            }
        }
        out
    }

    fn max_gap(curve: &[Point], flat: &[Point]) -> f64 {
        curve
            .iter()
            .map(|p| flat.iter().map(|q| p.dist(*q)).fold(f64::INFINITY, f64::min))
            .fold(0.0, f64::max)
    }

    #[test]
    fn ellipse_matches_its_flattening() {
        let c = Point::new(100.0, 50.0);
        let segs = ellipse_path(c, 600.0, 300.0, 0.4);
        assert_eq!(segs.len(), 5, "move and four quarter curves");
        let flat = flatten_ellipse(c, 600.0, 300.0, 0.4, 1.0);
        assert!(max_gap(&sample(&segs), &flat) < 2.0);
    }

    #[test]
    fn arcs_match_their_flattening_both_ways() {
        for sweep in [2.5, -2.5, 5.9] {
            let a = ArcGeom {
                center: Point::new(10.0, 20.0),
                radius: 900.0,
                start: 0.3,
                sweep,
            };
            let segs = arc_path(&a, false);
            let flat = flatten_arc(&a, 1.0);
            assert!(max_gap(&sample(&segs), &flat) < 2.0, "{sweep}");
            let ends = sample(&segs);
            assert!(ends.last().unwrap().dist(*flat.last().unwrap()) < 1e-6);
        }
        let pie = arc_path(
            &ArcGeom {
                center: Point::ZERO,
                radius: 100.0,
                start: 0.0,
                sweep: 1.0,
            },
            true,
        );
        assert_eq!(pie.last(), Some(&Seg::Line(Point::ZERO)));
    }

    #[test]
    fn round_rects_and_bad_input() {
        let r = Rect::new(Point::ZERO, Point::new(1000.0, 500.0));
        let segs = round_rect_path(&r, 100.0);
        assert_eq!(segs.iter().filter(|s| matches!(s, Seg::Cubic(..))).count(), 4);
        assert!(round_rect_path(&Rect::EMPTY, 10.0).is_empty());
        assert_eq!(ellipse_path(Point::ZERO, f64::NAN, 1.0, 0.0).len(), 1);
    }
}
