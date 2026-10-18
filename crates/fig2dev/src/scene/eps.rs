//! Encapsulated PostScript backend.
//!
//! EPS pictures the export supplies ([`EmbeddedEps`]) are embedded as
//! genps.c embeds them: scaled into the picture box, clipped to their
//! bounding box, `showpage` and `setpagedevice` disabled.
//
// The picture embedding is ported from fig2dev dev/genps.c (`genps_line`,
// picture branch) and dev/readeps.c (`read_eps`); the page structure of
// `to_ps` from `genps_start` and `genps_end` (xfig licence, see
// LICENSE-xfig).

use super::pattern::{coverage, PatternFrame};
use super::{EmbeddedEps, PageLayout};
use super::{Fill, Prim, Rgba, Scene, Stroke, SubPath, TextPrim};
use figbase::geom::{Point, Rect, Seg};
use figbase::style::{CapStyle, JoinStyle, Justify};
use std::fmt::Write as _;

struct Ctx {
    out: String,
    /// Fig units → points.
    k: f64,
    min_x: f64,
    max_y: f64,
}

fn n(v: f64) -> String {
    if v.is_finite() {
        let r = (v * 1000.0).round() / 1000.0;
        format!("{r}")
    } else {
        "0".into()
    }
}

impl Ctx {
    fn p(&self, p: Point) -> (f64, f64) {
        ((p.x - self.min_x) * self.k, (self.max_y - p.y) * self.k)
    }

    fn path(&mut self, subs: &[SubPath]) {
        self.out.push_str("newpath\n");
        for s in subs {
            if let Some(curve) = &s.curve {
                for seg in curve {
                    let _ = match *seg {
                        Seg::Move(q) => {
                            let (x, y) = self.p(q);
                            writeln!(self.out, "{} {} m", n(x), n(y))
                        }
                        Seg::Line(q) => {
                            let (x, y) = self.p(q);
                            writeln!(self.out, "{} {} l", n(x), n(y))
                        }
                        Seg::Cubic(a, b, q) => {
                            let ((x1, y1), (x2, y2), (x3, y3)) = (self.p(a), self.p(b), self.p(q));
                            writeln!(
                                self.out,
                                "{} {} {} {} {} {} curveto",
                                n(x1),
                                n(y1),
                                n(x2),
                                n(y2),
                                n(x3),
                                n(y3)
                            )
                        }
                    };
                }
            } else {
                for (i, q) in s.pts.iter().enumerate() {
                    let (x, y) = self.p(*q);
                    let _ = writeln!(self.out, "{} {} {}", n(x), n(y), if i == 0 { "m" } else { "l" });
                }
            }
            if s.closed && !s.pts.is_empty() {
                self.out.push_str("closepath\n");
            }
        }
    }

    fn rgb(&mut self, c: Rgba) {
        let _ = writeln!(
            self.out,
            "{} {} {} setrgbcolor",
            n(f64::from(c.r) / 255.0),
            n(f64::from(c.g) / 255.0),
            n(f64::from(c.b) / 255.0)
        );
    }

    fn stroke(&mut self, s: &Stroke) {
        self.rgb(s.color);
        let _ = writeln!(self.out, "{} setlinewidth", n(s.width * self.k));
        let d: Vec<String> = s.dash.iter().map(|v| n((v * self.k).max(0.001))).collect();
        let _ = writeln!(self.out, "[{}] 0 setdash", d.join(" "));
        let _ = writeln!(
            self.out,
            "{} setlinecap {} setlinejoin",
            match s.cap {
                CapStyle::Butt => 0,
                CapStyle::Round => 1,
                CapStyle::Projecting => 2,
            },
            match s.join {
                JoinStyle::Miter => 0,
                JoinStyle::Round => 1,
                JoinStyle::Bevel => 2,
            }
        );
        self.out.push_str("stroke\n");
    }
}

