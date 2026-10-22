//! xfig's "Dimension Line Settings" panel (`popup_dimline_panel` in
//! `w_indpanel.c`, xfig licence, see LICENSE-xfig): a window of its own,
//! opened from the indicator's "Dimension line" button in Polyline mode,
//! with the line, box, arrow, tick and text settings of new dimension
//! lines. OK stores the draft in the indicator (`indicator.set`), Cancel
//! drops it.

use crate::attributes::{color32, color_name, colors, font_name};
use crate::App;
use egui::Ui;
use rustfig_engine::dimline::DimlineSettings;
use rustfig_model::LineKind;
use serde_json::json;

const OPEN: &str = "dimline_panel_open";
const DRAFT: &str = "dimline_panel_draft";

/// Open the panel (the draft is loaded from the indicator on the next frame).
pub fn open(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(OPEN), true));
}

fn close(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new(OPEN), false);
        d.remove::<DimlineSettings>(egui::Id::new(DRAFT));
    });
}

/// Whether the panel is open (tests and the control channel).
pub fn is_open(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::new(OPEN)).unwrap_or(false))
}

/// Draw the panel when it is open.
pub fn show(app: &mut App, ctx: &egui::Context) {
    if !is_open(ctx) {
        return;
    }
    let mut draft = ctx
        .data(|d| d.get_temp::<DimlineSettings>(egui::Id::new(DRAFT)))
        .unwrap_or_else(|| app.engine.indicator.dimline.clone());
    let mut result: Option<bool> = None;
    let mut window_open = true;
    egui::Window::new("Xfig: Dimension Line Settings")
        .open(&mut window_open)
        .resizable(false)
        .default_pos(egui::pos2(260.0, 120.0))
        .show(ctx, |ui| {
            fields(app, ui, &mut draft);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    result = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    result = Some(false);
                }
            });
        });
    if !window_open {
        result = Some(false);
    }
    match result {
        Some(ok) => {
            if ok {
                draft.sanitize();
                if let Ok(v) = serde_json::to_value(&draft) {
                    app.run("indicator.set", json!({ "dimline": v }));
                }
            }
            close(ctx);
        }
        None => ctx.data_mut(|d| d.insert_temp(egui::Id::new(DRAFT), draft)),
    }
}

fn int(ui: &mut Ui, name: &str, v: &mut i32, hi: i32) {
    ui.label(name);
    ui.add(egui::DragValue::new(v).range(0..=hi))
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, name));
}

fn float(ui: &mut Ui, name: &str, v: &mut f64, lo: f64, hi: f64) {
    ui.label(name);
    ui.add(egui::DragValue::new(v).range(lo..=hi).speed(0.1).fixed_decimals(1))
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, name));
}

fn color_box(app: &App, ui: &mut Ui, name: &str, c: &mut i32) {
    ui.label(name);
    egui::ComboBox::from_id_salt(name)
        .selected_text(color_name(*c))
        .show_ui(ui, |ui| {
            for n in colors(app.engine.doc()) {
                let sw = egui::RichText::new(format!("■ {}", color_name(n))).color(color32(app.engine.doc(), n));
                ui.selectable_value(c, n, sw);
            }
        })
        .response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, name));
}

fn arrow_name(v: i32) -> String {
    if v < 0 {
        "none".to_string()
    } else {
        let kind = (v + 1) / 2;
        let style = if v - (kind * 2 - 1) == 1 { "filled" } else { "hollow" };
        format!("type {kind}, {style}")
    }
}

fn arrow_box(ui: &mut Ui, name: &str, v: &mut i32) {
    egui::ComboBox::from_id_salt(name)
        .selected_text(arrow_name(*v))
        .show_ui(ui, |ui| {
            for n in -1..=28 {
                ui.selectable_value(v, n, arrow_name(n));
            }
        })
        .response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, name));
}

fn fields(app: &App, ui: &mut Ui, d: &mut DimlineSettings) {
    ui.strong("Line");
    ui.horizontal(|ui| {
        int(ui, "Line thickness", &mut d.thick, 500);
        ui.label("Style");
        egui::ComboBox::from_id_salt("dimline_style")
            .selected_text(format!("{:?}", d.style))
            .show_ui(ui, |ui| {
                for k in LineKind::ALL {
                    ui.selectable_value(&mut d.style, k, format!("{k:?}"));
                }
            })
            .response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "Line style"));
        color_box(app, ui, "Line color", &mut d.color);
    });
    ui.strong("Box");
    ui.horizontal(|ui| {
        int(ui, "Box thickness", &mut d.box_thick, 500);
        color_box(app, ui, "Box color", &mut d.box_color);
    });
    ui.strong("Arrows");
    ui.horizontal(|ui| {
        ui.label("Types");
        arrow_box(ui, "Left arrow", &mut d.left_arrow);
        arrow_box(ui, "Right arrow", &mut d.right_arrow);
    });
    ui.horizontal(|ui| {
        float(ui, "Arrow length", &mut d.arrow_length, 0.1, 200.0);
        float(ui, "Arrow width", &mut d.arrow_width, 0.1, 200.0);
    });
    ui.strong("Ticks");
    ui.horizontal(|ui| {
        ui.checkbox(&mut d.ticks, "Show ticks");
        int(ui, "Tick thickness", &mut d.tick_thick, 500);
    });
    ui.strong("Text");
    ui.horizontal(|ui| {
        ui.label("Font");
        egui::ComboBox::from_id_salt("dimline_font")
            .selected_text(font_name(d.font, d.postscript))
            .show_ui(ui, |ui| {
                for f in 0..rustfig_text::PS_FONT_COUNT {
                    if ui
                        .selectable_label(d.postscript && f == d.font, font_name(f, true))
                        .clicked()
                    {
                        d.font = f;
                        d.postscript = true;
                    }
                }
                for f in 0..rustfig_text::LATEX_FONT_COUNT {
                    if ui
                        .selectable_label(!d.postscript && f == d.font, font_name(f, false))
                        .clicked()
                    {
                        d.font = f;
                        d.postscript = false;
                    }
                }
            })
            .response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "Dimension text font"));
        float(ui, "Size", &mut d.font_size, 1.0, 500.0);
        color_box(app, ui, "Text color", &mut d.text_color);
    });
    ui.horizontal(|ui| {
        ui.label("String");
        ui.radio_value(&mut d.fixed, false, "Actual length");
        ui.radio_value(&mut d.fixed, true, "Fixed text");
        let mut p = i32::try_from(d.precision).unwrap_or(1);
        int(ui, "Precision", &mut p, 9);
        d.precision = u32::try_from(p).unwrap_or(1);
    });
}
