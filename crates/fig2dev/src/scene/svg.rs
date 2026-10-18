//! SVG backend.

use super::pattern::{coverage, PatternFrame};
use super::{Fill, Prim, Rgba, Scene, Stroke, SubPath, TextPrim};
use figbase::geom::{Point, Rect, Seg};
use figbase::image::RasterImage;
use figbase::style::{CapStyle, JoinStyle, Justify};
use std::fmt::Write as _;

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            c if (c as u32) < 0x20 && c != '\t' => {}
            c => o.push(c),
        }
    }
    o
}

fn n(v: f64) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    let r = (v * 100.0).round() / 100.0;
    if r == r.trunc() {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

fn rgb(c: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

pub(crate) fn path_data(subs: &[SubPath]) -> String {
    let mut d = String::new();
    for s in subs {
        if let Some(curve) = &s.curve {
            for seg in curve {
                let _ = match *seg {
                    Seg::Move(p) => write!(d, "M{} {} ", n(p.x), n(p.y)),
                    Seg::Line(p) => write!(d, "L{} {} ", n(p.x), n(p.y)),
                    Seg::Cubic(a, b, p) => {
                        write!(d, "C{} {} {} {} {} {} ", n(a.x), n(a.y), n(b.x), n(b.y), n(p.x), n(p.y))
                    }
                };
            }
        } else {
            for (i, p) in s.pts.iter().enumerate() {
                let _ = write!(d, "{}{} {} ", if i == 0 { "M" } else { "L" }, n(p.x), n(p.y));
            }
        }
        if s.closed && !s.pts.is_empty() {
            d.push_str("Z ");
        }
    }
    d.trim_end().to_string()
}

fn stroke_attrs(s: &Stroke) -> String {
    let mut a = format!(" stroke=\"{}\" stroke-width=\"{}\"", rgb(s.color), n(s.width));
    if !s.dash.is_empty() {
        let d: Vec<String> = s.dash.iter().map(|v| n(v.max(0.01))).collect();
        let _ = write!(a, " stroke-dasharray=\"{}\"", d.join(" "));
    }
    a.push_str(match s.cap {
        CapStyle::Butt => "",
        CapStyle::Round => " stroke-linecap=\"round\"",
        CapStyle::Projecting => " stroke-linecap=\"square\"",
    });
    a.push_str(match s.join {
        JoinStyle::Miter => "",
        JoinStyle::Round => " stroke-linejoin=\"round\"",
        JoinStyle::Bevel => " stroke-linejoin=\"bevel\"",
    });
    a
}

/// CSS font family and style for a text.
pub(crate) fn css_font(t: &TextPrim) -> (&'static str, bool, bool) {
    let req = t.request();
    let fam = if t.postscript {
        match t.font {
            -1..=3 => "Times, 'Times New Roman', serif",
            4..=7 => "'URW Gothic', 'Avant Garde', sans-serif",
            8..=11 => "'Bookman Old Style', Bookman, serif",
            12..=15 => "Courier, 'Courier New', monospace",
            16..=23 => "Helvetica, Arial, sans-serif",
            24..=27 => "'Century Schoolbook', serif",
            28..=31 => "Palatino, 'Palatino Linotype', serif",
            32 => "Symbol",
            33 => "'Zapf Chancery', cursive",
            _ => "'Zapf Dingbats', Dingbats",
        }
    } else {
        match t.font {
            4 => "sans-serif",
            5 => "monospace",
            _ => "'Latin Modern Roman', 'Computer Modern', serif",
        }
    };
    (fam, req.bold, req.italic)
}

fn text(out: &mut String, t: &TextPrim) {
    let (fam, bold, italic) = css_font(t);
    let anchor = match t.justify {
        Justify::Left => "start",
        Justify::Center => "middle",
        Justify::Right => "end",
    };
    let deg = -t.angle.to_degrees();
    let size = super::font_px(t.size);
    let _ = write!(
        out,
        "<text transform=\"translate({} {}){}\" font-family=\"{}\" font-size=\"{}\"{}{} text-anchor=\"{}\" fill=\"{}\" xml:space=\"preserve\">",
        n(t.origin.x),
        n(t.origin.y),
        if deg.abs() > 1e-9 { format!(" rotate({})", n(deg)) } else { String::new() },
        fam,
        n(size),
        if bold { " font-weight=\"bold\"" } else { "" },
        if italic { " font-style=\"italic\"" } else { "" },
        anchor,
        rgb(t.color)
    );
    let lines: Vec<&str> = t.text.split('\n').collect();
    if lines.len() == 1 {
        out.push_str(&esc(&t.text));
    } else {
        for (i, l) in lines.iter().enumerate() {
            let _ = write!(
                out,
                "<tspan x=\"0\" y=\"{}\">{}</tspan>",
                n(i as f64 * size * figbase::text::LINE_SPACING),
                esc(l)
            );
        }
    }
    out.push_str("</text>\n");
}

pub(crate) fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let b = [
            c.first().copied().unwrap_or(0),
            c.get(1).copied().unwrap_or(0),
            c.get(2).copied().unwrap_or(0),
        ];
        let v = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let idx = |sh: u32| char::from(T.get(((v >> sh) & 63) as usize).copied().unwrap_or(b'A'));
        s.push(idx(18));
        s.push(idx(12));
        s.push(if c.len() > 1 { idx(6) } else { '=' });
        s.push(if c.len() > 2 { idx(0) } else { '=' });
    }
    s
}

