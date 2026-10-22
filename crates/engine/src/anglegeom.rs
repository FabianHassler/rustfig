//! Angle geometry (xfig's "Angle Geom" indicator): the constraint that
//! turns the pointer position into the next point of a line or spline.
//! Ported from xfig src/u_elastic.c (`latex_line`, `constrainedangle_line`,
//! `angle0_line` ... `angle135_line`), src/u_geom.c (`get_slope`,
//! `latex_endpoint`, the slope tables) and src/w_indpanel.c (`show_anglegeom`)
//! (xfig licence, see LICENSE-xfig).

use rustfig_geom::Point;
use serde::{Deserialize, Serialize};

/// xfig's `cur_anglegeom` choices (`L_UNCONSTRAINED` ... `L_MOUNTAIN` in
/// `mode.h`, in the order of the choice panel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AngleGeom {
    /// Any direction.
    #[default]
    Unconstrained,
    /// Only the slopes LaTeX allows for lines.
    LatexLine,
    /// Only the slopes LaTeX allows for arrows (vectors).
    LatexArrow,
    /// Horizontal, vertical and 45 degree diagonals ("mount-hattan").
    MountHattan,
    /// Horizontal or vertical only.
    Manhattan,
    /// 45 degree diagonals only.
    Mountain,
}

impl AngleGeom {
    pub const ALL: [AngleGeom; 6] = [
        AngleGeom::Unconstrained,
        AngleGeom::LatexLine,
        AngleGeom::LatexArrow,
        AngleGeom::MountHattan,
        AngleGeom::Manhattan,
        AngleGeom::Mountain,
    ];

    /// xfig's message when the choice is made (`show_anglegeom`).
    pub fn label(self) -> &'static str {
        match self {
            AngleGeom::Unconstrained => "unconstrained",
            AngleGeom::LatexLine => "latex line slopes",
            AngleGeom::LatexArrow => "latex arrow slopes",
            AngleGeom::MountHattan => "mount-hattan",
            AngleGeom::Manhattan => "manhattan",
            AngleGeom::Mountain => "mountain",
        }
    }
}

/// Slopes `(x, y)` LaTeX allows for lines (`line_angles`).
const LINE_SLOPES: [(i64, i64); 25] = [
    (0, 1),
    (1, 0),
    (1, 1),
    (1, 2),
    (1, 3),
    (1, 4),
    (1, 5),
    (1, 6),
    (2, 1),
    (2, 3),
    (2, 5),
    (3, 1),
    (3, 2),
    (3, 4),
    (3, 5),
    (4, 1),
    (4, 3),
    (4, 5),
    (5, 1),
    (5, 2),
    (5, 3),
    (5, 4),
    (5, 6),
    (6, 1),
    (6, 5),
];

/// Slopes LaTeX allows for arrows (`arrow_angles`).
const ARROW_SLOPES: [(i64, i64); 13] = [
    (0, 1),
    (1, 0),
    (1, 1),
    (1, 2),
    (1, 3),
    (1, 4),
    (2, 1),
    (2, 3),
    (3, 1),
    (3, 2),
    (3, 4),
    (4, 1),
    (4, 3),
];

/// Coordinates beyond this are not constrained (they would overflow the
/// integer arithmetic below); such points come from hostile input only.
const MAX_COORD: f64 = 1.0e9;

/// The point the pointer `p` stands for when the next point of a line is
/// drawn from `fix` with `mode` (and `magnet`, the point positioning step
/// in Fig units, 1 for free positioning, used by the LaTeX modes). xfig's
/// `latex_line` / `constrainedangle_line` / `unconstrained_line`.
pub fn constrain(mode: AngleGeom, fix: Point, p: Point, magnet: i64) -> Point {
    if mode == AngleGeom::Unconstrained || !fix.is_finite() || !p.is_finite() {
        return p;
    }
    if fix.x.abs() > MAX_COORD || fix.y.abs() > MAX_COORD || p.x.abs() > MAX_COORD || p.y.abs() > MAX_COORD {
        return p;
    }
    // xfig works on integer Fig coordinates.
    let (fx, fy) = (fix.x.round() as i64, fix.y.round() as i64);
    let (x, y) = (p.x.round() as i64, p.y.round() as i64);
    let (cx, cy) = match mode {
        AngleGeom::Unconstrained => (x, y),
        AngleGeom::LatexLine | AngleGeom::LatexArrow => {
            latex_endpoint(fx, fy, x, y, mode == AngleGeom::LatexArrow, magnet.clamp(1, 1_000_000))
        }
        AngleGeom::MountHattan | AngleGeom::Manhattan | AngleGeom::Mountain => {
            match constrained_angle(mode, fx, fy, x, y) {
                Some(c) => c,
                // The pointer is within 2 units of the fixed point: xfig
                // leaves the rubber band where it was; there it is `fix`.
                None => (fx, fy),
            }
        }
    };
    Point::new(cx as f64, cy as f64)
}

