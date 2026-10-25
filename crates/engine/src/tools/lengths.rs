//! The lengths shown while drawing: the message panel's length, dx, dy and
//! angle of the segment being drawn, and with `show_lengths` the triangle
//! with its two sides labelled on the canvas.
//
// Ported from xfig src/w_msgpanel.c (`length_msg`, `altlength_msg`,
// `boxsize_msg`, `boxsize_scale_msg`, `length_msg2`, `arc_msg`) (xfig
// licence, see LICENSE-xfig).

use super::{Overlay, ToolKind, ToolState};
use crate::Engine;
use rustfig_geom::Point;
use rustfig_model::{Arc, ObjectKind, PolyKind};

/// What the message panel says (xfig's `MSG_*`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Msg {
    PntsLength(usize),
    Dist,
    Length,
    Radius,
    Diameter,
    Radius2,
    Box,
}

struct Source {
    msg: Msg,
    fix: Point,
    cur: Point,
}

impl Engine {
    /// The segment being drawn, as xfig measures it from the fixed point.
    fn length_source(&self) -> Option<Source> {
        match &self.tool_state {
            ToolState::Points {
                pts,
                cursor,
                freehand: false,
                center: None,
                ..
            } => Some(Source {
                msg: Msg::PntsLength(pts.len()),
                fix: *pts.last()?,
                cur: *cursor,
            }),
            ToolState::Drag {
                start, cursor, kind, ..
            } => {
                let msg = match kind {
                    ToolKind::Box | ToolKind::ArcBox | ToolKind::Picture | ToolKind::Plot => Msg::Box,
                    ToolKind::CircleRadius => Msg::Radius,
                    ToolKind::RegularPolygon => Msg::Length,
                    ToolKind::CircleDiameter | ToolKind::EllipseDiameter => Msg::Diameter,
                    ToolKind::EllipseRadii => Msg::Radius2,
                    _ => return None,
                };
                Some(Source {
                    msg,
                    fix: *start,
                    cur: *cursor,
                })
            }
            ToolState::MoveObjs { start, cursor, .. } => Some(Source {
                msg: Msg::Dist,
                fix: *start,
                cur: *cursor,
            }),
            _ => None,
        }
    }

    /// Put the length of the segment being drawn in the message panel (only
    /// while one is being drawn; other messages stay).
    pub(crate) fn update_length_message(&mut self) {
        let fmt = crate::cmd::dimension_formatter(self, self.indicator.dimline.precision);
        if let Some(text) = self.point_drag_message(&fmt).or_else(|| self.scale_message(&fmt)) {
            self.set_status(text);
            return;
        }
        let Some(s) = self.length_source() else { return };
        let (dx, dy) = (s.cur.x - s.fix.x, s.cur.y - s.fix.y);
        let len = dx.hypot(dy);
        let ang = if dx != 0.0 || dy != 0.0 {
            -dy.atan2(dx).to_degrees() + 0.0
        } else {
            0.0
        };
        let (l, x, y) = (fmt(len), fmt(dx), fmt(dy));
        let text = match s.msg {
            Msg::PntsLength(n) => format!(
                "{n} point{}, Length = {l}, dx = {x}, dy = {y} ({ang:.1} deg)",
                if n == 1 { "" } else { "s" }
            ),
            Msg::Dist => format!("Distance = {l}, dx = {x}, dy = {y} ({ang:.1}) deg"),
            Msg::Radius => format!("Radius = {l}, dx = {x}, dy = {y}"),
            Msg::Diameter => format!("Diameter = {l}, dx = {x}, dy = {y}"),
            Msg::Radius2 => format!("Radius1 = {x}, Radius2 = {y}"),
            Msg::Length => format!("Length = {l}, dx = {x}, dy = {y} ({ang:.1}) deg"),
            // `boxsize_msg`: the factor compares with `from_x`/`from_y`,
            // which xfig leaves at 0 unless a move-point or scale mode set
            // them.
            Msg::Box => {
                let (od_x, od_y) = (s.fix.x.abs(), s.fix.y.abs());
                let (nd_x, nd_y) = (dx.abs(), dy.abs());
                let factor = if od_x != 0.0 && nd_x != 0.0 {
                    nd_x / od_x
                } else if od_y != 0.0 && nd_y != 0.0 {
                    nd_y / od_y
                } else {
                    0.0
                };
                format!(
                    "Width = {}, Height = {}, Factor = {factor:.3}",
                    fmt(dx.abs()),
                    fmt(dy.abs())
                )
            }
        };
        self.set_status(text);
    }

