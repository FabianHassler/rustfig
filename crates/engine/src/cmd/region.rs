//! Region and end-point commands behind the functions on xfig's free mouse
//! buttons (plan W13, W14): update, move and point deletion by region,
//! points at the ends of a line, reversing arrows, breaking all levels of a
//! compound, aligning to a reference, circles through three points and
//! pasting the clipboard as text.
//!
//! A region is the rectangle between two corners `(x1, y1)` and `(x2, y2)`
//! in Fig units, or a lasso `polygon` (W15). It takes the objects completely inside it, on visible
//! depths, that the mode's object mask lets through (`tools::mode_mask`).

use super::object::{insert_point, remove_point};
use super::shape::{added, make_ellipse, make_text, style_from};
use super::{has_objects, ids_value, num, opt_str, point_value, CmdResult, Command};
use crate::tools::{mode_mask, ToolKind};
use crate::{Engine, EngineError, Region};
use rustfig_geom::{Affine, Point, Rect};
use rustfig_model::{EllipseKind, Object, ObjectId, ObjectKind, PolyKind};
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "object.update_region",
            label: "Update Region",
            menu: "",
            shortcut: "",
            params: "{\"x1\", \"y1\", \"x2\", \"y2\" | \"polygon\": [[x,y]…] (lasso)}  — every object completely inside the rectangle (visible depths) takes the indicator's attributes, as object.update does; Update mode, right button",
            mutates: true,
            enabled: has_objects,
            run: update_region,
        },
        Command {
            id: "transform.move_region",
            label: "Move Region",
            menu: "",
            shortcut: "",
            params: "{\"x1\", \"y1\", \"x2\", \"y2\" | \"polygon\", \"dx\", \"dy\"}  — move every object completely inside the rectangle (visible depths); Move mode, right button",
            mutates: true,
            enabled: has_objects,
            run: move_region,
        },
        Command {
            id: "point.delete_region",
            label: "Delete Points in Region",
            menu: "",
            shortcut: "",
            params: "{\"x1\", \"y1\", \"x2\", \"y2\" | \"polygon\", \"id\"?}  — delete the points inside the rectangle from one line, polygon or spline: the given one, else the front-most with a point inside; Delete point mode, middle button",
            mutates: true,
            enabled: has_objects,
            run: delete_points_region,
        },
        Command {
            id: "point.add_end",
            label: "Add Point at End",
            menu: "",
            shortcut: "",
            params: "{\"id\", \"x\", \"y\", \"end\"?: nearest|start|end}  — add a point at (x, y) beyond the first or last point of an open line or spline (default: the end nearer to the point); Add point mode, middle button",
            mutates: true,
            enabled: has_objects,
            run: add_end,
        },
        Command {
            id: "object.reverse_arrows",
            label: "Reverse Arrows",
            menu: "Object/Arrows",
            shortcut: "",
            params: "{\"ids\"?: [id]}  — swap the forward and backward arrow of lines, open splines and arcs; Arrow mode, right button",
            mutates: true,
            enabled: |e| !e.selection().is_empty(),
            run: reverse_arrows,
        },
        Command {
            id: "object.break_all",
            label: "Break All Levels",
            menu: "Object",
            shortcut: "",
            params: "{\"ids\"?: [id]}  — break compounds and the compounds inside them down to single objects; Break mode, right button",
            mutates: true,
            enabled: |e| !e.selection().is_empty(),
            run: break_all,
        },
        Command {
            id: "arrange.align_to",
            label: "Align to Object",
            menu: "",
            shortcut: "",
            params: "{\"reference\": id, \"ids\"?: [id], \"h\"?: none|left|center|right, \"v\"?: none|top|center|bottom}  — move the objects so that their boxes line up with the reference object's box (defaults from the indicator); Align mode, right button",
            mutates: true,
            enabled: has_objects,
            run: align_to,
        },
        Command {
            id: "shape.circle3",
            label: "Circle through Three Points",
            menu: "",
            shortcut: "",
            params: "{\"points\": [[x,y] x3], \"kind\"?: radius|diameter} + style  — Circle modes, right button",
            mutates: true,
            enabled: |_| true,
            run: circle3,
        },
        Command {
            id: "edit.paste_text",
            label: "Paste as Text",
            menu: "",
            shortcut: "",
            params: "{\"x\", \"y\"}  — new text at (x, y) from the text objects in the clipboard, one line each; Text mode, middle button",
            mutates: true,
            enabled: |e| e.clipboard_text().is_some(),
            run: paste_text,
        },
    ]
}

