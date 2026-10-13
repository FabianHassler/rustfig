//! Edit commands: undo/redo, clipboard, delete, selection, search/replace.

use super::{always, has_objects, has_selection, ids_value, num_or, opt_bool, string, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_model::{Object, ObjectId, ObjectKind};
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "edit.undo",
            label: "Undo",
            menu: "Edit",
            shortcut: "Ctrl+Z",
            params: "{}",
            mutates: false,
            enabled: |e| e.can_undo(),
            run: |e, _| Ok(json!({"done": e.undo()})),
        },
        Command {
            id: "edit.redo",
            label: "Redo",
            menu: "Edit",
            shortcut: "Ctrl+Shift+Z",
            params: "{}",
            mutates: false,
            enabled: |e| e.can_redo(),
            run: |e, _| Ok(json!({"done": e.redo()})),
        },
        Command {
            id: "edit.cut",
            label: "Cut",
            menu: "Edit",
            shortcut: "Ctrl+X",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: has_selection,
            run: cut,
        },
        Command {
            id: "edit.copy",
            label: "Copy",
            menu: "Edit",
            shortcut: "Ctrl+C",
            params: "{\"ids\"?: [id]}",
            mutates: false,
            enabled: has_selection,
            run: copy,
        },
        Command {
            id: "edit.paste",
            label: "Paste",
            menu: "Edit",
            shortcut: "Ctrl+V",
            params: "{\"dx\"?: number, \"dy\"?: number}  — offset in Fig units (default 150)",
            mutates: true,
            enabled: |e| !e.clipboard().is_empty(),
            run: paste,
        },
        Command {
            id: "edit.duplicate",
            label: "Duplicate",
            menu: "Edit",
            shortcut: "Ctrl+D",
            params: "{\"ids\"?: [id], \"dx\"?: number, \"dy\"?: number}",
            mutates: true,
            enabled: has_selection,
            run: duplicate,
        },
        Command {
            id: "edit.delete",
            label: "Delete",
            menu: "Edit",
            shortcut: "Delete",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: has_selection,
            run: delete,
        },
        Command {
            id: "edit.delete_all",
            label: "Delete All",
            menu: "Edit",
            shortcut: "",
            params: "{}",
            mutates: true,
            enabled: has_objects,
            run: |e, _| {
                e.doc_mut().objects.clear();
                e.set_selection(Vec::new());
                Ok(json!({}))
            },
        },
        Command {
            id: "edit.select_all",
            label: "Select All",
            menu: "Edit",
            shortcut: "Ctrl+A",
            params: "{}",
            mutates: false,
            enabled: always,
            run: |e, _| {
                let ids: Vec<ObjectId> = e
                    .doc()
                    .objects
                    .iter()
                    .filter(|o| e.is_visible(o))
                    .map(|o| o.id)
                    .collect();
                e.set_selection(ids.clone());
                Ok(json!({"ids": ids_value(&ids)}))
            },
        },
        Command {
            id: "edit.select_none",
            label: "Select None",
            menu: "Edit",
            shortcut: "Escape",
            params: "{}",
            mutates: false,
            enabled: always,
            run: |e, _| {
                e.set_selection(Vec::new());
                Ok(json!({}))
            },
        },
        Command {
            id: "edit.select",
            label: "Select objects",
            menu: "",
            shortcut: "",
            params: "{\"ids\": [id], \"add\"?: bool}",
            mutates: false,
            enabled: always,
            run: select,
        },
        Command {
            id: "edit.search_replace",
            label: "Search/Replace text",
            menu: "",
            shortcut: "",
            params: "{\"find\": string, \"replace\"?: string, \"case_sensitive\"?: bool}  — without replace, selects matching texts",
            mutates: true,
            enabled: has_objects,
            run: search_replace,
        },
    ]
}

fn clone_targets(e: &Engine, p: &Value) -> Result<Vec<Object>, EngineError> {
    let ids = e.target_ids(p)?;
    Ok(e.doc()
        .objects
        .iter()
        .filter(|o| ids.contains(&o.id))
        .cloned()
        .collect())
}

fn copy(e: &mut Engine, p: &Value) -> CmdResult {
    let objs = clone_targets(e, p)?;
    let n = objs.len();
    e.set_clipboard(objs);
    Ok(json!({"copied": n}))
}

fn cut(e: &mut Engine, p: &Value) -> CmdResult {
    copy(e, p)?;
    delete(e, p)
}

