//! SVG backend.
//!
//! Text follows gensvg.c: Symbol and ZapfDingbats codes become Unicode
//! character references, special text (`^`, `_`, `{`, `}`, `$`) becomes
//! `tspan`s with a baseline shift.

// Ported from fig2dev dev/gensvg.c (`gensvg_text`, `symbolchar`,
// `dingbatchar`, `gensvg_start`'s paper size) (xfig licence, see
// LICENSE-xfig).

use super::pattern::{coverage, PatternFrame};
use super::{Fill, Prim, Rgba, Scene, Stroke, SubPath, TextPrim};
use figbase::geom::{Point, Rect, Seg};
use figbase::image::RasterImage;
use figbase::style::{CapStyle, JoinStyle, Justify};
use std::fmt::Write as _;

/// gensvg.c `symbolchar[256]`: Unicode of each Fig Symbol code, 0 where none.
const SYMBOL_CHAR: [u32; 256] = [
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0020, 0x0021, 0x2200, 0x0023, 0x2203, 0x0025, 0x0026, 0x220B, 0x0028, 0x0029,
    0x2217, 0x002B, 0x002C, 0x2212, 0x002E, 0x002F, 0x0030, 0x0031, 0x0032, 0x0033, 0x0034, 0x0035, 0x0036, 0x0037,
    0x0038, 0x0039, 0x003A, 0x003B, 0x003C, 0x003D, 0x003E, 0x003F, 0x2245, 0x0391, 0x0392, 0x03A7, 0x0394, 0x0395,
    0x03A6, 0x0393, 0x0397, 0x0399, 0x03D1, 0x039A, 0x039B, 0x039C, 0x039D, 0x039F, 0x03A0, 0x0398, 0x03A1, 0x03A3,
    0x03A4, 0x03A5, 0x03C2, 0x03A9, 0x039E, 0x03A8, 0x0396, 0x005B, 0x2234, 0x005D, 0x22A5, 0x005F, 0xF8E5, 0x03B1,
    0x03B2, 0x03C7, 0x03B4, 0x03B5, 0x03D5, 0x03B3, 0x03B7, 0x03B9, 0x03D5, 0x03BA, 0x03BB, 0x03BC, 0x03BD, 0x03BF,
    0x03C0, 0x03B8, 0x03C1, 0x03C3, 0x03C4, 0x03C5, 0x03D6, 0x03C9, 0x03BE, 0x03C8, 0x03B6, 0x007B, 0x007C, 0x007D,
    0x223C, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x20AC, 0x03D2, 0x2032, 0x2264, 0x2044, 0x221E, 0x0192, 0x2663,
    0x2666, 0x2665, 0x2660, 0x2194, 0x2190, 0x2191, 0x2192, 0x2193, 0x00B0, 0x00B1, 0x2033, 0x2265, 0x00D7, 0x221D,
    0x2202, 0x2022, 0x00F7, 0x2260, 0x2261, 0x2248, 0x2026, 0xF8E6, 0xF8E7, 0x21B5, 0x2135, 0x2111, 0x211C, 0x2118,
    0x2297, 0x2295, 0x2205, 0x2229, 0x222A, 0x2283, 0x2287, 0x2284, 0x2282, 0x2286, 0x2208, 0x2209, 0x2220, 0x2207,
    0xF6DA, 0xF6D9, 0xF6DB, 0x220F, 0x221A, 0x22C5, 0x00AC, 0x2227, 0x2228, 0x21D4, 0x21D0, 0x21D1, 0x21D2, 0x21D3,
    0x25CA, 0x2329, 0xF8E8, 0xF8E9, 0xF8EA, 0x2211, 0xF8EB, 0xF8EC, 0xF8ED, 0xF8EE, 0xF8EF, 0xF8F0, 0xF8F1, 0xF8F2,
    0xF8F3, 0xF8F4, 0x0000, 0x232A, 0x222B, 0x2320, 0xF8F5, 0x2321, 0xF8F6, 0xF8F7, 0xF8F8, 0xF8F9, 0xF8FA, 0xF8FB,
    0xF8FC, 0xF8FD, 0xF8FE, 0x0000,
];

