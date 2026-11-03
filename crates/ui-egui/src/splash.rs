//! The start screen: a card over the canvas for a few seconds after the
//! window opens, as xfig shows its splash graphic (`splash_screen` in
//! w_util.c, cleared by the first key or mouse press in main.c). xfig's
//! `splash` resource and `-nosplash` / `-splash` switch it. Drawn in code,
//! with the application icon ([`crate::app_icon`]): no image files.

use egui::{
    epaint::{Mesh, Shadow},
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Id, Order, Painter, Pos2, Rect, Shape, Stroke,
};

/// Seconds the card stays (fade-in included) before it fades out.
const SHOW_SECS: f64 = 3.0;
/// Seconds of the fade-in and of the fade-out.
const FADE_SECS: f64 = 0.35;
/// The card's size in points, at the start and as Help > About.
const CARD: egui::Vec2 = vec2(500.0, 260.0);
const ABOUT_CARD: egui::Vec2 = vec2(540.0, 340.0);
/// Width of the colored band with the icon.
const BAND: f32 = 180.0;
/// The icon's side in points.
const ICON: f32 = 128.0;
/// The icon's colors (`app_icon`): its slate square and its rust spline.
const SLATE: Color32 = Color32::from_rgb(0x2b, 0x30, 0x3b);
const SLATE_LIGHT: Color32 = Color32::from_rgb(0x46, 0x4e, 0x5e);
const RUST: Color32 = Color32::from_rgb(0xd2, 0x69, 0x2a);

/// The start screen's clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct Splash {
    /// When the card first appeared (egui time), once a frame drew it.
    shown_at: Option<f64>,
    /// When it started to fade out early (a key or a click).
    dismissed_at: Option<f64>,
}

impl Splash {
    /// Opacity at `now`, `None` once gone.
    fn opacity(&self, now: f64) -> Option<f32> {
        let start = self.shown_at.unwrap_or(now);
        let t = now - start;
        let fade_in = (t / FADE_SECS).clamp(0.0, 1.0);
        let end = self.dismissed_at.map_or(SHOW_SECS, |d| (d - start).min(SHOW_SECS));
        if t >= end + FADE_SECS {
            return None;
        }
        let fade_out = 1.0 - ((t - end) / FADE_SECS).clamp(0.0, 1.0);
        Some(fade_in.min(fade_out) as f32)
    }
}

/// Draw the card centerd on the canvas `rect`, if it is still due; the
/// first key or mouse press starts the fade-out (and still does its job).
pub fn show(splash: &mut Option<Splash>, ctx: &egui::Context, rect: Rect) {
    let Some(s) = splash.as_mut() else { return };
    let now = ctx.input(|i| i.time);
    if s.shown_at.is_none() {
        s.shown_at = Some(now);
    }
    let pressed = ctx.input(|i| {
        i.events.iter().any(|e| {
            matches!(
                e,
                egui::Event::PointerButton { pressed: true, .. } | egui::Event::Key { pressed: true, .. }
            )
        })
    });
    if pressed && s.dismissed_at.is_none() {
        s.dismissed_at = Some(now);
    }
    let Some(alpha) = s.opacity(now) else {
        *splash = None;
        return;
    };
    ctx.request_repaint();
    if rect.width() < 220.0 || rect.height() < 140.0 {
        return;
    }
    // A little lift as it fades in.
    let lift = (1.0 - alpha) * 8.0;
    card_area(ctx, rect.center() + vec2(0.0, lift), alpha, false);
}

/// Help > About RustFig: the same card with the license and xfig's
/// copyrights, until a click or a key (Escape too) closes it.
pub fn show_about(open: &mut bool, ctx: &egui::Context) {
    if !*open {
        return;
    }
    let screen = ctx.content_rect();
    let resp = card_area(ctx, screen.center(), 1.0, true);
    let elsewhere =
        ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !resp.rect.contains(p)));
    let key = ctx.input(|i| {
        i.events
            .iter()
            .any(|e| matches!(e, egui::Event::Key { pressed: true, .. }))
    });
    if resp.clicked() || elsewhere || key {
        *open = false;
    }
}

