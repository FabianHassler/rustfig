//! Dialog windows: edit object, export, search/replace, document settings,
//! command list, about, unsaved changes.

use crate::panels::{color_combo, fill_combo, font_combo, line_combo};
use crate::{menus, App};
use rustfig_engine::fig2dev::languages::{language, Kind, LANGUAGES};
use rustfig_engine::Engine;
use rustfig_model::{CapStyle, JoinStyle, Justify, LineKind, ObjectId, ObjectKind, PaperSize, PolyKind};
use serde_json::{json, Value};

/// Attribute editor state for one object.
#[derive(Debug, Clone)]
pub struct EditState {
    pub id: ObjectId,
    pub kind: String,
    pub is_text: bool,
    pub has_style: bool,
    pub has_arrows: bool,
    pub is_arcbox: bool,
    pub line: LineKind,
    pub thickness: i32,
    pub pen: i32,
    pub fill: i32,
    pub area: i32,
    pub depth: i32,
    pub style_val: f64,
    pub cap: CapStyle,
    pub join: JoinStyle,
    pub fwd: bool,
    pub back: bool,
    pub radius: i32,
    pub text: String,
    pub font: i32,
    pub postscript: bool,
    pub size: f64,
    pub justify: Justify,
    pub angle: f64,
    pub comment: String,
}

#[derive(Debug, Clone)]
pub struct ExportState {
    pub open: bool,
    pub format: &'static str,
    pub magnification: f64,
    pub margin: f64,
    pub dpi: f64,
    pub white_background: bool,
    /// Leave out depths that are off (xfig's "active layers only").
    pub active_layers_only: bool,
    /// With active layers only: size the page by the whole figure.
    pub whole_figure_bounds: bool,
    /// Bitmap smoothing (fig2dev's `-S`: 1 none, 2 some, 4 more); RustFig
    /// starts with 4, the closest to the antialiased canvas (xfig: none).
    pub smoothing: Option<u32>,
    /// JPEG quality (fig2dev's `-q`).
    pub jpeg_quality: u32,
}

impl Default for ExportState {
    fn default() -> Self {
        ExportState {
            open: false,
            format: "pdf",
            magnification: 100.0,
            margin: 0.0,
            dpi: 150.0,
            white_background: false,
            active_layers_only: false,
            whole_figure_bounds: false,
            smoothing: Some(4),
            jpeg_quality: 75,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SearchState {
    pub open: bool,
    pub find: String,
    pub replace: String,
    pub case_sensitive: bool,
}

#[derive(Default)]
pub struct Dialogs {
    pub edit: Option<EditState>,
    pub export: ExportState,
    pub search: SearchState,
    pub settings_open: bool,
    pub commands_open: bool,
    pub about_open: bool,
    pub messages_open: bool,
    /// Edit › Global settings (RustFig's preferences).
    pub global_open: bool,
    /// The xfig resources window (E1).
    pub resources_open: bool,
    resource_filter: String,
    /// Values being typed into the resources window, by resource.
    resource_edits: std::collections::BTreeMap<&'static str, String>,
    /// The user colour being added: name and colour.
    new_color: (String, [u8; 3]),
    command_filter: String,
}

impl Dialogs {
    pub fn open_edit(&mut self, engine: &Engine, id: ObjectId) {
        let Some(o) = engine.doc().get(id) else { return };
        let mut st = EditState {
            id,
            kind: o.type_name().to_string(),
            is_text: false,
            has_style: false,
            has_arrows: false,
            is_arcbox: false,
            line: LineKind::Solid,
            thickness: 1,
            pen: 0,
            fill: 7,
            area: -1,
            depth: o.depth(),
            style_val: 4.0,
            cap: CapStyle::Butt,
            join: JoinStyle::Miter,
            fwd: false,
            back: false,
            radius: 7,
            text: String::new(),
            font: 0,
            postscript: true,
            size: 12.0,
            justify: Justify::Left,
            angle: 0.0,
            comment: o.comments.join("\n"),
        };
        if let Some(s) = o.style() {
            st.has_style = true;
            st.line = s.line;
            st.thickness = s.thickness;
            st.pen = s.pen_color.0;
            st.fill = s.fill_color.0;
            st.area = s.area_fill;
            st.style_val = s.style_val;
            st.cap = s.cap;
            st.join = s.join;
        }
        let mut o2 = o.clone();
        if let Some((f, b)) = o2.arrows_mut() {
            st.has_arrows = true;
            st.fwd = f.is_some();
            st.back = b.is_some();
        }
        match &o.kind {
            ObjectKind::Text(t) => {
                st.is_text = true;
                st.text = t.text.clone();
                st.font = t.font;
                st.postscript = t.is_postscript();
                st.size = t.size;
                st.justify = t.justify;
                st.angle = t.angle.to_degrees();
                st.pen = t.color.0;
            }
            ObjectKind::Polyline(p) if p.kind == PolyKind::ArcBox => {
                st.is_arcbox = true;
                st.radius = p.radius;
            }
            _ => {}
        }
        self.edit = Some(st);
    }
}

/// Extensions offered when picking a figure.
pub const FIGURE_EXTS: &[&str] = &["fig", "xfig", "rustfig"];
/// Extensions offered when picking a picture.
pub const PICTURE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff", "ppm", "pgm"];

/// File pickers. The app uses [`NativeDialogs`]; tests substitute a script.
pub trait FileDialogs {
    /// Choose an existing file (`None` when cancelled or unavailable).
    fn pick_file(&self, what: &str, exts: &[&str]) -> Option<String>;
    /// Choose where to save.
    fn save_file(&self, default_name: &str, what: &str, exts: &[&str]) -> Option<String>;
}

/// The operating system's file dialogs (none on the web yet).
pub struct NativeDialogs;

impl FileDialogs for NativeDialogs {
    fn pick_file(&self, what: &str, exts: &[&str]) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            rfd::FileDialog::new()
                .add_filter(what, exts)
                .pick_file()
                .and_then(|p| p.to_str().map(str::to_string))
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (what, exts);
            None
        }
    }

    fn save_file(&self, default_name: &str, what: &str, exts: &[&str]) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            rfd::FileDialog::new()
                .set_file_name(default_name)
                .add_filter(what, exts)
                .save_file()
                .and_then(|p| p.to_str().map(str::to_string))
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (default_name, what, exts);
            None
        }
    }
}

