//! Geometric transforms of objects.

use super::{always, num, num_or, opt_bool, opt_num, CmdResult, Command};
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
            params: "{\"ids\"?: [id], \"dx\", \"dy\", \"copy\"?: bool}  — Fig units; with smart links on (indicator linkmode move|slide), the end points of lines attached to the objects follow, and a copy copies the attached lines (xfig's linked move/copy)",
            mutates: true,
            enabled: has_selection_or_ids,
            run: move_by,
        },
        Command {
            id: "transform.nudge",
            label: "Nudge",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id], \"dir\": left|right|up|down, \"steps\"?: number (1), \"distance\"?: Fig units per step (15)}  — move the objects a small step; smart links follow as for transform.move",
            mutates: true,
            enabled: always,
            run: nudge,
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
            params: "{\"ids\"?: [id], \"angle\": degrees counter-clockwise, \"cx\"?, \"cy\"?, \"copy\"?: bool, \"copies\"?: 1..99}  — copy: rotate copies, keep the originals; copies: that many copies, turned by 1x, 2x, ... the angle",
            mutates: true,
            enabled: has_selection_or_ids,
            run: rotate,
        },
        Command {
            id: "transform.rotate_ccw",
            label: "Rotate Counter-clockwise",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id]}  — by the indicator's rotation angle",
            mutates: true,
            enabled: always,
            run: |e, p| {
                let a = e.indicator.rotate_angle;
                rotate_by(e, p, a)
            },
        },
        Command {
            id: "transform.rotate_cw",
            label: "Rotate Clockwise",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: always,
            run: |e, p| {
                let a = -e.indicator.rotate_angle;
                rotate_by(e, p, a)
            },
        },
        Command {
            id: "transform.flip_horizontal",
            label: "Flip Left/Right",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id], \"cx\"?, \"copy\"?: bool}  — copy: flip copies, keep the originals",
            mutates: true,
            enabled: always,
            run: |e, p| flip(e, p, true),
        },
        Command {
            id: "transform.flip_vertical",
            label: "Flip Up/Down",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id], \"cy\"?, \"copy\"?: bool}",
            mutates: true,
            enabled: always,
            run: |e, p| flip(e, p, false),
        },
    ]
}

fn has_selection_or_ids(_: &Engine) -> bool {
    true
}

/// Apply a transform to the target objects. With smart links on, the end
/// points of lines attached to the shapes go through the same transform
/// (`links`); when copying, the attached lines are copied and the originals
/// follow the copies (xfig's copy with links).
pub(crate) fn apply(e: &mut Engine, p: &Value, t: Affine, scale_text: bool) -> CmdResult {
    let ids = e.target_ids(p)?;
    if ids.is_empty() {
        return Err(EngineError::bad_params("no objects selected"));
    }
    let mode = e.indicator.linkmode;
    let links = crate::links::collect(e, &ids);
    let follow = |q: Point| t.apply(q);
    if opt_bool(p, "copy")?.unwrap_or(false) {
        let copies: Vec<_> = ids.iter().filter_map(|i| e.doc().get(*i).cloned()).collect();
        let mut new_ids = Vec::new();
        for mut o in copies {
            o.transform(&t, scale_text);
            new_ids.push(e.add_object(o));
        }
        crate::links::copy_adjust(e, mode, &links, &follow);
        e.set_selection(new_ids.clone());
        return Ok(json!({"count": new_ids.len(), "ids": super::ids_value(&new_ids)}));
    }
    for o in e.doc_mut().objects.iter_mut() {
        if ids.contains(&o.id) {
            Arc::make_mut(o).transform(&t, scale_text);
        }
    }
    crate::links::adjust(e, mode, &links, &follow);
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

/// Nudge by `steps` (default 1) times `distance` (default 15 Fig units, one
/// pixel of xfig's 80 dpi screen) in a direction: what the arrow keys give
/// with the objects in hand. Linked like any move.
fn nudge(e: &mut Engine, p: &Value) -> CmdResult {
    let dir = super::string(p, "dir")?;
    let steps = num_or(p, "steps", 1.0)?;
    let distance = num_or(p, "distance", 15.0)?;
    let d = steps * distance;
    if !d.is_finite() || d.abs() > 1.0e7 {
        return Err(EngineError::bad_params("nudge distance out of range"));
    }
    let (dx, dy) = match dir.as_str() {
        "left" => (-d, 0.0),
        "right" => (d, 0.0),
        "up" => (0.0, -d),
        "down" => (0.0, d),
        other => return Err(EngineError::bad_params(format!("unknown direction {other:?}"))),
    };
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
    let ids = e.target_ids(p)?;
    let r = apply(e, p, Affine::about(Affine::scale(sx, sy), c), text);
    // A scaled dimension line shows its new length (`rescale_dimension_line`).
    super::dimline::recompute_objects(e, &ids);
    r
}

fn rotate_by(e: &mut Engine, p: &Value, deg: f64) -> CmdResult {
    let c = center(e, p)?;
    let copy = opt_bool(p, "copy")?.unwrap_or(false);
    let n = if copy {
        opt_num(p, "copies")?.unwrap_or(1.0).clamp(1.0, 99.0) as u32
    } else {
        1
    };
    if n <= 1 {
        return apply(e, p, Affine::about(Affine::rotate(deg.to_radians()), c), false);
    }
    // xfig's copy & rotate (`init_copynrotate`, `e_rotate.c`): `n` copies of
    // the originals, the k-th turned by k times the angle about one centre.
    let ids = e.target_ids(p)?;
    let ids_v = super::ids_value(&ids);
    let mut all = Vec::new();
    for k in 1..=n {
        let q = json!({"ids": ids_v, "copy": true});
        let r = apply(
            e,
            &q,
            Affine::about(Affine::rotate((deg * f64::from(k)).to_radians()), c),
            false,
        )?;
        if let Some(Value::Array(a)) = r.get("ids") {
            all.extend(a.iter().filter_map(Value::as_u64).map(rustfig_model::ObjectId));
        }
    }
    e.set_selection(all.clone());
    Ok(json!({"count": all.len(), "ids": super::ids_value(&all)}))
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
