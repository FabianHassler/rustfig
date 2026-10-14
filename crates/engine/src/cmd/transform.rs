//! Geometric transforms of objects.

use super::{has_selection, num, num_or, opt_bool, opt_num, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_geom::{Affine, Point};
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "transform.move",
            label: "Move by",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id], \"dx\", \"dy\"}  — Fig units",
            mutates: true,
            enabled: has_selection_or_ids,
            run: move_by,
        },
        Command {
            id: "transform.scale",
            label: "Scale",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id], \"sx\", \"sy\"?, \"cx\"?, \"cy\"?, \"scale_text\"?: bool}  — about the centre of the objects by default",
            mutates: true,
            enabled: has_selection_or_ids,
            run: scale,
        },
        Command {
            id: "transform.rotate",
            label: "Rotate",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id], \"angle\": degrees counter-clockwise, \"cx\"?, \"cy\"?, \"copy\"?: bool}  — copy: rotate copies, keep the originals",
            mutates: true,
            enabled: has_selection_or_ids,
            run: rotate,
        },
        Command {
            id: "transform.rotate_ccw",
            label: "Rotate Counter-clockwise",
            menu: "Object/Transform",
            shortcut: "Ctrl+L",
            params: "{\"ids\"?: [id]}  — by the indicator's rotation angle",
            mutates: true,
            enabled: has_selection,
            run: |e, p| {
                let a = e.indicator.rotate_angle;
                rotate_by(e, p, a)
            },
        },
        Command {
            id: "transform.rotate_cw",
            label: "Rotate Clockwise",
            menu: "Object/Transform",
            shortcut: "Ctrl+R",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: has_selection,
            run: |e, p| {
                let a = -e.indicator.rotate_angle;
                rotate_by(e, p, a)
            },
        },
        Command {
            id: "transform.flip_horizontal",
            label: "Flip Left/Right",
            menu: "Object/Transform",
            shortcut: "Ctrl+Shift+H",
            params: "{\"ids\"?: [id], \"cx\"?, \"copy\"?: bool}  — copy: flip copies, keep the originals",
            mutates: true,
            enabled: has_selection,
            run: |e, p| flip(e, p, true),
        },
        Command {
            id: "transform.flip_vertical",
            label: "Flip Up/Down",
            menu: "Object/Transform",
            shortcut: "Ctrl+Shift+V",
            params: "{\"ids\"?: [id], \"cy\"?, \"copy\"?: bool}",
            mutates: true,
            enabled: has_selection,
            run: |e, p| flip(e, p, false),
        },
    ]
}

fn has_selection_or_ids(_: &Engine) -> bool {
    true
}

/// Apply a transform to the target objects.
pub(crate) fn apply(e: &mut Engine, p: &Value, t: Affine, scale_text: bool) -> CmdResult {
    let ids = e.target_ids(p)?;
    if ids.is_empty() {
        return Err(EngineError::bad_params("no objects selected"));
    }
    if opt_bool(p, "copy")?.unwrap_or(false) {
        let copies: Vec<_> = ids.iter().filter_map(|i| e.doc().get(*i).cloned()).collect();
        let mut new_ids = Vec::new();
        for mut o in copies {
            o.transform(&t, scale_text);
            new_ids.push(e.add_object(o));
        }
        e.set_selection(new_ids.clone());
        return Ok(json!({"count": new_ids.len(), "ids": super::ids_value(&new_ids)}));
    }
    for o in e.doc_mut().objects.iter_mut() {
        if ids.contains(&o.id) {
            Arc::make_mut(o).transform(&t, scale_text);
        }
    }
    Ok(json!({"count": ids.len()}))
}

fn center(e: &Engine, p: &Value) -> Result<Point, EngineError> {
    let ids = e.target_ids(p)?;
    let bb = ids
        .iter()
        .filter_map(|i| e.doc().get(*i))
        .fold(rustfig_geom::Rect::EMPTY, |r, o| r.union(&o.geometry_bbox()));
    let c = if bb.is_empty() { Point::ZERO } else { bb.center() };
    Ok(Point::new(num_or(p, "cx", c.x)?, num_or(p, "cy", c.y)?))
}

fn move_by(e: &mut Engine, p: &Value) -> CmdResult {
    let (dx, dy) = (num(p, "dx")?, num(p, "dy")?);
    apply(e, p, Affine::translate(dx, dy), false)
}

fn scale(e: &mut Engine, p: &Value) -> CmdResult {
    let sx = num(p, "sx")?;
    let sy = opt_num(p, "sy")?.unwrap_or(sx);
    if sx.abs() < 1e-6 || sy.abs() < 1e-6 || sx.abs() > 1e6 || sy.abs() > 1e6 {
        return Err(EngineError::bad_params("scale factors must be non-zero and reasonable"));
    }
    let c = center(e, p)?;
    let text = opt_bool(p, "scale_text")?.unwrap_or(true);
    apply(e, p, Affine::about(Affine::scale(sx, sy), c), text)
}

fn rotate_by(e: &mut Engine, p: &Value, deg: f64) -> CmdResult {
    let c = center(e, p)?;
    apply(e, p, Affine::about(Affine::rotate(deg.to_radians()), c), false)
}

fn rotate(e: &mut Engine, p: &Value) -> CmdResult {
    let a = num(p, "angle")?;
    rotate_by(e, p, a)
}

fn flip(e: &mut Engine, p: &Value, horizontal: bool) -> CmdResult {
    let c = center(e, p)?;
    let t = if horizontal {
        Affine::scale(-1.0, 1.0)
    } else {
        Affine::scale(1.0, -1.0)
    };
    apply(e, p, Affine::about(t, c), false)
}
