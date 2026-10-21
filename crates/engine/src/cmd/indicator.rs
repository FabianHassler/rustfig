//! Current attributes (the indicator panel).

use super::{always, CmdResult, Command};
use crate::{Engine, EngineError, Indicator, UpdateFlag};
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "indicator.get",
            label: "Get current attributes",
            menu: "",
            shortcut: "",
            params: "{}",
            mutates: false,
            enabled: always,
            run: |e, _| serde_json::to_value(&e.indicator).map_err(|err| EngineError::internal(err.to_string())),
        },
        Command {
            id: "indicator.set",
            label: "Set current attributes",
            menu: "",
            shortcut: "",
            params: "{any indicator field: line, thickness, pen_color, fill_color, area_fill, depth, style_val, cap, join, forward_arrow, backward_arrow, arrow, font, font_size, postscript, justify, text_angle, box_radius, dot_gap, sides, rotate_angle, grid, snap, align_h, align_v, pie_arc, text_step, linkmode: off|move|slide}",
            mutates: false,
            enabled: always,
            run: set,
        },
        Command {
            id: "indicator.set_update_flag",
            label: "Set update flag",
            menu: "",
            shortcut: "",
            params: "{\"attr\": text_just|font|font_size|pen_color|fill_color|fill_style|line_style|join_style|cap_style|line_width|depth|text_angle|text_flags|arrow_mode|arrow_type|arrow_size|box_radius|arc_type, \"on\"?: bool (default: toggle)}  — whether Update (and update region, update settings) copies this attribute (xfig's update box on the attribute button)",
            mutates: false,
            enabled: always,
            run: set_update_flag,
        },
        Command {
            id: "indicator.update_flags",
            label: "Update control",
            menu: "",
            shortcut: "",
            params: "{\"all\": true | false | \"toggle\"}  — set, clear or toggle every update flag (xfig's Update Control buttons); returns the flags that are on",
            mutates: false,
            enabled: always,
            run: update_flags,
        },
        Command {
            id: "indicator.reset",
            label: "Reset current attributes",
            menu: "",
            shortcut: "",
            params: "{}",
            mutates: false,
            enabled: always,
            run: |e, _| {
                e.indicator = Indicator::default();
                Ok(json!({}))
            },
        },
    ]
}

fn set(e: &mut Engine, p: &Value) -> CmdResult {
    let Value::Object(changes) = p else {
        return Err(EngineError::bad_params("expected an object of indicator fields"));
    };
    let mut cur = serde_json::to_value(&e.indicator).map_err(|err| EngineError::internal(err.to_string()))?;
    let Value::Object(fields) = &mut cur else {
        return Err(EngineError::internal("indicator is not an object"));
    };
    for (k, v) in changes {
        if !fields.contains_key(k) {
            return Err(EngineError::bad_params(format!("unknown indicator field {k:?}")));
        }
        fields.insert(k.clone(), v.clone());
    }
    let mut ind: Indicator =
        serde_json::from_value(cur).map_err(|err| EngineError::bad_params(format!("bad indicator value: {err}")))?;
    ind.thickness = ind.thickness.clamp(0, 10_000);
    ind.depth = ind.depth.clamp(0, rustfig_model::MAX_DEPTH);
    ind.area_fill = ind.area_fill.clamp(-1, 62);
    ind.sides = ind.sides.clamp(3, 1000);
    if !ind.font_size.is_finite() {
        ind.font_size = 12.0;
    }
    ind.font_size = ind.font_size.clamp(1.0, 2000.0);
    if !ind.text_angle.is_finite() {
        ind.text_angle = 0.0;
    }
    if !ind.rotate_angle.is_finite() {
        ind.rotate_angle = 90.0;
    }
    if !ind.style_val.is_finite() {
        ind.style_val = 4.0;
    }
    if !ind.dot_gap.is_finite() {
        ind.dot_gap = 3.0;
    }
    e.indicator = ind;
    Ok(json!({}))
}

fn set_update_flag(e: &mut Engine, p: &Value) -> CmdResult {
    let name = p
        .get("attr")
        .and_then(Value::as_str)
        .ok_or_else(|| EngineError::bad_params("attr must be an update flag name"))?;
    let flag =
        UpdateFlag::from_name(name).ok_or_else(|| EngineError::bad_params(format!("unknown update flag {name:?}")))?;
    let on = match p.get("on") {
        None | Some(Value::Null) => !e.indicator.updates(flag),
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err(EngineError::bad_params("on must be true or false")),
    };
    e.indicator.set_update_flag(flag, on);
    Ok(json!({"attr": flag.name(), "on": on}))
}

fn update_flags(e: &mut Engine, p: &Value) -> CmdResult {
    match p.get("all") {
        Some(Value::Bool(true)) => {
            e.indicator.update_off.clear();
            e.set_status("Update commands are now ENABLED for all buttons");
        }
        Some(Value::Bool(false)) => {
            e.indicator.update_off = UpdateFlag::ALL.into_iter().collect();
            e.set_status("Update commands will be IGNORED for all buttons");
        }
        Some(Value::String(s)) if s == "toggle" => {
            e.indicator.update_off = UpdateFlag::ALL
                .into_iter()
                .filter(|f| e.indicator.updates(*f))
                .collect();
            e.set_status("Update command status TOGGLED for all buttons");
        }
        _ => return Err(EngineError::bad_params("all must be true, false or \"toggle\"")),
    }
    let on: Vec<String> = UpdateFlag::ALL
        .into_iter()
        .filter(|f| e.indicator.updates(*f))
        .map(UpdateFlag::name)
        .collect();
    Ok(json!({"on": on}))
}
