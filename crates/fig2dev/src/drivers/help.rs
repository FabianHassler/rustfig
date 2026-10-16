//! The help text of the drivers in this crate (`fig2dev -L lang -h`):
//! for each language, the options its driver here really acts on, with
//! fig2dev's argument names and help lines.
//
// Ported from fig2dev fig2dev.c (`help_msg`) and the drivers' sources
// (xfig licence, see LICENSE-xfig).

/// One option of a driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionHelp {
    /// fig2dev's option letter.
    pub letter: char,
    /// The argument's name, empty for a flag.
    pub arg: &'static str,
    /// The help line; `\n` starts a continuation line.
    pub text: &'static str,
}

/// A driver's help section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverHelp {
    /// The section title, as fig2dev's help names the driver.
    pub title: &'static str,
    /// Whether the driver draws the grid of `-G`.
    pub grid: bool,
    pub options: &'static [OptionHelp],
}

const fn o(letter: char, arg: &'static str, text: &'static str) -> OptionHelp {
    OptionHelp { letter, arg, text }
}

const fn d(title: &'static str, options: &'static [OptionHelp]) -> DriverHelp {
    DriverHelp {
        title,
        grid: false,
        options,
    }
}

// -a (binary CGM through ralcgm) is accepted, but RustFig runs no external
// programs and always writes ASCII CGM.
const CGM: DriverHelp = d(
    "CGM",
    &[o(
        'r',
        "",
        "position arrowheads for CGM viewers that use rounded arrowheads",
    )],
);

// The pattern (-l) and pen (-p) tables and -P are read but not used by
// the DXF driver.
const DXF: DriverHelp = d(
    "DXF",
    &[o('f', "fontfile", "load text character specs from table in file")],
);

const EPIC: DriverHelp = d(
    "EPIC, EEPIC and EEPICEMU",
    &[
        o('d', "scale", "divide size of arrowheads by scale"),
        o('F', "", "do not set font family, series or shape"),
        o('f', "font", "set default font"),
        o('l', "lwidth", "use \"thicklines\" when width of line is > lwidth"),
        o('P', "", "generate a complete LaTeX file"),
        o('R', "dummyarg", "allow rotated text (uses \\rotatebox)"),
        o('S', "scale", "scale figure to scale/12, scale must be between 8 and 12"),
        o('t', "stretch", "set the stretch of dashed lines (default = 30)"),
        o('v', "", "include comments in the output"),
        o('W', "", "enable variable line width"),
        o('w', "", "disable variable line width"),
    ],
);

const GBX: DriverHelp = d(
    "GBX (Gerber, RS-247-X)",
    &[
        o('d', "[mm|in]", "output dimensions assumed to be millimeters or inches"),
        o(
            'p',
            "[pos|neg]",
            "image Polarity - positive results in lines being drawn,\nnegative results in lines erasing background",
        ),
        o(
            'g',
            "<x scale>x<y scale>+<x offset>+<y offset>",
            "scale and shift output.\nTypically, use -g 1x-1+0+0, mirroring about the x axis.",
        ),
        o(
            'f',
            "<n digits>.<n digits>",
            "digits of precision before and\nafter the implied decimal point.",
        ),
        o('v', "", "include comments in the output"),
    ],
);

const GE: DriverHelp = d("GE", &[]);

const IBMGL: DriverHelp = d(
    "IBM-GL",
    &[
        o('a', "", "select ANSI A paper size instead of the default ISO A4"),
        o('c', "", "generate instructions for IBM 6180 plotter without IBM-GEC"),
        o('d', "xll,yll,xur,yur", "restrict plotting to area specified by coords"),
        o('F', "", "use correct font sizes (points instead of 1/80inch)"),
        o('f', "fontfile", "load text character specs from table in file"),
        o('k', "", "precede output with PCL command to use HP/GL"),
        o('l', "pattfile", "load patterns for pattern fill from file"),
        o('P', "", "rotate figure to portrait (default is landscape)"),
        o('p', "penfile", "load plotter pen specs from file"),
        o('S', "speed", "set pen speed in cm/sec"),
        o(
            'v',
            "",
            "print figure upside-down in portrait or backwards in landscape",
        ),
        o('x', "offset", "shift figure left by offset inches"),
        o('y', "offset", "shift figure up by offset inches"),
        o(
            'z',
            "papersize",
            "set the papersize (see man pages for available sizes)",
        ),
    ],
);

const LATEX: DriverHelp = d(
    "LaTeX",
    &[
        o('b', "width", "specify width of blank border around figure (1/72 inch)"),
        o(
            'd',
            "dmag",
            "set separate magnification for length of line dashes to dmag",
        ),
        o('F', "", "do not set font family/series/shape"),
        o('f', "font", "set default font"),
        o('l', "lwidth", "set threshold between thin and thick lines to lwidth"),
        o('v', "", "verbose mode"),
    ],
);

// pstex_t and pdftex_t set only the special texts (genpstex.c), so the
// latex driver's line options do nothing there.
const PSTEX_T: DriverHelp = d(
    "PSTEX_T and PDFTEX_T",
    &[
        o('b', "width", "specify width of blank border around figure (1/72 inch)"),
        o('F', "", "do not set font family/series/shape"),
        o('f', "font", "set default font"),
        o('p', "file", "specify name of the graphics file to include"),
    ],
);

const MAP: DriverHelp = d(
    "MAP (HTML image map)",
    &[o(
        'b',
        "width",
        "specify width of blank border around figure (points, 1/72 inch)",
    )],
);

