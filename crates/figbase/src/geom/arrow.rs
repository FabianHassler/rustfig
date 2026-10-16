//! Arrowhead outlines, as fig2dev draws them.
//
// Ported from fig2dev bound.c (`arrow_shapes`, `calc_arrow`) and the arrow
// painting in dev/genps.c (xfig licence, see LICENSE-xfig).

use crate::geom::{stroke_width, Point};

/// How an arrowhead is painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowFill {
    /// Outline only (stick, wye, bar and fork arrows).
    None,
    /// Filled with white ("hollow" arrows).
    White,
    /// Filled with the pen colour.
    Pen,
}

/// An arrow as stored in a Fig file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrowSpec {
    /// Type 0–14.
    pub kind: i32,
    /// 0 hollow, 1 filled (for types 13 and 14: the second variant).
    pub style: i32,
    /// Outline thickness in 1/80 inch.
    pub thickness: f64,
    /// Width in Fig units.
    pub width: f64,
    /// Length in Fig units.
    pub height: f64,
}

/// An arrowhead ready to paint.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowShape {
    /// Outline points.
    pub outline: Vec<Point>,
    /// Whether the outline is stroked as a closed path.
    pub closed: bool,
    pub fill: ArrowFill,
    /// A part filled with the pen colour on top of a white fill (types 9).
    pub pen_part: Option<Vec<Point>>,
    /// Where the line should end so it does not poke through the head
    /// (fig2dev clips the line instead; for a straight end this is the same).
    pub line_end: Point,
}

/// One entry of fig2dev's `arrow_shapes`: points in units of (length,
/// width), the tip at (0, 0) and the line arriving along negative x.
struct Shape {
    /// Acuteness of the tip: how far the stroked tip pokes out (> 0), a blunt
    /// end (0) or no correction (< 0).
    tipmv: f64,
    points: &'static [(f64, f64)],
    /// The part filled with the pen colour, when not the whole outline.
    fill: &'static [(f64, f64)],
    /// Half-width heads are shifted to sit flush with the line.
    half: bool,
    /// Where the line ends, in units of the length behind the tip.
    clip: f64,
}

const STICK: &[(f64, f64)] = &[(-1.0, -0.5), (0.0, 0.0), (-1.0, 0.5)];
const TRIANGLE: &[(f64, f64)] = &[(-1.0, 0.5), (0.0, 0.0), (-1.0, -0.5), (-1.0, 0.5)];
const CONCAVE: &[(f64, f64)] = &[(-1.25, 0.5), (0.0, 0.0), (-1.25, -0.5), (-1.0, 0.0), (-1.25, 0.5)];
const CONVEX: &[(f64, f64)] = &[(-0.75, 0.5), (0.0, 0.0), (-0.75, -0.5), (-1.0, 0.0), (-0.75, 0.5)];
const DIAMOND: &[(f64, f64)] = &[(-0.5, 0.5), (0.0, 0.0), (-0.5, -0.5), (-1.0, 0.0), (-0.5, 0.5)];
const SQUARE: &[(f64, f64)] = &[(-1.0, 0.5), (0.0, 0.5), (0.0, -0.5), (-1.0, -0.5), (-1.0, 0.5)];
const REVERSE: &[(f64, f64)] = &[(0.0, 0.5), (0.0, -0.5), (-1.0, 0.0), (0.0, 0.5)];
const HALF_TRIANGLE: &[(f64, f64)] = &[(-1.0, 0.5), (0.0, 0.0), (-1.0, 0.0), (-1.0, 0.5)];
const HALF_CONCAVE: &[(f64, f64)] = &[(-1.25, 0.5), (0.0, 0.0), (-1.0, 0.0), (-1.25, 0.5)];
const HALF_CONVEX: &[(f64, f64)] = &[(-0.75, 0.5), (0.0, 0.0), (-1.0, 0.0), (-0.75, 0.5)];
const WYE: &[(f64, f64)] = &[(0.0, -0.5), (-1.0, 0.0), (0.0, 0.5)];
const BAR: &[(f64, f64)] = &[(0.0, -0.5), (0.0, 0.5)];
const FORK: &[(f64, f64)] = &[(0.0, -0.5), (-1.0, -0.5), (-1.0, 0.5), (0.0, 0.5)];
const BACK_FORK: &[(f64, f64)] = &[(-1.0, -0.5), (0.0, -0.5), (0.0, 0.5), (-1.0, 0.5)];

const fn shape(tipmv: f64, points: &'static [(f64, f64)], clip: f64) -> Shape {
    Shape {
        tipmv,
        points,
        fill: &[],
        half: false,
        clip,
    }
}

