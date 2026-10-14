//! xfig's attribute (indicator) panel. Ported from xfig's `w_indpanel.c`
//! and the mode masks in `w_modepanel.c` / `w_indpanel.h` (xfig licence,
//! see LICENSE-xfig): one button per attribute, in xfig's order, each with
//! its label and a picture of the current value; only the attributes the
//! current mode uses are shown. Left click opens a choice panel, the middle
//! button decreases and the right button increases the value (xfig's
//! "Menu / Dec/Prev / Inc/Next").

use crate::theme::Tokens;
use crate::App;
use egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Sense, Stroke, Ui};
use rustfig_engine::{Indicator, SnapMode, ToolKind};
use rustfig_model::{area_fill_color, CapStyle, ColorRef, Document, JoinStyle, Justify, LineKind, Rgb};
use serde_json::{json, Value};

/// The attribute buttons, in xfig's order (`ind_switches` in `w_indpanel.c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attr {
    Zoom,
    Grid,
    PointPosn,
    Depth,
    RotnAngle,
    NumSides,
    VAlign,
    HAlign,
    PenColor,
    FillColor,
    FillStyle,
    ArcType,
    LineWidth,
    LineStyle,
    JoinStyle,
    CapStyle,
    ArrowMode,
    ArrowType,
    ArrowSize,
    BoxRadius,
    TextJust,
    TextAngle,
    TextFlags,
    FontSize,
    TextStep,
    Font,
}

impl Attr {
    pub const ORDER: [Attr; 26] = [
        Attr::Zoom,
        Attr::Grid,
        Attr::PointPosn,
        Attr::Depth,
        Attr::RotnAngle,
        Attr::NumSides,
        Attr::VAlign,
        Attr::HAlign,
        Attr::PenColor,
        Attr::FillColor,
        Attr::FillStyle,
        Attr::ArcType,
        Attr::LineWidth,
        Attr::LineStyle,
        Attr::JoinStyle,
        Attr::CapStyle,
        Attr::ArrowMode,
        Attr::ArrowType,
        Attr::ArrowSize,
        Attr::BoxRadius,
        Attr::TextJust,
        Attr::TextAngle,
        Attr::TextFlags,
        Attr::FontSize,
        Attr::TextStep,
        Attr::Font,
    ];

    /// The button's label, one or two lines, as in xfig.
    pub fn label(self) -> (&'static str, &'static str) {
        match self {
            Attr::Zoom => ("Zoom", ""),
            Attr::Grid => ("Grid", "Mode"),
            Attr::PointPosn => ("Point", "Posn"),
            Attr::Depth => ("Depth", ""),
            Attr::RotnAngle => ("Rotn", "Angle"),
            Attr::NumSides => ("Num", "Sides"),
            Attr::VAlign => ("Vert", "Align"),
            Attr::HAlign => ("Horiz", "Align"),
            Attr::PenColor => ("PenColor", ""),
            Attr::FillColor => ("FillColor", ""),
            Attr::FillStyle => ("Fill", "Style"),
            Attr::ArcType => ("Arc", "Type"),
            Attr::LineWidth => ("Line", "Width"),
            Attr::LineStyle => ("Line", "Style"),
            Attr::JoinStyle => ("Join", "Style"),
            Attr::CapStyle => ("Cap", "Style"),
            Attr::ArrowMode => ("Arrow", "Mode"),
            Attr::ArrowType => ("Arrow", "Type"),
            Attr::ArrowSize => ("Arrow Size", ""),
            Attr::BoxRadius => ("Box", "Curve"),
            Attr::TextJust => ("Text", "Just"),
            Attr::TextAngle => ("Text/Ellipse", "Angle"),
            Attr::TextFlags => ("Text Flags", ""),
            Attr::FontSize => ("Text Size", ""),
            Attr::TextStep => ("Text", "Step"),
            Attr::Font => ("Text", "Font"),
        }
    }

    /// One-line name for menus, tooltips and accessibility.
    pub fn name(self) -> String {
        match self.label() {
            (a, "") => a.to_string(),
            (a, b) => format!("{a} {b}"),
        }
    }

    /// Button width (xfig's NARROW/DEF/WIDE/XWIDE/FONT widths).
    fn width(self) -> f32 {
        match self {
            Attr::Zoom | Attr::Depth | Attr::NumSides | Attr::TextStep => 58.0,
            Attr::RotnAngle | Attr::ArrowSize | Attr::TextFlags | Attr::FontSize => 90.0,
            Attr::PenColor | Attr::FillColor | Attr::TextAngle => 104.0,
            Attr::Font => 170.0,
            _ => 76.0,
        }
    }

    /// Whether the value is picked from a grid (the panel closes on a click)
    /// rather than entered (it stays open until a click outside).
    fn is_choice(self) -> bool {
        !matches!(
            self,
            Attr::Zoom
                | Attr::Depth
                | Attr::RotnAngle
                | Attr::NumSides
                | Attr::LineWidth
                | Attr::LineStyle
                | Attr::ArrowSize
                | Attr::BoxRadius
                | Attr::TextAngle
                | Attr::TextFlags
                | Attr::FontSize
                | Attr::TextStep
        )
    }
}

