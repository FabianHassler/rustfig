//! Find objects by properties (W10-39): `edit.find_objects` turns criteria
//! into the ids of the matching top-level objects. It is read-only; the
//! actions on the matches are the existing commands that take `ids`
//! (`object.update`, `arrange.set_depth`, `edit.delete`, `object.group`),
//! each one undo step.
//!
//! Find mode (`tool.find`) keeps a match set: `find.set` fills it from the
//! same criteria, a left click or `find.toggle` adds or removes one object,
//! `find.state` reads it and `find.apply` acts on all of it (update, depth,
//! delete, group), each one undo step.
//!
//! Compounds: by default only top-level objects are tested, and a compound
//! matches by its kind and depth only (it has no other attributes). With
//! `inside` a compound also matches when any member, at any depth of
//! nesting, matches; the result then names the top-level compound, because
//! the commands act on top-level objects.

use super::{always, num, opt_bool, opt_num, opt_str, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_model::{LineKind, Object, ObjectId, ObjectKind, PolyKind, Style};
use serde_json::{json, Value};

/// Longest result list returned in `items` (all ids are always returned).
const MAX_ITEMS: usize = 500;

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "find.set",
            label: "Find: set matches",
            menu: "",
            shortcut: "",
            params: "{same criteria as edit.find_objects}  — enters Find mode (as tool.find) and makes the objects that match the match set, replacing it; returns what find.state returns",
            mutates: false,
            enabled: |e| !e.doc().objects.is_empty(),
            run: find_set,
        },
        Command {
            id: "find.toggle",
            label: "Find: add or remove object",
            menu: "",
            shortcut: "",
            params: "{\"id\": object id}  — Find mode: adds the object to the match set, or removes it when it is in (what a left click does); returns what find.state returns",
            mutates: false,
            enabled: always,
            run: find_toggle,
        },
        Command {
            id: "find.state",
            label: "Find: matches",
            menu: "",
            shortcut: "",
            params: "{}  — read-only; returns {mode: bool (Find mode active), count, ids, items: [{id, type, depth, label}]} for the match set",
            mutates: false,
            enabled: always,
            run: |e, _| Ok(find_state(e)),
        },
        Command {
            id: "find.apply",
            label: "Find: act on matches",
            menu: "",
            shortcut: "",
            params: "{\"action\": update|depth|delete|group, \"depth\"?: 0-999 (for depth)}  — Find mode: one undo step on all matches; update gives them the indicator's attributes by the update flags, delete empties the match set, group leaves the new compound as the only match; returns {count}",
            mutates: false,
            enabled: always,
            run: find_apply,
        },
        Command {
        id: "edit.find_objects",
        label: "Find objects",
        menu: "",
        shortcut: "",
        params: "{\"kind\"?: line|box|polygon|spline|arc|ellipse|text|picture|compound (or a list of them), \"pen_color\"?: colour number, \"fill_color\"?: colour number, \"fill\"?: none|filled|pattern, \"depth\"?: n, \"depth_min\"?, \"depth_max\"?: 0-999, \"thickness\"?: n, \"line\"?: line style name, \"arrows\"?: any|none, \"font\"?: font number, \"font_size\"?: points, \"text\"?: substring (case-insensitive), \"inside\"?: bool (also match members of compounds; the result names the top-level compound)}  — read-only; criteria combine with AND; returns {count, ids, items: [{id, type, depth, label}]}; act on the matches with object.update, arrange.set_depth, edit.delete or object.group and \"ids\"",
        mutates: false,
        enabled: |e| !e.doc().objects.is_empty(),
        run: find_objects,
        },
    ]
}

#[derive(Default)]
struct Criteria {
    kinds: Vec<String>,
    pen: Option<i32>,
    fill_color: Option<i32>,
    fill: Option<String>,
    depth_min: Option<i32>,
    depth_max: Option<i32>,
    thickness: Option<i32>,
    line: Option<LineKind>,
    arrows: Option<bool>,
    font: Option<i32>,
    font_size: Option<f64>,
    text: Option<String>,
    inside: bool,
}

const KINDS: [&str; 9] = [
    "line", "box", "polygon", "spline", "arc", "ellipse", "text", "picture", "compound",
];

