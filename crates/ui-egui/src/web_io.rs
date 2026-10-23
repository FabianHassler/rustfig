//! File input and output without a file system (C2): the browser.
//!
//! The web build has no paths. Open picks a file through the browser's file
//! picker (or takes one dropped on the window) and hands its bytes to
//! `file.open_data`; Save and Export make the bytes with `file.save_data` and
//! `file.export_data` and give them to the user as a download. The pickers and
//! downloads come through [`FileDialogs`](crate::dialogs::FileDialogs) (`is_web`,
//! `pick_web`, `download`), so a test can script them on the desktop too.

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::{json, Value};

use crate::App;

/// A file the user picked or dropped in the browser.
#[derive(Debug, Clone, PartialEq)]
pub struct WebPick {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Where asynchronous picks arrive: the browser's file picker answers some
/// frames after the request. Cloning shares the queue.
#[derive(Clone, Default)]
pub struct WebInbox {
    queue: Rc<RefCell<Vec<WebPick>>>,
    ctx: Rc<RefCell<Option<egui::Context>>>,
}

impl WebInbox {
    /// Remember the context, so an arriving pick wakes the UI (the web app
    /// only draws on events).
    pub fn attach(&self, ctx: &egui::Context) {
        if let Ok(mut c) = self.ctx.try_borrow_mut() {
            if c.is_none() {
                *c = Some(ctx.clone());
            }
        }
    }

    /// A pick arrived.
    pub fn push(&self, pick: WebPick) {
        if let Ok(mut q) = self.queue.try_borrow_mut() {
            q.push(pick);
        }
        if let Ok(c) = self.ctx.try_borrow() {
            if let Some(c) = c.as_ref() {
                c.request_repaint();
            }
        }
    }

    /// The picks that arrived since the last call.
    pub fn take(&self) -> Vec<WebPick> {
        self.queue
            .try_borrow_mut()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default()
    }
}

/// The part of `name` after the last `/` or `\`, which is how a browser
/// reports a file (no folders), without control characters.
pub fn base_name(name: &str) -> String {
    let n = name.rsplit(['/', '\\']).next().unwrap_or(name);
    n.chars().filter(|c| !c.is_control()).collect()
}

fn extension(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default()
}

/// Open a picked or dropped file as the figure: a Fig or native file, or a
/// PDF, SVG or PNG export with the figure embedded (which opens under a
/// `.fig` name, as on the desktop, so Save never names the export).
pub fn open_pick(app: &mut App, pick: &WebPick) {
    let mut name = base_name(&pick.name);
    if name.is_empty() {
        name = "figure.fig".to_string();
    }
    let embedded = !matches!(extension(&name).as_str(), "fig" | "xfig" | "rustfig");
    if embedded {
        name = match name.rsplit_once('.') {
            Some((stem, _)) if !stem.is_empty() => format!("{stem}.fig"),
            _ => format!("{name}.fig"),
        };
    }
    if app
        .run(
            "file.open_data",
            json!({"base64": rustfig_engine::base64(&pick.bytes), "name": name}),
        )
        .is_some()
    {
        app.engine.set_modified(embedded);
        app.info(format!("Opened {name}"));
    }
}

/// Save: the figure as a download named after the open file (`figure.fig`
/// for a new one; a `.rustfig` name keeps the native format).
pub fn save(app: &mut App) {
    let current = app.engine.path().map(base_name).filter(|n| !n.is_empty());
    let name = current.unwrap_or_else(|| "figure.fig".to_string());
    let format = if extension(&name) == "rustfig" {
        "rustfig"
    } else {
        "fig"
    };
    let Some(r) = app.run("file.save_data", json!({ "format": format })) else {
        return;
    };
    let Some(text) = r.get("data").and_then(Value::as_str) else {
        app.error("Save: no data");
        return;
    };
    match app.file_dialogs.download(&name, text.as_bytes().to_vec()) {
        Ok(()) => {
            app.engine.set_path(Some(name.clone()));
            app.engine.set_modified(false);
            app.info(format!("Saved {name} (downloaded)"));
        }
        Err(e) => app.error(e),
    }
}

/// Export: `file.export_data` with the panel's parameters, each file it
/// produces as a download (`default_name` for the one file, the figure's name
/// with each file's extension for several, as PostScript with PDF).
pub fn export(app: &mut App, params: &Value, default_name: &str) -> bool {
    let mut p = params.clone();
    if let Some(m) = p.as_object_mut() {
        m.remove("path");
    }
    let Some(r) = app.run("file.export_data", p) else {
        return false;
    };
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut add = |name: String, v: &Value| {
        if let Some(t) = v.get("data").and_then(Value::as_str) {
            files.push((name, t.as_bytes().to_vec()));
        } else if let Some(b) = v.get("base64").and_then(Value::as_str) {
            files.push((name, rustfig_engine::base64_decode(b)));
        }
    };
    match r.get("files").and_then(Value::as_array) {
        Some(list) => {
            let stem = default_name.rsplit_once('.').map_or(default_name, |(s, _)| s);
            for f in list {
                let ext = f.get("extension").and_then(Value::as_str).unwrap_or("out");
                add(format!("{stem}.{ext}"), f);
            }
        }
        None => add(default_name.to_string(), &r),
    }
    if files.is_empty() {
        app.error("Export: nothing was produced");
        return false;
    }
    let count = files.len();
    for (name, bytes) in files {
        if let Err(e) = app.file_dialogs.download(&name, bytes) {
            app.error(e);
            return false;
        }
    }
    app.info(format!(
        "Exported {default_name} (downloaded{})",
        if count > 1 { ", several files" } else { "" }
    ));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_name_strips_folders_and_controls() {
        assert_eq!(base_name("a/b/c.fig"), "c.fig");
        assert_eq!(base_name("C:\\x\\Zeichnung ä.fig"), "Zeichnung ä.fig");
        assert_eq!(base_name("x\n.fig"), "x.fig");
        assert_eq!(base_name(""), "");
    }
}
