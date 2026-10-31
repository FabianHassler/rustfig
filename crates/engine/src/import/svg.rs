//! SVG import (plan G7): an SVG file as editable Fig objects.
//!
//! usvg resolves CSS, transforms, `use` and shapes; this module walks its
//! tree and makes Fig lines, polygons, boxes, arc-boxes, ellipses, texts and
//! pictures from it. User units are CSS pixels (96 per inch) and become Fig
//! units (1200 per inch) with y down, as in SVG. Curves are flattened
//! (tolerance one Fig unit) except exact circles, ellipses and rounded
//! rectangles. Gradients become their average colour; clip paths, masks,
//! filters and transparency are dropped, one warning each. An SVG is
//! untrusted input: object, point and image sizes are capped, external files
//! are read only from the SVG's own folder, and everything returns `Result`.

use super::pdf::{map_font, split_subpaths, subpath_object};
use hayro::kurbo::{Affine, BezPath, PathEl, Point as KPoint};
use rustfig_geom::Point;
use rustfig_model::{
    text_flags, CapStyle, ColorRef, Document, Ellipse, EllipseKind, JoinStyle, Justify, LineKind, Object, ObjectKind,
    Picture, PolyKind, Polyline, Rgb, Style, Text,
};
use std::io::Read;
use std::sync::{Arc, OnceLock};

/// Most objects one import creates.
const MAX_OBJECTS: usize = super::pdf::MAX_OBJECTS;
/// Largest SVG text after unpacking an svgz, in bytes.
const MAX_SVG_BYTES: u64 = 64 << 20;
/// Fig units per CSS pixel (1200 per inch, 96 pixels per inch).
const K: f64 = 1200.0 / 96.0;
/// Coordinates beyond this (Fig units) are not drawn.
const MAX_COORD: f64 = 1.0e7;
/// Most image pixels one import keeps in all (memory cap).
const MAX_IMAGE_PIXELS: u64 = 100_000_000;
/// Deepest nesting of groups and nested SVG images followed.
const MAX_DEPTH: usize = 200;
/// Deepest element nesting accepted in the XML.
const MAX_NESTING: usize = 256;
/// Largest external image file read next to the SVG, in bytes.
const MAX_EXTERNAL_BYTES: u64 = 64 << 20;

/// What the import made.
pub struct Imported {
    /// The figure: objects and user colours.
    pub doc: Document,
    /// The SVG's raster images as PNG files: (file name, bytes); the picture
    /// objects name them.
    pub images: Vec<(String, Vec<u8>)>,
    /// What was dropped or approximated.
    pub warnings: Vec<String>,
}

/// Whether the data looks like an SVG or SVGZ file (`<svg` within the first
/// 4 KB, also inside a gzip stream).
pub fn is_svg(bytes: &[u8]) -> bool {
    let has_svg = |head: &[u8]| head.windows(4).any(|w| w == b"<svg");
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut head = Vec::new();
        let ok = flate2::read::GzDecoder::new(bytes)
            .take(4096)
            .read_to_end(&mut head)
            .is_ok();
        return ok && has_svg(&head);
    }
    has_svg(bytes.get(..bytes.len().min(4096)).unwrap_or(bytes))
}

/// The SVG text of the file: svgz unpacked up to [`MAX_SVG_BYTES`].
fn unpack(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        return Ok(bytes.to_vec());
    }
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes)
        .take(MAX_SVG_BYTES + 1)
        .read_to_end(&mut out)
        .map_err(|_| "the compressed SVG is damaged".to_string())?;
    if out.len() as u64 > MAX_SVG_BYTES {
        return Err("the SVG is too large".into());
    }
    Ok(out)
}

/// The deepest element nesting of the XML text, counted without parsing it
/// (comments and CDATA skipped). usvg and its XML parser recurse per level,
/// so a very deep document could overflow the stack.
fn nesting(data: &[u8]) -> usize {
    let (mut depth, mut deepest, mut i) = (0usize, 0usize, 0usize);
    while i < data.len() {
        if data[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &data[i..];
        let skip_to = |end: &[u8]| rest.windows(end.len()).position(|w| w == end).map(|n| n + end.len());
        let adv = if rest.starts_with(b"<!--") {
            skip_to(b"-->")
        } else if rest.starts_with(b"<![CDATA[") {
            skip_to(b"]]>")
        } else if rest.starts_with(b"<?") || rest.starts_with(b"<!") {
            skip_to(b">")
        } else {
            // A tag: ends at the first '>' outside quotes.
            let mut quote = 0u8;
            let mut end = None;
            for (j, &c) in rest.iter().enumerate().skip(1) {
                if quote != 0 {
                    if c == quote {
                        quote = 0;
                    }
                } else if c == b'"' || c == b'\'' {
                    quote = c;
                } else if c == b'>' {
                    end = Some(j);
                    break;
                }
            }
            if let Some(j) = end {
                if rest.get(1) == Some(&b'/') {
                    depth = depth.saturating_sub(1);
                } else if rest.get(j.saturating_sub(1)) != Some(&b'/') {
                    depth += 1;
                    deepest = deepest.max(depth);
                }
                Some(j + 1)
            } else {
                None
            }
        };
        match adv {
            Some(n) => i += n,
            None => break,
        }
    }
    deepest
}

/// The fonts usvg lays text out with: the built-in ones (no system fonts, so
/// the import does not depend on the machine). Only the layout needs them;
/// the Fig text keeps the font family's name, mapped to a Fig font.
fn fontdb() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_font_data(epaint_default_fonts::UBUNTU_LIGHT.to_vec());
        db.load_font_data(epaint_default_fonts::HACK_REGULAR.to_vec());
        let family = |db: &usvg::fontdb::Database, needle: &str| {
            db.faces()
                .find(|f| f.post_script_name.contains(needle))
                .and_then(|f| f.families.first().map(|(n, _)| n.clone()))
        };
        if let Some(n) = family(&db, "Ubuntu") {
            db.set_serif_family(n.clone());
            db.set_sans_serif_family(n.clone());
            db.set_cursive_family(n.clone());
            db.set_fantasy_family(n);
        }
        if let Some(n) = family(&db, "Hack") {
            db.set_monospace_family(n);
        }
        Arc::new(db)
    })
    .clone()
}

