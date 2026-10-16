//! One entry point for every export format.
//
// Ported from fig2dev fig2dev.c (`main`: `-Z`, metric magnification),
// read.c (`-s`, the GIF transparent colour), dev/genps.c (`-f`,
// `PSFONTMAG`) and dev/genbitmaps.c (bitmap size and border) (xfig
// licence, see LICENSE-xfig).

use crate::options::{DriverOptions, SizeDim};
use crate::{raster, Rgba, Scene, SceneOptions};
use rustfig_geom::Rect;
use rustfig_model::{Document, ObjectKind, Rgb, Units};
use std::collections::BTreeSet;

/// Export formats rendered from a scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Svg,
    Eps,
    Pdf,
    Png,
    /// fig2dev's LaTeX box (`-L box`).
    LatexBox,
    Gif,
    Jpeg,
    Pcx,
    Ppm,
    Tiff,
    Xbm,
    Xpm,
}

impl ExportFormat {
    pub fn from_name(s: &str) -> Option<ExportFormat> {
        match s.to_ascii_lowercase().trim_start_matches('.') {
            "svg" => Some(ExportFormat::Svg),
            "eps" | "ps" => Some(ExportFormat::Eps),
            "pdf" => Some(ExportFormat::Pdf),
            "png" => Some(ExportFormat::Png),
            "box" => Some(ExportFormat::LatexBox),
            "gif" => Some(ExportFormat::Gif),
            "jpeg" | "jpg" => Some(ExportFormat::Jpeg),
            "pcx" => Some(ExportFormat::Pcx),
            "ppm" => Some(ExportFormat::Ppm),
            "tiff" | "tif" => Some(ExportFormat::Tiff),
            "xbm" => Some(ExportFormat::Xbm),
            "xpm" => Some(ExportFormat::Xpm),
            _ => None,
        }
    }

    pub fn from_path(path: &str) -> Option<ExportFormat> {
        let ext = path.rsplit_once('.').map(|(_, e)| e)?;
        ExportFormat::from_name(ext)
    }

    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Svg => "svg",
            ExportFormat::Eps => "eps",
            ExportFormat::Pdf => "pdf",
            ExportFormat::Png => "png",
            ExportFormat::LatexBox => "box",
            ExportFormat::Gif => "gif",
            ExportFormat::Jpeg => "jpg",
            ExportFormat::Pcx => "pcx",
            ExportFormat::Ppm => "ppm",
            ExportFormat::Tiff => "tif",
            ExportFormat::Xbm => "xbm",
            ExportFormat::Xpm => "xpm",
        }
    }

    /// Whether it is a bitmap (fig2dev's genbitmaps languages and PNG).
    pub fn is_bitmap(self) -> bool {
        !matches!(
            self,
            ExportFormat::Svg | ExportFormat::Eps | ExportFormat::Pdf | ExportFormat::LatexBox
        )
    }

    /// Whether the output is text (the rest is binary).
    pub fn is_text(self) -> bool {
        matches!(
            self,
            ExportFormat::Svg | ExportFormat::Eps | ExportFormat::LatexBox | ExportFormat::Xbm | ExportFormat::Xpm
        )
    }
}

/// Export parameters.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Magnification in percent; `None` uses the document's.
    pub magnification: Option<f64>,
    /// Border around the drawing in Fig units.
    pub margin: f64,
    /// Bitmap resolution (pixels per inch at 100 %).
    pub dpi: f64,
    /// Background colour; `None` is transparent (bitmaps) or no background.
    pub background: Option<Rgb>,
    /// Depths to leave out (xfig's "active layers only").
    pub hidden_depths: BTreeSet<i32>,
    /// With depths left out: size the page by the whole figure rather than
    /// by the active layers alone.
    pub whole_figure_bounds: bool,
    /// fig2dev's options (`file.export`'s `options`).
    pub fig2dev: DriverOptions,
    /// Leave special (LaTeX) texts out of the drawing, as the graphics
    /// half of fig2dev's `pstex` and `pdftex` does; they still count in
    /// the bounding box, so that the `pstex_t` overlay matches.
    pub skip_special_text: bool,
    /// EPS pictures to embed in EPS output, by the picture's file name
    /// (RustFig can't draw them; other formats show a placeholder).
    pub embedded_eps: std::collections::BTreeMap<String, EmbeddedEps>,
}

