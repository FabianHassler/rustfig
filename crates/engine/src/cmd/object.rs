//! Object commands: grouping, attributes, points, conversion, inspection.

use super::{has_selection, ids_value, num, opt_bool, opt_num, opt_str, point_value, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_geom::{polygon_area, polyline_length, Point, Rect};
use rustfig_model::{
    flatten_object, Arrow, CapStyle, ColorRef, Compound, JoinStyle, LineKind, Object, ObjectId, ObjectKind, PolyKind,
    Polyline, Spline, SplineKind, MAX_DEPTH,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "object.group",
            label: "Group (Compound)",
            menu: "Object",
            shortcut: "Ctrl+G",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: |e| e.selection().len() >= 2,
            run: group,
        },
        Command {
            id: "object.ungroup",
            label: "Break Compound",
            menu: "Object",
            shortcut: "Ctrl+Shift+G",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: has_selection,
            run: ungroup,
        },
        Command {
            id: "object.update",
            label: "Update from Current Attributes",
            menu: "Object",
            shortcut: "Ctrl+U",
            params: "{\"ids\"?: [id]}  — apply the indicator's attributes",
            mutates: true,
            enabled: has_selection,
            run: update,
        },
        Command {
            id: "object.set_attributes",
            label: "Set attributes",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id], \"line\"?, \"thickness\"?, \"pen_color\"?, \"fill_color\"?, \"area_fill\"?, \"depth\"?, \"style_val\"?, \"cap\"?: butt|round|projecting, \"join\"?: miter|round|bevel, \"forward_arrow\"?: bool, \"backward_arrow\"?: bool, \"arrow_kind\"?, \"arrow_style\"?, \"arrow_width\"?, \"arrow_height\"?, \"radius\"?, \"font\"?, \"font_size\"?, \"justify\"?, \"angle\"?: degrees, \"text\"?, \"postscript\"?: bool, \"hidden\"?: bool, \"comment\"?: string}",
            mutates: true,
            enabled: |_| true,
            run: set_attributes,
        },
        Command {
            id: "object.get",
            label: "Get objects",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id]}  — full object JSON",
            mutates: false,
            enabled: |_| true,
            run: get,
        },
        Command {
            id: "object.list",
            label: "List objects",
            menu: "",
            shortcut: "",
            params: "{}  — id, type, depth and bounds of every top-level object",
            mutates: false,
            enabled: |_| true,
            run: list,
        },
        Command {
            id: "object.measure",
            label: "Measure",
            menu: "Object",
            shortcut: "Ctrl+M",
            params: "{\"ids\"?: [id]}  — length, area and size",
            mutates: false,
            enabled: has_selection,
            run: measure,
        },
        Command {
            id: "object.convert",
            label: "Convert Line ↔ Spline",
            menu: "Object",
            shortcut: "",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: has_selection,
            run: convert,
        },
        Command {
            id: "object.toggle_closed",
            label: "Open/Close",
            menu: "Object",
            shortcut: "",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: has_selection,
            run: toggle_closed,
        },
        Command {
            id: "object.toggle_forward_arrow",
            label: "Toggle Forward Arrow",
            menu: "Object/Arrows",
            shortcut: "",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: has_selection,
            run: |e, p| toggle_arrow(e, p, true),
        },
        Command {
            id: "object.toggle_backward_arrow",
            label: "Toggle Backward Arrow",
            menu: "Object/Arrows",
            shortcut: "",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: has_selection,
            run: |e, p| toggle_arrow(e, p, false),
        },
        Command {
            id: "point.add",
            label: "Add point",
            menu: "",
            shortcut: "",
            params: "{\"id\", \"index\": insert before this vertex, \"x\", \"y\"}",
            mutates: true,
            enabled: |_| true,
            run: point_add,
        },
        Command {
            id: "point.delete",
            label: "Delete point",
            menu: "",
            shortcut: "",
            params: "{\"id\", \"index\"}",
            mutates: true,
            enabled: |_| true,
            run: point_delete,
        },
        Command {
            id: "point.move",
            label: "Move point",
            menu: "",
            shortcut: "",
            params: "{\"id\", \"index\", \"x\", \"y\"}",
            mutates: true,
            enabled: |_| true,
            run: point_move,
        },
    ]
}

