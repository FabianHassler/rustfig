//! The Chop mode's mouse side (plan D3): collect axe objects with the left
//! button, chop a log object with the middle one, clear the list with the
//! right one.
//!
//! RustFig adds three operations on the same buttons with Shift held (plan
//! D15; xfig's three buttons keep their meaning without it): Shift+left
//! trims the clicked piece (between its intersections with the axe objects,
//! or with every other object when none is selected), Shift+middle splits
//! the clicked object at the point, Shift+right extends the clicked line's
//! nearest end to the nearest boundary (the axe objects, or every other
//! object).
//
// Ported from xfig src/e_chop.c (xfig licence, see LICENSE-xfig).

use super::{markers::ObjMask, Button, ToolEvent, ToolKind, ToolState};
use crate::{Engine, EngineError};
use rustfig_geom::Point;
use serde_json::json;

impl Engine {
    /// Chop (`chop_selected`, `select_axe_object`, `select_log_object`,
    /// `clear_axe_objects`). The search mask is xfig's
    /// `M_POLYLINE | M_ARC | M_ELLIPSE`.
    pub(super) fn chop_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let ToolEvent::Press { pos, button, shift } = ev else {
            return Ok(());
        };
        if self.tool != ToolKind::Chop {
            return Ok(());
        }
        let tol = self.pick_tol();
        let mask = ObjMask::POLYLINE
            .union(ObjMask::ARC)
            .union(ObjMask::ELLIPSE)
            .union(ObjMask::SPLINE)
            .union(ObjMask::COMPOUND);
        if shift {
            return self.chop_shift_press(pos, button, tol, mask);
        }
        match button {
            Button::Left => {
                let Some(id) = self.hit_test_where(pos, tol, &|o| mask.admits(o)) else {
                    return Ok(());
                };
                let mut axes = match &self.tool_state {
                    ToolState::Chop { axes } => axes.clone(),
                    _ => Vec::new(),
                };
                match axes.iter().position(|a| *a == id) {
                    Some(i) => {
                        axes.remove(i);
                        self.set_status("Axe object removed.");
                    }
                    None => {
                        axes.push(id);
                        self.set_status(format!("Axe object {} selected", axes.len()));
                    }
                }
                self.tool_state = ToolState::Chop { axes };
            }
            Button::Right => {
                self.tool_state = ToolState::Chop { axes: Vec::new() };
                self.set_status("Axe object list cleared.");
            }
            Button::Middle => {
                let axes = match &self.tool_state {
                    ToolState::Chop { axes } => axes.clone(),
                    _ => Vec::new(),
                };
                let Some(id) = self.hit_test_where(pos, tol, &|o| mask.admits(o)) else {
                    return Ok(());
                };
                // The log object is the selection while the command runs, so
                // a click always has a visible effect (and scripts can see
                // which object the next axe list will cut).
                self.set_selection(vec![id]);
                if axes.is_empty() {
                    self.set_status("Select an axe object first.");
                    return Ok(());
                }
                let axes: Vec<u64> = axes.iter().map(|a| a.0).collect();
                let r = self.execute("object.chop", &json!({"id": id.0, "axes": axes}));
                match r {
                    Ok(v) => {
                        let n = v.get("pieces").and_then(serde_json::Value::as_u64).unwrap_or(0);
                        let what = match &self.doc().get(id) {
                            Some(o) => o.type_name(),
                            None => "object",
                        };
                        if n == 0 {
                            self.set_status(format!("{what} does not intersect with any selected axe elements."));
                        } else {
                            self.set_status(format!("{what} chopped into {n} pieces."));
                        }
                    }
                    Err(err) => self.set_status(err.message),
                }
            }
        }
        Ok(())
    }

    /// The axe list as command parameters (`None` when empty).
    fn chop_axe_ids(&self) -> Option<Vec<u64>> {
        match &self.tool_state {
            ToolState::Chop { axes } if !axes.is_empty() => Some(axes.iter().map(|a| a.0).collect()),
            _ => None,
        }
    }

    /// Shift+left: Trim; Shift+middle: Split at point; Shift+right: Extend.
    fn chop_shift_press(&mut self, pos: Point, button: Button, tol: f64, mask: ObjMask) -> Result<(), EngineError> {
        let Some(id) = self.hit_test_where(pos, tol, &|o| mask.admits(o)) else {
            return Ok(());
        };
        let axes = self.chop_axe_ids();
        let (command, mut params, done) = match button {
            Button::Left => (
                "object.trim",
                json!({"id": id.0, "x": pos.x, "y": pos.y}),
                "piece trimmed",
            ),
            Button::Middle => ("object.split_at", json!({"id": id.0, "x": pos.x, "y": pos.y}), "split"),
            Button::Right => (
                "object.extend",
                json!({"id": id.0, "x": pos.x, "y": pos.y}),
                "line extended",
            ),
        };
        if let (Some(a), Some(map)) = (axes, params.as_object_mut()) {
            let key = if button == Button::Right { "boundaries" } else { "axes" };
            if button != Button::Middle {
                map.insert(key.to_string(), json!(a));
            }
        }
        self.set_selection(vec![id]);
        match self.execute(command, &params) {
            Ok(v) => {
                let n = v
                    .get("ids")
                    .and_then(serde_json::Value::as_array)
                    .map_or(0, |a| a.len());
                if command == "object.trim" && n == 0 && v.get("removed") == Some(&json!(false)) {
                    self.set_status("Nothing to trim: the object has no piece between two intersections.");
                } else {
                    self.set_status(format!("Object {done}."));
                }
            }
            Err(err) => self.set_status(err.message),
        }
        Ok(())
    }
}
