//! xfig's object snaps (the Snap menu). Ported from xfig's `w_snap.c` and
//! `w_intersect.c` (xfig licence, see LICENSE-xfig): the next clicked point
//! moves to a feature of the object under it — an endpoint, a midpoint, the
//! nearest point, a focus, the point diametrically opposite the current
//! point, the foot of the normal or the tangent point from the current
//! point, or an intersection with a second object.
//!
//! Where xfig solves ellipses analytically (normals, tangents, intersections
//! with a quartic solver) RustFig works on the flattened outline, which also
//! covers splines (xfig: "not yet implemented") and every pair of objects.

use rustfig_geom::Point;
use rustfig_model::{flatten_object, Object, ObjectKind, PolyKind};

/// The snap modes, in the Snap menu's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectSnap {
    Endpoint,
    Midpoint,
    Nearest,
    Focus,
    Diameter,
    Normal,
    Tangent,
    Intersect,
}

impl ObjectSnap {
    pub const ALL: [ObjectSnap; 8] = [
        ObjectSnap::Endpoint,
        ObjectSnap::Midpoint,
        ObjectSnap::Nearest,
        ObjectSnap::Focus,
        ObjectSnap::Diameter,
        ObjectSnap::Normal,
        ObjectSnap::Tangent,
        ObjectSnap::Intersect,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ObjectSnap::Endpoint => "endpoint",
            ObjectSnap::Midpoint => "midpoint",
            ObjectSnap::Nearest => "nearest",
            ObjectSnap::Focus => "focus",
            ObjectSnap::Diameter => "diameter",
            ObjectSnap::Normal => "normal",
            ObjectSnap::Tangent => "tangent",
            ObjectSnap::Intersect => "intersect",
        }
    }

    pub fn from_name(s: &str) -> Option<ObjectSnap> {
        ObjectSnap::ALL.into_iter().find(|m| m.name() == s)
    }

    /// The Snap Mode indicator's text (xfig's `snap_indicator_label`).
    pub fn label(self) -> &'static str {
        match self {
            ObjectSnap::Endpoint => "Endpoint",
            ObjectSnap::Midpoint => "Midpoint",
            ObjectSnap::Nearest => "Nearest",
            ObjectSnap::Focus => "Focus",
            ObjectSnap::Diameter => "Diameter",
            ObjectSnap::Normal => "Normal",
            ObjectSnap::Tangent => "Tangent",
            ObjectSnap::Intersect => "Intersect",
        }
    }

    /// Whether the mode measures from the current point.
    pub fn needs_current_point(self) -> bool {
        matches!(self, ObjectSnap::Diameter | ObjectSnap::Normal | ObjectSnap::Tangent)
    }
}

/// A snap that found nothing, with xfig's message.
pub type SnapResult = Result<Point, &'static str>;

/// The polyline a snap works on: an object's vertices (with the closing
/// segment for closed shapes, as xfig's lists repeat the first point), or a
/// text's bounding box (`build_text_bounding_box`).
fn vertex_path(o: &Object) -> Option<(Vec<Point>, bool)> {
    match &o.kind {
        ObjectKind::Polyline(p) => Some((p.points.clone(), p.kind != PolyKind::Polyline)),
        ObjectKind::Text(t) => Some((t.corners().to_vec(), true)),
        _ => None,
    }
}

fn nearest_of(points: impl IntoIterator<Item = Point>, click: Point) -> Option<Point> {
    points
        .into_iter()
        .filter(|p| p.is_finite())
        .min_by(|a, b| a.dist(click).total_cmp(&b.dist(click)))
}

fn segments(pts: &[Point], closed: bool) -> Vec<(Point, Point)> {
    let mut s: Vec<(Point, Point)> = pts.windows(2).filter_map(|w| Some((*w.first()?, *w.get(1)?))).collect();
    if closed && pts.len() > 2 {
        if let (Some(a), Some(b)) = (pts.last(), pts.first()) {
            s.push((*a, *b));
        }
    }
    s
}

