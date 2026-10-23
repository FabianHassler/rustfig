//! Library mode (plan D6): list the figure libraries and their objects,
//! place an object, copy objects to a library of your own.
//
// Ported from xfig src/w_library.c and src/e_placelib.c (xfig licence, see LICENSE-xfig).

#[cfg(not(target_arch = "wasm32"))]
use super::opt_str;
use super::{always, opt_bool, opt_num, string, CmdResult, Command};
use crate::library::{self, LibOp};
use crate::{Engine, EngineError, UiRequest};
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "library.panel",
            label: "Library",
            menu: "",
            shortcut: "Shift+L",
            params: "{}  — xfig's library mode button: open the library panel (Load a Library, object list or icons, preview, comments, Select object); a front end without one does nothing",
            mutates: false,
            enabled: always,
            run: |e, _| {
                e.request(UiRequest::OpenLibrary);
                Ok(json!({"requested": true}))
            },
        },
        Command {
            id: "library.list",
            label: "List libraries",
            menu: "",
            shortcut: "",
            params: "{}  — the bundled libraries and the ones in the preference library_dir: {\"libraries\": [{\"name\": \"Electronic / Schematic / Classic\", \"objects\": n, \"user\": bool}]}",
            mutates: false,
            enabled: always,
            run: list,
        },
        Command {
            id: "library.source",
            label: "Library source",
            menu: "",
            shortcut: "",
            params: "{}  — what the preference library_dir names, a folder or xfig's library list file (one library folder per line as `PATH [NAME]`, `#` comments): {\"path\", \"kind\": \"folder\"|\"list\"|\"missing\"|\"none\", \"entries\"?: [{\"name\", \"path\", \"exists\": bool}]}",
            mutates: false,
            enabled: always,
            run: source,
        },
        Command {
            id: "library.use",
            label: "Use library folder or list",
            menu: "",
            shortcut: "",
            params: "{\"path\": folder or list file}  — makes it your library source (sets library_dir, as `-library_dir` does) after checking that it exists and, for a list file, names at least one folder; returns library.source's reply plus \"libraries\": n",
            mutates: false,
            enabled: always,
            run: use_source,
        },
        Command {
            id: "library.search",
            label: "Search libraries",
            menu: "",
            shortcut: "",
            params: "{\"query\": text, \"limit\"?: n (default 200, at most 1000), \"comments\"?: bool (default true)}  — the figures of all libraries whose name (and comment lines) hold every word of the query, case-insensitive, names first: {\"query\", \"hits\": [{\"library\", \"name\", \"user\": bool, \"comment\": bool}], \"truncated\": bool}; place one with library.place or library.hand",
            mutates: false,
            enabled: always,
            run: search,
        },
        Command {
            id: "library.objects",
            label: "List library objects",
            menu: "",
            shortcut: "",
            params: "{\"library\": name, \"comments\"?: bool}  — the figures of a library: {\"objects\": [{\"name\", \"comments\"?: [line]}]}",
            mutates: false,
            enabled: always,
            run: objects,
        },
        Command {
            id: "library.place",
            label: "Place library object",
            menu: "",
            shortcut: "",
            params: "{\"library\": name, \"name\": object, \"x\", \"y\" (Fig units: where its top-left corner goes) or \"orig\": true (at its position in the library file), \"ops\"?: [rotate_cw|rotate_ccw|flip_h|flip_v|smaller|larger] (xfig's keys r, l, h, v, s, S, applied about the object's centre)}  — adds the object as one compound and selects it; returns {\"id\", \"ids\"}",
            mutates: true,
            enabled: always,
            run: place,
        },
        Command {
            id: "library.hand",
            label: "Take library object",
            menu: "",
            shortcut: "",
            params: "{\"library\": name, \"name\": object, \"ops\"?: [...] (as library.place)}  — xfig's Select object: the object follows the pointer (no mode is current): left places it (library.place; the mode stays), middle opens the library panel again, right or Escape cancels, Shift+left places and edits it, Shift+middle shows a box instead of the object, Shift+right places it at its original position; the keys r l h v s S turn, flip, shrink and enlarge it; returns {\"attached\": true}",
            mutates: false,
            enabled: always,
            run: hand,
        },
        Command {
            id: "library.transform",
            label: "Change library object in hand",
            menu: "",
            shortcut: "",
            params: "{\"op\": rotate_cw|rotate_ccw|flip_h|flip_v|smaller|larger}  — while a library object follows the pointer (xfig's keys r, l, h, v, s, S)",
            mutates: false,
            enabled: |e| e.library_in_hand().is_some(),
            run: |e, p| {
                let op = string(p, "op")?;
                let op = LibOp::from_name(&op).ok_or_else(|| EngineError::bad_params(format!("unknown op {op:?}")))?;
                e.library_transform(op)?;
                Ok(json!({"ops": e.library_in_hand().map(|(_, _, o)| o.len()).unwrap_or(0)}))
            },
        },
        Command {
            id: "library.save",
            label: "Copy to library",
            menu: "",
            shortcut: "",
            params: "{\"name\": figure name (a file name without .fig), \"library\"?: folder inside your library folder (`A/B`; default the folder itself), \"ids\"?: [id] (default the selection, else the whole figure), \"comments\"?: string}  — saves the objects as a library figure in the folder of the preference library_dir; returns {\"path\", \"library\", \"objects\"}",
            mutates: false,
            enabled: always,
            run: save,
        },
    ]
}

