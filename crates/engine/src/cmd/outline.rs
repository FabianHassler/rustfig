//! Cutting any drawn outline (plan D15): the machinery behind Chop, Trim,
//! Split at point and Cut all.
//!
//! xfig's chop (`e_chop.c`) knows polylines, polygons, arcs and circles.
//! RustFig also cuts boxes, arc-boxes, ellipses, splines and compounds. The
//! intersections of those come from the outline the renderer draws (the
//! flattened path: `rustfig_model::flatten_object`), exact for straight
//! segments and within the flattening tolerance (5 Fig units, a chord, so the
//! error of a point is far below one unit) for curves.
//!
//! The pieces of a flattened outline become new objects:
//!
//! - boxes, arc-boxes and polygons give polylines (open pieces) or polygons
//!   (closed pieces, as xfig's chop does);
//! - splines and ellipses give X-splines. Fig has no elliptical arc and no
//!   way to keep a part of a spline, so a piece is *approximated*: its end
//!   points are the cut points themselves (shape factor 0, the curve passes
//!   through them), the points in between are taken from the flattened
//!   original every 1/48 of the piece's length (at least every 0.1 inch)
//!   with shape factor -1 (the spline interpolates them), and a vertex of the
//!   original where the outline turns by more than 0.6 radian stays a sharp
//!   control point (shape 0). The curve therefore follows the original within
//!   a few Fig units on smooth outlines;
//! - arrowheads stay only at the original ends (a piece keeps the forward
//!   arrow if it ends where the original ended, the backward arrow if it
//!   starts where it started; closed pieces have none).

use super::chop::{
    arc_geometry, arc_pieces_at, bad, centroid, chop_arc, chop_circle, chop_polyline, object_segments, outline_cuts,
    round, valid_piece, Isect,
};
use crate::EngineError;
use rustfig_geom::{polygon_area, Point};
use rustfig_model::{
    flatten_object, Arrow, Compound, Document, Object, ObjectKind, PolyKind, Polyline, Spline, SplineKind, Style,
};

/// How closed figures come out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    /// Chop: closed figures give closed pieces (wedges, pie arcs).
    Chop,
    /// Trim, Split and Cut all: pieces are open outlines.
    Open,
}

/// What a cut does to one object.
pub(super) enum Cut {
    /// Nothing intersects (or nothing can be cut).
    Unchanged,
    /// The object is replaced by these.
    Pieces(Vec<ObjectKind>),
    /// A compound whose members were replaced (the number of members cut).
    Compound(Compound, usize),
}

/// Most points of one outline a cut works on.
const MAX_OUTLINE_POINTS: usize = 400_000;
/// Most control points of one spline piece.
const MAX_PIECE_CONTROLS: usize = 64;

/// A flattened outline.
pub(super) struct Outline {
    pub pts: Vec<Point>,
    pub closed: bool,
}

impl Outline {
    pub fn segments(&self) -> Vec<(Point, Point)> {
        let mut out: Vec<(Point, Point)> = self
            .pts
            .windows(2)
            .filter_map(|w| Some((*w.first()?, *w.get(1)?)))
            .collect();
        if let (true, Some(f), Some(l)) = (self.closed, self.pts.first(), self.pts.last()) {
            if self.pts.len() > 2 {
                out.push((*l, *f));
            }
        }
        out
    }
}

/// The flattened outline of a polyline, ellipse or spline (not an arc).
pub(super) fn outline_of(kind: &ObjectKind) -> Result<Outline, EngineError> {
    let o = Object::new(kind.clone());
    let Some(path) = flatten_object(&o).into_iter().next() else {
        return Err(bad("this object has no outline to cut"));
    };
    let mut pts = path.pts;
    if pts.len() > MAX_OUTLINE_POINTS {
        return Err(bad("the outline has too many points to cut"));
    }
    if path.closed {
        if let (Some(f), Some(l)) = (pts.first().copied(), pts.last().copied()) {
            if pts.len() > 1 && f.dist(l) < 1e-9 {
                pts.pop();
            }
        }
    }
    pts.dedup_by(|a, b| a.dist(*b) < 1e-9);
    if pts.len() < 2 {
        return Err(bad("this object has no outline to cut"));
    }
    Ok(Outline {
        pts,
        closed: path.closed,
    })
}

