//! File commands: new, open, merge, save, export.
//
// Ported from xfig src/u_print.c (the fig2dev options an export passes)
// (xfig licence, see LICENSE-xfig).

use super::{always, has_objects, num_or, opt_bool, opt_num, opt_str, string, CmdResult, Command};
use crate::fig2dev::languages::{language, language_for_path, Language, Status, LANGUAGES};
use crate::fig2dev::options::{option_spec, options_for, Arg, Support};
use crate::{import, Engine, EngineError};
use fig2dev::drivers::scene;
use fig2dev::scene::ExportFormat;
use rustfig_format::FileFormat;
use rustfig_model::{Document, ObjectKind, Rgb};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "file.new",
            label: "New",
            menu: "",
            shortcut: "",
            params: "{}",
            mutates: true,
            enabled: always,
            run: new,
        },
        Command {
            id: "file.open",
            label: "Open…",
            menu: "",
            shortcut: "",
            params: "{\"path\": string, \"offset_x\"?: number, \"offset_y\"?: number}  — open a .fig or .rustfig file; the optional offset (Fig units) moves the figure, as xfig's Figure Offset",
            mutates: false,
            enabled: always,
            run: open,
        },
        Command {
            id: "file.open_data",
            label: "Open from data",
            menu: "",
            shortcut: "",
            params: "{\"data\": string, \"name\"?: string}  — open a document from its text",
            mutates: false,
            enabled: always,
            run: open_data,
        },
        Command {
            id: "file.recover",
            label: "Recover unsaved drawing",
            menu: "",
            shortcut: "",
            params: "{\"id\": string}  — open a drawing kept for crash recovery (see file.recoverable); it opens unsaved, under the name of the file it came from",
            mutates: false,
            enabled: always,
            run: recover,
        },
        Command {
            id: "file.recoverable",
            label: "Recoverable drawings",
            menu: "",
            shortcut: "",
            params: "{}  — returns {\"drawings\": [{\"id\", \"original\"}]}: drawings left by RustFig instances that stopped without saving",
            mutates: false,
            enabled: always,
            run: recoverable,
        },
        Command {
            id: "file.discard_recovery",
            label: "Discard recovered drawing",
            menu: "",
            shortcut: "",
            params: "{\"id\": string}  — delete a drawing kept for crash recovery",
            mutates: false,
            enabled: always,
            run: discard_recovery,
        },
        Command {
            id: "file.merge",
            label: "Merge…",
            menu: "",
            shortcut: "",
            params: "{\"path\": string, \"offset_x\"?: number, \"offset_y\"?: number}  — add the objects of another file as one compound (a file that is one compound is added as it is), moved by the optional offset in Fig units, as xfig's Merge",
            mutates: true,
            enabled: always,
            run: merge,
        },
        Command {
            id: "file.merge_data",
            label: "Merge from data",
            menu: "",
            shortcut: "",
            params: "{\"data\": string, \"offset_x\"?: number, \"offset_y\"?: number}",
            mutates: true,
            enabled: always,
            run: merge_data,
        },
        Command {
            id: "file.save",
            label: "Save",
            menu: "",
            shortcut: "",
            params: "{\"path\"?: string}  — save (to the current path if omitted); format from the extension",
            mutates: false,
            enabled: always,
            run: save,
        },
        Command {
            id: "file.save_data",
            label: "Save to data",
            menu: "",
            shortcut: "",
            params: "{\"format\"?: \"fig\"|\"rustfig\"}  — returns {\"data\": string}",
            mutates: false,
            enabled: always,
            run: save_data,
        },
        Command {
            id: "file.export",
            label: "Export…",
            menu: "",
            shortcut: "",
            params: "{\"path\": string, \"format\"?: language id (see file.export_formats; default from the file name), \"magnification\"?: percent, \"margin\"?: fig units, \"dpi\"?: number, \"background\"?: \"#rrggbb\", \"active_layers_only\"?: bool, \"whole_figure_bounds\"?: bool, \"options\"?: {key: value} fig2dev's options for the language, by key or letter (see file.export_formats)}  — active_layers_only leaves out depths that are off; whole_figure_bounds then still sizes the page by the whole figure; pspdf writes NAME.eps and NAME.pdf",
            mutates: false,
            enabled: has_objects,
            run: export_file,
        },
        Command {
            id: "file.export_data",
            label: "Export to data",
            menu: "",
            shortcut: "",
            params: "{\"format\": language id, ...export options}  — returns {\"data\": text} or {\"base64\": ...}; pspdf returns {\"files\": [{\"extension\", \"data\"|\"base64\"}]}",
            mutates: false,
            enabled: has_objects,
            run: export_data,
        },
        Command {
            id: "file.export_formats",
            label: "Export formats",
            menu: "",
            shortcut: "",
            params: "{}  — returns {\"formats\": [{\"id\", \"description\", \"extension\", \"aliases\", \"kind\", \"available\", \"status\", \"task\", \"options\": [{\"key\", \"letter\", \"arg\", \"help\", \"available\", \"task\"}]}]}: xfig's export languages in its order; unavailable ones are still being ported from fig2dev",
            mutates: false,
            enabled: always,
            run: export_formats,
        },
    ]
}

