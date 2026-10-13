//! The current drawing attributes (xfig's indicator panel) and grid/snap modes.

use rustfig_geom::Point;
use rustfig_model::{Arrow, CapStyle, ColorRef, JoinStyle, Justify, LineKind, Style, Units};
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
    /// Dash length / dot gap in 1/80 inch.
    pub style_val: f64,
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
            style_val: 4.0,
            cap: CapStyle::Butt,
            join: JoinStyle::Miter,
            forward_arrow: false,
            backward_arrow: false,
            arrow: Arrow::default(),
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
            snap: SnapMode::Small,
            align_h: "center".into(),
            align_v: "none".into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping() {
        let ind = Indicator::default();
        let p = ind.snap_point(Point::new(160.0, 220.0), Units::Inches);
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

    #[test]
    fn json_partial_update() {
        let v = serde_json::json!({"thickness": 3, "line": "dashed"});
        let ind: Indicator = serde_json::from_value(v).unwrap();
        assert_eq!(ind.thickness, 3);
        assert_eq!(ind.line, LineKind::Dashed);
        assert_eq!(ind.font_size, 12.0);
    }
}
