//! Menu bar, keyboard shortcuts and the UI-only commands.
//
// Ported from xfig src/w_cmdpanel.c (menu entries, order and dividers),
// app-defaults/Fig (`Fig*accelerators`) and src/w_modepanel.c (mode keys)
// (xfig licence, see LICENSE-xfig).

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
        id: "app.redraw",
        label: "Redraw",
        shortcut: "Ctrl+Shift+L",
        params: "{}",
    },
    UiCommand {
        id: "app.global_settings",
        label: "Global settings…",
        shortcut: "",
        params: "{}",
    },
    UiCommand {
        id: "app.toggle_xfig_keys",
        label: "xfig keyboard shortcuts",
        shortcut: "",
        params: "{\"on\"?: bool}  — xfig's Meta keys (Alt, macOS Option) and Ctrl mode keys",
    },
    UiCommand {
        id: "app.unit_panel",
        label: "Unit/Scale settings…",
        shortcut: "",
        params: "{}",
    },
    UiCommand {
        id: "app.toggle_layout",
        label: "Classic xfig layout",
        shortcut: "",
        params: "{\"classic\"?: bool}",
    },
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
        id: "app.export_now",
        label: "Export with current settings",
        shortcut: "",
        params: "{}  — xfig's ExportFile: next to the figure, the export panel's language and settings",
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
        id: "app.toggle_vector_icons",
        label: "Sharp attribute pictures",
        shortcut: "",
        params: "{}  — vector pictures at the screen's density, or xfig's bitmaps",
    },
    UiCommand {
        id: "app.button_style",
        label: "Button style",
        shortcut: "",
        params: "{\"style\"?: \"3d\"|\"flat\"}  — bevelled or flat buttons; toggles when omitted",
    },
    UiCommand {
        id: "app.theme",
        label: "Theme",
        shortcut: "",
        params: "{\"theme\": \"light\"|\"dark\"|\"system\"}  — colour theme (for screenshots and tests)",
    },
    UiCommand {
        id: "app.toggle_balloons",
        label: "Show info balloons",
        shortcut: "",
        params: "{}  — xfig's tooltips (Alt+Y, macOS Option+Y)",
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
            if let Some(p) = path.or_else(|| app.file_dialogs.pick_file("Figures", dialogs::FIGURE_EXTS)) {
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
            if let Some(p) = path.or_else(|| app.file_dialogs.save_file("figure.fig", "Figures", &["fig", "rustfig"])) {
                let r = app
                    .engine
                    .execute("file.save", &json!({ "path": p }))
                    .map_err(|e| e.message)?;
                app.info(format!("Saved {p}"));
                return Ok(r);
            }
        }
        "app.export" => app.dialogs.export.open = true,
        "app.export_now" => dialogs::export_now(app),
        "app.quit" => app.guard_unsaved(Pending::Quit),
        "app.edit_selected" => match app.engine.selection().first().copied() {
            Some(id) => app.dialogs.open_edit(&app.engine, id),
            None => return Err("select an object first".into()),
        },
        "app.search" => app.dialogs.search.open = true,
        "app.document_settings" => app.dialogs.settings_open = true,
        "app.unit_panel" => app.unit_panel_open = true,
        "app.global_settings" => app.dialogs.global_open = true,
        "app.toggle_xfig_keys" => {
            app.xfig_keys = p.get("on").and_then(Value::as_bool).unwrap_or(!app.xfig_keys);
        }
        "app.toggle_depths" => app.show_depths = !app.show_depths,
        "app.toggle_balloons" => app.show_balloons = !app.show_balloons,
        "app.toggle_vector_icons" => app.vector_icons = !app.vector_icons,
        "app.button_style" => {
            app.button_3d = match p.get("style").and_then(Value::as_str) {
                Some("3d") => true,
                Some("flat") => false,
                Some(other) => return Err(format!("unknown button style {other:?} (3d or flat)")),
                None => !app.button_3d,
            };
        }
        "app.theme" => {
            app.theme_request = Some(match p.get("theme").and_then(Value::as_str) {
                Some("light") => egui::ThemePreference::Light,
                Some("dark") => egui::ThemePreference::Dark,
                Some("system") => egui::ThemePreference::System,
                _ => return Err("theme must be light, dark or system".into()),
            });
        }
        "app.redraw" => app.canvas.invalidate(),
        "app.toggle_layout" => {
            let classic = p.get("classic").and_then(Value::as_bool).unwrap_or(!app.classic());
            app.layout = if classic {
                crate::Layout::Classic
            } else {
                crate::Layout::Modern
            };
        }
        "app.toggle_indicator" => app.show_indicator = !app.show_indicator,
        "app.commands" => app.dialogs.commands_open = true,
        "app.about" => app.dialogs.about_open = true,
        "app.import_picture" => {
            let file = path.or_else(|| app.file_dialogs.pick_file("Pictures", dialogs::PICTURE_EXTS));
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

/// Menu layout, after xfig's (File, Edit, View, Snap, Help) with RustFig's
/// Object and Tools menus as extras. "-" is a separator, "Name>" opens a
/// submenu until "<". The Snap menu is built in code ([`snap_menu`]).
const LAYOUT: &[(&str, &[&str])] = &[
    (
        "File",
        &[
            "app.new",
            "app.open",
            "app.merge",
            "app.save",
            "app.save_as",
            "app.export",
            "-",
            "app.import_picture",
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
            "edit.paste",
            "edit.cut",
            "edit.copy",
            "edit.duplicate",
            "edit.delete",
            "-",
            "app.search",
            "edit.delete_all",
            "-",
            "app.document_settings",
            "app.unit_panel",
            "-",
            "edit.select_all",
            "edit.select_none",
            "app.edit_selected",
        ],
    ),
    (
        "View",
        &[
            "app.redraw",
            "doc.toggle_orientation",
            "-",
            "view.zoom_in",
            "view.zoom_out",
            "view.zoom_fit",
            "view.zoom_actual",
            "view.pan_origin",
            "-",
            "view.toggle_page_border",
            "app.toggle_depths",
            "view.toggle_grid",
            "app.toggle_indicator",
            "depth.show_all",
            "-",
            "app.toggle_layout",
            "tool.mouse_buttons",
            "app.toggle_xfig_keys",
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

/// An entry of xfig's menus (`file_menu_items` … in `w_cmdpanel.c`).
#[derive(Clone, Copy)]
pub enum XfigItem {
    /// xfig's label and the RustFig command it runs.
    Cmd(&'static str, &'static str),
    /// An entry RustFig doesn't have yet: label, xfig's accelerator, plan task.
    Missing(&'static str, &'static str, &'static str),
    Sep,
}

/// xfig's menus, word for word and in order, for the classic layout. The
/// Snap menu is [`snap_menu`]. RustFig's own menus (Object, Tools) are in
/// the modern layout; its preferences are under Edit › Global settings.
pub const XFIG_MENUS: &[(&str, &[XfigItem])] = {
    use XfigItem::{Cmd, Missing, Sep};
    &[
        (
            "File",
            &[
                Cmd("New", "app.new"),
                Cmd("Open...", "app.open"),
                Cmd("Merge...", "app.merge"),
                Missing("Digitize...", "Alt+Z", "—"),
                Cmd("Save", "app.save"),
                Cmd("Save As...", "app.save_as"),
                Cmd("Export...", "app.export"),
                Missing("Print...", "Alt+P", "B5"),
                Cmd("Exit", "app.quit"),
            ],
        ),
        (
            "Edit",
            &[
                Cmd("Undo", "edit.undo"),
                Cmd("Paste Objects", "edit.paste"),
                Missing("Paste Text", "F18/F20", "E7"),
                Cmd("Search/Replace...", "app.search"),
                Missing("Spell Check...", "Alt+K", "F5"),
                Cmd("Delete All", "edit.delete_all"),
                Sep,
                Cmd("Global settings...", "app.global_settings"),
                Cmd("Set units...", "app.unit_panel"),
            ],
        ),
        (
            "View",
            &[
                Missing("Manage Styles...", "Ctrl-Y", "E6"),
                Cmd("Redraw", "app.redraw"),
                Cmd("Portrait/Landscape", "doc.toggle_orientation"),
                Cmd("Zoom In", "view.zoom_in"),
                Cmd("Zoom Out", "view.zoom_out"),
                Cmd("Zoom to Fit canvas", "view.zoom_fit"),
                Cmd("Unzoom", "view.zoom_actual"),
                Cmd("Pan to origin", "view.pan_origin"),
                Missing("Character map", "", "E5"),
                Sep,
                Cmd("Show page borders", "view.toggle_page_border"),
                Cmd("Show depth manager", "app.toggle_depths"),
                Cmd("Show info balloons", "app.toggle_balloons"),
                Missing("Show line lengths", "Alt+L", "D10"),
                Missing("Show vertex numbers", "", "—"),
                Missing("Autorefresh mode", "", "—"),
            ],
        ),
        (
            "Help",
            &[
                Cmd("RustFig Reference (commands)...", "app.commands"),
                Cmd("About RustFig...", "app.about"),
            ],
        ),
    ]
};

/// The letters that pick an entry while one of xfig's menus is open
/// (`Fig*commands*File*translations` … in `app-defaults/Fig`): menu, letter,
/// and the entry's command or, for an entry RustFig lacks, its label.
pub const XFIG_MENU_LETTERS: &[(&str, char, &str)] = &[
    ("File", 'n', "app.new"),
    ("File", 'o', "app.open"),
    ("File", 'm', "app.merge"),
    ("File", 'd', "Digitize..."),
    ("File", 's', "app.save"),
    ("File", 'a', "app.save_as"),
    ("File", 'e', "app.export"),
    ("File", 'p', "Print..."),
    ("File", 'x', "app.quit"),
    ("Edit", 'u', "edit.undo"),
    ("Edit", 'p', "edit.paste"),
    ("Edit", 't', "Paste Text"),
    ("Edit", 'f', "app.search"),
    ("Edit", 's', "Spell Check..."),
    ("Edit", 'd', "edit.delete_all"),
    ("Edit", 'g', "app.global_settings"),
    ("Edit", 'n', "app.unit_panel"),
    ("View", 's', "Manage Styles..."),
    ("View", 'r', "app.redraw"),
    ("View", 't', "doc.toggle_orientation"),
    ("View", 'i', "view.zoom_in"),
    ("View", 'o', "view.zoom_out"),
    ("View", 'f', "view.zoom_fit"),
    ("View", 'u', "view.zoom_actual"),
    ("View", 'p', "view.pan_origin"),
    ("View", 'c', "Character map"),
    ("View", 'd', "app.toggle_depths"),
    ("View", 'n', "app.toggle_balloons"),
    ("View", 'b', "view.toggle_page_border"),
    ("View", 'l', "Show line lengths"),
    ("View", 'a', "Autorefresh mode"),
    ("Help", 'x', "app.commands"),
    ("Help", 'a', "app.about"),
];

/// The menus Meta (Alt) + letter pops up (`Fig*commands*File*accelerators` …).
const XFIG_MENU_KEYS: &[(&str, &str)] = &[
    ("Alt+F", "File"),
    ("Alt+E", "Edit"),
    ("Alt+V", "View"),
    ("Alt+H", "Help"),
];

/// The command xfig's letter picks in the open `menu`, if RustFig has it.
pub fn xfig_menu_letter(menu: &str, letter: char) -> Option<&'static str> {
    XFIG_MENU_LETTERS
        .iter()
        .find(|(m, l, _)| *m == menu && *l == letter)
        .map(|(_, _, c)| *c)
        .filter(|c| cmd::find(c).is_some() || find_ui(c).is_some())
}

/// Recent files (xfig lists them at the end of its File menu, numbered;
/// 1–9 open them while the menu is open).
fn recent_files(app: &mut App, ui: &mut egui::Ui) {
    let files = app.engine.prefs.rustfig.recent_files.clone();
    if files.is_empty() {
        return;
    }
    ui.separator();
    for (i, f) in files.iter().enumerate() {
        let name = std::path::Path::new(f)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(f.as_str());
        let digit = egui::Key::from_name(&(i + 1).to_string());
        let key = digit.is_some_and(|k| ui.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, k)));
        if ui.button(format!("{} {name}", i + 1)).on_hover_text(f).clicked() || key {
            ui.close();
            app.run("app.open", json!({ "path": f }));
        }
    }
}

fn xfig_menu(app: &mut App, ui: &mut egui::Ui, menu: &str, items: &[XfigItem]) {
    // A letter picks its entry, as in xfig.
    for (m, letter, _) in XFIG_MENU_LETTERS.iter().filter(|(m, _, _)| *m == menu) {
        let Some(key) = egui::Key::from_name(&letter.to_ascii_uppercase().to_string()) else {
            continue;
        };
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
            if let Some(id) = xfig_menu_letter(m, *letter) {
                if app.enabled(id) {
                    ui.close();
                    app.run(id, json!({}));
                    return;
                }
            }
        }
    }
    for item in items {
        match *item {
            XfigItem::Sep => {
                ui.separator();
            }
            XfigItem::Cmd(label, id) => {
                let text = match checked(app, id) {
                    Some(true) => format!("✔ {label}"),
                    Some(false) => format!("    {label}"),
                    None => label.to_string(),
                };
                // With xfig's keys on, an entry shows xfig's accelerator or none
                // (xfig's Unzoom has none), never RustFig's own shortcut.
                let shortcut = if app.xfig_keys {
                    xfig_accelerator(id).map(xfig_key_label).unwrap_or_default()
                } else {
                    display_shortcut(label_of(id).1)
                };
                let mut b = egui::Button::new(text);
                if !shortcut.is_empty() {
                    b = b.shortcut_text(shortcut);
                }
                if ui.add_enabled(app.enabled(id), b).clicked() {
                    ui.close();
                    app.run(id, json!({}));
                }
            }
            XfigItem::Missing(label, accel, task) => {
                let mut b = egui::Button::new(label);
                if !accel.is_empty() {
                    b = b.shortcut_text(xfig_key_label(accel));
                }
                let why = if task == "—" {
                    "Not available in RustFig".to_string()
                } else {
                    format!("Not available yet (plan task {task})")
                };
                ui.add_enabled(false, b).on_disabled_hover_text(why);
            }
        }
    }
    if menu == "File" {
        recent_files(app, ui);
    }
}

/// Menu labels that differ from the command labels, using xfig's wording.
const MENU_LABELS: &[(&str, &str)] = &[
    ("app.quit", "Exit"),
    ("edit.paste", "Paste Objects"),
    ("view.zoom_fit", "Zoom to Fit canvas"),
    ("view.toggle_page_border", "Show page borders"),
    ("app.toggle_depths", "Show depth manager"),
    ("view.toggle_grid", "Show grid"),
    ("app.toggle_indicator", "Show attribute panel"),
    ("app.document_settings", "Units and Page…"),
];

/// Check mark state of toggle entries (xfig marks them with an asterisk).
fn checked(app: &App, id: &str) -> Option<bool> {
    match id {
        "view.toggle_page_border" => Some(app.engine.show_page_border),
        "app.toggle_depths" => Some(app.show_depths),
        "app.toggle_balloons" => Some(app.show_balloons),
        "view.toggle_grid" => Some(app.engine.indicator.grid.is_some()),
        "app.toggle_indicator" => Some(app.show_indicator),
        "app.toggle_layout" => Some(app.classic()),
        "app.toggle_xfig_keys" => Some(app.xfig_keys),
        "tool.mouse_buttons" => Some(app.engine.mouse_scheme() == rustfig_engine::MouseScheme::Xfig),
        _ => None,
    }
}

fn label_of(id: &str) -> (&'static str, &'static str) {
    if let Some((_, l)) = MENU_LABELS.iter().find(|(i, _)| *i == id) {
        let shortcut = cmd::find(id)
            .map(|c| c.shortcut)
            .or_else(|| find_ui(id).map(|c| c.shortcut))
            .unwrap_or("");
        return (l, shortcut);
    }
    if let Some(c) = cmd::find(id) {
        return (c.label, c.shortcut);
    }
    if let Some(c) = find_ui(id) {
        return (c.label, c.shortcut);
    }
    ("?", "")
}

fn entry(app: &mut App, ui: &mut egui::Ui, id: &str) {
    let (label, platform) = label_of(id);
    // With xfig's keymap the menus show its accelerators, as xfig does.
    let xfig = app.xfig_keys.then(|| xfig_accelerator(id)).flatten();
    let shortcut = xfig.map(xfig_key_label).unwrap_or_else(|| display_shortcut(platform));
    let text = match checked(app, id) {
        Some(true) => format!("✔ {label}"),
        Some(false) => format!("    {label}"),
        None => label.to_string(),
    };
    let mut b = egui::Button::new(text);
    if !shortcut.is_empty() {
        b = b.shortcut_text(shortcut);
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
    egui::MenuBar::new().ui(ui, |ui| menu_buttons(app, ui));
}

/// One of the classic top strip's menu buttons, bevelled in the 3D style
/// (sunken while its menu is open).
fn bar_button(
    app: &mut App,
    ui: &mut egui::Ui,
    tk: &crate::theme::Tokens,
    name: &str,
    content: impl FnOnce(&mut App, &mut egui::Ui),
) -> egui::Response {
    let ctx = ui.ctx().clone();
    let resp = crate::bevel::around(
        ui,
        tk,
        |r| egui::Popup::is_id_open(&ctx, egui::Popup::default_response_id(r)),
        |ui| {
            ui.scope(|ui| {
                crate::bevel::bare_buttons(ui, tk);
                ui.add(egui::Button::new(name))
            })
            .inner
        },
    );
    egui::Popup::menu(&resp).show(|ui| content(app, ui));
    resp
}

/// The menu buttons alone, laid out in the current row.
pub fn menu_buttons(app: &mut App, ui: &mut egui::Ui) {
    if app.classic() {
        // xfig's menus: File, Edit, View, Snap, Help.
        let tk = crate::theme::Tokens::new(ui.visuals().dark_mode, app.button_3d);
        for (name, items) in XFIG_MENUS {
            if *name == "Help" {
                bar_button(app, ui, &tk, "Snap", snap_menu);
            }
            let resp = bar_button(app, ui, &tk, name, |app, ui| xfig_menu(app, ui, name, items));
            if app.open_menu == Some(*name) {
                app.open_menu = None;
                egui::Popup::close_all(ui.ctx());
                egui::Popup::open_id(ui.ctx(), egui::Popup::default_response_id(&resp));
            }
        }
        return;
    }
    {
        for (name, list) in LAYOUT {
            if *name == "Object" {
                ui.menu_button("Snap", |ui| snap_menu(app, ui));
            }
            ui.menu_button(*name, |ui| {
                entries(app, ui, list);
                if *name == "File" {
                    recent_files(app, ui);
                }
            });
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
    }
}

/// xfig's Snap menu (`snap_menu_items` in `w_cmdpanel.c`): Hold and
/// Release, then the snap modes for the next clicked point. Point
/// positioning and the grid are on the attribute panel, as in xfig.
pub fn snap_menu(app: &mut App, ui: &mut egui::Ui) {
    use rustfig_engine::ObjectSnap as S;
    let held = app.engine.snap_held();
    let hold_label = if held { "\u{2713} Hold" } else { "Hold" };
    if ui
        .add(egui::Button::new(hold_label).selected(held))
        .on_hover_text("Keep the snap mode after each point until Release")
        .clicked()
    {
        app.run("snap.hold", json!({}));
        ui.close();
    }
    if ui
        .add_enabled(held, egui::Button::new("Release"))
        .on_hover_text("Stop holding and disarm the snap mode")
        .clicked()
    {
        app.run("snap.release", json!({}));
        ui.close();
    }
    let current = app.engine.object_snap();
    for group in [
        &[S::Endpoint, S::Midpoint, S::Nearest, S::Focus][..],
        &[S::Normal, S::Tangent, S::Intersect, S::Diameter][..],
    ] {
        ui.separator();
        for m in group {
            let label = if *m == S::Intersect { "Intersection" } else { m.label() };
            if ui.add(egui::Button::new(label).selected(current == Some(*m))).clicked() {
                app.run("snap.mode", json!({ "mode": m.name() }));
                ui.close();
            }
        }
    }
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

/// xfig's keyboard accelerators (`app-defaults/Fig` and the mode tooltips
/// in `w_modepanel.c`): Meta (Alt/Option) + letter for menu entries, and
/// physical Ctrl + letter for some modes. Specs use "Alt", "Shift" and
/// "PhysCtrl" (the Control key itself, also on macOS where the platform's
/// shortcuts use Cmd); xfig entries RustFig has no command for yet (print,
/// spell check, global settings, named styles) are left out.
pub const XFIG_KEYS: &[(&str, &str)] = &[
    ("Alt+N", "app.new"),
    ("Alt+O", "app.open"),
    ("Alt+M", "app.merge"),
    ("Alt+S", "app.save"),
    ("Alt+A", "app.save_as"),
    ("Alt+X", "app.export"),
    ("Shift+Alt+X", "app.export_now"),
    ("Alt+Q", "app.quit"),
    ("Alt+U", "edit.undo"),
    ("Alt+T", "edit.paste"),
    ("Alt+I", "app.search"),
    ("Alt+D", "edit.delete_all"),
    ("PhysCtrl+L", "app.redraw"),
    ("Alt+R", "app.redraw"),
    ("Alt+C", "doc.toggle_orientation"),
    ("PhysCtrl+Z", "view.zoom_fit"),
    ("Alt+B", "view.toggle_page_border"),
    ("Alt+Y", "app.toggle_balloons"),
    ("Alt+G", "app.global_settings"),
    ("Shift+U", "app.unit_panel"),
    ("Shift+Z", "view.zoom_in"),
    ("Z", "view.zoom_out"),
    ("PhysCtrl+S", "tool.scale"),
    ("PhysCtrl+C", "tool.copy"),
    ("PhysCtrl+A", "tool.add_point"),
    ("PhysCtrl+E", "tool.edit"),
    ("PhysCtrl+R", "tool.rotate_cw"),
    ("PhysCtrl+P", "tool.picture"),
];

/// (alt, shift, physical ctrl, key) of an xfig key spec.
fn parse_xfig_key(spec: &str) -> Option<(bool, bool, bool, egui::Key)> {
    let (mut alt, mut shift, mut ctrl, mut key) = (false, false, false, None);
    for part in spec.split('+') {
        match part {
            "Alt" => alt = true,
            "Shift" => shift = true,
            "PhysCtrl" => ctrl = true,
            k => key = egui::Key::from_name(k),
        }
    }
    key.map(|k| (alt, shift, ctrl, k))
}

/// An xfig accelerator as the platform names its keys. xfig prints "Meta-N",
/// "Shift-Meta-X", "Ctrl-L"; its Meta is the Alt key (Option on macOS), so
/// macOS shows "Option+N", "Shift+Option+X", "Ctrl+L" (the key's printed
/// name; the ⌥ symbol is missing from the fallback fonts) and Linux and
/// Windows "Alt+N", "Shift+Alt+X", "Ctrl+L".
pub fn xfig_key_label(spec: &str) -> String {
    key_label(spec, cfg!(target_os = "macos"))
}

fn key_label(spec: &str, mac: bool) -> String {
    if !spec.contains('+') {
        // A bare letter is lower case in xfig's labels: "(z)".
        return spec.to_ascii_lowercase();
    }
    let spec = spec.replace("PhysCtrl", "Ctrl");
    if mac {
        spec.replace("Alt", "Option")
    } else {
        spec
    }
}

/// The xfig accelerator shown for a command in the menus.
pub fn xfig_accelerator(id: &str) -> Option<&'static str> {
    XFIG_KEYS.iter().find(|(_, c)| *c == id).map(|(k, _)| *k)
}

/// Consume one key press matching an xfig spec exactly.
fn take_xfig_key(ctx: &egui::Context, spec: &str) -> bool {
    let Some((alt, shift, ctrl, key)) = parse_xfig_key(spec) else {
        return false;
    };
    ctx.input_mut(|i| {
        let pos = i.events.iter().position(|e| match e {
            egui::Event::Key {
                key: k,
                pressed: true,
                modifiers: m,
                ..
            } => {
                *k == key
                    && m.alt == alt
                    && m.shift == shift
                    && m.ctrl == ctrl
                    && !(cfg!(target_os = "macos") && m.mac_cmd)
            }
            _ => false,
        });
        match pos {
            Some(p) => {
                i.events.remove(p);
                true
            }
            None => false,
        }
    })
}

/// xfig's compose key (`CompKeyDB`, `w_canvas.c`): Meta (Alt) pressed and
/// released on its own while typing arms it; the next two characters make
/// one (`rustfig_text::compose`), or are typed as they are if they make
/// none.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Compose {
    #[default]
    Off,
    /// Alt is down and nothing else was pressed yet.
    AltAlone,
    Armed,
    First(char),
}

impl Compose {
    fn watch_alt(&mut self, alt: bool, events: &[egui::Event]) {
        let other_key = events
            .iter()
            .any(|e| matches!(e, egui::Event::Key { pressed: true, .. } | egui::Event::Text(_)));
        *self = match (*self, alt) {
            (Compose::Off, true) if !other_key => Compose::AltAlone,
            (Compose::AltAlone, true) if other_key => Compose::Off,
            (Compose::AltAlone, false) => Compose::Armed,
            (s, _) => s,
        };
    }
}

/// Typed text after the compose key: what reaches the text tool, if anything.
fn compose_text(app: &mut App, text: String) -> Option<String> {
    let mut out = String::new();
    for c in text.chars() {
        app.compose = match app.compose {
            Compose::Armed => Compose::First(c),
            Compose::First(a) => {
                match rustfig_text::compose(a, c) {
                    Some(x) => out.push(x),
                    None => {
                        out.push(a);
                        out.push(c);
                    }
                }
                Compose::Off
            }
            other => {
                out.push(c);
                other
            }
        };
    }
    (!out.is_empty()).then_some(out)
}

/// xfig's canvas keys (`w_canvas.c`): the arrow keys pan by a ruler step
/// (Shift: five), Home pans to the origin. While text is typed, Left, Right
/// and Home move the caret instead.
fn pan_keys(app: &mut App, ctx: &egui::Context, typing: bool) {
    let units = app.engine.doc().settings.units;
    let zoom = app.engine.view.zoom;
    let s = app.engine.view.px_per_fig();
    // Shift first: a pattern without Shift may match Shift+key too.
    for shift in [true, false] {
        let mods = if shift {
            egui::Modifiers::SHIFT
        } else {
            egui::Modifiers::NONE
        };
        let step = crate::rulers::pan_step(units, zoom, shift) * s;
        // xfig: Left is `pan_right` (the drawing moves right), Up `pan_down`.
        let keys = [
            (egui::Key::ArrowLeft, step, 0.0, false),
            (egui::Key::ArrowRight, -step, 0.0, false),
            (egui::Key::ArrowUp, 0.0, step, true),
            (egui::Key::ArrowDown, 0.0, -step, true),
        ];
        for (key, dx, dy, while_typing) in keys {
            if (!typing || while_typing) && ctx.input_mut(|i| i.consume_key(mods, key)) {
                app.run("view.pan", json!({"dx": dx, "dy": dy}));
            }
        }
    }
    if !typing && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Home)) {
        app.run("view.pan_origin", json!({}));
    }
}

/// Keyboard handling: typing goes to the text tool; otherwise shortcuts run commands.
pub fn handle_shortcuts(app: &mut App, ctx: &egui::Context) {
    if ctx.wants_keyboard_input() {
        return;
    }
    dialogs::escape_closes_panels(app, ctx);
    if app.mode_popup.is_some() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        app.mode_popup = None;
    }
    if app.engine.is_typing() {
        let events = ctx.input(|i| i.events.clone());
        let alt = ctx.input(|i| i.modifiers.alt);
        app.compose.watch_alt(alt, &events);
        for ev in events {
            let tev = match ev {
                egui::Event::Text(t) => compose_text(app, t).map(ToolEvent::Text),
                egui::Event::Paste(t) => Some(ToolEvent::Text(t)),
                egui::Event::Key { key, pressed: true, .. } => match key {
                    egui::Key::Enter => Some(ToolEvent::Key(Key::Enter)),
                    egui::Key::Escape => Some(ToolEvent::Key(Key::Escape)),
                    egui::Key::Backspace => Some(ToolEvent::Key(Key::Backspace)),
                    egui::Key::Delete => Some(ToolEvent::Key(Key::Delete)),
                    egui::Key::ArrowLeft => Some(ToolEvent::Key(Key::Left)),
                    egui::Key::ArrowRight => Some(ToolEvent::Key(Key::Right)),
                    egui::Key::Home => Some(ToolEvent::Key(Key::Home)),
                    egui::Key::End => Some(ToolEvent::Key(Key::End)),
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
        // xfig pans with Up and Down even while text is typed.
        pan_keys(app, ctx, true);
        return;
    }
    app.compose = Compose::Off;
    pan_keys(app, ctx, false);
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
    // Meta-F/E/V/H pop up xfig's menus; while a menu or another popup is
    // open, letters belong to it, not to the shortcuts (xfig's mode keys).
    if app.xfig_keys && app.classic() {
        for (spec, menu) in XFIG_MENU_KEYS {
            if take_xfig_key(ctx, spec) {
                app.open_menu = Some(menu);
            }
        }
    }
    if egui::Popup::is_any_open(ctx) {
        return;
    }
    // xfig's accelerators first: on Windows and Linux they take Ctrl+S,
    // Ctrl+C, … from the platform shortcuts while the xfig keymap is on.
    if app.xfig_keys {
        for (spec, id) in XFIG_KEYS {
            if take_xfig_key(ctx, spec) && app.enabled(id) {
                app.run(id, json!({}));
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
    fn xfig_menus_reach_commands() {
        for (_, items) in XFIG_MENUS {
            for item in *items {
                if let XfigItem::Cmd(_, id) = item {
                    assert!(cmd::find(id).is_some() || find_ui(id).is_some(), "unknown {id}");
                }
            }
        }
    }

    #[test]
    fn xfig_keys_reach_commands_and_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for (spec, id) in XFIG_KEYS {
            assert!(parse_xfig_key(spec).is_some(), "{spec}");
            assert!(
                cmd::find(id).is_some() || find_ui(id).is_some(),
                "{spec} -> unknown {id}"
            );
            assert!(seen.insert(*spec), "{spec} twice");
        }
        assert_eq!(key_label("Shift+Alt+X", false), "Shift+Alt+X");
        assert_eq!(key_label("PhysCtrl+L", false), "Ctrl+L");
        assert_eq!(key_label("Shift+Alt+X", true), "Shift+Option+X");
        assert_eq!(key_label("PhysCtrl+L", true), "Ctrl+L");
        assert_eq!(xfig_key_label("Z"), "z");
        assert_eq!(xfig_accelerator("edit.undo"), Some("Alt+U"));
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
