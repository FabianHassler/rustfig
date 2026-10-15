//! fig2dev's bounds (`bound.c`): bounding boxes of objects and compounds,
//! arrowhead shapes (`arrow_shapes`, `calc_arrow`) and the arc-arrow
//! angles the drivers share (`compute_arcarrow_angle`).
//!
//! Bounds are `(xmin, ymin, xmax, ymax)` in Fig units, computed as fig2dev
//! computes them, including its integer truncations, so that the drivers'
//! output matches fig2dev's.
//
// Ported from fig2dev bound.c, bound.h and localmath.c (xfig licence, see
// LICENSE-xfig).

use crate::object::{
    Arc, Arrow, Compound, DPoint, Ellipse, Line, Point, Spline, Text, T_CENTER_JUSTIFIED, T_PIC_BOX, T_PIE_WEDGE_ARC,
    T_RIGHT_JUSTIFIED,
};
use crate::Ctx;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// A bounding box: `(xmin, ymin, xmax, ymax)`.
pub type Bounds = (i32, i32, i32, i32);

const TWO_SEVENTY_DEG: f64 = PI + FRAC_PI_2;
/// fig2dev's "nothing yet" bounds.
const BIG: i32 = 10_000_000;
const EMPTY: Bounds = (BIG, BIG, -BIG, -BIG);
/// Above this many scan lines, `ellipse_bound` uses the closed form
/// instead of fig2dev's row-by-row scan (which takes one step per Fig unit
/// of height, too slow for hostile radii).
const MAX_ELLIPSE_ROWS: i64 = 1 << 20;

/// An arrowhead shape (`struct _arrow_shape`). `points.len()` is C's
/// `numpts`, `fillpoints.len()` its `numfillpts`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowShape {
    /// Which point is the tip.
    pub tipno: usize,
    /// The point at which the clip region starts.
    pub startclip: usize,
    /// Fill with `points` (true) or with `fillpoints`.
    pub simplefill: bool,
    /// Half-wide arrowhead, shifted to cover the line.
    pub half: bool,
    /// Acuteness of the tip (smaller angle, larger tipmv); 0 blunt,
    /// negative: no adjustment.
    pub tipmv: f64,
    /// The outline, in units of (length, width); the last point is the
    /// upper-left point of the tail.
    pub points: &'static [DPoint],
    /// The part to fill, if not `simplefill`.
    pub fillpoints: &'static [DPoint],
}

const fn p(x: f64, y: f64) -> DPoint {
    DPoint { x, y }
}

const fn shape(
    tipno: usize,
    startclip: usize,
    simplefill: bool,
    half: bool,
    tipmv: f64,
    points: &'static [DPoint],
    fillpoints: &'static [DPoint],
) -> ArrowShape {
    ArrowShape {
        tipno,
        startclip,
        simplefill,
        half,
        tipmv,
        points,
        fillpoints,
    }
}

const STICK: &[DPoint] = &[p(-1.0, -0.5), p(0.0, 0.0), p(-1.0, 0.5)];
const TRIANGLE: &[DPoint] = &[p(-1.0, 0.5), p(0.0, 0.0), p(-1.0, -0.5), p(-1.0, 0.5)];
const CONCAVE: &[DPoint] = &[p(-1.25, 0.5), p(0.0, 0.0), p(-1.25, -0.5), p(-1.0, 0.0), p(-1.25, 0.5)];
const CONVEX: &[DPoint] = &[p(-0.75, 0.5), p(0.0, 0.0), p(-0.75, -0.5), p(-1.0, 0.0), p(-0.75, 0.5)];
const DIAMOND: &[DPoint] = &[p(-0.5, 0.5), p(0.0, 0.0), p(-0.5, -0.5), p(-1.0, 0.0), p(-0.5, 0.5)];
const SQUARE: &[DPoint] = &[p(-1.0, 0.5), p(0.0, 0.5), p(0.0, -0.5), p(-1.0, -0.5), p(-1.0, 0.5)];
const REVERSE: &[DPoint] = &[p(0.0, 0.5), p(0.0, -0.5), p(-1.0, 0.0), p(0.0, 0.5)];
const HALF_TRIANGLE: &[DPoint] = &[p(-1.0, 0.5), p(0.0, 0.0), p(-1.0, 0.0), p(-1.0, 0.5)];
const HALF_CONCAVE: &[DPoint] = &[p(-1.25, 0.5), p(0.0, 0.0), p(-1.0, 0.0), p(-1.25, 0.5)];
const HALF_CONVEX: &[DPoint] = &[p(-0.75, 0.5), p(0.0, 0.0), p(-1.0, 0.0), p(-0.75, 0.5)];
const NONE: &[DPoint] = &[];

