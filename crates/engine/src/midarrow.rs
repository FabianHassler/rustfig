//! Arrowheads partway along a line (plan Q7).
//!
//! Fig has arrowheads only at the ends of a line, so a line with a head in
//! it is several lines: [`split`] cuts a polyline, an open spline or an arc
//! at the given positions into pieces glued into a compound. Each piece but
//! the last ends in a real Fig arrowhead (Arrow mode and the edit panel work
//! on it, in xfig too).
//!
//! The pieces look like one line with a head in it. The head is centered on
//! the position (its tip half a head length beyond it); the next piece
//! starts where the head's own line ends (under the head, not at its tip),
//! so the line runs through the head with no seam, cap or gap. The pieces
//! are stored last first, so each head is drawn over the start of the next
//! piece (xfig and fig2dev draw in that order), which keeps a hollow head's
//! fill clean.

use crate::EngineError;
use rustfig_geom::{arc_from_three_points, arrowhead, ArcGeom, Point};
use rustfig_model::{arc_geom, Arc, Arrow, Compound, Object, ObjectKind, PolyKind, Polyline, Style};

/// The most heads one line takes.
pub const MAX_MID_ARROWS: usize = 64;

/// A curve to cut: its length, points by arc length, and the points of a
/// piece between two arc lengths.
enum Curve {
    Poly { pts: Vec<Point>, cum: Vec<f64> },
    Arc(ArcGeom),
}

fn round(p: Point) -> Point {
    Point::new(p.x.round(), p.y.round())
}

impl Curve {
    fn poly(points: &[Point]) -> Option<Curve> {
        let mut pts: Vec<Point> = Vec::with_capacity(points.len());
        for &p in points {
            let p = round(p);
            if !p.is_finite() {
                return None;
            }
            if pts.last().is_none_or(|l| l.dist(p) > 0.5) {
                pts.push(p);
            }
        }
        if pts.len() < 2 {
            return None;
        }
        let mut cum = vec![0.0];
        for w in pts.windows(2) {
            if let [a, b] = w {
                cum.push(cum.last().copied().unwrap_or(0.0) + a.dist(*b));
            }
        }
        Some(Curve::Poly { pts, cum })
    }

    fn length(&self) -> f64 {
        match self {
            Curve::Poly { cum, .. } => cum.last().copied().unwrap_or(0.0),
            Curve::Arc(g) => g.radius * g.sweep.abs(),
        }
    }

    fn at(&self, s: f64) -> Point {
        match self {
            Curve::Poly { pts, cum } => {
                let s = s.clamp(0.0, self.length());
                // The segment holding s.
                let i = cum
                    .partition_point(|c| *c < s)
                    .clamp(1, pts.len().saturating_sub(1).max(1));
                let (Some(a), Some(b), Some(ca), Some(cb)) = (pts.get(i - 1), pts.get(i), cum.get(i - 1), cum.get(i))
                else {
                    return pts.first().copied().unwrap_or(Point::ZERO);
                };
                let t = if cb > ca { (s - ca) / (cb - ca) } else { 0.0 };
                Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
            }
            Curve::Arc(g) => {
                let len = self.length();
                let t = if len > 0.0 { s.clamp(0.0, len) / len } else { 0.0 };
                let a = g.start + g.sweep * t;
                Point::new(g.center.x + g.radius * a.cos(), g.center.y + g.radius * a.sin())
            }
        }
    }

    /// The points of the piece from arc length `a` to `b`: the vertices in
    /// between for a polyline, start, middle and end for an arc.
    fn slice(&self, a: f64, b: f64) -> Vec<Point> {
        match self {
            Curve::Poly { pts, cum } => {
                let mut out = vec![round(self.at(a))];
                for (p, c) in pts.iter().zip(cum) {
                    if *c > a + 0.5 && *c < b - 0.5 {
                        out.push(*p);
                    }
                }
                let end = round(self.at(b));
                if out.last().is_none_or(|l| l.dist(end) > 0.5) {
                    out.push(end);
                }
                out
            }
            Curve::Arc(_) => vec![round(self.at(a)), round(self.at((a + b) / 2.0)), round(self.at(b))],
        }
    }
}

/// The part of `o` that is cut: its curve, style and own arrows.
struct Source {
    curve: Curve,
    style: Style,
    forward: Option<Arrow>,
    backward: Option<Arrow>,
    arc: bool,
    template: Option<Polyline>,
}

fn source(o: &Object) -> Result<Source, EngineError> {
    let bad = || EngineError::bad_params("the line is too short or has no length");
    match &o.kind {
        ObjectKind::Polyline(p) if p.kind == PolyKind::Polyline => Ok(Source {
            curve: Curve::poly(&p.points).ok_or_else(bad)?,
            style: p.style,
            forward: p.forward_arrow,
            backward: p.backward_arrow,
            arc: false,
            template: Some(p.clone()),
        }),
        ObjectKind::Spline(s) if !s.closed => {
            // A spline is cut where it is drawn: the pieces are polylines
            // along its curve (a Fig spline cannot be cut in two).
            let flat = rustfig_model::flatten_object(o);
            let pts = flat.first().map(|f| f.pts.as_slice()).unwrap_or(&[]);
            Ok(Source {
                curve: Curve::poly(pts).ok_or_else(bad)?,
                style: s.style,
                forward: s.forward_arrow,
                backward: s.backward_arrow,
                arc: false,
                template: None,
            })
        }
        ObjectKind::Arc(a) if !a.pie => {
            let g = arc_from_three_points(a.points[0], a.points[1], a.points[2]).ok_or_else(bad)?;
            Ok(Source {
                curve: Curve::Arc(arc_geom(a, g)),
                style: a.style,
                forward: a.forward_arrow,
                backward: a.backward_arrow,
                arc: true,
                template: None,
            })
        }
        _ => Err(EngineError::bad_params(
            "arrowheads partway along a line need a polyline, an open spline or an arc",
        )),
    }
}

