//! Side and bottom panels: mode (tool) panel, attribute (indicator) panel,
//! depth manager and status bar.

use crate::theme::Tokens;
use crate::{icons, App};
use egui::{vec2, Color32, RichText, Sense};
use rustfig_engine::{SnapMode, ToolKind};
use rustfig_model::{CapStyle, Document, JoinStyle, Justify, LineKind, Rgb, STANDARD_COLORS, STANDARD_COLOR_NAMES};
use serde_json::{json, Value};

pub fn mode_panel(app: &mut App, ui: &mut egui::Ui, tk: &Tokens) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(4.0);
        ui.label(RichText::new("Drawing").small().color(tk.muted));
        tool_grid(app, ui, tk, true);
        ui.add_space(6.0);
        ui.label(RichText::new("Editing").small().color(tk.muted));
        tool_grid(app, ui, tk, false);
    });
}

fn tool_grid(app: &mut App, ui: &mut egui::Ui, tk: &Tokens, drawing: bool) {
    let tools: Vec<ToolKind> = ToolKind::ALL
        .iter()
        .copied()
        .filter(|t| t.is_drawing() == drawing || (drawing && *t == ToolKind::Select))
        .filter(|t| drawing || *t != ToolKind::Select)
        .collect();
    egui::Grid::new(if drawing { "draw_tools" } else { "edit_tools" })
        .spacing(vec2(2.0, 2.0))
        .show(ui, |ui| {
            for (i, t) in tools.iter().enumerate() {
                let size = vec2(36.0, 30.0);
                let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                let active = app.engine.tool() == *t;
                let bg = if active {
                    tk.tool_active_bg
                } else if resp.hovered() {
                    ui.visuals().widgets.hovered.bg_fill
                } else {
                    ui.visuals().widgets.inactive.bg_fill
                };
                ui.painter().rect_filled(rect, 4.0, bg);
                icons::paint(
                    ui.painter(),
                    rect.shrink(4.0),
                    *t,
                    if active { tk.icon_active } else { tk.icon },
                );
                let tip = if t.shortcut().is_empty() {
                    t.label().to_string()
                } else {
                    format!("{} ({})", t.label(), t.shortcut())
                };
                let label = t.label();
                resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, active, label));
                if resp.on_hover_text(tip).clicked() {
                    app.run(t.id(), json!({}));
                }
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
}

/// Colour swatch + name.
fn color_label(doc: &Document, c: i32) -> (Color32, String) {
    let rgb = doc.rgb(rustfig_model::ColorRef(c));
    let name = if c < 0 {
        "Default".to_string()
    } else if let Some(n) = usize::try_from(c).ok().and_then(|i| STANDARD_COLOR_NAMES.get(i)) {
        (*n).to_string()
    } else {
        format!("User {c}")
    };
    (Color32::from_rgb(rgb.r, rgb.g, rgb.b), name)
}

fn swatch(ui: &mut egui::Ui, c: Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
    ui.painter().rect_filled(r, 2.0, c);
    ui.painter().rect_stroke(
        r,
        2.0,
        egui::Stroke::new(1.0_f32, Color32::GRAY),
        egui::StrokeKind::Middle,
    );
}

/// A colour picker combo; returns the chosen colour number.
pub fn color_combo(ui: &mut egui::Ui, id: &str, doc: &Document, current: i32) -> Option<i32> {
    let mut chosen = None;
    let (col, name) = color_label(doc, current);
    ui.horizontal(|ui| {
        swatch(ui, col);
        egui::ComboBox::from_id_salt(id)
            .selected_text(name)
            .width(90.0)
            .show_ui(ui, |ui| {
                let mut all: Vec<i32> = (-1..32).collect();
                all.extend(doc.colors.iter().map(|c| c.number));
                for c in all {
                    let (col, name) = color_label(doc, c);
                    ui.horizontal(|ui| {
                        swatch(ui, col);
                        if ui.selectable_label(c == current, name).clicked() {
                            chosen = Some(c);
                        }
                    });
                }
            });
    });
    chosen
}

/// Area fill choices: (value, label).
pub fn fill_choices() -> Vec<(i32, String)> {
    let mut v = vec![(-1, "None".to_string())];
    for f in (0..=40).step_by(5) {
        v.push((f, format!("{f} ({})", if f <= 20 { "shade" } else { "tint" })));
    }
    for (i, name) in rustfig_render::pattern::PATTERN_NAMES.iter().enumerate() {
        v.push((41 + i as i32, format!("{} — {name}", 41 + i)));
    }
    v
}

pub fn fill_combo(ui: &mut egui::Ui, id: &str, current: i32) -> Option<i32> {
    let mut chosen = None;
    let choices = fill_choices();
    let text = choices
        .iter()
        .find(|(v, _)| *v == current)
        .map(|(_, l)| l.clone())
        .unwrap_or_else(|| current.to_string());
    egui::ComboBox::from_id_salt(id)
        .selected_text(text)
        .width(110.0)
        .show_ui(ui, |ui| {
            for (v, l) in choices {
                if ui.selectable_label(v == current, l).clicked() {
                    chosen = Some(v);
                }
            }
        });
    chosen
}

pub fn line_combo(ui: &mut egui::Ui, id: &str, current: LineKind) -> Option<LineKind> {
    let mut chosen = None;
    egui::ComboBox::from_id_salt(id)
        .selected_text(current.name())
        .width(110.0)
        .show_ui(ui, |ui| {
            for k in LineKind::ALL {
                if ui.selectable_label(k == current, k.name()).clicked() {
                    chosen = Some(k);
                }
            }
        });
    chosen
}

pub fn font_combo(ui: &mut egui::Ui, id: &str, font: i32, postscript: bool) -> Option<(i32, bool)> {
    let mut chosen = None;
    let name = if postscript {
        rustfig_text::ps_font_display_name(font).to_string()
    } else {
        format!("LaTeX {}", rustfig_text::latex_font_name(font))
    };
    egui::ComboBox::from_id_salt(id)
        .selected_text(name)
        .width(170.0)
        .show_ui(ui, |ui| {
            for f in 0..rustfig_text::PS_FONT_COUNT {
                if ui
                    .selectable_label(postscript && f == font, rustfig_text::ps_font_display_name(f))
                    .clicked()
                {
                    chosen = Some((f, true));
                }
            }
            ui.separator();
            for f in 0..rustfig_text::LATEX_FONT_COUNT {
                let label = format!("LaTeX {}", rustfig_text::latex_font_name(f));
                if ui.selectable_label(!postscript && f == font, label).clicked() {
                    chosen = Some((f, false));
                }
            }
        });
    chosen
}

fn set(app: &mut App, field: &str, v: Value) {
    let mut m = serde_json::Map::new();
    m.insert(field.to_string(), v);
    app.run("indicator.set", Value::Object(m));
}

pub fn indicator_panel(app: &mut App, ui: &mut egui::Ui) {
    let ind = app.engine.indicator.clone();
    let units = app.engine.doc().settings.units;
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
        ui.label("Line");
        if let Some(k) = line_combo(ui, "ind_line", ind.line) {
            set(app, "line", json!(k));
        }
        let mut th = ind.thickness;
        if ui
            .add(egui::DragValue::new(&mut th).range(0..=100).prefix("w "))
            .changed()
        {
            set(app, "thickness", json!(th));
        }
        if ind.line != LineKind::Solid {
            let mut sv = ind.style_val;
            if ui
                .add(
                    egui::DragValue::new(&mut sv)
                        .range(0.5..=100.0)
                        .speed(0.1)
                        .prefix("dash "),
                )
                .changed()
            {
                set(app, "style_val", json!(sv));
            }
        }
        ui.separator();
        ui.label("Pen");
        if let Some(c) = color_combo(ui, "ind_pen", app.engine.doc(), ind.pen_color) {
            set(app, "pen_color", json!(c));
        }
        ui.label("Fill");
        if let Some(c) = color_combo(ui, "ind_fill", app.engine.doc(), ind.fill_color) {
            set(app, "fill_color", json!(c));
        }
        if let Some(f) = fill_combo(ui, "ind_area", ind.area_fill) {
            set(app, "area_fill", json!(f));
        }
        ui.separator();
        let mut depth = ind.depth;
        if ui
            .add(egui::DragValue::new(&mut depth).range(0..=999).prefix("depth "))
            .changed()
        {
            set(app, "depth", json!(depth));
        }
        ui.separator();
        let mut fwd = ind.forward_arrow;
        if ui
            .toggle_value(&mut fwd, "fwd arrow")
            .on_hover_text("Forward arrow")
            .changed()
        {
            set(app, "forward_arrow", json!(fwd));
        }
        let mut back = ind.backward_arrow;
        if ui
            .toggle_value(&mut back, "back arrow")
            .on_hover_text("Backward arrow")
            .changed()
        {
            set(app, "backward_arrow", json!(back));
        }
        if ind.forward_arrow || ind.backward_arrow {
            let names = [
                "stick",
                "triangle",
                "indented",
                "pointed",
                "circle",
                "half circle",
                "square",
                "diamond",
                "bar",
                "fork",
            ];
            let mut a = ind.arrow;
            let cur = usize::try_from(a.kind)
                .ok()
                .and_then(|i| names.get(i))
                .copied()
                .unwrap_or("?");
            egui::ComboBox::from_id_salt("ind_arrow")
                .selected_text(cur)
                .width(80.0)
                .show_ui(ui, |ui| {
                    for (i, n) in names.iter().enumerate() {
                        if ui.selectable_label(a.kind == i as i32, *n).clicked() {
                            a.kind = i as i32;
                        }
                    }
                });
            let mut filled = a.style == 1;
            ui.checkbox(&mut filled, "filled");
            a.style = i32::from(filled);
            ui.add(egui::DragValue::new(&mut a.width).range(10.0..=2000.0).prefix("aw "));
            ui.add(egui::DragValue::new(&mut a.height).range(10.0..=2000.0).prefix("ah "));
            if a != ind.arrow {
                set(app, "arrow", json!(a));
            }
        }
        ui.separator();
        egui::ComboBox::from_id_salt("ind_cap")
            .selected_text(format!("cap: {:?}", ind.cap))
            .width(110.0)
            .show_ui(ui, |ui| {
                for c in [CapStyle::Butt, CapStyle::Round, CapStyle::Projecting] {
                    if ui.selectable_label(c == ind.cap, format!("{c:?}")).clicked() {
                        set(app, "cap", json!(c));
                    }
                }
            });
        egui::ComboBox::from_id_salt("ind_join")
            .selected_text(format!("join: {:?}", ind.join))
            .width(110.0)
            .show_ui(ui, |ui| {
                for j in [JoinStyle::Miter, JoinStyle::Round, JoinStyle::Bevel] {
                    if ui.selectable_label(j == ind.join, format!("{j:?}")).clicked() {
                        set(app, "join", json!(j));
                    }
                }
            });
        ui.separator();
        if let Some((f, ps)) = font_combo(ui, "ind_font", ind.font, ind.postscript) {
            set(app, "font", json!(f));
            set(app, "postscript", json!(ps));
        }
        let mut size = ind.font_size;
        if ui
            .add(egui::DragValue::new(&mut size).range(1.0..=500.0).suffix(" pt"))
            .changed()
        {
            set(app, "font_size", json!(size));
        }
        for (j, label) in [(Justify::Left, "L"), (Justify::Center, "C"), (Justify::Right, "R")] {
            if ui
                .selectable_label(ind.justify == j, label)
                .on_hover_text(format!("{j:?} justified"))
                .clicked()
            {
                set(app, "justify", json!(j));
            }
        }
        let mut ta = ind.text_angle;
        if ui
            .add(egui::DragValue::new(&mut ta).range(-360.0..=360.0).suffix("°"))
            .on_hover_text("Text angle")
            .changed()
        {
            set(app, "text_angle", json!(ta));
        }
        ui.separator();
        let mut r = ind.box_radius;
        if ui
            .add(egui::DragValue::new(&mut r).range(0..=200).prefix("radius "))
            .changed()
        {
            set(app, "box_radius", json!(r));
        }
        let mut sides = ind.sides;
        if ui
            .add(egui::DragValue::new(&mut sides).range(3..=100).prefix("sides "))
            .changed()
        {
            set(app, "sides", json!(sides));
        }
        let mut rot = ind.rotate_angle;
        if ui
            .add(
                egui::DragValue::new(&mut rot)
                    .range(-360.0..=360.0)
                    .prefix("rotate ")
                    .suffix("°"),
            )
            .changed()
        {
            set(app, "rotate_angle", json!(rot));
        }
        ui.separator();
        egui::ComboBox::from_id_salt("ind_snap")
            .selected_text(format!("snap: {}", ind.snap.label(units)))
            .width(110.0)
            .show_ui(ui, |ui| {
                for m in SnapMode::ALL {
                    if ui.selectable_label(m == ind.snap, m.label(units)).clicked() {
                        set(app, "snap", json!(m));
                    }
                }
            });
        let grid_label = ind.grid.map(|g| g.label(units)).unwrap_or("off");
        egui::ComboBox::from_id_salt("ind_grid")
            .selected_text(format!("grid: {grid_label}"))
            .width(110.0)
            .show_ui(ui, |ui| {
                if ui.selectable_label(ind.grid.is_none(), "off").clicked() {
                    set(app, "grid", Value::Null);
                }
                for m in SnapMode::ALL.iter().skip(1) {
                    if ui.selectable_label(ind.grid == Some(*m), m.label(units)).clicked() {
                        set(app, "grid", json!(m));
                    }
                }
            });
    });
    ui.add_space(2.0);
}