fn new(e: &mut Engine, _: &Value) -> CmdResult {
    let settings = e.doc().settings.clone();
    *e.doc_mut() = Document {
        settings,
        ..Document::new()
    };
    e.set_selection(Vec::new());
    e.set_path(None);
    Ok(json!({}))
}

fn read_file(path: &str) -> Result<Vec<u8>, EngineError> {
    let meta = std::fs::metadata(path).map_err(|err| EngineError::io(format!("{path}: {err}")))?;
    if meta.len() > rustfig_format::MAX_FILE_BYTES as u64 {
        return Err(EngineError::io(format!("{path}: file is too large")));
    }
    std::fs::read(path).map_err(|err| EngineError::io(format!("{path}: {err}")))
}

/// Read a document (Fig 1.3 to 3.2 through fig2dev's reader). The document
/// and the reader's messages.
fn parse(bytes: &[u8]) -> Result<(Document, Vec<String>), EngineError> {
    rustfig_format::read_any_messages(bytes).map_err(|err| EngineError::format(err.to_string()))
}

fn open(e: &mut Engine, p: &Value) -> CmdResult {
    let path = string(p, "path")?;
    let (mut doc, mut warnings) = parse(&read_file(&path)?)?;
    let (dx, dy) = offset(p)?;
    if dx != 0.0 || dy != 0.0 {
        for o in &mut doc.objects {
            Arc::make_mut(o).translate(dx, dy);
        }
    }
    let base = Path::new(&path).parent().map(Path::to_path_buf);
    warnings.extend(import::load_all_pictures(&mut doc, base.as_deref()));
    let count = doc.objects.len();
    e.replace_document(doc, Some(path.clone()));
    e.prefs.add_recent(&path);
    e.prefs_dirty = true;
    let r = e.doc_bounds();
    e.view.fit(&r);
    e.set_status(format!("Opened {path}"));
    Ok(json!({"objects": count, "warnings": warnings}))
}

/// The recovery folder and a validated recovery id (letters, digits and
/// dashes only, so it can't name a file outside the folder).
#[cfg(not(target_arch = "wasm32"))]
fn recovery_id(e: &Engine, p: &Value) -> Result<(std::path::PathBuf, String), EngineError> {
    let id = string(p, "id")?;
    if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(EngineError::bad_params("not a recovery id"));
    }
    let dir = e
        .recovery_dir
        .clone()
        .or_else(crate::recovery::default_dir)
        .ok_or_else(|| EngineError::io("no folder for recovery files"))?;
    Ok((dir, id))
}

#[cfg(not(target_arch = "wasm32"))]
fn recover(e: &mut Engine, p: &Value) -> CmdResult {
    let (dir, id) = recovery_id(e, p)?;
    let file = dir.join(format!("{id}.rustfig"));
    let (mut doc, _) = parse(&read_file(&file.to_string_lossy())?)?;
    let original = std::fs::read_to_string(dir.join(format!("{id}.path")))
        .ok()
        .filter(|s| !s.is_empty());
    let base = original
        .as_deref()
        .and_then(|o| Path::new(o).parent())
        .map(Path::to_path_buf);
    let warnings = import::load_all_pictures(&mut doc, base.as_deref());
    e.replace_document(doc, original.clone());
    e.set_modified(true);
    let r = e.doc_bounds();
    e.view.fit(&r);
    e.set_status("Recovered an unsaved drawing; save it to keep it");
    Ok(json!({"original": original, "warnings": warnings}))
}

