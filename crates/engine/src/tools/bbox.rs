//! Bounding box mode (plan F11): the figure's export box drawn as a frame
//! with eight handles. The left button drags a corner or an edge (that
//! side follows the pointer) or, inside the frame, the whole box; the
//! release commits it with `doc.set_bounding_box`, one undo step. Starting
//! from the standard box (the page) turns it into the figure's own.

use super::{Button, ToolEvent, ToolState};
use crate::{Engine, EngineError};
use rustfig_geom::{Point, Rect};
use rustfig_model::BoundingBox;
use serde_json::json;

/// The sides a drag moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Grip {
    pub left: bool,
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
}

impl Grip {
    const MOVE: Grip = Grip {
        left: true,
        top: true,
        right: true,
        bottom: true,
    };
}

/// The handles of `b`: corners and edge middles, clockwise from the top
/// left, with the sides each one drags.
pub(crate) fn handles(b: &BoundingBox) -> [(Point, Grip); 8] {
    let r = b.rect();
    let (x1, y1, x2, y2) = (r.min.x, r.min.y, r.max.x, r.max.y);
    let (xm, ym) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
    let g = |left, top, right, bottom| Grip {
        left,
        top,
        right,
        bottom,
    };
    [
        (Point::new(x1, y1), g(true, true, false, false)),
        (Point::new(xm, y1), g(false, true, false, false)),
        (Point::new(x2, y1), g(false, true, true, false)),
        (Point::new(x2, ym), g(false, false, true, false)),
        (Point::new(x2, y2), g(false, false, true, true)),
        (Point::new(xm, y2), g(false, false, false, true)),
        (Point::new(x1, y2), g(true, false, false, true)),
        (Point::new(x1, ym), g(true, false, false, false)),
    ]
}

/// `from` with the gripped sides moved by `d` (Fig units); `None` when the
/// box would have no width or height.
pub(crate) fn dragged(from: &BoundingBox, grip: Grip, d: Point) -> Option<BoundingBox> {
    if !d.is_finite() {
        return None;
    }
    let lim = 1.0e9;
    let mv = |v: i32, on: bool, by: f64| -> i32 {
        if on {
            (f64::from(v) + by).round().clamp(-lim, lim) as i32
        } else {
            v
        }
    };
    BoundingBox::new(
        (mv(from.x1, grip.left, d.x), mv(from.y1, grip.top, d.y)),
        (mv(from.x2, grip.right, d.x), mv(from.y2, grip.bottom, d.y)),
    )
}

impl Engine {
    /// A mouse event in Bounding box mode.
    pub(super) fn bounding_box_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        match ev {
            ToolEvent::Press {
                pos,
                button: Button::Left,
                ..
            } if pos.is_finite() => {
                let from = self.doc().export_box();
                let tol = self.pick_tol();
                let grip = handles(&from)
                    .into_iter()
                    .find(|(h, _)| h.dist(pos) <= tol)
                    .map(|(_, g)| g)
                    .or_else(|| near_edge(&from.rect(), pos, tol))
                    .or_else(|| from.rect().contains(pos).then_some(Grip::MOVE));
                if let Some(grip) = grip {
                    let p = self.snap(pos);
                    self.tool_state = ToolState::BoxDrag {
                        from,
                        grip,
                        start: p,
                        cursor: p,
                    };
                }
                Ok(())
            }
            ToolEvent::Move { pos } => {
                let p = self.snap(pos);
                if let ToolState::BoxDrag { cursor, .. } = &mut self.tool_state {
                    *cursor = p;
                }
                Ok(())
            }
            ToolEvent::Release { pos, .. } => {
                let ToolState::BoxDrag { from, grip, start, .. } = self.tool_state.clone() else {
                    return Ok(());
                };
                self.tool_state = ToolState::Idle;
                let end = self.snap(pos);
                if end == start {
                    return Ok(());
                }
                match dragged(&from, grip, end - start) {
                    Some(b) => {
                        let params = json!({"x1": b.x1, "y1": b.y1, "x2": b.x2, "y2": b.y2});
                        self.execute("doc.set_bounding_box", &params).map(|_| ())
                    }
                    None => {
                        self.set_status("The bounding box needs a width and a height.");
                        Ok(())
                    }
                }
            }
            _ => Ok(()),
        }
    }

    /// The box the frame shows now: the one being dragged, else the
    /// figure's export box.
    pub fn shown_bounding_box(&self) -> BoundingBox {
        match &self.tool_state {
            ToolState::BoxDrag {
                from,
                grip,
                start,
                cursor,
            } => dragged(from, *grip, *cursor - *start).unwrap_or(*from),
            _ => self.doc().export_box(),
        }
    }
}

/// The side of `r` within `tol` of `p` (between its corners).
fn near_edge(r: &Rect, p: Point, tol: f64) -> Option<Grip> {
    let inside_x = p.x >= r.min.x && p.x <= r.max.x;
    let inside_y = p.y >= r.min.y && p.y <= r.max.y;
    let none = Grip {
        left: false,
        top: false,
        right: false,
        bottom: false,
    };
    if inside_y && (p.x - r.min.x).abs() <= tol {
        Some(Grip { left: true, ..none })
    } else if inside_y && (p.x - r.max.x).abs() <= tol {
        Some(Grip { right: true, ..none })
    } else if inside_x && (p.y - r.min.y).abs() <= tol {
        Some(Grip { top: true, ..none })
    } else if inside_x && (p.y - r.max.y).abs() <= tol {
        Some(Grip { bottom: true, ..none })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_corner_drags_two_sides() {
        let b = BoundingBox::new((0, 0), (100, 50)).unwrap();
        let (_, g) = handles(&b)[4];
        assert_eq!(
            dragged(&b, g, Point::new(10.0, -5.0)),
            BoundingBox::new((0, 0), (110, 45))
        );
        assert_eq!(
            dragged(&b, Grip::MOVE, Point::new(-10.0, 5.0)),
            BoundingBox::new((-10, 5), (90, 55))
        );
        // Dragged across the opposite side: the corners swap.
        let (_, left) = handles(&b)[7];
        assert_eq!(
            dragged(&b, left, Point::new(150.0, 0.0)),
            BoundingBox::new((100, 0), (150, 50))
        );
        assert_eq!(dragged(&b, left, Point::new(100.0, 0.0)), None);
        assert_eq!(dragged(&b, left, Point::new(f64::NAN, 0.0)), None);
    }
}