/// The polygon shapes by fig2dev's index `2 * type + style` (circles,
/// types 5 and 6, are computed).
fn table(index: usize) -> Option<Shape> {
    Some(match index {
        0 | 1 => shape(2.15, STICK, 0.0),
        2 | 3 => shape(2.1, TRIANGLE, 1.0),
        4 | 5 => shape(2.6, CONCAVE, 1.0),
        6 | 7 => shape(1.5, CONVEX, 1.0),
        8 | 9 => shape(1.15, DIAMOND, 1.0),
        14 | 15 => shape(0.0, SQUARE, 1.0),
        16 | 17 => shape(0.0, REVERSE, 1.0),
        18 => Shape {
            fill: &[(-1.25, -0.5), (0.0, 0.0), (-1.0, 0.0)],
            ..shape(2.6, CONCAVE, 1.0)
        },
        19 => Shape {
            fill: &[(-1.25, 0.5), (0.0, 0.0), (-1.0, 0.0)],
            ..shape(2.6, CONCAVE, 1.0)
        },
        20 | 21 => Shape {
            half: true,
            ..shape(2.5, HALF_TRIANGLE, 1.0)
        },
        22 | 23 => Shape {
            half: true,
            ..shape(3.5, HALF_CONCAVE, 1.0)
        },
        24 | 25 => Shape {
            half: true,
            ..shape(2.5, HALF_CONVEX, 1.0)
        },
        26 => shape(-1.0, WYE, 1.0),
        27 => shape(0.0, BAR, 0.0),
        28 => shape(-1.0, FORK, 1.0),
        29 => shape(0.0, BACK_FORK, 1.0),
        _ => return None,
    })
}

/// Number of arrow types (0–14).
pub const ARROW_TYPES: i32 = 15;