/// A PostScript string literal (Latin-1; other characters become '?').
pub(crate) fn ps_string(s: &str) -> String {
    let mut o = String::from("(");
    for c in s.chars() {
        match c {
            '(' | ')' | '\\' => {
                o.push('\\');
                o.push(c);
            }
            c if (' '..='~').contains(&c) => o.push(c),
            c if (c as u32) >= 0xa0 && (c as u32) <= 0xff => {
                let _ = write!(o, "\\{:03o}", c as u32);
            }
            _ => o.push('?'),
        }
    }
    o.push(')');
    o
}

fn text(c: &mut Ctx, t: &TextPrim) {
    let name = if t.postscript {
        figbase::text::ps_font_name(t.font)
    } else {
        match t.font {
            2 => "Times-Bold",
            3 => "Times-Italic",
            4 => "Helvetica",
            5 => "Courier",
            _ => "Times-Roman",
        }
    };
    let latin = !matches!(name, "Symbol" | "ZapfDingbats");
    let size = super::font_px(t.size) * c.k;
    let (x, y) = c.p(t.origin);
    c.rgb(t.color);
    let _ = writeln!(
        c.out,
        "gsave {} {} translate {} rotate /{}{} findfont {} scalefont setfont",
        n(x),
        n(y),
        n(t.angle.to_degrees()),
        name,
        if latin { "-iso" } else { "" },
        n(size)
    );
    for (i, line) in t.text.split('\n').enumerate() {
        let dy = -(i as f64) * size * figbase::text::LINE_SPACING;
        let s = ps_string(line);
        let just = match t.justify {
            Justify::Left => String::new(),
            Justify::Center => format!("{s} stringwidth pop -2 div 0 rmoveto "),
            Justify::Right => format!("{s} stringwidth pop neg 0 rmoveto "),
        };
        let _ = writeln!(c.out, "0 {} moveto {}{} show", n(dy), just, s);
    }
    c.out.push_str("grestore\n");
}

/// The pixels as hex RGB lines, alpha composited against white.
fn push_hex_pixels(out: &mut String, img: &figbase::image::RasterImage) {
    let mut col = 0;
    for &[r, g, b, a] in img.rgba.as_chunks::<4>().0 {
        let blend = |v: u8| -> u8 {
            let a = u16::from(a);
            ((u16::from(v) * a + 255 * (255 - a) + 127) / 255) as u8
        };
        let _ = write!(out, "{:02x}{:02x}{:02x}", blend(r), blend(g), blend(b));
        col += 6;
        if col >= 72 {
            out.push('\n');
            col = 0;
        }
    }
}

/// A standalone colour EPS file showing `img` at 1 pixel = 1 point (as
/// bmeps does), alpha composited against white. `None` for an empty or
/// inconsistent image.
pub fn raster_eps(img: &figbase::image::RasterImage) -> Option<String> {
    let (w, h) = (img.width, img.height);
    let expected = (w as usize).checked_mul(h as usize)?.checked_mul(4)?;
    if w == 0 || h == 0 || img.rgba.len() != expected {
        return None;
    }
    let mut out = String::new();
    let _ = writeln!(out, "%!PS-Adobe-3.0 EPSF-3.0");
    let _ = writeln!(out, "%%BoundingBox: 0 0 {w} {h}");
    let _ = writeln!(out, "%%Creator: RustFig fig2dev");
    let _ = writeln!(out, "%%EndComments");
    let _ = writeln!(out, "gsave");
    let _ = writeln!(out, "/picstr {} string def", u64::from(w) * 3);
    // The image matrix maps the picture onto the unit square; scale it to
    // one point per pixel, as bmeps does.
    let _ = writeln!(out, "{w} {h} scale");
    let _ = writeln!(
        out,
        "{w} {h} 8 [{w} 0 0 -{h} 0 {h}] {{currentfile picstr readhexstring pop}} false 3 colorimage"
    );
    push_hex_pixels(&mut out, img);
    out.push_str("\ngrestore\nshowpage\n%%EOF\n");
    Some(out)
}

