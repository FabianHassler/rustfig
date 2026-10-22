//! Object commands: grouping, attributes, points, conversion, inspection.

use super::{has_selection, ids_value, num, opt_bool, opt_num, opt_str, point_value, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_geom::{polygon_area, polyline_length, Affine, Point, Rect};
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
            params: "{\"ids\"?: [id], \"line\"?, \"thickness\"?, \"pen_color\"?, \"fill_color\"?, \"area_fill\"?, \"depth\"?, \"style_val\"?, \"cap\"?: butt|round|projecting, \"join\"?: miter|round|bevel, \"forward_arrow\"?: bool, \"backward_arrow\"?: bool, \"arrow_kind\"?, \"arrow_style\"?, \"arrow_width\"?, \"arrow_height\"?, \"radius\"?, \"font\"?, \"font_size\"?, \"justify\"?, \"angle\"?: degrees, \"text\"?, \"postscript\"?: bool, \"hidden\"?: bool, \"rigid\"?: bool, \"special\"?: bool, \"pie\"?: bool, \"flipped\"?: bool (pictures), \"arrow_thickness\"?, \"comment\"?: string, \"texts\"?: [string] (compound: the texts directly in it, in order), \"forward_arrow_spec\"?, \"backward_arrow_spec\"?: {kind, style, thickness, width, height}, \"geometry\"?: as object.set_geometry without id}",
            mutates: true,
            enabled: |_| true,
            run: set_attributes,
        },
        Command {
            id: "object.set_geometry",
            label: "Set geometry",
            menu: "",
            shortcut: "",
            params: "{\"id\", \"points\"?: [[x, y]...] (box/arc-box/picture: two opposite corners; polyline/polygon/spline: all points, same count; arc: three points; ellipse/circle: [centre]; text: [origin]), \"radii\"?: [rx, ry], \"angle\"?: degrees (ellipse), \"shapes\"?: [factor per spline point], \"resize\"?: bool (polyline/polygon/spline: the number of points may change; a spline then takes one shape factor per point or none), compound: points [top-left, bottom-right] repositions and rescales it}  — xfig's edit panel rows, Fig units",
            mutates: true,
            enabled: |_| true,
            run: set_geometry,
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

pub(super) fn update(e: &mut Engine, p: &Value) -> CmdResult {
    // The indicator's update flags (xfig's Update Control) choose which
    // attributes are copied.
    let ind = e.indicator.clone();
    let n = for_targets(e, p, &mut |o| ind.update_object(o))?;
    Ok(json!({"count": n}))
}

fn set_attributes(e: &mut Engine, p: &Value) -> CmdResult {
    // The edit panel's position rows travel with its attributes, so one
    // Apply is one undo step.
    if let Some(g) = p.get("geometry") {
        let ids = e.target_ids(p)?;
        let [id] = ids.as_slice() else {
            return Err(EngineError::bad_params("geometry needs exactly one object"));
        };
        let mut gp = g.clone();
        match gp.as_object_mut() {
            Some(m) => {
                m.insert("id".into(), json!(id.0));
            }
            None => return Err(EngineError::bad_params("geometry must be an object")),
        }
        set_geometry(e, &gp)?;
    }
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
    let rigid = opt_bool(p, "rigid")?;
    let special = opt_bool(p, "special")?;
    let pie = opt_bool(p, "pie")?;
    let flipped = opt_bool(p, "flipped")?;
    let athick = opt_num(p, "arrow_thickness")?.map(|v| v.clamp(0.0, 1.0e4));
    let fwd = opt_bool(p, "forward_arrow")?;
    let back = opt_bool(p, "backward_arrow")?;
    let akind = int("arrow_kind", 0, 14)?;
    let astyle = int("arrow_style", 0, 1)?;
    let aw = opt_num(p, "arrow_width")?.map(|v| v.clamp(1.0, 1.0e5));
    let ah = opt_num(p, "arrow_height")?.map(|v| v.clamp(1.0, 1.0e5));
    let comment = opt_str(p, "comment")?.map(str::to_string);
    let forward_spec = arrow_spec(p, "forward_arrow_spec")?;
    let backward_spec = arrow_spec(p, "backward_arrow_spec")?;
    let template = e.indicator.arrow;
    let n = for_targets(e, p, &mut |o| {
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
            set(&mut t.flags, rustfig_model::text_flags::RIGID, rigid);
            set(&mut t.flags, rustfig_model::text_flags::SPECIAL, special);
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
        if let (Some(v), ObjectKind::Arc(a)) = (pie, &mut o.kind) {
            a.pie = v;
        }
        if let (Some(v), ObjectKind::Polyline(pl)) = (flipped, &mut o.kind) {
            if let Some(pic) = &mut pl.picture {
                pic.flipped = v;
            }
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
                    if let Some(v) = athick {
                        a.thickness = v;
                    }
                }
            };
            adjust(f, fwd);
            adjust(b, back);
            // Per-arrow type and size (xfig's two arrow rows).
            for (slot, spec) in [(f, forward_spec.as_ref()), (b, backward_spec.as_ref())] {
                if let (Some(a), Some(sp)) = (slot.as_mut(), spec) {
                    a.kind = sp.0.kind;
                    a.style = sp.0.style;
                    a.thickness = sp.0.thickness;
                    a.width = sp.0.width;
                    a.height = sp.0.height;
                }
            }
        }
    })?;
    // A compound's panel lists the texts directly in it (`e_edit.c`).
    if let Some(v) = p.get("texts") {
        let bad_texts = || EngineError::bad_params("texts must be an array of strings");
        let list: Vec<String> = v
            .as_array()
            .ok_or_else(bad_texts)?
            .iter()
            .map(|x| x.as_str().map(str::to_string))
            .collect::<Option<_>>()
            .ok_or_else(bad_texts)?;
        if list.iter().any(|t| t.len() > rustfig_format::MAX_TEXT_BYTES) {
            return Err(EngineError::bad_params("text is too long"));
        }
        let ids = e.target_ids(p)?;
        for o in e.doc_mut().objects.iter_mut() {
            if !ids.contains(&o.id) {
                continue;
            }
            if let ObjectKind::Compound(c) = &mut Arc::make_mut(o).kind {
                let texts = c.objects.iter_mut().filter_map(|m| match &mut m.kind {
                    ObjectKind::Text(t) => Some(t),
                    _ => None,
                });
                for (t, new) in texts.zip(list.iter()) {
                    t.text = new.clone();
                }
            }
        }
    }
    // A comment belongs to the edited object itself (a compound's own
    // comment, not its members').
    if let Some(c) = &comment {
        let ids = e.target_ids(p)?;
        let lines: Vec<String> = if c.is_empty() {
            Vec::new()
        } else {
            c.lines().map(str::to_string).collect()
        };
        for o in e.doc_mut().objects.iter_mut() {
            if ids.contains(&o.id) {
                Arc::make_mut(o).comments = lines.clone();
            }
        }
    }
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
    // xfig's `init_convert_line_spline`: lines and splines turn into each
    // other, a spline coming back as the kind it was; a box becomes a
    // rounded box (radius of the indicator's Box Curve) and the other way
    // round (`box_2_box`).
    let radius = e.indicator.box_radius.max(1);
    let mut remembered = e.convert_kind;
    let n = for_targets(e, p, &mut |o| {
        let new = match &o.kind {
            ObjectKind::Polyline(pl) if pl.kind == PolyKind::Box || pl.kind == PolyKind::ArcBox => {
                let mut pl = pl.clone();
                if pl.kind == PolyKind::Box {
                    pl.kind = PolyKind::ArcBox;
                    // xfig keeps a radius the box already has only when
                    // it is neither default nor 0; RustFig's plain boxes
                    // carry the Box Curve they were drawn with, so the
                    // indicator's value always applies.
                    pl.radius = radius;
                } else {
                    pl.kind = PolyKind::Box;
                }
                ObjectKind::Polyline(pl)
            }
            ObjectKind::Polyline(pl) if pl.kind != PolyKind::Picture && pl.points.len() >= 2 => {
                let closed = pl.kind.is_closed();
                if closed && pl.points.len() < 3 {
                    return;
                }
                let mut s = Spline::new(remembered, closed, pl.style, pl.points.clone());
                s.forward_arrow = pl.forward_arrow;
                s.backward_arrow = pl.backward_arrow;
                ObjectKind::Spline(s)
            }
            ObjectKind::Spline(s) => {
                remembered = s.kind;
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
    e.convert_kind = remembered;
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
    insert_point(o, idx, pt)?;
    Ok(json!({}))
}

/// Insert a point before vertex `idx` of a line, polygon or spline.
pub(super) fn insert_point(o: &mut Object, idx: usize, pt: Point) -> Result<(), EngineError> {
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
    Ok(())
}

fn point_delete(e: &mut Engine, p: &Value) -> CmdResult {
    let (id, idx) = target(e, p)?;
    let o = e
        .doc_mut()
        .get_mut(id)
        .ok_or_else(|| EngineError::not_found("object vanished"))?;
    remove_point(o, idx)?;
    Ok(json!({}))
}

/// Remove vertex `idx` of a line, polygon or spline, keeping the minimum
/// number of points.
pub(super) fn remove_point(o: &mut Object, idx: usize) -> Result<(), EngineError> {
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
    Ok(())
}

fn point_move(e: &mut Engine, p: &Value) -> CmdResult {
    let (id, idx) = target(e, p)?;
    let pt = xy(p)?;
    // Smart links: a box corner moved resizes the box, and the line ends
    // attached to its edges stay on them.
    let mode = e.indicator.linkmode;
    let links = crate::links::collect(e, &[id]);
    let old_rect = match e.doc().get(id).map(|o| &o.kind) {
        Some(ObjectKind::Polyline(pl)) if pl.kind.is_rect() => Some(Rect::from_points(pl.points.iter().copied())),
        _ => None,
    };
    let r = point_move_object(e, id, idx, pt);
    if r.is_ok() {
        if let (Some(old), Some(ObjectKind::Polyline(pl))) = (old_rect, e.doc().get(id).map(|o| &o.kind)) {
            let new = Rect::from_points(pl.points.iter().copied());
            crate::links::adjust(e, mode, &links, &crate::links::rect_map(old, new));
        }
    }
    r
}

fn point_move_object(e: &mut Engine, id: ObjectId, idx: usize, pt: Point) -> CmdResult {
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
            // The Fig writer rounds coordinates to integers, so a box
            // smaller than half a unit would be written (and read back)
            // as a zero-size box and dropped.
            if r.width().round() <= 0.0 || r.height().round() <= 0.0 {
                return Err(EngineError::bad_params("box corners must not coincide"));
            }
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

fn pt_list(p: &Value, key: &str) -> Result<Option<Vec<Point>>, EngineError> {
    let Some(v) = p.get(key) else { return Ok(None) };
    let arr = v
        .as_array()
        .ok_or_else(|| EngineError::bad_params(format!("{key} must be an array of [x, y]")))?;
    let mut out = Vec::new();
    for item in arr {
        let xy = item.as_array().filter(|a| a.len() == 2);
        let (Some(x), Some(y)) = (
            xy.and_then(|a| a.first()).and_then(Value::as_f64),
            xy.and_then(|a| a.get(1)).and_then(Value::as_f64),
        ) else {
            return Err(EngineError::bad_params(format!("{key} must be an array of [x, y]")));
        };
        if !x.is_finite() || !y.is_finite() || x.abs() > 1.0e7 || y.abs() > 1.0e7 {
            return Err(EngineError::bad_params("coordinate out of range"));
        }
        out.push(Point::new(x, y));
    }
    Ok(Some(out))
}

/// xfig's edit panel rows for positions (`get_new_*_values` in `e_edit.c`).
fn set_geometry(e: &mut Engine, p: &Value) -> CmdResult {
    let id = ObjectId(
        p.get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| EngineError::bad_params("id is required"))?,
    );
    let points = pt_list(p, "points")?;
    let radii = match p.get("radii") {
        Some(v) => {
            let a = v.as_array().filter(|a| a.len() == 2);
            match (
                a.and_then(|a| a.first()).and_then(Value::as_f64),
                a.and_then(|a| a.get(1)).and_then(Value::as_f64),
            ) {
                (Some(x), Some(y)) if x.is_finite() && y.is_finite() => {
                    Some(Point::new(x.abs().min(1.0e7), y.abs().min(1.0e7)))
                }
                _ => return Err(EngineError::bad_params("radii must be [rx, ry]")),
            }
        }
        None => None,
    };
    let angle = opt_num(p, "angle")?;
    let shapes: Option<Vec<f64>> = match p.get("shapes") {
        None => None,
        Some(v) => Some(
            v.as_array()
                .ok_or_else(|| EngineError::bad_params("shapes must be an array of numbers"))?
                .iter()
                .map(|x| x.as_f64().filter(|f| f.is_finite()).map(|f| f.clamp(-1.0, 1.0)))
                .collect::<Option<Vec<f64>>>()
                .ok_or_else(|| EngineError::bad_params("shapes must be an array of numbers"))?,
        ),
    };
    let resize = opt_bool(p, "resize")?.unwrap_or(false);
    let bad = |m: &str| EngineError::bad_params(m.to_string());
    // A compound's corners are its fig2dev bounds (xfig's `nwcorner` and
    // `secorner`).
    let old_bounds = match e.doc().get(id) {
        Some(o) if matches!(o.kind, ObjectKind::Compound(_)) => {
            let mut ctx = rustfig_format::bridge::from_model::context(e.doc());
            Some(rustfig_format::bridge::object_bounds(o, &mut ctx))
        }
        _ => None,
    };
    let o = e
        .doc_mut()
        .get_mut(id)
        .ok_or_else(|| EngineError::not_found("no such object"))?;
    let mut new = o.clone();
    match &mut new.kind {
        ObjectKind::Polyline(pl) if pl.kind.is_rect() => {
            if let Some(pts) = &points {
                let (Some(a), Some(b)) = (pts.first(), pts.get(1)) else {
                    return Err(bad("two corners are required"));
                };
                // xfig deletes a zero-size box when it reads the file.
                if a.x == b.x || a.y == b.y {
                    return Err(bad("a box needs a width and a height"));
                }
                pl.points = vec![*a, Point::new(b.x, a.y), *b, Point::new(a.x, b.y)];
            }
        }
        ObjectKind::Polyline(pl) => {
            if let Some(pts) = points {
                if pts.len() != pl.points.len() {
                    if !resize || !matches!(pl.kind, PolyKind::Polyline | PolyKind::Polygon) {
                        return Err(bad("the number of points cannot change"));
                    }
                    if pts.is_empty() {
                        return Err(bad("a line needs at least one point"));
                    }
                }
                // xfig deletes a polygon with fewer than 3 distinct points.
                if pl.kind == PolyKind::Polygon && distinct_run(&pts) < 3 {
                    return Err(bad("a polygon needs at least 3 distinct points"));
                }
                pl.points = pts;
            }
        }
        ObjectKind::Spline(s) => {
            if let Some(pts) = points {
                if pts.len() != s.points.len() {
                    if !resize {
                        return Err(bad("the number of points cannot change"));
                    }
                    if pts.len() < if s.closed { 3 } else { 2 } {
                        return Err(bad("the spline would have too few points"));
                    }
                    // New points get the kind's own shape factor; `shapes`
                    // below overrides them.
                    let shape = s.kind.default_shape();
                    s.shapes.resize(pts.len(), shape);
                }
                if pts.iter().all(|q| Some(q) == pts.first()) {
                    return Err(bad("a spline needs at least two different points"));
                }
                s.points = pts;
            }
            if let Some(sh) = shapes {
                if sh.len() != s.points.len() {
                    return Err(bad("one shape factor per point is required"));
                }
                // xfig: a factor that is not the kind's own makes it an X-spline.
                let own = match s.kind {
                    SplineKind::Approximated => Some(1.0),
                    SplineKind::Interpolated => Some(-1.0),
                    SplineKind::X => None,
                };
                if let Some(own) = own {
                    let n = sh.len();
                    let differs = sh.iter().enumerate().any(|(i, v)| {
                        let end = !s.closed && (i == 0 || i + 1 == n);
                        !end && (*v - own).abs() > 1e-9
                    });
                    if differs {
                        s.kind = SplineKind::X;
                    }
                }
                s.shapes = sh;
                if !s.closed {
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
            if let Some(pts) = points {
                let [p0, p1, p2] = pts.as_slice() else {
                    return Err(bad("three points are required"));
                };
                let (p0, p1, p2) = (*p0, *p1, *p2);
                if rustfig_geom::arc_from_three_points(p0, p1, p2).is_none() {
                    return Err(bad("arc points must not be collinear"));
                }
                a.points = [p0, p1, p2];
                a.refit();
            }
        }
        ObjectKind::Ellipse(el) => {
            if let Some(c) = points.as_ref().and_then(|v| v.first()) {
                el.center = *c;
            }
            if let Some(r) = radii {
                if r.x < 1.0 || (!el.kind.is_circle() && r.y < 1.0) {
                    return Err(bad("radii must be above zero"));
                }
                el.radii = if el.kind.is_circle() { Point::new(r.x, r.x) } else { r };
            }
            if let Some(a) = angle {
                el.angle = a.to_radians().rem_euclid(std::f64::consts::TAU);
            }
            use rustfig_model::EllipseKind as K;
            el.start = match el.kind {
                K::EllipseRadii | K::CircleRadius => el.center,
                K::EllipseDiameter | K::CircleDiameter => Point::new(el.center.x - el.radii.x, el.center.y),
            };
            el.end = Point::new(el.center.x + el.radii.x, el.center.y);
        }
        ObjectKind::Text(t) => {
            if let Some(c) = points.as_ref().and_then(|v| v.first()) {
                t.origin = *c;
            }
        }
        ObjectKind::Compound(_) => {
            // xfig's `get_new_compound_values`: translate to the new
            // top-left corner, then scale about it to the bottom-right one.
            let corners = || bad("a compound needs its top-left and bottom-right corners");
            let Some(pts) = &points else { return Err(corners()) };
            let (Some(nw), Some(se)) = (pts.first(), pts.get(1)) else {
                return Err(corners());
            };
            let Some(b) = old_bounds.filter(|b| !b.is_empty()) else {
                return Err(bad("the compound has no extent"));
            };
            let ratio = |new: f64, old: f64| if old.abs() < 1e-9 { 1.0 } else { new / old };
            let sx = ratio(se.x - nw.x, b.max.x - b.min.x);
            let sy = ratio(se.y - nw.y, b.max.y - b.min.y);
            if sx.abs() < 1e-6 || sy.abs() < 1e-6 || sx.abs() > 1e6 || sy.abs() > 1e6 {
                return Err(bad("the compound's size must not be zero"));
            }
            new.translate(nw.x - b.min.x, nw.y - b.min.y);
            if (sx - 1.0).abs() > 1e-12 || (sy - 1.0).abs() > 1e-12 {
                new.transform(&Affine::about(Affine::scale(sx, sy), *nw), true);
            }
        }
    }
    *o = new;
    Ok(json!({}))
}

/// `{"kind", "style", "thickness", "width", "height"}` (width and height in
/// Fig units) for one arrow, as the edit panel's arrow rows give it.
struct ArrowSpec(Arrow);

fn arrow_spec(p: &Value, key: &str) -> Result<Option<ArrowSpec>, EngineError> {
    let Some(v) = p.get(key) else { return Ok(None) };
    let f = |k: &str, lo: f64, hi: f64| -> Result<f64, EngineError> {
        v.get(k)
            .and_then(Value::as_f64)
            .filter(|x| x.is_finite())
            .map(|x| x.clamp(lo, hi))
            .ok_or_else(|| EngineError::bad_params(format!("{key}.{k} must be a number")))
    };
    Ok(Some(ArrowSpec(Arrow {
        kind: f("kind", 0.0, 14.0)? as i32,
        style: f("style", 0.0, 1.0)? as i32,
        thickness: f("thickness", 0.0, 1.0e4)?,
        width: f("width", 0.0, 1.0e5)?,
        height: f("height", 0.0, 1.0e5)?,
    })))
}

/// Number of points after dropping consecutive repeats.
fn distinct_run(pts: &[Point]) -> usize {
    let mut n = 0;
    let mut last = None;
    for p in pts {
        if last != Some(*p) {
            n += 1;
        }
        last = Some(*p);
    }
    n
}
