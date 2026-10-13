//! RustFig engine (layer L4): the document session.
//!
//! The engine owns the document, selection, undo history, current drawing
//! attributes (the "indicator panel"), view state and the active tool.
//! Every user-visible action is a command in [`cmd`]; tools turn pointer
//! events into commands. The UI, the control channel and MCP all drive the
//! engine through [`Engine::execute`] and [`Engine::tool_event`].

pub mod cmd;
mod error;
pub mod guard;
mod history;
pub mod import;
mod indicator;
pub mod tools;
mod view;

pub use error::EngineError;
pub use indicator::{GridMode, Indicator, SnapMode};
pub use tools::{Key, Overlay, ToolEvent, ToolKind};
pub use view::View;

pub use rustfig_format as format;
pub use rustfig_geom as geom;
pub use rustfig_model as model;
pub use rustfig_render as render;

use history::History;
use rustfig_geom::{Point, Rect};
use rustfig_model::{Document, Object, ObjectId, ObjectKind, Text};
use serde_json::Value;
use std::collections::BTreeSet;

/// Something the engine asks the UI to do (open a dialog, pick a file).
#[derive(Debug, Clone, PartialEq)]
pub enum UiRequest {
    /// Choose a picture file to place in `rect` (Fig units).
    PickPicture { rect: Rect },
    /// Open the attribute editor for an object.
    EditObject { id: ObjectId },
}

/// The document session.
pub struct Engine {
    doc: Document,
    selection: Vec<ObjectId>,
    history: History,
    /// Current attributes for new objects.
    pub indicator: Indicator,
    pub view: View,
    tool: ToolKind,
    tool_state: tools::ToolState,
    clipboard: Vec<Object>,
    path: Option<String>,
    modified: bool,
    revision: u64,
    /// Depths hidden in the depth manager.
    pub hidden_depths: BTreeSet<i32>,
    pub show_page_border: bool,
    requests: Vec<UiRequest>,
    status: String,
    /// Set by [`Engine::doc_mut`] while a command runs.
    changed: bool,
    /// Bounding boxes of top-level objects, valid for one revision.
    bbox_cache: std::cell::RefCell<(u64, Vec<Rect>)>,
}

