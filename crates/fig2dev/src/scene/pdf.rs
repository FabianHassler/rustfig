//! PDF backend (pdf-writer). Text uses the standard 14 PDF fonts; characters
//! beyond WinAnsi are set in Type 3 fonts built from RustFig's glyph outlines.

use super::pattern::{coverage, PatternFrame};
use super::{Fill, PageLayout, Prim, Rgba, Scene, Stroke, SubPath, TextPrim};
use figbase::geom::{Point, Rect, Seg};
use figbase::image::RasterImage;
use figbase::style::{CapStyle, JoinStyle, Justify};
use pdf_writer::types::{LineCapStyle, LineJoinStyle};
use pdf_writer::types::{SystemInfo, UnicodeCmap};
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect as PdfRect, Ref, Str, TextStr};

/// The standard fonts used for text, by resource name.
const BASE14: [(&str, &str); 14] = [
    ("F0", "Times-Roman"),
    ("F1", "Times-Italic"),
    ("F2", "Times-Bold"),
    ("F3", "Times-BoldItalic"),
    ("F4", "Helvetica"),
    ("F5", "Helvetica-Oblique"),
    ("F6", "Helvetica-Bold"),
    ("F7", "Helvetica-BoldOblique"),
    ("F8", "Courier"),
    ("F9", "Courier-Oblique"),
    ("F10", "Courier-Bold"),
    ("F11", "Courier-BoldOblique"),
    ("F12", "Symbol"),
    ("F13", "ZapfDingbats"),
];

fn base14_index(t: &TextPrim) -> usize {
    let req = t.request();
    let style = usize::from(req.italic) + 2 * usize::from(req.bold);
    if t.postscript {
        match t.font {
            32 => return 12,
            34 => return 13,
            12..=15 => return 8 + style,
            4..=7 | 16..=23 => return 4 + style,
            _ => {}
        }
    } else {
        match t.font {
            4 => return 4,
            5 => return 8,
            _ => {}
        }
    }
    style
}

/// The WinAnsi code of a character, if it has one.
fn win_ansi_byte(c: char) -> Option<u8> {
    let u = c as u32;
    if (0x20..0x7f).contains(&u) || (0xa0..=0xff).contains(&u) {
        return u8::try_from(u).ok();
    }
    match c {
        '€' => Some(0x80),
        '‘' => Some(0x91),
        '’' => Some(0x92),
        '“' => Some(0x93),
        '”' => Some(0x94),
        '•' => Some(0x95),
        '–' => Some(0x96),
        '—' => Some(0x97),
        _ => None,
    }
}

/// Encode a string in WinAnsi (other characters become '?'; text that has
/// any is set in a Type 3 font instead, see [`T3Font`]).
fn win_ansi(s: &str) -> Vec<u8> {
    s.chars().map(|c| win_ansi_byte(c).unwrap_or(b'?')).collect()
}

/// A Type 3 font for characters beyond WinAnsi: the glyph outlines of
/// RustFig's own fonts ([`figbase::text`]: craft-fonts when built in, else
/// the fallback faces), 1000 units to the em, with a `ToUnicode` map so the
/// text stays selectable. One font holds up to 255 characters of one Fig
/// font.
struct T3Font {
    font: i32,
    postscript: bool,
    glyphs: Vec<T3Glyph>,
}

struct T3Glyph {
    ch: char,
    /// Advance in glyph units (1000 per em).
    adv: f64,
    /// The glyph's content stream.
    proc_: Vec<u8>,
    /// Bounding box, glyph units.
    bbox: [f32; 4],
}