/// `constrainedangle_line`: `None` when the pointer has not moved 2 units
/// from the fixed point.
fn constrained_angle(mode: AngleGeom, fx: i64, fy: i64, x: i64, y: i64) -> Option<(i64, i64)> {
    let dx = (x - fx) as f64;
    let dy = (fy - y) as f64;
    if (dx * dx + dy * dy).sqrt() < 2.0 {
        return None;
    }
    let angle = if x == fx { -90.0 } else { (dy / dx).atan().to_degrees() };
    let a0 = || (x, fy);
    let a90 = || (fx, y);
    let a45 = || {
        if (x - fx).abs() < (y - fy).abs() {
            (fx - y + fy, y)
        } else {
            (x, fy + fx - x)
        }
    };
    let a135 = || {
        if (x - fx).abs() < (y - fy).abs() {
            (fx + y - fy, y)
        } else {
            (x, fy + x - fx)
        }
    };
    Some(match mode {
        AngleGeom::MountHattan => {
            if angle < -67.5 {
                a90()
            } else if angle < -22.5 {
                a135()
            } else if angle < 22.5 {
                a0()
            } else if angle < 67.5 {
                a45()
            } else {
                a90()
            }
        }
        AngleGeom::Manhattan => {
            if angle < -45.0 {
                a90()
            } else if angle < 45.0 {
                a0()
            } else {
                a90()
            }
        }
        _ => {
            if angle < 0.0 {
                a135()
            } else {
                a45()
            }
        }
    })
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn lcm(a: i64, b: i64) -> i64 {
    let g = gcd(a, b);
    if g == 0 {
        0
    } else {
        (a * b).abs() / g
    }
}

/// `get_slope`: the allowed slope nearest in angle to (`dx`, `dy`), with
/// the signs of the vector.
fn get_slope(dx: i64, dy: i64, arrow: bool) -> (i64, i64) {
    if dx == 0 {
        return (0, dy.signum());
    }
    let angle = (dy.abs() as f64 / dx.abs() as f64).atan().to_degrees();
    let table: &[(i64, i64)] = if arrow { &ARROW_SLOPES } else { &LINE_SLOPES };
    let mut best = (0, 1);
    let mut d = 9.9e9;
    for &(sx, sy) in table {
        let a = if sx == 0 {
            90.0
        } else {
            (sy as f64 / sx as f64).atan().to_degrees()
        };
        let d1 = (angle - a).abs();
        if d1 < d {
            best = (sx, sy);
            d = d1;
        }
    }
    let (mut sx, mut sy) = best;
    if dx < 0 {
        sx = -sx;
    }
    if dy < 0 {
        sy = -sy;
    }
    (sx, sy)
}

/// `latex_endpoint`: the point on an allowed slope from (`x1`, `y1`) nearest
/// to (`x2`, `y2`), a whole multiple of the smallest step that keeps both
/// coordinates on the `magnet` grid.
pub fn latex_endpoint(x1: i64, y1: i64, x2: i64, y2: i64, arrow: bool, magnet: i64) -> (i64, i64) {
    let (dx, dy) = (x2 - x1, y2 - y1);
    let (sx, sy) = get_slope(dx, dy, arrow);
    if sx.abs() >= sy.abs() && sx != 0 {
        let ds = lcm(sx, magnet * gcd(sx, magnet)).max(1);
        let n = (2 * dx.abs() / ds + 1) / 2;
        let dsx = if dx >= 0 { n * ds } else { -n * ds };
        (x1 + dsx, y1 + dsx * sy / sx)
    } else if sy != 0 {
        let ds = lcm(sy, magnet * gcd(sy, magnet)).max(1);
        let n = (2 * dy.abs() / ds + 1) / 2;
        let dsy = if dy >= 0 { n * ds } else { -n * ds };
        (x1 + dsy * sx / sy, y1 + dsy)
    } else {
        (x2, y2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(mode: AngleGeom, fix: (f64, f64), p: (f64, f64)) -> (f64, f64) {
        let r = constrain(mode, Point::new(fix.0, fix.1), Point::new(p.0, p.1), 75);
        (r.x, r.y)
    }

    #[test]
    fn unconstrained_is_identity() {
        assert_eq!(c(AngleGeom::Unconstrained, (0.0, 0.0), (123.0, 77.0)), (123.0, 77.0));
    }

    #[test]
    fn manhattan_picks_the_nearer_axis() {
        let m = AngleGeom::Manhattan;
        assert_eq!(c(m, (100.0, 100.0), (700.0, 220.0)), (700.0, 100.0));
        assert_eq!(c(m, (100.0, 100.0), (220.0, 700.0)), (100.0, 700.0));
        assert_eq!(c(m, (100.0, 100.0), (-500.0, 130.0)), (-500.0, 100.0));
        assert_eq!(c(m, (100.0, 100.0), (100.0, -400.0)), (100.0, -400.0));
        // Within 2 units of the fixed point nothing moves.
        assert_eq!(c(m, (100.0, 100.0), (101.0, 100.0)), (100.0, 100.0));
    }

    #[test]
    fn mountain_snaps_to_diagonals() {
        let m = AngleGeom::Mountain;
        // Up and right (angle > 0): the 45 degree line; dx > dy uses x.
        assert_eq!(c(m, (0.0, 0.0), (500.0, -300.0)), (500.0, -500.0));
        assert_eq!(c(m, (0.0, 0.0), (300.0, -500.0)), (500.0, -500.0));
        // Down and right (angle < 0): the 135 degree line.
        assert_eq!(c(m, (0.0, 0.0), (500.0, 300.0)), (500.0, 500.0));
        assert_eq!(c(m, (0.0, 0.0), (-300.0, 500.0)), (-500.0, 500.0));
    }

    #[test]
    fn mounthattan_has_eight_directions() {
        let m = AngleGeom::MountHattan;
        assert_eq!(c(m, (0.0, 0.0), (600.0, 100.0)), (600.0, 0.0));
        assert_eq!(c(m, (0.0, 0.0), (600.0, -400.0)), (600.0, -600.0));
        assert_eq!(c(m, (0.0, 0.0), (100.0, -600.0)), (0.0, -600.0));
        assert_eq!(c(m, (0.0, 0.0), (600.0, 400.0)), (600.0, 600.0));
        assert_eq!(c(m, (0.0, 0.0), (-600.0, -100.0)), (-600.0, 0.0));
    }

    #[test]
    fn latex_line_slopes() {
        let m = AngleGeom::LatexLine;
        // About 26.6 degrees is the slope 2:1; the step is the smallest
        // multiple of the slope on the 75-unit grid.
        assert_eq!(latex_endpoint(0, 0, 400, 205, false, 75), (450, 225));
        // Horizontal and vertical stay so.
        assert_eq!(c(m, (0.0, 0.0), (305.0, 3.0)), (300.0, 0.0));
        assert_eq!(c(m, (0.0, 0.0), (4.0, 305.0)), (0.0, 300.0));
        // 45 degrees.
        assert_eq!(c(m, (0.0, 0.0), (300.0, 310.0)), (300.0, 300.0));
        // Negative directions mirror.
        assert_eq!(c(m, (0.0, 0.0), (-300.0, -310.0)), (-300.0, -300.0));
        // Free positioning (magnet 1): exact multiples of the slope step.
        assert_eq!(latex_endpoint(0, 0, 400, 205, false, 1), (400, 200));
    }

    #[test]
    fn latex_arrow_has_fewer_slopes() {
        // 1:5 exists for lines (11.3 degrees) but not for arrows, where
        // the nearest is 1:4 (14.0 degrees).
        assert_eq!(get_slope(500, 100, false), (5, 1));
        assert_eq!(get_slope(500, 100, true), (4, 1));
        assert_eq!(get_slope(0, -5, true), (0, -1));
    }

    #[test]
    fn hostile_input_is_harmless() {
        for mode in AngleGeom::ALL {
            for v in [f64::NAN, f64::INFINITY, 1e300, -1e300, 0.0] {
                let _ = constrain(mode, Point::new(v, v), Point::new(1.0, 2.0), 0);
                let _ = constrain(mode, Point::new(0.0, 0.0), Point::new(v, 5.0), -3);
            }
        }
        assert_eq!(latex_endpoint(0, 0, 0, 0, false, 0), (0, 0));
    }
}