/// The usvg options: 96 dpi, built-in fonts, and external images only from
/// `dir` (no absolute paths, no `..`, size capped; none on the web).
fn options(dir: Option<&std::path::Path>) -> usvg::Options<'static> {
    let mut opt = usvg::Options {
        fontdb: fontdb(),
        ..usvg::Options::default()
    };
    opt.font_family = "serif".to_string();
    opt.resources_dir = dir.map(std::path::Path::to_path_buf);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let base = dir.map(std::path::Path::to_path_buf);
        opt.image_href_resolver.resolve_string = Box::new(move |href, _| {
            let base = base.as_ref()?;
            let rel = std::path::Path::new(href);
            let plain = rel
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_) | std::path::Component::CurDir));
            if !plain || rel.is_absolute() || href.contains('\0') {
                return None;
            }
            let path = base.join(rel);
            if std::fs::metadata(&path).ok()?.len() > MAX_EXTERNAL_BYTES {
                return None;
            }
            let data = std::fs::read(path).ok()?;
            image_kind(data)
        });
    }
    #[cfg(target_arch = "wasm32")]
    {
        opt.image_href_resolver.resolve_string = Box::new(|_, _| None);
    }
    opt
}

/// A raster file's bytes as the usvg image kind of its format.
#[cfg(not(target_arch = "wasm32"))]
fn image_kind(data: Vec<u8>) -> Option<usvg::ImageKind> {
    let data = Arc::new(data);
    if data.starts_with(b"\x89PNG") {
        Some(usvg::ImageKind::PNG(data))
    } else if data.starts_with(&[0xff, 0xd8]) {
        Some(usvg::ImageKind::JPEG(data))
    } else if data.starts_with(b"GIF8") {
        Some(usvg::ImageKind::GIF(data))
    } else {
        None
    }
}

/// An SVG file as Fig objects. `stem` names the pictures (`stem-1.png`);
/// `dir` is the folder external images are read from.
pub fn import(bytes: &[u8], stem: &str, dir: Option<&std::path::Path>) -> Result<Imported, String> {
    crate::guard::catch(|| import_inner(bytes, stem, dir)).map_err(|e| format!("the SVG cannot be imported ({e})"))?
}

fn import_inner(bytes: &[u8], stem: &str, dir: Option<&std::path::Path>) -> Result<Imported, String> {
    let data = unpack(bytes)?;
    if nesting(&data) > MAX_NESTING {
        return Err("the SVG is nested too deeply".into());
    }
    let tree = usvg::Tree::from_data(&data, &options(dir)).map_err(|e| format!("not a usable SVG: {e}"))?;
    let size = tree.size();
    if !(size.width().is_finite() && size.height().is_finite()) || size.width() > 1.0e6 || size.height() > 1.0e6 {
        return Err("the drawing size is not usable".into());
    }
    let mut imp = Importer {
        doc: Document::new(),
        objects: Vec::new(),
        images: Vec::new(),
        warnings: Vec::new(),
        warned: Vec::new(),
        stem: stem.to_string(),
        image_pixels: 0,
        depth: 999,
        rank: 0,
        out_of_colours: false,
        capped: false,
    };
    imp.group(tree.root(), Affine::IDENTITY, 0);
    Ok(imp.finish())
}

struct Importer {
    doc: Document,
    objects: Vec<Object>,
    images: Vec<(String, Vec<u8>)>,
    warnings: Vec<String>,
    warned: Vec<&'static str>,
    stem: String,
    image_pixels: u64,
    /// The depth the next object goes to and the highest xfig type rank on it.
    depth: i32,
    rank: u8,
    out_of_colours: bool,
    capped: bool,
}

/// A usvg transform as a kurbo one.
fn affine(t: usvg::Transform) -> Affine {
    Affine::new([
        f64::from(t.sx),
        f64::from(t.ky),
        f64::from(t.kx),
        f64::from(t.sy),
        f64::from(t.tx),
        f64::from(t.ty),
    ])
}

