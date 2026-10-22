//! xfig's command line (plan S3): the options of `xfig.man.in` for RustFig.
//!
//! Options with an equivalent become resource values that override the
//! preferences for one run (never written to the settings file). X toolkit
//! options are accepted; the ones without meaning here are ignored with a
//! note. As in Xt, an option may be abbreviated to a prefix: the exact name
//! wins, otherwise the first option of the table that starts with the
//! argument (application options before the toolkit's). Options that take a
//! value consume the next argument, whatever it looks like.
//
// Ported from xfig src/main.c (`options`, `help_list`, argument handling)
// and doc/xfig.man.in (xfig licence, see LICENSE-xfig).

use crate::prefs::{resource, Prefs, NOT_APPLICABLE};

/// What an option does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Sets resource `.0` to the fixed value `.1`.
    Flag(&'static str, &'static str),
    /// Sets resource `.0` to the next argument.
    Arg(&'static str),
    /// X toolkit option without an argument, ignored.
    IgnoreFlag,
    /// X toolkit option with an argument, ignored.
    IgnoreArg,
}

use Action::{Arg, Flag, IgnoreArg, IgnoreFlag};

/// xfig's option table (`main.c`), in its order, then the X toolkit's.
#[rustfmt::skip]
pub const OPTIONS: &[(&str, Action)] = &[
    ("-visual", Arg("visual")), ("-depth", Arg("depth")),
    ("-allownegcoords", Flag("allownegcoords", "true")),
    ("-autorefresh", Flag("autorefresh", "true")),
    ("-balloon_delay", Arg("balloon_delay")),
    ("-buttonFont", Arg("buttonFont")),
    ("-but_per_row", Arg("but_per_row")),
    ("-cbg", Arg("canvasbackground")),
    ("-center", Flag("flushleft", "false")),
    ("-centimeters", Flag("inches", "false")),
    ("-cfg", Arg("canvasforeground")),
    ("-correct_font_size", Flag("correctfontsize", "true")),
    ("-crosshair", Flag("crosshair", "true")),
    ("-debug", Flag("debug", "true")),
    ("-dontallownegcoords", Flag("allownegcoords", "false")),
    ("-dontshowaxislines", Flag("showaxislines", "false")),
    ("-dontshowballoons", Flag("showballoons", "false")),
    ("-dontshowlengths", Flag("showlengths", "false")),
    ("-dontshownums", Flag("shownums", "false")),
    ("-dontshowpageborder", Flag("showpageborder", "false")),
    ("-dontswitchcmap", Flag("dontswitchcmap", "true")),
    ("-encoding", Arg("encoding")),
    ("-exportLanguage", Arg("exportLanguage")),
    ("-export_margin", Arg("export_margin")),
    ("-flipvisualhints", Flag("flipvisualhints", "true")),
    ("-noflipvisualhints", Flag("flipvisualhints", "false")),
    ("-flushleft", Flag("flushleft", "true")),
    ("-freehand_resolution", Arg("freehand_resolution")),
    ("-ghostscript", Arg("ghostscript")),
    ("-grid_color", Arg("grid_color")),
    ("-grid_unit", Arg("grid_unit")),
    ("-gslib", Arg("gslib")),
    ("-hiddentext", Flag("hiddentext", "true")),
    ("-dontshowdepthmanager", Flag("showdepthmanager", "false")),
    ("-iconGeometry", Arg("iconGeometry")),
    ("-icon_view", Flag("icon_view", "true")),
    ("-image_editor", Arg("image_editor")),
    ("-imperial", Flag("inches", "true")),
    ("-inches", Flag("inches", "true")),
    ("-installowncmap", Flag("installowncmap", "true")),
    ("-internalBW", Arg("internalborderwidth")),
    ("-jpeg_quality", Arg("jpeg_quality")),
    ("-keyFile", Arg("keyFile")),
    ("-Landscape", Flag("landscape", "true")),
    ("-landscape", Flag("landscape", "true")),
    ("-latexfonts", Flag("latexfonts", "true")),
    ("-left", Flag("justify", "false")),
    ("-library_dir", Arg("library_dir")),
    ("-library_icon_size", Arg("library_icon_size")),
    ("-list_view", Flag("icon_view", "false")),
    ("-magnification", Arg("magnification")),
    ("-max_image_colors", Arg("max_image_colors")),
    ("-metric", Flag("inches", "false")),
    ("-monochrome", Flag("monochrome", "true")),
    ("-multiple", Flag("multiple", "true")),
    ("-nooverlap", Flag("overlap", "false")),
    ("-normalFont", Arg("normalFont")),
    ("-nosplash", Flag("splash", "false")),
    ("-notrack", Flag("trackCursor", "false")),
    ("-nowrite_bak", Flag("write_bak", "false")),
    ("-overlap", Flag("overlap", "true")),
    ("-pageborder", Arg("pageborder")),
    ("-paper_size", Arg("paper_size")),
    ("-pheight", Arg("pheight")),
    ("-Portrait", Flag("landscape", "false")),
    ("-portrait", Flag("landscape", "false")),
    ("-pwidth", Arg("pwidth")),
    ("-right", Flag("justify", "true")),
    ("-rigidtext", Flag("rigidtext", "true")),
    ("-rulerthick", Arg("rulerthick")),
    ("-scale_factor", Arg("scale_factor")),
    ("-showallbuttons", Flag("showallbuttons", "true")),
    ("-showballoons", Flag("showballoons", "true")),
    ("-showdepthmanager", Flag("showdepthmanager", "true")),
    ("-showlengths", Flag("showlengths", "true")),
    ("-shownums", Flag("shownums", "true")),
    ("-showpageborder", Flag("showpageborder", "true")),
    ("-showaxislines", Flag("showaxislines", "true")),
    ("-single", Flag("multiple", "false")),
    ("-smooth_factor", Arg("smooth_factor")),
    ("-smallicons", Flag("smallicons", "true")),
    ("-specialtext", Flag("specialtext", "true")),
    ("-spellcheckcommand", Arg("spellcheckcommand")),
    ("-spinner_delay", Arg("spinner_delay")),
    ("-spinner_rate", Arg("spinner_rate")),
    ("-splash", Flag("splash", "true")),
    ("-startfillstyle", Arg("startfillstyle")),
    ("-startarrowtype", Arg("startarrowtype")),
    ("-startarrowthick", Arg("startarrowthick")),
    ("-startarrowwidth", Arg("startarrowwidth")),
    ("-startarrowlength", Arg("startarrowlength")),
    ("-startFontSize", Arg("startfontsize")),
    ("-startfontsize", Arg("startfontsize")),
    ("-startgridmode", Arg("startgridmode")),
    ("-startgridtype", Arg("startgridtype")),
    ("-startlatexFont", Arg("startlatexFont")),
    ("-startlinewidth", Arg("startlinewidth")),
    ("-startposnmode", Arg("startposnmode")),
    ("-startpsFont", Arg("startpsFont")),
    ("-starttextstep", Arg("starttextstep")),
    ("-tablet", Flag("tablet", "true")),
    ("-track", Flag("trackCursor", "true")),
    ("-transparent_color", Arg("transparent_color")),
    ("-userscale", Arg("userscale")),
    ("-write_v40", Flag("write_v40", "true")),
    ("-write_bak", Flag("write_bak", "true")),
    ("-userunit", Arg("userunit")),
    ("-axislines", Arg("axislines")),
    ("-zoom", Arg("zoom")),
    ("-international", Flag("international", "true")),
    ("-nointernational", Flag("international", "false")),
    ("-inputStyle", Arg("inputStyle")),
    // The X toolkit's own options (Xt appends them after the application's).
    ("-geometry", Arg("geometry")),
    ("-display", IgnoreArg), ("-iconic", IgnoreFlag), ("-name", IgnoreArg),
    ("-title", IgnoreArg), ("-background", IgnoreArg), ("-bg", IgnoreArg),
    ("-foreground", IgnoreArg), ("-fg", IgnoreArg), ("-bordercolor", IgnoreArg),
    ("-bd", IgnoreArg), ("-borderwidth", IgnoreArg), ("-bw", IgnoreArg),
    ("-font", IgnoreArg), ("-fn", IgnoreArg), ("-reverse", IgnoreFlag),
    ("-rv", IgnoreFlag), ("+rv", IgnoreFlag), ("-synchronous", IgnoreFlag),
    ("+synchronous", IgnoreFlag), ("-selectionTimeout", IgnoreArg),
    ("-xnllanguage", IgnoreArg), ("-xtsessionID", IgnoreArg), ("-xrm", IgnoreArg),
];

