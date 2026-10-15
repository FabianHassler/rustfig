//! The current drawing attributes (xfig's indicator panel) and grid/snap modes.

use rustfig_geom::Point;
use rustfig_model::{
    Arrow, CapStyle, ColorRef, JoinStyle, Justify, LineKind, Object, ObjectKind, PolyKind, Style, Units,
};
use serde::{Deserialize, Serialize};

/// Grid spacing / point positioning steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SnapMode {
    /// Free positioning.
    Any,
    /// 1/16 inch or 1 mm.
    Fine,
    /// 1/8 inch or 2 mm.
    #[default]
    Small,
    /// 1/4 inch or 5 mm.
    Medium,
    /// 1/2 inch or 1 cm.
    Large,
    /// 1 inch or 2 cm.
    Coarse,
}

impl SnapMode {
    pub const ALL: [SnapMode; 6] = [
        SnapMode::Any,
        SnapMode::Fine,
        SnapMode::Small,
        SnapMode::Medium,
        SnapMode::Large,
        SnapMode::Coarse,
    ];

    /// Step in Fig units (0 for free positioning).
    pub fn step(self, units: Units) -> f64 {
        let (inch, metric) = match self {
            SnapMode::Any => (0.0, 0.0),
            SnapMode::Fine => (75.0, 45.0),
            SnapMode::Small => (150.0, 90.0),
            SnapMode::Medium => (300.0, 225.0),
            SnapMode::Large => (600.0, 450.0),
            SnapMode::Coarse => (1200.0, 900.0),
        };
        match units {
            Units::Inches => inch,
            Units::Metric => metric,
        }
    }

    pub fn label(self, units: Units) -> &'static str {
        match (self, units) {
            (SnapMode::Any, _) => "any",
            (SnapMode::Fine, Units::Inches) => "1/16 in",
            (SnapMode::Small, Units::Inches) => "1/8 in",
            (SnapMode::Medium, Units::Inches) => "1/4 in",
            (SnapMode::Large, Units::Inches) => "1/2 in",
            (SnapMode::Coarse, Units::Inches) => "1 in",
            (SnapMode::Fine, Units::Metric) => "1 mm",
            (SnapMode::Small, Units::Metric) => "2 mm",
            (SnapMode::Medium, Units::Metric) => "5 mm",
            (SnapMode::Large, Units::Metric) => "1 cm",
            (SnapMode::Coarse, Units::Metric) => "2 cm",
        }
    }
}

/// Visible grid (None hides it).
pub type GridMode = Option<SnapMode>;

/// Current attributes applied to new objects, and drawing settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Indicator {
    pub line: LineKind,
    pub thickness: i32,
    pub pen_color: i32,
    pub fill_color: i32,
    /// -1 no fill, 0–40 shade/tint, 41–62 pattern.
    pub area_fill: i32,
    pub depth: i32,
    /// Dash length in 1/80 inch (xfig's `cur_dashlength`).
    pub style_val: f64,
    /// Gap between dots of dotted lines in 1/80 inch (xfig's `cur_dotgap`).
    pub dot_gap: f64,
    pub cap: CapStyle,
    pub join: JoinStyle,
    pub forward_arrow: bool,
    pub backward_arrow: bool,
    pub arrow: Arrow,
    pub font: i32,
    pub font_size: f64,
    pub postscript: bool,
    pub justify: Justify,
    /// Text angle in degrees.
    pub text_angle: f64,
    pub hidden_text: bool,
    pub special_text: bool,
    pub rigid_text: bool,
    /// Arc-box corner radius in 1/80 inch.
    pub box_radius: i32,
    /// Regular polygon sides.
    pub sides: usize,
    /// Rotation step in degrees for the rotate tool.
    pub rotate_angle: f64,
    pub grid: Option<SnapMode>,
    pub snap: SnapMode,
    /// Align modes for the align command: "none", "left", "center", "right" / "top", "center", "bottom".
    pub align_h: String,
    pub align_v: String,
    /// Arcs are pie wedges rather than open (xfig's "Arc Type").
    pub pie_arc: bool,
    /// Line spacing of the text tool as a multiple of the font's (xfig's
    /// "Text Step").
    pub text_step: f64,
}

