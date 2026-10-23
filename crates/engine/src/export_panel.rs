//! The Export panel's options (W10-4, W10-11): which rows xfig shows for a
//! language, the values it keeps, and how they become fig2dev's options.
//!
//! xfig shows only the rows a language can use (`manage_optional`) and hands
//! them to fig2dev as command-line options (`print_export`); the paper size,
//! orientation, justification and single/multiple pages are the figure's own
//! settings (stored in the .fig file); everything else is kept per language
//! in the settings file (`[rustfig.export.<language>]`), never in the figure.
//!
// Ported from xfig src/w_export.c, src/u_print.c, src/f_util.c (xfig licence, see LICENSE-xfig).

use crate::{Engine, EngineError};
use rustfig_geom::Rect;
use rustfig_model::{ColorRef, PaperSize, Units};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// xfig's `COLOR_NONE`: no background colour chosen.
pub const COLOR_NONE: i32 = -1;
/// xfig's `TRANSP_NONE` / `TRANSP_BACKGROUND` for the GIF transparent colour.
pub const TRANSP_NONE: i32 = -2;
pub const TRANSP_BACKGROUND: i32 = -3;
/// The most a grid, border or offset text may be (sanity cap).
const MAX_NUMBER: f64 = 1.0e6;

/// The values of the panel kept for a language.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelSettings {
    /// Border around the figure: points (1/72 inch), pixels for bitmaps.
    pub border: f64,
    /// Background colour number, `COLOR_NONE` for none.
    pub background: i32,
    /// Minor and major grid spacing as the menu gives them ("None", "1/4",
    /// "10" …), in the figure's unit (inches or mm).
    pub grid_minor: String,
    pub grid_major: String,
    /// EPS/PS preview: 0 none, 1 ASCII (EPSI), 2 colour TIFF, 3 monochrome.
    pub preview: u8,
    /// PDF: a full page of the paper size instead of a figure-sized one.
    pub pdf_page_mode: bool,
    /// Multiple pages overlap.
    pub overlap: bool,
    /// Offsets on the page, in the unit chosen (0 in, 1 cm, 2 fig units).
    pub x_offset: f64,
    pub y_offset: f64,
    pub x_unit: u8,
    pub y_unit: u8,
    /// GIF transparent colour: a colour number, `TRANSP_NONE` or `TRANSP_BACKGROUND`.
    pub transparent: i32,
    /// Bitmaps: resolution (RustFig's addition), smoothing (fig2dev's `-S`:
    /// 1 none, 2 some, 4 more) and JPEG quality.
    pub dpi: f64,
    pub smoothing: Option<u32>,
    pub quality: u32,
    /// PDF, SVG, PNG: store the figure's Fig text in the file (plan E16).
    pub embed: bool,
}

impl Default for PanelSettings {
    /// What RustFig starts with: xfig's defaults, except that bitmaps are
    /// smoothed (the closest to the antialiased canvas) and have a
    /// resolution.
    fn default() -> Self {
        PanelSettings {
            smoothing: Some(4),
            ..PanelSettings::xfig()
        }
    }
}

impl PanelSettings {
    /// xfig's own defaults ("Reset to xfig defaults").
    pub fn xfig() -> PanelSettings {
        PanelSettings {
            border: 0.0,
            background: COLOR_NONE,
            grid_minor: "None".into(),
            grid_major: "None".into(),
            preview: 0,
            pdf_page_mode: false,
            overlap: true,
            x_offset: 0.0,
            y_offset: 0.0,
            x_unit: 0,
            y_unit: 0,
            transparent: TRANSP_NONE,
            dpi: 150.0,
            smoothing: Some(1),
            quality: 75,
            embed: false,
        }
    }

