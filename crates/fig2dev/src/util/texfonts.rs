//! TeX font tables and text output shared by the LaTeX-family drivers
//! (latex, pict2e, tikz, pictex, pstricks, textyl, mp, …): `select_font`
//! writes the font and size selection, `put_string` writes a text with
//! LaTeX's special characters quoted.
//!
//! C keeps the tables in globals that some drivers change through their
//! options (`texfontsizes[0] = …`, `texfontnames[0] = …`); here they live
//! in [`TexFonts`], which a driver owns and may change.
//!
//! Texts reach the drivers as the bytes of the input (`Text::raw`), as in C;
//! see `util::textconvert` for the conversions `put_string` applies.
//
// Ported from fig2dev dev/texfonts.c, dev/texfonts.h and dev/textconvert.c
// (xfig licence, see LICENSE-xfig).

use std::borrow::Cow;

use crate::gendev::Ctx;
use crate::object::{Text, MAX_FONT, MAX_PSFONT};
use crate::util::psfonts::unpsfont_clear;
use crate::util::textconvert::{conv_textisutf8, conv_textstring, convertutf8tolatin1};
use crate::{fprintf, round, Out};

/// Font sizes above this are used as they are (`MAXFONTSIZE`).
pub const MAXFONTSIZE: i32 = 42;

/// Number of LaTeX fonts in the tables (default + 5).
pub const NUM_TEXFONTS: usize = 6;

/// NFSS font families (`texfontfamily`).
pub const TEXFONTFAMILY: [&str; NUM_TEXFONTS] = [
    "\\familydefault", // default
    "\\rmdefault",     // roman
    "\\rmdefault",     // bold
    "\\rmdefault",     // italic
    "\\sfdefault",     // sans serif
    "\\ttdefault",     // typewriter
];

/// NFSS font series (`texfontseries`).
pub const TEXFONTSERIES: [&str; NUM_TEXFONTS] = [
    "\\mddefault", // default
    "\\mddefault", // roman
    "\\bfdefault", // bold
    "\\mddefault", // italic
    "\\mddefault", // sans serif
    "\\mddefault", // typewriter
];

/// NFSS font shapes (`texfontshape`).
pub const TEXFONTSHAPE: [&str; NUM_TEXFONTS] = [
    "\\updefault", // default
    "\\updefault", // roman
    "\\updefault", // bold
    "\\itdefault", // italic
    "\\updefault", // sans serif
    "\\updefault", // typewriter
];

/// LaTeX font switches (`texfonts`).
pub const TEXFONTS: [&str; NUM_TEXFONTS] = [
    "",           // default
    "\\rmfamily", // roman
    "\\bfseries", // bold
    "\\itshape",  // italic
    "\\sffamily", // sans serif
    "\\ttfamily", // typewriter
];

