//! fig2dev's command-line options, as export options.
//!
//! Every option fig2dev accepts for a language RustFig writes is listed
//! here with its key (the name `file.export` takes in `options`), its
//! fig2dev letter, and whether RustFig honours it yet. Unknown keys and
//! options not ported yet are errors, so nothing is silently ignored.
//
// Ported from fig2dev fig2dev.c (`get_args`, `help_msg`, `depth_option`,
// `depth_filter`), dev/genps.c (`gen_ps_eps_option`), dev/genpdf.c,
// dev/genbitmaps.c and dev/gensvg.c (`*_option`) (xfig licence, see
// LICENSE-xfig).

use crate::languages::Language;
use figbase::color::{x_color, Rgb};

/// The kind of value an option takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arg {
    /// true or false.
    Flag,
    /// A number.
    Number,
    /// Free text.
    Text,
    /// An X11 colour name or `#rrggbb`.
    Color,
    /// fig2dev's depth list: `+list` keeps, `-list` drops (`+10,40:50`).
    Depths,
    /// fig2dev's `-Z`: a number, `Nw`/`N:` for the width, `Nh`/`:N` for the height.
    Size,
    /// A standard PostScript font name.
    Font,
}

/// Whether RustFig honours an option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    Ported,
    /// Not ported yet; the plan task that ports it.
    Planned(&'static str),
}

/// One fig2dev option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionSpec {
    /// The key in `file.export`'s `options`.
    pub key: &'static str,
    /// fig2dev's option letter.
    pub letter: char,
    pub arg: Arg,
    /// The languages accepting it (fig2dev's driver takes the letter);
    /// empty means every language.
    pub langs: &'static [&'static str],
    /// The languages where it has an effect; empty means all of `langs`.
    /// Elsewhere fig2dev accepts and ignores it (`genps.c` ignores the page
    /// options for EPS and the bounding box for PostScript, for example).
    pub effect: &'static [&'static str],
    /// fig2dev's help text.
    pub help: &'static str,
    pub support: Support,
}

impl OptionSpec {
    /// Whether `lang` takes this option.
    pub fn applies_to(&self, lang: &str) -> bool {
        self.langs.is_empty() || self.langs.contains(&lang)
    }

    /// Whether it does something for `lang` (it is accepted and ignored
    /// where it doesn't).
    pub fn effective_for(&self, lang: &str) -> bool {
        self.applies_to(lang) && (self.effect.is_empty() || self.effect.contains(&lang))
    }
}

const fn opt(
    key: &'static str,
    letter: char,
    arg: Arg,
    langs: &'static [&'static str],
    help: &'static str,
    support: Support,
) -> OptionSpec {
    OptionSpec {
        key,
        letter,
        arg,
        langs,
        effect: &[],
        help,
        support,
    }
}

/// An option accepted by `langs` but effective only for `effect`.
const fn opt_for(
    key: &'static str,
    letter: char,
    arg: Arg,
    langs: &'static [&'static str],
    effect: &'static [&'static str],
    help: &'static str,
    support: Support,
) -> OptionSpec {
    OptionSpec {
        key,
        letter,
        arg,
        langs,
        effect,
        help,
        support,
    }
}

use Arg::{Color, Depths, Flag, Font, Number, Size, Text};
use Support::{Planned, Ported};

const ALL: &[&str] = &[];
/// The PostScript driver's languages.
const PS: &[&str] = &["ps", "eps", "pdf", "pspdf"];
/// Options of genps that also reach the bitmaps (genbitmaps runs genps).
const PS_AND_BITMAPS: &[&str] = &[
    "ps", "eps", "pdf", "pspdf", "gif", "jpeg", "pcx", "png", "ppm", "tiff", "xbm", "xpm",
];
/// Options only for whole pages (PostScript, or PDF in page mode).
const PAGE: &[&str] = &["ps", "pdf"];
/// EPS options (a PDF is converted from EPS unless in page mode).
const EPS: &[&str] = &["eps", "pdf", "pspdf"];
/// Previews: PostScript and EPS, not PDF.
const PREVIEW: &[&str] = &["ps", "eps", "pspdf"];
const BITMAPS: &[&str] = &["gif", "jpeg", "pcx", "png", "ppm", "tiff", "xbm", "xpm"];

