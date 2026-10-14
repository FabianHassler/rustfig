//! One entry point for every export format.

use crate::{raster, Rgba, Scene, SceneOptions};
use rustfig_geom::Rect;
use rustfig_model::{Document, Rgb, Units};
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
}

impl ExportFormat {
    pub fn from_name(s: &str) -> Option<ExportFormat> {
        match s.to_ascii_lowercase().trim_start_matches('.') {
            "svg" => Some(ExportFormat::Svg),
            "eps" | "ps" => Some(ExportFormat::Eps),
            "pdf" => Some(ExportFormat::Pdf),
            "png" => Some(ExportFormat::Png),
            "box" => Some(ExportFormat::LatexBox),
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
        }
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
    let scene = Scene::build(
        doc,
        &SceneOptions {
            hidden_depths: opts.hidden_depths.clone(),
            ..SceneOptions::default()
        },
    );
    let bounds = if opts.whole_figure_bounds && !opts.hidden_depths.is_empty() {
        export_bounds(&Scene::build(doc, &SceneOptions::default()), opts.margin)
    } else {
        export_bounds(&scene, opts.margin)
    };
    if bounds.is_empty() {
        return Err("the drawing is empty".into());
    }
    let mag = opts
        .magnification
        .unwrap_or(doc.settings.magnification)
        .clamp(0.1, 100_000.0)
        / 100.0;
    // fig2dev convention: metric drawings (450 units/cm) are magnified to true size.
    let unit_fix = match doc.settings.units {
        Units::Metric => 1200.0 / 2.54 / 450.0,
        Units::Inches => 1.0,
    };
    let scale = mag * unit_fix;
    let bg = opts.background.map(Rgba::opaque);
    Ok(match format {
        ExportFormat::Svg => crate::svg::to_svg(&scene, &bounds, scale, bg).into_bytes(),
        ExportFormat::Eps => crate::eps::to_eps(&scene, &bounds, scale, bg).into_bytes(),
        ExportFormat::Pdf => crate::pdf::to_pdf(&scene, &bounds, scale, bg),
        ExportFormat::LatexBox => crate::drivers::latex_box::to_box(&doc.comments, &bounds, scale).into_bytes(),
        ExportFormat::Png => {
            let dpi = if opts.dpi.is_finite() {
                opts.dpi.clamp(1.0, 4800.0)
            } else {
                150.0
            };
            let px_per_fig = dpi / 1200.0 * scale;
            let pm = raster::render_region(&scene, &bounds, px_per_fig, bg)?;
            raster::encode_png(&pm)?
        }
    })
}