/// The card in its own layer above the canvas, centerd on `center`; a
/// click on it goes to the card, not to the drawing below.
fn card_area(ctx: &egui::Context, center: Pos2, alpha: f32, about: bool) -> egui::Response {
    let size = if about { ABOUT_CARD } else { CARD };
    let size = vec2(
        size.x.min(ctx.content_rect().width() - 24.0),
        size.y.min(ctx.content_rect().height() - 24.0),
    );
    let icon = icon_texture(ctx);
    let dark = ctx.style().visuals.dark_mode;
    egui::Area::new(Id::new(if about { "rustfig-about" } else { "rustfig-splash" }))
        .order(Order::Foreground)
        .fixed_pos(center - size / 2.0)
        .show(ctx, |ui| {
            ui.set_opacity(alpha);
            let (r, resp) = ui.allocate_exact_size(size, egui::Sense::click());
            paint_card(ui.painter(), r, dark, icon.as_ref(), about);
            resp.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Other,
                    true,
                    if about { "About RustFig" } else { "Start screen" },
                )
            });
            resp
        })
        .inner
}

/// The application icon as a texture, rendered once at the screen's
/// resolution and kept in egui's memory.
fn icon_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = Id::new("rustfig-splash-icon");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    let px = (ICON * ctx.pixels_per_point()).round().clamp(16.0, 1024.0) as u32;
    let rgba = crate::app_icon::rgba(px)?;
    let img = egui::ColorImage::from_rgba_unmultiplied([px as usize, px as usize], &rgba);
    let t = ctx.load_texture("rustfig-splash-icon", img, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    Some(t)
}