#[cfg(target_arch = "wasm32")]
fn recover(_: &mut Engine, _: &Value) -> CmdResult {
    Err(EngineError::bad_params("crash recovery is not available on the web"))
}

#[cfg(not(target_arch = "wasm32"))]
fn recoverable(e: &mut Engine, _: &Value) -> CmdResult {
    let Some(dir) = e.recovery_dir.clone().or_else(crate::recovery::default_dir) else {
        return Ok(json!({"drawings": []}));
    };
    let list: Vec<Value> = crate::recovery::orphans(&dir, None)
        .into_iter()
        .map(|o| json!({"id": o.id, "original": o.original}))
        .collect();
    Ok(json!({ "drawings": list }))
}

#[cfg(target_arch = "wasm32")]
fn recoverable(_: &mut Engine, _: &Value) -> CmdResult {
    Ok(json!({"drawings": []}))
}

#[cfg(not(target_arch = "wasm32"))]
fn discard_recovery(e: &mut Engine, p: &Value) -> CmdResult {
    let (dir, id) = recovery_id(e, p)?;
    crate::recovery::discard(&dir, &id);
    Ok(json!({}))
}

#[cfg(target_arch = "wasm32")]
fn discard_recovery(_: &mut Engine, _: &Value) -> CmdResult {
    Ok(json!({}))
}

fn open_data(e: &mut Engine, p: &Value) -> CmdResult {
    let data = string(p, "data")?;
    let (doc, warnings) = parse(data.as_bytes())?;
    let count = doc.objects.len();
    e.replace_document(doc, opt_str(p, "name")?.map(str::to_string));
    let r = e.doc_bounds();
    e.view.fit(&r);
    Ok(json!({"objects": count, "warnings": warnings}))
}

/// xfig's Figure Offset of Open and Merge, in Fig units.
fn offset(p: &Value) -> Result<(f64, f64), EngineError> {
    Ok((num_or(p, "offset_x", 0.0)?, num_or(p, "offset_y", 0.0)?))
}

fn merge_doc(e: &mut Engine, other: Document, (dx, dy): (f64, f64)) -> CmdResult {
    // Map user colours of the merged document into this one.
    let mut map = std::collections::BTreeMap::new();
    for c in &other.colors {
        let n = e
            .doc_mut()
            .color_for_rgb(c.rgb)
            .map(|r| r.0)
            .unwrap_or(rustfig_model::ColorRef::BLACK.0);
        map.insert(c.number, n);
    }
    fn remap(objs: &mut [rustfig_model::Object], map: &std::collections::BTreeMap<i32, i32>) {
        for o in objs {
            let m = |c: &mut rustfig_model::ColorRef| {
                if let Some(n) = map.get(&c.0) {
                    c.0 = *n;
                }
            };
            match &mut o.kind {
                ObjectKind::Text(t) => m(&mut t.color),
                ObjectKind::Compound(c) => remap(&mut c.objects, map),
                _ => {
                    if let Some(s) = o.style_mut() {
                        m(&mut s.pen_color);
                        m(&mut s.fill_color);
                    }
                }
            }
        }
    }
    let mut objects: Vec<rustfig_model::Object> = other.objects.into_iter().map(Arc::unwrap_or_clone).collect();
    remap(&mut objects, &map);
    for o in &mut objects {
        o.translate(dx, dy);
    }
    // xfig's merge_file adds the file as one compound, unless it is one
    // compound already.
    let single_compound = matches!(objects.as_slice(), [o] if matches!(o.kind, ObjectKind::Compound(_)));
    if !objects.is_empty() && !single_compound {
        objects = vec![rustfig_model::Object::new(ObjectKind::Compound(
            rustfig_model::Compound { objects },
        ))];
    }
    let mut ids = Vec::new();
    for o in objects {
        ids.push(e.add_object(o));
    }
    e.set_selection(ids.clone());
    Ok(json!({"ids": super::ids_value(&ids)}))
}

