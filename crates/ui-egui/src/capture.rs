//! Screen capture into a picture (D11, replaces xfig's `w_capture.c`).
//!
//! xfig hid its windows, let the user click a window or drag a rectangle on the screen, wrote
//! the image to `<figure>_<seconds>.pcx` and showed it as a picture object. Here the system's
//! own selector does the picking (`rustfig_os::capture`: macOS `screencapture -i`, the Linux
//! portal, the whole screen on Windows); this module hides the window while it runs, asks for the
//! PNG file name, writes the file and adds the picture object (`shape.picture`).

use crate::App;
use serde_json::{json, Value};
use std::sync::mpsc::Receiver;

/// What the capture thread sends back.
pub type CaptureResult = Result<Option<Vec<u8>>, String>;

/// A capture in progress.
#[derive(Default)]
pub struct Capture {
    /// The command asked for a capture; the next frame starts it (it needs the egui context).
    pub requested: bool,
    rx: Option<Receiver<CaptureResult>>,
}

impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Capture").field("running", &self.rx.is_some()).finish()
    }
}

impl Capture {
    /// The system's selector is open.
    pub fn running(&self) -> bool {
        self.rx.is_some()
    }

    /// Hand a result in as if the capture thread had delivered it (tests).
    pub fn inject(&mut self, r: CaptureResult) {
        let (tx, rx) = std::sync::mpsc::channel();
        // The receiver is right here; the send cannot fail.
        let _ = tx.send(r);
        self.rx = Some(rx);
    }
}

impl App {
    /// Called every frame: start a requested capture, deliver a finished one.
    pub(crate) fn poll_capture(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.capture.requested) {
            self.start_capture(ctx);
        }
        let Some(rx) = &self.capture.rx else { return };
        let r = match rx.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
                return;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err("The capture ended without an image".into()),
        };
        self.capture.rx = None;
        if rustfig_os::capture::needs_window_hidden() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        match r {
            Ok(Some(bytes)) => {
                if let Err(e) = self.finish_capture(&bytes) {
                    self.error(e);
                }
            }
            Ok(None) => self.info("Capture cancelled"),
            Err(e) => self.error(e),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_capture(&mut self, ctx: &egui::Context) {
        if self.capture.running() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let wake = ctx.clone();
        let spawned = std::thread::Builder::new()
            .name("screen-capture".into())
            .spawn(move || {
                let r = rustfig_os::capture::capture_region();
                // The window may have closed meanwhile: nobody to tell then.
                let _ = tx.send(r);
                wake.request_repaint();
            });
        match spawned {
            Ok(_) => {
                self.capture.rx = Some(rx);
                // xfig unmaps its windows during the capture.
                if rustfig_os::capture::needs_window_hidden() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                self.info("Select a window or an area of the screen (Escape cancels)");
            }
            Err(e) => self.error(format!("Could not start the capture: {e}")),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn start_capture(&mut self, _ctx: &egui::Context) {
        self.error(rustfig_os::capture::unavailable_reason());
    }

    /// The captured PNG: ask where to save it (default: `<figure>_<seconds>.png`), write it and
    /// add the picture object. Cancelling the file dialog drops the image.
    pub fn finish_capture(&mut self, png: &[u8]) -> Result<Value, String> {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let name = rustfig_engine::cmd::picture::capture_file_name(self.engine.path(), secs);
        let default = match self.engine.path().and_then(|p| std::path::Path::new(p).parent()) {
            Some(dir) if !dir.as_os_str().is_empty() => dir.join(&name).to_string_lossy().into_owned(),
            _ => name,
        };
        let Some(file) = self.file_dialogs.save_file(&default, "PNG pictures", &["png"]) else {
            self.info("Capture not saved");
            return Ok(json!({}));
        };
        std::fs::write(&file, png).map_err(|e| format!("{file}: {e}"))?;
        let c = self.engine.view.visible().center();
        self.engine
            .execute("shape.picture", &json!({"path": file, "x": c.x, "y": c.y}))
            .map_err(|e| e.message)
    }
}