    /// With Show Lengths on: the dx / dy triangle of the segment being drawn
    /// and its two sides' lengths in the ruler units (`altlength_msg`).
    pub(crate) fn length_overlay(&self, ov: &mut Overlay) {
        if !self.show_lengths {
            return;
        }
        let Some(s) = self.length_source() else { return };
        let (dx, dy) = (s.cur.x - s.fix.x, s.cur.y - s.fix.y);
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        // One screen pixel is 15 / zoom Fig units; the sides sit two off the line.
        let px = 15.0 / self.view.zoom.max(1e-3);
        let sdx = if dx < 0.0 { 2.0 * px } else { -2.0 * px };
        let sdy = if dy < 0.0 { -2.0 * px } else { 2.0 * px };
        let corner = Point::new(s.fix.x + sdx, s.cur.y + sdy);
        ov.guides.push(vec![Point::new(s.cur.x + sdx, s.cur.y + sdy), corner]);
        ov.guides.push(vec![corner, Point::new(s.fix.x + sdx, s.fix.y + sdy)]);
        let per = match self.doc().settings.units {
            rustfig_model::Units::Inches => 1200.0,
            rustfig_model::Units::Metric => 450.0,
        };
        let k = self.unit_scale.factor / per;
        let (ax, ay) = ((dx * k).abs(), (dy * k).abs());
        let off = 5.0 * px;
        let tx = Point::new(
            (s.cur.x + s.fix.x) / 2.0,
            if dy < 0.0 {
                s.cur.y + sdy - off
            } else {
                s.cur.y + sdy + 2.0 * off
            },
        );
        let ty = Point::new(
            if dx < 0.0 {
                s.fix.x + sdx + off
            } else {
                s.fix.x + sdx - 6.0 * off
            },
            (s.cur.y + s.fix.y) / 2.0 + sdy,
        );
        ov.lengths.push((tx, format!("{ax:.3}")));
        ov.lengths.push((ty, format!("{ay:.3}")));
    }
}

/// `length_msg2`: lengths and offsets of the moving point `p3` from the two
/// fixed points (a missing one is `None`, xfig's -999).
pub(crate) fn length_msg2(p1: Option<Point>, p2: Option<Point>, p3: Point, fmt: &dyn Fn(f64) -> String) -> String {
    let side = |q: Option<Point>| {
        q.map_or((0.0, 0.0, 0.0), |q| {
            (p3.x - q.x, p3.y - q.y, (p3.x - q.x).hypot(p3.y - q.y))
        })
    };
    let (dx1, dy1, len1) = side(p1);
    let (dx2, dy2, len2) = side(p2);
    format!(
        "Len 1 = {}, Len 2 = {}, dx1 = {}, dy1 = {}, dx2 = {}, dy2 = {}",
        fmt(len1),
        fmt(len2),
        fmt(dx1),
        fmt(dy1),
        fmt(dx2),
        fmt(dy2)
    )
}

/// `arc_msg`: the middle point `p2` of an arc moves; with the radius when the
/// three points define a circle, else as `length_msg2`.
pub(crate) fn arc_msg(p1: Point, p2: Point, p3: Point, fmt: &dyn Fn(f64) -> String) -> String {
    let Some(arc) = Arc::through(Default::default(), p1, p2, p3, false) else {
        return length_msg2(Some(p1), Some(p3), p2, fmt);
    };
    let r = p2.dist(arc.center);
    if r == 0.0 || !r.is_finite() {
        return length_msg2(Some(p1), Some(p3), p2, fmt);
    }
    let (dx1, dy1) = (p2.x - p1.x, p2.y - p1.y);
    let (dx2, dy2) = (p2.x - p3.x, p2.y - p3.y);
    format!(
        "Len 1 = {}, Len 2 = {}, Rad = {}, dx1 = {}, dy1 = {}, dx2 = {}, dy2 = {}",
        fmt(dx1.hypot(dy1)),
        fmt(dx2.hypot(dy2)),
        fmt(r),
        fmt(dx1),
        fmt(dy1),
        fmt(dx2),
        fmt(dy2)
    )
}

/// `boxsize_scale_msg`: width, length and scale factor of the box being
/// scaled about `fix`, from `from` to `cur`; `fact` is 1, or 2 when the
/// scaling is about the centre.
pub(crate) fn boxsize_scale_msg(fix: Point, from: Point, cur: Point, fact: f64, fmt: &dyn Fn(f64) -> String) -> String {
    let (old_dx, old_dy) = ((from.x - fix.x).abs(), (from.y - fix.y).abs());
    let (new_dx, new_dy) = ((cur.x - fix.x).abs(), (cur.y - fix.y).abs());
    let factor = if old_dx != 0.0 && new_dx != 0.0 {
        new_dx / old_dx
    } else if old_dy != 0.0 && new_dy != 0.0 {
        new_dy / old_dy
    } else {
        0.0
    };
    format!(
        "Width = {}, Length = {}, Factor = {factor:.3}",
        fmt(fact * new_dx),
        fmt(fact * new_dy)
    )
}