fn image(c: &mut Ctx, corners: &[Point; 4], img: &figbase::image::RasterImage, flipped: bool) {
    let (w, h) = (img.width, img.height);
    let expected = (w as usize).saturating_mul(h as usize).saturating_mul(4);
    if w == 0 || h == 0 || img.rgba.len() != expected {
        return;
    }
    let [tl, tr, _, bl] = corners.map(|p| {
        let (x, y) = c.p(p);
        Point::new(x, y)
    });
    let (ex, ey) = if flipped {
        (bl - tl, tr - tl)
    } else {
        (tr - tl, bl - tl)
    };
    // Map the unit square (image space, y down) onto the box.
    let _ = writeln!(
        c.out,
        "gsave [{} {} {} {} {} {}] concat",
        n(ex.x),
        n(ex.y),
        n(ey.x),
        n(ey.y),
        n(tl.x),
        n(tl.y)
    );
    let _ = writeln!(c.out, "/picstr {} string def", u64::from(w) * 3);
    let _ = writeln!(
        c.out,
        "{w} {h} 8 [{w} 0 0 {h} 0 0] {{currentfile picstr readhexstring pop}} false 3 colorimage"
    );
    push_hex_pixels(&mut c.out, img);
    c.out.push_str("\ngrestore\n");
}

const ISO_FONTS: [&str; 39] = [
    "Times-Roman",
    "Times-Italic",
    "Times-Bold",
    "Times-BoldItalic",
    "AvantGarde-Book",
    "AvantGarde-BookOblique",
    "AvantGarde-Demi",
    "AvantGarde-DemiOblique",
    "Bookman-Light",
    "Bookman-LightItalic",
    "Bookman-Demi",
    "Bookman-DemiItalic",
    "Courier",
    "Courier-Oblique",
    "Courier-Bold",
    "Courier-BoldOblique",
    "Helvetica",
    "Helvetica-Oblique",
    "Helvetica-Bold",
    "Helvetica-BoldOblique",
    "Helvetica-Narrow",
    "Helvetica-Narrow-Oblique",
    "Helvetica-Narrow-Bold",
    "Helvetica-Narrow-BoldOblique",
    "NewCenturySchlbk-Roman",
    "NewCenturySchlbk-Italic",
    "NewCenturySchlbk-Bold",
    "NewCenturySchlbk-BoldItalic",
    "Palatino-Roman",
    "Palatino-Italic",
    "Palatino-Bold",
    "Palatino-BoldItalic",
    "ZapfChancery-MediumItalic",
    "Times-Roman",
    "Times-Roman",
    "Times-Roman",
    "Times-Roman",
    "Times-Roman",
    "Times-Roman",
];

/// The procedure and font definitions of the prolog.
fn prolog(out: &mut String) {
    out.push_str("/m {moveto} bind def /l {lineto} bind def\n");
    out.push_str(
        "/isofont { findfont dup length dict begin {1 index /FID ne {def} {pop pop} ifelse} forall \
         /Encoding ISOLatin1Encoding def currentdict end definefont pop } bind def\n",
    );
    let mut seen = std::collections::BTreeSet::new();
    for f in ISO_FONTS {
        if seen.insert(f) {
            let _ = writeln!(out, "/{f}-iso /{f} isofont");
        }
    }
}

/// The scene's primitives, in the frame of `c`.
fn draw_prims(c: &mut Ctx, scene: &Scene, embeds: &std::collections::BTreeMap<String, EmbeddedEps>) {
    let mut skip_placeholder = false;
    for p in &scene.prims {
        if std::mem::take(&mut skip_placeholder) {
            continue;
        }
        match p {
            Prim::PictureRef { corners, flipped, file } => {
                if let Some(e) = embeds.get(file) {
                    embed(c, corners, *flipped, e);
                    // The next primitive is the placeholder for the picture.
                    skip_placeholder = true;
                }
            }
            Prim::Path { subpaths, fill, stroke } => {
                match fill {
                    Some(Fill::Solid(col)) => {
                        c.rgb(*col);
                        c.path(subpaths);
                        c.out.push_str("eofill\n");
                    }
                    Some(Fill::Pattern { kind, fg, bg }) => {
                        c.rgb(*bg);
                        c.path(subpaths);
                        c.out.push_str("gsave eofill grestore gsave eoclip\n");
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
                        c.out.push_str("grestore\n");
                    }
                    None => {}
                }
                if let Some(s) = stroke {
                    c.path(subpaths);
                    c.stroke(s);
                }
            }
            Prim::Text(t) => text(c, t),
            Prim::Image {
                corners,
                image: img,
                flipped,
            } => image(c, corners, img, *flipped),
        }
    }
}

