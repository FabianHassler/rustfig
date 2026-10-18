//! The writers behind one entry point: a scene, the page it covers and the
//! settings of the language, written as EPS, PDF, SVG, a LaTeX box or one
//! of the bitmap languages.
//
// Ported from fig2dev dev/genbitmaps.c (bitmap size and smoothing) and
// dev/genps.c (PostScript pages) (xfig licence, see LICENSE-xfig).

use super::preview::{self, PreviewKind, UserBox};
use super::{bitmaps, eps, latex_box, pattern, pdf, raster, svg, Rgba, Scene};
use figbase::color::Rgb;
use figbase::geom::Rect;
use std::collections::BTreeMap;

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

/// Region covered by an export: the drawing's bounds plus the margin.
pub fn export_bounds(scene: &Scene, margin: f64) -> Rect {
    let m = if margin.is_finite() {
        margin.clamp(0.0, 1.0e7)
    } else {
        0.0
    };
    scene.bbox().expand(m)
}

/// The page a scene is written on and the language's settings.
#[derive(Debug, Clone)]
pub struct Page<'a> {
    /// The region written, in Fig units.
    pub bounds: Rect,
    /// Magnification, including the metric correction.
    pub scale: f64,
    /// Background colour; `None` is transparent (bitmaps) or none.
    pub background: Option<Rgba>,
    /// `-n`: the title of EPS and PDF output.
    pub title: Option<&'a str>,
    /// EPS pictures to embed in EPS output, by the picture's file name.
    pub embedded_eps: &'a BTreeMap<String, EmbeddedEps>,
    /// The whole-figure comments (written by the LaTeX box).
    pub comments: &'a [String],
    /// Bitmap resolution at 100 % (pixels per inch).
    pub dpi: f64,
    /// Bitmap smoothing: `None` antialiased, else genbitmaps' `-S`
    /// (0 and 1 hard edges, 2 and 4 supersampled).
    pub smoothing: Option<u32>,
    /// JPEG quality (`-q`).
    pub quality: Option<u8>,
    /// The GIF transparent colour (`-t`, or the figure's).
    pub transparent: Option<Rgb>,
    /// PostScript and PDF page mode: the paper and the figure's places
    /// on it (`-L ps`; `-L pdf` with `-P` or `-z`).
    pub layout: Option<super::PageLayout>,
    /// The PDF version 1.n (`-Y`).
    pub pdf_minor: u8,
    /// The figure's magnification, for PostScript's `%%Magnification`.
    pub magnification: f64,
    /// EPS: the user's bounding box (`-B`, `-R`).
    pub bbox: Option<UserBox<'a>>,
    /// EPS and PostScript: a preview (`-A`, `-T`, `-C`).
    pub preview: Option<PreviewKind>,
    /// A metric figure (the user's bounding box is in centimetres).
    pub metric: bool,
    /// PNG: composite over white and write no alpha channel (fig2dev's
    /// PNG comes from ghostscript's opaque image).
    pub opaque: bool,
    /// `-b`, the border in whole points (the user's bounding box adds it).
    pub border_pt: i32,
    /// SVG `-z`: the paper's width and height in points.
    pub svg_paper: Option<(i32, i32)>,
}

/// The preview's canvas: the bounding box's size plus one point each way,
/// with the figure at the page's lower left corner (or, for a page, where
/// the layout puts it).
fn preview_canvas(
    scene: &Scene,
    page: &Page<'_>,
    bbox: [i32; 4],
    fig_pt: (f64, f64),
    at: (f64, f64),
) -> Result<preview::Canvas, String> {
    let scale = page.scale;
    let pt = 72.0 / 1200.0 * scale;
    let (cw, ch) = raster::checked_size(
        f64::from(bbox[2]) - f64::from(bbox[0]) + 1.0,
        f64::from(bbox[3]) - f64::from(bbox[1]) + 1.0,
    )?;
    let (fw, fh) = raster::checked_size(fig_pt.0.ceil(), fig_pt.1.ceil())?;
    let pm = raster::render_sized(
        scene,
        &page.bounds,
        pt,
        (fw, fh),
        page.background,
        pattern::PatternFrame::export(&page.bounds, scale),
        raster::Smoothing::None,
    )?;
    let mut canvas = preview::Canvas::white(cw, ch).ok_or("preview too large")?;
    canvas.paste(
        &bitmaps::rgb_over_white(&pm),
        fw,
        fh,
        at.0.round() as i64,
        at.1.round() as i64,
    );
    Ok(canvas)
}

