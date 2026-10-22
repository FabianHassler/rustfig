//! Side and bottom panels: mode (tool) panel, attribute (indicator) panel,
//! depth manager and status bar.
//
// Ported from xfig src/w_modepanel.c (mode order, tooltips) and
// src/w_mousefun.c (mouse-function indicator); the depth panel follows
// src/w_layers.c and the Snap Mode indicator src/w_snap.c (xfig licence, see
// LICENSE-xfig).

use crate::theme::Tokens;
use crate::{xfig_icons, App};
use egui::{vec2, Color32, RichText, Sense, Stroke};
use rustfig_engine::ToolKind;
use rustfig_model::{Document, LineKind, Rgb, STANDARD_COLORS, STANDARD_COLOR_NAMES};
use serde_json::json;

/// A mode panel button: one of RustFig's tools with xfig's icon, or an xfig
/// mode RustFig doesn't have yet (shown greyed out, with the plan task).
/// RustFig's tools without an xfig mode (Select, Open/close, X-splines,
/// separate ellipse tools) are in the Tools menu only.
#[derive(Clone, Copy)]
enum Slot {
    Tool(ToolKind, &'static xfig_icons::Xbm),
    Missing(&'static xfig_icons::Xbm, &'static str, &'static str),
}

/// xfig's drawing modes in its order (`mode_switches` in `w_modepanel.c`).
/// The circle/ellipse modes are RustFig's circle tools, which draw ellipses
/// with the middle button as xfig does.
const DRAWING: [Slot; 15] = [
    Slot::Tool(ToolKind::CircleRadius, &xfig_icons::ELLRAD),
    Slot::Tool(ToolKind::CircleDiameter, &xfig_icons::ELLDIA),
    Slot::Tool(ToolKind::SplineApproxClosed, &xfig_icons::C_SPL),
    Slot::Tool(ToolKind::SplineApprox, &xfig_icons::SPL),
    Slot::Tool(ToolKind::SplineInterpClosed, &xfig_icons::C_INTSPL),
    Slot::Tool(ToolKind::SplineInterp, &xfig_icons::INTSPL),
    Slot::Tool(ToolKind::Polygon, &xfig_icons::POLYGON),
    Slot::Tool(ToolKind::Polyline, &xfig_icons::LINE),
    Slot::Tool(ToolKind::Box, &xfig_icons::BOX),
    Slot::Tool(ToolKind::ArcBox, &xfig_icons::ARC_BOX),
    Slot::Tool(ToolKind::RegularPolygon, &xfig_icons::REGPOLY),
    Slot::Tool(ToolKind::Arc, &xfig_icons::ARC),
    Slot::Tool(ToolKind::Picture, &xfig_icons::PICOBJ),
    Slot::Tool(ToolKind::Text, &xfig_icons::TEXT),
    Slot::Missing(&xfig_icons::LIBRARY, "Place a library object", "D6"),
];

/// xfig's editing modes in its order.
const EDITING: [Slot; 25] = [
    Slot::Tool(ToolKind::Glue, &xfig_icons::GLUE),
    Slot::Tool(ToolKind::Break, &xfig_icons::BREAK),
    Slot::Tool(ToolKind::OpenCompound, &xfig_icons::OPEN_COMP),
    Slot::Tool(ToolKind::JoinSplit, &xfig_icons::JOIN_SPLIT),
    Slot::Tool(ToolKind::Chop, &xfig_icons::CHOP),
    Slot::Tool(ToolKind::Scale, &xfig_icons::SCALE),
    Slot::Tool(ToolKind::Align, &xfig_icons::ALIGN),
    Slot::Tool(ToolKind::MovePoint, &xfig_icons::MOVEPT),
    Slot::Tool(ToolKind::Move, &xfig_icons::MOVE),
    Slot::Tool(ToolKind::AddPoint, &xfig_icons::ADDPT),
    Slot::Tool(ToolKind::Copy, &xfig_icons::COPY),
    Slot::Tool(ToolKind::DeletePoint, &xfig_icons::DELETEPT),
    Slot::Tool(ToolKind::Delete, &xfig_icons::DELETE),
    Slot::Tool(ToolKind::Update, &xfig_icons::UPDATE),
    Slot::Tool(ToolKind::Edit, &xfig_icons::EDIT),
    Slot::Tool(ToolKind::FlipV, &xfig_icons::FLIP_Y),
    Slot::Tool(ToolKind::FlipH, &xfig_icons::FLIP_X),
    Slot::Tool(ToolKind::RotateCw, &xfig_icons::ROTCW),
    Slot::Tool(ToolKind::RotateCcw, &xfig_icons::ROTCCW),
    Slot::Tool(ToolKind::Convert, &xfig_icons::CONVERT),
    Slot::Tool(ToolKind::Arrow, &xfig_icons::AUTOARROW),
    Slot::Missing(&xfig_icons::TANGENT, "Add tangent/normal to curve", "D4"),
    Slot::Missing(&xfig_icons::ANGLEMEAS, "Measure angle", "D5"),
    Slot::Missing(&xfig_icons::LENMEAS, "Measure length", "D5"),
    Slot::Missing(&xfig_icons::AREAMEAS, "Measure area", "D5"),
];

/// Width of the mode panel: two columns of buttons, the frame's margin and
/// stroke (nothing is clipped).
pub const MODE_PANEL_MARGIN: f32 = 3.0;
pub const MODE_PANEL_WIDTH: f32 = 2.0 * SLOT.x + 2.0 + 2.0 * MODE_PANEL_MARGIN + 2.0;

/// Rows of buttons in the mode panel: the drawing modes with the library row
/// and the editing modes.
const MODE_ROWS: f32 = 8.0 + 13.0;

/// The height of one mode button so that every row fits the panel (xfig's
/// window is sized to fit); only a very short window falls back to
/// scrolling.
fn slot_height(available: f32) -> f32 {
    // Two captions, the gap between the groups and the grid spacing.
    let overhead = 2.0 * 16.0 + 6.0 + 2.0 * MODE_ROWS + 4.0;
    ((available - overhead) / MODE_ROWS).floor().clamp(26.0, SLOT.y)
}

/// xfig's mode panel: its icons (`w_icons.c`, 36×32), two columns, the
/// active mode shown sunken.
pub fn mode_panel(app: &mut App, ui: &mut egui::Ui, tk: &Tokens) {
    let size = vec2(SLOT.x, slot_height(ui.available_height()));
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new("Drawing").small().color(tk.muted));
            slot_grid(app, ui, tk, "draw_tools", &DRAWING, size);
            ui.add_space(4.0);
            ui.label(RichText::new("Editing").small().color(tk.muted));
            slot_grid(app, ui, tk, "edit_tools", &EDITING, size);
        });
}

