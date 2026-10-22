//! The measure modes' mouse side (plan D5): angle (three points, or the
//! corner of a line or an arc), length and area (an object, with a running
//! total). Nothing is added to the drawing and the selection stays as it
//! is; the results go to the message panel.
//
// Ported from xfig src/e_measure.c (`anglemeas_selected`,
// `init_anglemeas_threepoints`, `anglemeas_second`, `anglemeas_third`,
// `cancel_anglemeas`, `lenmeas_selected`, `areameas_selected`) and
// src/u_search.c (`do_object_search`, `do_point_search`) (xfig licence, see
// LICENSE-xfig).

use super::{markers::ObjMask, Button, ToolEvent, ToolKind, ToolState};
use crate::cmd::measure::angle_target;
use crate::{Engine, EngineError};
use rustfig_geom::Point;
use serde_json::json;

impl Engine {
    /// A press in one of the measure modes. With Shift held the left and
    /// middle buttons only show the object under the pointer in xfig
    /// ("Locate Object", plan S4-1): they measure nothing.
    pub(super) fn measure_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        if let ToolEvent::Move { pos } = ev {
            let p = self.snap(pos);
            if let ToolState::Angle { cursor, .. } = &mut self.tool_state {
                *cursor = p;
            }
            return Ok(());
        }
        let ToolEvent::Press { pos, button, shift } = ev else {
            return Ok(());
        };
        match self.tool {
            ToolKind::AngleMeasure => self.angle_press(pos, button, shift),
            ToolKind::LengthMeasure | ToolKind::AreaMeasure => self.object_measure_press(pos, button, shift),
            _ => Ok(()),
        }
    }

    fn object_measure_press(&mut self, pos: Point, button: Button, shift: bool) -> Result<(), EngineError> {
        let area = self.tool == ToolKind::AreaMeasure;
        let what = if area { "area" } else { "length" };
        if button == Button::Right {
            // The area mode's Shift+right button keeps the sign from now on.
            let params = json!({"what": what, "signed": area && shift});
            return self.execute("object.measure_reset", &params).map(|_| ());
        }
        if shift {
            return Ok(());
        }
        let mask = if area {
            ObjMask::AREAMEAS_OBJECT
        } else {
            ObjMask::LENMEAS_OBJECT
        };
        let tol = self.pick_tol();
        let Some(id) = self.hit_test_where(pos, tol, &|o| mask.admits(o)) else {
            return Ok(());
        };
        let params = json!({"id": id.0, "add": button == Button::Middle});
        let cmd = if area {
            "object.measure_area"
        } else {
            "object.measure_length"
        };
        // A refused object reports in the message panel (xfig's "Sorry,
        // can't measure ..."): not an error of the tool.
        let _ = self.execute(cmd, &params);
        Ok(())
    }

    fn angle_press(&mut self, pos: Point, button: Button, shift: bool) -> Result<(), EngineError> {
        let p = self.snap(pos);
        let placed = match &self.tool_state {
            ToolState::Angle { pts, .. } => Some(pts.clone()),
            _ => None,
        };
        let Some(pts) = placed else {
            // Idle: the left button starts the three points; the middle
            // and right buttons pick a corner or an arc (middle: also
            // store the angle as the rotation angle).
            match button {
                Button::Left => {
                    self.tool_state = ToolState::Angle {
                        pts: vec![p],
                        cursor: p,
                    }
                }
                _ if shift => {}
                _ => {
                    let tol = self.pick_tol();
                    if let Some(id) = angle_target(self, pos.x, pos.y, tol) {
                        let params = json!({"id": id.0, "x": pos.x, "y": pos.y, "save": button == Button::Middle});
                        let _ = self.execute("object.measure_angle", &params);
                    }
                }
            }
            return Ok(());
        };
        match (button, pts.as_slice()) {
            (Button::Right, _) => self.tool_state = ToolState::Idle,
            // The second point; a click on the first is ignored.
            (Button::Left, [a]) if *a != p => {
                self.tool_state = ToolState::Angle {
                    pts: vec![*a, p],
                    cursor: p,
                }
            }
            // The third point: the left button measures, the middle one
            // also saves the angle.
            (Button::Left | Button::Middle, [a, b]) if *b != p => {
                let params = json!({
                    "points": [[a.x, a.y], [b.x, b.y], [p.x, p.y]],
                    "save": button == Button::Middle,
                });
                self.tool_state = ToolState::Idle;
                let _ = self.execute("object.measure_angle", &params);
            }
            _ => {}
        }
        Ok(())
    }
}
