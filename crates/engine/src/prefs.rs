//! Preferences that survive restarts (plan E1), stored as TOML.
//!
//! The `[xfig]` table holds xfig's own resources under xfig's names (the
//! table in `xfig.man.in`, `app-defaults/Fig`): a settings file reads like
//! an X resource file, and xfig's command-line options (S3) set the same
//! keys for one run. `[rustfig]` holds RustFig's switches, recent files and
//! the user's colours. Unknown keys are kept, so a newer file survives an
//! older RustFig. The engine applies the drawing-related resources; the UI
//! reads its own (panels, balloons, export panel).
//
// Ported from xfig src/main.c (`application_resources`, `options`) and
// doc/xfig.man.in (the resource table) (xfig licence, see LICENSE-xfig).

use crate::{Engine, SnapMode};
use rustfig_model::{Justify, Orientation, PaperSize, Units};
use serde::{Deserialize, Serialize};

/// A table of preference values (TOML).
pub use toml::Table as TomlTable;
/// A preference value (TOML).
pub use toml::Value as TomlValue;

/// The settings file in the platform's config folder (native).
#[cfg(not(target_arch = "wasm32"))]
pub fn config_path() -> Option<std::path::PathBuf> {
    use crate::dirs::{config_dir, process_env, HostOs};
    // Scripts, tests and agents keep their settings apart from the user's.
    if let Some(d) = process_env("RUSTFIG_SETTINGS_DIR") {
        return Some(d.join("settings.toml"));
    }
    config_dir(HostOs::current(), &process_env).map(|d| d.join("settings.toml"))
}

/// Recently used colours kept (E4).
pub const MAX_RECENT_COLORS: usize = 12;

/// Most recent files kept (xfig's `MAX_RECENT_FILES`).
pub const MAX_RECENT: usize = 9;
/// Most user colours kept.
pub const MAX_USER_COLORS: usize = 512;

/// The type of a resource's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Bool,
    Int,
    Float,
    Str,
}

/// What RustFig does with a resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Applied at start.
    Applied,
    /// Kept; it takes effect once the feature exists (plan task).
    Planned(&'static str),
}

/// xfig resources with no counterpart in RustFig (X11 visuals, colormaps,
/// fonts and toolkit timing, or programs RustFig replaced): not offered as
/// preferences, dropped quietly from settings files; xfig's options for
/// them are accepted and ignored on the command line (S3).
pub const NOT_APPLICABLE: &[&str] = &[
    "buttonFont",
    "debug",
    "depth",
    "dontswitchcmap",
    "ghostscript",
    "iconGeometry",
    "installowncmap",
    "internalborderwidth",
    "international",
    "monochrome",
    "normalFont",
    "spinner_delay",
    "spinner_rate",
    "splash",
    "tablet",
    "visual",
];

/// xfig's default for the helper programs (image editor, browser, PDF
/// viewer): the system's own opener, named as on this system.
pub const SYSTEM_OPEN: &str = if cfg!(target_os = "macos") {
    "open"
} else if cfg!(windows) {
    "start"
} else {
    "xdg-open"
};

/// One of xfig's resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resource {
    pub name: &'static str,
    pub kind: Kind,
    /// xfig's default, as its man page gives it.
    pub default: &'static str,
    /// xfig's command-line option(s) setting it.
    pub option: &'static str,
    pub status: Status,
}

const fn r(name: &'static str, kind: Kind, default: &'static str, option: &'static str, status: Status) -> Resource {
    Resource {
        name,
        kind,
        default,
        option,
        status,
    }
}

use Kind::{Bool, Float, Int, Str};
use Status::{Applied, Planned};

