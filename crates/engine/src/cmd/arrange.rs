//! Alignment, distribution and stacking order (depth).

use super::{always, num, opt_str, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_geom::Rect;
use rustfig_model::{ObjectId, MAX_DEPTH};
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "arrange.align",
            label: "Align",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id], \"h\"?: none|left|center|right, \"v\"?: none|top|center|bottom}  — defaults from the indicator",
            mutates: true,
            enabled: always,
            run: align,
        },
        Command {
            id: "arrange.align_canvas",
            label: "Align to Canvas",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id] (all objects), \"h\"?: none|left|center|right, \"v\"?: none|top|center|bottom}  — each object is aligned to the page (xfig's Align mode, middle button; init_align_canvas), by the indicator's align modes",
            mutates: true,
            enabled: |_| true,
            run: align_canvas,
        },
        Command {
            id: "arrange.distribute",
            label: "Distribute",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id], \"axis\": horizontal|vertical}  — equal gaps between objects",
            mutates: true,
            enabled: always,
            run: distribute,
        },
        Command {
            id: "arrange.to_front",
            label: "Bring to Front",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: always,
            run: |e, p| restack(e, p, Stack::Front),
        },
        Command {
            id: "arrange.forward",
            label: "Bring Forward",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id]}  — depth - 1",
            mutates: true,
            enabled: always,
            run: |e, p| restack(e, p, Stack::Forward),
        },
        Command {
            id: "arrange.backward",
            label: "Send Backward",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id]}  — depth + 1",
            mutates: true,
            enabled: always,
            run: |e, p| restack(e, p, Stack::Backward),
        },
        Command {
            id: "arrange.to_back",
            label: "Send to Back",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: always,
            run: |e, p| restack(e, p, Stack::Back),
        },
        Command {
            id: "arrange.set_depth",
            label: "Set depth",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id], \"depth\": 0-999}",
            mutates: true,
            enabled: has_selection_or_ids,
            run: set_depth,
        },
    ]
}

fn has_selection_or_ids(_: &Engine) -> bool {
    true
}

fn targets(e: &Engine, p: &Value) -> Result<Vec<(ObjectId, Rect)>, EngineError> {
    let ids = e.target_ids(p)?;
    if ids.is_empty() {
        return Err(EngineError::bad_params("no objects selected"));
    }
    Ok(ids
        .iter()
        .filter_map(|i| e.doc().get(*i).map(|o| (*i, o.geometry_bbox())))
        .filter(|(_, r)| !r.is_empty())
        .collect())
}

fn translate(e: &mut Engine, moves: Vec<(ObjectId, f64, f64)>) {
    // Smart links: the ends attached to each object follow it.
    crate::links::translate_each(e, &moves);
}

fn align(e: &mut Engine, p: &Value) -> CmdResult {
    let h = opt_str(p, "h")?
        .map(str::to_string)
        .unwrap_or_else(|| e.indicator.align_h.clone());
    let v = opt_str(p, "v")?
        .map(str::to_string)
        .unwrap_or_else(|| e.indicator.align_v.clone());
    let items = targets(e, p)?;
    let all = items.iter().fold(Rect::EMPTY, |r, (_, b)| r.union(b));
    let mut moves = Vec::new();
    for (id, b) in &items {
        let dx = match h.as_str() {
            "left" => all.min.x - b.min.x,
            "center" => all.center().x - b.center().x,
            "right" => all.max.x - b.max.x,
            "none" | "" | "distribute_centres" | "distribute_edges" | "abut" => 0.0,
            other => {
                return Err(EngineError::bad_params(format!(
                    "unknown horizontal alignment {other:?}"
                )))
            }
        };
        let dy = match v.as_str() {
            "top" => all.min.y - b.min.y,
            "center" => all.center().y - b.center().y,
            "bottom" => all.max.y - b.max.y,
            "none" | "" | "distribute_centres" | "distribute_edges" | "abut" => 0.0,
            other => return Err(EngineError::bad_params(format!("unknown vertical alignment {other:?}"))),
        };
        moves.push((*id, dx, dy));
    }
    // xfig's distribute_horizontally / _vertically for the last three choices.
    for (horizontal, mode) in [(true, h.as_str()), (false, v.as_str())] {
        let Some(mode) = Spread::from_name(mode) else { continue };
        for (m, d) in moves.iter_mut().zip(spread(&items, horizontal, mode)) {
            if horizontal {
                m.1 = d;
            } else {
                m.2 = d;
            }
        }
    }
    translate(e, moves);
    Ok(json!({"count": items.len()}))
}

