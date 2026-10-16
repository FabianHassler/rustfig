//! The Fig font tables and the mapping from Fig font numbers to font requests.
//
// Font tables from xfig src/u_fonts.c and fig2dev dev/psfonts.c (xfig
// licence, see LICENSE-xfig).

/// Number of PostScript fonts in the Fig format (0–34).
pub const PS_FONT_COUNT: i32 = 35;
/// Number of LaTeX fonts in the Fig format (0–5).
pub const LATEX_FONT_COUNT: i32 = 6;

/// PostScript names of the 35 standard fonts, by Fig font number.
const PS_NAMES: [&str; 35] = [
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

const PS_DISPLAY: [&str; 35] = [
    "Times Roman",
    "Times Italic",
    "Times Bold",
    "Times Bold Italic",
    "AvantGarde Book",
    "AvantGarde Book Oblique",
    "AvantGarde Demi",
    "AvantGarde Demi Oblique",
    "Bookman Light",
    "Bookman Light Italic",
    "Bookman Demi",
    "Bookman Demi Italic",
    "Courier",
    "Courier Oblique",
    "Courier Bold",
    "Courier Bold Oblique",
    "Helvetica",
    "Helvetica Oblique",
    "Helvetica Bold",
    "Helvetica Bold Oblique",
    "Helvetica Narrow",
    "Helvetica Narrow Oblique",
    "Helvetica Narrow Bold",
    "Helvetica Narrow Bold Oblique",
    "New Century Schoolbook Roman",
    "New Century Schoolbook Italic",
    "New Century Schoolbook Bold",
    "New Century Schoolbook Bold Italic",
    "Palatino Roman",
    "Palatino Italic",
    "Palatino Bold",
    "Palatino Bold Italic",
    "Symbol",
    "Zapf Chancery Medium Italic",
    "Zapf Dingbats",
];

const LATEX_NAMES: [&str; 6] = ["Default", "Roman", "Bold", "Italic", "Sans Serif", "Typewriter"];

/// PostScript font name for a Fig PostScript font number (-1 = default = Times-Roman).
pub fn ps_font_name(n: i32) -> &'static str {
    usize::try_from(n)
        .ok()
        .and_then(|i| PS_NAMES.get(i))
        .copied()
        .unwrap_or("Times-Roman")
}

/// The Fig font number of a standard PostScript font name.
pub fn ps_font_number(name: &str) -> Option<i32> {
    PS_NAMES
        .iter()
        .position(|n| *n == name)
        .and_then(|i| i32::try_from(i).ok())
}

/// Human-readable name for a Fig PostScript font number.
pub fn ps_font_display_name(n: i32) -> &'static str {
    if n < 0 {
        return "Default";
    }
    usize::try_from(n)
        .ok()
        .and_then(|i| PS_DISPLAY.get(i))
        .copied()
        .unwrap_or("Times Roman")
}

/// Human-readable name for a Fig LaTeX font number.
pub fn latex_font_name(n: i32) -> &'static str {
    usize::try_from(n)
        .ok()
        .and_then(|i| LATEX_NAMES.get(i))
        .copied()
        .unwrap_or("Default")
}

/// Generic font families used as fallbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generic {
    Serif,
    Sans,
    Mono,
}

impl Generic {
    pub(crate) fn keywords(self) -> &'static [&'static str] {
        match self {
            Generic::Serif => &["serif", "roman", "termes", "times"],
            Generic::Sans => &["sans", "heros", "helvet", "arial"],
            Generic::Mono => &["mono", "cursor", "courier", "code"],
        }
    }
}

/// What a text object asks of the font library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontRequest {
    /// Lower-case name fragments of preferred faces (e.g. "termes", "times").
    pub keywords: &'static [&'static str],
    pub generic: Generic,
    pub bold: bool,
    pub italic: bool,
}

impl FontRequest {
    /// The request for a Fig font number. `postscript` selects the
    /// PostScript font table, otherwise the LaTeX table is used.
    pub fn for_fig(font: i32, postscript: bool) -> FontRequest {
        if !postscript {
            let (generic, bold, italic) = match font {
                2 => (Generic::Serif, true, false),
                3 => (Generic::Serif, false, true),
                4 => (Generic::Sans, false, false),
                5 => (Generic::Mono, false, false),
                _ => (Generic::Serif, false, false),
            };
            let keywords: &'static [&'static str] = match generic {
                Generic::Serif => &["latin modern roman", "lmroman", "cmu serif", "computer modern"],
                Generic::Sans => &["latin modern sans", "lmsans", "cmu sans"],
                Generic::Mono => &["latin modern mono", "lmmono", "cmu typewriter"],
            };
            return FontRequest {
                keywords,
                generic,
                bold,
                italic,
            };
        }
        let n = if (0..PS_FONT_COUNT).contains(&font) { font } else { 0 };
        let (keywords, generic): (&'static [&'static str], Generic) = match n / 4 {
            0 => (
                &["termes", "nimbus roman", "times", "liberation serif", "tinos"],
                Generic::Serif,
            ),
            1 => (&["adventor", "urw gothic", "avant"], Generic::Sans),
            2 => (&["bonum", "bookman"], Generic::Serif),
            3 => (
                &["cursor", "nimbus mono", "courier", "liberation mono", "cousine"],
                Generic::Mono,
            ),
            4 => (
                &["heros", "nimbus sans", "helvetica", "liberation sans", "arimo"],
                Generic::Sans,
            ),
            5 => (
                &[
                    "heros cn",
                    "nimbus sans narrow",
                    "helvetica narrow",
                    "liberation sans narrow",
                ],
                Generic::Sans,
            ),
            6 => (&["schola", "century schoolbook", "c059"], Generic::Serif),
            7 => (&["pagella", "palladio", "palatino", "p052"], Generic::Serif),
            _ => match n {
                32 => (&["symbol", "standard symbols"], Generic::Serif),
                33 => (&["chorus", "chancery", "z003"], Generic::Serif),
                _ => (&["dingbat", "d050000l"], Generic::Sans),
            },
        };
        let (bold, italic) = if n < 32 {
            let style = n % 4;
            (style >= 2, style == 1 || style == 3)
        } else {
            (false, n == 33)
        };
        FontRequest {
            keywords,
            generic,
            bold,
            italic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables() {
        assert_eq!(ps_font_name(0), "Times-Roman");
        assert_eq!(ps_font_name(34), "ZapfDingbats");
        assert_eq!(ps_font_name(-1), "Times-Roman");
        assert_eq!(ps_font_name(1000), "Times-Roman");
        assert_eq!(latex_font_name(5), "Typewriter");
        let r = FontRequest::for_fig(19, true);
        assert!(r.bold && r.italic && r.generic == Generic::Sans);
        let r = FontRequest::for_fig(12, true);
        assert_eq!(r.generic, Generic::Mono);
    }
}