impl Default for Indicator {
    fn default() -> Self {
        Indicator {
            line: LineKind::Solid,
            thickness: 1,
            pen_color: 0,
            fill_color: 7,
            area_fill: -1,
            depth: 50,
            // xfig's start values (mode.c, main.c's resource defaults).
            style_val: 4.0,
            dot_gap: 3.0,
            cap: CapStyle::Butt,
            join: JoinStyle::Miter,
            forward_arrow: false,
            backward_arrow: false,
            // xfig's startarrowtype 0: the stick arrow.
            arrow: Arrow {
                kind: 0,
                style: 0,
                ..Arrow::default()
            },
            font: 0,
            font_size: 12.0,
            postscript: true,
            justify: Justify::Left,
            text_angle: 0.0,
            hidden_text: false,
            special_text: false,
            rigid_text: false,
            box_radius: 7,
            sides: 6,
            rotate_angle: 90.0,
            grid: None,
            // xfig's startposnmode 1: magnet (1/16 inch, 1 mm).
            snap: SnapMode::Fine,
            align_h: "none".into(),
            align_v: "none".into(),
            pie_arc: false,
            text_step: 1.0,
        }
    }
}

impl Indicator {
    /// The style for a new object.
    pub fn style(&self) -> Style {
        Style {
            line: self.line,
            thickness: self.thickness.clamp(0, 10_000),
            pen_color: ColorRef(self.pen_color),
            fill_color: ColorRef(self.fill_color),
            depth: self.depth.clamp(0, rustfig_model::MAX_DEPTH),
            pen_style: -1,
            area_fill: self.area_fill.clamp(-1, 62),
            style_val: match self.line {
                LineKind::Solid | LineKind::Default => 0.0,
                LineKind::Dotted => self.dot_gap,
                _ => self.style_val,
            },
            cap: self.cap,
            join: self.join,
        }
    }

    pub fn arrows(&self) -> (Option<Arrow>, Option<Arrow>) {
        let a = Arrow {
            thickness: f64::from(self.thickness.max(1)),
            ..self.arrow
        };
        (self.forward_arrow.then_some(a), self.backward_arrow.then_some(a))
    }

    /// Copy an object's attributes into the indicator (xfig's Update mode,
    /// middle button: "update settings"). The reverse of `object.update`.
    pub fn take_from(&mut self, o: &Object) {
        if let ObjectKind::Text(t) = &o.kind {
            use rustfig_model::text_flags::*;
            self.pen_color = t.color.0;
            self.depth = t.depth;
            self.font = t.font;
            self.font_size = t.size;
            self.justify = t.justify;
            self.text_angle = t.angle.to_degrees();
            self.rigid_text = t.flags & RIGID != 0;
            self.special_text = t.flags & SPECIAL != 0;
            self.postscript = t.flags & POSTSCRIPT != 0;
            self.hidden_text = t.flags & HIDDEN != 0;
            return;
        }
        if let Some(s) = o.style() {
            self.line = s.line;
            self.thickness = s.thickness;
            self.pen_color = s.pen_color.0;
            self.fill_color = s.fill_color.0;
            self.depth = s.depth;
            self.area_fill = s.area_fill;
            if s.style_val > 0.0 {
                if s.line == LineKind::Dotted {
                    self.dot_gap = s.style_val;
                } else {
                    self.style_val = s.style_val;
                }
            }
            self.cap = s.cap;
            self.join = s.join;
        }
        if let Some((f, b)) = o.arrows() {
            self.forward_arrow = f.is_some();
            self.backward_arrow = b.is_some();
            if let Some(a) = f.or(b) {
                self.arrow = a;
            }
        }
        if let ObjectKind::Polyline(p) = &o.kind {
            if p.kind == PolyKind::ArcBox {
                self.box_radius = p.radius;
            }
        }
        if let ObjectKind::Arc(a) = &o.kind {
            self.pie_arc = a.pie;
        }
    }

