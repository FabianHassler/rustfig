//! RustFig's application icon (plan W9 d): an original drawing made for
//! RustFig, generated in code and rendered by RustFig's own rasteriser, so
//! there is no image file: a sheet of paper on a dark rounded square, with
//! a rust-coloured spline, a teal box and a circle, as drawn with xfig's
//! tools. MIT OR Apache-2.0, like the code.

use rustfig_geom::{Affine, Point};
use rustfig_model::{
    ColorRef, Document, Ellipse, EllipseKind, LineKind, Object, ObjectKind, PolyKind, Polyline, Rgb, Spline,
    SplineKind, Style, UserColor,
};
use rustfig_render::raster::{rasterize, Pixmap, RasterOptions};
use rustfig_render::{Scene, SceneOptions};
use std::sync::Arc;

/// The drawing's side in Fig units (one icon "pixel" at 64 px = 15 units).
const SIDE: f64 = 64.0 * 15.0;

const SLATE: ColorRef = ColorRef(32);
const RUST: ColorRef = ColorRef(33);
const TEAL: ColorRef = ColorRef(34);
const PAPER: ColorRef = ColorRef(35);
const FOLD: ColorRef = ColorRef(36);

/// A style at `depth` (xfig draws higher depths first; within one depth it
/// draws ellipses before polylines, so the layers need their own depths).
fn st_at(depth: i32, pen: ColorRef, thickness: i32, fill: Option<ColorRef>) -> Style {
    Style {
        depth,
        ..st(pen, thickness, fill)
    }
}

fn st(pen: ColorRef, thickness: i32, fill: Option<ColorRef>) -> Style {
    Style {
        line: LineKind::Solid,
        thickness,
        pen_color: pen,
        fill_color: fill.unwrap_or(ColorRef::WHITE),
        area_fill: if fill.is_some() { 20 } else { -1 },
        ..Style::default()
    }
}

/// `x`, `y` on a 64-unit grid.
fn q(x: f64, y: f64) -> Point {
    Point::new(x * 15.0, y * 15.0)
}

fn drawing() -> Document {
    let mut doc = Document::new();
    doc.colors = vec![
        UserColor {
            number: 32,
            rgb: Rgb::new(0x2b, 0x30, 0x3b),
        },
        UserColor {
            number: 33,
            rgb: Rgb::new(0xd2, 0x69, 0x2a),
        },
        UserColor {
            number: 34,
            rgb: Rgb::new(0x2a, 0x9d, 0x8f),
        },
        UserColor {
            number: 35,
            rgb: Rgb::new(0xfa, 0xf7, 0xf0),
        },
        UserColor {
            number: 36,
            rgb: Rgb::new(0xd9, 0xd3, 0xc7),
        },
    ];
    let mut objs = Vec::new();
    // The rounded square.
    let mut bg = Polyline::new(
        PolyKind::ArcBox,
        st_at(60, SLATE, 0, Some(SLATE)),
        vec![q(2.0, 2.0), q(62.0, 2.0), q(62.0, 62.0), q(2.0, 62.0)],
    );
    bg.radius = 12; // 1/80 inch, as Fig radii are
    objs.push(ObjectKind::Polyline(bg));
    // The sheet of paper, its corner turned down.
    objs.push(ObjectKind::Polyline(Polyline::new(
        PolyKind::Polygon,
        st_at(55, PAPER, 0, Some(PAPER)),
        vec![q(13.0, 9.0), q(43.0, 9.0), q(52.0, 18.0), q(52.0, 55.0), q(13.0, 55.0)],
    )));
    objs.push(ObjectKind::Polyline(Polyline::new(
        PolyKind::Polygon,
        st_at(54, FOLD, 0, Some(FOLD)),
        vec![q(43.0, 9.0), q(43.0, 18.0), q(52.0, 18.0)],
    )));
    // A teal box and a circle.
    objs.push(ObjectKind::Polyline(Polyline::new(
        PolyKind::Box,
        st(TEAL, 4, None),
        vec![q(18.0, 34.0), q(31.0, 34.0), q(31.0, 48.0), q(18.0, 48.0)],
    )));
    objs.push(ObjectKind::Ellipse(Ellipse {
        kind: EllipseKind::CircleRadius,
        style: st(TEAL, 4, None),
        direction: 1,
        angle: 0.0,
        center: q(40.0, 41.0),
        radii: Point::new(6.0 * 15.0, 6.0 * 15.0),
        start: q(40.0, 41.0),
        end: q(46.0, 41.0),
    }));
    // The rust-coloured spline across the sheet.
    objs.push(ObjectKind::Spline(Spline::new(
        SplineKind::Interpolated,
        false,
        st(RUST, 6, None),
        vec![
            q(17.0, 28.0),
            q(24.0, 16.0),
            q(32.0, 24.0),
            q(40.0, 30.0),
            q(47.0, 18.0),
        ],
    )));
    doc.objects = objs.into_iter().map(|k| Arc::new(Object::new(k))).collect();
    doc
}

/// The icon as straight (not premultiplied) RGBA, `size` pixels square.
pub fn rgba(size: u32) -> Option<Vec<u8>> {
    let size = size.clamp(16, 1024);
    let scene = Scene::build(&drawing(), &SceneOptions::default());
    let mut pm = Pixmap::new(size, size)?;
    let k = f64::from(size) / SIDE;
    rasterize(
        &scene,
        &mut pm,
        &RasterOptions {
            transform: Affine::scale(k, k),
            min_line_px: 1.0,
            pattern: rustfig_render::pattern::PatternFrame::screen(),
        },
    );
    // tiny-skia keeps premultiplied alpha; window icons want it straight.
    let mut out = pm.data().to_vec();
    for px in out.as_chunks_mut::<4>().0.iter_mut() {
        let a = u32::from(px.get(3).copied().unwrap_or(0));
        if a > 0 && a < 255 {
            for c in px.iter_mut().take(3) {
                *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    Some(out)
}

/// The window icon.
pub fn icon_data() -> Option<egui::IconData> {
    let size = 256;
    Some(egui::IconData {
        rgba: rgba(size)?,
        width: size,
        height: size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_renders_with_its_colours() {
        let px = rgba(64).unwrap();
        assert_eq!(px.len(), 64 * 64 * 4);
        let at = |x: usize, y: usize| &px[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4];
        assert_eq!(at(0, 0)[3], 0, "transparent outside the rounded square");
        assert_eq!(at(5, 32), &[0x2b, 0x30, 0x3b, 255], "slate border");
        assert_eq!(at(48, 52)[..3], [0xfa, 0xf7, 0xf0], "paper");
        assert!(icon_data().is_some());
    }

    /// Writes the icon for review: `RUSTFIG_APP_ICON=icon.png cargo test
    /// -p rustfig-ui-egui --lib app_icon_png -- --ignored`.
    #[test]
    #[ignore]
    fn app_icon_png() {
        let path = std::env::var("RUSTFIG_APP_ICON").unwrap_or_else(|_| "app_icon.png".into());
        let mut pm = Pixmap::new(256, 256).unwrap();
        pm.data_mut().copy_from_slice(&rgba(256).unwrap());
        pm.save_png(path).unwrap();
    }
}
