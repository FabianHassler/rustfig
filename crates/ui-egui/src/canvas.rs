//! The drawing canvas: document raster, overlay, grid, page border and
//! pointer input.

use crate::theme::Tokens;
use crate::App;
use egui::{pos2, vec2, Color32, Pos2, Rect, Sense, Shape, Stroke, TextureHandle, TextureOptions};
use rustfig_engine::{Button, MouseScheme, Overlay, ToolEvent};
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

#[derive(Default)]
pub struct CanvasState {
    doc_tex: Option<TextureHandle>,
    doc_key: Option<DocKey>,
    ov_tex: Option<TextureHandle>,
    ov_key: Option<(Overlay, DocKey)>,
    /// Pointer position in Fig units, if over the canvas.
    pub cursor: Option<Point>,
    last_sent: Option<Point>,
    /// The tool button of a press that started on the canvas.
    pressing: Option<(egui::PointerButton, Button)>,
    /// Panning with space+drag (or the middle button in the modern scheme).
    panning: bool,
    /// Milliseconds the last document render took.
    pub render_ms: f64,
}

impl CanvasState {
    /// Drop cached rasters so the next frame redraws everything.
    pub fn invalidate(&mut self) {
        self.doc_key = None;
        self.ov_key = None;
    }
}

fn to_pos(p: Point, origin: Pos2) -> Pos2 {
    pos2(origin.x + p.x as f32, origin.y + p.y as f32)
}

/// Render `doc` (minus `hide`) into a texture of `w`×`h` physical pixels.
fn render_doc(
    doc: &Document,
    opts: &SceneOptions,
    hide: &[ObjectId],
    t: Affine,
    w: usize,
    h: usize,
) -> egui::ColorImage {
    let scene = if hide.is_empty() {
        Scene::build(doc, opts)
    } else {
        let mut d = Document {
            settings: doc.settings.clone(),
            comments: Vec::new(),
            colors: doc.colors.clone(),
            objects: Vec::with_capacity(doc.objects.len()),
            next_id: 0,
        };
        d.objects = doc.objects.iter().filter(|o| !hide.contains(&o.id)).cloned().collect();
        Scene::build(&d, opts)
    };
    rasterize_scene(&scene, t, w, h)
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
        },
    );
    egui::ColorImage::from_rgba_premultiplied([w, h], pm.data())
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
        let step = g.step(doc.settings.units);
        let px = step * view.px_per_fig();
        if step > 0.0 && px >= 6.0 {
            let vis = view.visible();
            let stroke = Stroke::new(1.0_f32, tk.grid);
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
    };
    if app.canvas.doc_key.as_ref() != Some(&key) {
        let start = web_time_now();
        let img = render_doc(app.engine.doc(), &scene_opts, &overlay.hide, t, w, h);
        app.canvas.render_ms = web_time_now() - start;
        match &mut app.canvas.doc_tex {
            Some(tex) => tex.set(img, TextureOptions::NEAREST),
            None => app.canvas.doc_tex = Some(ui.ctx().load_texture("doc", img, TextureOptions::NEAREST)),
        }
        app.canvas.doc_key = Some(key.clone());
    }
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    if let Some(tex) = &app.canvas.doc_tex {
        painter.image(tex.id(), rect, uv, Color32::WHITE);
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
                colors: doc.colors.clone(),
                objects: overlay.preview.clone(),
                next_id: 0,
            };
            if let Some(t) = &overlay.text {
                d.objects.push(t.object.clone());
            }
            let scene = Scene::build(&d, &SceneOptions::default());
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
    if app.engine.tool() == rustfig_engine::ToolKind::Select && !app.engine.tool_busy() {
        let bb = app.engine.selection_bbox();
        if !bb.is_empty() {
            let r = Rect::from_two_pos(to_screen(bb.min), to_screen(bb.max)).expand(3.0);
            let c = [
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
                r.left_top(),
            ];
            painter.extend(Shape::dashed_line(&c, Stroke::new(1.0_f32, tk.selection), 5.0, 3.0));
        }
    }
    for h in &overlay.handles {
        let p = to_screen(*h);
        let r = Rect::from_center_size(p, vec2(6.0, 6.0));
        painter.rect_filled(r, 0.0, tk.handle_fill);
        painter.rect_stroke(r, 0.0, Stroke::new(1.0_f32, tk.handle_stroke), egui::StrokeKind::Middle);
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
    let before: String = t.text.chars().take(ot.cursor).collect();
    let line_idx = before.matches('\n').count();
    let prefix = before.rsplit('\n').next().unwrap_or("");
    let line = t.text.split('\n').nth(line_idx).unwrap_or("");
    let pw = rustfig_text::measure(&req, t.size, prefix).width;
    let lw = rustfig_text::measure(&req, t.size, line).width;
    let x0 = match t.justify {
        rustfig_model::Justify::Left => 0.0,
        rustfig_model::Justify::Center => -lw / 2.0,
        rustfig_model::Justify::Right => -lw,
    };
    let em = t.size * rustfig_geom::FIG_PER_POINT;
    let y = line_idx as f64 * em * rustfig_text::LINE_SPACING;
    let rot = Affine::rotate(t.angle);
    let a = t.origin + rot.apply_vec(Point::new(x0 + pw, y + em * 0.2));
    let b = t.origin + rot.apply_vec(Point::new(x0 + pw, y - em * 0.8));
    painter.line_segment([to_screen(a), to_screen(b)], Stroke::new(1.5_f32, tk.caret));
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

fn handle_input(app: &mut App, ui: &egui::Ui, resp: &egui::Response, rect: Rect) {
    const BUTTONS: [egui::PointerButton; 3] = [
        egui::PointerButton::Primary,
        egui::PointerButton::Middle,
        egui::PointerButton::Secondary,
    ];
    let (hover, pressed, released, mid_down, any_down, double, scroll, zoom, space) = ui.input(|i| {
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
            i.pointer.middle_down(),
            i.pointer.any_down(),
            i.pointer.button_double_clicked(egui::PointerButton::Primary),
            i.smooth_scroll_delta,
            i.zoom_delta(),
            i.key_down(egui::Key::Space),
        )
    });
    let over = hover.is_some_and(|p| rect.contains(p)) && resp.hovered();
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
        } else if scroll != egui::Vec2::ZERO {
            app.engine.view.pan_pixels(f64::from(scroll.x), f64::from(scroll.y));
        }
    }
    let mid_pans = mid_down && app.engine.mouse_scheme() == MouseScheme::Modern;
    if (mid_pans || space) && (resp.dragged() || app.canvas.panning) && (over || app.canvas.panning) {
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
        if let Err(e) = app.engine.tool_event(ev) {
            app.error(e.message);
        }
    };
    if let Some(p) = hover {
        let fp = to_fig(p);
        for (phys, m) in pressed.iter().filter(|_| over) {
            let Some(button) = tool_button(*phys, *m) else { continue };
            if button != Button::Right {
                app.canvas.pressing = Some((*phys, button));
            }
            send(
                app,
                ToolEvent::Press {
                    pos: fp,
                    button,
                    shift: m.shift,
                },
            );
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
        if over && double {
            send(app, ToolEvent::DoubleClick { pos: fp });
        }
    }
    if app.engine.tool_busy() {
        ui.ctx().request_repaint();
    }
    if over {
        ui.ctx()
            .set_cursor_icon(if app.engine.tool() == rustfig_engine::ToolKind::Select {
                egui::CursorIcon::Default
            } else {
                egui::CursorIcon::Crosshair
            });
    }
}
