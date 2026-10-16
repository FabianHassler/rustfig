//! Colours: RGB values, the 32 standard Fig colours, area-fill shading and
//! X11 colour names.
//
// The standard colours and area-fill shading follow xfig src/u_colors.c and
// fig2dev colors.c (xfig licence, see LICENSE-xfig).

mod xcolors;

pub use xcolors::{x_color, x_color_names};

use serde::{Deserialize, Serialize};

/// An sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb::new(0, 0, 0);
    pub const WHITE: Rgb = Rgb::new(255, 255, 255);

    pub const fn new(r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r, g, b }
    }

    /// Parse `#rrggbb` (the `#` is optional).
    pub fn from_hex(s: &str) -> Option<Rgb> {
        let h = s.trim().trim_start_matches('#');
        if h.len() != 6 || !h.is_ascii() {
            return None;
        }
        let v = u32::from_str_radix(h, 16).ok()?;
        Some(Rgb::new((v >> 16) as u8, (v >> 8) as u8, v as u8))
    }

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Linear mix: `t = 0` gives `self`, `t = 1` gives `o`.
    pub fn mix(self, o: Rgb, t: f64) -> Rgb {
        let t = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 };
        let m = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round().clamp(0.0, 255.0) as u8;
        Rgb::new(m(self.r, o.r), m(self.g, o.g), m(self.b, o.b))
    }
}

/// The 32 standard Fig colours, by colour number.
pub const STANDARD_COLORS: [Rgb; 32] = [
    Rgb::new(0, 0, 0),       // 0 black
    Rgb::new(0, 0, 255),     // 1 blue
    Rgb::new(0, 255, 0),     // 2 green
    Rgb::new(0, 255, 255),   // 3 cyan
    Rgb::new(255, 0, 0),     // 4 red
    Rgb::new(255, 0, 255),   // 5 magenta
    Rgb::new(255, 255, 0),   // 6 yellow
    Rgb::new(255, 255, 255), // 7 white
    Rgb::new(0, 0, 144),     // 8 blue4
    Rgb::new(0, 0, 176),     // 9 blue3
    Rgb::new(0, 0, 208),     // 10 blue2
    Rgb::new(135, 206, 255), // 11 ltblue
    Rgb::new(0, 144, 0),     // 12 green4
    Rgb::new(0, 176, 0),     // 13 green3
    Rgb::new(0, 208, 0),     // 14 green2
    Rgb::new(0, 144, 144),   // 15 cyan4
    Rgb::new(0, 176, 176),   // 16 cyan3
    Rgb::new(0, 208, 208),   // 17 cyan2
    Rgb::new(144, 0, 0),     // 18 red4
    Rgb::new(176, 0, 0),     // 19 red3
    Rgb::new(208, 0, 0),     // 20 red2
    Rgb::new(144, 0, 144),   // 21 magenta4
    Rgb::new(176, 0, 176),   // 22 magenta3
    Rgb::new(208, 0, 208),   // 23 magenta2
    Rgb::new(128, 48, 0),    // 24 brown4
    Rgb::new(160, 64, 0),    // 25 brown3
    Rgb::new(192, 96, 0),    // 26 brown2
    Rgb::new(255, 128, 128), // 27 pink4
    Rgb::new(255, 160, 160), // 28 pink3
    Rgb::new(255, 192, 192), // 29 pink2
    Rgb::new(255, 224, 224), // 30 pink
    Rgb::new(255, 215, 0),   // 31 gold
];

/// Display names of the standard colours.
pub const STANDARD_COLOR_NAMES: [&str; 32] = [
    "Black", "Blue", "Green", "Cyan", "Red", "Magenta", "Yellow", "White", "Blue4", "Blue3", "Blue2", "LtBlue",
    "Green4", "Green3", "Green2", "Cyan4", "Cyan3", "Cyan2", "Red4", "Red3", "Red2", "Magenta4", "Magenta3",
    "Magenta2", "Brown4", "Brown3", "Brown2", "Pink4", "Pink3", "Pink2", "Pink", "Gold",
];

/// The solid colour produced by an area-fill value (0–40) applied to a fill
/// colour, following the Fig format's shade/tint rules. `is_default_or_black`
/// selects the grey-scale interpretation used for black and default colours,
/// `is_white` the inverted one used for white. Returns `None` for "no fill"
/// and for patterns (41 and up).
pub fn area_fill_color(color: Rgb, is_default_or_black: bool, is_white: bool, fill: i32) -> Option<Rgb> {
    if !(0..=40).contains(&fill) {
        return None;
    }
    let f = fill as f64;
    if is_default_or_black {
        // 0 = white … 20 = black.
        return Some(Rgb::WHITE.mix(Rgb::BLACK, (f / 20.0).min(1.0)));
    }
    if is_white {
        // 0 = black … 20 = white.
        return Some(Rgb::BLACK.mix(Rgb::WHITE, (f / 20.0).min(1.0)));
    }
    if fill <= 20 {
        Some(Rgb::BLACK.mix(color, f / 20.0))
    } else {
        Some(color.mix(Rgb::WHITE, (f - 20.0) / 20.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let c = Rgb::from_hex("#330099").unwrap();
        assert_eq!(c, Rgb::new(0x33, 0, 0x99));
        assert_eq!(c.to_hex(), "#330099");
        assert!(Rgb::from_hex("#33009").is_none());
        assert!(Rgb::from_hex("#ééé").is_none());
    }

    #[test]
    fn shading() {
        let red = Rgb::new(255, 0, 0);
        assert_eq!(area_fill_color(red, false, false, 20), Some(red));
        assert_eq!(area_fill_color(red, false, false, 0), Some(Rgb::BLACK));
        assert_eq!(area_fill_color(red, false, false, 40), Some(Rgb::WHITE));
        assert_eq!(area_fill_color(Rgb::BLACK, true, false, 0), Some(Rgb::WHITE));
        assert_eq!(area_fill_color(Rgb::BLACK, true, false, 20), Some(Rgb::BLACK));
        assert_eq!(area_fill_color(Rgb::WHITE, false, true, 20), Some(Rgb::WHITE));
        assert_eq!(area_fill_color(red, false, false, -1), None);
        assert_eq!(area_fill_color(red, false, false, 45), None);
    }
}
