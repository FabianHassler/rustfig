//! The Manage Styles window (E6, View › Manage Styles…, Ctrl+Y), laid out
//! like xfig's style panel (`w_style.c`): a Family list (name field, Add,
//! Delete) and a Style list (name field, Add, Delete, Set) side by side.
//! Add keeps the indicator's attributes under the name; Set applies a style
//! to the indicator. The name fields are for new entries only: after Add
//! (or Return) the field empties and the new entry is selected in its list. Styles live in the settings, never in the drawing. It
//! only calls the `style.*` commands.

use crate::App;
use serde_json::json;

/// State of the Styles window.
#[derive(Default)]
pub struct StylesWindow {
    pub open: bool,
    /// The field for a new family's name.
    pub family_name: String,
    /// The field for a new style's name.
    pub style_name: String,
    /// The selected family.
    pub family: Option<String>,
    /// The selected style.
    pub style: Option<String>,
}

fn labelled(r: egui::Response, label: &str) -> egui::Response {
    let label = label.to_string();
    r.widget_info(move || egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.clone()));
    r
}

/// A name field with a grey hint; true when Return was pressed in it.
fn field(ui: &mut egui::Ui, text: &mut String, label: &str, hint: &str) -> bool {
    let r = ui.add(egui::TextEdit::singleline(text).hint_text(hint));
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, label));
    r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
}

/// Show the window when it is open.
pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.styles.open {
        return;
    }
    let mut open = true;
    let families: Vec<(String, Vec<String>)> = app
        .engine
        .prefs
        .rustfig
        .style_families
        .iter()
        .map(|f| (f.name.clone(), f.styles.iter().map(|s| s.name.clone()).collect()))
        .collect();
    // A selection that no longer exists is dropped.
    if !app
        .styles
        .family
        .as_ref()
        .is_some_and(|f| families.iter().any(|(n, _)| n == f))
    {
        app.styles.family = None;
    }
    let styles: Vec<String> = app
        .styles
        .family
        .as_ref()
        .and_then(|f| families.iter().find(|(n, _)| n == f))
        .map(|(_, s)| s.clone())
        .unwrap_or_default();
    if !app.styles.style.as_ref().is_some_and(|s| styles.contains(s)) {
        app.styles.style = None;
    }
    // Actions run after the layout, so the lists aren't changed while drawn.
    let mut close = false;
    let mut actions: Vec<(&'static str, serde_json::Value)> = Vec::new();
    egui::Window::new("Manage Styles")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| crate::dialog_body(ui, |ui| {
            ui.columns(2, |cols| {
                // Family list.
                let ui = &mut cols[0];
                ui.strong("Family");
                egui::ScrollArea::vertical()
                    .id_salt("style_families")
                    .max_height(160.0)
                    .show(ui, |ui| {
                        for (n, _) in &families {
                            let sel = app.styles.family.as_ref() == Some(n);
                            if labelled(ui.selectable_label(sel, n), &format!("Family {n}")).clicked() {
                                app.styles.family = Some(n.clone());
                                app.styles.style = None;
                            }
                        }
                    });
                let enter = field(ui, &mut app.styles.family_name, "Family name", "New family name");
                let named = !app.styles.family_name.trim().is_empty();
                ui.horizontal(|ui| {
                    let add = labelled(ui.add_enabled(named, egui::Button::new("Add")), "Add family").clicked();
                    if named && (add || enter) {
                        let name = std::mem::take(&mut app.styles.family_name).trim().to_string();
                        actions.push(("style.family_add", json!({"name": name})));
                        app.styles.family = Some(name);
                        app.styles.style = None;
                    }
                    let can = app.styles.family.is_some();
                    if labelled(ui.add_enabled(can, egui::Button::new("Delete")), "Delete family").clicked() {
                        if let Some(f) = app.styles.family.take() {
                            actions.push(("style.family_delete", json!({"family": f})));
                        }
                    }
                });
                // Style list.
                let ui = &mut cols[1];
                ui.strong("Style");
                egui::ScrollArea::vertical()
                    .id_salt("style_styles")
                    .max_height(160.0)
                    .show(ui, |ui| {
                        for n in &styles {
                            let sel = app.styles.style.as_ref() == Some(n);
                            if labelled(ui.selectable_label(sel, n), &format!("Style {n}")).clicked() {
                                app.styles.style = Some(n.clone());
                            }
                        }
                    });
                let has_family = app.styles.family.is_some();
                let hint = if has_family {
                    "New style name"
                } else {
                    "Pick a family first"
                };
                let enter = field(ui, &mut app.styles.style_name, "Style name", hint);
                let named = !app.styles.style_name.trim().is_empty();
                ui.horizontal(|ui| {
                    let add = labelled(ui.add_enabled(has_family && named, egui::Button::new("Add")), "Add style")
                        .clicked();
                    if named && (add || enter) {
                        if let Some(f) = &app.styles.family {
                            let name = std::mem::take(&mut app.styles.style_name).trim().to_string();
                            actions.push(("style.save", json!({"family": f, "name": name})));
                            app.styles.style = Some(name);
                        }
                    }
                    let sel = app.styles.family.clone().zip(app.styles.style.clone());
                    if labelled(
                        ui.add_enabled(sel.is_some(), egui::Button::new("Delete")),
                        "Delete style",
                    )
                    .clicked()
                    {
                        if let Some((f, s)) = &sel {
                            actions.push(("style.delete", json!({"family": f, "name": s})));
                            app.styles.style = None;
                        }
                    }
                    if labelled(ui.add_enabled(sel.is_some(), egui::Button::new("Set")), "Set style")
                        .clicked()
                    {
                        if let Some((f, s)) = &sel {
                            actions.push(("style.apply", json!({"family": f, "name": s})));
                        }
                    }
                });
            });
            ui.separator();
            ui.label("Add keeps the current attributes of the indicator panel under the new name (a style of the same name is replaced); Set makes the selected style the indicator's.");
            if labelled(ui.button("Close"), "Close styles").clicked() {
                close = true;
            }
        }));
    app.styles.open &= open && !close;
    for (id, params) in actions {
        app.run(id, params);
    }
}