/// The attributes a mode shows (xfig's per-mode masks: `I_ELLIPSE`,
/// `I_CLOSED`, `I_OPEN`, `I_LINE`, `I_BOX`, … in `w_indpanel.h`). Zoom and
/// Grid Mode are always shown. Attributes whose feature RustFig doesn't
/// have yet (Angle Geom, Smart Links, copy counts, dimension lines, tangent
/// length) are left out until it does.
pub fn attributes_for(tool: ToolKind) -> Vec<Attr> {
    use Attr::*;
    const MIN2: &[Attr] = &[PointPosn];
    const LINE0: &[Attr] = &[FillStyle, LineStyle, LineWidth, PenColor, FillColor, Depth];
    const LINE1: &[Attr] = &[
        FillStyle, LineStyle, JoinStyle, LineWidth, PenColor, FillColor, Depth, CapStyle,
    ];
    const ARROWS: &[Attr] = &[ArrowMode, ArrowType, ArrowSize];
    const TEXT0: &[Attr] = &[TextJust, Font, FontSize, PenColor, Depth, TextAngle, TextFlags];
    let parts: &[&[Attr]] = match tool {
        ToolKind::CircleRadius | ToolKind::CircleDiameter | ToolKind::EllipseRadii | ToolKind::EllipseDiameter => {
            &[MIN2, LINE0, &[TextAngle]]
        }
        ToolKind::SplineApproxClosed | ToolKind::SplineInterpClosed | ToolKind::XSplineClosed | ToolKind::Polygon => {
            &[MIN2, LINE1]
        }
        ToolKind::SplineApprox | ToolKind::SplineInterp | ToolKind::XSpline => &[MIN2, LINE1, ARROWS],
        ToolKind::Polyline => &[MIN2, LINE1, ARROWS],
        ToolKind::Box => &[MIN2, LINE1],
        ToolKind::ArcBox => &[MIN2, LINE1, &[BoxRadius]],
        ToolKind::RegularPolygon => &[MIN2, LINE1, &[NumSides]],
        ToolKind::Arc => &[MIN2, LINE1, ARROWS, &[ArcType]],
        ToolKind::Picture => &[MIN2, &[Depth, PenColor]],
        ToolKind::Text => &[MIN2, &[TextStep], TEXT0],
        ToolKind::Glue
        | ToolKind::Scale
        | ToolKind::FlipV
        | ToolKind::FlipH
        | ToolKind::MovePoint
        | ToolKind::AddPoint
        | ToolKind::Move
        | ToolKind::Copy => &[MIN2],
        ToolKind::Align => &[&[HAlign, VAlign]],
        ToolKind::Update => &[TEXT0, LINE1, ARROWS, &[BoxRadius, ArcType]],
        // RustFig's own Select tool edits with the same attributes as Update.
        ToolKind::Select => &[MIN2, TEXT0, LINE1, ARROWS, &[BoxRadius, ArcType]],
        ToolKind::RotateCcw | ToolKind::RotateCw => &[MIN2, &[RotnAngle]],
        ToolKind::Arrow => &[&[LineWidth, ArrowType, ArrowSize]],
        ToolKind::Break
        | ToolKind::Delete
        | ToolKind::DeletePoint
        | ToolKind::Edit
        | ToolKind::Convert
        | ToolKind::OpenClose => &[],
    };
    let mut set: Vec<Attr> = parts.concat();
    set.extend([Zoom, Grid]);
    Attr::ORDER.iter().copied().filter(|a| set.contains(a)).collect()
}

/// Mouse hints while the pointer is over an attribute button (xfig's
/// `draw_mousefun_ind`).
pub const BUTTON_HINTS: (&str, &str, &str) = ("Menu", "Dec/Prev", "Inc/Next");

const BUTTON_HEIGHT: f32 = 34.0;

/// The attribute panel: one row of buttons, wrapping when the window is narrow.
pub fn attribute_panel(app: &mut App, ui: &mut Ui, tk: &Tokens) {
    let attrs = attributes_for(app.engine.tool());
    let mut hovered = false;
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
        for a in attrs {
            hovered |= button(app, ui, tk, a);
        }
    });
    ui.add_space(2.0);
    app.hint_override = hovered.then_some(BUTTON_HINTS);
}

/// One attribute button; returns whether the pointer is over it.
fn button(app: &mut App, ui: &mut Ui, tk: &Tokens, a: Attr) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(a.width(), BUTTON_HEIGHT), Sense::click());
    let visuals = ui.style().interact(&resp);
    let p = ui.painter();
    p.rect_filled(rect, 2.0, visuals.weak_bg_fill);
    p.rect_stroke(rect, 2.0, Stroke::new(1.0_f32, tk.muted), egui::StrokeKind::Inside);
    let text = ui.visuals().text_color();
    let font = egui::FontId::proportional(11.0);
    let (l1, l2) = a.label();
    p.text(
        rect.left_top() + vec2(4.0, 3.0),
        egui::Align2::LEFT_TOP,
        l1,
        font.clone(),
        text,
    );
    if !l2.is_empty() {
        p.text(
            rect.left_top() + vec2(4.0, 17.0),
            egui::Align2::LEFT_TOP,
            l2,
            font,
            text,
        );
    }
    // The value picture fills the right part of the button.
    let label_w = [l1, l2]
        .iter()
        .map(|s| s.chars().count() as f32 * 6.0)
        .fold(0.0_f32, f32::max)
        .min(a.width() - 24.0);
    let value_rect = Rect::from_min_max(
        pos2(rect.left() + label_w + 8.0, rect.top() + 3.0),
        pos2(rect.right() - 3.0, rect.bottom() - 3.0),
    );
    let one_line = l2.is_empty();
    let value_rect = if one_line && value_rect.width() < 24.0 {
        Rect::from_min_max(
            pos2(rect.left() + 4.0, rect.top() + 16.0),
            rect.right_bottom() - vec2(3.0, 2.0),
        )
    } else {
        value_rect
    };
    draw_value(app, ui, p, value_rect, rect, tk, a);
    let summary = format!("{}: {}", a.name(), value_text(app, a));
    if resp.secondary_clicked() {
        step(app, a, 1);
    } else if resp.middle_clicked() {
        step(app, a, -1);
    }
    let close = if a.is_choice() {
        egui::PopupCloseBehavior::CloseOnClick
    } else {
        egui::PopupCloseBehavior::CloseOnClickOutside
    };
    egui::Popup::from_toggle_button_response(&resp)
        .align(egui::RectAlign::TOP_START)
        .close_behavior(close)
        .show(|ui| choice_panel(app, ui, tk, a));
    let hovered = resp.hovered();
    resp.on_hover_text(format!(
        "{summary}\nClick: choose · middle button: previous/decrease · right button: next/increase"
    ))
    .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, summary.clone()));
    hovered
}

