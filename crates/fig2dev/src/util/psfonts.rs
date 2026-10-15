//! PostScript font tables (`PSfontnames`, `PSfontmap`, `PSneedsutf8`) and
//! `unpsfont`, which swaps a PostScript font for the nearest LaTeX font.
//!
//! The tables have 36 entries: index `font + 1`, so that the default
//! PostScript font (−1) is entry 0.
//
// Ported from fig2dev dev/psfonts.c (xfig licence, see LICENSE-xfig).

use crate::gendev::Ctx;
use crate::object::{Text, BOLD_FONT, ITALIC_FONT, MODERN_FONT, PSFONT_TEXT, ROMAN_FONT, TYPEWRITER_FONT};

/// Number of entries in the PostScript font tables (default + 35 fonts).
pub const NUM_PSFONTS: usize = 36;

/// PostScript font names, index `font + 1` (`PSfontnames`).
pub const PS_FONT_NAMES: [&str; NUM_PSFONTS] = [
    "Times-Roman", // default
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
    "Symbol",
    "ZapfChancery-MediumItalic",
    "ZapfDingbats",
];

/// The LaTeX font that stands in for each PostScript font, index
/// `font + 1` (`PSfontmap`).
pub const PS_FONT_MAP: [i32; NUM_PSFONTS] = [
    ROMAN_FONT,
    ROMAN_FONT,      // Times-Roman
    ITALIC_FONT,     // Times-Italic
    BOLD_FONT,       // Times-Bold
    BOLD_FONT,       // Times-BoldItalic
    ROMAN_FONT,      // AvantGarde
    ROMAN_FONT,      // AvantGarde-BookOblique
    ROMAN_FONT,      // AvantGarde-Demi
    ROMAN_FONT,      // AvantGarde-DemiOblique
    ROMAN_FONT,      // Bookman-Light
    ITALIC_FONT,     // Bookman-LightItalic
    ROMAN_FONT,      // Bookman-Demi
    ITALIC_FONT,     // Bookman-DemiItalic
    TYPEWRITER_FONT, // Courier
    TYPEWRITER_FONT, // Courier-Oblique
    BOLD_FONT,       // Courier-Bold
    BOLD_FONT,       // Courier-BoldItalic
    MODERN_FONT,     // Helvetica
    MODERN_FONT,     // Helvetica-Oblique
    BOLD_FONT,       // Helvetica-Bold
    BOLD_FONT,       // Helvetica-BoldOblique
    MODERN_FONT,     // Helvetica-Narrow
    MODERN_FONT,     // Helvetica-Narrow-Oblique
    BOLD_FONT,       // Helvetica-Narrow-Bold
    BOLD_FONT,       // Helvetica-Narrow-BoldOblique
    ROMAN_FONT,      // NewCenturySchlbk-Roman
    ITALIC_FONT,     // NewCenturySchlbk-Italic
    BOLD_FONT,       // NewCenturySchlbk-Bold
    BOLD_FONT,       // NewCenturySchlbk-BoldItalic
    ROMAN_FONT,      // Palatino-Roman
    ITALIC_FONT,     // Palatino-Italic
    BOLD_FONT,       // Palatino-Bold
    BOLD_FONT,       // Palatino-BoldItalic
    ROMAN_FONT,      // Symbol
    ROMAN_FONT,      // ZapfChancery-MediumItalic
    ROMAN_FONT,      // ZapfDingbats
];

/// Whether substituting the font deserves a warning (`PSmapwarn`).
const PS_MAP_WARN: [bool; NUM_PSFONTS] = [
    false, false, false, false, false, // Times
    true, true, true, true, // AvantGarde
    true, true, true, true, // Bookman
    false, true, true, true, // Courier
    false, true, true, true, // Helvetica
    true, true, true, true, // Helvetica-Narrow
    true, true, true, true, // NewCenturySchlbk
    true, true, true, true, // Palatino
    true, true, true, // Symbol, ZapfChancery, ZapfDingbats
];

