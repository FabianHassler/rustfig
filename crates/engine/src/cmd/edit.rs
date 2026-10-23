//! Edit commands: undo/redo, clipboard, delete, selection, search/replace.

use super::{always, has_objects, ids_value, num_or, opt_bool, string, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_model::{Object, ObjectId, ObjectKind};
use serde_json::{json, Value};
use std::sync::Arc;

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
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: always,
            run: cut,
        },
        Command {
            id: "edit.copy",
            label: "Copy",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id]}",
            mutates: false,
            enabled: always,
            run: copy,
        },
        Command {
            id: "edit.copy_picture_pick",
            label: "Copy as Picture",
            menu: "Edit",
            shortcut: "",
            params: "{}  — the next click chooses what is copied as a picture: left the object under the pointer, middle a region (two corners or a loop), right the whole figure; Escape cancels",
            mutates: false,
            enabled: |e| e.picture_clipboard.is_some(),
            run: |e, _| {
                e.copy_pick = true;
                e.set_status("Copy as picture: click an object (left), a region (middle) or the whole figure (right); Escape cancels.");
                Ok(json!({}))
            },
        },
        Command {
            id: "edit.copy_picture",
            label: "Copy as Picture (ids)",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id], \"whole\"?: bool, \"magnification\"?: percent, \"margin\"?: fig units, \"dpi\"?: number, \"background\"?: \"#rrggbb\", \"options\"?: {key: value} fig2dev's options (for the PNG: e.g. transparent)}  — puts the selection (or the whole figure) on the system clipboard as the platform's vector picture (macOS PDF, Windows EMF, Linux SVG) and a PNG; returns {\"formats\": [...], \"bytes\": n}",
            mutates: false,
            enabled: always,
            run: copy_picture,
        },
        Command {
            id: "edit.paste",
            label: "Paste",
            menu: "Edit",
            shortcut: "Ctrl+V",
            params: "{\"dx\"?: number, \"dy\"?: number}  — without an offset the objects follow the pointer: left places them there, middle at their original position, right or Escape cancels; with dx/dy (Fig units) they are placed at once, moved by that offset",
            mutates: true,
            enabled: |e| !e.clipboard().is_empty() || e.picture_clipboard.is_some(),
            run: paste,
        },
        Command {
            id: "edit.duplicate",
            label: "Duplicate",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id], \"dx\"?: number, \"dy\"?: number}  — with smart links on, the lines attached to the objects are copied too (the original line ends follow the copy)",
            mutates: true,
            enabled: always,
            run: duplicate,
        },
        Command {
            id: "edit.copy_array",
            label: "Copy as array",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id], \"dx\", \"dy\", \"nx\"?: columns (indicator num_x_copies), \"ny\"?: rows (indicator num_y_copies)}  — xfig's array placement (u_drag.c): copies at i*dx, j*dy; with one column or row only: the copy at dx (columns) and/or at dy (rows); 0 and 0 is a plain copy",
            mutates: true,
            enabled: always,
            run: copy_array,
        },
        Command {
            id: "edit.delete",
            label: "Delete",
            menu: "",
            shortcut: "",
            params: "{\"ids\"?: [id]}",
            mutates: true,
            enabled: always,
            run: delete,
        },
        Command {
            id: "edit.delete_all",
            label: "Delete All",
            menu: "Edit",
            shortcut: "",
            params: "{}",
            mutates: true,
            enabled: |e| has_objects(e) || e.open_compounds() > 0,
            run: |e, _| {
                // xfig closes every open compound first (`e_delete.c`).
                e.doc_mut().close_all();
                e.doc_mut().objects.clear();
                e.set_selection(Vec::new());
                Ok(json!({}))
            },
        },
        Command {
            id: "edit.select_all",
            label: "Select All",
            menu: "",
            shortcut: "",
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
            menu: "",
            shortcut: "",
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
            params: "{\"find\": string, \"replace\"?: string, \"case_sensitive\"?: bool}  — without replace, selects matching texts and lists them in \"texts\" ({x,y,text}); an empty find matches every text",
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
        .map(|o| (**o).clone())
        .collect())
}

