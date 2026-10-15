//! xfig's rulers and units box. Ported from xfig's `w_rulers.c` and
//! `u_pan.c` (xfig licence, see LICENSE-xfig): tick and label spacing chosen
//! from xfig's tables by zoom and units, a marker following the cursor, the
//! left and right buttons panning by half an inch (1 cm) of the drawing and
//! the middle button dragging (five times faster with Shift); the units box
//! pans to the origin (left) and opens the Unit/Scale settings (right).

use crate::theme::Tokens;
use crate::App;
use egui::{pos2, vec2, Rect, Sense, Stroke, Ui};
use rustfig_model::Units;
use serde_json::json;

/// Top ruler height and side ruler width in points (xfig: `DEF_RULER_WD`,
/// side ruler 20 wider for long numbers), units box width (`UNITBOX_WD`).
pub const TOP_HT: f32 = 24.0;
pub const SIDE_WD: f32 = 44.0;
pub const UNITBOX_WD: f32 = 95.0;

// xfig's Fig-unit distances (1200 per inch, 450 per cm).
const PPI: i64 = 1200;
const PPCM: i64 = 450;
const HINCH: i64 = PPI / 2;
const QINCH: i64 = PPI / 4;
const SINCH: i64 = PPI / 16;
const TINCH: i64 = PPI / 10;
const HALFCM: i64 = PPCM / 2;
const ONEMM: i64 = PPCM / 10;

// Tick lengths in pixels.
const INCH_MARK: f32 = 8.0;
const HALF_MARK: f32 = 6.0;
const QUARTER_MARK: f32 = 4.0;
const SIXTEENTH_MARK: f32 = 3.0;
const CM_MARK: f32 = 8.0;
const HALFCM_MARK: f32 = 6.0;
const MM_MARK: f32 = 4.0;

/// (minimum zoom, Fig units between ticks, tick length), longest first.
type TickInfo = (f64, i64, f32);
/// (largest zoom it applies to — 0 for the last row —, step, labels every
/// `step × n`, decimals for fractional labels).
type RulerSkip = (f64, i64, i64, usize);

const TICK_INCHES: [TickInfo; 4] = [
    (0.0, PPI, INCH_MARK),
    (0.1, HINCH, HALF_MARK),
    (0.2, QINCH, QUARTER_MARK),
    (0.6, SINCH, SIXTEENTH_MARK),
];
const TICK_TENTH_INCHES: [TickInfo; 6] = [
    (0.0, PPI, INCH_MARK),
    (0.1, HINCH, HALF_MARK),
    (0.2, QINCH, QUARTER_MARK),
    (0.6, TINCH, SIXTEENTH_MARK),
    (1.0, TINCH / 2, SIXTEENTH_MARK),
    (4.0, TINCH / 4, SIXTEENTH_MARK),
];
const TICK_METRIC: [TickInfo; 3] = [(0.0, PPCM, CM_MARK), (0.6, HALFCM, HALFCM_MARK), (1.0, ONEMM, MM_MARK)];

const SKIP_INCHES: [RulerSkip; 11] = [
    (0.01, 12 * PPI, 4, 1),
    (0.0201, 6 * PPI, 4, 1),
    (0.0401, 2 * PPI, 6, 1),
    (0.0501, PPI, 12, 1),
    (0.0801, PPI, 6, 1),
    (0.201, SINCH, 64, 1),
    (0.301, SINCH, 32, 1),
    (2.0, SINCH, 16, 1),
    (4.0, SINCH, 8, 1),
    (8.0, SINCH, 4, 2),
    (0.0, SINCH, 1, 4),
];
const SKIP_TENTH_INCHES: [RulerSkip; 11] = [
    (0.01, 10 * PPI, 10, 1),
    (0.0201, 5 * PPI, 10, 1),
    (0.0401, 2 * PPI, 10, 1),
    (0.0801, PPI, 10, 1),
    (0.201, PPI, 5, 1),
    (0.401, TINCH * 2, 10, 1),
    (2.0, TINCH, 10, 1),
    (5.0, TINCH, 5, 1),
    (15.0, TINCH / 2, 2, 1),
    (50.0, TINCH / 4, 2, 2),
    (0.0, TINCH / 4, 1, 3),
];
const SKIP_METRIC: [RulerSkip; 10] = [
    (0.011, 20 * PPCM, 5, 1),
    (0.041, 10 * PPCM, 5, 1),
    (0.081, 5 * PPCM, 4, 1),
    (0.11, 2 * PPCM, 5, 1),
    (0.31, PPCM, 5, 1),
    (0.61, ONEMM, 20, 1),
    (2.01, ONEMM, 10, 1),
    (8.01, ONEMM, 5, 1),
    (12.01, ONEMM, 2, 1),
    (0.0, ONEMM, 1, 1),
];