/// Dialogs that answer from a script, in order (for tests).
#[derive(Default)]
pub struct ScriptedDialogs {
    answers: std::cell::RefCell<std::collections::VecDeque<Option<String>>>,
}

impl ScriptedDialogs {
    pub fn new(answers: impl IntoIterator<Item = Option<String>>) -> Self {
        ScriptedDialogs {
            answers: std::cell::RefCell::new(answers.into_iter().collect()),
        }
    }

    fn next(&self) -> Option<String> {
        self.answers.borrow_mut().pop_front().flatten()
    }
}

impl FileDialogs for ScriptedDialogs {
    fn pick_file(&self, _what: &str, _exts: &[&str]) -> Option<String> {
        self.next()
    }
    fn save_file(&self, _default_name: &str, _what: &str, _exts: &[&str]) -> Option<String> {
        self.next()
    }
}

/// xfig's Escape: close the panels (Cancel), when no text field has the
/// keyboard (there Escape just leaves the field). Runs before the shortcuts,
/// so an open panel takes Escape from Edit › Select none.
pub(crate) fn escape_closes_panels(app: &mut App, ctx: &egui::Context) {
    if ctx.wants_keyboard_input() || app.engine.is_typing() || app.engine.tool_busy() {
        return;
    }
    let d = &mut app.dialogs;
    let any = d.export.open
        || d.search.open
        || d.settings_open
        || d.commands_open
        || d.about_open
        || d.global_open
        || d.edit.is_some()
        || d.messages_open
        || app.unit_panel_open;
    if any && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        // The edit popup's Escape is Cancel (`e_edit.c`): nothing applied.
        d.edit = None;
        d.export.open = false;
        d.search.open = false;
        d.settings_open = false;
        d.commands_open = false;
        d.about_open = false;
        d.global_open = false;
        d.messages_open = false;
        app.unit_panel_open = false;
    }
}

