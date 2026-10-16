//! The figure bounds exports use, computed as fig2dev computes them.
//!
//! fig2dev's bounds are not the extent of the ink: polylines (and splines,
//! which fig2dev turns into polylines) are padded by (t + 1)/√2, arcs and
//! ellipses by (t + 1)/2, texts use the height and length stored in the
//! file. Exports sized by these bounds match fig2dev's pages, EPS bounding
//! boxes and LaTeX boxes.
//
// Ported from fig2dev bound.c (`compound_bound`, `arc_bound`,
// `ellipse_bound`, `line_bound`, `text_bound`, `arrow_bound`) and read.c
// (X-splines become polylines) (xfig licence, see LICENSE-xfig).

use crate::head_at_end;
use rustfig_geom::{Point, Rect, FIG_PER_THICKNESS};
use rustfig_model::{arc_geom, flatten_object, Arrow, Document, Justify, Object, ObjectKind, PolyKind, Style};
use std::collections::BTreeSet;
use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, TAU};

/// The bounds of every object not on a `hidden` depth, in Fig units.
/// `include_text`: whether texts count (fig2dev's drivers decide; special
/// text containing `\` or `$` never counts).
pub fn fig2dev_bounds(doc: &Document, hidden: &BTreeSet<i32>, include_text: bool) -> Rect {
    let mut r = Rect::EMPTY;
    doc.for_each_leaf(&mut |o| {
        if !hidden.contains(&o.depth()) {
            if let Some(b) = object_bounds(o, include_text) {
                r = r.union(&whole_units(b));
            }
        }
    });
    r
}

/// `r` rounded outwards to whole Fig units, as fig2dev's integer bounds are
/// (values within rounding noise of a whole unit stay there).
fn whole_units(r: Rect) -> Rect {
    if r.is_empty() {
        return r;
    }
    let down = |v: f64| {
        if (v - v.round()).abs() < 1e-6 {
            v.round()
        } else {
            v.floor()
        }
    };
    let up = |v: f64| {
        if (v - v.round()).abs() < 1e-6 {
            v.round()
        } else {
            v.ceil()
        }
    };
    Rect::from_points([
        Point::new(down(r.min.x), down(r.min.y)),
        Point::new(up(r.max.x), up(r.max.y)),
    ])
}

/// Line thickness in Fig units, as fig2dev scales it on reading.
fn thick(style: &Style) -> f64 {
    f64::from(style.thickness.max(0)) * FIG_PER_THICKNESS
}

fn object_bounds(o: &Object, include_text: bool) -> Option<Rect> {
    match &o.kind {
        ObjectKind::Text(t) => {
            let include = include_text && !(t.is_special() && (t.text.contains('\\') || t.text.contains('$')));
            Some(text_bounds(t, include))
        }
        ObjectKind::Ellipse(e) => {
            let (a, b) = (e.radii.x.abs(), e.radii.y.abs());
            let (c, s) = (e.angle.cos(), e.angle.sin());
            let hw = (a * a * c * c + b * b * s * s).sqrt();
            let hh = (a * a * s * s + b * b * c * c).sqrt();
            let half = ((thick(&e.style) + 1.0) / 2.0).floor();
            Some(Rect::from_points([e.center - Point::new(hw, hh), e.center + Point::new(hw, hh)]).expand(half))
        }
        ObjectKind::Arc(a) => {
            let mut r = Rect::from_points(a.points.iter().copied());
            if let Some(g) = rustfig_geom::arc_from_three_points(a.points[0], a.points[1], a.points[2]) {
                let g = arc_geom(a, g);
                // The axis extremes the arc passes, one unit out as fig2dev rounds.
                for k in 0..4 {
                    let ang = f64::from(k) * FRAC_PI_2;
                    if sweeps_through(g.start, g.sweep, ang) {
                        let p = g.center + Point::new(ang.cos(), ang.sin()) * (g.radius + 1.0);
                        r = r.union(&Rect::from_points([p]));
                    }
                }
                if a.pie {
                    r = r.union(&Rect::from_points([g.center]));
                }
            }
            r = with_arrows(o, r, &a.style, a.forward_arrow, a.backward_arrow);
            Some(r.expand(((thick(&a.style) + 1.0) / 2.0).floor()))
        }
        ObjectKind::Polyline(p) => {
            let mut r = Rect::from_points(p.points.iter().copied());
            if p.kind == PolyKind::Picture {
                return Some(r);
            }
            if p.kind == PolyKind::Polyline {
                r = with_arrows(o, r, &p.style, p.forward_arrow, p.backward_arrow);
            }
            Some(r.expand(((thick(&p.style) + 1.0) * FRAC_1_SQRT_2).ceil()))
        }
        ObjectKind::Spline(s) => {
            // fig2dev turns X-splines into polylines when it reads them.
            let pts: Vec<Point> = flatten_object(o).into_iter().flat_map(|p| p.pts).collect();
            let mut r = Rect::from_points(pts.iter().copied());
            if !s.closed {
                r = with_arrows(o, r, &s.style, s.forward_arrow, s.backward_arrow);
            }
            Some(r.expand(((thick(&s.style) + 1.0) * FRAC_1_SQRT_2).ceil()))
        }
        ObjectKind::Compound(_) => None,
    }
}

/// Whether the arc from `start` sweeping `sweep` (screen radians) passes `ang`.
fn sweeps_through(start: f64, sweep: f64, ang: f64) -> bool {
    if !sweep.is_finite() || !start.is_finite() {
        return false;
    }
    let d = if sweep >= 0.0 {
        (ang - start).rem_euclid(TAU)
    } else {
        (start - ang).rem_euclid(TAU)
    };
    d <= sweep.abs()
}

