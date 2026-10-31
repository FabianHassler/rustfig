//! PDF import (plan G5): a PDF page as editable Fig objects.
//!
//! hayro's interpreter walks the page and calls [`Importer`] (a hayro
//! `Device`) for every path, glyph run and image; they become Fig lines,
//! polygons, boxes, ellipses, texts and pictures. Curves are flattened
//! (tolerance one Fig unit) except for axis-aligned ellipses; clip paths and
//! transparency are dropped with one warning each. A PDF is untrusted input:
//! object, point and image sizes are capped and everything returns `Result`.

use hayro::hayro_interpret::font::{FontQuery, Glyph, GlyphRun};
use hayro::hayro_interpret::hayro_syntax::Pdf;
use hayro::hayro_interpret::util::TransformExt;
use hayro::hayro_interpret::{
    interpret_page, BlendMode, ClipPath, Context, Device, DrawMode, DrawProps, Image, ImageDrawProps, InterpreterCache,
    InterpreterSettings, Paint, SoftMask, StrokeProps,
};
use hayro::kurbo::{Affine, BezPath, Cap, Join, PathEl, Point as KPoint, Rect as KRect, Shape};
use rustfig_geom::Point;
use rustfig_model::{
    text_flags, CapStyle, ColorRef, Document, Ellipse, EllipseKind, JoinStyle, LineKind, Object, ObjectKind, Picture,
    PolyKind, Polyline, Rgb, Style, Text,
};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// Most objects one import creates.
pub const MAX_OBJECTS: usize = 200_000;
/// Most points in one path.
pub const MAX_PATH_POINTS: usize = 20_000;
/// Most pages looked at when counting.
const MAX_PAGES: usize = 100_000;
/// Fig units per PDF point (1200 per inch, 72 points per inch).
const K: f64 = 1200.0 / 72.0;
/// Coordinates beyond this (Fig units) are not drawn.
const MAX_COORD: f64 = 1.0e7;
/// Largest picture the import keeps, in pixels per edge.
const MAX_IMAGE_EDGE: u32 = crate::import::MAX_PICTURE_EDGE;
/// Most image pixels one import keeps in all (memory cap).
const MAX_IMAGE_PIXELS: u64 = 100_000_000;

/// What the import made.
pub struct Imported {
    /// The figure: objects and user colours.
    pub doc: Document,
    /// The page's pictures as PNG files: (file name, bytes); the picture
    /// objects name them.
    pub images: Vec<(String, Vec<u8>)>,
    /// What was dropped or approximated.
    pub warnings: Vec<String>,
}

fn open(bytes: &[u8]) -> Result<Pdf, String> {
    Pdf::new(Arc::new(bytes.to_vec())).map_err(|_| "the PDF cannot be read (damaged or encrypted)".to_string())
}

/// Whether the data looks like a PDF (the signature within the first KB).
pub fn is_pdf(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .take(1024)
        .collect::<Vec<_>>()
        .windows(5)
        .any(|w| w.iter().copied().eq(b"%PDF-".iter()))
}

/// The number of pages of a PDF.
pub fn page_count(bytes: &[u8]) -> Result<usize, String> {
    crate::guard::catch(|| {
        let pdf = open(bytes)?;
        let n = pdf.pages().len();
        if n == 0 || n > MAX_PAGES {
            return Err("the PDF has no readable pages".to_string());
        }
        Ok(n)
    })
    .map_err(|e| format!("the PDF cannot be read ({e})"))?
}

/// Page `page` (1-based) of a PDF as Fig objects. `stem` names the pictures
/// (`stem-p1-1.png`).
pub fn import(bytes: &[u8], page: usize, stem: &str) -> Result<Imported, String> {
    crate::guard::catch(|| import_inner(bytes, page, stem)).map_err(|e| format!("the PDF cannot be imported ({e})"))?
}

fn import_inner(bytes: &[u8], page: usize, stem: &str) -> Result<Imported, String> {
    let pdf = open(bytes)?;
    let pages = pdf.pages();
    if pages.len() > MAX_PAGES {
        return Err("the PDF has too many pages".into());
    }
    let p = pages
        .get(page.max(1) - 1)
        .ok_or_else(|| format!("the PDF has {} pages, not {page}", pages.len()))?;
    let (w, h) = p.render_dimensions();
    if !(w.is_finite() && h.is_finite() && w >= 1.0 && h >= 1.0 && w <= 200_000.0 && h <= 200_000.0) {
        return Err("the page size is not usable".into());
    }
    let init = p.initial_transform(true).to_kurbo();
    let cache = InterpreterCache::new();
    // hayro tells a device nothing about fonts it draws from its own copies
    // of the 14 standard fonts (or substitutes); the resolver sees their
    // names when it loads them, in the order the page first uses them.
    let names: Arc<Mutex<VecDeque<String>>> = Arc::default();
    let mut settings = InterpreterSettings::default();
    let inner = settings.font_resolver.clone();
    let seen = names.clone();
    settings.font_resolver = Arc::new(move |q| {
        let name = match q {
            FontQuery::Standard(s) => Some(s.postscript_name().to_string()),
            FontQuery::Fallback(f) => f.post_script_name.clone().or_else(|| f.font_name.clone()),
        };
        if let (Some(n), Ok(mut v)) = (name, seen.lock()) {
            if v.len() < 1000 {
                v.push_back(n);
            }
        }
        inner(q)
    });
    let mut ctx = Context::new(
        init,
        KRect::new(0.0, 0.0, f64::from(w), f64::from(h)),
        &cache,
        p.xref(),
        settings,
    );
    let mut imp = Importer::new(stem, page, KRect::new(0.0, 0.0, f64::from(w), f64::from(h)));
    imp.font_names = names;
    interpret_page(p, &mut ctx, &mut imp);
    Ok(imp.finish())
}