/// `arrow_shapes[]`, indexed by `2 * type + style`.
pub const ARROW_SHAPES: [ArrowShape; crate::gendev::NUMARROWS] = [
    // type 0
    shape(1, 0, true, false, 2.15, STICK, NONE),
    // placeholder for type 0 filled: a copy of type 0, since malicious
    // files might choose it
    shape(1, 0, true, false, 2.15, STICK, NONE),
    // type 1a simple triangle, 1b filled
    shape(1, 2, true, false, 2.1, TRIANGLE, NONE),
    shape(1, 2, true, false, 2.1, TRIANGLE, NONE),
    // type 2a concave spearhead, 2b filled
    shape(1, 2, true, false, 2.6, CONCAVE, NONE),
    shape(1, 2, true, false, 2.6, CONCAVE, NONE),
    // type 3a convex spearhead, 3b filled
    shape(1, 2, true, false, 1.5, CONVEX, NONE),
    shape(1, 2, true, false, 1.5, CONVEX, NONE),
    // type 4a diamond, 4b filled
    shape(1, 2, true, false, 1.15, DIAMOND, NONE),
    shape(1, 2, true, false, 1.15, DIAMOND, NONE),
    // type 5a/b circle, handled in code
    shape(0, 0, true, false, 0.0, NONE, NONE),
    shape(0, 0, true, false, 0.0, NONE, NONE),
    // type 6a/b half circle, handled in code
    shape(0, 0, true, false, -1.0, NONE, NONE),
    shape(0, 0, true, false, -1.0, NONE, NONE),
    // type 7a square, 7b filled
    shape(1, 3, true, false, 0.0, SQUARE, NONE),
    shape(1, 3, true, false, 0.0, SQUARE, NONE),
    // type 8a reverse triangle, 8b filled
    shape(1, 1, true, false, 0.0, REVERSE, NONE),
    shape(1, 1, true, false, 0.0, REVERSE, NONE),
    // type 9a top-half filled concave spearhead
    shape(
        1,
        2,
        false,
        false,
        2.6,
        CONCAVE,
        &[p(-1.25, -0.5), p(0.0, 0.0), p(-1.0, 0.0)],
    ),
    // type 9b bottom-half filled concave spearhead
    shape(
        1,
        2,
        false,
        false,
        2.6,
        CONCAVE,
        &[p(-1.25, 0.5), p(0.0, 0.0), p(-1.0, 0.0)],
    ),
    // type 10o top-half simple triangle, 10f filled
    shape(1, 2, true, true, 2.5, HALF_TRIANGLE, NONE),
    shape(1, 2, true, true, 2.5, HALF_TRIANGLE, NONE),
    // type 11o top-half concave spearhead, 11f filled
    shape(1, 2, true, true, 3.5, HALF_CONCAVE, NONE),
    shape(1, 2, true, true, 3.5, HALF_CONCAVE, NONE),
    // type 12o top-half convex spearhead, 12f filled
    shape(1, 2, true, true, 2.5, HALF_CONVEX, NONE),
    shape(1, 2, true, true, 2.5, HALF_CONVEX, NONE),
    // type 13a "wye"
    shape(
        0,
        0,
        true,
        false,
        -1.0,
        &[p(0.0, -0.5), p(-1.0, 0.0), p(0.0, 0.5)],
        NONE,
    ),
    // type 13b bar
    shape(1, 0, true, false, 0.0, &[p(0.0, -0.5), p(0.0, 0.5)], NONE),
    // type 14a two-prong fork
    shape(
        0,
        0,
        true,
        false,
        -1.0,
        &[p(0.0, -0.5), p(-1.0, -0.5), p(-1.0, 0.5), p(0.0, 0.5)],
        NONE,
    ),
    // type 14b backward two-prong fork
    shape(
        1,
        0,
        true,
        false,
        0.0,
        &[p(-1.0, -0.5), p(0.0, -0.5), p(0.0, 0.5), p(-1.0, 0.5)],
        NONE,
    ),
];

/// The shape of an arrowhead (`arrow_shapes[2*type + style]`); `None` for
/// a type or style outside the table.
pub fn arrow_shape(arrow: &Arrow) -> Option<&'static ArrowShape> {
    let indx = arrow.type_.checked_mul(2)?.checked_add(arrow.style)?;
    if arrow.style < 0 {
        return None;
    }
    ARROW_SHAPES.get(usize::try_from(indx).ok()?)
}

/// C's `(int)` conversion of a double: towards zero, saturating, NaN 0.
fn trunc(v: f64) -> i32 {
    v as i32
}

/// `floor(v + 0.5)` as an int (the `ROT?` macros of `calc_arrow`).
fn ri(v: f64) -> i32 {
    (v + 0.5).floor() as i32
}

/// Merge `b` into `acc`, fig2dev's way: the first contribution replaces
/// the "nothing yet" bounds.
fn merge(acc: &mut Option<Bounds>, b: Bounds) {
    *acc = Some(match *acc {
        None => b,
        Some(a) => (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)),
    });
}

fn pad(b: Bounds, half_wd: i32) -> Bounds {
    (
        b.0.saturating_sub(half_wd),
        b.1.saturating_sub(half_wd),
        b.2.saturating_add(half_wd),
        b.3.saturating_add(half_wd),
    )
}

/// `compound_bound`: the bounds of the figure, with or without texts. With
/// `-K` (`ctx.adjust_boundingbox`), only the depths `-D` keeps count.
pub fn compound_bound(ctx: &Ctx, compound: &Compound, include_text: bool) -> Bounds {
    compound_list_bound(ctx, std::slice::from_ref(compound), include_text)
}

/// `compound_bound` on C's sibling list: one bound for all of them.
fn compound_list_bound(ctx: &Ctx, list: &[Compound], include: bool) -> Bounds {
    let skip = |depth: i32| ctx.adjust_boundingbox && !ctx.depth_filter(depth);
    let mut acc: Option<Bounds> = None;
    for compound in list {
        for a in compound.arcs.iter().filter(|a| !skip(a.depth)) {
            let half_wd = a.thickness.saturating_add(1) / 2;
            merge(&mut acc, pad(arc_bound(ctx, a), half_wd));
        }
        if !compound.compounds.is_empty() {
            // An empty or filtered-out child gives fig2dev's "nothing yet"
            // bounds, which then count as found, as in C.
            merge(&mut acc, compound_list_bound(ctx, &compound.compounds, include));
        }
        for e in compound.ellipses.iter().filter(|e| !skip(e.depth)) {
            merge(&mut acc, ellipse_bound(e));
        }
        for l in compound.lines.iter().filter(|l| !skip(l.depth)) {
            let Some(b) = line_bound(ctx, l) else { continue };
            // pictures have no line thickness; otherwise leave space for
            // the corners
            let half_wd = if l.type_ == T_PIC_BOX {
                0
            } else {
                ((l.thickness as f64 + 1.0) / std::f64::consts::SQRT_2).ceil() as i32
            };
            merge(&mut acc, pad(b, half_wd));
        }
        for s in compound.splines.iter().filter(|s| !skip(s.depth)) {
            let Some(b) = spline_bound(ctx, s) else { continue };
            let half_wd = s.thickness.saturating_add(1) / 2;
            merge(&mut acc, pad(b, half_wd));
        }
        for t in compound.texts.iter().filter(|t| !skip(t.depth)) {
            merge(&mut acc, text_bound(ctx, t, include));
        }
    }
    acc.unwrap_or(EMPTY)
}

