//! fig2dev: fig2dev 3.2.9a's command line on RustFig's ported drivers (plan
//! B10), so scripts and Makefiles that call fig2dev keep working without
//! the C program or ghostscript.
//!
//! `fig2dev [-L language] [options] [infile [outfile]]`: options are handled
//! in order, as fig2dev's `get_args` does: general options in fig2dev.c,
//! the others by the language's driver, which `-L` (or the output file's
//! suffix) chooses. Options a driver accepts but ignores (for example the
//! page options for EPS) are accepted silently; options RustFig hasn't
//! ported yet are ignored with a warning naming the plan task; options the
//! driver doesn't know are errors (warnings for the bitmap languages), as in
//! fig2dev. Languages not ported yet fail with a message naming their task.
//
// Ported from fig2dev fig2dev.c (`get_args`, `main`, `help_msg`), drivers.h
// and messages.c (xfig licence, see LICENSE-xfig).

use rustfig_format::{read_fig_encoded, TextEncoding};
use rustfig_render::options::{option_spec, parse_options, Arg, OptionValue, Support, OPTIONS};
use rustfig_render::{ExportFormat, ExportOptions};
use std::io::{Read, Write};
use std::process::ExitCode;

const VERSION: &str = "3.2.9a";

/// fig2dev's option letters (fig2dev.c): a letter followed by `:` takes an
/// argument.
const OPTSTRING: &str = "AaB:b:C:cD:d:E:eFf:G:g:hi:jKkL:l:Mm:Nn:OoPp:q:R:rS:s:Tt:VvWwX:x:Y:y:Z:z:?";

const USAGE: &str =
    "Usage:\n fig2dev -hV\n fig2dev -L language -h\n fig2dev [-L language] [other options] [in.fig [output]]\n";

/// How RustFig writes one of fig2dev's languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Writer {
    /// A ported driver.
    Format(ExportFormat),
    /// Not ported yet: the plan task.
    Planned(&'static str),
}

/// fig2dev's drivers (drivers.h): name, suffix alias, writer.
const DRIVERS: &[(&str, Option<&str>, Writer)] = &[
    ("box", None, Writer::Format(ExportFormat::LatexBox)),
    ("cgm", None, Writer::Planned("B9")),
    ("dxf", None, Writer::Planned("B9")),
    ("eepic", None, Writer::Planned("B9")),
    ("eepicemu", None, Writer::Planned("B9")),
    ("emf", None, Writer::Planned("B9")),
    ("epic", None, Writer::Planned("B9")),
    ("eps", None, Writer::Format(ExportFormat::Eps)),
    ("gbx", None, Writer::Planned("B9")),
    ("ge", None, Writer::Planned("B9")),
    ("gif", None, Writer::Planned("B6")),
    ("ibmgl", None, Writer::Planned("B9")),
    ("jpeg", Some("jpg"), Writer::Planned("B6")),
    ("latex", None, Writer::Planned("B9")),
    ("map", None, Writer::Planned("B9")),
    ("mf", None, Writer::Planned("B9")),
    ("mp", None, Writer::Planned("B9")),
    ("pcx", None, Writer::Planned("B6")),
    ("pdf", None, Writer::Format(ExportFormat::Pdf)),
    ("pdftex", None, Writer::Planned("B2")),
    ("pdftex_t", None, Writer::Planned("B2")),
    ("pic", None, Writer::Planned("B9")),
    ("pict2e", None, Writer::Planned("B3")),
    ("pictex", None, Writer::Planned("B9")),
    ("png", None, Writer::Format(ExportFormat::Png)),
    ("ppm", None, Writer::Planned("B6")),
    // PostScript pages: RustFig writes the figure as EPS-style PostScript
    // until the page layout is ported (B4).
    ("ps", None, Writer::Format(ExportFormat::Eps)),
    ("pstex", None, Writer::Planned("B2")),
    ("pstex_t", None, Writer::Planned("B2")),
    ("pstricks", None, Writer::Planned("B3")),
    ("ptk", None, Writer::Planned("B9")),
    ("shape", None, Writer::Planned("B9")),
    ("sld", None, Writer::Planned("B9")),
    ("svg", None, Writer::Format(ExportFormat::Svg)),
    ("textyl", None, Writer::Planned("B9")),
    ("tiff", Some("tif"), Writer::Planned("B6")),
    ("tikz", None, Writer::Planned("B1")),
    ("tk", None, Writer::Planned("B9")),
    ("tpic", None, Writer::Planned("B9")),
    ("xbm", None, Writer::Planned("B6")),
    ("xpm", None, Writer::Planned("B6")),
];