pub(crate) fn image_png(img: &RasterImage) -> Option<Vec<u8>> {
    let size = tiny_skia::IntSize::from_wh(img.width, img.height)?;
    let mut data = img.rgba.clone();
    let expected = (img.width as usize).checked_mul(img.height as usize)?.checked_mul(4)?;
    if data.len() != expected {
        return None;
    }
    for [r, g, b, a] in data.as_chunks_mut::<4>().0.iter_mut() {
        let al = u16::from(*a);
        *r = ((u16::from(*r) * al + 127) / 255) as u8;
        *g = ((u16::from(*g) * al + 127) / 255) as u8;
        *b = ((u16::from(*b) * al + 127) / 255) as u8;
    }
    tiny_skia::Pixmap::from_vec(data, size)?.encode_png().ok()
}

/// Render a scene as an SVG document covering `bounds` (Fig units), sized
/// at `scale` (1.0 = actual size).
pub fn to_svg(scene: &Scene, bounds: &Rect, scale: f64, background: Option<Rgba>) -> String {
    let b = if bounds.is_empty() {
        Rect::new(Point::ZERO, Point::new(1.0, 1.0))
    } else {
        *bounds
    };
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let mut out = String::new();
    let _ = writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{}in\" height=\"{}in\" viewBox=\"{} {} {} {}\">",
        n(b.width() / 1200.0 * scale),
        n(b.height() / 1200.0 * scale),
        n(b.min.x),
        n(b.min.y),
        n(b.width()),
        n(b.height())
    );
    let _ = writeln!(out, "<!-- Produced by RustFig -->");
    if let Some(bg) = background {
        let _ = writeln!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            n(b.min.x),
            n(b.min.y),
            n(b.width()),
            n(b.height()),
            rgb(bg)
        );
    }
    let mut clip_id = 0;
    for p in &scene.prims {
        match p {
            Prim::Path { subpaths, fill, stroke } => {
                let d = path_data(subpaths);
                if d.is_empty() {
                    continue;
                }
                match fill {
                    Some(Fill::Solid(c)) => {
                        let _ = writeln!(out, "<path d=\"{d}\" fill=\"{}\" fill-rule=\"evenodd\"/>", rgb(*c));
                    }
                    Some(Fill::Pattern { kind, fg, bg, nonzero }) => {
                        clip_id += 1;
                        let rule = if *nonzero { "nonzero" } else { "evenodd" };
                        let _ = writeln!(
                            out,
                            "<clipPath id=\"c{clip_id}\"><path d=\"{d}\" clip-rule=\"{rule}\"/></clipPath>"
                        );
                        let _ = writeln!(out, "<path d=\"{d}\" fill=\"{}\" fill-rule=\"{rule}\"/>", rgb(*bg));
                        let region = Rect::from_points(subpaths.iter().flat_map(|s| s.pts.iter().copied()));
                        let (lines, line_width) = coverage(*kind, &region, &PatternFrame::export(&b, scale));
                        let lines: Vec<SubPath> = lines
                            .into_iter()
                            .map(|pts| SubPath {
                                pts,
                                closed: false,
                                curve: None,
                            })
                            .collect();
                        let _ = writeln!(
                            out,
                            "<path clip-path=\"url(#c{clip_id})\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>",
                            path_data(&lines),
                            rgb(*fg),
                            n(line_width)
                        );
                    }
                    None => {}
                }
                if let Some(s) = stroke {
                    let _ = writeln!(out, "<path d=\"{d}\" fill=\"none\"{}/>", stroke_attrs(s));
                }
            }
            Prim::Text(t) => text(&mut out, t),
            Prim::PictureRef { .. } => {}
            Prim::Image {
                corners,
                image,
                flipped,
            } => {
                let Some(png) = image_png(image) else { continue };
                let [tl, tr, _, bl] = *corners;
                let (ex, ey) = if *flipped {
                    (bl - tl, tr - tl)
                } else {
                    (tr - tl, bl - tl)
                };
                let (w, h) = (f64::from(image.width.max(1)), f64::from(image.height.max(1)));
                let _ = writeln!(
                    out,
                    "<image width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\" transform=\"matrix({} {} {} {} {} {})\" xlink:href=\"data:image/png;base64,{}\"/>",
                    image.width,
                    image.height,
                    n6(ex.x / w),
                    n6(ex.y / w),
                    n6(ey.x / h),
                    n6(ey.y / h),
                    n(tl.x),
                    n(tl.y),
                    base64(&png)
                );
            }
        }
    }
    out.push_str("</svg>\n");
    out
}

fn n6(v: f64) -> String {
    if v.is_finite() {
        format!("{v:.6}")
    } else {
        "0".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_known() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b""), "");
    }

    #[test]
    fn escapes() {
        assert_eq!(esc("a<b>&\"c\u{1}"), "a&lt;b&gt;&amp;&quot;c");
    }
}