/// The result of reading the command line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parsed {
    /// The Fig file to open.
    pub file: Option<String>,
    /// `--control PORT`.
    pub control: Option<u16>,
    /// `--background`.
    pub background: bool,
    /// `-help`, `-h`, `--help`: print the usage and quit.
    pub help: bool,
    /// `-v`, `-V`, `--version`: print the version and quit.
    pub version: bool,
    /// `-update FILE...`: rewrite the files in the current format and quit.
    pub update: Option<Vec<String>>,
    /// Resource values for this run, in order (a later one wins).
    pub overrides: Vec<(String, String)>,
    /// Messages for stderr (ignored options, bad values).
    pub notes: Vec<String>,
}

/// The option `arg` names: an exact match, else the first that starts with it.
pub fn lookup(arg: &str) -> Option<&'static (&'static str, Action)> {
    if arg.len() < 2 {
        return None;
    }
    OPTIONS
        .iter()
        .find(|(n, _)| *n == arg)
        .or_else(|| OPTIONS.iter().find(|(n, _)| n.starts_with(arg)))
}

/// Read the arguments (without the program name). An unknown option, a
/// missing value or a second file name is an error, as in xfig.
pub fn parse<I, S>(args: I) -> Result<Parsed, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args: Vec<String> = args.into_iter().map(|a| a.as_ref().to_string()).collect();
    let mut p = Parsed::default();
    if args.first().is_some_and(|a| a.eq_ignore_ascii_case("-update")) {
        let (opts, files): (Vec<_>, Vec<_>) = args.iter().skip(1).partition(|a| a.starts_with('-'));
        for a in opts {
            p.notes.push(format!("-update: option {a} ignored"));
        }
        p.update = Some(files.into_iter().cloned().collect());
        return Ok(p);
    }
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-help" | "-h" | "--help" => p.help = true,
            "-V" | "--version" => p.version = true,
            "--background" => p.background = true,
            "--control" => {
                let v = it.next().ok_or("rustfig: --control needs a port")?;
                p.control = Some(v.parse().map_err(|_| format!("rustfig: bad control port {v:?}"))?);
            }
            _ if a.eq_ignore_ascii_case("-v") => p.version = true,
            _ if (a.starts_with('-') || a.starts_with('+')) && a.len() > 1 => {
                let Some((name, action)) = lookup(a) else {
                    return Err(format!("xfig: unknown option or missing parameter: {a}"));
                };
                let missing = || format!("xfig: unknown option or missing parameter: {a}");
                match *action {
                    Flag(r, v) => p.set(r, v.to_string(), name),
                    Arg(r) => {
                        let v = it.next().cloned().ok_or_else(missing)?;
                        p.set(r, v, name);
                    }
                    IgnoreFlag => p.notes.push(format!("{name}: X toolkit option, ignored")),
                    IgnoreArg => {
                        let v = it.next().cloned().ok_or_else(missing)?;
                        if *name == "-xrm" {
                            p.xrm(&v);
                        } else {
                            p.notes.push(format!("{name} {v}: X toolkit option, ignored"));
                        }
                    }
                }
            }
            _ => {
                if let Some(first) = &p.file {
                    return Err(format!("xfig: too many parameters: {a} (after {first})"));
                }
                p.file = Some(a.clone());
            }
        }
    }
    Ok(p)
}

