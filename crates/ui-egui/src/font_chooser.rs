//! E11: the font chooser. xfig's font menus (`w_fontpanel.c`: 35 PostScript
//! fonts, six LaTeX fonts, a PostScript/LaTeX switch) with RustFig's
//! additions: every name drawn in its own font (the glyph source of the
//! canvas, `rustfig_text::shape`), a search field, a size field and a live
//! preview of sample text, the current font marked.
//!
//! With the built-in fonts only (no installed fonts used: wasm, or
//! `FIG2DEV_BUILTIN_FONTS=1`), the names are drawn in the fallback face and
//! tagged "(substitute)".

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use egui::{vec2, Color32, Rect, RichText, Sense};
use rustfig_text::{FontRequest, PathOp};

/// What the chooser changed this frame.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Outcome {
    pub font: Option<(i32, bool)>,
    pub size: Option<f64>,
}

pub const SAMPLE: &str = "The quick brown fox 0123";
const MAX_TEX_W: usize = 1600;
const MAX_TEX_H: usize = 256;

/// The name a font is listed under.
pub fn list_name(font: i32, postscript: bool) -> String {
    if postscript {
        rustfig_text::ps_font_display_name(font).to_string()
    } else {
        format!("LaTeX {}", rustfig_text::latex_font_name(font))
    }
}

/// Whether installed faces are used (not only the built-in fallback fonts).
pub fn has_real_face() -> bool {
    !rustfig_text::builtin_fonts()
}

/// Whether the font matches the search text (case-insensitive, every word).
pub fn matches(name: &str, query: &str) -> bool {
    let name = name.to_lowercase();
    query.split_whitespace().all(|w| name.contains(&w.to_lowercase()))
}

#[derive(Clone)]
struct Tex {
    tex: egui::TextureHandle,
    size: egui::Vec2,
}

type Cache = Arc<Mutex<HashMap<(i32, bool, String, u32), Option<Tex>>>>;

/// A string drawn in a Fig font as a texture (coverage in the alpha channel).
fn rendered(ctx: &egui::Context, font: i32, ps: bool, text: &str, px: f32) -> Option<Tex> {
    let cache: Cache = ctx.data_mut(|d| {
        d.get_temp_mut_or_insert_with(egui::Id::new("font_chooser_cache"), Cache::default)
            .clone()
    });
    let ppp = ctx.pixels_per_point().max(0.5);
    let key = (font, ps, text.to_string(), (px * ppp * 4.0) as u32);
    let mut map = cache.lock().ok()?;
    if map.len() > 600 {
        map.clear();
    }
    if let Some(t) = map.get(&key) {
        return t.clone();
    }
    let img = rasterise(font, ps, text, px * ppp);
    let tex = img.map(|(image, w, h)| Tex {
        tex: ctx.load_texture("font_sample", image, egui::TextureOptions::LINEAR),
        size: vec2(w as f32 / ppp, h as f32 / ppp),
    });
    map.insert(key, tex.clone());
    tex
}

