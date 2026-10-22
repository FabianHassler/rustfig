//! Open compound (plan D1): edit the members of a compound in place.
//!
//! xfig's "open compound" (`e_compound.c`) replaces the drawing by the
//! members of the compound, so every mode edits inside it without taking it
//! apart; closing puts them back. Here the drawing around the compound waits
//! in `Document::open` (model), so `doc().objects` is the open level and
//! every command and tool works on it unchanged; saving and exporting go
//! through `Engine::whole_doc`.
//
// Ported from xfig src/e_compound.c (xfig licence, see LICENSE-xfig).

use super::{opt_bool, CmdResult, Command};
use crate::tools::ToolKind;
use crate::{Engine, EngineError};
use rustfig_model::{ObjectId, ObjectKind};
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "compound.open",
            label: "Open Compound",
            menu: "Object",
            shortcut: "",
            params: "{\"id\"?: top-level compound (default: the selected one), \"keep_visible\"?: bool (xfig's middle button: the rest of the figure stays visible, gray)}  — edit the compound's members in place; opens nest",
            mutates: true,
            enabled: |_| true,
            run: open,
        },
        Command {
            id: "compound.close",
            label: "Close This Compound",
            menu: "Object",
            shortcut: "",
            params: "{}  — close the innermost open compound; one that lost all its members is removed",
            mutates: true,
            enabled: |e| e.open_compounds() > 0,
            run: close,
        },
        Command {
            id: "compound.close_all",
            label: "Close All Compounds",
            menu: "Object",
            shortcut: "",
            params: "{}  — close every open compound",
            mutates: true,
            enabled: |e| e.open_compounds() > 0,
            run: close_all,
        },
    ]
}

/// xfig refuses to open or close while an object is being drawn
/// (`check_action_on`).
fn idle(e: &Engine) -> Result<(), EngineError> {
    if e.tool_busy() {
        Err(EngineError::bad_params(
            "finish or cancel the action in progress before opening or closing a compound",
        ))
    } else {
        Ok(())
    }
}

fn open(e: &mut Engine, p: &Value) -> CmdResult {
    idle(e)?;
    let id = match p.get("id") {
        None | Some(Value::Null) => match e.selection() {
            [one] => *one,
            _ => {
                return Err(EngineError::bad_params(
                    "give the compound's \"id\" or select exactly one",
                ))
            }
        },
        Some(v) => ObjectId(
            v.as_u64()
                .ok_or_else(|| EngineError::bad_params("\"id\" must be an object id"))?,
        ),
    };
    let keep = opt_bool(p, "keep_visible")?.unwrap_or(false);
    match e.doc().get(id).map(|o| &o.kind) {
        Some(ObjectKind::Compound(_)) => {}
        Some(_) => return Err(EngineError::bad_params(format!("object {} is not a compound", id.0))),
        None => return Err(EngineError::not_found(format!("no top-level object with id {}", id.0))),
    }
    if !e.doc_mut().open_compound(id, keep) {
        return Err(EngineError::bad_params("too many compounds are open"));
    }
    e.set_selection(Vec::new());
    let depth = e.open_compounds();
    e.set_status(format!("Compound opened ({depth} open)"));
    Ok(json!({"id": id.0, "open": depth, "keep_visible": keep}))
}

fn close(e: &mut Engine, _: &Value) -> CmdResult {
    idle(e)?;
    let id = e
        .doc_mut()
        .close_compound()
        .ok_or_else(|| EngineError::bad_params("no compound is open"))?;
    // xfig's close_compound recomputes a dimension line that was opened
    // and edited (`rescale_dimension_line`).
    super::dimline::recompute_objects(e, &[id]);
    e.set_selection(Vec::new());
    // xfig selects the open-compound mode again after a close.
    e.set_tool(ToolKind::OpenCompound);
    let still_there = e.doc().get(id).is_some();
    Ok(json!({"id": id.0, "open": e.open_compounds(), "removed_empty": !still_there}))
}

fn close_all(e: &mut Engine, _: &Value) -> CmdResult {
    idle(e)?;
    let n = e.open_compounds();
    if n == 0 {
        return Err(EngineError::bad_params("no compound is open"));
    }
    e.doc_mut().close_all();
    let all: Vec<_> = e.doc().objects.iter().map(|o| o.id).collect();
    super::dimline::recompute_objects(e, &all);
    e.set_selection(Vec::new());
    e.set_tool(ToolKind::OpenCompound);
    Ok(json!({"closed": n}))
}