fn set(app: &mut App, v: Value) {
    app.run("indicator.set", v);
}

fn units(app: &App) -> rustfig_model::Units {
    app.engine.doc().settings.units
}

/// Colours offered: Default, the 32 standard colours, then user colours.
fn colors(doc: &Document) -> Vec<i32> {
    let mut v: Vec<i32> = (-1..32).collect();
    v.extend(doc.colors.iter().map(|c| c.number));
    v
}

fn color_name(c: i32) -> String {
    if c < 0 {
        "Default".to_string()
    } else if let Some(n) = usize::try_from(c)
        .ok()
        .and_then(|i| rustfig_model::STANDARD_COLOR_NAMES.get(i))
    {
        (*n).to_string()
    } else {
        format!("User {c}")
    }
}

fn color32(doc: &Document, c: i32) -> Color32 {
    let rgb = doc.rgb(ColorRef(c));
    Color32::from_rgb(rgb.r, rgb.g, rgb.b)
}

/// xfig's arrow type list: 0 is the stick arrow, then each type hollow and
/// filled. Returns (kind, style).
fn arrow_choice(n: i32) -> (i32, i32) {
    if n <= 0 {
        (0, 0)
    } else {
        ((n + 1) / 2, if n % 2 == 1 { 0 } else { 1 })
    }
}

fn arrow_choice_of(kind: i32, style: i32) -> i32 {
    if kind <= 0 {
        0
    } else {
        kind * 2 - 1 + i32::from(style == 1)
    }
}

/// Arrow types RustFig draws (0–9; xfig's 10–14 come with D9).
const ARROW_CHOICES: i32 = 19;

fn arrow_mode(ind: &Indicator) -> usize {
    match (ind.forward_arrow, ind.backward_arrow) {
        (false, false) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (true, true) => 3,
    }
}

fn set_arrow_mode(app: &mut App, m: usize) {
    let (f, b) = match m % 4 {
        1 => (true, false),
        2 => (false, true),
        3 => (true, true),
        _ => (false, false),
    };
    set(app, json!({"forward_arrow": f, "backward_arrow": b}));
}

const H_ALIGN: [&str; 4] = ["none", "left", "center", "right"];
const V_ALIGN: [&str; 4] = ["none", "top", "center", "bottom"];
const JUSTIFY: [Justify; 3] = [Justify::Left, Justify::Center, Justify::Right];
const JOINS: [JoinStyle; 3] = [JoinStyle::Miter, JoinStyle::Round, JoinStyle::Bevel];
const CAPS: [CapStyle; 3] = [CapStyle::Butt, CapStyle::Round, CapStyle::Projecting];

/// Grid choices: off, then the snap steps other than "any".
fn grid_choices() -> Vec<Option<SnapMode>> {
    std::iter::once(None)
        .chain(SnapMode::ALL.iter().skip(1).map(|m| Some(*m)))
        .collect()
}

fn cycle<T: PartialEq + Copy>(list: &[T], cur: T, dir: i32) -> Option<T> {
    let n = list.len() as i32;
    if n == 0 {
        return None;
    }
    let i = list.iter().position(|x| *x == cur).map_or(0, |i| i as i32);
    list.get((i + dir).rem_euclid(n) as usize).copied()
}

/// The value as text (tooltips, accessibility, numeric buttons).
fn value_text(app: &App, a: Attr) -> String {
    let ind = &app.engine.indicator;
    let u = units(app);
    match a {
        Attr::Zoom => format!("{:.2}", app.engine.view.zoom),
        Attr::Grid => ind.grid.map_or("none", |g| g.label(u)).to_string(),
        Attr::PointPosn => ind.snap.label(u).to_string(),
        Attr::Depth => ind.depth.to_string(),
        Attr::RotnAngle => format!("{:.1}", ind.rotate_angle),
        Attr::NumSides => ind.sides.to_string(),
        Attr::VAlign => ind.align_v.clone(),
        Attr::HAlign => ind.align_h.clone(),
        Attr::PenColor => color_name(ind.pen_color),
        Attr::FillColor => color_name(ind.fill_color),
        Attr::FillStyle => match ind.area_fill {
            -1 => "none".into(),
            f @ 0..=20 => format!("shade {f}"),
            f @ 21..=40 => format!("tint {f}"),
            f => format!("pattern {f}"),
        },
        Attr::ArcType => if ind.pie_arc { "pie wedge" } else { "open" }.into(),
        Attr::LineWidth => ind.thickness.to_string(),
        Attr::LineStyle => ind.line.name().into(),
        Attr::JoinStyle => format!("{:?}", ind.join).to_lowercase(),
        Attr::CapStyle => format!("{:?}", ind.cap).to_lowercase(),
        Attr::ArrowMode => ["none", "forward", "backward", "both"]
            .get(arrow_mode(ind))
            .copied()
            .unwrap_or("none")
            .into(),
        Attr::ArrowType => {
            let a = ind.arrow;
            if a.kind == 0 {
                "type 0".into()
            } else {
                format!("{} type {}", if a.style == 1 { "solid" } else { "hollow" }, a.kind)
            }
        }
        Attr::ArrowSize => format!(
            "{:.1} × {:.1}",
            ind.arrow.width / rustfig_geom::FIG_PER_STYLE_UNIT,
            ind.arrow.height / rustfig_geom::FIG_PER_STYLE_UNIT
        ),
        Attr::BoxRadius => ind.box_radius.to_string(),
        Attr::TextJust => format!("{:?}", ind.justify).to_lowercase(),
        Attr::TextAngle => format!("{:.1}", ind.text_angle),
        Attr::TextFlags => {
            let mut f = Vec::new();
            if ind.hidden_text {
                f.push("hidden");
            }
            if ind.rigid_text {
                f.push("rigid");
            }
            if ind.special_text {
                f.push("TeX");
            }
            if f.is_empty() {
                "normal".into()
            } else {
                f.join(", ")
            }
        }
        Attr::FontSize => format!("{}", ind.font_size),
        Attr::TextStep => format!("{:.1}", ind.text_step),
        Attr::Font => font_name(ind.font, ind.postscript),
    }
}

