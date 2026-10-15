//! Attribute pictures drawn as vectors (plan W9): each is a small Fig
//! drawing rendered by RustFig's own rasteriser at the screen's pixel
//! density, so it is sharp at any scale and shows exactly what the canvas
//! draws (dash patterns, joins, caps, arrowheads). The compositions follow
//! xfig's 32×32 pictures (`w_icons.c`); xfig's bitmaps remain the fallback.
//
// Compositions after xfig src/w_icons.c (xfig licence, see LICENSE-xfig).

use rustfig_geom::{Affine, Point};
use rustfig_model::{
    Arc, Arrow, CapStyle, ColorRef, Document, JoinStyle, Justify, LineKind, Object, ObjectKind, PolyKind, Polyline,
    Style,
};
use rustfig_render::raster::{rasterize, Pixmap, RasterOptions};
use rustfig_render::{Scene, SceneOptions};
use std::sync::Arc as Shared;

/// Fig units per icon pixel (xfig's pictures are 32×32 pixels).
const U: f64 = crate::mode_icons::U;

/// A picture of an attribute value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VIcon {
    Line(LineKind),
    Join(JoinStyle),
    Cap(CapStyle),
    /// Arrowhead type and style (`style` 0 hollow, 1 filled).
    Arrow(i32, i32),
    /// Arrows at the start (backward) and end (forward).
    ArrowMode {
        backward: bool,
        forward: bool,
    },
    Arc {
        pie: bool,
    },
    Justify(Justify),
    /// One of xfig's bitmaps traced as a vector drawing (`mode_icons`), by
    /// its name.
    Traced(&'static str),
}

impl VIcon {
    /// Size in icon pixels.
    fn size(self) -> (usize, usize) {
        match self {
            VIcon::Traced(name) => crate::mode_icons::size(name),
            _ => (32, 32),
        }
    }
}

fn style(thickness: i32) -> Style {
    Style {
        line: LineKind::Solid,
        thickness,
        pen_color: ColorRef::BLACK,
        ..Style::default()
    }
}

/// `x`, `y` in icon pixels.
fn px(x: f64, y: f64) -> Point {
    Point::new(x * U, y * U)
}

fn line(st: Style, pts: &[(f64, f64)]) -> Object {
    let p = Polyline::new(PolyKind::Polyline, st, pts.iter().map(|&(x, y)| px(x, y)).collect());
    Object::new(ObjectKind::Polyline(p))
}

fn arrow(kind: i32, style: i32) -> Arrow {
    Arrow {
        kind,
        style,
        thickness: 1.0,
        width: 7.0 * U,
        height: 10.0 * U,
    }
}

/// The picture as a drawing in a 32×32-pixel box (Fig units, black ink;
/// hollow arrowheads are filled white, as fig2dev draws them).
fn drawing(icon: VIcon) -> Vec<Object> {
    match icon {
        VIcon::Line(kind) => {
            let st = Style {
                line: kind,
                style_val: 4.0,
                ..style(2)
            };
            vec![line(st, &[(2.0, 16.0), (30.0, 16.0)])]
        }
        VIcon::Join(join) => {
            let st = Style { join, ..style(10) };
            vec![line(st, &[(6.0, 15.0), (26.0, 15.0), (7.0, 28.0)])]
        }
        VIcon::Cap(cap) => {
            let thick = Style { cap, ..style(10) };
            let thin = style(1);
            vec![
                line(thick, &[(8.0, 16.0), (24.0, 16.0)]),
                // xfig's marks at the line's true end points.
                line(thin, &[(8.0, 9.0), (8.0, 23.0)]),
                line(thin, &[(24.0, 9.0), (24.0, 23.0)]),
            ]
        }
        VIcon::Arrow(kind, style_n) => {
            let mut p = Polyline::new(PolyKind::Polyline, style(1), vec![px(2.0, 16.0), px(28.0, 16.0)]);
            p.forward_arrow = Some(arrow(kind, style_n));
            vec![Object::new(ObjectKind::Polyline(p))]
        }
        VIcon::ArrowMode { backward, forward } => {
            let mut p = Polyline::new(PolyKind::Polyline, style(1), vec![px(3.0, 16.0), px(29.0, 16.0)]);
            let a = Arrow {
                width: 6.0 * U,
                height: 6.0 * U,
                ..arrow(1, 1)
            };
            p.forward_arrow = forward.then_some(a);
            p.backward_arrow = backward.then_some(a);
            vec![Object::new(ObjectKind::Polyline(p))]
        }
        VIcon::Arc { pie } => Arc::through(style(2), px(16.0, 3.0), px(22.0, 16.0), px(16.0, 29.0), pie)
            .map(|a| vec![Object::new(ObjectKind::Arc(a))])
            .unwrap_or_default(),
        VIcon::Traced(name) => crate::mode_icons::drawing(name).unwrap_or_default(),
        VIcon::Justify(j) => {
            // Lines of text of different lengths, aligned as `j` aligns them.
            let st = style(3);
            [(5.0, 22.0), (10.0, 10.0), (15.0, 16.0), (20.0, 12.0), (25.0, 19.0)]
                .iter()
                .map(|&(y, len)| {
                    let (x0, x1) = match j {
                        Justify::Left => (5.0, 5.0 + len),
                        Justify::Center => (16.0 - len / 2.0, 16.0 + len / 2.0),
                        Justify::Right => (27.0 - len, 27.0),
                    };
                    line(st, &[(x0, y), (x1, y)])
                })
                .collect()
        }
    }
}