    /// Clamp what a file or a command gave to what the panel can show.
    pub fn sanitized(mut self) -> PanelSettings {
        let fin = |v: f64, lo: f64, hi: f64| if v.is_finite() { v.clamp(lo, hi) } else { lo.max(0.0) };
        self.border = fin(self.border, 0.0, MAX_NUMBER);
        self.x_offset = fin(self.x_offset, -MAX_NUMBER, MAX_NUMBER);
        self.y_offset = fin(self.y_offset, -MAX_NUMBER, MAX_NUMBER);
        self.dpi = if self.dpi.is_finite() {
            self.dpi.clamp(10.0, 2400.0)
        } else {
            150.0
        };
        self.quality = self.quality.clamp(1, 100);
        self.preview = self.preview.min(3);
        self.x_unit = self.x_unit.min(2);
        self.y_unit = self.y_unit.min(2);
        if !(COLOR_NONE..=2047).contains(&self.background) {
            self.background = COLOR_NONE;
        }
        if !(TRANSP_BACKGROUND..=2047).contains(&self.transparent) {
            self.transparent = TRANSP_NONE;
        }
        if !matches!(self.smoothing, None | Some(1) | Some(2) | Some(4)) {
            self.smoothing = Some(1);
        }
        for g in [&mut self.grid_minor, &mut self.grid_major] {
            if g.len() > 16
                || g.chars().any(|c| !(c.is_ascii_digit() || c == '.' || c == '/')) && !g.eq_ignore_ascii_case("none")
            {
                *g = "None".into();
            }
        }
        self
    }
}

/// The rows of the panel for a language (xfig's `manage_optional`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Rows {
    /// GIF: transparent colour.
    pub transparent: bool,
    /// JPEG: image quality.
    pub quality: bool,
    /// PDF, SVG, PNG: "Embed the figure" (plan E16).
    pub embed: bool,
    pub border: bool,
    /// The border is in pixels (bitmaps), else points.
    pub border_pixels: bool,
    pub background: bool,
    pub grid: bool,
    /// EPS/PS: preview.
    pub preview: bool,
    /// PDF: page mode.
    pub pdf_page_mode: bool,
    /// Paper size, orientation, justification and offsets: PostScript, PDF
    /// (greyed unless in page mode) and HPGL.
    pub page: bool,
    /// Single/multiple pages and overlap (PostScript and PDF).
    pub multiple: bool,
    /// Bitmaps: smoothing and resolution.
    pub bitmap: bool,
}

/// The rows of language `id`.
pub fn rows(id: &str) -> Rows {
    let bitmap = matches!(
        id,
        "gif" | "jpeg" | "pcx" | "png" | "ppm" | "sld" | "tiff" | "xbm" | "xpm"
    );
    let ps = matches!(id, "ps" | "eps" | "pdf" | "pstex" | "pspdf" | "pdftex" | "pspdftex");
    Rows {
        transparent: id == "gif",
        quality: id == "jpeg",
        embed: rustfig_format::embed::can_embed(id),
        border: ps || bitmap || matches!(id, "pict2e" | "tikz" | "map"),
        border_pixels: bitmap,
        background: ps || bitmap || id == "tk",
        grid: ps || bitmap,
        preview: matches!(id, "ps" | "eps"),
        pdf_page_mode: id == "pdf",
        page: matches!(id, "ps" | "pdf"),
        multiple: matches!(id, "ps" | "pdf"),
        bitmap,
    }
}

/// The settings kept for `id` (the defaults when none).
pub fn settings_for(e: &Engine, id: &str) -> PanelSettings {
    // Without kept values, xfig's `export_margin` resource gives the border.
    e.prefs
        .rustfig
        .export
        .get(id)
        .cloned()
        .unwrap_or_else(|| PanelSettings {
            border: e.prefs.int("export_margin").map_or(0.0, |m| m.clamp(0, 10_000) as f64),
            ..PanelSettings::default()
        })
        .sanitized()
}

/// Keep `s` for `id`; the settings file is written when the engine's
/// preferences are next saved.
pub fn store(e: &mut Engine, id: &str, s: PanelSettings) {
    let s = s.sanitized();
    if e.prefs.rustfig.export.get(id) != Some(&s) {
        e.prefs.rustfig.export.insert(id.to_string(), s);
        e.prefs_dirty = true;
    }
}

/// xfig's grid spec for fig2dev: `minor:major` with the unit, or `None`.
pub fn grid_spec(minor: &str, major: &str, units: Units) -> Option<String> {
    let clean = |g: &str| {
        let g = g.trim();
        if g.eq_ignore_ascii_case("none") { "" } else { g }.to_string()
    };
    let (minor, major) = (clean(minor), clean(major));
    let mut spec = minor;
    if !major.is_empty() {
        spec.push(':');
        spec.push_str(&major);
    }
    if spec.is_empty() {
        return None;
    }
    spec.push_str(if units == Units::Metric { "mm" } else { "in" });
    Some(spec)
}

