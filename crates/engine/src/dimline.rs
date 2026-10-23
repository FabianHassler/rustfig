//! Dimension lines: a compound of the measured line, a box with its length
//! and (optionally) a tick at each end, recomputed whenever the line
//! changes.
//!
// Ported from xfig src/d_line.c, src/u_create.c (`create_dimension_line`), src/e_scale.c
// (`rescale_dimension_line`), src/f_util.c (`dimline_components`) and src/w_msgpanel.c
// (`make_dimension_string`) (xfig licence, see LICENSE-xfig).

use crate::view::UnitScale;
use rustfig_geom::Point;
use rustfig_model::{
    Arrow, CapStyle, ColorRef, Compound, JoinStyle, Justify, LineKind, Object, ObjectKind, PolyKind, Polyline, Style,
    Text, Units,
};
use serde::{Deserialize, Serialize};

/// The comment that marks the compound (`dimline_components`).
const COMPOUND_PREFIX: &str = "Dimension line:";
const MAIN: &str = "main dimension line";
const BOX: &str = "text box";
const TICK: &str = "tick";
const FIXED: &str = "fixed text";

/// The dimension-line settings of the indicator (xfig's `cur_dimline_*`,
/// `mode.c`; its "Dimension line settings" panel).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DimlineSettings {
    /// Thickness of the main line and the ticks' default.
    pub thick: i32,
    pub style: LineKind,
    pub color: i32,
    /// Arrow at the start and the end: xfig's type*2 - 1 + style, or -1 for none (2: filled triangle).
    pub left_arrow: i32,
    pub right_arrow: i32,
    /// Arrow length and width as multiples of the line thickness.
    pub arrow_length: f64,
    pub arrow_width: f64,
    pub ticks: bool,
    pub tick_thick: i32,
    pub box_thick: i32,
    pub box_color: i32,
    pub text_color: i32,
    pub font: i32,
    pub font_size: f64,
    pub postscript: bool,
    /// The text is typed instead of showing the length.
    pub fixed: bool,
    /// Decimals of the length.
    pub precision: u32,
}

impl Default for DimlineSettings {
    fn default() -> Self {
        DimlineSettings {
            thick: 1,
            style: LineKind::Solid,
            color: 0,
            left_arrow: 2,
            right_arrow: 2,
            arrow_length: 8.0,
            arrow_width: 4.0,
            ticks: true,
            tick_thick: 1,
            box_thick: 1,
            box_color: 7,
            text_color: 0,
            font: 0,
            font_size: 12.0,
            postscript: true,
            fixed: false,
            precision: 1,
        }
    }
}

impl DimlineSettings {
    /// Keep the values in range (hostile `indicator.set` input).
    pub fn sanitize(&mut self) {
        self.thick = self.thick.clamp(0, 10_000);
        self.tick_thick = self.tick_thick.clamp(0, 10_000);
        self.box_thick = self.box_thick.clamp(0, 10_000);
        self.left_arrow = self.left_arrow.clamp(-1, 40);
        self.right_arrow = self.right_arrow.clamp(-1, 40);
        self.precision = self.precision.min(9);
        for v in [&mut self.arrow_length, &mut self.arrow_width] {
            if !v.is_finite() {
                *v = 4.0;
            }
            *v = v.clamp(0.0, 1000.0);
        }
        if !self.font_size.is_finite() {
            self.font_size = 12.0;
        }
        self.font_size = self.font_size.clamp(1.0, 2000.0);
        self.color = self.color.clamp(-1, 543);
        self.box_color = self.box_color.clamp(-1, 543);
        self.text_color = self.text_color.clamp(-1, 543);
    }

    /// `backward_dim_arrow` / `forward_dim_arrow`: the arrow of an end.
    fn arrow(&self, v: i32) -> Option<Arrow> {
        if v < 0 {
            return None;
        }
        let kind = (v + 1) / 2;
        let t = f64::from(self.thick.max(1));
        Some(Arrow {
            kind,
            style: v - (kind * 2 - 1),
            thickness: t,
            // 60 x 120 Fig units for a thickness of 1, like xfig's default arrow.
            width: (self.arrow_width * 15.0 * t).max(1.0),
            height: (self.arrow_length * 15.0 * t).max(1.0),
        })
    }
}

