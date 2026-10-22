//! Ctrl+click on the canvas (plans E3, S4-6): the zoom box, pan to origin and
//! unzoom with Ctrl alone, and the shape factor of a spline point with
//! Ctrl+Shift in the editing modes.
//
// Ported from xfig src/w_zoom.c (`zoom_selected`, `init_zoombox_drawing`,
// `do_zoom`, `cancel_zoom`), src/w_canvas.c (the Ctrl and Ctrl+Shift cases of
// `canvas_selected`), src/e_edit.c (`change_sfactor`) and src/u_search.c
// (`get_spline_point`) (xfig licence, see LICENSE-xfig).

use super::{nearest_vertex, Button, ToolState};
use crate::{Engine, EngineError};
use rustfig_geom::Point;
use rustfig_model::ObjectKind;
use serde_json::json;

/// A zoom box dragged less than this many screen pixels is a click: the
/// box stays open until the next click.
const CLICK_PX: f64 = 4.0;

impl Engine {
    /// Ctrl (Cmd on macOS) held with a button press.
    pub(super) fn ctrl_press(&mut self, pos: Point, button: Button, shift: bool) -> Result<(), EngineError> {
        if !pos.is_finite() {
            return Ok(());
        }
        if shift {
            // Only the editing modes (`cur_mode >= FIRST_EDIT_MODE`).
            if self.tool.is_drawing() || self.tool == super::ToolKind::Select {
                return Ok(());
            }
            return self.change_sfactor(pos, button);
        }
        match button {
            // A tool at work (`check_action_on`) refuses the zoom box.
            Button::Left if !self.tool_busy() => {
                self.tool_state = ToolState::ZoomBox {
                    start: pos,
                    cursor: pos,
                };
            }
            Button::Left => {}
            Button::Middle => {
                // `pan_origin`: the figure's origin at the canvas's corner.
                self.execute("view.pan_origin", &json!({}))?;
            }
            Button::Right => {
                // `unzoom`: back to 100 %, the corner stays.
                self.view.zoom_about(1.0, Point::ZERO);
            }
        }
        Ok(())
    }

    /// Events while the zoom box is open.
    pub(super) fn zoom_box_event(&mut self, ev: super::ToolEvent, start: Point) -> Result<(), EngineError> {
        use super::ToolEvent as E;
        match ev {
            E::Move { pos } if pos.is_finite() => {
                self.tool_state = ToolState::ZoomBox { start, cursor: pos };
            }
            E::Press {
                pos,
                button: Button::Left,
                ..
            }
            | E::Ctrl {
                pos,
                button: Button::Left,
                ..
            } if pos.is_finite() => {
                return self.finish_zoom_box(start, pos);
            }
            E::Release {
                pos,
                button: Button::Left,
            } if pos.is_finite() => {
                // A drag closes the box on release; a click leaves it open.
                if start.dist(pos) * self.view.px_per_fig() >= CLICK_PX {
                    return self.finish_zoom_box(start, pos);
                }
            }
            E::Press {
                button: Button::Right, ..
            }
            | E::Ctrl {
                button: Button::Right, ..
            }
            | E::Key(super::Key::Escape) => {
                self.tool_state = ToolState::Idle;
            }
            _ => {}
        }
        Ok(())
    }

    fn finish_zoom_box(&mut self, a: Point, b: Point) -> Result<(), EngineError> {
        self.tool_state = ToolState::Idle;
        self.execute("view.zoom_box", &json!({"x1": a.x, "y1": a.y, "x2": b.x, "y2": b.y}))?;
        Ok(())
    }

    /// `change_sfactor`: the spline point under the click, front-most
    /// spline first (`get_spline_point`).
    fn change_sfactor(&mut self, pos: Point, button: Button) -> Result<(), EngineError> {
        let tol = self.pick_tol();
        let mut found = None;
        for o in self.doc().objects.iter().rev() {
            if !self.is_visible(o) {
                continue;
            }
            if let ObjectKind::Spline(s) = &o.kind {
                if let Some(i) = nearest_vertex(&s.points, pos, tol) {
                    found = Some((o.id, i));
                    break;
                }
            }
        }
        let Some((id, index)) = found else {
            self.set_status("Only spline points can be edited");
            return Ok(());
        };
        let action = match button {
            Button::Left => "approximate",
            Button::Middle => "cycle",
            Button::Right => "interpolate",
        };
        match self.execute(
            "spline.shape_factor",
            &json!({"id": id.0, "index": index, "action": action}),
        ) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.set_status(&e.message);
                Ok(())
            }
        }
    }
}
