//! fig2dev's spline conversion (`trans_spline.c`): X-splines (Blanc and
//! Schlick, "X-splines: a spline model designed for the end user",
//! SIGGRAPH '95) as polylines. fig2dev's reader turns every Fig 3.2
//! spline into a line this way; some drivers convert their own shapes.
//
// Ported from fig2dev trans_spline.c and trans_spline.h (xfig licence, see
// LICENSE-xfig).

use crate::object::{
    Control, Line, Point, Spline, S_SPLINE_ANGULAR, S_SPLINE_APPROX, S_SPLINE_INTERP, T_POLYGON, T_POLYLINE,
};
use crate::Ctx;

/// At most this many points per converted spline (`MAXNUMPTS`).
pub const MAXNUMPTS: usize = 25000;
/// The precision the reader converts splines with.
pub const HIGH_PRECISION: f32 = 0.5;
/// Points dropped at an end with an arrowhead, so that the arrowhead
/// gets a good direction (`ARROW_START`).
pub const ARROW_START: usize = 4;
const MAX_SPLINE_STEP: f64 = 0.2;

/// The point array the curve is collected in (C's static `points`).
struct PointArray {
    pts: Vec<Point>,
    /// A point was refused because the array was full.
    overflow: bool,
}

impl PointArray {
    fn new() -> PointArray {
        PointArray {
            pts: Vec::with_capacity(300),
            overflow: false,
        }
    }

    /// `add_point`: identical successive points are dropped.
    fn add(&mut self, x: i32, y: i32) {
        if self.pts.last() == Some(&Point::new(x, y)) {
            return;
        }
        if self.pts.len() >= MAXNUMPTS {
            self.overflow = true;
            return;
        }
        self.pts.push(Point::new(x, y));
    }

    /// Hand over the points, reporting an overflow once (fig2dev prints the
    /// message for every point it refuses).
    fn finish(self, ctx: &mut Ctx) -> Vec<Point> {
        if self.overflow {
            ctx.warn(format!(
                "Too many points, recompile with MAXNUMPTS > {MAXNUMPTS} in trans_spline.h"
            ));
        }
        self.pts
    }
}

/// The control points of one segment, and their shape factors.
#[derive(Clone, Copy)]
struct Segment {
    p: [Point; 4],
    s1: f64,
    s2: f64,
}

/// The shape factors of `s`: its own if there is one per point, otherwise
/// those `make_control_factors` would give it.
fn shape_factors(s: &Spline) -> Vec<f64> {
    if s.controls.len() == s.points.len() {
        s.controls.iter().map(|c| c.s).collect()
    } else {
        control_factors(s).iter().map(|c| c.s).collect()
    }
}

fn segment(s: &Spline, factors: &[f64], idx: [usize; 4]) -> Option<Segment> {
    let pt = |i: usize| s.points.get(i).copied();
    Some(Segment {
        p: [pt(idx[0])?, pt(idx[1])?, pt(idx[2])?, pt(idx[3])?],
        s1: *factors.get(idx[1])?,
        s2: *factors.get(idx[2])?,
    })
}

/// `compute_open_spline`: the points of an open X-spline. `None` for a
/// spline with fewer than two points.
pub fn compute_open_spline(ctx: &mut Ctx, s: &Spline, precision: f32) -> Option<Vec<Point>> {
    let n = s.points.len();
    let mut arr = PointArray::new();
    if n < 2 {
        return None;
    }
    // a two-point spline is a straight line
    if n == 2 {
        for q in &s.points {
            arr.add(q.x, q.y);
        }
        return Some(arr.finish(ctx));
    }
    let factors = shape_factors(s);
    // The first control point counts twice for the first segment, the
    // last twice for the last segment.
    let last = n - 1;
    let mut k = 0;
    for kk in 0..=n - 3 {
        k = kk;
        let idx = [kk.saturating_sub(1), kk, kk + 1, kk + 2];
        let seg = segment(s, &factors, idx)?;
        spline_segment(&mut arr, kk as i32, &seg, precision);
        if arr.overflow {
            break;
        }
    }
    let seg = segment(s, &factors, [last - 2, last - 1, last, last])?;
    if !arr.overflow {
        spline_segment(&mut arr, k as i32, &seg, precision);
    }
    let end = seg.p[3];
    arr.add(end.x, end.y);
    Some(arr.finish(ctx))
}