impl T3Glyph {
    fn new(t: &TextPrim, ch: char) -> T3Glyph {
        use figbase::text::PathOp;
        let em_pt = 1000.0 / figbase::text::FIG_PER_POINT;
        let shaped = figbase::text::shape(&t.request(), em_pt, &ch.to_string());
        let adv = shaped.line_widths.first().copied().unwrap_or(0.0);
        let mut c = Content::new();
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        let mut grow = |x: f64, y: f64| {
            x0 = x0.min(x);
            y0 = y0.min(-y);
            x1 = x1.max(x);
            y1 = y1.max(-y);
        };
        let mut body = Content::new();
        let mut cur = (0.0, 0.0);
        let mut any = false;
        for op in shaped.lines.first().map(Vec::as_slice).unwrap_or(&[]) {
            any = true;
            match *op {
                PathOp::Move(x, y) => {
                    grow(x, y);
                    body.move_to(f(x), f(-y));
                    cur = (x, y);
                }
                PathOp::Line(x, y) => {
                    grow(x, y);
                    body.line_to(f(x), f(-y));
                    cur = (x, y);
                }
                PathOp::Quad(qx, qy, x, y) => {
                    grow(qx, qy);
                    grow(x, y);
                    let (c1x, c1y) = (cur.0 + 2.0 / 3.0 * (qx - cur.0), cur.1 + 2.0 / 3.0 * (qy - cur.1));
                    let (c2x, c2y) = (x + 2.0 / 3.0 * (qx - x), y + 2.0 / 3.0 * (qy - y));
                    body.cubic_to(f(c1x), f(-c1y), f(c2x), f(-c2y), f(x), f(-y));
                    cur = (x, y);
                }
                PathOp::Cubic(ax, ay, bx, by, x, y) => {
                    grow(ax, ay);
                    grow(bx, by);
                    grow(x, y);
                    body.cubic_to(f(ax), f(-ay), f(bx), f(-by), f(x), f(-y));
                    cur = (x, y);
                }
                PathOp::Close => {
                    body.close_path();
                }
            }
        }
        let bbox = if any && x0 <= x1 {
            [f(x0), f(y0), f(x1), f(y1)]
        } else {
            [0.0; 4]
        };
        c.start_shape_glyph(f(adv), bbox[0], bbox[1], bbox[2], bbox[3]);
        if any {
            if shaped.synthetic_bold {
                // The face has no bold: embolden by stroking, as the raster does.
                c.set_line_width(40.0).set_line_join(LineJoinStyle::RoundJoin);
            }
            let mut bytes = c.finish().to_vec();
            bytes.push(b'\n');
            bytes.extend_from_slice(&body.finish());
            bytes.push(b'\n');
            bytes.extend_from_slice(if shaped.synthetic_bold { b"B\n" } else { b"f\n" });
            return T3Glyph {
                ch,
                adv,
                proc_: bytes,
                bbox,
            };
        }
        T3Glyph {
            ch,
            adv,
            proc_: c.finish().to_vec(),
            bbox,
        }
    }
}

fn f(v: f64) -> f32 {
    if v.is_finite() {
        v.clamp(-1.0e7, 1.0e7) as f32
    } else {
        0.0
    }
}

struct Ctx {
    c: Content,
    k: f64,
    min_x: f64,
    max_y: f64,
    t3: Vec<T3Font>,
}

impl Ctx {
    /// The Type 3 font number and code of `ch` in the font of `t`
    /// (registering the glyph); `None` when 255 codes per font run out of
    /// reach (never: a new font is started).
    fn t3_code(&mut self, t: &TextPrim, ch: char) -> (usize, u8) {
        for (i, fnt) in self.t3.iter().enumerate() {
            if fnt.font == t.font && fnt.postscript == t.postscript {
                if let Some(k) = fnt.glyphs.iter().position(|g| g.ch == ch) {
                    return (i, u8::try_from(k + 1).unwrap_or(1));
                }
            }
        }
        let room = self
            .t3
            .iter()
            .position(|fnt| fnt.font == t.font && fnt.postscript == t.postscript && fnt.glyphs.len() < 255);
        let i = room.unwrap_or_else(|| {
            self.t3.push(T3Font {
                font: t.font,
                postscript: t.postscript,
                glyphs: Vec::new(),
            });
            self.t3.len() - 1
        });
        let glyph = T3Glyph::new(t, ch);
        let Some(fnt) = self.t3.get_mut(i) else { return (0, 1) };
        fnt.glyphs.push(glyph);
        (i, u8::try_from(fnt.glyphs.len()).unwrap_or(1))
    }

    fn p(&self, p: Point) -> (f32, f32) {
        (f((p.x - self.min_x) * self.k), f((self.max_y - p.y) * self.k))
    }