/// Render a scene as EPS covering `bounds` (Fig units) at `scale`.
/// `title` goes into `%%Title` (fig2dev's `-n`).
/// `embeds` holds the EPS pictures to embed, by file name.
pub fn to_eps(
    scene: &Scene,
    bounds: &Rect,
    scale: f64,
    background: Option<Rgba>,
    title: Option<&str>,
    embeds: &std::collections::BTreeMap<String, EmbeddedEps>,
) -> String {
    let b = if bounds.is_empty() {
        Rect::new(Point::ZERO, Point::new(1.0, 1.0))
    } else {
        *bounds
    };
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let k = 72.0 / 1200.0 * scale;
    let mut c = Ctx {
        out: String::new(),
        k,
        min_x: b.min.x,
        max_y: b.max.y,
    };
    let (w, h) = (b.width() * k, b.height() * k);
    let _ = writeln!(c.out, "%!PS-Adobe-3.0 EPSF-3.0");
    if let Some(t) = title {
        let _ = writeln!(c.out, "%%Title: {}", t.replace(['\r', '\n'], " "));
    }
    let _ = writeln!(c.out, "%%Creator: RustFig");
    let _ = writeln!(c.out, "%%BoundingBox: 0 0 {} {}", w.ceil() as i64, h.ceil() as i64);
    let _ = writeln!(c.out, "%%HiResBoundingBox: 0 0 {} {}", n(w), n(h));
    let _ = writeln!(c.out, "%%LanguageLevel: 2\n%%Pages: 1\n%%EndComments\n%%BeginProlog");
    prolog(&mut c.out);
    c.out.push_str("%%EndProlog\n%%Page: 1 1\nsave\n");
    if let Some(bg) = background {
        c.rgb(bg);
        let _ = writeln!(c.out, "0 0 {} {} rectfill", n(w), n(h));
    }
    draw_prims(&mut c, scene, embeds);
    c.out.push_str("restore\nshowpage\n%%Trailer\n%%EOF\n");
    c.out
}

/// Render a scene as a DSC PostScript document on `layout`'s pages:
/// `bounds` (Fig units) at `scale` placed on each page as
/// [`PageLayout::pages`] says, clipped to the paper. `magnification` is
/// the figure's, for the `%%Magnification` comment.
#[allow(clippy::too_many_arguments)]
pub fn to_ps(
    scene: &Scene,
    bounds: &Rect,
    scale: f64,
    background: Option<Rgba>,
    title: Option<&str>,
    embeds: &std::collections::BTreeMap<String, EmbeddedEps>,
    layout: &PageLayout,
    magnification: f64,
) -> String {
    let b = if bounds.is_empty() {
        Rect::new(Point::ZERO, Point::new(1.0, 1.0))
    } else {
        *bounds
    };
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let k = 72.0 / 1200.0 * scale;
    let mut c = Ctx {
        out: String::new(),
        k,
        min_x: b.min.x,
        max_y: b.max.y,
    };
    draw_prims(&mut c, scene, embeds);
    let body = std::mem::take(&mut c.out);
    let (pw, ph) = (layout.width, layout.height);
    let o = &mut c.out;
    let _ = writeln!(o, "%!PS-Adobe-3.0");
    if let Some(t) = title {
        let _ = writeln!(o, "%%Title: {}", t.replace(['\r', '\n'], " "));
    }
    let _ = writeln!(o, "%%Creator: RustFig");
    let _ = writeln!(o, "%%Pages: {}", layout.declared_pages);
    let _ = writeln!(o, "%%BoundingBox: 0 0 {pw} {ph}");
    let _ = writeln!(o, "%%DocumentPaperSizes: {}", layout.paper.to_ascii_lowercase());
    let _ = writeln!(o, "%%Magnification: {magnification:.4}");
    o.push_str("%%EndComments\n");
    o.push_str("%%BeginSetup\n[{\n");
    let _ = writeln!(o, "%%BeginFeature: *PageSize {}", layout.paper);
    let _ = writeln!(o, "<</PageSize [{pw} {ph}]>> setpagedevice");
    o.push_str("%%EndFeature\n} stopped cleartomark\n%%EndSetup\n%%BeginProlog\n");
    prolog(o);
    o.push_str("%%EndProlog\n");
    for (i, (tx, ty)) in layout.pages.iter().enumerate() {
        let _ = writeln!(o, "%%Page: {} {}\nsave", i + 1, i + 1);
        // The background fills the first page, as fig2dev's prolog does.
        if let (0, Some(bg)) = (i, background) {
            let (r, g, bl) = (
                f64::from(bg.r) / 255.0,
                f64::from(bg.g) / 255.0,
                f64::from(bg.b) / 255.0,
            );
            let _ = writeln!(o, "{r:.4} {g:.4} {bl:.4} setrgbcolor\n0 0 {pw} {ph} rectfill");
        }
        let _ = writeln!(
            o,
            "newpath 0 0 moveto {pw} 0 lineto {pw} {ph} lineto 0 {ph} lineto closepath clip newpath"
        );
        let _ = writeln!(o, "{} {} translate", n(*tx), n(*ty));
        o.push_str(&body);
        o.push_str("restore\nshowpage\n");
    }
    o.push_str("%%Trailer\n%%EOF\n");
    c.out
}