/// xfig's depth panel: All On / All Off / Toggle, Gray or Blank display of
/// depths that are off, and one checkbox row per depth in use with its
/// object count. Pressing a checkbox and dragging over others sets them all
/// to the new state of the first one.
pub fn depth_panel(app: &mut App, ui: &mut egui::Ui, tk: &Tokens) {
    ui.heading("Depths");
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (label, cmd, tip) in [
            ("All On", "depth.show_all", "Show and edit every depth"),
            ("All Off", "depth.hide_all", "Hide every depth"),
            ("Toggle", "depth.toggle_all", "Switch every depth"),
        ] {
            if ui.small_button(label).on_hover_text(tip).clicked() {
                app.run(cmd, json!({}));
            }
        }
    });
    ui.horizontal(|ui| {
        let gray = app.engine.gray_hidden_depths;
        if ui
            .radio(gray, "Gray")
            .on_hover_text("Draw depths that are off in gray")
            .clicked()
        {
            app.run("depth.set_display", json!({"mode": "gray"}));
        }
        if ui
            .radio(!gray, "Blank")
            .on_hover_text("Don't draw depths that are off")
            .clicked()
        {
            app.run("depth.set_display", json!({"mode": "blank"}));
        }
    });
    ui.separator();
    let counts = app.engine.depth_counts();
    let (pressed, down, pos) = ui.input(|i| {
        (
            i.pointer.primary_pressed(),
            i.pointer.primary_down(),
            i.pointer.interact_pos(),
        )
    });
    let mut sweep: Vec<i32> = Vec::new();
    egui::ScrollArea::vertical().show(ui, |ui| {
        if counts.is_empty() {
            ui.label(RichText::new("No objects").color(tk.muted));
        }
        let text = ui.visuals().text_color();
        for (d, n) in counts {
            let shown = !app.engine.hidden_depths.contains(&d);
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::click());
            let over = pos.is_some_and(|p| rect.contains(p) && ui.clip_rect().contains(p));
            if pressed && over && app.depth_drag.is_none() {
                app.depth_drag = Some(!shown);
            }
            if app.depth_drag.is_some_and(|target| down && over && shown != target) {
                sweep.push(d);
            }
            // Keyboard or accessibility clicks (a pointer press was handled above).
            if resp.clicked() && app.depth_drag.is_none() {
                app.run("depth.set_visible", json!({"depth": d, "visible": !shown}));
            }
            let p = ui.painter();
            if resp.hovered() {
                p.rect_filled(rect, 2.0, ui.visuals().widgets.hovered.weak_bg_fill);
            }
            let bx = egui::Rect::from_center_size(egui::pos2(rect.left() + 10.0, rect.center().y), vec2(13.0, 13.0));
            p.rect_stroke(bx, 2.0, egui::Stroke::new(1.0_f32, tk.muted), egui::StrokeKind::Inside);
            if shown {
                p.rect_filled(bx.shrink(3.0), 1.0, tk.selection);
            }
            let color = if shown { text } else { tk.muted };
            let font = egui::FontId::proportional(13.0);
            p.text(
                egui::pos2(bx.right() + 6.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                d.to_string(),
                font.clone(),
                color,
            );
            p.text(
                egui::pos2(rect.right() - 4.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                n.to_string(),
                egui::FontId::proportional(11.0),
                tk.muted,
            );
            let objects = if n == 1 { "object" } else { "objects" };
            resp.on_hover_text(format!("Depth {d}: {n} {objects}")).widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::Checkbox,
                    true,
                    shown,
                    format!("Depth {d} ({n} {objects})"),
                )
            });
        }
    });
    if let (Some(target), false) = (app.depth_drag, sweep.is_empty()) {
        app.run("depth.set_visible", json!({"depths": sweep, "visible": target}));
    }
    if !down {
        app.depth_drag = None;
    }
}