/// The angle of `(x, y)` around `c`, y up, in `[0, 2π)`.
fn angle_of(c: DPoint, q: Point) -> f64 {
    let a = (c.y - q.y as f64).atan2(q.x as f64 - c.x);
    if a < 0.0 {
        a + TAU
    } else {
        a
    }
}

/// `arc_bound`: the arc's three points, the axis extremes it passes, the
/// centre of a pie wedge, and its arrowheads.
pub fn arc_bound(ctx: &Ctx, arc: &Arc) -> Bounds {
    let [p0, p1, p2] = arc.point;
    let c = arc.center;
    let alpha = angle_of(c, p0);
    let dx = p0.x as f64 - c.x;
    let dy = c.y - p0.y as f64;
    let radius = (dx * dx + dy * dy).sqrt();
    let beta = angle_of(c, p2);

    let mut bx = p0.x.max(p1.x).max(p2.x);
    let mut by = p0.y.max(p1.y).max(p2.y);
    let mut sx = p0.x.min(p1.x).min(p2.x);
    let mut sy = p0.y.min(p1.y).min(p2.y);

    let right = trunc(c.x + radius + 1.0);
    let top = trunc(c.y - radius - 1.0);
    let left = trunc(c.x - radius - 1.0);
    let bottom = trunc(c.y + radius + 1.0);
    // Whether the arc passes the axis at angle `ang`, as bound.c tests it.
    let passes = |ang: f64| -> bool {
        if arc.direction == 1 {
            // counter clockwise
            if alpha > beta {
                alpha <= ang || ang <= beta
            } else {
                ang <= beta && alpha <= ang
            }
        } else if alpha > beta {
            beta <= ang && ang <= alpha
        } else {
            ang <= alpha || beta <= ang
        }
    };
    if passes(0.0) {
        bx = right;
    }
    if passes(FRAC_PI_2) {
        sy = top;
    }
    if passes(PI) {
        sx = left;
    }
    if passes(TWO_SEVENTY_DEG) {
        by = bottom;
    }
    // a pie wedge contains its centre
    if arc.type_ == T_PIE_WEDGE_ARC {
        sx = sx.min(trunc(c.x));
        bx = bx.max(trunc(c.x));
        sy = sy.min(trunc(c.y));
        by = by.max(trunc(c.y));
    }
    let mut b = (sx, sy, bx, by);
    if let Some(arrow) = &arc.for_arrow {
        let (p1x, p1y) = compute_arcarrow_angle(ctx, c.x, c.y, p2.x as f64, p2.y as f64, arc.direction, arrow);
        b = arrow_extend(ctx, b, Point::new(p1x, p1y), p2, arc.thickness, arrow);
    }
    if let Some(arrow) = &arc.back_arrow {
        let (p1x, p1y) = compute_arcarrow_angle(ctx, c.x, c.y, p0.x as f64, p0.y as f64, arc.direction ^ 1, arrow);
        b = arrow_extend(ctx, b, Point::new(p1x, p1y), p0, arc.thickness, arrow);
    }
    b
}

/// `ellipse_bound` (from xfig 2.1.8): scans the ellipse row by row, then
/// adds half the line thickness.
pub fn ellipse_bound(e: &Ellipse) -> Bounds {
    // fig2dev computes in float here.
    let xcen = e.center.x as f32 as f64;
    let ycen = e.center.y as f32 as f64;
    let a = e.radiuses.x as f32;
    let b = e.radiuses.y as f32;
    if a == 0.0 || b == 0.0 {
        return (trunc(xcen), trunc(ycen), trunc(xcen), trunc(ycen));
    }
    let cphi = e.angle.cos();
    let sphi = e.angle.sin();
    let cphisqr = cphi * cphi;
    let sphisqr = sphi * sphi;
    let asqr = (a * a) as f64;
    let bsqr = (b * b) as f64;

    let c1 = cphisqr / asqr + sphisqr / bsqr;
    let c2 = (cphi * sphi / asqr - cphi * sphi / bsqr) / c1;
    let mut c3 = bsqr * cphisqr + asqr * sphisqr;
    let yymax_f = c3.sqrt();
    let half_wd = e.thickness.saturating_add(1) / 2;

    if !yymax_f.is_finite() || yymax_f as i64 > MAX_ELLIPSE_ROWS {
        // The closed form, for radii too large to scan.
        let hw = (asqr * cphisqr + bsqr * sphisqr).sqrt().ceil();
        let hh = yymax_f.ceil();
        let r = (trunc(xcen - hw), trunc(ycen - hh), trunc(xcen + hw), trunc(ycen + hh));
        return pad(r, half_wd);
    }
    let yymax = yymax_f as i32;
    let c4 = (a * b) as f64 / c3;
    let mut c5 = 0.0;
    let mut v1 = c4 * c4;
    let c6 = 2.0 * v1;
    c3 = c3 * v1 - v1;

    // fig2dev's min and max mix ints and doubles; the result is truncated.
    let (mut xmin, mut ymin, mut xmax, mut ymax) = (BIG, BIG, -BIG, -BIG);
    let lo = |m: i32, v: f64| if (m as f64) < v { m } else { trunc(v) };
    let hi = |m: i32, v: f64| if (m as f64) > v { m } else { trunc(v) };
    let mut yy = 0.0;
    // odd first points
    if yymax % 2 != 0 {
        let d = c3.sqrt();
        xmin = lo(xmin, xcen - d.ceil());
        xmax = hi(xmax, xcen + d.ceil());
        ymin = lo(ymin, ycen);
        ymax = hi(ymax, ycen);
        c5 = c2;
        yy = 1.0;
    }
    while c3 >= 0.0 {
        let d = c3.sqrt();
        let xleft = c5 - d;
        let xright = c5 + d;
        xmin = lo(xmin, xcen + xleft.floor());
        xmax = hi(xmax, xcen + xleft.ceil());
        ymax = hi(ymax, ycen + yy);
        xmin = lo(xmin, xcen + xright.floor());
        xmax = hi(xmax, xcen + xright.ceil());
        xmin = lo(xmin, xcen - xright.ceil());
        xmax = hi(xmax, xcen - xright.floor());
        ymin = lo(ymin, ycen - yy);
        xmin = lo(xmin, xcen - xleft.ceil());
        xmax = hi(xmax, xcen - xleft.floor());
        c5 += c2;
        v1 += c6;
        c3 -= v1;
        yy += 1.0;
    }
    pad((xmin, ymin, xmax, ymax), half_wd)
}