fn merge(e: &mut Engine, p: &Value) -> CmdResult {
    let path = string(p, "path")?;
    let (mut doc, _) = parse(&read_file(&path)?)?;
    let base = Path::new(&path).parent().map(Path::to_path_buf);
    import::load_all_pictures(&mut doc, base.as_deref());
    merge_doc(e, doc, offset(p)?)
}

fn merge_data(e: &mut Engine, p: &Value) -> CmdResult {
    let data = string(p, "data")?;
    merge_doc(e, parse(data.as_bytes())?.0, offset(p)?)
}

fn save(e: &mut Engine, p: &Value) -> CmdResult {
    let path = match opt_str(p, "path")? {
        Some(s) => s.to_string(),
        None => e
            .path()
            .map(str::to_string)
            .ok_or_else(|| EngineError::bad_params("no file name: give a \"path\""))?,
    };
    let format = FileFormat::from_path(&path).unwrap_or(FileFormat::Fig);
    let text = rustfig_format::write(e.doc(), format).map_err(|err| EngineError::format(err.to_string()))?;
    write_atomic(&path, text.as_bytes())?;
    e.set_path(Some(path.clone()));
    e.set_modified(false);
    e.prefs.add_recent(&path);
    e.prefs_dirty = true;
    e.set_status(format!("Saved {path}"));
    Ok(json!({"path": path}))
}

/// Write via a temporary file and rename, so a failed save never truncates the old file.
pub(crate) fn write_atomic(path: &str, data: &[u8]) -> Result<(), EngineError> {
    let tmp = format!("{path}.rustfig-tmp");
    std::fs::write(&tmp, data).map_err(|err| EngineError::io(format!("{path}: {err}")))?;
    std::fs::rename(&tmp, path).map_err(|err| {
        let _ = std::fs::remove_file(&tmp); // best effort cleanup; the rename error is what matters
        EngineError::io(format!("{path}: {err}"))
    })
}

fn save_data(e: &mut Engine, p: &Value) -> CmdResult {
    let format = match opt_str(p, "format")?.unwrap_or("fig") {
        "fig" | "xfig" => FileFormat::Fig,
        "rustfig" | "native" => FileFormat::Native,
        other => return Err(EngineError::bad_params(format!("unknown format {other:?}"))),
    };
    let text = rustfig_format::write(e.doc(), format).map_err(|err| EngineError::format(err.to_string()))?;
    Ok(json!({"data": text}))
}

/// The `options` object of an export (empty when absent).
fn option_items(p: &Value) -> Result<Vec<(&str, &Value)>, EngineError> {
    match p.get("options") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Object(m)) => Ok(m.iter().map(|(k, v)| (k.as_str(), v)).collect()),
        Some(_) => Err(EngineError::bad_params("options must be an object")),
    }
}

/// The directory of the document, where relative picture names point.
fn doc_dir(e: &Engine) -> Option<std::path::PathBuf> {
    e.path().and_then(|p| Path::new(p).parent()).map(Path::to_path_buf)
}

/// Which share of an export one fig2dev run writes.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Half {
    /// The whole figure.
    Whole,
    /// The graphics of a combined LaTeX export (fig2dev's `pstex` or
    /// `pdftex`: no special texts); only the options it knows reach it.
    Graphics,
    /// The LaTeX part (`pstex_t`, `pdftex_t`) next to graphics in the
    /// given language, whose own options stay with the graphics.
    Latex(&'static str),
}

/// One file an export writes: fig2dev's driver for `lang`; `graphics` is
/// `pstex_t`'s `-p`.
#[derive(Debug, Clone, PartialEq)]
struct Part {
    lang: &'static str,
    half: Half,
    graphics: Option<String>,
}