fn font_name(font: i32, postscript: bool) -> String {
    if postscript {
        rustfig_text::ps_font_display_name(font).to_string()
    } else {
        format!("LaTeX {}", rustfig_text::latex_font_name(font))
    }
}

/// Middle button (dir −1) and right button (dir +1).
fn step(app: &mut App, a: Attr, dir: i32) {
    let ind = app.engine.indicator.clone();
    let d = f64::from(dir);
    match a {
        Attr::Zoom => {
            app.run(if dir > 0 { "view.zoom_in" } else { "view.zoom_out" }, json!({}));
        }
        Attr::Grid => {
            if let Some(g) = cycle(&grid_choices(), ind.grid, dir) {
                set(app, json!({ "grid": g }));
            }
        }
        Attr::PointPosn => {
            if let Some(m) = cycle(&SnapMode::ALL, ind.snap, dir) {
                set(app, json!({ "snap": m }));
            }
        }
        Attr::Depth => set(
            app,
            json!({"depth": (ind.depth + dir).clamp(0, rustfig_model::MAX_DEPTH)}),
        ),
        Attr::RotnAngle => set(
            app,
            json!({"rotate_angle": (ind.rotate_angle + 15.0 * d).clamp(-360.0, 360.0)}),
        ),
        Attr::NumSides => set(app, json!({"sides": (ind.sides as i64 + i64::from(dir)).clamp(3, 200)})),
        Attr::VAlign => {
            if let Some(v) = cycle(&V_ALIGN, ind.align_v.as_str(), dir) {
                set(app, json!({ "align_v": v }));
            }
        }
        Attr::HAlign => {
            if let Some(v) = cycle(&H_ALIGN, ind.align_h.as_str(), dir) {
                set(app, json!({ "align_h": v }));
            }
        }
        Attr::PenColor | Attr::FillColor => {
            let field = if a == Attr::PenColor { "pen_color" } else { "fill_color" };
            let cur = if a == Attr::PenColor {
                ind.pen_color
            } else {
                ind.fill_color
            };
            if let Some(c) = cycle(&colors(app.engine.doc()), cur, dir) {
                let mut m = serde_json::Map::new();
                m.insert(field.into(), json!(c));
                set(app, Value::Object(m));
            }
        }
        // xfig darkens and lightens the fill one step at a time.
        Attr::FillStyle => set(app, json!({"area_fill": (ind.area_fill + dir).clamp(-1, 62)})),
        Attr::ArcType => set(app, json!({"pie_arc": !ind.pie_arc})),
        Attr::LineWidth => set(app, json!({"thickness": (ind.thickness + dir).clamp(0, 500)})),
        Attr::LineStyle => {
            if let Some(k) = cycle(&LineKind::ALL, ind.line, dir) {
                set(app, json!({ "line": k }));
            }
        }
        Attr::JoinStyle => {
            if let Some(j) = cycle(&JOINS, ind.join, dir) {
                set(app, json!({ "join": j }));
            }
        }
        Attr::CapStyle => {
            if let Some(c) = cycle(&CAPS, ind.cap, dir) {
                set(app, json!({ "cap": c }));
            }
        }
        Attr::ArrowMode => set_arrow_mode(app, (arrow_mode(&ind) as i32 + dir).rem_euclid(4) as usize),
        Attr::ArrowType => {
            let n = (arrow_choice_of(ind.arrow.kind, ind.arrow.style) + dir).rem_euclid(ARROW_CHOICES);
            set_arrow_type(app, &ind, n);
        }
        Attr::BoxRadius => set(app, json!({"box_radius": (ind.box_radius + dir).clamp(0, 1000)})),
        Attr::TextJust => {
            if let Some(j) = cycle(&JUSTIFY, ind.justify, dir) {
                set(app, json!({ "justify": j }));
            }
        }
        Attr::TextAngle => set(
            app,
            json!({"text_angle": (ind.text_angle + 15.0 * d).clamp(-360.0, 360.0)}),
        ),
        Attr::FontSize => set(app, json!({"font_size": (ind.font_size + d).clamp(1.0, 500.0)})),
        Attr::TextStep => set(
            app,
            json!({"text_step": ((ind.text_step + 0.1 * d) * 10.0).round() / 10.0}),
        ),
        Attr::Font => {
            let count = if ind.postscript {
                rustfig_text::PS_FONT_COUNT
            } else {
                rustfig_text::LATEX_FONT_COUNT
            };
            set(app, json!({"font": (ind.font + dir).rem_euclid(count.max(1))}));
        }
        // These open panels only.
        Attr::ArrowSize | Attr::TextFlags => {}
    }
}

fn set_arrow_type(app: &mut App, ind: &Indicator, n: i32) {
    let (kind, style) = arrow_choice(n);
    let mut a = ind.arrow;
    a.kind = kind;
    a.style = style;
    set(app, json!({ "arrow": a }));
}

// ---- value pictures ------------------------------------------------------