/// The region of the parameters: `"polygon": [[x, y], …]` (a lasso, at
/// least three points), else the rectangle `x1, y1, x2, y2`.
fn region(p: &Value) -> Result<Region, EngineError> {
    if p.get("polygon").is_some_and(|v| !v.is_null()) {
        let pts = super::points(p, "polygon")?;
        if pts.len() < 3 {
            return Err(EngineError::bad_params("a lasso needs at least three points"));
        }
        return Ok(Region::Lasso(pts));
    }
    let a = Point::new(num(p, "x1")?, num(p, "y1")?);
    let b = Point::new(num(p, "x2")?, num(p, "y2")?);
    Ok(Region::Rect(Rect::new(a, b)))
}

fn update_region(e: &mut Engine, p: &Value) -> CmdResult {
    let r = region(p)?;
    let ids = e.objects_in_region(&r, mode_mask(ToolKind::Update, 90.0));
    if ids.is_empty() {
        return Err(EngineError::bad_params("no objects completely inside the region"));
    }
    e.set_selection(ids.clone());
    super::object::update(e, &json!({ "ids": ids_value(&ids) }))
}

fn move_region(e: &mut Engine, p: &Value) -> CmdResult {
    let r = region(p)?;
    let (dx, dy) = (num(p, "dx")?, num(p, "dy")?);
    let ids = e.objects_in_region(&r, mode_mask(ToolKind::Move, 90.0));
    if ids.is_empty() {
        return Err(EngineError::bad_params("no objects completely inside the region"));
    }
    e.set_selection(ids.clone());
    super::transform::apply(e, &json!({ "ids": ids_value(&ids) }), Affine::translate(dx, dy), false)
}

fn delete_points_region(e: &mut Engine, p: &Value) -> CmdResult {
    let r = region(p)?;
    let mask = mode_mask(ToolKind::DeletePoint, 90.0);
    let inside = |o: &Object| -> Vec<usize> {
        match o.points() {
            Some(pts) if mask.admits(o) && !matches!(o.kind, ObjectKind::Arc(_)) => pts
                .iter()
                .enumerate()
                .filter(|(_, q)| r.contains(**q))
                .map(|(i, _)| i)
                .collect(),
            _ => Vec::new(),
        }
    };
    let id = match p.get("id") {
        None | Some(Value::Null) => {
            // The front-most (lowest depth, last drawn) object with a point inside.
            let mut best: Option<(i32, usize, ObjectId)> = None;
            for (i, o) in e.doc().objects.iter().enumerate() {
                if !e.is_visible(o) || inside(o).is_empty() {
                    continue;
                }
                let better = match best {
                    None => true,
                    Some((d, bi, _)) => o.depth() < d || (o.depth() == d && i > bi),
                };
                if better {
                    best = Some((o.depth(), i, o.id));
                }
            }
            best.map(|(_, _, id)| id)
                .ok_or_else(|| EngineError::bad_params("no line or spline has a point inside the region"))?
        }
        Some(v) => {
            let id = ObjectId(
                v.as_u64()
                    .ok_or_else(|| EngineError::bad_params("id must be an object id"))?,
            );
            if e.doc().get(id).is_none() {
                return Err(EngineError::not_found(format!("no top-level object with id {}", id.0)));
            }
            id
        }
    };
    let idx = match e.doc().get(id) {
        Some(o) => inside(o),
        None => Vec::new(),
    };
    if idx.is_empty() {
        return Err(EngineError::bad_params("the object has no point inside the region"));
    }
    let o = e
        .doc_mut()
        .get_mut(id)
        .ok_or_else(|| EngineError::not_found("object vanished"))?;
    // From the last point to the first, so the indices stay valid.
    for i in idx.iter().rev() {
        remove_point(o, *i)?;
    }
    e.set_selection(vec![id]);
    Ok(json!({"id": id.0, "count": idx.len()}))
}