fn finite(a: Affine) -> bool {
    a.as_coeffs().iter().all(|c| c.is_finite())
}

/// Pixels per user unit of a transform.
fn scale_of(a: Affine) -> f64 {
    let c = a.as_coeffs();
    (c[0] * c[3] - c[1] * c[2]).abs().sqrt()
}

impl Importer {
    fn warn_once(&mut self, key: &'static str, msg: &str) {
        if !self.warned.contains(&key) {
            self.warned.push(key);
            self.warnings.push(msg.to_string());
        }
    }

    fn finish(mut self) -> Imported {
        if self.capped {
            self.warnings
                .push(format!("Only the first {MAX_OBJECTS} objects were imported"));
        }
        self.doc.objects = self.objects.into_iter().map(Arc::new).collect();
        Imported {
            doc: self.doc,
            images: self.images,
            warnings: self.warnings,
        }
    }

    /// Add an object on top of everything so far. Within a depth xfig draws
    /// by type (ellipses, lines, splines, texts), so an object whose type
    /// would come out below an earlier one on the same depth gets a new depth.
    fn push(&mut self, mut o: Object) {
        if self.objects.len() >= MAX_OBJECTS {
            self.capped = true;
            return;
        }
        let r = o.xfig_type_rank();
        if r < self.rank {
            if self.depth > 0 {
                self.depth -= 1;
            } else {
                self.warn_once(
                    "depth",
                    "More stacking levels than Fig has: the stacking order is approximate",
                );
            }
            self.rank = 0;
        }
        self.rank = self.rank.max(r);
        o.set_depth(self.depth);
        self.objects.push(o);
    }

    fn color(&mut self, rgb: Rgb) -> ColorRef {
        match self.doc.color_for_rgb(rgb) {
            Some(c) => c,
            None => {
                if !self.out_of_colours {
                    self.out_of_colours = true;
                    self.warnings
                        .push("More than 512 colours: the rest is drawn in black".into());
                }
                ColorRef::BLACK
            }
        }
    }

    /// The colour of a paint; gradients become the average of their stops,
    /// patterns grey, each with a warning.
    fn paint_rgb(&mut self, paint: &usvg::Paint, opacity: f32) -> Option<Rgb> {
        if opacity <= 0.0 {
            return None;
        }
        if opacity < 1.0 {
            self.warn_once("alpha", "Transparency is ignored: translucent shapes are drawn solid");
        }
        match paint {
            usvg::Paint::Color(c) => Some(Rgb::new(c.red, c.green, c.blue)),
            usvg::Paint::LinearGradient(g) => self.gradient(g.stops()),
            usvg::Paint::RadialGradient(g) => self.gradient(g.stops()),
            usvg::Paint::Pattern(p) => {
                self.warn_once("pattern", "Patterns are drawn in their ink colour, solid");
                Some(pattern_ink(p.root(), 0).unwrap_or(Rgb::new(128, 128, 128)))
            }
        }
    }

    fn gradient(&mut self, stops: &[usvg::Stop]) -> Option<Rgb> {
        self.warn_once("gradient", "Gradients are drawn in their average colour");
        if stops.is_empty() {
            return None;
        }
        let n = stops.len() as f64;
        let avg =
            |f: fn(&usvg::Color) -> u8| (stops.iter().map(|s| f64::from(f(&s.color()))).sum::<f64>() / n).round() as u8;
        Some(Rgb::new(avg(|c| c.red), avg(|c| c.green), avg(|c| c.blue)))
    }

    fn group(&mut self, g: &usvg::Group, base: Affine, depth: usize) {
        if depth > MAX_DEPTH {
            self.warn_once("deep", "Groups nested too deeply were skipped");
            return;
        }
        if g.clip_path().is_some() {
            self.warn_once("clip", "Clipping is ignored: clipped drawings are shown whole");
        }
        if g.mask().is_some() {
            self.warn_once("mask", "Masks are ignored");
        }
        if !g.filters().is_empty() {
            self.warn_once("filter", "Filters are ignored");
        }
        if g.opacity().get() < 1.0 {
            self.warn_once("alpha", "Transparency is ignored: translucent shapes are drawn solid");
        }
        for n in g.children() {
            if self.objects.len() >= MAX_OBJECTS {
                self.capped = true;
                return;
            }
            match n {
                usvg::Node::Group(g) => self.group(g, base, depth + 1),
                usvg::Node::Path(p) => self.path(p, base),
                usvg::Node::Text(t) => self.text(t, base, depth),
                usvg::Node::Image(i) => self.image(i, base, depth),
            }
        }
    }

    // ------------------------------------------------------------ paths