/// `compute_closed_spline`: the points of a closed X-spline, the first
/// point repeated at the end. `None` (with fig2dev's message) for a
/// spline with fewer than three points.
pub fn compute_closed_spline(ctx: &mut Ctx, s: &Spline, precision: f32) -> Option<Vec<Point>> {
    let n = s.points.len();
    if n < 3 {
        ctx.warn("A closed spline with less than three points.");
        return None;
    }
    let factors = shape_factors(s);
    let mut arr = PointArray::new();
    // one segment per point, joining the end to the beginning
    for k in 0..n {
        let seg = segment(s, &factors, [k, (k + 1) % n, (k + 2) % n, (k + 3) % n])?;
        spline_segment(&mut arr, k as i32, &seg, precision);
        if arr.overflow {
            break;
        }
    }
    if let Some(&first) = arr.pts.first() {
        arr.add(first.x, first.y);
    }
    Some(arr.finish(ctx))
}

fn f_blend(numerator: f64, denominator: f64) -> f64 {
    let p = 2.0 * denominator * denominator;
    let u = numerator / denominator;
    let u2 = u * u;
    u * u2 * (10.0 - p + (2.0 * p - 15.0) * u + (6.0 - p) * u2)
}

/// p equals 2.
fn g_blend(u: f64, q: f64) -> f64 {
    u * (q + u * (2.0 * q + u * (8.0 - 12.0 * q + u * (14.0 * q - 11.0 + u * (4.0 - 5.0 * q)))))
}

fn h_blend(u: f64, q: f64) -> f64 {
    let u2 = u * u;
    u * (q + u * (2.0 * q + u2 * (-2.0 * q - u * q)))
}

/// `(A0, A2)` for a negative `s1`.
fn negative_s1_influence(t: f64, s1: f64) -> (f64, f64) {
    (h_blend(-t, -s1), g_blend(t, -s1))
}

/// `(A1, A3)` for a negative `s2`.
fn negative_s2_influence(t: f64, s2: f64) -> (f64, f64) {
    (g_blend(1.0 - t, -s2), h_blend(t - 1.0, -s2))
}

/// `(A0, A2)` for a positive `s1`.
fn positive_s1_influence(k: i32, t: f64, s1: f64) -> (f64, f64) {
    let k = k as f64;
    let tk = k + 1.0 + s1;
    let a0 = if t + k + 1.0 < tk {
        f_blend(t + k + 1.0 - tk, k - tk)
    } else {
        0.0
    };
    let tk = k + 1.0 - s1;
    let a2 = f_blend(t + k + 1.0 - tk, k + 2.0 - tk);
    (a0, a2)
}

/// `(A1, A3)` for a positive `s2`.
fn positive_s2_influence(k: i32, t: f64, s2: f64) -> (f64, f64) {
    let k = k as f64;
    let tk = k + 2.0 + s2;
    let a1 = f_blend(t + k + 1.0 - tk, k + 1.0 - tk);
    let tk = k + 2.0 - s2;
    let a3 = if t + k + 1.0 > tk {
        f_blend(t + k + 1.0 - tk, k + 3.0 - tk)
    } else {
        0.0
    };
    (a1, a3)
}

/// The blending weights `[A0, A1, A2, A3]` at `t`, by the signs of the
/// shape factors, as `spline_segment_computing` picks them.
fn blend(k: i32, t: f64, s1: f64, s2: f64) -> [f64; 4] {
    let ((a0, a2), (a1, a3)) = if s1 < 0.0 {
        if s2 < 0.0 {
            (negative_s1_influence(t, s1), negative_s2_influence(t, s2))
        } else {
            (negative_s1_influence(t, s1), positive_s2_influence(k, t, s2))
        }
    } else if s2 < 0.0 {
        (positive_s1_influence(k, t, s1), negative_s2_influence(t, s2))
    } else {
        (positive_s1_influence(k, t, s1), positive_s2_influence(k, t, s2))
    };
    [a0, a1, a2, a3]
}

/// `point_computing`: the weighted mean of the four control points.
fn point_computing(a: &[f64; 4], p: &[Point; 4]) -> (i32, i32) {
    let w = a[0] + a[1] + a[2] + a[3];
    let num = |f: fn(&Point) -> i32| {
        a[0] * f(&p[0]) as f64 + a[1] * f(&p[1]) as f64 + a[2] * f(&p[2]) as f64 + a[3] * f(&p[3]) as f64
    };
    (crate::round(num(|q| q.x) / w), crate::round(num(|q| q.y) / w))
}