impl Parsed {
    /// Record resource `name` = `value`, or say why it has no effect.
    fn set(&mut self, name: &str, value: String, option: &str) {
        if NOT_APPLICABLE.iter().any(|n| n.eq_ignore_ascii_case(name)) {
            self.notes.push(format!("{option}: no meaning in RustFig, ignored"));
            return;
        }
        if resource(name).is_none() {
            self.notes.push(format!("{option}: not supported yet, ignored"));
            return;
        }
        // Check the value's type the way the preferences do.
        if let Err(e) = Prefs::default().set(name, toml::Value::String(value.clone())) {
            self.notes.push(format!("{option}: {e}, ignored"));
            return;
        }
        self.overrides.push((name.to_string(), value));
    }

    /// `-xrm "Fig.name: value"`.
    fn xrm(&mut self, spec: &str) {
        match parse_resources(spec).pop() {
            Some((n, v)) => self.set(&n, v, "-xrm"),
            None => self.notes.push(format!("-xrm {spec}: not a Fig resource, ignored")),
        }
    }

    /// The overrides as a preferences table (`[xfig]` values, typed).
    pub fn override_table(&self) -> toml::Table {
        resource_table(&self.overrides)
    }
}

/// `(resource, text)` pairs as typed `[xfig]` values; pairs that don't fit
/// their resource are left out.
pub fn resource_table(pairs: &[(String, String)]) -> toml::Table {
    let mut p = Prefs::default();
    for (n, v) in pairs {
        let _ = p.set(n, toml::Value::String(v.clone())); // a wrong type: the pair is dropped
    }
    p.xfig
}