/// Cap on ticks per ruler, against absurd zoom or scale values.
const MAX_TICKS: usize = 20_000;

/// A tick on a ruler.
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    /// Position in Fig units.
    pub at: f64,
    /// Length in pixels.
    pub len: f32,
    pub label: Option<String>,
}

/// Ticks and labels for a ruler showing Fig coordinates `from..=to` at
/// `px_per_fig` (xfig's `get_skip_prec` and `reset_topruler`). `zoom` is
/// the display zoom (1 = 80 px per inch), `scale` the figure scale factor,
/// `unit` the label suffix used every ten units, `char_w` the label font's
/// character width in pixels.
#[allow(clippy::too_many_arguments)]
pub fn ticks(
    units: Units,
    decimal_inches: bool,
    zoom: f64,
    scale: f64,
    unit: &str,
    from: f64,
    to: f64,
    px_per_fig: f64,
    char_w: f64,
) -> Vec<Tick> {
    if !(zoom.is_finite() && scale.is_finite() && from.is_finite() && to.is_finite() && px_per_fig.is_finite())
        || zoom <= 0.0
        || scale <= 0.0
        || px_per_fig <= 0.0
        || to < from
    {
        return Vec::new();
    }
    let (ruler_unit, skips, tick_table): (i64, &[RulerSkip], &[TickInfo]) = match units {
        Units::Metric => (PPCM, &SKIP_METRIC, &TICK_METRIC),
        Units::Inches if decimal_inches || scale != 1.0 => (PPI, &SKIP_TENTH_INCHES, &TICK_TENTH_INCHES),
        Units::Inches => (PPI, &SKIP_INCHES, &TICK_INCHES),
    };
    let Some(&(_, skipt, nx, prec)) = skips
        .iter()
        .find(|(max, ..)| zoom / scale <= *max || *max == 0.0)
        .or(skips.last())
    else {
        return Vec::new();
    };
    let skipt_f = skipt as f64;
    let mut skip = if skipt_f > scale { (skipt_f / scale) as i64 } else { 1 }.max(1);
    let mut skipx = skip.saturating_mul(nx).max(1);
    let tickmod = ((ruler_unit as f64 / scale).round() as i64).max(1);
    // Widen the label spacing until the widest label fits.
    let widest = format!("{}{unit}", (to as i64) / tickmod).chars().count() as f64 * char_w;
    let mut guard = 0;
    while (skipx as f64) < (widest + 5.0) / px_per_fig && guard < 60 {
        skip = skip.saturating_mul(2);
        skipx = skipx.saturating_mul(2);
        guard += 1;
    }
    let start = (from.floor() as i64).div_euclid(skip) * skip;
    let end = to.ceil() as i64;
    let mut out = Vec::new();
    let mut i = start;
    while i <= end && out.len() < MAX_TICKS {
        let label = (i.rem_euclid(skipx) == 0).then(|| {
            if (i / 10).rem_euclid(tickmod) == 0 {
                format!("{}{unit}", i / tickmod)
            } else if i.rem_euclid(tickmod) == 0 {
                format!("{}", i / tickmod)
            } else {
                format!("{:.*}", prec, i as f64 / tickmod as f64)
            }
        });
        let len = tick_table
            .iter()
            .take_while(|(min_zoom, ..)| zoom >= *min_zoom)
            .filter(|(_, tick_skip, _)| {
                let mut ts = *tick_skip;
                let mut g = 0;
                while ((ts as f64) / scale) as i64 == 0 && g < 60 {
                    ts = ts.saturating_mul(2);
                    g += 1;
                }
                let step = ((ts as f64) / scale) as i64;
                step > 0 && i.rem_euclid(step) == 0
            })
            .map(|(_, _, len)| *len)
            .fold(0.0_f32, f32::max);
        if len > 0.0 || label.is_some() {
            out.push(Tick {
                at: i as f64,
                len,
                label,
            });
        }
        i = i.saturating_add(skip);
    }
    out
}

