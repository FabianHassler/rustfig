//! The library panel (plan D6; xfig's "Select an object or library" window,
//! `w_library.c`): Load a Library, status, the objects as icons or a list,
//! icon size, the selected object with its comments, Cancel and Select
//! object. Select object runs `library.hand`; the object then follows the
//! pointer (library mode of the engine). Thumbnails are the objects drawn by
//! the canvas renderer.
//
// Ported from xfig src/w_library.c (panel layout and texts; xfig licence, see LICENSE-xfig).

use crate::App;
use egui::{vec2, Color32, Sense, TextureOptions};
use rustfig_engine::library::{self, Library};
use rustfig_geom::Affine;
use rustfig_model::Document;
use rustfig_render::raster::{rasterize, RasterOptions};
use rustfig_render::SceneOptions;
use serde_json::json;
use std::collections::HashMap;

/// xfig's icon sizes.
pub const ICON_SIZES: [u32; 5] = [40, 60, 80, 100, 120];
/// Thumbnails drawn per frame, so a big library opens at once.
const PER_FRAME: usize = 6;
/// Side of the preview, in points.
const PREVIEW: usize = 180;

/// State of the library panel.
#[derive(Default)]
pub struct LibraryPanel {
    pub open: bool,
    libs: Vec<Library>,
    /// The loaded library's name.
    pub current: Option<String>,
    objects: Vec<String>,
    /// The picked object.
    pub selected: Option<usize>,
    comments: String,
    thumbs: HashMap<(String, String, u32), Option<egui::TextureHandle>>,
    preview: Option<((String, String), Option<egui::TextureHandle>)>,
    thumb_px: u32,
    /// Copy to library: the figure's name and the folder in the user library.
    save_name: String,
    save_folder: String,
    message: String,
}

impl LibraryPanel {
    /// Open the panel (the engine asked for it): rescan the libraries.
    pub fn show_panel(&mut self, app_libs: Vec<Library>) {
        self.open = true;
        self.libs = app_libs;
        if let Some(c) = self.current.clone() {
            if !self.libs.iter().any(|l| l.name == c) {
                self.current = None;
                self.objects.clear();
                self.selected = None;
            }
        }
        if self.current.is_none() {
            if let Some(first) = self.libs.first().map(|l| l.name.clone()) {
                self.load(&first);
            }
        }
    }

    fn lib(&self) -> Option<&Library> {
        let c = self.current.as_deref()?;
        self.libs.iter().find(|l| l.name == c)
    }

    /// Load a library by name (the panel's "Load a Library" menu).
    pub fn load(&mut self, name: &str) {
        self.current = Some(name.to_string());
        self.objects = self
            .libs
            .iter()
            .find(|l| l.name == name)
            .map(|l| library::object_names(&l.path))
            .unwrap_or_default();
        self.selected = None;
        self.comments.clear();
        self.preview = None;
    }

    fn pick(&mut self, i: usize) {
        self.selected = Some(i);
        let Some(lib) = self.lib().cloned() else { return };
        let Some(name) = self.objects.get(i).cloned() else {
            return;
        };
        self.comments = library::object_comments(&lib.path, &name).join("\n");
        self.preview = None;
    }
}

/// The figure drawn into a square of `px` pixels (a margin around it).
fn render(doc: &Document, px: usize) -> Option<egui::ColorImage> {
    let bb = doc
        .objects
        .iter()
        .fold(rustfig_geom::Rect::EMPTY, |r, o| r.union(&o.geometry_bbox()));
    if bb.is_empty() || !bb.min.x.is_finite() || !bb.max.x.is_finite() || !bb.min.y.is_finite() || !bb.max.y.is_finite()
    {
        return None;
    }
    let (w, h) = ((bb.max.x - bb.min.x).max(1.0), (bb.max.y - bb.min.y).max(1.0));
    let margin = (px as f64 * 0.06).max(2.0);
    let s = ((px as f64 - 2.0 * margin) / w.max(h)).max(1e-6);
    let (ox, oy) = ((px as f64 - w * s) / 2.0, (px as f64 - h * s) / 2.0);
    let t = Affine::translate(-bb.min.x, -bb.min.y)
        .then(&Affine::scale(s, s))
        .then(&Affine::translate(ox, oy));
    let scene = rustfig_render::build_scene(doc, &SceneOptions::default());
    let mut pm = rustfig_render::raster::Pixmap::new(px as u32, px as u32)?;
    rasterize(
        &scene,
        &mut pm,
        &RasterOptions {
            transform: t,
            min_line_px: 1.0,
            pattern: rustfig_render::pattern::PatternFrame::screen(),
        },
    );
    Some(egui::ColorImage::from_rgba_premultiplied([px, px], pm.data()))
}

fn thumbnail(ctx: &egui::Context, lib: &Library, name: &str, px: usize, label: &str) -> Option<egui::TextureHandle> {
    let doc = library::read_figure(&lib.path, name).ok()?;
    let img = render(&doc, px)?;
    Some(ctx.load_texture(label, img, TextureOptions::LINEAR))
}