/// xfig's "Close Compound" popup (`popup_close_compound`, `e_compound.c`):
/// shown while a compound is open, with the two buttons that close it.
pub fn close_compound_popup(app: &mut App, ctx: &egui::Context) {
    if app.engine.open_compounds() == 0 {
        return;
    }
    egui::Window::new("Xfig: Close Compound")
        .collapsible(false)
        .resizable(false)
        .default_pos(egui::pos2(MODE_PANEL_WIDTH + 60.0, 90.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (label, id) in [
                    ("Close This Compound", "compound.close"),
                    ("Close All Compounds", "compound.close_all"),
                ] {
                    if ui.button(label).clicked() {
                        // xfig finishes a text being typed before closing.
                        if let Err(e) = app.engine.finish_text() {
                            app.error(e.message);
                        }
                        app.run(id, json!({}));
                    }
                }
            });
        });
}

/// xfig's mode menus popped up by Alt+wheel (`PopupModePanel` in
/// `w_modepanel.c`): the drawing or editing mode buttons at the pointer.
/// Picking a mode, Escape, a click elsewhere or the wheel the other way
/// (`Fig*draw_menu*translations`) closes it.
pub fn mode_popup(app: &mut App, ctx: &egui::Context, tk: &Tokens) {
    let Some((drawing, pos)) = app.mode_popup else { return };
    let tool = app.engine.tool();
    let resp = egui::Area::new(egui::Id::new("xfig_mode_popup"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                let title = if drawing { "Drawing modes" } else { "Editing modes" };
                ui.label(RichText::new(title).small().color(tk.muted));
                if drawing {
                    slot_grid(app, ui, tk, "popup_draw_tools", &DRAWING, SLOT);
                } else {
                    slot_grid(app, ui, tk, "popup_edit_tools", &EDITING, SLOT);
                }
            });
        })
        .response;
    let (escape, clicked_outside, wheel) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::Escape),
            i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !resp.rect.contains(p)),
            i.raw_scroll_delta.y,
        )
    });
    let away = (drawing && wheel < 0.0) || (!drawing && wheel > 0.0);
    if app.engine.tool() != tool || escape || clicked_outside || away {
        app.mode_popup = None;
    }
}