/// An EPS picture's content, for embedding (fig2dev's `read_eps`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedEps {
    /// The name fig2dev writes in its comments.
    pub title: String,
    /// The bounding box, in points.
    pub llx: i32,
    pub lly: i32,
    pub urx: i32,
    pub ury: i32,
    /// The PostScript (without a DOS EPS header).
    pub ps: Vec<u8>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            magnification: None,
            margin: 0.0,
            dpi: 150.0,
            background: None,
            hidden_depths: BTreeSet::new(),
            whole_figure_bounds: false,
            fig2dev: DriverOptions::default(),
            skip_special_text: false,
            embedded_eps: std::collections::BTreeMap::new(),
        }
    }
}

/// Region covered by an export: the drawing's bounds plus the margin.
pub fn export_bounds(scene: &Scene, margin: f64) -> Rect {
    let m = if margin.is_finite() {
        margin.clamp(0.0, 1.0e7)
    } else {
        0.0
    };
    scene.bbox().expand(m)
}

/// Render a document in the given format.
pub fn export(doc: &Document, format: ExportFormat, opts: &ExportOptions) -> Result<Vec<u8>, String> {
    let d = &opts.fig2dev;
    // fig2dev convention: metric drawings (450 units/cm) are magnified to true
    // size, but their text keeps its point size.
    let unit_fix = match doc.settings.units {
        Units::Metric => 1200.0 / 2.54 / 450.0,
        Units::Inches => 1.0,
    };
    // `-s` and `-f` change the texts before anything is measured.
    let adjusted;
    let doc = if d.font_size.is_some() || d.font.is_some() {
        adjusted = with_text_options(doc, d);
        &adjusted
    } else {
        doc
    };
    // `-D` leaves depths out; `-K` also leaves them out of the bounds.
    let filtered: BTreeSet<i32> = match &d.depths {
        Some(f) => doc.depths().into_iter().filter(|dep| !f.keeps(*dep)).collect(),
        None => BTreeSet::new(),
    };
    let mut hidden = opts.hidden_depths.clone();
    hidden.extend(filtered.iter().copied());
    // Without `-F`, fig2dev reads font sizes as 1/80 inch (`PSFONTMAG`).
    let text_scale = if d.correct_font_size {
        1.0 / unit_fix
    } else {
        72.0 / 80.0
    };
    let without_special;
    let scene_doc = if opts.skip_special_text {
        without_special = without_special_text(doc);
        &without_special
    } else {
        doc
    };
    let mut scene = Scene::build(
        scene_doc,
        &SceneOptions {
            hidden_depths: hidden,
            text_scale: Some(text_scale),
            ..SceneOptions::default()
        },
    );
    // The page is sized by fig2dev's bounds (`bound.c`), not by the ink.
    let mut bounds_hidden = if opts.whole_figure_bounds {
        BTreeSet::new()
    } else {
        opts.hidden_depths.clone()
    };
    if d.adjust_bbox {
        bounds_hidden.extend(filtered.iter().copied());
    }
    let mut raw = crate::bound::fig2dev_bounds(doc, &bounds_hidden, true);
    if raw.is_empty() {
        return Err("the drawing is empty".into());
    }
    // fig2dev gives a figure without width or height some (only LaTeX text).
    if raw.width() == 0.0 {
        raw.max.x += 100.0;
    }
    if raw.height() == 0.0 {
        raw.max.y += 100.0;
    }
    let mag = if let Some(z) = d.max_size {
        // `-Z`: the size of one dimension, in inches (cm in metric drawings).
        let figdim = match z.dim {
            SizeDim::Width => raw.width(),
            SizeDim::Height => raw.height(),
            SizeDim::Largest => raw.width().max(raw.height()),
        };
        let inches = match doc.settings.units {
            Units::Metric => z.size / 2.54,
            Units::Inches => z.size,
        };
        inches * 1200.0 / figdim
    } else if let Some(m) = d.magnification {
        m
    } else {
        opts.magnification.unwrap_or(doc.settings.magnification) / 100.0
    };
    let mag = if mag.is_finite() { mag.clamp(0.001, 1000.0) } else { 1.0 };
    let scale = mag * unit_fix;
    // The margin is in Fig units; fig2dev's `-b` border is in points on the page.
    let m = if opts.margin.is_finite() {
        opts.margin.clamp(0.0, 1.0e7)
    } else {
        0.0
    };
    let border = if !d.border.is_finite() {
        0.0
    } else if format.is_bitmap() {
        // genbitmaps: pixels at 80 dpi before the magnification, so
        // `THICK_SCALE` (15) Fig units each.
        d.border.clamp(0.0, 72_000.0) * 15.0
    } else {
        d.border.clamp(0.0, 72_000.0) * 1200.0 / 72.0 / scale
    };
    let bounds = raw.expand(m + border);
    let mut bg = d.background.or(opts.background).map(Rgba::opaque);
    if d.grayscale {
        scene.map_colors(Rgba::luminance_gray);
        bg = bg.map(Rgba::luminance_gray);
    }
    let title = d.title.as_deref();
    Ok(match format {
        ExportFormat::Svg => crate::svg::to_svg(&scene, &bounds, scale, bg).into_bytes(),
        ExportFormat::Eps => crate::eps::to_eps(&scene, &bounds, scale, bg, title, &opts.embedded_eps).into_bytes(),
        ExportFormat::Pdf => crate::pdf::to_pdf(&scene, &bounds, scale, bg, title),
        ExportFormat::LatexBox => crate::drivers::latex_box::to_box(&doc.comments, &bounds, scale).into_bytes(),
        ExportFormat::Png
        | ExportFormat::Gif
        | ExportFormat::Jpeg
        | ExportFormat::Pcx
        | ExportFormat::Ppm
        | ExportFormat::Tiff
        | ExportFormat::Xbm
        | ExportFormat::Xpm => {
            let dpi = if opts.dpi.is_finite() {
                opts.dpi.clamp(1.0, 4800.0)
            } else {
                150.0
            };
            let px_per_fig = dpi / 1200.0 * scale;
            // genbitmaps sizes the image as `(int)(size + 0.9)`.
            let (w, h) = raster::checked_size(
                (bounds.width() * px_per_fig + 0.9).floor(),
                (bounds.height() * px_per_fig + 0.9).floor(),
            )?;
            let smoothing = match d.smoothing {
                None => raster::Smoothing::Antialias,
                Some(0 | 1) => raster::Smoothing::None,
                Some(k) => raster::Smoothing::Supersample(k),
            };
            let pm = raster::render_sized(
                &scene,
                &bounds,
                px_per_fig,
                (w, h),
                bg,
                crate::pattern::PatternFrame::export(&bounds, scale),
                smoothing,
            )?;
            if format == ExportFormat::Png {
                raster::encode_png(&pm)?
            } else {
                let transparent = d.transparent.or_else(|| figure_transparent_color(doc));
                crate::drivers::bitmaps::encode(
                    format,
                    &pm,
                    &crate::drivers::bitmaps::Encoding {
                        quality: d.quality.unwrap_or(crate::drivers::bitmaps::DEFAULT_JPEG_QUALITY),
                        transparent,
                        dpi,
                    },
                )?
            }
        }
    })
}

