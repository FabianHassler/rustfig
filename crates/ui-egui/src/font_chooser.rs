//! E11: the font chooser. xfig's font menus (`w_fontpanel.c`: 35 PostScript
//! fonts, six LaTeX fonts, a PostScript/LaTeX switch) with RustFig's
//! additions: every name drawn in its own font (the glyph source of the
//! canvas, `rustfig_text::shape`), a search field, a size field and a live
//! preview of sample text, the current font marked.
//!
//! F7 adds a third choice, "Local fonts": every installed face
//! (`rustfig_text::local_faces`), each name in its own font, loaded lazily
//! for the visible rows only.
//!
//! With the built-in fonts only (no installed fonts used: wasm, or
//! `FIG2DEV_BUILTIN_FONTS=1`), the names are drawn in the fallback face and
//! tagged "(substitute)".

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use egui::{vec2, Color32, Rect, RichText, Sense};
use rustfig_text::{FontRequest, PathOp};

/// What the chooser changed this frame.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Outcome {
    /// The Fig font chosen; for an installed font its nearest Fig font.
    pub font: Option<(i32, bool)>,
    /// Set with `font`: the installed font chosen (`Some(Some(name))`), or
    /// `Some(None)` when a Fig font was chosen.
    pub local: Option<Option<String>>,
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

type Cache = Arc<Mutex<HashMap<(i32, bool, String, String, u32), Option<Tex>>>>;

/// A string drawn in a Fig font as a texture (coverage in the alpha channel).
fn rendered(ctx: &egui::Context, font: i32, ps: bool, local: Option<&str>, text: &str, px: f32) -> Option<Tex> {
    let cache: Cache = ctx.data_mut(|d| {
        d.get_temp_mut_or_insert_with(egui::Id::new("font_chooser_cache"), Cache::default)
            .clone()
    });
    let ppp = ctx.pixels_per_point().max(0.5);
    let key = (
        font,
        ps,
        local.unwrap_or_default().to_string(),
        text.to_string(),
        (px * ppp * 4.0) as u32,
    );
    let mut map = cache.lock().ok()?;
    if map.len() > 600 {
        map.clear();
    }
    if let Some(t) = map.get(&key) {
        return t.clone();
    }
    let img = rasterise(font, ps, local, text, px * ppp);
    let tex = img.map(|(image, w, h)| Tex {
        tex: ctx.load_texture("font_sample", image, egui::TextureOptions::LINEAR),
        size: vec2(w as f32 / ppp, h as f32 / ppp),
    });
    map.insert(key, tex.clone());
    tex
}

/// Flatten the outlines, fill them (non-zero rule, 4 sub-scanlines, exact
/// horizontal coverage) and return the alpha image with its size in pixels.
fn rasterise(
    font: i32,
    ps: bool,
    local: Option<&str>,
    text: &str,
    px: f32,
) -> Option<(egui::ColorImage, usize, usize)> {
    if !px.is_finite() || px < 1.0 {
        return None;
    }
    let size_pt = f64::from(px.min(400.0));
    // An installed font that is not there is drawn as its Fig font.
    let shaped = match local.and_then(|n| rustfig_text::shape_local(n, size_pt, text)) {
        Some(s) => s,
        None => rustfig_text::shape(&FontRequest::for_fig(font, ps), size_pt, text),
    };
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
    let t = rendered(ctx, font, ps, None, text, px)?;
    Some(egui::Image::new(&t.tex).fit_to_exact_size(t.size))
}

/// How an installed font shows its own name: `None` when it has the
/// letters of the name, otherwise a sample in a script it has (Arabic for
/// Waseem), or `Some(None)` for none. Cached: the list asks every frame.
fn uncovered(ctx: &egui::Context, name: &str) -> Option<Option<&'static str>> {
    type Covers = Arc<Mutex<HashMap<String, Option<Option<&'static str>>>>>;
    let cache: Covers = ctx.data_mut(|d| {
        d.get_temp_mut_or_insert_with(egui::Id::new("font_chooser_covers"), Covers::default)
            .clone()
    });
    let Ok(mut map) = cache.lock() else {
        return None;
    };
    *map.entry(name.to_string()).or_insert_with(|| {
        // A font that is not installed is drawn as its Fig font.
        if !rustfig_text::local_font_available(name) || rustfig_text::local_font_covers(name, name) {
            None
        } else {
            Some(rustfig_text::local_font_sample(name))
        }
    })
}

