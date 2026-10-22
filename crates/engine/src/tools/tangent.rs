//! The Tangent mode's mouse side (plan D4): the left button adds a tangent
//! line, the middle button a normal line, at the clicked point of a curve.
//
// Ported from xfig src/e_tangent.c (`tangent_selected`,
// `init_tangent_adding`, `init_normal_adding`) and src/u_smartsearch.c
// (`do_smart_object_search`: the search order) (xfig licence, see
// LICENSE-xfig).

use super::{markers::ObjMask, Button, ToolEvent, ToolKind};
use crate::cmd::tangent::{accepts, curve_point};
use crate::{Engine, EngineError};
use rustfig_geom::Point;
use rustfig_model::{ObjectId, ObjectKind};
use serde_json::json;

impl Engine {
    /// The object a click at `pos` is on, searched as xfig's smart search
    /// does: ellipses, lines, splines, then arcs, each from the last drawn
    /// to the first, the first with a curve point under the click.
    pub fn tangent_target(&self, pos: Point, tol: f64) -> Option<ObjectId> {
        let rank = |k: &ObjectKind| match k {
            ObjectKind::Ellipse(_) => Some(0),
            ObjectKind::Polyline(_) => Some(1),
            ObjectKind::Spline(_) => Some(2),
            ObjectKind::Arc(_) => Some(3),
            ObjectKind::Text(_) | ObjectKind::Compound(_) => None,
        };
        (0..4).find_map(|want| {
            self.doc()
                .objects
                .iter()
                .rev()
                .filter(|o| rank(&o.kind) == Some(want) && accepts(o) && ObjMask::TANGENT_OBJECT.admits(o))
                .filter(|o| self.is_visible(o))
                .find(|o| curve_point(o, pos.x, pos.y, tol).is_some())
                .map(|o| o.id)
        })
    }

    /// Tangent mode (`tangent_selected`): left button tangent, middle
    /// button normal, right button nothing.
    pub(super) fn tangent_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let ToolEvent::Press { pos, button, .. } = ev else {
            return Ok(());
        };
        if self.tool != ToolKind::Tangent || button == Button::Right {
            return Ok(());
        }
        let tol = self.pick_tol();
        let Some(id) = self.tangent_target(pos, tol) else {
            return Ok(());
        };
        let normal = button == Button::Middle;
        let params = json!({"id": id.0, "x": pos.x, "y": pos.y, "normal": normal, "tolerance": tol});
        match self.execute("object.tangent_line", &params) {
            Ok(_) => self.set_status(if normal { "added normal" } else { "added tangent" }),
            Err(err) => self.set_status(err.message),
        }
        Ok(())
    }
}
