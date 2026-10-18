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

const EMF: DriverHelp = d(
    "EMF",
    &[
        o(
            'l',
            "lang",
            "set compatibility level to win95, win98 or winnt, default winnt",
        ),
        o(
            'r',
            "",
            "position arrowheads for EMF viewers that use rounded arrowheads",
        ),
    ],
);

// -O is accepted, but genpict2e.c doesn't act on it.
const PICT2E: DriverHelp = d(
    "PICT2E",
    &[
        o(
            'b',
            "width",
            "specify width of blank border around figure (points, 1/72 inch)",
        ),
        o(
            'C',
            "num",
            "do not issue color commands for color num (0 = black,\n1 = blue, etc., see xfig color chooser)",
        ),
        o('e', "", "do not try to be compatible with epic/eepic"),
        o('F', "", "do not set font family/series/shape"),
        o('f', "font", "set default font"),
        o('i', "dir", "prepend the string \"dir\" to included graphics files"),
        o('o', "", "do not set the font size"),
        o('P', "", "pagemode, generate stand-alone LaTeX-file"),
        o('R', "num", "use LaTeX-arrows for arrow-type num"),
        o('r', "", "use LaTeX-arrows for all arrows"),
        o('T', "", "only use TeX fonts (even when PS-fonts are specified)"),
        o('v', "", "verbose mode, include comments in the output"),
        o('w', "", "remove suffix from included graphics-files"),
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

// -f is accepted, but fig2dev's help doesn't list it.
const TIKZ: DriverHelp = DriverHelp {
    title: "TIKZ",
    grid: true,
    options: &[
        o('b', "width", "specify width of blank border around figure (1/72 inch)"),
        o(
            'C',
            "num",
            "do not issue color commands for color num (0 = black,\n1 = blue, etc., see xfig color chooser)",
        ),
        o('F', "", "do not set font family/series/shape"),
        o('i', "dir", "prepend the string \"dir\" to included graphics files"),
        o(
            'O',
            "",
            "do not quote characters special to TeX/LaTeX\n(effectively, set the text flag \"TeX\" for all text)",
        ),
        o('o', "", "do not set the font size"),
        o('P', "", "pagemode, generate stand-alone LaTeX-file"),
        o('T', "", "only use TeX fonts (even when PS-fonts are specified)"),
        o('v', "", "verbose mode, include comments in the output"),
        o('W', "", "disable code that enables scaling of figure from TeX file"),
        o('w', "", "remove suffix from included graphics-files"),
    ],
};

// -M, -N and -O are listed by fig2dev's help but the driver rejects them,
// so they are left out here.
const PSTRICKS: DriverHelp = d(
    "PSTRICKS",
    &[
        o('f', "fontname", "set default font"),
        o('G', "dummyarg", "draw pstricks standard grid (sizes ignored)"),
        o(
            'l',
            "weight",
            "set line weight factor 0 to 2.0 (default 0.5 matches PS driver)",
        ),
        o('n', "mode", "page mode (0, 1, 2, or 3)"),
        o('P', "", "generate complete {document}; scale to fit one page"),
        o('p', "dir", "turn on auto picture conversion to EPS; dir is repository"),
        o(
            'R',
            "0|1|2",
            "arrow style (default 0 = Fig, 1 = PST with Fig sizes,\n   2 = PST default, ignore Fig sizes)",
        ),
        o(
            'S',
            "scale",
            "hard scale factor, scale must be between 8 and 12 inclusively",
        ),
        o('t', "version", "set PSTricks version"),
        o('v', "", "verbose warnings and comments in output file"),
        o(
            'x',
            "marginsize",
            "add margin on left and right (default is tight bounding box)",
        ),
        o(
            'y',
            "marginsize",
            "add margin on over and under (default is tight bounding box)",
        ),
        o(
            'z',
            "0|1|2",
            "font handling (default 0=full, 1=size only, 2=none,PST default)",
        ),
    ],
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

// -l and -p take a dummy argument in fig2dev's help; -g is Tcl/Tk only, -w
// Perl/Tk only.
const TK: DriverHelp = d(
    "Tcl/Tk (tk)",
    &[
        o('g', "color", "set background color"),
        o('l', "dummyarg", "landscape mode (dummy argument required after \"-l\")"),
        o('p', "dummyarg", "portrait mode (dummy argument required after \"-p\")"),
        o('P', "", "generate canvas of full page size instead of figure bounds"),
        o(
            'z',
            "papersize",
            "set the papersize (see man pages for available sizes)",
        ),
    ],
);

const PTK: DriverHelp = d(
    "Perl/Tk (ptk)",
    &[
        o('l', "dummyarg", "landscape mode (dummy argument required after \"-l\")"),
        o('p', "dummyarg", "portrait mode (dummy argument required after \"-p\")"),
        o('P', "", "generate canvas of full page size instead of figure bounds"),
        o('w', "", "add wrapper to generate complete perl file"),
        o(
            'z',
            "papersize",
            "set the papersize (see man pages for available sizes)",
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
        "emf" => &EMF,
        "epic" | "eepic" | "eepicemu" => &EPIC,
        "gbx" => &GBX,
        "ge" => &GE,
        "ibmgl" => &IBMGL,
        "latex" => &LATEX,
        "map" => &MAP,
        "mf" => &MF,
        "mp" | "mmp" => &MP,
        "pic" => &PIC,
        "pict2e" => &PICT2E,
        "pictex" => &PICTEX,
        "pstricks" => &PSTRICKS,
        "tk" => &TK,
        "ptk" => &PTK,
        "pstex_t" | "pdftex_t" => &PSTEX_T,
        "shape" => &SHAPE,
        "textyl" => &TEXTYL,
        "tikz" => &TIKZ,
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
        // The scene languages' help comes from `crate::options`.
        for l in LANGS {
            let listed = driver(l).is_some() && !crate::drivers::scene::writes(l);
            assert_eq!(help(l).is_some(), listed, "{l}");
        }
    }

    #[test]
    fn the_drivers_take_the_options_they_list() {
        for l in LANGS {
            let (Some(h), Some(mut dev)) = (help(l), driver(l)) else {
                continue;
            };
            // pstricks -p writes a readme into its picture directory
            let tmp = std::env::temp_dir().to_string_lossy().into_owned();
            for opt in h.options {
                let arg = match (*l, opt.letter) {
                    ("pic", 'p') => "all",
                    (_, 'S') if l.contains("epic") => "10",
                    ("textyl", 'l') => "2",
                    ("gbx", 'd') => "mm",
                    ("gbx", 'p') => "pos",
                    ("gbx", 'g') => "1x1+0+0",
                    ("gbx", 'f') => "2.4",
                    ("pstricks", 'p') => &tmp,
                    ("pstricks", 'S') => "10",
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
