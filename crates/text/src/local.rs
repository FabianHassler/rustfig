//! Installed fonts per text (plan F7): the faces of the system's font
//! database, and drawing and measuring a string in one of them by name.
//!
//! Fig 3.2 knows fonts only as numbers; a text that uses an installed font
//! keeps the nearest Fig font as its number and the font's name beside it
//! (see `rustfig_model::Text::local_font`). figbase picks faces by Fig
//! number only, so this module keeps its own `fontdb` database (system
//! fonts and the folders given to [`add_font_dirs`]) and lays glyphs out
//! the way figbase does (kerning from the `kern` table, 1.2 line spacing).
//! With the built-in fonts switched on (tests, wasm) the database is empty,
//! unless a test injects a family list with [`override_local_faces`].

use crate::{Generic, Metrics, PathOp, Shaped, FIG_PER_POINT, LINE_SPACING, MAX_CHARS};
use fontdb::{Database, Style, Weight, ID};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use ttf_parser::{Face, GlyphId, OutlineBuilder};

/// One face of an installed family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFace {
    pub family: String,
    /// `Regular`, `Bold`, `Italic`, `Semibold Italic`, ...
    pub label: String,
    /// What a text stores and the chooser shows: the family for the regular
    /// face, `Family Label` for the others.
    pub name: String,
    pub generic: Generic,
    pub bold: bool,
    pub italic: bool,
}

struct State {
    extra: Vec<PathBuf>,
    built: Option<Built>,
    injected: Option<Vec<LocalFace>>,
}

struct Built {
    builtin: bool,
    db: Database,
    faces: Vec<(LocalFace, ID)>,
}

static STATE: Mutex<State> = Mutex::new(State {
    extra: Vec::new(),
    built: None,
    injected: None,
});

fn lock() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|p| p.into_inner())
}

/// Add folders to search for fonts (also for figbase's lookup).
pub fn add_font_dirs(dirs: &[PathBuf]) {
    figbase::text::add_font_dirs(dirs);
    let mut g = lock();
    for d in dirs {
        if !g.extra.contains(d) {
            g.extra.push(d.clone());
        }
    }
    g.built = None;
}

/// Replace the family list with a fixed one (`None`: the installed fonts
/// again). For tests and hosts without a font database; such faces have no
/// glyphs, so texts set in them fall back to their Fig font.
pub fn override_local_faces(faces: Option<Vec<LocalFace>>) {
    lock().injected = faces;
}

fn weight_label(w: u16) -> &'static str {
    match w {
        0..=150 => "Thin",
        151..=250 => "ExtraLight",
        251..=350 => "Light",
        351..=450 => "",
        451..=550 => "Medium",
        551..=650 => "Semibold",
        651..=750 => "Bold",
        751..=850 => "ExtraBold",
        _ => "Black",
    }
}

fn is_serif_name(n: &str) -> bool {
    let n = n.to_lowercase();
    if [
        "sans",
        "gothic",
        "grotesk",
        "grotesque",
        "helvet",
        "arial",
        "verdana",
        "tahoma",
        "calibri",
    ]
    .iter()
    .any(|k| n.contains(k))
    {
        return false;
    }
    [
        "serif",
        "roman",
        "times",
        "georgia",
        "garamond",
        "palatino",
        "book",
        "minion",
        "caslon",
        "baskerville",
        "didot",
        "cambria",
        "century",
        "charter",
        "cmu",
        "latin modern",
        "termes",
        "antiqua",
    ]
    .iter()
    .any(|k| n.contains(k))
}

fn describe(db: &Database) -> Vec<(LocalFace, ID)> {
    let mut out: Vec<(LocalFace, ID)> = Vec::new();
    for info in db.faces() {
        let Some((family, _)) = info
            .families
            .iter()
            .find(|(_, l)| *l == fontdb::Language::English_UnitedStates)
            .or_else(|| info.families.first())
        else {
            continue;
        };
        let italic = info.style != Style::Normal;
        let mut parts: Vec<&str> = Vec::new();
        let w = weight_label(info.weight.0);
        if !w.is_empty() {
            parts.push(w);
        }
        match info.stretch {
            fontdb::Stretch::Normal => {}
            fontdb::Stretch::UltraCondensed | fontdb::Stretch::ExtraCondensed => parts.push("ExtraCondensed"),
            fontdb::Stretch::Condensed | fontdb::Stretch::SemiCondensed => parts.push("Condensed"),
            _ => parts.push("Expanded"),
        }
        if italic {
            parts.push(if info.style == Style::Oblique {
                "Oblique"
            } else {
                "Italic"
            });
        }
        let label = if parts.is_empty() {
            "Regular".to_string()
        } else {
            parts.join(" ")
        };
        let name = if label == "Regular" {
            family.clone()
        } else {
            format!("{family} {label}")
        };
        if out.iter().any(|(f, _)| f.name == name) {
            continue;
        }
        let generic = if info.monospaced {
            Generic::Mono
        } else if is_serif_name(family) {
            Generic::Serif
        } else {
            Generic::Sans
        };
        out.push((
            LocalFace {
                family: family.clone(),
                label,
                name,
                generic,
                bold: info.weight >= Weight::SEMIBOLD,
                italic,
            },
            info.id,
        ));
    }
    out.sort_by_key(|(f, _)| {
        (
            f.family.to_lowercase(),
            u8::from(f.bold) * 2 + u8::from(f.italic),
            f.name.clone(),
        )
    });
    out
}