/// The classic xfig top strip: menus and the current file on the first row,
/// the message panel on the second, the mouse-function indicator and the
/// units box on the right.
pub fn classic_top(app: &mut App, ui: &mut egui::Ui, tk: &Tokens) {
    let right_w = INDICATOR_SIZE.x + 16.0;
    ui.horizontal(|ui| {
        let left_w = (ui.available_width() - right_w - 8.0).max(200.0);
        ui.allocate_ui_with_layout(vec2(left_w, 48.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                crate::menus::menu_buttons(app, ui);
                ui.separator();
                current_file(app, ui, tk);
            });
            message_panel(app, ui, tk);
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            ui.add_space(4.0);
            ui.vertical(|ui| {
                mouse_indicator(app, ui, tk);
                ui.horizontal(|ui| {
                    units_box(app, ui);
                    if let Some(c) = app.canvas.cursor {
                        let d = app.engine.doc();
                        ui.monospace(format!("{:>7.3} {:>7.3}", d.to_units(c.x), d.to_units(c.y)));
                    }
                    ui.label(RichText::new(format!("{:.0}%", app.engine.view.zoom * 100.0)).color(tk.muted));
                });
            });
        });
    });
}

fn current_file(app: &App, ui: &mut egui::Ui, tk: &Tokens) {
    let name = app
        .engine
        .path()
        .and_then(|p| std::path::Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("(untitled)");
    let text = if app.engine.is_modified() {
        format!("{name} (modified)")
    } else {
        name.to_string()
    };
    egui::Frame::NONE
        .stroke(egui::Stroke::new(1.0_f32, tk.muted))
        .inner_margin(vec2(6.0, 1.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).monospace())
                .on_hover_text(app.engine.path().unwrap_or("not saved yet"));
        });
}