/// The file name of `path` (after the last separator).
fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// The files exporting `lang` to `path` writes, as xfig's export
/// (`u_print.c`) names them: pspdf writes NAME.eps and NAME.pdf; pstex
/// writes the graphics to PATH and the LaTeX part to PATH_t (pdftex
/// likewise); pspdftex writes NAME_SUFFIX.eps and NAME_SUFFIX.pdf (the
/// first dot of the file name made a `_`) and the LaTeX part to PATH.
fn parts(lang: &Language, path: &str) -> Vec<(String, Part)> {
    let part = |lang, half, graphics| Part { lang, half, graphics };
    match lang.id {
        "pspdf" => vec![
            (with_extension(path, "eps"), part("eps", Half::Whole, None)),
            (with_extension(path, "pdf"), part("pdf", Half::Whole, None)),
        ],
        "pstex" | "pdftex" => {
            let (graphics, graphics_lang, latex) = if lang.id == "pstex" {
                ("pstex", "eps", "pstex_t")
            } else {
                ("pdftex", "pdf", "pdftex_t")
            };
            vec![
                (path.to_string(), part(graphics, Half::Graphics, None)),
                (
                    format!("{path}_t"),
                    part(latex, Half::Latex(graphics_lang), Some(file_name(path).to_string())),
                ),
            ]
        }
        "pspdftex" => {
            let name = file_name(path);
            let stem = name.replacen('.', "_", 1);
            let dir = path.get(..path.len() - name.len()).unwrap_or("");
            vec![
                (format!("{dir}{stem}.eps"), part("pstex", Half::Graphics, None)),
                (format!("{dir}{stem}.pdf"), part("pdftex", Half::Graphics, None)),
                (path.to_string(), part("pstex_t", Half::Latex("eps"), Some(stem))),
            ]
        }
        id => vec![(path.to_string(), part(id, Half::Whole, None))],
    }
}

/// Whether fig2dev's driver for `drv` takes option `letter` (`key` in the
/// option table).
fn takes(drv: &str, key: &str, letter: char) -> bool {
    let olang = scene::options_lang(drv);
    option_spec(olang, key).is_some_and(|s| s.letter == letter && s.effective_for(olang))
        || fig2dev::drivers::help::help(drv).is_some_and(|h| h.options.iter().any(|o| o.letter == letter))
}

