//! Tools › Find Objects… (W10-39): Find mode's window. It finds the objects
//! with given properties (`find.set`), lists the match set, outlines it on
//! the canvas and acts on all of it with `find.apply` (Update, Set depth,
//! Delete, Glue). The window shows exactly while Find mode is on: choosing
//! another mode, Escape or closing the window ends the mode.

use crate::attributes::{line_menu, number_popup, spec};
use crate::panels::color_menu;
use crate::App;
use rustfig_model::LineKind;
use serde_json::{json, Value};

const KINDS: [&str; 9] = [
    "line", "box", "polygon", "spline", "arc", "ellipse", "text", "picture", "compound",
];
const KIND_NAMES: [&str; 9] = [
    "Polyline", "Box", "Polygon", "Spline", "Arc", "Ellipse", "Text", "Picture", "Compound",
];
const FILLS: [&str; 3] = ["none", "filled", "pattern"];
const FILL_NAMES: [&str; 3] = ["None", "Filled", "Pattern"];
const ARROW_NAMES: [&str; 2] = ["With arrows", "Without arrows"];

/// One tick box per property: a ticked property must match, an unticked one
/// is ignored (and left out of the criteria) but keeps its last value.
#[derive(Debug, Clone)]
pub struct FindState {
    pub open: bool,
    pub on_kind: bool,
    pub on_width: bool,
    pub on_line: bool,
    pub on_pen: bool,
    pub on_fill_color: bool,
    pub on_fill: bool,
    pub on_arrows: bool,
    pub on_depth: bool,
    pub on_font: bool,
    pub on_size: bool,
    pub on_text: bool,
    /// Index into [`KINDS`].
    pub kind: usize,
    pub thickness: f64,
    pub line: LineKind,
    pub pen: i32,
    pub fill_color: i32,
    /// Index into [`FILLS`].
    pub fill: usize,
    /// Index into [`ARROW_NAMES`]: 0 with arrows, 1 without.
    pub arrows: usize,
    pub depth_min: f64,
    pub depth_max: f64,
    pub font: i32,
    pub size: f64,
    pub text: String,
    pub inside: bool,
    /// The depth for "Set depth".
    pub new_depth: f64,
    /// The matches: id and list line.
    pub hits: Vec<(u64, String)>,
    /// All matching ids (the list is cut at 500).
    pub ids: Vec<u64>,
    pub message: String,
    /// What the hits were computed from (match set, document revision).
    key: Option<(Vec<u64>, u64)>,
    /// The criteria the engine's match set was last made from.
    criteria: Option<String>,
}

impl Default for FindState {
    fn default() -> Self {
        FindState {
            open: false,
            on_kind: false,
            on_width: false,
            on_line: false,
            on_pen: false,
            on_fill_color: false,
            on_fill: false,
            on_arrows: false,
            on_depth: false,
            on_font: false,
            on_size: false,
            on_text: false,
            kind: 0,
            thickness: 1.0,
            line: LineKind::ALL.first().copied().unwrap_or_default(),
            pen: 0,
            fill_color: 7,
            fill: 0,
            arrows: 0,
            depth_min: 50.0,
            depth_max: 50.0,
            font: 0,
            size: 12.0,
            text: String::new(),
            inside: false,
            new_depth: 50.0,
            hits: Vec::new(),
            ids: Vec::new(),
            message: String::new(),
            key: None,
            criteria: None,
        }
    }
}

impl FindState {
    /// Whether any property is ticked.
    pub fn any_ticked(&self) -> bool {
        self.on_kind
            || self.on_width
            || self.on_line
            || self.on_pen
            || self.on_fill_color
            || self.on_fill
            || self.on_arrows
            || self.on_depth
            || self.on_font
            || self.on_size
            || (self.on_text && !self.text.is_empty())
    }

    /// The ticked properties as `edit.find_objects` parameters; unticked
    /// ones are left out.
    pub fn params(&self) -> Value {
        let mut p = serde_json::Map::new();
        let fin = |v: f64| if v.is_finite() { v } else { 0.0 };
        if let Some(k) = KINDS.get(self.kind).filter(|_| self.on_kind) {
            p.insert("kind".into(), json!(k));
        }
        if self.on_pen {
            p.insert("pen_color".into(), json!(self.pen));
        }
        if self.on_fill_color {
            p.insert("fill_color".into(), json!(self.fill_color));
        }
        if let Some(f) = FILLS.get(self.fill).filter(|_| self.on_fill) {
            p.insert("fill".into(), json!(f));
        }
        if self.on_depth {
            let (lo, hi) = (fin(self.depth_min), fin(self.depth_max));
            p.insert("depth_min".into(), json!(lo.min(hi)));
            p.insert("depth_max".into(), json!(lo.max(hi)));
        }
        if self.on_width {
            p.insert("thickness".into(), json!(fin(self.thickness)));
        }
        if self.on_line {
            p.insert("line".into(), json!(self.line.name()));
        }
        if self.on_arrows {
            p.insert("arrows".into(), json!(if self.arrows == 0 { "any" } else { "none" }));
        }
        if self.on_font {
            p.insert("font".into(), json!(self.font));
        }
        if self.on_size {
            p.insert("font_size".into(), json!(fin(self.size)));
        }
        if self.on_text && !self.text.is_empty() {
            p.insert("text".into(), json!(self.text));
        }
        // "Also inside compounds" only widens the other properties.
        if self.inside && !p.is_empty() {
            p.insert("inside".into(), json!(true));
        }
        Value::Object(p)
    }
}