fn message_panel(app: &mut App, ui: &mut egui::Ui, tk: &Tokens) {
    let (text, color) = match &app.message {
        Some(m) => (
            m.text.clone(),
            if m.error { tk.error } else { ui.visuals().text_color() },
        ),
        None => (format!("{} mode", app.engine.tool().label()), tk.muted),
    };
    let resp = egui::Frame::NONE
        .fill(ui.visuals().extreme_bg_color)
        .inner_margin(vec2(6.0, 2.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                egui::Label::new(RichText::new(text).color(color))
                    .truncate()
                    .sense(Sense::click()),
            )
        })
        .inner;
    if resp.on_hover_text("Click for the message history").clicked() {
        app.dialogs.messages_open = !app.dialogs.messages_open;
    }
}

const INDICATOR_SIZE: egui::Vec2 = vec2(380.0, 44.0);

/// xfig's mouse-function indicator: a small three-button mouse in the
/// middle, the left button's function left of it, the middle one's above
/// and the right one's to the right, each button with a tick pointing at its
/// label. Geometry ported from xfig's `w_mousefun.c` (xfig licence, see
/// LICENSE-xfig): buttons 4.5 % of the width wide and half the height high,
/// starting at 45 % of the height.
pub fn mouse_indicator(app: &App, ui: &mut egui::Ui, tk: &Tokens) {
    let h = app.engine.mouse_hints();
    let xfig = app.engine.mouse_scheme() == rustfig_engine::MouseScheme::Xfig;
    let (rect, resp) = ui.allocate_exact_size(INDICATOR_SIZE, Sense::hover());
    let p = ui.painter_at(rect);
    let stroke = egui::Stroke::new(1.0_f32, tk.muted);
    p.rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Inside);
    let text = ui.visuals().text_color();
    let small = egui::FontId::proportional(10.0);
    let font = egui::FontId::proportional(12.0);
    p.text(
        rect.left_top() + vec2(4.0, 2.0),
        egui::Align2::LEFT_TOP,
        if xfig {
            "Mouse Buttons"
        } else {
            "Mouse Buttons (modern)"
        },
        small.clone(),
        tk.muted,
    );
    if h.keyboard {
        p.text(
            rect.right_top() + vec2(-4.0, 2.0),
            egui::Align2::RIGHT_TOP,
            "keyboard",
            small,
            tk.caret,
        );
    }
    let (w, ht) = (rect.width(), rect.height());
    let but_w = (w * 0.045).round();
    let but_h = (ht * 0.5).round();
    let left_space = ((w - 4.0 * but_w) / 2.0).round();
    let top = rect.top() + (ht * 0.45).round();
    let x0 = rect.left() + left_space;
    let line = egui::Stroke::new(1.0_f32, text);
    for k in [0.0, 1.5, 3.0] {
        let r = egui::Rect::from_min_size(egui::pos2(x0 + k * but_w, top), vec2(but_w, but_h));
        p.rect_stroke(r, 1.0, line, egui::StrokeKind::Inside);
    }
    let mid_y = top + but_h / 2.0;
    p.line_segment([egui::pos2(x0, mid_y), egui::pos2(x0 - 5.0, mid_y)], line);
    let mid_x = x0 + 2.0 * but_w;
    p.line_segment([egui::pos2(mid_x, top), egui::pos2(mid_x, top - 5.0)], line);
    let right_x = x0 + 4.0 * but_w;
    p.line_segment([egui::pos2(right_x, mid_y), egui::pos2(right_x + 5.0, mid_y)], line);
    // Labels centred in the space beside the mouse (left, right) and above
    // it (middle), truncated to fit.
    let label = |s: &str, centre: egui::Pos2, max_w: f32| {
        let mut job = egui::text::LayoutJob::simple_singleline(s.to_string(), font.clone(), text);
        job.wrap = egui::text::TextWrapping::truncate_at_width(max_w);
        let g = ui.fonts_mut(|f| f.layout_job(job));
        p.galley(centre - g.size() / 2.0, g, text);
    };
    let side_w = left_space - 10.0;
    label(h.left, egui::pos2(rect.left() + left_space / 2.0, mid_y + 2.0), side_w);
    label(h.middle, egui::pos2(rect.center().x, rect.top() + 8.0), w - 2.0 * 90.0);
    label(
        h.right,
        egui::pos2(rect.right() - left_space / 2.0, mid_y + 2.0),
        side_w,
    );
    let summary = format!(
        "Left: {}; middle: {}; right: {}",
        if h.left.is_empty() { "—" } else { h.left },
        if h.middle.is_empty() { "—" } else { h.middle },
        if h.right.is_empty() { "—" } else { h.right }
    );
    let resp = resp.on_hover_text(format!(
        "{summary}\n\nTwo-button mouse or trackpad: Alt/Option+click is the middle button, \
         Ctrl+click the right button. View › xfig mouse buttons switches between xfig's \
         three-button scheme and the modern one."
    ));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, summary.clone()));
}