fn add_end(e: &mut Engine, p: &Value) -> CmdResult {
    let id = ObjectId(num(p, "id")?.round().max(0.0) as u64);
    let pt = point_value(&json!([num(p, "x")?, num(p, "y")?]))?;
    let which = opt_str(p, "end")?.unwrap_or("nearest");
    let o = e
        .doc_mut()
        .get_mut(id)
        .ok_or_else(|| EngineError::not_found(format!("no top-level object with id {}", id.0)))?;
    let (first, last, len) = match &o.kind {
        ObjectKind::Polyline(pl) if pl.kind == PolyKind::Polyline => {
            (pl.points.first(), pl.points.last(), pl.points.len())
        }
        ObjectKind::Spline(s) if !s.closed => (s.points.first(), s.points.last(), s.points.len()),
        _ => {
            return Err(EngineError::bad_params(
                "points can only be added at the ends of an open line or spline",
            ))
        }
    };
    let (Some(first), Some(last)) = (first.copied(), last.copied()) else {
        return Err(EngineError::bad_params("the object has no points"));
    };
    let at_start = match which {
        "start" => true,
        "end" => false,
        "nearest" => pt.dist(first) < pt.dist(last),
        other => return Err(EngineError::bad_params(format!("unknown end {other:?}"))),
    };
    let o = Arc::make_mut(
        e.doc_mut()
            .objects
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or_else(|| EngineError::not_found("object vanished"))?,
    );
    insert_point(o, if at_start { 0 } else { len }, pt)?;
    e.set_selection(vec![id]);
    Ok(json!({"id": id.0, "index": if at_start { 0 } else { len }}))
}

fn reverse_arrows(e: &mut Engine, p: &Value) -> CmdResult {
    let ids = e.target_ids(p)?;
    if ids.is_empty() {
        return Err(EngineError::bad_params("no objects selected"));
    }
    let mut n = 0;
    for o in e.doc_mut().objects.iter_mut() {
        if !ids.contains(&o.id) {
            continue;
        }
        let has = matches!(o.arrows(), Some((f, b)) if f.is_some() || b.is_some());
        if !has {
            continue;
        }
        if let Some((f, b)) = Arc::make_mut(o).arrows_mut() {
            std::mem::swap(f, b);
            n += 1;
        }
    }
    if n == 0 {
        return Err(EngineError::bad_params("none of the objects has an arrow"));
    }
    Ok(json!({"count": n}))
}

fn break_all(e: &mut Engine, p: &Value) -> CmdResult {
    fn flatten(o: Object, out: &mut Vec<Object>) {
        match o.kind {
            ObjectKind::Compound(c) => {
                for child in c.objects {
                    flatten(child, out);
                }
            }
            _ => out.push(o),
        }
    }
    let ids = e.target_ids(p)?;
    let doc = e.doc_mut();
    let mut out = Vec::new();
    let mut freed = Vec::new();
    let mut broke = 0;
    for o in std::mem::take(&mut doc.objects) {
        if matches!(o.kind, ObjectKind::Compound(_)) && ids.contains(&o.id) {
            broke += 1;
            let mut parts = Vec::new();
            flatten(Arc::unwrap_or_clone(o), &mut parts);
            for part in parts {
                freed.push(part.id);
                out.push(Arc::new(part));
            }
        } else {
            out.push(o);
        }
    }
    doc.objects = out;
    if broke == 0 {
        return Err(EngineError::bad_params("no compound selected"));
    }
    e.set_selection(freed.clone());
    Ok(json!({"ids": ids_value(&freed)}))
}