/// Drawings a stopped RustFig left unsaved (A6): restore or discard each.
fn recovery_dialog(app: &mut App, ctx: &egui::Context) {
    if app.recoverable.is_empty() {
        return;
    }
    let mut open = true;
    let mut action: Option<(usize, bool)> = None;
    egui::Window::new("Recover unsaved drawings")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label("RustFig stopped without saving these drawings. Restore one to keep working on it, then save it.");
            ui.add_space(4.0);
            egui::Grid::new("recoverable").num_columns(3).show(ui, |ui| {
                for (i, (_, original)) in app.recoverable.iter().enumerate() {
                    let name = original
                        .as_deref()
                        .and_then(|o| std::path::Path::new(o).file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("Untitled");
                    ui.label(name);
                    if ui.button("Restore").clicked() {
                        action = Some((i, true));
                    }
                    if ui.button("Discard").clicked() {
                        action = Some((i, false));
                    }
                    ui.end_row();
                }
            });
        });
    if let Some((i, restore)) = action {
        if let Some((id, _)) = app.recoverable.get(i).cloned() {
            if restore && app.engine.is_modified() {
                app.error("Save or close the current drawing before restoring another");
                return;
            }
            let ok = !restore || app.run("file.recover", json!({ "id": id })).is_some();
            if ok {
                // The restored drawing is this instance's now (its own
                // copy follows); the old files go.
                app.run("file.discard_recovery", json!({ "id": id }));
                app.recoverable.remove(i);
            }
        }
    }
    if !open {
        // Later: the files stay and are offered again at the next start.
        app.recoverable.clear();
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    recovery_dialog(app, ctx);
    resources_dialog(app, ctx);
    edit_dialog(app, ctx);
    export_dialog(app, ctx);
    search_dialog(app, ctx);
    settings_dialog(app, ctx);
    commands_dialog(app, ctx);
    about_dialog(app, ctx);
    messages_dialog(app, ctx);
    global_dialog(app, ctx);
    unsaved_dialog(app, ctx);
}

/// xfig's Global settings panel holds its preferences; RustFig's are its
/// layout, mouse and keyboard choices.
fn global_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.global_open {
        return;
    }
    let mut open = true;
    egui::Window::new("Global settings")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            for (id, label, on) in [
                ("app.toggle_layout", "Classic xfig layout", app.classic()),
                (
                    "tool.mouse_buttons",
                    "xfig mouse buttons (3 buttons)",
                    app.engine.mouse_scheme() == rustfig_engine::MouseScheme::Xfig,
                ),
                ("app.toggle_xfig_keys", "xfig keyboard accelerators", app.xfig_keys),
                ("app.toggle_indicator", "Show attribute panel", app.show_indicator),
                (
                    "app.toggle_vector_icons",
                    "Sharp attribute pictures (off: xfig's bitmaps)",
                    app.vector_icons,
                ),
            ] {
                let mut v = on;
                if ui.checkbox(&mut v, label).changed() {
                    app.run(id, serde_json::json!({}));
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Units and page…").clicked() {
                    app.run("app.document_settings", serde_json::json!({}));
                }
                if ui
                    .button("xfig resources…")
                    .on_hover_text("All of xfig's settings (its X resources and command-line options)")
                    .clicked()
                {
                    app.dialogs.resources_open = true;
                }
            });
            if ui
                .button("Use current attributes as defaults")
                .on_hover_text(
                    "The attribute panel's values become the start values (startfontsize, startlinewidth, …)",
                )
                .clicked()
            {
                app.run("prefs.use_current_attributes", serde_json::json!({}));
            }
            ui.separator();
            user_colors_editor(app, ui);
            ui.separator();
            ui.label(
                egui::RichText::new(format!("Saved in {}", app.settings.location()))
                    .small()
                    .weak(),
            );
        });
    app.dialogs.global_open &= open;
}

/// The colours kept for every drawing (E1): offered in every colour menu.
fn user_colors_editor(app: &mut App, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("Your colours").strong());
    let colors = app.engine.prefs.rustfig.user_colors.clone();
    if colors.is_empty() {
        ui.label(egui::RichText::new("None yet: add colours to have them in every drawing.").weak());
    }
    let mut remove = None;
    egui::Grid::new("user_colors").num_columns(3).show(ui, |ui| {
        for c in &colors {
            let rgb = rustfig_model::Rgb::from_hex(&c.rgb).unwrap_or(rustfig_model::Rgb::BLACK);
            let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 14.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(r, 2.0, egui::Color32::from_rgb(rgb.r, rgb.g, rgb.b));
            ui.label(format!("{}  {}", c.name, c.rgb));
            if ui.small_button("Remove").clicked() {
                remove = Some(c.name.clone());
            }
            ui.end_row();
        }
    });
    if let Some(name) = remove {
        app.run("prefs.remove_color", json!({ "name": name }));
    }
    ui.horizontal(|ui| {
        let (name, rgb) = &mut app.dialogs.new_color;
        ui.color_edit_button_srgb(rgb);
        ui.add(egui::TextEdit::singleline(name).hint_text("name").desired_width(120.0));
        let hex = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
        let ready = !name.trim().is_empty();
        if ui.add_enabled(ready, egui::Button::new("Add colour")).clicked() {
            let p = json!({ "name": name.trim(), "rgb": hex });
            app.dialogs.new_color.0.clear();
            app.run("prefs.add_color", p);
        }
    });
}

