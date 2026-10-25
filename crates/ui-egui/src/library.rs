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
use rustfig_engine::library::{self, LibRoot, Library};
use rustfig_geom::Affine;
use rustfig_model::Document;
use rustfig_render::raster::{rasterize, RasterOptions};
use rustfig_render::SceneOptions;
use serde_json::json;
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{atomic::AtomicBool, atomic::Ordering, mpsc, Arc};

/// xfig's icon sizes.
pub const ICON_SIZES: [u32; 5] = [40, 60, 80, 100, 120];
/// Thumbnails drawn per frame, so a big library opens at once.
const PER_FRAME: usize = 6;
/// Side of the preview, in points.
const PREVIEW: usize = 180;

/// A library scan running on its own thread (xfig's Stop button stops it).
#[cfg(not(target_arch = "wasm32"))]
struct ScanJob {
    rx: mpsc::Receiver<Vec<Library>>,
    cancel: Arc<AtomicBool>,
}

/// State of the library panel.
#[derive(Default)]
pub struct LibraryPanel {
    pub open: bool,
    #[cfg(not(target_arch = "wasm32"))]
    scan: Option<ScanJob>,
    /// The browser: the roots to scan once the served index is in (C2).
    #[cfg(target_arch = "wasm32")]
    web_wait: Option<Vec<LibRoot>>,
    /// Stop was pressed: the icons not yet drawn stay blank until Resume.
    pub stopped: bool,
    /// The folder or list file field (preference `library_dir`).
    pub source: String,
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
    /// The search text, and the text `hits` were found for.
    query: String,
    searched: String,
    /// Search results: library, figure, found by comment.
    hits: Vec<(String, String, bool)>,
    truncated: bool,
}