/// Split an outline at the cuts into open point lists. A closed outline is
/// split into one list per pair of consecutive cuts (the last wrapping
/// around the start); with a single cut the whole loop is one list.
pub(super) fn split_outline(o: &Outline, cuts: &[Isect]) -> Vec<Vec<Point>> {
    let n = o.pts.len();
    let nseg = if o.closed { n } else { n.saturating_sub(1) };
    let mut per: Vec<Vec<(f64, Point)>> = vec![Vec::new(); nseg];
    for c in cuts {
        let (Some(start), Some(list)) = (o.pts.get(c.seg_idx), per.get_mut(c.seg_idx)) else {
            continue;
        };
        list.push((start.dist(c.point), c.point));
    }
    let mut ring: Vec<(Point, bool)> = Vec::with_capacity(n + cuts.len());
    for (i, list) in per.iter_mut().enumerate() {
        if let Some(p) = o.pts.get(i) {
            ring.push((*p, false));
        }
        list.sort_by(|a, b| a.0.total_cmp(&b.0));
        ring.extend(list.iter().map(|(_, p)| (*p, true)));
    }
    if !o.closed {
        if let Some(l) = o.pts.last() {
            ring.push((*l, false));
        }
    }
    let finish = |mut cur: Vec<Point>, pieces: &mut Vec<Vec<Point>>| {
        cur.dedup_by(|a, b| a.dist(*b) < 1e-6);
        if cur.len() >= 2 {
            pieces.push(cur);
        }
    };
    let mut pieces = Vec::new();
    if o.closed {
        let Some(first) = ring.iter().position(|(_, c)| *c) else {
            return pieces;
        };
        let len = ring.len();
        let mut cur = vec![ring.get(first).map(|r| r.0).unwrap_or_default()];
        for k in 1..=len {
            let Some((p, is_cut)) = ring.get((first + k) % len).copied() else {
                continue;
            };
            cur.push(p);
            if is_cut {
                finish(std::mem::replace(&mut cur, vec![p]), &mut pieces);
            }
        }
    } else {
        let mut cur = Vec::new();
        for (p, is_cut) in ring {
            cur.push(p);
            if is_cut {
                finish(std::mem::replace(&mut cur, vec![p]), &mut pieces);
            }
        }
        finish(cur, &mut pieces);
    }
    pieces
}

/// The forward and backward arrow a piece keeps: those whose end is still
/// where the original's was.
fn end_arrows(
    orig: (Point, Point),
    piece: (Point, Point),
    forward: &Option<Arrow>,
    backward: &Option<Arrow>,
) -> (Option<Arrow>, Option<Arrow>) {
    (
        if piece.1.dist(orig.1) < 1e-6 { *forward } else { None },
        if piece.0.dist(orig.0) < 1e-6 { *backward } else { None },
    )
}

/// How an X-spline piece is closed.
#[derive(Clone, Copy)]
enum Closing {
    Open,
    /// Straight chord from the last point back to the first.
    Chord,
    /// Straight lines to this point (the wedge's tip) and back.
    Focus(Point),
}

/// The angle by which the path turns at `b`.
fn turn(a: Point, b: Point, c: Point) -> f64 {
    let (u, v) = (b - a, c - b);
    let cross = u.x * v.y - u.y * v.x;
    let dot = u.dot(v);
    if cross == 0.0 && dot == 0.0 {
        return 0.0;
    }
    cross.atan2(dot).abs()
}

