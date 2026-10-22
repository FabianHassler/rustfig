//! Shape factor of a spline point (plan S4-6): one step towards approximating
//! or interpolating, or the next of the three shapes, turning the spline into
//! an X-spline.
//
// Ported from xfig src/e_edit.c (`change_sfactor`) (xfig licence, see
// LICENSE-xfig).

use super::{num, opt_str, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_model::{ObjectId, ObjectKind, SplineKind};
use serde_json::{json, Value};

/// xfig's `2*STEP_VALUE` (`STEP_VALUE` is 0.02).
const STEP: f64 = 0.04;
const APPROX: f64 = 1.0;
const INTERP: f64 = -1.0;
const ANGULAR: f64 = 0.0;

pub(super) fn commands() -> Vec<Command> {
    vec![Command {
        id: "spline.shape_factor",
        label: "Change shape factor",
        menu: "",
        shortcut: "",
        params: "{\"id\": spline, \"index\": point, \"action\"?: \"approximate\" (default; shape +0.04, up to 1), \"cycle\" (next of interpolating -1, angular 0, approximating 1) or \"interpolate\" (shape -0.04, down to -1)}  — change the shape factor of one point and make the spline an X-spline (xfig's Ctrl+Shift+click on a spline point: left, middle, right button). The end points of an open spline are refused. Returns {\"shape\"}",
        mutates: true,
        enabled: super::always,
        run: shape_factor,
    }]
}

/// The new shape factor for a button's action (`change_sfactor`).
fn next_shape(action: &str, s: f64) -> Result<f64, EngineError> {
    Ok(match action {
        "approximate" => (s + STEP).min(APPROX),
        "cycle" => {
            let n = s.round() + (APPROX - ANGULAR);
            if n > APPROX {
                INTERP
            } else {
                n
            }
        }
        "interpolate" => (s - STEP).max(INTERP),
        _ => {
            return Err(EngineError::bad_params(
                "action must be approximate, cycle or interpolate",
            ))
        }
    })
}

fn shape_factor(e: &mut Engine, p: &Value) -> CmdResult {
    let id = ObjectId(num(p, "id")?.round().max(0.0) as u64);
    let index = num(p, "index")?;
    if index < 0.0 {
        return Err(EngineError::bad_params("index must not be negative"));
    }
    let index = index as usize;
    let action = opt_str(p, "action")?.unwrap_or("approximate");
    let o = e
        .doc_mut()
        .get_mut(id)
        .ok_or_else(|| EngineError::not_found(format!("no top-level object with id {}", id.0)))?;
    let ObjectKind::Spline(s) = &mut o.kind else {
        return Err(EngineError::bad_params("Only spline points can be edited"));
    };
    if index >= s.points.len() {
        return Err(EngineError::bad_params("no such point"));
    }
    if !s.closed && (index == 0 || index + 1 == s.points.len()) {
        return Err(EngineError::bad_params("Cannot edit end-points"));
    }
    // Every spline keeps one shape factor per point; repair a short list.
    let fill = s.kind.default_shape();
    if s.shapes.len() < s.points.len() {
        s.shapes.resize(s.points.len(), fill);
    }
    let cur = s.shapes.get(index).copied().unwrap_or(fill);
    let new = next_shape(action, cur)?;
    if let Some(v) = s.shapes.get_mut(index) {
        *v = new;
    }
    s.kind = SplineKind::X;
    Ok(json!({"shape": new}))
}