/// `step_computing`: the parameter step for the segment from `p1` to
/// `p2`, finer for long and for strongly curved segments.
fn step_computing(k: i32, seg: &Segment, precision: f32) -> f32 {
    let Segment { p, s1, s2 } = *seg;
    // origin
    let (xstart, ystart) = if s1 > 0.0 {
        let a = if s2 < 0.0 {
            let ((a0, a2), (a1, a3)) = (positive_s1_influence(k, 0.0, s1), negative_s2_influence(0.0, s2));
            [a0, a1, a2, a3]
        } else {
            let ((a0, a2), (a1, a3)) = (positive_s1_influence(k, 0.0, s1), positive_s2_influence(k, 0.0, s2));
            [a0, a1, a2, a3]
        };
        point_computing(&a, &p)
    } else {
        (p[1].x, p[1].y)
    };
    // extremity
    let (xend, yend) = if s2 > 0.0 {
        let a = if s1 < 0.0 {
            let ((a0, a2), (a1, a3)) = (negative_s1_influence(1.0, s1), positive_s2_influence(k, 1.0, s2));
            [a0, a1, a2, a3]
        } else {
            let ((a0, a2), (a1, a3)) = (positive_s1_influence(k, 1.0, s1), positive_s2_influence(k, 1.0, s2));
            [a0, a1, a2, a3]
        };
        point_computing(&a, &p)
    } else {
        (p[2].x, p[2].y)
    };
    // middle
    let ((a0, a2), (a1, a3)) = if s2 > 0.0 {
        if s1 < 0.0 {
            (negative_s1_influence(0.5, s1), positive_s2_influence(k, 0.5, s2))
        } else {
            (positive_s1_influence(k, 0.5, s1), positive_s2_influence(k, 0.5, s2))
        }
    } else if s1 < 0.0 {
        (negative_s1_influence(0.5, s1), negative_s2_influence(0.5, s2))
    } else {
        (positive_s1_influence(k, 0.5, s1), negative_s2_influence(0.5, s2))
    };
    let (xmid, ymid) = point_computing(&[a0, a1, a2, a3], &p);

    // fig2dev computes the angle in float
    let xv1 = (xstart as i64 - xmid as i64) as f32;
    let yv1 = (ystart as i64 - ymid as i64) as f32;
    let xv2 = (xend as i64 - xmid as i64) as f32;
    let yv2 = (yend as i64 - ymid as i64) as f32;
    let scal_prod = xv1 * xv2 + yv1 * yv2;
    let sides_length_prod = (((xv1 * xv1 + yv1 * yv1) * (xv2 * xv2 + yv2 * yv2)) as f64).sqrt() as f32;
    // the cosine of the origin-middle-extremity angle approximates the
    // curvature of the segment
    let angle_cos = if sides_length_prod == 0.0 {
        0.0
    } else {
        scal_prod / sides_length_prod
    };
    let xlength = (xend as i64 - xstart as i64) as f64;
    let ylength = (yend as i64 - ystart as i64) as f64;
    let start_to_end_dist = (xlength * xlength + ylength * ylength).sqrt() as i32;
    // more steps if the segment's ends are far apart
    let mut number_of_steps = ((start_to_end_dist as f64).sqrt() as i32) / 2;
    // more steps if the curve is high
    number_of_steps = number_of_steps.saturating_add(((1.0 + angle_cos as f64) * 10.0) as i32);
    let mut step = if number_of_steps == 0 || number_of_steps > 999 {
        1.0
    } else {
        precision / number_of_steps as f32
    };
    if step as f64 > MAX_SPLINE_STEP || step == 0.0 || !step.is_finite() {
        step = MAX_SPLINE_STEP as f32;
    }
    step
}

/// `SPLINE_SEGMENT_LOOP`: the points of one segment, `t` from 0 to 1
/// (exclusive).
fn spline_segment(arr: &mut PointArray, k: i32, seg: &Segment, precision: f32) {
    let step = step_computing(k, seg, precision) as f64;
    let mut t = 0.0;
    while t < 1.0 {
        let a = blend(k, t, seg.s1, seg.s2);
        let (x, y) = point_computing(&a, &seg.p);
        arr.add(x, y);
        if arr.overflow {
            return;
        }
        t += step;
    }
}