/// Rebuild the list from the engine's match set when it or the drawing
/// changed, and run the search again when the criteria changed.
fn refresh(app: &mut App) {
    let params = app.find.params().to_string();
    if app.find.criteria.as_deref() != Some(params.as_str()) {
        app.find.criteria = Some(params);
        let p = app.find.params();
        // An empty drawing has nothing to find (the command is disabled).
        if let Err(m) = app.try_run("find.set", &p) {
            if !app.engine.doc().objects.is_empty() {
                app.find.message = m;
            }
        }
        app.find.key = None;
    }
    let ids = app.engine.find_matches();
    let key = (ids.clone(), app.engine.revision());
    if app.find.key.as_ref() == Some(&key) {
        return;
    }
    let state = app.try_run("find.state", &json!({}));
    let st = &mut app.find;
    st.key = Some(key);
    st.hits.clear();
    st.ids = ids;
    if let Ok(r) = state {
        let n = |v: Option<&Value>| v.and_then(Value::as_u64);
        if let Some(a) = r.get("items").and_then(Value::as_array) {
            for i in a {
                let label = i.get("label").and_then(Value::as_str).unwrap_or("");
                let depth = n(i.get("depth")).unwrap_or(0);
                if let Some(id) = n(i.get("id")) {
                    st.hits.push((id, format!("{label}  (depth {depth})")));
                }
            }
        }
    }
    st.message = match st.ids.len() {
        0 if st.any_ticked() => "No objects found".to_string(),
        0 => "Tick a property or click objects".to_string(),
        1 => "1 object found".to_string(),
        c => format!("{c} objects found"),
    };
}

/// A list of choices without an "any" entry (unticked means any).
fn combo(ui: &mut egui::Ui, id: &str, cur: &mut usize, names: &[&str], width: f32) {
    let label = names.get(*cur).copied().unwrap_or("");
    let r = egui::ComboBox::from_id_salt(id)
        .selected_text(label)
        .width(width)
        .show_ui(ui, |ui| {
            for (i, n) in names.iter().enumerate() {
                if ui.selectable_label(*cur == i, *n).clicked() {
                    *cur = i;
                }
            }
        });
    r.response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, format!("{id}: {label}")));
}

/// A number button (the Edit panel's pop-up).
fn number(ui: &mut egui::Ui, name: &str, v: &mut f64, sp: &crate::attributes::NumSpec) {
    if let Some(x) = number_popup(ui, &format!("find_{name}"), name, *v, sp) {
        *v = x;
    }
}