/// A number as an option argument (`2`, not `2.0`, for whole numbers).
fn number_arg(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// fig2dev's option letters and arguments for driver `drv` from the
/// export's parameters, as xfig makes them up (`u_print.c`): `options` by
/// key or letter; RustFig's settings mapped onto fig2dev's options: the
/// magnification (percent) as `-m`, the bitmap resolution as `-m` too
/// (fig2dev's bitmaps are 80 dpi at 100 %), the margin (Fig units) as
/// `-b` points, the background as `-g`, the active layers as `-D` (and
/// `-K` unless the page takes the whole figure); `-F` (true font sizes)
/// as xfig always passes it to the PostScript, PDF and bitmap drivers.
fn driver_args(e: &Engine, drv: &str, half: Half, p: &Value) -> Result<Vec<(char, Option<String>)>, EngineError> {
    let mut args: Vec<(char, Option<String>)> = Vec::new();
    // Letters given, also those switched off (`false`).
    let mut given = Vec::new();
    let olang = scene::options_lang(drv);
    let help = fig2dev::drivers::help::help(drv);
    for (k, v) in option_items(p)? {
        let letter = match half {
            Half::Latex(g) => {
                let general = option_spec(drv, k).filter(|s| s.langs.is_empty());
                // In a combined export, the graphics' options stay with
                // them, except the border, which both halves take (xfig
                // passes it on).
                let graphics_spec = option_spec(g, k).filter(|s| !s.langs.is_empty());
                match (general, graphics_spec) {
                    (Some(s), _) => Some(s.letter),
                    (None, Some(s)) if s.letter == 'b' => Some('b'),
                    (None, Some(_)) => continue,
                    (None, None) => None,
                }
            }
            Half::Graphics => match option_spec(olang, k) {
                Some(s) => Some(s.letter),
                // An option of the LaTeX half.
                None => continue,
            },
            Half::Whole => option_spec(olang, k).map(|s| s.letter),
        };
        let letter = match letter {
            Some(l) => l,
            None => {
                let key = k.strip_prefix('-').unwrap_or(k);
                let mut cs = key.chars();
                match (cs.next(), cs.next()) {
                    (Some(c), None) if c.is_ascii_alphanumeric() => c,
                    _ => {
                        return Err(EngineError::bad_params(format!(
                            "{drv} export has no option {k:?}: give fig2dev's option letter (see file.export_formats)"
                        )));
                    }
                }
            }
        };
        // RustFig's own drivers check their options with fig2dev's
        // messages; the ported ones are checked against their help.
        let general = option_spec(drv, k).is_some_and(|s| s.langs.is_empty());
        if !general && !scene::writes(drv) {
            if let Some(h) = help {
                if !h.options.iter().any(|o| o.letter == letter) {
                    let known: Vec<String> = h.options.iter().map(|o| o.letter.to_string()).collect();
                    return Err(EngineError::bad_params(format!(
                        "{drv} export has no option -{letter}; its options are: {}",
                        known.join(", ")
                    )));
                }
            }
        }
        given.push(letter);
        let arg = match v {
            Value::Bool(true) => None,
            Value::Bool(false) | Value::Null => continue,
            Value::Number(n) => Some(number_arg(n.as_f64().unwrap_or(f64::NAN))),
            Value::String(s) => Some(s.clone()),
            _ => {
                return Err(EngineError::bad_params(format!(
                    "option {k} must be a boolean, number or string"
                )))
            }
        };
        if arg.as_deref().is_some_and(|a| a.len() > 4096 || a.contains('\0')) {
            return Err(EngineError::bad_params(format!("option {k}: argument too long")));
        }
        args.push((letter, arg));
    }
    let has = |given: &[char], c: char| given.contains(&c);
    // -m: the magnification, times the resolution for bitmaps.
    let mut mag = None;
    if let Some(m) = opt_num(p, "magnification")? {
        if !(m.is_finite() && m > 0.0 && m <= 100_000.0) {
            return Err(EngineError::bad_params("magnification must be a positive percentage"));
        }
        mag = Some(m / 100.0);
    }
    let bitmap = scene::writes(drv) && ExportFormat::from_name(olang).is_some_and(ExportFormat::is_bitmap);
    if let Some(dpi) = opt_num(p, "dpi")? {
        if !(dpi.is_finite() && dpi > 0.0 && dpi <= 100_000.0) {
            return Err(EngineError::bad_params("dpi must be a positive number"));
        }
        if bitmap && !has(&given, 'Z') {
            let m = match args.iter().rposition(|(l, _)| *l == 'm') {
                // An explicit -m: scale it.
                Some(i) => {
                    let (_, a) = args.remove(i);
                    fig2dev::gendev::atof(a.as_deref().unwrap_or(""))
                }
                None => mag.unwrap_or(e.doc().settings.magnification / 100.0),
            };
            mag = Some(m * dpi / 80.0);
            given.retain(|c| *c != 'm');
        }
    }
    if let Some(m) = mag {
        if !has(&given, 'm') && !has(&given, 'Z') {
            args.push(('m', Some(format!("{m}"))));
        }
    }
    // -b: the margin, in points (xfig's export margin is in points).
    let margin = num_or(p, "margin", 0.0)?;
    if !(margin.is_finite() && (0.0..=1.0e7).contains(&margin)) {
        return Err(EngineError::bad_params(
            "margin must be a number of fig units, 0 or more",
        ));
    }
    if margin > 0.0 && takes(drv, "border", 'b') {
        let points = margin * 72.0 / 1200.0;
        match args.iter_mut().find(|(l, _)| *l == 'b') {
            Some((_, a)) => {
                let b = fig2dev::gendev::atof(a.as_deref().unwrap_or("")) + points;
                *a = Some(number_arg(b));
            }
            None => args.push(('b', Some(number_arg(points)))),
        }
    }
    if let Some(bg) = opt_str(p, "background")? {
        if Rgb::from_hex(bg).is_none() {
            return Err(EngineError::bad_params("background must be #rrggbb"));
        }
        if !has(&given, 'g') && takes(drv, "background", 'g') {
            args.push(('g', Some(bg.to_string())));
        }
    }
    if opt_bool(p, "active_layers_only")?.unwrap_or(false) && !e.hidden_depths.is_empty() && !has(&given, 'D') {
        let list: Vec<String> = e.hidden_depths.iter().map(i32::to_string).collect();
        args.push(('D', Some(format!("-{}", list.join(",")))));
        if !opt_bool(p, "whole_figure_bounds")?.unwrap_or(false) && !has(&given, 'K') {
            args.push(('K', None));
        }
    }
    if scene::writes(drv) && !has(&given, 'F') && takes(drv, "correct_font_size", 'F') {
        args.push(('F', None));
    }
    Ok(args)
}

/// Write one part of an export through fig2dev; the bytes and fig2dev's
/// messages.
fn write_part(e: &Engine, part: &Part, p: &Value) -> Result<(Vec<u8>, Vec<String>), EngineError> {
    let mut args = driver_args(e, part.lang, part.half, p)?;
    if let Some(g) = &part.graphics {
        args.push(('p', Some(g.clone())));
    }
    crate::fig2dev::export(e.doc(), part.lang, &args, doc_dir(e).as_deref()).map_err(EngineError::format)
}

/// Whether `part` writes text (the rest comes back as base64).
fn is_text(part: &Part, bytes: &[u8]) -> bool {
    match ExportFormat::from_name(scene::options_lang(part.lang)) {
        Some(f) if scene::writes(part.lang) => f.is_text(),
        _ => std::str::from_utf8(bytes).is_ok(),
    }
}

/// The export language from `format`, or else from the file name.
fn export_language(p: &Value, path: Option<&str>) -> Result<&'static Language, EngineError> {
    let lang = match (opt_str(p, "format")?, path) {
        (Some(f), _) => language(f).ok_or_else(|| {
            EngineError::bad_params(format!("unknown export language {f:?} (see file.export_formats)"))
        })?,
        (None, Some(path)) => language_for_path(path).ok_or_else(|| {
            EngineError::bad_params(
                "cannot tell the export language from the file name; pass \"format\" (see file.export_formats)",
            )
        })?,
        (None, None) => return Err(EngineError::bad_params("missing \"format\"")),
    };
    if !crate::fig2dev::language_available(lang) {
        return Err(EngineError::bad_params(format!(
            "{} export is not available yet: it is being ported from fig2dev (plan task {})",
            lang.description,
            lang.task().unwrap_or("B0")
        )));
    }
    Ok(lang)
}