impl Engine {
    /// The message while a point of a line, spline or arc is moved or added
    /// (`reshaping_line`, `reshaping_arc`).
    fn point_drag_message(&self, fmt: &dyn Fn(f64) -> String) -> Option<String> {
        let ToolState::PointDrag {
            id,
            index,
            insert,
            cursor,
            ..
        } = &self.tool_state
        else {
            return None;
        };
        let o = self.doc().get(*id)?;
        let pts = o.points()?;
        let (index, cursor) = (*index, *cursor);
        let dist_msg = |q: Point| {
            let (dx, dy) = (cursor.x - q.x, cursor.y - q.y);
            let ang = if dx != 0.0 || dy != 0.0 {
                -dy.atan2(dx).to_degrees()
            } else {
                0.0
            };
            format!(
                "Length = {}, dx = {}, dy = {} ({ang:.1}) deg",
                fmt(dx.hypot(dy)),
                fmt(dx),
                fmt(dy)
            )
        };
        if let ObjectKind::Arc(_) = &o.kind {
            let (p0, p1, p2) = (*pts.first()?, *pts.get(1)?, *pts.get(2)?);
            return Some(if index == 1 {
                arc_msg(p0, cursor, p2, fmt)
            } else {
                dist_msg(p1)
            });
        }
        let closed = matches!(&o.kind, ObjectKind::Polyline(pl) if pl.kind == PolyKind::Polygon)
            || matches!(&o.kind, ObjectKind::Spline(sp) if sp.closed);
        let before = if index > 0 {
            pts.get(index - 1)
        } else if closed {
            pts.last()
        } else {
            None
        };
        let after = if *insert { pts.get(index) } else { pts.get(index + 1) };
        let after = after.or(if closed { pts.first() } else { None });
        match (before.copied(), after.copied()) {
            (Some(l), Some(r)) => Some(length_msg2(Some(l), Some(r), cursor, fmt)),
            (Some(q), None) | (None, Some(q)) => Some(dist_msg(q)),
            _ => None,
        }
    }

    /// The message while an object is scaled (`constrained_resizing_scale_box`,
    /// `elastic_scalecompound`, the arc scaling).
    fn scale_message(&self, fmt: &dyn Fn(f64) -> String) -> Option<String> {
        let ToolState::Scale {
            id,
            center,
            start,
            cursor,
            button,
            ..
        } = &self.tool_state
        else {
            return None;
        };
        let o = self.doc().get(*id)?;
        match &o.kind {
            ObjectKind::Polyline(pl) if pl.kind.is_rect() => {
                let fact = if *button == super::Button::Middle { 2.0 } else { 1.0 };
                Some(boxsize_scale_msg(*center, *start, *cursor, fact, fmt))
            }
            ObjectKind::Compound(_) => Some(boxsize_scale_msg(*center, *start, *cursor, 2.0, fmt)),
            ObjectKind::Arc(a) => {
                let d0 = start.dist(*center);
                if d0 <= 0.0 {
                    return None;
                }
                let f = cursor.dist(*center) / d0;
                let sc = |p: Point| {
                    Point::new(
                        (center.x + (p.x - center.x) * f).round(),
                        (center.y + (p.y - center.y) * f).round(),
                    )
                };
                Some(length_msg2(
                    Some(sc(a.points[0])),
                    Some(sc(a.points[2])),
                    sc(a.points[1]),
                    fmt,
                ))
            }
            _ => None,
        }
    }
}

impl Engine {
    /// The pointer position of the operation in progress, where xfig's
    /// `crosshair` resource draws its full-window lines (`canvas_selected`
    /// in `w_canvas.c`: only while an action is on).
    pub(crate) fn crosshair_point(&self) -> Option<Point> {
        if !self.crosshair {
            return None;
        }
        match &self.tool_state {
            ToolState::Points { cursor, .. }
            | ToolState::Drag { cursor, .. }
            | ToolState::ThreePoints { cursor, .. }
            | ToolState::Angle { cursor, .. }
            | ToolState::Paste { cursor, .. }
            | ToolState::Library { cursor, .. }
            | ToolState::ZoomBox { cursor, .. }
            | ToolState::MoveObjs { cursor, .. }
            | ToolState::RubberBand { cursor, .. }
            | ToolState::PointDrag { cursor, .. }
            | ToolState::Scale { cursor, .. } => Some(*cursor),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(v: f64) -> String {
        format!("{v:.1}")
    }

    #[test]
    fn length_msg2_lists_both_sides() {
        let m = length_msg2(
            Some(Point::new(0.0, 0.0)),
            Some(Point::new(10.0, 0.0)),
            Point::new(0.0, 5.0),
            &fmt,
        );
        assert_eq!(
            m,
            "Len 1 = 5.0, Len 2 = 11.2, dx1 = 0.0, dy1 = 5.0, dx2 = -10.0, dy2 = 5.0"
        );
    }

    #[test]
    fn arc_msg_has_radius_or_falls_back() {
        let m = arc_msg(
            Point::new(-10.0, 0.0),
            Point::new(0.0, 10.0),
            Point::new(10.0, 0.0),
            &fmt,
        );
        assert!(m.contains("Rad = 10.0"), "{m}");
        let m = arc_msg(Point::new(0.0, 0.0), Point::new(5.0, 0.0), Point::new(10.0, 0.0), &fmt);
        assert!(!m.contains("Rad"), "{m}");
    }

    #[test]
    fn scale_box_message() {
        let m = boxsize_scale_msg(
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(20.0, 30.0),
            1.0,
            &fmt,
        );
        assert_eq!(m, "Width = 20.0, Length = 30.0, Factor = 2.000");
        let m = boxsize_scale_msg(
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            1.0,
            &fmt,
        );
        assert!(m.ends_with("Factor = 0.000"));
    }
}
