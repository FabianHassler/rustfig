//! File > Print… (plan B5): xfig's Print panel with a live page preview.
//!
//! The panel keeps xfig's rows (magnification and Fit to Page, paper size,
//! orientation, justification, single or multiple pages with overlap,
//! background, grid, layers, the batch); the preview is the PDF that Print
//! hands to the operating system's own print dialog (`rustfig-os`), where
//! printer, copies and page ranges are chosen; it is rendered again when an
//! option changes. Everything it does is a command (`print.*`, `doc.set_*`,
//! `export.fit_page`).
//
// Ported from xfig src/w_print.c (the panel's rows) (xfig licence, see LICENSE-xfig).

use crate::dialogs::{text_combo, GRID_INCH, GRID_MM};
use crate::App;
use rustfig_engine::export_panel::{figure_size, COLOR_NONE};
use rustfig_engine::print::{self, PrintSettings};
use rustfig_model::{Orientation, PaperSize, Units};
use serde_json::json;

/// Largest figure (objects) previewed live: bigger ones would stall the panel.
const MAX_PREVIEW_OBJECTS: usize = 20_000;
/// Pixels along the preview page's longer side.
const PREVIEW_EDGE: u32 = 720;

#[derive(Default)]
pub struct PreviewState {
    /// What the preview was made from; made again when this changes.
    key: String,
    pdf: Vec<u8>,
    pub pages: usize,
    pub page: usize,
    page_key: String,
    pub texture: Option<egui::TextureHandle>,
    pub error: Option<String>,
}

impl std::fmt::Debug for PreviewState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PreviewState({} pages)", self.pages)
    }
}

#[derive(Debug, Default)]
pub struct PrintState {
    pub open: bool,
    /// The panel's values, kept in the settings file as soon as they change.
    pub settings: PrintSettings,
    stored: PrintSettings,
    /// The figure's magnification as shown (percent).
    pub magnification: f64,
    loaded: bool,
    pub preview: PreviewState,
}

/// What a click asked for, done after the panel is drawn.
#[derive(Default)]
struct Actions {
    fit: bool,
    print: bool,
    add: bool,
    print_batch: bool,
    clear: bool,
    close: bool,
    magnification: Option<f64>,
    page: Vec<(&'static str, &'static str)>,
    paper: Option<PaperSize>,
}

/// Shows the operating system's print dialog for each job (`rustfig-os`).
/// The engine runs commands on the UI thread, which the macOS dialog needs.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsSpooler;

impl print::Spooler for OsSpooler {
    fn submit(&mut self, job: &print::PrintJob) -> Result<Option<String>, String> {
        match rustfig_os::print::print_pdf(&job.pdf, &job.name)? {
            rustfig_os::print::PrintOutcome::Printed => Ok(Some(String::new())),
            rustfig_os::print::PrintOutcome::Cancelled => Ok(None),
        }
    }
}