fn paint_card(p: &Painter, card: Rect, dark: bool, icon: Option<&egui::TextureHandle>, about: bool) {
    let radius = 14u8;
    let (bg, text, muted, rule) = if dark {
        (
            Color32::from_rgb(30, 32, 36),
            Color32::from_rgb(236, 237, 240),
            Color32::from_rgb(150, 154, 162),
            Color32::from_rgb(58, 61, 68),
        )
    } else {
        (
            Color32::from_rgb(252, 252, 253),
            Color32::from_rgb(28, 30, 34),
            Color32::from_rgb(112, 116, 124),
            Color32::from_rgb(226, 228, 232),
        )
    };
    let shadow = Shadow {
        offset: [0, 10],
        blur: 32,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 140 } else { 60 }),
    };
    p.add(shadow.as_shape(card, CornerRadius::same(radius)));
    p.rect_filled(card, CornerRadius::same(radius), bg);

    // The band: the icon's slate as a soft gradient, the icon on it.
    let band = Rect::from_min_size(card.min, vec2(BAND.min(card.width() * 0.4), card.height()));
    gradient(p, band, SLATE_LIGHT, SLATE, radius);
    if let Some(t) = icon {
        let side = ICON.min(band.width() - 24.0).min(band.height() - 24.0);
        let r = Rect::from_center_size(band.center(), vec2(side, side));
        p.image(
            t.id(),
            r,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    // The words.
    let x = band.right() + 28.0;
    let top = card.top() + 34.0;
    let title = FontId::proportional(34.0);
    let rust = p.text(pos2(x, top), Align2::LEFT_TOP, "Rust", title.clone(), RUST);
    p.text(pos2(rust.right(), top), Align2::LEFT_TOP, "Fig", title, text);
    p.text(
        pos2(x, rust.bottom() + 6.0),
        Align2::LEFT_TOP,
        format!("Version {}", env!("CARGO_PKG_VERSION")),
        FontId::proportional(13.0),
        muted,
    );
    let tag_y = rust.bottom() + 40.0;
    p.text(
        pos2(x, tag_y),
        Align2::LEFT_TOP,
        "Drawing figures, the xfig way,",
        FontId::proportional(16.0),
        text,
    );
    p.text(
        pos2(x, tag_y + 21.0),
        Align2::LEFT_TOP,
        "on macOS, Windows, Linux and the web.",
        FontId::proportional(16.0),
        text,
    );
    let small = FontId::proportional(11.0);
    let lines: &[&str] = if about {
        &[
            "Licensed MIT OR Apache-2.0; the parts from xfig and",
            "fig2dev keep the xfig license (see NOTICE).",
            "",
            "xfig: \u{a9} 1985\u{2013}1988 Supoj Sutanthavibul,",
            "\u{a9} 1989\u{2013}2015 Brian V. Smith, \u{a9} 1991 Paul King,",
            "\u{a9} 2015\u{2013}2024 Thomas Loimer and the other xfig authors.",
        ]
    } else {
        &[
            "Based on xfig and fig2dev by Supoj Sutanthavibul,",
            "Brian V. Smith, Paul King, Thomas Loimer and many others.",
        ]
    };
    let foot = card.bottom() - 28.0 - 15.0 * lines.len() as f32 - if about { 22.0 } else { 0.0 };
    p.line_segment(
        [pos2(x, foot), pos2(card.right() - 28.0, foot)],
        Stroke::new(1.0_f32, rule),
    );
    for (k, l) in lines.iter().enumerate() {
        p.text(
            pos2(x, foot + 12.0 + 15.0 * k as f32),
            Align2::LEFT_TOP,
            *l,
            small.clone(),
            muted,
        );
    }
    if about {
        p.text(
            pos2(card.right() - 28.0, card.bottom() - 22.0),
            Align2::RIGHT_BOTTOM,
            "Click or press a key to close",
            small,
            muted,
        );
    }
}

/// `r` filled from `a` (top left) to `b` (bottom right), its left corners
/// rounded like the card's.
fn gradient(p: &Painter, r: Rect, a: Color32, b: Color32, radius: u8) {
    let mix = |t: f32| {
        let l = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
        Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
    };
    // Clockwise: the top-left corner's arc, the straight right edge, the
    // bottom-left corner's arc (screen angles, y down).
    let rad = f32::from(radius).min(r.width() / 2.0).min(r.height() / 2.0);
    let arc = |c: Pos2, from: f32, out: &mut Vec<Pos2>| {
        for k in 0..=8 {
            let (s, co) = (from + 90.0 * k as f32 / 8.0).to_radians().sin_cos();
            out.push(c + vec2(co * rad, s * rad));
        }
    };
    let mut outline: Vec<Pos2> = Vec::new();
    arc(pos2(r.left() + rad, r.top() + rad), 180.0, &mut outline);
    outline.push(r.right_top());
    outline.push(r.right_bottom());
    arc(pos2(r.left() + rad, r.bottom() - rad), 90.0, &mut outline);
    let mut mesh = Mesh::default();
    let diag = (r.width() + r.height()).max(1.0);
    for q in &outline {
        let t = ((q.x - r.left()) + (q.y - r.top())) / diag;
        mesh.colored_vertex(*q, mix(t.clamp(0.0, 1.0)));
    }
    let n = outline.len() as u32;
    for i in 1..n.saturating_sub(1) {
        mesh.add_triangle(0, i, i + 1);
    }
    p.add(Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fades_in_stays_and_fades_out() {
        let s = Splash {
            shown_at: Some(10.0),
            ..Splash::default()
        };
        assert!(s.opacity(10.0).is_none_or(|a| a < 0.01));
        assert_eq!(s.opacity(11.0), Some(1.0));
        assert!(s.opacity(10.0 + SHOW_SECS + FADE_SECS / 2.0).is_some_and(|a| a < 1.0));
        assert!(s.opacity(10.0 + SHOW_SECS + FADE_SECS + 0.01).is_none());
    }

    #[test]
    fn a_press_fades_it_out_early() {
        let s = Splash {
            shown_at: Some(0.0),
            dismissed_at: Some(1.0),
        };
        assert_eq!(s.opacity(0.9), Some(1.0));
        assert!(s.opacity(1.0 + FADE_SECS + 0.01).is_none());
    }
}
