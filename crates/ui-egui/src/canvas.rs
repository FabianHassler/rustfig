//! The drawing canvas: document raster, overlay, grid, page border and
//! pointer input.

use crate::theme::Tokens;
use crate::App;
use egui::{pos2, vec2, Color32, Pos2, Rect, Sense, Shape, Stroke, TextureHandle, TextureOptions};
use rustfig_engine::{Button, Overlay, ToolEvent};
use rustfig_geom::{Affine, Point};
use rustfig_model::{Document, ObjectId, ObjectKind, Orientation};
use rustfig_render::raster::{rasterize, RasterOptions};
use rustfig_render::{Scene, SceneOptions};

#[derive(Debug, Clone, PartialEq)]
struct DocKey {
    revision: u64,
    zoom: u64,
    ox: u64,
    oy: u64,
    w: usize,
    h: usize,
    hide: Vec<ObjectId>,
}

impl DocKey {
    /// The view the raster was made for.
    fn view(&self, ppp: f32) -> rustfig_engine::View {
        let mut v = rustfig_engine::View {
            zoom: f64::from_bits(self.zoom),
            origin: Point::new(f64::from_bits(self.ox), f64::from_bits(self.oy)),
            ..Default::default()
        };
        v.size = (self.w as f64 / f64::from(ppp), self.h as f64 / f64::from(ppp));
        v
    }

    /// Same drawing, maybe another view.
    fn same_drawing(&self, o: &DocKey) -> bool {
        self.revision == o.revision && self.hide == o.hide
    }
}

/// A document raster being made on a background thread (A7).
#[cfg(not(target_arch = "wasm32"))]
struct Job {
    key: DocKey,
    /// The raster and how long it took (ms).
    rx: std::sync::mpsc::Receiver<(egui::ColorImage, f64)>,
}

#[derive(Default)]
pub struct CanvasState {
    doc_tex: Option<TextureHandle>,
    /// What `doc_tex` shows (it may lag behind the view while a new raster
    /// is made; it is then drawn moved and scaled to the current view).
    doc_key: Option<DocKey>,
    /// The scene of the current drawing, kept across pans and zooms.
    scene: Option<(DocKey, std::sync::Arc<Scene>)>,
    #[cfg(not(target_arch = "wasm32"))]
    job: Option<Job>,
    ov_tex: Option<TextureHandle>,
    ov_key: Option<(Overlay, DocKey)>,
    /// Pointer position in Fig units, if over the canvas.
    pub cursor: Option<Point>,
    last_sent: Option<Point>,
    /// The tool button of a press that started on the canvas.
    pressing: Option<(egui::PointerButton, Button)>,
    /// Panning with space+drag.
    panning: bool,
    /// Milliseconds the last document render took.
    pub render_ms: f64,
}

impl CanvasState {
    /// Drop cached rasters so the next frame redraws everything.
    pub fn invalidate(&mut self) {
        self.doc_key = None;
        self.ov_key = None;
        self.scene = None;
    }
}

/// Side of an object marker in points.
const MARKER: f32 = 7.0;

fn to_pos(p: Point, origin: Pos2) -> Pos2 {
    pos2(origin.x + p.x as f32, origin.y + p.y as f32)
}

/// The scene of `doc` minus `hide`.
fn build_scene(doc: &Document, opts: &SceneOptions, hide: &[ObjectId]) -> Scene {
    if hide.is_empty() {
        rustfig_render::build_scene(doc, opts)
    } else {
        let mut d = Document {
            settings: doc.settings.clone(),
            comments: Vec::new(),
            export_on_save: doc.export_on_save.clone(),
            colors: doc.colors.clone(),
            objects: Vec::with_capacity(doc.objects.len()),
            next_id: 0,
            open: doc.open.clone(),
        };
        d.objects = doc.objects.iter().filter(|o| !hide.contains(&o.id)).cloned().collect();
        rustfig_render::build_scene(&d, opts)
    }
}