/// Embeds an EPS file into the picture box `corners` (genps.c).
fn embed(c: &mut Ctx, corners: &[Point; 4], flipped: bool, e: &EmbeddedEps) {
    let (pllx, plly, purx, pury) = (e.llx, e.lly, e.urx, e.ury);
    let (img_w, img_h) = (f64::from(purx) - f64::from(pllx), f64::from(pury) - f64::from(plly));
    let (pic_w, pic_h) = if flipped { (img_h, img_w) } else { (img_w, img_h) };
    if !(pic_w > 0.0 && pic_h > 0.0) {
        return;
    }
    let [tl, _, br, _] = *corners;
    let o = &mut c.out;
    o.push_str("%\n% pen to black in case this eps object doesn't set color first\n0 0 0 setrgbcolor\n");
    let _ = writeln!(o, "% Begin Imported EPS File: {}", e.title);
    let _ = writeln!(o, "%%BeginDocument: {}", e.title);
    o.push_str("%\nnewpath gsave\n");
    // Fig coordinates, as genps works in them.
    let _ = writeln!(
        o,
        "[{} 0 0 {} {} {}] concat",
        n(c.k),
        n(-c.k),
        n(-c.min_x * c.k),
        n(c.max_y * c.k)
    );
    let _ = writeln!(o, "{} {} translate", n(tl.x), n(tl.y));
    let _ = writeln!(
        o,
        "{:.6} {:.6} scale",
        ((br.x - tl.x) / pic_w).abs(),
        -(br.y - tl.y) / pic_h
    );
    if flipped {
        let _ = writeln!(o, "{} 0 translate\n270 rotate\n1 -1 scale", n(pic_w));
    } else {
        let _ = writeln!(o, "0 {} translate", n(-pic_h));
    }
    let _ = writeln!(o, "{} {} translate", -pllx, -plly);
    o.push_str("save\n");
    let _ = writeln!(
        o,
        "newpath {pllx} {plly} moveto {purx} {plly} lineto {purx} {pury} lineto {pllx} {pury} lineto closepath clip newpath"
    );
    o.push_str("countdictstack\nmark\n/showpage {} def\n/setpagedevice {pop} def\n% EPS file follows:\n");
    o.push_str(&String::from_utf8_lossy(&e.ps));
    if !o.ends_with('\n') {
        o.push('\n');
    }
    o.push_str("\ncleartomark\ncountdictstack exch sub { end } repeat\nrestore grestore\n%\n");
    let _ = writeln!(o, "% End Imported PIC File: {}", e.title);
    o.push_str("%%EndDocument\n%\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ps_strings() {
        assert_eq!(ps_string("a(b)\\"), "(a\\(b\\)\\\\)");
        assert_eq!(ps_string("é€"), "(\\351?)");
    }
}
