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
use serde_json::json;

impl Engine {
    /// Chop (`chop_selected`, `select_axe_object`, `select_log_object`,
    /// `clear_axe_objects`). The search mask is xfig's
    /// `M_POLYLINE | M_ARC | M_ELLIPSE`.
    pub(super) fn chop_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        // The pointer, for the log object highlighted under it.
        if let ToolEvent::Move { pos } = ev {
            if let ToolState::Chop { hover, .. } = &mut self.tool_state {
                *hover = Some(pos);
            }
            return Ok(());
        }
        let ToolEvent::Press { pos, button, .. } = ev else {
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
        match button {
            Button::Left => {
                let Some(id) = self.hit_test_where(pos, tol, &|o| mask.admits(o)) else {
                    return Ok(());
                };
                let mut axes = match &self.tool_state {
                    ToolState::Chop { axes, .. } => axes.clone(),
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
                self.tool_state = ToolState::Chop { axes, hover: Some(pos) };
            }
            Button::Right => {
                self.tool_state = ToolState::Chop {
                    axes: Vec::new(),
                    hover: Some(pos),
                };
                self.set_status("Axe object list cleared.");
            }
            Button::Middle => {
                let axes = match &self.tool_state {
                    ToolState::Chop { axes, .. } => axes.clone(),
                    _ => Vec::new(),
                };
                let Some(id) = self.hit_test_where(pos, tol, &|o| mask.admits(o)) else {
                    return Ok(());
                };
                // The log object is the selection while the command runs;
                // `tool_event` restores the selection afterwards (only the
                // selecting modes tag objects).
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
}
