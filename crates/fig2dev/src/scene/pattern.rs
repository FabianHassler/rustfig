//! Fig fill patterns 41–62, as fig2dev's PostScript driver defines them.
//!
//! Each pattern is a tile of line art in PostScript points (y up), clipped to
//! its cell, with its own line width. In exports the tiles keep their
//! physical size whatever the magnification and start at the page's
//! lower-left corner, as in fig2dev's output; on screen they are anchored at
//! the Fig origin at 100 %.
//
// Ported from fig2dev dev/psprolog.h (`FILL_PAT01` … `FILL_PAT22`)
// (xfig licence, see LICENSE-xfig).

use figbase::geom::{Point, Rect, FIG_PER_POINT};
use std::f64::consts::PI;

/// Most tiles generated for one fill; larger regions get coarser tiles.
pub const MAX_TILES: f64 = 40_000.0;

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

/// Where pattern space sits in Fig coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatternFrame {
    /// Fig point of pattern space's origin (the tiles' lower-left corner).
    pub origin: Point,
    /// Fig units per PostScript point of pattern space.
    pub fig_per_pt: f64,
}

impl PatternFrame {
    /// On screen: anchored at the Fig origin, at 100 %.
    pub fn screen() -> PatternFrame {
        PatternFrame {
            origin: Point::ZERO,
            fig_per_pt: FIG_PER_POINT,
        }
    }

    /// In an export of `bounds` at `scale`: anchored at the page's lower-left
    /// corner, tiles at their physical size.
    pub fn export(bounds: &Rect, scale: f64) -> PatternFrame {
        let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
        PatternFrame {
            origin: Point::new(bounds.min.x, bounds.max.y),
            fig_per_pt: FIG_PER_POINT / scale,
        }
    }
}

impl Default for PatternFrame {
    fn default() -> Self {
        PatternFrame::screen()
    }
}

/// A pattern's tile: cell size and line width in points, and line art in
/// the cell (y up), which may cross the cell's edge and is clipped to it.
pub struct Tile {
    pub width: f64,
    pub height: f64,
    pub line_width: f64,
    pub lines: Vec<Vec<(f64, f64)>>,
}

fn seg(a: (f64, f64), b: (f64, f64)) -> Vec<(f64, f64)> {
    vec![a, b]
}

