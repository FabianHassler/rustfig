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

use rustfig_fig2dev::options::{Arg, Support, OPTIONS};
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
    /// A ported driver (`rustfig_fig2dev::drivers::driver`).
    Ported,
    /// Not ported yet: the plan task.
    Planned(&'static str),
}

/// fig2dev's drivers (drivers.h): name, suffix alias, writer.
const DRIVERS: &[(&str, Option<&str>, Writer)] = &[
    ("box", None, Writer::Ported),
    ("cgm", None, Writer::Planned("B9")),
    ("dxf", None, Writer::Planned("B9")),
    ("eepic", None, Writer::Planned("B9")),
    ("eepicemu", None, Writer::Planned("B9")),
    ("emf", None, Writer::Planned("B9")),
    ("epic", None, Writer::Planned("B9")),
    ("eps", None, Writer::Ported),
    ("gbx", None, Writer::Planned("B9")),
    ("ge", None, Writer::Planned("B9")),
    ("gif", None, Writer::Ported),
    ("ibmgl", None, Writer::Planned("B9")),
    ("jpeg", Some("jpg"), Writer::Ported),
    ("latex", None, Writer::Planned("B9")),
    ("map", None, Writer::Planned("B9")),
    ("mf", None, Writer::Planned("B9")),
    ("mp", None, Writer::Planned("B9")),
    ("pcx", None, Writer::Ported),
    ("pdf", None, Writer::Ported),
    // The graphics halves of the combined LaTeX languages (genpstex.c):
    // EPS and PDF without special texts.
    ("pdftex", None, Writer::Ported),
    ("pdftex_t", None, Writer::Planned("B2")),
    ("pic", None, Writer::Planned("B9")),
    ("pict2e", None, Writer::Planned("B3")),
    ("pictex", None, Writer::Planned("B9")),
    ("png", None, Writer::Ported),
    ("ppm", None, Writer::Ported),
    ("ps", None, Writer::Ported),
    ("pstex", None, Writer::Ported),
    ("pstex_t", None, Writer::Planned("B2")),
    ("pstricks", None, Writer::Planned("B3")),
    ("ptk", None, Writer::Planned("B9")),
    ("shape", None, Writer::Planned("B9")),
    ("sld", None, Writer::Planned("B9")),
    ("svg", None, Writer::Ported),
    ("textyl", None, Writer::Planned("B9")),
    ("tiff", Some("tif"), Writer::Ported),
    ("tikz", None, Writer::Planned("B1")),
    ("tk", None, Writer::Planned("B9")),
    ("tpic", None, Writer::Planned("B9")),
    ("xbm", None, Writer::Ported),
    ("xpm", None, Writer::Ported),
];

fn find_driver(name: &str) -> Option<&'static (&'static str, Option<&'static str>, Writer)> {
    DRIVERS.iter().find(|(n, _, _)| *n == name)
}

/// An error that ends the run (fig2dev's `exit(1)`).
struct Exit(String);

/// One parsed option: the letter and its argument.
type Opt = (char, Option<String>);

/// POSIX getopt over `args` (without the program name) with fig2dev's
/// option string, permuting as GNU getopt does (operands may come between
/// options; `--` ends the options). Returns the options in order, the
/// operands, and getopt's message for an unknown option or a missing
/// argument, which ends the options (the options before it still count).
fn getopt(args: &[String]) -> (Vec<Opt>, Vec<String>, Option<String>) {
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
                None => return (opts, operands, Some(format!("fig2dev: invalid option -- '{c}'"))),
                Some(false) => opts.push((c, None)),
                Some(true) => {
                    let rest = &flags[pos + c.len_utf8()..];
                    let value = if !rest.is_empty() {
                        rest.to_string()
                    } else if let Some(next) = args.get(i) {
                        i += 1;
                        next.clone()
                    } else {
                        return (
                            opts,
                            operands,
                            Some(format!("fig2dev: option requires an argument -- '{c}'")),
                        );
                    };
                    opts.push((c, Some(value)));
                    break;
                }
            }
        }
    }
    (opts, operands, None)
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

/// fig2dev's general options (fig2dev.c `help_msg`): letter and lines.
const GENERAL_HELP: &[(char, &str)] = &[
    ('h', "  -h          print this message, fig2dev version number and exit"),
    ('V', "  -V          print fig2dev version number and exit"),
    ('D', "  -D +/-list  include or exclude depths listed"),
    (
        'K',
        "  -K          adjust bounding box according to selected depths\n                      given with '-D +/-list' option",
    ),
    ('E', "  -E enc      set the character encoding of the input file"),
    (
        'G',
        "  -G minor[:major][unit]    draw light gray grid with thin/thick lines at\n                minor/major units (e.g., -G .25:1cm)",
    ),
    ('m', "  -m mag      set magnification.  This may not be used with the -Z option"),
    ('s', "  -s size     set default font size in points"),
    (
        'Z',
        "  -Z maxdim   scale the figure so that the maximum dimension (width or height)\n                is maxdim inches/cm.  This may not be used with the -m option\n  -Z <width>w   append 'w' or a colon (':') to specify the width, append 'h' or\n  -Z <height>h  prepend a colon to specify the height",
    ),
];