/// What a new dimension line takes from the rest of the indicator.
pub struct DimlineDefaults {
    pub pen_color: i32,
    pub depth: i32,
    pub join: JoinStyle,
    pub style_val: f64,
    pub dot_gap: f64,
}

/// Whether the object is a dimension line (its compound comment).
pub fn is_dimline(o: &Object) -> bool {
    matches!(o.kind, ObjectKind::Compound(_)) && o.comments.first().is_some_and(|c| c.starts_with(COMPOUND_PREFIX))
}

/// xfig's `make_dimension_string`: the length in the figure units and scale,
/// with `precision` decimals, or fractions of an inch ([`crate::view::length_string`]).
pub fn length_text(length: f64, units: Units, scale: &UnitScale, ruler: &str, precision: u32) -> String {
    crate::view::length_string(length, units, scale, ruler, precision)
}

fn line_style(kind: LineKind, thickness: i32, d: &DimlineDefaults) -> (LineKind, f64) {
    let t = f64::from(thickness);
    match kind {
        LineKind::Solid | LineKind::Default => (kind, 0.0),
        LineKind::Dotted => (kind, d.dot_gap * (t + 1.0) / 2.0),
        k => (k, d.style_val * (t + 1.0) / 2.0),
    }
}

/// `create_dimension_line`: the compound for the line from `a` to `b`. The
/// shapes of the ticks and box are filled in by [`recompute`].
pub fn create(
    s: &DimlineSettings,
    d: &DimlineDefaults,
    a: Point,
    b: Point,
    text: Option<&str>,
    fmt: &dyn Fn(f64) -> String,
) -> Object {
    // Two objects are needed on top of the line.
    let depth = d.depth.max(2);
    let (kind, style_val) = line_style(s.style, s.thick, d);
    let main_style = Style {
        line: kind,
        thickness: s.thick,
        pen_color: ColorRef(s.color),
        fill_color: ColorRef::WHITE,
        depth,
        pen_style: -1,
        area_fill: -1,
        style_val,
        cap: CapStyle::Butt,
        join: d.join,
    };
    let mut line = Polyline::new(PolyKind::Polyline, main_style, vec![a, b]);
    line.backward_arrow = s.arrow(s.left_arrow);
    line.forward_arrow = s.arrow(s.right_arrow);
    let mut main = Object::new(ObjectKind::Polyline(line));
    main.comments = vec![MAIN.into()];

    let box_style = Style {
        line: LineKind::Solid,
        thickness: s.box_thick,
        pen_color: ColorRef(d.pen_color),
        fill_color: ColorRef(s.box_color),
        depth: depth - 1,
        pen_style: -1,
        // Full saturation of the fill colour.
        area_fill: 20,
        style_val: 0.0,
        cap: CapStyle::Butt,
        join: d.join,
    };
    let mut boxed = Object::new(ObjectKind::Polyline(Polyline::new(
        PolyKind::Polygon,
        box_style,
        vec![Point::ZERO; 5],
    )));
    boxed.comments = vec![BOX.into()];

    let mut members = vec![main, boxed];
    if s.ticks {
        for _ in 0..2 {
            let tick_style = Style {
                line: LineKind::Solid,
                thickness: s.tick_thick,
                pen_color: ColorRef(s.color),
                fill_color: ColorRef::WHITE,
                depth,
                pen_style: -1,
                area_fill: -1,
                style_val: 0.0,
                cap: CapStyle::Butt,
                join: d.join,
            };
            let mut t = Object::new(ObjectKind::Polyline(Polyline::new(
                PolyKind::Polyline,
                tick_style,
                vec![Point::ZERO; 2],
            )));
            t.comments = vec![TICK.into()];
            members.push(t);
        }
    }

    let mut label = Object::new(ObjectKind::Text(Text {
        justify: Justify::Center,
        color: ColorRef(s.text_color),
        depth: depth - 2,
        pen_style: -1,
        font: s.font,
        size: s.font_size,
        angle: 0.0,
        flags: if s.postscript {
            rustfig_model::text_flags::POSTSCRIPT
        } else {
            0
        },
        height: 0.0,
        length: 0.0,
        origin: Point::ZERO,
        text: String::new(),
    }));
    if s.fixed {
        label.comments = vec![FIXED.into()];
        if let ObjectKind::Text(t) = &mut label.kind {
            t.text = text.unwrap_or_default().to_string();
        }
    }
    members.push(label);

    let mut comp = Object::new(ObjectKind::Compound(Compound { objects: members }));
    comp.comments = vec![if s.fixed {
        format!("{COMPOUND_PREFIX} User-defined text")
    } else {
        COMPOUND_PREFIX.to_string()
    }];
    recompute(&mut comp, s, fmt);
    comp
}