/// 0: the font needs only ASCII; 2: Symbol and Zapf Dingbats, which need
/// Latin-1 (`PSneedsutf8`).
pub const PS_NEEDS_UTF8: [i32; NUM_PSFONTS] = {
    let mut t = [0; NUM_PSFONTS];
    t[33] = 2; // Symbol
    t[35] = 2; // ZapfDingbats
    t
};

/// Names of the LaTeX fonts, index `PSfontmap[…] + 1` (`figfontnames`).
const FIG_FONT_NAMES: [&str; 7] = ["Roman", "Roman", "Roman", "Bold", "Italic", "Modern", "Typewriter"];

/// The table index of a PostScript font number: `font + 1`, with numbers
/// outside −1..=34 taken as the default font (C indexes out of bounds).
pub fn ps_index(font: i32) -> usize {
    match usize::try_from(font.saturating_add(1)) {
        Ok(i) if i < NUM_PSFONTS => i,
        _ => 0,
    }
}

/// The PostScript name of `font` (`PSfontnames[font + 1]`).
pub fn ps_font_name(font: i32) -> &'static str {
    PS_FONT_NAMES.get(ps_index(font)).copied().unwrap_or("Times-Roman")
}

/// `unpsfont`: turns a PostScript-font text into one in the nearest LaTeX
/// font, warning where the substitute differs much. Leaves the
/// `PSFONT_TEXT` flag alone, as C does; the caller clears it.
pub fn unpsfont(ctx: &mut Ctx, t: &mut Text) {
    if !t.psfont_text() {
        return;
    }
    let i = ps_index(t.font);
    let map = PS_FONT_MAP.get(i).copied().unwrap_or(ROMAN_FONT);
    if PS_MAP_WARN.get(i).copied().unwrap_or(false) {
        let sub = usize::try_from(map + 1)
            .ok()
            .and_then(|j| FIG_FONT_NAMES.get(j))
            .copied()
            .unwrap_or("Roman");
        ctx.warn(format!(
            "PS fonts not supported; substituting {} for {}",
            sub,
            ps_font_name(t.font)
        ));
    }
    // Leave the default to be the default, but not a PostScript font.
    t.font = if t.font == -1 { 0 } else { map };
}

/// `unpsfont` followed by clearing the flag, as `select_font` does.
pub fn unpsfont_clear(ctx: &mut Ctx, t: &mut Text) {
    if t.psfont_text() {
        unpsfont(ctx, t);
        t.flags &= !PSFONT_TEXT;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables() {
        assert_eq!(PS_FONT_NAMES[0], "Times-Roman");
        assert_eq!(PS_FONT_NAMES[33], "Symbol");
        assert_eq!(PS_FONT_NAMES[35], "ZapfDingbats");
        assert_eq!(PS_FONT_MAP[14], TYPEWRITER_FONT);
        assert_eq!(PS_NEEDS_UTF8.iter().filter(|&&v| v == 2).count(), 2);
        assert_eq!(ps_font_name(-1), "Times-Roman");
        assert_eq!(ps_font_name(99), "Times-Roman");
        assert_eq!(ps_font_name(16), "Helvetica");
    }

    #[test]
    fn unpsfont_maps_and_warns() {
        let mut ctx = Ctx::default();
        let mut t = Text {
            font: 16,
            flags: PSFONT_TEXT,
            ..Text::default()
        };
        unpsfont(&mut ctx, &mut t);
        assert_eq!(t.font, MODERN_FONT);
        assert!(ctx.messages.is_empty());
        let mut t = Text {
            font: 32,
            flags: PSFONT_TEXT,
            ..Text::default()
        };
        unpsfont_clear(&mut ctx, &mut t);
        assert_eq!(t.font, ROMAN_FONT);
        assert!(!t.psfont_text());
        assert_eq!(ctx.messages, ["PS fonts not supported; substituting Roman for Symbol"]);
        let mut t = Text {
            font: -1,
            flags: PSFONT_TEXT,
            ..Text::default()
        };
        unpsfont(&mut ctx, &mut t);
        assert_eq!(t.font, 0);
        // Not a PostScript font: unchanged.
        let mut t = Text {
            font: 3,
            flags: 0,
            ..Text::default()
        };
        unpsfont(&mut ctx, &mut t);
        assert_eq!(t.font, 3);
    }
}
