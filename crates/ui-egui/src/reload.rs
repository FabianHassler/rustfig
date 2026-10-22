//! Autorefresh and New window (plan S4-9).
//!
//! xfig's Autorefresh mode (`main.c` `check_refresh`, `set_autorefresh`)
//! reloads the figure when its file is newer; RustFig does the same once a
//! second while the mode is on (View menu, kept in the preferences), and
//! asks first when the figure has unsaved edits. "New window" is xfig's "New
//! xfig" button of the Open panel (`w_file.c` `new_xfig_request`): a second
//! RustFig process, optionally with a file. Both are native only: a browser
//! tab has no file on disk to watch and cannot start a program.

// Ported from xfig src/main.c and src/w_file.c (xfig licence, see LICENSE-xfig).

use crate::App;
use serde_json::json;
use std::time::SystemTime;

/// xfig's `CHECK_REFRESH_TIME`, in seconds.
pub const CHECK_SECONDS: f64 = 1.0;

/// State of the file watch.
#[derive(Debug, Default, Clone)]
pub struct Reload {
    /// Autorefresh mode.
    pub on: bool,
    watched: Option<String>,
    seen: Option<SystemTime>,
    was_modified: bool,
    last_check: f64,
    /// The file changed while the figure has unsaved edits: asking.
    pub asking: Option<String>,
}

/// The file's modification time (native only).
fn mtime(path: &str) -> Option<SystemTime> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::metadata(path).and_then(|m| m.modified()).ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        None
    }
}

impl Reload {
    /// Turn the mode on or off (xfig: "Autorefresh mode ON/OFF").
    pub fn set(&mut self, on: bool) {
        self.on = on;
        self.watched = None; // note the file's time afresh
        self.asking = None;
    }

    /// One look at the file: whether it changed since the last look and
    /// should be reloaded now (`modified`: the figure has unsaved edits, so
    /// the user is asked instead). `now` is in seconds; the file is looked at
    /// once per [`CHECK_SECONDS`].
    pub fn check(&mut self, path: Option<&str>, modified: bool, now: f64) -> Check {
        if !self.on {
            return Check::Nothing;
        }
        let path = path.map(str::to_string);
        // Opened another file or saved: this figure's own writing is not a
        // change by another program.
        let own = self.watched != path || (self.was_modified && !modified);
        self.was_modified = modified;
        if own {
            self.seen = path.as_deref().and_then(mtime);
            self.watched = path;
            self.last_check = now;
            return Check::Nothing;
        }
        if now - self.last_check < CHECK_SECONDS {
            return Check::Nothing;
        }
        self.last_check = now;
        let Some(p) = self.watched.clone() else {
            return Check::Nothing;
        };
        let m = mtime(&p);
        if m.is_none() || m == self.seen {
            return Check::Nothing;
        }
        self.seen = m;
        if modified {
            Check::Ask(p)
        } else {
            Check::Reload(p)
        }
    }
}

/// What a look at the file found.
#[derive(Debug, PartialEq, Eq)]
pub enum Check {
    Nothing,
    Reload(String),
    Ask(String),
}

/// Once per frame (native only; a no-op on the web).
pub fn tick(app: &mut App, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if !app.reload.on {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(CHECK_SECONDS));
        let now = ctx.input(|i| i.time);
        let path = app.engine.path().map(str::to_string);
        let modified = app.engine.is_modified();
        match app.reload.check(path.as_deref(), modified, now) {
            Check::Nothing => {}
            Check::Reload(p) => reload(app, &p),
            Check::Ask(p) => app.reload.asking = Some(p),
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (app, ctx);
}

/// Load the file again (xfig's `load_file` in `check_refresh`).
pub fn reload(app: &mut App, path: &str) {
    if app.run("file.open", json!({ "path": path })).is_some() {
        app.reload.set(true);
        app.info(format!("Reloaded {path}"));
    }
}

/// The question when the file changed and the figure has unsaved edits.
pub fn ask(app: &mut App, ctx: &egui::Context) {
    let Some(path) = app.reload.asking.clone() else { return };
    let mut choice = None;
    egui::Modal::new(egui::Id::new("reload_changed")).show(ctx, |ui| {
        ui.heading("File changed on disk");
        ui.label(format!(
            "{path} was changed by another program, and this figure has unsaved edits."
        ));
        ui.horizontal(|ui| {
            if ui.button("Reload").clicked() {
                choice = Some(true);
            }
            if ui.button("Keep my version").clicked() {
                choice = Some(false);
            }
        });
    });
    match choice {
        Some(true) => {
            app.reload.asking = None;
            reload(app, &path);
        }
        Some(false) => app.reload.asking = None,
        None => {}
    }
}

/// The command line of a new window: the program's own arguments are not
/// handed on, only the file to open.
pub fn new_window_args(path: Option<&str>) -> Vec<String> {
    path.filter(|p| !p.is_empty()).map(str::to_string).into_iter().collect()
}

/// Start a second RustFig (xfig's "New xfig..."). Native only.
pub fn new_window(path: Option<&str>) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let exe = std::env::current_exe().map_err(|e| format!("Can't find the program: {e}"))?;
        std::process::Command::new(exe)
            .args(new_window_args(path))
            .spawn()
            .map(|_| ()) // the new window runs on its own
            .map_err(|e| format!("Couldn't open a new window: {e}"))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        Err("A new window is not available in the browser: open the page again".to_string())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn scratch() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("rustfig-reload-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d.join("a.fig")
    }

    fn touch(path: &std::path::Path, secs_ahead: u64) {
        let f = std::fs::OpenOptions::new().write(true).open(path).unwrap();
        f.set_modified(SystemTime::now() + std::time::Duration::from_secs(secs_ahead))
            .unwrap();
    }

    #[test]
    fn changed_file_reloads_or_asks() {
        let path = scratch();
        std::fs::write(&path, "x").unwrap();
        let p = path.to_string_lossy().into_owned();
        let mut r = Reload::default();
        r.set(true);
        // First look notes the time.
        assert_eq!(r.check(Some(&p), false, 0.0), Check::Nothing);
        // Not before a second has passed, and not when nothing changed.
        assert_eq!(r.check(Some(&p), false, 0.5), Check::Nothing);
        assert_eq!(r.check(Some(&p), false, 1.6), Check::Nothing);
        touch(&path, 10);
        assert_eq!(r.check(Some(&p), false, 3.0), Check::Reload(p.clone()));
        assert_eq!(r.check(Some(&p), false, 5.0), Check::Nothing);
        // With unsaved edits it asks.
        assert_eq!(r.check(Some(&p), true, 6.0), Check::Nothing);
        touch(&path, 20);
        assert_eq!(r.check(Some(&p), true, 8.0), Check::Ask(p.clone()));
        // Our own save (modified turns false) is not a change.
        touch(&path, 30);
        assert_eq!(r.check(Some(&p), false, 9.0), Check::Nothing);
        assert_eq!(r.check(Some(&p), false, 11.0), Check::Nothing);
        // Off: never.
        r.set(false);
        touch(&path, 40);
        assert_eq!(r.check(Some(&p), false, 20.0), Check::Nothing);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn new_window_passes_only_the_file() {
        assert_eq!(new_window_args(None), Vec::<String>::new());
        assert_eq!(new_window_args(Some("")), Vec::<String>::new());
        assert_eq!(new_window_args(Some("/a/b.fig")), vec!["/a/b.fig".to_string()]);
    }
}