/// xfig's resources (`xfig.man.in`'s table and `main.c`), in the man page's order.
#[rustfmt::skip]
pub const RESOURCES: &[Resource] = &[
    r("allownegcoords", Bool, "true", "-allownegcoords, -dontallownegcoords", Applied),
    r("autorefresh", Bool, "false", "-autorefresh", Applied),
    r("axislines", Str, "pink", "-axislines", Planned("E9")),
    r("balloon_delay", Int, "500", "-balloon_delay", Applied),
    r("but_per_row", Int, "2", "-but_per_row", Planned("E9")),
    r("browser", Str, SYSTEM_OPEN, "-browser", Planned("D11")),
    r("canvasbackground", Str, "white", "-cbg", Planned("E9")),
    r("canvasforeground", Str, "black", "-cfg", Planned("E9")),
    r("correctfontsize", Bool, "false", "-correctfontsize", Applied),
    r("crosshair", Bool, "false", "-crosshair", Planned("E9")),
    r("encoding", Int, "1", "-encoding", Planned("D11")),
    r("exportLanguage", Str, "eps", "-exportLanguage", Applied),
    r("export_margin", Int, "0", "-export_margin", Applied),
    r("flipvisualhints", Bool, "false", "-flipvisualhints", Planned("E9")),
    r("flushleft", Bool, "false", "-flushleft, -center", Applied),
    r("freehand_resolution", Int, "25", "-freehand_resolution", Applied),
    r("geometry", Str, "", "-geometry", Applied),
    r("grid_color", Str, "black", "-grid_color", Planned("E9")),
    r("grid_unit", Str, "1/16", "-grid_unit", Planned("E9")),
    r("hiddentext", Bool, "false", "-hiddentext", Applied),
    r("icon_view", Bool, "true", "-icon_view, -list_view", Planned("D6")),
    r("image_editor", Str, SYSTEM_OPEN, "-image_editor", Applied),
    r("inches", Bool, "true", "-inches, -imperial, -metric, -centimeters", Applied),
    r("jpeg_quality", Int, "75", "-jpeg_quality", Planned("B6")),
    r("justify", Bool, "false", "-left, -right", Applied),
    r("keyFile", Str, "CompKeyDB", "-keyFile", Planned("D11")),
    r("landscape", Bool, "true", "-landscape, -portrait", Applied),
    r("latexfonts", Bool, "false", "-latexfonts", Applied),
    r("library_dir", Str, "~/xfiglib", "-library_dir", Planned("D6")),
    r("max_image_colors", Int, "0", "-max_image_colors", Planned("B6")),
    r("max_recent_files", Int, "9", "", Applied),
    r("magnification", Float, "100", "-magnification", Applied),
    r("multiple", Bool, "false", "-multiple, -single", Applied),
    r("pdf_viewer", Str, SYSTEM_OPEN, "-pdf_viewer", Planned("D11")),
    r("overlap", Bool, "true", "-overlap, -nooverlap", Planned("B4")),
    r("pageborder", Str, "lightblue", "-pageborder", Planned("E9")),
    r("paper_size", Str, "Letter", "-paper_size", Applied),
    r("pheight", Float, "8.5", "-pheight", Planned("B4")),
    r("pwidth", Float, "11", "-pwidth", Planned("B4")),
    r("rigidtext", Bool, "false", "-rigidtext", Applied),
    r("rulerthick", Int, "24", "-rulerthick", Planned("E9")),
    r("scale_factor", Float, "1.0", "-scale_factor", Planned("D11")),
    r("showallbuttons", Bool, "false", "-showallbuttons", Planned("E9")),
    r("showaxislines", Bool, "true", "-showaxislines, -dontshowaxislines", Applied),
    r("showballoons", Bool, "true", "-showballoons, -dontshowballoons", Applied),
    r("showdepthmanager", Bool, "true", "-showdepthmanager, -dontshowdepthmanager", Applied),
    r("showlengths", Bool, "false", "-showlengths, -dontshowlengths", Applied),
    r("shownums", Bool, "false", "-shownums, -dontshownums", Applied),
    r("showpageborder", Bool, "true", "-showpageborder, -dontshowpageborder", Applied),
    r("save_versions", Int, "5", "-save_versions", Applied),
    r("smallicons", Bool, "false", "-smallicons", Planned("E9")),
    r("smooth_factor", Int, "0", "-smooth_factor", Planned("B6")),
    r("specialtext", Bool, "false", "-specialtext", Applied),
    r("spellcheckcommand", Str, "spell %s", "-spellcheckcommand", Planned("D11")),
    r("startarrowlength", Float, "8", "-startarrowlength", Applied),
    r("startarrowthick", Float, "1", "-startarrowthick", Applied),
    r("startarrowtype", Int, "0", "-startarrowtype", Applied),
    r("startarrowwidth", Float, "4", "-startarrowwidth", Applied),
    r("startfillstyle", Int, "-1", "-startfillstyle", Applied),
    r("startfontsize", Float, "12", "-startfontsize", Applied),
    r("startgridmode", Int, "0", "-startgridmode", Applied),
    r("startlatexFont", Str, "Default", "-startlatexFont", Applied),
    r("startlinewidth", Int, "1", "-startlinewidth", Applied),
    r("startposnmode", Int, "1", "-startposnmode", Applied),
    r("startpsFont", Str, "Times-Roman", "-startpsFont", Applied),
    r("starttextstep", Float, "1.0", "-starttextstep", Applied),
    r("trackCursor", Bool, "true", "-track, -notrack", Applied),
    r("transparent_color", Int, "-2", "-transparent_color", Planned("B6")),
    r("userscale", Float, "1.0", "-userscale", Applied),
    r("userunit", Str, "in", "-userunit", Applied),
    r("write_bak", Bool, "true", "-write_bak", Applied),
    r("zoom", Float, "1.0", "-zoom", Applied),
];

