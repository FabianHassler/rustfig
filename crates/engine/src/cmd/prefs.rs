//! Preferences (E1): xfig's resources, RustFig's settings, user colours.

use super::{always, string, CmdResult, Command};
use crate::prefs::{resource, Kind, NamedColor, Status, MAX_USER_COLORS, RESOURCES};
use crate::{Engine, EngineError};
use rustfig_model::Rgb;
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "prefs.get",
            label: "Preferences",
            menu: "",
            shortcut: "",
            params: "{}  — returns {\"xfig\": {resource: value}, \"rustfig\": {...}, \"resources\": [{\"name\", \"type\", \"default\", \"option\", \"status\", \"task\", \"value\"}]}: xfig's resources with what RustFig does with each",
            mutates: false,
            enabled: always,
            run: get,
        },
        Command {
            id: "prefs.set",
            label: "Set preference",
            menu: "",
            shortcut: "",
            params: "{\"resource\": xfig resource name, \"value\": bool | number | string | null}  — null removes it (xfig's default applies at the next start); applied at once where RustFig uses it",
            mutates: false,
            enabled: always,
            run: set,
        },
        Command {
            id: "prefs.use_current_attributes",
            label: "Use current attributes as defaults",
            menu: "",
            shortcut: "",
            params: "{}  — the attribute panel's values become xfig's start resources (startfontsize, startlinewidth, …)",
            mutates: false,
            enabled: always,
            run: use_current,
        },
        Command {
            id: "prefs.add_color",
            label: "Add user colour",
            menu: "",
            shortcut: "",
            params: "{\"name\": string, \"rgb\": \"#rrggbb\"}  — a colour kept for every drawing (replaces one of the same name)",
            mutates: false,
            enabled: always,
            run: add_color,
        },
        Command {
            id: "prefs.remove_color",
            label: "Remove user colour",
            menu: "",
            shortcut: "",
            params: "{\"name\": string}",
            mutates: false,
            enabled: always,
            run: remove_color,
        },
    ]
}

fn to_json(v: &toml::Value) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

fn get(e: &mut Engine, _: &Value) -> CmdResult {
    let resources: Vec<Value> = RESOURCES
        .iter()
        .map(|r| {
            let (status, task) = match r.status {
                Status::Applied => ("applied", None),
                Status::Planned(t) => ("planned", Some(t)),
            };
            json!({
                "name": r.name,
                "type": format!("{:?}", r.kind).to_lowercase(),
                "default": r.default,
                "option": r.option,
                "status": status,
                "task": task,
                "value": e.prefs.get(r.name).as_ref().map(to_json),
            })
        })
        .collect();
    let rustfig = serde_json::to_value(&e.prefs.rustfig).unwrap_or(Value::Null);
    let xfig: serde_json::Map<String, Value> = e.prefs.xfig.iter().map(|(k, v)| (k.clone(), to_json(v))).collect();
    Ok(json!({"xfig": xfig, "rustfig": rustfig, "resources": resources}))
}

fn set(e: &mut Engine, p: &Value) -> CmdResult {
    let name = string(p, "resource")?;
    let r = resource(&name).ok_or_else(|| EngineError::bad_params(format!("unknown xfig resource {name:?}")))?;
    let value = match p.get("value") {
        None | Some(Value::Null) => {
            e.prefs.xfig.retain(|k, _| !k.eq_ignore_ascii_case(r.name));
            e.prefs_dirty = true;
            return Ok(json!({"resource": r.name, "value": null}));
        }
        Some(Value::Bool(b)) => toml::Value::Boolean(*b),
        Some(Value::String(s)) => toml::Value::String(s.clone()),
        Some(Value::Number(n)) => match (r.kind, n.as_i64(), n.as_f64()) {
            (Kind::Int, Some(i), _) => toml::Value::Integer(i),
            (_, _, Some(f)) if f.is_finite() => toml::Value::Float(f),
            _ => return Err(EngineError::bad_params("value is not a finite number")),
        },
        Some(_) => return Err(EngineError::bad_params("value must be a boolean, number or string")),
    };
    e.prefs.set(r.name, value).map_err(EngineError::bad_params)?;
    e.prefs_dirty = true;
    let max = e.prefs.recent_max();
    e.prefs.rustfig.recent_files.truncate(max);
    if !e.allow_negative_coords() {
        // xfig pans to the origin when negative coordinates are switched off.
        e.view.origin.x = e.view.origin.x.max(0.0);
        e.view.origin.y = e.view.origin.y.max(0.0);
    }
    let warnings = if r.status == Status::Applied {
        let prefs = e.prefs.clone();
        prefs.apply(e)
    } else {
        Vec::new()
    };
    Ok(json!({"resource": r.name, "value": e.prefs.get(r.name).as_ref().map(to_json), "warnings": warnings}))
}

fn use_current(e: &mut Engine, _: &Value) -> CmdResult {
    let mut prefs = std::mem::take(&mut e.prefs);
    prefs.take_attributes(e);
    e.prefs = prefs;
    e.prefs_dirty = true;
    e.set_status("The current attributes are now the defaults");
    Ok(json!({}))
}

fn add_color(e: &mut Engine, p: &Value) -> CmdResult {
    let name = string(p, "name")?.trim().to_string();
    if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err(EngineError::bad_params("a colour needs a name of 1-64 characters"));
    }
    let rgb = Rgb::from_hex(&string(p, "rgb")?).ok_or_else(|| EngineError::bad_params("rgb must be #rrggbb"))?;
    let colors = &mut e.prefs.rustfig.user_colors;
    colors.retain(|c| c.name != name);
    if colors.len() >= MAX_USER_COLORS {
        return Err(EngineError::bad_params(format!(
            "at most {MAX_USER_COLORS} user colours"
        )));
    }
    colors.push(NamedColor {
        name,
        rgb: rgb.to_hex(),
    });
    e.prefs_dirty = true;
    Ok(json!({"count": colors.len()}))
}

fn remove_color(e: &mut Engine, p: &Value) -> CmdResult {
    let name = string(p, "name")?;
    let colors = &mut e.prefs.rustfig.user_colors;
    let before = colors.len();
    colors.retain(|c| c.name != name);
    if colors.len() == before {
        return Err(EngineError::bad_params(format!("no user colour {name:?}")));
    }
    e.prefs_dirty = true;
    Ok(json!({"count": colors.len()}))
}
