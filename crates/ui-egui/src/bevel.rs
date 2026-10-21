//! The "3D" button style: raised, bevelled buttons (light top-left edge,
//! dark bottom-right edge, soft shading) that sink when pressed or selected,
//! a modern take on xfig's Xaw3d look. Colours come from [`Tokens`]; the
//! "Flat" style (Global settings) paints a plain face with a thin outline.
//! Edge widths are whole device pixels, so the bevel stays crisp at every
//! scale.

use crate::theme::Tokens;
use egui::emath::GuiRounding;
use egui::{Color32, Mesh, Painter, Rect, Response, Shape, Stroke, Ui};

/// How a button looks right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Normal,
    Hover,
    /// Mouse button held on it, or its menu is open.
    Pressed,
    /// The current mode or choice (stays sunken).
    Selected,
    /// Not available (raised, dimmed face).
    Disabled,
}

impl State {
    /// The state of a button from its response.
    pub fn of(resp: &Response, selected: bool, open: bool) -> State {
        if selected {
            State::Selected
        } else if open || resp.is_pointer_button_down_on() {
            State::Pressed
        } else if resp.hovered() {
            State::Hover
        } else {
            State::Normal
        }
    }

    fn sunken(self) -> bool {
        matches!(self, State::Pressed | State::Selected)
    }
}

/// One device pixel in points, at least one whole pixel per edge.
fn edge_width(painter: &Painter) -> f32 {
    let ppp = painter.ctx().pixels_per_point().max(0.5);
    ppp.round().max(1.0) / ppp
}

fn gradient(top: Color32, bottom: Color32, r: Rect) -> Shape {
    let mut m = Mesh::default();
    m.colored_vertex(r.left_top(), top);
    m.colored_vertex(r.right_top(), top);
    m.colored_vertex(r.right_bottom(), bottom);
    m.colored_vertex(r.left_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    Shape::mesh(m)
}

/// The face of a button of `state`.
fn face(tk: &Tokens, state: State) -> (Color32, Color32) {
    match state {
        State::Normal | State::Disabled => (tk.face_top, tk.face_bottom),
        State::Hover => (tk.face_hover, tk.face_hover),
        State::Pressed => (tk.face_pressed, tk.face_pressed),
        State::Selected => (tk.tool_active_bg, tk.tool_active_bg),
    }
}

fn rect_of(painter: &Painter, r: Rect) -> Rect {
    r.round_to_pixels(painter.ctx().pixels_per_point())
}

/// The four-sided bevel: `light` on top and left, `dark` on bottom and right,
/// `width` wide, inside `r`.
fn ring(out: &mut Vec<Shape>, r: Rect, width: f32, light: Color32, dark: Color32) {
    let (l, t, rt, b) = (r.left(), r.top(), r.right(), r.bottom());
    let rc = |x0: f32, y0: f32, x1: f32, y1: f32, c: Color32| {
        Shape::rect_filled(Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1)), 0.0, c)
    };
    out.push(rc(l, t, rt, t + width, light));
    out.push(rc(l, t, l + width, b, light));
    out.push(rc(l, b - width, rt, b, dark));
    out.push(rc(rt - width, t, rt, b, dark));
}

/// The bevel edges alone (raised, or sunken when `sunken`).
fn edges(painter: &Painter, r: Rect, tk: &Tokens, sunken: bool) -> Shape {
    let w = edge_width(painter);
    let mut out = Vec::with_capacity(8);
    let (lo, li, do_, di) = if sunken {
        (tk.edge_dark, tk.edge_dark_in, tk.edge_light, tk.edge_light_in)
    } else {
        (tk.edge_light, tk.edge_light_in, tk.edge_dark, tk.edge_dark_in)
    };
    ring(&mut out, r, w, lo, do_);
    ring(&mut out, r.shrink(w), w, li, di);
    Shape::Vec(out)
}

/// Paint a button's face and edges into `rect`.
pub fn paint(painter: &Painter, rect: Rect, tk: &Tokens, state: State) {
    let r = rect_of(painter, rect);
    if !tk.bevel {
        let fill = match state {
            State::Normal | State::Disabled => tk.face_top,
            _ => face(tk, state).0,
        };
        painter.rect_filled(r, 2.0, fill);
        painter.rect_stroke(r, 2.0, Stroke::new(1.0_f32, tk.muted), egui::StrokeKind::Inside);
        return;
    }
    let (top, bottom) = face(tk, state);
    painter.add(gradient(top, bottom, r));
    painter.add(edges(painter, r, tk, state.sunken()));
}

/// A sunken field (message panel, file name, indicator box): `fill` inside a
/// sunken bevel, or a thin outline in the flat style.
pub fn field(painter: &Painter, rect: Rect, tk: &Tokens, fill: Color32) {
    let r = rect_of(painter, rect);
    painter.rect_filled(r, if tk.bevel { 0.0 } else { 2.0 }, fill);
    if tk.bevel {
        painter.add(edges(painter, r, tk, true));
    } else {
        painter.rect_stroke(r, 2.0, Stroke::new(1.0_f32, tk.muted), egui::StrokeKind::Inside);
    }
}

/// Wrap an egui widget (a menu button, a small button) so it gets the 3D
/// face: the face is painted under what `add` draws and the edges over it.
/// `add` should return the widget's response; `open` tells whether its popup
/// is open (shown sunken).
pub fn around(
    ui: &mut Ui,
    tk: &Tokens,
    open: impl FnOnce(&Response) -> bool,
    add: impl FnOnce(&mut Ui) -> Response,
) -> Response {
    if !tk.bevel {
        return add(ui);
    }
    let slot = ui.painter().add(Shape::Noop);
    let resp = add(ui);
    let state = State::of(&resp, false, open(&resp));
    let r = rect_of(ui.painter(), resp.rect);
    let (top, bottom) = face(tk, state);
    ui.painter().set(slot, gradient(top, bottom, r));
    ui.painter().add(edges(ui.painter(), r, tk, state.sunken()));
    resp
}

/// Remove egui's own button faces in `ui` (the bevel paints them).
pub fn bare_buttons(ui: &mut Ui, tk: &Tokens) {
    if !tk.bevel {
        return;
    }
    let w = &mut ui.visuals_mut().widgets;
    for v in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        v.weak_bg_fill = Color32::TRANSPARENT;
        v.bg_fill = Color32::TRANSPARENT;
        v.bg_stroke = Stroke::NONE;
        v.expansion = 0.0;
        v.corner_radius = egui::CornerRadius::ZERO;
    }
    let sp = ui.spacing_mut();
    sp.button_padding = egui::vec2(8.0, 3.0);
}

/// A text button in the current style.
pub fn text_button(ui: &mut Ui, tk: &Tokens, label: &str) -> Response {
    let font = egui::FontId::proportional(12.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, ui.visuals().text_color());
    let size = galley.size() + egui::vec2(14.0, 6.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    paint(ui.painter(), rect, tk, State::of(&resp, false, false));
    let sink = if resp.is_pointer_button_down_on() {
        egui::vec2(1.0, 1.0)
    } else {
        egui::Vec2::ZERO
    };
    ui.painter().galley(
        rect.center() - galley.size() / 2.0 + sink,
        galley,
        ui.visuals().text_color(),
    );
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    resp
}