const SLOT: egui::Vec2 = vec2(38.0, 34.0);

fn slot_grid(app: &mut App, ui: &mut egui::Ui, tk: &Tokens, id: &str, slots: &[Slot], size: egui::Vec2) {
    // The library icon is twice as wide: it gets its own row below.
    let wide = |s: &Slot| matches!(s, Slot::Missing(icon, ..) if icon.width > 40);
    egui::Grid::new(id).spacing(vec2(2.0, 2.0)).show(ui, |ui| {
        for (i, slot) in slots.iter().filter(|s| !wide(s)).enumerate() {
            match slot {
                Slot::Missing(icon, tip, task) => missing_button(ui, tk, icon, tip, task, size),
                Slot::Tool(t, icon) => tool_button(app, ui, tk, *t, icon, size),
            }
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
    for slot in slots.iter().filter(|s| wide(s)) {
        if let Slot::Missing(icon, tip, task) = slot {
            missing_button(ui, tk, icon, tip, task, vec2(size.x * 2.0 + 2.0, size.y));
        }
    }
}

fn tool_button(
    app: &mut App,
    ui: &mut egui::Ui,
    tk: &Tokens,
    t: ToolKind,
    icon: &'static xfig_icons::Xbm,
    size: egui::Vec2,
) {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let active = app.engine.tool() == t;
    let state = crate::bevel::State::of(&resp, active, false);
    crate::bevel::paint(ui.painter(), rect, tk, state);
    let ink = if active { tk.icon_active } else { tk.icon };
    // A sunken button's picture sinks a pixel with it.
    let sunk = matches!(state, crate::bevel::State::Selected | crate::bevel::State::Pressed) && tk.bevel;
    let inner = if sunk { rect.translate(vec2(1.0, 1.0)) } else { rect };
    paint_mode_icon(ui, icon, inner, ink);
    let tip = match xfig_tip(t) {
        Some(tip) => tip.to_string(),
        None if t.shortcut().is_empty() => t.label().to_string(),
        None => format!("{} ({})", t.label(), t.shortcut()),
    };
    let label = t.label();
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, active, label));
    if resp.on_hover_text(tip).clicked() {
        app.run(t.id(), json!({}));
    }
}

/// The balloon text of xfig's mode buttons (`mode_switches`, `w_modepanel.c`).
fn xfig_tip(t: ToolKind) -> Option<&'static str> {
    Some(match t {
        ToolKind::CircleRadius => "CIRCLE/ELLIPSE drawing: specify RADII   (c or e)",
        ToolKind::CircleDiameter => "CIRCLE/ELLIPSE drawing: specify DIAMETER(S)   (Shift-c or e)",
        ToolKind::SplineApproxClosed => "CLOSED APPROXIMATED SPLINE drawing: specify control points   (Shift-s)",
        ToolKind::SplineApprox => "APPROXIMATED SPLINE drawing: specify control points   (s)",
        ToolKind::SplineInterpClosed => "CLOSED INTERPOLATED SPLINE drawing: specify control points   (Shift-i)",
        ToolKind::SplineInterp => "INTERPOLATED SPLINE drawing: specify control points   (i)",
        ToolKind::Polygon => "POLYGON drawing   (p)",
        ToolKind::Polyline => "POLYLINE drawing   (l)",
        ToolKind::Box => "Rectangular BOX drawing   (b)",
        ToolKind::ArcBox => "Rectangular BOX drawing with ROUNDED CORNERS   (Shift-b)",
        ToolKind::RegularPolygon => "Regular Polygon   (Shift-p)",
        ToolKind::Arc => "ARC drawing: specify three points on the arc   (r)",
        ToolKind::Picture => "Picture Object   (Ctrl-p)",
        ToolKind::Text => "TEXT input from keyboard   (t)",
        ToolKind::Glue => "GLUE objects into COMPOUND object   (g)",
        ToolKind::Break => "BREAK COMPOUND object   (Shift-g)",
        ToolKind::OpenCompound => "OPEN COMPOUND object   (o)",
        ToolKind::JoinSplit => "Join or Split lines/splines   (j)",
        ToolKind::Chop => "Chop objects   (x)",
        ToolKind::Scale => "SCALE objects   (Ctrl-s)",
        ToolKind::Align => "ALIGN objects within a COMPOUND or to CANVAS   (a)",
        ToolKind::MovePoint => "MOVE POINTs   (Shift-m)",
        ToolKind::Move => "MOVE objects   (m)",
        ToolKind::AddPoint => "ADD POINTs to lines, polygons and splines   (Ctrl-a)",
        ToolKind::Copy => "COPY objects  (Ctrl-c)",
        ToolKind::DeletePoint => "DELETE POINTs from lines, polygons and splines   (Shift-d)",
        ToolKind::Delete => "DELETE objects   (d)",
        ToolKind::Update => "UPDATE object <-> current settings   (u)",
        ToolKind::Edit => "CHANGE OBJECT via EDIT panel   (Ctrl-e)",
        ToolKind::FlipV => "FLIP objects up or down   (f)",
        ToolKind::FlipH => "FLIP objects left or right   (Shift-f)",
        ToolKind::RotateCw => "ROTATE objects clockwise   (Ctrl-r)",
        ToolKind::RotateCcw => "ROTATE objects counter-clockwise   (Shift-r)",
        ToolKind::Convert => "CONVERSION between lines, polygons and splines   (v)",
        ToolKind::Arrow => "ADD/DELETE ARROWHEADS   (Shift-a)",
        _ => return None,
    })
}