/// Every xfig resource with its value, default, option and what RustFig
/// does with it; values can be edited (E1).
fn resources_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.resources_open {
        return;
    }
    let mut open = true;
    let mut commit: Option<(&'static str, Option<String>)> = None;
    let values = app.engine.prefs.xfig.clone();
    egui::Window::new("xfig resources")
        .open(&mut open)
        .default_size([620.0, 460.0])
        .show(ctx, |ui| {
            ui.label("xfig's settings under xfig's names. A value set here is kept for every start; xfig's command-line options override it for one run. Empty means xfig's default.");
            ui.add(egui::TextEdit::singleline(&mut app.dialogs.resource_filter).hint_text("filter"));
            let filter = app.dialogs.resource_filter.to_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("resources").striped(true).num_columns(4).show(ui, |ui| {
                    ui.strong("Resource");
                    ui.strong("Value");
                    ui.strong("Option");
                    ui.strong("In RustFig");
                    ui.end_row();
                    for r in rustfig_engine::prefs::RESOURCES {
                        if !filter.is_empty() && !r.name.to_lowercase().contains(&filter) && !r.option.contains(&filter) {
                            continue;
                        }
                        ui.label(r.name);
                        let current = values
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case(r.name))
                            .map(|(_, v)| match v {
                                toml_str if toml_str.is_str() => toml_str.as_str().unwrap_or_default().to_string(),
                                other => other.to_string(),
                            })
                            .unwrap_or_default();
                        let text = app.dialogs.resource_edits.entry(r.name).or_insert_with(|| current.clone());
                        let resp = ui.add(
                            egui::TextEdit::singleline(text)
                                .hint_text(r.default)
                                .desired_width(140.0),
                        );
                        if resp.lost_focus() && *text != current {
                            let t = text.trim().to_string();
                            commit = Some((r.name, (!t.is_empty()).then_some(t)));
                        }
                        ui.label(egui::RichText::new(r.option).small());
                        let status = match r.status {
                            rustfig_engine::prefs::Status::Applied => "applied".to_string(),
                            rustfig_engine::prefs::Status::Planned(t) => format!("kept; in use with {t}"),
                        };
                        ui.label(egui::RichText::new(status).small());
                        ui.end_row();
                    }
                });
            });
        });
    if let Some((name, value)) = commit {
        let v = match value {
            Some(t) => json!(t),
            None => serde_json::Value::Null,
        };
        if app.run("prefs.set", json!({ "resource": name, "value": v })).is_none() {
            // Refused: show the stored value again.
            app.dialogs.resource_edits.remove(name);
        }
    }
    if !open {
        app.dialogs.resources_open = false;
        app.dialogs.resource_edits.clear();
    }
}