/// xfig's pan step: the third point-positioning step (1/2 inch or 1 cm of
/// the drawing) divided by the zoom; five times that with Shift.
pub fn pan_step(units: Units, zoom: f64, shift: bool) -> f64 {
    let base = match units {
        Units::Metric => PPCM as f64,
        Units::Inches => HINCH as f64,
    };
    base / zoom.max(1e-6) * if shift { 5.0 } else { 1.0 }
}

/// Lay out the rulers and units box around `area`; returns the canvas rect.
pub fn layout(area: Rect) -> (Rect, Rect, Rect, Rect) {
    let top = Rect::from_min_max(area.min, pos2(area.right() - UNITBOX_WD, area.top() + TOP_HT));
    let unit_box = Rect::from_min_max(
        pos2(area.right() - UNITBOX_WD, area.top()),
        pos2(area.right(), area.top() + TOP_HT),
    );
    let side = Rect::from_min_max(pos2(area.right() - SIDE_WD, area.top() + TOP_HT), area.max);
    let canvas = Rect::from_min_max(
        pos2(area.left(), area.top() + TOP_HT),
        pos2(area.right() - SIDE_WD, area.bottom()),
    );
    (canvas, top, side, unit_box)
}

fn unit_suffix(app: &App) -> String {
    if app.engine.unit_scale.unit.is_empty() {
        app.engine.doc().unit_name().to_string()
    } else {
        app.engine.unit_scale.unit.clone()
    }
}

/// Draw both rulers and the units box and handle their mouse buttons.
/// `canvas` is the drawing area the rulers measure.
pub fn show(app: &mut App, ui: &mut Ui, tk: &Tokens, canvas: Rect, top: Rect, side: Rect, unit_box: Rect) {
    ruler(app, ui, tk, canvas, top, true);
    ruler(app, ui, tk, canvas, side, false);
    units_box(app, ui, tk, unit_box);
}