/// An arc in PostScript's convention: centre, radius, degrees counter-clockwise.
fn arc(cx: f64, cy: f64, r: f64, a0: f64, a1: f64) -> Vec<(f64, f64)> {
    let n = 24;
    (0..=n)
        .map(|i| {
            let a = (a0 + (a1 - a0) * f64::from(i) / f64::from(n)) * PI / 180.0;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

/// The tile of pattern `kind` (41–62).
pub fn tile(kind: i32) -> Option<Tile> {
    let t = |width: f64, height: f64, line_width: f64, lines: Vec<Vec<(f64, f64)>>| Tile {
        width,
        height,
        line_width,
        lines,
    };
    let steps = |from: i32, step: i32, to: i32| {
        let mut v = Vec::new();
        let mut i = from;
        while (step > 0 && i <= to) || (step < 0 && i >= to) {
            v.push(f64::from(i));
            i += step;
        }
        v
    };
    Some(match kind {
        // left30
        41 => {
            let mut l: Vec<_> = steps(3, 4, 24)
                .into_iter()
                .map(|y| seg((-2.0, y), (2.0 * y, -1.0)))
                .collect();
            l.extend(steps(1, 4, 22).into_iter().map(|i| seg((2.0 * i, 25.0), (50.0, i))));
            t(48.0, 24.0, 0.7, l)
        }
        // right30
        42 => {
            let mut l: Vec<_> = steps(-3, -4, -24)
                .into_iter()
                .map(|i| seg((-2.0, 24.0 + i), (-2.0 * i, 25.0)))
                .collect();
            l.extend(
                steps(-1, -4, -22)
                    .into_iter()
                    .map(|i| seg((-2.0 * i, -1.0), (50.0, 24.0 + i))),
            );
            t(48.0, 24.0, 0.7, l)
        }
        // crosshatch30
        43 => {
            let mut l = Vec::new();
            for y in steps(3, 4, 24) {
                l.push(seg((-2.0, y), (2.0 * y, -1.0)));
                l.push(seg((-2.0, 24.0 - y), (2.0 * y, 25.0)));
            }
            for i in steps(1, 4, 22) {
                l.push(seg((2.0 * i, 25.0), (50.0, i)));
                l.push(seg((2.0 * i, -1.0), (50.0, 24.0 - i)));
            }
            t(48.0, 24.0, 0.7, l)
        }
        // left45
        44 => {
            let mut l = Vec::new();
            for i in steps(7, 8, 24) {
                l.push(seg((-1.0, i), (i, -1.0)));
                l.push(seg((i - 2.0, 25.0), (25.0, i - 2.0)));
            }
            t(24.0, 24.0, 1.0, l)
        }
        // right45
        45 => {
            let mut l = Vec::new();
            for i in steps(1, 8, 18) {
                l.push(seg((i, -1.0), (25.0, 24.0 - i)));
                l.push(seg((-1.0, i + 4.0), (20.0 - i, 25.0)));
            }
            t(24.0, 24.0, 1.0, l)
        }
        // crosshatch45
        46 => {
            let mut l = Vec::new();
            for i in steps(1, 8, 18) {
                l.push(seg((i, -1.0), (25.0, 24.0 - i)));
                l.push(seg((-1.0, i + 4.0), (20.0 - i, 25.0)));
                l.push(seg((-1.0, i + 6.0), (i + 6.0, -1.0)));
                l.push(seg((i + 4.0, 25.0), (25.0, i + 4.0)));
            }
            t(24.0, 24.0, 1.0, l)
        }
        // bricks
        47 => {
            let mut l: Vec<_> = [2.0, 10.0, 18.0, 26.0]
                .iter()
                .map(|&y| seg((-1.0, y), (33.0, y)))
                .collect();
            for x in [2.0, 18.0] {
                l.push(seg((x, 2.0), (x, 10.0)));
                l.push(seg((x, 18.0), (x, 26.0)));
            }
            for x in [10.0, 26.0] {
                l.push(seg((x, -1.0), (x, 2.0)));
                l.push(seg((x, 10.0), (x, 18.0)));
                l.push(seg((x, 26.0), (x, 33.0)));
            }
            t(32.0, 32.0, 1.0, l)
        }
        // vertical bricks: bricks with x and y swapped
        48 => {
            let b = tile(47)?;
            let lines = b
                .lines
                .into_iter()
                .map(|l| l.into_iter().map(|(x, y)| (y, x)).collect())
                .collect();
            t(32.0, 32.0, 1.0, lines)
        }
        // horizontal lines
        49 => t(
            36.0,
            24.0,
            1.0,
            steps(2, 4, 22).into_iter().map(|y| seg((-1.0, y), (37.0, y))).collect(),
        ),
        // vertical lines
        50 => t(
            24.0,
            36.0,
            1.0,
            steps(2, 4, 22).into_iter().map(|x| seg((x, -1.0), (x, 37.0))).collect(),
        ),
        // crosshatch lines
        51 => {
            let mut l = Vec::new();
            for v in steps(2, 4, 34) {
                l.push(seg((-1.0, v), (37.0, v)));
                l.push(seg((v, -1.0), (v, 37.0)));
            }
            t(36.0, 36.0, 1.0, l)
        }
        // left-shingles, right-shingles
        52 | 53 => {
            let mut l: Vec<_> = [2.0, 10.0, 18.0].iter().map(|&y| seg((-1.0, y), (25.0, y))).collect();
            let extra: [((f64, f64), (f64, f64)); 4] = if kind == 52 {
                [
                    ((2.0, 10.0), (6.0, 18.0)),
                    ((10.0, 2.0), (14.0, 10.0)),
                    ((18.0, 18.0), (22.0, 26.0)),
                    ((20.0, -2.0), (22.0, 2.0)),
                ]
            } else {
                [
                    ((2.0, 26.0), (6.0, 18.0)),
                    ((14.0, 2.0), (10.0, 10.0)),
                    ((18.0, 18.0), (22.0, 10.0)),
                    ((2.0, 2.0), (4.0, -2.0)),
                ]
            };
            l.extend(extra.iter().map(|&(a, b)| seg(a, b)));
            t(24.0, 24.0, 1.0, l)
        }
        // vertical left-shingles, vertical right-shingles
        54 | 55 => {
            let mut l: Vec<_> = [2.0, 10.0, 18.0].iter().map(|&x| seg((x, -1.0), (x, 25.0))).collect();
            let extra: [((f64, f64), (f64, f64)); 4] = if kind == 54 {
                [
                    ((26.0, 2.0), (18.0, 6.0)),
                    ((2.0, 14.0), (10.0, 10.0)),
                    ((18.0, 18.0), (10.0, 22.0)),
                    ((2.0, 2.0), (-2.0, 4.0)),
                ]
            } else {
                [
                    ((10.0, 2.0), (18.0, 6.0)),
                    ((2.0, 10.0), (10.0, 14.0)),
                    ((18.0, 18.0), (26.0, 22.0)),
                    ((-2.0, 20.0), (2.0, 22.0)),
                ]
            };
            l.extend(extra.iter().map(|&(a, b)| seg(a, b)));
            t(24.0, 24.0, 1.0, l)
        }
        // fishscales
        56 => {
            let centres = [
                (8.0, 9.0),
                (8.0, 17.0),
                (8.0, 25.0),
                (0.0, 13.0),
                (16.0, 13.0),
                (0.0, 21.0),
                (16.0, 21.0),
            ];
            t(
                16.0,
                8.0,
                0.7,
                centres.iter().map(|&(x, y)| arc(x, y, 11.0, 223.0, 317.0)).collect(),
            )
        }
        // small fishscales: rows of lower half circles, joined
        57 => {
            let rows = [(-6, 2.0), (-2, 6.0), (-6, 10.0), (-2, 14.0), (-6, 18.0)];
            let lines = rows
                .iter()
                .map(|&(from, y)| {
                    let to = if from == -6 { 27 } else { 23 };
                    steps(from, 8, to)
                        .into_iter()
                        .flat_map(|x| arc(x, y, 4.0, 180.0, 360.0))
                        .collect()
                })
                .collect();
            t(24.0, 16.0, 0.7, lines)
        }
        // circles
        58 => t(16.0, 16.0, 0.7, vec![arc(8.0, 8.0, 8.0, 0.0, 360.0)]),
        // hexagons
        59 => t(
            26.0,
            16.0,
            0.7,
            vec![
                vec![
                    (-1.0, 8.0),
                    (2.0, 8.0),
                    (6.0, 0.0),
                    (15.0, 0.0),
                    (19.0, 8.0),
                    (15.0, 16.0),
                    (6.0, 16.0),
                    (2.0, 8.0),
                ],
                seg((19.0, 8.0), (27.0, 8.0)),
            ],
        ),
        // octagons
        60 => t(
            16.0,
            16.0,
            0.8,
            vec![vec![
                (5.0, 0.0),
                (11.0, 0.0),
                (16.0, 5.0),
                (16.0, 11.0),
                (11.0, 16.0),
                (5.0, 16.0),
                (0.0, 11.0),
                (0.0, 5.0),
                (5.0, 0.0),
            ]],
        ),
        // horizontal sawtooth lines
        61 => {
            let lines = [5.0, 13.0, 21.0]
                .iter()
                .map(|&y| {
                    let mut l = vec![(-1.0, y), (2.0, y - 3.0)];
                    let mut x = 2.0;
                    for _ in 0..3 {
                        l.push((x + 4.0, y + 1.0));
                        l.push((x + 8.0, y - 3.0));
                        x += 8.0;
                    }
                    l
                })
                .collect();
            t(24.0, 24.0, 0.8, lines)
        }
        // vertical sawtooth lines
        62 => {
            let lines = [5.0, 13.0, 21.0]
                .iter()
                .map(|&x| {
                    let mut l = vec![(x, -1.0), (x - 3.0, 2.0)];
                    let mut y = 2.0;
                    for _ in 0..3 {
                        l.push((x + 1.0, y + 4.0));
                        l.push((x - 3.0, y + 8.0));
                        y += 8.0;
                    }
                    l
                })
                .collect();
            t(24.0, 24.0, 0.8, lines)
        }
        _ => return None,
    })
}

/// The part of segment `a`–`b` inside `[0, w] × [0, h]` (Liang–Barsky).
fn clip(a: (f64, f64), b: (f64, f64), w: f64, h: f64) -> Option<((f64, f64), (f64, f64))> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [(-dx, a.0), (dx, w - a.0), (-dy, a.1), (dy, h - a.1)] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    (t0 < t1).then_some(((a.0 + t0 * dx, a.1 + t0 * dy), (a.0 + t1 * dx, a.1 + t1 * dy)))
}

/// The tile's lines clipped to its cell, joined where they continue.
fn clipped(t: &Tile) -> Vec<Vec<(f64, f64)>> {
    let mut out: Vec<Vec<(f64, f64)>> = Vec::new();
    for line in &t.lines {
        let mut cur: Vec<(f64, f64)> = Vec::new();
        for w in line.windows(2) {
            let (Some(&a), Some(&b)) = (w.first(), w.get(1)) else {
                continue;
            };
            match clip(a, b, t.width, t.height) {
                Some((p, q)) => {
                    let joins = cur
                        .last()
                        .is_some_and(|l| (l.0 - p.0).abs() < 1e-9 && (l.1 - p.1).abs() < 1e-9);
                    if !joins && cur.len() > 1 {
                        out.push(std::mem::take(&mut cur));
                    }
                    if !joins {
                        cur = vec![p];
                    }
                    cur.push(q);
                }
                None if cur.len() > 1 => out.push(std::mem::take(&mut cur)),
                None => cur.clear(),
            }
        }
        if cur.len() > 1 {
            out.push(cur);
        }
    }
    out
}

/// Pattern lines covering `region` (Fig units) and their stroke width in Fig
/// units, in `frame`.
pub fn coverage(kind: i32, region: &Rect, frame: &PatternFrame) -> (Vec<Vec<Point>>, f64) {
    let Some(t) = tile(kind) else {
        return (Vec::new(), 0.0);
    };
    let k0 = frame.fig_per_pt;
    if region.is_empty()
        || !region.width().is_finite()
        || !region.height().is_finite()
        || !k0.is_finite()
        || k0 <= 0.0
        || !frame.origin.is_finite()
    {
        return (Vec::new(), 0.0);
    }
    // Very large regions get coarser tiles rather than too many.
    let mut k = k0;
    let count = |k: f64| (region.width() / (t.width * k) + 2.0) * (region.height() / (t.height * k) + 2.0);
    while count(k) > MAX_TILES && k < k0 * 1.0e12 {
        k *= 2.0;
    }
    let (w, h) = (t.width * k, t.height * k);
    let lines = clipped(&t);
    // Pattern space has y up: tile (i, j) covers Fig x from origin.x + i·w,
    // y from origin.y − (j + 1)·h.
    let i0 = ((region.min.x - frame.origin.x) / w).floor() as i64;
    let i1 = ((region.max.x - frame.origin.x) / w).ceil() as i64;
    let j0 = ((frame.origin.y - region.max.y) / h).floor() as i64;
    let j1 = ((frame.origin.y - region.min.y) / h).ceil() as i64;
    let mut out = Vec::new();
    for j in j0..j1 {
        for i in i0..i1 {
            let (ox, oy) = (frame.origin.x + i as f64 * w, frame.origin.y - j as f64 * h);
            for l in &lines {
                out.push(l.iter().map(|&(x, y)| Point::new(ox + x * k, oy - y * k)).collect());
            }
        }
    }
    (out, t.line_width * k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_patterns_have_tiles() {
        for k in 41..=62 {
            let t = tile(k).unwrap();
            assert!(
                t.width > 0.0 && t.height > 0.0 && t.line_width > 0.0 && !t.lines.is_empty(),
                "{k}"
            );
            assert!(!clipped(&t).is_empty(), "{k}");
        }
        assert!(tile(40).is_none());
        assert!(tile(63).is_none());
    }

    #[test]
    fn lines_stay_in_their_cell() {
        for k in 41..=62 {
            let t = tile(k).unwrap();
            for l in clipped(&t) {
                for (x, y) in l {
                    assert!(
                        (-1e-9..=t.width + 1e-9).contains(&x) && (-1e-9..=t.height + 1e-9).contains(&y),
                        "{k}"
                    );
                }
            }
        }
    }

    #[test]
    fn left30_matches_fig2dev() {
        // fig2dev: "3 4 24 {dup -2 exch m 2 mul -1 l} for": first line -2 3 m 6 -1 l.
        let t = tile(41).unwrap();
        assert_eq!(t.lines[0], vec![(-2.0, 3.0), (6.0, -1.0)]);
        assert_eq!(t.lines.len(), 6 + 6);
        assert_eq!((t.width, t.height, t.line_width), (48.0, 24.0, 0.7));
    }

    #[test]
    fn coverage_is_bounded_and_framed() {
        let huge = Rect::new(Point::new(-1e9, -1e9), Point::new(1e9, 1e9));
        let (c, _) = coverage(46, &huge, &PatternFrame::screen());
        assert!(c.len() < 400_000);
        // Horizontal lines (36 × 24 pt, lines 1 pt) at 100 %: one tile is 600 × 400 Fig units.
        let frame = PatternFrame::screen();
        let small = Rect::new(Point::new(0.0, -400.0), Point::new(600.0, 0.0));
        let (c, w) = coverage(49, &small, &frame);
        assert_eq!(c.len(), 6);
        assert!((w - FIG_PER_POINT).abs() < 1e-9);
        // A line at y = 2 pt lies 2 pt above the frame origin.
        assert!(c
            .iter()
            .any(|l| l.iter().all(|p| (p.y + 2.0 * FIG_PER_POINT).abs() < 1e-6)));
        // At half scale the tiles double in Fig units.
        let half = PatternFrame::export(&small, 0.5);
        assert!((half.fig_per_pt - 2.0 * FIG_PER_POINT).abs() < 1e-9);
    }
}
