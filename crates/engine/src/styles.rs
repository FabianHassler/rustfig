//! Named attribute sets ("styles", E6): the indicator's drawing attributes
//! kept under a name in the preferences (never in the drawing, so the .fig
//! file stays xfig's), applied back to the indicator or to the selection.

use crate::Indicator;
use rustfig_model::{Arrow, CapStyle, JoinStyle, Justify, LineKind};
use serde::{Deserialize, Serialize};

/// Most families (xfig: 16).
pub const MAX_FAMILIES: usize = 32;
/// Most styles in one family (xfig: 16).
pub const MAX_STYLES: usize = 64;
/// Longest style or family name, in characters.
pub const MAX_STYLE_NAME: usize = 64;

/// The attributes Update mode copies (everything of the indicator that
/// describes how new objects look).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AttrStyle {
    pub line: LineKind,
    pub thickness: i32,
    pub pen_color: i32,
    pub fill_color: i32,
    pub area_fill: i32,
    pub depth: i32,
    pub style_val: f64,
    pub dot_gap: f64,
    pub cap: CapStyle,
    pub join: JoinStyle,
    pub forward_arrow: bool,
    pub backward_arrow: bool,
    pub arrow: Arrow,
    pub font: i32,
    pub font_size: f64,
    pub local_font: Option<String>,
    pub postscript: bool,
    pub justify: Justify,
    pub text_angle: f64,
    pub hidden_text: bool,
    pub special_text: bool,
    pub rigid_text: bool,
    pub box_radius: i32,
    pub pie_arc: bool,
}

impl Default for AttrStyle {
    fn default() -> Self {
        AttrStyle::of(&Indicator::default())
    }
}

impl AttrStyle {
    /// The indicator's attributes.
    pub fn of(i: &Indicator) -> AttrStyle {
        AttrStyle {
            line: i.line,
            thickness: i.thickness,
            pen_color: i.pen_color,
            fill_color: i.fill_color,
            area_fill: i.area_fill,
            depth: i.depth,
            style_val: i.style_val,
            dot_gap: i.dot_gap,
            cap: i.cap,
            join: i.join,
            forward_arrow: i.forward_arrow,
            backward_arrow: i.backward_arrow,
            arrow: i.arrow,
            font: i.font,
            font_size: i.font_size,
            local_font: i.local_font.clone(),
            postscript: i.postscript,
            justify: i.justify,
            text_angle: i.text_angle,
            hidden_text: i.hidden_text,
            special_text: i.special_text,
            rigid_text: i.rigid_text,
            box_radius: i.box_radius,
            pie_arc: i.pie_arc,
        }
    }

    /// Set the indicator's attributes from the style (other indicator
    /// fields, such as grid and snap, stay). Values from a hand-edited
    /// settings file are clamped to what the indicator accepts.
    pub fn apply_to(&self, i: &mut Indicator) {
        i.line = self.line;
        i.thickness = self.thickness.clamp(0, 10_000);
        i.pen_color = self.pen_color;
        i.fill_color = self.fill_color;
        i.area_fill = self.area_fill.clamp(-1, 62);
        i.depth = self.depth.clamp(0, rustfig_model::MAX_DEPTH);
        i.style_val = if self.style_val.is_finite() {
            self.style_val
        } else {
            0.0
        };
        i.dot_gap = if self.dot_gap.is_finite() { self.dot_gap } else { 0.0 };
        i.cap = self.cap;
        i.join = self.join;
        i.forward_arrow = self.forward_arrow;
        i.backward_arrow = self.backward_arrow;
        i.arrow = self.arrow;
        i.font = self.font;
        i.local_font = self.local_font.clone();
        i.font_size = if self.font_size.is_finite() {
            self.font_size
        } else {
            i.font_size
        };
        i.postscript = self.postscript;
        i.justify = self.justify;
        i.text_angle = if self.text_angle.is_finite() {
            self.text_angle
        } else {
            0.0
        };
        i.hidden_text = self.hidden_text;
        i.special_text = self.special_text;
        i.rigid_text = self.rigid_text;
        i.box_radius = self.box_radius;
        i.pie_arc = self.pie_arc;
    }
}

/// A style and its name, as kept in the preferences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedStyle {
    pub name: String,
    #[serde(flatten)]
    pub attrs: AttrStyle,
}

/// A family of styles, as kept in the preferences.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleFamily {
    pub name: String,
    pub styles: Vec<NamedStyle>,
}

/// A valid style or family name: trimmed, 1 to 64 characters, no control characters.
pub fn clean_name(name: &str) -> Result<String, String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > MAX_STYLE_NAME || n.chars().any(char::is_control) {
        return Err(format!(
            "a style or family needs a name of 1-{MAX_STYLE_NAME} characters"
        ));
    }
    Ok(n.to_string())
}