fn ruler(app: &mut App, ui: &mut Ui, tk: &Tokens, canvas: Rect, r: Rect, horizontal: bool) {
    let id = if horizontal { "top ruler" } else { "side ruler" };
    let resp = ui.interact(r, ui.id().with(id), Sense::click_and_drag());
    let p = ui.painter_at(r);
    let visuals = ui.visuals();
    p.rect_filled(r, 0.0, visuals.panel_fill);
    let ink = visuals.text_color();
    let line = Stroke::new(1.0_f32, ink);
    let view = app.engine.view;
    let units = app.engine.doc().settings.units;
    let scale = &app.engine.unit_scale;
    let s = view.px_per_fig();
    let (from, to) = if horizontal {
        (view.origin.x, view.origin.x + f64::from(canvas.width()) / s)
    } else {
        (view.origin.y, view.origin.y + f64::from(canvas.height()) / s)
    };
    let font = egui::FontId::monospace(10.0);
    // xfig sizes labels for its 6×13 ruler font.
    let char_w = 6.0;
    let suffix = unit_suffix(app);
    let marks = ticks(
        units,
        scale.decimal_inches,
        view.zoom,
        scale.factor,
        &suffix,
        from,
        to,
        s,
        char_w,
    );
    let at = |f: f64| ((f - from) * s) as f32;
    for t in &marks {
        let d = at(t.at);
        if horizontal {
            let x = canvas.left() + d;
            p.line_segment([pos2(x, r.bottom() - 1.0), pos2(x, r.bottom() - 1.0 - t.len)], line);
            if let Some(l) = &t.label {
                p.text(
                    pos2(x, r.bottom() - INCH_MARK - 3.0),
                    egui::Align2::CENTER_BOTTOM,
                    l,
                    font.clone(),
                    ink,
                );
            }
        } else {
            let y = canvas.top() + d;
            p.line_segment([pos2(r.left(), y), pos2(r.left() + t.len, y)], line);
            if let Some(l) = &t.label {
                p.text(
                    pos2(r.left() + INCH_MARK + 3.0, y),
                    egui::Align2::LEFT_CENTER,
                    l,
                    font.clone(),
                    ink,
                );
            }
        }
    }
    // Edge towards the canvas.
    if horizontal {
        p.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0_f32, tk.muted));
    } else {
        p.line_segment([r.left_top(), r.left_bottom()], Stroke::new(1.0_f32, tk.muted));
    }
    // Marker following the cursor (xfig's ruler arrows).
    if let Some(c) = app.canvas.cursor {
        let d = at(if horizontal { c.x } else { c.y });
        let m = tk.selection;
        if horizontal {
            let x = canvas.left() + d;
            if r.x_range().contains(x) {
                p.add(egui::Shape::convex_polygon(
                    vec![
                        pos2(x - 4.0, r.bottom() - 8.0),
                        pos2(x + 4.0, r.bottom() - 8.0),
                        pos2(x, r.bottom()),
                    ],
                    m,
                    Stroke::NONE,
                ));
            }
        } else {
            let y = canvas.top() + d;
            if r.y_range().contains(y) {
                p.add(egui::Shape::convex_polygon(
                    vec![
                        pos2(r.left() + 8.0, y - 4.0),
                        pos2(r.left() + 8.0, y + 4.0),
                        pos2(r.left(), y),
                    ],
                    m,
                    Stroke::NONE,
                ));
            }
        }
    }
    // Buttons: left pans left/up, right pans right/down, middle drags
    // (Alt+right too, as in xfig); Shift makes it five times as far.
    let shift = ui.input(|i| i.modifiers.shift);
    if resp.hovered() || resp.dragged() {
        app.hint_override = Some(match (horizontal, shift) {
            (true, false) => ("Pan Left", "Drag", "Pan Right"),
            (true, true) => ("Pan Left x5", "Drag x5", "Pan Right x5"),
            (false, false) => ("Pan Up", "Drag", "Pan Down"),
            (false, true) => ("Pan Up x5", "Drag x5", "Pan Down x5"),
        });
    }
    let step_px = pan_step(units, view.zoom, shift) * s;
    let pan = |app: &mut App, sign: f64| {
        let (dx, dy) = if horizontal {
            (sign * step_px, 0.0)
        } else {
            (0.0, sign * step_px)
        };
        app.run("view.pan", json!({"dx": dx, "dy": dy}));
    };
    let alt = ui.input(|i| i.modifiers.alt);
    if resp.clicked() {
        // Pan left/up: the drawing moves left/up by one step.
        pan(app, -1.0);
    } else if resp.secondary_clicked() && !alt {
        pan(app, 1.0);
    }
    let dragging_middle =
        resp.dragged_by(egui::PointerButton::Middle) || (alt && resp.dragged_by(egui::PointerButton::Secondary));
    if dragging_middle {
        let d = resp.drag_delta();
        let k = if shift { 5.0 } else { 1.0 };
        let (dx, dy) = if horizontal {
            (f64::from(d.x) * k, 0.0)
        } else {
            (0.0, f64::from(d.y) * k)
        };
        app.run("view.pan", json!({"dx": dx, "dy": dy}));
    }
    let name = if horizontal { "Top ruler" } else { "Side ruler" };
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, name));
}

fn units_box(app: &mut App, ui: &mut Ui, tk: &Tokens, r: Rect) {
    let resp = ui.interact(r, ui.id().with("units box"), Sense::click());
    let p = ui.painter_at(r);
    let v = ui.style().interact(&resp);
    p.rect_filled(r, 0.0, v.weak_bg_fill);
    p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, tk.muted), egui::StrokeKind::Inside);
    let label = app.engine.unit_label();
    p.text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        &label,
        egui::FontId::proportional(11.0),
        ui.visuals().text_color(),
    );
    if resp.hovered() {
        app.hint_override = Some(("Pan to Origin", "", "Set Units/Scale"));
    }
    if resp.clicked() {
        app.run("view.pan_origin", json!({}));
    }
    // xfig opens its Unit menu as a small window.
    if resp.secondary_clicked() {
        app.unit_panel_open = !app.unit_panel_open;
    }
    if app.unit_panel_open {
        let mut open = true;
        egui::Window::new("Unit menu")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_pos(r.left_bottom() + vec2(-220.0, 6.0))
            .show(ui.ctx(), |ui| unit_panel(app, ui));
        app.unit_panel_open &= open;
    }
    resp.on_hover_text("Left: pan to the origin · right: Unit/Scale settings")
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Units: {label}")));
}