    fn path(&mut self, subs: &[SubPath]) {
        for s in subs {
            if let Some(curve) = &s.curve {
                for seg in curve {
                    match *seg {
                        Seg::Move(q) => {
                            let (x, y) = self.p(q);
                            self.c.move_to(x, y);
                        }
                        Seg::Line(q) => {
                            let (x, y) = self.p(q);
                            self.c.line_to(x, y);
                        }
                        Seg::Cubic(a, b, q) => {
                            let ((x1, y1), (x2, y2), (x3, y3)) = (self.p(a), self.p(b), self.p(q));
                            self.c.cubic_to(x1, y1, x2, y2, x3, y3);
                        }
                    }
                }
            } else {
                for (i, q) in s.pts.iter().enumerate() {
                    let (x, y) = self.p(*q);
                    if i == 0 {
                        self.c.move_to(x, y);
                    } else {
                        self.c.line_to(x, y);
                    }
                }
            }
            if s.closed && !s.pts.is_empty() {
                self.c.close_path();
            }
        }
    }

    fn fill_rgb(&mut self, c: Rgba) {
        self.c
            .set_fill_rgb(f32::from(c.r) / 255.0, f32::from(c.g) / 255.0, f32::from(c.b) / 255.0);
    }

    fn stroke(&mut self, s: &Stroke) {
        let c = s.color;
        self.c
            .set_stroke_rgb(f32::from(c.r) / 255.0, f32::from(c.g) / 255.0, f32::from(c.b) / 255.0);
        self.c.set_line_width(f(s.width * self.k));
        let dash: Vec<f32> = s.dash.iter().map(|v| f((v * self.k).max(0.001))).collect();
        self.c.set_dash_pattern(dash, 0.0);
        self.c.set_line_cap(match s.cap {
            CapStyle::Butt => LineCapStyle::ButtCap,
            CapStyle::Round => LineCapStyle::RoundCap,
            CapStyle::Projecting => LineCapStyle::ProjectingSquareCap,
        });
        self.c.set_line_join(match s.join {
            JoinStyle::Miter => LineJoinStyle::MiterJoin,
            JoinStyle::Round => LineJoinStyle::RoundJoin,
            JoinStyle::Bevel => LineJoinStyle::BevelJoin,
        });
        self.c.stroke();
    }
}

fn text(c: &mut Ctx, t: &TextPrim) {
    let idx = base14_index(t);
    let Some((res, _)) = BASE14.get(idx) else { return };
    let size = super::font_px(t.size) * c.k;
    let (x, y) = c.p(t.origin);
    let (s, co) = t.angle.sin_cos();
    c.fill_rgb(t.color);
    let shaped = super::textcache::shaped(t);
    for (i, line) in t.text.split('\n').enumerate() {
        // Characters beyond WinAnsi: the line is set in Type 3 fonts.
        let t3_line: Option<Vec<(usize, u8)>> = if idx < 12 && line.chars().any(|ch| win_ansi_byte(ch).is_none()) {
            Some(line.chars().map(|ch| c.t3_code(t, ch)).collect())
        } else {
            None
        };
        // Line widths come from RustFig's own font metrics.
        let w = match &t3_line {
            Some(codes) => {
                codes
                    .iter()
                    .map(|&(fi, code)| {
                        c.t3.get(fi)
                            .and_then(|fnt| fnt.glyphs.get(usize::from(code) - 1))
                            .map_or(0.0, |g| g.adv)
                    })
                    .sum::<f64>()
                    / 1000.0
                    * size
            }
            None => shaped.line_widths.get(i).copied().unwrap_or(0.0) * c.k,
        };
        let dx = match t.justify {
            Justify::Left => 0.0,
            Justify::Center => -w / 2.0,
            Justify::Right => -w,
        };
        let dy = -(i as f64) * size * figbase::text::LINE_SPACING;
        // Rotate (dx, dy) by the text angle (counter-clockwise, y up).
        let ox = f64::from(x) + dx * co - dy * s;
        let oy = f64::from(y) + dx * s + dy * co;
        c.c.begin_text();
        if let Some(codes) = t3_line {
            c.c.set_text_matrix([f(co), f(s), f(-s), f(co), f(ox), f(oy)]);
            let mut k = 0;
            while let Some(&(fi, _)) = codes.get(k) {
                let run: Vec<u8> = codes
                    .iter()
                    .skip(k)
                    .take_while(|(g, _)| *g == fi)
                    .map(|&(_, b)| b)
                    .collect();
                k += run.len();
                c.c.set_font(Name(format!("T3_{fi}").as_bytes()), f(size));
                c.c.show(Str(&run));
            }
        } else {
            c.c.set_font(Name(res.as_bytes()), f(size));
            c.c.set_text_matrix([f(co), f(s), f(-s), f(co), f(ox), f(oy)]);
            c.c.show(Str(&win_ansi(line)));
        }
        c.c.end_text();
    }
}