fn built(g: &mut State) -> &Built {
    let builtin = crate::builtin_fonts();
    if g.built.as_ref().is_some_and(|b| b.builtin != builtin) {
        g.built = None;
    }
    let extra = g.extra.clone();
    g.built.get_or_insert_with(|| {
        // Only the native build adds the system's fonts.
        #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
        let mut db = Database::new();
        if !builtin {
            #[cfg(not(target_arch = "wasm32"))]
            {
                db.load_system_fonts();
                for d in &extra {
                    db.load_fonts_dir(d);
                }
            }
            #[cfg(target_arch = "wasm32")]
            let _ = &extra;
        }
        let faces = describe(&db);
        Built { builtin, db, faces }
    })
}

/// Every installed face, by family (empty with the built-in fonts).
pub fn local_faces() -> Vec<LocalFace> {
    let mut g = lock();
    if let Some(v) = &g.injected {
        return v.clone();
    }
    built(&mut g).faces.iter().map(|(f, _)| f.clone()).collect()
}

/// The face a stored name stands for (case-insensitive), if it is in the
/// list.
pub fn find_local_face(name: &str) -> Option<LocalFace> {
    let name = name.trim().to_lowercase();
    local_faces().into_iter().find(|f| f.name.to_lowercase() == name)
}

/// Whether text in `name` can be drawn: the face is installed.
pub fn local_font_available(name: &str) -> bool {
    let mut g = lock();
    let key = name.trim().to_lowercase();
    built(&mut g).faces.iter().any(|(f, _)| f.name.to_lowercase() == key)
}

struct Builder<'a> {
    ops: &'a mut Vec<PathOp>,
    scale: f64,
    dx: f64,
    dy: f64,
}

impl Builder<'_> {
    fn map(&self, x: f32, y: f32) -> (f64, f64) {
        (self.dx + f64::from(x) * self.scale, self.dy - f64::from(y) * self.scale)
    }
}

impl OutlineBuilder for Builder<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.map(x, y);
        self.ops.push(PathOp::Move(x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.map(x, y);
        self.ops.push(PathOp::Line(x, y));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (a, b) = self.map(x1, y1);
        let (x, y) = self.map(x, y);
        self.ops.push(PathOp::Quad(a, b, x, y));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, b) = self.map(x1, y1);
        let (c, d) = self.map(x2, y2);
        let (x, y) = self.map(x, y);
        self.ops.push(PathOp::Cubic(a, b, c, d, x, y));
    }
    fn close(&mut self) {
        self.ops.push(PathOp::Close);
    }
}

fn layout(face: &Face<'_>, size_pt: f64, text: &str, outlines: bool) -> Shaped {
    let size = if size_pt.is_finite() {
        size_pt.clamp(0.1, 5000.0)
    } else {
        12.0
    };
    let em = size * FIG_PER_POINT;
    let scale = em / f64::from(face.units_per_em().max(1));
    let ascent = f64::from(face.ascender()) * scale;
    let descent = -f64::from(face.descender()) * scale;
    let mut out = Shaped::default();
    let mut count = 0usize;
    let mut y = 0.0;
    let mut nlines = 0usize;
    for line in text.split('\n') {
        nlines += 1;
        let mut ops = Vec::new();
        let mut x = 0.0;
        let mut prev: Option<GlyphId> = None;
        for ch in line.chars() {
            count += 1;
            if count > MAX_CHARS {
                break;
            }
            let gid = face.glyph_index(ch).unwrap_or(GlyphId(0));
            if let (Some(p), Some(kern)) = (prev, face.tables().kern) {
                for st in kern.subtables {
                    if st.horizontal {
                        if let Some(k) = st.glyphs_kerning(p, gid) {
                            x += f64::from(k) * scale;
                            break;
                        }
                    }
                }
            }
            if outlines {
                let mut b = Builder {
                    ops: &mut ops,
                    scale,
                    dx: x,
                    dy: y,
                };
                let _ = face.outline_glyph(gid, &mut b);
            }
            x += f64::from(face.glyph_hor_advance(gid).unwrap_or(0)) * scale;
            prev = Some(gid);
        }
        out.line_widths.push(x);
        out.lines.push(ops);
        y += em * LINE_SPACING;
    }
    let nlines = nlines.max(1);
    out.metrics = Metrics {
        width: out.line_widths.iter().copied().fold(0.0, f64::max),
        ascent,
        descent: descent + (nlines - 1) as f64 * em * LINE_SPACING,
        lines: nlines,
    };
    out
}