fn dist_to_line(p: Point, a: Point, b: Point) -> f64 {
    let d = b - a;
    let l = d.len();
    if l < 1e-12 {
        return p.dist(a);
    }
    ((p.x - a.x) * d.y - (p.y - a.y) * d.x).abs() / l
}

/// Foot of the perpendicular from `from` onto the line through `a` and `b`
/// (xfig's `do_snap_polyline_normal`: the extended segment).
fn foot(from: Point, a: Point, b: Point) -> Point {
    let d = b - a;
    let l2 = d.x * d.x + d.y * d.y;
    if l2 < 1e-12 {
        return a;
    }
    let t = ((from.x - a.x) * d.x + (from.y - a.y) * d.y) / l2;
    a + d * t
}

/// xfig's polyline normal: the segment (extended) nearest the click, and
/// the foot of the perpendicular from `from` on it.
fn polyline_normal(pts: &[Point], closed: bool, click: Point, from: Point) -> SnapResult {
    let seg = segments(pts, closed)
        .into_iter()
        .min_by(|(a1, b1), (a2, b2)| dist_to_line(click, *a1, *b1).total_cmp(&dist_to_line(click, *a2, *b2)));
    let (a, b) = seg.ok_or("No point found.")?;
    Ok(foot(from, a, b))
}

fn centroid(pts: &[Point]) -> Option<Point> {
    if pts.is_empty() {
        return None;
    }
    let n = pts.len() as f64;
    let s = pts.iter().fold(Point::ZERO, |s, p| s + *p);
    Some(s * (1.0 / n))
}

/// Points of the object's flattened outline(s), as (segment list).
fn outline(o: &Object) -> Vec<(Point, Point)> {
    if let ObjectKind::Text(t) = &o.kind {
        return segments(&t.corners(), true);
    }
    flatten_object(o)
        .iter()
        .flat_map(|fp| segments(&fp.pts, fp.closed))
        .collect()
}

fn nearest_on_segment(p: Point, a: Point, b: Point) -> Point {
    let d = b - a;
    let l2 = d.x * d.x + d.y * d.y;
    if l2 < 1e-12 {
        return a;
    }
    let t = (((p.x - a.x) * d.x + (p.y - a.y) * d.y) / l2).clamp(0.0, 1.0);
    a + d * t
}

/// The nearest point on a flattened outline.
fn outline_nearest(o: &Object, click: Point) -> Option<Point> {
    nearest_of(
        outline(o).into_iter().map(|(a, b)| nearest_on_segment(click, a, b)),
        click,
    )
}

/// Points on the outline where the segment from `from` meets the outline at
/// a right angle (feet of perpendiculars that fall inside a segment).
fn outline_normals(o: &Object, from: Point) -> Vec<Point> {
    outline(o)
        .into_iter()
        .filter_map(|(a, b)| {
            let d = b - a;
            let l2 = d.x * d.x + d.y * d.y;
            if l2 < 1e-12 {
                return None;
            }
            let t = ((from.x - a.x) * d.x + (from.y - a.y) * d.y) / l2;
            (-1e-9..=1.0 + 1e-9).contains(&t).then(|| a + d * t)
        })
        .collect()
}

/// Tangent points from `from` to a flattened outline: vertices where the
/// line from `from` stops crossing the outline (the cross product of the
/// sight line with the outline direction changes sign).
fn outline_tangents(o: &Object, from: Point) -> Vec<Point> {
    let cross = |p: Point, dir: Point| (p.x - from.x) * dir.y - (p.y - from.y) * dir.x;
    let mut out = Vec::new();
    for fp in flatten_object(o) {
        let pts = &fp.pts;
        let n = pts.len();
        if n < 3 {
            continue;
        }
        let at = |i: usize| pts.get(i % n).copied().unwrap_or(Point::ZERO);
        let last = if fp.closed { n } else { n - 1 };
        let mut prev: Option<(f64, Point)> = None;
        for i in 0..last {
            let (a, b) = (at(i), at(i + 1));
            let c = cross(a, b - a);
            if let Some((pc, pp)) = prev {
                if pc * c < 0.0 {
                    out.push(pp.lerp(a, pc.abs() / (pc.abs() + c.abs())));
                }
            }
            prev = Some((c, a));
        }
    }
    out
}

