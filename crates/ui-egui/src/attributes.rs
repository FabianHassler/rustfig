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
use rustfig_engine::{arrow_choice, arrow_choice_of, Indicator, SnapMode, ToolKind};
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
        ToolKind::None
        | ToolKind::Break
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
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp));
    crate::bevel::paint(ui.painter(), rect, tk, crate::bevel::State::of(&resp, false, open));
    let p = ui.painter();
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
    // xfig: Alt + right button is the middle button (`sel_ind_but`).
    if resp.secondary_clicked() {
        step(app, a, if ui.input(|i| i.modifiers.alt) { -1 } else { 1 });
    } else if resp.middle_clicked() {
        step(app, a, -1);
    } else if resp.hovered() {
        // xfig's wheel (`Fig*ind_panel*Command.translations`): up is the
        // middle button, down the right one; one step per wheel notch.
        let dy = ui.input(|i| i.raw_scroll_delta.y);
        if dy != 0.0 {
            let id = resp.id.with("wheel");
            let acc = ui.data(|d| d.get_temp::<f32>(id).unwrap_or(0.0)) + dy;
            let notch = 40.0;
            let steps = (acc / notch).trunc();
            ui.data_mut(|d| d.insert_temp(id, acc - steps * notch));
            if steps > 0.0 {
                step(app, a, -1);
            } else if steps < 0.0 {
                step(app, a, 1);
            }
        }
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
        "{summary}\nClick: choose · middle button or wheel up: previous/decrease · right button or wheel down: next/increase"
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

/// xfig's arrow type choices: type 0, then types 1–14 hollow and filled
/// (`arrowtype_choices` in `w_indpanel.c`, built with `ARROWS4TO14`).
const ARROW_CHOICES: i32 = 29;

/// Arrow mode in xfig's order: none, forward, both, backward.
fn arrow_mode(ind: &Indicator) -> usize {
    match (ind.forward_arrow, ind.backward_arrow) {
        (false, false) => 0,
        (true, false) => 1,
        (true, true) => 2,
        (false, true) => 3,
    }
}

fn set_arrow_mode(app: &mut App, m: usize) {
    let (f, b) = match m % 4 {
        1 => (true, false),
        2 => (true, true),
        3 => (false, true),
        _ => (false, false),
    };
    set(app, json!({"forward_arrow": f, "backward_arrow": b}));
}

const H_ALIGN: [&str; 4] = ["none", "left", "center", "right"];
const V_ALIGN: [&str; 4] = ["none", "top", "center", "bottom"];
const JUSTIFY: [Justify; 3] = [Justify::Left, Justify::Center, Justify::Right];
const JOINS: [JoinStyle; 3] = [JoinStyle::Miter, JoinStyle::Round, JoinStyle::Bevel];
const CAPS: [CapStyle; 3] = [CapStyle::Butt, CapStyle::Round, CapStyle::Projecting];

/// Grid choices (xfig's): none, then grid steps 1–4.
fn grid_choices() -> Vec<Option<SnapMode>> {
    std::iter::once(None)
        .chain(SnapMode::ALL.iter().skip(2).map(|m| Some(*m)))
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
        Attr::ArrowMode => ["none", "forward", "both", "backward"]
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
        Attr::RotnAngle => set(app, json!({"rotate_angle": step_rotn(ind.rotate_angle, dir)})),
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
        Attr::FillStyle => {
            let plain = matches!(ind.fill_color, -1 | 0 | 7);
            set(app, json!({"area_fill": step_fill(ind.area_fill, plain, dir)}));
        }
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
        Attr::TextAngle => set(app, json!({"text_angle": step_angle(ind.text_angle, dir)})),
        Attr::FontSize => set(app, json!({"font_size": step_font_size(ind.font_size, dir)})),
        Attr::TextStep => set(app, json!({"text_step": step_text_step(ind.text_step, dir)})),
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

/// xfig's `inc_rotnangle`/`dec_rotnangle`: the preset angles 15, 30, 45, 60,
/// 90, 120, 180, wrapping.
fn step_rotn(cur: f64, dir: i32) -> f64 {
    const P: [f64; 7] = [15.0, 30.0, 45.0, 60.0, 90.0, 120.0, 180.0];
    if dir > 0 {
        P.iter().copied().find(|p| *p > cur).unwrap_or(15.0)
    } else {
        P.iter().rev().copied().find(|p| *p < cur).unwrap_or(180.0)
    }
}

/// xfig's `inc_elltextangle`/`dec_elltextangle`: the next multiple of 15
/// degrees, within -360..360.
fn step_angle(cur: f64, dir: i32) -> f64 {
    let v = if dir > 0 {
        ((cur + 15.0 + 1e-9) / 15.0).floor() * 15.0
    } else {
        ((cur - 15.0 - 1e-9) / 15.0).ceil() * 15.0
    };
    v.clamp(-360.0, 360.0)
}

/// xfig's `inc_fontsize`/`dec_fontsize`: steps of 1, 2 (from 24), 4 (from
/// 64) and 10 (from 120), kept within 1..500.
fn step_font_size(cur: f64, dir: i32) -> f64 {
    let v = if dir > 0 {
        if cur >= 120.0 {
            (cur / 10.0).floor() * 10.0 + 10.0
        } else if cur >= 64.0 {
            (cur / 4.0).floor() * 4.0 + 4.0
        } else if cur >= 24.0 {
            (cur / 2.0).floor() * 2.0 + 2.0
        } else {
            cur.floor() + 1.0
        }
    } else if cur > 120.0 {
        (cur / 10.0).ceil() * 10.0 - 10.0
    } else if cur > 64.0 {
        (cur / 4.0).ceil() * 4.0 - 4.0
    } else if cur > 24.0 {
        (cur / 2.0).ceil() * 2.0 - 2.0
    } else {
        cur.ceil() - 1.0
    };
    v.clamp(1.0, 500.0)
}

/// xfig's `inc_textstep`/`dec_textstep`: 0.1 below 2, 0.2 up to 5, 0.5 up to
/// 10, then 1; rounded to 0.1, within 0..1000.
fn step_text_step(cur: f64, dir: i32) -> f64 {
    let v = if dir > 0 {
        if cur >= 10.0 {
            cur.trunc() + 1.0
        } else if cur >= 5.0 {
            ((cur * 2.0 + 0.01).trunc()) / 2.0 + 0.5
        } else if cur >= 2.0 {
            ((cur * 5.0 + 0.01).trunc()) / 5.0 + 0.2
        } else {
            cur + 0.1
        }
    } else if cur > 10.0 {
        cur.trunc() - 1.0
    } else if cur > 5.0 {
        ((cur * 2.0 + 0.01).trunc()) / 2.0 - 0.5
    } else if cur > 2.0 {
        ((cur * 5.0 + 0.01).trunc()) / 5.0 - 0.2
    } else if cur > 0.4 {
        cur - 0.1
    } else {
        cur
    };
    ((v * 10.0).round() / 10.0).clamp(0.0, 1000.0)
}

/// xfig's `darken_fill`/`lighten_fill`: -1 (none) to 62, wrapping; the tints
/// 21..=40 are skipped for black, white and default fill colours.
fn step_fill(cur: i32, plain: bool, dir: i32) -> i32 {
    let mut v = cur + dir;
    if v > 62 {
        v = -1;
    } else if v < -1 {
        v = 62;
    }
    if plain && (21..=40).contains(&v) {
        v = if dir > 0 { 41 } else { 20 };
    }
    v
}

fn set_arrow_type(app: &mut App, ind: &Indicator, n: i32) {
    let (kind, style) = arrow_choice(n);
    let mut a = ind.arrow;
    a.kind = kind;
    a.style = style;
    set(app, json!({ "arrow": a }));
}

// ---- xfig's pictures (w_icons.c) -------------------------------------------

use crate::vector_icons::VIcon;
use crate::xfig_icons::{self as xi, Xbm};

fn arrow_type_icon(n: i32) -> &'static Xbm {
    const T: [&Xbm; 29] = [
        &xi::ARROW0,
        &xi::ARROW1O,
        &xi::ARROW1F,
        &xi::ARROW2O,
        &xi::ARROW2F,
        &xi::ARROW3O,
        &xi::ARROW3F,
        &xi::ARROW4O,
        &xi::ARROW4F,
        &xi::ARROW5O,
        &xi::ARROW5F,
        &xi::ARROW6O,
        &xi::ARROW6F,
        &xi::ARROW7O,
        &xi::ARROW7F,
        &xi::ARROW8O,
        &xi::ARROW8F,
        &xi::ARROW9A,
        &xi::ARROW9B,
        &xi::ARROW10O,
        &xi::ARROW10F,
        &xi::ARROW11O,
        &xi::ARROW11F,
        &xi::ARROW12O,
        &xi::ARROW12F,
        &xi::ARROW13A,
        &xi::ARROW13B,
        &xi::ARROW14A,
        &xi::ARROW14B,
    ];
    usize::try_from(n)
        .ok()
        .and_then(|i| T.get(i))
        .copied()
        .unwrap_or(&xi::ARROW0)
}

fn line_icon(k: LineKind) -> &'static Xbm {
    match k {
        LineKind::Dashed => &xi::DASHLINE,
        LineKind::Dotted => &xi::DOTTEDLINE,
        LineKind::DashDotted => &xi::DASHDOTLINE,
        LineKind::DashDoubleDotted => &xi::DASH2DOTSLINE,
        LineKind::DashTripleDotted => &xi::DASH3DOTSLINE,
        LineKind::Solid | LineKind::Default => &xi::SOLIDLINE,
    }
}

