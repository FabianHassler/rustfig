//! Current attributes (the indicator panel).

use super::{always, CmdResult, Command};
use crate::{Engine, EngineError, Indicator};
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
            params: "{any indicator field: line, thickness, pen_color, fill_color, area_fill, depth, style_val, cap, join, forward_arrow, backward_arrow, arrow, font, font_size, postscript, justify, text_angle, box_radius, sides, rotate_angle, grid, snap, align_h, align_v, pie_arc, text_step}",
            mutates: false,
            enabled: always,
            run: set,
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
    e.indicator = ind;
    Ok(json!({}))
}
