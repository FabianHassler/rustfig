//! Typed coordinates (xfig's keyboard panel, `w_keyboard.c`, Ctrl+K in xfig;
//! xfig licence, see LICENSE-xfig): a small window with a text field; Enter
//! (or a button) runs `tool.point`, which uses the typed point as if the
//! canvas had been clicked with that mouse button. The last entries are
//! kept as a history to click.

use crate::App;
use serde_json::json;

const MAX_HISTORY: usize = 10;

/// State of the entry window.
#[derive(Default)]
pub struct PointEntry {
    pub open: bool,
    pub text: String,
    pub history: Vec<String>,
    pub error: String,
    focus: bool,
}

impl PointEntry {
    pub fn open(&mut self) {
        self.open = true;
        self.focus = true;
    }
}

fn submit(app: &mut App, button: &str) {
    let text = app.point_entry.text.trim().to_string();
    if text.is_empty() {
        return;
    }
    match app.try_run("tool.point", &json!({"text": text, "button": button})) {
        Ok(_) => {
            let h = &mut app.point_entry.history;
            h.retain(|t| *t != text);
            h.insert(0, text);
            h.truncate(MAX_HISTORY);
            app.point_entry.text.clear();
            app.point_entry.error.clear();
        }
        Err(e) => app.point_entry.error = e,
    }
    app.point_entry.focus = true;
}

/// Draw the window when it is open.
pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.point_entry.open {
        return;
    }
    let mut open = true;
    egui::Window::new("Xfig: Keyboard")
        .open(&mut open)
        .resizable(false)
        .default_pos(egui::pos2(300.0, 200.0))
        .show(ctx, |ui| {
            ui.label("Point as x,y or length<angle (r before a number: relative):");
            let mut enter = false;
            ui.horizontal(|ui| {
                let resp = ui
                    .add(egui::TextEdit::singleline(&mut app.point_entry.text).desired_width(180.0))
                    .on_hover_text("Enter: left button");
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Point entry"));
                if app.point_entry.focus {
                    resp.request_focus();
                    app.point_entry.focus = false;
                }
                enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
            let mut button = enter.then_some("left");
            ui.horizontal(|ui| {
                for (label, name) in [("Left", "left"), ("Middle", "middle"), ("Right", "right")] {
                    if ui.button(label).clicked() {
                        button = Some(name);
                    }
                }
            });
            if !app.point_entry.error.is_empty() {
                ui.colored_label(ui.visuals().error_fg_color, app.point_entry.error.clone());
            }
            if !app.point_entry.history.is_empty() {
                ui.separator();
                ui.label("History");
                let mut pick = None;
                for h in &app.point_entry.history {
                    if ui.link(h).clicked() {
                        pick = Some(h.clone());
                    }
                }
                if let Some(h) = pick {
                    app.point_entry.text = h;
                    app.point_entry.focus = true;
                }
            }
            if let Some(b) = button {
                submit(app, b);
            }
        });
    app.point_entry.open &= open;
}