fn circle_normal(center: Point, r: f64, click: Point, from: Point) -> Point {
    // From the centre itself any radius is normal: use the click's.
    let dir = if (from - center).len() < 1e-9 {
        center - click
    } else {
        center - from
    };
    let a = dir.y.atan2(dir.x);
    let pa = Point::new(center.x - r * a.cos(), center.y - r * a.sin());
    let pb = Point::new(center.x + r * a.cos(), center.y + r * a.sin());
    if pa.dist(click) < pb.dist(click) {
        pa
    } else {
        pb
    }
}

fn circle_tangent(center: Point, r: f64, click: Point, from: Point) -> SnapResult {
    let p = from.dist(center);
    if p <= r {
        return Err("No tangent can be drawn from the current point.");
    }
    let a = (from.y - center.y).atan2(from.x - center.x);
    let b = (r / p).acos();
    let at = |ang: f64| Point::new(center.x + r * ang.cos(), center.y + r * ang.sin());
    let (ta, tb) = (at(a + b), at(a - b));
    Ok(if ta.dist(click) < tb.dist(click) { ta } else { tb })
}

/// xfig's `is_point_on_arc`: on the same side of the end-point chord as the
/// arc's middle point.
fn on_arc(points: &[Point; 3], p: Point) -> bool {
    let [p0, p1, p2] = *points;
    let (dx, dy) = (p2.x - p0.x, p2.y - p0.y);
    let d1 = dx * (p0.y - p1.y) - dy * (p0.x - p1.x);
    let dt = dx * (p0.y - p.y) - dy * (p0.x - p.x);
    d1.is_sign_negative() == dt.is_sign_negative()
}

/// xfig rounds snapped points to whole Fig units (`rint`).
fn round(r: SnapResult) -> SnapResult {
    r.map(|p| Point::new(p.x.round(), p.y.round()))
}

/// Snap `click` to a feature of `o`. `current` is the point being drawn
/// from (Diameter, Normal, Tangent need it).
pub fn snap(mode: ObjectSnap, o: &Object, click: Point, current: Option<Point>) -> SnapResult {
    round(snap_exact(mode, o, click, current))
}

