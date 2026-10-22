//! Dimension lines (plan D10): create one, recompute after edits.
//
// Ported from xfig src/d_line.c (`create_lineobject` with Shift) and
// src/e_scale.c (`rescale_dimension_line`) (xfig licence, see LICENSE-xfig).

use super::{ids_value, num, opt_str, CmdResult, Command};
use crate::dimline::{self, DimlineDefaults};
use crate::{Engine, EngineError};
use rustfig_geom::Point;
use rustfig_model::{Object, ObjectKind};
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "shape.dimension_line",
            label: "Dimension line",
            menu: "",
            shortcut: "",
            params: "{\"x1\", \"y1\", \"x2\", \"y2\", \"text\"?: string (only with the indicator's dimline.fixed)}  — a compound of the measured line, a box with its length in the figure units, and ticks at the ends (xfig: Shift+left button, then the final point, in polyline mode); settings in the indicator's dimline. Returns {\"id\"}",
            mutates: true,
            enabled: |_| true,
            run: create,
        },
        Command {
            id: "object.recompute_dimension",
            label: "Recompute dimension line",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id]}  — recompute the length text, box and ticks of dimension lines from their main line (done by itself when an open compound closes and after scaling). Returns {\"count\"}",
            mutates: true,
            enabled: |_| true,
            run: recompute,
        },
    ]
}

fn create(e: &mut Engine, p: &Value) -> CmdResult {
    let a = Point::new(num(p, "x1")?, num(p, "y1")?);
    let b = Point::new(num(p, "x2")?, num(p, "y2")?);
    let ok = |q: Point| q.x.is_finite() && q.y.is_finite() && q.x.abs() <= 1e8 && q.y.abs() <= 1e8;
    if !ok(a) || !ok(b) {
        return Err(EngineError::bad_params("coordinates out of range"));
    }
    // A dimension line must have two different points.
    if a == b {
        return Err(EngineError::bad_params("a dimension line needs two different points"));
    }
    let text = opt_str(p, "text")?;
    let ind = &e.indicator;
    let defaults = DimlineDefaults {
        pen_color: ind.pen_color,
        depth: ind.depth,
        join: ind.join,
        style_val: ind.style_val,
        dot_gap: ind.dot_gap,
    };
    let settings = ind.dimline.clone();
    let fmt = formatter(e, settings.precision);
    let obj = dimline::create(&settings, &defaults, a, b, text, &fmt);
    let id = e.add_object(obj);
    e.set_selection(vec![id]);
    Ok(json!({"id": id.0}))
}

/// The length formatter of the current units and unit scale.
pub(crate) fn formatter(e: &Engine, precision: u32) -> impl Fn(f64) -> String {
    let units = e.doc().settings.units;
    let scale = e.unit_scale.clone();
    let ruler = e.doc().unit_name();
    move |l| dimline::length_text(l, units, &scale, ruler, precision)
}

/// Recompute every dimension line inside `o` (also nested ones).
fn recompute_in(o: &mut Object, e: &Engine) -> usize {
    let settings = e.indicator.dimline.clone();
    let fmt = formatter(e, settings.precision);
    let mut n = 0;
    if dimline::is_dimline(o) && dimline::recompute(o, &settings, &fmt) {
        n += 1;
    }
    if let ObjectKind::Compound(c) = &mut o.kind {
        for m in &mut c.objects {
            n += recompute_in(m, e);
        }
    }
    n
}

/// Recompute the dimension lines among (and inside) the given top-level
/// objects; the number recomputed.
pub(crate) fn recompute_objects(e: &mut Engine, ids: &[rustfig_model::ObjectId]) -> usize {
    let mut n = 0;
    for id in ids {
        let Some(mut o) = e.doc().get(*id).cloned() else {
            continue;
        };
        let k = recompute_in(&mut o, e);
        if k > 0 {
            n += k;
            for slot in e.doc_mut().objects.iter_mut() {
                if slot.id == *id {
                    *slot = Arc::new(o);
                    break;
                }
            }
        }
    }
    n
}

fn recompute(e: &mut Engine, p: &Value) -> CmdResult {
    let ids = match p.get("ids") {
        Some(_) => e.target_ids(p)?,
        None => e.doc().objects.iter().map(|o| o.id).collect(),
    };
    let n = recompute_objects(e, &ids);
    Ok(json!({"count": n, "ids": ids_value(&ids)}))
}