    fn path(&mut self, p: &usvg::Path, base: Affine) {
        if !p.is_visible() {
            return;
        }
        let t = base * affine(p.abs_transform());
        if !finite(t) {
            return;
        }
        let fill = p.fill().and_then(|f| self.paint_rgb(f.paint(), f.opacity().get()));
        let stroke = p
            .stroke()
            .and_then(|s| self.paint_rgb(s.paint(), s.opacity().get()).map(|c| (s, c)));
        if fill.is_none() && stroke.is_none() {
            return;
        }
        if p.fill().is_some_and(|f| f.rule() == usvg::FillRule::EvenOdd) && p.data().len() > 6 {
            self.warn_once(
                "holes",
                "Filled shapes with holes become separate shapes (Fig has no holes)",
            );
        }
        let pen = stroke.map_or_else(|| fill.unwrap_or(Rgb::new(0, 0, 0)), |(_, c)| c);
        let pen_color = self.color(pen);
        let fill_color = fill.map_or(pen_color, |c| self.color(c));
        let mut style = Style {
            pen_color,
            fill_color,
            thickness: 0,
            area_fill: if fill.is_some() { 20 } else { -1 },
            ..Style::default()
        };
        if let Some((s, _)) = stroke {
            apply_stroke(&mut style, s, scale_of(t));
        }
        let m = Affine::scale(K) * t;
        let path = bez(p.data(), m);
        for group in split_groups(&path) {
            self.subpath(&group, style, fill.is_some());
        }
    }

    /// One subpath: an exact rounded box or ellipse, else flattened.
    fn subpath(&mut self, g: &[PathEl], style: Style, fill: bool) {
        if g.iter().any(|el| !el_finite(el)) {
            return;
        }
        if let Some(o) = round_box(g, style) {
            self.push(o);
            return;
        }
        if let Some(o) = ellipse_object(g, style) {
            self.push(o);
            return;
        }
        let (subs, ok) = split_subpaths(&BezPath::from_vec(g.to_vec()));
        if !ok {
            return;
        }
        // An open path stays a polyline: Fig fills it as if closed and
        // strokes only the open line, as SVG does.
        let closed = matches!(g.last(), Some(PathEl::ClosePath));
        for sp in subs {
            if let Some(o) = subpath_object(&sp, style, fill && closed) {
                self.push(o);
            }
        }
    }

    // ------------------------------------------------------------- text

    fn text(&mut self, t: &usvg::Text, base: Affine, depth: usize) {
        if self.simple_text(t, base) {
            return;
        }
        self.warn_once("glyphs", "Text that Fig cannot lay out became outlines");
        self.group(t.flattened(), base, depth + 1);
    }

    /// Texts made from the chunks of `t`; `false` when the layout is too
    /// complex (text on a path, vertical, mixed styles in one chunk,
    /// per-letter rotation), then the caller draws the outlines.
    fn simple_text(&mut self, t: &usvg::Text, base: Affine) -> bool {
        if t.writing_mode() != usvg::WritingMode::LeftToRight
            || t.rotate().iter().any(|r| *r != 0.0)
            || t.chunks().is_empty()
        {
            return false;
        }
        let m = base * affine(t.abs_transform());
        if !finite(m) {
            return true;
        }
        let mut out: Vec<Text> = Vec::new();
        for ch in t.chunks() {
            if !matches!(ch.text_flow(), usvg::TextFlow::Linear) {
                return false;
            }
            let Some(first) = ch.spans().first() else {
                continue;
            };
            let same = ch.spans().iter().all(|s| {
                s.font().families() == first.font().families()
                    && s.font().weight() == first.font().weight()
                    && s.font().style() == first.font().style()
                    && s.font_size() == first.font_size()
                    && s.fill().map(|f| f.paint().clone()) == first.fill().map(|f| f.paint().clone())
                    && s.letter_spacing() == 0.0
            });
            if !same || first.small_caps() {
                return false;
            }
            let Some(f) = first.fill() else {
                continue;
            };
            let Some(rgb) = self.paint_rgb(f.paint(), f.opacity().get()) else {
                continue;
            };
            if ch.spans().iter().any(|s| {
                let d = s.decoration();
                d.underline().is_some() || d.overline().is_some() || d.line_through().is_some()
            }) {
                self.warn_once("decoration", "Underlines and strike-through are dropped");
            }
            let c = m.as_coeffs();
            let size = f64::from(first.font_size().get()) * c[0].hypot(c[1]) * 0.75;
            if !(0.5..=4000.0).contains(&size) {
                return false;
            }
            let angle = (-c[1]).atan2(c[0]);
            let angle = if angle < 0.0 {
                angle + std::f64::consts::TAU
            } else {
                angle
            };
            let at = m * KPoint::new(f64::from(ch.x().unwrap_or(0.0)), f64::from(ch.y().unwrap_or(0.0)));
            let (x, y) = (at.x * K, at.y * K);
            if !(x.is_finite() && y.is_finite()) || x.abs() > MAX_COORD || y.abs() > MAX_COORD {
                continue;
            }
            let text: String = ch
                .text()
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect();
            if text.trim().is_empty() {
                continue;
            }
            let (font, ps) = fig_font(first.font());
            // Symbol and ZapfDingbats texts store the glyph's code, not Unicode.
            let text: String = if font == 32 || font == 34 {
                text.chars()
                    .map(|c| rustfig_text::symbol_tables::store_char(font, c).unwrap_or(c))
                    .collect()
            } else {
                text
            };
            if text.chars().any(|c| c as u32 > 0xFF) {
                self.warn_once(
                    "latin1",
                    "Characters outside Latin-1 are kept, but Fig's PostScript fonts cannot show them",
                );
            }
            let color = self.color(rgb);
            out.push(Text {
                justify: match ch.anchor() {
                    usvg::TextAnchor::Start => Justify::Left,
                    usvg::TextAnchor::Middle => Justify::Center,
                    usvg::TextAnchor::End => Justify::Right,
                },
                color,
                depth: 50,
                pen_style: -1,
                font,
                size: (size * 10.0).round() / 10.0,
                angle,
                flags: if ps { text_flags::POSTSCRIPT } else { 0 },
                height: 0.0,
                length: 0.0,
                origin: Point::new(x.round(), y.round()),
                text,
                local_font: None,
            });
        }
        if t.dx().iter().chain(t.dy()).any(|d| *d != 0.0) {
            self.warn_once("dxdy", "Letter offsets (dx, dy) in text are ignored");
        }
        for x in out {
            self.push(Object::new(ObjectKind::Text(x)));
        }
        true
    }