fn list(e: &mut Engine, _: &Value) -> CmdResult {
    let libs: Vec<Value> = e
        .libraries()
        .into_iter()
        .map(|l| json!({"name": l.name, "objects": l.objects, "user": l.user}))
        .collect();
    Ok(json!({"libraries": libs}))
}

fn source(e: &mut Engine, _: &Value) -> CmdResult {
    let pref = e
        .prefs
        .get("library_dir")
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "~/xfiglib".to_string());
    Ok(source_json(&pref))
}

fn source_json(pref: &str) -> Value {
    use library::Source;
    match library::source(pref) {
        None => json!({"path": "", "kind": "none"}),
        Some(Source::Folder(p)) => json!({"path": p.to_string_lossy(), "kind": "folder"}),
        Some(Source::Missing(p)) => json!({"path": p.to_string_lossy(), "kind": "missing"}),
        Some(Source::List(p, entries)) => {
            let es: Vec<Value> = entries
                .iter()
                .map(|x| json!({"name": x.name, "path": x.path.to_string_lossy(), "exists": x.path.is_dir()}))
                .collect();
            json!({"path": p.to_string_lossy(), "kind": "list", "entries": es})
        }
    }
}

fn use_source(e: &mut Engine, p: &Value) -> CmdResult {
    use library::Source;
    let path = string(p, "path")?;
    match library::source(&path) {
        None => return Err(EngineError::bad_params("the path is empty")),
        Some(Source::Missing(_)) => {
            return Err(EngineError::bad_params(format!(
                "{path:?} is not a folder or a readable library list"
            )))
        }
        Some(Source::List(_, entries)) if entries.is_empty() => {
            return Err(EngineError::bad_params(format!("{path:?} names no library folder")))
        }
        Some(_) => {}
    }
    e.prefs
        .set("library_dir", toml::Value::String(path.clone()))
        .map_err(EngineError::bad_params)?;
    let mut v = source_json(&path);
    if let Some(o) = v.as_object_mut() {
        o.insert("libraries".into(), json!(e.libraries().len()));
    }
    Ok(v)
}

fn search(e: &mut Engine, p: &Value) -> CmdResult {
    let query = string(p, "query")?;
    let limit = match opt_num(p, "limit")? {
        Some(n) if n >= 0.0 => (n as usize).min(library::MAX_HITS),
        Some(_) => return Err(EngineError::bad_params("limit must not be negative")),
        None => 200,
    };
    let comments = opt_bool(p, "comments")?.unwrap_or(true);
    let hits = library::search(&e.libraries(), &query, limit.saturating_add(1), comments);
    let truncated = hits.len() > limit;
    let hits: Vec<Value> = hits
        .into_iter()
        .take(limit)
        .map(|h| json!({"library": h.library, "name": h.name, "user": h.user, "comment": h.by_comment}))
        .collect();
    Ok(json!({"query": query, "hits": hits, "truncated": truncated}))
}

fn objects(e: &mut Engine, p: &Value) -> CmdResult {
    let lib = e.find_library(&string(p, "library")?)?;
    let with_comments = opt_bool(p, "comments")?.unwrap_or(false);
    let objs: Vec<Value> = library::object_names(&lib.path)
        .into_iter()
        .map(|n| {
            if with_comments {
                let c = library::object_comments(&lib.path, &n);
                json!({"name": n, "comments": c})
            } else {
                json!({"name": n})
            }
        })
        .collect();
    Ok(json!({"library": lib.name, "objects": objs}))
}

