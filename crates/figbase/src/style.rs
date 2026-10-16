//! Line and text styles the drawing code shares: dash kinds and patterns,
//! cap and join styles, text justification.
//
// Ported in parts from fig2dev dev/genps.c (`set_style` dash patterns)
// (xfig licence, see LICENSE-xfig).

use serde::{Deserialize, Serialize};

/// Line dash style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LineKind {
    Default,
    #[default]
    Solid,
    Dashed,
    Dotted,
    DashDotted,
    DashDoubleDotted,
    DashTripleDotted,
}

impl LineKind {
    pub const ALL: [LineKind; 6] = [
        LineKind::Solid,
        LineKind::Dashed,
        LineKind::Dotted,
        LineKind::DashDotted,
        LineKind::DashDoubleDotted,
        LineKind::DashTripleDotted,
    ];

    pub fn from_fig(v: i32) -> LineKind {
        match v {
            0 => LineKind::Solid,
            1 => LineKind::Dashed,
            2 => LineKind::Dotted,
            3 => LineKind::DashDotted,
            4 => LineKind::DashDoubleDotted,
            5 => LineKind::DashTripleDotted,
            _ => LineKind::Default,
        }
    }

    pub fn to_fig(self) -> i32 {
        match self {
            LineKind::Default => -1,
            LineKind::Solid => 0,
            LineKind::Dashed => 1,
            LineKind::Dotted => 2,
            LineKind::DashDotted => 3,
            LineKind::DashDoubleDotted => 4,
            LineKind::DashTripleDotted => 5,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            LineKind::Default => "default",
            LineKind::Solid => "solid",
            LineKind::Dashed => "dashed",
            LineKind::Dotted => "dotted",
            LineKind::DashDotted => "dash-dotted",
            LineKind::DashDoubleDotted => "dash-double-dotted",
            LineKind::DashTripleDotted => "dash-triple-dotted",
        }
    }

    pub fn from_name(s: &str) -> Option<LineKind> {
        [LineKind::Default]
            .into_iter()
            .chain(LineKind::ALL)
            .find(|k| k.name() == s)
    }

    /// Dash pattern in Fig units for a given style value (1/80 inch) and
    /// line width. Empty for solid lines. Dots are zero-length dashes, meant
    /// to be drawn with round caps.
    /// Dash array in Fig units for a style value in 1/80 inch.
    // Ported from fig2dev dev/genps.c `set_style` (xfig licence, see LICENSE-xfig).
    pub fn dash_pattern(self, style_val: f64, _width: f64) -> Vec<f64> {
        let unit = crate::geom::FIG_PER_STYLE_UNIT;
        let v = if style_val.is_finite() && style_val > 0.0 {
            style_val * unit
        } else {
            4.0 * unit
        };
        // A dot is 1/80 inch long.
        let dot = unit;
        match self {
            LineKind::Default | LineKind::Solid => Vec::new(),
            LineKind::Dashed => vec![v, v],
            LineKind::Dotted => vec![dot, v],
            LineKind::DashDotted => vec![v, v * 0.5, dot, v * 0.5],
            LineKind::DashDoubleDotted => vec![v, v * 0.45, dot, v * 0.333, dot, v * 0.45],
            LineKind::DashTripleDotted => vec![v, v * 0.4, dot, v * 0.3, dot, v * 0.3, dot, v * 0.4],
        }
    }
}

/// Line cap style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapStyle {
    #[default]
    Butt,
    Round,
    Projecting,
}

impl CapStyle {
    pub fn from_fig(v: i32) -> CapStyle {
        match v {
            1 => CapStyle::Round,
            2 => CapStyle::Projecting,
            _ => CapStyle::Butt,
        }
    }
    pub fn to_fig(self) -> i32 {
        self as i32
    }
}

/// Line join style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JoinStyle {
    #[default]
    Miter,
    Round,
    Bevel,
}

impl JoinStyle {
    pub fn from_fig(v: i32) -> JoinStyle {
        match v {
            1 => JoinStyle::Round,
            2 => JoinStyle::Bevel,
            _ => JoinStyle::Miter,
        }
    }
    pub fn to_fig(self) -> i32 {
        self as i32
    }
}

/// Text justification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Justify {
    #[default]
    Left,
    Center,
    Right,
}

impl Justify {
    pub fn from_fig(v: i32) -> Justify {
        match v {
            1 => Justify::Center,
            2 => Justify::Right,
            _ => Justify::Left,
        }
    }
    pub fn to_fig(self) -> i32 {
        self as i32
    }
    pub fn swapped(self) -> Justify {
        match self {
            Justify::Left => Justify::Right,
            Justify::Right => Justify::Left,
            Justify::Center => Justify::Center,
        }
    }
}