const BITMAP_DRIVERS: &[&str] = &["gif", "jpeg", "pcx", "png", "ppm", "sld", "tiff", "xbm", "xpm"];

/// The name the driver uses in its messages (`Err_badarg`): genps speaks
/// for ps, eps and pdf.
fn driver_name(lang: &str) -> &str {
    match lang {
        "eps" | "pdf" => "ps",
        other => other,
    }
}

fn find_driver(name: &str) -> Option<&'static (&'static str, Option<&'static str>, Writer)> {
    DRIVERS.iter().find(|(n, _, _)| *n == name)
}

/// An error that ends the run (fig2dev's `exit(1)`).
struct Exit(String);

/// One parsed option: the letter and its argument.
type Opt = (char, Option<String>);

/// POSIX getopt over `args` (without the program name) with fig2dev's
/// option string, permuting as GNU getopt does (operands may come between
/// options; `--` ends the options). Returns the options in order and the
/// operands, or the message for an unknown option or a missing argument.
fn getopt(args: &[String]) -> Result<(Vec<Opt>, Vec<String>), String> {
    let takes_arg = |c: char| {
        OPTSTRING
            .char_indices()
            .find(|(_, x)| *x == c && c != ':')
            .map(|(i, _)| OPTSTRING[i + c.len_utf8()..].starts_with(':'))
    };
    let mut opts = Vec::new();
    let mut operands = Vec::new();
    let mut i = 0;
    while let Some(a) = args.get(i) {
        i += 1;
        if a == "--" {
            operands.extend(args.iter().skip(i).cloned());
            break;
        }
        let Some(flags) = a.strip_prefix('-').filter(|f| !f.is_empty()) else {
            operands.push(a.clone());
            continue;
        };
        for (pos, c) in flags.char_indices() {
            match takes_arg(c) {
                None => return Err(format!("fig2dev: invalid option -- '{c}'")),
                Some(false) => opts.push((c, None)),
                Some(true) => {
                    let rest = &flags[pos + c.len_utf8()..];
                    let value = if !rest.is_empty() {
                        rest.to_string()
                    } else if let Some(next) = args.get(i) {
                        i += 1;
                        next.clone()
                    } else {
                        return Err(format!("fig2dev: option requires an argument -- '{c}'"));
                    };
                    opts.push((c, Some(value)));
                    break;
                }
            }
        }
    }
    Ok((opts, operands))
}

/// The language chosen by the output file's suffix, as fig2dev guesses it
/// from the last argument before reading any options.
fn guess_from_suffix(last: &str) -> Option<&'static str> {
    let ext = last.rsplit_once('.')?.1;
    DRIVERS
        .iter()
        .find(|(n, alias, _)| *n == ext || *alias == Some(ext))
        .map(|(n, _, _)| *n)
}

fn print_version() {
    println!("fig2dev Version {VERSION} (RustFig {})", env!("CARGO_PKG_VERSION"));
}

