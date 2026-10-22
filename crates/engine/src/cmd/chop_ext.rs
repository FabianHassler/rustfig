//! Chop's extensions (plan D15, a RustFig addition; xfig has none of them):
//! Trim, Split at point, Cut all and Extend. Each is one command and one
//! undo step; Chop mode reaches them with the Shift key
//! (`tools/chop.rs`).

use super::chop::{axe_leaves, bad, id_param, leaf_objects, object};
use super::outline::{self, Cut, Mode};
use super::{num, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_geom::{line_segment_intersection, Point};
use rustfig_model::{Object, ObjectId, ObjectKind, PolyKind};
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "object.trim",
            label: "Trim object",
            menu: "",
            shortcut: "",
            params: "{\"id\", \"x\", \"y\", \"axes\": [object ids, optional]}  — remove the piece of the object (line, polygon, box, arc, circle, ellipse or spline) at the point (x, y) between its intersections with the cutting objects (default: every other object); the rest stays as separate pieces. Closed figures need two intersections. Returns {\"ids\": remaining pieces}",
            mutates: true,
            enabled: super::always,
            run: trim,
        },
        Command {
            id: "object.split_at",
            label: "Split at point",
            menu: "",
            shortcut: "",
            params: "{\"id\", \"x\", \"y\"}  — cut a line, polygon, box, arc or spline in two where the outline is nearest to (x, y), no cutting object needed; a closed figure is opened there. Returns {\"ids\": the pieces}",
            mutates: true,
            enabled: super::always,
            run: split_at,
        },
        Command {
            id: "object.cut_all",
            label: "Cut all against each other",
            menu: "",
            shortcut: "",
            params: "{\"ids\": [object ids, default the selection]}  — cut every selected line, polygon, box, arc, circle, ellipse, spline and compound at its intersections with the others in one step (open pieces; a closed figure needs two intersections). Returns {\"ids\": new pieces, \"cut\": number of objects cut}",
            mutates: true,
            enabled: super::always,
            run: cut_all,
        },
        Command {
            id: "object.extend",
            label: "Extend line to boundary",
            menu: "",
            shortcut: "",
            params: "{\"id\", \"x\", \"y\", \"boundaries\": [object ids, optional]}  — lengthen the end of an open polyline nearest to (x, y), along its last segment, to the nearest boundary object (default: every other object). Fails when the line hits none",
            mutates: true,
            enabled: super::always,
            run: extend,
        },
    ]
}

/// Put the result of a cut into the drawing: the pieces take the place of
/// the log object (so the depth order within a depth is kept), a compound
/// keeps its place and id. Returns the ids of what now stands there.
pub(super) fn replace_with(e: &mut Engine, log: &Object, cut: Cut) -> Vec<ObjectId> {
    let doc = e.doc_mut();
    match cut {
        Cut::Unchanged => Vec::new(),
        Cut::Compound(c, _) => match doc.get_mut(log.id) {
            Some(o) => {
                o.kind = ObjectKind::Compound(c);
                vec![log.id]
            }
            None => Vec::new(),
        },
        Cut::Pieces(kinds) => {
            // A piece that shrank to one point (a cut exactly at a vertex)
            // is dropped: Fig readers refuse a spline whose points all
            // coincide, as xfig's `read_splineobject` does.
            let kinds: Vec<ObjectKind> = kinds.into_iter().filter(|k| !collapsed(k)).collect();
            if kinds.is_empty() {
                return Vec::new();
            }
            let Some(at) = doc.objects.iter().position(|o| o.id == log.id) else {
                return Vec::new();
            };
            doc.objects.remove(at);
            let mut ids = Vec::with_capacity(kinds.len());
            let mut objs = Vec::with_capacity(kinds.len());
            for kind in kinds {
                let mut o = Object::new(kind);
                o.comments = log.comments.clone();
                o.id = doc.alloc_id();
                ids.push(o.id);
                objs.push(std::sync::Arc::new(o));
            }
            let tail = doc.objects.split_off(at);
            doc.objects.extend(objs);
            doc.objects.extend(tail);
            ids
        }
    }
}

/// Whether a line or spline's points all round to the same Fig coordinate.
fn collapsed(k: &ObjectKind) -> bool {
    let pts = match k {
        ObjectKind::Polyline(l) if l.kind != PolyKind::Picture => &l.points,
        ObjectKind::Spline(s) => &s.points,
        _ => return false,
    };
    let round = |p: &Point| (p.x.round(), p.y.round());
    match pts.first() {
        Some(first) => pts.iter().all(|p| round(p) == round(first)),
        None => true,
    }
}

fn point(p: &Value) -> Result<Point, EngineError> {
    Ok(Point::new(num(p, "x")?, num(p, "y")?))
}

/// The leaves of every top-level object but `except` (the default cutting
/// objects of Trim and Extend).
fn other_leaves(e: &Engine, except: ObjectId) -> Vec<Object> {
    let mut out = Vec::new();
    for o in &e.doc().objects {
        if o.id != except {
            leaf_objects(o, &mut out, 0);
        }
    }
    out
}

fn cutting_objects(e: &Engine, p: &Value, key: &str, log: ObjectId) -> Result<Vec<Object>, EngineError> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(other_leaves(e, log)),
        Some(Value::Array(a)) => axe_leaves(e, a, log, true),
        Some(_) => Err(bad(format!("{key:?} must be an array of object ids"))),
    }
}