fn edit_dialog(app: &mut App, ctx: &egui::Context) {
    let Some(mut st) = app.dialogs.edit.take() else { return };
    let mut open = true;
    let mut apply = false;
    let mut close = false;
    egui::Window::new(format!("Edit {}", st.kind))
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            egui::Grid::new("edit_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    if st.is_text {
                        ui.label("Text");
                        ui.add(
                            egui::TextEdit::multiline(&mut st.text)
                                .desired_rows(3)
                                .desired_width(240.0),
                        );
                        ui.end_row();
                        ui.label("Font");
                        if let Some((f, ps)) = font_combo(ui, "edit_font", st.font, st.postscript) {
                            st.font = f;
                            st.postscript = ps;
                        }
                        ui.end_row();
                        ui.label("Size");
                        ui.add(egui::DragValue::new(&mut st.size).range(1.0..=500.0).suffix(" pt"));
                        ui.end_row();
                        ui.label("Justify");
                        ui.horizontal(|ui| {
                            for j in [Justify::Left, Justify::Center, Justify::Right] {
                                ui.radio_value(&mut st.justify, j, format!("{j:?}"));
                            }
                        });
                        ui.end_row();
                        ui.label("Angle");
                        ui.add(egui::DragValue::new(&mut st.angle).range(-360.0..=360.0).suffix("°"));
                        ui.end_row();
                        ui.label("Colour");
                        if let Some(c) = color_combo(ui, "edit_tcolor", app.engine.doc(), st.pen) {
                            st.pen = c;
                        }
                        ui.end_row();
                    }
                    if st.has_style {
                        ui.label("Line style");
                        if let Some(k) = line_combo(ui, "edit_line", st.line) {
                            st.line = k;
                        }
                        ui.end_row();
                        ui.label("Thickness");
                        ui.add(egui::DragValue::new(&mut st.thickness).range(0..=100));
                        ui.end_row();
                        if st.line != LineKind::Solid {
                            ui.label("Dash length");
                            ui.add(egui::DragValue::new(&mut st.style_val).range(0.5..=100.0).speed(0.1));
                            ui.end_row();
                        }
                        ui.label("Pen colour");
                        if let Some(c) = color_combo(ui, "edit_pen", app.engine.doc(), st.pen) {
                            st.pen = c;
                        }
                        ui.end_row();
                        ui.label("Fill colour");
                        if let Some(c) = color_combo(ui, "edit_fill", app.engine.doc(), st.fill) {
                            st.fill = c;
                        }
                        ui.end_row();
                        ui.label("Fill");
                        if let Some(f) = fill_combo(ui, "edit_area", st.area) {
                            st.area = f;
                        }
                        ui.end_row();
                        ui.label("Cap / join");
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("edit_cap")
                                .selected_text(format!("{:?}", st.cap))
                                .show_ui(ui, |ui| {
                                    for c in [CapStyle::Butt, CapStyle::Round, CapStyle::Projecting] {
                                        ui.selectable_value(&mut st.cap, c, format!("{c:?}"));
                                    }
                                });
                            egui::ComboBox::from_id_salt("edit_join")
                                .selected_text(format!("{:?}", st.join))
                                .show_ui(ui, |ui| {
                                    for j in [JoinStyle::Miter, JoinStyle::Round, JoinStyle::Bevel] {
                                        ui.selectable_value(&mut st.join, j, format!("{j:?}"));
                                    }
                                });
                        });
                        ui.end_row();
                    }
                    if st.has_arrows {
                        ui.label("Arrows");
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut st.fwd, "forward");
                            ui.checkbox(&mut st.back, "backward");
                        });
                        ui.end_row();
                    }
                    if st.is_arcbox {
                        ui.label("Corner radius");
                        ui.add(egui::DragValue::new(&mut st.radius).range(0..=500));
                        ui.end_row();
                    }
                    ui.label("Depth");
                    ui.add(egui::DragValue::new(&mut st.depth).range(0..=999));
                    ui.end_row();
                    ui.label("Comment");
                    ui.add(
                        egui::TextEdit::multiline(&mut st.comment)
                            .desired_rows(2)
                            .desired_width(240.0),
                    );
                    ui.end_row();
                });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Apply").clicked() {
                    apply = true;
                }
                if ui.button("Done").clicked() {
                    apply = true;
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
    // xfig's edit popup (`e_edit.c`): Return applies, Ctrl+Return is Done.
    // A single-line field gives up the focus on Return; a multi-line one
    // keeps it, and Return there is a new line.
    let (enter, command) = ctx.input(|i| {
        i.events
            .iter()
            .find_map(|e| match e {
                egui::Event::Key {
                    key: egui::Key::Enter,
                    pressed: true,
                    modifiers,
                    ..
                } => Some((true, modifiers.command)),
                _ => None,
            })
            .unwrap_or((false, false))
    });
    if enter && ctx.memory(|m| m.focused()).is_none() && !app.engine.is_typing() {
        apply = true;
        close |= command;
    }
    if apply {
        let mut p = json!({
            "ids": [st.id.0],
            "depth": st.depth,
            "comment": st.comment,
        });
        if st.is_text {
            p["text"] = json!(st.text);
            p["font"] = json!(st.font);
            p["postscript"] = json!(st.postscript);
            p["font_size"] = json!(st.size);
            p["justify"] = json!(match st.justify {
                Justify::Left => "left",
                Justify::Center => "center",
                Justify::Right => "right",
            });
            p["angle"] = json!(st.angle);
            p["pen_color"] = json!(st.pen);
        }
        if st.has_style {
            p["line"] = json!(st.line.name());
            p["thickness"] = json!(st.thickness);
            p["style_val"] = json!(st.style_val);
            p["pen_color"] = json!(st.pen);
            p["fill_color"] = json!(st.fill);
            p["area_fill"] = json!(st.area);
            p["cap"] = json!(format!("{:?}", st.cap).to_lowercase());
            p["join"] = json!(format!("{:?}", st.join).to_lowercase());
        }
        if st.has_arrows {
            p["forward_arrow"] = json!(st.fwd);
            p["backward_arrow"] = json!(st.back);
        }
        if st.is_arcbox {
            p["radius"] = json!(st.radius);
        }
        app.run("object.set_attributes", p);
    }
    if open && !close && app.engine.doc().get(st.id).is_some() {
        app.dialogs.edit = Some(st);
    }
}