/// Flatten the outlines, fill them (non-zero rule, 4 sub-scanlines, exact
/// horizontal coverage) and return the alpha image with its size in pixels.
fn rasterise(font: i32, ps: bool, text: &str, px: f32) -> Option<(egui::ColorImage, usize, usize)> {
    if !px.is_finite() || px < 1.0 {
        return None;
    }
    let size_pt = f64::from(px.min(400.0));
    let shaped = rustfig_text::shape(&FontRequest::for_fig(font, ps), size_pt, text);
    let k = 1.0 / rustfig_text::FIG_PER_POINT;
    let (asc, desc) = (shaped.metrics.ascent * k, shaped.metrics.descent * k);
    let width = shaped.metrics.width * k;
    if !(asc.is_finite() && desc.is_finite() && width.is_finite()) {
        return None;
    }
    let w = ((width + 3.0).ceil() as usize).clamp(1, MAX_TEX_W);
    let h = ((asc + desc + 3.0).ceil() as usize).clamp(1, MAX_TEX_H);
    let base = asc + 1.0;
    // Edges (x0, y0, x1, y1) in pixels.
    let mut edges: Vec<(f64, f64, f64, f64)> = Vec::new();
    let mut cur = (0.0, 0.0);
    let mut start = (0.0, 0.0);
    let line = |from: (f64, f64), to: (f64, f64), e: &mut Vec<(f64, f64, f64, f64)>| {
        e.push((from.0, from.1, to.0, to.1));
    };
    for ops in &shaped.lines {
        for op in ops {
            let p = |x: f64, y: f64| (x * k + 1.0, y * k + base);
            match *op {
                PathOp::Move(x, y) => {
                    if cur != start {
                        line(cur, start, &mut edges);
                    }
                    cur = p(x, y);
                    start = cur;
                }
                PathOp::Line(x, y) => {
                    let n = p(x, y);
                    line(cur, n, &mut edges);
                    cur = n;
                }
                PathOp::Quad(cx, cy, x, y) => {
                    let (c, n) = (p(cx, cy), p(x, y));
                    let from = cur;
                    for i in 1..=8 {
                        let t = f64::from(i) / 8.0;
                        let u = 1.0 - t;
                        let pt = (
                            u * u * from.0 + 2.0 * u * t * c.0 + t * t * n.0,
                            u * u * from.1 + 2.0 * u * t * c.1 + t * t * n.1,
                        );
                        line(cur, pt, &mut edges);
                        cur = pt;
                    }
                }
                PathOp::Cubic(c1x, c1y, c2x, c2y, x, y) => {
                    let (c1, c2, n) = (p(c1x, c1y), p(c2x, c2y), p(x, y));
                    let from = cur;
                    for i in 1..=10 {
                        let t = f64::from(i) / 10.0;
                        let u = 1.0 - t;
                        let b = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                        let pt = (
                            b.0 * from.0 + b.1 * c1.0 + b.2 * c2.0 + b.3 * n.0,
                            b.0 * from.1 + b.1 * c1.1 + b.2 * c2.1 + b.3 * n.1,
                        );
                        line(cur, pt, &mut edges);
                        cur = pt;
                    }
                }
                PathOp::Close => {
                    if cur != start {
                        line(cur, start, &mut edges);
                    }
                    cur = start;
                }
            }
        }
        if cur != start {
            line(cur, start, &mut edges);
        }
        cur = start;
    }
    edges.retain(|e| e.0.is_finite() && e.1.is_finite() && e.2.is_finite() && e.3.is_finite());
    const SS: usize = 4;
    let mut cover = vec![0f32; w * h];
    let mut xs: Vec<(f64, i32)> = Vec::new();
    for sy in 0..h * SS {
        let y = (sy as f64 + 0.5) / SS as f64;
        xs.clear();
        for &(x0, y0, x1, y1) in &edges {
            if (y0 <= y && y < y1) || (y1 <= y && y < y0) {
                let t = (y - y0) / (y1 - y0);
                xs.push((x0 + t * (x1 - x0), if y1 > y0 { 1 } else { -1 }));
            }
        }
        xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let row = (sy / SS) * w;
        let mut wind = 0;
        for pair in xs.windows(2) {
            wind += pair[0].1;
            if wind == 0 {
                continue;
            }
            let (a, b) = (pair[0].0.max(0.0), pair[1].0.min(w as f64));
            if b <= a {
                continue;
            }
            let (ia, ib) = (a.floor() as usize, (b.ceil() as usize).min(w));
            for ix in ia..ib {
                let cov = (b.min(ix as f64 + 1.0) - a.max(ix as f64)).max(0.0);
                if let Some(c) = cover.get_mut(row + ix) {
                    *c += cov as f32 / SS as f32;
                }
            }
        }
    }
    let mut rgba = Vec::with_capacity(w * h * 4);
    for c in cover {
        let a = (c.clamp(0.0, 1.0) * 255.0) as u8;
        rgba.extend_from_slice(&[255, 255, 255, a]);
    }
    Some((egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba), w, h))
}

/// `text` drawn in the font at `px` (points) as an image to tint, or None
/// when it can't be rendered.
pub(crate) fn sample_image(
    ctx: &egui::Context,
    font: i32,
    ps: bool,
    text: &str,
    px: f32,
) -> Option<egui::Image<'static>> {
    let t = rendered(ctx, font, ps, text, px)?;
    Some(egui::Image::new(&t.tex).fit_to_exact_size(t.size))
}

/// Draw `text` in the font at `px` (points), tinted `ink`. Falls back to the
/// UI font when it can't be rendered.
fn paint_text(ui: &mut egui::Ui, font: i32, ps: bool, text: &str, px: f32, ink: Color32) {
    match rendered(ui.ctx(), font, ps, text, px) {
        Some(t) => {
            ui.add(egui::Image::new(&t.tex).fit_to_exact_size(t.size).tint(ink));
        }
        None => {
            ui.label(RichText::new(text).size(px));
        }
    }
}