fn join_icon(j: JoinStyle) -> &'static Xbm {
    match j {
        JoinStyle::Miter => &xi::JOINMITER,
        JoinStyle::Round => &xi::JOINROUND,
        JoinStyle::Bevel => &xi::JOINBEVEL,
    }
}

fn cap_icon(c: CapStyle) -> &'static Xbm {
    match c {
        CapStyle::Butt => &xi::CAPBUTT,
        CapStyle::Round => &xi::CAPROUND,
        CapStyle::Projecting => &xi::CAPPROJECT,
    }
}

const ARROW_MODE_ICONS: [&Xbm; 4] = [&xi::NOARROWS, &xi::FARROWS, &xi::FBARROWS, &xi::BARROWS];

fn just_icon(j: Justify) -> &'static Xbm {
    match j {
        Justify::Left => &xi::TEXTL,
        Justify::Center => &xi::TEXTC,
        Justify::Right => &xi::TEXTR,
    }
}

/// xfig's point positioning pictures: any, magnet, grid 1–4.
fn posn_icon(m: SnapMode) -> &'static Xbm {
    match m {
        SnapMode::Any => &xi::ANY,
        SnapMode::Fine => &xi::FINE_GRID,
        SnapMode::Small => &xi::GRID1,
        SnapMode::Medium => &xi::GRID2,
        SnapMode::Large => &xi::GRID3,
        SnapMode::Coarse => &xi::GRID4,
    }
}