/// Whether RustFig's fig2dev writes `lang`.
fn writes(lang: &str) -> bool {
    rustfig_fig2dev::drivers::help(lang).is_some()
        || rustfig_fig2dev::drivers::driver(lang).is_some()
        || matches!(find_driver(lang).map(|d| d.2), Some(Writer::Ported))
}

/// Whether general option `c` does something for `lang`: -h and -V always,
/// -G where the grid is drawn, the others for every language RustFig writes.
fn general_effective(c: char, lang: &str) -> bool {
    match c {
        'h' | 'V' => true,
        'G' => match rustfig_fig2dev::drivers::help(lang) {
            Some(h) => h.grid,
            None => OPTIONS
                .iter()
                .any(|o| o.letter == 'G' && o.support == Support::Ported && o.effective_for(lang)),
        },
        _ => writes(lang),
    }
}

/// The options of `lang` that do something in RustFig, as help lines,
/// with the section title; general options are left out.
fn language_help(lang: &str) -> (String, Vec<String>) {
    if let Some(h) = rustfig_fig2dev::drivers::help(lang) {
        let lines = h
            .options
            .iter()
            .map(|o| rustfig_fig2dev::drivers::help::format_option(o.letter, o.arg, o.text))
            .collect();
        return (h.title.to_string(), lines);
    }
    let general: Vec<char> = GENERAL_HELP.iter().map(|g| g.0).chain(['L']).collect();
    let title = lang.to_uppercase();
    let lang = match lang {
        "pstex" => "eps",
        "pdftex" => "pdf",
        other => other,
    };
    let lines = OPTIONS
        .iter()
        .filter(|o| o.support == Support::Ported && o.effective_for(lang) && !general.contains(&o.letter))
        .map(|o| {
            let arg = match o.arg {
                Arg::Flag => "",
                Arg::Number => "num",
                Arg::Color => "color",
                Arg::Font => "font",
                Arg::Text | Arg::Depths | Arg::Size => "arg",
            };
            rustfig_fig2dev::drivers::help::format_option(o.letter, arg, o.help)
        })
        .collect();
    (title, lines)
}

/// fig2dev's help (`help_msg`): with a language, the general options that
/// do something for it and its own options that RustFig honours; without
/// one, the general options and the languages.
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
    for (c, text) in GENERAL_HELP {
        let shown = match lang {
            Some(l) => general_effective(*c, l),
            None => DRIVERS.iter().any(|(l, _, _)| general_effective(*c, l)),
        };
        if shown {
            println!("{text}");
        }
    }
    let Some(lang) = lang else {
        println!("\nUse fig2dev -L language -h for the options of one language.");
        return;
    };
    if let Some((_, _, Writer::Planned(task))) = find_driver(lang).filter(|_| !writes(lang)) {
        println!("\nThe {lang} language is not available in RustFig's fig2dev yet (plan task {task}).");
        return;
    }
    let (title, lines) = language_help(lang);
    if !lines.is_empty() {
        println!("\n{title} Options:");
        for l in lines {
            println!("{l}");
        }
    }
}

/// The drivers ported in `rustfig-fig2dev`, created when `-L` (or the
/// output suffix) first names them; each keeps its options, as fig2dev's
/// static driver state does.
#[derive(Default)]
struct Devices {
    list: Vec<(&'static str, Box<dyn rustfig_fig2dev::Driver>)>,
}

impl Devices {
    fn get(&mut self, lang: &'static str) -> Option<&mut Box<dyn rustfig_fig2dev::Driver>> {
        if !self.list.iter().any(|(l, _)| *l == lang) {
            let d = rustfig_fig2dev::drivers::driver(lang)?;
            self.list.push((lang, d));
        }
        self.list.iter_mut().find(|(l, _)| *l == lang).map(|(_, d)| d)
    }