/// A mode icon: the vector redraw when the icon has been traced (W9), else
/// xfig's bitmap.
fn paint_mode_icon(ui: &egui::Ui, icon: &'static xfig_icons::Xbm, rect: egui::Rect, ink: Color32) {
    if crate::mode_icons::traced(icon.name) {
        crate::vector_icons::paint(ui, crate::vector_icons::VIcon::Traced(icon.name), rect, ink);
    } else {
        xfig_icons::paint(ui, icon, rect, ink);
    }
}

fn missing_button(
    ui: &mut egui::Ui,
    tk: &Tokens,
    icon: &'static xfig_icons::Xbm,
    tip: &str,
    task: &str,
    size: egui::Vec2,
) {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    crate::bevel::paint(ui.painter(), rect, tk, crate::bevel::State::Disabled);
    paint_mode_icon(ui, icon, rect, tk.muted.gamma_multiply(0.6));
    let text = format!("{tip} (not available yet, plan task {task})");
    resp.on_hover_text(text.clone())
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, false, text.clone()));
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

/// xfig's depth panel: All On / All Off / Toggle, Gray or Blank display of
/// depths that are off, and one checkbox row per depth in use with its
/// object count. Pressing a checkbox and dragging over others sets them all
/// to the new state of the first one.
pub fn depth_panel(app: &mut App, ui: &mut egui::Ui, tk: &Tokens) {
    // xfig's Snap Mode indicator sits above the depth panel (`w_snap.c`); here
    // it is also a chooser, opening the same modes as the Snap menu.
    let snap = app.engine.snap_indicator();
    ui.label(RichText::new("Snap Mode").strong());
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::click());
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp));
    crate::bevel::paint(ui.painter(), r, tk, crate::bevel::State::of(&resp, false, open));
    let armed = snap != "None";
    let ink = if armed { tk.selection } else { ui.visuals().text_color() };
    ui.painter().text(
        r.left_center() + vec2(7.0, 0.0),
        egui::Align2::LEFT_CENTER,
        snap,
        egui::FontId::proportional(12.0),
        ink,
    );
    // A drop-down arrow (drawn: the font has no glyph for it).
    let c = r.right_center() - vec2(10.0, 0.0);
    ui.painter().add(egui::Shape::convex_polygon(
        vec![c + vec2(-4.0, -2.0), c + vec2(4.0, -2.0), c + vec2(0.0, 3.0)],
        tk.muted,
        Stroke::NONE,
    ));
    let is_held = app.engine.snap_held();
    let held = if is_held { " (held)" } else { "" };
    if is_held {
        // xfig keeps this only in a variable; a tag makes it visible.
        let font = egui::FontId::proportional(10.0);
        let galley = ui.painter().layout_no_wrap("HOLD".to_string(), font, tk.icon_active);
        let tag = egui::Rect::from_center_size(
            egui::pos2(r.right() - 26.0 - galley.size().x / 2.0, r.center().y),
            galley.size() + vec2(8.0, 2.0),
        );
        ui.painter().rect_filled(tag, 2.0, tk.tool_active_bg);
        ui.painter()
            .galley(tag.center() - galley.size() / 2.0, galley, tk.icon_active);
    }
    egui::Popup::from_toggle_button_response(&resp)
        .align(egui::RectAlign::BOTTOM_START)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
        .show(|ui| {
            ui.set_min_width(130.0);
            if ui
                .add(egui::Button::new("None").selected(!armed))
                .on_hover_text("Don't snap to objects")
                .clicked()
            {
                app.run("snap.mode", json!({"mode": "none"}));
                ui.close();
            }
            ui.separator();
            crate::menus::snap_menu(app, ui);
        });
    let tip = if is_held {
        "Snap held: stays on after each point; Snap > Release to stop"
    } else {
        "Click to choose what the next point snaps to (as the Snap menu)"
    };
    resp.on_hover_text(tip)
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Snap Mode: {snap}{held}")));
    ui.separator();
    ui.heading("Depths");
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (label, cmd, tip) in [
            ("All On", "depth.show_all", "Show and edit every depth"),
            ("All Off", "depth.hide_all", "Hide every depth"),
            ("Toggle", "depth.toggle_all", "Switch every depth"),
        ] {
            if crate::bevel::text_button(ui, tk, label).on_hover_text(tip).clicked() {
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
            // xfig's right button sets the depth new objects get
            // (`SetDepthToLayer` in `w_layers.c`).
            if resp.secondary_clicked() {
                app.run("indicator.set", json!({"depth": d}));
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

/// The file-name box: xfig's is a label (`name_panel`); here a click turns
/// it into a text field. Enter renames the file (`file.rename`), Escape or
/// clicking away cancels.
fn current_file(app: &mut App, ui: &mut egui::Ui, tk: &Tokens) {
    let name = app
        .engine
        .path()
        .and_then(|p| std::path::Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("(untitled)")
        .to_string();
    let saved_name = app.engine.path().is_some();
    let full = app.engine.path().unwrap_or("not saved yet").to_string();
    let id = egui::Id::new("file_name_field");
    if let Some(text) = app.file_edit.as_mut() {
        let width = (text.chars().count() as f32 * 8.0 + 16.0).clamp(120.0, 360.0);
        let len = text.chars().count();
        let mut out = egui::TextEdit::singleline(text)
            .id(id)
            .font(egui::TextStyle::Monospace)
            .desired_width(width)
            .show(ui);
        let resp = out.response.clone();
        if !resp.has_focus() && !resp.lost_focus() {
            // First frame: focus the field with the whole name selected.
            resp.request_focus();
            out.state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(len),
            )));
            out.state.store(ui.ctx(), id);
        }
        let (enter, escape) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "File name"));
        resp.clone().on_hover_text(full);
        if escape {
            ui.memory_mut(|m| m.surrender_focus(id));
            app.file_edit = None;
        } else if resp.lost_focus() {
            let typed = app.file_edit.take().unwrap_or_default();
            if enter {
                crate::dialogs::rename_file(app, &typed, false);
            }
        }
        return;
    }
    let text = if app.engine.is_modified() {
        format!("{name} (modified)")
    } else {
        name.clone()
    };
    let galley = ui.painter().layout_no_wrap(
        text.clone(),
        egui::TextStyle::Monospace.resolve(ui.style()),
        ui.visuals().text_color(),
    );
    let size = galley.size() + vec2(14.0, 4.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    // A sunken field, as xfig's name panel, lighter while hovered (a click
    // edits it).
    let fill = if resp.hovered() {
        tk.face_hover
    } else {
        ui.visuals().extreme_bg_color
    };
    crate::bevel::field(ui.painter(), rect, tk, fill);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, ui.visuals().text_color());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text.clone()));
    let clicked = resp.clicked();
    resp.on_hover_text(format!("{full}\nClick to rename the file"));
    if clicked {
        // An unsaved figure starts with an empty field.
        app.file_edit = Some(if saved_name { name } else { String::new() });
    }
}