fn draw_value(app: &App, ui: &Ui, p: &Painter, r: Rect, button: Rect, tk: &Tokens, a: Attr) {
    let ind = &app.engine.indicator;
    let doc = app.engine.doc();
    let ink = ui.visuals().text_color();
    let small = egui::FontId::proportional(11.0);
    let text_right = |s: &str| {
        p.text(r.right_center(), egui::Align2::RIGHT_CENTER, s, small.clone(), ink);
    };
    match a {
        Attr::PenColor | Attr::FillColor => {
            let c = if a == Attr::PenColor {
                ind.pen_color
            } else {
                ind.fill_color
            };
            let sw = Rect::from_min_size(pos2(r.right() - 16.0, r.center().y - 8.0), vec2(16.0, 16.0));
            p.rect_filled(sw, 1.0, color32(doc, c));
            p.rect_stroke(sw, 1.0, Stroke::new(1.0_f32, tk.muted), egui::StrokeKind::Inside);
            // The name goes on the label's second line, like xfig.
            p.text(
                button.left_top() + vec2(4.0, 17.0),
                egui::Align2::LEFT_TOP,
                color_name(c),
                small,
                ink,
            );
        }
        Attr::FillStyle => {
            let sw = Rect::from_center_size(r.center(), vec2(r.height().min(26.0), r.height().min(26.0)));
            fill_swatch(p, sw, doc, ind.fill_color, ind.area_fill, ink, tk);
        }
        Attr::LineWidth => {
            text_right(&ind.thickness.to_string());
            let y = r.center().y;
            let w = (ind.thickness as f32).clamp(0.5, 8.0);
            p.line_segment([pos2(r.left(), y), pos2(r.right() - 16.0, y)], Stroke::new(w, ink));
        }
        Attr::LineStyle => line_sample(p, r, ind.line, ind.style_val, ink),
        Attr::JoinStyle => join_sample(p, r, ind.join, ink, ui.visuals().widgets.inactive.weak_bg_fill),
        Attr::CapStyle => cap_sample(p, r, ind.cap, ink, tk),
        Attr::ArrowMode => arrow_mode_sample(p, r, arrow_mode(ind), ink),
        Attr::ArrowType => arrow_sample(
            p,
            r,
            ind.arrow.kind,
            ind.arrow.style,
            ink,
            ui.visuals().extreme_bg_color,
        ),
        Attr::ArcType => arc_sample(p, r, ind.pie_arc, ink),
        Attr::TextJust => just_sample(p, r, ind.justify, ink),
        _ => text_right(&value_text(app, a)),
    }
}

/// A swatch of an area fill: none, a shade or tint of the fill colour, or a
/// pattern drawn in the pen colour over the fill colour.
fn fill_swatch(p: &Painter, r: Rect, doc: &Document, fill_color: i32, area_fill: i32, ink: Color32, tk: &Tokens) {
    let stroke = Stroke::new(1.0_f32, tk.muted);
    match area_fill {
        -1 => {
            p.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                "NONE",
                egui::FontId::proportional(9.0),
                ink,
            );
        }
        f @ 0..=40 => {
            let base = doc.rgb(ColorRef(fill_color));
            let rgb = area_fill_color(base, fill_color < 1, fill_color == 7, f).unwrap_or(base);
            p.rect_filled(r, 0.0, Color32::from_rgb(rgb.r, rgb.g, rgb.b));
        }
        kind => {
            let bg: Rgb = doc.rgb(ColorRef(fill_color));
            p.rect_filled(r, 0.0, Color32::from_rgb(bg.r, bg.g, bg.b));
            if let Some((tw, th, lines)) = rustfig_render::pattern::tile(kind) {
                // Two tiles across the swatch.
                let s = r.width() as f64 / (2.0 * tw.max(1.0));
                let painter = p.with_clip_rect(r);
                for ty in 0..(r.height() as f64 / (th * s)).ceil() as i32 + 1 {
                    for tx in 0..3 {
                        for l in &lines {
                            let pts: Vec<Pos2> = l
                                .iter()
                                .map(|q| {
                                    pos2(
                                        r.left() + ((q.x + f64::from(tx) * tw) * s) as f32,
                                        r.top() + ((q.y + f64::from(ty) * th) * s) as f32,
                                    )
                                })
                                .collect();
                            painter.add(egui::Shape::line(pts, Stroke::new(1.0_f32, Color32::BLACK)));
                        }
                    }
                }
            }
        }
    }
    p.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Inside);
}

fn line_sample(p: &Painter, r: Rect, kind: LineKind, style_val: f64, ink: Color32) {
    let y = r.center().y;
    let pts = [pos2(r.left(), y), pos2(r.right(), y)];
    // Dash lengths at about one pixel per 1/80 inch.
    let px = |v: f64| (v / rustfig_geom::FIG_PER_STYLE_UNIT).max(1.0) as f32;
    let pat = kind.dash_pattern(style_val, 15.0);
    if pat.is_empty() {
        p.line_segment(pts, Stroke::new(1.5_f32, ink));
        return;
    }
    let dashes: Vec<f32> = pat.iter().step_by(2).map(|v| px(*v)).collect();
    let gaps: Vec<f32> = pat.iter().skip(1).step_by(2).map(|v| px(*v)).collect();
    p.extend(egui::Shape::dashed_line_with_offset(
        &pts,
        Stroke::new(1.5_f32, ink),
        &dashes,
        &gaps,
        0.0,
    ));
}

fn join_sample(p: &Painter, r: Rect, join: JoinStyle, ink: Color32, bg: Color32) {
    let h = r.height().min(22.0);
    let c = r.center();
    let tip = pos2(c.x + h * 0.5, c.y);
    let a = pos2(c.x - h * 0.5, c.y - h * 0.4);
    let b = pos2(c.x - h * 0.5, c.y + h * 0.4);
    let w = 5.0;
    p.add(egui::Shape::line(vec![a, tip, b], Stroke::new(w, ink)));
    match join {
        JoinStyle::Miter => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    pos2(tip.x - 2.0, tip.y - 3.0),
                    pos2(tip.x + 4.0, tip.y),
                    pos2(tip.x - 2.0, tip.y + 3.0),
                ],
                ink,
                Stroke::NONE,
            ));
        }
        JoinStyle::Round => {
            p.circle_filled(tip, w / 2.0 + 0.5, ink);
        }
        JoinStyle::Bevel => {
            p.rect_filled(
                Rect::from_min_max(pos2(tip.x + 0.5, tip.y - 4.0), pos2(tip.x + 4.0, tip.y + 4.0)),
                0.0,
                bg,
            );
        }
    }
}