fn export_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.export.open {
        return;
    }
    let mut open = true;
    let mut go = false;
    let some_hidden = !app.engine.hidden_depths.is_empty();
    let st = &mut app.dialogs.export;
    egui::Window::new("Export")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            egui::Grid::new("export_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Language");
                    let current = language(st.format).map_or(st.format, |l| l.description);
                    let combo = egui::ComboBox::from_id_salt("export_language")
                        .selected_text(current)
                        .width(280.0)
                        .show_ui(ui, |ui| {
                            for l in LANGUAGES {
                                let available = rustfig_engine::fig2dev::language_available(l);
                                let item = ui
                                    .add_enabled(available, egui::Button::selectable(st.format == l.id, l.description));
                                let item = match l.task() {
                                    Some(t) if !available => item
                                        .on_disabled_hover_text(format!("Being ported from fig2dev (plan task {t})")),
                                    _ => item,
                                };
                                if item.clicked() {
                                    st.format = l.id;
                                }
                            }
                        });
                    combo
                        .response
                        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, current));
                    ui.end_row();
                    ui.label("Magnification");
                    ui.add(
                        egui::DragValue::new(&mut st.magnification)
                            .range(1.0..=10000.0)
                            .suffix(" %"),
                    );
                    ui.end_row();
                    ui.label("Border");
                    ui.add(
                        egui::DragValue::new(&mut st.margin)
                            .range(0.0..=12000.0)
                            .suffix(" fig units"),
                    );
                    ui.end_row();
                    let bitmap = language(st.format).is_some_and(|l| l.kind == Kind::Bitmap);
                    if bitmap {
                        ui.label("Resolution");
                        ui.add(egui::DragValue::new(&mut st.dpi).range(10.0..=2400.0).suffix(" dpi"));
                        ui.end_row();
                        // xfig's smoothing menu (fig2dev's -S).
                        let choices: [(Option<u32>, &str); 3] = [
                            (Some(1), "No smoothing"),
                            (Some(2), "Some smoothing"),
                            (Some(4), "More smoothing"),
                        ];
                        let current = choices
                            .iter()
                            .find(|(v, _)| *v == st.smoothing)
                            .map_or("No smoothing", |(_, t)| *t);
                        ui.label("Smoothing");
                        let combo = egui::ComboBox::from_id_salt("export_smoothing")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for (v, t) in choices {
                                    ui.selectable_value(&mut st.smoothing, v, t);
                                }
                            });
                        combo
                            .response
                            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, current));
                        ui.end_row();
                    }
                    if st.format == "jpeg" {
                        ui.label("Image quality");
                        ui.add(egui::DragValue::new(&mut st.jpeg_quality).range(1..=100).suffix(" %"));
                        ui.end_row();
                    }
                    ui.label("Background");
                    ui.checkbox(&mut st.white_background, "white (otherwise transparent)");
                    ui.end_row();
                    ui.label("Depths");
                    ui.vertical(|ui| {
                        ui.add_enabled_ui(some_hidden, |ui| {
                            ui.checkbox(&mut st.active_layers_only, "active layers only")
                                .on_disabled_hover_text("Every depth is on");
                            ui.add_enabled_ui(st.active_layers_only, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("bounds:");
                                    ui.radio_value(&mut st.whole_figure_bounds, true, "whole figure");
                                    ui.radio_value(&mut st.whole_figure_bounds, false, "active layers");
                                });
                            });
                        });
                    });
                    ui.end_row();
                });
            ui.separator();
            if ui.button("Export…").clicked() {
                go = true;
            }
        });
    app.dialogs.export.open = open;
    // xfig's Return in the export panel exports (`ExportFile`).
    if open
        && ctx.input(|i| i.key_pressed(egui::Key::Enter))
        && ctx.memory(|m| m.focused()).is_none()
        && !app.engine.is_typing()
    {
        go = true;
    }
    if go {
        let st = app.dialogs.export.clone();
        let stem = app
            .engine
            .path()
            .and_then(|p| std::path::Path::new(p).file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("figure")
            .to_string();
        let ext = language(st.format).map_or(st.format, |l| l.extension);
        if let Some(path) = app.file_dialogs.save_file(&format!("{stem}.{ext}"), ext, &[ext]) {
            let p = export_params(app, &st, &path);
            if app.run("file.export", p).is_some() {
                app.dialogs.export.open = false;
            }
        }
    }
}