/// xfig's Unit/Scale settings panel.
fn unit_panel(app: &mut App, ui: &mut Ui) {
    ui.label(egui::RichText::new("Unit/Scale settings").strong());
    let units = app.engine.doc().settings.units;
    let decimal = app.engine.unit_scale.decimal_inches;
    egui::Grid::new("unit_panel")
        .num_columns(2)
        .spacing(vec2(8.0, 4.0))
        .show(ui, |ui| {
            ui.label("Ruler units");
            ui.vertical(|ui| {
                for (name, metric, dec) in [
                    ("Metric (cm)", true, false),
                    ("Imperial (fraction)", false, false),
                    ("Imperial (decimal)", false, true),
                ] {
                    let selected = (units == Units::Metric) == metric && (metric || decimal == dec);
                    if ui.radio(selected, name).clicked() && !selected {
                        let wanted = if metric { "metric" } else { "inches" };
                        if (units == Units::Metric) != metric {
                            app.run("doc.set_units", json!({ "units": wanted }));
                        }
                        app.run("view.set_scale", json!({ "decimal": dec }));
                    }
                }
            });
            ui.end_row();
            ui.label("Figure units");
            let mut unit = app.engine.unit_scale.unit.clone();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut unit)
                        .desired_width(80.0)
                        .hint_text("ruler units"),
                )
                .changed()
            {
                app.run("view.set_scale", json!({ "unit": unit }));
            }
            ui.end_row();
            ui.label("Scale factor");
            let mut f = app.engine.unit_scale.factor;
            if ui
                .add(egui::DragValue::new(&mut f).range(0.000_001..=1_000_000.0).speed(0.01))
                .changed()
            {
                app.run("view.set_scale", json!({ "scale": f }));
            }
            ui.end_row();
        });
    // xfig's Return (SetUnits) closes the panel; the settings already
    // apply as they change. Escape closes it too (`dialogs`).
    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter)) && ui.ctx().memory(|m| m.focused()).is_none();
    if ui.button("Close").clicked() || enter {
        app.unit_panel_open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(t: &[Tick]) -> Vec<String> {
        t.iter().filter_map(|t| t.label.clone()).collect()
    }

    #[test]
    fn metric_ruler_at_zoom_1_labels_each_cm() {
        let s = 80.0 / 1200.0;
        let t = ticks(Units::Metric, false, 1.0, 1.0, "cm", 0.0, 4500.0, s, 6.0);
        assert_eq!(labels(&t).first().map(String::as_str), Some("0cm"));
        assert!(labels(&t).contains(&"3".to_string()));
        assert_eq!(labels(&t).last().map(String::as_str), Some("10cm"));
        // Millimetre ticks at zoom 1, longest at whole centimetres.
        let at = |f: f64| t.iter().find(|x| x.at == f).map(|x| x.len);
        assert_eq!(at(45.0), Some(MM_MARK));
        assert_eq!(at(225.0), Some(HALFCM_MARK));
        assert_eq!(at(450.0), Some(CM_MARK));
    }

    #[test]
    fn inch_ruler_ticks_sixteenths_at_zoom_1() {
        let s = 80.0 / 1200.0;
        let t = ticks(Units::Inches, false, 1.0, 1.0, "in", 0.0, 2400.0, s, 6.0);
        // xfig adds the unit every ten units, so 0 reads "0in".
        assert_eq!(labels(&t), vec!["0in", "1", "2"]);
        let at = |f: f64| t.iter().find(|x| x.at == f).map(|x| x.len);
        assert_eq!(at(75.0), Some(SIXTEENTH_MARK));
        assert_eq!(at(600.0), Some(HALF_MARK));
        // Zoomed far out the labels thin out instead of overlapping.
        let far = ticks(Units::Inches, false, 0.05, 1.0, "in", 0.0, 120_000.0, s * 0.05, 6.0);
        assert!(labels(&far).len() < 20, "{:?}", labels(&far));
    }

    #[test]
    fn hostile_values_give_no_ticks() {
        for (z, sc, a, b) in [
            (f64::NAN, 1.0, 0.0, 1.0),
            (1.0, 0.0, 0.0, 1.0),
            (1.0, 1.0, 5.0, 1.0),
            (-1.0, 1.0, 0.0, 1.0),
        ] {
            assert!(ticks(Units::Metric, false, z, sc, "cm", a, b, 1.0, 6.0).is_empty());
        }
        // Huge ranges are capped.
        assert!(ticks(Units::Metric, false, 50.0, 1.0, "cm", -1e12, 1e12, 3.0, 6.0).len() <= MAX_TICKS);
    }

    #[test]
    fn pan_steps_follow_xfig() {
        assert_eq!(pan_step(Units::Inches, 1.0, false), 600.0);
        assert_eq!(pan_step(Units::Metric, 2.0, true), 450.0 / 2.0 * 5.0);
    }
}