fn cap_sample(p: &Painter, r: Rect, cap: CapStyle, ink: Color32, tk: &Tokens) {
    let y = r.center().y;
    let (x0, x1) = (r.left() + 6.0, r.right() - 6.0);
    let w = 7.0;
    let ext = if cap == CapStyle::Projecting { w / 2.0 } else { 0.0 };
    p.rect_filled(
        Rect::from_min_max(pos2(x0 - ext, y - w / 2.0), pos2(x1 + ext, y + w / 2.0)),
        0.0,
        ink,
    );
    if cap == CapStyle::Round {
        p.circle_filled(pos2(x0, y), w / 2.0, ink);
        p.circle_filled(pos2(x1, y), w / 2.0, ink);
    }
    // The end points, as xfig's cap pictures mark them.
    let mark = Stroke::new(1.0_f32, tk.selection);
    p.line_segment([pos2(x0, y - w), pos2(x0, y + w)], mark);
    p.line_segment([pos2(x1, y - w), pos2(x1, y + w)], mark);
}

fn arrowhead_shape(p: &Painter, tip: Pos2, from: Pos2, kind: i32, style: i32, ink: Color32, bg: Color32) {
    use rustfig_geom::{ArrowFill, Point};
    let Some(shape) = rustfig_geom::arrowhead(
        Point::new(f64::from(tip.x), f64::from(tip.y)),
        Point::new(f64::from(from.x), f64::from(from.y)),
        kind,
        style,
        8.0,
        10.0,
    ) else {
        return;
    };
    let pts: Vec<Pos2> = shape.outline.iter().map(|q| pos2(q.x as f32, q.y as f32)).collect();
    let stroke = Stroke::new(1.0_f32, ink);
    match (shape.closed, shape.fill) {
        (false, _) | (_, ArrowFill::None) => {
            p.add(egui::Shape::line(pts, stroke));
        }
        (true, fill) => {
            let f = if fill == ArrowFill::Pen { ink } else { bg };
            p.add(egui::Shape::Path(egui::epaint::PathShape {
                points: pts,
                closed: true,
                fill: f,
                stroke: stroke.into(),
            }));
        }
    }
}

fn arrow_sample(p: &Painter, r: Rect, kind: i32, style: i32, ink: Color32, bg: Color32) {
    let y = r.center().y;
    let (x0, x1) = (r.left() + 2.0, r.right() - 4.0);
    p.line_segment([pos2(x0, y), pos2(x1 - 6.0, y)], Stroke::new(1.0_f32, ink));
    arrowhead_shape(p, pos2(x1, y), pos2(x0, y), kind, style, ink, bg);
}

fn arrow_mode_sample(p: &Painter, r: Rect, mode: usize, ink: Color32) {
    let y = r.center().y;
    let (x0, x1) = (r.left() + 2.0, r.right() - 2.0);
    p.line_segment([pos2(x0, y), pos2(x1, y)], Stroke::new(1.0_f32, ink));
    let head = |tip: Pos2, dir: f32| {
        p.add(egui::Shape::line(
            vec![
                pos2(tip.x - 6.0 * dir, tip.y - 4.0),
                tip,
                pos2(tip.x - 6.0 * dir, tip.y + 4.0),
            ],
            Stroke::new(1.0_f32, ink),
        ));
    };
    if mode == 1 || mode == 3 {
        head(pos2(x1, y), 1.0);
    }
    if mode == 2 || mode == 3 {
        head(pos2(x0, y), -1.0);
    }
}

fn arc_sample(p: &Painter, r: Rect, pie: bool, ink: Color32) {
    let c = pos2(r.center().x, r.bottom() - 2.0);
    let rad = (r.height() - 4.0).min(r.width() / 2.0 - 2.0).max(4.0);
    let pts: Vec<Pos2> = (0..=16)
        .map(|i| {
            let t = std::f32::consts::PI * (i as f32 / 16.0);
            pos2(c.x + rad * t.cos(), c.y - rad * t.sin())
        })
        .collect();
    let stroke = Stroke::new(1.0_f32, ink);
    if pie {
        let mut closed = pts.clone();
        closed.push(c);
        p.add(egui::Shape::closed_line(closed, stroke));
    } else {
        p.add(egui::Shape::line(pts, stroke));
    }
}

fn just_sample(p: &Painter, r: Rect, j: Justify, ink: Color32) {
    let w = r.width().min(26.0);
    for (k, frac) in [1.0_f32, 0.6, 0.8].iter().enumerate() {
        let lw = w * frac;
        let y = r.top() + 6.0 + k as f32 * 7.0;
        let x0 = match j {
            Justify::Left => r.right() - w,
            Justify::Center => r.right() - w / 2.0 - lw / 2.0,
            Justify::Right => r.right() - lw,
        };
        p.line_segment([pos2(x0, y), pos2(x0 + lw, y)], Stroke::new(2.0_f32, ink));
    }
}

// ---- choice panels ---------------------------------------------------------

/// A clickable cell with a picture; returns whether it was clicked.
fn cell(ui: &mut Ui, size: egui::Vec2, selected: bool, name: &str, paint: impl FnOnce(&Painter, Rect)) -> bool {
    let (r, resp) = ui.allocate_exact_size(size, Sense::click());
    let v = ui.style().interact_selectable(&resp, selected);
    ui.painter().rect_filled(r, 2.0, v.weak_bg_fill);
    paint(ui.painter(), r.shrink(3.0));
    if selected {
        ui.painter().rect_stroke(
            r,
            2.0,
            Stroke::new(2.0_f32, ui.visuals().selection.stroke.color),
            egui::StrokeKind::Inside,
        );
    }
    let clicked = resp.clicked();
    resp.on_hover_text(name)
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, name));
    clicked
}