fn align_to(e: &mut Engine, p: &Value) -> CmdResult {
    let rid = ObjectId(num(p, "reference")?.round().max(0.0) as u64);
    let reference = e
        .doc()
        .get(rid)
        .ok_or_else(|| EngineError::bad_params(format!("no top-level object with id {}", rid.0)))?
        .geometry_bbox();
    let h = opt_str(p, "h")?.map_or_else(|| e.indicator.align_h.clone(), str::to_string);
    let v = opt_str(p, "v")?.map_or_else(|| e.indicator.align_v.clone(), str::to_string);
    if !matches!(h.as_str(), "none" | "" | "left" | "center" | "right") {
        return Err(EngineError::bad_params(format!("unknown horizontal alignment {h:?}")));
    }
    if !matches!(v.as_str(), "none" | "" | "top" | "center" | "bottom") {
        return Err(EngineError::bad_params(format!("unknown vertical alignment {v:?}")));
    }
    if reference.is_empty() {
        return Err(EngineError::bad_params("the reference object has no extent"));
    }
    let ids: Vec<ObjectId> = e.target_ids(p)?.into_iter().filter(|i| *i != rid).collect();
    if ids.is_empty() {
        return Err(EngineError::bad_params("no objects to align"));
    }
    let mut moves = Vec::new();
    for id in ids {
        let Some(o) = e.doc().get(id) else { continue };
        let b = o.geometry_bbox();
        if b.is_empty() {
            continue;
        }
        let dx = match h.as_str() {
            "left" => reference.min.x - b.min.x,
            "center" => reference.center().x - b.center().x,
            "right" => reference.max.x - b.max.x,
            _ => 0.0,
        };
        let dy = match v.as_str() {
            "top" => reference.min.y - b.min.y,
            "center" => reference.center().y - b.center().y,
            "bottom" => reference.max.y - b.max.y,
            _ => 0.0,
        };
        if dx != 0.0 || dy != 0.0 {
            moves.push((id, dx, dy));
        }
    }
    let moved = moves.len();
    // Smart links: the ends attached to each object follow it.
    crate::links::translate_each(e, &moves);
    Ok(json!({"count": moved}))
}

/// The circle through three points: centre and radius.
pub(crate) fn circumcircle(a: Point, b: Point, c: Point) -> Option<(Point, f64)> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if !d.is_finite() || d.abs() < 1e-9 {
        return None;
    }
    let (a2, b2, c2) = (a.x * a.x + a.y * a.y, b.x * b.x + b.y * b.y, c.x * c.x + c.y * c.y);
    let ux = (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d;
    let uy = (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d;
    let center = Point::new(ux, uy);
    let r = center.dist(a);
    (center.is_finite() && r.is_finite() && (1.0..1.0e9).contains(&r)).then_some((center, r))
}

fn circle3(e: &mut Engine, p: &Value) -> CmdResult {
    let pts = super::points(p, "points")?;
    let [a, b, c] = pts.as_slice() else {
        return Err(EngineError::bad_params(
            "a circle through points needs exactly three points",
        ));
    };
    let (center, r) = circumcircle(*a, *b, *c).ok_or_else(|| {
        EngineError::bad_params("the three points are on a line, or the circle is too small or too large")
    })?;
    let kind = match opt_str(p, "kind")?.unwrap_or("radius") {
        "radius" => EllipseKind::CircleRadius,
        "diameter" => EllipseKind::CircleDiameter,
        other => return Err(EngineError::bad_params(format!("unknown circle kind {other:?}"))),
    };
    let style = style_from(e, p)?;
    added(e, make_ellipse(style, center, r, r, 0.0, kind))
}

fn paste_text(e: &mut Engine, p: &Value) -> CmdResult {
    let origin = Point::new(num(p, "x")?, num(p, "y")?);
    let lines = e
        .clipboard_text()
        .ok_or_else(|| EngineError::bad_params("the clipboard holds no text"))?;
    let step = if e.indicator.text_step.is_finite() {
        e.indicator.text_step.clamp(0.0, 100.0)
    } else {
        1.0
    };
    let size = e.indicator.font_size * rustfig_geom::FIG_PER_POINT * rustfig_text::LINE_SPACING * step;
    let down = Affine::rotate(e.indicator.text_angle.to_radians()).apply_vec(Point::new(0.0, size));
    let mut ids = Vec::new();
    for (i, line) in lines.into_iter().enumerate() {
        let t = make_text(e, origin + down * i as f64, line);
        ids.push(e.add_object(Object::new(ObjectKind::Text(t))));
    }
    e.set_selection(ids.clone());
    Ok(json!({"ids": ids_value(&ids)}))
}