/// `file.export` parameters for the export panel's settings.
fn export_params(app: &App, st: &ExportState, path: &str) -> serde_json::Value {
    let mut p = json!({
        "path": path,
        "format": st.format,
        "magnification": st.magnification,
        "margin": st.margin,
        "dpi": st.dpi,
        "active_layers_only": st.active_layers_only && !app.engine.hidden_depths.is_empty(),
        "whole_figure_bounds": st.whole_figure_bounds,
    });
    if st.white_background {
        p["background"] = json!("#ffffff");
    }
    let mut options = serde_json::Map::new();
    if language(st.format).is_some_and(|l| l.kind == Kind::Bitmap) {
        if let Some(s) = st.smoothing {
            options.insert("smoothing".into(), json!(s));
        }
    }
    if st.format == "jpeg" {
        options.insert("quality".into(), json!(st.jpeg_quality));
    }
    if !options.is_empty() {
        p["options"] = Value::Object(options);
    }
    p
}

/// xfig's ExportFile (Shift-Meta-X): export with the panel's current
/// settings to the figure's name with the language's suffix, without the
/// panel; a figure not saved yet opens the panel instead.
pub fn export_now(app: &mut App) {
    let st = app.dialogs.export.clone();
    let ext = language(st.format).map_or(st.format, |l| l.extension);
    let Some(fig) = app.engine.path().map(str::to_string) else {
        app.dialogs.export.open = true;
        return;
    };
    let target = std::path::Path::new(&fig).with_extension(ext);
    let p = export_params(app, &st, &target.to_string_lossy());
    app.run("file.export", p);
}

fn search_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.search.open {
        return;
    }
    let mut open = true;
    let mut action: Option<bool> = None;
    let st = &mut app.dialogs.search;
    egui::Window::new("Search and Replace")
        .open(&mut open)
        .collapsible(false)
        .show(ctx, |ui| {
            egui::Grid::new("search_grid").num_columns(2).show(ui, |ui| {
                // xfig: Return in the search field searches, in the
                // replace field replaces (`w_srchrepl.c`).
                let enter =
                    |r: &egui::Response, ui: &egui::Ui| r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.label("Find");
                let r = ui.text_edit_singleline(&mut st.find);
                if enter(&r, ui) {
                    action = Some(false);
                }
                ui.end_row();
                ui.label("Replace with");
                let r = ui.text_edit_singleline(&mut st.replace);
                if enter(&r, ui) {
                    action = Some(true);
                }
                ui.end_row();
            });
            ui.checkbox(&mut st.case_sensitive, "Match case");
            ui.horizontal(|ui| {
                if ui.button("Find").clicked() {
                    action = Some(false);
                }
                if ui.button("Replace all").clicked() {
                    action = Some(true);
                }
            });
        });
    app.dialogs.search.open = open;
    if let Some(replace) = action {
        let st = app.dialogs.search.clone();
        let mut p = json!({"find": st.find, "case_sensitive": st.case_sensitive});
        if replace {
            p["replace"] = json!(st.replace);
        }
        if let Some(r) = app.run("edit.search_replace", p) {
            let m = r.get("matches").and_then(Value::as_u64).unwrap_or(0);
            let n = r.get("replaced").and_then(Value::as_u64).unwrap_or(0);
            app.info(if replace {
                format!("Replaced {n} occurrences")
            } else {
                format!("{m} matching texts selected")
            });
        }
    }
}