fn choice_panel(app: &mut App, ui: &mut Ui, tk: &Tokens, a: Attr) {
    ui.label(egui::RichText::new(a.name()).strong());
    let ind = app.engine.indicator.clone();
    let ink = ui.visuals().text_color();
    let bg = ui.visuals().extreme_bg_color;
    match a {
        Attr::Zoom => {
            let mut z = app.engine.view.zoom;
            if ui
                .add(
                    egui::DragValue::new(&mut z)
                        .range(0.05..=50.0)
                        .speed(0.01)
                        .fixed_decimals(2),
                )
                .changed()
            {
                app.run("view.zoom_set", json!({ "zoom": z }));
            }
            ui.horizontal(|ui| {
                if ui.button("Fit to canvas").clicked() {
                    app.run("view.zoom_fit", json!({}));
                }
                if ui.button("1.00").clicked() {
                    app.run("view.zoom_set", json!({ "zoom": 1.0 }));
                }
            });
        }
        Attr::Grid => {
            let u = units(app);
            for g in grid_choices() {
                let name = g.map_or("none", |m| m.label(u));
                if ui.selectable_label(ind.grid == g, name).clicked() {
                    set(app, json!({ "grid": g }));
                }
            }
        }
        Attr::PointPosn => {
            let u = units(app);
            for m in SnapMode::ALL {
                if ui.selectable_label(ind.snap == m, m.label(u)).clicked() {
                    set(app, json!({ "snap": m }));
                }
            }
        }
        Attr::VAlign | Attr::HAlign => {
            let (list, cur, field) = if a == Attr::VAlign {
                (V_ALIGN, ind.align_v.as_str(), "align_v")
            } else {
                (H_ALIGN, ind.align_h.as_str(), "align_h")
            };
            for v in list {
                if ui.selectable_label(cur == v, v).clicked() {
                    let mut m = serde_json::Map::new();
                    m.insert(field.into(), json!(v));
                    set(app, Value::Object(m));
                }
            }
        }
        Attr::PenColor | Attr::FillColor => {
            let field = if a == Attr::PenColor { "pen_color" } else { "fill_color" };
            let cur = if a == Attr::PenColor {
                ind.pen_color
            } else {
                ind.fill_color
            };
            let doc_colors = colors(app.engine.doc());
            let mut chosen = None;
            egui::Grid::new(("colors", field))
                .spacing(vec2(2.0, 2.0))
                .show(ui, |ui| {
                    for (i, c) in doc_colors.iter().enumerate() {
                        let col = color32(app.engine.doc(), *c);
                        let name = color_name(*c);
                        if cell(ui, vec2(34.0, 20.0), *c == cur, &name, |p, r| {
                            p.rect_filled(r, 1.0, col);
                            if *c < 0 {
                                p.text(
                                    r.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "Def",
                                    egui::FontId::proportional(9.0),
                                    Color32::GRAY,
                                );
                            }
                        }) {
                            chosen = Some(*c);
                        }
                        if i % 7 == 6 {
                            ui.end_row();
                        }
                    }
                });
            if let Some(c) = chosen {
                let mut m = serde_json::Map::new();
                m.insert(field.into(), json!(c));
                set(app, Value::Object(m));
            }
        }
        Attr::FillStyle => {
            let mut chosen = None;
            let doc = app.engine.doc();
            ui.label("Fill Style");
            egui::Grid::new("fill_shades").spacing(vec2(2.0, 2.0)).show(ui, |ui| {
                for (i, f) in (-1..=40).enumerate() {
                    let name = if f < 0 { "none".to_string() } else { format!("{f}") };
                    if cell(ui, vec2(26.0, 26.0), ind.area_fill == f, &name, |p, r| {
                        fill_swatch(p, r, doc, ind.fill_color, f, ink, tk)
                    }) {
                        chosen = Some(f);
                    }
                    if i % 11 == 10 {
                        ui.end_row();
                    }
                }
            });
            ui.label("Patterns");
            egui::Grid::new("fill_patterns").spacing(vec2(2.0, 2.0)).show(ui, |ui| {
                for (i, f) in (41..=62).enumerate() {
                    let name = rustfig_render::pattern::PATTERN_NAMES
                        .get(i)
                        .copied()
                        .unwrap_or("pattern");
                    if cell(ui, vec2(26.0, 26.0), ind.area_fill == f, name, |p, r| {
                        fill_swatch(p, r, doc, 7, f, ink, tk)
                    }) {
                        chosen = Some(f);
                    }
                    if i % 11 == 10 {
                        ui.end_row();
                    }
                }
            });
            if let Some(f) = chosen {
                set(app, json!({ "area_fill": f }));
            }
        }
        Attr::ArcType => {
            ui.horizontal(|ui| {
                for (pie, name) in [(false, "open"), (true, "pie wedge")] {
                    if cell(ui, vec2(40.0, 26.0), ind.pie_arc == pie, name, |p, r| {
                        arc_sample(p, r, pie, ink)
                    }) {
                        set(app, json!({ "pie_arc": pie }));
                    }
                }
            });
        }
        Attr::LineStyle => {
            ui.horizontal(|ui| {
                for k in LineKind::ALL {
                    if cell(ui, vec2(40.0, 24.0), ind.line == k, k.name(), |p, r| {
                        line_sample(p, r, k, ind.style_val, ink)
                    }) {
                        set(app, json!({ "line": k }));
                    }
                }
            });
            // xfig: "Default dash length" / "Default dot gap".
            let mut sv = ind.style_val;
            ui.horizontal(|ui| {
                ui.label(if ind.line == LineKind::Dotted {
                    "Dot gap"
                } else {
                    "Dash length"
                });
                if ui
                    .add(
                        egui::DragValue::new(&mut sv)
                            .range(0.5..=100.0)
                            .speed(0.1)
                            .suffix(" /80 in"),
                    )
                    .changed()
                {
                    set(app, json!({ "style_val": sv }));
                }
            });
        }
        Attr::JoinStyle => {
            ui.horizontal(|ui| {
                for j in JOINS {
                    let name = format!("{j:?}").to_lowercase();
                    if cell(ui, vec2(40.0, 26.0), ind.join == j, &name, |p, r| {
                        join_sample(p, r, j, ink, bg)
                    }) {
                        set(app, json!({ "join": j }));
                    }
                }
            });
        }
        Attr::CapStyle => {
            ui.horizontal(|ui| {
                for c in CAPS {
                    let name = format!("{c:?}").to_lowercase();
                    if cell(ui, vec2(44.0, 26.0), ind.cap == c, &name, |p, r| {
                        cap_sample(p, r, c, ink, tk)
                    }) {
                        set(app, json!({ "cap": c }));
                    }
                }
            });
        }
        Attr::ArrowMode => {
            ui.horizontal(|ui| {
                for (m, name) in ["no arrows", "forward", "backward", "both"].iter().enumerate() {
                    if cell(ui, vec2(44.0, 24.0), arrow_mode(&ind) == m, name, |p, r| {
                        arrow_mode_sample(p, r, m, ink)
                    }) {
                        set_arrow_mode(app, m);
                    }
                }
            });
        }
        Attr::ArrowType => {
            let cur = arrow_choice_of(ind.arrow.kind, ind.arrow.style);
            let mut chosen = None;
            egui::Grid::new("arrow_types").spacing(vec2(2.0, 2.0)).show(ui, |ui| {
                for n in 0..ARROW_CHOICES {
                    let (kind, style) = arrow_choice(n);
                    let name = if kind == 0 {
                        "type 0".to_string()
                    } else {
                        format!("{} type {kind}", if style == 1 { "solid" } else { "hollow" })
                    };
                    if cell(ui, vec2(40.0, 24.0), n == cur, &name, |p, r| {
                        arrow_sample(p, r, kind, style, ink, bg)
                    }) {
                        chosen = Some(n);
                    }
                    if n % 9 == 8 {
                        ui.end_row();
                    }
                }
            });
            if let Some(n) = chosen {
                set_arrow_type(app, &ind, n);
            }
        }
        Attr::ArrowSize => {
            let unit = rustfig_geom::FIG_PER_STYLE_UNIT;
            let mut w = ind.arrow.width / unit;
            let mut h = ind.arrow.height / unit;
            let mut changed = false;
            egui::Grid::new("arrow_size").show(ui, |ui| {
                ui.label("Width");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut w)
                            .range(0.5..=200.0)
                            .speed(0.1)
                            .suffix(" /80 in"),
                    )
                    .changed();
                ui.end_row();
                ui.label("Length");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut h)
                            .range(0.5..=200.0)
                            .speed(0.1)
                            .suffix(" /80 in"),
                    )
                    .changed();
                ui.end_row();
            });
            ui.label(egui::RichText::new("Thickness follows the line width.").small());
            if changed {
                let mut arrow = ind.arrow;
                arrow.width = w * unit;
                arrow.height = h * unit;
                set(app, json!({ "arrow": arrow }));
            }
        }
        Attr::TextJust => {
            ui.horizontal(|ui| {
                for j in JUSTIFY {
                    let name = format!("{j:?}").to_lowercase();
                    if cell(ui, vec2(34.0, 26.0), ind.justify == j, &name, |p, r| {
                        just_sample(p, r, j, ink)
                    }) {
                        set(app, json!({ "justify": j }));
                    }
                }
            });
        }
        Attr::TextFlags => {
            // xfig's Text Flags panel: Hidden, Rigid and TeX (special).
            for (field, name, on) in [
                ("hidden_text", "Hidden", ind.hidden_text),
                ("rigid_text", "Rigid", ind.rigid_text),
                ("special_text", "TeX (special)", ind.special_text),
            ] {
                let mut v = on;
                if ui.checkbox(&mut v, name).changed() {
                    let mut m = serde_json::Map::new();
                    m.insert(field.into(), json!(v));
                    set(app, Value::Object(m));
                }
            }
        }
        Attr::Font => {
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                ui.label(egui::RichText::new("PostScript fonts").small());
                for f in 0..rustfig_text::PS_FONT_COUNT {
                    if ui
                        .selectable_label(ind.postscript && f == ind.font, font_name(f, true))
                        .clicked()
                    {
                        set(app, json!({"font": f, "postscript": true}));
                    }
                }
                ui.separator();
                ui.label(egui::RichText::new("LaTeX fonts").small());
                for f in 0..rustfig_text::LATEX_FONT_COUNT {
                    if ui
                        .selectable_label(!ind.postscript && f == ind.font, font_name(f, false))
                        .clicked()
                    {
                        set(app, json!({"font": f, "postscript": false}));
                    }
                }
            });
        }
        Attr::Depth
        | Attr::RotnAngle
        | Attr::NumSides
        | Attr::LineWidth
        | Attr::BoxRadius
        | Attr::TextAngle
        | Attr::FontSize
        | Attr::TextStep => value_entry(app, ui, &ind, a),
    }
}