/// `points_bound`; `None` without points.
fn points_bound(points: &[Point]) -> Option<Bounds> {
    let first = points.first()?;
    Some(points.iter().fold((first.x, first.y, first.x, first.y), |b, q| {
        (b.0.min(q.x), b.1.min(q.y), b.2.max(q.x), b.3.max(q.y))
    }))
}

/// `line_bound`: the points and, for two or more points, the arrowheads.
/// `None` for a line without points (which fig2dev's reader rejects).
pub fn line_bound(ctx: &Ctx, l: &Line) -> Option<Bounds> {
    let mut b = points_bound(&l.points)?;
    if let [first, second, ..] = l.points.as_slice() {
        let n = l.points.len();
        // `l->last[1]`, `l->last[0]`
        if let (Some(arrow), Some(&q), Some(&tip)) = (&l.for_arrow, l.points.get(n - 2), l.points.get(n - 1)) {
            b = arrow_extend(ctx, b, q, tip, l.thickness, arrow);
        }
        if let Some(arrow) = &l.back_arrow {
            b = arrow_extend(ctx, b, *second, *first, l.thickness, arrow);
        }
    }
    Some(b)
}

/// The bounds of an interpolated spline of an old file: the points and
/// the halves of the Bézier control polygon.
fn int_spline_bound(s: &Spline) -> Option<Bounds> {
    let half = |a: f64, b: f64| (a + b) / 2.0;
    let p0 = s.points.first()?;
    if s.controls.len() < s.points.len() {
        // fig2dev would read past the controls; use the points.
        return points_bound(&s.points);
    }
    let (mut sx, mut sy) = (p0.x as f64, p0.y as f64);
    let (mut bx, mut by) = (sx, sy);
    for (pp, cp) in s.points.windows(2).zip(s.controls.windows(2)) {
        let ([p1, p2], [cp1, cp2]) = (pp, cp) else { continue };
        let (x0, y0) = (p1.x as f64, p1.y as f64);
        let (x1, y1) = (cp1.rx, cp1.ry);
        let (x2, y2) = (cp2.lx, cp2.ly);
        let (x3, y3) = (p2.x as f64, p2.y as f64);
        let (tx, ty) = (half(x1, x2), half(y1, y2));
        let (sx1, sy1) = (half(x0, x1), half(y0, y1));
        let (sx2, sy2) = (half(sx1, tx), half(sy1, ty));
        let (tx2, ty2) = (half(x2, x3), half(y2, y3));
        let (tx1, ty1) = (half(tx2, tx), half(ty2, ty));
        for (x, y) in [(x0, y0), (sx1, sy1), (sx2, sy2), (tx1, ty1), (tx2, ty2), (x3, y3)] {
            sx = fmin(x, sx);
            sy = fmin(y, sy);
            bx = fmax(x, bx);
            by = fmax(y, by);
        }
    }
    Some((crate::round(sx), crate::round(sy), crate::round(bx), crate::round(by)))
}

/// C's `min` macro on doubles: `a < b ? a : b`.
fn fmin(a: f64, b: f64) -> f64 {
    if a < b {
        a
    } else {
        b
    }
}

/// C's `max` macro on doubles: `a > b ? a : b`.
fn fmax(a: f64, b: f64) -> f64 {
    if a > b {
        a
    } else {
        b
    }
}

/// The bounds of an approximated (B-)spline: the midpoints its quadratic
/// pieces pass through.
fn normal_spline_bound(s: &Spline) -> Option<Bounds> {
    let (first, rest) = s.points.split_first()?;
    let Some((second, rest)) = rest.split_first() else {
        return points_bound(&s.points);
    };
    let (mut x1, mut y1) = (first.x as f64, first.y as f64);
    let (mut x2, mut y2) = (second.x as f64, second.y as f64);
    let (cx1, cy1) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
    let (cx2, cy2) = ((cx1 + x2) / 2.0, (cy1 + y2) / 2.0);
    if s.closed_spline() {
        x1 = (cx1 + x1) / 2.0;
        y1 = (cy1 + y1) / 2.0;
    }
    let mut sx = fmin(x1, cx2);
    let mut sy = fmin(y1, cy2);
    let mut bx = fmax(x1, cx2);
    let mut by = fmax(y1, cy2);
    for q in rest {
        x1 = x2;
        y1 = y2;
        x2 = q.x as f64;
        y2 = q.y as f64;
        let (cx4, cy4) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
        let (cx3, cy3) = ((x1 + cx4) / 2.0, (y1 + cy4) / 2.0);
        let (cx2, cy2) = ((cx4 + x2) / 2.0, (cy4 + y2) / 2.0);
        sx = fmin(sx, fmin(cx2, cx3));
        sy = fmin(sy, fmin(cy2, cy3));
        bx = fmax(bx, fmax(cx2, cx3));
        by = fmax(by, fmax(cy2, cy3));
    }
    if s.closed_spline() {
        Some((trunc(sx.floor()), trunc(sy.floor()), trunc(bx.ceil()), trunc(by.ceil())))
    } else {
        Some((
            trunc(fmin(sx, x2).floor()),
            trunc(fmin(sy, y2).floor()),
            trunc(fmax(bx, x2).ceil()),
            trunc(fmax(by, y2).ceil()),
        ))
    }
}

/// `spline_bound`: the curve and its arrowheads. (Fig 3.2 splines reach
/// the drivers as lines; this serves older files and approximated
/// splines.) `None` for a spline without points.
pub fn spline_bound(ctx: &Ctx, s: &Spline) -> Option<Bounds> {
    let mut b = if s.int_spline() {
        int_spline_bound(s)?
    } else {
        normal_spline_bound(s)?
    };
    let n = s.points.len();
    if n >= 2 {
        // This doesn't work very well for a spline with few points and
        // lots of curvature (fig2dev's comment).
        if let (Some(arrow), Some(&q), Some(&tip)) = (&s.for_arrow, s.points.get(n - 2), s.points.get(n - 1)) {
            b = arrow_extend(ctx, b, q, tip, s.thickness, arrow);
        }
        if let (Some(arrow), Some(&tip), Some(&q)) = (&s.back_arrow, s.points.first(), s.points.get(1)) {
            b = arrow_extend(ctx, b, q, tip, s.thickness, arrow);
        }
    }
    Some(b)
}