fn int(p: &Value, key: &str) -> Result<Option<i32>, EngineError> {
    Ok(opt_num(p, key)?.map(|f| f.round().clamp(-1.0e6, 1.0e6) as i32))
}

fn parse(p: &Value) -> Result<Criteria, EngineError> {
    let mut c = Criteria::default();
    match p.get("kind") {
        None | Some(Value::Null) => {}
        Some(Value::String(s)) => c.kinds.push(s.clone()),
        Some(Value::Array(a)) => {
            for v in a {
                let s = v
                    .as_str()
                    .ok_or_else(|| EngineError::bad_params("\"kind\" must be a name or a list of names"))?;
                c.kinds.push(s.to_string());
            }
        }
        Some(_) => return Err(EngineError::bad_params("\"kind\" must be a name or a list of names")),
    }
    for k in &c.kinds {
        if !KINDS.contains(&k.as_str()) {
            return Err(EngineError::bad_params(format!(
                "unknown kind {k:?} (one of {})",
                KINDS.join(", ")
            )));
        }
    }
    c.pen = int(p, "pen_color")?;
    c.fill_color = int(p, "fill_color")?;
    c.fill = match opt_str(p, "fill")? {
        None => None,
        Some(s @ ("none" | "filled" | "pattern")) => Some(s.to_string()),
        Some(o) => return Err(EngineError::bad_params(format!("unknown fill {o:?}"))),
    };
    if let Some(d) = int(p, "depth")? {
        c.depth_min = Some(d);
        c.depth_max = Some(d);
    }
    if let Some(d) = int(p, "depth_min")? {
        c.depth_min = Some(d);
    }
    if let Some(d) = int(p, "depth_max")? {
        c.depth_max = Some(d);
    }
    c.thickness = int(p, "thickness")?;
    c.line = match opt_str(p, "line")? {
        Some(n) => {
            Some(LineKind::from_name(n).ok_or_else(|| EngineError::bad_params(format!("unknown line style {n:?}")))?)
        }
        None => None,
    };
    c.arrows = match opt_str(p, "arrows")? {
        None => None,
        Some("any") => Some(true),
        Some("none") => Some(false),
        Some(o) => {
            return Err(EngineError::bad_params(format!(
                "arrows must be \"any\" or \"none\", not {o:?}"
            )))
        }
    };
    c.font = int(p, "font")?;
    c.font_size = opt_num(p, "font_size")?;
    c.text = opt_str(p, "text")?.map(str::to_lowercase);
    c.inside = opt_bool(p, "inside")?.unwrap_or(false);
    Ok(c)
}

fn kind_name(o: &Object) -> &'static str {
    match &o.kind {
        ObjectKind::Ellipse(_) => "ellipse",
        ObjectKind::Polyline(p) => match p.kind {
            PolyKind::Polyline => "line",
            PolyKind::Box | PolyKind::ArcBox => "box",
            PolyKind::Polygon => "polygon",
            PolyKind::Picture => "picture",
        },
        ObjectKind::Spline(_) => "spline",
        ObjectKind::Text(_) => "text",
        ObjectKind::Arc(_) => "arc",
        ObjectKind::Compound(_) => "compound",
    }
}

fn fill_class(s: &Style) -> &'static str {
    match s.area_fill {
        i32::MIN..=-1 => "none",
        0..=40 => "filled",
        _ => "pattern",
    }
}

fn depth_ok(o: &Object, c: &Criteria) -> bool {
    let d = o.depth();
    c.depth_min.is_none_or(|m| d >= m) && c.depth_max.is_none_or(|m| d <= m)
}

fn kind_ok(o: &Object, c: &Criteria) -> bool {
    c.kinds.is_empty() || c.kinds.iter().any(|k| k == kind_name(o))
}

