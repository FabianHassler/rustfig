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
        params: "{\"part\"?: bool}  — with a compound open, asks (xfig's Save All / Save Part) unless part is given",
    },
    UiCommand {
        id: "app.save_as",
        label: "Save As…",
        shortcut: "Ctrl+Shift+S",
        params: "{\"path\"?: string, \"part\"?: bool}  — with a compound open, asks unless path or part is given",
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
        id: "app.commands",
        label: "Command List",
        shortcut: "",
        params: "{}",
    },
    UiCommand {
        id: "help.open",
        label: "Manual",
        shortcut: "",
        params: "{\"topic\"?: string}  — open the bundled manual, at a topic (file name such as \"chop.md\", or its title) or at the first page",
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

/// Whether a save asks "all or part" first: with a compound open, unless
/// the caller (a script) already said which with `part` or named the file.
fn asks_scope(app: &App, p: &Value) -> bool {
    app.engine.open_compounds() > 0 && p.get("part").is_none() && p.get("path").is_none()
}

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
        "app.save" | "app.save_as" if asks_scope(app, p) => {
            // xfig asks before saving inside an open compound.
            app.dialogs.save_scope = Some(if id == "app.save" { "app.save" } else { "app.save_as" });
        }
        "app.save" => {
            let part = p.get("part").and_then(Value::as_bool).unwrap_or(false);
            if app.engine.path().is_some() {
                let r = app
                    .engine
                    .execute("file.save", &json!({ "part": part }))
                    .map_err(|e| e.message)?;
                let status = app.engine.status().to_string();
                if status.is_empty() {
                    app.info("Saved");
                } else {
                    app.info(status);
                    app.engine.set_status("");
                }
                return Ok(r);
            }
            return run_ui(app, "app.save_as", p);
        }
        "app.save_as" => {
            let part = p.get("part").and_then(Value::as_bool).unwrap_or(false);
            if let Some(p) = path.or_else(|| app.file_dialogs.save_file("figure.fig", "Figures", &["fig", "rustfig"])) {
                let r = app
                    .engine
                    .execute("file.save", &json!({ "path": p, "part": part }))
                    .map_err(|e| e.message)?;
                let status = app.engine.status().to_string();
                if status.is_empty() {
                    app.info(format!("Saved {p}"));
                } else {
                    app.info(status);
                    app.engine.set_status("");
                }
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
        "app.commands" => app.dialogs.commands_open = true,
        "app.about" => app.dialogs.about_open = true,
        "help.open" => app.dialogs.help.show(p.get("topic").and_then(Value::as_str))?,
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

/// An entry of xfig's menus (`file_menu_items` … in `w_cmdpanel.c`).
#[derive(Clone, Copy)]
pub enum XfigItem {
    /// xfig's label and the RustFig command it runs.
    Cmd(&'static str, &'static str),
    /// An entry RustFig doesn't have yet: label, xfig's accelerator, plan task.
    Missing(&'static str, &'static str, &'static str),
    /// A RustFig command xfig has no entry for, labelled as the command is.
    Own(&'static str),
    /// A submenu.
    Sub(&'static str, &'static [XfigItem]),
    Sep,
}

/// xfig's menus, word for word and in order; RustFig's extra entries
/// (`Own`) follow xfig's own, and the Object menu is RustFig's. The Snap
/// menu is [`snap_menu`] and sits before Help; preferences are under
/// Edit › Global settings.
pub const XFIG_MENUS: &[(&str, &[XfigItem])] = {
    use XfigItem::{Cmd, Missing, Own, Sep, Sub};
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
                Own("app.import_picture"),
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
                Own("app.document_settings"),
                Sep,
                Own("edit.redo"),
                Own("edit.delete"),
                Own("edit.cut"),
                Own("edit.copy"),
                Own("edit.copy_picture"),
                Own("edit.duplicate"),
                Own("edit.select_all"),
                Own("edit.select_none"),
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
                Own("view.toggle_grid"),
                Own("depth.show_all"),
            ],
        ),
        (
            "Object",
            &[
                Own("object.group"),
                Own("object.ungroup"),
                Sep,
                Own("object.update"),
                Own("object.convert"),
                Own("object.toggle_closed"),
                Sub(
                    "Arrows",
                    &[Own("object.toggle_forward_arrow"), Own("object.toggle_backward_arrow")],
                ),
                Sub(
                    "Transform",
                    &[
                        Own("transform.rotate_ccw"),
                        Own("transform.rotate_cw"),
                        Own("transform.flip_horizontal"),
                        Own("transform.flip_vertical"),
                    ],
                ),
                Sub(
                    "Arrange",
                    &[
                        Own("arrange.align"),
                        Own("arrange.distribute"),
                        Sep,
                        Own("arrange.to_front"),
                        Own("arrange.forward"),
                        Own("arrange.backward"),
                        Own("arrange.to_back"),
                    ],
                ),
                Sep,
                Own("object.measure"),
                Own("app.edit_selected"),
            ],
        ),
        (
            "Help",
            &[
                Cmd("RustFig Manual...", "help.open"),
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
    ("Help", 'x', "help.open"),
    ("Help", 'c', "app.commands"),
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
    xfig_items(app, ui, items);
    if menu == "File" {
        recent_files(app, ui);
    }
}

fn xfig_items(app: &mut App, ui: &mut egui::Ui, items: &[XfigItem]) {
    for item in items {
        match *item {
            XfigItem::Sep => {
                ui.separator();
            }
            XfigItem::Sub(name, sub) => {
                ui.menu_button(name, |ui| xfig_items(app, ui, sub));
            }
            XfigItem::Own(id) => menu_entry(app, ui, label_of(id).0, id),
            XfigItem::Cmd(label, id) => menu_entry(app, ui, label, id),
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
}

fn menu_entry(app: &mut App, ui: &mut egui::Ui, label: &str, id: &str) {
    let text = match checked(app, id) {
        Some(true) => format!("✔ {label}"),
        Some(false) => format!("    {label}"),
        None => label.to_string(),
    };
    let shortcut = entry_keys(id, app.xfig_keys, cfg!(target_os = "macos"));
    let mut b = egui::Button::new(text);
    if !shortcut.is_empty() {
        b = b.shortcut_text(shortcut);
    }
    if ui.add_enabled(app.enabled(id), b).clicked() {
        ui.close();
        app.run(id, json!({}));
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
    ("app.document_settings", "Units and Page…"),
];

/// Check mark state of toggle entries (xfig marks them with an asterisk).
fn checked(app: &App, id: &str) -> Option<bool> {
    match id {
        "view.toggle_page_border" => Some(app.engine.show_page_border),
        "app.toggle_depths" => Some(app.show_depths),
        "app.toggle_balloons" => Some(app.show_balloons),
        "view.toggle_grid" => Some(app.engine.indicator.grid.is_some()),
        "app.toggle_xfig_keys" => Some(app.xfig_keys),
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
    // xfig's menus: File, Edit, View, Snap, then RustFig's Object, Help.
    let tk = crate::theme::Tokens::new(ui.visuals().dark_mode, app.button_3d);
    for (name, items) in XFIG_MENUS {
        if *name == "Object" {
            bar_button(app, ui, &tk, "Snap", snap_menu);
        }
        let resp = bar_button(app, ui, &tk, name, |app, ui| xfig_menu(app, ui, name, items));
        if app.open_menu == Some(*name) {
            app.open_menu = None;
            egui::Popup::close_all(ui.ctx());
            egui::Popup::open_id(ui.ctx(), egui::Popup::default_response_id(&resp));
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

/// The key a menu entry shows: with xfig's keys on, xfig's accelerator, or
/// else the platform's shortcut where xfig's keys leave it free (always on
/// macOS, where the platform's shortcuts use Cmd and xfig's Ctrl and Option;
/// on Linux and Windows not where xfig's Ctrl keys take it); otherwise the
/// platform's shortcut.
fn entry_keys(id: &str, xfig_keys: bool, mac: bool) -> String {
    let platform = label_of(id).1;
    let platform = if platform.is_empty() || (xfig_keys && shadowed_by_xfig(platform, mac)) {
        String::new()
    } else if mac {
        mac_shortcut(platform)
    } else {
        platform.to_string()
    };
    let xfig = if xfig_keys {
        xfig_accelerator(id).map(|k| key_label(k, mac)).unwrap_or_default()
    } else {
        String::new()
    };
    // One key per entry: xfig's where it has one.
    if xfig.is_empty() {
        platform
    } else {
        xfig
    }
}

/// Whether one of xfig's Ctrl keys takes the platform shortcut `s` (Linux
/// and Windows, where the platform's Ctrl is xfig's Ctrl too; xfig's keys are
/// handled first).
fn shadowed_by_xfig(s: &str, mac: bool) -> bool {
    let Some(k) = parse_shortcut(s) else { return false };
    !mac && k.modifiers.command
        && XFIG_KEYS
            .iter()
            .filter_map(|(spec, _)| parse_xfig_key(spec))
            .any(|(alt, shift, ctrl, key)| {
                ctrl && key == k.logical_key && shift == k.modifiers.shift && alt == k.modifiers.alt
            })
}

/// A shortcut as macOS writes it, Shift and Option before ⌘ ("Shift+⌘Z").
/// Only ⌘ is drawn as a symbol: the fonts have no ⇧, ⌥ or ⌃ (test
/// `key_symbols_exist_in_the_fonts`), so those are spelled out as in xfig's
/// labels ("Option+U").
fn mac_shortcut(s: &str) -> String {
    let parts: Vec<&str> = s.split('+').collect();
    let has = |m: &str| parts.contains(&m);
    let key = parts
        .iter()
        .rev()
        .find(|p| !matches!(**p, "Ctrl" | "Shift" | "Alt"))
        .copied();
    // "Ctrl++" (a plus key) splits into an empty last part.
    let key = if s.ends_with("++") || s == "+" {
        "+"
    } else {
        key.unwrap_or("")
    };
    let mut out = String::new();
    if has("Shift") {
        out.push_str("Shift+");
    }
    if has("Alt") {
        out.push_str("Option+");
    }
    if has("Ctrl") {
        out.push('⌘');
    }
    out.push_str(key);
    out
}

/// Human-readable shortcut ("Ctrl" shows as ⌘ on macOS).
pub fn display_shortcut(s: &str) -> String {
    if cfg!(target_os = "macos") {
        mac_shortcut(s)
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
    ("PhysCtrl+G", "tool.angle_measure"),
    ("PhysCtrl+N", "tool.length_measure"),
    ("PhysCtrl+M", "tool.area_measure"),
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
    if app.xfig_keys {
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
    let mut ran = Vec::new();
    for (id, k) in all {
        if ctx.input_mut(|i| i.consume_shortcut(&k)) && app.enabled(id) {
            app.run(id, json!({}));
            ran.push(id);
        }
    }
    // The platform's cut, copy and paste keys (Cmd+X/C/V on macOS, Ctrl+X/C/V
    // elsewhere) reach egui as clipboard events, not as key presses. On Linux
    // and Windows xfig's Ctrl-C (Copy mode) keeps the key while xfig's keys
    // are on.
    let mac = cfg!(target_os = "macos");
    let clip: Vec<&'static str> = ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Cut => Some("edit.cut"),
                egui::Event::Copy if mac || !app.xfig_keys => Some("edit.copy"),
                egui::Event::Paste(_) => Some("edit.paste"),
                _ => None,
            })
            .collect()
    });
    for id in clip {
        if !ran.contains(&id) && app.enabled(id) {
            app.run(id, json!({}));
            ran.push(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu_ids(items: &[XfigItem], out: &mut Vec<&'static str>) {
        for item in items {
            match item {
                XfigItem::Cmd(_, id) | XfigItem::Own(id) => out.push(id),
                XfigItem::Sub(_, sub) => menu_ids(sub, out),
                XfigItem::Missing(..) | XfigItem::Sep => {}
            }
        }
    }

    #[test]
    fn xfig_menus_reach_commands() {
        let mut ids = Vec::new();
        for (_, items) in XFIG_MENUS {
            menu_ids(items, &mut ids);
        }
        for id in ids {
            assert!(cmd::find(id).is_some() || find_ui(id).is_some(), "unknown {id}");
        }
    }

    /// Commands that are deliberately in no menu, with the reason.
    const MENU_LESS: &[(&str, &str)] = &[
        // A prefix ending in "." covers every command that starts with it.
        ("tool.", "the mode panel's buttons and the mode keys choose tools"),
        (
            "file.",
            "the File menu's app.* entries wrap them (dialogs, unsaved-changes guard)",
        ),
        ("shape.", "scripting API; the drawing tools make shapes"),
        ("point.", "scripting API; the point tools edit vertices"),
        ("compound.", "the Open compound tool and the Close Compound popup"),
        ("prefs.", "scripting API; Global settings and the resources window"),
        ("indicator.", "scripting API; the attribute panel sets them"),
        ("color.", "scripting API; the colour dialogs"),
        (
            "doc.",
            "scripting API; Units and Page, Set units and the document dialogs",
        ),
        ("depth.", "scripting API; the depth manager"),
        (
            "view.",
            "scripting API (zoom, pan, grid and snap by value); the View menu and the rulers have the rest",
        ),
        ("app.toggle_xfig_keys", "a checkbox in Global settings"),
        ("app.button_style", "a choice in Global settings"),
        ("app.theme", "set by the system; for screenshots and tests"),
        ("app.export_now", "the Export panel's Export button and the E key"),
        ("edit.select", "scripting API; the Select tool picks objects"),
        ("edit.search_replace", "scripting API; Search/Replace... runs it"),
        ("edit.paste_text", "the middle button of the Text tool"),
        (
            "edit.copy_array",
            "the middle button while a copy is placed (Num X / Num Y Copies)",
        ),
        ("arrange.align_canvas", "the middle button of the Align tool"),
        (
            "object.recompute_dimension",
            "scripting API; runs by itself on scaling and closing",
        ),
        ("object.add", "scripting API"),
        (
            "object.set_",
            "scripting API; the Edit panel and the attribute panel set them",
        ),
        ("object.get", "scripting API"),
        ("object.list", "scripting API"),
        ("object.join", "the Join/Split tool"),
        ("object.split", "the Join/Split tool"),
        ("object.chop", "the Chop tool"),
        ("object.trim", "the Chop tool's Shift+left button"),
        ("object.split_at", "the Chop tool's Shift+middle button"),
        ("object.extend", "the Chop tool's Shift+right button"),
        (
            "object.tangent_line",
            "the Tangent tool (left button; middle button for the normal)",
        ),
        (
            "object.measure_angle",
            "the Measure angle tool (three points, or a corner or arc)",
        ),
        ("object.measure_length", "the Measure length tool"),
        ("object.measure_area", "the Measure area tool"),
        ("object.measure_reset", "the measure tools' right button"),
        ("object.cut_all", "scripting API (cut the selection against itself)"),
        ("object.update_region", "the Update tool's right button"),
        ("object.reverse_arrows", "the Arrow tool's right button"),
        ("object.break_all", "the Break tool's right button"),
        ("transform.move", "scripting API; the Move tools"),
        (
            "transform.nudge",
            "scripting API (a small move step); the arrow keys pan the canvas, as xfig",
        ),
        ("transform.scale", "scripting API; the Scale tool"),
        ("transform.rotate", "scripting API; the Rotate tools"),
        (
            "arrange.set_depth",
            "scripting API; the Depth box of the attribute panel",
        ),
        ("arrange.align_to", "the Align tool's right button"),
        ("picture.", "the picture edit panel's buttons; scripting API"),
    ];

    /// Every command is in a menu or listed above with a reason.
    #[test]
    fn every_command_is_reachable_from_a_menu() {
        let mut ids = Vec::new();
        for (_, items) in XFIG_MENUS {
            menu_ids(items, &mut ids);
        }
        // The Snap menu is built in code.
        let mut missing = Vec::new();
        let all = cmd::all().iter().map(|c| c.id).chain(UI_COMMANDS.iter().map(|c| c.id));
        for id in all {
            let exempt = MENU_LESS.iter().any(|(p, _)| id.starts_with(p));
            if !exempt && !ids.contains(&id) && !id.starts_with("snap.") {
                missing.push(id);
            }
        }
        assert!(missing.is_empty(), "in no menu: {missing:#?}");
        // The exemptions are not stale.
        for (p, _) in MENU_LESS {
            assert!(
                cmd::all()
                    .iter()
                    .map(|c| c.id)
                    .chain(UI_COMMANDS.iter().map(|c| c.id))
                    .any(|i| i.starts_with(p)),
                "{p}"
            );
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
    fn menus_show_the_platform_keys_xfig_leaves_free() {
        // macOS: Cmd never clashes with xfig's Ctrl and Option keys.
        assert_eq!(entry_keys("edit.undo", true, true), "Option+U");
        assert_eq!(entry_keys("edit.copy", true, true), "⌘C");
        // Linux and Windows: xfig's Ctrl-Z (zoom fit) and Ctrl-C (Copy mode)
        // take the keys; Ctrl+X and Ctrl+V stay free.
        assert_eq!(entry_keys("edit.undo", true, false), "Alt+U");
        assert_eq!(entry_keys("edit.copy", true, false), "");
        assert_eq!(entry_keys("edit.cut", true, false), "Ctrl+X");
        assert_eq!(entry_keys("edit.paste", true, false), "Alt+T");
        assert_eq!(entry_keys("view.zoom_fit", true, false), "Ctrl+Z");
        // Without xfig's keys, the platform's shortcuts.
        assert_eq!(entry_keys("edit.undo", false, false), "Ctrl+Z");
    }

    #[test]
    fn key_symbols_exist_in_the_fonts() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |_| {});
        let font = egui::FontId::proportional(13.0);
        // Every symbol the menus draw for keys is in the fonts.
        let all: String = cmd::all()
            .iter()
            .map(|c| c.shortcut)
            .chain(UI_COMMANDS.iter().map(|c| c.shortcut))
            .map(|s| mac_shortcut(s) + &display_shortcut(s))
            .collect();
        for c in all.chars() {
            assert!(ctx.fonts_mut(|f| f.has_glyph(&font, c)), "no glyph for {c:?}");
        }
        // And the other symbols the UI writes (degrees, check marks, …).
        for c in "°±²·×—•…↔✔".chars() {
            assert!(ctx.fonts_mut(|f| f.has_glyph(&font, c)), "no glyph for {c:?}");
        }
        assert_eq!(mac_shortcut("Ctrl+Shift+Z"), "Shift+⌘Z");
        assert_eq!(mac_shortcut("Ctrl+="), "⌘=");
        assert_eq!(mac_shortcut("Ctrl+Shift+Up"), "Shift+⌘Up");
        assert_eq!(mac_shortcut("Delete"), "Delete");
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