/// An approximating X-spline through the flattened piece `pts`; see the
/// module doc for the approximation. `None` when the piece is too small.
fn spline_piece(style: Style, pts: &[Point], closing: Closing) -> Option<Spline> {
    let (first, last) = (pts.first().copied()?, pts.last().copied()?);
    let total: f64 = pts.windows(2).map(|w| w[0].dist(w[1])).sum();
    if !total.is_finite() || total < 1e-6 {
        return None;
    }
    let step = (total / 48.0).max(120.0);
    let mut ctrl: Vec<(Point, f64)> = vec![(first, 0.0)];
    let (mut acc, mut done) = (0.0, 0.0);
    for i in 1..pts.len().saturating_sub(1) {
        let (Some(a), Some(b), Some(c)) = (pts.get(i - 1), pts.get(i), pts.get(i + 1)) else {
            continue;
        };
        let d = a.dist(*b);
        acc += d;
        done += d;
        if ctrl.len() >= MAX_PIECE_CONTROLS {
            continue;
        }
        if turn(*a, *b, *c) > 0.6 {
            ctrl.push((*b, 0.0));
            acc = 0.0;
        } else if acc >= step && total - done >= step * 0.5 {
            ctrl.push((*b, -1.0));
            acc = 0.0;
        }
    }
    ctrl.push((last, 0.0));
    let closed = !matches!(closing, Closing::Open);
    match closing {
        Closing::Open => {}
        Closing::Chord => ctrl.push((first.lerp(last, 0.5), 0.0)),
        Closing::Focus(f) => ctrl.push((f, 0.0)),
    }
    if closed && ctrl.len() < 3 {
        return None;
    }
    Some(Spline {
        kind: SplineKind::X,
        closed,
        style,
        forward_arrow: None,
        backward_arrow: None,
        points: ctrl.iter().map(|c| c.0).collect(),
        shapes: ctrl.iter().map(|c| c.1).collect(),
    })
}

/// Turn the point lists of an outline's pieces into objects.
fn convert_pieces(
    kind: &ObjectKind,
    out: &Outline,
    pieces: Vec<Vec<Point>>,
    mode: Mode,
    pie_arc: bool,
) -> Vec<ObjectKind> {
    let wedge = mode == Mode::Chop && out.closed;
    let mut res = Vec::new();
    match kind {
        ObjectKind::Polyline(l) => {
            let focus = centroid(&l.points);
            let orig = (
                l.points.first().copied().unwrap_or_default(),
                l.points.last().copied().unwrap_or_default(),
            );
            for mut pts in pieces {
                let mut p = l.clone();
                p.picture = None;
                if wedge {
                    if pie_arc {
                        pts.push(focus);
                    }
                    if !valid_piece(&mut pts, true) || polygon_area(&pts) <= 1e-6 {
                        continue;
                    }
                    p.kind = PolyKind::Polygon;
                    p.forward_arrow = None;
                    p.backward_arrow = None;
                } else {
                    let ends = (
                        pts.first().copied().unwrap_or_default(),
                        pts.last().copied().unwrap_or_default(),
                    );
                    let (f, b) = if l.kind == PolyKind::Polyline {
                        end_arrows(orig, ends, &l.forward_arrow, &l.backward_arrow)
                    } else {
                        (None, None)
                    };
                    p.kind = PolyKind::Polyline;
                    p.forward_arrow = f;
                    p.backward_arrow = b;
                }
                p.points = pts;
                res.push(ObjectKind::Polyline(p));
            }
        }
        ObjectKind::Spline(s) => {
            let focus = centroid(&out.pts);
            let orig = (
                s.points.first().copied().unwrap_or_default(),
                s.points.last().copied().unwrap_or_default(),
            );
            for pts in pieces {
                let closing = match (wedge, pie_arc) {
                    (false, _) => Closing::Open,
                    (true, true) => Closing::Focus(focus),
                    (true, false) => Closing::Chord,
                };
                let ends = (
                    pts.first().copied().unwrap_or_default(),
                    pts.last().copied().unwrap_or_default(),
                );
                if let Some(mut p) = spline_piece(s.style, &pts, closing) {
                    if !p.closed && !s.closed {
                        let (f, b) = end_arrows(orig, ends, &s.forward_arrow, &s.backward_arrow);
                        p.forward_arrow = f;
                        p.backward_arrow = b;
                    }
                    res.push(ObjectKind::Spline(p));
                }
            }
        }
        ObjectKind::Ellipse(e) => {
            for pts in pieces {
                let closing = if wedge { Closing::Focus(e.center) } else { Closing::Open };
                if let Some(p) = spline_piece(e.style, &pts, closing) {
                    res.push(ObjectKind::Spline(p));
                }
            }
        }
        _ => {}
    }
    res
}

