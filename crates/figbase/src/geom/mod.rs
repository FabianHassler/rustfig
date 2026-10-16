//! Fig geometry: points, rectangles, affine transforms, curve flattening
//! (X-splines, arcs, ellipses, rounded boxes), arrowhead construction and
//! line widths. Coordinates are Fig units (1200 per inch), y grows downwards.

mod arc;
mod arrow;
mod curves;
mod xspline;

pub use arc::{arc_from_three_points, flatten_arc, flatten_ellipse, ArcGeom};
pub use arrow::{arrowhead, ArrowFill, ArrowShape, ArrowSpec, ARROW_TYPES};
pub use curves::{arc_path, ellipse_path, round_rect_path, Seg};
pub use xspline::{flatten_xspline, XSPLINE_MAX_POINTS};

use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Neg, Sub};

/// Fig units per inch.
pub const FIG_PER_INCH: f64 = 1200.0;
/// Fig units per centimetre. xfig defines 1 cm as 450 units to keep a regular grid.
pub const FIG_PER_CM: f64 = 450.0;
/// Fig units per point (1/72 inch).
pub const FIG_PER_POINT: f64 = FIG_PER_INCH / 72.0;
/// Fig units per line-thickness unit (1/80 inch).
pub const FIG_PER_THICKNESS: f64 = FIG_PER_INCH / 80.0;

/// Drawn stroke width in Fig units for a line thickness in 1/80 inch:
/// thickness 1 is drawn 1/160 inch wide, thicker lines one unit thinner than
/// their nominal width, (t − 1)/80 inch.
// Ported from fig2dev dev/genps.c and dev/gensvg.c `linewidth_adj` (xfig licence, see LICENSE-xfig).
pub fn stroke_width(thickness: f64) -> f64 {
    if !thickness.is_finite() || thickness <= 0.0 {
        0.0
    } else if thickness <= 1.0 {
        thickness * FIG_PER_THICKNESS / 2.0
    } else {
        (thickness - 1.0) * FIG_PER_THICKNESS
    }
}
/// Fig units per style unit (dash length, arc-box radius: 1/80 inch).
pub const FIG_PER_STYLE_UNIT: f64 = FIG_PER_INCH / 80.0;