/// The resource `name` (xfig's spelling; case ignored, as X resources are
/// commonly written either way).
pub fn resource(name: &str) -> Option<&'static Resource> {
    RESOURCES.iter().find(|r| r.name.eq_ignore_ascii_case(name))
}

/// A colour the user keeps for all drawings (E1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedColor {
    pub name: String,
    /// `#rrggbb`.
    pub rgb: String,
}

/// RustFig's own settings.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RustFigPrefs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xfig_keys: Option<bool>,
    /// "3d" (bevelled buttons, default) or "flat".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button_style: Option<String>,
    /// E9: "system" (default), "light" or "dark".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// E9: UI scale, 1.0 = 100 % (0.75 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_scale: Option<f32>,
    /// Arrow Size panel (W10-31): absolute values or multiples of the line width,
    /// and the multiples (thickness, width, length).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrow_absolute: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arrow_mult: Option<[f64; 3]>,
    pub recent_files: Vec<String>,
    pub user_colors: Vec<NamedColor>,
    /// Colours used lately (`#rrggbb`, newest first), offered in the colour pop-ups (E4).
    pub recent_colors: Vec<String>,
    /// Style families of named attribute sets (E6), kept for all drawings.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub style_families: Vec<crate::styles::StyleFamily>,
    /// The Export panel's values per language id (W10-11), kept across
    /// sessions; never in the drawing file.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub export: std::collections::BTreeMap<String, crate::export_panel::PanelSettings>,
    /// Keys this version doesn't know, kept as they are.
    #[serde(flatten)]
    pub other: toml::Table,
}

/// All preferences.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// xfig's resources by name.
    pub xfig: toml::Table,
    pub rustfig: RustFigPrefs,
    #[serde(flatten)]
    pub other: toml::Table,
}

/// `[rustfig]` keys of switches that no longer exist.
const RETIRED: [&str; 4] = ["layout", "mouse", "show_attribute_panel", "vector_icons"];

const HEADER: &str = "# RustFig settings.
# [xfig] holds xfig's resources under xfig's names (see xfig(1)); xfig's
# command-line options override them for one run. [rustfig] holds RustFig's
# own switches, recent files and colours. Edit freely; unknown keys are kept.

";

impl Prefs {
    /// Read a settings file. A damaged file is an error; the caller falls
    /// back to the defaults and reports it. Unknown xfig resources and
    /// values of the wrong type give warnings, not errors.
    pub fn parse(text: &str) -> Result<(Prefs, Vec<String>), String> {
        let mut p: Prefs = toml::from_str(text).map_err(|e| format!("settings file: {e}"))?;
        p.xfig
            .retain(|k, _| !NOT_APPLICABLE.iter().any(|n| n.eq_ignore_ascii_case(k)));
        let mut warnings = Vec::new();
        for (k, v) in &p.xfig {
            match resource(k) {
                None => warnings.push(format!("settings: unknown xfig resource {k:?} (kept)")),
                Some(r) if !type_ok(r.kind, v) => {
                    warnings.push(format!("settings: {k} should be {:?}, not {v} (ignored)", r.kind))
                }
                Some(_) => {}
            }
        }
        // Switches removed in W16/W17 (layout, mouse scheme, attribute-panel
        // switch, icon choice): read and ignored, not written back.
        p.rustfig.other.retain(|k, _| !RETIRED.contains(&k));
        p.rustfig.recent_files.truncate(MAX_RECENT);
        p.rustfig.user_colors.truncate(MAX_USER_COLORS);
        p.rustfig.recent_colors.truncate(MAX_RECENT_COLORS);
        Ok((p, warnings))
    }

