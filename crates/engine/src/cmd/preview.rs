//! Preview of attribute changes (`preview.set`, `preview.clear`): the front
//! end shows what a choice would do to objects while the pointer rests on it,
//! without touching the document, the undo history or the selection. The
//! changed copies are drawn through [`Engine::overlay`] in place of the
//! originals; the preview ends at the next document change, mode change or
//! `preview.clear`.

use super::{always, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_model::Object;
use serde_json::{json, Value};

/// Most objects one preview shows (a larger set is refused, not cut).
const MAX_PREVIEW: usize = 20_000;

/// The changed copies of the objects, valid for one revision of the document.
#[derive(Debug, Clone)]
pub(crate) struct Preview {
    pub(crate) rev: u64,
    pub(crate) objects: Vec<Object>,
}

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "preview.set",
            label: "Preview attributes",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id] (default: the selection), \"attrs\"?: object as object.set_attributes without ids, \"indicator\"?: object as indicator.set (the objects as Update would leave them with that change to the current attributes)}  — shows the objects changed on the canvas without changing the document or the undo history; ends at the next document change, mode change or preview.clear",
            mutates: false,
            enabled: always,
            run: set,
        },
        Command {
            id: "preview.clear",
            label: "Clear preview",
            menu: "",
            shortcut: "",
            params: "{}  — ends the preview; the objects show as they are",
            mutates: false,
            enabled: always,
            run: clear,
        },
    ]
}

fn clear(e: &mut Engine, _: &Value) -> CmdResult {
    let was = e.preview.take().is_some();
    Ok(json!({"cleared": was}))
}

fn set(e: &mut Engine, p: &Value) -> CmdResult {
    let attrs = p.get("attrs").filter(|v| !v.is_null());
    let indicator = p.get("indicator").filter(|v| !v.is_null());
    if attrs.is_none() && indicator.is_none() {
        return Err(EngineError::bad_params("preview.set needs \"attrs\" or \"indicator\""));
    }
    for (name, v) in [("attrs", attrs), ("indicator", indicator)] {
        if v.is_some_and(|v| !v.is_object()) {
            return Err(EngineError::bad_params(format!("\"{name}\" must be an object")));
        }
    }
    let ids = e.target_ids(p)?;
    if ids.is_empty() {
        return Err(EngineError::bad_params("no objects to preview"));
    }
    if ids.len() > MAX_PREVIEW {
        return Err(EngineError::bad_params("too many objects to preview"));
    }
    // Run the real change on the document and put everything back: the
    // preview is exactly what the command would do.
    let saved = (e.doc.clone(), e.selection.clone(), e.changed, e.indicator.clone());
    let result = changed_copies(e, &ids, attrs, indicator);
    (e.doc, e.selection, e.changed, e.indicator) = saved;
    let objects = result?;
    let n = objects.len();
    e.preview = Some(Preview {
        rev: e.revision(),
        objects,
    });
    Ok(json!({"count": n}))
}

fn changed_copies(
    e: &mut Engine,
    ids: &[rustfig_model::ObjectId],
    attrs: Option<&Value>,
    indicator: Option<&Value>,
) -> Result<Vec<Object>, EngineError> {
    let id_list = json!(ids.iter().map(|i| i.0).collect::<Vec<_>>());
    if let Some(ind) = indicator {
        let cmd = super::find("indicator.set").ok_or_else(|| EngineError::internal("indicator.set is missing"))?;
        (cmd.run)(e, ind)?;
        let ind = e.indicator.clone();
        super::object::for_targets(e, &json!({ "ids": id_list }), &mut |o| ind.update_object(o))?;
    }
    if let Some(a) = attrs {
        let mut a = a.clone();
        if let Some(m) = a.as_object_mut() {
            m.insert("ids".into(), id_list.clone());
        }
        super::object::set_attributes_inner(e, &a)?;
    }
    Ok(ids.iter().filter_map(|id| e.doc.get(*id).cloned()).collect())
}

impl Engine {
    /// The ids and changed copies of a live preview, if one is showing for the
    /// current state of the document.
    pub(crate) fn live_preview(&self) -> Option<&Preview> {
        self.preview.as_ref().filter(|p| p.rev == self.revision())
    }

    /// Whether a preview is showing (for the UI's bookkeeping).
    pub fn preview_active(&self) -> bool {
        self.live_preview().is_some()
    }
}