/// fig2dev's options, as its sources accept them (`get_args` in
/// fig2dev.c, `gen_ps_eps_option` in genps.c, `genpdf_option`,
/// `genbitmaps_option`, `gensvg_option`, `genbox_option`), for the
/// languages RustFig writes or is porting next.
#[rustfmt::skip]
pub const OPTIONS: &[OptionSpec] = &[
    // General options (fig2dev.c); -G also reaches every driver.
    opt("depths", 'D', Depths, ALL, "include or exclude depths listed (+list or -list)", Ported),
    opt("adjust_bbox", 'K', Flag, ALL, "adjust bounding box according to the depths selected with depths (-D)", Ported),
    opt("magnification", 'm', Number, ALL, "set magnification (1 = 100 %); not with max_size (-Z)", Ported),
    opt("font_size", 's', Number, ALL, "set default font size in points", Ported),
    opt("max_size", 'Z', Size, ALL, "scale the figure so that its largest dimension is maxdim inches/cm; append w or : for the width, h or prepend : for the height", Ported),
    opt_for("grid", 'G', Text, ALL, PS_AND_BITMAPS, "draw light gray grid with thin/thick lines at minor/major units (e.g. .25:1cm)", Planned("B4")),
    // PostScript, EPS and PDF (gen_ps_eps_option, genpdf_option); the
    // bitmaps share some (genbitmaps_option).
    opt("border", 'b', Number, PS, "specify width of blank border around figure (points, 1/72 inch)", Ported),
    // genbitmaps_option: the bitmaps' border is in pixels (1/80 inch times
    // the magnification).
    opt("border", 'b', Number, BITMAPS, "specify width of blank border around figure (pixels)", Ported),
    opt("background", 'g', Color, PS_AND_BITMAPS, "set background color", Ported),
    opt("grayscale", 'N', Flag, PS_AND_BITMAPS, "convert all colors to grayscale", Ported),
    opt("correct_font_size", 'F', Flag, PS_AND_BITMAPS, "use correct font sizes (points instead of 1/80 inch); xfig sets it, so it is on unless false", Ported),
    opt("title", 'n', Text, PS, "set title part of PostScript output to name", Ported),
    opt("font", 'f', Font, PS, "set default font", Ported),
    opt_for("ascii_preview", 'A', Flag, PS, PREVIEW, "add ASCII preview (produce EPSI file; not for PDF output)", Planned("B4")),
    opt_for("color_tiff_preview", 'C', Text, PS, PREVIEW, "add color TIFF preview (for Microsoft apps; not for PDF output)", Planned("B4")),
    opt_for("tiff_preview", 'T', Flag, PS, PREVIEW, "add monochrome TIFF preview (for Microsoft apps; not for PDF)", Planned("B4")),
    opt("composite_fonts", 'j', Flag, PS, "enable composite fonts for japanese or korean text", Planned("B8")),
    opt_for("bbox", 'B', Text, PS, EPS, "force width, height and origin in absolute coordinates (\"Wx [Wy X0 Y0]\")", Planned("B4")),
    opt_for("relative_bbox", 'R', Text, PS, EPS, "force width, height and origin in relative coordinates (\"Wx [Wy X0 Y0]\")", Planned("B4")),
    opt_for("center", 'c', Flag, PS, PAGE, "center figure on page", Planned("B4")),
    opt_for("edge", 'e', Flag, PS, PAGE, "put figure at left page edge", Planned("B4")),
    opt_for("landscape", 'l', Text, PS, PAGE, "force landscape mode (takes an argument it ignores)", Planned("B4")),
    opt_for("portrait", 'p', Text, PS, PAGE, "force portrait mode (takes an argument it ignores)", Planned("B4")),
    opt_for("multi_page", 'M', Flag, PS, PAGE, "generate multiple pages for large figure", Planned("B4")),
    opt_for("single_page", 'o', Flag, PS, PAGE, "generate single page even for large figure", Planned("B4")),
    opt_for("overlap", 'O', Flag, PS, PAGE, "overlap pages when multiple pages are generated", Planned("B4")),
    opt_for("x_offset", 'x', Number, PS, PAGE, "shift figure horizontally by offset points", Planned("B4")),
    opt_for("y_offset", 'y', Number, PS, PAGE, "shift figure vertically by offset points", Planned("B4")),
    opt_for("paper_size", 'z', Text, PS, PAGE, "set the paper size (Letter, A4, ...)", Planned("B4")),
    opt_for("page_mode", 'P', Flag, PS, &["pdf"], "generate a full page instead of a figure-sized one", Planned("B4")),
    opt_for("pdf_version", 'Y', Number, PS, &["pdf", "pspdf"], "set the PDF minor version (1-7)", Planned("B4")),
    // SVG (gensvg_option).
    opt("paper_size", 'z', Text, &["svg"], "set the paper size (Letter, A4, ...)", Planned("B9")),
    // Bitmaps (genbitmaps_option): -q and -t are accepted by every bitmap
    // language, with a warning outside JPEG and GIF.
    opt("smoothing", 'S', Number, BITMAPS, "specify smoothing factor (1 = none, 2 = some, 4 = more)", Ported),
    opt_for("quality", 'q', Number, BITMAPS, &["jpeg"], "specify image quality factor (1-100)", Ported),
    opt_for("transparent", 't', Color, BITMAPS, &["gif"], "specify GIF transparent color in hexadecimal (e.g. #ff0000=red)", Ported),
];