/// Show the window when it is open.
pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.library.open {
        return;
    }
    let mut open = true;
    let mut load: Option<String> = None;
    let mut pick: Option<usize> = None;
    let mut select = false;
    let mut cancel = false;
    let mut save = false;
    let icon_view = app
        .engine
        .prefs
        .get("icon_view")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let size = app
        .engine
        .prefs
        .get("library_icon_size")
        .and_then(|v| v.as_integer())
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(60)
        .clamp(ICON_SIZES[0], ICON_SIZES[ICON_SIZES.len() - 1]);
    let mut set_view: Option<bool> = None;
    let mut set_size: Option<u32> = None;
    let have_objects = !app.engine.doc().objects.is_empty();
    egui::Window::new("Select an object or library")
        .open(&mut open)
        .default_size(vec2(440.0, 560.0))
        .collapsible(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| ui.strong("Load a Library"));
            let p = &mut app.library;
            ui.horizontal(|ui| {
                ui.label("Library:");
                let text = match &p.current {
                    Some(c) => c.clone(),
                    None if p.libs.is_empty() => "No libraries".to_string(),
                    None => "None Loaded".to_string(),
                };
                let combo = egui::ComboBox::from_id_salt("library_choice").selected_text(text).width(300.0);
                let r = combo.show_ui(ui, |ui| {
                    for l in &p.libs {
                        let sel = p.current.as_deref() == Some(l.name.as_str());
                        let label = if l.user { format!("{} (yours)", l.name) } else { l.name.clone() };
                        if ui.selectable_label(sel, label).clicked() {
                            load = Some(l.name.clone());
                        }
                    }
                });
                r.response
                    .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "Library"));
            });
            ui.horizontal(|ui| {
                ui.label("Status:");
                let status = match &p.current {
                    Some(_) => format!("{} library objects in library", p.objects.len()),
                    None if p.libs.is_empty() => {
                        "No libraries found: set the preference library_dir".to_string()
                    }
                    None => String::new(),
                };
                ui.label(status);
            });
            ui.horizontal(|ui| {
                ui.label("Selected object:");
                let name = p.selected.and_then(|i| p.objects.get(i)).cloned().unwrap_or_default();
                ui.strong(name);
            });
            ui.horizontal(|ui| {
                ui.label("View:");
                egui::ComboBox::from_id_salt("library_view")
                    .selected_text(if icon_view { "Icon View" } else { "List View" })
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(icon_view, "Icon View").clicked() {
                            set_view = Some(true);
                        }
                        if ui.selectable_label(!icon_view, "List View").clicked() {
                            set_view = Some(false);
                        }
                    });
                if icon_view {
                    ui.label("Icon size:");
                    egui::ComboBox::from_id_salt("library_icon_size")
                        .selected_text(size.to_string())
                        .width(50.0)
                        .show_ui(ui, |ui| {
                            for s in ICON_SIZES {
                                if ui.selectable_label(s == size, s.to_string()).clicked() {
                                    set_size = Some(s);
                                }
                            }
                        });
                }
            });
            ui.separator();
            let lib = p.lib().cloned();
            let n = p.objects.len();
            let budget = std::cell::Cell::new(PER_FRAME);
            let pending = std::cell::Cell::new(false);
            let area = 240.0_f32;
            if let Some(lib) = lib {
                let px = size as usize;
                if p.thumb_px != size {
                    p.thumb_px = size;
                    p.thumbs.clear();
                }
                egui::ScrollArea::vertical()
                    .id_salt("library_objects")
                    .max_height(area)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if icon_view {
                            ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
                            ui.horizontal_wrapped(|ui| {
                                for i in 0..n {
                                    let Some(name) = p.objects.get(i).cloned() else { continue };
                                    let side = size as f32 + 6.0;
                                    let (rect, resp) = ui.allocate_exact_size(vec2(side, side), Sense::click());
                                    let selected = p.selected == Some(i);
                                    if ui.is_rect_visible(rect) {
                                        let key = (lib.name.clone(), name.clone(), size);
                                        if !p.thumbs.contains_key(&key) {
                                            if budget.get() > 0 {
                                                budget.set(budget.get() - 1);
                                                let t = thumbnail(ui.ctx(), &lib, &name, px, &format!("lib-{name}"));
                                                p.thumbs.insert(key.clone(), t);
                                            } else {
                                                pending.set(true);
                                            }
                                        }
                                        ui.painter().rect_filled(rect, 2.0, Color32::WHITE);
                                        if let Some(Some(tex)) = p.thumbs.get(&key) {
                                            let img = egui::Rect::from_min_size(rect.min + vec2(3.0, 3.0), vec2(size as f32, size as f32));
                                            ui.painter().image(
                                                tex.id(),
                                                img,
                                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                                Color32::WHITE,
                                            );
                                        }
                                        let stroke = if selected {
                                            egui::Stroke::new(2.0_f32, Color32::BLACK)
                                        } else {
                                            egui::Stroke::new(1.0_f32, Color32::from_gray(190))
                                        };
                                        ui.painter().rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Inside);
                                    }
                                    let label = format!("Library object {name}");
                                    resp.widget_info(|| {
                                        egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label.clone())
                                    });
                                    if resp.clone().on_hover_text(name.clone()).clicked() {
                                        pick = Some(i);
                                    }
                                    if resp.double_clicked() {
                                        pick = Some(i);
                                        select = true;
                                    }
                                }
                            });
                        } else {
                            for i in 0..n {
                                let Some(name) = p.objects.get(i).cloned() else { continue };
                                let r = ui.selectable_label(p.selected == Some(i), &name);
                                if r.clicked() {
                                    pick = Some(i);
                                }
                                if r.double_clicked() {
                                    pick = Some(i);
                                    select = true;
                                }
                            }
                        }
                    });
                if pending.get() {
                    ui.ctx().request_repaint();
                }
            } else {
                ui.allocate_space(vec2(1.0, area));
            }
            ui.separator();
            ui.horizontal(|ui| {
                // The preview of the picked object (xfig: List View's preview canvas).
                let (rect, _) = ui.allocate_exact_size(vec2(PREVIEW as f32 * 0.6, PREVIEW as f32 * 0.6), Sense::hover());
                ui.painter().rect_filled(rect, 2.0, Color32::WHITE);
                ui.painter()
                    .rect_stroke(rect, 2.0, egui::Stroke::new(1.0_f32, Color32::from_gray(150)), egui::StrokeKind::Inside);
                if let (Some(i), Some(lib)) = (p.selected, p.lib().cloned()) {
                    if let Some(name) = p.objects.get(i).cloned() {
                        let key = (lib.name.clone(), name.clone());
                        if p.preview.as_ref().map(|(k, _)| k) != Some(&key) {
                            let t = thumbnail(ui.ctx(), &lib, &name, PREVIEW, "lib-preview");
                            p.preview = Some((key, t));
                        }
                        if let Some((_, Some(tex))) = &p.preview {
                            ui.painter().image(
                                tex.id(),
                                rect,
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        }
                    }
                }
                ui.vertical(|ui| {
                    ui.strong("Object comments:");
                    let mut c = p.comments.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut c)
                            .desired_rows(4)
                            .desired_width(f32::INFINITY)
                            .interactive(false),
                    )
                    .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, false, "Object comments"));
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
                let can = p.selected.is_some();
                if ui.add_enabled(can, egui::Button::new("Select object")).clicked() {
                    select = true;
                }
            });
            ui.collapsing("Copy to library", |ui| {
                ui.label("Saves the selected objects (else the whole figure) as a library figure in your library folder (preference library_dir).");
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut p.save_name)
                        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Library figure name"));
                });
                ui.horizontal(|ui| {
                    ui.label("Library folder:");
                    ui.text_edit_singleline(&mut p.save_folder)
                        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Library folder"));
                });
                let ok = !p.save_name.trim().is_empty() && have_objects;
                if ui.add_enabled(ok, egui::Button::new("Copy to library")).clicked() {
                    save = true;
                }
                if !p.message.is_empty() {
                    ui.label(p.message.clone());
                }
            });
        });
    if let Some(v) = set_view {
        app.run("prefs.set", json!({"resource": "icon_view", "value": v}));
    }
    if let Some(s) = set_size {
        app.run("prefs.set", json!({"resource": "library_icon_size", "value": s}));
    }
    if let Some(l) = load {
        app.library.load(&l);
    }
    if let Some(i) = pick {
        app.library.pick(i);
    }
    if save {
        let name = app.library.save_name.trim().to_string();
        let mut p = json!({"name": name});
        let folder = app.library.save_folder.trim().to_string();
        if !folder.is_empty() {
            p["library"] = json!(folder);
        }
        let r = app.try_run("library.save", &p);
        app.library.message = match r {
            Ok(v) => {
                let libs = app.engine.libraries();
                let cur = app.library.current.clone();
                app.library.show_panel(libs);
                if let Some(c) = cur {
                    if app.library.libs.iter().any(|l| l.name == c) {
                        app.library.load(&c);
                    }
                }
                format!("Saved {}", v["path"].as_str().unwrap_or(""))
            }
            Err(e) => e,
        };
    }
    if select {
        let (lib, obj) = (
            app.library.current.clone(),
            app.library.selected.and_then(|i| app.library.objects.get(i).cloned()),
        );
        if let (Some(lib), Some(obj)) = (lib, obj) {
            if app.run("library.hand", json!({"library": lib, "name": obj})).is_some() {
                app.library.open = false;
            }
        }
    }
    if cancel || !open {
        app.library.open = false;
    }
}