impl LibraryPanel {
    /// Open the panel (the engine asked for it) and scan the library
    /// folders on a thread: a big library list does not freeze the window,
    /// and Stop ends the scan with what was found. `source` is the
    /// preference `library_dir`.
    pub fn start_scan(&mut self, roots: Vec<LibRoot>, source: String) {
        self.open = true;
        self.stopped = false;
        self.source = source;
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(old) = self.scan.take() {
                old.cancel.store(true, Ordering::Relaxed);
            }
            let cancel = Arc::new(AtomicBool::new(false));
            let (tx, rx) = mpsc::channel();
            let (c, r) = (cancel.clone(), roots.clone());
            let spawned = std::thread::Builder::new()
                .name("library-scan".into())
                .spawn(move || {
                    // The panel may be gone by now: nobody to tell then.
                    let _ = tx.send(library::scan_cancellable(&r, &c));
                })
                .is_ok();
            if spawned {
                // Small libraries are there at once; a big one finishes in the background.
                match rx.recv_timeout(std::time::Duration::from_millis(250)) {
                    Ok(libs) => self.show_panel(libs),
                    Err(_) => self.scan = Some(ScanJob { rx, cancel }),
                }
                return;
            }
        }
        // The browser: the bundle's index is fetched first (C2).
        #[cfg(target_arch = "wasm32")]
        if !library::web::index_loaded() {
            crate::web_fetch::request_index();
            self.web_wait = Some(roots);
            self.show_panel(Vec::new());
            return;
        }
        self.show_panel(library::scan(&roots));
    }

    /// How many icons are drawn so far (tests).
    pub fn icons_drawn(&self) -> usize {
        self.thumbs.len()
    }

    /// Whether a scan is still running.
    pub fn scanning(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.scan.is_some()
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.web_wait.is_some()
        }
    }

    /// Take the scan's result once it is in.
    fn poll_scan(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            if library::web::index_loaded() {
                if let Some(roots) = self.web_wait.take() {
                    self.show_panel(library::scan(&roots));
                }
            } else if self.web_wait.is_some() {
                if let Some(e) = crate::web_fetch::take_index_error() {
                    self.web_wait = None;
                    self.message = format!("The libraries could not be loaded: {e}");
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(job) = &self.scan {
            match job.rx.try_recv() {
                Ok(libs) => {
                    self.scan = None;
                    self.show_panel(libs);
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => self.scan = None,
            }
        }
    }

    /// xfig's Stop: end the scan and stop drawing icons.
    pub fn stop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(job) = &self.scan {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.stopped = true;
    }

    /// Set the libraries a scan found.
    pub fn show_panel(&mut self, app_libs: Vec<Library>) {
        self.open = true;
        // The libraries may have changed: search again.
        self.searched = "\u{0}".into();
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
        self.stopped = false;
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

    /// Pick a search result: load its library and pick the figure.
    fn pick_hit(&mut self, library: &str, name: &str) {
        if self.current.as_deref() != Some(library) {
            self.load(library);
        }
        if let Some(i) = self.objects.iter().position(|n| n == name) {
            self.pick(i);
        }
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
    let scene = rustfig_render::build_scene(
        doc,
        &SceneOptions {
            approx_latex: true,
            ..SceneOptions::default()
        },
    );
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

/// The figure's texture: `None` while the browser still fetches the figure
/// (ask again later), `Some(None)` when it can't be drawn.
fn thumbnail(
    ctx: &egui::Context,
    lib: &Library,
    name: &str,
    px: usize,
    label: &str,
) -> Option<Option<egui::TextureHandle>> {
    #[cfg(target_arch = "wasm32")]
    if !crate::web_fetch::figure_ready(&lib.path, name) {
        return None;
    }
    let tex = library::read_figure(&lib.path, name)
        .ok()
        .and_then(|doc| render(&doc, px))
        .map(|img| ctx.load_texture(label, img, TextureOptions::LINEAR));
    Some(tex)
}

/// Show the window when it is open.
pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.library.open {
        return;
    }
    let mut open = true;
    app.library.poll_scan();
    if app.library.scanning() {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
    let mut stop = false;
    let mut resume = false;
    let mut use_source = false;
    let mut load: Option<String> = None;
    let mut pick: Option<usize> = None;
    let mut pick_hit: Option<(String, String)> = None;
    if app.library.open && app.library.query != app.library.searched {
        let q = app.library.query.clone();
        app.library.searched = q.clone();
        app.library.hits.clear();
        app.library.truncated = false;
        if !q.trim().is_empty() {
            if let Ok(v) = app.try_run("library.search", &json!({"query": q, "limit": 300})) {
                let s = |h: &serde_json::Value, k: &str| h[k].as_str().unwrap_or("").to_string();
                app.library.hits = v["hits"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|h| (s(h, "library"), s(h, "name"), h["comment"].as_bool().unwrap_or(false)))
                            .collect()
                    })
                    .unwrap_or_default();
                app.library.truncated = v["truncated"].as_bool().unwrap_or(false);
            }
        }
    }
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
                ui.label("Source:");
                if cfg!(target_arch = "wasm32") {
                    ui.label(source_text(true, &p.source));
                    return;
                }
                let r = ui.add(
                    egui::TextEdit::singleline(&mut p.source)
                        .hint_text("folder, or a list file naming several folders")
                        .desired_width(260.0),
                );
                r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Library source"));
                if ui.button("Use").clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                    use_source = true;
                }
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
            ui.horizontal(|ui| {
                ui.label("Search:");
                let r = ui.add(
                    egui::TextEdit::singleline(&mut p.query)
                        .hint_text("name or comment, in all libraries")
                        .desired_width(220.0),
                );
                r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Search libraries"));
                if !p.query.is_empty() && ui.button("Clear").clicked() {
                    p.query.clear();
                }
            });
            ui.separator();
            let searching = !p.query.trim().is_empty();
            let lib = p.lib().cloned();
            let n = p.objects.len();
            let budget = std::cell::Cell::new(PER_FRAME);
            let pending = std::cell::Cell::new(false);
            let area = 240.0_f32;
            if searching {
                egui::ScrollArea::vertical()
                    .id_salt("library_search_results")
                    .max_height(area)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if p.hits.is_empty() {
                            ui.label("No figure found.");
                        }
                        for (l, n, by_comment) in &p.hits {
                            let sel = p.current.as_deref() == Some(l.as_str())
                                && p.selected.and_then(|i| p.objects.get(i)) == Some(n);
                            let text = format!("{n}   ({l}{})", if *by_comment { ", comment" } else { "" });
                            let r = ui.selectable_label(sel, text);
                            let label = format!("Search result {n} in {l}");
                            r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, sel, label.clone()));
                            if r.clicked() {
                                pick_hit = Some((l.clone(), n.clone()));
                            }
                            if r.double_clicked() {
                                pick_hit = Some((l.clone(), n.clone()));
                                select = true;
                            }
                        }
                        if p.truncated {
                            ui.weak("More figures match: type more of the name.");
                        }
                    });
            } else if let Some(lib) = lib {
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
                                            if p.stopped {
                                                // Stop was pressed: leave it blank.
                                            } else if budget.get() > 0 {
                                                budget.set(budget.get() - 1);
                                                match thumbnail(ui.ctx(), &lib, &name, px, &format!("lib-{name}")) {
                                                    Some(t) => {
                                                        p.thumbs.insert(key.clone(), t);
                                                    }
                                                    // Still being fetched (the browser).
                                                    None => pending.set(true),
                                                }
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
            let scanning = p.scanning();
            if scanning || pending.get() || p.stopped {
                ui.horizontal(|ui| {
                    if scanning || pending.get() {
                        ui.spinner();
                        ui.label(if scanning { "Scanning libraries..." } else { "Loading icons..." });
                        if ui.button("Stop").clicked() {
                            stop = true;
                        }
                    } else {
                        ui.label("Stopped: icons not drawn yet.");
                        if ui.button("Resume").clicked() {
                            resume = true;
                        }
                    }
                });
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
                            if let Some(t) = thumbnail(ui.ctx(), &lib, &name, PREVIEW, "lib-preview") {
                                // The browser reads the comments once the figure is in.
                                #[cfg(target_arch = "wasm32")]
                                {
                                    p.comments = library::object_comments(&lib.path, &name).join("\n");
                                }
                                p.preview = Some((key, t));
                            } else {
                                ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
                            }
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
    if stop {
        app.library.stop();
    }
    if resume {
        app.library.stopped = false;
    }
    if use_source {
        let path = app.library.source.clone();
        app.library.message = match app.try_run("library.use", &json!({"path": path})) {
            Ok(_) => {
                let roots = app.engine.library_roots();
                app.library.start_scan(roots, path);
                String::new()
            }
            Err(m) => m,
        };
    }
    if let Some(l) = load {
        app.library.load(&l);
    }
    if let Some(i) = pick {
        app.library.pick(i);
    }
    if let Some((l, n)) = pick_hit {
        app.library.pick_hit(&l, &n);
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

/// The library "Source" line: on the web the bundled libraries (there is no
/// folder to choose), on the desktop the folder as typed.
pub fn source_text(web: bool, source: &str) -> String {
    if web {
        "bundled libraries".to_string()
    } else {
        source.to_string()
    }
}

#[cfg(test)]
mod source_tests {
    use super::source_text;

    #[test]
    fn web_source_is_bundled() {
        assert_eq!(source_text(true, "~/xfiglib"), "bundled libraries");
        assert_eq!(source_text(false, "~/xfiglib"), "~/xfiglib");
    }
}
