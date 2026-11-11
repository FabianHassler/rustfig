//! The command registry. Every user-visible action is a command with an
//! id, label, menu path, shortcut, parameter documentation, an `enabled`
//! predicate and a `run` function.

mod annotate;
mod arrange;
mod chop;
mod chop_ext;
pub mod color;
mod compound;
mod digitize;
mod dimline;
mod doc;
mod edit;
mod export_panel;
pub(crate) mod file;
mod find;
mod indicator;
mod joinsplit;
mod library;
pub(crate) mod measure;
mod modes;
mod object;
mod outline;
pub mod picture;
pub(crate) mod plot;
mod prefs;
mod preview;
pub(crate) use preview::Preview;
mod print;
mod region;
mod sfactor;
pub(crate) mod shape;
mod spell;
mod styles;
pub(crate) mod tangent;

pub(crate) use dimline::formatter as dimension_formatter;
pub(crate) use region::circumcircle;
pub use shape::STYLE_PARAMS;
mod tool;
mod transform;
mod view;

use crate::{Engine, EngineError};
use rustfig_geom::Point;
use serde_json::Value;
use std::sync::OnceLock;

/// Result of running a command.
pub type CmdResult = Result<Value, EngineError>;

/// A command definition.
pub struct Command {
    /// Stable id, e.g. `edit.undo`.
    pub id: &'static str,
    /// Menu label.
    pub label: &'static str,
    /// Menu path (`"Edit"`, `"Object/Arrange"`), empty if not in a menu.
    pub menu: &'static str,
    /// Keyboard shortcut (`"Ctrl+Z"`), empty if none. `Ctrl` means Cmd on macOS.
    pub shortcut: &'static str,
    /// Parameter documentation (JSON-ish).
    pub params: &'static str,
    /// Whether the command changes the document (and is undoable).
    pub mutates: bool,
    pub enabled: fn(&Engine) -> bool,
    pub run: fn(&mut Engine, &Value) -> CmdResult,
}

pub(crate) fn always(_: &Engine) -> bool {
    true
}

pub(crate) fn has_objects(e: &Engine) -> bool {
    !e.doc().objects.is_empty()
}

/// All commands.
pub fn all() -> &'static [Command] {
    static ALL: OnceLock<Vec<Command>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut v = Vec::new();
        v.extend(file::commands());
        v.extend(digitize::commands());
        v.extend(export_panel::commands());
        v.extend(print::commands());
        v.extend(edit::commands());
        v.extend(find::commands());
        v.extend(view::commands());
        v.extend(shape::commands());
        v.extend(picture::commands());
        v.extend(transform::commands());
        v.extend(arrange::commands());
        v.extend(object::commands());
        v.extend(compound::commands());
        v.extend(chop::commands());
        v.extend(tangent::commands());
        v.extend(sfactor::commands());
        v.extend(measure::commands());
        v.extend(annotate::commands());
        v.extend(dimline::commands());
        v.extend(joinsplit::commands());
        v.extend(library::commands());
        v.extend(modes::commands());
        v.extend(plot::commands());
        v.extend(region::commands());
        v.extend(doc::commands());
        v.extend(color::commands());
        v.extend(indicator::commands());
        v.extend(tool::commands());
        v.extend(prefs::commands());
        v.extend(preview::commands());
        v.extend(styles::commands());
        v.extend(spell::commands());
        v
    })
}

/// Look a command up by id.
pub fn find(id: &str) -> Option<&'static Command> {
    all().iter().find(|c| c.id == id)
}

// ---- parameter helpers -------------------------------------------------

thread_local! {
    /// The parameter names the running command has looked at ([`param`]).
    static READ: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Most names one command run notes (a guard against loops over input).
const MAX_READ: usize = 1_000;

/// `p[key]`, noting that the command read `key`: a parameter it never
/// reads is reported as ignored ([`ignored_params`]). Every helper below
/// reads through it.
pub(crate) fn param<'a>(p: &'a Value, key: &str) -> Option<&'a Value> {
    READ.with(|r| {
        // (Already borrowed cannot happen; skipping the note only risks a
        // spurious "ignored" warning.)
        if let Ok(mut r) = r.try_borrow_mut() {
            if r.len() < MAX_READ && !r.iter().any(|k| k == key) {
                r.push(key.to_string());
            }
        }
    });
    p.get(key)
}

/// Starts noting the parameters a command reads; returns the names of an
/// outer command running it, for [`end_reading`].
pub(crate) fn begin_reading() -> Vec<String> {
    READ.with(|r| {
        r.try_borrow_mut()
            .map(|mut r| std::mem::take(&mut *r))
            .unwrap_or_default()
    })
}

/// The names read since [`begin_reading`]; the outer command's come back.
pub(crate) fn end_reading(outer: Vec<String>) -> Vec<String> {
    READ.with(|r| {
        r.try_borrow_mut()
            .map(|mut r| std::mem::replace(&mut *r, outer))
            .unwrap_or_default()
    })
}