fn grid_icon(g: Option<SnapMode>) -> &'static Xbm {
    g.map_or(&xi::NONE, posn_icon)
}

fn align_icon(v: &str, horizontal: bool) -> &'static Xbm {
    match (v, horizontal) {
        ("left", true) => &xi::HALIGNL,
        ("center", true) => &xi::HALIGNC,
        ("right", true) => &xi::HALIGNR,
        ("top", false) => &xi::VALIGNT,
        ("center", false) => &xi::VALIGNC,
        ("bottom", false) => &xi::VALIGNB,
        _ => &xi::NONE,
    }
}

/// The picture of arc type `pie`.
fn arc_pic(app: &App, pie: bool) -> Pic {
    let b = if pie { &xi::PIE_WEDGE_ARC } else { &xi::OPEN_ARC };
    Pic::of(app, b, VIcon::Arc { pie })
}

/// The picture of arrow mode `m` (xfig's order: none, forward, both, backward).
fn arrow_mode_pic(app: &App, m: usize) -> Pic {
    let (forward, backward) = match m {
        1 => (true, false),
        2 => (true, true),
        3 => (false, true),
        _ => (false, false),
    };
    let b = ARROW_MODE_ICONS.get(m).copied().unwrap_or(&xi::NOARROWS);
    Pic::of(app, b, VIcon::ArrowMode { backward, forward })
}