/// fig2dev's help (`help_msg`): general options, then the options of the
/// chosen language (or of every language RustFig writes).
fn print_help(lang: Option<&str>) {
    print!("{USAGE}");
    println!("General Options (all drivers):");
    println!("  -L language\tchoose output language (this must be first, if the language");
    println!("                cannot be inferred from the suffix of the output file)");
    print!("                Available languages are:\n                   ");
    let mut n = 20;
    for (name, _, _) in DRIVERS {
        if n > 72 {
            print!("\n                   ");
            n = 20;
        }
        print!(" {name}");
        n += name.len() + 1;
    }
    println!();
    println!("  -h          print this message, fig2dev version number and exit");
    println!("  -V          print fig2dev version number and exit");
    println!("  -D +/-list  include or exclude depths listed");
    println!("  -K          adjust bounding box according to selected depths");
    println!("                      given with '-D +/-list' option");
    println!("  -E enc      set the character encoding of the input file");
    println!("  -G minor[:major][unit]    draw light gray grid with thin/thick lines at");
    println!("                minor/major units (e.g., -G .25:1cm)");
    println!("  -m mag      set magnification.  This may not be used with the -Z option");
    println!("  -s size     set default font size in points");
    println!("  -Z maxdim   scale the figure so that the maximum dimension (width or height)");
    println!("                is maxdim inches/cm.  This may not be used with the -m option");
    println!("  -Z <width>w   append 'w' or a colon (':') to specify the width, append 'h' or");
    println!("  -Z <height>h  prepend a colon to specify the height");
    let general = ['D', 'K', 'E', 'G', 'm', 's', 'Z', 'L', 'h', 'V'];
    let langs: Vec<&str> = match lang {
        Some(l) => vec![l],
        None => DRIVERS
            .iter()
            .filter(|(_, _, w)| matches!(w, Writer::Format(_)))
            .map(|(n, _, _)| *n)
            .collect(),
    };
    for l in langs {
        let opts: Vec<_> = OPTIONS
            .iter()
            .filter(|o| o.applies_to(l) && !general.contains(&o.letter))
            .collect();
        if opts.is_empty() {
            continue;
        }
        println!("\n{} Options:", l.to_uppercase());
        for o in opts {
            let arg = match o.arg {
                Arg::Flag => "",
                Arg::Number => " num",
                Arg::Color => " color",
                Arg::Font => " font",
                Arg::Text | Arg::Depths | Arg::Size => " arg",
            };
            let note = match o.support {
                Support::Ported if o.effective_for(l) => String::new(),
                Support::Ported => " (no effect here)".to_string(),
                Support::Planned(t) => format!(" (not in RustFig yet: {t})"),
            };
            println!("  -{}{arg:<10} {}{note}", o.letter, o.help);
        }
    }
}

/// The run's settings, gathered as fig2dev's `get_args` gathers them.
struct Run {
    lang: Option<&'static str>,
    encoding: TextEncoding,
    /// Options for the driver and the general ones RustFig applies in
    /// `export` (by fig2dev letter).
    items: Vec<(String, OwnedValue)>,
    magspec: bool,
    maxdimspec: bool,
    warnings: Vec<String>,
}

enum OwnedValue {
    Bool(bool),
    Text(String),
}

impl Run {
    /// An option for the chosen driver (`dev->option(c, optarg)`).
    fn driver_option(&mut self, lang: &'static str, c: char, arg: Option<String>) -> Result<(), Exit> {
        // -L and -G reach every driver; they are handled globally.
        if c == 'L' {
            return Ok(());
        }
        let Some(spec) = option_spec(lang, &c.to_string()).filter(|s| s.letter == c) else {
            let msg = format!("Argument -{c} unknown to {} driver.", driver_name(lang));
            if BITMAP_DRIVERS.contains(&lang) {
                // genbitmaps_option warns and goes on.
                self.warnings.push(msg);
                return Ok(());
            }
            return Err(Exit(msg));
        };
        if !spec.effective_for(lang) {
            return Ok(()); // fig2dev accepts it here and does nothing
        }
        if let Support::Planned(task) = spec.support {
            self.warnings.push(format!(
                "fig2dev: option -{c} is not implemented in RustFig yet (plan task {task}); ignored"
            ));
            return Ok(());
        }
        let value = match (spec.arg, arg) {
            (Arg::Flag, _) => OwnedValue::Bool(true),
            (_, Some(a)) => OwnedValue::Text(a),
            (_, None) => OwnedValue::Bool(true),
        };
        self.items.push((spec.key.to_string(), value));
        Ok(())
    }
}