/// Draw `text` in the font at `px` (points), tinted `ink`. Falls back to the
/// UI font when it can't be rendered.
fn paint_text(ui: &mut egui::Ui, font: i32, ps: bool, local: Option<&str>, text: &str, px: f32, ink: Color32) {
    match rendered(ui.ctx(), font, ps, local, text, px) {
        Some(t) => {
            ui.add(egui::Image::new(&t.tex).fit_to_exact_size(t.size).tint(ink));
        }
        None => {
            ui.label(RichText::new(text).size(px));
        }
    }
}

/// The chooser. `size`: the current font size when the size field is offered;
/// `local`: the current installed font (F7), if the text has one.
/// State (list, search) lives in egui's memory under `id`.
pub fn show(
    ui: &mut egui::Ui,
    id: &str,
    font: i32,
    postscript: bool,
    local: Option<&str>,
    size: Option<f64>,
) -> Outcome {
    let mut out = Outcome::default();
    ui.set_width(280.0);
    let sid = egui::Id::new(("font_chooser", id));
    /// What the list shows: the 35 PostScript fonts, the LaTeX fonts or the
    /// installed ones.
    #[derive(Clone, Copy, PartialEq)]
    enum Kind {
        PostScript,
        Latex,
        Local,
    }
    /// A font the preview shows.
    #[derive(Clone, PartialEq)]
    enum Shown {
        Fig(i32, bool),
        Local(String),
    }
    #[derive(Clone, Default)]
    struct St {
        kind: Option<u8>,
        query: String,
        hover: Option<Shown>,
    }
    let mut st: St = ui.data_mut(|d| d.get_temp(sid)).unwrap_or_default();
    let mut kind = match st.kind {
        Some(0) => Kind::PostScript,
        Some(1) => Kind::Latex,
        Some(_) => Kind::Local,
        None if local.is_some() => Kind::Local,
        None if postscript => Kind::PostScript,
        None => Kind::Latex,
    };
    let ink = ui.visuals().text_color();
    ui.horizontal(|ui| {
        for (v, label, text) in [
            (Kind::PostScript, "PostScript fonts", "PostScript"),
            (Kind::Latex, "LaTeX fonts", "LaTeX"),
            (Kind::Local, "Local fonts", "Local fonts"),
        ] {
            let r = ui.radio_value(&mut kind, v, text);
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::RadioButton, true, label));
        }
    });
    st.kind = Some(match kind {
        Kind::PostScript => 0,
        Kind::Latex => 1,
        Kind::Local => 2,
    });
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
    let current = match local {
        Some(n) => Shown::Local(n.to_string()),
        None => Shown::Fig(font, postscript),
    };
    let shown = st.hover.clone().unwrap_or_else(|| current.clone());
    ui.separator();
    let shown_name = match &shown {
        Shown::Fig(f, ps) => list_name(*f, *ps),
        Shown::Local(n) => n.clone(),
    };
    ui.label(RichText::new(format!("Preview: {shown_name}")).small());
    // A fixed height: a preview that grew or shrank with the hovered font
    // would move the list under the pointer and make the hover flicker.
    let preview_h = sample_px * 2.0 + 8.0;
    egui::ScrollArea::horizontal()
        .id_salt(("font_preview", id))
        .max_height(preview_h)
        .min_scrolled_height(preview_h)
        .auto_shrink([false, false])
        .show(ui, |ui| match &shown {
            Shown::Fig(f, ps) => paint_text(ui, *f, *ps, None, SAMPLE, sample_px, ink),
            Shown::Local(n) => {
                let fig = engine_font(n);
                // A font without the Latin letters shows what it has.
                let text = if rustfig_text::local_font_covers(n, SAMPLE) {
                    SAMPLE
                } else {
                    rustfig_text::local_font_sample(n).unwrap_or(SAMPLE)
                };
                paint_text(ui, fig, true, Some(n), text, sample_px, ink)
            }
        });
    ui.separator();
    let real = has_real_face();
    let mut hover = None;
    let mut any = false;
    // One row: the name in its own font. Rows touch, so the pointer is never
    // between two of them.
    let row = |ui: &mut egui::Ui,
               name: &str,
               fig: i32,
               ps: bool,
               loc: Option<&str>,
               is_current: bool,
               substitute: bool|
     -> (bool, bool) {
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width().max(230.0), 24.0), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, is_current, name.to_string()));
        if ui.is_rect_visible(rect) {
            let p = ui.painter();
            if is_current {
                p.rect_filled(rect, 2.0, ui.visuals().selection.bg_fill);
            } else if resp.hovered() {
                p.rect_filled(rect, 2.0, ui.visuals().widgets.hovered.weak_bg_fill);
            }
            let color = if is_current {
                ui.visuals().selection.stroke.color
            } else {
                ink
            };
            if is_current {
                p.text(
                    rect.left_center() + vec2(4.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    "\u{2022}",
                    egui::FontId::proportional(14.0),
                    color,
                );
            }
            let inner = Rect::from_min_max(rect.min + vec2(18.0, 1.0), rect.max - vec2(2.0, 1.0));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
            child.horizontal_centered(|ui| {
                match loc.and_then(|n| uncovered(ui.ctx(), n)) {
                    // The name in the UI font, then what the font can draw.
                    Some(sample) => {
                        // Painted, not a label: the row already carries the name.
                        let g = ui
                            .painter()
                            .layout_no_wrap(name.to_string(), egui::FontId::proportional(15.0), color);
                        let (r, _) = ui.allocate_exact_size(g.size(), Sense::hover());
                        ui.painter().galley(r.min, g, color);
                        if let Some(s) = sample {
                            paint_text(ui, fig, ps, loc, s, 15.0, color);
                        }
                    }
                    None => paint_text(ui, fig, ps, loc, name, 15.0, color),
                }
                if substitute {
                    ui.label(RichText::new("(substitute)").small().weak());
                }
            });
        }
        (resp.hovered(), resp.clicked())
    };
    let list = egui::ScrollArea::vertical()
        .id_salt(("font_list", id))
        .max_height(300.0)
        .min_scrolled_height(300.0)
        .auto_shrink([false, false]);
    let list = match kind {
        Kind::PostScript | Kind::Latex => {
            let ps_list = kind == Kind::PostScript;
            let count = if ps_list {
                rustfig_text::PS_FONT_COUNT
            } else {
                rustfig_text::LATEX_FONT_COUNT
            };
            list.show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for f in 0..count {
                    let name = list_name(f, ps_list);
                    if !matches(&name, &st.query) {
                        continue;
                    }
                    any = true;
                    let is_current = local.is_none() && f == font && ps_list == postscript;
                    let (hov, clicked) = row(ui, &name, f, ps_list, None, is_current, !real);
                    if hov {
                        hover = Some(Shown::Fig(f, ps_list));
                    }
                    if clicked {
                        out.font = Some((f, ps_list));
                        out.local = Some(None);
                    }
                }
                if !any {
                    ui.label(RichText::new("No font matches.").weak());
                }
            })
        }
        Kind::Local => {
            // Only the visible rows are drawn, so only their fonts are loaded.
            let faces: Vec<rustfig_text::LocalFace> = rustfig_text::local_faces()
                .into_iter()
                .filter(|f| matches(&f.name, &st.query))
                .collect();
            if faces.is_empty() {
                list.show(ui, |ui| {
                    let msg = if rustfig_text::local_faces().is_empty() {
                        "No installed fonts are available here."
                    } else {
                        "No font matches."
                    };
                    ui.label(RichText::new(msg).weak());
                })
            } else {
                list.show_rows(ui, 24.0, faces.len(), |ui, range| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for f in faces.get(range).unwrap_or_default() {
                        let fig = engine_font(&f.name);
                        let is_current = local.is_some_and(|l| l.eq_ignore_ascii_case(&f.name));
                        let (hov, clicked) = row(ui, &f.name, fig, true, Some(&f.name), is_current, false);
                        if hov {
                            hover = Some(Shown::Local(f.name.clone()));
                        }
                        if clicked {
                            out.font = Some((fig, true));
                            out.local = Some(Some(f.name.clone()));
                        }
                    }
                })
            }
        }
    };
    // Inside the list the preview keeps the last hovered font (the pointer
    // may be on the scroll bar); it returns to the current one on leaving.
    let in_list = ui.rect_contains_pointer(list.inner_rect);
    st.hover = hover.or(if in_list { st.hover } else { None });
    if out.font.is_some() {
        // The next opening starts with the list of the font's own kind.
        st = St::default();
    }
    ui.data_mut(|d| d.insert_temp(sid, st));
    out
}

/// The Fig font number nearest to the installed font `name` (0 if unknown).
fn engine_font(name: &str) -> i32 {
    rustfig_engine::local_font_choice(name).map(|(_, f)| f).unwrap_or(0)
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
                let (img, w, h) = rasterise(f, ps, None, &list_name(f, ps), 15.0).unwrap();
                assert!(w > 4 && h > 4);
                assert!(img.pixels.iter().any(|p| p.a() > 100), "font {f} {ps}");
            }
        }
        assert!(rasterise(0, true, None, "x", f32::NAN).is_none());
        let _ = rasterise(9999, true, None, "\u{0}", 12.0);
        // An installed font that is missing is drawn as its Fig font.
        assert!(rasterise(0, true, Some("No Such Family"), "abc", 15.0).is_some());
    }
}