fn settings_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.settings_open {
        return;
    }
    let mut open = true;
    let s = app.engine.doc().settings.clone();
    let mut cmds: Vec<(&str, Value)> = Vec::new();
    egui::Window::new("Document Settings")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            egui::Grid::new("settings_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Units");
                    ui.horizontal(|ui| {
                        if ui.radio(s.units == rustfig_model::Units::Inches, "Inches").clicked() {
                            cmds.push(("doc.set_units", json!({"units": "inches"})));
                        }
                        if ui.radio(s.units == rustfig_model::Units::Metric, "Metric").clicked() {
                            cmds.push(("doc.set_units", json!({"units": "metric"})));
                        }
                    });
                    ui.end_row();
                    ui.label("Paper");
                    egui::ComboBox::from_id_salt("paper")
                        .selected_text(s.paper.name())
                        .show_ui(ui, |ui| {
                            for p in PaperSize::ALL {
                                if ui.selectable_label(p == s.paper, p.name()).clicked() {
                                    cmds.push(("doc.set_paper", json!({"paper": p.name()})));
                                }
                            }
                        });
                    ui.end_row();
                    ui.label("Orientation");
                    if ui.button(format!("{:?} (toggle)", s.orientation)).clicked() {
                        cmds.push(("doc.toggle_orientation", json!({})));
                    }
                    ui.end_row();
                    ui.label("Magnification");
                    let mut m = s.magnification;
                    if ui
                        .add(egui::DragValue::new(&mut m).range(1.0..=10000.0).suffix(" %"))
                        .changed()
                    {
                        cmds.push(("doc.set_magnification", json!({"magnification": m})));
                    }
                    ui.end_row();
                });
            ui.label(format!("{} user colours", app.engine.doc().colors.len()));
        });
    for (c, p) in cmds {
        app.run(c, p);
    }
    app.dialogs.settings_open = open;
}

fn commands_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.commands_open {
        return;
    }
    let mut open = true;
    let filter = &mut app.dialogs.command_filter;
    egui::Window::new("Commands")
        .open(&mut open)
        .default_size([520.0, 420.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter");
                ui.text_edit_singleline(filter);
            });
            ui.separator();
            let f = filter.to_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("cmd_grid").striped(true).num_columns(3).show(ui, |ui| {
                    for c in rustfig_engine::cmd::all() {
                        if !f.is_empty() && !c.id.contains(&f) && !c.label.to_lowercase().contains(&f) {
                            continue;
                        }
                        ui.monospace(c.id);
                        ui.label(c.label);
                        ui.label(menus::display_shortcut(c.shortcut));
                        ui.end_row();
                    }
                    for c in menus::UI_COMMANDS {
                        if !f.is_empty() && !c.id.contains(&f) && !c.label.to_lowercase().contains(&f) {
                            continue;
                        }
                        ui.monospace(c.id);
                        ui.label(c.label);
                        ui.label(menus::display_shortcut(c.shortcut));
                        ui.end_row();
                    }
                });
            });
        });
    app.dialogs.commands_open = open;
}

fn messages_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.messages_open {
        return;
    }
    let mut open = true;
    let tk = crate::theme::Tokens::for_dark(ctx.style().visuals.dark_mode);
    egui::Window::new("Messages")
        .open(&mut open)
        .default_size([460.0, 260.0])
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                if app.messages.is_empty() {
                    ui.label("No messages yet.");
                }
                for m in &app.messages {
                    let c = if m.error { tk.error } else { ui.visuals().text_color() };
                    ui.label(egui::RichText::new(&m.text).color(c));
                }
            });
        });
    // xfig's Return dismisses the message panel (`w_msgpanel.c`).
    let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter)) && ctx.memory(|m| m.focused()).is_none();
    app.dialogs.messages_open = open && !(enter && !app.engine.is_typing());
}

fn about_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.about_open {
        return;
    }
    let mut open = true;
    egui::Window::new("About RustFig")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.heading("RustFig");
            ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
            ui.label("A clean-room, Rust-native vector drawing program in the spirit of xfig.");
            ui.label("Reads and writes Fig 3.2 files; exports PDF, SVG, EPS, PNG and the LaTeX box (more of fig2dev's languages are being ported).");
            ui.label("Licensed MIT OR Apache-2.0.");
        });
    app.dialogs.about_open = open;
}

fn unsaved_dialog(app: &mut App, ctx: &egui::Context) {
    let Some(action) = app.pending.clone() else { return };
    let mut choice = None;
    egui::Modal::new(egui::Id::new("unsaved")).show(ctx, |ui| {
        ui.heading("Save changes?");
        ui.label("The drawing has unsaved changes.");
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                choice = Some(0);
            }
            if ui.button("Discard").clicked() {
                choice = Some(1);
            }
            if ui.button("Cancel").clicked() {
                choice = Some(2);
            }
        });
    });
    match choice {
        Some(0) => {
            app.pending = None;
            app.run("app.save", json!({}));
            if !app.engine.is_modified() {
                app.do_pending(action);
            }
        }
        Some(1) => {
            app.pending = None;
            app.do_pending(action);
        }
        Some(2) => app.pending = None,
        _ => {}
    }
}