/// fig2dev's `get_args` and `main`.
fn run(args: Vec<String>) -> Result<(), Exit> {
    if args.is_empty() {
        eprint!("{USAGE}");
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("--version") {
        println!("fig2dev Version {VERSION}");
        return Ok(());
    }
    let mut run = Run {
        lang: args.last().and_then(|l| guess_from_suffix(l)),
        encoding: TextEncoding::Auto,
        items: Vec::new(),
        magspec: false,
        maxdimspec: false,
        warnings: Vec::new(),
    };
    let (opts, operands) = getopt(&args).map_err(|e| Exit(format!("{e}\n{USAGE}")))?;
    for (c, arg) in opts {
        let text = || arg.clone().unwrap_or_default();
        match c {
            'h' => {
                print_version();
                print_help(run.lang);
                return Ok(());
            }
            'V' => {
                print_version();
                return Ok(());
            }
            'D' => run.items.push(("depths".into(), OwnedValue::Text(text()))),
            'K' => run.items.push(("adjust_bbox".into(), OwnedValue::Bool(true))),
            'E' => {
                run.encoding = TextEncoding::from_name(&text()).ok_or_else(|| {
                    Exit(format!(
                        "fig2dev: input encoding {:?} is not supported (UTF-8, ISO-8859-1, ISO-8859-2)",
                        text()
                    ))
                })?;
            }
            'm' => {
                run.magspec = true;
                run.items.push(("magnification".into(), OwnedValue::Text(text())));
            }
            's' => {
                // fig2dev: a size of zero or less means the default size.
                if text().trim().parse::<f64>().is_ok_and(|s| s > 0.0) {
                    run.items.push(("font_size".into(), OwnedValue::Text(text())));
                }
            }
            'Z' => {
                run.maxdimspec = true;
                run.items.push(("max_size".into(), OwnedValue::Text(text())));
            }
            'L' => {
                let l = text();
                let Some((name, _, _)) = find_driver(&l) else {
                    let known: Vec<&str> = DRIVERS.iter().map(|(n, _, _)| *n).collect();
                    return Err(Exit(format!(
                        "Unknown graphics language {l}\nKnown languages are:\n{} ",
                        known.join(" ")
                    )));
                };
                run.lang = Some(name);
            }
            '?' => return Err(Exit(USAGE.trim_end().to_string())),
            _ => {
                let Some(lang) = run.lang else {
                    return Err(Exit("No graphics language specified.".into()));
                };
                if c == 'G' {
                    // Parsed in fig2dev.c, drawn by the driver (not ported).
                    run.driver_option(lang, c, arg)?;
                    continue;
                }
                run.driver_option(lang, c, arg)?;
            }
        }
    }
    let Some(lang) = run.lang else {
        return Err(Exit("No graphics language specified.".into()));
    };
    if run.magspec && run.maxdimspec {
        return Err(Exit(
            "Must specify only one of -m (magnification) and -Z (max dimension).".into(),
        ));
    }
    let from = operands.first().filter(|f| f.as_str() != "-");
    let to = operands.get(1).filter(|t| t.as_str() != "-");
    for w in &run.warnings {
        eprintln!("{w}");
    }
    let format = match find_driver(lang).map(|d| d.2) {
        Some(Writer::Format(f)) => f,
        Some(Writer::Planned(task)) => {
            return Err(Exit(format!(
                "fig2dev: the {lang} language is not available in RustFig's fig2dev yet (plan task {task})"
            )));
        }
        None => return Err(Exit("No graphics language specified.".into())),
    };
    if lang == "ps" {
        eprintln!("fig2dev: PostScript page layout is not ported yet (plan task B4); writing the figure as EPS-style PostScript");
    }

    // Read the figure.
    let src = match from {
        Some(path) => std::fs::read(path).map_err(|_| Exit(format!("File \"{path}\" is not accessible")))?,
        None => {
            let mut buf = Vec::new();
            std::io::stdin()
                .take(rustfig_format::MAX_FILE_BYTES as u64 + 1)
                .read_to_end(&mut buf)
                .map_err(|_| Exit("Input error".into()))?;
            buf
        }
    };
    if src.len() > rustfig_format::MAX_FILE_BYTES {
        return Err(Exit("fig2dev: input file is too large".into()));
    }
    let mut doc = read_fig_encoded(&src, run.encoding).map_err(|e| Exit(format!("fig2dev: {e}")))?;
    let base = from.and_then(|f| std::path::Path::new(f).parent().map(std::path::Path::to_path_buf));
    for w in rustfig_engine::import::load_all_pictures(&mut doc, base.as_deref()) {
        eprintln!("fig2dev: {w}");
    }

    // fig2dev reads font sizes in 1/80 inch unless -F says points.
    let mut items: Vec<(String, OwnedValue)> = Vec::new();
    if option_spec(lang, "correct_font_size").is_some_and(|s| s.effective_for(lang)) {
        items.push(("correct_font_size".into(), OwnedValue::Bool(false)));
    }
    items.extend(run.items);
    let borrowed: Vec<(&str, OptionValue<'_>)> = items
        .iter()
        .map(|(k, v)| {
            (
                k.as_str(),
                match v {
                    OwnedValue::Bool(b) => OptionValue::Bool(*b),
                    OwnedValue::Text(t) => OptionValue::Text(t.as_str()),
                },
            )
        })
        .collect();
    let language = rustfig_render::languages::language(match lang {
        "ps" => "eps",
        l => l,
    })
    .ok_or_else(|| Exit(format!("fig2dev: no writer for {lang}")))?;
    let fig2dev = parse_options(language, &borrowed).map_err(|e| Exit(format!("fig2dev: {e}")))?;
    let opts = ExportOptions {
        // fig2dev's bitmaps: ghostscript at 80 dpi times the magnification.
        dpi: 80.0,
        fig2dev,
        ..ExportOptions::default()
    };
    let out = rustfig_render::export(&doc, format, &opts).map_err(|e| Exit(format!("fig2dev: {e}")))?;

    // Write the result.
    match to {
        Some(path) => {
            if path.ends_with(".fig") {
                return Err(Exit("Outfile is a .fig file, aborting".into()));
            }
            std::fs::write(path, &out).map_err(|_| Exit(format!("Couldn't open {path}")))?;
        }
        None => {
            let mut stdout = std::io::stdout().lock();
            stdout
                .write_all(&out)
                .and_then(|()| stdout.flush())
                .map_err(|e| Exit(format!("fig2dev: {e}")))?;
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Exit(msg)) => {
            eprintln!("{msg}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn getopt_groups_attached_and_permuted() {
        let (o, ops) = getopt(&s(&["-L", "eps", "-Fb10", "in.fig", "-N", "out.eps"])).unwrap();
        assert_eq!(
            o,
            vec![
                ('L', Some("eps".into())),
                ('F', None),
                ('b', Some("10".into())),
                ('N', None)
            ]
        );
        assert_eq!(ops, s(&["in.fig", "out.eps"]));
        let (_, ops) = getopt(&s(&["-Leps", "--", "-weird.fig"])).unwrap();
        assert_eq!(ops, s(&["-weird.fig"]));
        assert!(getopt(&s(&["-u"])).is_err(), "not in fig2dev's option string");
        assert!(getopt(&s(&["-b"])).is_err(), "-b needs an argument");
        let (_, ops) = getopt(&s(&["-", "-"])).unwrap();
        assert_eq!(ops, s(&["-", "-"]), "- is stdin/stdout");
    }

    #[test]
    fn suffixes_choose_the_language() {
        assert_eq!(guess_from_suffix("a/b.eps"), Some("eps"));
        assert_eq!(guess_from_suffix("x.jpg"), Some("jpeg"));
        assert_eq!(guess_from_suffix("x.tif"), Some("tiff"));
        assert_eq!(guess_from_suffix("x.EPS"), None, "fig2dev compares exactly");
        assert_eq!(guess_from_suffix("x"), None);
    }
}
