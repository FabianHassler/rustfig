//! Tools › Find Objects… (W10-39): Find mode's window. It finds the objects
//! with given properties (`find.set`), lists the match set, outlines it on
//! the canvas and acts on all of it with `find.apply` (Update, Set depth,
//! Delete, Glue). The window shows exactly while Find mode is on: choosing
//! another mode, Escape or closing the window ends the mode.

use crate::panels::color_menu;
use crate::App;
use rustfig_model::LineKind;
use serde_json::{json, Value};

const KINDS: [&str; 9] = [
    "line", "box", "polygon", "spline", "arc", "ellipse", "text", "picture", "compound",
];
const FILLS: [&str; 3] = ["none", "filled", "pattern"];
/// "Any" in the colour and font choosers.
const ANY: i32 = -2;

#[derive(Debug, Clone)]
pub struct FindState {
    pub open: bool,
    /// Index into [`KINDS`]; `None` is any.
    pub kind: Option<usize>,
    pub pen: i32,
    pub fill_color: i32,
    pub fill: Option<usize>,
    pub depth_min: String,
    pub depth_max: String,
    pub thickness: String,
    pub line: Option<LineKind>,
    /// `None` any, `Some(true)` with arrows, `Some(false)` without.
    pub arrows: Option<bool>,
    pub font: i32,
    pub size: String,
    pub text: String,
    pub inside: bool,
    /// The number for "Set depth".
    pub new_depth: String,
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
            kind: None,
            pen: ANY,
            fill_color: ANY,
            fill: None,
            depth_min: String::new(),
            depth_max: String::new(),
            thickness: String::new(),
            line: None,
            arrows: None,
            font: ANY,
            size: String::new(),
            text: String::new(),
            inside: false,
            new_depth: "50".to_string(),
            hits: Vec::new(),
            ids: Vec::new(),
            message: String::new(),
            key: None,
            criteria: None,
        }
    }
}

impl FindState {
    /// The criteria as `edit.find_objects` parameters; empty or unreadable
    /// number fields mean "any".
    pub fn params(&self) -> Value {
        let mut p = serde_json::Map::new();
        let num = |s: &str| s.trim().parse::<f64>().ok().filter(|v| v.is_finite());
        if let Some(k) = self.kind.and_then(|i| KINDS.get(i)) {
            p.insert("kind".into(), json!(k));
        }
        if self.pen != ANY {
            p.insert("pen_color".into(), json!(self.pen));
        }
        if self.fill_color != ANY {
            p.insert("fill_color".into(), json!(self.fill_color));
        }
        if let Some(f) = self.fill.and_then(|i| FILLS.get(i)) {
            p.insert("fill".into(), json!(f));
        }
        if let Some(v) = num(&self.depth_min) {
            p.insert("depth_min".into(), json!(v));
        }
        if let Some(v) = num(&self.depth_max) {
            p.insert("depth_max".into(), json!(v));
        }
        if let Some(v) = num(&self.thickness) {
            p.insert("thickness".into(), json!(v));
        }
        if let Some(l) = self.line {
            p.insert("line".into(), json!(l.name()));
        }
        if let Some(a) = self.arrows {
            p.insert("arrows".into(), json!(if a { "any" } else { "none" }));
        }
        if self.font != ANY {
            p.insert("font".into(), json!(self.font));
        }
        if let Some(v) = num(&self.size) {
            p.insert("font_size".into(), json!(v));
        }
        if !self.text.is_empty() {
            p.insert("text".into(), json!(self.text));
        }
        if self.inside {
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
        0 => "No objects: set criteria or click objects".to_string(),
        1 => "1 object".to_string(),
        c => format!("{c} objects"),
    };
}

fn pick<T: Copy + PartialEq>(ui: &mut egui::Ui, id: &str, cur: &mut Option<T>, choices: &[(T, &str)], width: f32) {
    let label = choices
        .iter()
        .find(|(v, _)| Some(*v) == *cur)
        .map_or("Any", |(_, n)| *n);
    let r = egui::ComboBox::from_id_salt(id)
        .selected_text(label)
        .width(width)
        .show_ui(ui, |ui| {
            if ui.selectable_label(cur.is_none(), "Any").clicked() {
                *cur = None;
            }
            for (v, n) in choices {
                if ui.selectable_label(*cur == Some(*v), *n).clicked() {
                    *cur = Some(*v);
                }
            }
        });
    r.response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, format!("{id}: {label}")));
}

