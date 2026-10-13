//! Fig fill patterns 41–62, generated in code as line art tiled over a region.

use rustfig_geom::{Point, Rect};

/// Base tile size in Fig units (1/10 inch).
pub const TILE: f64 = 120.0;
/// Most tiles generated for one fill; larger regions get coarser tiles.
pub const MAX_TILES: f64 = 40_000.0;
/// Stroke width of pattern lines in Fig units.
pub const PATTERN_LINE_WIDTH: f64 = 6.0;

/// Display names of the patterns, index 0 = pattern 41.
pub const PATTERN_NAMES: [&str; 22] = [
    "30° left diagonal",
    "30° right diagonal",
    "30° crosshatch",
    "45° left diagonal",
    "45° right diagonal",
    "45° crosshatch",
    "horizontal bricks",
    "vertical bricks",
    "horizontal lines",
    "vertical lines",
    "crosshatch",
    "horizontal shingles right",
    "horizontal shingles left",
    "vertical shingles 1",
    "vertical shingles 2",
    "fish scales",
    "small fish scales",
    "circles",
    "hexagons",
    "octagons",
    "horizontal tire treads",
    "vertical tire treads",
];

fn arc(c: Point, r: f64, a0: f64, a1: f64) -> Vec<Point> {
    let n = 12;
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * f64::from(i) / f64::from(n);
            Point::new(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect()
}

fn transpose(v: Vec<Vec<Point>>) -> Vec<Vec<Point>> {
    v.into_iter()
        .map(|l| l.into_iter().map(|p| Point::new(p.y, p.x)).collect())
        .collect()
}

/// The tile of a pattern: (width, height, polylines inside the tile).
pub fn tile(kind: i32) -> Option<(f64, f64, Vec<Vec<Point>>)> {
    let t = TILE;
    let p = Point::new;
    let s3 = 3f64.sqrt();
    use std::f64::consts::PI;
    Some(match kind {
        41 => (t * s3, t, vec![vec![p(0.0, 0.0), p(t * s3, t)]]),
        42 => (t * s3, t, vec![vec![p(0.0, t), p(t * s3, 0.0)]]),
        43 => (
            t * s3,
            t,
            vec![vec![p(0.0, 0.0), p(t * s3, t)], vec![p(0.0, t), p(t * s3, 0.0)]],
        ),
        44 => (t, t, vec![vec![p(0.0, 0.0), p(t, t)]]),
        45 => (t, t, vec![vec![p(0.0, t), p(t, 0.0)]]),
        46 => (t, t, vec![vec![p(0.0, 0.0), p(t, t)], vec![p(0.0, t), p(t, 0.0)]]),
        47 => (
            2.0 * t,
            t,
            vec![
                vec![p(0.0, 0.0), p(2.0 * t, 0.0)],
                vec![p(0.0, t / 2.0), p(2.0 * t, t / 2.0)],
                vec![p(0.0, 0.0), p(0.0, t / 2.0)],
                vec![p(t, t / 2.0), p(t, t)],
            ],
        ),
        48 => {
            let (w, h, l) = tile(47)?;
            (h, w, transpose(l))
        }
        49 => (t, t / 2.0, vec![vec![p(0.0, 0.0), p(t, 0.0)]]),
        50 => (t / 2.0, t, vec![vec![p(0.0, 0.0), p(0.0, t)]]),
        51 => (
            t * 0.6,
            t * 0.6,
            vec![vec![p(0.0, 0.0), p(t * 0.6, 0.0)], vec![p(0.0, 0.0), p(0.0, t * 0.6)]],
        ),
        52 | 53 => {
            let sk = if kind == 52 { t * 0.25 } else { -t * 0.25 };
            (
                2.0 * t,
                t,
                vec![
                    vec![p(0.0, 0.0), p(2.0 * t, 0.0)],
                    vec![p(0.0, t / 2.0), p(2.0 * t, t / 2.0)],
                    vec![p(t * 0.5, 0.0), p(t * 0.5 + sk, t / 2.0)],
                    vec![p(t * 1.5, t / 2.0), p(t * 1.5 + sk, t)],
                ],
            )
        }
        54 | 55 => {
            let (w, h, l) = tile(kind - 2)?;
            (h, w, transpose(l))
        }
        56 | 57 => {
            let r = if kind == 56 { t / 2.0 } else { t / 4.0 };
            let w = 2.0 * r;
            let h = r;
            (
                w,
                h,
                vec![
                    arc(p(r, 0.0), r, 0.0, PI),
                    arc(p(0.0, h), r, 0.0, PI / 2.0),
                    arc(p(w, h), r, PI / 2.0, PI),
                ],
            )
        }
        58 => (t, t, vec![arc(p(t / 2.0, t / 2.0), t * 0.42, 0.0, 2.0 * PI)]),
        59 => {
            let s = t / 2.0;
            let h = s3 * s;
            (
                3.0 * s,
                h,
                vec![
                    vec![
                        p(0.0, h / 2.0),
                        p(s / 2.0, 0.0),
                        p(1.5 * s, 0.0),
                        p(2.0 * s, h / 2.0),
                        p(1.5 * s, h),
                        p(s / 2.0, h),
                        p(0.0, h / 2.0),
                    ],
                    vec![p(2.0 * s, h / 2.0), p(3.0 * s, h / 2.0)],
                ],
            )
        }
        60 => {
            let c = t * 0.29;
            (
                t,
                t,
                vec![vec![
                    p(c, 0.0),
                    p(t - c, 0.0),
                    p(t, c),
                    p(t, t - c),
                    p(t - c, t),
                    p(c, t),
                    p(0.0, t - c),
                    p(0.0, c),
                    p(c, 0.0),
                ]],
            )
        }
        61 => (t, t / 2.0, vec![vec![p(0.0, 0.0), p(t / 2.0, t / 2.0), p(t, 0.0)]]),
        62 => (t / 2.0, t, vec![vec![p(0.0, 0.0), p(t / 2.0, t / 2.0), p(0.0, t)]]),
        _ => return None,
    })
}

/// Pattern line art covering `region` (Fig units), anchored at the origin so
/// adjacent fills line up.
pub fn coverage(kind: i32, region: &Rect) -> Vec<Vec<Point>> {
    let Some((w0, h0, lines)) = tile(kind) else {
        return Vec::new();
    };
    if region.is_empty() || !region.width().is_finite() || !region.height().is_finite() {
        return Vec::new();
    }
    let mut k = 1.0;
    let count = |k: f64| (region.width() / (w0 * k) + 2.0) * (region.height() / (h0 * k) + 2.0);
    while count(k) > MAX_TILES && k < 1.0e12 {
        k *= 2.0;
    }
    let (w, h) = (w0 * k, h0 * k);
    let ix0 = (region.min.x / w).floor() as i64;
    let ix1 = (region.max.x / w).ceil() as i64;
    let iy0 = (region.min.y / h).floor() as i64;
    let iy1 = (region.max.y / h).ceil() as i64;
    let mut out = Vec::new();
    for iy in iy0..iy1 {
        for ix in ix0..ix1 {
            let off = Point::new(ix as f64 * w, iy as f64 * h);
            for l in &lines {
                out.push(l.iter().map(|q| off + *q * k).collect());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_patterns_have_tiles() {
        for k in 41..=62 {
            let (w, h, l) = tile(k).unwrap();
            assert!(w > 0.0 && h > 0.0 && !l.is_empty(), "{k}");
        }
        assert!(tile(40).is_none());
        assert!(tile(63).is_none());
    }

    #[test]
    fn coverage_is_bounded() {
        let huge = Rect::new(Point::new(-1e9, -1e9), Point::new(1e9, 1e9));
        let c = coverage(46, &huge);
        assert!(c.len() < 200_000);
        let small = Rect::new(Point::ZERO, Point::new(240.0, 240.0));
        assert_eq!(coverage(44, &small).len(), 4);
    }
}