/// A render that took this long or more goes to a background thread next
/// time (native), so pans and zooms never wait for it.
const SYNC_RENDER_MS: f64 = 20.0;

/// Show `img`, the raster for `key`.
fn set_doc_texture(app: &mut App, ctx: &egui::Context, img: egui::ColorImage, key: DocKey) {
    match &mut app.canvas.doc_tex {
        Some(tex) => tex.set(img, TextureOptions::NEAREST),
        None => app.canvas.doc_tex = Some(ctx.load_texture("doc", img, TextureOptions::NEAREST)),
    }
    app.canvas.doc_key = Some(key);
}

fn rasterize_scene(scene: &Scene, t: Affine, w: usize, h: usize) -> egui::ColorImage {
    let Some(mut pm) = tiny_skia_pixmap(w, h) else {
        return egui::ColorImage::new([1, 1], vec![Color32::TRANSPARENT]);
    };
    rasterize(
        scene,
        &mut pm,
        &RasterOptions {
            transform: t,
            min_line_px: 1.0,
            pattern: rustfig_render::pattern::PatternFrame::screen(),
        },
    );
    egui::ColorImage::from_rgba_premultiplied([w, h], pm.data())
}

/// The grid step (Fig units) and its spacing on screen (points). As xfig's
/// `setup_grid` (w_grid.c), a grid too fine for the zoom shows the next
/// coarser one; the caller hides it when even the coarsest is under 5 px.
fn grid_spacing(g: rustfig_engine::SnapMode, units: rustfig_model::Units, px_per_fig: f64) -> (f64, f64) {
    let all = rustfig_engine::SnapMode::ALL;
    let from = all.iter().position(|&s| s == g).unwrap_or(0);
    let mut best = (0.0, 0.0);
    for s in all.iter().skip(from) {
        let step = s.step(units);
        if step <= 0.0 {
            continue;
        }
        best = (step, step * px_per_fig);
        if best.1 >= 5.0 {
            break;
        }
    }
    best
}