    /// The settings file's text.
    pub fn to_toml(&self) -> String {
        match toml::to_string_pretty(self) {
            Ok(t) => format!("{HEADER}{t}"),
            Err(_) => HEADER.to_string(), // a table always serializes; keep the header if not
        }
    }

    /// Set xfig resource `name` (validated by the catalogue's type).
    pub fn set(&mut self, name: &str, value: toml::Value) -> Result<(), String> {
        let r = resource(name).ok_or_else(|| format!("unknown xfig resource {name:?}"))?;
        let value = coerce(r.kind, value).ok_or_else(|| format!("{} takes a {:?} value", r.name, r.kind))?;
        self.xfig.insert(r.name.to_string(), value);
        Ok(())
    }

    /// The value of resource `name`, if set and of its type.
    pub fn get(&self, name: &str) -> Option<toml::Value> {
        let r = resource(name)?;
        let v = self
            .xfig
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(r.name))
            .map(|(_, v)| v.clone())?;
        coerce(r.kind, v)
    }

    pub fn bool(&self, name: &str) -> Option<bool> {
        self.get(name).and_then(|v| v.as_bool())
    }

    pub fn float(&self, name: &str) -> Option<f64> {
        self.get(name).and_then(|v| v.as_float()).filter(|f| f.is_finite())
    }

    pub fn int(&self, name: &str) -> Option<i64> {
        self.get(name).and_then(|v| v.as_integer())
    }

    pub fn str(&self, name: &str) -> Option<String> {
        self.get(name).and_then(|v| v.as_str().map(str::to_string))
    }

    /// Note `rgb` as the most recently used colour.
    /// How many recently used files are kept (xfig's `max_recent_files`, 0 to 9).
    pub fn recent_max(&self) -> usize {
        self.int("max_recent_files")
            .map_or(MAX_RECENT, |n| usize::try_from(n).unwrap_or(0).min(MAX_RECENT))
    }

    pub fn add_recent_color(&mut self, rgb: &str) {
        let r = &mut self.rustfig.recent_colors;
        r.retain(|c| c != rgb);
        r.insert(0, rgb.to_string());
        r.truncate(MAX_RECENT_COLORS);
    }

    /// Note `path` as the most recent file.
    pub fn add_recent(&mut self, path: &str) {
        let max = self.recent_max();
        let r = &mut self.rustfig.recent_files;
        r.retain(|p| p != path);
        r.insert(0, path.to_string());
        r.truncate(max);
    }

    /// xfig's "start" resources from the current attributes (Edit ›
    /// Global settings › Use current attributes as defaults).
    pub fn take_attributes(&mut self, e: &Engine) {
        let ind = &e.indicator;
        let mut set = |k: &str, v: toml::Value| {
            self.xfig.insert(k.to_string(), v);
        };
        set("startfontsize", toml::Value::Float(ind.font_size));
        set("startlinewidth", toml::Value::Integer(i64::from(ind.thickness)));
        set("startfillstyle", toml::Value::Integer(i64::from(ind.area_fill)));
        set("starttextstep", toml::Value::Float(ind.text_step));
        set(
            "startposnmode",
            toml::Value::Integer(SnapMode::ALL.iter().position(|m| *m == ind.snap).unwrap_or(1) as i64),
        );
        let grid = match ind.grid {
            None => 0,
            Some(m) => SnapMode::ALL
                .iter()
                .position(|x| *x == m)
                .map_or(0, |i| i.saturating_sub(1)),
        };
        let grid = if ind.grid_iso && grid > 0 { grid + 4 } else { grid };
        set("startgridmode", toml::Value::Integer(grid as i64));
        if ind.postscript {
            set("latexfonts", toml::Value::Boolean(false));
            set(
                "startpsFont",
                toml::Value::String(rustfig_text::ps_font_name(ind.font).to_string()),
            );
        } else {
            set("latexfonts", toml::Value::Boolean(true));
            set(
                "startlatexFont",
                toml::Value::String(rustfig_text::latex_font_name(ind.font).to_string()),
            );
        }
        set("specialtext", toml::Value::Boolean(ind.special_text));
        set("hiddentext", toml::Value::Boolean(ind.hidden_text));
        set("rigidtext", toml::Value::Boolean(ind.rigid_text));
        set("justify", toml::Value::Boolean(ind.justify == Justify::Right));
        let unit = rustfig_geom::FIG_PER_STYLE_UNIT;
        set(
            "startarrowtype",
            toml::Value::Integer(i64::from(crate::arrow_choice_of(ind.arrow.kind, ind.arrow.style))),
        );
        set("startarrowthick", toml::Value::Float(ind.arrow.thickness));
        set("startarrowwidth", toml::Value::Float(ind.arrow.width / unit));
        set("startarrowlength", toml::Value::Float(ind.arrow.height / unit));
        self.rustfig.arrow_absolute = Some(ind.arrow_absolute);
        self.rustfig.arrow_mult = Some([ind.arrow_mult.0, ind.arrow_mult.1, ind.arrow_mult.2]);
    }

    /// Apply the drawing-related resources that are set; returns warnings
    /// for values RustFig can't use.
    pub fn apply(&self, e: &mut Engine) -> Vec<String> {
        let mut warn = Vec::new();
        // The drawing's settings (for the empty start drawing and, as
        // file.new keeps them, every new one).
        let mut s = e.doc().settings.clone();
        if let Some(i) = self.bool("inches") {
            s.units = if i { Units::Inches } else { Units::Metric };
        }
        if let Some(m) = self.float("magnification").filter(|m| (0.1..=100_000.0).contains(m)) {
            s.magnification = m;
        }
        if let Some(l) = self.bool("landscape") {
            s.orientation = if l {
                Orientation::Landscape
            } else {
                Orientation::Portrait
            };
        }
        if let Some(f) = self.bool("flushleft") {
            s.center = !f;
        }
        if let Some(m) = self.bool("multiple") {
            s.multiple_pages = m;
        }
        if let Some(p) = self.str("paper_size") {
            match PaperSize::from_name(&p) {
                Some(ps) => s.paper = ps,
                None => warn.push(format!("paper_size: unknown paper {p:?}")),
            }
        }
        if s != e.doc().settings && !e.is_modified() && e.doc().objects.is_empty() {
            e.doc_mut().settings = s;
            e.set_modified(false);
        }
        if let Some(z) = self.float("zoom").filter(|z| (0.01..=100.0).contains(z)) {
            e.view.zoom = z;
        }
        if let Some(sc) = self.float("userscale").filter(|v| *v > 0.0) {
            e.unit_scale.factor = sc;
        }
        if let Some(u) = self.str("userunit") {
            e.unit_scale.unit = u;
        }
        if let Some(b) = self.bool("showlengths") {
            e.show_lengths = b;
        }
        if let Some(b) = self.bool("showpageborder") {
            e.show_page_border = b;
        }
        // The attribute panel's starting values.
        let ind = &mut e.indicator;
        if let Some(f) = self.float("startfontsize").filter(|f| (1.0..=500.0).contains(f)) {
            ind.font_size = f;
        }
        if let Some(t) = self.int("startlinewidth").and_then(|t| i32::try_from(t).ok()) {
            ind.thickness = t.clamp(0, 500);
        }
        if let Some(f) = self.int("startfillstyle").and_then(|t| i32::try_from(t).ok()) {
            ind.area_fill = f.clamp(-1, 62);
        }
        if let Some(t) = self.float("starttextstep").filter(|t| (0.0..=100.0).contains(t)) {
            ind.text_step = t;
        }
        if let Some(n) = self.int("startposnmode") {
            match usize::try_from(n).ok().and_then(|i| SnapMode::ALL.get(i)) {
                Some(m) => ind.snap = *m,
                None => warn.push(format!("startposnmode: {n} is not 0-5")),
            }
        }
        if let Some(n) = self.int("startgridmode") {
            match n {
                0 => {
                    ind.grid = None;
                    ind.grid_iso = false;
                }
                1..=8 => {
                    ind.grid_iso = n > 4;
                    let k = if n > 4 { n - 4 } else { n };
                    ind.grid = usize::try_from(k + 1).ok().and_then(|i| SnapMode::ALL.get(i)).copied();
                }
                _ => warn.push(format!("startgridmode: {n} is not 0-8")),
            }
        }
        let latex = self.bool("latexfonts").unwrap_or(!ind.postscript);
        if latex {
            ind.postscript = false;
            if let Some(f) = self.str("startlatexFont") {
                match (0..rustfig_text::LATEX_FONT_COUNT)
                    .find(|i| rustfig_text::latex_font_name(*i).eq_ignore_ascii_case(&f))
                {
                    Some(i) => ind.font = i,
                    None => warn.push(format!("startlatexFont: unknown LaTeX font {f:?}")),
                }
            }
        } else {
            ind.postscript = true;
            if let Some(f) = self.str("startpsFont") {
                match rustfig_text::ps_font_number(&f) {
                    Some(i) => ind.font = i,
                    None => warn.push(format!("startpsFont: unknown PostScript font {f:?}")),
                }
            }
        }
        if let Some(b) = self.bool("specialtext") {
            ind.special_text = b;
        }
        if let Some(b) = self.bool("hiddentext") {
            ind.hidden_text = b;
        }
        if let Some(b) = self.bool("rigidtext") {
            ind.rigid_text = b;
        }
        if let Some(right) = self.bool("justify") {
            ind.justify = if right { Justify::Right } else { Justify::Left };
        }
        let unit = rustfig_geom::FIG_PER_STYLE_UNIT;
        // startarrowtype is a choice of xfig's arrow list (0 stick, then
        // each type hollow and filled).
        if let Some(n) = self.int("startarrowtype").and_then(|k| i32::try_from(k).ok()) {
            let (kind, style) = crate::arrow_choice(n.clamp(0, 28));
            ind.arrow.kind = kind;
            ind.arrow.style = style;
        }
        if let Some(t) = self.float("startarrowthick").filter(|t| (0.0..=100.0).contains(t)) {
            ind.arrow.thickness = t;
        }
        if let Some(w) = self.float("startarrowwidth").filter(|w| (0.0..=1000.0).contains(w)) {
            ind.arrow.width = w * unit;
        }
        if let Some(h) = self.float("startarrowlength").filter(|h| (0.0..=1000.0).contains(h)) {
            ind.arrow.height = h * unit;
        }
        if let Some(b) = self.rustfig.arrow_absolute {
            ind.arrow_absolute = b;
        }
        if let Some([t, w, h]) = self
            .rustfig
            .arrow_mult
            .filter(|m| m.iter().all(|v| (0.1..=10_000.0).contains(v)))
        {
            ind.arrow_mult = (t, w, h);
        }
        warn
    }
}