fn group(e: &mut Engine, p: &Value) -> CmdResult {
    let ids = e.target_ids(p)?;
    if ids.len() < 2 {
        return Err(EngineError::bad_params("select at least two objects to group"));
    }
    let doc = e.doc_mut();
    let first = doc.objects.iter().position(|o| ids.contains(&o.id)).unwrap_or(0);
    let mut members = Vec::new();
    let mut rest = Vec::new();
    for o in std::mem::take(&mut doc.objects) {
        if ids.contains(&o.id) {
            members.push(Arc::unwrap_or_clone(o));
        } else {
            rest.push(o);
        }
    }
    let mut c = Object::new(ObjectKind::Compound(Compound { objects: members }));
    c.id = doc.alloc_id();
    let id = c.id;
    rest.insert(first.min(rest.len()), Arc::new(c));
    doc.objects = rest;
    e.set_selection(vec![id]);
    Ok(json!({"id": id.0}))
}

fn ungroup(e: &mut Engine, p: &Value) -> CmdResult {
    let ids = e.target_ids(p)?;
    let doc = e.doc_mut();
    let mut out = Vec::new();
    let mut freed = Vec::new();
    let mut broke = 0;
    for o in std::mem::take(&mut doc.objects) {
        match &o.kind {
            ObjectKind::Compound(_) if ids.contains(&o.id) => {
                broke += 1;
                if let ObjectKind::Compound(c) = Arc::unwrap_or_clone(o).kind {
                    for child in c.objects {
                        freed.push(child.id);
                        out.push(Arc::new(child));
                    }
                }
            }
            _ => out.push(o),
        }
    }
    doc.objects = out;
    if broke == 0 {
        return Err(EngineError::bad_params("no compound selected"));
    }
    e.set_selection(freed.clone());
    Ok(json!({"ids": ids_value(&freed)}))
}

fn walk_mut(o: &mut Object, f: &mut dyn FnMut(&mut Object)) {
    if let ObjectKind::Compound(c) = &mut o.kind {
        for x in &mut c.objects {
            walk_mut(x, f);
        }
    } else {
        f(o);
    }
}

fn for_targets(e: &mut Engine, p: &Value, f: &mut dyn FnMut(&mut Object)) -> Result<usize, EngineError> {
    let ids = e.target_ids(p)?;
    if ids.is_empty() {
        return Err(EngineError::bad_params("no objects selected"));
    }
    for o in e.doc_mut().objects.iter_mut() {
        if ids.contains(&o.id) {
            walk_mut(Arc::make_mut(o), f);
        }
    }
    Ok(ids.len())
}

fn update(e: &mut Engine, p: &Value) -> CmdResult {
    let style = e.indicator.style();
    let ind = e.indicator.clone();
    let (fa, ba) = ind.arrows();
    let n = for_targets(e, p, &mut |o| {
        if let ObjectKind::Text(t) = &mut o.kind {
            t.color = ColorRef(ind.pen_color);
            t.depth = style.depth;
            t.font = ind.font;
            t.size = ind.font_size.clamp(1.0, 2000.0);
            t.justify = ind.justify;
            t.flags = ind.text_flags();
            return;
        }
        if let Some(s) = o.style_mut() {
            *s = style;
        }
        if let Some((f, b)) = o.arrows_mut() {
            *f = fa;
            *b = ba;
        }
        if let ObjectKind::Polyline(pl) = &mut o.kind {
            if pl.kind == PolyKind::ArcBox {
                pl.radius = ind.box_radius;
            }
        }
        if let ObjectKind::Arc(a) = &mut o.kind {
            a.pie = ind.pie_arc;
        }
    })?;
    Ok(json!({"count": n}))
}