/// Whether `o` (a top-level object, or a member when `c.inside`) matches.
fn matches(o: &Object, c: &Criteria) -> bool {
    if let ObjectKind::Compound(cm) = &o.kind {
        let attrs = c.pen.is_some()
            || c.fill_color.is_some()
            || c.fill.is_some()
            || c.thickness.is_some()
            || c.line.is_some()
            || c.arrows.is_some()
            || c.font.is_some()
            || c.font_size.is_some()
            || c.text.is_some();
        let itself = kind_ok(o, c) && !attrs && depth_ok(o, c);
        return itself || (c.inside && cm.objects.iter().any(|m| matches(m, c)));
    }
    if !kind_ok(o, c) || !depth_ok(o, c) {
        return false;
    }
    if let Some(a) = c.arrows {
        let has = o.arrows().is_some_and(|(f, b)| f.is_some() || b.is_some());
        if has != a {
            return false;
        }
    }
    match &o.kind {
        ObjectKind::Text(t) => {
            c.pen.is_none_or(|v| t.color.0 == v)
                && c.fill_color.is_none()
                && c.fill.is_none()
                && c.thickness.is_none()
                && c.line.is_none()
                && c.font.is_none_or(|v| t.font == v)
                && c.font_size.is_none_or(|v| (t.size - v).abs() < 0.01)
                && c.text
                    .as_ref()
                    .is_none_or(|s| t.text.to_lowercase().contains(s.as_str()))
        }
        _ => {
            let Some(s) = o.style() else { return false };
            c.font.is_none()
                && c.font_size.is_none()
                && c.text.is_none()
                && c.pen.is_none_or(|v| s.pen_color.0 == v)
                && c.fill_color.is_none_or(|v| s.fill_color.0 == v)
                && c.fill.as_deref().is_none_or(|f| fill_class(s) == f)
                && c.thickness.is_none_or(|v| s.thickness == v)
                && c.line.is_none_or(|v| s.line == v)
        }
    }
}

fn label(o: &Object) -> String {
    match &o.kind {
        ObjectKind::Text(t) => {
            let first = t.text.lines().next().unwrap_or("");
            let short: String = first.chars().take(24).collect();
            format!("text \"{short}\"")
        }
        _ => o.type_name().to_string(),
    }
}

/// The result shape of `edit.find_objects` and `find.state`.
fn result_of(hits: &[&Object]) -> Value {
    let items: Vec<Value> = hits
        .iter()
        .take(MAX_ITEMS)
        .map(|o| json!({"id": o.id.0, "type": o.type_name(), "depth": o.depth(), "label": label(o)}))
        .collect();
    let ids: Vec<u64> = hits.iter().map(|o| o.id.0).collect();
    json!({"count": ids.len(), "ids": ids, "items": items})
}

fn find_objects(e: &mut Engine, p: &Value) -> CmdResult {
    let c = parse(p)?;
    let hits: Vec<&Object> = e
        .doc()
        .objects
        .iter()
        .map(|o| &**o)
        .filter(|o| matches(o, &c))
        .collect();
    Ok(result_of(&hits))
}

fn find_state(e: &Engine) -> Value {
    let hits: Vec<&Object> = e
        .find_matches()
        .into_iter()
        .filter_map(|id| e.doc().get(ObjectId(id)))
        .collect();
    let mut r = result_of(&hits);
    if let Some(m) = r.as_object_mut() {
        m.insert("mode".into(), json!(e.tool() == crate::tools::ToolKind::Find));
    }
    r
}

fn find_set(e: &mut Engine, p: &Value) -> CmdResult {
    let c = parse(p)?;
    let ids: Vec<ObjectId> = e
        .doc()
        .objects
        .iter()
        .filter(|o| matches(o, &c))
        .map(|o| o.id)
        .collect();
    e.find_replace(ids);
    Ok(find_state(e))
}

fn find_toggle(e: &mut Engine, p: &Value) -> CmdResult {
    let id = num(p, "id")?;
    if !id.is_finite() || id < 0.0 {
        return Err(EngineError::bad_params("\"id\" must be an object id"));
    }
    e.find_toggle(ObjectId(id as u64))?;
    Ok(find_state(e))
}

fn find_apply(e: &mut Engine, p: &Value) -> CmdResult {
    let action = opt_str(p, "action")?.ok_or_else(|| EngineError::bad_params("\"action\" is required"))?;
    let n = e.find_apply(action, opt_num(p, "depth")?)?;
    Ok(json!({"count": n}))
}