const MF: DriverHelp = d(
    "METAFONT",
    &[
        o('C', "code", "specifies the starting METAFONT font code"),
        o('n', "name", "name to use in the output file"),
        o(
            'p',
            "pen_mag",
            "linewidth magnification compared to the original figure",
        ),
        o('t', "top", "specifies the top of the coordinate system"),
        o('x', "xmin", "specifies minimum x coordinate of figure (inches)"),
        o('y', "ymin", "specifies minimum y coordinate of figure (inches)"),
        o('X', "xmax", "specifies maximum x coordinate of figure (inches)"),
        o('Y', "ymax", "specifies maximum y coordinate of figure (inches)"),
    ],
);

const MP: DriverHelp = d(
    "METAPOST",
    &[
        o('d', "file", "add file content as additional header"),
        o('i', "file", "include file content via \\input-command"),
        o('M', "", "multipage, make one figure for each depth"),
        o('o', "", "old mode (no latex)"),
        o('p', "number", "add the line \"prologues:=number\""),
    ],
);

const PIC: DriverHelp = d(
    "PIC",
    &[
        o('f', "font", "set default font"),
        o('p', "ext", "enables certain PIC extensions (see man pages)"),
    ],
);

const PICTEX: DriverHelp = d(
    "PICTeX",
    &[
        o('f', "font", "set default font"),
        o('l', "dimen", "set line thickness to TeX dimen"),
        o('p', "symbol", "set plot symbol (default \\tencirc\\symbol{'160})"),
        o('r', "", "do not use dvips"),
    ],
);

const SHAPE: DriverHelp = d(
    "SHAPE (ShapePar driver)",
    &[o(
        'n',
        "name",
        "Set basename of the macro\n(e.g. \"face\" gives faceshape and facepar)",
    )],
);

const TEXTYL: DriverHelp = d(
    "TEXTYL",
    &[
        o('f', "font", "set default font"),
        o(
            'l',
            "width",
            "set line width, width must be between 1 and 12 inclusively",
        ),
    ],
);

const TPIC: DriverHelp = d("TPIC", &[o('f', "font", "set default font")]);

/// The help of language `lang`'s driver in this crate, or `None` when
/// this crate doesn't write `lang` (yet). Lists only the options the
/// driver acts on; options it accepts and ignores are left out.
pub fn help(lang: &str) -> Option<&'static DriverHelp> {
    Some(match lang {
        "cgm" => &CGM,
        "dxf" => &DXF,
        "epic" | "eepic" | "eepicemu" => &EPIC,
        "gbx" => &GBX,
        "ge" => &GE,
        "ibmgl" => &IBMGL,
        "latex" => &LATEX,
        "map" => &MAP,
        "mf" => &MF,
        "mp" | "mmp" => &MP,
        "pic" => &PIC,
        "pictex" => &PICTEX,
        "pstex_t" | "pdftex_t" => &PSTEX_T,
        "shape" => &SHAPE,
        "textyl" => &TEXTYL,
        "tpic" => &TPIC,
        _ => return None,
    })
}

/// One option as fig2dev's help prints it: `  -x arg      text`, the
/// continuation lines indented.
pub fn format_option(letter: char, arg: &str, text: &str) -> String {
    let head = if arg.is_empty() {
        format!("-{letter}")
    } else {
        format!("-{letter} {arg}")
    };
    let sep = if head.len() < 12 {
        " ".repeat(12 - head.len())
    } else {
        "    ".to_string()
    };
    format!("  {head}{sep}{}", text.replace('\n', "\n                "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::driver;
    use crate::Ctx;

    const LANGS: &[&str] = &[
        "box", "cgm", "dxf", "eepic", "eepicemu", "emf", "epic", "eps", "gbx", "ge", "gif", "ibmgl", "jpeg", "latex",
        "map", "mf", "mmp", "mp", "pcx", "pdf", "pdftex", "pdftex_t", "pic", "pict2e", "pictex", "png", "ppm", "ps",
        "pstex", "pstex_t", "pstricks", "ptk", "shape", "sld", "svg", "textyl", "tiff", "tikz", "tk", "tpic", "xbm",
        "xpm",
    ];

    #[test]
    fn every_ported_driver_has_help() {
        for l in LANGS {
            assert_eq!(help(l).is_some(), driver(l).is_some(), "{l}");
        }
    }

    #[test]
    fn the_drivers_take_the_options_they_list() {
        for l in LANGS {
            let (Some(h), Some(mut dev)) = (help(l), driver(l)) else {
                continue;
            };
            for opt in h.options {
                let arg = match (*l, opt.letter) {
                    ("pic", 'p') => "all",
                    (_, 'S') if l.contains("epic") => "10",
                    ("textyl", 'l') => "2",
                    ("gbx", 'd') => "mm",
                    ("gbx", 'p') => "pos",
                    ("gbx", 'g') => "1x1+0+0",
                    ("gbx", 'f') => "2.4",
                    (_, 'f' | 'l' | 'p') if *l == "ibmgl" || *l == "dxf" => "/nonexistent",
                    _ => "1",
                };
                let arg = (!opt.arg.is_empty()).then_some(arg);
                let mut ctx = Ctx::default();
                assert!(dev.option(&mut ctx, opt.letter, arg).is_ok(), "{l} -{}", opt.letter);
            }
            let mut letters: Vec<char> = h.options.iter().map(|o| o.letter).collect();
            letters.sort_unstable();
            letters.dedup();
            assert_eq!(letters.len(), h.options.len(), "{l}: duplicate letters");
        }
    }

    #[test]
    fn options_look_like_fig2devs() {
        assert_eq!(
            format_option('d', "file", "add file content"),
            "  -d file     add file content"
        );
        assert_eq!(format_option('M', "", "multipage"), "  -M          multipage");
        assert_eq!(
            format_option('g', "<x scale>x<y scale>", "a\nb"),
            "  -g <x scale>x<y scale>    a\n                b"
        );
    }
}