fn set_attributes(e: &mut Engine, p: &Value) -> CmdResult {
    // Parse everything first so errors leave nothing half-applied.
    let line = match opt_str(p, "line")? {
        Some(n) => {
            Some(LineKind::from_name(n).ok_or_else(|| EngineError::bad_params(format!("unknown line style {n:?}")))?)
        }
        None => None,
    };
    let cap = match opt_str(p, "cap")? {
        Some("butt") => Some(CapStyle::Butt),
        Some("round") => Some(CapStyle::Round),
        Some("projecting") => Some(CapStyle::Projecting),
        Some(o) => return Err(EngineError::bad_params(format!("unknown cap {o:?}"))),
        None => None,
    };
    let join = match opt_str(p, "join")? {
        Some("miter") => Some(JoinStyle::Miter),
        Some("round") => Some(JoinStyle::Round),
        Some("bevel") => Some(JoinStyle::Bevel),
        Some(o) => return Err(EngineError::bad_params(format!("unknown join {o:?}"))),
        None => None,
    };
    let justify = match opt_str(p, "justify")? {
        Some(j) => Some(super::shape::parse_justify(j)?),
        None => None,
    };
    let int = |k: &str, lo: i32, hi: i32| -> Result<Option<i32>, EngineError> {
        Ok(opt_num(p, k)?.map(|v| (v.round() as i32).clamp(lo, hi)))
    };
    let thickness = int("thickness", 0, 10_000)?;
    let pen = int("pen_color", -1, rustfig_model::LAST_USER_COLOR)?;
    let fill = int("fill_color", -1, rustfig_model::LAST_USER_COLOR)?;
    let area = int("area_fill", -1, 62)?;
    let depth = int("depth", 0, MAX_DEPTH)?;
    let radius = int("radius", 0, 100_000)?;
    let font = int("font", -1, 34)?;
    let style_val = opt_num(p, "style_val")?.map(|v| v.clamp(0.0, 1000.0));
    let font_size = opt_num(p, "font_size")?.map(|v| v.clamp(1.0, 2000.0));
    let angle = opt_num(p, "angle")?.map(f64::to_radians);
    let text = opt_str(p, "text")?.map(str::to_string);
    if text.as_ref().is_some_and(|t| t.len() > rustfig_format::MAX_TEXT_BYTES) {
        return Err(EngineError::bad_params("text is too long"));
    }
    let postscript = opt_bool(p, "postscript")?;
    let hidden = opt_bool(p, "hidden")?;
    let fwd = opt_bool(p, "forward_arrow")?;
    let back = opt_bool(p, "backward_arrow")?;
    let akind = int("arrow_kind", 0, 14)?;
    let astyle = int("arrow_style", 0, 1)?;
    let aw = opt_num(p, "arrow_width")?.map(|v| v.clamp(1.0, 1.0e5));
    let ah = opt_num(p, "arrow_height")?.map(|v| v.clamp(1.0, 1.0e5));
    let comment = opt_str(p, "comment")?.map(str::to_string);
    let template = e.indicator.arrow;
    let n = for_targets(e, p, &mut |o| {
        if let Some(c) = &comment {
            o.comments = if c.is_empty() {
                Vec::new()
            } else {
                c.lines().map(str::to_string).collect()
            };
        }
        if let ObjectKind::Text(t) = &mut o.kind {
            if let Some(v) = pen {
                t.color = ColorRef(v);
            }
            if let Some(v) = depth {
                t.depth = v;
            }
            if let Some(v) = font {
                t.font = v;
            }
            if let Some(v) = font_size {
                t.size = v;
            }
            if let Some(v) = justify {
                t.justify = v;
            }
            if let Some(v) = angle {
                t.angle = v;
            }
            if let Some(v) = &text {
                t.text = v.clone();
            }
            let set = |flags: &mut i32, bit: i32, on: Option<bool>| match on {
                Some(true) => *flags |= bit,
                Some(false) => *flags &= !bit,
                None => {}
            };
            set(&mut t.flags, rustfig_model::text_flags::POSTSCRIPT, postscript);
            set(&mut t.flags, rustfig_model::text_flags::HIDDEN, hidden);
            return;
        }
        if let Some(s) = o.style_mut() {
            if let Some(v) = line {
                s.line = v;
                if v != LineKind::Solid && s.style_val <= 0.0 {
                    s.style_val = 4.0;
                }
            }
            if let Some(v) = thickness {
                s.thickness = v;
            }
            if let Some(v) = pen {
                s.pen_color = ColorRef(v);
            }
            if let Some(v) = fill {
                s.fill_color = ColorRef(v);
            }
            if let Some(v) = area {
                s.area_fill = v;
            }
            if let Some(v) = depth {
                s.depth = v;
            }
            if let Some(v) = style_val {
                s.style_val = v;
            }
            if let Some(v) = cap {
                s.cap = v;
            }
            if let Some(v) = join {
                s.join = v;
            }
        }
        if let (Some(r), ObjectKind::Polyline(pl)) = (radius, &mut o.kind) {
            pl.radius = r;
        }
        if let Some((f, b)) = o.arrows_mut() {
            let adjust = |a: &mut Option<Arrow>, on: Option<bool>| {
                match on {
                    Some(true) if a.is_none() => *a = Some(template),
                    Some(false) => *a = None,
                    _ => {}
                }
                if let Some(a) = a {
                    if let Some(v) = akind {
                        a.kind = v;
                    }
                    if let Some(v) = astyle {
                        a.style = v;
                    }
                    if let Some(v) = aw {
                        a.width = v;
                    }
                    if let Some(v) = ah {
                        a.height = v;
                    }
                }
            };
            adjust(f, fwd);
            adjust(b, back);
        }
    })?;
    Ok(json!({"count": n}))
}