fn copy(e: &mut Engine, p: &Value) -> CmdResult {
    let objs = clone_targets(e, p)?;
    let n = objs.len();
    e.set_clipboard(objs.clone());
    publish_cut_buffer(e, objs);
    Ok(json!({"copied": n}))
}

/// Biggest Fig text taken from the system clipboard (untrusted data).
const MAX_CLIP_FIG: usize = 64 * 1024 * 1024;
/// Biggest image taken from the system clipboard.
const MAX_CLIP_IMAGE: usize = 64 * 1024 * 1024;
/// Longest edge of a pasted image in Fig units (6 inches).
const MAX_PASTE_EDGE: f64 = 7200.0;

/// E7: the cut buffer also goes onto the system clipboard, as Fig text and
/// the platform's pictures. Best effort: the internal cut buffer works
/// without a clipboard, and a busy clipboard must not fail Copy or Delete.
fn publish_cut_buffer(e: &mut Engine, objs: Vec<Object>) {
    use crate::picture::{PictureData, PictureKind};
    if e.picture_clipboard.is_none() || objs.is_empty() {
        return;
    }
    let mut doc = e.whole_doc().into_owned();
    doc.open.clear();
    doc.objects = objs.into_iter().map(Arc::new).collect();
    let fig = rustfig_format::write_fig(&doc);
    let params = json!({"magnification": e.doc().settings.magnification});
    let mut items = Vec::new();
    for kind in PictureKind::platform_set() {
        let kind = if kind == PictureKind::Emf && !crate::fig2dev::driver_available("emf") {
            PictureKind::Svg
        } else {
            kind
        };
        // A picture that cannot be made is left out; the Fig text still goes.
        if let Ok(bytes) = super::file::export_bytes(e, &doc, kind.language(), &params) {
            items.push(PictureData { kind, bytes });
        }
    }
    let Some(sink) = e.picture_clipboard.as_mut() else {
        return;
    };
    if sink.set_objects(&fig, &items).is_ok() {
        e.last_fig = Some(fig);
    } else {
        // The clipboard is busy or gone: Paste uses the cut buffer.
        e.last_fig = None;
    }
}

/// E7: before Paste, take what the system clipboard holds. Fig text from
/// another RustFig window (or a later copy) becomes the cut buffer; an image
/// becomes a picture object in it; the cut buffer's own text, nothing or
/// unreadable data leave the cut buffer as it is.
fn sync_from_system_clipboard(e: &mut Engine) {
    let (fig, image) = {
        let Some(sink) = e.picture_clipboard.as_mut() else {
            return;
        };
        let fig = sink.get_fig();
        let image = if fig.is_none() { sink.get_image() } else { None };
        (fig, image)
    };
    if let Some(fig) = fig {
        if e.last_fig.as_deref() == Some(fig.as_str()) {
            return;
        }
        match paste_fig_text(e, &fig) {
            Ok(objs) => {
                e.set_clipboard(objs);
                e.last_fig = Some(fig);
            }
            Err(msg) => e.set_status(format!("The clipboard's figure was ignored: {msg}")),
        }
    } else if let Some(img) = image {
        match picture_from_image(e, &img) {
            Ok(obj) => {
                e.set_clipboard(vec![obj]);
                // The image replaced whatever was copied before.
                e.last_fig = None;
            }
            Err(msg) => e.set_status(format!("The clipboard's image was ignored: {msg}")),
        }
    }
}

