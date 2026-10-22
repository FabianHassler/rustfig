//! The colour picker and editor window (E4, W10-36).
//!
//! A hue/saturation square, RGB and HSV fields, a hex or X11-name field,
//! recently used colours and the drawing's user colours (xfig's colour
//! panel: add, modify, delete, remove unused). Everything it does goes
//! through the `color.*` commands.

use crate::App;
use egui::{pos2, vec2, Color32, Rect, Sense, Stroke};
use rustfig_engine::cmd::color::{from_hsv_values, to_hsv};
use rustfig_model::{ColorRef, Rgb};
use serde_json::{json, Value};

/// The current colour the picker's "Use" buttons set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorField {
    Pen,
    Fill,
}

pub struct ColorDialog {
    pub open: bool,
    rgb: Rgb,
    /// Hue 0-360, saturation and value 0-100 (the hue stays when saturation is 0).
    hsv: (f32, f32, f32),
    hex: String,
    /// The user colour being edited.
    selected: Option<i32>,
    /// A name for keeping the colour in the preferences.
    name: String,
    /// A pick from the screen in progress (E4b).
    pub pick: crate::screen_pick::ScreenPicker,
}

impl Default for ColorDialog {
    fn default() -> Self {
        ColorDialog {
            open: false,
            rgb: Rgb::BLACK,
            hsv: (0.0, 0.0, 0.0),
            hex: Rgb::BLACK.to_hex(),
            selected: None,
            name: String::new(),
            pick: Default::default(),
        }
    }
}

impl ColorDialog {
    fn set_rgb(&mut self, rgb: Rgb) {
        self.rgb = rgb;
        let (h, s, v) = to_hsv(rgb);
        // Keep the hue where it carries no information.
        let hue = if s == 0.0 || v == 0.0 { self.hsv.0 } else { h as f32 };
        self.hsv = (hue, s as f32, v as f32);
        self.hex = rgb.to_hex();
    }

    fn set_hsv(&mut self, hsv: (f32, f32, f32)) {
        self.hsv = hsv;
        self.rgb = from_hsv_values(f64::from(hsv.0), f64::from(hsv.1), f64::from(hsv.2));
        self.hex = self.rgb.to_hex();
    }
}

/// Open the window on colour `current` (a colour number; Default shows black).
pub fn open(app: &mut App, current: i32) {
    let rgb = app.engine.doc().rgb(ColorRef(current.max(0)));
    let d = &mut app.dialogs.color;
    d.open = true;
    d.selected = (current >= rustfig_model::FIRST_USER_COLOR).then_some(current);
    d.set_rgb(rgb);
}

fn c32(c: Rgb) -> Color32 {
    Color32::from_rgb(c.r, c.g, c.b)
}

fn swatch_button(ui: &mut egui::Ui, rgb: Rgb, selected: bool, label: &str) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(22.0, 18.0), Sense::click());
    ui.painter().rect_filled(r, 2.0, c32(rgb));
    let stroke = if selected {
        Stroke::new(2.0_f32, ui.visuals().selection.stroke.color)
    } else {
        Stroke::new(1.0_f32, Color32::GRAY)
    };
    ui.painter().rect_stroke(r, 2.0, stroke, egui::StrokeKind::Inside);
    let clicked = resp.clicked();
    resp.on_hover_text(label)
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, label));
    clicked
}

fn parse(app: &mut App, text: &str) -> Option<Rgb> {
    let r = app.run("color.parse", json!({ "text": text }))?;
    Rgb::from_hex(r.get("rgb")?.as_str()?)
}

fn field_name(f: ColorField) -> &'static str {
    match f {
        ColorField::Pen => "pen_color",
        ColorField::Fill => "fill_color",
    }
}

/// Take the result of a pick from the screen: the picker shows the colour
/// (the user applies it with "Use as pen/fill colour").
pub(crate) fn poll_pick(app: &mut App, ctx: &egui::Context) {
    use crate::screen_pick::Outcome;
    match app.dialogs.color.pick.poll(ctx) {
        Some(Outcome::Picked([r, g, b])) => {
            app.dialogs.color.set_rgb(Rgb::new(r, g, b));
            let hex = app.dialogs.color.rgb.to_hex();
            app.info(format!("Picked {hex} from the screen"));
        }
        Some(Outcome::Cancelled) => app.info("Picking from the screen cancelled"),
        Some(Outcome::Failed(e)) => app.error(e),
        None => {}
    }
}