/// `create_line_with_spline`: the spline as a polyline (open) or polygon
/// (closed) with its attributes and comments. At an end with an arrowhead,
/// the three points before the end are dropped, so that the arrowhead
/// points along the curve. `None` if the spline can't be converted (too
/// few points; fig2dev's message goes to `ctx`).
pub fn create_line_with_spline(ctx: &mut Ctx, s: &Spline) -> Option<Line> {
    let mut points = if s.open_spline() {
        compute_open_spline(ctx, s, HIGH_PRECISION)?
    } else {
        compute_closed_spline(ctx, s, HIGH_PRECISION)?
    };
    let mut start = 0;
    let mut npoints = points.len();
    if s.for_arrow.is_some() && npoints > ARROW_START {
        let last = *points.get(npoints - 1)?;
        *points.get_mut(npoints - ARROW_START)? = last;
        npoints -= ARROW_START - 1;
    }
    if s.back_arrow.is_some() && npoints > ARROW_START {
        let first = *points.first()?;
        *points.get_mut(ARROW_START - 1)? = first;
        npoints -= ARROW_START - 1;
        start = ARROW_START - 1;
    }
    let points = points.get(start..start + npoints)?.to_vec();
    Some(Line {
        type_: if s.open_spline() { T_POLYLINE } else { T_POLYGON },
        style: s.style,
        thickness: s.thickness,
        pen_color: s.pen_color,
        fill_color: s.fill_color,
        depth: s.depth,
        pen: s.pen,
        fill_style: s.fill_style,
        style_val: s.style_val,
        for_arrow: s.for_arrow,
        back_arrow: s.back_arrow,
        cap_style: s.cap_style,
        // prevents spikes in tight splines
        join_style: 2,
        radius: 0,
        points,
        pic: None,
        comments: s.comments.clone(),
    })
}

/// The controls `make_control_factors` gives a spline: the shape factor
/// of an approximated (1) or interpolated (−1) spline at every point, the
/// ends of an open spline angular (0).
fn control_factors(s: &Spline) -> Vec<Control> {
    let type_s = if s.approx_spline() {
        S_SPLINE_APPROX
    } else {
        S_SPLINE_INTERP
    };
    let first = if s.closed_spline() { type_s } else { S_SPLINE_ANGULAR };
    let n = s.points.len();
    (0..n)
        .map(|i| Control {
            s: if i == 0 || i + 1 == n { first } else { type_s },
            ..Control::default()
        })
        .collect()
}

