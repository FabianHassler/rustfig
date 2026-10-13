//! Dialog windows: edit object, export, search/replace, document settings,
//! command list, about, unsaved changes.

use crate::panels::{color_combo, fill_combo, font_combo, line_combo};
use crate::{menus, App};
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

pub fn show(app: &mut App, ctx: &egui::Context) {
    edit_dialog(app, ctx);
    export_dialog(app, ctx);
    search_dialog(app, ctx);
    settings_dialog(app, ctx);
    commands_dialog(app, ctx);
    about_dialog(app, ctx);
    messages_dialog(app, ctx);
    unsaved_dialog(app, ctx);
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
                    ui.label("Format");
                    ui.horizontal(|ui| {
                        for f in ["pdf", "svg", "eps", "png"] {
                            ui.radio_value(&mut st.format, f, f.to_uppercase());
                        }
                    });
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
                    if st.format == "png" {
                        ui.label("Resolution");
                        ui.add(egui::DragValue::new(&mut st.dpi).range(10.0..=2400.0).suffix(" dpi"));
                        ui.end_row();
                    }
                    ui.label("Background");
                    ui.checkbox(&mut st.white_background, "white (otherwise transparent)");
                    ui.end_row();
                });
            ui.separator();
            if ui.button("Export…").clicked() {
                go = true;
            }
        });
    app.dialogs.export.open = open;
    if go {
        let st = app.dialogs.export.clone();
        let stem = app
            .engine
            .path()
            .and_then(|p| std::path::Path::new(p).file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("figure")
            .to_string();
        if let Some(path) = app
            .file_dialogs
            .save_file(&format!("{stem}.{}", st.format), st.format, &[st.format])
        {
            let mut p = json!({
                "path": path,
                "format": st.format,
                "magnification": st.magnification,
                "margin": st.margin,
                "dpi": st.dpi,
            });
            if st.white_background {
                p["background"] = json!("#ffffff");
            }
            if app.run("file.export", p).is_some() {
                app.dialogs.export.open = false;
            }
        }
    }
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
                ui.label("Find");
                ui.text_edit_singleline(&mut st.find);
                ui.end_row();
                ui.label("Replace with");
                ui.text_edit_singleline(&mut st.replace);
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
    app.dialogs.messages_open = open;
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
            ui.label("Reads and writes Fig 3.2 files; exports PDF, SVG, EPS and PNG.");
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