fn get(e: &mut Engine, p: &Value) -> CmdResult {
    let ids = e.target_ids(p)?;
    let objs: Vec<&Object> = e
        .doc()
        .objects
        .iter()
        .filter(|o| ids.contains(&o.id))
        .map(|o| &**o)
        .collect();
    serde_json::to_value(objs)
        .map(|v| json!({"objects": v}))
        .map_err(|err| EngineError::internal(err.to_string()))
}

fn rect_json(r: &Rect) -> Value {
    if r.is_empty() {
        return Value::Null;
    }
    json!({"x": r.min.x, "y": r.min.y, "width": r.width(), "height": r.height()})
}

fn list(e: &mut Engine, _: &Value) -> CmdResult {
    let v: Vec<Value> = e
        .doc()
        .objects
        .iter()
        .map(|o| {
            json!({
                "id": o.id.0,
                "type": o.type_name(),
                "depth": o.depth(),
                "bounds": rect_json(&o.geometry_bbox()),
                "selected": e.selection().contains(&o.id),
            })
        })
        .collect();
    Ok(json!({"objects": v}))
}

fn measure(e: &mut Engine, p: &Value) -> CmdResult {
    let ids = e.target_ids(p)?;
    let mut length = 0.0;
    let mut area = 0.0;
    let mut bb = Rect::EMPTY;
    for id in &ids {
        let Some(o) = e.doc().get(*id) else { continue };
        bb = bb.union(&o.geometry_bbox());
        let mut leaves = Vec::new();
        collect_leaves(o, &mut leaves);
        for l in leaves {
            for fp in flatten_object(l) {
                length += polyline_length(&fp.pts, fp.closed);
                if fp.closed {
                    area += polygon_area(&fp.pts);
                }
            }
        }
    }
    let d = e.doc();
    let unit = d.unit_name();
    let u = |v: f64| d.to_units(v);
    let msg = format!(
        "length {:.3} {unit}, area {:.3} {unit}², size {:.3} × {:.3} {unit}",
        u(length),
        u(u(area)),
        u(bb.width()),
        u(bb.height())
    );
    e.set_status(msg.clone());
    Ok(json!({"length": length, "area": area, "bounds": rect_json(&bb), "units": unit, "summary": msg}))
}

fn collect_leaves<'a>(o: &'a Object, out: &mut Vec<&'a Object>) {
    match &o.kind {
        ObjectKind::Compound(c) => c.objects.iter().for_each(|x| collect_leaves(x, out)),
        _ => out.push(o),
    }
}