/// Cut a polyline, box, arc-box, polygon, ellipse, spline or arc at its
/// intersections with the axe objects. An empty result means nothing was
/// cut. In [`Mode::Chop`] a closed figure needs two intersections (an
/// error, as in xfig); in [`Mode::Open`] it is left alone.
pub(super) fn cut_kind(
    kind: &ObjectKind,
    axes: &[Object],
    mode: Mode,
    pie_arc: bool,
) -> Result<Vec<ObjectKind>, EngineError> {
    let pieces = match kind {
        ObjectKind::Arc(a) => chop_arc(a, axes)?,
        ObjectKind::Ellipse(e) if e.kind.is_circle() => chop_circle(e, axes, mode == Mode::Chop)?,
        ObjectKind::Polyline(l) if l.kind == PolyKind::Picture => {
            return Err(bad("a picture cannot be cut"));
        }
        ObjectKind::Polyline(l) if mode == Mode::Chop && matches!(l.kind, PolyKind::Polyline | PolyKind::Polygon) => {
            chop_polyline(l, axes, pie_arc)?
        }
        ObjectKind::Polyline(_) | ObjectKind::Ellipse(_) | ObjectKind::Spline(_) => {
            let out = outline_of(kind)?;
            let cuts = outline_cuts(&out.segments(), axes)?;
            if cuts.is_empty() {
                return Ok(Vec::new());
            }
            if out.closed && cuts.len() < 2 {
                return if mode == Mode::Chop {
                    Err(bad("Closed figures require two or more intersects."))
                } else {
                    Ok(Vec::new())
                };
            }
            convert_pieces(kind, &out, split_outline(&out, &cuts), mode, pie_arc)
        }
        _ => return Err(bad("only polylines, boxes, arcs, ellipses and splines can be cut")),
    };
    Ok(match (kind, mode) {
        (ObjectKind::Polyline(l), _) => fix_poly_arrows(pieces, l),
        _ => pieces,
    })
}

/// Arrowheads of chopped polyline pieces stay only at the original ends.
fn fix_poly_arrows(pieces: Vec<ObjectKind>, l: &Polyline) -> Vec<ObjectKind> {
    let orig = (
        l.points.first().copied().unwrap_or_default(),
        l.points.last().copied().unwrap_or_default(),
    );
    pieces
        .into_iter()
        .map(|k| match k {
            ObjectKind::Polyline(mut p) => {
                if p.kind == PolyKind::Polyline && l.kind == PolyKind::Polyline {
                    let ends = (
                        p.points.first().copied().unwrap_or_default(),
                        p.points.last().copied().unwrap_or_default(),
                    );
                    let (f, b) = end_arrows(orig, ends, &l.forward_arrow, &l.backward_arrow);
                    p.forward_arrow = f;
                    p.backward_arrow = b;
                } else {
                    p.forward_arrow = None;
                    p.backward_arrow = None;
                }
                ObjectKind::Polyline(p)
            }
            other => other,
        })
        .collect()
}

