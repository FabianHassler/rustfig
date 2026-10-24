//! Two small Help pages (W18): the mouse cheat sheet (Help › Mouse
//! buttons…), generated from `Engine::mouse_hints()` for every mode so it
//! cannot drift from the mouse-function indicator, and "What's new"
//! (Help › What's new…), shown once after the version changes (the version
//! last seen is kept in the preferences, `whats_new_seen`).

use crate::platform::Os;
use crate::App;
use rustfig_engine::{Engine, ToolKind};

/// One mode's three buttons (and the Shift variant where the mode has one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseRow {
    pub id: &'static str,
    pub label: &'static str,
    pub left: &'static str,
    pub middle: &'static str,
    pub right: &'static str,
    pub shift: &'static str,
}

/// Every mode with the words of its mouse-function indicator.
pub fn mouse_rows() -> Vec<MouseRow> {
    ToolKind::ALL
        .iter()
        .map(|&t| {
            let mut e = Engine::new();
            e.set_tool(t);
            let h = e.mouse_hints();
            MouseRow {
                id: t.id(),
                label: t.label(),
                left: h.left,
                middle: h.middle,
                right: h.right,
                shift: h.shift,
            }
        })
        .collect()
}

/// How to press the middle and right buttons without them, for `os`.
pub fn substitutes(os: Os) -> Vec<&'static str> {
    let mut v = match os {
        Os::Mac => vec![
            "Middle button: Option+click.",
            "Right button: Ctrl+click, or a two-finger click.",
        ],
        Os::Windows | Os::Linux => vec![
            "Middle button: Alt+click, or the left and right buttons together (a chord, as on a two-button mouse).",
            "Right button: the right button (Ctrl+click is xfig's zoom).",
        ],
    };
    v.push("Touch screens: a long press is the right button.");
    v
}

/// State of the mouse sheet.
#[derive(Default)]
pub struct MouseSheet {
    pub open: bool,
    rows: Vec<MouseRow>,
}

impl MouseSheet {
    pub fn show(&mut self) {
        self.rows = mouse_rows();
        self.open = true;
    }
}

/// Show the mouse sheet when it is open.
pub fn show_mouse_sheet(app: &mut App, ctx: &egui::Context) {
    if !app.mouse_sheet.open {
        return;
    }
    let mut open = true;
    let os = Os::current(ctx);
    let rows = &app.mouse_sheet.rows;
    egui::Window::new("Mouse buttons")
        .open(&mut open)
        .default_size([640.0, 460.0])
        .show(ctx, |ui| {
            for s in substitutes(os) {
                ui.label(s);
            }
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("mouse_grid")
                    .striped(true)
                    .num_columns(5)
                    .show(ui, |ui| {
                        for h in ["Mode", "Left", "Middle", "Right", "With Shift"] {
                            ui.strong(h);
                        }
                        ui.end_row();
                        for r in rows {
                            ui.label(r.label);
                            ui.label(r.left);
                            ui.label(r.middle);
                            ui.label(r.right);
                            ui.label(r.shift);
                            ui.end_row();
                        }
                    });
            });
        });
    app.mouse_sheet.open = open;
}

/// The "What's new" text: a few lines per release, newest first.
pub const WHATS_NEW: &str = "\
Command search: press Ctrl+Shift+K (Cmd+Shift+K on macOS) to find any command or menu entry by name and run it.
F1 over a mode button, the indicator panel or an open dialog opens the matching page of the manual.
Help > Mouse buttons lists what the left, middle and right buttons do in every mode.
Help > What's new shows this page again.";

/// Whether the page is shown at start: when the version last seen is not
/// the running one. Not on the first start (none stored): there is nothing
/// new yet, and scratch settings folders would show it on every launch.
pub fn should_show(seen: Option<&str>, current: &str) -> bool {
    seen.is_some_and(|s| s != current)
}

/// At start: open the page if the version changed and remember it, so it
/// shows once per version.
pub fn check_version(app: &mut App) {
    let current = env!("CARGO_PKG_VERSION");
    let seen = app.engine.prefs.rustfig.whats_new_seen.as_deref();
    if seen == Some(current) {
        return;
    }
    app.whats_new_open = should_show(seen, current);
    app.engine.prefs.rustfig.whats_new_seen = Some(current.to_string());
    app.engine.prefs_dirty = true;
}

/// Show the "What's new" window when it is open.
pub fn show_whats_new(app: &mut App, ctx: &egui::Context) {
    if !app.whats_new_open {
        return;
    }
    let mut open = true;
    egui::Window::new("What's new")
        .open(&mut open)
        .default_width(420.0)
        .show(ctx, |ui| {
            ui.strong(format!("RustFig {}", env!("CARGO_PKG_VERSION")));
            ui.separator();
            for l in WHATS_NEW.lines() {
                ui.label(format!("\u{2022} {l}"));
            }
        });
    app.whats_new_open = open;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_once_per_version() {
        assert!(!should_show(None, "1.2.0"));
        assert!(should_show(Some("1.1.0"), "1.2.0"));
        assert!(!should_show(Some("1.2.0"), "1.2.0"));
    }

    #[test]
    fn check_version_marks_the_version_seen() {
        let mut app = App::with_engine(Engine::new());
        check_version(&mut app);
        assert!(!app.whats_new_open, "not on the first start");
        assert_eq!(
            app.engine.prefs.rustfig.whats_new_seen.as_deref(),
            Some(env!("CARGO_PKG_VERSION"))
        );
        app.engine.prefs.rustfig.whats_new_seen = Some("0.0.1".into());
        check_version(&mut app);
        assert!(app.whats_new_open);
        app.whats_new_open = false;
        check_version(&mut app);
        assert!(!app.whats_new_open);
    }

    #[test]
    fn sheet_covers_every_mode_and_follows_the_indicator() {
        let rows = mouse_rows();
        assert_eq!(rows.len(), ToolKind::ALL.len());
        for t in ToolKind::ALL {
            let r = rows.iter().find(|r| r.id == t.id()).unwrap();
            let mut e = Engine::new();
            e.set_tool(t);
            let h = e.mouse_hints();
            assert_eq!((r.left, r.middle, r.right), (h.left, h.middle, h.right), "{}", t.id());
            assert!(
                !(r.left.is_empty() && r.middle.is_empty() && r.right.is_empty()),
                "{}",
                t.id()
            );
        }
    }

    #[test]
    fn substitutes_per_system() {
        assert!(substitutes(Os::Mac).iter().any(|s| s.contains("Option+click")));
        assert!(substitutes(Os::Mac).iter().any(|s| s.contains("Ctrl+click")));
        for os in [Os::Windows, Os::Linux] {
            let v = substitutes(os).join(" ");
            assert!(v.contains("Alt+click") && v.contains("together"));
        }
        for os in [Os::Mac, Os::Windows, Os::Linux] {
            assert!(substitutes(os).iter().any(|s| s.contains("long press")));
        }
    }
}