fn snap_exact(mode: ObjectSnap, o: &Object, click: Point, current: Option<Point>) -> SnapResult {
    let cur = || current.ok_or("No current point: start drawing first.");
    match &o.kind {
        ObjectKind::Polyline(_) | ObjectKind::Text(_) => {
            let (pts, closed) = vertex_path(o).ok_or("No point found.")?;
            match mode {
                ObjectSnap::Endpoint => nearest_of(pts.iter().copied(), click).ok_or("No point found."),
                ObjectSnap::Midpoint => {
                    nearest_of(segments(&pts, closed).into_iter().map(|(a, b)| a.lerp(b, 0.5)), click)
                        .ok_or("No point found.")
                }
                ObjectSnap::Tangent => Err("Polylines have no tangents."),
                ObjectSnap::Normal => polyline_normal(&pts, closed, click, cur()?),
                ObjectSnap::Focus => centroid(&pts).ok_or("No point found."),
                ObjectSnap::Diameter => {
                    let c = centroid(&pts).ok_or("No point found.")?;
                    Ok(c + (c - cur()?))
                }
                ObjectSnap::Nearest => polyline_normal(&pts, closed, click, click),
                ObjectSnap::Intersect => Err("Select two objects."),
            }
        }
        ObjectKind::Ellipse(e) => {
            let circle = e.kind.is_circle();
            let (rx, ry) = (e.radii.x.abs(), e.radii.y.abs());
            // Fig's y axis points down, angles count counter-clockwise.
            let (s, c) = (-e.angle).sin_cos();
            let axis = |x: f64, y: f64| e.center + Point::new(x * c - y * s, x * s + y * c);
            match mode {
                ObjectSnap::Endpoint if circle => Err("Circles have no endpoints."),
                ObjectSnap::Endpoint => {
                    nearest_of([axis(rx, 0.0), axis(-rx, 0.0), axis(0.0, ry), axis(0.0, -ry)], click)
                        .ok_or("No point found.")
                }
                ObjectSnap::Midpoint => Ok(e.center),
                ObjectSnap::Focus if circle => Ok(e.center),
                ObjectSnap::Focus => {
                    let f = (rx * rx - ry * ry).abs().sqrt();
                    let foci = if rx >= ry {
                        [axis(f, 0.0), axis(-f, 0.0)]
                    } else {
                        [axis(0.0, f), axis(0.0, -f)]
                    };
                    nearest_of(foci, click).ok_or("No point found.")
                }
                ObjectSnap::Diameter => Ok(e.center + (e.center - cur()?)),
                ObjectSnap::Normal if circle => Ok(circle_normal(e.center, rx, click, cur()?)),
                ObjectSnap::Nearest if circle => Ok(circle_normal(e.center, rx, click, click)),
                ObjectSnap::Tangent if circle => circle_tangent(e.center, rx, click, cur()?),
                ObjectSnap::Normal => nearest_of(outline_normals(o, cur()?), click).ok_or("No point found."),
                ObjectSnap::Nearest => outline_nearest(o, click).ok_or("No point found."),
                ObjectSnap::Tangent => nearest_of(outline_tangents(o, cur()?), click)
                    .ok_or("No tangent can be drawn from the current point."),
                ObjectSnap::Intersect => Err("Select two objects."),
            }
        }
        ObjectKind::Arc(a) => {
            let r = a.center.dist(a.points[1]);
            match mode {
                ObjectSnap::Endpoint => nearest_of(a.points, click).ok_or("No point found."),
                // xfig: the average of the three arc points.
                ObjectSnap::Midpoint => Ok((a.points[0] + a.points[1] + a.points[2]) * (1.0 / 3.0)),
                ObjectSnap::Focus => Ok(a.center),
                ObjectSnap::Diameter => Ok(a.center + (a.center - cur()?)),
                ObjectSnap::Tangent => {
                    let p = circle_tangent(a.center, r, click, cur()?)?;
                    if on_arc(&a.points, p) {
                        Ok(p)
                    } else {
                        Err("The tangent point found is not on the arc.")
                    }
                }
                ObjectSnap::Normal | ObjectSnap::Nearest => {
                    let from = if mode == ObjectSnap::Normal { cur()? } else { click };
                    let p = circle_normal(a.center, r, click, from);
                    if on_arc(&a.points, p) {
                        Ok(p)
                    } else if mode == ObjectSnap::Normal {
                        Err("The normal point found is not on the arc.")
                    } else {
                        Err("The closest point found is not on the arc.")
                    }
                }
                ObjectSnap::Intersect => Err("Select two objects."),
            }
        }
        ObjectKind::Spline(s) => match mode {
            // xfig snaps to the centroid of the control points.
            ObjectSnap::Focus => centroid(&s.points).ok_or("No point found."),
            // xfig leaves the rest of the spline snaps unimplemented; the
            // flattened curve gives them.
            ObjectSnap::Endpoint if s.closed => Err("Closed splines have no endpoints."),
            ObjectSnap::Endpoint => nearest_of(
                [s.points.first().copied(), s.points.last().copied()]
                    .into_iter()
                    .flatten(),
                click,
            )
            .ok_or("No point found."),
            ObjectSnap::Nearest => outline_nearest(o, click).ok_or("No point found."),
            ObjectSnap::Normal => nearest_of(outline_normals(o, cur()?), click).ok_or("No point found."),
            ObjectSnap::Tangent => {
                nearest_of(outline_tangents(o, cur()?), click).ok_or("No tangent can be drawn from the current point.")
            }
            ObjectSnap::Midpoint => Err("Spline midpoints are not defined."),
            ObjectSnap::Diameter => Err("Splines have no centre."),
            ObjectSnap::Intersect => Err("Select two objects."),
        },
        ObjectKind::Compound(_) => Err("No point found."),
    }
}

