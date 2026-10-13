//! Writer for the Fig 3.2 format.

use rustfig_geom::Point;
use rustfig_model::{Arrow, Document, Object, ObjectKind, Orientation, PolyKind, Style, Units};
use std::fmt::Write as _;

fn i(v: f64) -> i64 {
    if v.is_finite() {
        v.round().clamp(-2.0e9, 2.0e9) as i64
    } else {
        0
    }
}

fn f(v: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

fn comments(out: &mut String, c: &[String]) {
    for line in c {
        for l in line.split('\n') {
            let _ = writeln!(out, "# {l}");
        }
    }
}

fn style_common(out: &mut String, s: &Style) {
    let _ = write!(
        out,
        " {} {} {} {} {} {} {} {:.3}",
        s.line.to_fig(),
        s.thickness,
        s.pen_color.0,
        s.fill_color.0,
        s.depth,
        s.pen_style,
        s.area_fill,
        f(s.style_val)
    );
}

fn arrow(out: &mut String, a: &Option<Arrow>) {
    if let Some(a) = a {
        let _ = writeln!(
            out,
            "\t{} {} {:.2} {:.2} {:.2}",
            a.kind,
            a.style,
            f(a.thickness),
            f(a.width),
            f(a.height)
        );
    }
}

fn points(out: &mut String, pts: &[Point]) {
    for chunk in pts.chunks(6) {
        out.push('\t');
        for (k, p) in chunk.iter().enumerate() {
            if k > 0 {
                out.push(' ');
            }
            let _ = write!(out, "{} {}", i(p.x), i(p.y));
        }
        out.push('\n');
    }
}

fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\u{1}' => {}
            c => out.push(c),
        }
    }
    out
}

fn object(out: &mut String, o: &Object) {
    comments(out, &o.comments);
    match &o.kind {
        ObjectKind::Ellipse(e) => {
            let _ = write!(out, "1 {}", e.kind.to_fig());
            style_common(out, &e.style);
            let _ = writeln!(
                out,
                " {} {:.4} {} {} {} {} {} {} {} {}",
                e.direction,
                f(e.angle),
                i(e.center.x),
                i(e.center.y),
                i(e.radii.x),
                i(e.radii.y),
                i(e.start.x),
                i(e.start.y),
                i(e.end.x),
                i(e.end.y)
            );
        }
        ObjectKind::Polyline(p) => {
            let mut pts = p.points.clone();
            if p.kind.is_closed() {
                if let Some(first) = pts.first().copied() {
                    pts.push(first);
                }
            }
            let (fa, ba) = if p.kind == PolyKind::Polyline {
                (&p.forward_arrow, &p.backward_arrow)
            } else {
                (&None, &None)
            };
            let _ = write!(out, "2 {}", p.kind.to_fig());
            style_common(out, &p.style);
            let _ = writeln!(
                out,
                " {} {} {} {} {} {}",
                p.style.join.to_fig(),
                p.style.cap.to_fig(),
                p.radius,
                i32::from(fa.is_some()),
                i32::from(ba.is_some()),
                pts.len()
            );
            arrow(out, fa);
            arrow(out, ba);
            if p.kind == PolyKind::Picture {
                let (flipped, file) = p
                    .picture
                    .as_ref()
                    .map(|pic| (pic.flipped, pic.file.replace('\n', " ")))
                    .unwrap_or((false, String::new()));
                let _ = writeln!(out, "\t{} {}", i32::from(flipped), file);
            }
            points(out, &pts);
        }
        ObjectKind::Spline(s) => {
            let (fa, ba) = if s.closed {
                (&None, &None)
            } else {
                (&s.forward_arrow, &s.backward_arrow)
            };
            let _ = write!(out, "3 {}", s.fig_subtype());
            style_common(out, &s.style);
            let _ = writeln!(
                out,
                " {} {} {} {}",
                s.style.cap.to_fig(),
                i32::from(fa.is_some()),
                i32::from(ba.is_some()),
                s.points.len()
            );
            arrow(out, fa);
            arrow(out, ba);
            points(out, &s.points);
            let shapes: Vec<f64> = (0..s.points.len())
                .map(|k| f(s.shapes.get(k).copied().unwrap_or(0.0)).clamp(-1.0, 1.0))
                .collect();
            for chunk in shapes.chunks(8) {
                out.push('\t');
                let line: Vec<String> = chunk.iter().map(|v| format!("{v:.3}")).collect();
                out.push_str(&line.join(" "));
                out.push('\n');
            }
        }
        ObjectKind::Text(t) => {
            let _ = writeln!(
                out,
                "4 {} {} {} {} {} {} {:.4} {} {} {} {} {} {}\\001",
                t.justify.to_fig(),
                t.color.0,
                t.depth,
                t.pen_style,
                t.font,
                f(t.size),
                f(t.angle),
                t.flags,
                i(t.height),
                i(t.length),
                i(t.origin.x),
                i(t.origin.y),
                escape_text(&t.text)
            );
        }
        ObjectKind::Arc(a) => {
            let (fa, ba) = if a.pie {
                (&None, &None)
            } else {
                (&a.forward_arrow, &a.backward_arrow)
            };
            let _ = write!(out, "5 {}", if a.pie { 2 } else { 1 });
            style_common(out, &a.style);
            let _ = writeln!(
                out,
                " {} {} {} {} {:.3} {:.3} {} {} {} {} {} {}",
                a.style.cap.to_fig(),
                i32::from(a.ccw),
                i32::from(fa.is_some()),
                i32::from(ba.is_some()),
                f(a.center.x),
                f(a.center.y),
                i(a.points[0].x),
                i(a.points[0].y),
                i(a.points[1].x),
                i(a.points[1].y),
                i(a.points[2].x),
                i(a.points[2].y)
            );
            arrow(out, fa);
            arrow(out, ba);
        }
        ObjectKind::Compound(c) => {
            let b = o.geometry_bbox();
            let (a, z) = if b.is_empty() {
                (Point::ZERO, Point::ZERO)
            } else {
                (b.min, b.max)
            };
            let _ = writeln!(out, "6 {} {} {} {}", i(a.x), i(a.y), i(z.x), i(z.y));
            for child in &c.objects {
                object(out, child);
            }
            out.push_str("-6\n");
        }
    }
}

/// Serialise a document as Fig 3.2.
pub fn write_fig(doc: &Document) -> String {
    let mut out = String::new();
    out.push_str("#FIG 3.2  Produced by RustFig\n#encoding: UTF-8\n");
    let s = &doc.settings;
    out.push_str(match s.orientation {
        Orientation::Landscape => "Landscape\n",
        Orientation::Portrait => "Portrait\n",
    });
    out.push_str(if s.center { "Center\n" } else { "Flush Left\n" });
    out.push_str(match s.units {
        Units::Inches => "Inches\n",
        Units::Metric => "Metric\n",
    });
    let _ = writeln!(out, "{}", s.paper.name());
    let _ = writeln!(out, "{:.2}", f(s.magnification));
    out.push_str(if s.multiple_pages { "Multiple\n" } else { "Single\n" });
    let _ = writeln!(out, "{}", s.transparent_color);
    comments(&mut out, &doc.comments);
    out.push_str("1200 2\n");
    let mut colors = doc.colors.clone();
    colors.sort_by_key(|c| c.number);
    for c in &colors {
        let _ = writeln!(out, "0 {} {}", c.number, c.rgb.to_hex());
    }
    for o in &doc.objects {
        object(&mut out, o);
    }
    out
}