/// Whether a picture object of the drawing reads `path` (xfig's "is an
/// input file" check in `do_export`).
fn uses_picture_file(objects: &[rustfig_model::Object], dir: Option<&Path>, path: &Path) -> bool {
    objects.iter().any(|o| match &o.kind {
        ObjectKind::Polyline(l) => l.picture.as_ref().is_some_and(|pic| {
            let f = Path::new(&pic.file);
            let full = if f.is_absolute() {
                f.to_path_buf()
            } else {
                dir.map_or_else(|| f.to_path_buf(), |d| d.join(f))
            };
            full == path || pic.file == path.to_string_lossy()
        }),
        ObjectKind::Compound(c) => uses_picture_file(&c.objects, dir, path),
        _ => false,
    })
}

fn export_file(e: &mut Engine, p: &Value) -> CmdResult {
    let path = string(p, "path")?;
    let objs: Vec<rustfig_model::Object> = e.doc().objects.iter().map(|o| (**o).clone()).collect();
    if uses_picture_file(&objs, doc_dir(e).as_deref(), Path::new(&path)) {
        return Err(EngineError::bad_params(format!(
            "\"{path}\" is an input file.\nExport aborted"
        )));
    }
    let lang = export_language(p, Some(&path))?;
    let parts = parts(lang, &path);
    // Write every part before touching a file, so a failure leaves none.
    let mut outputs = Vec::with_capacity(parts.len());
    let mut warnings = Vec::new();
    for (target, part) in &parts {
        let (bytes, msgs) = write_part(e, part, p)?;
        warnings.extend(msgs);
        outputs.push((target.clone(), bytes));
    }
    let mut written = Vec::new();
    let mut total = 0;
    for (target, bytes) in outputs {
        write_atomic(&target, &bytes)?;
        total += bytes.len();
        written.push(target);
    }
    e.set_status(format!("Exported {}", written.join(" and ")));
    Ok(json!({"path": path, "paths": written, "bytes": total, "format": lang.id, "warnings": warnings}))
}