/// Write `scene` on `page` in `format`.
pub fn write(scene: &Scene, format: ExportFormat, page: &Page<'_>) -> Result<Vec<u8>, String> {
    write_with_notes(scene, format, page).map(|(bytes, _)| bytes)
}

/// [`write`], and the messages fig2dev prints on stderr while writing
/// (a preview that could not be made).
pub fn write_with_notes(
    scene: &Scene,
    format: ExportFormat,
    page: &Page<'_>,
) -> Result<(Vec<u8>, Vec<String>), String> {
    let (bounds, scale, bg) = (&page.bounds, page.scale, page.background);
    let mut notes = Vec::new();
    let bytes = match format {
        ExportFormat::Svg => svg::to_svg_page(scene, bounds, scale, bg, page.svg_paper).into_bytes(),
        ExportFormat::Eps => {
            let mut text = match &page.layout {
                Some(l) => eps::to_ps(
                    scene,
                    bounds,
                    scale,
                    bg,
                    page.title,
                    page.embedded_eps,
                    l,
                    page.magnification,
                ),
                None => eps::to_eps(scene, bounds, scale, bg, page.title, page.embedded_eps),
            };
            let pt = 72.0 / 1200.0 * scale;
            let fig_pt = (bounds.width() * pt, bounds.height() * pt);
            // genps: the box of the figure, or of the paper.
            let mut bbox = match &page.layout {
                Some(l) => [0, 0, l.width, l.height],
                None => [0, 0, fig_pt.0.ceil() as i32, fig_pt.1.ceil() as i32],
            };
            if let (Some(b), None) = (page.bbox, &page.layout) {
                bbox = preview::user_bbox(
                    b,
                    page.metric,
                    bounds.min.x * pt,
                    bounds.min.y * pt,
                    bounds.max.x * pt,
                    bounds.max.y * pt,
                    page.border_pt,
                );
                text = preview::set_bbox(&text, bbox);
            }
            match page.preview {
                None => text.into_bytes(),
                Some(kind) => {
                    let at = page
                        .layout
                        .as_ref()
                        .and_then(|l| l.pages.first())
                        .copied()
                        .unwrap_or((0.0, 0.0));
                    match preview_canvas(scene, page, bbox, fig_pt, at)
                        .and_then(|c| preview::attach(kind, &text, &c, page.layout.is_none()))
                    {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            notes.push(e);
                            notes.push("No preview will be produced".into());
                            text.into_bytes()
                        }
                    }
                }
            }
        }
        ExportFormat::Pdf => pdf::to_pdf_pages(
            scene,
            bounds,
            scale,
            bg,
            page.title,
            page.layout.as_ref(),
            page.pdf_minor,
        ),
        ExportFormat::LatexBox => latex_box::to_box(page.comments, bounds, scale).into_bytes(),
        ExportFormat::Png
        | ExportFormat::Gif
        | ExportFormat::Jpeg
        | ExportFormat::Pcx
        | ExportFormat::Ppm
        | ExportFormat::Tiff
        | ExportFormat::Xbm
        | ExportFormat::Xpm => {
            let dpi = if page.dpi.is_finite() {
                page.dpi.clamp(1.0, 4800.0)
            } else {
                150.0
            };
            let px_per_fig = dpi / 1200.0 * scale;
            // genbitmaps sizes the image as `(int)(size + 0.9)`.
            let (w, h) = raster::checked_size(
                (bounds.width() * px_per_fig + 0.9).floor(),
                (bounds.height() * px_per_fig + 0.9).floor(),
            )?;
            let smoothing = match page.smoothing {
                None => raster::Smoothing::Antialias,
                Some(0 | 1) => raster::Smoothing::None,
                Some(k) => raster::Smoothing::Supersample(k),
            };
            let pm = raster::render_sized(
                scene,
                bounds,
                px_per_fig,
                (w, h),
                bg,
                pattern::PatternFrame::export(bounds, scale),
                smoothing,
            )?;
            if format == ExportFormat::Png && page.opaque {
                let rgb = bitmaps::rgb_over_white(&pm);
                let mut rgba = Vec::with_capacity(rgb.len() / 3 * 4);
                for p in rgb.as_chunks::<3>().0 {
                    rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
                }
                raster::encode_rgba_png(pm.width(), pm.height(), &rgba)?
            } else if format == ExportFormat::Png {
                raster::encode_png(&pm)?
            } else {
                bitmaps::encode(
                    format,
                    &pm,
                    &bitmaps::Encoding {
                        quality: page.quality.unwrap_or(bitmaps::DEFAULT_JPEG_QUALITY),
                        transparent: page.transparent,
                        dpi,
                    },
                )?
            }
        }
    };
    Ok((bytes, notes))
}
