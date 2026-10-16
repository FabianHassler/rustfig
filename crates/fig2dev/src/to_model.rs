//! fig2dev's figure as a RustFig document: the objects `read.c` returns
//! ([`Compound`]) and the header values it sets in [`Ctx`] become a
//! [`rustfig_model::Document`], so that whatever fig2dev reads (Fig 1.3 to
//! 3.2, with its repairs) RustFig can draw, export and edit.
//!
//! The conversion undoes what the reader does to the file's numbers:
//! coordinates go to 1200 per inch (the file's `ppi` scaled), thicknesses
//! and arc-box radii back to 1/80 inch, arrowhead thicknesses back to
//! 1/80 inch, user colours back to their numbers in the file, closed
//! polylines and old closed splines lose their repeated end point (as
//! RustFig's own reader keeps them), and boxes become their corners.
//!
//! What is lost, because `read.c` drops it before the conversion sees it:
//! - Fig 3.2 splines arrive as polylines (fig2dev converts every X-spline
//!   with `create_line_with_spline`), so their control points and shape
//!   factors are gone; files before 3.2 keep their splines (approximated or
//!   interpolated, with the default shape factors RustFig's reader gives
//!   them; interpolated splines' Bézier controls are dropped, as there).
//! - Objects come in fig2dev's list order (per compound: arcs, compounds,
//!   ellipses, lines, splines, texts), which is xfig's drawing order within
//!   a depth, but not necessarily the file's order.
//! - The transparent-colour number of the header: `read.c` keeps only its
//!   colour, so it is found again by colour; "none", "default" and
//!   "background" all come back as none (−2), which exports alike.
//! - With fig2dev's `-m`, `-l`/`-p`, `-z`, `-M` given, the header's
//!   values were not read; the options' values are taken instead.
//! - Comments beyond `read.c`'s 100 per object.
//! - Text flags as fig2dev reads them: before Fig 2.1, texts in the
//!   default font are special (LaTeX) texts, the others PostScript texts.

use crate::gendev::{FIG_COLOR_NAMES, NUM_STD_COLS};
use crate::object::{self as f, Compound};
use crate::Ctx;
use figbase::geom::Point;
use rustfig_model::{
    pt, Arc, Arrow, CapStyle, ColorRef, DocSettings, Document, Ellipse, EllipseKind, JoinStyle, Justify, LineKind,
    Object, ObjectKind, Orientation, PaperSize, Picture, PolyKind, Polyline, Rgb, Spline, SplineKind, Style, Text,
    Units, UserColor, FIRST_USER_COLOR, LAST_USER_COLOR,
};

/// How the reader's numbers convert back to the file's.
struct Conv<'a> {
    ctx: &'a Ctx,
    /// Fig units (1200 per inch) per file unit.
    scale: f64,
    /// `round(THICK_SCALE)`, by which `read.c` multiplied thicknesses and
    /// radii; 0 for Fig 1.3, which keeps them.
    thick_factor: i32,
}

/// The figure `objects` read by [`crate::read::readfp_fig`] into `ctx`, as a
/// RustFig document (see the module documentation for what is lost).
pub fn to_document(ctx: &Ctx, objects: &Compound) -> Document {
    let scale = if ctx.ppi.is_finite() && ctx.ppi > 0.0 {
        1200.0 / ctx.ppi
    } else {
        1.0
    };
    let c = Conv {
        ctx,
        scale,
        thick_factor: crate::round(f64::from(ctx.thick_scale)),
    };
    let mut doc = Document::new();
    doc.settings = settings(ctx);
    doc.comments = objects.comments.clone();
    for (i, u) in ctx.user_colors.iter().enumerate() {
        let Some(&number) = ctx.user_col_indx.get(i) else {
            continue;
        };
        if !(FIRST_USER_COLOR..=LAST_USER_COLOR).contains(&number) || doc.colors.iter().any(|c| c.number == number) {
            continue;
        }
        doc.colors.push(UserColor {
            number,
            rgb: Rgb::new(byte(u.r), byte(u.g), byte(u.b)),
        });
    }
    doc.objects = c.compound(objects).into_iter().map(std::sync::Arc::new).collect();
    doc.assign_ids();
    doc
}

