//! The Export panel's export in the background (W10-13): the conversion
//! runs on a thread while the UI keeps drawing, with a progress window,
//! Cancel and "Show in folder". Native only: on the web the export stays
//! a synchronous download path. Commands and MCP stay synchronous.

use crate::App;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

/// An export on its thread.
pub struct Running {
    rx: Receiver<Result<Value, String>>,
    cancel: Arc<AtomicBool>,
    path: String,
    started: std::time::Instant,
    /// What to do once the file is written: the `file.export` parameters
    /// to keep up to date on save, or switching that off.
    after: After,
}

/// The export-on-save change that follows a finished export.
pub enum After {
    Nothing,
    Keep(Value),
    Off,
}

/// Start `p` (`file.export`'s parameters) in the background. An error
/// from the checks (bad options, input file) comes back at once.
pub fn start(app: &mut App, p: &Value, after: After) -> Result<(), String> {
    if app.export_bg.is_some() {
        return Err("An export is already running".into());
    }
    let job = rustfig_engine::export_job(&app.engine, p).map_err(|e| e.message)?;
    let path = job.path().to_string();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    let (tx, rx) = channel();
    std::thread::Builder::new()
        .name("export".into())
        .spawn(move || {
            // A panic in a driver becomes an error message, not a crash.
            let r = match rustfig_engine::guard::catch(|| job.run(&flag).map_err(|e| e.message)) {
                Ok(r) => r,
                Err(m) => Err(format!("internal error (please report): {m}")),
            };
            let _ = tx.send(r); // the UI may be gone (cancelled)
        })
        .map_err(|e| format!("cannot start the export: {e}"))?;
    app.export_bg = Some(Running {
        rx,
        cancel,
        path,
        started: std::time::Instant::now(),
        after,
    });
    Ok(())
}

/// Cancel the running export: its result is discarded and no file is
/// written once the conversion in progress ends (fig2dev's drivers have
/// no cancel hook inside one conversion, so that step runs to its end).
pub fn cancel(app: &mut App) {
    if let Some(r) = app.export_bg.take() {
        r.cancel.store(true, Ordering::Relaxed);
        app.info("Export cancelled");
    }
}

/// Collect a finished export, and show the progress window.
pub fn tick(app: &mut App, ctx: &egui::Context) {
    let Some(r) = &app.export_bg else {
        show_done(app, ctx);
        return;
    };
    match r.rx.try_recv() {
        Ok(result) => {
            let Some(r) = app.export_bg.take() else { return };
            match result {
                Ok(v) => finished(app, r.after, &v),
                Err(m) => app.error(m),
            }
        }
        Err(TryRecvError::Disconnected) => {
            app.export_bg = None;
            app.error("The export stopped unexpectedly");
        }
        Err(TryRecvError::Empty) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            let secs = r.started.elapsed().as_secs();
            let name = r.path.clone();
            let mut cancel_clicked = false;
            egui::Window::new("Exporting")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(format!("Writing {name} ({secs} s)"));
                    });
                    if ui.button("Cancel").clicked() {
                        cancel_clicked = true;
                    }
                });
            if cancel_clicked {
                cancel(app);
            }
        }
    }
}

fn finished(app: &mut App, after: After, v: &Value) {
    let paths: Vec<String> = v["paths"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p.as_str().map(str::to_string))
        .collect();
    app.info(format!("Exported {}", paths.join(" and ")));
    match after {
        After::Nothing => {}
        After::Keep(p) => {
            app.run("file.export_on_save", p);
        }
        After::Off => {
            if app.engine.doc().export_on_save.is_some() {
                app.run("file.export_on_save", serde_json::json!({ "off": true }));
            }
        }
    }
    app.dialogs.export.open = false;
    app.last_export = paths.first().cloned();
}

/// A small notice after a background export, with "Show in folder".
fn show_done(app: &mut App, ctx: &egui::Context) {
    let Some(path) = app.last_export.clone() else { return };
    let mut close = false;
    let mut show = false;
    egui::Window::new("Export finished")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -12.0))
        .show(ctx, |ui| {
            ui.label(format!("Exported {path}"));
            ui.horizontal(|ui| {
                if ui.button("Show in folder").clicked() {
                    show = true;
                }
                if ui.button("Close").clicked() {
                    close = true;
                }
            });
        });
    if show {
        app.run("app.show_export_folder", serde_json::json!({}));
    }
    if close {
        app.last_export = None;
    }
}

/// Open the folder holding `path` with the file selected where the
/// system can (macOS `open -R`, Windows `explorer /select,`, elsewhere
/// xdg-open on the folder).
pub fn show_in_folder(path: &str) -> Result<(), String> {
    use std::process::Command;
    let p = std::path::Path::new(path);
    let dir = p
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let mut cmd;
    if cfg!(target_os = "macos") {
        cmd = Command::new("open");
        cmd.arg("-R").arg(p);
    } else if cfg!(target_os = "windows") {
        cmd = Command::new("explorer");
        cmd.arg(format!("/select,{path}"));
    } else {
        cmd = Command::new("xdg-open");
        cmd.arg(dir);
    }
    // The child is the file manager; it is not waited for.
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("cannot show the folder: {e}"))
}
