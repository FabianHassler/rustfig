//! Arrowhead outlines.

use crate::Point;

/// How an arrowhead is painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowFill {
    /// Outline only (stick arrows).
    None,
    /// Filled with white ("hollow" arrows).
    White,
    /// Filled with the pen colour.
    Pen,
}

/// An arrowhead ready to paint.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowShape {
    /// Outline points (closed if `closed`).
    pub outline: Vec<Point>,
    pub closed: bool,
    pub fill: ArrowFill,
    /// Where the line should end so it does not poke through the head.
    pub line_end: Point,
}

/// Build the arrowhead for a line ending at `tip`, arriving from `from`.
/// `kind` and `style` follow the Fig format (type 0–14, style 0 hollow / 1
/// filled); `width` and `height` are in Fig units.
pub fn arrowhead(tip: Point, from: Point, kind: i32, style: i32, width: f64, height: f64) -> Option<ArrowShape> {
    let d = (tip - from).normalized();
    if d == Point::ZERO || !width.is_finite() || !height.is_finite() {
        return None;
    }
    let n = d.perp();
    let (w, h) = (width.abs() / 2.0, height.abs());
    let base = tip - d * h;
    let fill = if style == 1 { ArrowFill::Pen } else { ArrowFill::White };
    let pt = |along: f64, side: f64| tip - d * along + n * side;
    let shape = match kind {
        0 => ArrowShape {
            outline: vec![base + n * w, tip, base - n * w],
            closed: false,
            fill: ArrowFill::None,
            line_end: tip,
        },
        2 => ArrowShape {
            outline: vec![pt(h, w), tip, pt(h, -w), pt(h * 0.7, 0.0)],
            closed: true,
            fill,
            line_end: pt(h * 0.7, 0.0),
        },
        3 => ArrowShape {
            outline: vec![pt(h * 0.7, w), tip, pt(h * 0.7, -w), pt(h, 0.0)],
            closed: true,
            fill,
            line_end: pt(h, 0.0),
        },
        // Circle / half circle.
        4 | 5 => {
            let r = w.max(h / 2.0);
            let c = tip - d * r;
            let steps = 24;
            let (lo, hi) = if kind == 4 {
                (0.0, std::f64::consts::TAU)
            } else {
                (-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2)
            };
            let outline = (0..=steps)
                .map(|i| {
                    let a = lo + (hi - lo) * i as f64 / steps as f64;
                    c + d * (r * a.cos()) + n * (r * a.sin())
                })
                .collect();
            ArrowShape {
                outline,
                closed: true,
                fill,
                line_end: if kind == 4 { tip - d * 2.0 * r } else { c },
            }
        }
        // Square.
        6 => ArrowShape {
            outline: vec![pt(0.0, w), pt(0.0, -w), pt(2.0 * w, -w), pt(2.0 * w, w)],
            closed: true,
            fill,
            line_end: pt(2.0 * w, 0.0),
        },
        // Diamond.
        7 => ArrowShape {
            outline: vec![tip, pt(h / 2.0, w), pt(h, 0.0), pt(h / 2.0, -w)],
            closed: true,
            fill,
            line_end: pt(h, 0.0),
        },
        // Bar (perpendicular line).
        8 => ArrowShape {
            outline: vec![tip + n * w, tip - n * w],
            closed: false,
            fill: ArrowFill::None,
            line_end: tip,
        },
        // Wide half-open "fork" pointing backwards.
        9 => ArrowShape {
            outline: vec![tip + n * w, base, tip - n * w],
            closed: false,
            fill: ArrowFill::None,
            line_end: tip,
        },
        // 1 and anything unknown: closed triangle.
        _ => ArrowShape {
            outline: vec![base + n * w, tip, base - n * w],
            closed: true,
            fill,
            line_end: base,
        },
    };
    Some(shape)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle_arrow() {
        let a = arrowhead(Point::new(100.0, 0.0), Point::ZERO, 1, 1, 40.0, 80.0).unwrap();
        assert!(a.closed);
        assert_eq!(a.fill, ArrowFill::Pen);
        assert!(a.line_end.dist(Point::new(20.0, 0.0)) < 1e-9);
        assert_eq!(a.outline.len(), 3);
    }

    #[test]
    fn degenerate_direction() {
        assert!(arrowhead(Point::ZERO, Point::ZERO, 1, 1, 40.0, 80.0).is_none());
    }

    #[test]
    fn all_kinds_build() {
        for k in -1..20 {
            for s in 0..2 {
                assert!(arrowhead(Point::new(5.0, 5.0), Point::ZERO, k, s, 30.0, 60.0).is_some());
            }
        }
    }
}