fn field(ui: &mut egui::Ui, text: &mut String, label: &str, width: f32) {
    let r = ui.add(egui::TextEdit::singleline(text).desired_width(width));
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, label));
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
    egui::Window::new("Find Objects")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            crate::dialog_body(ui, |ui| {
                let doc = app.engine.doc();
                egui::Grid::new("find_grid")
                    .num_columns(4)
                    .spacing([6.0, 3.0])
                    .show(ui, |ui| {
                        ui.label("Kind");
                        let mut kind = st.kind;
                        let kinds: Vec<(usize, &str)> = KINDS.iter().copied().enumerate().collect();
                        pick(ui, "Find kind", &mut kind, &kinds, 80.0);
                        st.kind = kind;
                        ui.label("Fill style");
                        let mut fill = st.fill;
                        let fills: Vec<(usize, &str)> = FILLS.iter().copied().enumerate().collect();
                        pick(ui, "Find fill style", &mut fill, &fills, 70.0);
                        st.fill = fill;
                        ui.end_row();

                        ui.label("Pen");
                        if let Some(c) = color_menu(ui, "find_pen", doc, st.pen, &[(ANY, "Any")]) {
                            st.pen = c;
                        }
                        ui.label("Fill");
                        if let Some(c) = color_menu(ui, "find_fill", doc, st.fill_color, &[(ANY, "Any")]) {
                            st.fill_color = c;
                        }
                        ui.end_row();

                        ui.label("Depth");
                        ui.horizontal(|ui| {
                            field(ui, &mut st.depth_min, "Find depth from", 34.0);
                            ui.label("to");
                            field(ui, &mut st.depth_max, "Find depth to", 34.0);
                        });
                        ui.label("Width");
                        field(ui, &mut st.thickness, "Find line width", 34.0);
                        ui.end_row();

                        ui.label("Line");
                        let mut line = st.line;
                        let lines: Vec<(LineKind, &str)> = LineKind::ALL.iter().map(|k| (*k, k.name())).collect();
                        pick(ui, "Find line style", &mut line, &lines, 80.0);
                        st.line = line;
                        ui.label("Arrows");
                        pick(
                            ui,
                            "Find arrows",
                            &mut st.arrows,
                            &[(true, "with"), (false, "without")],
                            70.0,
                        );
                        ui.end_row();

                        ui.label("Font");
                        let font_label = if st.font == ANY {
                            "Any".to_string()
                        } else {
                            rustfig_text::ps_font_display_name(st.font).to_string()
                        };
                        let r = egui::ComboBox::from_id_salt("find_font")
                            .selected_text(font_label.clone())
                            .width(80.0)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(st.font == ANY, "Any").clicked() {
                                    st.font = ANY;
                                }
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
                        ui.label("Size");
                        field(ui, &mut st.size, "Find font size", 34.0);
                        ui.end_row();

                        ui.label("Text");
                        let r = ui.add(egui::TextEdit::singleline(&mut st.text).desired_width(150.0));
                        r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Find text"));
                        ui.checkbox(&mut st.inside, "In compounds")
                            .on_hover_text("Also match members of compounds; the compound is the match");
                        ui.end_row();
                    });
                ui.label(st.message.as_str());
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
                let any = !st.ids.is_empty();
                ui.horizontal(|ui| {
                    let b = |ui: &mut egui::Ui, label: &str, tip: &str| {
                        ui.add_enabled(any, egui::Button::new(label))
                            .on_hover_text(tip)
                            .clicked()
                    };
                    if b(
                        ui,
                        "Update",
                        "Give the matches the attribute panel's settings (as Update mode)",
                    ) {
                        action = Some("update");
                    }
                    if b(ui, "Delete", "Delete the matches") {
                        action = Some("delete");
                    }
                    if b(ui, "Glue", "Glue the matches into one compound") {
                        action = Some("group");
                    }
                });
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(any, egui::Button::new("Set depth"))
                        .on_hover_text("Give the matches this depth (0-999)")
                        .clicked()
                    {
                        action = Some("depth");
                    }
                    field(ui, &mut st.new_depth, "New depth", 34.0);
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
        "depth" => match app.find.new_depth.trim().parse::<f64>() {
            Ok(d) if (0.0..=999.0).contains(&d) => Some(d),
            _ => {
                app.error("Depth must be a number from 0 to 999".to_string());
                return;
            }
        },
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