/// The `Fig.name: value` (also `xfig*name:`) lines of an X resource file
/// (`~/.Xdefaults` style) as (resource, value); comments (`!`) and other
/// applications' lines are skipped.
pub fn parse_resources(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('!') || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let name = ["Fig.", "Fig*", "xfig.", "xfig*", "*"]
            .iter()
            .find_map(|pre| key.strip_prefix(pre));
        if let Some(name) = name {
            if !name.contains(['.', '*']) && !name.is_empty() {
                out.push((name.to_string(), value.trim().to_string()));
            }
        }
    }
    out
}

/// The usage text (`-help`), as xfig prints it.
pub fn usage() -> String {
    let mut s = String::from("Usage:\nxfig [-h[elp]] [-v[ersion]]\n     ");
    let mut col = 5;
    for (n, a) in OPTIONS {
        let item = match a {
            Flag(..) | IgnoreFlag => format!("[{n}] "),
            _ => format!("[{n} <value>] "),
        };
        if col + item.len() > 85 {
            s.push_str("\n     ");
            col = 5;
        }
        col += item.len();
        s.push_str(&item);
    }
    s.push_str(
        "[-update file1 file2 ...] [--control PORT] [--background] [file]\n  Note: all options may be abbreviated to minimum unique string.\n",
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Parsed {
        parse(args.iter().copied()).unwrap()
    }

    fn has(p: &Parsed, n: &str, v: &str) -> bool {
        p.overrides.iter().any(|(a, b)| a == n && b == v)
    }

    #[test]
    fn every_option_is_accepted() {
        // Every table entry parses (a value is supplied where needed) and
        // either sets a resource or says why not; none is an error.
        for (name, action) in OPTIONS {
            let mut args = vec![name.to_string()];
            match action {
                Arg(r) => args.push(match resource(r).map(|r| r.kind) {
                    Some(crate::prefs::Kind::Str) | None => "x".into(),
                    _ => "3".into(),
                }),
                IgnoreArg => args.push("v".into()),
                _ => {}
            }
            let r = parse(&args).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(
                !r.overrides.is_empty() || !r.notes.is_empty(),
                "{name} has no effect and no note"
            );
        }
    }

    #[test]
    fn options_with_equivalents_set_resources() {
        assert!(has(&p(&["-metric"]), "inches", "false"));
        assert!(has(&p(&["-inches"]), "inches", "true"));
        assert!(has(&p(&["-imperial"]), "inches", "true"));
        assert!(has(&p(&["-centimeters"]), "inches", "false"));
        assert!(has(&p(&["-specialtext"]), "specialtext", "true"));
        assert!(has(&p(&["-latexfonts"]), "latexfonts", "true"));
        assert!(has(&p(&["-startfontsize", "16"]), "startfontsize", "16"));
        assert!(has(&p(&["-startgridmode", "2"]), "startgridmode", "2"));
        assert!(has(&p(&["-startposnmode", "0"]), "startposnmode", "0"));
        assert!(has(&p(&["-zoom", "1.5"]), "zoom", "1.5"));
        assert!(has(&p(&["-exportLanguage", "pdf"]), "exportLanguage", "pdf"));
        assert!(has(&p(&["-startarrowtype", "2"]), "startarrowtype", "2"));
        assert!(has(&p(&["-portrait"]), "landscape", "false"));
        assert!(has(&p(&["-Landscape"]), "landscape", "true"));
        assert!(has(&p(&["-right"]), "justify", "true"));
        assert!(has(&p(&["-flushleft"]), "flushleft", "true"));
        assert!(has(&p(&["-correct_font_size"]), "correctfontsize", "true"));
        assert!(has(&p(&["-paper_size", "A4"]), "paper_size", "A4"));
        assert!(has(&p(&["-nowrite_bak"]), "write_bak", "false"));
    }

    #[test]
    fn values_are_consumed_even_if_they_look_like_options() {
        let r = p(&["-startpsFont", "-metric", "a.fig"]);
        assert!(has(&r, "startpsFont", "-metric"));
        assert!(!has(&r, "inches", "false"));
        assert_eq!(r.file.as_deref(), Some("a.fig"));
    }

    #[test]
    fn abbreviations_follow_xt() {
        assert_eq!(lookup("-me").map(|o| o.0), Some("-metric"));
        assert_eq!(lookup("-spec").map(|o| o.0), Some("-specialtext"));
        assert_eq!(lookup("-startfo").map(|o| o.0), Some("-startfontsize"));
        assert_eq!(lookup("-lan").map(|o| o.0), Some("-landscape"));
        assert_eq!(
            lookup("-center").map(|o| o.0),
            Some("-center"),
            "exact wins over -centimeters"
        );
        assert_eq!(lookup("-ce").map(|o| o.0), Some("-center"), "first of the table");
        assert!(has(&p(&["-zo", "2"]), "zoom", "2"));
        assert!(has(&p(&["-startpo", "3"]), "startposnmode", "3"));
        assert!(lookup("-nosuchthing").is_none());
        assert!(lookup("-").is_none());
    }

    #[test]
    fn toolkit_options_are_accepted_and_noted() {
        let r = p(&[
            "-display",
            ":0",
            "-iconic",
            "-bg",
            "red",
            "+rv",
            "-geometry",
            "800x600+10+10",
            "x.fig",
        ]);
        assert_eq!(r.file.as_deref(), Some("x.fig"));
        assert!(has(&r, "geometry", "800x600+10+10"));
        assert_eq!(r.notes.len(), 4, "{:?}", r.notes);
        let r = p(&["-xrm", "Fig.startfontsize: 20"]);
        assert!(has(&r, "startfontsize", "20"));
    }

    #[test]
    fn options_without_meaning_are_noted_not_applied() {
        let r = p(&["-visual", "TrueColor", "-monochrome", "-debug", "-nosplash"]);
        assert!(r.overrides.is_empty());
        assert_eq!(r.notes.len(), 4);
    }

    #[test]
    fn errors_and_special_forms() {
        assert!(parse(["-bogus"]).is_err());
        assert!(parse(["-zoom"]).is_err(), "missing value");
        assert!(parse(["a.fig", "b.fig"]).is_err());
        assert!(parse(["--control", "x"]).is_err());
        assert!(p(&["-help"]).help && p(&["-h"]).help && p(&["-v"]).version && p(&["-V"]).version);
        let r = p(&["--control", "7986", "--background", "-metric", "f.fig"]);
        assert_eq!((r.control, r.background), (Some(7986), true));
        let r = p(&["-update", "a.fig", "b.fig"]);
        assert_eq!(r.update, Some(vec!["a.fig".into(), "b.fig".into()]));
        // A bad value is noted and dropped, not an error.
        let r = p(&["-zoom", "big"]);
        assert!(r.overrides.is_empty() && r.notes.len() == 1);
        assert_eq!(
            p(&["-metric"]).override_table().get("inches"),
            Some(&toml::Value::Boolean(false))
        );
        assert!(usage().contains("[-metric]"));
        // Hostile arguments don't panic.
        for a in ["", "-", "+", "--", "é", "-é", "\u{0}"] {
            let _ = parse([a]);
        }
    }

    #[test]
    fn resource_files_are_read() {
        let r = parse_resources("! c\nFig.inches: false\nxfig*startfontsize:  14\nOther.x: 1\nFig.a.b: 2\n*zoom: 2\n");
        assert_eq!(r.len(), 3);
        assert_eq!(r[0], ("inches".into(), "false".into()));
    }
}