/// Builds the arrowhead for a line ending at `tip`, arriving from `from`.
/// `line_thickness` is the line's thickness in 1/80 inch (half-width heads
/// sit flush with it). Returns `None` for an invalid arrow, as fig2dev
/// ignores those: a type or style out of range, or zero width (except
/// circles).
pub fn arrowhead(tip: Point, from: Point, a: &ArrowSpec, line_thickness: f64) -> Option<ArrowShape> {
    let (wd, len) = (a.width, a.height);
    if !(0..ARROW_TYPES).contains(&a.kind) || !(0..=1).contains(&a.style) || !wd.is_finite() || !len.is_finite() {
        return None;
    }
    let circle = a.kind == 5 || a.kind == 6;
    if wd.abs() < 1e-2 && !circle {
        return None;
    }
    let d = (tip - from).normalized();
    if d == Point::ZERO {
        return None;
    }
    // Screen frame: x along the line towards the tip, y to its right.
    let n = Point::new(-d.y, d.x);
    let thk = stroke_width(a.thickness);
    let line_thk = stroke_width(line_thickness);
    let index = (2 * a.kind + a.style) as usize;
    let fill = if a.style == 1 { ArrowFill::Pen } else { ArrowFill::White };

    if circle {
        // Circles and half circles, approximated with 40 points.
        let radius = len / 2.0;
        // Type 5 moves the tip back by half the outline (a blunt end).
        let end = if a.kind == 5 { tip - d * (thk / 2.0) } else { tip };
        let (centre, sweep) = if a.kind == 5 {
            (end - d * radius, std::f64::consts::TAU)
        } else {
            (end, std::f64::consts::PI)
        };
        // Start at the side, sweep through the back (away from the tip).
        let start = if a.kind == 5 { 0.0 } else { std::f64::consts::FRAC_PI_2 };
        let outline = (0..40)
            .map(|i| {
                let t = start + sweep * f64::from(i) / 39.0;
                centre + d * (radius * t.cos()) + n * (radius * t.sin())
            })
            .collect();
        return Some(ArrowShape {
            outline,
            closed: a.kind == 5,
            fill,
            pen_part: None,
            line_end: if a.kind == 5 { centre } else { centre - d * radius },
        });
    }

    let s = table(index)?;
    // How far the stroked tip pokes beyond the end of the line (miter).
    let lpt = if s.tipmv > 0.0 {
        thk * (wd * wd + s.tipmv * s.tipmv * len * len).sqrt() / 2.0 / wd.abs()
    } else if s.tipmv == 0.0 {
        thk / 2.0
    } else {
        0.0
    };
    let end = tip - d * lpt;
    let offset = if s.half { (line_thk - thk) / 2.0 } else { 0.0 };
    let place = |&(x, y): &(f64, f64)| end + d * (x * len) + n * (y * wd - offset);
    let outline: Vec<Point> = s.points.iter().map(place).collect();
    let closed = a.kind != 0 && a.kind < 13;
    let pen_part = (!s.fill.is_empty()).then(|| s.fill.iter().map(place).collect());
    let fill = match a.kind {
        0 | 13 | 14 => ArrowFill::None,
        9 => ArrowFill::White,
        _ => fill,
    };
    Some(ArrowShape {
        outline,
        closed,
        fill,
        pen_part,
        line_end: end - d * (s.clip * len),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(kind: i32, style: i32) -> ArrowSpec {
        ArrowSpec {
            kind,
            style,
            thickness: 0.0,
            width: 40.0,
            height: 80.0,
        }
    }

    #[test]
    fn triangle_arrow() {
        let a = arrowhead(Point::new(100.0, 0.0), Point::ZERO, &spec(1, 1), 1.0).unwrap();
        assert!(a.closed);
        assert_eq!(a.fill, ArrowFill::Pen);
        assert!(a.line_end.dist(Point::new(20.0, 0.0)) < 1e-9);
        assert_eq!(a.outline.len(), 4);
    }

    #[test]
    fn tip_moves_back_by_the_miter() {
        // fig2dev: lpt = thk * sqrt(wd² + tipmv² len²) / 2 / wd.
        let s = ArrowSpec {
            thickness: 2.0,
            ..spec(1, 0)
        };
        let a = arrowhead(Point::new(1000.0, 0.0), Point::ZERO, &s, 1.0).unwrap();
        let thk = stroke_width(2.0);
        let lpt = thk * (40.0f64 * 40.0 + 2.1 * 2.1 * 80.0 * 80.0).sqrt() / 2.0 / 40.0;
        let tip = a.outline[1];
        assert!((tip.x - (1000.0 - lpt)).abs() < 1e-9, "{tip:?}");
    }

    #[test]
    fn diamond_is_type_4_and_square_type_7() {
        let d = arrowhead(Point::new(100.0, 0.0), Point::ZERO, &spec(4, 0), 1.0).unwrap();
        assert_eq!(d.outline.len(), 5);
        assert!(d.outline.iter().any(|p| (p.x - 20.0).abs() < 1e-9 && p.y.abs() < 1e-9));
        let s = arrowhead(Point::new(100.0, 0.0), Point::ZERO, &spec(7, 1), 1.0).unwrap();
        assert!(s
            .outline
            .iter()
            .any(|p| (p.x - 100.0).abs() < 1e-9 && (p.y - 20.0).abs() < 1e-9));
    }

    #[test]
    fn circles_and_open_shapes() {
        let c = arrowhead(Point::new(100.0, 0.0), Point::ZERO, &spec(5, 0), 1.0).unwrap();
        assert_eq!(c.outline.len(), 40);
        assert!(c.line_end.dist(Point::new(60.0, 0.0)) < 1e-9);
        let h = arrowhead(Point::new(100.0, 0.0), Point::ZERO, &spec(6, 1), 1.0).unwrap();
        assert!(!h.closed);
        assert!(h.outline.iter().all(|p| p.x <= 100.0 + 1e-9));
        let w = arrowhead(Point::new(100.0, 0.0), Point::ZERO, &spec(13, 0), 1.0).unwrap();
        assert_eq!(w.fill, ArrowFill::None);
        let half = arrowhead(Point::new(100.0, 0.0), Point::ZERO, &spec(9, 1), 1.0).unwrap();
        assert_eq!(half.fill, ArrowFill::White);
        assert_eq!(half.pen_part.map(|p| p.len()), Some(3));
    }

    #[test]
    fn invalid_arrows_are_ignored() {
        assert!(arrowhead(Point::ZERO, Point::ZERO, &spec(1, 1), 1.0).is_none());
        for (k, s) in [(-1, 0), (15, 0), (1, 2), (1, -1)] {
            assert!(arrowhead(Point::new(5.0, 5.0), Point::ZERO, &spec(k, s), 1.0).is_none());
        }
        let zero = ArrowSpec {
            width: 0.0,
            ..spec(1, 0)
        };
        assert!(arrowhead(Point::new(5.0, 5.0), Point::ZERO, &zero, 1.0).is_none());
        let nan = ArrowSpec {
            height: f64::NAN,
            ..spec(1, 0)
        };
        assert!(arrowhead(Point::new(5.0, 5.0), Point::ZERO, &nan, 1.0).is_none());
    }

    #[test]
    fn all_kinds_build() {
        for k in 0..ARROW_TYPES {
            for s in 0..2 {
                assert!(
                    arrowhead(Point::new(5.0, 5.0), Point::ZERO, &spec(k, s), 3.0).is_some(),
                    "{k} {s}"
                );
            }
        }
    }
}
