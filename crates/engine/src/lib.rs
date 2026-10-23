//! RustFig engine (layer L4): the document session.
//!
//! The engine owns the document, selection, undo history, current drawing
//! attributes (the "indicator panel"), view state and the active tool.
//! Every user-visible action is a command in [`cmd`]; tools turn pointer
//! events into commands. The UI, the control channel and MCP all drive the
//! engine through [`Engine::execute`] and [`Engine::tool_event`].

mod anglegeom;
pub mod cmd;
pub mod cmdline;
pub mod coords;
pub mod dimline;
mod error;
pub mod export_panel;
pub mod fig2dev;
mod globals;
pub mod guard;
mod history;
pub mod import;
mod indicator;
mod links;
pub mod picture;
pub mod prefs;
#[cfg(not(target_arch = "wasm32"))]
pub mod recovery;
mod region;
pub mod snap;
pub mod styles;
pub mod tools;
mod view;

pub use anglegeom::{constrain as constrain_angle, AngleGeom};
pub use cmd::file::{export_job, ExportJob};
pub use error::EngineError;
pub use indicator::{arrow_choice, arrow_choice_of, GridMode, Indicator, LinkMode, SnapMode, UpdateFlag};
pub use region::Region;
pub use snap::ObjectSnap;
pub use tools::{Button, CursorKind, Key, MouseHints, Overlay, ToolEvent, ToolKind};
pub use view::{UnitScale, View};

pub use rustfig_format as format;
pub use rustfig_geom as geom;
pub use rustfig_model as model;
pub use rustfig_render as render;

use history::History;
use rustfig_geom::{Point, Rect};
use rustfig_model::{Document, Object, ObjectId, ObjectKind, Text};
use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::Arc;

/// A cached box of a top-level object: the picking box (a superset of the
/// drawn object) and, once asked for, fig2dev's bounds of it.
type BoxEntry = (Arc<Object>, Rect, std::cell::Cell<Option<Rect>>);

/// Something the engine asks the UI to do (open a dialog, pick a file).
#[derive(Debug, Clone, PartialEq)]
pub enum UiRequest {
    /// Choose a picture file to place in `rect` (Fig units).
    PickPicture { rect: Rect, keep_aspect: bool },
    /// Open the attribute editor for an object.
    EditObject { id: ObjectId },
    /// Edit the whole figure's comments (xfig's middle button in Edit mode).
    EditFigureComments,
    /// Edit the shape factor of one control point of a spline (xfig's right
    /// button in Edit mode).
    EditSplinePoint { id: ObjectId, index: usize },
    /// Paste the system clipboard's text into the text being typed (xfig's
    /// Shift+middle "paste text" in Text mode); it comes back as text input.
    PasteText,
}