/// A text collected from glyph runs, waiting to be joined with the next run.
struct PendingText {
    text: Text,
    /// Where the next glyph would start, in Fig units.
    next: (f64, f64),
    /// Baseline direction.
    dir: (f64, f64),
    /// Type 3 glyphs have no advance: never joined.
    open: bool,
}

struct Importer {
    doc: Document,
    objects: Vec<Object>,
    images: Vec<(String, Vec<u8>)>,
    warnings: Vec<String>,
    warned: Vec<&'static str>,
    page: KRect,
    stem: String,
    page_no: usize,
    pending: Option<PendingText>,
    image_pixels: u64,
    /// The depth the next object goes to and the highest xfig type rank on it.
    depth: i32,
    rank: u8,
    out_of_colours: bool,
    capped: bool,
    /// Font names the resolver saw and the fig font chosen per hayro font.
    font_names: Arc<Mutex<VecDeque<String>>>,
    fonts: HashMap<u128, (i32, bool)>,
    /// Active clip paths in Fig units: convex polygons (counter-clockwise),
    /// or `None` for one that can't be applied.
    clips: Vec<Option<Vec<(f64, f64)>>>,
}

fn fig(p: KPoint) -> (f64, f64) {
    (p.x * K, p.y * K)
}

impl Importer {
    fn new(stem: &str, page_no: usize, page: KRect) -> Importer {
        Importer {
            doc: Document::new(),
            objects: Vec::new(),
            images: Vec::new(),
            warnings: Vec::new(),
            warned: Vec::new(),
            page,
            stem: stem.to_string(),
            page_no,
            pending: None,
            image_pixels: 0,
            depth: 999,
            rank: 0,
            out_of_colours: false,
            capped: false,
            font_names: Arc::default(),
            fonts: HashMap::new(),
            clips: Vec::new(),
        }
    }

    fn warn_once(&mut self, key: &'static str, msg: &str) {
        if !self.warned.contains(&key) {
            self.warned.push(key);
            self.warnings.push(msg.to_string());
        }
    }