/// How objects are spread along one axis (xfig's last three align choices).
#[derive(Clone, Copy)]
enum Spread {
    /// Equal distance between the objects' centres.
    Centres,
    /// Equal gaps between the objects' edges.
    Edges,
    /// Edge to edge, from the first object on.
    Abut,
}

impl Spread {
    fn from_name(s: &str) -> Option<Spread> {
        match s {
            "distribute_centres" => Some(Spread::Centres),
            "distribute_edges" => Some(Spread::Edges),
            "abut" => Some(Spread::Abut),
            _ => None,
        }
    }
}

/// The shift of each of `items` (in their order) along one axis.
fn spread(items: &[(ObjectId, Rect)], horizontal: bool, mode: Spread) -> Vec<f64> {
    let lo = |r: &Rect| if horizontal { r.min.x } else { r.min.y };
    let size = |r: &Rect| if horizontal { r.width() } else { r.height() };
    let mid = |r: &Rect| lo(r) + size(r) / 2.0;
    let key = |i: usize| match (items.get(i), mode) {
        (Some((_, r)), Spread::Centres) => mid(r),
        (Some((_, r)), _) => lo(r),
        _ => 0.0,
    };
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|a, b| key(*a).total_cmp(&key(*b)));
    let mut shifts = vec![0.0; items.len()];
    let (Some(&first), Some(&last)) = (order.first(), order.last()) else {
        return shifts;
    };
    let (Some((_, rf)), Some((_, rl))) = (items.get(first), items.get(last)) else {
        return shifts;
    };
    let n = order.len();
    let total: f64 = items.iter().map(|(_, r)| size(r)).sum();
    let gap = match mode {
        Spread::Edges if n > 1 => (lo(rl) + size(rl) - lo(rf) - total) / (n - 1) as f64,
        _ => 0.0,
    };
    let step = if n > 1 {
        (mid(rl) - mid(rf)) / (n - 1) as f64
    } else {
        0.0
    };
    let mut pos = lo(rf);
    for (k, i) in order.iter().enumerate() {
        let Some((_, r)) = items.get(*i) else { continue };
        let d = match mode {
            Spread::Centres => mid(rf) + step * k as f64 - mid(r),
            _ => pos - lo(r),
        };
        pos += size(r) + gap;
        if let Some(s) = shifts.get_mut(*i) {
            *s = if d.is_finite() { d } else { 0.0 };
        }
    }
    shifts
}

/// Align every object (not each other) to the page: xfig's
/// `init_align_canvas` in `e_align.c`. The page is the paper size, swapped in
/// landscape; distribute and abut have no meaning against the canvas.
fn align_canvas(e: &mut Engine, p: &Value) -> CmdResult {
    let h = opt_str(p, "h")?.map_or_else(|| e.indicator.align_h.clone(), str::to_string);
    let v = opt_str(p, "v")?.map_or_else(|| e.indicator.align_v.clone(), str::to_string);
    if !matches!(
        h.as_str(),
        "none" | "" | "left" | "center" | "right" | "distribute_centres" | "distribute_edges" | "abut"
    ) {
        return Err(EngineError::bad_params(format!("unknown horizontal alignment {h:?}")));
    }
    if !matches!(
        v.as_str(),
        "none" | "" | "top" | "center" | "bottom" | "distribute_centres" | "distribute_edges" | "abut"
    ) {
        return Err(EngineError::bad_params(format!("unknown vertical alignment {v:?}")));
    }
    let ids: Vec<ObjectId> = match p.get("ids") {
        Some(_) => e.target_ids(p)?,
        None => e.doc().objects.iter().map(|o| o.id).collect(),
    };
    let (pw, ph) = e.doc().settings.paper.inches();
    let (pw, ph) = match e.doc().settings.orientation {
        rustfig_model::Orientation::Landscape => (ph, pw),
        rustfig_model::Orientation::Portrait => (pw, ph),
    };
    let page = Rect::from_points([
        rustfig_geom::Point::ZERO,
        rustfig_geom::Point::new(pw * 1200.0, ph * 1200.0),
    ]);
    let mut moves = Vec::new();
    for id in ids {
        let Some(o) = e.doc().get(id) else { continue };
        let b = o.geometry_bbox();
        if b.is_empty() {
            continue;
        }
        let dx = match h.as_str() {
            "left" => page.min.x - b.min.x,
            "center" => page.center().x - b.center().x,
            "right" => page.max.x - b.max.x,
            _ => 0.0,
        };
        let dy = match v.as_str() {
            "top" => page.min.y - b.min.y,
            "center" => page.center().y - b.center().y,
            "bottom" => page.max.y - b.max.y,
            _ => 0.0,
        };
        if dx.is_finite() && dy.is_finite() && (dx != 0.0 || dy != 0.0) {
            moves.push((id, dx, dy));
        }
    }
    let n = moves.len();
    translate(e, moves);
    Ok(json!({"count": n}))
}

