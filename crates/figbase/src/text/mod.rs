//! Fig text: the Fig font tables, font selection, text measurement and
//! glyph outlines.
//!
//! Fonts come from the optional craft-fonts collection ([`CRAFT_FONTS`],
//! embedded at build time through `CRAFT_FONTS_DIR`). Without it, figbase
//! uses the open fonts shipped in the `epaint_default_fonts` crate and
//! synthesises bold and italic styles.

mod fonts;

pub use fonts::{
    latex_font_name, ps_font_display_name, ps_font_name, ps_font_number, FontRequest, Generic, LATEX_FONT_COUNT,
    PS_FONT_COUNT,
};

include!(concat!(env!("OUT_DIR"), "/craft_fonts.rs"));

use std::sync::OnceLock;
use ttf_parser::{Face, GlyphId, OutlineBuilder};

/// Fig units per point.
pub const FIG_PER_POINT: f64 = 1200.0 / 72.0;
/// Line spacing as a multiple of the font size.
pub const LINE_SPACING: f64 = 1.2;
/// Upper bound on characters laid out at once (longer strings are truncated).
pub const MAX_CHARS: usize = 100_000;

/// A path command in text-local Fig units (origin on the baseline at the
/// left end of the first line, y down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathOp {
    Move(f64, f64),
    Line(f64, f64),
    Quad(f64, f64, f64, f64),
    Cubic(f64, f64, f64, f64, f64, f64),
    Close,
}

/// Measured extent of a (possibly multi-line) string, in Fig units.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Metrics {
    /// Advance width of the widest line.
    pub width: f64,
    /// Ascent above the first baseline.
    pub ascent: f64,
    /// Descent below the last baseline (positive).
    pub descent: f64,
    /// Number of lines.
    pub lines: usize,
}

/// Laid-out glyph outlines for one string.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Shaped {
    /// Outlines, one sub-list per line so callers can justify each line.
    pub lines: Vec<Vec<PathOp>>,
    pub metrics: Metrics,
    /// Per-line advance widths (for justification).
    pub line_widths: Vec<f64>,
    /// The face has no bold variant: the renderer should embolden by stroking.
    pub synthetic_bold: bool,
}

struct LoadedFace {
    face: Face<'static>,
    /// Lower-case family + file name, for matching.
    key: String,
    bold: bool,
    italic: bool,
}

struct Library {
    faces: Vec<LoadedFace>,
}

fn library() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let mut faces = Vec::new();
        let mut add = |stem: &str, bytes: &'static [u8]| {
            let Ok(face) = Face::parse(bytes, 0) else { return };
            let mut family = String::new();
            for name in face.names() {
                if name.name_id == ttf_parser::name_id::FULL_NAME || name.name_id == ttf_parser::name_id::FAMILY {
                    if let Some(s) = name.to_string() {
                        family.push_str(&s);
                        family.push(' ');
                    }
                }
            }
            let key = format!("{family} {stem}").to_lowercase();
            let bold = face.is_bold() || key.contains("bold") || key.contains("demi") || key.contains("heavy");
            let italic = face.is_italic() || face.is_oblique() || key.contains("italic") || key.contains("oblique");
            faces.push(LoadedFace {
                face,
                key,
                bold,
                italic,
            });
        };
        for (stem, bytes) in CRAFT_FONTS {
            add(stem, bytes);
        }
        add("fallback-sans ubuntu", epaint_default_fonts::UBUNTU_LIGHT);
        add("fallback-mono hack", epaint_default_fonts::HACK_REGULAR);
        Library { faces }
    })
}

/// Number of usable font faces (craft fonts plus fallbacks).
pub fn face_count() -> usize {
    library().faces.len()
}

/// Pick the best face for a request; returns (face index, needs synthetic
/// bold, needs synthetic italic).
fn pick(req: &FontRequest) -> Option<(usize, bool, bool)> {
    let lib = library();
    let mut best: Option<(i32, usize)> = None;
    for (i, f) in lib.faces.iter().enumerate() {
        let mut score = 0;
        if req.keywords.iter().any(|k| f.key.contains(k)) {
            score += 100;
        } else if req.generic.keywords().iter().any(|k| f.key.contains(k)) {
            score += 40;
        } else if f.key.contains("fallback") {
            let mono = f.key.contains("mono");
            score += if mono == (req.generic == Generic::Mono) { 20 } else { 5 };
        }
        if f.bold == req.bold {
            score += 10;
        }
        if f.italic == req.italic {
            score += 8;
        }
        if best.is_none_or(|(s, _)| score > s) {
            best = Some((score, i));
        }
    }
    let (_, i) = best?;
    let f = lib.faces.get(i)?;
    Some((i, req.bold && !f.bold, req.italic && !f.italic))
}