fn insert_copies(e: &mut Engine, objs: Vec<Object>, dx: f64, dy: f64) -> CmdResult {
    let mut ids = Vec::new();
    for mut o in objs {
        o.translate(dx, dy);
        ids.push(e.add_object(o));
    }
    e.set_selection(ids.clone());
    Ok(json!({"ids": ids_value(&ids)}))
}

fn paste(e: &mut Engine, p: &Value) -> CmdResult {
    let objs = e.clipboard().to_vec();
    let dx = num_or(p, "dx", 150.0)?;
    let dy = num_or(p, "dy", 150.0)?;
    insert_copies(e, objs, dx, dy)
}

fn duplicate(e: &mut Engine, p: &Value) -> CmdResult {
    let objs = clone_targets(e, p)?;
    let dx = num_or(p, "dx", 150.0)?;
    let dy = num_or(p, "dy", 150.0)?;
    insert_copies(e, objs, dx, dy)
}

fn delete(e: &mut Engine, p: &Value) -> CmdResult {
    let ids = e.target_ids(p)?;
    if ids.is_empty() {
        return Err(EngineError::bad_params("nothing to delete"));
    }
    e.doc_mut().objects.retain(|o| !ids.contains(&o.id));
    let sel: Vec<ObjectId> = e.selection().iter().copied().filter(|i| !ids.contains(i)).collect();
    e.set_selection(sel);
    Ok(json!({"deleted": ids.len()}))
}

fn select(e: &mut Engine, p: &Value) -> CmdResult {
    let mut ids = e.target_ids(p)?;
    if opt_bool(p, "add")?.unwrap_or(false) {
        let mut cur = e.selection().to_vec();
        cur.append(&mut ids);
        ids = cur;
    }
    e.set_selection(ids);
    Ok(json!({"ids": ids_value(e.selection())}))
}

fn search_replace(e: &mut Engine, p: &Value) -> CmdResult {
    let find = string(p, "find")?;
    if find.is_empty() {
        return Err(EngineError::bad_params("\"find\" must not be empty"));
    }
    let replace = super::opt_str(p, "replace")?.map(str::to_string);
    let case = opt_bool(p, "case_sensitive")?.unwrap_or(true);
    let matches = |s: &str| {
        if case {
            s.contains(&find)
        } else {
            s.to_lowercase().contains(&find.to_lowercase())
        }
    };
    let mut hits: Vec<ObjectId> = Vec::new();
    let mut count = 0usize;
    // Searching never changes the document; replacing does.
    let top: Vec<(ObjectId, bool)> = e
        .doc()
        .objects
        .iter()
        .map(|o| (o.id, contains_text(o, &matches)))
        .collect();
    for (id, hit) in top {
        if hit {
            hits.push(id);
        }
    }
    if let Some(rep) = replace {
        let find_c = find.clone();
        for o in e.doc_mut().objects.iter_mut() {
            count += replace_text(o, &find_c, &rep, case);
        }
        e.fix_all_text();
    }
    e.set_selection(hits.clone());
    Ok(json!({"matches": hits.len(), "replaced": count, "ids": ids_value(&hits)}))
}

fn contains_text(o: &Object, m: &dyn Fn(&str) -> bool) -> bool {
    match &o.kind {
        ObjectKind::Text(t) => m(&t.text),
        ObjectKind::Compound(c) => c.objects.iter().any(|x| contains_text(x, m)),
        _ => false,
    }
}

fn replace_text(o: &mut Object, find: &str, rep: &str, case: bool) -> usize {
    match &mut o.kind {
        ObjectKind::Text(t) => {
            if case {
                let n = t.text.matches(find).count();
                if n > 0 {
                    t.text = t.text.replace(find, rep);
                }
                n
            } else {
                let lower = t.text.to_lowercase();
                let fl = find.to_lowercase();
                if !lower.contains(&fl) || lower.len() != t.text.len() {
                    // Fall back to exact replacement when case folding changes lengths.
                    let n = t.text.matches(find).count();
                    t.text = t.text.replace(find, rep);
                    return n;
                }
                let mut out = String::new();
                let mut n = 0;
                let mut i = 0;
                while let Some(pos) = lower.get(i..).and_then(|s| s.find(&fl)) {
                    out.push_str(t.text.get(i..i + pos).unwrap_or(""));
                    out.push_str(rep);
                    i += pos + fl.len();
                    n += 1;
                }
                out.push_str(t.text.get(i..).unwrap_or(""));
                t.text = out;
                n
            }
        }
        ObjectKind::Compound(c) => c.objects.iter_mut().map(|x| replace_text(x, find, rep, case)).sum(),
        _ => 0,
    }
}