/// `rot_x`: x of `(x, y)` rotated by `-angle`.
pub fn rot_x(x: f64, y: f64, angle: f64) -> f64 {
    x * (-angle).cos() - y * (-angle).sin()
}

/// `rot_y`: y of `(x, y)` rotated by `-angle`.
pub fn rot_y(x: f64, y: f64, angle: f64) -> f64 {
    x * (-angle).sin() + y * (-angle).cos()
}

/// Characters of the Symbol font that descend below the baseline
/// (bound.c's list; the 8-bit ones as the code points the reader decodes
/// them to, Latin-1).
const SYMBOL_DESCENDERS: &[char] = &[
    'b', 'c', 'f', 'g', 'h', 'j', 'm', 'r', 'x', 'y', 'z', '\u{c3}', '\u{ca}', '\u{cd}', '\u{d5}', '\u{e1}', '\u{e5}',
    '\u{e6}', '\u{e8}', '\u{f1}', '\u{f2}', '\u{f6}', '\u{f8}', '\u{a6}',
];

/// `text_bound`: the text's box from its stored length and height, rotated
/// by its angle, one `THICK_SCALE` larger on each side. The extent along
/// the baseline counts only with `inc_text` and not for special (LaTeX)
/// text that contains `\` or `$`.
pub fn text_bound(ctx: &Ctx, t: &Text, inc_text: bool) -> Bounds {
    let s = t.cstring.as_str();
    let has = |cs: &[char]| s.contains(cs);
    let include = inc_text && (!t.special_text() || (!s.contains('\\') && !s.contains('$')));
    // look for descenders (fig2dev's kludge)
    let mut descend = has(&[
        'g', 'j', 'p', 'q', 'y', '$', '(', ')', '{', '}', '[', ']', ',', ';', '_',
    ]);
    // Symbol font descenders
    if !descend && t.psfont_text() && t.font == 32 {
        descend = has(SYMBOL_DESCENDERS);
    }
    let len = t.length;
    let h = t.height;
    let mut d = match t.type_ {
        T_CENTER_JUSTIFIED => {
            let w = if include { len / 1.95 } else { 0.0 };
            [(w, 0.0), (-w, 0.0), (w, -h), (-w, -h)]
        }
        T_RIGHT_JUSTIFIED => {
            let w = if include { -len * 1.0256 } else { 0.0 };
            [(0.0, 0.0), (w, 0.0), (0.0, -h), (w, -h)]
        }
        _ => {
            let w = if include { len * 1.0256 } else { 0.0 };
            [(w, 0.0), (0.0, 0.0), (w, -h), (0.0, -h)]
        }
    };
    if descend {
        d[0].1 = 0.3 * h;
        d[1].1 = 0.3 * h;
        d[2].1 = -0.8 * h;
        d[3].1 = -0.8 * h;
    }
    let rx = d.map(|(x, y)| rot_x(x, y, t.angle));
    let ry = d.map(|(x, y)| rot_y(x, y, t.angle));
    let max4 = |v: [f64; 4]| fmax(fmax(v[0], v[1]), fmax(v[2], v[3]));
    let min4 = |v: [f64; 4]| fmin(fmin(v[0], v[1]), fmin(v[2], v[3]));
    let ts = ctx.thick_scale as f64;
    let bx = t.base_x as f64;
    let by = t.base_y as f64;
    (
        trunc(bx + min4(rx) - ts),
        trunc(by + min4(ry) - ts),
        trunc(bx + max4(rx) + ts),
        trunc(by + max4(ry) + ts),
    )
}

/// `arrow_bound` for one arrowhead: extend `b` by the outline of the
/// arrowhead heading from `from` to the tip `tip`.
fn arrow_extend(ctx: &Ctx, b: Bounds, from: Point, tip: Point, thickness: i32, arrow: &Arrow) -> Bounds {
    let pts = calc_arrow(ctx, from.x, from.y, tip.x, tip.y, thickness, arrow);
    let f = pts
        .points
        .iter()
        .fold(EMPTY, |f, q| (f.0.min(q.x), f.1.min(q.y), f.2.max(q.x), f.3.max(q.y)));
    (b.0.min(f.0), b.1.min(f.1), b.2.max(f.2), b.3.max(f.3))
}

/// The result of `calc_arrow`: the arrowhead's outline, the part to fill
/// if it differs from the outline (otherwise empty), and the clip area
/// that keeps the line out of the arrowhead.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArrowPoints {
    pub points: Vec<Point>,
    pub fillpoints: Vec<Point>,
    pub clippts: Vec<Point>,
}

/// `THICKNESS(T)`: the drawn width of a line of thickness `t`.
fn drawn_thickness(t: f64, thick_scale: f64) -> f64 {
    if t <= thick_scale {
        0.5 * t
    } else {
        t - thick_scale
    }
}