/// A point (or vector) in Fig units.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ZERO: Point = Point { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    pub fn len(self) -> f64 {
        self.x.hypot(self.y)
    }

    pub fn dist(self, o: Point) -> f64 {
        (self - o).len()
    }

    pub fn dot(self, o: Point) -> f64 {
        self.x * o.x + self.y * o.y
    }

    pub fn cross(self, o: Point) -> f64 {
        self.x * o.y - self.y * o.x
    }

    /// Unit vector in the same direction, or zero for a zero vector.
    pub fn normalized(self) -> Point {
        let l = self.len();
        if l > 1e-12 && l.is_finite() {
            Point::new(self.x / l, self.y / l)
        } else {
            Point::ZERO
        }
    }

    /// Perpendicular (rotated 90° counter-clockwise on screen: (x,y) -> (y,-x)).
    pub fn perp(self) -> Point {
        Point::new(self.y, -self.x)
    }

    pub fn lerp(self, o: Point, t: f64) -> Point {
        Point::new(self.x + (o.x - self.x) * t, self.y + (o.y - self.y) * t)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Rounded to whole Fig units, as stored in .fig files.
    pub fn rounded(self) -> Point {
        Point::new(self.x.round(), self.y.round())
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, o: Point) -> Point {
        Point::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Point {
    type Output = Point;
    fn sub(self, o: Point) -> Point {
        Point::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f64> for Point {
    type Output = Point;
    fn mul(self, s: f64) -> Point {
        Point::new(self.x * s, self.y * s)
    }
}

impl Neg for Point {
    type Output = Point;
    fn neg(self) -> Point {
        Point::new(-self.x, -self.y)
    }
}

/// Axis-aligned rectangle. An empty rectangle has `min > max`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Default for Rect {
    fn default() -> Self {
        Rect::EMPTY
    }
}

impl Rect {
    pub const EMPTY: Rect = Rect {
        min: Point::new(f64::INFINITY, f64::INFINITY),
        max: Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    };

    pub fn new(a: Point, b: Point) -> Rect {
        Rect {
            min: Point::new(a.x.min(b.x), a.y.min(b.y)),
            max: Point::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    pub fn from_points<I: IntoIterator<Item = Point>>(pts: I) -> Rect {
        let mut r = Rect::EMPTY;
        for p in pts {
            r.add_point(p);
        }
        r
    }

    pub fn is_empty(&self) -> bool {
        !(self.min.x <= self.max.x && self.min.y <= self.max.y)
    }

    pub fn add_point(&mut self, p: Point) {
        if !p.is_finite() {
            return;
        }
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
    }

    pub fn union(&self, o: &Rect) -> Rect {
        if self.is_empty() {
            return *o;
        }
        if o.is_empty() {
            return *self;
        }
        Rect {
            min: Point::new(self.min.x.min(o.min.x), self.min.y.min(o.min.y)),
            max: Point::new(self.max.x.max(o.max.x), self.max.y.max(o.max.y)),
        }
    }

    pub fn expand(&self, d: f64) -> Rect {
        if self.is_empty() {
            return *self;
        }
        Rect {
            min: Point::new(self.min.x - d, self.min.y - d),
            max: Point::new(self.max.x + d, self.max.y + d),
        }
    }

    /// Overlap of two rectangles (empty if they do not intersect).
    pub fn intersection_or_empty(&self, o: &Rect) -> Rect {
        let r = Rect {
            min: Point::new(self.min.x.max(o.min.x), self.min.y.max(o.min.y)),
            max: Point::new(self.max.x.min(o.max.x), self.max.y.min(o.max.y)),
        };
        if r.is_empty() {
            Rect::EMPTY
        } else {
            r
        }
    }

    pub fn width(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            self.max.x - self.min.x
        }
    }

    pub fn height(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            self.max.y - self.min.y
        }
    }

    pub fn center(&self) -> Point {
        self.min.lerp(self.max, 0.5)
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    pub fn contains_rect(&self, o: &Rect) -> bool {
        !o.is_empty() && self.contains(o.min) && self.contains(o.max)
    }

    pub fn intersects(&self, o: &Rect) -> bool {
        !self.is_empty()
            && !o.is_empty()
            && self.min.x <= o.max.x
            && o.min.x <= self.max.x
            && self.min.y <= o.max.y
            && o.min.y <= self.max.y
    }

    pub fn corners(&self) -> [Point; 4] {
        [
            self.min,
            Point::new(self.max.x, self.min.y),
            self.max,
            Point::new(self.min.x, self.max.y),
        ]
    }
}

/// 2D affine transform `[a c e; b d f]`, mapping (x,y) to (a x + c y + e, b x + d y + f).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Default for Affine {
    fn default() -> Self {
        Affine::IDENTITY
    }
}

impl Affine {
    pub const IDENTITY: Affine = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    pub fn translate(dx: f64, dy: f64) -> Affine {
        Affine {
            e: dx,
            f: dy,
            ..Affine::IDENTITY
        }
    }

    pub fn scale(sx: f64, sy: f64) -> Affine {
        Affine {
            a: sx,
            d: sy,
            ..Affine::IDENTITY
        }
    }

    /// Rotation by `angle` radians. Positive angles turn counter-clockwise on
    /// screen (y down), matching xfig's convention for text and ellipse angles.
    pub fn rotate(angle: f64) -> Affine {
        let (s, c) = angle.sin_cos();
        Affine {
            a: c,
            b: -s,
            c: s,
            d: c,
            e: 0.0,
            f: 0.0,
        }
    }

    /// `self` applied after `first`.
    pub fn then(&self, next: &Affine) -> Affine {
        // next ∘ self
        Affine {
            a: next.a * self.a + next.c * self.b,
            b: next.b * self.a + next.d * self.b,
            c: next.a * self.c + next.c * self.d,
            d: next.b * self.c + next.d * self.d,
            e: next.a * self.e + next.c * self.f + next.e,
            f: next.b * self.e + next.d * self.f + next.f,
        }
    }

    /// Transform `t` applied about the point `p`.
    pub fn about(t: Affine, p: Point) -> Affine {
        Affine::translate(-p.x, -p.y)
            .then(&t)
            .then(&Affine::translate(p.x, p.y))
    }

    pub fn apply(&self, p: Point) -> Point {
        Point::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }

    /// Apply to a vector (no translation).
    pub fn apply_vec(&self, p: Point) -> Point {
        Point::new(self.a * p.x + self.c * p.y, self.b * p.x + self.d * p.y)
    }

    pub fn determinant(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    pub fn invert(&self) -> Option<Affine> {
        let det = self.determinant();
        if det.abs() < 1e-300 || !det.is_finite() {
            return None;
        }
        let ia = self.d / det;
        let ib = -self.b / det;
        let ic = -self.c / det;
        let id = self.a / det;
        Some(Affine {
            a: ia,
            b: ib,
            c: ic,
            d: id,
            e: -(ia * self.e + ic * self.f),
            f: -(ib * self.e + id * self.f),
        })
    }

    /// Mean linear scale factor (for stroke widths, radii).
    pub fn mean_scale(&self) -> f64 {
        self.determinant().abs().sqrt()
    }

    /// True if the transform mirrors (negative determinant).
    pub fn is_mirroring(&self) -> bool {
        self.determinant() < 0.0
    }
}

/// Points of a rounded rectangle (arc-box) outline, closed implicitly.
pub fn flatten_round_rect(r: &Rect, radius: f64) -> Vec<Point> {
    if r.is_empty() {
        return Vec::new();
    }
    let rad = radius.max(0.0).min(r.width() / 2.0).min(r.height() / 2.0);
    if rad <= 0.0 || !rad.is_finite() {
        return r.corners().to_vec();
    }
    let steps = 8;
    let mut out = Vec::with_capacity(4 * (steps + 1));
    let corners = [
        (Point::new(r.max.x - rad, r.min.y + rad), -90.0_f64),
        (Point::new(r.max.x - rad, r.max.y - rad), 0.0),
        (Point::new(r.min.x + rad, r.max.y - rad), 90.0),
        (Point::new(r.min.x + rad, r.min.y + rad), 180.0),
    ];
    for (c, start) in corners {
        for i in 0..=steps {
            let a = (start + 90.0 * i as f64 / steps as f64).to_radians();
            out.push(Point::new(c.x + rad * a.cos(), c.y + rad * a.sin()));
        }
    }
    out
}

/// Vertices of a regular polygon with `sides` sides, centred at `c`, with the
/// first vertex at `first`.
pub fn regular_polygon(c: Point, first: Point, sides: usize) -> Vec<Point> {
    let sides = sides.clamp(3, 1000);
    let r = c.dist(first);
    let a0 = (first.y - c.y).atan2(first.x - c.x);
    (0..sides)
        .map(|i| {
            let a = a0 + std::f64::consts::TAU * i as f64 / sides as f64;
            Point::new(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_roundtrip() {
        let t = Affine::rotate(0.3)
            .then(&Affine::scale(2.0, 3.0))
            .then(&Affine::translate(5.0, -7.0));
        let inv = t.invert().unwrap();
        let p = Point::new(11.0, 13.0);
        let q = inv.apply(t.apply(p));
        assert!(p.dist(q) < 1e-9);
    }

    #[test]
    fn rotate_is_ccw_on_screen() {
        // +x rotated by 90° goes to -y (up on screen).
        let p = Affine::rotate(std::f64::consts::FRAC_PI_2).apply(Point::new(1.0, 0.0));
        assert!((p.x).abs() < 1e-12 && (p.y + 1.0).abs() < 1e-12);
    }

    #[test]
    fn rect_union_and_contains() {
        let a = Rect::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let b = Rect::new(Point::new(5.0, 5.0), Point::new(20.0, 15.0));
        let u = a.union(&b);
        assert_eq!(u.max, Point::new(20.0, 15.0));
        assert!(u.contains_rect(&a));
        assert!(a.intersects(&b));
        assert!(Rect::EMPTY.is_empty());
        assert_eq!(Rect::EMPTY.union(&a), a);
    }

    #[test]
    fn regular_polygon_has_sides() {
        let p = regular_polygon(Point::ZERO, Point::new(100.0, 0.0), 6);
        assert_eq!(p.len(), 6);
        for q in &p {
            assert!((q.len() - 100.0).abs() < 1e-9);
        }
    }

    #[test]
    fn round_rect_bounds() {
        let r = Rect::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        let pts = flatten_round_rect(&r, 10.0);
        let b = Rect::from_points(pts);
        assert!((b.width() - 100.0).abs() < 1e-9);
        assert!((b.height() - 50.0).abs() < 1e-9);
    }
}