/// The options `lang` takes, in fig2dev's order.
pub fn options_for(lang: &str) -> impl Iterator<Item = &'static OptionSpec> + '_ {
    OPTIONS.iter().filter(move |o| o.applies_to(lang))
}

/// The option `key` (or its fig2dev letter, like `"b"` or `"-b"`) of `lang`.
pub fn option_spec(lang: &str, key: &str) -> Option<&'static OptionSpec> {
    let letter = {
        let k = key.strip_prefix('-').unwrap_or(key);
        let mut cs = k.chars();
        match (cs.next(), cs.next()) {
            (Some(c), None) => Some(c),
            _ => None,
        }
    };
    options_for(lang).find(|o| o.key == key || Some(o.letter) == letter)
}

/// A value given for an option.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptionValue<'a> {
    Bool(bool),
    Number(f64),
    Text(&'a str),
}

/// fig2dev's `-D`: which depths an export keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepthFilter {
    /// `+list` keeps only the listed depths; `-list` drops them.
    pub keep_listed: bool,
    /// Inclusive ranges.
    pub ranges: Vec<(i32, i32)>,
}

/// Most ranges a depth list may hold (fig2dev's `NUMDEPTHS`).
const MAX_DEPTH_RANGES: usize = 100;

impl DepthFilter {
    /// Parse `+list` or `-list`, a list of depths and `a:b` ranges
    /// separated by commas.
    pub fn parse(s: &str) -> Result<DepthFilter, String> {
        let usage = || format!("depths must be +list or -list, like +10,40:50 (got {s:?})");
        let s = s.trim();
        let (keep_listed, list) = match s.split_at_checked(1) {
            Some(("+", rest)) => (true, rest),
            Some(("-", rest)) => (false, rest),
            _ => return Err(usage()),
        };
        let mut ranges = Vec::new();
        for item in list.split(',').map(str::trim).filter(|i| !i.is_empty()) {
            if ranges.len() >= MAX_DEPTH_RANGES {
                return Err(format!("too many depths (at most {MAX_DEPTH_RANGES})"));
            }
            let num = |t: &str| t.trim().parse::<i32>().ok().filter(|d| *d >= 0);
            let range = match item.split_once(':') {
                Some((a, b)) => (num(a).ok_or_else(usage)?, num(b).ok_or_else(usage)?),
                None => {
                    let d = num(item).ok_or_else(usage)?;
                    (d, d)
                }
            };
            if range.1 < range.0 {
                return Err(usage());
            }
            ranges.push(range);
        }
        if ranges.is_empty() {
            return Err(usage());
        }
        Ok(DepthFilter { keep_listed, ranges })
    }