/// The picture of arrow choice `n`.
fn arrow_type_pic(app: &App, n: i32) -> Pic {
    let (kind, style) = arrow_choice(n);
    Pic::of(app, arrow_type_icon(n), VIcon::Arrow(kind, style))
}

/// The picture xfig shows on an attribute button, if it shows one.
fn value_icon(app: &App, a: Attr) -> Option<Pic> {
    let ind = &app.engine.indicator;
    Some(match a {
        Attr::Grid => Pic::traced(app, grid_icon(ind.grid)),
        Attr::PointPosn => Pic::traced(app, posn_icon(ind.snap)),
        Attr::HAlign => Pic::traced(app, align_icon(&ind.align_h, true)),
        Attr::VAlign => Pic::traced(app, align_icon(&ind.align_v, false)),
        Attr::ArcType => arc_pic(app, ind.pie_arc),
        Attr::LineStyle => Pic::of(app, line_icon(ind.line), VIcon::Line(ind.line)),
        Attr::JoinStyle => Pic::of(app, join_icon(ind.join), VIcon::Join(ind.join)),
        Attr::CapStyle => Pic::of(app, cap_icon(ind.cap), VIcon::Cap(ind.cap)),
        Attr::ArrowMode => arrow_mode_pic(app, arrow_mode(ind)),
        Attr::ArrowType => arrow_type_pic(app, arrow_choice_of(ind.arrow.kind, ind.arrow.style)),
        Attr::TextJust => Pic::of(app, just_icon(ind.justify), VIcon::Justify(ind.justify)),
        _ => return None,
    })
}

/// A row (wrapping every `per_row`) of xfig pictures to choose from;
/// returns the index clicked.
/// A picture: xfig's bitmap, or the same drawn as a vector (W9).
#[derive(Clone, Copy)]
enum Pic {
    Bitmap(&'static Xbm),
    Vector(VIcon),
}

impl Pic {
    /// The vector picture when they are on, else xfig's bitmap.
    fn of(app: &App, bitmap: &'static Xbm, vector: VIcon) -> Pic {
        if app.vector_icons {
            Pic::Vector(vector)
        } else {
            Pic::Bitmap(bitmap)
        }
    }

    /// xfig's bitmap, or its traced vector drawing when vectors are on.
    fn traced(app: &App, bitmap: &'static Xbm) -> Pic {
        if app.vector_icons && crate::mode_icons::traced(bitmap.name) {
            Pic::Vector(VIcon::Traced(bitmap.name))
        } else {
            Pic::Bitmap(bitmap)
        }
    }