    // ----------------------------------------------------------- images

    fn image(&mut self, im: &usvg::Image, base: Affine, depth: usize) {
        if !im.is_visible() {
            return;
        }
        let t = base * affine(im.abs_transform());
        if !finite(t) {
            return;
        }
        let kind = match im.kind() {
            usvg::ImageKind::SVG(tree) => {
                if depth + 1 > MAX_DEPTH / 10 {
                    self.warn_once("deep", "Groups nested too deeply were skipped");
                } else {
                    self.group(tree.root(), t, depth + 10);
                }
                return;
            }
            usvg::ImageKind::JPEG(d) | usvg::ImageKind::PNG(d) | usvg::ImageKind::GIF(d) | usvg::ImageKind::WEBP(d) => {
                d.clone()
            }
        };
        let (w, h) = (f64::from(im.size().width()), f64::from(im.size().height()));
        let Some(rgba) = decode(&kind) else {
            self.warn_once("imgfmt", "Images in a format Fig cannot read were skipped");
            return;
        };
        self.picture(rgba, t, w, h);
    }

    fn picture(&mut self, rgba: image::RgbaImage, t: Affine, w: f64, h: f64) {
        let max = f64::from(crate::import::MAX_PICTURE_EDGE);
        let (pw, ph) = (rgba.width(), rgba.height());
        self.image_pixels += u64::from(pw) * u64::from(ph);
        if pw == 0 || ph == 0 || f64::from(pw) > max || f64::from(ph) > max || self.image_pixels > MAX_IMAGE_PIXELS {
            self.warn_once("imgsize", "Images that are too large were skipped");
            return;
        }
        let m = Affine::scale(K) * t;
        let pts = [
            m * KPoint::new(0.0, 0.0),
            m * KPoint::new(w, 0.0),
            m * KPoint::new(w, h),
            m * KPoint::new(0.0, h),
        ];
        if pts
            .iter()
            .any(|p| !(p.x.is_finite() && p.y.is_finite()) || p.x.abs() > MAX_COORD || p.y.abs() > MAX_COORD)
        {
            return;
        }
        let c = m.as_coeffs();
        let mut flipped = false;
        let mut img = rgba;
        // Axis-aligned images only: a mirrored one is flipped, a turned one
        // is drawn in its bounding box.
        if c[1].abs() > 1e-6 * c[0].abs().max(1e-9) || c[2].abs() > 1e-6 * c[3].abs().max(1e-9) {
            self.warn_once("rotimg", "Rotated or skewed images are placed in their bounding box");
        } else {
            flipped = c[0] < 0.0;
            if c[3] < 0.0 {
                image::imageops::flip_vertical_in_place(&mut img);
            }
        }
        let mut png = Vec::new();
        if image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .is_err()
        {
            return;
        }
        let name = format!("{}-{}.png", self.stem, self.images.len() + 1);
        let xs = pts.iter().map(|p| p.x);
        let ys = pts.iter().map(|p| p.y);
        let (x0, x1) = (xs.clone().fold(f64::MAX, f64::min), xs.fold(f64::MIN, f64::max));
        let (y0, y1) = (ys.clone().fold(f64::MAX, f64::min), ys.fold(f64::MIN, f64::max));
        let st = Style {
            thickness: 0,
            ..Style::default()
        };
        let mut pl = Polyline::rect(
            st,
            Point::new(x0.round(), y0.round()),
            Point::new(x1.round(), y1.round()),
        );
        pl.kind = PolyKind::Picture;
        pl.picture = Some(Picture {
            flipped,
            file: name.clone(),
            image: None,
        });
        self.images.push((name, png));
        self.push(Object::new(ObjectKind::Polyline(pl)));
    }
}

/// Raster bytes (PNG, JPEG, GIF) as RGBA, within the picture size cap.
fn decode(data: &[u8]) -> Option<image::RgbaImage> {
    let reader = image::ImageReader::new(std::io::Cursor::new(data))
        .with_guessed_format()
        .ok()?;
    let (w, h) = reader.into_dimensions().ok()?;
    if w == 0 || h == 0 || w > crate::import::MAX_PICTURE_EDGE || h > crate::import::MAX_PICTURE_EDGE {
        return None;
    }
    Some(image::load_from_memory(data).ok()?.to_rgba8())
}