fn paste_fig_text(e: &mut Engine, fig: &str) -> Result<Vec<Object>, String> {
    if fig.len() > MAX_CLIP_FIG {
        return Err("too large".into());
    }
    let (mut doc, _) = rustfig_format::read_any_messages(fig.as_bytes()).map_err(|err| err.to_string())?;
    let base = e
        .path()
        .and_then(|p| std::path::Path::new(p).parent().map(std::path::Path::to_path_buf));
    let _ = crate::import::load_all_pictures(&mut doc, base.as_deref());
    let objs = super::file::mapped_objects(e, doc);
    if objs.is_empty() {
        return Err("it holds no objects".into());
    }
    Ok(objs)
}

/// A picture object (at the origin, at xfig's 80 pixels per inch, at most
/// six inches wide or high) for image bytes from the clipboard. The picture
/// file is written next to the figure (`clipboard-<hash>.png`), or into the
/// temporary folder while the figure has no file name, and the object
/// references it by path, like any picture in a .fig file.
fn picture_from_image(e: &Engine, bytes: &[u8]) -> Result<Object, String> {
    use rustfig_model::{Picture, PolyKind, Polyline};
    if bytes.is_empty() || bytes.len() > MAX_CLIP_IMAGE {
        return Err("not a usable size".into());
    }
    let img = crate::import::decode_picture(bytes)?;
    if img.width == 0 || img.height == 0 {
        return Err("empty image".into());
    }
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3)
    });
    let name = format!("clipboard-{hash:016x}.png");
    let dir = match e.path().and_then(|p| std::path::Path::new(p).parent()) {
        Some(d) if !d.as_os_str().is_empty() => d.to_path_buf(),
        _ => std::env::temp_dir().join("rustfig-clipboard"),
    };
    let file = write_clip_picture(&dir, &name, bytes)?;
    let (w, h) = (f64::from(img.width) * 15.0, f64::from(img.height) * 15.0);
    let k = (MAX_PASTE_EDGE / w.max(h)).min(1.0);
    let r = rustfig_geom::Rect::new(
        rustfig_geom::Point::new(0.0, 0.0),
        rustfig_geom::Point::new(w * k, h * k),
    );
    let mut pl = Polyline::new(PolyKind::Picture, e.indicator.style(), r.corners().to_vec());
    pl.style.thickness = 0;
    pl.picture = Some(Picture {
        flipped: false,
        file,
        image: Some(Arc::new(img)),
    });
    Ok(Object::new(ObjectKind::Polyline(pl)))
}

#[cfg(not(target_arch = "wasm32"))]
fn write_clip_picture(dir: &std::path::Path, name: &str, bytes: &[u8]) -> Result<String, String> {
    std::fs::create_dir_all(dir).map_err(|err| format!("{}: {err}", dir.display()))?;
    let path = dir.join(name);
    if !path.exists() {
        std::fs::write(&path, bytes).map_err(|err| format!("{}: {err}", path.display()))?;
    }
    Ok(path.to_string_lossy().into_owned())
}

#[cfg(target_arch = "wasm32")]
fn write_clip_picture(_dir: &std::path::Path, _name: &str, _bytes: &[u8]) -> Result<String, String> {
    Err("pictures cannot be saved here".into())
}