fn run(name: &str, size_pt: f64, text: &str, outlines: bool) -> Option<Shaped> {
    let mut g = lock();
    let key = name.trim().to_lowercase();
    let b = built(&mut g);
    let id = b.faces.iter().find(|(f, _)| f.name.to_lowercase() == key)?.1;
    b.db.with_face_data(id, |data, index| {
        let face = Face::parse(data, index).ok()?;
        Some(layout(&face, size_pt, text, outlines))
    })?
}

/// Glyph outlines of `text` in the installed face `name`; `None` when it
/// is not installed (the caller falls back to the Fig font).
pub fn shape_local(name: &str, size_pt: f64, text: &str) -> Option<Shaped> {
    run(name, size_pt, text, true)
}

/// The extent of `text` in the installed face `name`.
pub fn measure_local(name: &str, size_pt: f64, text: &str) -> Option<Metrics> {
    run(name, size_pt, text, false).map(|s| s.metrics)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The tests that touch the global font state (the list, built-in fonts
    /// only) run one at a time, also with other modules' tests.
    pub(crate) static SERIAL: Mutex<()> = Mutex::new(());

    #[test]
    fn lays_out_and_scales() {
        let data = epaint_default_fonts::HACK_REGULAR;
        let face = Face::parse(data, 0).unwrap();
        let a = layout(&face, 12.0, "Hi\nthere", true);
        let b = layout(&face, 24.0, "Hi\nthere", true);
        assert_eq!(a.metrics.lines, 2);
        assert!(a.metrics.width > 0.0);
        assert!((b.metrics.width / a.metrics.width - 2.0).abs() < 1e-6);
        assert!(a.lines.iter().all(|l| !l.is_empty()));
        // Hack is monospaced: five letters are 2.5 times two.
        assert!((a.line_widths[1] / a.line_widths[0] - 2.5).abs() < 1e-6);
    }

    #[test]
    fn builtin_mode_has_no_installed_faces() {
        let _s = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        crate::set_builtin_fonts(true);
        override_local_faces(None);
        assert!(local_faces().is_empty());
        assert!(shape_local("Anything", 12.0, "x").is_none());
        assert!(!local_font_available("Anything"));
    }

    #[test]
    fn injected_list_is_listed() {
        let _s = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let f = LocalFace {
            family: "Fake".into(),
            label: "Regular".into(),
            name: "Fake".into(),
            generic: Generic::Serif,
            bold: false,
            italic: false,
        };
        override_local_faces(Some(vec![f.clone()]));
        assert_eq!(find_local_face("fake"), Some(f));
        override_local_faces(None);
    }

    /// A font found in a folder is listed, drawn and measured by its name
    /// (the font file is written by the test, from the built-in fonts).
    #[test]
    fn a_font_folder_gives_a_family_by_name() {
        let _s = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let dir = std::env::temp_dir().join(format!("rustfig-local-fonts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hack-test.ttf"), epaint_default_fonts::HACK_REGULAR).unwrap();
        crate::set_builtin_fonts(false);
        override_local_faces(None);
        add_font_dirs(std::slice::from_ref(&dir));
        let face = local_faces()
            .into_iter()
            .find(|f| f.family == "Hack")
            .expect("Hack is listed");
        assert_eq!(face.generic, Generic::Mono);
        assert_eq!(face.name, "Hack");
        assert!(local_font_available("hack"));
        let m = measure_local("Hack", 12.0, "abcde").unwrap();
        let s = shape_local("hack", 12.0, "abcde").unwrap();
        assert!((m.width - s.metrics.width).abs() < 1e-9 && m.width > 0.0);
        assert!(s.lines[0].len() > 5);
        assert!(shape_local("Not Installed", 12.0, "x").is_none());
        // Back to the built-in fonts only.
        crate::set_builtin_fonts(true);
        assert!(!local_font_available("Hack"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn serif_guess() {
        assert!(is_serif_name("Minion Pro"));
        assert!(!is_serif_name("DejaVu Sans"));
    }
}