/// The closest point of an open or closed path to `click`: the segment
/// index and the (rounded) point on it.
pub(super) fn nearest_on_path(pts: &[Point], closed: bool, click: Point) -> Option<(usize, Point)> {
    let n = pts.len();
    let nseg = if closed { n } else { n.saturating_sub(1) };
    let mut best: Option<(f64, usize, Point)> = None;
    for i in 0..nseg {
        let (Some(a), Some(b)) = (pts.get(i), pts.get((i + 1) % n)) else {
            continue;
        };
        let d = *b - *a;
        let len2 = d.dot(d);
        let t = if len2 > 0.0 {
            ((click - *a).dot(d) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let q = *a + d * t;
        let dist = q.dist(click);
        if best.is_none_or(|(bd, _, _)| dist < bd) {
            best = Some((dist, i, q));
        }
    }
    best.map(|(_, i, q)| (i, round(q)))
}

/// Cut `kind` at the single point of its outline nearest to `click`
/// (Split at point). The pieces are open outlines; a closed figure becomes
/// one open piece starting and ending at the cut.
pub(super) fn split_kind(kind: &ObjectKind, click: Point) -> Result<Vec<ObjectKind>, EngineError> {
    match kind {
        ObjectKind::Arc(a) => {
            let g = arc_geometry(a).ok_or_else(|| bad("the arc is degenerate"))?;
            let path = flatten_object(&Object::new(kind.clone()))
                .into_iter()
                .next()
                .map(|p| p.pts)
                .unwrap_or_default();
            let (_, q) = nearest_on_path(&path, false, click).ok_or_else(|| bad("the arc has no outline"))?;
            let dir = q - g.center;
            let len = dir.len();
            if !len.is_finite() || len <= 0.0 {
                return Err(bad("cannot split the arc here"));
            }
            let cut = round(g.center + dir * (g.radius / len));
            let pieces = arc_pieces_at(a, &g, vec![cut]);
            if pieces.len() < 2 {
                return Err(bad("cannot split at an end point"));
            }
            Ok(pieces)
        }
        ObjectKind::Ellipse(_) => Err(bad(
            "circles and ellipses need two cut points: chop or trim them against other objects",
        )),
        ObjectKind::Polyline(l) if l.kind == PolyKind::Picture => Err(bad("a picture cannot be split")),
        ObjectKind::Polyline(_) | ObjectKind::Spline(_) => {
            let out = outline_of(kind)?;
            let (seg_idx, point) = nearest_on_path(&out.pts, out.closed, click).ok_or_else(|| bad("no outline"))?;
            let pieces = split_outline(&out, &[Isect { point, seg_idx }]);
            if !out.closed && pieces.len() < 2 {
                return Err(bad("cannot split at an end point"));
            }
            let res = convert_pieces(kind, &out, pieces, Mode::Open, false);
            if res.is_empty() {
                return Err(bad("cannot split here"));
            }
            Ok(res)
        }
        _ => Err(bad("only polylines, boxes, arcs and splines can be split")),
    }
}

/// Distance from `p` to the drawn outline of `kind`.
pub(super) fn dist_to_kind(kind: &ObjectKind, p: Point) -> f64 {
    flatten_object(&Object::new(kind.clone()))
        .iter()
        .map(|path| rustfig_geom::dist_to_polyline(p, &path.pts, path.closed))
        .fold(f64::INFINITY, f64::min)
}

/// Cut one object (a compound cuts every member) against the axe objects.
pub(super) fn cut_object(
    doc: &mut Document,
    log: &Object,
    axes: &[Object],
    mode: Mode,
    pie_arc: bool,
) -> Result<Cut, EngineError> {
    match &log.kind {
        ObjectKind::Compound(c) => Ok(match cut_compound(doc, c, axes, mode, pie_arc, 0) {
            Some((c, n)) => Cut::Compound(c, n),
            None => Cut::Unchanged,
        }),
        ObjectKind::Text(_) => Err(bad("texts cannot be cut")),
        kind => {
            let pieces = cut_kind(kind, axes, mode, pie_arc)?;
            Ok(if pieces.is_empty() {
                Cut::Unchanged
            } else {
                Cut::Pieces(pieces)
            })
        }
    }
}

/// A compound with every cuttable member replaced by its pieces; `None`
/// when no member was cut. Members that cannot be cut (texts, a polygon
/// with one intersection) stay as they are.
fn cut_compound(
    doc: &mut Document,
    c: &Compound,
    axes: &[Object],
    mode: Mode,
    pie_arc: bool,
    depth: usize,
) -> Option<(Compound, usize)> {
    if depth > 64 {
        return None;
    }
    let mut members = Vec::with_capacity(c.objects.len());
    let mut changed = 0;
    for m in &c.objects {
        match &m.kind {
            ObjectKind::Compound(inner) => match cut_compound(doc, inner, axes, mode, pie_arc, depth + 1) {
                Some((nc, n)) => {
                    let mut o = m.clone();
                    o.kind = ObjectKind::Compound(nc);
                    members.push(o);
                    changed += n;
                }
                None => members.push(m.clone()),
            },
            ObjectKind::Text(_) => members.push(m.clone()),
            kind => match cut_kind(kind, axes, mode, pie_arc) {
                Ok(pieces) if !pieces.is_empty() => {
                    changed += 1;
                    for k in pieces {
                        let mut o = Object::new(k);
                        o.comments = m.comments.clone();
                        o.id = doc.alloc_id();
                        members.push(o);
                    }
                }
                // Nothing to cut here, or a member that cannot be (xfig
                // reports such a chop to the user; inside a compound the
                // member just stays).
                _ => members.push(m.clone()),
            },
        }
    }
    if changed == 0 {
        None
    } else {
        Some((Compound { objects: members }, changed))
    }
}

/// Segments of every object's drawn outline, for rays (Extend).
pub(super) fn boundary_segments(objs: &[Object]) -> Vec<(Point, Point)> {
    objs.iter().flat_map(object_segments).collect()
}