struct PendingImage {
    name: String,
    rgb: Vec<u8>,
    alpha: Option<Vec<u8>>,
    w: u32,
    h: u32,
}

fn image(c: &mut Ctx, corners: &[Point; 4], img: &RasterImage, flipped: bool, images: &mut Vec<PendingImage>) {
    let (w, h) = (img.width, img.height);
    let expected = (w as usize).saturating_mul(h as usize).saturating_mul(4);
    if w == 0 || h == 0 || img.rgba.len() != expected {
        return;
    }
    let name = format!("Im{}", images.len());
    let mut rgb = Vec::with_capacity(expected / 4 * 3);
    let mut alpha = Vec::with_capacity(expected / 4);
    for &[r, g, b, a] in img.rgba.as_chunks::<4>().0 {
        rgb.extend_from_slice(&[r, g, b]);
        alpha.push(a);
    }
    let opaque = alpha.iter().all(|a| *a == 255);
    let [tl, tr, _, bl] = corners.map(|p| {
        let (x, y) = c.p(p);
        Point::new(f64::from(x), f64::from(y))
    });
    let (ex, ey) = if flipped {
        (bl - tl, tr - tl)
    } else {
        (tr - tl, bl - tl)
    };
    // PDF images map the unit square with row 0 at the top (v = 1).
    c.c.save_state();
    c.c.transform([f(ex.x), f(ex.y), f(-ey.x), f(-ey.y), f(tl.x + ey.x), f(tl.y + ey.y)]);
    c.c.x_object(Name(name.as_bytes()));
    c.c.restore_state();
    images.push(PendingImage {
        name,
        rgb,
        alpha: if opaque { None } else { Some(alpha) },
        w,
        h,
    });
}

/// Render a scene as a one-page PDF covering `bounds` (Fig units) at `scale`.
/// `title` becomes the document's `/Title` (fig2dev's `-n`).
pub fn to_pdf(scene: &Scene, bounds: &Rect, scale: f64, background: Option<Rgba>, title: Option<&str>) -> Vec<u8> {
    to_pdf_pages(scene, bounds, scale, background, title, None, 5)
}