/// gensvg.c `dingbatchar[256]`: Unicode of each Fig ZapfDingbats code, 0 where none.
const DINGBAT_CHAR: [u32; 256] = [
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0020, 0x2701, 0x2702, 0x2703, 0x2704, 0x260E, 0x2706, 0x2707, 0x2708, 0x2709,
    0x261B, 0x261E, 0x270C, 0x270D, 0x270E, 0x270F, 0x2710, 0x2711, 0x2712, 0x2713, 0x2714, 0x2715, 0x2716, 0x2717,
    0x2718, 0x2719, 0x271A, 0x271B, 0x271C, 0x271D, 0x271E, 0x271F, 0x2720, 0x2721, 0x2722, 0x2723, 0x2724, 0x2725,
    0x2726, 0x2727, 0x2605, 0x2729, 0x272A, 0x272B, 0x272C, 0x272D, 0x272E, 0x272F, 0x2730, 0x2731, 0x2732, 0x2733,
    0x2734, 0x2735, 0x2736, 0x2737, 0x2738, 0x2739, 0x273A, 0x273B, 0x273C, 0x273D, 0x273E, 0x273F, 0x2740, 0x2741,
    0x2742, 0x2743, 0x2744, 0x2745, 0x2746, 0x2747, 0x2748, 0x2749, 0x274A, 0x274B, 0x25CF, 0x274D, 0x25A0, 0x274F,
    0x2750, 0x2751, 0x2752, 0x25B2, 0x25BC, 0x25C6, 0x2756, 0x25D7, 0x2758, 0x2759, 0x275A, 0x275B, 0x275C, 0x275D,
    0x275E, 0x0000, 0xF8D7, 0xF8D8, 0xF8D9, 0xF8DA, 0xF8DB, 0xF8DC, 0xF8DD, 0xF8DE, 0xF8DF, 0xF8E0, 0xF8E1, 0xF8E2,
    0xF8E3, 0xF8E4, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
    0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x2761, 0x2762, 0x2763, 0x2764, 0x2765, 0x2766, 0x2767,
    0x2663, 0x2666, 0x2665, 0x2660, 0x2460, 0x2461, 0x2462, 0x2463, 0x2464, 0x2465, 0x2466, 0x2467, 0x2468, 0x2469,
    0x2776, 0x2777, 0x2778, 0x2779, 0x277A, 0x277B, 0x277C, 0x277D, 0x277E, 0x277F, 0x2780, 0x2781, 0x2782, 0x2783,
    0x2784, 0x2785, 0x2786, 0x2787, 0x2788, 0x2789, 0x278A, 0x278B, 0x278C, 0x278D, 0x278E, 0x278F, 0x2790, 0x2791,
    0x2792, 0x2793, 0x2794, 0x2192, 0x2194, 0x2195, 0x2798, 0x2799, 0x279A, 0x279B, 0x279C, 0x279D, 0x279E, 0x279F,
    0x27A0, 0x27A1, 0x27A2, 0x27A3, 0x27A4, 0x27A5, 0x27A6, 0x27A7, 0x27A8, 0x27A9, 0x27AA, 0x27AB, 0x27AC, 0x27AD,
    0x27AE, 0x27AF, 0x0000, 0x27B1, 0x27B2, 0x27B3, 0x27B4, 0x27B5, 0x27B6, 0x27B7, 0x27B8, 0x27B9, 0x27BA, 0x27BB,
    0x27BC, 0x27BD, 0x27BE, 0x0000,
];

/// gensvg's `put_sanitized_char` over a string, as far as it matters here:
/// `<`, `>` and `&` escaped; RustFig also escapes `"` and drops control
/// characters, so the output is always well-formed XML.
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

/// The Unicode `table` gives for each byte of a Symbol or ZapfDingbats
/// string, as `&#N;` (gensvg_text writes `&#0;` for codes without one, which
/// no XML parser accepts; those are left out).
fn table_text(table: &[u32; 256], bytes: &[u8]) -> String {
    let mut o = String::new();
    for &b in bytes {
        if let Some(&u) = table.get(usize::from(b)).filter(|&&u| u != 0) {
            let _ = write!(o, "&#{u};");
        }
    }
    o
}