fn trim(e: &mut Engine, p: &Value) -> CmdResult {
    let id = id_param(p, "id")?;
    let click = point(p)?;
    let log = object(e, id)?;
    if matches!(log.kind, ObjectKind::Compound(_) | ObjectKind::Text(_)) {
        return Err(bad(
            "trim works on a line, polygon, box, arc, circle, ellipse or spline (open a compound first)",
        ));
    }
    let axes = cutting_objects(e, p, "axes", id)?;
    let pie = e.indicator.pie_arc;
    let pieces = outline::cut_kind(&log.kind, &axes, Mode::Open, pie)?;
    if pieces.len() < 2 {
        return Ok(json!({"ids": [], "removed": false}));
    }
    let victim = pieces
        .iter()
        .enumerate()
        .map(|(i, k)| (i, outline::dist_to_kind(k, click)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let kept: Vec<ObjectKind> = pieces
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i != victim)
        .map(|(_, k)| k)
        .collect();
    let ids = replace_with(e, &log, Cut::Pieces(kept));
    e.set_selection(ids.clone());
    Ok(json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>(), "removed": true}))
}

fn split_at(e: &mut Engine, p: &Value) -> CmdResult {
    let id = id_param(p, "id")?;
    let click = point(p)?;
    let log = object(e, id)?;
    let pieces = outline::split_kind(&log.kind, click)?;
    let ids = replace_with(e, &log, Cut::Pieces(pieces));
    e.set_selection(ids.clone());
    Ok(json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>()}))
}

fn cut_all(e: &mut Engine, p: &Value) -> CmdResult {
    let ids = e.target_ids(p)?;
    let mut set: Vec<ObjectId> = Vec::new();
    for id in ids {
        if !set.contains(&id) {
            set.push(id);
        }
    }
    if set.len() < 2 {
        return Err(bad("select at least two objects to cut against each other"));
    }
    // Everyone is cut against the original drawing, so the order of the
    // objects does not matter.
    let objs: Vec<Object> = set.iter().filter_map(|id| e.doc().get(*id).cloned()).collect();
    let pie = e.indicator.pie_arc;
    let (mut new_ids, mut cut_count) = (Vec::new(), 0usize);
    let mut selection = Vec::new();
    for log in &objs {
        let mut axes = Vec::new();
        for o in objs.iter().filter(|o| o.id != log.id) {
            leaf_objects(o, &mut axes, 0);
        }
        let cut = match &log.kind {
            ObjectKind::Text(_) => Cut::Unchanged,
            ObjectKind::Polyline(l) if l.kind == PolyKind::Picture => Cut::Unchanged,
            _ => outline::cut_object(e.doc_mut(), log, &axes, Mode::Open, pie)?,
        };
        let made = replace_with(e, log, cut);
        if made.is_empty() {
            selection.push(log.id);
        } else {
            cut_count += 1;
            new_ids.extend(made.iter().copied());
            selection.extend(made);
        }
    }
    e.set_selection(selection);
    Ok(json!({"ids": new_ids.iter().map(|i| i.0).collect::<Vec<_>>(), "cut": cut_count}))
}

/// Farthest reach of Extend, in Fig units.
const EXTEND_REACH: f64 = 1.0e8;

fn extend(e: &mut Engine, p: &Value) -> CmdResult {
    let id = id_param(p, "id")?;
    let click = point(p)?;
    let log = object(e, id)?;
    let ObjectKind::Polyline(l) = &log.kind else {
        return Err(bad("only an open polyline can be extended"));
    };
    if l.kind != PolyKind::Polyline || l.points.len() < 2 {
        return Err(bad("only an open polyline can be extended"));
    }
    let (Some(first), Some(last)) = (l.points.first().copied(), l.points.last().copied()) else {
        return Err(bad("the polyline has no points"));
    };
    let at_start = click.dist(first) < click.dist(last);
    let end = if at_start { first } else { last };
    // Direction of the last segment that has a length.
    let ordered: Vec<Point> = if at_start {
        l.points.clone()
    } else {
        l.points.iter().rev().copied().collect()
    };
    let Some(dir) = ordered
        .iter()
        .skip(1)
        .map(|q| end - *q)
        .find(|v| v.len() > 1e-9)
        .map(|v| v.normalized())
    else {
        return Err(bad("the polyline has no direction"));
    };
    let bounds = cutting_objects(e, p, "boundaries", id)?;
    let far = end + dir * EXTEND_REACH;
    let mut best: Option<(f64, Point)> = None;
    for (a, b) in outline::boundary_segments(&bounds) {
        if let Some(h) = line_segment_intersection(end, far, a, b) {
            let h = Point::new(h.x.round(), h.y.round());
            let t = (h - end).dot(dir);
            if t > 0.5 && best.is_none_or(|(bt, _)| t < bt) {
                best = Some((t, h));
            }
        }
    }
    let Some((_, hit)) = best else {
        return Err(bad("the line reaches no boundary object in that direction"));
    };
    let doc = e.doc_mut();
    let Some(o) = doc.get_mut(id) else {
        return Err(EngineError::not_found(format!("no top-level object with id {}", id.0)));
    };
    if let ObjectKind::Polyline(pl) = &mut o.kind {
        let slot = if at_start {
            pl.points.first_mut()
        } else {
            pl.points.last_mut()
        };
        if let Some(s) = slot {
            *s = hit;
        }
    }
    e.set_selection(vec![id]);
    Ok(json!({"ids": [id.0], "x": hit.x, "y": hit.y}))
}