/// The picture rendered at `scale` device pixels per icon pixel: white,
/// with the ink's darkness as alpha (drawn black on white, so white fills
/// vanish).
pub fn image(icon: VIcon, scale: f32) -> Option<egui::ColorImage> {
    let (w, h) = icon.size();
    render_objects(drawing(icon), w, h, scale)
}

/// Render `objs` (Fig units, 15 per icon pixel) into a `w`×`h`-pixel icon
/// at `scale` device pixels per icon pixel.
pub fn render_objects(objs: Vec<Object>, w: usize, h: usize, scale: f32) -> Option<egui::ColorImage> {
    let scale = if scale.is_finite() {
        scale.clamp(0.25, 16.0)
    } else {
        1.0
    };
    let (pw, ph) = (
        ((w as f32 * scale).round() as usize).clamp(1, 1024),
        ((h as f32 * scale).round() as usize).clamp(1, 1024),
    );
    let mut doc = Document::new();
    doc.objects = objs.into_iter().map(Shared::new).collect();
    let scene = Scene::build(&doc, &SceneOptions::default());
    let mut pm = Pixmap::new(pw as u32, ph as u32)?;
    pm.data_mut().fill(255); // opaque white
    let k = pw as f64 / (w as f64 * U);
    rasterize(
        &scene,
        &mut pm,
        &RasterOptions {
            transform: Affine::scale(k, k),
            min_line_px: 1.0,
            pattern: rustfig_render::pattern::PatternFrame::screen(),
        },
    );
    let pixels = pm
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| {
            let dark = 255 - p.first().copied().unwrap_or(255);
            egui::Color32::from_rgba_premultiplied(dark, dark, dark, dark)
        })
        .collect();
    Some(egui::ColorImage {
        size: [pw, ph],
        source_size: egui::vec2(pw as f32, ph as f32),
        pixels,
    })
}

/// Paint `icon` centred in `rect` at its natural size (one icon pixel per
/// point, as xfig's bitmaps), shrunk if `rect` is smaller, tinted, at the
/// screen's pixel density (made once per icon and scale, kept in egui's
/// memory).
pub fn paint(ui: &egui::Ui, icon: VIcon, rect: egui::Rect, tint: egui::Color32) {
    let (w, h) = icon.size();
    let fit = (rect.width() / w as f32).min(rect.height() / h as f32).min(1.0);
    let scale = fit * ui.ctx().pixels_per_point();
    // Cache key: the scale in 1/8 device pixels per icon pixel.
    let key = (scale * 8.0).round() as i32;
    let id = egui::Id::new(("vector-icon", icon, key));
    let cached: Option<egui::TextureHandle> = ui.ctx().data(|d| d.get_temp(id));
    let tex = match cached {
        Some(t) => t,
        None => {
            let Some(img) = image(icon, key as f32 / 8.0) else {
                return;
            };
            let t = ui
                .ctx()
                .load_texture(format!("vector-icon-{icon:?}-{key}"), img, egui::TextureOptions::LINEAR);
            ui.ctx().data_mut(|d| d.insert_temp(id, t.clone()));
            t
        }
    };
    let r = egui::Rect::from_center_size(rect.center(), egui::vec2(w as f32 * fit, h as f32 * fit));
    ui.painter().image(
        tex.id(),
        r,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        tint,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ink(icon: VIcon) -> usize {
        let img = image(icon, 1.0).unwrap();
        img.pixels.iter().filter(|p| p.a() > 100).count()
    }

    #[test]
    fn every_picture_has_ink() {
        let mut icons = vec![
            VIcon::Arc { pie: false },
            VIcon::Arc { pie: true },
            VIcon::Join(JoinStyle::Miter),
            VIcon::Cap(CapStyle::Round),
            VIcon::Justify(Justify::Center),
        ];
        for k in [
            LineKind::Solid,
            LineKind::Dashed,
            LineKind::Dotted,
            LineKind::DashDotted,
            LineKind::DashDoubleDotted,
            LineKind::DashTripleDotted,
        ] {
            icons.push(VIcon::Line(k));
        }
        for kind in 0..=14 {
            for st in 0..=1 {
                icons.push(VIcon::Arrow(kind, st));
            }
        }
        for (b, f) in [(false, false), (false, true), (true, false), (true, true)] {
            icons.push(VIcon::ArrowMode {
                backward: b,
                forward: f,
            });
        }
        for i in icons {
            assert!(ink(i) > 10, "{i:?} is empty");
        }
    }

    #[test]
    fn styles_differ() {
        assert!(ink(VIcon::Line(LineKind::Solid)) > ink(VIcon::Line(LineKind::Dashed)));
        assert!(ink(VIcon::Arrow(1, 1)) > ink(VIcon::Arrow(1, 0)), "filled has more ink");
        assert!(ink(VIcon::Arc { pie: true }) > ink(VIcon::Arc { pie: false }));
        assert!(
            ink(VIcon::ArrowMode {
                backward: true,
                forward: true
            }) > ink(VIcon::ArrowMode {
                backward: false,
                forward: false
            })
        );
    }

    #[test]
    fn sharper_at_higher_density() {
        let lo = image(VIcon::Join(JoinStyle::Round), 1.0).unwrap();
        let hi = image(VIcon::Join(JoinStyle::Round), 3.0).unwrap();
        assert_eq!(hi.size, [96, 96]);
        let a = |img: &egui::ColorImage| img.pixels.iter().filter(|p| p.a() > 100).count() as f64;
        // The same shape: ink grows with the area (×9), within anti-aliasing.
        let ratio = a(&hi) / a(&lo);
        assert!((7.0..11.0).contains(&ratio), "{ratio}");
    }
}