    fn take(&mut self, lang: &str) -> Option<Box<dyn rustfig_fig2dev::Driver>> {
        let i = self.list.iter().position(|(l, _)| *l == lang)?;
        Some(self.list.swap_remove(i).1)
    }
}

/// Write fig2dev's messages gathered in `ctx`, one line each.
fn flush_messages(ctx: &mut rustfig_fig2dev::Ctx) {
    for m in ctx.messages.drain(..) {
        eprintln!("{m}");
    }
}

/// `dev->option(c, optarg)` for the language chosen so far.
fn pass_to_driver(
    lang: Option<&'static str>,
    devs: &mut Devices,
    ctx: &mut rustfig_fig2dev::Ctx,
    c: char,
    arg: Option<String>,
) -> Result<(), Exit> {
    let Some(lang) = lang else {
        return Err(Exit("No graphics language specified.".into()));
    };
    if let Some(d) = devs.get(lang) {
        let r = d.option(ctx, c, arg.as_deref());
        flush_messages(ctx);
        return r.map_err(Exit);
    }
    // A language not ported yet: its options are accepted; the run stops
    // with the plan task below.
    Ok(())
}

/// fig2dev's `strerror(errno)` for an I/O error; `None` is errno 0, as
/// after a read that failed on the file's content.
fn strerror(e: Option<&std::io::Error>) -> String {
    let code = e.map_or(Some(0), std::io::Error::raw_os_error);
    match code {
        Some(n) => {
            let s = std::io::Error::from_raw_os_error(n).to_string();
            match s.strip_suffix(&format!(" (os error {n})")) {
                Some(t) => t.to_string(),
                None => s,
            }
        }
        None => e.map(ToString::to_string).unwrap_or_default(),
    }
}

/// Read at most `MAX_FILE_BYTES` + 1 bytes.
fn read_capped(r: impl Read) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    r.take(rustfig_fig2dev::read::MAX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut buf)?;
    Ok(buf)
}

/// fig2dev's `main` reading the figure (`read_fig` or `readfp_fig`) with
/// its messages; `None` is `exit(EXIT_FAILURE)`.
fn read_figure(ctx: &mut rustfig_fig2dev::Ctx, from: Option<&str>) -> Option<rustfig_fig2dev::object::Compound> {
    // err_msg: the message, then ": " and strerror(errno).
    let not_accessible = |e: Option<&std::io::Error>| match from {
        Some(path) => format!("File \"{path}\" is not accessible: {}", strerror(e)),
        None => format!("Input error: {}", strerror(e)),
    };
    let src = match from {
        Some(path) => {
            match std::fs::metadata(path) {
                Err(e) => {
                    eprintln!("{}", not_accessible(Some(&e)));
                    return None;
                }
                Ok(m) if !m.is_file() => {
                    eprintln!("Not a regular file: {path}");
                    return None;
                }
                Ok(_) => {}
            }
            std::fs::File::open(path).and_then(read_capped)
        }
        None => read_capped(std::io::stdin().lock()),
    };
    let src = match src {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", not_accessible(Some(&e)));
            return None;
        }
    };
    if src.len() > rustfig_fig2dev::read::MAX_FILE_BYTES {
        eprintln!("fig2dev: input file is too large");
        return None;
    }
    let objects = rustfig_fig2dev::read::readfp_fig(ctx, &src);
    flush_messages(ctx);
    match objects {
        Ok(o) => Some(o),
        Err(-3) => {
            eprintln!("{}", not_accessible(None));
            None
        }
        Err(_) => None,
    }
}

/// The output (`tfp`): a file opened (and emptied) before the conversion,
/// as fig2dev's `main` opens it, or stdout.
enum Output {
    File(String, std::fs::File),
    Stdout,
}

fn open_output(to: Option<&String>) -> Result<Output, Exit> {
    let Some(path) = to else {
        return Ok(Output::Stdout);
    };
    if path.ends_with(".fig") {
        return Err(Exit("Outfile is a .fig file, aborting".into()));
    }
    let f = std::fs::File::create(path).map_err(|_| Exit(format!("Couldn't open {path}")))?;
    Ok(Output::File(path.clone(), f))
}

/// Write the result.
fn write_output(out: Output, bytes: &[u8]) -> Result<(), Exit> {
    match out {
        Output::File(path, mut f) => f
            .write_all(bytes)
            .and_then(|()| f.flush())
            .map_err(|e| Exit(format!("fig2dev: {path}: {}", strerror(Some(&e))))),
        Output::Stdout => {
            let mut stdout = std::io::stdout().lock();
            stdout
                .write_all(bytes)
                .and_then(|()| stdout.flush())
                .map_err(|e| Exit(format!("fig2dev: {}", strerror(Some(&e)))))
        }
    }
}

