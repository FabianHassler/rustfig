//! Menu bar, keyboard shortcuts and the UI-only commands.

use crate::{dialogs, App, Pending};
use rustfig_engine::{cmd, Key, ToolEvent};
use serde_json::{json, Value};

/// A command implemented by the UI (dialogs, quitting).
pub struct UiCommand {
    pub id: &'static str,
    pub label: &'static str,
    pub shortcut: &'static str,
    pub params: &'static str,
}

pub const UI_COMMANDS: &[UiCommand] = &[
    UiCommand {
        id: "app.new",
        label: "New",
        shortcut: "Ctrl+N",
        params: "{}",
    },
    UiCommand {
        id: "app.open",
        label: "Open…",
        shortcut: "Ctrl+O",
        params: "{\"path\"?: string}",
    },
    UiCommand {
        id: "app.merge",
        label: "Merge…",
        shortcut: "",
        params: "{\"path\"?: string}",
    },
    UiCommand {
        id: "app.save",
        label: "Save",
        shortcut: "Ctrl+S",
        params: "{}",
    },
    UiCommand {
        id: "app.save_as",
        label: "Save As…",
        shortcut: "Ctrl+Shift+S",
        params: "{\"path\"?: string}",
    },
    UiCommand {
        id: "app.export",
        label: "Export…",
        shortcut: "Ctrl+E",
        params: "{}",
    },
    UiCommand {
        id: "app.quit",
        label: "Quit",
        shortcut: "Ctrl+Q",
        params: "{}",
    },
    UiCommand {
        id: "app.edit_selected",
        label: "Edit Attributes…",
        shortcut: "Ctrl+Shift+E",
        params: "{}",
    },
    UiCommand {
        id: "app.search",
        label: "Search/Replace…",
        shortcut: "Ctrl+F",
        params: "{}",
    },
    UiCommand {
        id: "app.document_settings",
        label: "Document Settings…",
        shortcut: "",
        params: "{}",
    },
    UiCommand {
        id: "app.toggle_depths",
        label: "Depth Manager",
        shortcut: "",
        params: "{}",
    },
    UiCommand {
        id: "app.toggle_indicator",
        label: "Attribute Panel",
        shortcut: "",
        params: "{}",
    },
    UiCommand {
        id: "app.commands",
        label: "Command List",
        shortcut: "",
        params: "{}",
    },
    UiCommand {
        id: "app.about",
        label: "About RustFig",
        shortcut: "",
        params: "{}",
    },
    UiCommand {
        id: "app.import_picture",
        label: "Import Picture…",
        shortcut: "",
        params: "{\"path\"?: string}",
    },
];

pub fn find_ui(id: &str) -> Option<&'static UiCommand> {
    UI_COMMANDS.iter().find(|c| c.id == id)
}

/// Run a UI-only command.
pub fn run_ui(app: &mut App, id: &str, p: &Value) -> Result<Value, String> {
    let path = p.get("path").and_then(Value::as_str).map(str::to_string);
    match id {
        "app.new" => app.guard_unsaved(Pending::New),
        "app.open" => app.guard_unsaved(Pending::Open(path)),
        "app.merge" => {
            if let Some(p) = path.or_else(|| dialogs::pick_file("Figures", &["fig", "xfig", "rustfig"])) {
                return app
                    .engine
                    .execute("file.merge", &json!({ "path": p }))
                    .map_err(|e| e.message);
            }
        }
        "app.save" => {
            if app.engine.path().is_some() {
                let r = app.engine.execute("file.save", &json!({})).map_err(|e| e.message)?;
                app.info("Saved");
                return Ok(r);
            }
            return run_ui(app, "app.save_as", p);
        }
        "app.save_as" => {
            if let Some(p) = path.or_else(|| dialogs::save_file("figure.fig", "Figures", &["fig", "rustfig"])) {
                let r = app
                    .engine
                    .execute("file.save", &json!({ "path": p }))
                    .map_err(|e| e.message)?;
                app.info(format!("Saved {p}"));
                return Ok(r);
            }
        }
        "app.export" => app.dialogs.export.open = true,
        "app.quit" => app.guard_unsaved(Pending::Quit),
        "app.edit_selected" => match app.engine.selection().first().copied() {
            Some(id) => app.dialogs.open_edit(&app.engine, id),
            None => return Err("select an object first".into()),
        },
        "app.search" => app.dialogs.search.open = true,
        "app.document_settings" => app.dialogs.settings_open = true,
        "app.toggle_depths" => app.show_depths = !app.show_depths,
        "app.toggle_indicator" => app.show_indicator = !app.show_indicator,
        "app.commands" => app.dialogs.commands_open = true,
        "app.about" => app.dialogs.about_open = true,
        "app.import_picture" => {
            let file = path.or_else(|| {
                dialogs::pick_file(
                    "Pictures",
                    &["png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff", "ppm", "pgm"],
                )
            });
            if let Some(f) = file {
                let c = app.engine.view.visible().center();
                return app
                    .engine
                    .execute("shape.picture", &json!({"path": f, "x": c.x, "y": c.y}))
                    .map_err(|e| e.message);
            }
        }
        _ => return Err(format!("unknown UI command {id:?}")),
    }
    Ok(json!({}))
}