struct Builder<'a> {
    ops: &'a mut Vec<PathOp>,
    scale: f64,
    dx: f64,
    dy: f64,
    skew: f64,
}

impl Builder<'_> {
    fn map(&self, x: f32, y: f32) -> (f64, f64) {
        let (x, y) = (f64::from(x) * self.scale, -f64::from(y) * self.scale);
        (self.dx + x - y * self.skew, self.dy + y)
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

fn layout(req: &FontRequest, size_pt: f64, text: &str, outlines: bool) -> Shaped {
    let size = if size_pt.is_finite() {
        size_pt.clamp(0.1, 5000.0)
    } else {
        12.0
    };
    let em = size * FIG_PER_POINT;
    let mut out = Shaped::default();
    let lib = library();
    let Some((idx, synth_bold, synth_italic)) = pick(req) else {
        // No font at all: estimate.
        let widths: Vec<f64> = text.split('\n').map(|l| l.chars().count() as f64 * em * 0.5).collect();
        out.metrics = Metrics {
            width: widths.iter().copied().fold(0.0, f64::max),
            ascent: em * 0.75,
            descent: em * 0.25,
            lines: widths.len(),
        };
        out.line_widths = widths;
        return out;
    };
    let Some(lf) = lib.faces.get(idx) else { return out };
    let face = &lf.face;
    let scale = em / f64::from(face.units_per_em().max(1));
    let ascent = f64::from(face.ascender()) * scale;
    let descent = -f64::from(face.descender()) * scale;
    let fallback = lib
        .faces
        .iter()
        .find(|f| f.key.contains("fallback-sans"))
        .map(|f| &f.face);
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
            let (f, gid) = match face.glyph_index(ch) {
                Some(g) => (face, g),
                None => fallback
                    .and_then(|fb| fb.glyph_index(ch).map(|g| (fb, g)))
                    .unwrap_or((face, GlyphId(0))),
            };
            let fscale = em / f64::from(f.units_per_em().max(1));
            if let (Some(p), true) = (prev, std::ptr::eq(f, face)) {
                if let Some(kern) = face.tables().kern {
                    for st in kern.subtables {
                        if st.horizontal {
                            if let Some(k) = st.glyphs_kerning(p, gid) {
                                x += f64::from(k) * fscale;
                                break;
                            }
                        }
                    }
                }
            }
            if outlines {
                let mut b = Builder {
                    ops: &mut ops,
                    scale: fscale,
                    dx: x,
                    dy: y,
                    skew: if synth_italic { 0.2 } else { 0.0 },
                };
                let _ = f.outline_glyph(gid, &mut b);
            }
            x += f64::from(f.glyph_hor_advance(gid).unwrap_or(0)) * fscale;
            prev = Some(gid);
        }
        if synth_bold {
            x += em * 0.04;
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
    out.synthetic_bold = synth_bold;
    out
}

/// Measure a string set in the requested font at `size_pt` points.
pub fn measure(req: &FontRequest, size_pt: f64, text: &str) -> Metrics {
    layout(req, size_pt, text, false).metrics
}

/// Glyph outlines of a string. Each line starts at x = 0 and lines are
/// spaced [`LINE_SPACING`] em apart; justification is up to the caller (see
/// [`Shaped::line_widths`]).
pub fn shape(req: &FontRequest, size_pt: f64, text: &str) -> Shaped {
    layout(req, size_pt, text, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_and_shapes() {
        let req = FontRequest::for_fig(16, true);
        let m = measure(&req, 12.0, "Hello");
        assert!(m.width > 0.0 && m.ascent > 0.0);
        let wide = measure(&req, 24.0, "Hello");
        assert!((wide.width / m.width - 2.0).abs() < 0.05);
        let s = shape(&req, 12.0, "Hi\nthere");
        assert!(s.lines.iter().any(|l| !l.is_empty()));
        assert_eq!(s.metrics.lines, 2);
        assert_eq!(s.line_widths.len(), 2);
    }

    #[test]
    fn hostile_inputs() {
        let req = FontRequest::for_fig(9999, true);
        let _ = measure(&req, f64::NAN, "\u{0}\u{10FFFF}");
        let _ = measure(&FontRequest::for_fig(-5, false), -3.0, "");
        let big = "x".repeat(MAX_CHARS + 10);
        let m = measure(&req, 12.0, &big);
        assert!(m.width.is_finite());
    }

    #[test]
    fn fallback_faces_present() {
        assert!(face_count() >= 2);
    }

    #[test]
    fn courier_is_monospaced() {
        let req = FontRequest::for_fig(12, true);
        let a = measure(&req, 12.0, "iiii");
        let b = measure(&req, 12.0, "MMMM");
        assert!((a.width - b.width).abs() < 1.0);
    }
}