    /// Whether objects at `depth` are exported.
    pub fn keeps(&self, depth: i32) -> bool {
        let listed = self.ranges.iter().any(|&(a, b)| (a..=b).contains(&depth));
        listed == self.keep_listed
    }
}

/// Which dimension fig2dev's `-Z` sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeDim {
    Largest,
    Width,
    Height,
}

/// fig2dev's `-Z`: the size (inches, or cm in metric drawings) of one dimension.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaxSize {
    pub size: f64,
    pub dim: SizeDim,
}

impl MaxSize {
    pub fn parse(s: &str) -> Result<MaxSize, String> {
        let t = s.trim();
        let (num, dim) = if let Some(n) = t.strip_prefix(':') {
            (n, SizeDim::Height)
        } else if let Some(n) = t.strip_suffix('h') {
            (n, SizeDim::Height)
        } else if let Some(n) = t.strip_suffix('w').or_else(|| t.strip_suffix(':')) {
            (n, SizeDim::Width)
        } else {
            (t, SizeDim::Largest)
        };
        let size = num
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v > 0.0)
            .ok_or_else(|| format!("max_size must be a positive size, like 4, 4w or 4h (got {s:?})"))?;
        Ok(MaxSize { size, dim })
    }
}

/// The fig2dev options of one export.
#[derive(Debug, Clone, PartialEq)]
pub struct DriverOptions {
    pub depths: Option<DepthFilter>,
    /// With `depths`: size the figure by the depths kept (`-K`).
    pub adjust_bbox: bool,
    /// fig2dev's `-m` (1 = 100 %).
    pub magnification: Option<f64>,
    pub max_size: Option<MaxSize>,
    /// Every text set to this size in points (`-s`).
    pub font_size: Option<f64>,
    /// Blank border around the figure (`-b`): points for PostScript, PDF
    /// and SVG, pixels (1/80 inch times the magnification) for bitmaps.
    pub border: f64,
    pub background: Option<Rgb>,
    pub grayscale: bool,
    /// `%%Title` of PostScript, `/Title` of PDF.
    pub title: Option<String>,
    /// Font sizes in points (`-F`, which xfig always passes); false reads
    /// them as 1/80 inch, as fig2dev does without `-F`.
    pub correct_font_size: bool,
    /// The PostScript font replacing the default and Times-Roman (`-f`).
    pub font: Option<i32>,
    /// fig2dev's `-S` (0 or 1 none, 2 some, 4 more); `None` antialiases
    /// as RustFig draws on screen.
    pub smoothing: Option<u32>,
    /// JPEG quality, 1 to 100 (`-q`; fig2dev's default is 75).
    pub quality: Option<u8>,
    /// The GIF transparent colour (`-t`); without it, the figure's own.
    pub transparent: Option<Rgb>,
}

impl Default for DriverOptions {
    fn default() -> Self {
        DriverOptions {
            depths: None,
            adjust_bbox: false,
            magnification: None,
            max_size: None,
            font_size: None,
            border: 0.0,
            background: None,
            grayscale: false,
            title: None,
            correct_font_size: true,
            font: None,
            smoothing: None,
            quality: None,
            transparent: None,
        }
    }
}

/// Largest border, magnification and font size accepted.
const MAX_BORDER_PT: f64 = 72.0 * 1000.0;
const MAX_MAG: f64 = 1000.0;
const MAX_FONT_SIZE: f64 = 1000.0;