/// `calc_arrow`: the arrowhead heading from `(x1, y1)` to the tip
/// `(x2, y2)`. `linethick` is the thickness of the line, arc or spline,
/// not of the arrowhead. All lists are empty if the two points coincide or
/// the arrow type is unknown.
pub fn calc_arrow(ctx: &Ctx, x1: i32, y1: i32, x2: i32, y2: i32, linethick: i32, arrow: &Arrow) -> ArrowPoints {
    let mut out = ArrowPoints::default();
    let Some(sh) = arrow_shape(arrow) else {
        return out;
    };
    let ts = ctx.thick_scale as f64;
    let wd = arrow.wid;
    let len = arrow.ht;
    let line_thk = drawn_thickness(linethick as f64, ts);
    // to enlarge the clip area in case the line is thick
    let halfthick = line_thk / 2.0;
    // Slightly larger than the line; with less than 3, the ps driver
    // produces small shadows.
    let clipthick = halfthick + 3.0;

    let dx0 = x2 as i64 - x1 as i64;
    let dy0 = y1 as i64 - y2 as i64;
    if dx0 == 0 && dy0 == 0 {
        return out;
    }
    let (dx, dy) = (dx0 as f64, dy0 as f64);

    // lpt: how far the arrowhead extends beyond the end of the line
    // because of the sharp point (miter join)
    let tipmv = sh.tipmv;
    let thk = drawn_thickness(arrow.thickness, ts);
    let lpt = if tipmv > 0.0 {
        thk * (wd * wd + tipmv * tipmv * len * len).sqrt() / 2.0 / wd.abs()
    } else if tipmv == 0.0 {
        thk / 2.0 // blunt ends
    } else {
        0.0
    };

    let l = (dx * dx + dy * dy).sqrt();
    let sina = dy / l;
    let cosa = dx / l;
    // move the end of the line back so that the arrowhead's point ends
    // where the line used to
    let mx = x2 as f64 + lpt * -cosa;
    let my = y2 as f64 + lpt * sina;
    let xb = mx * cosa - my * sina;
    let yb = mx * sina + my * cosa;
    // (xa, ya): the rotated endpoint
    let xa = xb * cosa + yb * sina;
    let ya = -xb * sina + yb * cosa;
    let rot =
        |x: f64, y: f64, ox: f64, oy: f64| Point::new(ri(x * cosa + y * sina + ox), ri(-x * sina + y * cosa + oy));

    let type_ = arrow.type_;
    if type_ == 5 || type_ == 6 {
        // circle and half circle, approximated with 40 points
        let mut init_angle = dy.atan2(dx);
        if init_angle < 0.0 {
            init_angle += TAU;
        }
        let radius = len / 2.0;
        // (xs, ys): the length of the arrowhead back from the end of the
        // shaft; 0 for the half circle
        let (back, maxx) = if type_ == 5 { (len, radius) } else { (0.0, 0.0) };
        let xs = (xb - back) * cosa + yb * sina;
        let ys = -(xb - back) * sina + yb * cosa;
        // C stores these in ints
        let ddx = trunc(mx - xs) as f64;
        let ddy = trunc(my - ys) as f64;
        let fix_x = xs + ddx / 2.0;
        let fix_y = ys + ddy / 2.0;
        let np = 40;
        let (init_angle, rads) = if type_ == 5 {
            (5.0 * FRAC_PI_2 - init_angle, TAU)
        } else {
            (3.0 * FRAC_PI_2 - init_angle, PI)
        };
        out.points = (0..np)
            .map(|i| {
                let angle = init_angle - (rads * i as f64 / (np - 1) as f64);
                Point::new(ri(fix_x + radius * angle.cos()), ri(fix_y + radius * angle.sin()))
            })
            .collect();
        // clip to a box at least as large as the line thickness or the
        // circle's diameter
        let thickline = halfthick > radius;
        // start at the first point (half circle) or half into the circle
        let start = if type_ == 6 { 0 } else { np / 2 };
        out.clippts.extend(out.points.iter().skip(start).copied());
        if thickline {
            out.clippts.push(rot(0.0, clipthick, fix_x, fix_y));
        }
        out.clippts.push(rot(maxx + clipthick, clipthick, fix_x, fix_y));
        out.clippts.push(rot(maxx + clipthick, -clipthick, fix_x, fix_y));
        if thickline {
            out.clippts.push(rot(0.0, -clipthick, fix_x, fix_y));
        }
    } else {
        // all other heads; shift the half arrowheads down by half the
        // difference of the line's and the arrowhead's thickness, flush
        // with the line
        let offset = if sh.half { (line_thk - thk) / 2.0 } else { 0.0 };
        let mut miny = 100_000.0_f64;
        let mut maxy = -100_000.0_f64;
        let mut scaled = |q: &DPoint| {
            let x = q.x * len;
            let y = q.y * wd - offset;
            miny = fmin(y, miny);
            maxy = fmax(y, maxy);
            rot(x, y, xa, ya)
        };
        out.points = sh.points.iter().map(&mut scaled).collect();
        out.fillpoints = sh.fillpoints.iter().map(&mut scaled).collect();
        // clip to the last points of the arrowhead and the (enlarged) box
        // around it
        out.clippts.extend(out.points.iter().skip(sh.startclip).copied());
        // last x, upper y
        if halfthick > maxy {
            if let Some(q) = sh.points.last() {
                out.clippts.push(rot(q.x * len, clipthick, xa, ya));
            }
        }
        // the tip's x (always 0), same y, around the tip; add halfthick in
        // case the cap style is round or projecting
        out.clippts.push(rot(clipthick, clipthick, x2 as f64, y2 as f64));
        out.clippts.push(rot(clipthick, -clipthick, x2 as f64, y2 as f64));
        if -halfthick < miny {
            if let Some(q) = sh.points.get(sh.startclip) {
                out.clippts.push(rot(q.x * len, -clipthick, xa, ya));
            }
        }
    }
    out
}

/// `arc_tangent_int` (localmath.c): a point on the tangent of the arc
/// around `(x1, y1)` at `(x2, y2)`, in the arc's direction (1:
/// counter-clockwise).
pub fn arc_tangent_int(x1: f64, y1: f64, x2: f64, y2: f64, direction: i32) -> (i32, i32) {
    if direction != 0 {
        (trunc(x2 - (y2 - y1)), trunc(y2 + (x2 - x1)))
    } else {
        (trunc(x2 + (y2 - y1)), trunc(y2 - (x2 - x1)))
    }
}