/// Like [`to_pdf`]; with a `layout` the figure goes on its pages (fig2dev's
/// page mode) instead of one page of the figure's size. `minor` is the PDF
/// version 1.`minor` (`-Y`).
#[allow(clippy::too_many_arguments)]
pub fn to_pdf_pages(
    scene: &Scene,
    bounds: &Rect,
    scale: f64,
    background: Option<Rgba>,
    title: Option<&str>,
    layout: Option<&PageLayout>,
    minor: u8,
) -> Vec<u8> {
    let b = if bounds.is_empty() {
        Rect::new(Point::ZERO, Point::new(1.0, 1.0))
    } else {
        *bounds
    };
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let k = 72.0 / 1200.0 * scale;
    let (w, h) = (b.width() * k, b.height() * k);
    let mut c = Ctx {
        c: Content::new(),
        k,
        min_x: b.min.x,
        max_y: b.max.y,
        t3: Vec::new(),
    };
    if let (Some(bg), None) = (background, layout) {
        c.fill_rgb(bg);
        c.c.rect(0.0, 0.0, f(w), f(h));
        c.c.fill_even_odd();
    }
    let mut images = Vec::new();
    for p in &scene.prims {
        match p {
            Prim::Path { subpaths, fill, stroke } => {
                match fill {
                    Some(Fill::Solid(col)) => {
                        c.fill_rgb(*col);
                        c.path(subpaths);
                        c.c.fill_even_odd();
                    }
                    Some(Fill::Pattern { kind, fg, bg, nonzero }) => {
                        c.fill_rgb(*bg);
                        c.path(subpaths);
                        if *nonzero {
                            c.c.fill_nonzero();
                        } else {
                            c.c.fill_even_odd();
                        }
                        c.c.save_state();
                        c.path(subpaths);
                        if *nonzero {
                            c.c.clip_nonzero();
                        } else {
                            c.c.clip_even_odd();
                        }
                        c.c.end_path();
                        let region = Rect::from_points(subpaths.iter().flat_map(|s| s.pts.iter().copied()));
                        let frame = PatternFrame {
                            origin: Point::new(c.min_x, c.max_y),
                            fig_per_pt: 1.0 / c.k,
                        };
                        let (lines, line_width) = coverage(*kind, &region, &frame);
                        let lines: Vec<SubPath> = lines
                            .into_iter()
                            .map(|pts| SubPath {
                                pts,
                                closed: false,
                                curve: None,
                            })
                            .collect();
                        c.path(&lines);
                        c.stroke(&Stroke {
                            width: line_width,
                            color: *fg,
                            dash: Vec::new(),
                            cap: CapStyle::Butt,
                            join: JoinStyle::Miter,
                        });
                        c.c.restore_state();
                    }
                    None => {}
                }
                if let Some(s) = stroke {
                    c.path(subpaths);
                    c.stroke(s);
                }
            }
            Prim::Text(t) => text(&mut c, t),
            Prim::PictureRef { .. } => {}
            Prim::Image {
                corners,
                image: img,
                flipped,
            } => image(&mut c, corners, img, *flipped, &mut images),
        }
    }

    let mut pdf = Pdf::new();
    let catalog_id = Ref::new(1);
    let tree_id = Ref::new(2);
    let page_id = Ref::new(3);
    let content_id = Ref::new(4);
    let font_base = 5;
    let mut next = font_base + BASE14.len() as i32;
    let mut image_ids = Vec::new();
    for im in &images {
        let id = Ref::new(next);
        next += 1;
        let mask = if im.alpha.is_some() {
            let m = Ref::new(next);
            next += 1;
            Some(m)
        } else {
            None
        };
        image_ids.push((im.name.clone(), id, mask));
    }
    // One page of the figure's size, or fig2dev's pages on the paper.
    let figure_page = [(0.0, 0.0)];
    let (page_w, page_h, places): (f64, f64, &[(f64, f64)]) = match layout {
        Some(l) => (f64::from(l.width), f64::from(l.height), &l.pages),
        None => (w, h, &figure_page),
    };
    let mut page_ids = vec![(page_id, content_id)];
    for _ in 1..places.len() {
        page_ids.push((Ref::new(next), Ref::new(next + 1)));
        next += 2;
    }
    // Type 3 fonts: the font, its ToUnicode map and one id per glyph.
    let t3_ids: Vec<(Ref, Ref, Vec<Ref>)> =
        c.t3.iter()
            .map(|fnt| {
                let font = Ref::new(next);
                let tou = Ref::new(next + 1);
                next += 2;
                let glyphs = fnt
                    .glyphs
                    .iter()
                    .map(|_| {
                        next += 1;
                        Ref::new(next - 1)
                    })
                    .collect();
                (font, tou, glyphs)
            })
            .collect();
    pdf.set_version(1, minor);
    pdf.catalog(catalog_id).pages(tree_id);
    pdf.pages(tree_id)
        .kids(page_ids.iter().map(|p| p.0))
        .count(i32::try_from(page_ids.len()).unwrap_or(i32::MAX));
    for &(page_id, content_id) in &page_ids {
        let mut page = pdf.page(page_id);
        page.media_box(PdfRect::new(0.0, 0.0, f(page_w), f(page_h)));
        page.parent(tree_id);
        page.contents(content_id);
        let mut res = page.resources();
        {
            let mut fonts = res.fonts();
            for (i, (name, _)) in BASE14.iter().enumerate() {
                fonts.pair(Name(name.as_bytes()), Ref::new(font_base + i as i32));
            }
            for (i, (font, _, _)) in t3_ids.iter().enumerate() {
                fonts.pair(Name(format!("T3_{i}").as_bytes()), *font);
            }
        }
        {
            let mut xo = res.x_objects();
            for (name, id, _) in &image_ids {
                xo.pair(Name(name.as_bytes()), *id);
            }
        }
        res.finish();
        page.finish();
    }
    for (i, (_, base)) in BASE14.iter().enumerate() {
        let mut font = pdf.type1_font(Ref::new(font_base + i as i32));
        font.base_font(Name(base.as_bytes()));
        if !matches!(*base, "Symbol" | "ZapfDingbats") {
            font.encoding_predefined(Name(b"WinAnsiEncoding"));
        }
    }
    for (fnt, (font_id, tou_id, glyph_ids)) in c.t3.iter().zip(&t3_ids) {
        let names: Vec<String> = fnt.glyphs.iter().map(|g| format!("uni{:04X}", g.ch as u32)).collect();
        let bbox = fnt.glyphs.iter().fold([0.0f32, -300.0, 1000.0, 1000.0], |a, g| {
            [
                a[0].min(g.bbox[0]),
                a[1].min(g.bbox[1]),
                a[2].max(g.bbox[2]),
                a[3].max(g.bbox[3]),
            ]
        });
        let mut font = pdf.type3_font(*font_id);
        font.bbox(PdfRect::new(bbox[0], bbox[1], bbox[2], bbox[3]));
        font.matrix([0.001, 0.0, 0.0, 0.001, 0.0, 0.0]);
        {
            let mut cp = font.char_procs();
            for (name, id) in names.iter().zip(glyph_ids) {
                cp.pair(Name(name.as_bytes()), *id);
            }
        }
        font.encoding_custom()
            .differences()
            .consecutive(1, names.iter().map(|n| Name(n.as_bytes())));
        font.first_char(1);
        font.last_char(u8::try_from(fnt.glyphs.len()).unwrap_or(255));
        font.widths(fnt.glyphs.iter().map(|g| f(g.adv)));
        font.to_unicode(*tou_id);
        font.finish();
        let mut cmap = UnicodeCmap::<u8>::new(
            Name(b"Adobe-Identity-UCS"),
            SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"UCS"),
                supplement: 0,
            },
        );
        for (k, g) in fnt.glyphs.iter().enumerate() {
            cmap.pair(u8::try_from(k + 1).unwrap_or(1), g.ch);
        }
        pdf.stream(*tou_id, &cmap.finish());
        for (g, id) in fnt.glyphs.iter().zip(glyph_ids) {
            pdf.stream(*id, &g.proc_);
        }
    }
    let body = c.c.finish();
    for ((_, content_id), (i, &(tx, ty))) in page_ids.iter().zip(places.iter().enumerate()) {
        if layout.is_none() {
            pdf.stream(*content_id, &body);
            continue;
        }
        // The paper: background on the first page, clipped to the page,
        // the figure's corner at its place.
        let mut content = String::new();
        if let (0, Some(bg)) = (i, background) {
            let (r, g, b) = (
                f64::from(bg.r) / 255.0,
                f64::from(bg.g) / 255.0,
                f64::from(bg.b) / 255.0,
            );
            content.push_str(&format!("{r:.4} {g:.4} {b:.4} rg 0 0 {page_w} {page_h} re f\n"));
        }
        content.push_str(&format!("q 0 0 {page_w} {page_h} re W n 1 0 0 1 {tx:.3} {ty:.3} cm\n"));
        let mut bytes = content.into_bytes();
        bytes.extend_from_slice(&body);
        bytes.extend_from_slice(b"\nQ\n");
        pdf.stream(*content_id, &bytes);
    }
    for (im, (_, id, mask)) in images.iter().zip(image_ids.iter()) {
        let data = miniz_oxide::deflate::compress_to_vec_zlib(&im.rgb, 6);
        let mut x = pdf.image_xobject(*id, &data);
        x.filter(Filter::FlateDecode);
        x.width(im.w as i32);
        x.height(im.h as i32);
        x.color_space().device_rgb();
        x.bits_per_component(8);
        if let Some(m) = mask {
            x.s_mask(*m);
        }
        x.finish();
        if let (Some(m), Some(alpha)) = (mask, &im.alpha) {
            let data = miniz_oxide::deflate::compress_to_vec_zlib(alpha, 6);
            let mut s = pdf.image_xobject(*m, &data);
            s.filter(Filter::FlateDecode);
            s.width(im.w as i32);
            s.height(im.h as i32);
            s.color_space().device_gray();
            s.bits_per_component(8);
            s.finish();
        }
    }
    if let Some(t) = title {
        pdf.document_info(Ref::new(next)).title(TextStr(t));
    }
    pdf.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pdf_is_valid_header() {
        let pdf = to_pdf(&Scene::default(), &Rect::EMPTY, 1.0, None, Some("T"));
        assert!(pdf.starts_with(b"%PDF-"));
    }

    #[test]
    fn win_ansi_encoding() {
        assert_eq!(win_ansi("aé€☃"), vec![b'a', 0xe9, 0x80, b'?']);
    }
}