/// The figure's GIF transparent colour (its header line), as fig2dev's
/// `read_objects` takes it: a standard colour, or a user colour the
/// figure defines; the special values (none, default, background) give none.
fn figure_transparent_color(doc: &Document) -> Option<Rgb> {
    let n = doc.settings.transparent_color;
    if n < 0 {
        return None;
    }
    let defined = n < 32 || doc.colors.iter().any(|c| c.number == n);
    defined.then(|| doc.rgb(rustfig_model::ColorRef(n)))
}

/// The document without its special texts (genpstex.c `genpstex_text`).
fn without_special_text(doc: &Document) -> Document {
    fn keep(o: &mut rustfig_model::Object) -> bool {
        match &mut o.kind {
            ObjectKind::Text(t) => !t.is_special(),
            ObjectKind::Compound(c) => {
                c.objects.retain_mut(keep);
                true
            }
            _ => true,
        }
    }
    let mut doc = doc.clone();
    doc.objects.retain_mut(|o| keep(std::sync::Arc::make_mut(o)));
    doc
}

/// The document with fig2dev's `-s` (every text at one size, its stored
/// length and height scaled to match, as `read.c` does) and `-f` (the
/// default font and Times-Roman replaced, as `genps.c` does) applied.
fn with_text_options(doc: &Document, d: &DriverOptions) -> Document {
    fn walk(objs: &mut [rustfig_model::Object], d: &DriverOptions) {
        for o in objs {
            match &mut o.kind {
                ObjectKind::Compound(c) => walk(&mut c.objects, d),
                ObjectKind::Text(t) => {
                    if let Some(fs) = d.font_size {
                        // fig2dev scales `-s` by `-m` (`fontmag`).
                        let size = fs * d.magnification.unwrap_or(1.0);
                        if t.size > 0.0 && size.is_finite() && size > 0.0 {
                            t.length *= size / t.size;
                            t.height *= size / t.size;
                        }
                        t.size = size;
                    }
                    if let Some(font) = d.font {
                        // PostScript fonts -1 and 0, LaTeX fonts 0 and 1.
                        let replaced = if t.is_postscript() { t.font <= 0 } else { t.font <= 1 };
                        if replaced {
                            t.font = font;
                            t.flags |= rustfig_model::text_flags::POSTSCRIPT;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let mut doc = doc.clone();
    for o in &mut doc.objects {
        walk(std::slice::from_mut(std::sync::Arc::make_mut(o)), d);
    }
    doc
}

#[cfg(test)]
mod metric_text_tests {
    use super::*;
    use rustfig_geom::Point;
    use rustfig_model::{ColorRef, Justify, Object, ObjectKind, Text};

    fn text_scene_size(units: Units) -> f64 {
        let mut doc = Document::new();
        doc.settings.units = units;
        let t = Text {
            justify: Justify::Left,
            color: ColorRef::BLACK,
            depth: 50,
            pen_style: -1,
            font: 0,
            size: 12.0,
            angle: 0.0,
            flags: 4,
            height: 135.0,
            length: 100.0,
            origin: Point::new(0.0, 0.0),
            text: "x".into(),
        };
        doc.objects.push(std::sync::Arc::new(Object::new(ObjectKind::Text(t))));
        let unit_fix = if units == Units::Metric {
            1200.0 / 2.54 / 450.0
        } else {
            1.0
        };
        let scene = Scene::build(
            &doc,
            &SceneOptions {
                text_scale: Some(1.0 / unit_fix),
                ..Default::default()
            },
        );
        match scene.prims.first() {
            Some(crate::Prim::Text(t)) => t.size * unit_fix,
            _ => 0.0,
        }
    }

    #[test]
    fn metric_text_keeps_its_point_size() {
        // After the export's metric magnification the text is 12 points again.
        assert!((text_scene_size(Units::Metric) - 12.0).abs() < 1e-9);
        assert!((text_scene_size(Units::Inches) - 12.0).abs() < 1e-9);
    }
}

#[cfg(test)]
mod curve_tests {
    use super::*;
    use rustfig_geom::Point;
    use rustfig_model::{Ellipse, EllipseKind, Object, ObjectKind, Style};
    use std::sync::Arc;

    fn ellipse_doc() -> Document {
        let mut doc = Document::new();
        doc.objects.push(Arc::new(Object::new(ObjectKind::Ellipse(Ellipse {
            kind: EllipseKind::EllipseRadii,
            style: Style::default(),
            direction: 1,
            angle: 0.3,
            center: Point::new(1200.0, 1200.0),
            radii: Point::new(900.0, 500.0),
            start: Point::new(1200.0, 1200.0),
            end: Point::new(2100.0, 1200.0),
        }))));
        doc
    }

    /// A5: ellipses reach the vector formats as four curves, not hundreds of
    /// points; the raster backend draws the same flattened outline as before.
    #[test]
    fn ellipses_export_as_curves() {
        let doc = ellipse_doc();
        let opts = ExportOptions::default();
        let svg = String::from_utf8(export(&doc, ExportFormat::Svg, &opts).unwrap()).unwrap();
        assert_eq!(svg.matches('C').count(), 4, "{svg}");
        assert!(svg.len() < 3000, "{} bytes", svg.len());
        let eps = String::from_utf8(export(&doc, ExportFormat::Eps, &opts).unwrap()).unwrap();
        assert_eq!(eps.matches("curveto").count(), 4);
        let scene = Scene::build(&doc, &SceneOptions::default());
        let Some(crate::Prim::Path { subpaths, .. }) = scene.prims.first() else {
            panic!("no path")
        };
        assert!(subpaths[0].curve.is_some());
        assert!(subpaths[0].pts.len() > 100, "the raster keeps the flattening");
    }
}