/// `path` with its extension replaced (or added).
fn with_extension(path: &str, ext: &str) -> String {
    let name_start = path.rfind(['/', '\\']).map_or(0, |i| i + 1);
    match path.get(name_start..).and_then(|name| name.rfind('.')) {
        Some(dot) if dot > 0 => format!("{}.{ext}", path.get(..name_start + dot).unwrap_or(path)),
        _ => format!("{path}.{ext}"),
    }
}

fn export_data(e: &mut Engine, p: &Value) -> CmdResult {
    let lang = export_language(p, None)?;
    let virtual_path = format!("figure.{}", lang.extension);
    let parts = parts(lang, &virtual_path);
    let mut files = Vec::new();
    let mut warnings = Vec::new();
    for (target, part) in &parts {
        let (bytes, msgs) = write_part(e, part, p)?;
        warnings.extend(msgs);
        let text = is_text(part, &bytes);
        let extension = target.rsplit_once('.').map_or("", |(_, x)| x);
        files.push(if text {
            json!({"name": target, "extension": extension, "data": String::from_utf8_lossy(&bytes)})
        } else {
            json!({"name": target, "extension": extension, "base64": base64(&bytes)})
        });
    }
    match files.as_slice() {
        [one] => {
            let mut one = one.clone();
            if let Some(m) = one.as_object_mut() {
                m.remove("extension");
                m.remove("name");
                if !warnings.is_empty() {
                    m.insert("warnings".into(), json!(warnings));
                }
            }
            Ok(one)
        }
        _ => Ok(json!({ "files": files, "warnings": warnings })),
    }
}

fn arg_name(a: Arg) -> &'static str {
    match a {
        Arg::Flag => "bool",
        Arg::Number => "number",
        Arg::Text => "text",
        Arg::Color => "color",
        Arg::Depths => "depths",
        Arg::Size => "size",
        Arg::Font => "font",
    }
}

fn export_formats(_: &mut Engine, _: &Value) -> CmdResult {
    let formats: Vec<Value> = LANGUAGES
        .iter()
        .map(|l| {
            let available = crate::fig2dev::language_available(l);
            let (status, task) = match l.status {
                _ if available && !l.available() => ("available", None),
                Status::Available => ("available", None),
                Status::Partial(t) => ("partial", Some(t)),
                Status::Planned(t) => ("planned", Some(t)),
            };
            let mut options: Vec<Value> = options_for(l.id)
                .map(|o| {
                    json!({
                        "key": o.key,
                        "letter": o.letter.to_string(),
                        "arg": arg_name(o.arg),
                        "help": o.help,
                        "available": o.support == Support::Ported
                            || (available && !l.available() && o.langs.is_empty()),
                        "task": match o.support {
                            Support::Ported => None,
                            Support::Planned(t) => Some(t),
                        },
                    })
                })
                .collect();
            // The ported driver's own options, by letter.
            if !l.available() && available {
                let drv = match l.id {
                    "pstex" | "pspdftex" => "pstex_t",
                    "pdftex" => "pdftex_t",
                    id => id,
                };
                for o in fig2dev::drivers::help::help(drv).map_or(&[][..], |h| h.options) {
                    options.push(json!({
                        "key": o.letter.to_string(),
                        "letter": o.letter.to_string(),
                        "arg": if o.arg.is_empty() { "bool" } else { "text" },
                        "help": o.text,
                        "available": true,
                        "task": Value::Null,
                    }));
                }
            }
            json!({
                "id": l.id,
                "description": l.description,
                "extension": l.extension,
                "aliases": l.aliases,
                "kind": format!("{:?}", l.kind).to_lowercase(),
                "available": available,
                "status": status,
                "task": task,
                "options": options,
            })
        })
        .collect();
    Ok(json!({ "formats": formats }))
}

pub(crate) fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let b0 = c.first().copied().unwrap_or(0);
        let b1 = c.get(1).copied().unwrap_or(0);
        let b2 = c.get(2).copied().unwrap_or(0);
        let v = (u32::from(b0) << 16) | (u32::from(b1) << 8) | u32::from(b2);
        let ch = |sh: u32| char::from(T.get(((v >> sh) & 63) as usize).copied().unwrap_or(b'A'));
        s.push(ch(18));
        s.push(ch(12));
        s.push(if c.len() > 1 { ch(6) } else { '=' });
        s.push(if c.len() > 2 { ch(0) } else { '=' });
    }
    s
}