fn segment_intersection(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    let r = b - a;
    let s = d - c;
    let den = r.x * s.y - r.y * s.x;
    if den.abs() < 1e-12 {
        return None;
    }
    let t = ((c.x - a.x) * s.y - (c.y - a.y) * s.x) / den;
    let u = ((c.x - a.x) * r.y - (c.y - a.y) * r.x) / den;
    let eps = 1e-9;
    ((-eps..=1.0 + eps).contains(&t) && (-eps..=1.0 + eps).contains(&u)).then(|| a + r * t)
}

/// Cap on outline segments compared for an intersection (both objects).
const MAX_INTERSECT_SEGMENTS: usize = 20_000;

/// The intersection of two objects nearest the click (xfig's
/// `snap_intersect_handler`), on their flattened outlines.
pub fn intersect(a: &Object, b: &Object, click: Point) -> SnapResult {
    let (sa, sb) = (outline(a), outline(b));
    if sa.len() > MAX_INTERSECT_SEGMENTS || sb.len() > MAX_INTERSECT_SEGMENTS {
        return Err("The objects are too complex to intersect.");
    }
    let hits = sa
        .iter()
        .flat_map(|(p, q)| sb.iter().filter_map(move |(r, s)| segment_intersection(*p, *q, *r, *s)));
    round(nearest_of(hits, click).ok_or("The objects don't intersect."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustfig_model::{Arc, EllipseKind, Polyline, Style};

    fn poly(pts: &[(f64, f64)]) -> Object {
        Object::new(ObjectKind::Polyline(Polyline::new(
            PolyKind::Polyline,
            Style::default(),
            pts.iter().map(|(x, y)| Point::new(*x, *y)).collect(),
        )))
    }

    fn close(a: Point, b: Point) -> bool {
        a.dist(b) < 1.0
    }

    #[test]
    fn polyline_snaps() {
        let l = poly(&[(0.0, 0.0), (1200.0, 0.0), (1200.0, 1200.0)]);
        let click = Point::new(1100.0, 50.0);
        assert_eq!(snap(ObjectSnap::Endpoint, &l, click, None), Ok(Point::new(1200.0, 0.0)));
        assert_eq!(snap(ObjectSnap::Midpoint, &l, click, None), Ok(Point::new(600.0, 0.0)));
        assert_eq!(snap(ObjectSnap::Focus, &l, click, None), Ok(Point::new(800.0, 400.0)));
        assert_eq!(snap(ObjectSnap::Nearest, &l, click, None), Ok(Point::new(1100.0, 0.0)));
        let from = Point::new(300.0, 900.0);
        assert_eq!(
            snap(ObjectSnap::Normal, &l, click, Some(from)),
            Ok(Point::new(300.0, 0.0))
        );
        assert_eq!(
            snap(ObjectSnap::Diameter, &l, click, Some(from)),
            Ok(Point::new(1300.0, -100.0))
        );
        assert!(snap(ObjectSnap::Tangent, &l, click, Some(from)).is_err());
        assert!(snap(ObjectSnap::Normal, &l, click, None).is_err());
    }

    #[test]
    fn circle_and_ellipse_snaps() {
        let circle = crate::cmd::shape::make_ellipse(
            Style::default(),
            Point::ZERO,
            600.0,
            600.0,
            0.0,
            EllipseKind::CircleRadius,
        );
        let from = Point::new(1200.0, 0.0);
        let t = snap(ObjectSnap::Tangent, &circle, Point::new(300.0, -600.0), Some(from)).unwrap();
        // The tangent point is on the circle and the radius there is
        // perpendicular to the line from the current point.
        // (Rounded to whole Fig units, as xfig does.)
        assert!((t.len() - 600.0).abs() < 1.0);
        let cos = (t.x * (t.x - from.x) + t.y * (t.y - from.y)) / (t.len() * (t - from).len());
        assert!(cos.abs() < 1e-3, "{cos}");
        assert!(t.y < 0.0, "the tangent nearer the click");
        assert!(snap(ObjectSnap::Endpoint, &circle, from, None).is_err());
        assert_eq!(
            snap(ObjectSnap::Nearest, &circle, Point::new(900.0, 0.0), None),
            Ok(Point::new(600.0, 0.0))
        );
        let ellipse = crate::cmd::shape::make_ellipse(
            Style::default(),
            Point::ZERO,
            500.0,
            300.0,
            0.0,
            EllipseKind::EllipseRadii,
        );
        assert_eq!(
            snap(ObjectSnap::Focus, &ellipse, Point::new(500.0, 0.0), None),
            Ok(Point::new(400.0, 0.0))
        );
        assert!(close(
            snap(ObjectSnap::Endpoint, &ellipse, Point::new(10.0, 290.0), None).unwrap(),
            Point::new(0.0, 300.0)
        ));
        let n = snap(
            ObjectSnap::Normal,
            &ellipse,
            Point::new(0.0, 280.0),
            Some(Point::new(0.0, 1000.0)),
        )
        .unwrap();
        // On the flattened outline: within a couple of Fig units.
        assert!(n.dist(Point::new(0.0, 300.0)) <= 2.0, "{n:?}");
    }

    #[test]
    fn arc_snaps_check_the_arc() {
        let a = Arc::through(
            Style::default(),
            Point::new(-600.0, 0.0),
            Point::new(0.0, -600.0),
            Point::new(600.0, 0.0),
            false,
        )
        .unwrap();
        let o = Object::new(ObjectKind::Arc(a));
        assert!(close(
            snap(ObjectSnap::Focus, &o, Point::ZERO, None).unwrap(),
            Point::ZERO
        ));
        assert!(close(
            snap(ObjectSnap::Nearest, &o, Point::new(0.0, -700.0), None).unwrap(),
            Point::new(0.0, -600.0)
        ));
        // The lower half of the circle isn't part of the arc.
        assert!(snap(ObjectSnap::Nearest, &o, Point::new(0.0, 700.0), None).is_err());
    }

    #[test]
    fn intersections() {
        let a = poly(&[(0.0, 0.0), (1200.0, 1200.0)]);
        let b = poly(&[(0.0, 1200.0), (1200.0, 0.0)]);
        assert!(close(
            intersect(&a, &b, Point::new(500.0, 500.0)).unwrap(),
            Point::new(600.0, 600.0)
        ));
        let c = poly(&[(0.0, 2000.0), (1200.0, 2000.0)]);
        assert!(intersect(&a, &c, Point::ZERO).is_err());
        // Line through a circle: two hits, the nearer one wins.
        let circle = crate::cmd::shape::make_ellipse(
            Style::default(),
            Point::ZERO,
            600.0,
            600.0,
            0.0,
            EllipseKind::CircleRadius,
        );
        let h = poly(&[(-1200.0, 0.0), (1200.0, 0.0)]);
        let p = intersect(&circle, &h, Point::new(500.0, 50.0)).unwrap();
        assert!((p.x - 600.0).abs() < 5.0 && p.y.abs() < 1e-6, "{p:?}");
    }
}