/// The positions as fractions of the line's length, checked: finite, inside
/// (0, 1), ascending.
pub fn check_fractions(f: &[f64]) -> Result<(), EngineError> {
    if f.is_empty() || f.len() > MAX_MID_ARROWS {
        return Err(EngineError::bad_params(format!(
            "mid_arrows takes 1 to {MAX_MID_ARROWS} fractions of the line's length"
        )));
    }
    if f.iter().any(|v| !v.is_finite() || *v <= 0.0 || *v >= 1.0) || f.windows(2).any(|w| w[0] >= w[1]) {
        return Err(EngineError::bad_params(
            "mid_arrows: fractions strictly between 0 and 1, in ascending order",
        ));
    }
    Ok(())
}

/// `o` (a polyline, open spline or arc) cut at `fractions` of its length
/// into pieces glued into a compound, each piece but the last ending in
/// `arrow`; its own arrows stay at its ends. A head whose position is too
/// near an end (use the end arrow) or to the next head is an error.
pub fn split(o: &Object, fractions: &[f64], arrow: &Arrow) -> Result<Object, EngineError> {
    check_fractions(fractions)?;
    let src = source(o)?;
    let total = src.curve.length();
    let len = arrow.height;
    if !(len.is_finite() && len > 0.0 && arrow.width.is_finite()) {
        return Err(EngineError::bad_params("the arrow needs a length and a width"));
    }
    let spec = rustfig_render::arrow_spec(arrow);
    let pos: Vec<f64> = fractions.iter().map(|f| f * total).collect();
    let near = |what: &str| {
        EngineError::bad_params(format!(
            "mid_arrows: {what} (the arrowhead is {len:.0} units long, the line {total:.0}); use a smaller arrow or other positions"
        ))
    };
    if pos.first().is_some_and(|s| *s - len / 2.0 < 0.0) || pos.last().is_some_and(|s| *s + len / 2.0 > total) {
        return Err(near("a head would reach past an end of the line"));
    }
    if pos.windows(2).any(|w| w[1] - w[0] < len) {
        return Err(near("two heads would overlap"));
    }
    // The pieces' ranges: each ends half a head beyond its position and the
    // next starts where the head's line ends.
    let mut ranges: Vec<(f64, f64)> = Vec::new();
    let mut start = 0.0;
    for s in &pos {
        let end = s + len / 2.0;
        let pts = src.curve.slice(start, end);
        let (Some(tip), Some(from)) = (
            pts.last().copied(),
            pts.len().checked_sub(2).and_then(|i| pts.get(i)).copied(),
        ) else {
            return Err(near("a piece has no length"));
        };
        // (For an arc the head leans on the chord, close enough to find
        // where its line ends.)
        let from = if src.arc { src.curve.at(end - len / 2.0) } else { from };
        let head = arrowhead(tip, from, &spec, f64::from(src.style.thickness))
            .ok_or_else(|| EngineError::bad_params("the arrow needs a width"))?;
        let back = tip.dist(head.line_end).min(len * 1.5);
        let next = (end - back).max(start);
        ranges.push((start, end));
        start = next;
    }
    ranges.push((start, total));
    let n = ranges.len();
    let mut pieces: Vec<Object> = Vec::with_capacity(n);
    for (i, (a, b)) in ranges.iter().enumerate() {
        let pts = src.curve.slice(*a, *b);
        if pts.len() < 2 {
            return Err(near("a piece has no length"));
        }
        let fwd = if i + 1 < n { Some(*arrow) } else { src.forward };
        let back = if i == 0 { src.backward } else { None };
        let kind = if src.arc {
            let [p0, p1, p2] = pts.as_slice() else {
                return Err(near("a piece has no length"));
            };
            let mut arc = Arc::through(src.style, *p0, *p1, *p2, false)
                .ok_or_else(|| near("a piece of the arc has no length"))?;
            arc.forward_arrow = fwd;
            arc.backward_arrow = back;
            ObjectKind::Arc(arc)
        } else {
            let mut pl = match &src.template {
                Some(t) => Polyline {
                    points: pts,
                    ..t.clone()
                },
                None => Polyline::new(PolyKind::Polyline, src.style, pts),
            };
            pl.forward_arrow = fwd;
            pl.backward_arrow = back;
            ObjectKind::Polyline(pl)
        };
        pieces.push(Object::new(kind));
    }
    // Last piece first: each head is drawn over the start of the next piece.
    pieces.reverse();
    let mut c = Object::new(ObjectKind::Compound(Compound { objects: pieces }));
    c.comments = o.comments.clone();
    Ok(c)
}