/// Whether `v` fits `kind` (integers also serve as floats).
fn type_ok(kind: Kind, v: &toml::Value) -> bool {
    coerce(kind, v.clone()).is_some()
}

/// `v` as a value of `kind`: integers widen to floats, and the strings
/// "true"/"false" and numbers in strings are read as X resources write them.
fn coerce(kind: Kind, v: toml::Value) -> Option<toml::Value> {
    use toml::Value as V;
    match (kind, v) {
        (Kind::Bool, V::Boolean(b)) => Some(V::Boolean(b)),
        (Kind::Bool, V::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "on" | "yes" | "1" => Some(V::Boolean(true)),
            "false" | "off" | "no" | "0" => Some(V::Boolean(false)),
            _ => None,
        },
        (Kind::Int, V::Integer(i)) => Some(V::Integer(i)),
        (Kind::Int, V::Float(f)) if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e15 => Some(V::Integer(f as i64)),
        (Kind::Int, V::String(s)) => s.trim().parse().ok().map(V::Integer),
        (Kind::Float, V::Float(f)) if f.is_finite() => Some(V::Float(f)),
        (Kind::Float, V::Integer(i)) => Some(V::Float(i as f64)),
        (Kind::Float, V::String(s)) => s.trim().parse::<f64>().ok().filter(|f| f.is_finite()).map(V::Float),
        (Kind::Str, V::String(s)) => Some(V::String(s)),
        (Kind::Str, V::Integer(i)) => Some(V::String(i.to_string())),
        (Kind::Str, V::Float(f)) => Some(V::String(f.to_string())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_is_unique_and_known() {
        for (i, a) in RESOURCES.iter().enumerate() {
            for b in &RESOURCES[i + 1..] {
                assert!(!a.name.eq_ignore_ascii_case(b.name), "{}", a.name);
            }
        }
        assert!(RESOURCES.len() >= 60);
        for n in NOT_APPLICABLE {
            assert!(resource(n).is_none(), "{n} is both offered and not applicable");
        }
        assert_eq!(resource("StartFontSize").map(|r| r.name), Some("startfontsize"));
    }

    #[test]
    fn round_trip_keeps_unknown_keys() {
        let text = "
[xfig]
startfontsize = 14
visual = \"PseudoColor\"
inches = \"false\"
futureResource = 3

[rustfig]
layout = \"modern\"
mouse = \"modern\"
vector_icons = false
recent_files = [\"/a.fig\"]
future_switch = true

[[rustfig.user_colors]]
name = \"Rust\"
rgb = \"#d2692a\"

[later]
x = 1
";
        let (p, warnings) = Prefs::parse(text).unwrap();
        assert_eq!(p.float("startfontsize"), Some(14.0));
        assert_eq!(p.bool("inches"), Some(false));
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        // Retired switches load without complaint and are not written back.
        assert!(!p.to_toml().contains("layout") && !p.to_toml().contains("vector_icons"));
        assert_eq!(p.rustfig.user_colors[0].rgb, "#d2692a");
        let again = Prefs::parse(&p.to_toml()).unwrap().0;
        assert_eq!(again, p);
        assert!(p.to_toml().contains("future_switch"));
        assert!(p.to_toml().contains("[later]"));
    }

    #[test]
    fn damaged_files_are_errors() {
        assert!(Prefs::parse("[xfig\nzoom = ").is_err());
        assert!(Prefs::parse("xfig = 3").is_err());
    }

    #[test]
    fn set_validates_by_type() {
        let mut p = Prefs::default();
        p.set("zoom", toml::Value::Integer(2)).unwrap();
        assert_eq!(p.float("zoom"), Some(2.0));
        assert!(p.set("zoom", toml::Value::Boolean(true)).is_err());
        assert!(p.set("nosuch", toml::Value::Integer(1)).is_err());
    }

    #[test]
    fn apply_sets_start_attributes_and_drawing_settings() {
        let mut p = Prefs::default();
        for (k, v) in [
            ("startfontsize", toml::Value::Float(18.0)),
            ("startposnmode", toml::Value::Integer(0)),
            ("startgridmode", toml::Value::Integer(2)),
            ("startpsFont", toml::Value::String("Helvetica".into())),
            ("inches", toml::Value::Boolean(false)),
            ("landscape", toml::Value::Boolean(false)),
            ("paper_size", toml::Value::String("A4".into())),
            ("zoom", toml::Value::Float(2.0)),
            ("justify", toml::Value::Boolean(true)),
        ] {
            p.set(k, v).unwrap();
        }
        let mut e = Engine::new();
        assert!(p.apply(&mut e).is_empty());
        assert_eq!(e.indicator.font_size, 18.0);
        assert_eq!(e.indicator.snap, SnapMode::Any);
        assert_eq!(e.indicator.grid, Some(SnapMode::ALL[3]));
        assert_eq!(e.indicator.font, 16);
        assert_eq!(e.indicator.justify, Justify::Right);
        assert_eq!(e.doc().settings.units, Units::Metric);
        assert_eq!(e.doc().settings.orientation, Orientation::Portrait);
        assert_eq!(e.doc().settings.paper, PaperSize::from_name("A4").unwrap());
        assert_eq!(e.view.zoom, 2.0);
        assert!(!e.is_modified());
        // And back: the current attributes become the start values.
        let mut q = Prefs::default();
        q.take_attributes(&e);
        assert_eq!(q.float("startfontsize"), Some(18.0));
        assert_eq!(q.int("startgridmode"), Some(2));
        assert_eq!(q.str("startpsFont").as_deref(), Some("Helvetica"));
    }

    #[test]
    fn recent_files_are_unique_newest_first_and_capped() {
        let mut p = Prefs::default();
        for i in 0..12 {
            p.add_recent(&format!("/f{i}.fig"));
        }
        p.add_recent("/f5.fig");
        assert_eq!(p.rustfig.recent_files.len(), MAX_RECENT);
        assert_eq!(p.rustfig.recent_files[0], "/f5.fig");
        assert_eq!(p.rustfig.recent_files.iter().filter(|f| *f == "/f5.fig").count(), 1);
    }
}
