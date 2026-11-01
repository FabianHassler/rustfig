//! Find mode: the match set of Edit > Find Objects. The criteria
//! (`find.set`) or a left click on an object (`find.toggle`) change the set;
//! `find.apply` acts on all of it. The set lives only while the mode is
//! active: choosing another mode or Escape clears it.

use super::{Button, ToolEvent, ToolKind};
use crate::{Engine, EngineError};
use rustfig_model::ObjectId;
use serde_json::json;

/// Most objects one set holds (a click on a drawing of this size still works).
const MAX_SET: usize = 100_000;

impl Engine {
    /// A press in Find mode: the left button adds the object under the
    /// pointer to the match set or removes it; the other buttons do nothing.
    pub(super) fn find_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let ToolEvent::Press { pos, button, .. } = ev else {
            return Ok(());
        };
        if button != Button::Left || !pos.is_finite() {
            return Ok(());
        }
        let tol = self.pick_tol();
        if let Some(id) = self.pick(pos, tol) {
            self.find_toggle(id)?;
        }
        Ok(())
    }

    /// The match set: ids of objects that still exist.
    pub fn find_matches(&self) -> Vec<u64> {
        self.find_set
            .iter()
            .filter(|id| self.doc().get(**id).is_some())
            .map(|id| id.0)
            .collect()
    }

    /// Replace the match set (enters Find mode when it is not active).
    pub(crate) fn find_replace(&mut self, ids: Vec<ObjectId>) {
        if self.tool != ToolKind::Find {
            self.set_tool(ToolKind::Find);
        }
        let mut ids = ids;
        ids.truncate(MAX_SET);
        self.find_set = ids;
    }

    /// Add `id` to the set, or remove it when it is in.
    pub(crate) fn find_toggle(&mut self, id: ObjectId) -> Result<(), EngineError> {
        if self.tool != ToolKind::Find {
            return Err(EngineError::bad_params("Find mode is not active (tool.find)"));
        }
        if self.doc().get(id).is_none() {
            return Err(EngineError::bad_params(format!("no object {}", id.0)));
        }
        if let Some(i) = self.find_set.iter().position(|x| *x == id) {
            self.find_set.remove(i);
            self.set_status("Object removed from the matches");
        } else if self.find_set.len() < MAX_SET {
            self.find_set.push(id);
            self.set_status("Object added to the matches");
        }
        Ok(())
    }

    /// Run one action on the whole match set: one undo step. Afterwards the
    /// set follows the result (Delete empties it, Glue leaves the new
    /// compound).
    pub(crate) fn find_apply(&mut self, action: &str, depth: Option<f64>) -> Result<usize, EngineError> {
        if self.tool != ToolKind::Find {
            return Err(EngineError::bad_params("Find mode is not active (tool.find)"));
        }
        let ids = self.find_matches();
        let n = ids.len();
        if n == 0 {
            return Err(EngineError::bad_params("no matches"));
        }
        match action {
            "update" => {
                self.execute("object.update", &json!({ "ids": ids }))?;
            }
            "delete" => {
                self.execute("edit.delete", &json!({ "ids": ids }))?;
                self.find_set.clear();
            }
            "group" => {
                if n < 2 {
                    return Err(EngineError::bad_params("Glue needs at least two matches"));
                }
                let r = self.execute("object.group", &json!({ "ids": ids }))?;
                if let Some(id) = r.get("id").and_then(serde_json::Value::as_u64) {
                    self.find_set = vec![ObjectId(id)];
                }
            }
            "depth" => {
                let d = depth
                    .filter(|d| d.is_finite() && (0.0..=999.0).contains(d))
                    .ok_or_else(|| EngineError::bad_params("\"depth\" must be a number from 0 to 999"))?;
                self.execute("arrange.set_depth", &json!({ "ids": ids, "depth": d.round() }))?;
            }
            other => {
                return Err(EngineError::bad_params(format!(
                    "unknown action {other:?} (update, depth, delete or group)"
                )))
            }
        }
        Ok(n)
    }
}