fn message_panel(app: &mut App, ui: &mut egui::Ui, tk: &Tokens) {
    let (text, color) = match &app.message {
        Some(m) => (
            m.text.clone(),
            if m.error { tk.error } else { ui.visuals().text_color() },
        ),
        // xfig's start-up message until a mode is chosen.
        None if app.engine.tool() == ToolKind::None => ("READY. Select a mode or load a file".to_string(), tk.muted),
        None => (format!("{} mode", app.engine.tool().label()), tk.muted),
    };
    let fill = if tk.bevel {
        tk.inset_bg
    } else {
        ui.visuals().extreme_bg_color
    };
    let slot = ui.painter().add(egui::Shape::Noop);
    let resp = egui::Frame::NONE.inner_margin(vec2(6.0, 2.0)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.add(
            egui::Label::new(RichText::new(text).color(color))
                .truncate()
                .sense(Sense::click()),
        )
    });
    let rect = resp.response.rect;
    ui.painter().set(slot, egui::Shape::rect_filled(rect, 0.0, fill));
    if tk.bevel {
        crate::bevel::field(ui.painter(), rect, tk, Color32::TRANSPARENT);
    }
    let resp = resp.inner;
    if resp.on_hover_text("Click for the message history").clicked() {
        app.dialogs.messages_open = !app.dialogs.messages_open;
    }
}