/// Menu layout: (menu, entries). "-" is a separator, "Name>" opens a submenu
/// until "<".
const LAYOUT: &[(&str, &[&str])] = &[
    (
        "File",
        &[
            "app.new",
            "app.open",
            "app.merge",
            "-",
            "app.save",
            "app.save_as",
            "app.export",
            "-",
            "app.import_picture",
            "app.document_settings",
            "-",
            "app.quit",
        ],
    ),
    (
        "Edit",
        &[
            "edit.undo",
            "edit.redo",
            "-",
            "edit.cut",
            "edit.copy",
            "edit.paste",
            "edit.duplicate",
            "edit.delete",
            "edit.delete_all",
            "-",
            "edit.select_all",
            "edit.select_none",
            "-",
            "app.search",
            "app.edit_selected",
        ],
    ),
    (
        "View",
        &[
            "view.zoom_in",
            "view.zoom_out",
            "view.zoom_fit",
            "view.zoom_actual",
            "view.pan_origin",
            "-",
            "view.toggle_grid",
            "view.toggle_page_border",
            "doc.toggle_orientation",
            "-",
            "app.toggle_indicator",
            "app.toggle_depths",
            "depth.show_all",
        ],
    ),
    (
        "Object",
        &[
            "object.group",
            "object.ungroup",
            "-",
            "object.update",
            "object.convert",
            "object.toggle_closed",
            "Arrows>",
            "object.toggle_forward_arrow",
            "object.toggle_backward_arrow",
            "<",
            "Transform>",
            "transform.rotate_ccw",
            "transform.rotate_cw",
            "transform.flip_horizontal",
            "transform.flip_vertical",
            "<",
            "Arrange>",
            "arrange.align",
            "arrange.distribute",
            "-",
            "arrange.to_front",
            "arrange.forward",
            "arrange.backward",
            "arrange.to_back",
            "<",
            "-",
            "object.measure",
        ],
    ),
    ("Help", &["app.commands", "app.about"]),
];

fn label_of(id: &str) -> (&'static str, &'static str) {
    if let Some(c) = cmd::find(id) {
        return (c.label, c.shortcut);
    }
    if let Some(c) = find_ui(id) {
        return (c.label, c.shortcut);
    }
    ("?", "")
}

fn entry(app: &mut App, ui: &mut egui::Ui, id: &str) {
    let (label, shortcut) = label_of(id);
    let mut b = egui::Button::new(label);
    if !shortcut.is_empty() {
        b = b.shortcut_text(display_shortcut(shortcut));
    }
    let enabled = app.enabled(id);
    if ui.add_enabled(enabled, b).clicked() {
        ui.close();
        app.run(id, json!({}));
    }
}

fn entries(app: &mut App, ui: &mut egui::Ui, list: &[&str]) {
    let mut i = 0;
    while let Some(id) = list.get(i).copied() {
        i += 1;
        if id == "-" {
            ui.separator();
        } else if let Some(name) = id.strip_suffix('>') {
            let start = i;
            while list.get(i).is_some_and(|x| *x != "<") {
                i += 1;
            }
            let sub = list.get(start..i).unwrap_or(&[]);
            i += 1;
            ui.menu_button(name, |ui| entries(app, ui, sub));
        } else {
            entry(app, ui, id);
        }
    }
}

pub fn menu_bar(app: &mut App, ui: &mut egui::Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        for (name, list) in LAYOUT {
            ui.menu_button(*name, |ui| entries(app, ui, list));
            if *name == "Object" {
                ui.menu_button("Tools", |ui| {
                    ui.menu_button("Draw", |ui| {
                        for t in rustfig_engine::ToolKind::ALL.iter().filter(|t| t.is_drawing()) {
                            entry(app, ui, t.id());
                        }
                    });
                    ui.menu_button("Edit", |ui| {
                        for t in rustfig_engine::ToolKind::ALL.iter().filter(|t| !t.is_drawing()) {
                            entry(app, ui, t.id());
                        }
                    });
                });
            }
        }
    });
}

/// Human-readable shortcut ("Ctrl" shows as ⌘ on macOS).
pub fn display_shortcut(s: &str) -> String {
    if cfg!(target_os = "macos") {
        s.replace("Ctrl+", "⌘").replace("Shift+", "⇧")
    } else {
        s.to_string()
    }
}