/// fig2dev's `get_args` and `main`.
fn run(args: Vec<String>) -> Result<u8, Exit> {
    if args.is_empty() {
        eprint!("{USAGE}");
        return Ok(0);
    }
    if args.first().map(String::as_str) == Some("--version") {
        println!("fig2dev Version {VERSION}");
        return Ok(0);
    }
    let mut ctx = rustfig_fig2dev::Ctx::default();
    let mut devs = Devices::default();
    let mut lang: Option<&'static str> = None;
    // The language guessed from the last argument's suffix gets -L, as an
    // explicit -L would.
    if let Some(name) = args.last().and_then(|l| guess_from_suffix(l)) {
        lang = Some(name);
        ctx.general_option('L', Some(name)).map_err(Exit)?;
        pass_to_driver(lang, &mut devs, &mut ctx, 'L', Some(name.to_string()))?;
    }
    // One pass in command-line order, as fig2dev's getopt loop.
    let (opts, operands, error) = getopt(&args);
    for (c, arg) in opts {
        match c {
            'h' => {
                print_version();
                print_help(lang);
                return Ok(0);
            }
            'V' => {
                print_version();
                return Ok(0);
            }
            '?' => {
                eprint!("{USAGE}");
                return Ok(1);
            }
            'Z' => {
                ctx.general_option(c, arg.as_deref()).map_err(Exit)?;
                // fig2dev's -Z message has no newline.
                for m in ctx.messages.drain(..) {
                    eprint!("{m}");
                }
            }
            'L' => {
                // fig2dev keeps 11 characters (`lang[12]`).
                let full = arg.unwrap_or_default();
                let l = String::from_utf8_lossy(full.as_bytes().get(..11).unwrap_or(full.as_bytes())).into_owned();
                let Some((name, _, _)) = find_driver(&l) else {
                    let known: Vec<&str> = DRIVERS.iter().map(|(n, _, _)| *n).collect();
                    return Err(Exit(format!(
                        "Unknown graphics language {l}\nKnown languages are:\n{} ",
                        known.join(" ")
                    )));
                };
                lang = Some(name);
                ctx.general_option('L', Some(name)).map_err(Exit)?;
                pass_to_driver(lang, &mut devs, &mut ctx, 'L', Some(name.to_string()))?;
            }
            _ => {
                let general = ctx.general_option(c, arg.as_deref()).map_err(Exit)?;
                flush_messages(&mut ctx);
                if !general {
                    // -G and the driver's own options.
                    pass_to_driver(lang, &mut devs, &mut ctx, c, arg)?;
                }
            }
        }
    }
    if let Some(e) = error {
        // getopt's message, then fig2dev's usage.
        eprint!("{e}\n{USAGE}");
        return Ok(1);
    }
    // After the options, so that -m counts (fig2dev's get_args).
    ctx.font_size *= ctx.fontmag;
    let Some(lang) = lang else {
        return Err(Exit("No graphics language specified.".into()));
    };
    if ctx.magspec && ctx.maxdimspec != 0 {
        return Err(Exit(
            "Must specify only one of -m (magnification) and -Z (max dimension).".into(),
        ));
    }
    let from = operands.first().filter(|f| f.as_str() != "-");
    let to = operands.get(1).filter(|t| t.as_str() != "-");
    ctx.from = from.cloned();
    ctx.to = to.cloned();

    let Some(mut dev) = devs.take(lang) else {
        return Err(Exit(match find_driver(lang).map(|d| d.2) {
            Some(Writer::Planned(task)) => {
                format!("fig2dev: the {lang} language is not available in RustFig's fig2dev yet (plan task {task})")
            }
            _ => format!("fig2dev: no writer for {lang}"),
        }));
    };

    // Read the figure with fig2dev's reader: its checks, repairs and
    // messages.
    let Some(objects) = read_figure(&mut ctx, from.map(String::as_str)) else {
        return Ok(1);
    };
    let output = open_output(to)?;
    let status = rustfig_fig2dev::convert(&mut ctx, dev.as_mut(), &objects);
    flush_messages(&mut ctx);
    let status = status.map_err(Exit)?;
    write_output(output, &status.output)?;
    // exit(status): the low byte, as the shell sees it.
    Ok(status.exit as u8)
}

fn main() -> ExitCode {
    // Arguments that aren't UTF-8 are read lossily rather than aborting.
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    match run(args) {
        Ok(code) => ExitCode::from(code),
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
        let (o, ops, e) = getopt(&s(&["-L", "eps", "-Fb10", "in.fig", "-N", "out.eps"]));
        assert_eq!(e, None);
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
        let (_, ops, _) = getopt(&s(&["-Leps", "--", "-weird.fig"]));
        assert_eq!(ops, s(&["-weird.fig"]));
        assert!(getopt(&s(&["-u"])).2.is_some(), "not in fig2dev's option string");
        assert!(getopt(&s(&["-b"])).2.is_some(), "-b needs an argument");
        let (o, _, e) = getopt(&s(&["-K", "-u", "-N"]));
        assert_eq!(
            (o, e.is_some()),
            (vec![('K', None)], true),
            "options before the error count"
        );
        let (_, ops, _) = getopt(&s(&["-", "-"]));
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