fn byte(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// The header values `read.c` set (or the options that overrode them).
fn settings(ctx: &Ctx) -> DocSettings {
    let mag = ctx.mag * 100.0;
    DocSettings {
        orientation: if ctx.landscape {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        },
        center: ctx.center,
        units: if ctx.metric { Units::Metric } else { Units::Inches },
        paper: PaperSize::from_name(ctx.papersize.trim()).unwrap_or_default(),
        magnification: if mag.is_finite() && mag > 0.0 {
            mag.min(100_000.0)
        } else {
            100.0
        },
        multiple_pages: ctx.multi_page,
        transparent_color: transparent_number(ctx),
    }
}

/// The colour number of `gif_transparent` (a standard colour's name, or a
/// user colour as `read.c` formats it), −2 (none) otherwise.
fn transparent_number(ctx: &Ctx) -> i32 {
    let name = ctx.gif_transparent.as_str();
    if name.is_empty() {
        return -2;
    }
    if let Some(i) = FIG_COLOR_NAMES.iter().position(|n| *n == name) {
        return i as i32;
    }
    ctx.user_colors
        .iter()
        .zip(&ctx.user_col_indx)
        .find(|(u, _)| format!("#{:2x}{:2x}{:2x}", u.r, u.g, u.b) == name)
        .map_or(-2, |(_, &n)| n)
}

impl Conv<'_> {
    fn coord(&self, v: f64) -> f64 {
        let v = v * self.scale;
        if v.is_finite() {
            v
        } else {
            0.0
        }
    }

    fn point(&self, p: f::Point) -> Point {
        pt(self.coord(f64::from(p.x)), self.coord(f64::from(p.y)))
    }

    fn points(&self, ps: &[f::Point]) -> Vec<Point> {
        ps.iter().map(|&p| self.point(p)).collect()
    }

    /// A picture's name as the file gives it: `read.c` put the directory
    /// of `from` before relative names.
    fn picture_name(&self, file: &str) -> String {
        let Some(from) = self.ctx.from.as_deref() else {
            return file.to_string();
        };
        let sep = |c: char| c == '/' || (cfg!(windows) && c == '\\');
        match from.rfind(sep).and_then(|i| from.get(..=i)) {
            Some(dir) => file.strip_prefix(dir).unwrap_or(file).to_string(),
            None => file.to_string(),
        }
    }

    /// A thickness or radius back in 1/80 inch.
    fn thickness(&self, t: i32) -> i32 {
        if self.thick_factor > 0 {
            let q = f64::from(t) / f64::from(self.thick_factor);
            q.round() as i32
        } else {
            t
        }
    }

    /// The colour number in the file: user colours were renumbered to
    /// `NUM_STD_COLS` + their index.
    fn color(&self, c: i32) -> ColorRef {
        if c < NUM_STD_COLS as i32 {
            return ColorRef(c.max(-1));
        }
        let idx = usize::try_from(c - NUM_STD_COLS as i32).ok();
        match idx.and_then(|i| self.ctx.user_col_indx.get(i)) {
            Some(&n) => ColorRef(n),
            None => ColorRef::DEFAULT,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn style(
        &self,
        line: i32,
        thickness: i32,
        pen: i32,
        fill: i32,
        depth: i32,
        pen_style: i32,
        fill_style: i32,
        style_val: f64,
        cap: i32,
        join: i32,
    ) -> Style {
        Style {
            line: LineKind::from_fig(line),
            thickness: self.thickness(thickness).clamp(0, 10_000),
            pen_color: self.color(pen),
            fill_color: self.color(fill),
            depth: depth.clamp(0, rustfig_model::MAX_DEPTH),
            pen_style,
            area_fill: fill_style.clamp(-1, 62),
            style_val: if style_val.is_finite() { style_val } else { 0.0 },
            cap: CapStyle::from_fig(cap),
            join: JoinStyle::from_fig(join),
        }
    }

    fn arrow(&self, a: &Option<f::Arrow>) -> Option<Arrow> {
        let a = a.as_ref()?;
        let ts = f64::from(self.ctx.thick_scale);
        let thickness = if ts > 0.0 { a.thickness / ts } else { 1.0 };
        let finite = |v: f64| if v.is_finite() { v } else { 0.0 };
        Some(Arrow {
            kind: a.type_.clamp(0, 30),
            style: a.style.clamp(0, 1),
            thickness: finite(thickness).clamp(0.0, 10_000.0),
            width: finite(a.wid * self.scale).abs().min(1.0e7),
            height: finite(a.ht * self.scale).abs().min(1.0e7),
        })
    }

    /// The objects of a compound in fig2dev's list order.
    fn compound(&self, com: &Compound) -> Vec<Object> {
        let mut out = Vec::new();
        for a in &com.arcs {
            out.push(with_comments(ObjectKind::Arc(self.arc(a)), &a.comments));
        }
        for c in &com.compounds {
            let kind = ObjectKind::Compound(rustfig_model::Compound {
                objects: self.compound(c),
            });
            out.push(with_comments(kind, &c.comments));
        }
        for e in &com.ellipses {
            out.push(with_comments(ObjectKind::Ellipse(self.ellipse(e)), &e.comments));
        }
        for l in &com.lines {
            out.push(with_comments(ObjectKind::Polyline(self.line(l)), &l.comments));
        }
        for s in &com.splines {
            out.push(with_comments(ObjectKind::Spline(self.spline(s)), &s.comments));
        }
        for t in &com.texts {
            out.push(with_comments(ObjectKind::Text(self.text(t)), &t.comments));
        }
        out
    }

    fn arc(&self, a: &f::Arc) -> Arc {
        let style = self.style(
            a.style,
            a.thickness,
            a.pen_color,
            a.fill_color,
            a.depth,
            a.pen,
            a.fill_style,
            a.style_val,
            a.cap_style,
            0,
        );
        let mut arc = Arc {
            pie: a.type_ == f::T_PIE_WEDGE_ARC,
            style,
            ccw: a.direction == 1,
            forward_arrow: self.arrow(&a.for_arrow),
            backward_arrow: self.arrow(&a.back_arrow),
            center: pt(self.coord(a.center.x), self.coord(a.center.y)),
            points: a.point.map(|p| self.point(p)),
        };
        arc.refit();
        arc
    }

    fn ellipse(&self, e: &f::Ellipse) -> Ellipse {
        let radii = self.point(e.radiuses);
        Ellipse {
            kind: EllipseKind::from_fig(e.type_),
            style: self.style(
                e.style,
                e.thickness,
                e.pen_color,
                e.fill_color,
                e.depth,
                e.pen,
                e.fill_style,
                e.style_val,
                0,
                0,
            ),
            direction: e.direction,
            angle: if e.angle.is_finite() { e.angle } else { 0.0 },
            center: self.point(e.center),
            radii: pt(radii.x.abs(), radii.y.abs()),
            start: self.point(e.start),
            end: self.point(e.end),
        }
    }

    fn line(&self, l: &f::Line) -> Polyline {
        let kind = PolyKind::from_fig(l.type_);
        let mut points = self.points(&l.points);
        if kind.is_closed() && points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        if kind.is_rect() && !points.is_empty() {
            let mut lo = pt(f64::INFINITY, f64::INFINITY);
            let mut hi = pt(f64::NEG_INFINITY, f64::NEG_INFINITY);
            for p in &points {
                lo = pt(lo.x.min(p.x), lo.y.min(p.y));
                hi = pt(hi.x.max(p.x), hi.y.max(p.y));
            }
            points = vec![lo, pt(hi.x, lo.y), hi, pt(lo.x, hi.y)];
        }
        Polyline {
            kind,
            style: self.style(
                l.style,
                l.thickness,
                l.pen_color,
                l.fill_color,
                l.depth,
                l.pen,
                l.fill_style,
                l.style_val,
                l.cap_style,
                l.join_style,
            ),
            radius: self.thickness(l.radius).clamp(0, 100_000),
            forward_arrow: self.arrow(&l.for_arrow),
            backward_arrow: self.arrow(&l.back_arrow),
            points,
            picture: l.pic.as_ref().map(|p| Picture {
                flipped: p.flipped != 0,
                file: self.picture_name(&p.file),
                image: None,
            }),
        }
    }

    fn spline(&self, s: &f::Spline) -> Spline {
        let kind = if s.x_spline() {
            SplineKind::X
        } else if s.int_spline() {
            SplineKind::Interpolated
        } else {
            SplineKind::Approximated
        };
        let closed = s.closed_spline();
        let mut points = self.points(&s.points);
        let mut shapes: Vec<f64> = if s.controls.len() == s.points.len() && self.ctx.v32_flag {
            s.controls.iter().map(|c| c.s.clamp(-1.0, 1.0)).collect()
        } else {
            let mut v = vec![kind.default_shape(); points.len()];
            if !closed {
                if let Some(first) = v.first_mut() {
                    *first = 0.0;
                }
                if let Some(last) = v.last_mut() {
                    *last = 0.0;
                }
            }
            v
        };
        if closed && points.len() > 2 && points.first() == points.last() && !self.ctx.v32_flag {
            points.pop();
        }
        shapes.truncate(points.len());
        Spline {
            kind,
            closed,
            style: self.style(
                s.style,
                s.thickness,
                s.pen_color,
                s.fill_color,
                s.depth,
                s.pen,
                s.fill_style,
                s.style_val,
                s.cap_style,
                0,
            ),
            forward_arrow: self.arrow(&s.for_arrow),
            backward_arrow: self.arrow(&s.back_arrow),
            points,
            shapes,
        }
    }

    fn text(&self, t: &f::Text) -> Text {
        let finite = |v: f64, d: f64| if v.is_finite() { v } else { d };
        Text {
            justify: Justify::from_fig(t.type_),
            color: self.color(t.color),
            depth: t.depth.clamp(0, rustfig_model::MAX_DEPTH),
            pen_style: t.pen,
            font: t.font,
            size: finite(t.size, 12.0).clamp(0.1, 5000.0),
            angle: finite(t.angle, 0.0),
            flags: t.flags,
            height: self.coord(finite(t.height, 0.0)),
            length: self.coord(finite(t.length, 0.0)),
            origin: pt(self.coord(f64::from(t.base_x)), self.coord(f64::from(t.base_y))),
            text: t.cstring.clone(),
        }
    }
}

fn with_comments(kind: ObjectKind, comments: &[String]) -> Object {
    let mut o = Object::new(kind);
    o.comments = comments.to_vec();
    o
}

pub use crate::util::readpics::{load_picture as read_picture, pic_to_raster, LoadedPicture};

#[cfg(test)]
mod tests {
    use super::*;

    fn read(src: &str, from: Option<&str>) -> (Ctx, Document) {
        let mut ctx = Ctx {
            from: from.map(str::to_string),
            ..Ctx::default()
        };
        let objs = crate::read::readfp_fig(&mut ctx, src.as_bytes()).unwrap();
        let doc = to_document(&ctx, &objs);
        (ctx, doc)
    }

    const FIG21: &str = "#FIG 2.1\n80 2\n\
        2 2 0 1 -1 0 0 0 0.000 7 0 0\n\t 10 10 90 10 90 50 10 50 10 10 9999 9999\n\
        2 1 1 2 4 0 0 0 4.000 -1 1 0\n\t0 0 1.000 4.000 8.000\n\t 10 70 50 70 9999 9999\n\
        3 0 0 1 -1 0 0 0 0.000 0 0\n\t 10 100 50 120 90 100 9999 9999\n\
        4 0 -1 0 0 0 12 0.00000 4 9 30 10 150 Hello\x01\n";

    #[test]
    fn fig_2_1_in_fig_units() {
        let (_, doc) = read(FIG21, None);
        assert_eq!(doc.settings.orientation, Orientation::Portrait);
        let kinds: Vec<&str> = doc.objects.iter().map(|o| o.type_name()).collect();
        assert_eq!(kinds, ["box", "polyline", "spline", "text"]);
        let ObjectKind::Polyline(b) = &doc.objects[0].kind else {
            panic!()
        };
        assert_eq!(
            b.points,
            vec![pt(150.0, 150.0), pt(1350.0, 150.0), pt(1350.0, 750.0), pt(150.0, 750.0)]
        );
        assert_eq!((b.style.thickness, b.style.area_fill), (1, -1));
        let ObjectKind::Polyline(l) = &doc.objects[1].kind else {
            panic!()
        };
        assert_eq!(
            (l.style.thickness, l.style.pen_color, l.style.line),
            (2, ColorRef(4), LineKind::Dashed)
        );
        let a = l.forward_arrow.unwrap();
        assert_eq!((a.thickness, a.width, a.height), (1.0, 60.0, 120.0));
        let ObjectKind::Spline(s) = &doc.objects[2].kind else {
            panic!()
        };
        assert_eq!(
            (s.kind, s.closed, s.shapes.clone()),
            (SplineKind::Approximated, false, vec![0.0, 1.0, 0.0])
        );
        let ObjectKind::Text(t) = &doc.objects[3].kind else {
            panic!()
        };
        assert_eq!((t.text.as_str(), t.size, t.origin), ("Hello", 12.0, pt(150.0, 2250.0)));
        assert!(t.is_postscript() && !t.is_special());
        assert_eq!((t.height, t.length), (135.0, 450.0));
    }

    #[test]
    fn fig_1_3() {
        let (_, doc) = read("1200 2 9 9\n3 10 0 1 1 0 0 0 0 0 0\n100 100 200 0 9999 0\n", None);
        assert_eq!(doc.objects.len(), 1);
        let ObjectKind::Spline(s) = &doc.objects[0].kind else {
            panic!()
        };
        // The header line holds the first point.
        assert_eq!(s.points, vec![pt(0.0, 0.0), pt(100.0, 100.0), pt(200.0, 0.0)]);
        assert_eq!(s.style.thickness, 1);
    }

    #[test]
    fn fig_3_2_header_colours_and_pictures() {
        let src = "#FIG 3.2\nPortrait\nFlush left\nMetric\nA4\n50.00\nMultiple\n32\n# whole figure\n1200 2\n\
            0 32 #123456\n\
            2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5\n\t0 pics/a.png\n\t 0 0 1200 0 1200 600 0 600 0 0\n\
            2 4 0 3 32 32 40 -1 20 0.000 0 0 7 0 0 5\n\t 0 0 600 0 600 600 0 600 0 0\n\
            1 3 0 2 0 7 50 -1 -1 0.000 1 0.0000 600 600 300 300 600 600 900 600\n";
        let (_, doc) = read(src, Some("/data/figs/x.fig"));
        let s = &doc.settings;
        assert_eq!(
            (
                s.orientation,
                s.center,
                s.units,
                s.paper,
                s.magnification,
                s.multiple_pages,
                s.transparent_color
            ),
            (
                Orientation::Portrait,
                false,
                Units::Metric,
                PaperSize::A4,
                50.0,
                true,
                32
            )
        );
        assert_eq!(doc.comments, ["whole figure"]);
        assert_eq!(
            doc.colors,
            [UserColor {
                number: 32,
                rgb: Rgb::new(0x12, 0x34, 0x56)
            }]
        );
        let pic = doc
            .objects
            .iter()
            .find_map(|o| match &o.kind {
                ObjectKind::Polyline(p) => p.picture.clone(),
                _ => None,
            })
            .unwrap();
        assert_eq!(pic.file, "pics/a.png", "the name as in the file");
        let arcbox = doc
            .objects
            .iter()
            .find_map(|o| match &o.kind {
                ObjectKind::Polyline(p) if p.kind == PolyKind::ArcBox => Some(p.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!((arcbox.radius, arcbox.style.thickness), (7, 3));
        assert_eq!(
            (arcbox.style.pen_color, arcbox.style.fill_color, arcbox.style.depth),
            (ColorRef(32), ColorRef(32), 40)
        );
        assert_eq!(doc.leaf_count(), 3);
    }

    #[test]
    fn xbm_pictures_become_pixels() {
        let pic = f::Pic {
            subtype: f::P_XBM,
            bitmap: vec![0b1010_0000, 0b0100_0000],
            bit_size: f::Point::new(3, 2),
            ..f::Pic::default()
        };
        let img = pic_to_raster(&pic).unwrap();
        assert_eq!((img.width, img.height), (3, 2));
        let alpha: Vec<u8> = img.rgba.chunks(4).map(|p| p[3]).collect();
        assert_eq!(alpha, [255, 0, 255, 0, 255, 0]);
        assert!(pic_to_raster(&f::Pic::default()).is_err());
    }

    #[test]
    fn missing_pictures_say_so_as_fig2dev() {
        let e = load_picture_err("no/such/picture.png");
        assert_eq!(e, "No such picture file: no/such/picture.png");
    }

    fn load_picture_err(name: &str) -> String {
        read_picture(&Ctx::default(), name).unwrap_err()
    }
}