/// The document session.
pub struct Engine {
    doc: Document,
    selection: Vec<ObjectId>,
    history: History,
    /// Current attributes for new objects.
    pub indicator: Indicator,
    pub view: View,
    /// Ruler units and figure scale.
    pub unit_scale: UnitScale,
    /// The Snap menu's mode for the next point (xfig's `snap_mode`).
    object_snap: Option<ObjectSnap>,
    /// Keep the snap mode after it was used (Snap › Hold).
    snap_held: bool,
    /// Intersect: the first object, waiting for the second.
    intersect_first: Option<Object>,
    /// The measure modes' running totals (plan D5).
    measure: cmd::measure::MeasureState,
    /// Points placed by an object snap during the current operation; grid
    /// rounding leaves them alone.
    exact_points: Vec<Point>,
    /// The release that follows a snapped press happens at the snapped
    /// point, so the tool sees a click there.
    release_at: Option<Point>,
    /// Where the user clicked when the press was snapped: tools pick the
    /// object there and measure from the snapped point.
    snap_click: Option<Point>,
    /// Shift+click's "Locate Object" mark (xfig's `do_object_search`).
    locate: Option<tools::Locate>,
    /// Convert: the kind of the last spline made into a line, restored when
    /// a line becomes a spline (xfig's static `flag`).
    convert_kind: rustfig_model::SplineKind,
    /// While a plain click acts on a located object, the one it must pick.
    locate_pick: Option<ObjectId>,
    tool: ToolKind,
    tool_state: tools::ToolState,
    /// Rotate's centre or Flip's anchor set with the right button (xfig's
    /// `setcenter` / `setanchor`).
    set_point: Option<Point>,
    clipboard: Vec<Object>,
    /// The system clipboard for "Copy as picture" (E14), when the front end has one.
    picture_clipboard: Option<Box<dyn picture::PictureClipboard>>,
    /// The Fig text this engine last put on the system clipboard (E7): a
    /// clipboard holding the same text is the cut buffer itself.
    pub(crate) last_fig: Option<String>,
    path: Option<String>,
    modified: bool,
    revision: u64,
    /// Depths hidden in the depth manager.
    pub hidden_depths: BTreeSet<i32>,
    /// Where crash-recovery copies live (`recovery`); `None` is the
    /// platform's folder.
    pub recovery_dir: Option<std::path::PathBuf>,
    /// Preferences (E1): xfig's resources and RustFig's settings.
    pub prefs: prefs::Prefs,
    /// Set when `prefs` changed and should be written.
    pub prefs_dirty: bool,
    /// Edit > Copy as Picture waits for its objects (xfig has no
    /// selection): left an object, middle a region, right the whole figure.
    pub(crate) copy_pick: bool,
    /// Draw objects on hidden depths in gray (xfig's "Gray"); otherwise
    /// they aren't drawn ("Blank").
    pub gray_hidden_depths: bool,
    pub show_page_border: bool,
    /// Show Lengths: the dx / dy triangle of the segment being drawn.
    pub show_lengths: bool,
    requests: Vec<UiRequest>,
    status: String,
    /// Set by [`Engine::doc_mut`] while a command runs.
    changed: bool,
    /// Bounding boxes of the top-level objects, each with the object it
    /// belongs to: a changed object is a new `Arc`, so entries stay right
    /// even while a command runs.
    bbox_cache: std::cell::RefCell<Vec<BoxEntry>>,
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
            unit_scale: UnitScale::default(),
            object_snap: None,
            snap_held: false,
            intersect_first: None,
            measure: cmd::measure::MeasureState::default(),
            exact_points: Vec::new(),
            release_at: None,
            snap_click: None,
            locate: None,
            convert_kind: rustfig_model::SplineKind::Interpolated,
            locate_pick: None,
            tool: ToolKind::None,
            tool_state: tools::ToolState::Idle,
            set_point: None,
            clipboard: Vec::new(),
            picture_clipboard: None,
            last_fig: None,
            path: None,
            modified: false,
            revision: 1,
            hidden_depths: BTreeSet::new(),
            recovery_dir: None,
            prefs: prefs::Prefs::default(),
            prefs_dirty: false,
            copy_pick: false,
            gray_hidden_depths: true,
            show_page_border: true,
            show_lengths: false,
            requests: Vec::new(),
            status: "READY. Select a mode or load a file".to_string(),
            changed: false,
            bbox_cache: std::cell::RefCell::new(Vec::new()),
        }
    }

    pub fn doc(&self) -> &Document {
        &self.doc
    }

    /// The whole figure for saving and exporting: with a compound open
    /// (`compound.open`) the drawing waits in the open levels, and this
    /// closes them in a copy (xfig's `close_all_compounds` before saving).
    pub fn whole_doc(&self) -> std::borrow::Cow<'_, Document> {
        if self.doc.open.is_empty() {
            std::borrow::Cow::Borrowed(&self.doc)
        } else {
            std::borrow::Cow::Owned(self.doc.closed_all())
        }
    }

    /// Only the visible part of the figure: the members of the innermost
    /// open compound (xfig's "Save Part"), or the whole figure when none is
    /// open.
    pub fn visible_part(&self) -> std::borrow::Cow<'_, Document> {
        if self.doc.open.is_empty() {
            std::borrow::Cow::Borrowed(&self.doc)
        } else {
            let mut d = self.doc.clone();
            d.open.clear();
            std::borrow::Cow::Owned(d)
        }
    }

    /// How many compounds are open, one inside the other.
    pub fn open_compounds(&self) -> usize {
        self.doc.open.len()
    }

    /// Memory the undo and redo steps hold of their own (an estimate; they
    /// share unchanged objects with the document).
    pub fn undo_bytes(&self) -> usize {
        self.history.bytes()
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

    /// Install where "Copy as picture" puts its pictures (the front end's
    /// system clipboard); without one the command reports "not available".
    pub fn set_picture_clipboard(&mut self, c: Option<Box<dyn picture::PictureClipboard>>) {
        self.picture_clipboard = c;
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
                self.fix_changed_text(&doc);
                self.history.push(doc, sel, &self.doc);
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
        self.doc_mut().objects.push(Arc::new(obj));
        id
    }

    /// Recompute the text metrics of the top-level objects that differ from
    /// `before` (the document before a command); the rest are unchanged.
    fn fix_changed_text(&mut self, before: &Document) {
        let (a, b) = (&mut self.doc.objects, &before.objects);
        let positional = a.len() == b.len();
        let others: std::collections::HashSet<*const Object> = if positional {
            Default::default()
        } else {
            b.iter().map(Arc::as_ptr).collect()
        };
        for (i, o) in a.iter_mut().enumerate() {
            let unchanged = if positional {
                b.get(i).is_some_and(|x| Arc::ptr_eq(x, o))
            } else {
                others.contains(&Arc::as_ptr(o))
            };
            if !unchanged && text_stale(o) {
                fix_texts(Arc::make_mut(o));
            }
        }
    }

    /// Recompute text heights/lengths from font metrics. Only objects
    /// whose texts change are copied, so the rest stay shared with the undo
    /// history.
    pub fn fix_all_text(&mut self) {
        for o in &mut self.doc.objects {
            if text_stale(o) {
                fix_texts(Arc::make_mut(o));
            }
        }
    }

    /// Bounding boxes of the top-level objects, recomputed only for objects
    /// that changed since the last call.
    fn object_bboxes(&self) -> std::cell::Ref<'_, Vec<BoxEntry>> {
        let objs = &self.doc.objects;
        let fresh = {
            let c = self.bbox_cache.borrow();
            c.len() == objs.len() && c.iter().zip(objs).all(|((a, ..), b)| Arc::ptr_eq(a, b))
        };
        if !fresh {
            let mut c = self.bbox_cache.borrow_mut();
            let old = std::mem::take(&mut *c);
            // Unchanged runs at the start and the end keep their boxes.
            let prefix = old
                .iter()
                .zip(objs)
                .take_while(|((a, ..), b)| Arc::ptr_eq(a, b))
                .count();
            let suffix = old
                .iter()
                .rev()
                .zip(objs.iter().rev())
                .take(old.len().min(objs.len()) - prefix)
                .take_while(|((a, ..), b)| Arc::ptr_eq(a, b))
                .count();
            let old_tail = old.len() - suffix;
            let mut fresh = Vec::with_capacity(objs.len());
            fresh.extend(old.iter().take(prefix).cloned());
            fresh.extend(
                objs.iter()
                    .take(objs.len() - suffix)
                    .skip(prefix)
                    .map(|o| (Arc::clone(o), o.approx_bbox(), std::cell::Cell::new(None))),
            );
            fresh.extend(old.into_iter().skip(old_tail));
            *c = fresh;
        }
        self.bbox_cache.borrow()
    }

    /// The boxes of the top-level objects with fig2dev's bounds filled in
    /// (`fig2dev::bound` through the bridge), each computed once per
    /// changed object.
    fn fig_bounds(&self) -> std::cell::Ref<'_, Vec<BoxEntry>> {
        let boxes = self.object_bboxes();
        if boxes.iter().any(|(_, _, b)| b.get().is_none()) {
            let mut ctx = rustfig_format::bridge::from_model::context(&self.doc);
            for (o, _, b) in boxes.iter() {
                if b.get().is_none() {
                    b.set(Some(rustfig_format::bridge::object_bounds(o, &mut ctx)));
                }
            }
        }
        boxes
    }

    /// The bounds of the whole drawing as fig2dev computes them (what its
    /// drivers frame): used to fit the window and to report the size.
    pub fn doc_bounds(&self) -> Rect {
        self.fig_bounds()
            .iter()
            .fold(Rect::EMPTY, |r, (_, _, b)| r.union(&b.get().unwrap_or(Rect::EMPTY)))
    }

    /// The fig2dev bounds of one top-level object (a compound's corners in
    /// xfig's edit panel).
    pub fn object_fig_bounds(&self, id: ObjectId) -> Option<Rect> {
        self.fig_bounds()
            .iter()
            .find(|(o, _, _)| o.id == id)
            .and_then(|(_, _, b)| b.get())
    }

    /// Top-level objects (visible ones) whose fig2dev bounds lie inside `r`.
    pub fn objects_within(&self, r: &Rect) -> Vec<ObjectId> {
        self.objects_within_region(&Region::Rect(*r))
    }

    /// Top-level objects on active depths (a compound counts when any
    /// member is active, xfig's `active_layer`) whose fig2dev bounds lie
    /// completely inside the region.
    pub fn objects_within_region(&self, region: &Region) -> Vec<ObjectId> {
        self.fig_bounds()
            .iter()
            .filter(|(o, _, b)| self.is_visible(o) && region.contains_rect(&b.get().unwrap_or(Rect::EMPTY)))
            .map(|(o, ..)| o.id)
            .collect()
    }

    /// Visible top-level objects completely inside the region that `mask`
    /// lets through (the region of the functions on xfig's free buttons).
    pub fn objects_in_region(&self, region: &Region, mask: tools::ObjMask) -> Vec<ObjectId> {
        self.objects_within_region(region)
            .into_iter()
            .filter(|id| self.doc.get(*id).is_some_and(|o| mask.admits(o)))
            .collect()
    }

    /// The lines of text in the clipboard's text objects, if it holds any.
    pub fn clipboard_text(&self) -> Option<Vec<String>> {
        let lines: Vec<String> = self
            .clipboard
            .iter()
            .filter_map(|o| match &o.kind {
                ObjectKind::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect();
        (!lines.is_empty()).then_some(lines)
    }

    /// Topmost top-level object within `tol` Fig units of `p`.
    pub fn hit_test(&self, p: Point, tol: f64) -> Option<ObjectId> {
        self.hit_test_where(p, tol, &|_| true)
    }

    /// Topmost top-level object within `tol` of `p` that `accept` lets
    /// through (xfig's searches skip what the mode's mask excludes).
    pub fn hit_test_where(&self, p: Point, tol: f64, accept: &dyn Fn(&Object) -> bool) -> Option<ObjectId> {
        // A plain click on a located object (Shift+click) acts on it.
        if let Some(o) = self
            .locate_pick
            .and_then(|id| self.doc.get(id))
            .filter(|o| self.is_visible(o) && accept(o) && o.distance(p) <= tol)
        {
            return Some(o.id);
        }
        let boxes = self.object_bboxes();
        // (depth, xfig type rank, index): the front-most is the one drawn last.
        let mut best: Option<(i32, u8, usize)> = None;
        for (i, o) in self.doc.objects.iter().enumerate() {
            let Some((_, bb, _)) = boxes.get(i) else { break };
            if !bb.expand(tol).contains(p) || !self.is_visible(o) || !accept(o) {
                continue;
            }
            let d = o.distance(p);
            if d > tol {
                continue;
            }
            let key = (o.depth(), o.xfig_type_rank(), i);
            let better = match best {
                None => true,
                Some((bd, br, bi)) => key.0 < bd || (key.0 == bd && (key.1, key.2) > (br, bi)),
            };
            if better {
                best = Some(key);
            }
        }
        best.and_then(|(_, _, i)| self.doc.objects.get(i)).map(|o| o.id)
    }

    /// Every top-level object within `tol` of `p` that `accept` lets
    /// through, front-most first (xfig's `do_object_search` walks them).
    pub fn hits_where(&self, p: Point, tol: f64, accept: &dyn Fn(&Object) -> bool) -> Vec<ObjectId> {
        let boxes = self.object_bboxes();
        let mut found: Vec<(i32, u8, usize, ObjectId)> = Vec::new();
        for (i, o) in self.doc.objects.iter().enumerate() {
            let Some((_, bb, _)) = boxes.get(i) else { break };
            if !bb.expand(tol).contains(p) || !self.is_visible(o) || !accept(o) || o.distance(p) > tol {
                continue;
            }
            found.push((o.depth(), o.xfig_type_rank(), i, o.id));
        }
        found.sort_by(|a, b| a.0.cmp(&b.0).then((b.1, b.2).cmp(&(a.1, a.2))));
        found.into_iter().map(|f| f.3).collect()
    }

    /// After depths were switched on or off: objects that became hidden
    /// leave the selection, and work in progress is dropped (it may be on
    /// one of them).
    pub fn depths_changed(&mut self) {
        let sel: Vec<ObjectId> = self
            .selection
            .iter()
            .copied()
            .filter(|id| self.doc.get(*id).is_some_and(|o| self.is_visible(o)))
            .collect();
        self.selection = sel;
        if !self.is_typing() {
            self.tool_state = tools::ToolState::Idle;
        }
        self.bump_revision();
    }

    /// The units box text, as xfig's: "1cm = 1.00cm", or with a figure unit
    /// and scale "1cm = 2.00m".
    pub fn unit_label(&self) -> String {
        let ruler = self.doc().unit_name();
        let fig = if self.unit_scale.unit.is_empty() {
            ruler
        } else {
            self.unit_scale.unit.as_str()
        };
        format!("1{ruler} = {:.2}{fig}", self.unit_scale.factor)
    }

    /// Number of objects (leaves) at each depth in use, front to back.
    pub fn depth_counts(&self) -> Vec<(i32, usize)> {
        let mut m = std::collections::BTreeMap::new();
        self.doc
            .for_each_leaf(&mut |o| *m.entry(o.depth()).or_insert(0usize) += 1);
        m.into_iter().collect()
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
        let sel: std::collections::HashSet<ObjectId> = self.selection.iter().copied().collect();
        self.fig_bounds()
            .iter()
            .filter(|(o, ..)| sel.contains(&o.id))
            .fold(Rect::EMPTY, |r, (_, _, b)| r.union(&b.get().unwrap_or(Rect::EMPTY)))
    }

    /// A point typed exactly (`tool.point`): the next press there is not
    /// snapped or constrained.
    pub(crate) fn add_exact_point(&mut self, p: Point) {
        if self.exact_points.len() < 256 {
            self.exact_points.push(p);
        }
    }

    /// Snap a point according to the positioning mode.
    pub fn snap(&self, p: Point) -> Point {
        if self.exact_points.contains(&p) {
            return p;
        }
        self.limit_point(self.indicator.snap_point(p, self.doc.settings.units))
    }

    /// The Snap menu's mode, if one is armed.
    pub fn object_snap(&self) -> Option<ObjectSnap> {
        self.object_snap
    }

    /// Whether Snap › Hold keeps the mode after use.
    pub fn snap_held(&self) -> bool {
        self.snap_held
    }

    /// Arm a snap mode for the next point (or disarm with `None`).
    pub fn set_object_snap(&mut self, mode: Option<ObjectSnap>) {
        self.object_snap = mode;
        self.intersect_first = None;
    }

    /// Snap › Hold (keep the mode) or Snap › Release (`false`: also disarm).
    pub fn set_snap_held(&mut self, held: bool) {
        self.snap_held = held;
        if !held {
            self.set_object_snap(None);
        }
    }

    /// The Snap Mode indicator's text (xfig's labels).
    pub fn snap_indicator(&self) -> &'static str {
        match self.object_snap {
            None => "None",
            Some(ObjectSnap::Intersect) if self.intersect_first.is_some() => "I'sect 2",
            Some(m) => m.label(),
        }
    }

    /// The leaf object nearest `p` within `tol` on a visible depth.
    fn snap_target(&self, p: Point, tol: f64) -> Option<Object> {
        fn walk<'a, O: AsRef<Object>>(objs: &'a [O], e: &Engine, p: Point, best: &mut Option<(f64, &'a Object)>) {
            for o in objs {
                let o = o.as_ref();
                match &o.kind {
                    ObjectKind::Compound(c) => walk(&c.objects, e, p, best),
                    _ if !e.is_visible(o) => {}
                    _ => {
                        let d = o.distance(p);
                        if d.is_finite() && best.is_none_or(|(bd, _)| d < bd) {
                            *best = Some((d, o));
                        }
                    }
                }
            }
        }
        let mut best = None;
        walk(&self.doc.objects, self, p, &mut best);
        best.filter(|(d, _)| *d <= tol).map(|(_, o)| o.clone())
    }

    /// Apply the armed object snap to a press at `click`: the snapped
    /// point, or `None` when nothing was found (the press is then ignored,
    /// as in xfig, and the message says why).
    pub(crate) fn apply_object_snap(&mut self, click: Point, tol: f64, current: Option<Point>) -> Option<Point> {
        let mode = self.object_snap?;
        let Some(target) = self.snap_target(click, tol) else {
            self.set_status("No point found.");
            return None;
        };
        let result = if mode == ObjectSnap::Intersect {
            match self.intersect_first.take() {
                None => {
                    self.intersect_first = Some(target);
                    self.set_status("Select second object.");
                    return None;
                }
                Some(first) => snap::intersect(&first, &target, click),
            }
        } else {
            snap::snap(mode, &target, click, current)
        };
        match result {
            Ok(p) if p.is_finite() => {
                if !self.snap_held {
                    self.object_snap = None;
                }
                if self.exact_points.len() < 256 {
                    self.exact_points.push(p);
                }
                self.release_at = Some(p);
                Some(p)
            }
            Ok(_) => {
                self.set_status("No point found.");
                None
            }
            Err(msg) => {
                self.set_status(msg);
                None
            }
        }
    }
}

/// Recompute a text's stored height and length from font metrics.
/// Whether a text in `o` has metrics other than its font gives.
fn text_stale(o: &Object) -> bool {
    match &o.kind {
        ObjectKind::Text(t) => {
            let mut fixed = t.clone();
            fix_text_metrics(&mut fixed);
            fixed.length != t.length || fixed.height != t.height
        }
        ObjectKind::Compound(c) => c.objects.iter().any(text_stale),
        _ => false,
    }
}

/// Recompute the metrics of every text in `o`.
fn fix_texts(o: &mut Object) {
    match &mut o.kind {
        ObjectKind::Text(t) => fix_text_metrics(t),
        ObjectKind::Compound(c) => c.objects.iter_mut().for_each(fix_texts),
        _ => {}
    }
}

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