fn distribute(e: &mut Engine, p: &Value) -> CmdResult {
    let horizontal = match opt_str(p, "axis")?.unwrap_or("horizontal") {
        "horizontal" | "h" | "x" => true,
        "vertical" | "v" | "y" => false,
        other => return Err(EngineError::bad_params(format!("unknown axis {other:?}"))),
    };
    let mut items = targets(e, p)?;
    if items.len() < 3 {
        return Ok(json!({"count": items.len()}));
    }
    let lo = |r: &Rect| if horizontal { r.min.x } else { r.min.y };
    let size = |r: &Rect| if horizontal { r.width() } else { r.height() };
    items.sort_by(|a, b| lo(&a.1).total_cmp(&lo(&b.1)));
    let (Some(first), Some(last)) = (items.first(), items.last()) else {
        return Ok(json!({"count": 0}));
    };
    let start = lo(&first.1);
    let end = lo(&last.1) + size(&last.1);
    let total: f64 = items.iter().map(|(_, r)| size(r)).sum();
    let gap = (end - start - total) / (items.len() - 1) as f64;
    let mut pos = start;
    let mut moves = Vec::new();
    for (id, r) in &items {
        let d = pos - lo(r);
        moves.push(if horizontal { (*id, d, 0.0) } else { (*id, 0.0, d) });
        pos += size(r) + gap;
    }
    let n = moves.len();
    translate(e, moves);
    Ok(json!({"count": n}))
}

enum Stack {
    Front,
    Forward,
    Backward,
    Back,
}

fn restack(e: &mut Engine, p: &Value, how: Stack) -> CmdResult {
    let ids = e.target_ids(p)?;
    let others: Vec<i32> = e
        .doc()
        .objects
        .iter()
        .filter(|o| !ids.contains(&o.id))
        .map(|o| o.depth())
        .collect();
    let min = others.iter().copied().min().unwrap_or(50);
    let max = others.iter().copied().max().unwrap_or(50);
    for o in e.doc_mut().objects.iter_mut() {
        if !ids.contains(&o.id) {
            continue;
        }
        let d = o.depth();
        let nd = match how {
            Stack::Front => (min - 1).min(d),
            Stack::Forward => d - 1,
            Stack::Backward => d + 1,
            Stack::Back => (max + 1).max(d),
        };
        Arc::make_mut(o).set_depth(nd.clamp(0, MAX_DEPTH));
    }
    Ok(json!({"count": ids.len()}))
}

fn set_depth(e: &mut Engine, p: &Value) -> CmdResult {
    let ids = e.target_ids(p)?;
    let d = (num(p, "depth")?.round() as i32).clamp(0, MAX_DEPTH);
    for o in e.doc_mut().objects.iter_mut() {
        if ids.contains(&o.id) {
            Arc::make_mut(o).set_depth(d);
        }
    }
    Ok(json!({"count": ids.len()}))
}