/// An offset in points (1/72 inch) for fig2dev, from the panel's unit
/// (xfig's `offset_unit_conv`).
pub fn offset_points(value: f64, unit: u8) -> i64 {
    let conv = match unit {
        0 => 72.0,
        1 => 72.0 / 2.54,
        _ => 72.0 / 1200.0,
    };
    let v = (value * conv).round();
    if v.is_finite() {
        v.clamp(-1.0e9, 1.0e9) as i64
    } else {
        0
    }
}

fn hex(e: &Engine, n: i32) -> String {
    let c = e.doc().rgb(ColorRef(n));
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// The `file.export` parameters (the export's `options`, `background`,
/// `dpi`) for language `id` from the panel's values, as xfig's
/// `print_export` builds fig2dev's command line. The figure's paper size,
/// orientation, justification and single/multiple pages come from the
/// figure itself; fig2dev reads them from its header.
pub fn params(e: &Engine, id: &str, s: &PanelSettings) -> Map<String, Value> {
    let r = rows(id);
    let units = e.doc().settings.units;
    let mut options = Map::new();
    let mut out = Map::new();
    if r.border && s.border > 0.0 {
        options.insert("b".into(), json!(s.border));
    }
    if r.background && s.background != COLOR_NONE {
        out.insert("background".into(), json!(hex(e, s.background)));
    }
    if r.grid {
        if let Some(g) = grid_spec(&s.grid_minor, &s.grid_major, units) {
            options.insert("G".into(), json!(g));
        }
    }
    match (id, s.preview) {
        ("ps" | "eps", 1) => {
            options.insert("A".into(), json!(true));
        }
        ("ps" | "eps", 2) => {
            options.insert("C".into(), json!("x"));
        }
        ("ps" | "eps", 3) => {
            options.insert("T".into(), json!(true));
        }
        _ => {}
    }
    let page_mode = id == "ps" || (id == "pdf" && s.pdf_page_mode);
    if id == "pdf" && s.pdf_page_mode {
        options.insert("P".into(), json!(true));
    }
    if page_mode {
        let (x, y) = (offset_points(s.x_offset, s.x_unit), offset_points(s.y_offset, s.y_unit));
        if x != 0 {
            options.insert("x".into(), json!(x));
        }
        if y != 0 {
            options.insert("y".into(), json!(y));
        }
        if e.doc().settings.multiple_pages {
            options.insert("M".into(), json!(true));
            if s.overlap {
                options.insert("O".into(), json!(true));
            }
        }
    }
    if r.bitmap {
        out.insert("dpi".into(), json!(s.dpi));
        if let Some(sm) = s.smoothing {
            options.insert("smoothing".into(), json!(sm));
        }
    }
    if r.embed && s.embed {
        out.insert("embed".into(), json!(true));
    }
    if r.quality {
        options.insert("quality".into(), json!(s.quality));
    }
    if r.transparent && s.transparent != TRANSP_NONE {
        // "Background" makes the canvas colour (white) transparent and the
        // background that colour too.
        let color = if s.transparent == TRANSP_BACKGROUND {
            "#ffffff".to_string()
        } else {
            hex(e, s.transparent)
        };
        if s.transparent == TRANSP_BACKGROUND {
            out.insert("background".into(), json!(color));
        }
        options.insert("t".into(), json!(color));
    }
    if !options.is_empty() {
        out.insert("options".into(), Value::Object(options));
    }
    out
}

/// `p` (a `file.export` parameter object) with the panel's settings added
/// for `"panel": true` (the stored ones) or `"panel": {…}` (these): what
/// `p` gives itself wins.
pub fn apply(e: &Engine, p: &Value, id: &str) -> Result<Value, EngineError> {
    let s = match p.get("panel") {
        None | Some(Value::Null) | Some(Value::Bool(false)) => return Ok(p.clone()),
        Some(Value::Bool(true)) => settings_for(e, id),
        Some(v @ Value::Object(_)) => merge(settings_for(e, id), v)?,
        Some(_) => return Err(EngineError::bad_params("panel must be true or an object")),
    };
    let mut out = p.clone();
    let Some(obj) = out.as_object_mut() else {
        return Ok(p.clone());
    };
    obj.remove("panel");
    for (k, v) in params(e, id, &s) {
        match (k.as_str(), obj.get_mut(&k)) {
            ("options", Some(Value::Object(mine))) => {
                if let Value::Object(theirs) = v {
                    for (ok, ov) in theirs {
                        mine.entry(ok).or_insert(ov);
                    }
                }
            }
            (_, Some(_)) => {}
            _ => {
                obj.insert(k, v);
            }
        }
    }
    Ok(out)
}

/// `base` with the fields present in `v` replaced; unknown keys and wrong
/// types are errors.
pub fn merge(base: PanelSettings, v: &Value) -> Result<PanelSettings, EngineError> {
    let Value::Object(m) = v else {
        return Err(EngineError::bad_params("settings must be an object"));
    };
    let mut cur = serde_json::to_value(&base).map_err(|err| EngineError::bad_params(err.to_string()))?;
    if let Some(cm) = cur.as_object_mut() {
        for (k, val) in m {
            if !cm.contains_key(k) && k != "smoothing" {
                return Err(EngineError::bad_params(format!("unknown export setting {k:?}")));
            }
            cm.insert(k.clone(), val.clone());
        }
    }
    serde_json::from_value::<PanelSettings>(cur)
        .map(PanelSettings::sanitized)
        .map_err(|err| EngineError::bad_params(format!("export settings: {err}")))
}

/// The figure's size in the current unit (inches or cm) at `magnification`
/// percent, and the unit's name (xfig's `export_update_figure_size`).
pub fn figure_size(bounds: &Rect, units: Units, magnification: f64) -> (f64, f64, &'static str) {
    let (mult, unit) = match units {
        Units::Inches => (1200.0, "in"),
        Units::Metric => (450.0, "cm"),
    };
    if bounds.is_empty() {
        return (0.0, 0.0, unit);
    }
    (
        bounds.width() / mult * magnification / 100.0,
        bounds.height() / mult * magnification / 100.0,
        unit,
    )
}

/// The magnification that fits the figure on the paper, a half inch of
/// margin all round (xfig's `fit_page`); `None` for an empty figure.
pub fn fit_page(bounds: &Rect, paper: PaperSize, landscape: bool, units: Units) -> Option<f64> {
    if bounds.is_empty() || bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return None;
    }
    let (w, h) = paper.inches();
    let (mut pw, mut ph) = (w * 1200.0 - 1200.0, h * 1200.0 - 1200.0);
    if landscape {
        std::mem::swap(&mut pw, &mut ph);
    }
    let (fw, fh) = (bounds.width(), bounds.height());
    let mut m = if pw / fw < ph / fh {
        100.0 * pw / fw
    } else {
        100.0 * ph / fh
    };
    if units == Units::Metric {
        m *= 450.0 * 2.54 / 1200.0;
    }
    (m.is_finite() && m > 0.0).then_some(m.clamp(0.1, 100_000.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_spec_like_xfig() {
        assert_eq!(grid_spec("None", "None", Units::Inches), None);
        assert_eq!(grid_spec("1/4", "None", Units::Inches).as_deref(), Some("1/4in"));
        assert_eq!(grid_spec("None", "1", Units::Metric).as_deref(), Some(":1mm"));
        assert_eq!(grid_spec("2", "10", Units::Metric).as_deref(), Some("2:10mm"));
    }

    #[test]
    fn offsets_in_points() {
        assert_eq!(offset_points(1.0, 0), 72);
        assert_eq!(offset_points(2.54, 1), 72);
        assert_eq!(offset_points(1200.0, 2), 72);
        assert_eq!(offset_points(f64::NAN, 0), 0);
    }

    #[test]
    fn rows_per_language() {
        assert!(rows("gif").transparent && !rows("png").transparent);
        assert!(rows("jpeg").quality && rows("png").bitmap && rows("png").border_pixels);
        assert!(rows("pdf").pdf_page_mode && rows("pdf").multiple && !rows("eps").multiple);
        assert!(rows("eps").preview && !rows("pdf").preview);
        assert!(rows("tk").background && !rows("tk").grid && !rows("svg").background);
        assert!(!rows("box").border);
    }

    #[test]
    fn fit_page_letter_landscape() {
        let b = Rect::new(
            rustfig_geom::Point::new(0.0, 0.0),
            rustfig_geom::Point::new(12000.0, 6000.0),
        );
        // 10.5 in x 8 in usable (landscape) over 10 in x 5 in: width wins? 10.5/10 = 1.05, 7.5/5 = 1.5
        let m = fit_page(&b, PaperSize::Letter, true, Units::Inches).unwrap_or(0.0);
        assert!((m - 100.0 * (11.0 - 1.0) / 10.0).abs() < 1e-9, "{m}");
    }
}