/// The Fig font of an SVG font: the first family that names a known font
/// (Times, Helvetica, Courier, ... or the generic serif, sans-serif,
/// monospace), with bold and italic from weight and style.
fn fig_font(f: &usvg::Font) -> (i32, bool) {
    let name_of = |fam: &usvg::FontFamily| match fam {
        usvg::FontFamily::Serif => "serif".to_string(),
        usvg::FontFamily::SansSerif => "sans-serif".to_string(),
        usvg::FontFamily::Monospace => "monospace".to_string(),
        usvg::FontFamily::Cursive => "chancery".to_string(),
        usvg::FontFamily::Fantasy => "serif".to_string(),
        usvg::FontFamily::Named(n) => n.clone(),
    };
    // A name is known when the descriptor flags do not change its font.
    let known = |n: &str| map_font(Some(n), Some(1), None).0 == map_font(Some(n), Some(0), None).0;
    let family = f
        .families()
        .iter()
        .map(name_of)
        .find(|n| known(n))
        .unwrap_or_else(|| "times".to_string());
    let bold = f.weight() >= 600;
    let italic = f.style() != usvg::FontStyle::Normal;
    let name = format!(
        "{family}{}{}",
        if bold { " bold" } else { "" },
        if italic { " italic" } else { "" }
    );
    map_font(Some(&name), None, None)
}