fn ops_of(p: &Value) -> Result<Vec<LibOp>, EngineError> {
    match p.get("ops") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(a)) => {
            if a.len() > 256 {
                return Err(EngineError::bad_params("too many ops"));
            }
            a.iter()
                .map(|v| {
                    v.as_str()
                        .and_then(LibOp::from_name)
                        .ok_or_else(|| EngineError::bad_params(format!("unknown op {v}")))
                })
                .collect()
        }
        Some(_) => Err(EngineError::bad_params("ops must be a list of names")),
    }
}

fn hand(e: &mut Engine, p: &Value) -> CmdResult {
    e.begin_library(&string(p, "library")?, &string(p, "name")?, ops_of(p)?)?;
    Ok(json!({"attached": true}))
}

fn place(e: &mut Engine, p: &Value) -> CmdResult {
    let library = string(p, "library")?;
    let name = string(p, "name")?;
    let ops = ops_of(p)?;
    let orig = opt_bool(p, "orig")?.unwrap_or(false);
    let at = match (opt_num(p, "x")?, opt_num(p, "y")?) {
        (Some(x), Some(y)) => Some((x, y)),
        (None, None) => None,
        _ => return Err(EngineError::bad_params("give both x and y, or neither")),
    };
    if at.is_none() && !orig {
        return Err(EngineError::bad_params(
            "give x and y, or orig (library.hand takes the object in hand)",
        ));
    }
    let lo = e.library_object(&library, &name)?;
    let mut obj = library::with_ops(&lo.object, &ops);
    if let (false, Some((x, y))) = (orig, at) {
        let nw = library::nw_of(&obj);
        obj.translate(x - nw.x, y - nw.y);
    }
    let id = e.add_object(obj);
    e.set_selection(vec![id]);
    e.set_status(format!("Placed library object {name}.fig"));
    Ok(json!({"id": id.0, "ids": [id.0]}))
}

#[cfg(not(target_arch = "wasm32"))]
fn save(e: &mut Engine, p: &Value) -> CmdResult {
    let name = string(p, "name")?;
    if !library::valid_name(&name) {
        return Err(EngineError::bad_params("the name must be a plain file name"));
    }
    // With a library list file the first folder it names takes the figure.
    let root = e
        .library_roots()
        .into_iter()
        .find(|r| r.user && r.path.is_dir())
        .ok_or_else(|| EngineError::bad_params("no library folder: set the preference library_dir"))?;
    let mut dir = root.path;
    let mut sub_name = root.name.unwrap_or_default();
    if let Some(sub) = opt_str(p, "library")? {
        for part in sub.split(['/', '\\']).filter(|s| !s.is_empty()) {
            if !library::valid_name(part) {
                return Err(EngineError::bad_params(format!("bad library folder name {part:?}")));
            }
            dir.push(part);
            if !sub_name.is_empty() {
                sub_name.push_str(" / ");
            }
            sub_name.push_str(part);
        }
    }
    let mut ids = e.target_ids(p)?;
    if ids.is_empty() {
        ids = e.doc().objects.iter().map(|o| o.id).collect();
    }
    let mut doc = e.doc().clone();
    doc.objects = ids
        .iter()
        .filter_map(|i| e.doc().get(*i))
        .cloned()
        .map(std::sync::Arc::new)
        .collect();
    if doc.objects.is_empty() {
        return Err(EngineError::bad_params("there is nothing to copy to a library"));
    }
    doc.comments = opt_str(p, "comments")?
        .map(|c| c.lines().map(str::to_string).collect())
        .unwrap_or_default();
    doc.open.clear();
    let text = rustfig_format::write(&doc, rustfig_format::FileFormat::Fig)
        .map_err(|err| EngineError::bad_params(err.to_string()))?;
    std::fs::create_dir_all(&dir).map_err(|err| EngineError::bad_params(format!("{}: {err}", dir.display())))?;
    let path = dir.join(format!("{name}.fig"));
    std::fs::write(&path, text).map_err(|err| EngineError::bad_params(format!("{}: {err}", path.display())))?;
    Ok(json!({
        "path": path.to_string_lossy(),
        "library": if sub_name.is_empty() { library::TOP_NAME.to_string() } else { sub_name },
        "objects": doc.objects.len(),
    }))
}

#[cfg(target_arch = "wasm32")]
fn save(_: &mut Engine, _: &Value) -> CmdResult {
    Err(EngineError::bad_params("libraries cannot be written here"))
}