/// The parameters given to `command` that it neither read nor documents
/// (null values count as not given), as warnings: a misspelt or wrong name
/// (`color` for `pen_color`) would otherwise do nothing without a word.
pub(crate) fn ignored_params(command: &Command, params: &Value, read: &[String]) -> Vec<String> {
    let Some(given) = params.as_object() else {
        return Vec::new();
    };
    let unread: Vec<&String> = given
        .iter()
        .filter(|(k, v)| !v.is_null() && !read.iter().any(|r| r == *k))
        .map(|(k, _)| k)
        .collect();
    if unread.is_empty() {
        return Vec::new();
    }
    // Also the names of the commands its documentation refers to ("as
    // file.export"), and the tools' `reappend`, read only when it matters.
    let mut documented = documented_params(command.params);
    let referred: Vec<&str> = command
        .params
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
        .filter(|w| w.contains('.') && *w != command.id)
        .filter_map(|w| find(w.trim_end_matches('.')).map(|c| c.params))
        .collect();
    for doc in referred {
        documented.extend(documented_params(doc));
    }
    documented.push("reappend");
    // `names` goes wherever `ids` does (see `Engine::target_ids`).
    if documented.contains(&"ids") {
        documented.push("names");
    }
    unread
        .into_iter()
        .filter(|k| !documented.contains(&k.as_str()))
        .map(|k| format!("parameter {k:?} ignored: {} does not take it", command.id))
        .collect()
}

/// The quoted names in a command's parameter documentation.
fn documented_params(doc: &str) -> Vec<&str> {
    // (Every piece between quotes that is a name: unbalanced quotes in a
    // text cannot shift the names out.)
    doc.split('"')
        .filter(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .collect()
}

pub(crate) fn num(p: &Value, key: &str) -> Result<f64, EngineError> {
    opt_num(p, key)?.ok_or_else(|| EngineError::bad_params(format!("missing number parameter {key:?}")))
}

pub(crate) fn opt_num(p: &Value, key: &str) -> Result<Option<f64>, EngineError> {
    match param(p, key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => match v.as_f64() {
            Some(f) if f.is_finite() && f.abs() < 1.0e9 => Ok(Some(f)),
            _ => Err(EngineError::bad_params(format!(
                "parameter {key:?} must be a finite number"
            ))),
        },
    }
}

pub(crate) fn num_or(p: &Value, key: &str, default: f64) -> Result<f64, EngineError> {
    Ok(opt_num(p, key)?.unwrap_or(default))
}

pub(crate) fn opt_str<'a>(p: &'a Value, key: &str) -> Result<Option<&'a str>, EngineError> {
    match param(p, key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.as_str())),
        Some(_) => Err(EngineError::bad_params(format!("parameter {key:?} must be a string"))),
    }
}

pub(crate) fn string(p: &Value, key: &str) -> Result<String, EngineError> {
    opt_str(p, key)?
        .map(str::to_string)
        .ok_or_else(|| EngineError::bad_params(format!("missing string parameter {key:?}")))
}

pub(crate) fn opt_bool(p: &Value, key: &str) -> Result<Option<bool>, EngineError> {
    match param(p, key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(EngineError::bad_params(format!(
            "parameter {key:?} must be true or false"
        ))),
    }
}

/// Largest point list accepted from parameters.
pub const MAX_PARAM_POINTS: usize = 1_000_000;

/// `[[x, y], ...]` or `[{"x":..,"y":..}, ...]`.
pub(crate) fn points(p: &Value, key: &str) -> Result<Vec<Point>, EngineError> {
    let Some(Value::Array(a)) = param(p, key) else {
        return Err(EngineError::bad_params(format!("missing point list {key:?}")));
    };
    if a.len() > MAX_PARAM_POINTS {
        return Err(EngineError::bad_params("too many points"));
    }
    a.iter().map(point_value).collect()
}

pub(crate) fn point_value(v: &Value) -> Result<Point, EngineError> {
    let (x, y) = match v {
        Value::Array(xy) => (xy.first().and_then(Value::as_f64), xy.get(1).and_then(Value::as_f64)),
        Value::Object(_) => (v.get("x").and_then(Value::as_f64), v.get("y").and_then(Value::as_f64)),
        _ => (None, None),
    };
    match (x, y) {
        (Some(x), Some(y)) if x.is_finite() && y.is_finite() && x.abs() < 1.0e9 && y.abs() < 1.0e9 => {
            Ok(Point::new(x, y))
        }
        _ => Err(EngineError::bad_params("points must be [x, y] pairs of finite numbers")),
    }
}

pub(crate) fn ids_value(ids: &[rustfig_model::ObjectId]) -> Value {
    Value::Array(ids.iter().map(|i| Value::from(i.0)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_documented() {
        let mut seen = std::collections::BTreeSet::new();
        for c in all() {
            assert!(seen.insert(c.id), "duplicate command id {}", c.id);
            assert!(!c.label.is_empty(), "{} has no label", c.id);
            assert!(c.id.contains('.'), "{} is not namespaced", c.id);
        }
    }

    #[test]
    fn shortcuts_are_unique() {
        let mut seen = std::collections::BTreeMap::new();
        for c in all() {
            if !c.shortcut.is_empty() {
                if let Some(prev) = seen.insert(c.shortcut, c.id) {
                    panic!("shortcut {} used by {} and {}", c.shortcut, prev, c.id);
                }
            }
        }
    }
}