impl Default for Engine {
    fn default() -> Self {
        Engine::new()
    }
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            doc: Document::new(),
            selection: Vec::new(),
            history: History::default(),
            indicator: Indicator::default(),
            view: View::default(),
            tool: ToolKind::Select,
            tool_state: tools::ToolState::Idle,
            clipboard: Vec::new(),
            path: None,
            modified: false,
            revision: 1,
            hidden_depths: BTreeSet::new(),
            show_page_border: true,
            requests: Vec::new(),
            status: String::new(),
            changed: false,
            bbox_cache: std::cell::RefCell::new((0, Vec::new())),
        }
    }

    pub fn doc(&self) -> &Document {
        &self.doc
    }

    /// Mutable document access; marks the document as changed.
    pub fn doc_mut(&mut self) -> &mut Document {
        self.changed = true;
        &mut self.doc
    }

    /// Increases whenever the document or its visibility changes.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn selection(&self) -> &[ObjectId] {
        &self.selection
    }

    /// Replace the selection (unknown ids are dropped).
    pub fn set_selection(&mut self, ids: Vec<ObjectId>) {
        let mut seen = BTreeSet::new();
        self.selection = ids
            .into_iter()
            .filter(|id| self.doc.get(*id).is_some() && seen.insert(*id))
            .collect();
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub fn set_path(&mut self, p: Option<String>) {
        self.path = p;
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn set_modified(&mut self, m: bool) {
        self.modified = m;
    }

    pub fn tool(&self) -> ToolKind {
        self.tool
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn set_status(&mut self, s: impl Into<String>) {
        self.status = s.into();
    }

    pub fn clipboard(&self) -> &[Object] {
        &self.clipboard
    }

    pub fn set_clipboard(&mut self, objs: Vec<Object>) {
        self.clipboard = objs;
    }

    /// Requests for the UI, drained by the caller.
    pub fn take_requests(&mut self) -> Vec<UiRequest> {
        std::mem::take(&mut self.requests)
    }

    pub(crate) fn request(&mut self, r: UiRequest) {
        self.requests.push(r);
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Replace the whole document (new/open), clearing history and selection.
    pub fn replace_document(&mut self, mut doc: Document, path: Option<String>) {
        if doc.next_id == 0 {
            doc.assign_ids();
        }
        self.doc = doc;
        self.fix_all_text();
        self.selection.clear();
        self.history = History::default();
        self.path = path;
        self.modified = false;
        self.tool_state = tools::ToolState::Idle;
        self.hidden_depths.clear();
        self.bump_revision();
    }

    /// Run a command by id. Mutating commands are atomic and undoable; a
    /// failure leaves the document untouched.
    pub fn execute(&mut self, id: &str, params: &Value) -> Result<Value, EngineError> {
        let Some(command) = cmd::find(id) else {
            return Err(EngineError::unknown_command(id));
        };
        if !(command.enabled)(self) {
            return Err(EngineError::disabled(id));
        }
        let snapshot = if command.mutates {
            Some((self.doc.clone(), self.selection.clone()))
        } else {
            None
        };
        self.changed = false;
        let result = guard::catch(|| (command.run)(self, params));
        let result = match result {
            Ok(r) => r,
            Err(panic_msg) => Err(EngineError::internal(panic_msg)),
        };
        match (&result, snapshot) {
            (Err(_), Some((doc, sel))) => {
                self.doc = doc;
                self.selection = sel;
                self.changed = false;
            }
            (Ok(_), Some((doc, sel))) if self.changed => {
                self.history.push(doc, sel);
                self.modified = true;
                self.bump_revision();
                self.prune_selection();
            }
            _ => {}
        }
        self.changed = false;
        result
    }

    /// Undo the last change.
    pub fn undo(&mut self) -> bool {
        let cur = (self.doc.clone(), self.selection.clone());
        match self.history.undo(cur) {
            Some((doc, sel)) => {
                self.doc = doc;
                self.selection = sel;
                self.modified = true;
                self.tool_state = tools::ToolState::Idle;
                self.bump_revision();
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        let cur = (self.doc.clone(), self.selection.clone());
        match self.history.redo(cur) {
            Some((doc, sel)) => {
                self.doc = doc;
                self.selection = sel;
                self.modified = true;
                self.tool_state = tools::ToolState::Idle;
                self.bump_revision();
                true
            }
            None => false,
        }
    }

    fn prune_selection(&mut self) {
        let doc = &self.doc;
        self.selection.retain(|id| doc.get(*id).is_some());
    }

    /// Ids from a `"ids"` parameter, or the selection when absent.
    pub fn target_ids(&self, params: &Value) -> Result<Vec<ObjectId>, EngineError> {
        match params.get("ids") {
            None | Some(Value::Null) => Ok(self.selection.clone()),
            Some(Value::Array(a)) => {
                let mut out = Vec::new();
                for v in a {
                    let id = v
                        .as_u64()
                        .ok_or_else(|| EngineError::bad_params("ids must be an array of object ids"))?;
                    let id = ObjectId(id);
                    if self.doc.get(id).is_none() {
                        return Err(EngineError::bad_params(format!("no top-level object with id {}", id.0)));
                    }
                    out.push(id);
                }
                Ok(out)
            }
            Some(_) => Err(EngineError::bad_params("ids must be an array of object ids")),
        }
    }

    /// Add an object at the top level, assigning a fresh id. Returns the id.
    pub fn add_object(&mut self, mut obj: Object) -> ObjectId {
        if let ObjectKind::Text(t) = &mut obj.kind {
            fix_text_metrics(t);
        }
        self.doc_mut().reassign_ids(&mut obj);
        let id = obj.id;
        self.doc_mut().objects.push(obj);
        id
    }

    /// Recompute text heights/lengths from font metrics.
    pub fn fix_all_text(&mut self) {
        fn walk(objs: &mut [Object]) {
            for o in objs {
                match &mut o.kind {
                    ObjectKind::Text(t) => fix_text_metrics(t),
                    ObjectKind::Compound(c) => walk(&mut c.objects),
                    _ => {}
                }
            }
        }
        walk(&mut self.doc.objects);
    }

    /// Bounding boxes of the top-level objects (cached per revision).
    fn object_bboxes(&self) -> std::cell::Ref<'_, (u64, Vec<Rect>)> {
        let stale = {
            let c = self.bbox_cache.borrow();
            c.0 != self.revision || c.1.len() != self.doc.objects.len()
        };
        if stale {
            let boxes = self.doc.objects.iter().map(Object::bbox).collect();
            *self.bbox_cache.borrow_mut() = (self.revision, boxes);
        }
        self.bbox_cache.borrow()
    }

    /// Topmost top-level object within `tol` Fig units of `p`.
    pub fn hit_test(&self, p: Point, tol: f64) -> Option<ObjectId> {
        let boxes = self.object_bboxes();
        let mut best: Option<(i32, f64, usize)> = None;
        for (i, o) in self.doc.objects.iter().enumerate() {
            let Some(bb) = boxes.1.get(i) else { break };
            if !bb.expand(tol).contains(p) || !self.is_visible(o) {
                continue;
            }
            let d = o.distance(p);
            if d > tol {
                continue;
            }
            let depth = o.depth();
            // Prefer the front-most (smallest depth), then the latest drawn.
            let better = match best {
                None => true,
                Some((bd, _, bi)) => depth < bd || (depth == bd && i > bi),
            };
            if better {
                best = Some((depth, d, i));
            }
        }
        best.and_then(|(_, _, i)| self.doc.objects.get(i)).map(|o| o.id)
    }

    /// Whether any leaf of the object is on a visible depth.
    pub fn is_visible(&self, o: &Object) -> bool {
        if self.hidden_depths.is_empty() {
            return true;
        }
        match &o.kind {
            ObjectKind::Compound(c) => c.objects.iter().any(|x| self.is_visible(x)),
            _ => !self.hidden_depths.contains(&o.depth()),
        }
    }

    /// Bounding box of the selection.
    pub fn selection_bbox(&self) -> Rect {
        self.selection
            .iter()
            .filter_map(|id| self.doc.get(*id))
            .fold(Rect::EMPTY, |r, o| r.union(&o.bbox()))
    }

    /// Snap a point according to the positioning mode.
    pub fn snap(&self, p: Point) -> Point {
        self.indicator.snap_point(p, self.doc.settings.units)
    }
}

/// Recompute a text's stored height and length from font metrics.
pub fn fix_text_metrics(t: &mut Text) {
    let req = rustfig_text::FontRequest::for_fig(t.font, t.is_postscript());
    let m = rustfig_text::measure(&req, t.size, &t.text);
    t.length = m.width;
    t.height = m.ascent;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn execute_and_undo() {
        let mut e = Engine::new();
        let r = e
            .execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 50}))
            .unwrap();
        assert!(r.get("id").is_some());
        assert_eq!(e.doc().objects.len(), 1);
        assert!(e.can_undo());
        assert!(e.undo());
        assert_eq!(e.doc().objects.len(), 0);
        assert!(e.redo());
        assert_eq!(e.doc().objects.len(), 1);
    }

    #[test]
    fn failed_command_is_atomic() {
        let mut e = Engine::new();
        let before = e.doc().clone();
        assert!(e.execute("shape.rectangle", &json!({"x": "a"})).is_err());
        assert_eq!(e.doc(), &before);
        assert!(!e.can_undo());
        assert!(e.execute("no.such.command", &json!({})).is_err());
    }

    #[test]
    fn hit_test_prefers_front() {
        let mut e = Engine::new();
        e.execute(
            "shape.rectangle",
            &json!({"x": 0, "y": 0, "width": 100, "height": 100, "depth": 60}),
        )
        .unwrap();
        let r = e
            .execute(
                "shape.rectangle",
                &json!({"x": 0, "y": 0, "width": 100, "height": 100, "depth": 10}),
            )
            .unwrap();
        let front = ObjectId(r["id"].as_u64().unwrap());
        assert_eq!(e.hit_test(Point::new(0.0, 50.0), 5.0), Some(front));
        assert_eq!(e.hit_test(Point::new(500.0, 500.0), 5.0), None);
    }
}