/// Read the options given for `lang`. Unknown options, options `lang`
/// does not take and options not ported yet are errors.
pub fn parse_options(lang: &Language, items: &[(&str, OptionValue<'_>)]) -> Result<DriverOptions, String> {
    let mut o = DriverOptions::default();
    for &(key, value) in items {
        let spec = option_spec(lang.id, key).ok_or_else(|| {
            let known: Vec<&str> = options_for(lang.id).map(|o| o.key).collect();
            format!(
                "{} export has no option {key:?}; its options are: {}",
                lang.description,
                known.join(", ")
            )
        })?;
        if !spec.effective_for(lang.id) {
            // fig2dev accepts it for this language and does nothing.
            continue;
        }
        if let Support::Planned(task) = spec.support {
            return Err(format!(
                "option {} (fig2dev -{}) is not available yet: it is being ported (plan task {task})",
                spec.key, spec.letter
            ));
        }
        let key = spec.key;
        let flag = || match value {
            OptionValue::Bool(b) => Ok(b),
            _ => Err(format!("option {key} takes true or false")),
        };
        let number = |lo: f64, hi: f64| match value {
            OptionValue::Number(v) if v.is_finite() && (lo..=hi).contains(&v) => Ok(v),
            OptionValue::Text(t) => t
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite() && (lo..=hi).contains(v))
                .ok_or_else(|| format!("option {key} takes a number from {lo} to {hi}")),
            _ => Err(format!("option {key} takes a number from {lo} to {hi}")),
        };
        let text = || match value {
            OptionValue::Text(t) => Ok(t),
            _ => Err(format!("option {key} takes text")),
        };
        match key {
            "depths" => o.depths = Some(DepthFilter::parse(text()?)?),
            "adjust_bbox" => o.adjust_bbox = flag()?,
            "magnification" => o.magnification = Some(number(0.001, MAX_MAG)?),
            "font_size" => o.font_size = Some(number(0.1, MAX_FONT_SIZE)?),
            "max_size" => {
                o.max_size = Some(match value {
                    OptionValue::Number(v) => MaxSize::parse(&v.to_string())?,
                    OptionValue::Text(t) => MaxSize::parse(t)?,
                    OptionValue::Bool(_) => return Err("option max_size takes a size, like 4, 4w or 4h".into()),
                })
            }
            "border" => o.border = number(0.0, MAX_BORDER_PT)?,
            "background" => {
                let t = text()?;
                o.background = Some(x_color(t).ok_or_else(|| format!("can't parse color {t:?}"))?);
            }
            "grayscale" => o.grayscale = flag()?,
            "correct_font_size" => o.correct_font_size = flag()?,
            "title" => {
                let t = text()?;
                if t.chars().any(char::is_control) {
                    return Err("option title must be one line".into());
                }
                o.title = Some(t.to_string());
            }
            "font" => {
                let t = text()?;
                o.font = Some(figbase::text::ps_font_number(t.trim()).ok_or_else(|| {
                    format!("{t:?} is not a standard PostScript font name (like Helvetica or Times-Bold)")
                })?);
            }
            "smoothing" => {
                let v = number(0.0, 4.0)?;
                if ![0.0, 1.0, 2.0, 4.0].contains(&v) {
                    return Err(format!("bad value for smoothing (-S): {v}, should be 1, 2 or 4"));
                }
                o.smoothing = Some(v as u32);
            }
            "quality" => {
                // fig2dev clamps the quality to 1..100.
                let v = number(-1.0e9, 1.0e9)?;
                o.quality = Some(v.round().clamp(1.0, 100.0) as u8);
            }
            "transparent" => {
                let t = text()?;
                o.transparent = Some(x_color(t).ok_or_else(|| format!("can't parse color {t:?}"))?);
            }
            other => return Err(format!("option {other} is not handled (internal error)")),
        }
    }
    if o.magnification.is_some() && o.max_size.is_some() {
        return Err("specify only one of magnification (-m) and max_size (-Z)".into());
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::{language, LANGUAGES};

    fn parse(lang: &str, items: &[(&str, OptionValue<'_>)]) -> Result<DriverOptions, String> {
        parse_options(language(lang).unwrap(), items)
    }

    #[test]
    fn keys_and_letters_are_unique_per_language() {
        for l in LANGUAGES {
            let opts: Vec<_> = options_for(l.id).collect();
            for (i, a) in opts.iter().enumerate() {
                for b in &opts[i + 1..] {
                    assert_ne!(a.key, b.key, "{}", l.id);
                    assert_ne!(a.letter, b.letter, "{} {} {}", l.id, a.key, b.key);
                }
            }
        }
    }

    #[test]
    fn every_option_names_a_known_language() {
        for o in OPTIONS {
            for l in o.langs {
                assert!(language(l).is_some(), "{} {l}", o.key);
            }
        }
    }

    #[test]
    fn depth_lists() {
        let f = DepthFilter::parse("+10,40:50").unwrap();
        assert!(f.keeps(10) && f.keeps(45) && !f.keeps(11) && !f.keeps(51));
        let f = DepthFilter::parse("-0:49").unwrap();
        assert!(!f.keeps(0) && f.keeps(50));
        for bad in ["", "10", "+", "+a", "+5:3", "+-1"] {
            assert!(DepthFilter::parse(bad).is_err(), "{bad}");
        }
        let many = format!("+{}", vec!["1"; 101].join(","));
        assert!(DepthFilter::parse(&many).is_err());
    }

    #[test]
    fn max_sizes() {
        let m = |s: &str| MaxSize::parse(s).unwrap();
        assert_eq!(m("4").dim, SizeDim::Largest);
        assert_eq!(m("4w").dim, SizeDim::Width);
        assert_eq!(m("4:").dim, SizeDim::Width);
        assert_eq!(m("4h").dim, SizeDim::Height);
        assert_eq!(
            m(":2.5"),
            MaxSize {
                size: 2.5,
                dim: SizeDim::Height
            }
        );
        for bad in ["", "w", "0", "-3", "nan", "inf"] {
            assert!(MaxSize::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn options_parse_by_key_or_letter() {
        let o = parse(
            "eps",
            &[
                ("border", OptionValue::Number(10.0)),
                ("-g", OptionValue::Text("light blue")),
                ("N", OptionValue::Bool(true)),
                ("title", OptionValue::Text("Figure 1")),
                ("font", OptionValue::Text("Helvetica")),
                ("Z", OptionValue::Text("3w")),
            ],
        )
        .unwrap();
        assert_eq!(o.border, 10.0);
        assert_eq!(o.background, Some(Rgb::new(173, 216, 230)));
        assert!(o.grayscale);
        assert_eq!(o.title.as_deref(), Some("Figure 1"));
        assert_eq!(o.font, Some(16));
        assert_eq!(o.max_size.map(|m| m.dim), Some(SizeDim::Width));
    }

    #[test]
    fn bad_options_are_errors() {
        let err = |lang: &str, k: &str, v: OptionValue<'_>| parse(lang, &[(k, v)]).unwrap_err();
        assert!(err("eps", "bogus", OptionValue::Bool(true)).contains("no option"));
        assert!(err("svg", "border", OptionValue::Number(1.0)).contains("no option"));
        assert!(err("eps", "bbox", OptionValue::Text("1 1")).contains("B4"));
        assert!(err("eps", "border", OptionValue::Number(-1.0)).contains("number"));
        assert!(err("eps", "border", OptionValue::Number(f64::NAN)).contains("number"));
        assert!(err("eps", "grayscale", OptionValue::Number(1.0)).contains("true or false"));
        assert!(err("eps", "background", OptionValue::Text("nosuch")).contains("color"));
        assert!(err("eps", "font", OptionValue::Text("Comic Sans")).contains("PostScript"));
        assert!(err("eps", "title", OptionValue::Text("a\nb")).contains("one line"));
        let both = parse(
            "pdf",
            &[
                ("magnification", OptionValue::Number(2.0)),
                ("max_size", OptionValue::Number(3.0)),
            ],
        );
        assert!(both.unwrap_err().contains("only one"));
    }
}