/// `make_control_factors`: give `s` one control per point with the shape
/// factor of its type (the reader then overwrites them with the file's).
pub fn make_control_factors(s: &mut Spline) {
    s.controls = control_factors(s);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::{Arrow, T_CLOSED_XSPLINE, T_OPEN_XSPLINE};

    fn spline(type_: i32, pts: &[(i32, i32)], s: &[f64]) -> Spline {
        Spline {
            type_,
            thickness: 15,
            depth: 50,
            points: pts.iter().map(|&(x, y)| Point::new(x, y)).collect(),
            controls: s
                .iter()
                .map(|&s| Control {
                    s,
                    ..Control::default()
                })
                .collect(),
            comments: vec!["c".into()],
            ..Spline::default()
        }
    }

    #[test]
    fn control_factors_by_type() {
        let mut s = spline(T_OPEN_XSPLINE, &[(0, 0), (1, 1), (2, 2), (3, 3)], &[]);
        s.type_ = 0; // open approximated
        make_control_factors(&mut s);
        let f: Vec<f64> = s.controls.iter().map(|c| c.s).collect();
        assert_eq!(f, vec![0.0, 1.0, 1.0, 0.0]);
        s.type_ = 3; // closed interpolated
        make_control_factors(&mut s);
        let f: Vec<f64> = s.controls.iter().map(|c| c.s).collect();
        assert_eq!(f, vec![-1.0; 4]);
        let mut e = Spline::default();
        make_control_factors(&mut e);
        assert!(e.controls.is_empty());
    }

    #[test]
    fn two_points_are_a_straight_line() {
        let mut ctx = Ctx::default();
        let s = spline(T_OPEN_XSPLINE, &[(0, 0), (1200, 600)], &[0.0, 0.0]);
        let l = create_line_with_spline(&mut ctx, &s).unwrap();
        assert_eq!(l.points, vec![Point::new(0, 0), Point::new(1200, 600)]);
        assert_eq!((l.type_, l.join_style, l.thickness, l.depth), (T_POLYLINE, 2, 15, 50));
        assert_eq!(l.comments, vec!["c".to_string()]);
    }

    #[test]
    fn angular_spline_passes_its_points() {
        let mut ctx = Ctx::default();
        // all shape factors 0: a polyline through the points
        let s = spline(T_OPEN_XSPLINE, &[(0, 0), (1200, 0), (1200, 1200)], &[0.0, 0.0, 0.0]);
        let pts = compute_open_spline(&mut ctx, &s, HIGH_PRECISION).unwrap();
        assert_eq!(pts.first(), Some(&Point::new(0, 0)));
        assert_eq!(pts.last(), Some(&Point::new(1200, 1200)));
        assert!(pts.contains(&Point::new(1200, 0)));
        // all on the two straight segments
        assert!(pts.iter().all(|q| q.y == 0 || q.x == 1200));
        assert!(pts.len() > 20);
        assert!(ctx.messages.is_empty());
    }

    #[test]
    fn approximated_spline_point_counts() {
        let mut ctx = Ctx::default();
        let s = spline(
            T_OPEN_XSPLINE,
            &[(1200, 1200), (2400, 600), (3600, 1800), (4800, 1200)],
            &[0.0, 1.0, 1.0, 0.0],
        );
        let pts = compute_open_spline(&mut ctx, &s, HIGH_PRECISION).unwrap();
        assert_eq!(pts.first(), Some(&Point::new(1200, 1200)));
        assert_eq!(pts.last(), Some(&Point::new(4800, 1200)));
        // the curve stays inside the control polygon's hull
        assert!(pts.iter().all(|q| q.y > 600 && q.y < 1800));
        assert!(pts.len() > 10);
        let l = create_line_with_spline(&mut ctx, &s).unwrap();
        assert_eq!(l.points, pts);
    }

    #[test]
    fn closed_spline_ends_where_it_starts() {
        let mut ctx = Ctx::default();
        let s = spline(
            T_CLOSED_XSPLINE,
            &[(1200, 3000), (2400, 2400), (3600, 3600), (2400, 4200)],
            &[-1.0; 4],
        );
        let l = create_line_with_spline(&mut ctx, &s).unwrap();
        assert_eq!(l.type_, T_POLYGON);
        assert_eq!(l.points.first(), l.points.last());
        // interpolating: passes the control points
        assert!(l.points.contains(&Point::new(2400, 2400)));
        let two = spline(T_CLOSED_XSPLINE, &[(0, 0), (10, 10)], &[-1.0; 2]);
        assert!(create_line_with_spline(&mut ctx, &two).is_none());
        assert_eq!(ctx.messages.len(), 1);
    }

    #[test]
    fn arrows_drop_three_points_per_end() {
        let mut ctx = Ctx::default();
        let mut s = spline(T_OPEN_XSPLINE, &[(0, 0), (1200, 1200), (2400, 0)], &[0.0, 1.0, 0.0]);
        let plain = compute_open_spline(&mut ctx, &s, HIGH_PRECISION).unwrap();
        let a = Arrow {
            type_: 1,
            style: 1,
            thickness: 15.0,
            wid: 60.0,
            ht: 120.0,
        };
        s.for_arrow = Some(a);
        s.back_arrow = Some(a);
        let l = create_line_with_spline(&mut ctx, &s).unwrap();
        assert_eq!(l.points.len(), plain.len() - 6);
        assert_eq!(l.points.first(), plain.first());
        assert_eq!(l.points.last(), plain.last());
        assert_eq!(l.for_arrow, Some(a));
    }

    #[test]
    fn hostile_splines_dont_panic() {
        let mut ctx = Ctx::default();
        let pts: Vec<(i32, i32)> = (0..200).map(|i| (i * 1_000_000 % i32::MAX, -i * 7_000_000)).collect();
        let s = spline(T_OPEN_XSPLINE, &pts, &[f64::NAN; 200]);
        let _ = create_line_with_spline(&mut ctx, &s);
        let s = spline(
            T_CLOSED_XSPLINE,
            &[(i32::MAX, i32::MIN), (0, 0), (i32::MIN, i32::MAX)],
            &[1e300, -1e300, 0.5],
        );
        let _ = create_line_with_spline(&mut ctx, &s);
        // mismatched controls: the type's factors
        let s = spline(T_OPEN_XSPLINE, &[(0, 0), (100, 100), (200, 0)], &[]);
        assert!(create_line_with_spline(&mut ctx, &s).is_some());
        let one = spline(T_OPEN_XSPLINE, &[(0, 0)], &[0.0]);
        assert!(create_line_with_spline(&mut ctx, &one).is_none());
        // too many points: capped, one message
        let many: Vec<(i32, i32)> = (0..6000).map(|i| (i * 1000, (i % 2) * 100_000)).collect();
        let s = spline(T_OPEN_XSPLINE, &many, &vec![1.0; 6000]);
        let mut ctx = Ctx::default();
        let l = create_line_with_spline(&mut ctx, &s).unwrap();
        assert!(l.points.len() <= MAXNUMPTS);
        assert_eq!(ctx.messages.len(), 1);
    }
}