/// The text as C's `cstring` bytes: the characters below 256 as themselves
/// (Latin-1, as the reader decoded them), the others as UTF-8.
fn text_bytes(line: &str) -> Vec<u8> {
    let mut b = Vec::with_capacity(line.len());
    for c in line.chars() {
        match u8::try_from(u32::from(c)) {
            Ok(v) => b.push(v),
            Err(_) => {
                let mut buf = [0u8; 4];
                b.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    b
}

/// gensvg_text's special text: `^x` and `_x` (or `^{...}`, `_{...}`) in a
/// smaller `tspan` shifted up or down, `$` dropped. As in C a `}` ends only
/// the innermost shift and a lone `}` after a one-character shift is
/// swallowed; unlike C, tspans still open at the end of the text are closed
/// (C leaves the XML broken) and a `}` at the very end writes no NUL.
fn special_body(t: &TextPrim, line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let size = (t.size * 8.0).ceil();
    let size = if size.is_finite() { size as i64 } else { 0 };
    let mut o = String::new();
    let (mut supsub, mut open) = (0, 0);
    let mut i = 0;
    while let Some(&first) = chars.get(i) {
        let mut ch = first;
        if (supsub == 2 && ch == '}') || supsub == 1 {
            o.push_str("</tspan>");
            open -= 1;
            supsub = 0;
            if ch == '}' {
                i += 1;
                let Some(&c) = chars.get(i) else { break };
                ch = c;
            }
        }
        if ch == '_' || ch == '^' {
            if i + 1 >= chars.len() {
                break;
            }
            supsub = 1;
            let shift = if ch == '_' { "sub" } else { "super" };
            let _ = write!(o, "<tspan font-size=\"{size}\" baseline-shift=\"{shift}\">");
            open += 1;
            i += 1;
            ch = chars.get(i).copied().unwrap_or(' ');
            if ch == '{' {
                if i + 1 >= chars.len() {
                    break;
                }
                supsub = 2;
                i += 1;
                ch = chars.get(i).copied().unwrap_or(' ');
            }
        }
        if ch != '$' {
            o.push_str(&esc(ch.encode_utf8(&mut [0u8; 4])));
        }
        i += 1;
    }
    for _ in 0..open {
        o.push_str("</tspan>");
    }
    o
}

/// The characters of one line of `t`.
fn text_body(t: &TextPrim, line: &str) -> String {
    match t.font {
        32 => table_text(&SYMBOL_CHAR, &text_bytes(line)),
        34 => table_text(&DINGBAT_CHAR, &text_bytes(line)),
        _ if t.special => special_body(t, line),
        _ => esc(line),
    }
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
        out.push_str(&text_body(t, &t.text));
    } else {
        for (i, l) in lines.iter().enumerate() {
            let _ = write!(
                out,
                "<tspan x=\"0\" y=\"{}\">{}</tspan>",
                n(i as f64 * size * figbase::text::LINE_SPACING),
                text_body(t, l)
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
    to_svg_page(scene, bounds, scale, background, None)
}

/// [`to_svg`] with `-z`: the size of the paper in points (gensvg_start sets
/// width and height to the paper's and keeps the viewBox of the figure).
pub fn to_svg_page(
    scene: &Scene,
    bounds: &Rect,
    scale: f64,
    background: Option<Rgba>,
    paper: Option<(i32, i32)>,
) -> String {
    let b = if bounds.is_empty() {
        Rect::new(Point::ZERO, Point::new(1.0, 1.0))
    } else {
        *bounds
    };
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let mut out = String::new();
    let _ = writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    let (w, h) = match paper {
        Some((w, h)) => (format!("{w}pt"), format!("{h}pt")),
        None => (
            format!("{}in", n(b.width() / 1200.0 * scale)),
            format!("{}in", n(b.height() / 1200.0 * scale)),
        ),
    };
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{w}\" height=\"{h}\" viewBox=\"{} {} {} {}\">",
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