const INDICATOR_SIZE: egui::Vec2 = vec2(290.0, 46.0);

/// xfig's mouse-function indicator: a small three-button mouse in the
/// middle, the left button's function left of it, the middle one's above
/// and the right one's to the right, each button with a tick pointing at its
/// label. Geometry from xfig's `w_mousefun.c` (xfig licence, see
/// LICENSE-xfig): buttons 4.5 % of the width wide and half the height high,
/// starting at 45 % of the height; the box is compact and the left and
/// right labels sit next to the ticks (xfig centres them in the free space).
pub fn mouse_indicator(app: &App, ui: &mut egui::Ui, tk: &Tokens) {
    let mut h = app.engine.mouse_hints();
    if let Some((left, middle, right)) = app.hint_shown {
        h = rustfig_engine::MouseHints {
            left,
            middle,
            right,
            shift: "",
        };
    }
    let (rect, resp) = ui.allocate_exact_size(INDICATOR_SIZE, Sense::hover());
    let p = ui.painter_at(rect);
    crate::bevel::field(&p, rect, tk, if tk.bevel { tk.inset_bg } else { Color32::TRANSPARENT });
    let text = ui.visuals().text_color();
    let small = egui::FontId::proportional(10.0);
    let font = egui::FontId::proportional(12.0);
    p.text(
        rect.left_top() + vec2(5.0, 3.0),
        egui::Align2::LEFT_TOP,
        "Mouse Buttons",
        small,
        tk.muted,
    );
    let (w, ht) = (rect.width(), rect.height());
    let but_w = (w * 0.045).round();
    let but_h = (ht * 0.5).round();
    let left_space = ((w - 4.0 * but_w) / 2.0).round();
    let top = rect.top() + (ht * 0.45).round();
    let x0 = rect.left() + left_space;
    let line = egui::Stroke::new(1.0_f32, text);
    for k in [0.0, 1.5, 3.0] {
        let r = egui::Rect::from_min_size(egui::pos2(x0 + k * but_w, top), vec2(but_w, but_h));
        if tk.bevel {
            crate::bevel::paint(&p, r, tk, crate::bevel::State::Normal);
        } else {
            p.rect_stroke(r, 1.0, line, egui::StrokeKind::Inside);
        }
    }
    let mid_y = top + but_h / 2.0;
    let tick = 5.0;
    p.line_segment([egui::pos2(x0, mid_y), egui::pos2(x0 - tick, mid_y)], line);
    let mid_x = x0 + 2.0 * but_w;
    p.line_segment([egui::pos2(mid_x, top), egui::pos2(mid_x, top - tick)], line);
    let right_x = x0 + 4.0 * but_w;
    p.line_segment([egui::pos2(right_x, mid_y), egui::pos2(right_x + tick, mid_y)], line);
    // The left label ends at its tick, the right one starts at its tick, the
    // middle one is centred above its button; all truncated to fit.
    let gap = 3.0;
    let label = |s: &str, anchor: egui::Pos2, align: egui::Align2, max_w: f32| {
        let mut job = egui::text::LayoutJob::simple_singleline(s.to_string(), font.clone(), text);
        job.wrap = egui::text::TextWrapping::truncate_at_width(max_w.max(10.0));
        let g = ui.fonts_mut(|f| f.layout_job(job));
        let pos = align.anchor_size(anchor, g.size()).min;
        p.galley(pos, g, text);
    };
    let side_w = left_space - tick - gap - 4.0;
    label(
        h.left,
        egui::pos2(x0 - tick - gap, mid_y),
        egui::Align2::RIGHT_CENTER,
        side_w,
    );
    label(
        h.right,
        egui::pos2(right_x + tick + gap, mid_y),
        egui::Align2::LEFT_CENTER,
        side_w,
    );
    label(
        h.middle,
        egui::pos2(mid_x, top - tick - 1.0),
        egui::Align2::CENTER_BOTTOM,
        w - 2.0 * 80.0,
    );
    let summary = format!(
        "Left: {}; middle: {}; right: {}",
        if h.left.is_empty() { "—" } else { h.left },
        if h.middle.is_empty() { "—" } else { h.middle },
        if h.right.is_empty() { "—" } else { h.right }
    );
    let shift_note = if h.shift.is_empty() {
        String::new()
    } else {
        format!("\n{}", h.shift)
    };
    let resp = resp.on_hover_text(format!(
        "{summary}{shift_note}\n\nTwo-button mouse or trackpad: Alt/Option+click is the middle button, \
         Ctrl+click the right button (macOS); on Linux and Windows left and right together \
         are the middle button."
    ));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, summary.clone()));
}

/// RGB of a colour number, for UI swatches.
pub fn rgb_of(doc: &Document, c: i32) -> Rgb {
    doc.rgb(rustfig_model::ColorRef(c))
}

/// The standard colours (for tests and pickers).
pub fn standard_colors() -> &'static [Rgb; 32] {
    &STANDARD_COLORS
}
