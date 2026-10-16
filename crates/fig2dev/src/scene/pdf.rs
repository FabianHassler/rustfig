//! PDF backend (pdf-writer). Text uses the standard 14 PDF fonts.

use super::pattern::{coverage, PatternFrame};
use super::{Fill, Prim, Rgba, Scene, Stroke, SubPath, TextPrim};
use figbase::geom::{Point, Rect, Seg};
use figbase::image::RasterImage;
use figbase::style::{CapStyle, JoinStyle, Justify};
use pdf_writer::types::{LineCapStyle, LineJoinStyle};
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

/// Encode a string in WinAnsi (other characters become '?').
fn win_ansi(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| {
            let u = c as u32;
            if (0x20..0x7f).contains(&u) || (0xa0..=0xff).contains(&u) {
                u as u8
            } else {
                match c {
                    '€' => 0x80,
                    '‘' => 0x91,
                    '’' => 0x92,
                    '“' => 0x93,
                    '”' => 0x94,
                    '•' => 0x95,
                    '–' => 0x96,
                    '—' => 0x97,
                    _ => b'?',
                }
            }
        })
        .collect()
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
}

impl Ctx {
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
        // Line widths come from RustFig's own font metrics.
        let w = shaped.line_widths.get(i).copied().unwrap_or(0.0) * c.k;
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
        c.c.set_font(Name(res.as_bytes()), f(size));
        c.c.set_text_matrix([f(co), f(s), f(-s), f(co), f(ox), f(oy)]);
        c.c.show(Str(&win_ansi(line)));
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
    };
    if let Some(bg) = background {
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
                    Some(Fill::Pattern { kind, fg, bg }) => {
                        c.fill_rgb(*bg);
                        c.path(subpaths);
                        c.c.fill_even_odd();
                        c.c.save_state();
                        c.path(subpaths);
                        c.c.clip_even_odd();
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
    pdf.catalog(catalog_id).pages(tree_id);
    pdf.pages(tree_id).kids([page_id]).count(1);
    {
        let mut page = pdf.page(page_id);
        page.media_box(PdfRect::new(0.0, 0.0, f(w), f(h)));
        page.parent(tree_id);
        page.contents(content_id);
        let mut res = page.resources();
        {
            let mut fonts = res.fonts();
            for (i, (name, _)) in BASE14.iter().enumerate() {
                fonts.pair(Name(name.as_bytes()), Ref::new(font_base + i as i32));
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
    let content = c.c.finish();
    pdf.stream(content_id, &content);
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
