//! File commands: new, open, merge, save, export.

use super::{always, has_objects, num_or, opt_bool, opt_num, opt_str, string, CmdResult, Command};
use crate::{import, Engine, EngineError};
use rustfig_format::FileFormat;
use rustfig_model::{Document, ObjectKind, Rgb};
use rustfig_render::languages::{language, language_for_path, Language, Status, LANGUAGES};
use rustfig_render::options::{options_for, parse_options, Arg, DriverOptions, OptionValue, Support};
use rustfig_render::{ExportFormat, ExportOptions};
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
            params: "{\"path\": string}  — open a .fig or .rustfig file",
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
            params: "{\"path\": string}  — add the objects of another file",
            mutates: true,
            enabled: always,
            run: merge,
        },
        Command {
            id: "file.merge_data",
            label: "Merge from data",
            menu: "",
            shortcut: "",
            params: "{\"data\": string}",
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

fn parse(bytes: &[u8]) -> Result<Document, EngineError> {
    rustfig_format::read_any(bytes).map_err(|err| EngineError::format(err.to_string()))
}

fn open(e: &mut Engine, p: &Value) -> CmdResult {
    let path = string(p, "path")?;
    let mut doc = parse(&read_file(&path)?)?;
    let base = Path::new(&path).parent().map(Path::to_path_buf);
    let warnings = import::load_all_pictures(&mut doc, base.as_deref());
    let count = doc.objects.len();
    e.replace_document(doc, Some(path.clone()));
    let r = e.doc().bbox();
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
    let mut doc = parse(&read_file(&file.to_string_lossy())?)?;
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
    let r = e.doc().bbox();
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
    let doc = parse(data.as_bytes())?;
    let count = doc.objects.len();
    e.replace_document(doc, opt_str(p, "name")?.map(str::to_string));
    let r = e.doc().bbox();
    e.view.fit(&r);
    Ok(json!({"objects": count}))
}

fn merge_doc(e: &mut Engine, other: Document) -> CmdResult {
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
    let mut ids = Vec::new();
    for o in objects {
        ids.push(e.add_object(o));
    }
    e.set_selection(ids.clone());
    Ok(json!({"ids": super::ids_value(&ids)}))
}

fn merge(e: &mut Engine, p: &Value) -> CmdResult {
    let path = string(p, "path")?;
    let mut doc = parse(&read_file(&path)?)?;
    let base = Path::new(&path).parent().map(Path::to_path_buf);
    import::load_all_pictures(&mut doc, base.as_deref());
    merge_doc(e, doc)
}

fn merge_data(e: &mut Engine, p: &Value) -> CmdResult {
    let data = string(p, "data")?;
    merge_doc(e, parse(data.as_bytes())?)
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

/// fig2dev's options from the `options` object (see `file.export_formats`).
fn driver_options(lang: &Language, p: &Value) -> Result<DriverOptions, EngineError> {
    let items = match p.get("options") {
        None | Some(Value::Null) => return Ok(DriverOptions::default()),
        Some(Value::Object(m)) => m,
        Some(_) => return Err(EngineError::bad_params("options must be an object")),
    };
    let mut parsed = Vec::with_capacity(items.len());
    for (k, v) in items {
        parsed.push((
            k.as_str(),
            match v {
                Value::Bool(b) => OptionValue::Bool(*b),
                Value::Number(n) => OptionValue::Number(n.as_f64().unwrap_or(f64::NAN)),
                Value::String(s) => OptionValue::Text(s),
                _ => {
                    return Err(EngineError::bad_params(format!(
                        "option {k} must be a boolean, number or string"
                    )))
                }
            },
        ));
    }
    parse_options(lang, &parsed).map_err(EngineError::bad_params)
}

fn export_options(e: &Engine, lang: &Language, p: &Value) -> Result<ExportOptions, EngineError> {
    let background = match opt_str(p, "background")? {
        Some(s) => Some(Rgb::from_hex(s).ok_or_else(|| EngineError::bad_params("background must be #rrggbb"))?),
        None => None,
    };
    Ok(ExportOptions {
        magnification: opt_num(p, "magnification")?,
        margin: num_or(p, "margin", 0.0)?,
        dpi: num_or(p, "dpi", 150.0)?,
        background,
        hidden_depths: if opt_bool(p, "active_layers_only")?.unwrap_or(false) {
            e.hidden_depths.clone()
        } else {
            Default::default()
        },
        whole_figure_bounds: opt_bool(p, "whole_figure_bounds")?.unwrap_or(false),
        fig2dev: driver_options(lang, p)?,
    })
}

fn render(e: &Engine, lang: &Language, format: ExportFormat, p: &Value) -> Result<Vec<u8>, EngineError> {
    let opts = export_options(e, lang, p)?;
    rustfig_render::export(e.doc(), format, &opts).map_err(EngineError::format)
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
    if !lang.available() {
        return Err(EngineError::bad_params(format!(
            "{} export is not available yet: it is being ported from fig2dev (plan task {})",
            lang.description,
            lang.task().unwrap_or("B0")
        )));
    }
    Ok(lang)
}

/// The native format writing a language (pspdf is two of them).
fn formats_of(lang: &Language) -> Result<Vec<ExportFormat>, EngineError> {
    if lang.id == "pspdf" {
        return Ok(vec![ExportFormat::Eps, ExportFormat::Pdf]);
    }
    ExportFormat::from_name(lang.id)
        .map(|f| vec![f])
        .ok_or_else(|| EngineError::internal(format!("no writer for {}", lang.id)))
}

fn is_text(f: ExportFormat) -> bool {
    matches!(f, ExportFormat::Svg | ExportFormat::Eps | ExportFormat::LatexBox)
}

fn export_file(e: &mut Engine, p: &Value) -> CmdResult {
    let path = string(p, "path")?;
    let lang = export_language(p, Some(&path))?;
    let formats = formats_of(lang)?;
    let mut written = Vec::new();
    let mut total = 0;
    for f in &formats {
        let target = if formats.len() > 1 {
            with_extension(&path, f.extension())
        } else {
            path.clone()
        };
        let bytes = render(e, lang, *f, p)?;
        write_atomic(&target, &bytes)?;
        total += bytes.len();
        written.push(target);
    }
    e.set_status(format!("Exported {}", written.join(" and ")));
    Ok(json!({"path": path, "paths": written, "bytes": total, "format": lang.id}))
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
    let formats = formats_of(lang)?;
    let mut files = Vec::new();
    for f in &formats {
        let bytes = render(e, lang, *f, p)?;
        files.push(if is_text(*f) {
            json!({"extension": f.extension(), "data": String::from_utf8_lossy(&bytes)})
        } else {
            json!({"extension": f.extension(), "base64": base64(&bytes)})
        });
    }
    match files.as_slice() {
        [one] => {
            let mut one = one.clone();
            if let Some(m) = one.as_object_mut() {
                m.remove("extension");
            }
            Ok(one)
        }
        _ => Ok(json!({ "files": files })),
    }
}

fn export_formats(_: &mut Engine, _: &Value) -> CmdResult {
    let formats: Vec<Value> = LANGUAGES
        .iter()
        .map(|l| {
            let (status, task) = match l.status {
                Status::Available => ("available", None),
                Status::Partial(t) => ("partial", Some(t)),
                Status::Planned(t) => ("planned", Some(t)),
            };
            json!({
                "id": l.id,
                "description": l.description,
                "extension": l.extension,
                "aliases": l.aliases,
                "kind": format!("{:?}", l.kind).to_lowercase(),
                "available": l.available(),
                "status": status,
                "task": task,
                "options": options_for(l.id).map(|o| json!({
                    "key": o.key,
                    "letter": o.letter.to_string(),
                    "arg": match o.arg {
                        Arg::Flag => "bool",
                        Arg::Number => "number",
                        Arg::Text => "text",
                        Arg::Color => "color",
                        Arg::Depths => "depths",
                        Arg::Size => "size",
                        Arg::Font => "font",
                    },
                    "help": o.help,
                    "available": o.support == Support::Ported,
                    "task": match o.support {
                        Support::Ported => None,
                        Support::Planned(t) => Some(t),
                    },
                })).collect::<Vec<_>>(),
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