pub fn dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.print.open {
        app.dialogs.print.loaded = false;
        return;
    }
    if !app.dialogs.print.loaded {
        let s = print::settings_of(&app.engine);
        let st = &mut app.dialogs.print;
        st.settings = s.clone();
        st.stored = s;
        st.magnification = app.engine.doc().settings.magnification;
        st.loaded = true;
    }
    let d = app.engine.doc().settings.clone();
    let landscape = d.orientation == Orientation::Landscape;
    let (bw, bh, unit) = figure_size(&app.engine.doc_bounds(), d.units, app.dialogs.print.magnification);
    let batch = app.engine.print_batch_len();
    let some_hidden = !app.engine.hidden_depths.is_empty();
    let mut open = true;
    let mut act = Actions::default();
    let st = &mut app.dialogs.print;
    let doc = app.engine.doc();
    egui::Window::new("Print")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            crate::dialog_body(ui, |ui| {
                ui.horizontal_top(|ui| {
                    egui::Grid::new("print_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Magnification");
                            ui.horizontal(|ui| {
                                let r = ui.add(
                                    egui::DragValue::new(&mut st.magnification)
                                        .range(1.0..=10000.0)
                                        .suffix(" %"),
                                );
                                if r.changed() && !r.dragged() || r.drag_stopped() {
                                    act.magnification = Some(st.magnification);
                                }
                                if ui.button("Fit to Page").clicked() {
                                    act.fit = true;
                                }
                                ui.label("Figure size");
                                ui.label(format!("{bw:.1}{unit} x {bh:.1}{unit}"));
                            });
                            ui.end_row();
                            ui.label("Paper size");
                            ui.horizontal(|ui| {
                                let combo = egui::ComboBox::from_id_salt("print_paper")
                                    .selected_text(d.paper.name())
                                    .show_ui(ui, |ui| {
                                        for p in PaperSize::ALL {
                                            if ui.selectable_label(p == d.paper, p.name()).clicked() {
                                                act.paper = Some(p);
                                            }
                                        }
                                    });
                                combo.response.widget_info(|| {
                                    egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, d.paper.name())
                                });
                                ui.label("Orientation");
                                if ui.radio(!landscape, "Portrait").clicked() {
                                    act.page.push(("orientation", "portrait"));
                                }
                                if ui.radio(landscape, "Landscape").clicked() {
                                    act.page.push(("orientation", "landscape"));
                                }
                            });
                            ui.end_row();
                            ui.label("Justification");
                            ui.horizontal(|ui| {
                                if ui.radio(d.center, "Centered").clicked() {
                                    act.page.push(("justify", "center"));
                                }
                                if ui.radio(!d.center, "Flush left").clicked() {
                                    act.page.push(("justify", "flush_left"));
                                }
                            });
                            ui.end_row();
                            ui.label("Pages");
                            ui.horizontal(|ui| {
                                if ui.radio(!d.multiple_pages, "Single").clicked() {
                                    act.page.push(("pages", "single"));
                                }
                                if ui.radio(d.multiple_pages, "Multiple").clicked() {
                                    act.page.push(("pages", "multiple"));
                                }
                                ui.add_enabled(
                                    d.multiple_pages,
                                    egui::Checkbox::new(&mut st.settings.overlap, "Overlap"),
                                );
                            });
                            ui.end_row();
                            ui.label("Background");
                            if let Some(c) = crate::panels::color_menu(
                                ui,
                                "print_background",
                                doc,
                                st.settings.background,
                                &[(COLOR_NONE, "None")],
                            ) {
                                st.settings.background = c;
                            }
                            ui.end_row();
                            ui.label("Grid");
                            ui.horizontal(|ui| {
                                let choices: &[&str] = if d.units == Units::Metric { &GRID_MM } else { &GRID_INCH };
                                ui.label("Minor");
                                if let Some(c) = text_combo(ui, "print_grid_minor", &st.settings.grid_minor, choices) {
                                    st.settings.grid_minor = c;
                                }
                                ui.label("Major");
                                if let Some(c) = text_combo(ui, "print_grid_major", &st.settings.grid_major, choices) {
                                    st.settings.grid_major = c;
                                }
                                ui.label(if d.units == Units::Metric { "mm" } else { "in" });
                            });
                            ui.end_row();
                            ui.label("Layers");
                            ui.horizontal_wrapped(|ui| {
                                ui.radio_value(&mut st.settings.all_layers, true, "Print all layers");
                                ui.add_enabled_ui(some_hidden || !st.settings.all_layers, |ui| {
                                    ui.radio_value(&mut st.settings.all_layers, false, "Print only active")
                                        .on_hover_text("Leave out the depths switched off in the depth manager");
                                    ui.add_enabled(
                                        !st.settings.all_layers,
                                        egui::Checkbox::new(
                                            &mut st.settings.bound_active_layers,
                                            "Page sized by the active layers",
                                        ),
                                    );
                                });
                            });
                            ui.end_row();
                        });
                    ui.vertical(|ui| {
                        ui.label("Preview");
                        preview_ui(ui, &mut st.preview);
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Dismiss").clicked() {
                        act.close = true;
                    }
                    if ui
                        .button("Print FIGURE")
                        .on_hover_text("Open the system's print dialog")
                        .clicked()
                    {
                        act.print = true;
                    }
                    if ui.button("Add to Batch").clicked() {
                        act.add = true;
                    }
                    if ui.add_enabled(batch > 0, egui::Button::new("Print Batch")).clicked() {
                        act.print_batch = true;
                    }
                    if ui.add_enabled(batch > 0, egui::Button::new("Clear Batch")).clicked() {
                        act.clear = true;
                    }
                    ui.label("Figures in batch");
                    ui.label(batch.to_string());
                });
            })
        });
    app.dialogs.print.open = open && !act.close;
    apply(app, ctx, act);
}

