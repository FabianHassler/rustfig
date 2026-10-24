//! Menu bar, keyboard shortcuts and the UI-only commands.
//
// Ported from xfig src/w_cmdpanel.c (menu entries, order and dividers),
// app-defaults/Fig (`Fig*accelerators`) and src/w_modepanel.c (mode keys)
// (xfig licence, see LICENSE-xfig).

#[cfg(not(target_arch = "wasm32"))]
use crate::spell_dialog;
use crate::{dialogs, reload, App, Pending};
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
        id: "app.colors",
        label: "Colours…",
        shortcut: "",
        params: "{\"color\"?: colour number}  — the colour picker and editor (hue/saturation square, RGB/HSV, hex and X11 names, recent and user colours)",
    },
    UiCommand {
        id: "app.styles",
        label: "Manage Styles…",
        shortcut: "",
        params: "{}  — the Styles window: xfig's style manager: families of named attribute sets (Family and Style lists with Add/Delete, Set applies to the indicator; see style.*)",
    },
    UiCommand {
        id: "app.character_map",
        label: "Character map…",
        shortcut: "Ctrl+Shift+M",
        params: "{}  — the Character Map window (E5): the characters of the text font (Latin, Greek, arrows, maths; the Symbol and ZapfDingbats tables for those fonts); clicking one inserts it at the caret of the text being typed (script: text.insert_char)",
    },
    UiCommand {
        id: "app.keys",
        label: "Keys…",
        shortcut: "",
        params: "{}  — the shortcut sheet (E8): every command with its keys in the standard and in xfig's scheme, per system, searchable",
    },
    UiCommand {
        id: "help.command_search",
        label: "Command search…",
        shortcut: "Ctrl+Shift+K",
        params: "{}  — W18: a small window to find any command or menu entry by label, id or menu path; shows its keys, greys out the ones not available now, Enter runs it (Ctrl+K is xfig's Enter point)",
    },
    UiCommand {
        id: "app.mouse_sheet",
        label: "Mouse buttons…",
        shortcut: "",
        params: "{}  — W18: the mouse cheat sheet: what the left, middle and right buttons do in every mode (from the mouse-function indicator) and how to press them without a three-button mouse",
    },
    UiCommand {
        id: "app.whats_new",
        label: "What's new…",
        shortcut: "",
        params: "{}  — W18: what changed in this version (shown once after an update)",
    },
    UiCommand {
        id: "app.versions",
        label: "Saved versions…",
        shortcut: "",
        params: "{}  — the older saved versions of the current file kept in the recovery folder (W10-14), each restorable; script: file.versions, file.restore_version",
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
        id: "app.print",
        label: "Print…",
        shortcut: "Ctrl+P",
        params: "{}  — the Print panel (page setup, preview, printer, batch); print.run and print.quick print without it",
    },
    UiCommand {
        id: "app.export_now",
        label: "Export with current settings",
        shortcut: "",
        params: "{}  — xfig's ExportFile: next to the figure, the export panel's language and settings",
    },
    UiCommand {
        id: "app.cancel_export",
        label: "Cancel the running export",
        shortcut: "",
        params: "{}  — the Export panel's background export (native); its result is discarded",
    },
    UiCommand {
        id: "app.show_export_folder",
        label: "Show the exported file in its folder",
        shortcut: "",
        params: "{}  — opens the folder of the last file the Export panel wrote (native)",
    },
    UiCommand {
        id: "app.quit",
        label: "Quit",
        shortcut: "Ctrl+Q",
        params: "{}",
    },
    UiCommand {
        id: "app.search",
        label: "Search/Replace…",
        shortcut: "Ctrl+F",
        params: "{}",
    },
    UiCommand {
        id: "app.spell_check",
        label: "Spell Check…",
        shortcut: "",
        params: "{}  — checks the texts with the spell checker (preference spellcheckcommand) and opens the Misspelled words window; native only",
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
        params: "{\"theme\": \"light\"|\"dark\"|\"system\"}  — colour theme (a choice in Global settings; persists; the paper stays white)",
    },
    UiCommand {
        id: "app.ui_scale",
        label: "UI scale",
        shortcut: "",
        params: "{\"percent\": 75..200}  — size of the interface (a choice in Global settings; persists)",
    },
    UiCommand {
        id: "app.toggle_balloons",
        label: "Show info balloons",
        shortcut: "",
        params: "{}  — xfig's tooltips (Alt+Y, macOS Option+Y)",
    },
    UiCommand {
        id: "app.digitize",
        label: "Digitize...",
        shortcut: "",
        params: "{}  - xfig's Digitize (Alt+Z): save the points of the selected polylines and splines as numbered files (file.digitize)",
    },
    UiCommand {
        id: "app.toggle_autorefresh",
        label: "Autorefresh mode",
        shortcut: "",
        params: "{\"on\"?: bool}  - reload the figure when its file changes on disk (asks first with unsaved edits); native only",
    },
    UiCommand {
        id: "app.new_window",
        label: "New window...",
        shortcut: "",
        params: "{\"path\"?: string}  - xfig's New xfig: start a second RustFig (with the file); native only",
    },
    UiCommand {
        id: "app.keyboard_entry",
        label: "Enter point…",
        shortcut: "Ctrl+K",
        params: "{}  — xfig's keyboard panel: type the next point's coordinates (opens the window; tool.point does the work)",
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
        id: "app.install_tex_gyre",
        label: "Install TeX Gyre fonts…",
        shortcut: "",
        params: "{}  — download CTAN's TeX Gyre fonts (checked by SHA-256) into RustFig's user data folder, in the background; native only",
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
/// Set the theme setting (E9): "system", "light" or "dark".
pub fn set_theme(app: &mut App, theme: &str) -> Result<(), String> {
    let pref = match theme {
        "light" => egui::ThemePreference::Light,
        "dark" => egui::ThemePreference::Dark,
        "system" => egui::ThemePreference::System,
        _ => return Err("theme must be light, dark or system".into()),
    };
    app.theme = theme.to_string();
    app.theme_request = Some(pref);
    Ok(())
}

pub fn run_ui(app: &mut App, id: &str, p: &Value) -> Result<Value, String> {
    let path = p.get("path").and_then(Value::as_str).map(str::to_string);
    match id {
        "app.new" => app.guard_unsaved(Pending::New),
        // The browser asks for the file first, the unsaved changes after (web_io).
        "app.open" if path.is_none() && app.file_dialogs.is_web() => {
            app.file_dialogs
                .pick_web("Figures", dialogs::FIGURE_EXTS, &app.web_inbox);
        }
        "app.open" => app.guard_unsaved(Pending::Open(path)),
        "app.merge" => {
            if let Some(p) = path.or_else(|| app.file_dialogs.pick_file("Figures", dialogs::FIGURE_EXTS)) {
                return app
                    .engine
                    .execute("file.merge", &json!({ "path": p }))
                    .map_err(|e| e.message);
            }
        }
        // The browser saves by download (C2).
        "app.save" | "app.save_as" if path.is_none() && app.file_dialogs.is_web() => crate::web_io::save(app),
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
        "app.print" => app.dialogs.print.open = true,
        "app.export_now" => dialogs::export_now(app),
        "app.cancel_export" => {
            #[cfg(not(target_arch = "wasm32"))]
            crate::export_bg::cancel(app);
        }
        "app.show_export_folder" => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let path = app.last_export.clone().ok_or("No export to show")?;
                crate::export_bg::show_in_folder(&path)?;
            }
            #[cfg(target_arch = "wasm32")]
            return Err("Show in folder is not available on the web".into());
        }
        "app.quit" => app.guard_unsaved(Pending::Quit),
        "app.search" => app.dialogs.search.open = true,
        "app.spell_check" => {
            #[cfg(target_arch = "wasm32")]
            return Err("Spell check is not available on the web".into());
            #[cfg(not(target_arch = "wasm32"))]
            spell_dialog::check(app);
        }
        "app.document_settings" => app.dialogs.settings_open = true,
        "app.unit_panel" => app.unit_panel_open = true,
        "app.keyboard_entry" => app.point_entry.open(),
        "app.global_settings" => app.dialogs.global_open = true,
        "app.versions" => app.dialogs.versions_open = true,
        "app.styles" => app.styles.open = true,
        "app.character_map" => app.charmap.open = true,
        "app.keys" => app.keys_sheet.open = true,
        "help.command_search" => app.command_search.show(),
        "app.mouse_sheet" => app.mouse_sheet.show(),
        "app.whats_new" => app.whats_new_open = true,
        "app.colors" => {
            let current = p
                .get("color")
                .and_then(Value::as_i64)
                .and_then(|c| i32::try_from(c).ok())
                .unwrap_or(app.engine.indicator.pen_color);
            crate::color_dialog::open(app, current);
        }
        "app.toggle_xfig_keys" => {
            app.xfig_keys = p.get("on").and_then(Value::as_bool).unwrap_or(!app.xfig_keys);
        }
        "app.toggle_depths" => app.show_depths = !app.show_depths,
        "app.toggle_balloons" => app.show_balloons = !app.show_balloons,
        "app.digitize" => app.digitize.open = true,
        "app.toggle_autorefresh" => {
            let on = p.get("on").and_then(Value::as_bool).unwrap_or(!app.reload.on);
            app.reload.set(on);
            app.info(if on {
                "Autorefresh mode ON"
            } else {
                "Autorefresh mode OFF"
            });
        }
        "app.new_window" => {
            let file = p.get("path").and_then(Value::as_str);
            reload::new_window(file)?;
        }
        "app.button_style" => {
            app.button_3d = match p.get("style").and_then(Value::as_str) {
                Some("3d") => true,
                Some("flat") => false,
                Some(other) => return Err(format!("unknown button style {other:?} (3d or flat)")),
                None => !app.button_3d,
            };
        }
        "app.theme" => {
            set_theme(app, p.get("theme").and_then(Value::as_str).unwrap_or(""))?;
        }
        "app.ui_scale" => {
            // Percent (75 to 200) or a factor (0.75 to 2.0).
            let v = p
                .get("percent")
                .and_then(Value::as_f64)
                .map(|x| x / 100.0)
                .or_else(|| p.get("scale").and_then(Value::as_f64))
                .ok_or("ui_scale needs percent (75 to 200) or scale (0.75 to 2.0)")?;
            if !v.is_finite() || !(0.75..=2.0).contains(&v) {
                return Err("the UI scale is 75 % to 200 %".into());
            }
            app.ui_scale = crate::settings::clamp_scale(v as f32);
        }
        "app.redraw" => app.canvas.invalidate(),
        "app.commands" => app.dialogs.commands_open = true,
        "app.about" => app.dialogs.about_open = true,
        "app.install_tex_gyre" => {
            #[cfg(not(target_arch = "wasm32"))]
            crate::tex_gyre::start(app)?;
            #[cfg(target_arch = "wasm32")]
            return Err("Installing fonts is not available on the web".into());
        }
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
    use XfigItem::{Cmd, Own, Sep};
    &[
        (
            "File",
            &[
                Cmd("New", "app.new"),
                Cmd("Open...", "app.open"),
                Cmd("Merge...", "app.merge"),
                Cmd("Digitize...", "app.digitize"),
                Cmd("Save", "app.save"),
                Cmd("Save As...", "app.save_as"),
                Cmd("Export...", "app.export"),
                Own("app.import_picture"),
                Own("app.new_window"),
                Own("app.versions"),
                Cmd("Print...", "app.print"),
                Cmd("Exit", "app.quit"),
            ],
        ),
        (
            "Edit",
            &[
                Cmd("Undo", "edit.undo"),
                Cmd("Paste Objects", "edit.paste"),
                Cmd("Paste Text", "text.paste"),
                Cmd("Search/Replace...", "app.search"),
                #[cfg(not(target_arch = "wasm32"))]
                Cmd("Spell Check...", "app.spell_check"),
                #[cfg(target_arch = "wasm32")]
                XfigItem::Missing("Spell Check...", "Alt+K", "web"),
                Cmd("Delete All", "edit.delete_all"),
                Sep,
                Cmd("Global settings...", "app.global_settings"),
                Cmd("Colours...", "app.colors"),
                Cmd("Set units...", "app.unit_panel"),
                Own("app.document_settings"),
                Own("app.keyboard_entry"),
                Sep,
                Own("edit.redo"),
                Own("edit.copy_picture_pick"),
                Own("plot.panel"),
                Own("plot.edit"),
            ],
        ),
        (
            "View",
            &[
                Cmd("Manage Styles...", "app.styles"),
                Cmd("Redraw", "app.redraw"),
                Cmd("Portrait/Landscape", "doc.toggle_orientation"),
                Cmd("Zoom In", "view.zoom_in"),
                Cmd("Zoom Out", "view.zoom_out"),
                Cmd("Zoom to Fit canvas", "view.zoom_fit"),
                Cmd("Unzoom", "view.zoom_actual"),
                Cmd("Pan to origin", "view.pan_origin"),
                Cmd("Character map...", "app.character_map"),
                Sep,
                Cmd("Show page borders", "view.toggle_page_border"),
                Cmd("Typeset LaTeX text", "view.toggle_latex_preview"),
                Cmd("Show depth manager", "app.toggle_depths"),
                Cmd("Show info balloons", "app.toggle_balloons"),
                Cmd("Show line lengths", "view.toggle_lengths"),
                Cmd("Show vertex numbers", "view.toggle_vertex_numbers"),
                Cmd("Autorefresh mode", "app.toggle_autorefresh"),
                Own("view.toggle_grid"),
                Own("depth.show_all"),
            ],
        ),
        (
            "Help",
            &[
                Cmd("RustFig Manual...", "help.open"),
                Cmd("RustFig Reference (commands)...", "app.commands"),
                Cmd("Keys...", "app.keys"),
                Own("help.command_search"),
                Own("app.mouse_sheet"),
                Own("app.whats_new"),
                Cmd("Install TeX Gyre fonts...", "app.install_tex_gyre"),
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
    ("File", 'd', "app.digitize"),
    ("File", 's', "app.save"),
    ("File", 'a', "app.save_as"),
    ("File", 'e', "app.export"),
    ("File", 'p', "app.print"),
    ("File", 'x', "app.quit"),
    ("Edit", 'u', "edit.undo"),
    ("Edit", 'p', "edit.paste"),
    ("Edit", 't', "text.paste"),
    ("Edit", 'f', "app.search"),
    #[cfg(not(target_arch = "wasm32"))]
    ("Edit", 's', "app.spell_check"),
    #[cfg(target_arch = "wasm32")]
    ("Edit", 's', "Spell Check..."),
    ("Edit", 'd', "edit.delete_all"),
    ("Edit", 'g', "app.global_settings"),
    ("Edit", 'n', "app.unit_panel"),
    ("View", 's', "app.styles"),
    ("View", 'r', "app.redraw"),
    ("View", 't', "doc.toggle_orientation"),
    ("View", 'i', "view.zoom_in"),
    ("View", 'o', "view.zoom_out"),
    ("View", 'f', "view.zoom_fit"),
    ("View", 'u', "view.zoom_actual"),
    ("View", 'p', "view.pan_origin"),
    ("View", 'c', "app.character_map"),
    ("View", 'd', "app.toggle_depths"),
    ("View", 'n', "app.toggle_balloons"),
    ("View", 'b', "view.toggle_page_border"),
    ("View", 'x', "view.toggle_latex_preview"),
    ("View", 'l', "view.toggle_lengths"),
    ("View", 'v', "view.toggle_vertex_numbers"),
    ("View", 'a', "app.toggle_autorefresh"),
    ("Help", 'x', "help.open"),
    ("Help", 'c', "app.commands"),
    ("Help", 'k', "app.keys"),
    ("Help", 'a', "app.about"),
];

/// The menus Meta (Alt) + letter pops up (`Fig*commands*File*accelerators` …).
pub(crate) const XFIG_MENU_KEYS: &[(&str, &str)] = &[
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
                let why = if task == "web" {
                    "Not available on the web".to_string()
                } else if task == "—" {
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
    ("view.toggle_lengths", "Show line lengths"),
    ("view.toggle_vertex_numbers", "Show vertex numbers"),
    ("app.document_settings", "Units and Page…"),
];

/// Check mark state of toggle entries (xfig marks them with an asterisk).
fn checked(app: &App, id: &str) -> Option<bool> {
    match id {
        "view.toggle_page_border" => Some(app.engine.show_page_border),
        "view.toggle_latex_preview" => Some(app.engine.prefs.rustfig.latex_preview.unwrap_or(true)),
        "app.toggle_depths" => Some(app.show_depths),
        "app.toggle_balloons" => Some(app.show_balloons),
        "app.toggle_autorefresh" => Some(app.reload.on),
        "view.toggle_grid" => Some(app.engine.indicator.grid.is_some()),
        "view.toggle_lengths" => Some(app.engine.show_lengths),
        "view.toggle_vertex_numbers" => Some(app.engine.show_vertex_numbers()),
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

/// The one key a menu entry shows (both keys work; Help > Keys lists both):
/// xfig's accelerator while xfig's keys are on, else the platform's
/// shortcut where xfig's Ctrl keys leave it free (the user's choice, W20).
pub(crate) fn entry_keys(id: &str, xfig_keys: bool, mac: bool) -> String {
    // Paste Text: xfig's F18/F20 (Sun Paste keys) are gone from keyboards;
    // while typing, the platform's paste key inserts the clipboard's text.
    if id == "text.paste" {
        return if mac {
            mac_shortcut("Ctrl+V")
        } else {
            "Ctrl+V".to_string()
        };
    }
    let (platform, xfig) = entry_key_pair(id, mac);
    let platform_shadowed = xfig_keys && shadowed_by_xfig(label_of(id).1, mac);
    // Mode letters are always on; xfig's other keys follow its switch.
    if !xfig.is_empty() && (xfig_keys || is_mode_letter(label_of(id).1)) {
        return xfig;
    }
    if platform_shadowed {
        return String::new();
    }
    platform
}

/// A plain letter shortcut (`C`, `Shift+E`): xfig's mode key, not a system key.
fn is_mode_letter(spec: &str) -> bool {
    !spec.contains("Ctrl")
        && !spec.contains("Alt")
        && spec
            .rsplit('+')
            .next()
            .is_some_and(|k| k.len() == 1 && k.chars().all(|c| c.is_ascii_alphabetic()))
}

/// (platform's shortcut, xfig's accelerator) of a command as written for the
/// system; empty where there is none.
pub(crate) fn entry_key_pair(id: &str, mac: bool) -> (String, String) {
    let platform = label_of(id).1;
    let platform = if platform.is_empty() {
        String::new()
    } else {
        shortcut_text(platform, mac)
    };
    let mut xfig = xfig_accelerator(id).map(|k| key_label(k, mac)).unwrap_or_default();
    // xfig's mode letters belong to xfig's column; the system column shows a
    // key only where it differs from xfig's.
    let platform = if is_mode_letter(label_of(id).1) {
        if xfig.is_empty() {
            xfig = platform;
        }
        String::new()
    } else {
        platform
    };
    let platform = if platform == xfig { String::new() } else { platform };
    (platform, xfig)
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

/// Thin space between the modifier symbols and the key on macOS.
pub(crate) const THIN: char = '\u{2009}';

/// Modifiers and key as the system writes them. macOS: the symbols
/// ⌃ ⌥ ⇧ ⌘ (in Apple's order), a thin space, the key ("⇧⌘ Z"); Windows and
/// Linux: "Ctrl+Alt+Shift+Key". `ctrl` is the Control key itself, `cmd` the
/// platform's command key (Ctrl on Windows and Linux).
fn write_keys(mac: bool, ctrl: bool, alt: bool, shift: bool, cmd: bool, key: &str) -> String {
    if mac {
        let mut out = String::new();
        for (on, sym) in [(ctrl, '⌃'), (alt, '⌥'), (shift, '⇧'), (cmd, '⌘')] {
            if on {
                out.push(sym);
            }
        }
        if !out.is_empty() {
            out.push(THIN);
        }
        out + key
    } else {
        let mut parts = Vec::new();
        if ctrl || cmd {
            parts.push("Ctrl");
        }
        if alt {
            parts.push("Alt");
        }
        if shift {
            parts.push("Shift");
        }
        parts.push(key);
        parts.join("+")
    }
}

/// A shortcut spec ("Ctrl+Shift+Z", Ctrl = the platform's command key) as
/// macOS writes it.
fn mac_shortcut(s: &str) -> String {
    shortcut_text(s, true)
}

fn shortcut_text(s: &str, mac: bool) -> String {
    let (mut cmd, mut alt, mut shift, mut key) = (false, false, false, String::new());
    for part in s.split('+') {
        match part {
            "Ctrl" => cmd = true,
            "Shift" => shift = true,
            "Alt" => alt = true,
            // "Ctrl++" (a plus key) splits into empty parts.
            "" => key = "+".to_string(),
            k => key = k.to_string(),
        }
    }
    write_keys(mac, false, alt, shift, cmd, &key)
}

/// Human-readable shortcut ("Ctrl" shows as ⌘ on macOS).
pub fn display_shortcut(s: &str) -> String {
    shortcut_text(s, cfg!(target_os = "macos"))
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
/// shortcuts use Cmd); xfig entries RustFig has no command for yet (
/// spell check, global settings, named styles) are left out.
pub const XFIG_KEYS: &[(&str, &str)] = &[
    ("Alt+N", "app.new"),
    ("Alt+O", "app.open"),
    ("Alt+M", "app.merge"),
    ("Alt+Z", "app.digitize"),
    ("Alt+S", "app.save"),
    ("Alt+A", "app.save_as"),
    ("Alt+X", "app.export"),
    ("Shift+Alt+X", "app.export_now"),
    ("Alt+P", "app.print"),
    ("Shift+Alt+P", "print.quick"),
    ("Alt+Q", "app.quit"),
    ("Alt+U", "edit.undo"),
    ("Alt+T", "edit.paste"),
    ("Alt+I", "app.search"),
    ("Alt+K", "app.spell_check"),
    ("Alt+D", "edit.delete_all"),
    ("PhysCtrl+L", "app.redraw"),
    ("Alt+R", "app.redraw"),
    ("Alt+C", "doc.toggle_orientation"),
    ("PhysCtrl+Z", "view.zoom_fit"),
    ("Alt+B", "view.toggle_page_border"),
    ("Alt+Y", "app.toggle_balloons"),
    ("Alt+L", "view.toggle_lengths"),
    ("PhysCtrl+K", "app.keyboard_entry"),
    ("PhysCtrl+Y", "app.styles"),
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
/// macOS shows "⌥ N", "⇧⌥ X", "⌃ L" (symbols, see `write_keys`) and Linux and
/// Windows "Alt+N", "Shift+Alt+X", "Ctrl+L".
/// name; the ⌥ symbol is missing from the fallback fonts) and Linux and
/// Windows "Alt+N", "Shift+Alt+X", "Ctrl+L".
pub fn xfig_key_label(spec: &str) -> String {
    key_label(spec, cfg!(target_os = "macos"))
}

pub(crate) fn key_label_for(spec: &str, mac: bool) -> String {
    key_label(spec, mac)
}

fn key_label(spec: &str, mac: bool) -> String {
    if !spec.contains('+') {
        // A bare letter is lower case in xfig's labels: "(z)".
        return spec.to_ascii_lowercase();
    }
    let (mut ctrl, mut alt, mut shift, mut key) = (false, false, false, String::new());
    for part in spec.split('+') {
        match part {
            "PhysCtrl" => ctrl = true,
            "Alt" => alt = true,
            "Shift" => shift = true,
            k => key = k.to_string(),
        }
    }
    write_keys(mac, ctrl, alt, shift, false, &key)
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

/// A key while text is typed (E10): Shift extends the selection, Alt (macOS)
/// or Ctrl (Linux, Windows) jumps by words, Cmd+Left/Right is Home/End on
/// macOS, Cmd/Ctrl+A selects all.
fn text_key(os: crate::platform::Os, key: egui::Key, m: egui::Modifiers) -> Option<ToolEvent> {
    let mac = os == crate::platform::Os::Mac;
    let word = if mac { m.alt } else { m.ctrl };
    let line = mac && m.mac_cmd;
    let nav = |key, shift, word| Some(ToolEvent::Nav { key, shift, word });
    match key {
        egui::Key::Enter => Some(ToolEvent::Key(Key::Enter)),
        egui::Key::Escape => Some(ToolEvent::Key(Key::Escape)),
        egui::Key::Backspace => nav(Key::Backspace, false, word),
        egui::Key::Delete => nav(Key::Delete, false, word),
        egui::Key::ArrowLeft if line => nav(Key::Home, m.shift, false),
        egui::Key::ArrowRight if line => nav(Key::End, m.shift, false),
        egui::Key::ArrowLeft => nav(Key::Left, m.shift, word),
        egui::Key::ArrowRight => nav(Key::Right, m.shift, word),
        egui::Key::Home => nav(Key::Home, m.shift, false),
        egui::Key::End => nav(Key::End, m.shift, false),
        egui::Key::A if m.command => nav(Key::SelectAll, false, false),
        _ => None,
    }
}

/// Keyboard handling: typing goes to the text tool; otherwise shortcuts run commands.
pub fn handle_shortcuts(app: &mut App, ctx: &egui::Context) {
    // Escape closes the command search first: egui drops the field's focus
    // on Escape before this runs, so it would reach `tool.escape` instead.
    if app.command_search.open && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        app.command_search.open = false;
        return;
    }
    if ctx.wants_keyboard_input() {
        return;
    }
    dialogs::escape_closes_panels(app, ctx);
    if app.mode_popup.is_some() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        app.mode_popup = None;
    }
    if app.engine.is_typing() {
        // The character map's key works while typing too.
        if let Some(k) = find_ui("app.character_map").and_then(|c| parse_shortcut(c.shortcut)) {
            if ctx.input_mut(|i| i.consume_shortcut(&k)) {
                app.run("app.character_map", json!({}));
            }
        }
        let events = ctx.input(|i| i.events.clone());
        let os = crate::platform::Os::current(ctx);
        let alt = ctx.input(|i| i.modifiers.alt);
        app.compose.watch_alt(alt, &events);
        for ev in events {
            let tev = match ev {
                egui::Event::Text(t) => compose_text(app, t).map(ToolEvent::Text),
                egui::Event::Paste(t) => Some(ToolEvent::Text(t)),
                // E10: the selection goes to the system clipboard.
                egui::Event::Copy => {
                    if let Some(sel) = app.engine.text_selection() {
                        ctx.copy_text(sel);
                    }
                    None
                }
                egui::Event::Cut => {
                    if let Some(sel) = app.engine.text_cut() {
                        ctx.copy_text(sel);
                    }
                    None
                }
                // IME: the composition is shown at the caret, the commit is
                // typed text.
                egui::Event::Ime(egui::ImeEvent::Preedit(t)) => Some(ToolEvent::Preedit(t)),
                egui::Event::Ime(egui::ImeEvent::Commit(t)) => compose_text(app, t).map(ToolEvent::Text),
                egui::Event::Ime(egui::ImeEvent::Disabled) => Some(ToolEvent::Preedit(String::new())),
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => text_key(os, key, modifiers),
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
    // A library object in hand: xfig's r, l, h, v, s, S turn, flip, shrink
    // and enlarge it (`transform_lib_obj`), before they are mode keys.
    if app.engine.library_in_hand().is_some() {
        let keys = [
            (egui::Modifiers::NONE, egui::Key::R, "r"),
            (egui::Modifiers::NONE, egui::Key::L, "l"),
            (egui::Modifiers::NONE, egui::Key::H, "h"),
            (egui::Modifiers::NONE, egui::Key::V, "v"),
            (egui::Modifiers::NONE, egui::Key::S, "s"),
            (egui::Modifiers::SHIFT, egui::Key::S, "S"),
        ];
        for (m, k, c) in keys {
            if ctx.input_mut(|i| i.consume_key(m, k)) {
                if let Err(e) = app.engine.tool_event(ToolEvent::Text(c.to_string())) {
                    app.error(e.message);
                }
            }
        }
    }
    // Keys a busy tool wants (finish, remove last point). Escape is the
    // `tool.escape` shortcut: cancel, clear, leave the mode, one per press.
    if app.engine.tool_busy() {
        let keys = [(egui::Key::Enter, Key::Enter), (egui::Key::Backspace, Key::Backspace)];
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
    let mut ran = Vec::new();
    for (id, k) in all {
        if ctx.input_mut(|i| i.consume_shortcut(&k)) && app.enabled(id) {
            app.run(id, json!({}));
            ran.push(id);
        }
    }
    // The platform's paste key (Cmd+V on macOS, Ctrl+V elsewhere) reaches
    // egui as a clipboard event, not as a key press.
    let clip: Vec<&'static str> = ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                // xfig has no selection to cut or copy; the platform's
                // paste key pastes the cut buffer (Edit > Paste Objects).
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
        // xfig has no selection (decided 2026-10-22): these work on the
        // given ids for scripts and MCP; the modes do the same by mouse.
        (
            "edit.cut",
            "selection-based; scripting API (Delete mode's right button fills the cut buffer)",
        ),
        (
            "edit.copy",
            "selection-based; scripting API (Copy mode's right button; Copy as Picture picks after the command)",
        ),
        ("edit.duplicate", "selection-based; scripting API (Copy mode)"),
        ("edit.delete", "selection-based; scripting API (Delete mode)"),
        ("transform.flip_", "Flip modes; scripting API"),
        ("arrange.", "Align mode and the edit panel's depth; scripting API"),
        ("object.group", "Glue mode; scripting API"),
        ("object.ungroup", "Break mode; scripting API"),
        ("object.update", "Update mode; scripting API"),
        ("object.measure", "the measure modes; scripting API"),
        ("object.convert", "Convert mode; scripting API"),
        ("object.toggle_", "Open/close and Arrow modes; scripting API"),
        ("style.", "View > Manage Styles... (app.styles) is their window"),
        ("text.", "View > Character map... (app.character_map) is their window"),
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
        ("app.theme", "a choice in Global settings"),
        ("app.ui_scale", "a choice in Global settings"),
        ("print.", "the Print panel's buttons; Shift-Meta-P is print.quick"),
        ("app.export_now", "the Export panel's Export button and the E key"),
        ("app.cancel_export", "the Cancel button of the export progress window"),
        ("app.show_export_folder", "the Show in folder button after an export"),
        ("edit.select", "scripting API (xfig has no selection)"),
        ("edit.search_replace", "scripting API; Search/Replace... runs it"),
        ("text.spell_check", "scripting API; Spell Check... runs it"),
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
        ("object.trim", "the Trim tool's left button"),
        ("object.split_at", "the Trim tool's middle button"),
        ("object.extend", "the Trim tool's right button"),
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
        (
            "spline.shape_factor",
            "Ctrl+Shift+click on a spline point (Cmd+Shift on macOS) in the editing modes",
        ),
        ("picture.", "the picture edit panel's buttons; scripting API"),
        ("plot.get", "scripting API; the Plot dialog loads a plot with it"),
        (
            "plot.add",
            "scripting API; the Plot dialog and the Plot mode place plots with it",
        ),
        ("plot.update", "scripting API; the Plot dialog redraws a plot with it"),
        (
            "library.",
            "the library mode button (Shift+L) and the library panel; scripting API",
        ),
        (
            "export.",
            "the Export panel's rows and Fit to Page button; scripting API",
        ),
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
        assert_eq!(key_label("Shift+Alt+X", false), "Alt+Shift+X");
        assert_eq!(key_label("PhysCtrl+L", false), "Ctrl+L");
        assert_eq!(key_label("Shift+Alt+X", true), "⌥⇧\u{2009}X");
        assert_eq!(key_label("PhysCtrl+L", true), "⌃\u{2009}L");
        assert_eq!(xfig_key_label("Z"), "z");
        assert_eq!(xfig_accelerator("edit.undo"), Some("Alt+U"));
    }

    #[test]
    fn menus_show_one_key_xfigs_first() {
        // macOS: Cmd never clashes with xfig's Ctrl and Option keys.
        assert_eq!(entry_keys("edit.undo", true, true), "⌥\u{2009}U");
        assert_eq!(entry_keys("edit.paste", true, true), "⌥\u{2009}T");
        // Linux and Windows: xfig's Ctrl-Z (zoom fit) takes the key.
        assert_eq!(entry_keys("edit.undo", true, false), "Alt+U");
        assert_eq!(entry_keys("edit.paste", true, false), "Alt+T");
        assert_eq!(entry_keys("view.zoom_fit", true, false), "Ctrl+Z");
        // Without xfig's keys, the platform's shortcuts.
        assert_eq!(entry_keys("edit.undo", false, false), "Ctrl+Z");
    }

    #[test]
    fn key_symbols_exist_in_the_fonts() {
        let ctx = egui::Context::default();
        crate::key_font::install(&ctx);
        let _ = ctx.run(egui::RawInput::default(), |_| {});
        let font = egui::FontId::proportional(13.0);
        // Every symbol the menus draw for keys is in the fonts.
        let all: String = cmd::all()
            .iter()
            .map(|c| c.shortcut)
            .chain(UI_COMMANDS.iter().map(|c| c.shortcut))
            .map(|s| mac_shortcut(s) + &display_shortcut(s))
            .chain([THIN.to_string()])
            .collect();
        for c in all.chars() {
            assert!(ctx.fonts_mut(|f| f.has_glyph(&font, c)), "no glyph for {c:?}");
        }
        // And the other symbols the UI writes (degrees, check marks, …).
        for c in "°±²·×—•…↔✔".chars() {
            assert!(ctx.fonts_mut(|f| f.has_glyph(&font, c)), "no glyph for {c:?}");
        }
        assert_eq!(mac_shortcut("Ctrl+Shift+Z"), "⇧⌘\u{2009}Z");
        assert_eq!(mac_shortcut("Ctrl+="), "⌘\u{2009}=");
        assert_eq!(mac_shortcut("Ctrl+'"), "⌘\u{2009}'");
        assert_eq!(mac_shortcut("Ctrl+Alt+K"), "⌥⌘\u{2009}K");
        assert_eq!(shortcut_text("Ctrl+Alt+Shift+'", false), "Ctrl+Alt+Shift+'");
        assert_eq!(shortcut_text("Ctrl+-", false), "Ctrl+-");
        assert_eq!(key_label("PhysCtrl+Alt+K", true), "⌃⌥\u{2009}K");
        assert_eq!(mac_shortcut("Ctrl+Shift+Up"), "⇧⌘\u{2009}Up");
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