/// A text whose comment starts with "fixed text" keeps its string.
fn is_fixed(m: &Object) -> bool {
    m.comments
        .first()
        .and_then(|x| x.get(..FIXED.len()))
        .is_some_and(|x| x.eq_ignore_ascii_case(FIXED))
}

fn round(v: f64) -> f64 {
    v.round()
}

/// `rescale_dimension_line` with no scaling: recompute the text and its
/// angle, the box and the ticks from the main line. Returns false when the
/// object is not a dimension line or its main line was deleted.
pub fn recompute(o: &mut Object, s: &DimlineSettings, fmt: &dyn Fn(f64) -> String) -> bool {
    if !is_dimline(o) {
        return false;
    }
    let ObjectKind::Compound(c) = &mut o.kind else {
        return false;
    };
    let main_pts = c.objects.iter().find_map(|m| match &m.kind {
        ObjectKind::Polyline(p) if m.comments.first().map(String::as_str) == Some(MAIN) => Some(p.points.clone()),
        _ => None,
    });
    let Some((mut p1, mut p2)) = main_pts.as_deref().and_then(|p| Some((*p.first()?, *p.get(1)?))) else {
        return false;
    };
    // Drawn right to left, or top to bottom at 90 degrees: swap the two ends.
    if p1.x > p2.x || (p1.y < p2.y && p1.x == p2.x) {
        std::mem::swap(&mut p1, &mut p2);
    }
    let (dx, dy) = (p2.x - p1.x, p2.y - p1.y);
    let length = dx.hypot(dy);
    if !length.is_finite() || length < 1e-9 {
        return false;
    }
    let (cx, cy) = (round((p1.x + p2.x) / 2.0), round((p1.y + p2.y) / 2.0));
    let mut angle = -dy.atan2(dx);
    while angle < 0.0 {
        angle += std::f64::consts::TAU;
    }

    // The text: the length unless the text is fixed.
    let text_idx = c.objects.iter().position(|m| matches!(m.kind, ObjectKind::Text(_)));
    let (ascent, descent, text_len) = {
        let idx = match text_idx {
            Some(i) => i,
            None => {
                // The user deleted it: make a new one.
                let mut t = Object::new(ObjectKind::Text(Text {
                    justify: Justify::Center,
                    color: ColorRef(s.text_color),
                    depth: 0,
                    pen_style: -1,
                    font: s.font,
                    size: s.font_size,
                    angle: 0.0,
                    flags: if s.postscript {
                        rustfig_model::text_flags::POSTSCRIPT
                    } else {
                        0
                    },
                    height: 0.0,
                    length: 0.0,
                    origin: Point::ZERO,
                    text: String::new(),
                }));
                t.comments = Vec::new();
                c.objects.push(t);
                c.objects.len() - 1
            }
        };
        let fixed = c.objects.get(idx).is_some_and(is_fixed);
        let Some(Object {
            kind: ObjectKind::Text(t),
            ..
        }) = c.objects.get_mut(idx)
        else {
            return false;
        };
        if !fixed {
            t.text = fmt(length);
        }
        let req = rustfig_text::FontRequest::for_fig(t.font, t.is_postscript());
        let m = rustfig_text::measure(&req, t.size, &t.text);
        t.length = m.width;
        t.height = m.ascent;
        t.angle = angle;
        t.justify = Justify::Center;
        t.origin = Point::new(round(cx - dy / length * m.descent), round(cy + dx / length * m.descent));
        (m.ascent, m.descent, m.width)
    };
    let fixed_text = c
        .objects
        .iter()
        .any(|m| matches!(m.kind, ObjectKind::Text(_)) && is_fixed(m));
    if !fixed_text {
        o.comments = vec![format!("{COMPOUND_PREFIX} {}", fmt(length))];
    }
    let ObjectKind::Compound(c) = &mut o.kind else {
        return false;
    };

    // Half the text length plus a margin, half the height plus a margin.
    let tlen2 = text_len / 2.0 + 60.0;
    let tht2 = (ascent + descent) / 2.0 + 15.0;
    let (ux, uy) = (dx / length, dy / length);
    // A point at `a` along the line and `b` across it, from `origin`.
    let at = |origin: Point, a: f64, b: f64| {
        Point::new(round(origin.x + a * ux - b * uy), round(origin.y + a * uy + b * ux))
    };
    let centre = Point::new(cx, cy);
    let mut ticks_done = 0;
    for m in &mut c.objects {
        let tag = m.comments.first().map(String::as_str);
        let ObjectKind::Polyline(pl) = &mut m.kind else {
            continue;
        };
        match tag {
            Some(BOX) => {
                pl.points = vec![
                    at(centre, -tlen2, -tht2),
                    at(centre, -tlen2, tht2),
                    at(centre, tlen2, tht2),
                    at(centre, tlen2, -tht2),
                    at(centre, -tlen2, -tht2),
                ];
            }
            Some(TICK) => {
                let end = if ticks_done == 0 { p1 } else { p2 };
                ticks_done += 1;
                pl.points = vec![at(end, 0.0, -tht2), at(end, 0.0, tht2)];
            }
            _ => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(l: f64) -> String {
        length_text(l, Units::Inches, &UnitScale::default(), "in", 1)
    }

    fn parts(o: &Object) -> Vec<(String, Vec<Point>)> {
        let ObjectKind::Compound(c) = &o.kind else {
            return Vec::new();
        };
        c.objects
            .iter()
            .filter_map(|m| match &m.kind {
                ObjectKind::Polyline(p) => Some((m.comments.first().cloned().unwrap_or_default(), p.points.clone())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn horizontal_dimension_line() {
        let d = DimlineDefaults {
            pen_color: 0,
            depth: 50,
            join: JoinStyle::Miter,
            style_val: 4.0,
            dot_gap: 3.0,
        };
        let o = create(
            &DimlineSettings::default(),
            &d,
            Point::new(0.0, 0.0),
            Point::new(2400.0, 0.0),
            None,
            &fmt,
        );
        assert!(is_dimline(&o));
        assert_eq!(o.comments, vec!["Dimension line: 2 in".to_string()]);
        let p = parts(&o);
        assert_eq!(p.len(), 4, "line, box and two ticks, besides the text");
        let ticks: Vec<_> = p.iter().filter(|(t, _)| t == "tick").collect();
        assert_eq!(ticks.len(), 2);
        // The ticks stand upright at the ends.
        assert_eq!(ticks[0].1[0].x, 0.0);
        assert_eq!(ticks[0].1[1].x, 0.0);
        assert_eq!(ticks[1].1[0].x, 2400.0);
    }

    #[test]
    fn dimension_line_label_uses_fractions() {
        let d = DimlineDefaults {
            pen_color: 0,
            depth: 50,
            join: JoinStyle::Miter,
            style_val: 4.0,
            dot_gap: 3.0,
        };
        let o = create(
            &DimlineSettings::default(),
            &d,
            Point::new(0.0, 0.0),
            Point::new(1800.0, 0.0),
            None,
            &fmt,
        );
        assert_eq!(o.comments, vec!["Dimension line: 1-1/2 in".to_string()]);
    }

    #[test]
    fn length_text_uses_unit_scale() {
        let sc = UnitScale {
            factor: 2.0,
            unit: "m".into(),
            ..UnitScale::default()
        };
        assert_eq!(length_text(450.0, Units::Metric, &sc, "cm", 2), "2.00 m");
    }
}
