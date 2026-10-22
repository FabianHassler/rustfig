//! The lengths shown while drawing: the message panel's length, dx, dy and
//! angle of the segment being drawn, and with `show_lengths` the triangle
//! with its two sides labelled on the canvas.
//
// Ported from xfig src/w_msgpanel.c (`length_msg`, `altlength_msg`,
// `boxsize_msg`) (xfig licence, see LICENSE-xfig).

use super::{Overlay, ToolKind, ToolState};
use crate::Engine;
use rustfig_geom::Point;

/// What the message panel says (xfig's `MSG_*`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Msg {
    PntsLength(usize),
    Dist,
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
            } if self.tool != ToolKind::Arc => Some(Source {
                msg: Msg::PntsLength(pts.len()),
                fix: *pts.last()?,
                cur: *cursor,
            }),
            ToolState::Drag {
                start, cursor, kind, ..
            } => {
                let msg = match kind {
                    ToolKind::Box | ToolKind::ArcBox | ToolKind::Picture => Msg::Box,
                    ToolKind::CircleRadius | ToolKind::RegularPolygon => Msg::Radius,
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
        let Some(s) = self.length_source() else { return };
        let fmt = crate::cmd::dimension_formatter(self, self.indicator.dimline.precision);
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
            Msg::Box => format!("Width = {}, Height = {}", fmt(dx.abs()), fmt(dy.abs())),
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