/// xfig's value panel: a number field.
fn value_entry(app: &mut App, ui: &mut Ui, ind: &Indicator, a: Attr) {
    let (field, mut v, lo, hi, speed) = match a {
        Attr::Depth => (
            "depth",
            f64::from(ind.depth),
            0.0,
            f64::from(rustfig_model::MAX_DEPTH),
            1.0,
        ),
        Attr::RotnAngle => ("rotate_angle", ind.rotate_angle, -360.0, 360.0, 1.0),
        Attr::NumSides => ("sides", ind.sides as f64, 3.0, 200.0, 1.0),
        Attr::LineWidth => ("thickness", f64::from(ind.thickness), 0.0, 500.0, 1.0),
        Attr::BoxRadius => ("box_radius", f64::from(ind.box_radius), 0.0, 1000.0, 1.0),
        Attr::TextAngle => ("text_angle", ind.text_angle, -360.0, 360.0, 1.0),
        Attr::FontSize => ("font_size", ind.font_size, 1.0, 500.0, 0.5),
        Attr::TextStep => ("text_step", ind.text_step, 0.0, 100.0, 0.05),
        _ => return,
    };
    let integer = matches!(a, Attr::Depth | Attr::NumSides | Attr::LineWidth | Attr::BoxRadius);
    let mut dv = egui::DragValue::new(&mut v).range(lo..=hi).speed(speed);
    if integer {
        dv = dv.fixed_decimals(0);
    }
    if ui.add(dv).changed() {
        let value = if integer { json!(v.round() as i64) } else { json!(v) };
        let mut m = serde_json::Map::new();
        m.insert(field.into(), value);
        set(app, Value::Object(m));
    }
}