/// `\usefont` arguments for the PostScript fonts, index `font + 1`
/// (`texpsfonts`).
pub const TEXPSFONTS: [&str; 36] = [
    "{T1}{ptm}{m}{n}",   // Times-Roman, default
    "{T1}{ptm}{m}{n}",   // Times-Roman
    "{T1}{ptm}{m}{it}",  // Times-Italic
    "{T1}{ptm}{b}{n}",   // Times-Bold
    "{T1}{ptm}{b}{it}",  // Times-BoldItalic
    "{T1}{pag}{m}{n}",   // AvantGarde
    "{T1}{pag}{m}{sl}",  // AvantGarde-BookOblique
    "{T1}{pag}{db}{n}",  // AvantGarde-Demi
    "{T1}{pag}{db}{sl}", // AvantGarde-DemiOblique
    "{T1}{pbk}{l}{n}",   // Bookman-Light
    "{T1}{pbk}{l}{it}",  // Bookman-LightItalic
    "{T1}{pbk}{db}{n}",  // Bookman-Demi
    "{T1}{pbk}{db}{it}", // Bookman-DemiItalic
    "{T1}{pcr}{m}{n}",   // Courier
    "{T1}{pcr}{m}{sl}",  // Courier-Oblique
    "{T1}{pcr}{b}{n}",   // Courier-Bold
    "{T1}{pcr}{b}{sl}",  // Courier-BoldItalic
    "{T1}{phv}{m}{n}",   // Helvetica
    "{T1}{phv}{m}{sl}",  // Helvetica-Oblique
    "{T1}{phv}{b}{n}",   // Helvetica-Bold
    "{T1}{phv}{b}{sl}",  // Helvetica-BoldOblique
    "{T1}{phv}{mc}{n}",  // Helvetica-Narrow
    "{T1}{phv}{mc}{sl}", // Helvetica-Narrow-Oblique
    "{T1}{phv}{bc}{n}",  // Helvetica-Narrow-Bold
    "{T1}{phv}{bc}{sl}", // Helvetica-Narrow-BoldOblique
    "{T1}{pnc}{m}{n}",   // NewCenturySchlbk-Roman
    "{T1}{pnc}{m}{it}",  // NewCenturySchlbk-Italic
    "{T1}{pnc}{b}{n}",   // NewCenturySchlbk-Bold
    "{T1}{pnc}{b}{it}",  // NewCenturySchlbk-BoldItalic
    "{T1}{ppl}{m}{n}",   // Palatino-Roman
    "{T1}{ppl}{m}{it}",  // Palatino-Italic
    "{T1}{ppl}{b}{n}",   // Palatino-Bold
    "{T1}{ppl}{b}{it}",  // Palatino-BoldItalic
    "{U}{psy}{m}{n}",    // Symbol
    "{T1}{pzc}{mb}{it}", // ZapfChancery-MediumItalic
    "{U}{pzd}{m}{n}",    // ZapfDingbats
];

/// Old-style font names (`texfontnames`).
pub const TEXFONTNAMES: [&str; NUM_TEXFONTS] = [
    "rm", // default
    "rm", // roman
    "bf", // bold
    "it", // italic
    "sf", // sans serif
    "tt", // typewriter
];

/// The TeX font size for each point size 0..=43 (`texfontsizes`). Not
/// all fonts are preloaded at all sizes.
pub const TEXFONTSIZES: [i32; 44] = [
    11, // default
    5, 5, 5, 5, // 1-4: small fonts
    5, // five point font
    6, 7, 8, // etc
    9, 10, 11, //
    12, 12, 14, //
    14, 14, 17, //
    17, 17, 20, //
    20, 20, 20, 20, 25, //
    25, 25, 25, 29, //
    29, 29, 29, 29, //
    34, 34, 34, 34, //
    34, 34, 34, 41, //
    41, 41,
];

/// The LaTeX font index C's macros use: `F` within 0..=MAX_FONT, negative
/// numbers as 0 and larger ones as `MAX_FONT − 1`.
pub fn tex_font_index(font: i32) -> usize {
    if font > MAX_FONT {
        (MAX_FONT - 1) as usize
    } else {
        usize::try_from(font).unwrap_or(0)
    }
}