/// `compute_arcarrow_angle`: the point on the chord of the arc around
/// `(x1, y1)` that meets the arc the arrowhead's length (plus its point's
/// overshoot) back from the end `(x2, y2)`. The arrowhead is drawn from
/// there to the end. Falls back to the tangent when the chord would be
/// too long or too short.
pub fn compute_arcarrow_angle(
    ctx: &Ctx,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    direction: i32,
    arrow: &Arrow,
) -> (i32, i32) {
    let dy = y2 - y1;
    let dx = x2 - x1;
    let r = (dx * dx + dy * dy).sqrt();
    let mut h = arrow.ht;
    // lines are made a little thinner in set_linewidth
    let thick = drawn_thickness(arrow.thickness, ctx.thick_scale as f64);
    // how far the arrowhead extends beyond the end of the line
    let lpt = thick / 2.0 / (arrow.wid / h / 2.0);
    h += lpt;
    // the secant would be too large or too small
    if h > 2.0 * r || h < 0.01 * r {
        return arc_tangent_int(x1, y1, x2, y2, direction);
    }
    let beta = dy.atan2(dx);
    let alpha = if direction != 0 {
        2.0 * (h / 2.0 / r).asin()
    } else {
        -2.0 * (h / 2.0 / r).asin()
    };
    (
        crate::round(x1 + r * (beta + alpha).cos()),
        crate::round(y1 + r * (beta + alpha).sin()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::{Control, T_BOX, T_OPEN_ARC, T_POLYLINE};

    fn ctx() -> Ctx {
        Ctx {
            thick_scale: 15.0,
            ..Ctx::default()
        }
    }

    fn arrow(type_: i32, style: i32) -> Arrow {
        Arrow {
            type_,
            style,
            thickness: 15.0,
            wid: 60.0,
            ht: 120.0,
        }
    }

    fn rect(x0: i32, y0: i32, x1: i32, y1: i32, thickness: i32) -> Line {
        Line {
            type_: T_BOX,
            thickness,
            points: vec![
                Point::new(x0, y0),
                Point::new(x1, y0),
                Point::new(x1, y1),
                Point::new(x0, y1),
                Point::new(x0, y0),
            ],
            ..Line::default()
        }
    }

    #[test]
    fn table_has_one_shape_per_arrow() {
        assert_eq!(ARROW_SHAPES.len(), 30);
        assert!(arrow_shape(&arrow(14, 1)).is_some());
        assert!(arrow_shape(&arrow(15, 0)).is_none());
        assert!(arrow_shape(&arrow(-1, 1)).is_none());
        assert!(arrow_shape(&arrow(2, -1)).is_none());
        assert!(arrow_shape(&arrow(i32::MAX, 1)).is_none());
        for s in &ARROW_SHAPES {
            assert!(s.startclip <= s.points.len());
        }
    }

    #[test]
    fn box_bound_adds_corner_space() {
        let fig = Compound {
            lines: vec![rect(1200, 1200, 2400, 1800, 15)],
            ..Compound::default()
        };
        // fig2dev -L svg: viewBox="1188 1188 1224 624"
        assert_eq!(compound_bound(&ctx(), &fig, true), (1188, 1188, 2412, 1812));
        assert_eq!(line_bound(&ctx(), &fig.lines[0]), Some((1200, 1200, 2400, 1800)));
    }

    #[test]
    fn arrows_extend_a_line() {
        let l = Line {
            type_: T_POLYLINE,
            thickness: 30,
            points: vec![Point::new(1200, 1200), Point::new(2400, 1200)],
            for_arrow: Some(arrow(1, 1)),
            ..Line::default()
        };
        let b = line_bound(&ctx(), &l).unwrap();
        // the arrowhead is 60 wide and its point overshoots the end
        assert_eq!((b.1, b.3), (1170, 1230));
        // the tip ends where the line did
        assert_eq!((b.0, b.2), (1200, 2400));
        let empty = Line::default();
        assert_eq!(line_bound(&ctx(), &empty), None);
    }

    #[test]
    fn calc_arrow_points_follow_the_line() {
        let a = arrow(1, 0);
        let pts = calc_arrow(&ctx(), 0, 0, 1000, 0, 15, &a);
        assert_eq!(pts.points.len(), 4);
        // tip: the end, moved by the miter overshoot (lpt, then back)
        assert_eq!(pts.points[1].y, 0);
        assert_eq!(pts.points[0].y, 30);
        assert_eq!(pts.points[2].y, -30);
        assert!(pts.fillpoints.is_empty());
        assert!(!pts.clippts.is_empty());
        // coincident points: nothing
        assert_eq!(calc_arrow(&ctx(), 5, 5, 5, 5, 15, &a), ArrowPoints::default());
        // circle: 40 points
        let c = calc_arrow(&ctx(), 0, 0, 0, 1000, 15, &arrow(5, 1));
        assert_eq!(c.points.len(), 40);
        assert_eq!(c.clippts.len(), 22);
        // half-filled spearhead has its own fill points
        let h = calc_arrow(&ctx(), 0, 0, 1000, 0, 15, &arrow(9, 0));
        assert_eq!((h.points.len(), h.fillpoints.len()), (5, 3));
    }

    fn arc(center: (f64, f64), pts: [(i32, i32); 3], direction: i32) -> Arc {
        Arc {
            type_: T_OPEN_ARC,
            thickness: 15,
            direction,
            center: DPoint {
                x: center.0,
                y: center.1,
            },
            point: pts.map(|(x, y)| Point::new(x, y)),
            ..Arc::default()
        }
    }

    #[test]
    fn arc_bound_per_quadrant() {
        let c = ctx();
        // upper half, clockwise from the left through the top to the right
        let a = arc((1500.0, 1500.0), [(1200, 1500), (1500, 1200), (1800, 1500)], 0);
        assert_eq!(arc_bound(&c, &a), (1199, 1199, 1801, 1500));
        // lower half, counter-clockwise from the left through the bottom
        let a = arc((1500.0, 1500.0), [(1200, 1500), (1500, 1800), (1800, 1500)], 1);
        assert_eq!(arc_bound(&c, &a), (1199, 1500, 1801, 1801));
        // right half, counter-clockwise from the bottom through the right
        let a = arc((1500.0, 1500.0), [(1500, 1800), (1800, 1500), (1500, 1200)], 1);
        assert_eq!(arc_bound(&c, &a), (1500, 1199, 1801, 1801));
        // left half, clockwise from the bottom through the left
        let a = arc((1500.0, 1500.0), [(1500, 1800), (1200, 1500), (1500, 1200)], 0);
        assert_eq!(arc_bound(&c, &a), (1199, 1199, 1500, 1801));
        // a quarter from axis to axis: the axes count, one unit out
        let a = arc((0.0, 0.0), [(1000, 0), (707, -707), (0, -1000)], 1);
        assert_eq!(arc_bound(&c, &a), (0, -1001, 1001, 0));
        // a pie wedge contains its centre
        let mut a = arc((0.0, 0.0), [(1000, -100), (900, -400), (700, -700)], 1);
        a.type_ = T_PIE_WEDGE_ARC;
        assert_eq!(arc_bound(&c, &a), (0, -700, 1000, 0));
    }

    #[test]
    fn ellipse_bound_scans_rows() {
        let e = Ellipse {
            thickness: 30,
            center: Point::new(6000, 3000),
            radiuses: Point::new(301, 301),
            ..Ellipse::default()
        };
        // fig2dev: a circle of radius 301, thickness 2 (30): 15 more
        let b = ellipse_bound(&e);
        assert_eq!(b, (6000 - 301 - 15, 3000 - 301 - 15, 6000 + 301 + 15, 3000 + 301 + 15));
        let flat = Ellipse {
            center: Point::new(5, 6),
            ..Ellipse::default()
        };
        assert_eq!(ellipse_bound(&flat), (5, 6, 5, 6));
        // hostile radii take the closed form, quickly
        let huge = Ellipse {
            radiuses: Point::new(i32::MAX, i32::MAX / 2),
            angle: 0.3,
            ..Ellipse::default()
        };
        let b = ellipse_bound(&huge);
        assert!(b.0 < 0 && b.2 > 0);
        let nan = Ellipse {
            radiuses: Point::new(10, 20),
            angle: f64::NAN,
            ..Ellipse::default()
        };
        let _ = ellipse_bound(&nan);
    }

    fn text(type_: i32, angle: f64, s: &str) -> Text {
        Text {
            type_,
            font: 0,
            size: 12.0,
            angle,
            flags: 4,
            height: 135.0,
            length: 1020.0,
            base_x: 1200,
            base_y: 1200,
            cstring: s.into(),
            ..Text::default()
        }
    }

    #[test]
    fn text_bounds_with_angles() {
        let c = ctx();
        let t = text(0, 0.0, "HELLO");
        // fig2dev -L svg: viewBox="1185 1050 1076 165" (length * 1.0256,
        // one THICK_SCALE around)
        assert_eq!(text_bound(&c, &t, true), (1185, 1050, 2261, 1215));
        // without text, only the height counts
        assert_eq!(text_bound(&c, &t, false), (1185, 1050, 1215, 1215));
        // descenders reach below the baseline: 0.3 of the height
        let t = text(0, 0.0, "Hello Fig");
        assert_eq!(text_bound(&c, &t, true), (1185, 1077, 2261, 1255));
        // rotated by 90°: the text goes up
        let t = text(0, std::f64::consts::FRAC_PI_2, "HELLO");
        assert_eq!(text_bound(&c, &t, true), (1050, 138, 1215, 1215));
        // special text with TeX is not included
        let mut t = text(1, 0.0, "$x$");
        t.flags = 2;
        assert_eq!(text_bound(&c, &t, true), (1185, 1077, 1215, 1255));
        // a Symbol-font descender (summation, 0xe5)
        let mut t = text(0, 0.0, "AB\u{e5}");
        t.font = 32;
        assert_eq!(text_bound(&c, &t, true).3, 1255);
    }

    #[test]
    fn arcarrow_angle_on_the_chord() {
        let c = ctx();
        let a = arrow(1, 1);
        // a large circle: the point is about the arrow's length back
        let (x, y) = compute_arcarrow_angle(&c, 0.0, 0.0, 10000.0, 0.0, 1, &a);
        assert!(x < 10000 && y > 0 && y < 300);
        // a tiny circle: the tangent
        assert_eq!(compute_arcarrow_angle(&c, 0.0, 0.0, 10.0, 0.0, 1, &a), (10, 10));
        assert_eq!(compute_arcarrow_angle(&c, 0.0, 0.0, 10.0, 0.0, 0, &a), (10, -10));
    }

    #[test]
    fn splines_old_style() {
        let c = ctx();
        let s = Spline {
            type_: 0,
            points: vec![Point::new(0, 0), Point::new(1000, 1000), Point::new(2000, 0)],
            ..Spline::default()
        };
        // approximated: from end point to end point, through the
        // quarter points of the control polygon
        assert_eq!(spline_bound(&c, &s), Some((0, 0, 2000, 750)));
        let i = Spline {
            type_: 2,
            points: vec![Point::new(0, 0), Point::new(1000, 0)],
            controls: vec![
                Control {
                    rx: 0.0,
                    ry: -400.0,
                    ..Control::default()
                },
                Control {
                    lx: 1000.0,
                    ly: -400.0,
                    ..Control::default()
                },
            ],
            ..Spline::default()
        };
        assert_eq!(spline_bound(&c, &i), Some((0, -300, 1000, 0)));
        assert_eq!(spline_bound(&c, &Spline::default()), None);
    }

    #[test]
    fn depth_filter_with_k() {
        let mut c = ctx();
        let fig = Compound {
            lines: vec![
                rect(1200, 1200, 2400, 1800, 15),
                Line {
                    depth: 40,
                    ..rect(6000, 6000, 7200, 7800, 15)
                },
            ],
            ..Compound::default()
        };
        let fig = Compound {
            lines: fig
                .lines
                .into_iter()
                .map(|l| if l.depth == 0 { Line { depth: 50, ..l } } else { l })
                .collect(),
            ..Compound::default()
        };
        c.depths = Some(crate::gendev::depth_option("+50").unwrap());
        // without -K, -D doesn't change the bounds
        assert_eq!(compound_bound(&c, &fig, true), (1188, 1188, 7212, 7812));
        c.adjust_boundingbox = true;
        assert_eq!(compound_bound(&c, &fig, true), (1188, 1188, 2412, 1812));
        assert_eq!(compound_bound(&c, &Compound::default(), true), EMPTY);
    }
}