/// `edit.copy_picture` (E14): the selection, or the whole figure when
/// nothing is selected, as the platform's vector picture and a PNG on the
/// system clipboard.
fn copy_picture(e: &mut Engine, p: &Value) -> CmdResult {
    use crate::picture::{PictureData, PictureKind};
    if e.picture_clipboard.is_none() {
        return Err(EngineError::bad_params(
            "Copy as picture is not available here (there is no system clipboard)",
        ));
    }
    // `whole`: the whole figure (Copy as Picture's right button).
    let whole = p.get("whole").and_then(Value::as_bool).unwrap_or(false);
    let ids = if whole { Vec::new() } else { e.target_ids(p)? };
    let mut doc = e.whole_doc().into_owned();
    if !ids.is_empty() {
        let chosen: Vec<_> = ids
            .iter()
            .filter_map(|id| e.doc().objects.iter().find(|o| o.id == *id).cloned())
            .collect();
        doc.open.clear();
        doc.objects = chosen;
    }
    if doc.objects.is_empty() {
        return Err(EngineError::bad_params("nothing to copy: the figure is empty"));
    }
    // The export settings of the figure apply unless given.
    let mut params = p.clone();
    if let Some(m) = params.as_object_mut() {
        m.remove("ids");
        if !m.contains_key("magnification") {
            m.insert("magnification".into(), json!(e.doc().settings.magnification));
        }
    }
    let mut items = Vec::new();
    for kind in PictureKind::platform_set() {
        // Windows gets SVG when the EMF driver is missing.
        let kind = if kind == PictureKind::Emf && !crate::fig2dev::driver_available("emf") {
            PictureKind::Svg
        } else {
            kind
        };
        let bytes = super::file::export_bytes(e, &doc, kind.language(), &params)?;
        items.push(PictureData { kind, bytes });
    }
    let Some(sink) = e.picture_clipboard.as_mut() else {
        return Err(EngineError::bad_params(
            "cannot copy to the clipboard: the clipboard changed while copying",
        ));
    };
    sink.set_pictures(&items)
        .map_err(|err| EngineError::bad_params(format!("cannot copy to the clipboard: {err}")))?;
    let formats: Vec<&str> = items.iter().map(|i| i.kind.language()).collect();
    let bytes: usize = items.iter().map(|i| i.bytes.len()).sum();
    e.set_status(format!(
        "Copied the picture ({}) to the clipboard",
        formats.join(" and ")
    ));
    Ok(json!({"formats": formats, "bytes": bytes}))
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
    sync_from_system_clipboard(e);
    // Without an offset the objects follow the pointer until a click places
    // them (xfig's paste); scripts give `dx`/`dy`.
    if p.get("dx").is_none() && p.get("dy").is_none() {
        e.begin_paste()?;
        return Ok(json!({"attached": e.clipboard().len()}));
    }
    let objs = e.clipboard().to_vec();
    if objs.is_empty() {
        return Err(EngineError::bad_params("the cut buffer is empty"));
    }
    let dx = num_or(p, "dx", 150.0)?;
    let dy = num_or(p, "dy", 150.0)?;
    insert_copies(e, objs, dx, dy)
}

fn duplicate(e: &mut Engine, p: &Value) -> CmdResult {
    let objs = clone_targets(e, p)?;
    let dx = num_or(p, "dx", 150.0)?;
    let dy = num_or(p, "dy", 150.0)?;
    // Smart links: the lines attached to the originals are copied and the
    // originals' ends follow the copies (xfig's copy with links).
    let mode = e.indicator.linkmode;
    let links = crate::links::collect(e, &e.target_ids(p)?);
    let r = insert_copies(e, objs, dx, dy);
    crate::links::copy_adjust(e, mode, &links, &|q| rustfig_geom::Point::new(q.x + dx, q.y + dy));
    r
}

/// The offsets of an array copy as xfig places them (`place_*` in `u_drag.c`):
/// no counts is one copy at (dx, dy); counts below 2 are the special cases
/// (one copy along x and/or along y); otherwise an nx by ny grid without the
/// original's cell.
pub(crate) fn array_offsets(nx: u32, ny: u32, dx: f64, dy: f64) -> Vec<(f64, f64)> {
    if nx == 0 && ny == 0 {
        return vec![(dx, dy)];
    }
    if nx < 2 && ny < 2 {
        let mut v = Vec::new();
        if nx > 0 {
            v.push((dx, 0.0));
        }
        if ny > 0 {
            v.push((0.0, dy));
        }
        return v;
    }
    let (nx, ny) = (nx.max(1), ny.max(1));
    let mut v = Vec::new();
    for i in 0..nx {
        for j in 0..ny {
            if i != 0 || j != 0 {
                v.push((f64::from(i) * dx, f64::from(j) * dy));
            }
        }
    }
    v
}