fn convert(e: &mut Engine, p: &Value) -> CmdResult {
    let n = for_targets(e, p, &mut |o| {
        let new = match &o.kind {
            ObjectKind::Polyline(pl) if pl.kind != PolyKind::Picture && pl.points.len() >= 2 => {
                let closed = pl.kind.is_closed();
                if closed && pl.points.len() < 3 {
                    return;
                }
                let mut s = Spline::new(SplineKind::Interpolated, closed, pl.style, pl.points.clone());
                if pl.kind == PolyKind::ArcBox {
                    s.points = Rect::from_points(pl.points.iter().copied()).corners().to_vec();
                    s.shapes = vec![-1.0; 4];
                }
                s.forward_arrow = pl.forward_arrow;
                s.backward_arrow = pl.backward_arrow;
                ObjectKind::Spline(s)
            }
            ObjectKind::Spline(s) => {
                let kind = if s.closed {
                    PolyKind::Polygon
                } else {
                    PolyKind::Polyline
                };
                let mut pl = Polyline::new(kind, s.style, s.points.clone());
                pl.forward_arrow = s.forward_arrow;
                pl.backward_arrow = s.backward_arrow;
                ObjectKind::Polyline(pl)
            }
            _ => return,
        };
        o.kind = new;
    })?;
    Ok(json!({"count": n}))
}

fn toggle_closed(e: &mut Engine, p: &Value) -> CmdResult {
    let n = for_targets(e, p, &mut |o| match &mut o.kind {
        ObjectKind::Polyline(pl) => match pl.kind {
            PolyKind::Polyline if pl.points.len() >= 3 => {
                pl.kind = PolyKind::Polygon;
                pl.forward_arrow = None;
                pl.backward_arrow = None;
            }
            PolyKind::Polygon => pl.kind = PolyKind::Polyline,
            PolyKind::Box | PolyKind::ArcBox => {
                pl.kind = PolyKind::Polyline;
            }
            _ => {}
        },
        ObjectKind::Spline(s) => {
            if s.closed || s.points.len() >= 3 {
                s.closed = !s.closed;
                if s.closed {
                    s.forward_arrow = None;
                    s.backward_arrow = None;
                    let d = s.kind.default_shape();
                    if let Some(f) = s.shapes.first_mut() {
                        *f = d;
                    }
                    if let Some(l) = s.shapes.last_mut() {
                        *l = d;
                    }
                } else {
                    if let Some(f) = s.shapes.first_mut() {
                        *f = 0.0;
                    }
                    if let Some(l) = s.shapes.last_mut() {
                        *l = 0.0;
                    }
                }
            }
        }
        ObjectKind::Arc(a) => {
            a.pie = !a.pie;
            if a.pie {
                a.forward_arrow = None;
                a.backward_arrow = None;
            }
        }
        _ => {}
    })?;
    Ok(json!({"count": n}))
}

fn toggle_arrow(e: &mut Engine, p: &Value, forward: bool) -> CmdResult {
    let template = rustfig_model::Arrow {
        thickness: f64::from(e.indicator.thickness.max(1)),
        ..e.indicator.arrow
    };
    let n = for_targets(e, p, &mut |o| {
        if let Some((f, b)) = o.arrows_mut() {
            let a = if forward { f } else { b };
            *a = if a.is_some() { None } else { Some(template) };
        }
    })?;
    Ok(json!({"count": n}))
}

fn target(e: &Engine, p: &Value) -> Result<(ObjectId, usize), EngineError> {
    let id = ObjectId(num(p, "id")?.round().max(0.0) as u64);
    let idx = num(p, "index")?;
    if idx < 0.0 {
        return Err(EngineError::bad_params("index must not be negative"));
    }
    if e.doc().get(id).is_none() {
        return Err(EngineError::not_found(format!("no top-level object with id {}", id.0)));
    }
    Ok((id, idx as usize))
}

fn xy(p: &Value) -> Result<Point, EngineError> {
    point_value(&json!([num(p, "x")?, num(p, "y")?]))
}

