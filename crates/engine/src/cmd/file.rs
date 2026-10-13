//! File commands: new, open, merge, save, export.

use super::{always, has_objects, num_or, opt_num, opt_str, string, CmdResult, Command};
use crate::{import, Engine, EngineError};
use rustfig_format::FileFormat;
use rustfig_model::{Document, ObjectKind, Rgb};
use rustfig_render::{ExportFormat, ExportOptions};
use serde_json::{json, Value};
use std::path::Path;

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
            params: "{\"path\": string, \"format\"?: \"svg\"|\"eps\"|\"pdf\"|\"png\", \"magnification\"?: percent, \"margin\"?: fig units, \"dpi\"?: number, \"background\"?: \"#rrggbb\"}",
            mutates: false,
            enabled: has_objects,
            run: export_file,
        },
        Command {
            id: "file.export_data",
            label: "Export to data",
            menu: "",
            shortcut: "",
            params: "{\"format\": \"svg\"|\"eps\"|\"pdf\"|\"png\", ...export options}  — returns {\"data\": text} or {\"base64\": ...}",
            mutates: false,
            enabled: has_objects,
            run: export_data,
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

fn open_data(e: &mut Engine, p: &Value) -> CmdResult {
    let data = string(p, "data")?;
    let doc = parse(data.as_bytes())?;
    let count = doc.objects.len();
    e.replace_document(doc, opt_str(p, "name")?.map(str::to_string));
    let r = e.doc().bbox();
    e.view.fit(&r);
    Ok(json!({"objects": count}))
}

fn merge_doc(e: &mut Engine, mut other: Document) -> CmdResult {
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
    remap(&mut other.objects, &map);
    let mut ids = Vec::new();
    for o in other.objects {
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

fn export_options(p: &Value) -> Result<ExportOptions, EngineError> {
    let background = match opt_str(p, "background")? {
        Some(s) => Some(Rgb::from_hex(s).ok_or_else(|| EngineError::bad_params("background must be #rrggbb"))?),
        None => None,
    };
    Ok(ExportOptions {
        magnification: opt_num(p, "magnification")?,
        margin: num_or(p, "margin", 0.0)?,
        dpi: num_or(p, "dpi", 150.0)?,
        background,
    })
}

fn render(e: &Engine, format: ExportFormat, p: &Value) -> Result<Vec<u8>, EngineError> {
    let opts = export_options(p)?;
    rustfig_render::export(e.doc(), format, &opts).map_err(EngineError::format)
}

fn export_file(e: &mut Engine, p: &Value) -> CmdResult {
    let path = string(p, "path")?;
    let format = match opt_str(p, "format")? {
        Some(f) => ExportFormat::from_name(f),
        None => ExportFormat::from_path(&path),
    }
    .ok_or_else(|| EngineError::bad_params("unknown export format (use svg, eps, pdf or png)"))?;
    let bytes = render(e, format, p)?;
    write_atomic(&path, &bytes)?;
    e.set_status(format!("Exported {path}"));
    Ok(json!({"path": path, "bytes": bytes.len()}))
}

fn export_data(e: &mut Engine, p: &Value) -> CmdResult {
    let format = ExportFormat::from_name(&string(p, "format")?)
        .ok_or_else(|| EngineError::bad_params("unknown export format (use svg, eps, pdf or png)"))?;
    let bytes = render(e, format, p)?;
    Ok(match format {
        ExportFormat::Svg | ExportFormat::Eps => json!({"data": String::from_utf8_lossy(&bytes)}),
        ExportFormat::Pdf | ExportFormat::Png => json!({"base64": base64(&bytes)}),
    })
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
