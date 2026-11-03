//! Favorite modes: the mode panel's Favorites strip (kept in the preferences).

use super::{always, opt_bool, string, CmdResult, Command};
use crate::tools::ToolKind;
use crate::{Engine, EngineError};
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "mode.favourite",
            label: "Favorite mode",
            menu: "",
            shortcut: "",
            params: "{\"mode\": tool id (\"tool.box\" or \"box\"), \"on\"?: bool (default: toggle)}  — adds the mode to the Favorites strip at the top of the mode panel or removes it (right-click a mode button; at most 8, kept in the preferences): {\"favorite\": bool, \"count\": n}",
            mutates: false,
            enabled: always,
            run: favorite,
        },
        Command {
            id: "mode.favourites",
            label: "List favorite modes",
            menu: "",
            shortcut: "",
            params: "{}  — the favorite modes, oldest first: {\"modes\": [{\"id\", \"label\"}]}",
            mutates: false,
            enabled: always,
            run: |e, _| Ok(json!({"modes": listed(e)})),
        },
    ]
}

/// Whether a mode has a button in the mode panel (and so can be a favorite).
fn has_button(t: ToolKind) -> bool {
    !matches!(
        t,
        ToolKind::None
            | ToolKind::EllipseRadii
            | ToolKind::EllipseDiameter
            | ToolKind::XSpline
            | ToolKind::XSplineClosed
            | ToolKind::OpenClose
    )
}

fn listed(e: &Engine) -> Vec<Value> {
    e.prefs
        .rustfig
        .mode_favourites
        .iter()
        .filter_map(|id| ToolKind::from_id(id))
        .filter(|t| has_button(*t))
        .map(|t| json!({"id": t.id(), "label": t.label()}))
        .collect()
}

fn favorite(e: &mut Engine, p: &Value) -> CmdResult {
    let name = string(p, "mode")?;
    let id = if name.starts_with("tool.") {
        name.clone()
    } else {
        format!("tool.{name}")
    };
    let tool = ToolKind::from_id(&id)
        .filter(|t| has_button(*t))
        .ok_or_else(|| EngineError::bad_params(format!("no mode {name:?} in the mode panel")))?;
    let now = e.prefs.rustfig.mode_favourites.iter().any(|x| x == tool.id());
    let on = opt_bool(p, "on")?.unwrap_or(!now);
    if !e.prefs.set_mode_favorite(tool.id(), on) {
        return Err(EngineError::bad_params("too many favorite modes: remove one first"));
    }
    e.prefs_dirty = true;
    Ok(json!({"favourite": on, "count": e.prefs.rustfig.mode_favourites.len()}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_remove_toggle_and_limit() {
        let mut e = Engine::new();
        let r = favorite(&mut e, &json!({"mode": "box"})).unwrap();
        assert_eq!(r["favourite"], true);
        favorite(&mut e, &json!({"mode": "tool.arc", "on": true})).unwrap();
        assert_eq!(listed(&e).len(), 2);
        favorite(&mut e, &json!({"mode": "tool.box"})).unwrap();
        assert_eq!(listed(&e)[0]["id"], "tool.arc");
        assert!(favorite(&mut e, &json!({"mode": "none"})).is_err());
        assert!(favorite(&mut e, &json!({"mode": "xspline"})).is_err());
        for m in ["box", "move", "copy", "text", "glue", "align", "scale"] {
            favorite(&mut e, &json!({"mode": m, "on": true})).unwrap();
        }
        assert!(favorite(&mut e, &json!({"mode": "flip_vertical"})).is_err());
    }
}