/// The colour of the first solid fill or stroke in a pattern's tile.
fn pattern_ink(g: &usvg::Group, depth: usize) -> Option<Rgb> {
    if depth > 20 {
        return None;
    }
    for n in g.children() {
        let solid = |p: &usvg::Paint| match p {
            usvg::Paint::Color(c) => Some(Rgb::new(c.red, c.green, c.blue)),
            _ => None,
        };
        let found = match n {
            usvg::Node::Group(g) => pattern_ink(g, depth + 1),
            usvg::Node::Path(p) => p
                .stroke()
                .and_then(|s| solid(s.paint()))
                .or_else(|| p.fill().and_then(|f| solid(f.paint()))),
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Stroke width, dashes, caps and joins as Fig attributes. `scale` takes the
/// line width from user units to pixels.
fn apply_stroke(st: &mut Style, s: &usvg::Stroke, scale: f64) {
    let w_px = f64::from(s.width().get()) * scale;
    let w_in = if w_px.is_finite() { w_px / 96.0 } else { 0.0 };
    // geom::stroke_width: thickness 1 is 1/160 inch, t above 1 is (t - 1)/80.
    st.thickness = if w_in <= 0.0 || w_in < 1.5 / 160.0 {
        1
    } else {
        ((w_in * 80.0).round() as i32 + 1).clamp(1, 500)
    };
    st.cap = match s.linecap() {
        usvg::LineCap::Butt => CapStyle::Butt,
        usvg::LineCap::Round => CapStyle::Round,
        usvg::LineCap::Square => CapStyle::Projecting,
    };
    st.join = match s.linejoin() {
        usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => JoinStyle::Miter,
        usvg::LineJoin::Round => JoinStyle::Round,
        usvg::LineJoin::Bevel => JoinStyle::Bevel,
    };
    let dash: Vec<f64> = s
        .dasharray()
        .unwrap_or(&[])
        .iter()
        .map(|d| f64::from(*d) * scale)
        .filter(|d| d.is_finite() && *d > 0.0)
        .collect();
    if let Some(&d) = dash.first() {
        let gap = dash.get(1).copied().unwrap_or(d);
        let wp = w_px.max(0.01);
        let to_80 = |px: f64| (px / 96.0 * 80.0).clamp(1.0, 10_000.0);
        if d <= 1.5 * wp {
            st.line = LineKind::Dotted;
            st.style_val = to_80(d + gap).round();
        } else {
            st.line = LineKind::Dashed;
            st.style_val = to_80(d).round();
        }
    }
}

// --------------------------------------------------------------- geometry

fn kp(p: usvg::tiny_skia_path::Point, m: Affine) -> KPoint {
    m * KPoint::new(f64::from(p.x), f64::from(p.y))
}

/// A usvg path as a kurbo path, transformed by `m`.
fn bez(path: &usvg::tiny_skia_path::Path, m: Affine) -> BezPath {
    use usvg::tiny_skia_path::PathSegment as S;
    let mut out = BezPath::new();
    for s in path.segments() {
        match s {
            S::MoveTo(p) => out.move_to(kp(p, m)),
            S::LineTo(p) => out.line_to(kp(p, m)),
            S::QuadTo(a, b) => out.quad_to(kp(a, m), kp(b, m)),
            S::CubicTo(a, b, c) => out.curve_to(kp(a, m), kp(b, m), kp(c, m)),
            S::Close => out.close_path(),
        }
    }
    out
}

fn el_finite(el: &PathEl) -> bool {
    let ok = |p: &KPoint| p.x.is_finite() && p.y.is_finite() && p.x.abs() <= MAX_COORD && p.y.abs() <= MAX_COORD;
    match el {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => ok(p),
        PathEl::QuadTo(a, b) => ok(a) && ok(b),
        PathEl::CurveTo(a, b, c) => ok(a) && ok(b) && ok(c),
        PathEl::ClosePath => true,
    }
}

/// The path split at its `MoveTo`s.
fn split_groups(path: &BezPath) -> Vec<Vec<PathEl>> {
    let mut groups: Vec<Vec<PathEl>> = Vec::new();
    for el in path.elements() {
        match el {
            PathEl::MoveTo(_) => groups.push(vec![*el]),
            _ => {
                if let Some(g) = groups.last_mut() {
                    g.push(*el);
                }
            }
        }
    }
    groups
}

fn end_of(el: &PathEl) -> Option<KPoint> {
    match el {
        PathEl::MoveTo(p) | PathEl::LineTo(p) | PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => Some(*p),
        PathEl::ClosePath => None,
    }
}

/// A rectangle with equal round corners (the arcs as one cubic each) as an
/// arc-box; `None` for anything else.
fn round_box(g: &[PathEl], style: Style) -> Option<Object> {
    let mut pts = vec![];
    let mut curves = 0;
    let mut cur = None;
    let mut segs: Vec<(KPoint, &PathEl)> = Vec::new();
    for el in g {
        match el {
            PathEl::MoveTo(p) => {
                if cur.is_some() {
                    return None;
                }
                cur = Some(*p);
                pts.push(*p);
            }
            PathEl::LineTo(p) => {
                let from = cur?;
                // A zero-length closing line is no side.
                if (p.x - from.x).hypot(p.y - from.y) >= 0.5 {
                    segs.push((from, el));
                }
                cur = Some(*p);
            }
            PathEl::CurveTo(_, _, p) => {
                segs.push((cur?, el));
                curves += 1;
                cur = Some(*p);
            }
            PathEl::QuadTo(..) => return None,
            PathEl::ClosePath => {}
        }
    }
    if curves != 4 || segs.len() > 8 || segs.len() < 5 {
        return None;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    let mut radius = 0.0;
    for (from, el) in &segs {
        let to = end_of(el)?;
        for p in [from, &to] {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x);
            y1 = y1.max(p.y);
        }
        match el {
            PathEl::LineTo(_) => {
                if (to.x - from.x).abs() > 1.0 && (to.y - from.y).abs() > 1.0 {
                    return None;
                }
            }
            PathEl::CurveTo(a, b, _) => {
                let (dx, dy) = ((to.x - from.x).abs(), (to.y - from.y).abs());
                if (dx - dy).abs() > 1.5 || dx < 1.0 {
                    return None;
                }
                let kappa = 0.552_284_75 * dx;
                let ca = (
                    (a.x - from.x).abs().max((a.y - from.y).abs()),
                    (a.x - from.x).abs().min((a.y - from.y).abs()),
                );
                let cb = (
                    (b.x - to.x).abs().max((b.y - to.y).abs()),
                    (b.x - to.x).abs().min((b.y - to.y).abs()),
                );
                for c in [ca, cb] {
                    if (c.0 - kappa).abs() > 0.04 * dx + 1.0 || c.1 > 1.0 {
                        return None;
                    }
                }
                if radius == 0.0 {
                    radius = dx;
                } else if (radius - dx).abs() > 1.5 {
                    return None;
                }
            }
            _ => return None,
        }
    }
    let _ = pts;
    if radius < 1.0 || x1 - x0 < 2.0 * radius - 1.5 || y1 - y0 < 2.0 * radius - 1.5 {
        return None;
    }
    // Every side and arc starts and ends on the bounding box.
    let on_box = |p: KPoint| {
        ((p.x - x0).abs() < 1.0 || (p.x - x1).abs() < 1.0) || ((p.y - y0).abs() < 1.0 || (p.y - y1).abs() < 1.0)
    };
    if !segs.iter().all(|(f, e)| on_box(*f) && end_of(e).is_some_and(on_box)) {
        return None;
    }
    let mut pl = Polyline::rect(
        style,
        Point::new(x0.round(), y0.round()),
        Point::new(x1.round(), y1.round()),
    );
    pl.kind = PolyKind::ArcBox;
    // Fig's radius is in 1/80 inch (15 Fig units).
    pl.radius = ((radius / 15.0).round() as i32).max(1);
    Some(Object::new(ObjectKind::Polyline(pl)))
}

/// A closed path of curves only that is an ellipse (any rotation) as an
/// ellipse object; `None` for anything else.
fn ellipse_object(g: &[PathEl], style: Style) -> Option<Object> {
    let mut curves = 0;
    let mut start = None;
    let mut last = None;
    for el in g {
        match el {
            PathEl::MoveTo(p) => {
                if start.is_some() {
                    return None;
                }
                start = Some(*p);
                last = Some(*p);
            }
            PathEl::CurveTo(_, _, p) | PathEl::QuadTo(_, p) => {
                curves += 1;
                last = Some(*p);
            }
            PathEl::LineTo(p) => {
                // Only a zero-length closing line.
                let from = last?;
                if (p.x - from.x).hypot(p.y - from.y) >= 0.5 {
                    return None;
                }
                last = Some(*p);
            }
            PathEl::ClosePath => {}
        }
    }
    let (start, last) = (start?, last?);
    if curves < 4 || (start.x - last.x).hypot(start.y - last.y) > 1.0 {
        return None;
    }
    let mut pts: Vec<(f64, f64)> = Vec::new();
    hayro::kurbo::flatten(g.iter().copied(), 0.1, |el| {
        if let PathEl::MoveTo(p) | PathEl::LineTo(p) = el {
            if pts.len() < 20_000 {
                pts.push((p.x, p.y));
            }
        }
    });
    if pts.len() < 8 {
        return None;
    }
    let (x0, x1) = pts
        .iter()
        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
    let (y0, y1) = pts
        .iter()
        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    // Least squares for a x^2 + 2 b x y + c y^2 = 1 around the centre.
    let (mut s, mut r) = ([[0.0f64; 3]; 3], [0.0f64; 3]);
    for &(px, py) in &pts {
        let (u, v) = (px - cx, py - cy);
        let f = [u * u, 2.0 * u * v, v * v];
        for i in 0..3 {
            for j in 0..3 {
                s[i][j] += f[i] * f[j];
            }
            r[i] += f[i];
        }
    }
    let sol = solve3(s, r)?;
    let (a, b, c) = (sol[0], sol[1], sol[2]);
    let mean = (a + c) / 2.0;
    let disc = (((a - c) / 2.0).powi(2) + b * b).sqrt();
    let (l_big, l_small) = (mean + disc, mean - disc);
    if !(l_small > 0.0 && l_big.is_finite()) {
        return None;
    }
    let (r_major, r_minor) = (1.0 / l_small.sqrt(), 1.0 / l_big.sqrt());
    // Major axis direction (eigenvector of the small eigenvalue), y down.
    let phi = if b.abs() < 1e-18 && a <= c {
        0.0
    } else if b.abs() < 1e-18 {
        std::f64::consts::FRAC_PI_2
    } else {
        (l_small - a).atan2(b)
    };
    // Every point lies on the fitted ellipse.
    for &(px, py) in &pts {
        let (u, v) = (px - cx, py - cy);
        let q = a * u * u + 2.0 * b * u * v + c * v * v;
        if !q.is_finite() || (q.sqrt() - 1.0).abs() * u.hypot(v) > 1.2 {
            return None;
        }
    }
    let (mut rx, mut ry, mut angle) = (r_major, r_minor, (-phi).rem_euclid(std::f64::consts::PI));
    // Axis-aligned: x radius first, no angle.
    let snap = 0.002;
    if angle < snap || angle > std::f64::consts::PI - snap {
        angle = 0.0;
    } else if (angle - std::f64::consts::FRAC_PI_2).abs() < snap {
        angle = 0.0;
        std::mem::swap(&mut rx, &mut ry);
    }
    let (rx, ry) = (rx.round().max(1.0), ry.round().max(1.0));
    let circle = (rx - ry).abs() <= 1.0;
    let center = Point::new(cx.round(), cy.round());
    Some(Object::new(ObjectKind::Ellipse(Ellipse {
        kind: if circle {
            EllipseKind::CircleRadius
        } else {
            EllipseKind::EllipseRadii
        },
        style,
        direction: 1,
        angle: if circle { 0.0 } else { (angle * 1024.0).round() / 1024.0 },
        center,
        radii: if circle { Point::new(rx, rx) } else { Point::new(rx, ry) },
        start: center,
        end: Point::new(center.x + rx, center.y + if circle { rx } else { ry }),
    })))
}

/// Cramer's rule for a 3x3 system; `None` when it is singular.
fn solve3(m: [[f64; 3]; 3], r: [f64; 3]) -> Option<[f64; 3]> {
    let det = |a: &[[f64; 3]; 3]| {
        a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1]) - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
            + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
    };
    let d = det(&m);
    if !d.is_finite() || d.abs() < 1e-30 {
        return None;
    }
    let mut out = [0.0; 3];
    for (k, o) in out.iter_mut().enumerate() {
        let mut t = m;
        for i in 0..3 {
            t[i][k] = r[i];
        }
        *o = det(&t) / d;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_is_an_error() {
        assert!(import(b"<svg", "x", None).is_err());
        assert!(import(b"\x1f\x8bnot gzip", "x", None).is_err());
        assert!(!is_svg(b"#FIG 3.2"));
        assert!(is_svg(
            b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"/>"
        ));
    }

    #[test]
    fn a_rect_and_a_circle() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96">
            <rect x="10" y="10" width="20" height="10" fill="#ff0000"/>
            <circle cx="50" cy="50" r="10" fill="none" stroke="#000"/></svg>"##;
        let r = import(svg, "t", None).unwrap();
        assert_eq!(r.doc.objects.len(), 2);
    }
}