/// Make the picker's colour the current pen or fill colour.
fn use_for(app: &mut App, f: ColorField) {
    let hex = app.dialogs.color.rgb.to_hex();
    let number = match app.dialogs.color.selected {
        Some(n) if app.engine.doc().rgb(ColorRef(n)) == app.dialogs.color.rgb => Some(i64::from(n)),
        _ => app
            .run("color.add", json!({ "rgb": hex }))
            .and_then(|r| r.get("color").and_then(Value::as_i64)),
    };
    if let Some(n) = number {
        app.run("indicator.set", json!({ field_name(f): n }));
        app.run("color.use", json!({ "color": n }));
    }
}

/// The SV square and hue strip; returns true when the colour changed.
fn picker(ui: &mut egui::Ui, d: &mut ColorDialog) -> bool {
    let mut changed = false;
    let (mut h, mut s, mut v) = d.hsv;
    let (sq, resp) = ui.allocate_exact_size(vec2(160.0, 160.0), Sense::click_and_drag());
    {
        let hue = c32(from_hsv_values(f64::from(h), 100.0, 100.0));
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(sq.left_top(), Color32::WHITE);
        mesh.colored_vertex(sq.right_top(), hue);
        mesh.colored_vertex(sq.right_bottom(), Color32::BLACK);
        mesh.colored_vertex(sq.left_bottom(), Color32::BLACK);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        ui.painter().add(egui::Shape::mesh(mesh));
        let at = pos2(
            sq.left() + s / 100.0 * sq.width(),
            sq.top() + (1.0 - v / 100.0) * sq.height(),
        );
        ui.painter()
            .circle_stroke(at, 5.0, Stroke::new(2.0_f32, Color32::WHITE));
        ui.painter()
            .circle_stroke(at, 6.5, Stroke::new(1.0_f32, Color32::BLACK));
    }
    if let (true, Some(p)) = (resp.dragged() || resp.clicked(), resp.interact_pointer_pos()) {
        s = ((p.x - sq.left()) / sq.width() * 100.0).clamp(0.0, 100.0);
        v = (100.0 - (p.y - sq.top()) / sq.height() * 100.0).clamp(0.0, 100.0);
        changed = true;
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, "Saturation and brightness square"));
    let (strip, hresp) = ui.allocate_exact_size(vec2(18.0, 160.0), Sense::click_and_drag());
    {
        let mut mesh = egui::Mesh::default();
        for i in 0..=6u32 {
            let y = strip.top() + strip.height() * i as f32 / 6.0;
            let c = c32(from_hsv_values(f64::from(i) * 60.0, 100.0, 100.0));
            mesh.colored_vertex(pos2(strip.left(), y), c);
            mesh.colored_vertex(pos2(strip.right(), y), c);
            if i > 0 {
                let b = (i - 1) * 2;
                mesh.add_triangle(b, b + 1, b + 2);
                mesh.add_triangle(b + 1, b + 3, b + 2);
            }
        }
        ui.painter().add(egui::Shape::mesh(mesh));
        let y = strip.top() + h / 360.0 * strip.height();
        let mark = Rect::from_min_max(pos2(strip.left() - 2.0, y - 2.0), pos2(strip.right() + 2.0, y + 2.0));
        ui.painter().rect_stroke(
            mark,
            1.0,
            Stroke::new(1.5_f32, Color32::WHITE),
            egui::StrokeKind::Outside,
        );
    }
    if let (true, Some(p)) = (hresp.dragged() || hresp.clicked(), hresp.interact_pointer_pos()) {
        h = ((p.y - strip.top()) / strip.height() * 360.0).clamp(0.0, 359.9);
        changed = true;
    }
    hresp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, "Hue strip"));
    if changed {
        d.set_hsv((h, s, v));
    }
    changed
}

/// The window; called every frame.
pub fn dialog(app: &mut App, ctx: &egui::Context) {
    poll_pick(app, ctx);
    if !app.dialogs.color.open {
        return;
    }
    let mut open = true;
    egui::Window::new("Colours")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| body(app, ui));
    app.dialogs.color.open &= open;
}