/// The TeX font tables a driver works with, initialised from fig2dev's
/// globals; drivers change entries the way the C drivers change the
/// globals (e.g. `sizes[0]` and `sizes[1]` from `-f`/`-F` options).
#[derive(Clone, Debug, PartialEq)]
pub struct TexFonts {
    pub family: [Cow<'static, str>; NUM_TEXFONTS],
    pub series: [Cow<'static, str>; NUM_TEXFONTS],
    pub shape: [Cow<'static, str>; NUM_TEXFONTS],
    pub fonts: [Cow<'static, str>; NUM_TEXFONTS],
    pub psfonts: [Cow<'static, str>; 36],
    pub names: [Cow<'static, str>; NUM_TEXFONTS],
    pub sizes: [i32; 44],
}

impl Default for TexFonts {
    fn default() -> TexFonts {
        TexFonts {
            family: TEXFONTFAMILY.map(Cow::Borrowed),
            series: TEXFONTSERIES.map(Cow::Borrowed),
            shape: TEXFONTSHAPE.map(Cow::Borrowed),
            fonts: TEXFONTS.map(Cow::Borrowed),
            psfonts: TEXPSFONTS.map(Cow::Borrowed),
            names: TEXFONTNAMES.map(Cow::Borrowed),
            sizes: TEXFONTSIZES,
        }
    }
}

impl TexFonts {
    pub fn new() -> TexFonts {
        TexFonts::default()
    }

    /// `TEXFAMILY(F)`.
    pub fn texfamily(&self, font: i32) -> &str {
        self.family.get(tex_font_index(font)).map_or("", |s| s)
    }
    /// `TEXSERIES(F)`.
    pub fn texseries(&self, font: i32) -> &str {
        self.series.get(tex_font_index(font)).map_or("", |s| s)
    }
    /// `TEXSHAPE(F)`.
    pub fn texshape(&self, font: i32) -> &str {
        self.shape.get(tex_font_index(font)).map_or("", |s| s)
    }
    /// `TEXFONT(F)`.
    pub fn texfont(&self, font: i32) -> &str {
        self.names.get(tex_font_index(font)).map_or("", |s| s)
    }

    /// `TEXFONTSIZE(S)`: the table entry for sizes up to `MAXFONTSIZE`,
    /// the size itself above. Negative or non-finite sizes give the
    /// default entry (C indexes out of bounds).
    pub fn texfontsize(&self, s: f64) -> f64 {
        if s <= f64::from(MAXFONTSIZE) {
            let i = usize::try_from(round(s)).unwrap_or(0);
            f64::from(self.sizes.get(i).copied().unwrap_or(self.sizes[0]))
        } else if s.is_finite() {
            s
        } else {
            f64::from(self.sizes[0])
        }
    }

    /// `TEXFONTMAG(T)`: the TeX size of a text, scaled by `fontmag` unless
    /// the text is rigid.
    pub fn texfontmag(&self, t: &Text, fontmag: f64) -> f64 {
        self.texfontsize(t.size * if t.rigid_text() { 1.0 } else { fontmag })
    }

    /// `select_font`: writes the commands that select the text's font size
    /// (`\fontsize{…}{…}`) and font (`\usefont…` for PostScript fonts,
    /// `\normalfont…` otherwise). With `only_texfonts`, a PostScript font is
    /// first replaced by the nearest LaTeX font (changing `t`).
    pub fn select_font(
        &self,
        out: &mut Out,
        ctx: &mut Ctx,
        t: &mut Text,
        select_fontsize: bool,
        select_fontname: bool,
        only_texfonts: bool,
    ) {
        // The TeX drivers by default use the correct font size, a bit
        // different from the size on the screen.
        if select_fontsize {
            let texsize = round(t.size * if t.rigid_text() { 1.0 } else { ctx.fontmag });
            let baselineskip = f64::from(texsize) * 1.2;
            let bprec = if baselineskip == baselineskip.floor() { 0 } else { 1 };
            fprintf!(out, "\\fontsize{{{}}}{{{:.*}}}", texsize, bprec, baselineskip);
            if !select_fontname {
                out.puts("\\selectfont ");
                return;
            }
        } else if !select_fontname {
            return;
        }

        if only_texfonts {
            unpsfont_clear(ctx, t);
        }

        if t.psfont_text() {
            let i = if t.font <= MAX_PSFONT {
                usize::try_from(t.font + 1).unwrap_or(0)
            } else {
                0
            };
            let f = self.psfonts.get(i).map_or("", |s| s);
            fprintf!(out, "\\usefont{}", f);
        } else {
            // The default PostScript font is −1, the default TeX font 0;
            // accept −1 too.
            let f = self.fonts.get(tex_font_index(t.font)).map_or("", |s| s);
            fprintf!(out, "\\normalfont{} ", f);
        }
    }
}

/// Whether `font` is Symbol or Zapf Dingbats, whose texts are written as
/// Latin-1 character codes.
fn is_symbol_font(font: i32) -> bool {
    font == MAX_PSFONT || font == MAX_PSFONT - 2
}

/// `put_string`: writes a text, given as the bytes C holds (`Text::raw`).
/// In Symbol and Zapf Dingbats a conversion that is needed (an input
/// encoding other than the locale's) first turns it "from UTF-8 to
/// Latin-1" (`convertutf8tolatin1`, which on Latin-1 input drops the
/// non-ASCII bytes); then every byte beyond ASCII is `\charNNN`: in a UTF-8
/// file, one per UTF-8 byte. Other fonts get the bytes as they are, or
/// converted to UTF-8 where the input encoding differs from the locale's.
/// Unless `tex_text` (a special text, which is TeX code), LaTeX's special
/// characters are quoted.
pub fn put_string(out: &mut Out, ctx: &mut Ctx, raw: &[u8], font: i32, tex_text: bool) {
    if is_symbol_font(font) {
        let bytes: Cow<[u8]> = match conv_textisutf8(ctx, raw) {
            Some(at) => {
                let mut v = raw.get(..at).unwrap_or_default().to_vec();
                v.extend(convertutf8tolatin1(raw.get(at..).unwrap_or_default()));
                Cow::Owned(v)
            }
            None => Cow::Borrowed(raw),
        };
        if tex_text {
            put_characters(out, &bytes);
        } else {
            put_quoted_characters(out, &bytes);
        }
    } else {
        let bytes = conv_textstring(ctx, raw);
        if tex_text {
            out.bytes(&bytes);
        } else {
            put_quoted_bytes(out, &bytes);
        }
    }
}

/// Writes the bytes with the characters ` #$%&<>\\^_{|}~` quoted for LaTeX
/// (`put_quoted_string`); the quoted characters are ASCII, other bytes are
/// written as they are.
fn put_quoted_bytes(out: &mut Out, string: &[u8]) {
    for &b in string {
        let quoted = match b {
            b'<' => "\\textless ",
            b'>' => "\\textgreater ",
            b'|' => "\\textbar ",
            b'\\' => "\\textbackslash ",
            // With \usepackage[T1]{fontenc}, \^{} inserts ascii 2, while
            // \textasciicircum inserts the correct ascii 94.
            b'^' => "\\textasciicircum ",
            // With T1, \~{} inserts ascii 3, \textasciitilde ascii 126.
            b'~' => "\\textasciitilde ",
            b' ' => "\\ ",
            b'#' => "\\#",
            b'$' => "\\$",
            b'%' => "\\%",
            b'&' => "\\&",
            b'_' => "\\_",
            b'{' => "\\{",
            b'}' => "\\}",
            _ => {
                out.bytes(&[b]);
                continue;
            }
        };
        out.puts(quoted);
    }
}

/// Writes `\charNNN` for byte `b`, with a space if a digit follows.
fn put_char_code(out: &mut Out, b: u8, next: Option<&u8>) {
    fprintf!(out, "\\char{}", b);
    if next.is_some_and(u8::is_ascii_digit) {
        out.puts(" ");
    }
}

/// Writes Latin-1 bytes with the characters ` #$%&\^_`{}~` and those above
/// ASCII as `\charNNN` (`put_quoted_characters`).
fn put_quoted_characters(out: &mut Out, s: &[u8]) {
    for (i, &b) in s.iter().enumerate() {
        if b" #$%&\\^_`{}~".contains(&b) || b > 127 {
            put_char_code(out, b, s.get(i + 1));
        } else {
            out.bytes(&[b]);
        }
    }
}

/// Writes Latin-1 bytes with those above ASCII as `\charNNN`
/// (`put_characters`).
fn put_characters(out: &mut Out, s: &[u8]) {
    for (i, &b) in s.iter().enumerate() {
        if b > 127 {
            put_char_code(out, b, s.get(i + 1));
        } else {
            out.bytes(&[b]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::{PSFONT_TEXT, RIGID_TEXT};

    fn put(s: &str, font: i32, tex: bool) -> String {
        let mut out = Out::new();
        put_string(&mut out, &mut Ctx::default(), s.as_bytes(), font, tex);
        String::from_utf8(out.buf).unwrap()
    }

    #[test]
    fn tables() {
        assert_eq!(TEXFONTSIZES.len(), 44);
        assert_eq!(TEXPSFONTS.len(), 36);
        let tf = TexFonts::new();
        assert_eq!(tf.texfontsize(12.0), 12.0);
        assert_eq!(tf.texfontsize(13.0), 12.0);
        assert_eq!(tf.texfontsize(14.4), 14.0);
        assert_eq!(tf.texfontsize(42.0), 41.0);
        assert_eq!(tf.texfontsize(50.0), 50.0);
        assert_eq!(tf.texfontsize(-3.0), 11.0);
        assert_eq!(tf.texfontsize(f64::NAN), 11.0);
        assert_eq!(tf.texfont(-1), "rm");
        assert_eq!(tf.texfont(3), "it");
        assert_eq!(tf.texfont(9), "sf");
        assert_eq!(tf.texseries(2), "\\bfdefault");
        assert_eq!(tf.texshape(3), "\\itdefault");
        assert_eq!(tf.texfamily(5), "\\ttdefault");
    }

    #[test]
    fn quoting_matches_fig2dev() {
        // Output of fig2dev 3.2.9a -L pict2e for the same strings.
        assert_eq!(
            put("Grüße #$%&_{}~^\\<>| x", 0, false),
            "Grüße\\ \\#\\$\\%\\&\\_\\{\\}\\textasciitilde \\textasciicircum \\textbackslash \\textless \\textgreater \\textbar \\ x"
        );
        assert_eq!(put("Grüße #$%&_{}~^\\<>| x", 0, true), "Grüße #$%&_{}~^\\<>| x");
        assert_eq!(put("a#9", 32, false), "a\\char35 9");
        assert_eq!(put("a#9", 32, true), "a#9");
    }

    #[test]
    fn symbol_font_latin1() {
        // In a UTF-8 file fig2dev writes one \\charNNN per UTF-8 byte.
        assert_eq!(put("a±b", 32, false), "a\\char194\\char177b");
        assert_eq!(put("±1", 34, true), "\\char194\\char177 1");
    }

    #[test]
    fn select_font_output() {
        let tf = TexFonts::new();
        let mut ctx = Ctx::default();
        let mut out = Out::new();
        let mut t = Text {
            size: 12.0,
            font: 0,
            flags: PSFONT_TEXT,
            ..Text::default()
        };
        tf.select_font(&mut out, &mut ctx, &mut t, true, true, false);
        let mut t2 = Text {
            size: 11.0,
            font: 2,
            flags: RIGID_TEXT,
            ..Text::default()
        };
        tf.select_font(&mut out, &mut ctx, &mut t2, true, true, false);
        let mut t3 = Text {
            size: 10.0,
            font: 32,
            flags: PSFONT_TEXT,
            ..Text::default()
        };
        tf.select_font(&mut out, &mut ctx, &mut t3, true, false, false);
        tf.select_font(&mut out, &mut ctx, &mut t3, false, true, true);
        tf.select_font(&mut out, &mut ctx, &mut t3, false, false, true);
        assert_eq!(
            String::from_utf8(out.buf).unwrap(),
            "\\fontsize{12}{14.4}\\usefont{T1}{ptm}{m}{n}\
             \\fontsize{11}{13.2}\\normalfont\\bfseries \
             \\fontsize{10}{12}\\selectfont \
             \\normalfont\\rmfamily "
        );
        assert!(!t3.psfont_text());
        assert_eq!(ctx.messages.len(), 1);
    }
}
