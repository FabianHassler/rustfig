//! xfig's "Dimension Line Settings" panel (`popup_dimline_panel` in
//! `w_indpanel.c`, xfig licence, see LICENSE-xfig): a window of its own,
//! opened from the indicator's "Dimension line" button in Polyline mode,
//! with the line, box, arrow, tick and text settings of new dimension
//! lines. OK stores the draft in the indicator (`indicator.set`), Cancel
//! drops it.

use crate::attributes::{color32, color_name, colors, font_name};
use crate::App;
use egui::Ui;
use rustfig_engine::dimline::{self, DimlineDefaults, DimlineSettings};
use rustfig_geom::{Affine, Point};
use rustfig_model::{Document, LineKind};
use rustfig_render::raster::{rasterize, Pixmap, RasterOptions};
use rustfig_render::SceneOptions;
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
            crate::dialog_body(ui, |ui| {
                preview(app, ui, &draft);
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
            })
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

/// One arrow end: a "None" box and xfig's pictorial arrow type menu
/// (`attributes::arrow_menu`, the pictures of the attribute panel).
fn arrow_box(ui: &mut Ui, id: &str, name: &str, v: &mut i32) {
    ui.label(name);
    let mut none = *v < 0;
    let label = format!("{name} arrow: none");
    let r = ui.checkbox(&mut none, "None");
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Checkbox, true, label.clone()));
    if r.changed() {
        *v = if none { -1 } else { 2 };
    }
    if *v >= 0 {
        if let Some(n) = crate::attributes::arrow_menu(ui, id, *v) {
            *v = n;
        }
    }
}

/// Fig units per preview point.
const PREVIEW_U: f64 = 15.0;
const PREVIEW_W: f32 = 300.0;
const PREVIEW_H: f32 = 70.0;

/// The example dimension line as a Fig drawing: the engine's own builder
/// with the draft settings and the indicator's other attributes.
fn example_objects(app: &App, d: &DimlineSettings) -> Vec<rustfig_model::Object> {
    let e = &app.engine;
    let ind = &e.indicator;
    let defaults = DimlineDefaults {
        pen_color: ind.pen_color,
        depth: ind.depth,
        join: ind.join,
        style_val: ind.style_val,
        dot_gap: ind.dot_gap,
    };
    let doc = e.doc();
    let (units, scale, ruler) = (doc.settings.units, e.unit_scale.clone(), doc.unit_name());
    let prec = d.precision;
    let fmt = move |l: f64| dimline::length_text(l, units, &scale, ruler, prec);
    let y = f64::from(PREVIEW_H) * PREVIEW_U * 0.68;
    let a = Point::new(24.0 * PREVIEW_U, y);
    let b = Point::new(f64::from(PREVIEW_W - 24.0) * PREVIEW_U, y);
    let text = d.fixed.then_some("text");
    vec![dimline::create(d, &defaults, a, b, text, &fmt)]
}

/// The example dimension line (xfig's panel shows one too): the compound a
/// new dimension line would be, rendered by the canvas renderer at the
/// screen's pixel density.
fn preview(app: &App, ui: &mut Ui, d: &DimlineSettings) {
    let doc = app.engine.doc();
    let tk = crate::theme::Tokens::for_dark(ui.visuals().dark_mode);
    let (resp, _) = ui.allocate_painter(egui::vec2(PREVIEW_W, PREVIEW_H), egui::Sense::hover());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Dimension line example"));
    let r = resp.rect;
    let ppp = ui.ctx().pixels_per_point();
    let key = format!(
        "{:?}|{}|{}|{:?}|{:?}",
        d,
        ppp,
        ui.visuals().dark_mode,
        doc.settings.units,
        app.engine.indicator.pen_color
    );
    let id = egui::Id::new("dimline_preview_tex");
    let cached: Option<(String, egui::TextureHandle)> = ui.ctx().data(|x| x.get_temp(id));
    let tex = match cached {
        Some((k, t)) if k == key => Some(t),
        _ => {
            let paper = tk.paper;
            render_example(app, d, ppp, [paper.r(), paper.g(), paper.b()]).map(|img| {
                let t = ui
                    .ctx()
                    .load_texture("dimline-preview", img, egui::TextureOptions::LINEAR);
                ui.ctx().data_mut(|x| x.insert_temp(id, (key, t.clone())));
                t
            })
        }
    };
    if let Some(t) = tex {
        ui.painter().image(
            t.id(),
            r,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    ui.painter().rect_stroke(
        r,
        2.0,
        egui::Stroke::new(1.0_f32, tk.page_border),
        egui::StrokeKind::Inside,
    );
}

fn render_example(app: &App, d: &DimlineSettings, ppp: f32, paper: [u8; 3]) -> Option<egui::ColorImage> {
    let ppp = if ppp.is_finite() { ppp.clamp(0.5, 8.0) } else { 1.0 };
    let (pw, ph) = ((PREVIEW_W * ppp).round() as u32, (PREVIEW_H * ppp).round() as u32);
    let mut doc = Document::new();
    doc.colors = app.engine.doc().colors.clone();
    doc.settings = app.engine.doc().settings.clone();
    doc.objects = example_objects(app, d).into_iter().map(std::sync::Arc::new).collect();
    let scene = rustfig_render::build_scene(&doc, &SceneOptions::default());
    let mut pm = Pixmap::new(pw, ph)?;
    for px in pm.data_mut().as_chunks_mut::<4>().0 {
        *px = [paper[0], paper[1], paper[2], 255];
    }
    let k = f64::from(ppp) / PREVIEW_U;
    rasterize(
        &scene,
        &mut pm,
        &RasterOptions {
            transform: Affine::scale(k, k),
            min_line_px: 1.0,
            adjust_px: 0.0,
            pattern_k: 1,
            pattern: rustfig_render::pattern::PatternFrame::screen(),
        },
    );
    let pixels = pm
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
        .collect();
    Some(egui::ColorImage {
        size: [pw as usize, ph as usize],
        source_size: egui::vec2(pw as f32, ph as f32),
        pixels,
    })
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
        arrow_box(ui, "dimline_left_arrow", "Left", &mut d.left_arrow);
        arrow_box(ui, "dimline_right_arrow", "Right", &mut d.right_arrow);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `draw_cur_dimline`: the example is drawn with the chosen arrow types.
    #[test]
    fn preview_draws_the_chosen_arrow_types() {
        let app = App::with_engine(rustfig_engine::Engine::new());
        let image = |left: i32, right: i32| {
            let d = DimlineSettings {
                left_arrow: left,
                right_arrow: right,
                ..DimlineSettings::default()
            };
            render_example(&app, &d, 1.0, [255, 255, 255]).map(|i| i.pixels)
        };
        let base = image(2, 2);
        assert!(base.is_some());
        assert_ne!(base, image(-1, -1), "no arrows differ from triangles");
        assert_ne!(base, image(1, 2), "open and filled triangle differ");
        assert_ne!(base, image(6, 2), "another arrow type differs");
    }
}