fn body(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        picker(ui, &mut app.dialogs.color);
        ui.vertical(|ui| {
            let d = &mut app.dialogs.color;
            let (r, _) = ui.allocate_exact_size(vec2(120.0, 28.0), Sense::hover());
            ui.painter().rect_filled(r, 3.0, c32(d.rgb));
            ui.painter()
                .rect_stroke(r, 3.0, Stroke::new(1.0_f32, Color32::GRAY), egui::StrokeKind::Inside);
            let mut rgb = [d.rgb.r, d.rgb.g, d.rgb.b];
            let mut hsv = d.hsv;
            let mut by_rgb = false;
            let mut by_hsv = false;
            egui::Grid::new("color_fields").num_columns(2).show(ui, |ui| {
                for (i, name) in ["Red", "Green", "Blue"].iter().enumerate() {
                    ui.label(*name);
                    if let Some(c) = rgb.get_mut(i) {
                        let resp = ui.add(egui::DragValue::new(c).range(0..=255));
                        by_rgb |= resp.changed();
                        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, *name));
                    }
                    ui.end_row();
                }
                ui.label("Hue");
                let resp = ui.add(
                    egui::DragValue::new(&mut hsv.0)
                        .range(0.0..=359.0)
                        .speed(1.0)
                        .fixed_decimals(0),
                );
                by_hsv |= resp.changed();
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, "Hue"));
                ui.end_row();
                ui.label("Saturation");
                let resp = ui.add(
                    egui::DragValue::new(&mut hsv.1)
                        .range(0.0..=100.0)
                        .speed(1.0)
                        .fixed_decimals(0),
                );
                by_hsv |= resp.changed();
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, "Saturation"));
                ui.end_row();
                ui.label("Value");
                let resp = ui.add(
                    egui::DragValue::new(&mut hsv.2)
                        .range(0.0..=100.0)
                        .speed(1.0)
                        .fixed_decimals(0),
                );
                by_hsv |= resp.changed();
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, "Value"));
                ui.end_row();
            });
            if by_rgb {
                d.set_rgb(Rgb::new(rgb[0], rgb[1], rgb[2]));
            } else if by_hsv {
                d.set_hsv(hsv);
            }
        });
    });
    // Hex or X11 name: the colour follows while typing once the text is a
    // whole colour; Enter (or leaving the field) makes it canonical; matching
    // X11 names are offered while typing a name.
    let mut apply = None;
    let mut live = None;
    let mut suggestions: Vec<String> = Vec::new();
    ui.horizontal(|ui| {
        ui.label("Hex or name");
        let hex = &mut app.dialogs.color.hex;
        let resp = ui.add(egui::TextEdit::singleline(hex).desired_width(120.0));
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Colour hex or X11 name"));
        if resp.lost_focus() {
            apply = Some(hex.clone());
        } else if resp.changed() {
            live = Some(hex.clone());
        }
        if resp.has_focus() && !hex.trim().is_empty() && !hex.starts_with('#') {
            suggestions.push(hex.trim().to_string());
        }
    });
    if let Some(text) = apply {
        match parse(app, &text) {
            Some(rgb) => app.dialogs.color.set_rgb(rgb),
            // Not a colour: back to the current value.
            None => app.dialogs.color.hex = app.dialogs.color.rgb.to_hex(),
        }
    } else if let Some(text) = live {
        if let Some(rgb) = parse(app, &text) {
            let typed = text;
            app.dialogs.color.set_rgb(rgb);
            app.dialogs.color.hex = typed;
        }
    }
    if let Some(prefix) = suggestions.pop() {
        if let Some(r) = app.run("color.names", json!({"prefix": prefix, "limit": 6})) {
            suggestions = r
                .get("names")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|n| n.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
        }
    }
    if !suggestions.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for n in suggestions {
                if ui.small_button(&n).clicked() {
                    if let Some(rgb) = parse(app, &n) {
                        app.dialogs.color.set_rgb(rgb);
                    }
                }
            }
        });
    }
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("Use as pen colour").clicked() {
            use_for(app, ColorField::Pen);
        }
        if ui.button("Use as fill colour").clicked() {
            use_for(app, ColorField::Fill);
        }
    });
    ui.horizontal(|ui| {
        let can = crate::screen_pick::available();
        let picking = app.dialogs.color.pick.active();
        let b = ui
            .add_enabled(can && !picking, egui::Button::new("Pick from screen"))
            .on_hover_text("Take the colour of any pixel on the screen, also in other programs")
            .on_disabled_hover_text(if picking {
                "Click a pixel to take its colour; Escape cancels"
            } else {
                crate::screen_pick::unavailable_reason()
            });
        if b.clicked() {
            let ctx = ui.ctx().clone();
            if let Err(e) = app.dialogs.color.pick.start(&ctx) {
                app.error(e);
            } else if app.dialogs.color.pick.active() {
                app.info("Click a pixel anywhere on the screen to take its colour; Escape cancels");
            }
        }
        if let Some([r, g, b]) = app.dialogs.color.pick.live() {
            let (sw, _) = ui.allocate_exact_size(vec2(28.0, 18.0), Sense::hover());
            ui.painter().rect_filled(sw, 2.0, Color32::from_rgb(r, g, b));
            ui.label(Rgb::new(r, g, b).to_hex());
        }
    });
    // Recent colours.
    let recent: Vec<Rgb> = app
        .engine
        .prefs
        .rustfig
        .recent_colors
        .iter()
        .filter_map(|h| Rgb::from_hex(h))
        .collect();
    if !recent.is_empty() {
        ui.label(egui::RichText::new("Recent").small());
        ui.horizontal_wrapped(|ui| {
            for c in recent {
                if swatch_button(ui, c, false, &format!("Recent {}", c.to_hex())) {
                    app.dialogs.color.selected = None;
                    app.dialogs.color.set_rgb(c);
                }
            }
        });
    }
    // The drawing's user colours: the editor.
    ui.label(egui::RichText::new("Colours in this drawing").small());
    let user: Vec<(i32, Rgb)> = app.engine.doc().colors.iter().map(|u| (u.number, u.rgb)).collect();
    if user.is_empty() {
        ui.label(egui::RichText::new("None yet.").weak());
    }
    ui.horizontal_wrapped(|ui| {
        for (n, c) in &user {
            let sel = app.dialogs.color.selected == Some(*n);
            if swatch_button(ui, *c, sel, &format!("User colour {n} {}", c.to_hex())) {
                app.dialogs.color.selected = Some(*n);
                app.dialogs.color.set_rgb(*c);
            }
        }
    });
    let hex = app.dialogs.color.rgb.to_hex();
    ui.horizontal(|ui| {
        if ui.button("Add as new colour").clicked() {
            if let Some(n) = app
                .run("color.new", json!({ "rgb": hex }))
                .and_then(|r| r.get("color").and_then(Value::as_i64))
            {
                app.dialogs.color.selected = i32::try_from(n).ok();
            }
        }
        if let Some(n) = app.dialogs.color.selected {
            if ui.button(format!("Change colour {n}")).clicked() {
                app.run("color.edit", json!({ "color": n, "rgb": hex }));
            }
            if ui.button(format!("Delete colour {n}")).clicked()
                && app.run("color.delete", json!({ "color": n })).is_some()
            {
                app.dialogs.color.selected = None;
            }
        }
        if ui.button("Remove unused colours").clicked() {
            app.run("color.remove_unused", json!({}));
            let still = app
                .dialogs
                .color
                .selected
                .is_some_and(|n| user.iter().any(|(u, _)| *u == n));
            if !still {
                app.dialogs.color.selected = None;
            }
        }
    });
    // Keep a colour under a name for every drawing (E1's list).
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.dialogs.color.name)
                .hint_text("name")
                .desired_width(110.0),
        )
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Colour name"));
        let ready = !app.dialogs.color.name.trim().is_empty();
        if ui
            .add_enabled(ready, egui::Button::new("Keep for all drawings"))
            .clicked()
        {
            let name = app.dialogs.color.name.trim().to_string();
            app.dialogs.color.name.clear();
            app.run("prefs.add_color", json!({ "name": name, "rgb": hex }));
        }
    });
}