fn point_add(e: &mut Engine, p: &Value) -> CmdResult {
    let (id, idx) = target(e, p)?;
    let pt = xy(p)?;
    let o = e
        .doc_mut()
        .get_mut(id)
        .ok_or_else(|| EngineError::not_found("object vanished"))?;
    match &mut o.kind {
        ObjectKind::Polyline(pl) if matches!(pl.kind, PolyKind::Polyline | PolyKind::Polygon) => {
            let i = idx.min(pl.points.len());
            pl.points.insert(i, pt);
        }
        ObjectKind::Spline(s) => {
            let i = idx.min(s.points.len());
            s.points.insert(i, pt);
            let shape = s.kind.default_shape();
            let si = i.min(s.shapes.len());
            s.shapes.insert(si, shape);
            if !s.closed {
                if let Some(f) = s.shapes.first_mut() {
                    *f = 0.0;
                }
                if let Some(l) = s.shapes.last_mut() {
                    *l = 0.0;
                }
            }
        }
        _ => {
            return Err(EngineError::bad_params(
                "points can only be added to polylines, polygons and splines",
            ))
        }
    }
    Ok(json!({}))
}

fn point_delete(e: &mut Engine, p: &Value) -> CmdResult {
    let (id, idx) = target(e, p)?;
    let o = e
        .doc_mut()
        .get_mut(id)
        .ok_or_else(|| EngineError::not_found("object vanished"))?;
    let too_few = || EngineError::bad_params("the object would have too few points");
    match &mut o.kind {
        ObjectKind::Polyline(pl) if matches!(pl.kind, PolyKind::Polyline | PolyKind::Polygon) => {
            let min = if pl.kind == PolyKind::Polygon { 4 } else { 2 };
            if pl.points.len() < min || idx >= pl.points.len() {
                return Err(too_few());
            }
            pl.points.remove(idx);
        }
        ObjectKind::Spline(s) => {
            let min = if s.closed { 4 } else { 3 };
            if s.points.len() < min || idx >= s.points.len() {
                return Err(too_few());
            }
            s.points.remove(idx);
            if idx < s.shapes.len() {
                s.shapes.remove(idx);
            }
            if !s.closed {
                if let Some(f) = s.shapes.first_mut() {
                    *f = 0.0;
                }
                if let Some(l) = s.shapes.last_mut() {
                    *l = 0.0;
                }
            }
        }
        _ => {
            return Err(EngineError::bad_params(
                "points can only be deleted from polylines, polygons and splines",
            ))
        }
    }
    Ok(json!({}))
}

fn point_move(e: &mut Engine, p: &Value) -> CmdResult {
    let (id, idx) = target(e, p)?;
    let pt = xy(p)?;
    let o = e
        .doc_mut()
        .get_mut(id)
        .ok_or_else(|| EngineError::not_found("object vanished"))?;
    let bad = || EngineError::bad_params("no such point");
    match &mut o.kind {
        ObjectKind::Polyline(pl) if pl.kind.is_rect() => {
            if idx >= 4 || pl.points.len() != 4 {
                return Err(bad());
            }
            let opposite = pl.points.get((idx + 2) % 4).copied().ok_or_else(bad)?;
            // Keep the vertex order so the moved corner stays at `idx`.
            let r = Rect::new(opposite, pt);
            pl.points = r.corners().to_vec();
        }
        ObjectKind::Polyline(pl) => *pl.points.get_mut(idx).ok_or_else(bad)? = pt,
        ObjectKind::Spline(s) => *s.points.get_mut(idx).ok_or_else(bad)? = pt,
        ObjectKind::Arc(a) => {
            *a.points.get_mut(idx).ok_or_else(bad)? = pt;
            let before = a.clone();
            a.refit();
            if rustfig_geom::arc_from_three_points(a.points[0], a.points[1], a.points[2]).is_none() {
                *a = before;
                return Err(EngineError::bad_params("arc points must not be collinear"));
            }
        }
        ObjectKind::Ellipse(el) => {
            // Point 0 is the centre, point 1 the corner of the bounding box.
            match idx {
                0 => {
                    let d = pt - el.center;
                    el.center = pt;
                    el.start = el.start + d;
                    el.end = el.end + d;
                }
                1 => {
                    let d = pt - el.center;
                    el.radii = Point::new(d.x.abs(), d.y.abs());
                    if el.kind.is_circle() {
                        let r = d.len();
                        el.radii = Point::new(r, r);
                    }
                    el.end = pt;
                }
                _ => return Err(bad()),
            }
        }
        _ => return Err(bad()),
    }
    Ok(json!({}))
}