    fn finish(mut self) -> Imported {
        self.flush_text();
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

    /// The colour of a paint and whether it is visible; patterns and
    /// gradients become grey with a warning.
    fn paint_rgb(&mut self, paint: &Paint<'_>) -> Option<Rgb> {
        match paint {
            Paint::Color(c) => {
                let [r, g, b, a] = c.to_rgba().to_rgba8();
                if a == 0 {
                    return None;
                }
                if a < 255 {
                    self.warn_once("alpha", "Transparency is ignored: translucent shapes are drawn solid");
                }
                Some(Rgb::new(r, g, b))
            }
            Paint::Pattern(_) => {
                self.warn_once("pattern", "Patterns and gradients are drawn in grey");
                Some(Rgb::new(128, 128, 128))
            }
        }
    }

    fn flush_text(&mut self) {
        if let Some(p) = self.pending.take() {
            if !p.text.text.trim().is_empty() {
                self.push(Object::new(ObjectKind::Text(p.text)));
            }
        }
    }

    fn group_warning(&mut self, props_mask: bool, blend: BlendMode) {
        if props_mask || blend != BlendMode::Normal {
            self.warn_once("alpha", "Transparency is ignored: translucent shapes are drawn solid");
        }
    }

    fn style_of(&mut self, scale: f64, paint: &Paint<'_>, mode: &DrawMode) -> Option<(Style, bool, bool)> {
        let (fill, stroke) = match mode {
            DrawMode::Fill(_) => (true, None),
            DrawMode::Stroke(s) => (false, Some(s)),
            DrawMode::FillAndStroke(_, s) => (true, Some(s)),
            DrawMode::Invisible => return None,
        };
        let rgb = self.paint_rgb(paint)?;
        let color = self.color(rgb);
        let mut st = Style {
            pen_color: color,
            fill_color: color,
            thickness: 0,
            ..Style::default()
        };
        if fill {
            st.area_fill = 20;
        }
        if let Some(s) = stroke {
            apply_stroke(&mut st, s, scale);
        }
        Some((st, fill, stroke.is_some()))
    }

    fn path(&mut self, path: &BezPath, props: &DrawProps<'_>, mode: &DrawMode) {
        self.flush_text();
        let t = props.transform;
        let c = t.as_coeffs();
        let scale = (c[0] * c[3] - c[1] * c[2]).abs().sqrt();
        let Some((style, fill, _)) = self.style_of(scale, &props.paint, mode) else {
            return;
        };
        self.group_warning(props.soft_mask.is_some(), props.blend_mode);
        let mut p = path.clone();
        p.apply_affine(Affine::scale(K) * t);
        let (subpaths, ok) = split_subpaths(&p);
        if !ok {
            return;
        }
        if subpaths.len() > 1 && fill {
            self.warn_once(
                "holes",
                "Filled shapes with holes become separate shapes (Fig has no holes)",
            );
        }
        for sp in subpaths {
            for sp in self.clipped(sp, fill) {
                if let Some(o) = subpath_object(&sp, style, fill) {
                    self.push(o);
                }
            }
        }
    }

    /// A subpath cut to the active clip paths: filled shapes by
    /// Sutherland-Hodgman, open lines by parametric clipping.
    fn clipped(&mut self, sp: SubPath, fill: bool) -> Vec<SubPath> {
        let polys: Vec<Vec<(f64, f64)>> = self.clips.iter().flatten().cloned().collect();
        if polys.is_empty() {
            return vec![sp];
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for &(x, y) in &sp.points {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        let inside = polys.iter().all(|c| {
            // Inside when the bounding box's corners are all inside the convex clip.
            [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
                .iter()
                .all(|&p| inside_convex(c, p))
        });
        if inside {
            return vec![sp];
        }
        let closed = sp.closed || fill;
        let mut parts: Vec<(Vec<(f64, f64)>, bool)> = vec![(sp.points, closed)];
        for c in &polys {
            let mut next = Vec::new();
            for (pts, closed) in parts {
                if closed {
                    let q = clip_polygon(&pts, c);
                    if q.len() >= 3 {
                        next.push((q, true));
                    }
                } else {
                    next.extend(clip_polyline(&pts, c).into_iter().map(|l| (l, false)));
                }
            }
            parts = next;
        }
        parts
            .into_iter()
            .map(|(points, closed)| SubPath {
                points,
                closed,
                ellipse: None,
            })
            .collect()
    }

    fn glyph_outline(&mut self, outline: &BezPath, m: Affine, props: &DrawProps<'_>, mode: &DrawMode) {
        let mut p = outline.clone();
        p.apply_affine(m);
        let (rgb_style, fill, _) = match self.style_of(1.0, &props.paint, mode) {
            Some(s) => s,
            None => return,
        };
        let (subpaths, ok) = split_subpaths(&{
            let mut q = p;
            q.apply_affine(Affine::scale(K));
            q
        });
        if !ok {
            return;
        }
        for sp in subpaths {
            if let Some(o) = subpath_object(&sp, rgb_style, fill) {
                self.push(o);
            }
        }
    }

    fn text_run(&mut self, run: &GlyphRun<'_, '_>, props: &DrawProps<'_>) -> bool {
        // Text needs a Unicode value for every glyph and only outline fonts.
        let mut s = String::new();
        let mut first: Option<(Affine, Option<hayro::hayro_interpret::font::OutlineFontData>, u128)> = None;
        let mut end = (0.0, 0.0);
        let mut advance_known = true;
        for g in run.glyphs() {
            let Glyph::Outline(og) = &**g else {
                return false;
            };
            let Some(u) = og.as_unicode() else {
                return false;
            };
            let u: String = match u {
                hayro::hayro_interpret::hayro_cmap::BfString::Char(c) => c.to_string(),
                hayro::hayro_interpret::hayro_cmap::BfString::String(s) => s,
            };
            if u.chars().any(|c| c == '\u{fffd}') {
                return false;
            }
            for ch in u.chars() {
                if ch.is_control() {
                    continue;
                }
                s.push(ch);
            }
            let m = props.transform * g.transform();
            if first.is_none() {
                first = Some((m, og.font_data(), og.font_cache_key()));
            }
            match og.advance_width() {
                Some(a) if a.is_finite() => {
                    let o = m * KPoint::new(f64::from(a), 0.0);
                    end = fig(o);
                }
                _ => advance_known = false,
            }
        }
        let Some((m, font, key)) = first else {
            return true;
        };
        if s.is_empty() {
            return true;
        }
        let Some((rgb, _)) = self.paint_rgb(&props.paint).map(|c| (c, ())) else {
            return true;
        };
        let c = m.as_coeffs();
        let size = c[0].hypot(c[1]) * 1000.0;
        if !(0.5..=2000.0).contains(&size) {
            return false;
        }
        let angle = (-c[1]).atan2(c[0]);
        let angle = if angle < 0.0 {
            angle + std::f64::consts::TAU
        } else {
            angle
        };
        let o = fig(m * KPoint::new(0.0, 0.0));
        if o.0.abs() > MAX_COORD || o.1.abs() > MAX_COORD {
            return true;
        }
        let (ps_font, ps) = match self.fonts.get(&key) {
            Some(f) => *f,
            None => {
                let queued = if font.is_none() {
                    self.font_names.lock().ok().and_then(|mut q| q.pop_front())
                } else {
                    None
                };
                let f = map_font(font.as_ref(), queued.as_deref());
                self.fonts.insert(key, f);
                f
            }
        };
        let color = self.color(rgb);
        let dir = (angle.cos(), -angle.sin());
        let size = (size * 10.0).round() / 10.0;
        if s.chars().any(|c| c as u32 > 0xFF) {
            self.warn_once(
                "latin1",
                "Characters outside Latin-1 are kept, but Fig's PostScript fonts cannot show them",
            );
        }
        // Join onto the text before it when it continues on the same baseline.
        if let Some(p) = &mut self.pending {
            let t = &p.text;
            let same = p.open
                && t.font == ps_font
                && (t.size - size).abs() < 0.05
                && (t.angle - angle).abs() < 1e-3
                && t.color == color;
            if same {
                let dx = o.0 - p.next.0;
                let dy = o.1 - p.next.1;
                let along = dx * p.dir.0 + dy * p.dir.1;
                let across = -dx * p.dir.1 + dy * p.dir.0;
                let em = size * K;
                if across.abs() < 0.1 * em && along > -0.1 * em && along < 0.6 * em {
                    if along > 0.15 * em && !p.text.text.ends_with(' ') && !s.starts_with(' ') {
                        p.text.text.push(' ');
                    }
                    p.text.text.push_str(&s);
                    p.next = end;
                    p.open = advance_known;
                    return true;
                }
            }
        }
        self.flush_text();
        let text = Text {
            justify: rustfig_model::Justify::Left,
            color,
            depth: 50,
            pen_style: -1,
            font: ps_font,
            size,
            angle,
            flags: if ps { text_flags::POSTSCRIPT } else { 0 },
            height: 0.0,
            length: 0.0,
            origin: Point::new(o.0.round(), o.1.round()),
            text: s,
            local_font: None,
        };
        self.pending = Some(PendingText {
            text,
            next: end,
            dir,
            open: advance_known,
        });
        true
    }

    fn picture(&mut self, rgba: image::RgbaImage, t: Affine) {
        self.flush_text();
        let (w, h) = (rgba.width(), rgba.height());
        if w == 0 || h == 0 || w > MAX_IMAGE_EDGE || h > MAX_IMAGE_EDGE {
            self.warn_once("imgsize", "Images that are too large were skipped");
            return;
        }
        self.image_pixels += u64::from(w) * u64::from(h);
        if self.image_pixels > MAX_IMAGE_PIXELS {
            self.warn_once("imgsize", "Images that are too large were skipped");
            return;
        }
        let m = Affine::scale(K) * t;
        let pts = [
            m * KPoint::new(0.0, 0.0),
            m * KPoint::new(f64::from(w), 0.0),
            m * KPoint::new(f64::from(w), f64::from(h)),
            m * KPoint::new(0.0, f64::from(h)),
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
            if c[0] < 0.0 {
                flipped = true;
            }
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
        let name = format!("{}-p{}-{}.png", self.stem, self.page_no, self.images.len() + 1);
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

impl<'a> Device<'a> for Importer {
    fn draw_path(&mut self, path: &BezPath, props: DrawProps<'a>, draw_mode: &DrawMode) {
        self.path(path, &props, draw_mode);
    }

    fn push_clip_path(&mut self, clip_path: &ClipPath) {
        let bbox = clip_path.path.bounding_box();
        let p = self.page;
        let covers = bbox.x0 <= p.x0 + 1.0 && bbox.y0 <= p.y0 + 1.0 && bbox.x1 >= p.x1 - 1.0 && bbox.y1 >= p.y1 - 1.0;
        if covers {
            self.clips.push(None);
            return;
        }
        // The clip is in page points: Fig units, flattened, convex only.
        let mut path = clip_path.path.clone();
        path.apply_affine(Affine::scale(K));
        let (subs, ok) = split_subpaths(&path);
        let poly = match subs.as_slice() {
            [one] if ok => convex_ccw(&one.points),
            _ => None,
        };
        if poly.is_none() {
            self.warn_once(
                "clip",
                "Clipping to a concave shape is ignored: clipped drawings are shown whole",
            );
        }
        self.clips.push(poly);
    }

    fn push_clip_rect(&mut self, rect: &KRect) {
        self.push_clip_path(&ClipPath {
            path: rect.to_path(0.1),
            fill: hayro::hayro_interpret::FillRule::NonZero,
        });
    }

    fn push_transparency_group(&mut self, opacity: f32, mask: Option<SoftMask<'a>>, blend_mode: BlendMode) {
        if opacity < 1.0 || mask.is_some() || blend_mode != BlendMode::Normal {
            self.warn_once("alpha", "Transparency is ignored: translucent shapes are drawn solid");
        }
    }

    fn draw_glyph_run(&mut self, glyph_run: &GlyphRun<'_, 'a>, props: DrawProps<'a>, draw_mode: &DrawMode) {
        if matches!(draw_mode, DrawMode::Invisible) {
            return;
        }
        if matches!(draw_mode, DrawMode::Fill(_)) && self.text_run(glyph_run, &props) {
            return;
        }
        self.flush_text();
        self.warn_once("glyphs", "Text without readable characters became outlines");
        for g in glyph_run.glyphs() {
            match &**g {
                Glyph::Outline(og) => {
                    let m = props.transform * g.transform();
                    self.glyph_outline(&og.outline(), m, &props, draw_mode);
                }
                Glyph::Type3(t3) => {
                    t3.interpret(self, props.transform, g.transform(), &props.paint);
                }
            }
        }
    }

    fn draw_image(&mut self, image: Image<'a, '_>, props: ImageDrawProps<'a>) {
        self.group_warning(props.soft_mask.is_some(), props.blend_mode);
        match image {
            Image::Raster(r) => {
                let mut out: Option<(image::RgbaImage, Affine)> = None;
                r.with_rgba(
                    |data, alpha| {
                        let (sx, sy) = data.scale_factors();
                        let t = props.transform * Affine::scale_non_uniform(f64::from(sx), f64::from(sy));
                        let (w, h) = (data.width(), data.height());
                        if w == 0 || h == 0 || w > MAX_IMAGE_EDGE || h > MAX_IMAGE_EDGE {
                            return;
                        }
                        let n = (w as usize) * (h as usize);
                        let mut buf = Vec::with_capacity(n * 4);
                        let alpha = alpha.filter(|a| a.width == w && a.height == h && a.data.len() == n);
                        match &data {
                            hayro::hayro_interpret::ImageData::Rgb(d) if d.data.len() == n * 3 => {
                                for (i, px) in d.data.as_chunks::<3>().0.iter().enumerate() {
                                    let a = alpha.as_ref().and_then(|a| a.data.get(i)).copied().unwrap_or(255);
                                    buf.extend_from_slice(&[px[0], px[1], px[2], a]);
                                }
                            }
                            hayro::hayro_interpret::ImageData::Luma(d) if d.data.len() == n => {
                                for (i, g) in d.data.iter().enumerate() {
                                    let a = alpha.as_ref().and_then(|a| a.data.get(i)).copied().unwrap_or(255);
                                    buf.extend_from_slice(&[*g, *g, *g, a]);
                                }
                            }
                            _ => return,
                        }
                        if let Some(img) = image::RgbaImage::from_raw(w, h, buf) {
                            out = Some((img, t));
                        }
                    },
                    None,
                );
                if let Some((img, t)) = out {
                    self.picture(img, t);
                }
            }
            Image::Stencil(s) => {
                let mut out: Option<(image::RgbaImage, Affine)> = None;
                let mut color = None;
                if let Some(c) = match &s_paint(&s) {
                    Some(p) => self.paint_rgb(p),
                    None => None,
                } {
                    color = Some(c);
                }
                let Some(color) = color else {
                    return;
                };
                s.with_stencil(
                    |luma, _| {
                        let t = props.transform
                            * Affine::scale_non_uniform(
                                f64::from(luma.scale_factors.0),
                                f64::from(luma.scale_factors.1),
                            );
                        let (w, h) = (luma.width, luma.height);
                        let n = (w as usize) * (h as usize);
                        if w == 0 || h == 0 || w > MAX_IMAGE_EDGE || h > MAX_IMAGE_EDGE || luma.data.len() != n {
                            return;
                        }
                        let mut buf = Vec::with_capacity(n * 4);
                        for v in &luma.data {
                            buf.extend_from_slice(&[color.r, color.g, color.b, *v]);
                        }
                        if let Some(img) = image::RgbaImage::from_raw(w, h, buf) {
                            out = Some((img, t));
                        }
                    },
                    None,
                );
                if let Some((img, t)) = out {
                    self.picture(img, t);
                }
            }
        }
    }

    fn pop_clip(&mut self) {
        self.clips.pop();
    }

    fn pop_transparency_group(&mut self) {}

    fn draw_rect(&mut self, rect: &KRect, props: DrawProps<'a>, draw_mode: &DrawMode) {
        self.path(&rect.to_path(0.1), &props, draw_mode);
    }
}

/// The polygon (without a repeated end point) as counter-clockwise when it
/// is convex, else `None`.
fn convex_ccw(pts: &[(f64, f64)]) -> Option<Vec<(f64, f64)>> {
    let mut p: Vec<(f64, f64)> = pts.to_vec();
    if p.len() > 1 && p.first() == p.last() {
        p.pop();
    }
    if p.len() < 3 {
        return None;
    }
    let n = p.len();
    let cross = |i: usize| {
        let (a, b, c) = (p[i], p[(i + 1) % n], p[(i + 2) % n]);
        (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0)
    };
    let (mut pos, mut neg) = (false, false);
    for i in 0..n {
        let c = cross(i);
        if c > 1e-6 {
            pos = true;
        } else if c < -1e-6 {
            neg = true;
        }
    }
    if pos && neg {
        return None;
    }
    // y grows downwards, but only consistency matters: make `cross > 0` the inside.
    if neg {
        p.reverse();
    }
    Some(p)
}

/// Signed distance-like side of `p` from edge a->b: >= 0 is inside.
fn side(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> f64 {
    (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)
}

fn inside_convex(c: &[(f64, f64)], p: (f64, f64)) -> bool {
    let n = c.len();
    (0..n).all(|i| side(c[i], c[(i + 1) % n], p) >= -1e-6)
}

fn clip_polygon(pts: &[(f64, f64)], c: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = pts.to_vec();
    let n = c.len();
    for i in 0..n {
        let (a, b) = (c[i], c[(i + 1) % n]);
        let input = std::mem::take(&mut out);
        let m = input.len();
        for j in 0..m {
            let (p, q) = (input[j], input[(j + 1) % m]);
            let (sp, sq) = (side(a, b, p), side(a, b, q));
            if sp >= 0.0 {
                out.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                let t = sp / (sp - sq);
                out.push((p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1)));
            }
        }
        if out.is_empty() {
            break;
        }
    }
    out
}

fn clip_polyline(pts: &[(f64, f64)], c: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
    let mut lines: Vec<Vec<(f64, f64)>> = Vec::new();
    let n = c.len();
    for w in pts.windows(2) {
        let (p, q) = (w[0], w[1]);
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        let mut ok = true;
        for i in 0..n {
            let (a, b) = (c[i], c[(i + 1) % n]);
            let (sp, sq) = (side(a, b, p), side(a, b, q));
            if sp < 0.0 && sq < 0.0 {
                ok = false;
                break;
            }
            if sp < 0.0 {
                t0 = t0.max(sp / (sp - sq));
            } else if sq < 0.0 {
                t1 = t1.min(sp / (sp - sq));
            }
        }
        if !ok || t0 > t1 {
            continue;
        }
        let at = |t: f64| (p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1));
        let (a, b) = (at(t0), at(t1));
        match lines.last_mut() {
            Some(l) if t0 == 0.0 && l.last() == Some(&p) => l.push(b),
            _ => lines.push(vec![a, b]),
        }
    }
    lines
}

/// The paint of a stencil image.
fn s_paint<'a>(s: &hayro::hayro_interpret::StencilImage<'a, '_>) -> Option<Paint<'a>> {
    let mut p = None;
    s.with_stencil(|_, paint| p = Some(paint.clone()), Some((1, 1)));
    p
}

/// Stroke width, dashes, caps and joins as Fig attributes. `scale` takes the
/// line width from user space to points.
fn apply_stroke(st: &mut Style, s: &StrokeProps, scale: f64) {
    let w_pt = f64::from(s.line_width) * scale;
    let w_in = if w_pt.is_finite() { w_pt / 72.0 } else { 0.0 };
    // geom::stroke_width: thickness 1 is 1/160 inch, t above 1 is (t - 1)/80.
    st.thickness = if w_in <= 0.0 {
        1
    } else {
        ((w_in * 80.0).round() as i32 + 1).clamp(1, 500)
    };
    if w_in < 1.5 / 160.0 {
        st.thickness = 1;
    }
    st.cap = match s.line_cap {
        Cap::Butt => CapStyle::Butt,
        Cap::Round => CapStyle::Round,
        Cap::Square => CapStyle::Projecting,
    };
    st.join = match s.line_join {
        Join::Miter => JoinStyle::Miter,
        Join::Round => JoinStyle::Round,
        Join::Bevel => JoinStyle::Bevel,
    };
    let dash: Vec<f64> = s
        .dash_array
        .iter()
        .map(|d| f64::from(*d) * scale)
        .filter(|d| d.is_finite() && *d > 0.0)
        .collect();
    if let Some(&d) = dash.first() {
        let gap = dash.get(1).copied().unwrap_or(d);
        let wp = w_pt.max(0.01);
        let to_80 = |pt: f64| (pt / 72.0 * 80.0).clamp(1.0, 10_000.0);
        if d <= 1.5 * wp {
            st.line = LineKind::Dotted;
            st.style_val = to_80(d + gap).round();
        } else {
            st.line = LineKind::Dashed;
            st.style_val = to_80(d).round();
        }
    }
}

/// A subpath in Fig units: its flattened points, whether it was closed and
/// the ellipse it draws when it is an axis-aligned one.
struct SubPath {
    points: Vec<(f64, f64)>,
    closed: bool,
    ellipse: Option<((f64, f64), f64, f64)>,
}

/// Split a path (already in Fig units) into flattened subpaths. `false`
/// when it has coordinates that are not finite or too large.
fn split_subpaths(path: &BezPath) -> (Vec<SubPath>, bool) {
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
        let pts: &[KPoint] = match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => std::slice::from_ref(p),
            PathEl::QuadTo(a, b) => &[*a, *b],
            PathEl::CurveTo(a, b, c) => &[*a, *b, *c],
            PathEl::ClosePath => &[],
        };
        if pts
            .iter()
            .any(|p| !(p.x.is_finite() && p.y.is_finite()) || p.x.abs() > MAX_COORD || p.y.abs() > MAX_COORD)
        {
            return (Vec::new(), false);
        }
    }
    let mut out = Vec::new();
    for g in groups {
        let closed = matches!(g.last(), Some(PathEl::ClosePath));
        let ellipse = ellipse_of(&g);
        let mut points: Vec<(f64, f64)> = Vec::new();
        let mut capped = false;
        hayro::kurbo::flatten(g.iter().copied(), 1.0, |el| match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => {
                if points.len() < MAX_PATH_POINTS {
                    points.push((p.x, p.y));
                } else {
                    capped = true;
                }
            }
            _ => {}
        });
        out.push(SubPath {
            points,
            closed,
            ellipse,
        });
    }
    (out, true)
}

/// An axis-aligned ellipse drawn as four cubic segments (the usual
/// Bezier circle, control points at 0.5523 of the radius).
fn ellipse_of(g: &[PathEl]) -> Option<((f64, f64), f64, f64)> {
    let mut curves = Vec::new();
    let mut start = None;
    for el in g {
        match el {
            PathEl::MoveTo(p) => start = Some(*p),
            PathEl::CurveTo(a, b, c) => curves.push((*a, *b, *c)),
            PathEl::ClosePath => {}
            _ => return None,
        }
    }
    let start = start?;
    if curves.len() != 4 {
        return None;
    }
    let ends: Vec<KPoint> = curves.iter().map(|c| c.2).collect();
    let all = [start, ends[0], ends[1], ends[2]];
    let (x0, x1) = (
        all.iter().map(|p| p.x).fold(f64::MAX, f64::min),
        all.iter().map(|p| p.x).fold(f64::MIN, f64::max),
    );
    let (y0, y1) = (
        all.iter().map(|p| p.y).fold(f64::MAX, f64::min),
        all.iter().map(|p| p.y).fold(f64::MIN, f64::max),
    );
    let (rx, ry) = ((x1 - x0) / 2.0, (y1 - y0) / 2.0);
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    if rx < 1.0 || ry < 1.0 || (ends[2].x - start.x).abs() > 1.0 && (ends[3].x - start.x).abs() > 1.0 {
        return None;
    }
    if (ends[3].x - start.x).hypot(ends[3].y - start.y) > 1.0 {
        return None;
    }
    // Every segment end sits at an extreme point.
    let on_axis = |p: KPoint| {
        ((p.x - cx).abs() < 1.0 && ((p.y - cy).abs() - ry).abs() < 1.0)
            || ((p.y - cy).abs() < 1.0 && ((p.x - cx).abs() - rx).abs() < 1.0)
    };
    if !all.iter().all(|p| on_axis(*p)) {
        return None;
    }
    // Control points: 0.5523 of the radius along the tangent.
    let kappa = 0.552_284_75;
    let mut prev = start;
    for (a, b, c) in &curves {
        let horizontal_first = (prev.y - cy).abs() < 1.0 && (prev.x - cx).abs() > 1.0;
        let (da, db) = if horizontal_first {
            // leaves along y at a left/right extreme
            ((a.y - prev.y).abs(), (b.x - c.x).abs())
        } else {
            ((a.x - prev.x).abs(), (b.y - c.y).abs())
        };
        let (ra, rb) = if horizontal_first { (ry, rx) } else { (rx, ry) };
        if (da - kappa * ra).abs() > 0.03 * ra + 1.0 || (db - kappa * rb).abs() > 0.03 * rb + 1.0 {
            return None;
        }
        prev = *c;
    }
    Some(((cx, cy), rx, ry))
}

/// The Fig object a subpath makes, if it makes one.
fn subpath_object(sp: &SubPath, style: Style, fill: bool) -> Option<Object> {
    if let Some(((cx, cy), rx, ry)) = sp.ellipse {
        let center = Point::new(cx.round(), cy.round());
        let (rx, ry) = (rx.round().max(1.0), ry.round().max(1.0));
        let circle = (rx - ry).abs() <= 1.0;
        return Some(Object::new(ObjectKind::Ellipse(Ellipse {
            kind: if circle {
                EllipseKind::CircleRadius
            } else {
                EllipseKind::EllipseRadii
            },
            style,
            direction: 1,
            angle: 0.0,
            center,
            radii: if circle { Point::new(rx, rx) } else { Point::new(rx, ry) },
            start: center,
            end: Point::new(center.x + rx, center.y + if circle { rx } else { ry }),
        })));
    }
    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(sp.points.len());
    for &(x, y) in &sp.points {
        let q = (x.round(), y.round());
        if pts.last() != Some(&q) {
            pts.push(q);
        }
    }
    let closed = sp.closed || fill || (pts.len() > 3 && pts.first() == pts.last());
    if closed && pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    if pts.len() < 2 {
        return None;
    }
    let p: Vec<Point> = pts.iter().map(|&(x, y)| Point::new(x, y)).collect();
    if closed && pts.len() == 4 {
        let near = |a: f64, b: f64| (a - b).abs() <= 1.0;
        let (a, b, c, d) = (pts[0], pts[1], pts[2], pts[3]);
        let rect = (near(a.0, b.0) && near(b.1, c.1) && near(c.0, d.0) && near(d.1, a.1))
            || (near(a.1, b.1) && near(b.0, c.0) && near(c.1, d.1) && near(d.0, a.0));
        if rect {
            return Some(Object::new(ObjectKind::Polyline(Polyline::rect(style, p[0], p[2]))));
        }
    }
    let kind = if closed && p.len() >= 3 {
        PolyKind::Polygon
    } else {
        PolyKind::Polyline
    };
    Some(Object::new(ObjectKind::Polyline(Polyline::new(kind, style, p))))
}

/// The Fig font nearest to a PDF font and whether it is a PostScript font:
/// by name where it names one of the 35, else by the font's flags.
fn map_font(font: Option<&hayro::hayro_interpret::font::OutlineFontData>, queued: Option<&str>) -> (i32, bool) {
    let name = font
        .and_then(|f| f.postscript_name.as_deref())
        .or(queued)
        .map(|n| n.split_once('+').map_or(n, |(_, r)| r).to_ascii_lowercase())
        .unwrap_or_default();
    let has = |s: &[&str]| s.iter().any(|k| name.contains(k));
    let bold =
        has(&["bold", "black", "heavy", "demi", "semibold"]) || font.and_then(|f| f.weight).is_some_and(|w| w >= 600);
    let italic = has(&["italic", "oblique", "slanted"]) || font.is_some_and(|f| f.is_italic);
    // (base number, has the four styles)
    let base = if has(&["avantgarde", "gothic"]) {
        4
    } else if has(&["bookman"]) {
        8
    } else if has(&[
        "courier",
        "mono",
        "consolas",
        "typewriter",
        "cmtt",
        "menlo",
        "inconsolata",
    ]) {
        12
    } else if has(&["narrow"]) {
        20
    } else if has(&["newcentury", "century"]) {
        24
    } else if has(&["palatino", "palladio", "palat"]) {
        28
    } else if has(&["symbol"]) {
        return (32, true);
    } else if has(&["dingbats"]) {
        return (34, true);
    } else if has(&["chancery"]) {
        return (33, true);
    } else if has(&[
        "helvetica",
        "arial",
        "sans",
        "cmss",
        "calibri",
        "verdana",
        "swiss",
        "tahoma",
        "segoe",
        "lato",
        "roboto",
    ]) {
        16
    } else if has(&[
        "times", "cmr", "serif", "roman", "georgia", "garamond", "minion", "cambria",
    ]) {
        0
    } else if font.is_some_and(|f| f.is_monospace) {
        12
    } else if font.is_some_and(|f| !f.is_serif) && font.is_some() {
        16
    } else {
        0
    };
    // AvantGarde and Bookman order: book/light, oblique/italic, demi/demi-oblique...
    let n = match base {
        4 | 8 => base + if bold { 2 } else { 0 } + i32::from(italic),
        _ => base + if bold { 2 } else { 0 } + i32::from(italic),
    };
    (n, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_is_an_error() {
        assert!(import(b"%PDF-1.4 garbage", 1, "x").is_err());
        assert!(page_count(b"nothing").is_err());
        assert!(!is_pdf(b"#FIG 3.2"));
        assert!(is_pdf(b"%PDF-1.7\n"));
    }

    #[test]
    fn stroke_widths_follow_fig() {
        let mut st = Style::default();
        apply_stroke(
            &mut st,
            &StrokeProps {
                line_width: 1.0,
                ..StrokeProps::default()
            },
            1.0,
        );
        // 1 pt = 1.11 / 80 inch: thickness 2.
        assert_eq!(st.thickness, 2);
        apply_stroke(
            &mut st,
            &StrokeProps {
                line_width: 0.45,
                ..StrokeProps::default()
            },
            1.0,
        );
        assert_eq!(st.thickness, 1);
    }
}