fn copy_array(e: &mut Engine, p: &Value) -> CmdResult {
    let nx = opt_count(p, "nx")?.unwrap_or(e.indicator.num_x_copies);
    let ny = opt_count(p, "ny")?.unwrap_or(e.indicator.num_y_copies);
    let (dx, dy) = (super::num(p, "dx")?, super::num(p, "dy")?);
    if !dx.is_finite() || !dy.is_finite() || dx.abs() > 1.0e7 || dy.abs() > 1.0e7 {
        return Err(EngineError::bad_params("dx and dy out of range"));
    }
    let originals = clone_targets(e, p)?;
    let mut all = Vec::new();
    for (ox, oy) in array_offsets(nx, ny, dx, dy) {
        for mut o in originals.iter().cloned() {
            o.translate(ox, oy);
            all.push(e.add_object(o));
        }
    }
    e.set_selection(all.clone());
    Ok(json!({"count": all.len(), "ids": ids_value(&all)}))
}

fn opt_count(p: &Value, key: &str) -> Result<Option<u32>, EngineError> {
    match super::opt_num(p, key)? {
        None => Ok(None),
        Some(v) if v.is_finite() && (0.0..=99.0).contains(&v) => Ok(Some(v.round() as u32)),
        Some(_) => Err(EngineError::bad_params(format!("{key} must be between 0 and 99"))),
    }
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
    let replace = super::opt_str(p, "replace")?.map(str::to_string);
    // xfig: an empty search string matches every text (`w_srchrepl.c`);
    // replacing an empty string is refused.
    if find.is_empty() && replace.is_some() {
        return Err(EngineError::bad_params("\"find\" must not be empty when replacing"));
    }
    let case = opt_bool(p, "case_sensitive")?.unwrap_or(true);
    let matches = |s: &str| {
        if find.is_empty() {
            true
        } else if case {
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
            // Replace in a copy, kept only where something changed, so
            // untouched objects stay shared with the undo history.
            let mut copy = (**o).clone();
            let n = replace_text(&mut copy, &find_c, &rep, case);
            if n > 0 {
                *o = Arc::new(copy);
                count += n;
            }
        }
    }
    // xfig lists each match as "[x=.. y=..] text" (the text's anchor).
    let mut found = Vec::new();
    for o in e.doc().objects.iter() {
        if hits.contains(&o.id) {
            collect_texts(o, &matches, &mut found);
        }
    }
    e.set_selection(hits.clone());
    Ok(json!({"matches": hits.len(), "replaced": count, "ids": ids_value(&hits), "texts": found}))
}

/// The matching texts (also inside compounds) with their anchor positions.
fn collect_texts(o: &Object, m: &dyn Fn(&str) -> bool, out: &mut Vec<Value>) {
    match &o.kind {
        ObjectKind::Text(t) if m(&t.text) => {
            out.push(json!({"x": t.origin.x, "y": t.origin.y, "text": t.text}));
        }
        ObjectKind::Compound(c) => c.objects.iter().for_each(|x| collect_texts(x, m, out)),
        _ => {}
    }
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

#[cfg(test)]
mod array_tests {
    use super::array_offsets;

    #[test]
    fn array_offsets_follow_xfig() {
        assert_eq!(array_offsets(0, 0, 5.0, 7.0), vec![(5.0, 7.0)]);
        assert_eq!(array_offsets(1, 0, 5.0, 7.0), vec![(5.0, 0.0)]);
        assert_eq!(array_offsets(0, 1, 5.0, 7.0), vec![(0.0, 7.0)]);
        assert_eq!(array_offsets(1, 1, 5.0, 7.0), vec![(5.0, 0.0), (0.0, 7.0)]);
        assert_eq!(array_offsets(2, 0, 5.0, 7.0), vec![(5.0, 0.0)]);
        assert_eq!(array_offsets(3, 2, 1.0, 1.0).len(), 5);
    }
}