/// Parse "Ctrl+Shift+Z" into an egui shortcut.
pub fn parse_shortcut(s: &str) -> Option<egui::KeyboardShortcut> {
    let mut mods = egui::Modifiers::NONE;
    let mut key = None;
    for part in s.split('+') {
        match part {
            "Ctrl" => mods |= egui::Modifiers::COMMAND,
            "Shift" => mods |= egui::Modifiers::SHIFT,
            "Alt" => mods |= egui::Modifiers::ALT,
            "" => key = Some(egui::Key::Plus),
            k => {
                key = match k {
                    "=" => Some(egui::Key::Equals),
                    "-" => Some(egui::Key::Minus),
                    "'" => Some(egui::Key::Quote),
                    "Delete" => Some(egui::Key::Delete),
                    "Escape" => Some(egui::Key::Escape),
                    "Up" => Some(egui::Key::ArrowUp),
                    "Down" => Some(egui::Key::ArrowDown),
                    other => egui::Key::from_name(other),
                }
            }
        }
    }
    key.map(|k| egui::KeyboardShortcut::new(mods, k))
}

/// Keyboard handling: typing goes to the text tool; otherwise shortcuts run commands.
pub fn handle_shortcuts(app: &mut App, ctx: &egui::Context) {
    if ctx.wants_keyboard_input() {
        return;
    }
    if app.engine.is_typing() {
        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            let tev = match ev {
                egui::Event::Text(t) => Some(ToolEvent::Text(t)),
                egui::Event::Paste(t) => Some(ToolEvent::Text(t)),
                egui::Event::Key { key, pressed: true, .. } => match key {
                    egui::Key::Enter => Some(ToolEvent::Key(Key::Enter)),
                    egui::Key::Escape => Some(ToolEvent::Key(Key::Escape)),
                    egui::Key::Backspace => Some(ToolEvent::Key(Key::Backspace)),
                    egui::Key::Delete => Some(ToolEvent::Key(Key::Delete)),
                    egui::Key::ArrowLeft => Some(ToolEvent::Key(Key::Left)),
                    egui::Key::ArrowRight => Some(ToolEvent::Key(Key::Right)),
                    _ => None,
                },
                _ => None,
            };
            if let Some(t) = tev {
                if let Err(e) = app.engine.tool_event(t) {
                    app.error(e.message);
                }
            }
        }
        return;
    }
    // Keys a busy tool wants (finish, cancel, remove last point).
    if app.engine.tool_busy() {
        let keys = [
            (egui::Key::Enter, Key::Enter),
            (egui::Key::Escape, Key::Escape),
            (egui::Key::Backspace, Key::Backspace),
        ];
        for (k, tk) in keys {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, k)) {
                if let Err(e) = app.engine.tool_event(ToolEvent::Key(tk)) {
                    app.error(e.message);
                }
            }
        }
    }
    // Most specific shortcuts (more modifiers) first, so Ctrl+Shift+Z beats Ctrl+Z.
    let mut all: Vec<(&'static str, egui::KeyboardShortcut)> = cmd::all()
        .iter()
        .map(|c| (c.id, c.shortcut))
        .chain(UI_COMMANDS.iter().map(|c| (c.id, c.shortcut)))
        .filter(|(_, s)| !s.is_empty())
        .filter_map(|(id, s)| parse_shortcut(s).map(|k| (id, k)))
        .collect();
    all.sort_by_key(|(_, k)| {
        std::cmp::Reverse(k.modifiers.command as u8 + k.modifiers.shift as u8 + k.modifiers.alt as u8)
    });
    let backspace_deletes = !app.engine.selection().is_empty()
        && ctx.input(|i| i.key_pressed(egui::Key::Backspace) && i.modifiers.is_none());
    if backspace_deletes {
        app.run("edit.delete", json!({}));
    }
    for (id, k) in all {
        if ctx.input_mut(|i| i.consume_shortcut(&k)) && app.enabled(id) {
            app.run(id, json!({}));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_ids_exist() {
        for (_, list) in LAYOUT {
            for id in *list {
                if *id == "-" || *id == "<" || id.ends_with('>') {
                    continue;
                }
                assert!(cmd::find(id).is_some() || find_ui(id).is_some(), "unknown id {id}");
            }
        }
    }

    #[test]
    fn shortcuts_parse() {
        for c in cmd::all() {
            if !c.shortcut.is_empty() {
                assert!(parse_shortcut(c.shortcut).is_some(), "{}: {}", c.id, c.shortcut);
            }
        }
        for c in UI_COMMANDS {
            if !c.shortcut.is_empty() {
                assert!(parse_shortcut(c.shortcut).is_some(), "{}: {}", c.id, c.shortcut);
            }
        }
    }

    #[test]
    fn ui_and_engine_shortcuts_do_not_clash() {
        let engine: Vec<&str> = cmd::all()
            .iter()
            .map(|c| c.shortcut)
            .filter(|s| !s.is_empty())
            .collect();
        for c in UI_COMMANDS {
            assert!(
                c.shortcut.is_empty() || !engine.contains(&c.shortcut),
                "{} clashes",
                c.id
            );
        }
    }
}