    pub fn text_flags(&self) -> i32 {
        use rustfig_model::text_flags::*;
        let mut f = 0;
        if self.rigid_text {
            f |= RIGID;
        }
        if self.special_text {
            f |= SPECIAL;
        }
        if self.postscript {
            f |= POSTSCRIPT;
        }
        if self.hidden_text {
            f |= HIDDEN;
        }
        f
    }

    /// Snap a point to the positioning grid.
    pub fn snap_point(&self, p: Point, units: Units) -> Point {
        let step = self.snap.step(units);
        if step <= 0.0 || !p.is_finite() {
            return p;
        }
        Point::new((p.x / step).round() * step, (p.y / step).round() * step)
    }
}

/// xfig's arrow type choices (`arrowtype_choices` in `w_indpanel.c`): 0 is
/// the stick arrow, then each type 1–14 hollow and filled. Returns (kind,
/// style); the `startarrowtype` resource is such a choice.
pub fn arrow_choice(n: i32) -> (i32, i32) {
    if n <= 0 {
        (0, 0)
    } else {
        let n = n.min(28);
        ((n + 1) / 2, if n % 2 == 1 { 0 } else { 1 })
    }
}

/// The choice of arrow `kind` and `style` (the inverse of [`arrow_choice`]).
pub fn arrow_choice_of(kind: i32, style: i32) -> i32 {
    if kind <= 0 {
        0
    } else {
        kind * 2 - 1 + i32::from(style == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping() {
        // xfig's default: magnet, 1/16 inch.
        let ind = Indicator::default();
        assert_eq!(ind.snap, SnapMode::Fine);
        let p = ind.snap_point(Point::new(160.0, 220.0), Units::Inches);
        assert_eq!(p, Point::new(150.0, 225.0));
        let eighth = Indicator {
            snap: SnapMode::Small,
            ..Indicator::default()
        };
        let p = eighth.snap_point(Point::new(160.0, 220.0), Units::Inches);
        assert_eq!(p, Point::new(150.0, 150.0));
        let any = Indicator {
            snap: SnapMode::Any,
            ..Indicator::default()
        };
        assert_eq!(
            any.snap_point(Point::new(1.5, 2.5), Units::Inches),
            Point::new(1.5, 2.5)
        );
    }

    /// The start values are xfig's (`mode.c`, `main.c`'s resource defaults,
    /// `object.h`): gap 23.
    #[test]
    fn start_values_are_xfigs() {
        let i = Indicator::default();
        assert_eq!(i.snap, SnapMode::Fine, "startposnmode 1 (magnet)");
        assert_eq!(i.grid, None, "startgridmode 0");
        assert_eq!((i.arrow.kind, i.arrow.style), (0, 0), "startarrowtype 0");
        assert_eq!((i.arrow.width, i.arrow.height), (60.0, 120.0), "DEF_ARROW_WID/HT 4, 8");
        assert_eq!((i.style_val, i.dot_gap), (4.0, 3.0), "DEF_DASHLENGTH, DEF_DOTGAP");
        assert_eq!(i.text_step, 1.0, "cur_textstep (the man page's 1.2 is wrong)");
        assert_eq!((i.font_size, i.thickness, i.depth, i.box_radius), (12.0, 1, 50, 7));
        assert_eq!((i.area_fill, i.pen_color, i.fill_color), (-1, 0, 7));
        assert_eq!((i.align_h.as_str(), i.align_v.as_str()), ("none", "none"));
        assert_eq!((i.sides, i.rotate_angle), (6, 90.0));
    }

    #[test]
    fn dotted_lines_use_the_dot_gap() {
        let i = Indicator {
            line: LineKind::Dotted,
            ..Indicator::default()
        };
        assert_eq!(i.style().style_val, 3.0);
        let d = Indicator {
            line: LineKind::Dashed,
            ..Indicator::default()
        };
        assert_eq!(d.style().style_val, 4.0);
    }

    #[test]
    fn json_partial_update() {
        let v = serde_json::json!({"thickness": 3, "line": "dashed"});
        let ind: Indicator = serde_json::from_value(v).unwrap();
        assert_eq!(ind.thickness, 3);
        assert_eq!(ind.line, LineKind::Dashed);
        assert_eq!(ind.font_size, 12.0);
    }
}