/// The preview page with page buttons.
fn preview_ui(ui: &mut egui::Ui, p: &mut PreviewState) {
    match (&p.texture, &p.error) {
        (Some(t), _) => {
            let s = t.size_vec2();
            let k = (300.0 / s.x.max(1.0)).min(300.0 / s.y.max(1.0));
            let r = ui.add(egui::Image::new((t.id(), s * k)).bg_fill(egui::Color32::WHITE));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, "Print preview"));
        }
        (None, Some(e)) => {
            ui.weak(e.as_str());
        }
        (None, None) => {
            ui.weak("(nothing to show)");
        }
    }
    ui.horizontal(|ui| {
        if ui.add_enabled(p.page > 0, egui::Button::new("<")).clicked() {
            p.page = p.page.saturating_sub(1);
        }
        ui.label(format!("Page {} of {}", p.page + 1, p.pages.max(1)));
        if ui.add_enabled(p.page + 1 < p.pages, egui::Button::new(">")).clicked() {
            p.page += 1;
        }
    });
}

/// Do what the panel asked for, keep its values, and refresh the preview.
fn apply(app: &mut App, ctx: &egui::Context, act: Actions) {
    if let Some(p) = act.paper {
        app.run("doc.set_paper", json!({ "paper": p.name() }));
    }
    for (k, v) in act.page {
        app.run("doc.set_page", json!({ k: v }));
    }
    if act.fit {
        if let Some(r) = app.run("export.fit_page", json!({})) {
            if let Some(m) = r["magnification"].as_f64() {
                app.dialogs.print.magnification = m;
            }
        }
    }
    if let Some(m) = act.magnification {
        if (m - app.engine.doc().settings.magnification).abs() > 1e-9 {
            app.run("doc.set_magnification", json!({ "magnification": m }));
        }
    }
    // The values are kept across sessions as soon as they change.
    if app.dialogs.print.settings != app.dialogs.print.stored {
        if let Ok(v) = serde_json::to_value(&app.dialogs.print.settings) {
            app.run("print.settings", json!({ "settings": v }));
        }
        let s = print::settings_of(&app.engine);
        app.dialogs.print.settings = s.clone();
        app.dialogs.print.stored = s;
    }
    if app.dialogs.print.open {
        refresh_preview(app, ctx);
    }
    if act.clear {
        app.run("print.batch_clear", json!({}));
    }
    if act.add {
        if let Some(r) = app.run("print.batch_add", json!({})) {
            app.info(format!("Added to the batch ({} figures)", r["batch"]));
        }
    }
    if act.print {
        app.run("print.run", json!({}));
    }
    if act.print_batch {
        app.run("print.batch_print", json!({}));
    }
}

/// The page picture, made again only when the figure or an option changes.
fn refresh_preview(app: &mut App, ctx: &egui::Context) {
    let st = &app.dialogs.print;
    let d = &app.engine.doc().settings;
    let key = format!(
        "{}|{:?}|{:?}|{:?}|{}",
        app.engine.revision(),
        st.settings,
        app.engine.hidden_depths,
        (d.magnification, d.paper, d.orientation, d.center, d.multiple_pages),
        app.engine.doc().leaf_count()
    );
    if st.preview.key != key {
        let settings = st.settings.clone();
        let p = &mut app.dialogs.print.preview;
        p.key = key;
        p.page_key.clear();
        p.pdf.clear();
        p.pages = 0;
        p.texture = None;
        p.error = None;
        if app.engine.doc().leaf_count() > MAX_PREVIEW_OBJECTS {
            app.dialogs.print.preview.error = Some("(figure too large for a preview)".into());
        } else {
            match print::make_pdf(&app.engine, &settings) {
                Ok((pdf, _)) => {
                    let p = &mut app.dialogs.print.preview;
                    p.pages = print::pdf_info(&pdf).map_or(0, |(n, _, _)| n);
                    p.pdf = pdf;
                }
                Err(e) => app.dialogs.print.preview.error = Some(format!("({})", e.message)),
            }
        }
    }
    let p = &mut app.dialogs.print.preview;
    if p.pdf.is_empty() {
        return;
    }
    p.page = p.page.min(p.pages.saturating_sub(1));
    let page_key = format!("{}|{}", p.key, p.page);
    if p.page_key == page_key {
        return;
    }
    p.page_key = page_key;
    match print::render_page(&p.pdf, p.page, PREVIEW_EDGE) {
        Ok(img) => {
            let ci = egui::ColorImage::from_rgba_unmultiplied([img.width as usize, img.height as usize], &img.rgba);
            p.texture = Some(ctx.load_texture("print_preview", ci, egui::TextureOptions::LINEAR));
            p.error = None;
        }
        Err(e) => {
            p.texture = None;
            p.error = Some(format!("({e})"));
        }
    }
}