    fn paint(self, ui: &Ui, r: Rect, tint: Color32) {
        match self {
            Pic::Bitmap(b) => crate::xfig_icons::paint(ui, b, r, tint),
            Pic::Vector(v) => crate::vector_icons::paint(ui, v, r, tint),
        }
    }
}

fn icon_choices(ui: &mut Ui, id: &str, items: &[(bool, String, Pic)], per_row: usize) -> Option<usize> {
    let mut chosen = None;
    egui::Grid::new(("icon_choices", id))
        .spacing(vec2(2.0, 2.0))
        .show(ui, |ui| {
            for (i, (selected, name, icon)) in items.iter().enumerate() {
                let (r, resp) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::click());
                let v = ui.style().interact_selectable(&resp, *selected);
                ui.painter().rect_filled(r, 2.0, v.weak_bg_fill);
                icon.paint(ui, r, ui.visuals().text_color());
                if *selected {
                    ui.painter().rect_stroke(
                        r,
                        2.0,
                        Stroke::new(2.0_f32, ui.visuals().selection.stroke.color),
                        egui::StrokeKind::Inside,
                    );
                }
                if resp.clicked() {
                    chosen = Some(i);
                }
                resp.on_hover_text(name.as_str()).widget_info(|| {
                    egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, *selected, name)
                });
                if per_row > 0 && i % per_row == per_row - 1 {
                    ui.end_row();
                }
            }
        });
    chosen
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
    if let Some(icon) = value_icon(app, a) {
        // xfig's 32×32 picture, the full height of the button.
        let tall = Rect::from_min_max(
            pos2(r.left(), button.top() + 1.0),
            pos2(r.right(), button.bottom() - 1.0),
        );
        icon.paint(ui, tall, ink);
        return;
    }
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
            if let Some(t) = rustfig_render::pattern::tile(kind) {
                // Two tiles across the swatch; pattern space has y up.
                let (tw, th) = (t.width, t.height);
                let s = r.width() as f64 / (2.0 * tw.max(1.0));
                let painter = p.with_clip_rect(r);
                for ty in 0..(r.height() as f64 / (th * s)).ceil() as i32 + 1 {
                    for tx in 0..3 {
                        for l in &t.lines {
                            let pts: Vec<Pos2> = l
                                .iter()
                                .map(|&(x, y)| {
                                    pos2(
                                        r.left() + ((x + f64::from(tx) * tw) * s) as f32,
                                        r.top() + ((th - y + f64::from(ty) * th) * s) as f32,
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
    match a {
        Attr::Zoom => {
            let mut z = app.engine.view.zoom;
            if ui
                .add(
                    egui::DragValue::new(&mut z)
                        .range(0.01..=128.0)
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
            let list = grid_choices();
            let items: Vec<_> = list
                .iter()
                .map(|g| {
                    (
                        ind.grid == *g,
                        g.map_or("none", |m| m.label(u)).to_string(),
                        Pic::traced(app, grid_icon(*g)),
                    )
                })
                .collect();
            if let Some(g) = icon_choices(ui, "grid", &items, 5).and_then(|i| list.get(i)) {
                set(app, json!({ "grid": g }));
            }
        }
        Attr::PointPosn => {
            let u = units(app);
            let items: Vec<_> = SnapMode::ALL
                .iter()
                .map(|m| (ind.snap == *m, m.label(u).to_string(), Pic::traced(app, posn_icon(*m))))
                .collect();
            if let Some(m) = icon_choices(ui, "posn", &items, 6).and_then(|i| SnapMode::ALL.get(i)) {
                set(app, json!({ "snap": m }));
            }
        }
        Attr::VAlign | Attr::HAlign => {
            let horizontal = a == Attr::HAlign;
            let (list, cur, field) = if horizontal {
                (H_ALIGN, ind.align_h.as_str(), "align_h")
            } else {
                (V_ALIGN, ind.align_v.as_str(), "align_v")
            };
            let items: Vec<_> = list
                .iter()
                .map(|v| (cur == *v, (*v).to_string(), Pic::traced(app, align_icon(v, horizontal))))
                .collect();
            if let Some(v) = icon_choices(ui, field, &items, 4).and_then(|i| list.get(i)) {
                let mut m = serde_json::Map::new();
                m.insert(field.into(), json!(v));
                set(app, Value::Object(m));
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
            // The user's own colours (E1): picking one adds it to the drawing.
            let user = app.engine.prefs.rustfig.user_colors.clone();
            if !user.is_empty() {
                ui.label(egui::RichText::new("Your colours").small());
                egui::Grid::new(("user_colors", field))
                    .spacing(vec2(2.0, 2.0))
                    .show(ui, |ui| {
                        for (i, uc) in user.iter().enumerate() {
                            let rgb = rustfig_model::Rgb::from_hex(&uc.rgb).unwrap_or(rustfig_model::Rgb::BLACK);
                            let col = Color32::from_rgb(rgb.r, rgb.g, rgb.b);
                            if cell(ui, vec2(34.0, 20.0), false, &uc.name, |p, r| {
                                p.rect_filled(r, 1.0, col);
                            }) {
                                if let Some(r) = app.run("color.add", json!({ "rgb": uc.rgb })) {
                                    chosen = r
                                        .get("color")
                                        .and_then(Value::as_i64)
                                        .and_then(|c| i32::try_from(c).ok());
                                }
                            }
                            if i % 7 == 6 {
                                ui.end_row();
                            }
                        }
                    });
            }
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
                let plain = matches!(ind.fill_color, -1 | 0 | 7);
                for (i, f) in (-1..=40).filter(|f| !(plain && (21..=40).contains(f))).enumerate() {
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
            let items = [
                (!ind.pie_arc, "open".to_string(), arc_pic(app, false)),
                (ind.pie_arc, "pie wedge".to_string(), arc_pic(app, true)),
            ];
            if let Some(i) = icon_choices(ui, "arc", &items, 2) {
                set(app, json!({ "pie_arc": i == 1 }));
            }
        }
        Attr::LineStyle => {
            let items: Vec<_> = LineKind::ALL
                .iter()
                .map(|k| {
                    (
                        ind.line == *k,
                        k.name().to_string(),
                        Pic::of(app, line_icon(*k), VIcon::Line(*k)),
                    )
                })
                .collect();
            if let Some(k) = icon_choices(ui, "line", &items, 6).and_then(|i| LineKind::ALL.get(i)) {
                set(app, json!({ "line": k }));
            }
            // xfig: "Default dash length" and "Default dot gap" (0..10000,
            // both shown; a value of 0 or less means the default).
            let mut dash = ind.style_val;
            let mut gap = ind.dot_gap;
            let mut changed = false;
            egui::Grid::new("dash_gap").show(ui, |ui| {
                ui.label("Default dash length");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut dash)
                            .range(0.0..=10000.0)
                            .speed(0.1)
                            .max_decimals(1),
                    )
                    .changed();
                ui.end_row();
                ui.label("Default dot gap");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut gap)
                            .range(0.0..=10000.0)
                            .speed(0.1)
                            .max_decimals(1),
                    )
                    .changed();
                ui.end_row();
            });
            if changed {
                set(
                    app,
                    json!({ "style_val": if dash <= 0.0 { 4.0 } else { dash }, "dot_gap": if gap <= 0.0 { 3.0 } else { gap } }),
                );
            }
        }
        Attr::JoinStyle => {
            let items: Vec<_> = JOINS
                .iter()
                .map(|j| {
                    (
                        ind.join == *j,
                        format!("{j:?}").to_lowercase(),
                        Pic::of(app, join_icon(*j), VIcon::Join(*j)),
                    )
                })
                .collect();
            if let Some(j) = icon_choices(ui, "join", &items, 3).and_then(|i| JOINS.get(i)) {
                set(app, json!({ "join": j }));
            }
        }
        Attr::CapStyle => {
            let items: Vec<_> = CAPS
                .iter()
                .map(|c| {
                    (
                        ind.cap == *c,
                        format!("{c:?}").to_lowercase(),
                        Pic::of(app, cap_icon(*c), VIcon::Cap(*c)),
                    )
                })
                .collect();
            if let Some(c) = icon_choices(ui, "cap", &items, 3).and_then(|i| CAPS.get(i)) {
                set(app, json!({ "cap": c }));
            }
        }
        Attr::ArrowMode => {
            let names = ["no arrows", "forward", "both", "backward"];
            let items: Vec<_> = names
                .iter()
                .enumerate()
                .map(|(m, n)| (arrow_mode(&ind) == m, (*n).to_string(), arrow_mode_pic(app, m)))
                .collect();
            if let Some(m) = icon_choices(ui, "arrow_mode", &items, 4) {
                set_arrow_mode(app, m);
            }
        }
        Attr::ArrowType => {
            let cur = arrow_choice_of(ind.arrow.kind, ind.arrow.style);
            let items: Vec<_> = (0..ARROW_CHOICES)
                .map(|n| {
                    let (kind, style) = arrow_choice(n);
                    let name = if kind == 0 {
                        "type 0".to_string()
                    } else {
                        format!("{} type {kind}", if style == 1 { "solid" } else { "hollow" })
                    };
                    (n == cur, name, arrow_type_pic(app, n))
                })
                .collect();
            if let Some(n) = icon_choices(ui, "arrow_type", &items, 9) {
                set_arrow_type(app, &ind, n as i32);
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
            let items: Vec<_> = JUSTIFY
                .iter()
                .map(|j| {
                    (
                        ind.justify == *j,
                        format!("{j:?}").to_lowercase(),
                        Pic::of(app, just_icon(*j), VIcon::Justify(*j)),
                    )
                })
                .collect();
            if let Some(j) = icon_choices(ui, "just", &items, 3).and_then(|i| JUSTIFY.get(i)) {
                set(app, json!({ "justify": j }));
            }
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
    let resp = ui.add(dv);
    let mut changed = resp.changed();
    if resp.hovered() {
        // xfig's spinners (`Fig*spinner_form*Text.translations`): wheel up
        // decreases, wheel down increases, one step per notch.
        let dy = ui.input(|i| i.raw_scroll_delta.y);
        if dy != 0.0 {
            let id = resp.id.with("wheel");
            let acc = ui.data(|d| d.get_temp::<f32>(id).unwrap_or(0.0)) + dy;
            let notches = (acc / 40.0).trunc();
            ui.data_mut(|d| d.insert_temp(id, acc - notches * 40.0));
            if notches != 0.0 {
                let step = if a == Attr::TextStep { 0.1 } else { 1.0 };
                v = (v - f64::from(notches) * step).clamp(lo, hi);
                changed = true;
            }
        }
    }
    if changed {
        let value = if integer { json!(v.round() as i64) } else { json!(v) };
        let mut m = serde_json::Map::new();
        m.insert(field.into(), value);
        set(app, Value::Object(m));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_steps_through_xfigs_presets() {
        assert_eq!(step_rotn(90.0, 1), 120.0);
        assert_eq!(step_rotn(180.0, 1), 15.0);
        assert_eq!(step_rotn(90.0, -1), 60.0);
        assert_eq!(step_rotn(15.0, -1), 180.0);
        assert_eq!(step_rotn(0.0, 1), 15.0);
    }

    #[test]
    fn angle_steps_to_multiples_of_fifteen() {
        assert_eq!(step_angle(0.0, 1), 15.0);
        assert_eq!(step_angle(20.0, 1), 30.0);
        assert_eq!(step_angle(20.0, -1), 15.0);
        assert_eq!(step_angle(-20.0, -1), -30.0);
        assert_eq!(step_angle(360.0, 1), 360.0);
    }

    #[test]
    fn font_size_steps_grow_with_size() {
        assert_eq!(step_font_size(12.0, 1), 13.0);
        assert_eq!(step_font_size(24.0, 1), 26.0);
        assert_eq!(step_font_size(64.0, 1), 68.0);
        assert_eq!(step_font_size(120.0, 1), 130.0);
        assert_eq!(step_font_size(130.0, -1), 120.0);
        assert_eq!(step_font_size(1.0, -1), 1.0);
    }

    #[test]
    fn text_step_steps() {
        assert_eq!(step_text_step(1.0, 1), 1.1);
        assert_eq!(step_text_step(2.0, 1), 2.2);
        assert_eq!(step_text_step(5.0, 1), 5.5);
        assert_eq!(step_text_step(10.0, 1), 11.0);
        assert_eq!(step_text_step(0.4, -1), 0.4);
    }

    #[test]
    fn fill_steps_wrap_and_skip_tints_for_black() {
        assert_eq!(step_fill(62, false, 1), -1);
        assert_eq!(step_fill(-1, false, -1), 62);
        assert_eq!(step_fill(20, true, 1), 41);
        assert_eq!(step_fill(41, true, -1), 20);
        assert_eq!(step_fill(20, false, 1), 21);
    }
}