/// The chooser. `size`: the current font size when the size field is offered.
/// State (switch, search) lives in egui's memory under `id`.
pub fn show(ui: &mut egui::Ui, id: &str, font: i32, postscript: bool, size: Option<f64>) -> Outcome {
    let mut out = Outcome::default();
    ui.set_width(280.0);
    let sid = egui::Id::new(("font_chooser", id));
    #[derive(Clone, Default)]
    struct St {
        ps: Option<bool>,
        query: String,
        hover: Option<(i32, bool)>,
    }
    let mut st: St = ui.data_mut(|d| d.get_temp(sid)).unwrap_or_default();
    let mut ps_list = st.ps.unwrap_or(postscript);
    let ink = ui.visuals().text_color();
    ui.horizontal(|ui| {
        for (v, label) in [(true, "PostScript"), (false, "LaTeX")] {
            let r = ui.radio_value(&mut ps_list, v, label);
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::RadioButton, true, format!("{label} fonts")));
        }
    });
    st.ps = Some(ps_list);
    ui.horizontal(|ui| {
        let r = ui.add(
            egui::TextEdit::singleline(&mut st.query)
                .hint_text("Search fonts")
                .desired_width(150.0),
        );
        r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Search fonts"));
        if let Some(s) = size {
            ui.label("Size");
            let mut v = s;
            let r = ui.add(egui::DragValue::new(&mut v).range(1.0..=500.0).speed(0.5));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, "Font size"));
            if r.changed() && (v - s).abs() > f64::EPSILON {
                out.size = Some(v);
            }
        }
    });
    let sample_px = (size.unwrap_or(14.0) as f32).clamp(6.0, 36.0);
    let shown = st.hover.unwrap_or((font, postscript));
    ui.separator();
    ui.label(RichText::new(format!("Preview: {}", list_name(shown.0, shown.1))).small());
    // A fixed height: a preview that grew or shrank with the hovered font
    // would move the list under the pointer and make the hover flicker.
    let preview_h = sample_px * 2.0 + 8.0;
    egui::ScrollArea::horizontal()
        .id_salt(("font_preview", id))
        .max_height(preview_h)
        .min_scrolled_height(preview_h)
        .auto_shrink([false, false])
        .show(ui, |ui| paint_text(ui, shown.0, shown.1, SAMPLE, sample_px, ink));
    ui.separator();
    let count = if ps_list {
        rustfig_text::PS_FONT_COUNT
    } else {
        rustfig_text::LATEX_FONT_COUNT
    };
    let real = has_real_face();
    let mut hover = None;
    let mut any = false;
    let list = egui::ScrollArea::vertical()
        .id_salt(("font_list", id))
        .max_height(300.0)
        .min_scrolled_height(300.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Rows touch, so the pointer is never between two of them.
            ui.spacing_mut().item_spacing.y = 0.0;
            for f in 0..count {
                let name = list_name(f, ps_list);
                if !matches(&name, &st.query) {
                    continue;
                }
                any = true;
                let current = f == font && ps_list == postscript;
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width().max(230.0), 24.0), Sense::click());
                resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, current, name.clone()));
                if ui.is_rect_visible(rect) {
                    let p = ui.painter();
                    if current {
                        p.rect_filled(rect, 2.0, ui.visuals().selection.bg_fill);
                    } else if resp.hovered() {
                        p.rect_filled(rect, 2.0, ui.visuals().widgets.hovered.weak_bg_fill);
                    }
                    let color = if current {
                        ui.visuals().selection.stroke.color
                    } else {
                        ink
                    };
                    if current {
                        p.text(
                            rect.left_center() + vec2(4.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            "•",
                            egui::FontId::proportional(14.0),
                            color,
                        );
                    }
                    let inner = Rect::from_min_max(rect.min + vec2(18.0, 1.0), rect.max - vec2(2.0, 1.0));
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
                    child.horizontal_centered(|ui| {
                        paint_text(ui, f, ps_list, &name, 15.0, color);
                        if !real {
                            ui.label(RichText::new("(substitute)").small().weak());
                        }
                    });
                }
                if resp.hovered() {
                    hover = Some((f, ps_list));
                }
                if resp.clicked() {
                    out.font = Some((f, ps_list));
                }
            }
            if !any {
                ui.label(RichText::new("No font matches.").weak());
            }
        });
    // Inside the list the preview keeps the last hovered font (the pointer
    // may be on the scroll bar); it returns to the current one on leaving.
    let in_list = ui.rect_contains_pointer(list.inner_rect);
    st.hover = hover.or(if in_list { st.hover } else { None });
    if out.font.is_some() {
        // The next opening starts with the full list of the font's own kind.
        st = St::default();
    }
    ui.data_mut(|d| d.insert_temp(sid, st));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_matches_words() {
        assert!(matches("Times Bold Italic", "bold ital"));
        assert!(!matches("Courier", "bold"));
        assert!(matches("Anything", ""));
    }

    #[test]
    fn names_are_drawn() {
        for ps in [true, false] {
            let n = if ps {
                rustfig_text::PS_FONT_COUNT
            } else {
                rustfig_text::LATEX_FONT_COUNT
            };
            for f in 0..n {
                let (img, w, h) = rasterise(f, ps, &list_name(f, ps), 15.0).unwrap();
                assert!(w > 4 && h > 4);
                assert!(img.pixels.iter().any(|p| p.a() > 100), "font {f} {ps}");
            }
        }
        assert!(rasterise(0, true, "x", f32::NAN).is_none());
        let _ = rasterise(9999, true, "\u{0}", 12.0);
    }
}