fn units_box(app: &mut App, ui: &mut egui::Ui) {
    let u = app.engine.doc().unit_name();
    if ui
        .button(format!("1 {u} = 1.00 {u}"))
        .on_hover_text("Units and page settings")
        .clicked()
    {
        app.dialogs.settings_open = true;
    }
}

pub fn status_bar(app: &mut App, ui: &mut egui::Ui, tk: &Tokens) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(app.engine.tool().label()).strong());
        ui.separator();
        let doc = app.engine.doc();
        if let Some(c) = app.canvas.cursor {
            ui.monospace(format!(
                "{:>8.3} {:>8.3} {}",
                doc.to_units(c.x),
                doc.to_units(c.y),
                doc.unit_name()
            ));
            ui.separator();
        }
        ui.label(format!("{:.0}%", app.engine.view.zoom * 100.0));
        ui.separator();
        let n = app.engine.selection().len();
        if n > 0 {
            ui.label(format!("{n} selected"));
            ui.separator();
        }
        if let Some(m) = &app.message {
            let c = if m.error { tk.error } else { ui.visuals().text_color() };
            ui.label(RichText::new(&m.text).color(c));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(format!("{} objects", app.engine.doc().leaf_count())).color(tk.muted));
        });
    });
}

/// RGB of a colour number, for UI swatches.
pub fn rgb_of(doc: &Document, c: i32) -> Rgb {
    doc.rgb(rustfig_model::ColorRef(c))
}

/// The standard colours (for tests and pickers).
pub fn standard_colors() -> &'static [Rgb; 32] {
    &STANDARD_COLORS
}