/// `r` grown by the object's arrowheads.
fn with_arrows(o: &Object, mut r: Rect, style: &Style, fwd: Option<Arrow>, back: Option<Arrow>) -> Rect {
    if fwd.is_none() && back.is_none() {
        return r;
    }
    let poly = matches!(&o.kind, ObjectKind::Polyline(_));
    let mut open = o.clone();
    if let ObjectKind::Arc(a) = &mut open.kind {
        a.pie = false;
    }
    let pts = match &open.kind {
        ObjectKind::Polyline(p) => p.points.clone(),
        _ => flatten_object(&open)
            .into_iter()
            .next()
            .map(|p| p.pts)
            .unwrap_or_default(),
    };
    if pts.len() < 2 {
        return r;
    }
    let line = f64::from(style.thickness);
    if let Some(a) = fwd {
        if let Some(h) = head_at_end(&pts, &crate::arrow_spec(&a), line, poly) {
            r = r.union(&Rect::from_points(h.outline));
        }
    }
    if let Some(a) = back {
        let rev: Vec<Point> = pts.iter().rev().copied().collect();
        if let Some(h) = head_at_end(&rev, &crate::arrow_spec(&a), line, poly) {
            r = r.union(&Rect::from_points(h.outline));
        }
    }
    r
}

/// fig2dev's text box: the stored length and height, generous factors,
/// descenders guessed from the characters, padded by 1/80 inch.
fn text_bounds(t: &rustfig_model::Text, include: bool) -> Rect {
    let (len, h) = (t.length.max(0.0), t.height.max(0.0));
    let w = |f: f64| if include { f } else { 0.0 };
    let (mut d, x) = match t.justify {
        Justify::Center => ([(0.0, 0.0), (0.0, -h)], [w(len / 1.95), w(-len / 1.95)]),
        Justify::Right => ([(0.0, 0.0), (0.0, -h)], [0.0, w(-len * 1.0256)]),
        Justify::Left => ([(0.0, 0.0), (0.0, -h)], [w(len * 1.0256), 0.0]),
    };
    let descend = t.text.contains([
        'g', 'j', 'p', 'q', 'y', '$', '(', ')', '{', '}', '[', ']', ',', ';', '_',
    ]) || (t.is_postscript()
        && t.font == 32
        && t.text.contains(['b', 'c', 'f', 'g', 'h', 'j', 'm', 'r', 'x', 'y', 'z']));
    if descend {
        d = [(0.0, 0.3 * h), (0.0, -0.8 * h)];
    }
    let (s, c) = (-t.angle).sin_cos();
    let rot = |x: f64, y: f64| Point::new(x * c - y * s, x * s + y * c);
    let corners = [
        rot(x[0], d[0].1),
        rot(x[1], d[0].1),
        rot(x[0], d[1].1),
        rot(x[1], d[1].1),
    ];
    Rect::from_points(corners.iter().map(|p| t.origin + *p)).expand(FIG_PER_THICKNESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustfig_model::{Arc, ColorRef, Polyline, Text};

    fn style(t: i32) -> Style {
        Style {
            thickness: t,
            ..Style::default()
        }
    }

    fn bounds(o: Object) -> Rect {
        let mut doc = Document::new();
        doc.objects.push(std::sync::Arc::new(o));
        fig2dev_bounds(&doc, &BTreeSet::new(), true)
    }

    #[test]
    fn lines_are_padded_by_t_plus_one_over_root_two() {
        let p = Polyline::new(
            PolyKind::Polyline,
            style(3),
            vec![Point::new(0.0, 0.0), Point::new(1200.0, 0.0)],
        );
        let r = bounds(Object::new(ObjectKind::Polyline(p)));
        // thickness 3 = 45 units; ceil(46 / √2) = 33.
        assert_eq!(r.min, Point::new(-33.0, -33.0));
        assert_eq!(r.max, Point::new(1233.0, 33.0));
    }

    #[test]
    fn arcs_reach_their_axis_extremes() {
        let a = Arc::through(
            style(1),
            Point::new(-600.0, 0.0),
            Point::new(0.0, -600.0),
            Point::new(600.0, 0.0),
            false,
        )
        .unwrap();
        let r = bounds(Object::new(ObjectKind::Arc(a)));
        // Top at -601, padded by (15 + 1) / 2 = 8.
        assert_eq!(r.min.y, -609.0);
        assert_eq!(r.max.y, 8.0);
        assert_eq!(r.min.x, -609.0);
    }

    #[test]
    fn text_uses_the_stored_size() {
        let t = Text {
            justify: Justify::Left,
            color: ColorRef::BLACK,
            depth: 50,
            pen_style: -1,
            font: 0,
            size: 12.0,
            angle: 0.0,
            flags: 4,
            height: 135.0,
            length: 1000.0,
            origin: Point::new(0.0, 0.0),
            text: "AB".into(),
        };
        let r = bounds(Object::new(ObjectKind::Text(t.clone())));
        assert_eq!(r.max.x, 1041.0, "{r:?}"); // ceil(1025.6 + 15)
        assert!((r.min.y - (-135.0 - 15.0)).abs() < 1e-9, "{r:?}");
        let special = Text {
            flags: 2,
            text: "$x$".into(),
            ..t
        };
        let r = bounds(Object::new(ObjectKind::Text(special)));
        assert!((r.max.x - 15.0).abs() < 1e-9, "{r:?}");
    }
}