pub fn dialog(app: &mut App, ctx: &egui::Context) {
    // The window is Find mode's panel: it shows exactly while the mode is on.
    if app.engine.tool() != rustfig_engine::ToolKind::Find {
        app.find.open = false;
        return;
    }
    if !app.find.open {
        // Just entered: the match set starts empty, the criteria apply once changed.
        app.find.open = true;
        app.find.criteria = Some(app.find.params().to_string());
        app.find.key = None;
    }
    refresh(app);
    let mut open = true;
    let mut action: Option<&'static str> = None;
    let mut clicked: Option<u64> = None;
    let mut st = std::mem::take(&mut app.find);
    egui::Window::new("Find Properties")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            crate::dialog_body(ui, |ui| {
                let doc = app.engine.doc();
                egui::Grid::new("find_grid")
                    .num_columns(4)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.checkbox(&mut st.on_kind, "Object kind");
                        ui.add_enabled_ui(st.on_kind, |ui| {
                            combo(ui, "Find kind", &mut st.kind, &KIND_NAMES, 90.0);
                        });
                        ui.end_row();

                        ui.checkbox(&mut st.on_width, "Line width");
                        ui.add_enabled_ui(st.on_width, |ui| {
                            number(ui, "Line width", &mut st.thickness, &spec::THICKNESS)
                        });
                        ui.checkbox(&mut st.on_line, "Line style");
                        ui.add_enabled_ui(st.on_line, |ui| {
                            if let Some(l) = line_menu(ui, "find_line", st.line) {
                                st.line = l;
                            }
                        });
                        ui.end_row();

                        ui.checkbox(&mut st.on_pen, "Pen color");
                        ui.add_enabled_ui(st.on_pen, |ui| {
                            if let Some(c) = color_menu(ui, "find_pen", doc, st.pen, &[]) {
                                st.pen = c;
                            }
                        });
                        ui.checkbox(&mut st.on_fill_color, "Fill color");
                        ui.add_enabled_ui(st.on_fill_color, |ui| {
                            if let Some(c) = color_menu(ui, "find_fill", doc, st.fill_color, &[]) {
                                st.fill_color = c;
                            }
                        });
                        ui.end_row();

                        ui.checkbox(&mut st.on_fill, "Fill style");
                        ui.add_enabled_ui(st.on_fill, |ui| {
                            combo(ui, "Find fill style", &mut st.fill, &FILL_NAMES, 80.0);
                        });
                        ui.checkbox(&mut st.on_arrows, "Arrows");
                        ui.add_enabled_ui(st.on_arrows, |ui| {
                            combo(ui, "Find arrows", &mut st.arrows, &ARROW_NAMES, 110.0);
                        });
                        ui.end_row();

                        ui.checkbox(&mut st.on_depth, "Depth");
                        ui.add_enabled_ui(st.on_depth, |ui| {
                            ui.horizontal(|ui| {
                                number(ui, "Depth from", &mut st.depth_min, &spec::DEPTH);
                                ui.label("to");
                                number(ui, "Depth to", &mut st.depth_max, &spec::DEPTH);
                            });
                        });
                        ui.end_row();

                        ui.checkbox(&mut st.on_font, "Font");
                        ui.add_enabled_ui(st.on_font, |ui| {
                            let font_label = rustfig_text::ps_font_display_name(st.font).to_string();
                            let r = egui::ComboBox::from_id_salt("find_font")
                                .selected_text(font_label.clone())
                                .width(110.0)
                                .show_ui(ui, |ui| {
                                    for f in -1..35 {
                                        let n = rustfig_text::ps_font_display_name(f);
                                        if ui.selectable_label(st.font == f, n).clicked() {
                                            st.font = f;
                                        }
                                    }
                                });
                            r.response.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::ComboBox,
                                    true,
                                    format!("Find font: {font_label}"),
                                )
                            });
                        });
                        ui.checkbox(&mut st.on_size, "Font size");
                        ui.add_enabled_ui(st.on_size, |ui| number(ui, "Font size", &mut st.size, &spec::FONT_SIZE));
                        ui.end_row();

                        ui.checkbox(&mut st.on_text, "Text contains");
                        ui.add_enabled_ui(st.on_text, |ui| {
                            let r = ui.add(egui::TextEdit::singleline(&mut st.text).desired_width(150.0));
                            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Find text"));
                        });
                        ui.end_row();

                        ui.checkbox(&mut st.inside, "Also inside compounds")
                            .on_hover_text("Also match members of compounds; the compound is the match");
                        ui.end_row();
                    });
                ui.separator();
                ui.label(st.message.as_str());
                if !st.hits.is_empty() {
                    egui::CollapsingHeader::new("Show list").show(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(90.0).show(ui, |ui| {
                            for (id, line) in &st.hits {
                                if ui
                                    .selectable_label(false, line.as_str())
                                    .on_hover_text("Show this object")
                                    .clicked()
                                {
                                    clicked = Some(*id);
                                }
                            }
                        });
                    });
                }
                let any = !st.ids.is_empty();
                ui.horizontal(|ui| {
                    let b = |ui: &mut egui::Ui, label: &str, tip: &str| {
                        ui.add_enabled(any, egui::Button::new(label))
                            .on_hover_text(tip)
                            .clicked()
                    };
                    if b(
                        ui,
                        "Update to indicator",
                        "Give the matches the attribute panel's settings (as Update mode)",
                    ) {
                        action = Some("update");
                    }
                    if b(ui, "Set depth", "Give the matches the depth on the button (0-999)") {
                        action = Some("depth");
                    }
                    ui.add_enabled_ui(any, |ui| number(ui, "New depth", &mut st.new_depth, &spec::DEPTH));
                    if b(ui, "Glue", "Glue the matches into one compound") {
                        action = Some("group");
                    }
                    if b(ui, "Delete", "Delete the matches") {
                        action = Some("delete");
                    }
                });
            });
        });
    app.find = st;
    if !open {
        // Closing the window ends Find mode.
        app.run("tool.escape", json!({}));
        app.find.open = false;
        return;
    }
    if let Some(id) = clicked {
        app.run("edit.show_object", json!({ "id": id }));
    }
    if let Some(a) = action {
        run_action(app, a);
    }
}

/// One command on all matches: one undo step.
fn run_action(app: &mut App, action: &str) {
    let n = app.find.ids.len();
    let depth = match action {
        "depth" => Some(app.find.new_depth.clamp(0.0, 999.0)),
        _ => None,
    };
    let done = app.run("find.apply", json!({ "action": action, "depth": depth }));
    if done.is_some() {
        refresh(app);
        let verb = match action {
            "update" => "updated",
            "delete" => "deleted",
            "group" => "glued",
            _ => "set to the new depth",
        };
        app.find.message = format!("{n} object{} {verb}", if n == 1 { "" } else { "s" });
    }
}