fn tiny_skia_pixmap(w: usize, h: usize) -> Option<rustfig_render::raster::Pixmap> {
    rustfig_render::raster::Pixmap::new(w.clamp(1, 16_384) as u32, h.clamp(1, 16_384) as u32)
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tk: &Tokens) {
    let area = ui.available_rect_before_wrap();
    // xfig's rulers: top, right side, and the units box in the corner.
    let (rect, top, side, unit_box) = crate::rulers::layout(area);
    let resp = ui.allocate_rect(rect, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    let ppp = ui.ctx().pixels_per_point();
    app.engine.view.size = (f64::from(rect.width()), f64::from(rect.height()));
    let view = app.engine.view;
    let origin = rect.min;
    let to_screen = |p: Point| to_pos(view.to_screen(p), origin);

    painter.rect_filled(rect, 0.0, tk.canvas_bg);

    // Paper.
    let doc = app.engine.doc();
    let (pw, ph) = doc.settings.paper.inches();
    let (pw, ph) = match doc.settings.orientation {
        Orientation::Landscape => (ph, pw),
        Orientation::Portrait => (pw, ph),
    };
    let paper = Rect::from_two_pos(to_screen(Point::ZERO), to_screen(Point::new(pw * 1200.0, ph * 1200.0)));
    painter.rect_filled(paper, 0.0, tk.paper);
    if app.engine.show_page_border {
        painter.rect_stroke(
            paper,
            0.0,
            Stroke::new(1.0_f32, tk.page_border),
            egui::StrokeKind::Outside,
        );
        painter.text(
            paper.left_top() + vec2(4.0, -4.0),
            egui::Align2::LEFT_BOTTOM,
            doc.settings.paper.name(),
            egui::FontId::proportional(11.0),
            tk.page_border,
        );
    }

    // Grid.
    if let Some(g) = app.engine.indicator.grid {
        let (step, px) = grid_spacing(g, doc.settings.units, view.px_per_fig());
        if step > 0.0 && px >= 5.0 {
            let vis = view.visible();
            let stroke = Stroke::new(1.0_f32, tk.grid);
            if app.engine.indicator.grid_iso {
                // xfig's isometric grid (w_grid.c): vertical lines `step * cos 30`
                // apart and diagonals at plus and minus 30 degrees, a step apart.
                let xs = step * 0.75_f64.sqrt();
                let slope = 30.0_f64.to_radians().tan();
                let mut i = (vis.min.x / xs).floor();
                let mut n = 0;
                while i * xs <= vis.max.x && n < 4000 {
                    let sx = to_screen(Point::new(i * xs, 0.0)).x;
                    painter.line_segment([pos2(sx, rect.top()), pos2(sx, rect.bottom())], stroke);
                    i += 1.0;
                    n += 1;
                }
                let (x0, x1) = (vis.min.x, vis.max.x);
                for sign in [1.0, -1.0] {
                    let (lo, hi) = if sign > 0.0 {
                        (vis.min.y - x1 * slope, vis.max.y - x0 * slope)
                    } else {
                        (vis.min.y + x0 * slope, vis.max.y + x1 * slope)
                    };
                    let mut k = (lo / step).floor();
                    let mut n = 0;
                    while k * step <= hi && n < 4000 {
                        let y = k * step;
                        let a = to_screen(Point::new(x0, y + sign * x0 * slope));
                        let b = to_screen(Point::new(x1, y + sign * x1 * slope));
                        painter.line_segment([pos2(a.x, a.y), pos2(b.x, b.y)], stroke);
                        k += 1.0;
                        n += 1;
                    }
                }
            } else {
                let mut x = (vis.min.x / step).floor() * step;
                let mut n = 0;
                while x <= vis.max.x && n < 4000 {
                    let sx = to_screen(Point::new(x, 0.0)).x;
                    painter.line_segment([pos2(sx, rect.top()), pos2(sx, rect.bottom())], stroke);
                    x += step;
                    n += 1;
                }
                let mut y = (vis.min.y / step).floor() * step;
                while y <= vis.max.y && n < 8000 {
                    let sy = to_screen(Point::new(0.0, y)).y;
                    painter.line_segment([pos2(rect.left(), sy), pos2(rect.right(), sy)], stroke);
                    y += step;
                    n += 1;
                }
            }
        }
    }

    // Axis lines through the origin (xfig's "Draw axis lines", pink).
    if app.engine.show_axis_lines() {
        let stroke = Stroke::new(1.0_f32, Color32::from_rgb(255, 192, 203));
        let o = to_screen(Point::new(0.0, 0.0));
        if o.x >= rect.left() && o.x <= rect.right() {
            painter.line_segment([pos2(o.x, rect.top()), pos2(o.x, rect.bottom())], stroke);
        }
        if o.y >= rect.top() && o.y <= rect.bottom() {
            painter.line_segment([pos2(rect.left(), o.y), pos2(rect.right(), o.y)], stroke);
        }
    }

    // Document raster (physical pixels for crisp output).
    let overlay = app.engine.overlay();
    let (w, h) = (
        (rect.width() * ppp).round().max(1.0) as usize,
        (rect.height() * ppp).round().max(1.0) as usize,
    );
    let key = DocKey {
        revision: app.engine.revision(),
        zoom: view.zoom.to_bits(),
        ox: view.origin.x.to_bits(),
        oy: view.origin.y.to_bits(),
        w,
        h,
        hide: overlay.hide.clone(),
    };
    let t = view.transform().then(&Affine::scale(f64::from(ppp), f64::from(ppp)));
    let scene_opts = SceneOptions {
        hidden_depths: app.engine.hidden_depths.clone(),
        gray_hidden: app.engine.gray_hidden_depths,
        screen: true,
        text_scale: None,
    };
    // The scene: rebuilt only when the drawing changes, not on pan or zoom.
    let scene = match &app.canvas.scene {
        Some((k, sc)) if k.same_drawing(&key) => std::sync::Arc::clone(sc),
        _ => {
            let sc = std::sync::Arc::new(build_scene(app.engine.doc(), &scene_opts, &overlay.hide));
            app.canvas.scene = Some((key.clone(), std::sync::Arc::clone(&sc)));
            sc
        }
    };
    // A background render that finished: show it (the newest view wins).
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(job) = &app.canvas.job {
        match job.rx.try_recv() {
            Ok((img, ms)) => {
                let k = job.key.clone();
                app.canvas.job = None;
                app.canvas.render_ms = ms;
                if k.same_drawing(&key) {
                    set_doc_texture(app, ui.ctx(), img, k);
                }
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => app.canvas.job = None,
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
    }
    if app.canvas.doc_key.as_ref() != Some(&key) {
        #[cfg(not(target_arch = "wasm32"))]
        let busy = app.canvas.job.is_some();
        #[cfg(target_arch = "wasm32")]
        let busy = false;
        // Slow drawings render in the background (native); while a render
        // runs, the next starts when it is done.
        let background =
            cfg!(not(target_arch = "wasm32")) && app.canvas.render_ms >= SYNC_RENDER_MS && app.canvas.doc_tex.is_some();
        if background {
            #[cfg(not(target_arch = "wasm32"))]
            if !busy {
                let (tx, rx) = std::sync::mpsc::channel();
                let (sc, ctx, k) = (std::sync::Arc::clone(&scene), ui.ctx().clone(), key.clone());
                let spawned = std::thread::Builder::new()
                    .name("rustfig-render".into())
                    .spawn(move || {
                        let start = std::time::Instant::now();
                        let img = rasterize_scene(&sc, t, w, h);
                        let ms = start.elapsed().as_secs_f64() * 1000.0;
                        // The canvas may be gone (app closing): nothing to do.
                        let _ = tx.send((img, ms));
                        ctx.request_repaint();
                    });
                match spawned {
                    Ok(_) => app.canvas.job = Some(Job { key: k, rx }),
                    Err(_) => {
                        let img = rasterize_scene(&scene, t, w, h);
                        set_doc_texture(app, ui.ctx(), img, key.clone());
                    }
                }
            }
            #[cfg(target_arch = "wasm32")]
            let _ = busy;
        } else if !busy {
            let start = web_time_now();
            let img = rasterize_scene(&scene, t, w, h);
            app.canvas.render_ms = web_time_now() - start;
            set_doc_texture(app, ui.ctx(), img, key.clone());
        }
    }
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    if let (Some(tex), Some(shown)) = (&app.canvas.doc_tex, &app.canvas.doc_key) {
        // The raster where its view puts it in the current one: exact when
        // the views agree, moved and scaled while a new raster is made.
        let old = shown.view(ppp);
        let tl = view.to_screen(old.to_fig(Point::ZERO));
        let br = view.to_screen(old.to_fig(Point::new(old.size.0, old.size.1)));
        let r = Rect::from_min_max(to_pos(tl, origin), to_pos(br, origin));
        painter.image(tex.id(), r, uv, Color32::WHITE);
    }

    // Overlay raster (previews, text being typed).
    let has_overlay = !overlay.preview.is_empty() || overlay.text.is_some();
    if has_overlay {
        let ok = app.canvas.ov_key.as_ref() == Some(&(overlay.clone(), key.clone()));
        if !ok {
            let doc = app.engine.doc();
            let mut d = Document {
                settings: doc.settings.clone(),
                comments: Vec::new(),
                export_on_save: doc.export_on_save.clone(),
                colors: doc.colors.clone(),
                objects: overlay.preview.iter().cloned().map(std::sync::Arc::new).collect(),
                next_id: 0,
                open: Vec::new(),
            };
            if let Some(t) = &overlay.text {
                d.objects.push(std::sync::Arc::new(t.object.clone()));
            }
            let scene = rustfig_render::build_scene(&d, &SceneOptions::default());
            let img = rasterize_scene(&scene, t, w, h);
            match &mut app.canvas.ov_tex {
                Some(tex) => tex.set(img, TextureOptions::NEAREST),
                None => app.canvas.ov_tex = Some(ui.ctx().load_texture("overlay", img, TextureOptions::NEAREST)),
            }
            app.canvas.ov_key = Some((overlay.clone(), key));
        }
        if let Some(tex) = &app.canvas.ov_tex {
            painter.image(tex.id(), rect, uv, Color32::from_white_alpha(230));
        }
    }

    // Guides, selection, handles.
    let guide = Stroke::new(1.0_f32, tk.guide);
    for g in &overlay.guides {
        let pts: Vec<Pos2> = g.iter().map(|p| to_screen(*p)).collect();
        painter.extend(Shape::dashed_line(&pts, guide, 4.0, 3.0));
    }
    // Show vertex numbers (xfig's `shownums`): the point's number beside it.
    if app.engine.show_vertex_numbers() {
        let vis = view.visible();
        for (p, n) in app.engine.vertex_numbers() {
            if p.x >= vis.min.x && p.x <= vis.max.x && p.y >= vis.min.y && p.y <= vis.max.y {
                painter.text(
                    to_screen(p) + egui::vec2(3.0, -3.0),
                    egui::Align2::LEFT_BOTTOM,
                    n.to_string(),
                    egui::FontId::monospace(10.0),
                    Color32::from_rgb(0, 0, 255),
                );
            }
        }
    }
    // Show Lengths: the sides' lengths in red (xfig draws them in RED).
    for (p, s) in &overlay.lengths {
        painter.text(
            to_screen(*p),
            egui::Align2::LEFT_BOTTOM,
            s,
            egui::FontId::monospace(10.0),
            Color32::from_rgb(255, 0, 0),
        );
    }
    for h in &overlay.handles {
        let p = to_screen(*h);
        let r = Rect::from_center_size(p, vec2(6.0, 6.0));
        painter.rect_filled(r, 0.0, tk.handle_fill);
        painter.rect_stroke(r, 0.0, Stroke::new(1.0_f32, tk.handle_stroke), egui::StrokeKind::Middle);
    }
    // xfig's object markers: hollow squares on what the mode can act on
    // (xfig's 5 pixels at 80 dpi; 7 points here, readable on today's
    // screens); a tagged (selected) object's are filled (`toggle_*highlight`).
    for m in &overlay.markers {
        let p = to_screen(*m);
        if !rect.expand(MARKER).contains(p) {
            continue;
        }
        let r = Rect::from_center_size(p, vec2(MARKER, MARKER));
        painter.rect_stroke(r, 0.0, Stroke::new(1.0_f32, tk.handle_stroke), egui::StrokeKind::Middle);
    }
    for m in &overlay.tagged {
        let p = to_screen(*m);
        if rect.expand(MARKER).contains(p) {
            painter.rect_filled(Rect::from_center_size(p, vec2(MARKER, MARKER)), 0.0, tk.handle_stroke);
        }
    }
    // Chop: the axes in one colour, the log object under the pointer (what
    // the middle button cuts) in another.
    let chop = overlay.chop_axes.iter().map(|h| (h, tk.chop_axe));
    for (h, colour) in chop.chain(overlay.chop_log.iter().map(|h| (h, tk.chop_log))) {
        let outline: Vec<Pos2> = h.outline.iter().map(|p| to_screen(*p)).collect();
        painter.extend(Shape::dashed_line(&outline, Stroke::new(1.5_f32, colour), 5.0, 3.0));
        for m in &h.markers {
            let r = Rect::from_center_size(to_screen(*m), vec2(MARKER, MARKER));
            painter.rect_filled(r, 0.0, colour);
        }
    }
    // Join/Split: a magenta square around the first end point (xfig's
    // `draw_join_marker`, 20 pixels).
    if let Some(p) = overlay.join_marker {
        let r = Rect::from_center_size(to_screen(p), vec2(20.0, 20.0));
        painter.rect_stroke(
            r,
            0.0,
            Stroke::new(1.0_f32, Color32::from_rgb(255, 0, 255)),
            egui::StrokeKind::Middle,
        );
    }
    if let Some(ot) = &overlay.text {
        draw_caret(&painter, ot, &to_screen, tk);
    }

    handle_input(app, ui, &resp, rect);
    crate::rulers::show(app, ui, tk, rect, top, side, unit_box);
}

fn draw_caret(
    painter: &egui::Painter,
    ot: &rustfig_engine::tools::OverlayText,
    to_screen: &dyn Fn(Point) -> Pos2,
    tk: &Tokens,
) {
    let ObjectKind::Text(t) = &ot.object.kind else { return };
    let req = rustfig_text::FontRequest::for_fig(t.font, t.is_postscript());
    let em = t.size * rustfig_geom::FIG_PER_POINT;
    let rot = Affine::rotate(t.angle);
    let lines: Vec<&str> = t.text.split('\n').collect();
    // Top and bottom of the caret at character `idx` (screen points).
    let at = |idx: usize| -> (Pos2, Pos2) {
        let mut rest = idx;
        let mut li = 0;
        for (i, l) in lines.iter().enumerate() {
            li = i;
            let n = l.chars().count();
            if rest <= n || i + 1 == lines.len() {
                rest = rest.min(n);
                break;
            }
            rest -= n + 1;
        }
        let line = lines.get(li).copied().unwrap_or("");
        let prefix: String = line.chars().take(rest).collect();
        let pw = rustfig_text::measure(&req, t.size, &prefix).width;
        let lw = rustfig_text::measure(&req, t.size, line).width;
        let x0 = match t.justify {
            rustfig_model::Justify::Left => 0.0,
            rustfig_model::Justify::Center => -lw / 2.0,
            rustfig_model::Justify::Right => -lw,
        };
        let y = li as f64 * em * rustfig_text::LINE_SPACING;
        let a = t.origin + rot.apply_vec(Point::new(x0 + pw, y + em * 0.2));
        let b = t.origin + rot.apply_vec(Point::new(x0 + pw, y - em * 0.8));
        (to_screen(a), to_screen(b))
    };
    // E10: the selection, one quad per line it touches.
    if let Some((s0, s1)) = ot.selection {
        let mut start = 0;
        for l in &lines {
            let n = l.chars().count();
            let (a, b) = (s0.max(start), s1.min(start + n));
            if a < b {
                let (a0, a1) = at(a);
                let (b0, b1) = at(b);
                painter.add(egui::Shape::convex_polygon(
                    vec![a0, b0, b1, a1],
                    tk.caret.gamma_multiply(0.3),
                    Stroke::NONE,
                ));
            }
            start += n + 1;
        }
    }
    // The IME composition, underlined.
    let (ps, pl) = ot.preedit;
    if pl > 0 {
        let (a0, _) = at(ps);
        let (b0, _) = at(ps + pl);
        painter.line_segment([a0, b0], Stroke::new(1.5_f32, tk.caret));
    }
    let (a, b) = at(ot.cursor);
    painter.line_segment([a, b], Stroke::new(1.5_f32, tk.caret));
    // Tell the system where the caret is, for the IME's candidate window.
    let r = Rect::from_two_pos(a, b);
    painter.ctx().output_mut(|o| {
        o.ime = Some(egui::output::IMEOutput {
            rect: r.expand(40.0),
            cursor_rect: r,
        });
    });
}

fn web_time_now() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::OnceLock;
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0
    }
    #[cfg(target_arch = "wasm32")]
    {
        0.0
    }
}

/// The tool button for a physical button: Alt+left is the middle button and
/// Ctrl+left the right one, for two-button mice and trackpads.
pub fn tool_button(b: egui::PointerButton, m: egui::Modifiers) -> Option<Button> {
    match b {
        egui::PointerButton::Primary if m.alt => Some(Button::Middle),
        egui::PointerButton::Primary if m.ctrl => Some(Button::Right),
        egui::PointerButton::Primary => Some(Button::Left),
        egui::PointerButton::Secondary => Some(Button::Right),
        egui::PointerButton::Middle => Some(Button::Middle),
        _ => None,
    }
}

/// Whether the modifiers hold xfig's Ctrl key: Ctrl itself on Windows and
/// Linux, Cmd on macOS (Ctrl+click is the right button there).
pub fn ctrl_modifier(os: crate::platform::Os, m: &egui::Modifiers) -> bool {
    match os {
        crate::platform::Os::Mac => m.mac_cmd,
        _ => m.ctrl,
    }
}

/// The button of a press with xfig's Ctrl key held: Alt+left is the middle
/// button as ever, Ctrl itself no longer turns left into right.
fn physical_button(b: egui::PointerButton, m: &egui::Modifiers) -> Option<Button> {
    match b {
        egui::PointerButton::Primary if m.alt => Some(Button::Middle),
        egui::PointerButton::Primary => Some(Button::Left),
        egui::PointerButton::Secondary => Some(Button::Right),
        egui::PointerButton::Middle => Some(Button::Middle),
        _ => None,
    }
}

fn handle_input(app: &mut App, ui: &egui::Ui, resp: &egui::Response, rect: Rect) {
    const BUTTONS: [egui::PointerButton; 3] = [
        egui::PointerButton::Primary,
        egui::PointerButton::Middle,
        egui::PointerButton::Secondary,
    ];
    let (hover, pressed, released, any_down, double, triple, scroll, zoom, space) = ui.input(|i| {
        (
            i.pointer.hover_pos(),
            // Presses with the modifiers held at that moment.
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::PointerButton {
                        button,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some((*button, *modifiers)),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            BUTTONS.map(|b| i.pointer.button_released(b)),
            i.pointer.any_down(),
            i.pointer.button_double_clicked(egui::PointerButton::Primary),
            i.pointer.button_triple_clicked(egui::PointerButton::Primary),
            i.smooth_scroll_delta,
            i.zoom_delta(),
            i.key_down(egui::Key::Space),
        )
    });
    let over = hover.is_some_and(|p| rect.contains(p)) && resp.hovered();
    let os = crate::platform::Os::current(ui.ctx());
    let view0 = app.engine.view;
    let to_fig = |p: Pos2| view0.to_fig(Point::new(f64::from(p.x - rect.min.x), f64::from(p.y - rect.min.y)));
    app.canvas.cursor = if over { hover.map(to_fig) } else { None };

    // Zoom (pinch or Ctrl+scroll) and pan (scroll, space drag, and the
    // middle button when it isn't a tool button).
    if over {
        if (zoom - 1.0).abs() > 1e-4 {
            if let Some(p) = hover {
                let anchor = Point::new(f64::from(p.x - rect.min.x), f64::from(p.y - rect.min.y));
                let z = app.engine.view.zoom * f64::from(zoom);
                app.engine.view.zoom_about(z, anchor);
            }
        } else if let (true, Some(p)) = (ui.input(|i| i.modifiers.alt) && scroll.y != 0.0, hover) {
            // xfig's Alt (Meta) + wheel pops up the drawing (up) or
            // editing (down) modes at the pointer (`Fig*canvas.translations`).
            app.mode_popup = Some((scroll.y > 0.0, p));
        } else if scroll != egui::Vec2::ZERO {
            app.engine.view.pan_pixels(f64::from(scroll.x), f64::from(scroll.y));
        }
    }
    if space && (resp.dragged() || app.canvas.panning) && (over || app.canvas.panning) {
        app.canvas.panning = true;
        let d = ui.input(|i| i.pointer.delta());
        app.engine.view.pan_pixels(f64::from(d.x), f64::from(d.y));
        if !any_down {
            app.canvas.panning = false;
        }
        return;
    }
    app.canvas.panning = false;

    let send = |app: &mut App, ev: ToolEvent| {
        let moving = matches!(ev, ToolEvent::Move { .. });
        if let Err(e) = app.engine.tool_event(ev) {
            app.error(e.message);
        }
        app.show_tool_status(moving);
    };
    if let Some(p) = hover {
        let fp = to_fig(p);
        for (phys, m) in pressed.iter().filter(|_| over) {
            // xfig's Ctrl key (zoom box, shape factor); Cmd on macOS, where
            // Ctrl+click is the right button.
            let ctrl = ctrl_modifier(os, m);
            let Some(button) = (if ctrl {
                physical_button(*phys, m)
            } else {
                tool_button(*phys, *m)
            }) else {
                continue;
            };
            if button != Button::Right {
                app.canvas.pressing = Some((*phys, button));
            }
            let ev = if ctrl {
                ToolEvent::Ctrl {
                    pos: fp,
                    button,
                    shift: m.shift,
                }
            } else {
                ToolEvent::Press {
                    pos: fp,
                    button,
                    shift: m.shift,
                }
            };
            send(app, ev);
        }
        if (over || app.canvas.pressing.is_some()) && app.canvas.last_sent != Some(fp) {
            app.canvas.last_sent = Some(fp);
            send(app, ToolEvent::Move { pos: fp });
        }
        if let Some((phys, button)) = app.canvas.pressing {
            let i = BUTTONS.iter().position(|b| *b == phys).unwrap_or(0);
            if released.get(i).copied().unwrap_or(false) {
                app.canvas.pressing = None;
                send(app, ToolEvent::Release { pos: fp, button });
            }
        }
        if over && triple {
            send(app, ToolEvent::TripleClick { pos: fp });
        } else if over && double {
            send(app, ToolEvent::DoubleClick { pos: fp });
        }
    }
    if app.engine.tool_busy() {
        ui.ctx().request_repaint();
    }
    if over {
        ui.ctx().set_cursor_icon(cursor_icon(app.engine.cursor_kind()));
    }
}

/// egui's cursor for xfig's per-mode cursors (`w_cursor.c`).
pub(crate) fn cursor_icon(kind: rustfig_engine::CursorKind) -> egui::CursorIcon {
    use rustfig_engine::CursorKind as K;
    match kind {
        K::Arrow => egui::CursorIcon::Default,
        K::Crosshair => egui::CursorIcon::Crosshair,
        K::Pick => egui::CursorIcon::PointingHand,
        K::PickObject => egui::CursorIcon::Cell,
        K::Delete => egui::CursorIcon::NotAllowed,
        K::Text => egui::CursorIcon::Text,
        K::Move => egui::CursorIcon::Move,
        K::Grabbing => egui::CursorIcon::Grabbing,
    }
}

#[cfg(test)]
mod tests {
    use super::grid_spacing;
    use rustfig_engine::SnapMode;
    use rustfig_model::Units;

    #[test]
    fn a_grid_too_fine_for_the_zoom_shows_a_coarser_one() {
        let base = 80.0 / 1200.0;
        // 1/16 in at 100% is 5 points: shown as it is.
        assert_eq!(grid_spacing(SnapMode::Fine, Units::Inches, base).0, 75.0);
        // At 50% it would be 2.5 points: 1/8 in (5 points) instead.
        assert_eq!(grid_spacing(SnapMode::Fine, Units::Inches, base * 0.5).0, 150.0);
        // Far out, even 1 in is too fine: the caller hides it.
        assert!(grid_spacing(SnapMode::Fine, Units::Inches, base * 0.01).1 < 5.0);
    }
}
