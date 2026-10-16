//! B10: the standalone fig2dev's command line, messages and exit codes,
//! as fig2dev 3.2.9a's `fig2dev.c` has them.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const FIG: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 1200 0 1200 600 0 600 0 0\n";

fn fig2dev(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_fig2dev"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn bbox(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .find(|l| l.starts_with("%%BoundingBox:"))
        .unwrap_or_default()
        .to_string()
}

#[test]
fn eps_from_stdin_to_stdout() {
    let o = fig2dev(&["-L", "eps"], FIG);
    assert!(o.status.success(), "{}", err(&o));
    // As fig2dev 3.2.9a: 1 inch × 0.5 inch plus the line width, rounded out.
    assert_eq!(bbox(&o), "%%BoundingBox: 0 0 74 38");
    let o = fig2dev(&["-Leps", "-m", "2"], FIG);
    assert_eq!(bbox(&o), "%%BoundingBox: 0 0 147 75");
    let o = fig2dev(&["-L", "eps", "-b", "10"], FIG);
    assert_eq!(bbox(&o), "%%BoundingBox: 0 0 94 58");
}

#[test]
fn language_from_the_output_suffix() {
    let dir = std::env::temp_dir().join(format!("rustfig-fig2dev-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("in.fig");
    std::fs::write(&input, FIG).unwrap();
    for (name, magic) in [
        ("out.svg", &b"<?xml"[..]),
        ("out.pdf", b"%PDF"),
        ("out.png", b"\x89PNG"),
        ("out.gif", b"GIF8"),
        ("out.jpg", &[0xff, 0xd8]),
        ("out.jpeg", &[0xff, 0xd8]),
        ("out.pcx", &[0x0a, 5]),
        ("out.ppm", b"P6"),
        ("out.tif", b"II*\0"),
        ("out.tiff", b"II*\0"),
        ("out.xbm", b"#define"),
        ("out.xpm", b"/* XPM */"),
    ] {
        let out: PathBuf = dir.join(name);
        let o = fig2dev(&[input.to_str().unwrap(), out.to_str().unwrap()], "");
        assert!(o.status.success(), "{name}: {}", err(&o));
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.starts_with(magic), "{name}");
    }
    let o = fig2dev(&[input.to_str().unwrap(), dir.join("x.fig").to_str().unwrap()], "");
    assert_eq!(o.status.code(), Some(1));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn fig2devs_messages_and_exit_codes() {
    let o = fig2dev(&["-L", "nonsense"], FIG);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("Unknown graphics language nonsense"), "{}", err(&o));

    let o = fig2dev(&["-"], FIG);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("No graphics language specified."), "{}", err(&o));

    let o = fig2dev(&["-L", "eps", "-m", "2", "-Z", "4"], FIG);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("Must specify only one of -m"), "{}", err(&o));

    // svg doesn't know -n: fig2dev exits; bitmaps only warn.
    let o = fig2dev(&["-L", "svg", "-n", "x"], FIG);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("Argument -n unknown to svg driver."), "{}", err(&o));
    let o = fig2dev(&["-L", "png", "-n", "x"], FIG);
    assert!(o.status.success(), "{}", err(&o));

    let o = fig2dev(&["-V"], "");
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).starts_with("fig2dev Version 3.2.9a"));
    let o = fig2dev(&["--version"], "");
    assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), "fig2dev Version 3.2.9a");
}

/// Every option fig2dev's PostScript driver takes parses for eps, with
/// fig2dev's argument forms; options that change nothing for EPS (page
/// layout) are accepted silently.
#[test]
fn every_postscript_option_parses() {
    for args in [
        &["-F"][..],
        &["-j"],
        &["-K"],
        &["-M"],
        &["-N"],
        &["-O"],
        &["-P"],
        &["-c"],
        &["-e"],
        &["-l", "x"],
        &["-p", "x"],
        &["-z", "A4"],
        &["-x", "10"],
        &["-y", "-10"],
        &["-n", "name"],
        &["-g", "light blue"],
        &["-b", "5"],
        &["-B", "1 1 0 0"],
        &["-R", "1 1 0 0"],
        &["-f", "Times-Roman"],
        &["-D", "+40,50:60"],
        &["-D", "-0:10"],
        &["-Z", "4w"],
        &["-s", "12"],
        &["-E", "1"],
        &["-G", ".25:1cm"],
    ] {
        let mut a = vec!["-L", "eps"];
        a.extend_from_slice(args);
        let o = fig2dev(&a, FIG);
        assert!(o.status.success(), "{args:?}: {}", err(&o));
        assert!(!err(&o).contains("unknown"), "{args:?}: {}", err(&o));
        assert!(bbox(&o).starts_with("%%BoundingBox:"), "{args:?}");
    }
}

/// genbitmaps' options: size at 80 dpi times the magnification, -b in
/// pixels, -S, -q and -t, with fig2dev's warnings and errors.
#[test]
fn bitmap_options() {
    let ppm_size = |args: &[&str]| -> (u32, u32) {
        let mut a = vec!["-L", "ppm"];
        a.extend_from_slice(args);
        let o = fig2dev(&a, FIG);
        assert!(o.status.success(), "{args:?}: {}", err(&o));
        let text = String::from_utf8_lossy(o.stdout.get(..20).unwrap_or(&o.stdout)).into_owned();
        let v: Vec<u32> = text
            .split_whitespace()
            .skip(1)
            .take(2)
            .map(|n| n.parse().unwrap())
            .collect();
        (v[0], v[1])
    };
    // As fig2dev 3.2.9a with ghostscript: 1 inch × 0.5 inch plus the line
    // width and (int)(size + 0.9) rounding.
    assert_eq!(ppm_size(&[]), (82, 42));
    assert_eq!(ppm_size(&["-b", "5"]), (92, 52));
    assert_eq!(ppm_size(&["-m", "2"]).0, 164);
    // Without -S the edges are hard: only black and white.
    let ppm = |args: &[&str]| {
        let mut a = vec!["-L", "ppm"];
        a.extend_from_slice(args);
        fig2dev(&a, FIG).stdout
    };
    let grey = |b: &[u8]| b.iter().skip(13).any(|v| *v != 0 && *v != 255);
    assert!(!grey(&ppm(&[])));
    assert!(grey(&ppm(&["-S", "4"])));
    let o = fig2dev(&["-L", "ppm", "-S", "3"], FIG);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("should be 1, 2 or 4"), "{}", err(&o));
    // -q and -t outside JPEG and GIF: warned about and ignored.
    let o = fig2dev(&["-L", "png", "-q", "50", "-t", "#ffffff"], FIG);
    assert!(o.status.success());
    assert!(
        err(&o).contains("-q option only allowed for jpeg quality; ignored"),
        "{}",
        err(&o)
    );
    assert!(
        err(&o).contains("-t option only allowed for GIF transparent color; ignored"),
        "{}",
        err(&o)
    );
    let o = fig2dev(&["-L", "jpeg", "-q", "30"], FIG);
    assert!(o.status.success() && o.stdout.starts_with(&[0xff, 0xd8]), "{}", err(&o));
    let o = fig2dev(&["-L", "gif", "-t", "#ffffff"], FIG);
    assert!(o.status.success() && o.stdout.starts_with(b"GIF8"), "{}", err(&o));
}

fn help(args: &[&str]) -> String {
    let o = fig2dev(args, "");
    assert!(o.status.success(), "{}", err(&o));
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn help_lists_only_effective_options() {
    let all = help(&["-h"]);
    assert!(all.contains("Available languages are:") && all.contains(" mp"), "{all}");
    assert!(all.contains("Use fig2dev -L language -h"), "{all}");
    assert!(!all.contains(" Options:"), "no language sections: {all}");
    assert!(!all.contains("METAPOST") && !all.contains("EPS Options"), "{all}");

    let eps = help(&["-L", "eps", "-h"]);
    assert!(eps.contains("EPS Options:") && eps.contains("  -b num"), "{eps}");
    assert!(eps.contains("  -D +/-list"), "{eps}");
    for page_only in ["  -P", "  -c ", "  -z ", "  -M ", "  -G "] {
        assert!(!eps.contains(page_only), "eps lists {page_only}: {eps}");
    }

    let mp = help(&["-L", "mp", "-h"]);
    assert!(mp.contains("METAPOST Options:"), "{mp}");
    for line in [
        "  -d file     add file content",
        "  -i file",
        "  -M          multipage",
        "  -o ",
        "  -p number",
    ] {
        assert!(mp.contains(line), "mp lacks {line}: {mp}");
    }
    assert!(!mp.contains("EPS Options"), "{mp}");

    let epic = help(&["-L", "eepic", "-h"]);
    assert!(
        epic.contains("EPIC, EEPIC and EEPICEMU Options:") && epic.contains("  -S scale"),
        "{epic}"
    );

    let tikz = help(&["-L", "tikz", "-h"]);
    assert!(tikz.contains("not available in RustFig's fig2dev yet"), "{tikz}");
    assert!(!tikz.contains("  -D "), "{tikz}");

    for out in [&all, &eps, &mp, &epic, &tikz] {
        assert!(
            !out.contains("no effect here") && !out.contains("not in RustFig yet"),
            "{out}"
        );
    }
}

/// A figure with a PostScript text and a special (LaTeX) text.
const TEXTS: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 2400 0 2400 1200 0 1200 0 0\n\
4 0 0 50 -1 0 12 0.0000 4 135 450 300 600 plain\\001\n\
4 0 0 50 -1 0 12 0.0000 2 135 450 300 900 $x^2$\\001\n";

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn pstex_and_pstex_t_split_the_texts() {
    // B2: pstex is EPS without special texts, pstex_t the LaTeX overlay.
    for (graphics, overlay, magic) in [("pstex", "pstex_t", "%!PS-Adobe"), ("pdftex", "pdftex_t", "%PDF")] {
        let g = fig2dev(&["-L", graphics], TEXTS);
        assert!(g.status.success(), "{}", err(&g));
        assert!(g.stdout.starts_with(magic.as_bytes()), "{graphics}");
        if graphics == "pstex" {
            assert!(out(&g).contains("(plain)") && !out(&g).contains("x^2"), "{}", out(&g));
            assert_eq!(bbox(&g), "%%BoundingBox: 0 0 146 74", "special texts count in the box");
        }
        let t = fig2dev(&["-L", overlay, "-p", "fig.eps"], TEXTS);
        assert!(t.status.success(), "{}", err(&t));
        let tex = out(&t);
        // As fig2dev 3.2.9a writes it.
        assert!(
            tex.starts_with("\\begin{picture}(0,0)%\n\\includegraphics{fig.eps}%\n\\end{picture}%\n"),
            "{tex}"
        );
        assert!(tex.contains("\\begin{picture}(2424,1224)(-11,-373)"), "{tex}");
        assert!(tex.contains("$x^2$") && !tex.contains("plain"), "{tex}");
    }
    // pstex takes the EPS options (genps), pstex_t the latex ones.
    assert!(fig2dev(&["-L", "pstex", "-b", "10"], TEXTS).status.success());
    let o = fig2dev(&["-L", "pstex_t", "-N"], TEXTS);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("Argument -N unknown to latex driver."), "{}", err(&o));
}

#[test]
fn old_formats_render_through_fig2devs_reader() {
    // Fig 2.1 at 80 ppi; Fig 1.3; a file without a final newline.
    let fig21 = "#FIG 2.1\n80 2\n2 2 0 1 -1 0 0 0 0.000 7 0 0\n\t 10 10 90 10 90 50 10 50 10 10 9999 9999\n";
    let o = fig2dev(&["-L", "eps"], fig21);
    assert!(o.status.success(), "{}", err(&o));
    // As fig2dev 3.2.9a: the bounds pad the box by its line width.
    assert_eq!(bbox(&o), "%%BoundingBox: 0 0 76 40");
    let o = fig2dev(&["-L", "svg"], "80 2 9 9\n3 10 0 1 1 0 0 0 0 0 0\n10 10 40 0 9999 0\n");
    assert!(o.status.success(), "{}", err(&o));
    assert!(out(&o).contains("<svg"), "{}", out(&o));
    let o = fig2dev(&["-L", "box"], FIG.trim_end_matches('\n'));
    assert!(o.status.success(), "{}", err(&o));
    // read.c's repairs are reported, and the figure is still drawn.
    let open = "#FIG 2.1\n80 2\n2 2 0 1 -1 0 0 0 0.000 7 0 0\n\t 10 10 90 10 90 50 10 50 9999 9999\n";
    let o = fig2dev(&["-L", "png"], open);
    assert!(o.status.success(), "{}", err(&o));
    assert!(err(&o).contains("An open rectangle at line"), "{}", err(&o));
}

#[test]
fn picture_messages_are_fig2devs() {
    let fig = format!(
        "{}2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5\n\t0 no-such.png\n\t 0 0 1200 0 1200 600 0 600 0 0\n",
        FIG
    );
    let o = fig2dev(&["-L", "eps"], &fig);
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(err(&o), "No such picture file: no-such.png\n");
}

/// A scratch directory for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustfig-fig2dev-cli-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const USAGE: &str =
    "Usage:\n fig2dev -hV\n fig2dev -L language -h\n fig2dev [-L language] [other options] [in.fig [output]]\n";

/// fig2dev.c's `get_args`: one pass in command-line order; getopt's
/// message is followed by the usage, without a blank line.
#[test]
fn options_are_handled_in_command_line_order() {
    // -D comes before the unknown option, so its message wins.
    let o = fig2dev(&["-L", "eps", "-D", "x", "-u"], FIG);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).starts_with("fig2dev: help for -D option:\n"), "{}", err(&o));
    let o = fig2dev(&["-L", "eps", "-K", "-u"], FIG);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(err(&o), format!("fig2dev: invalid option -- 'u'\n{USAGE}"));
    let o = fig2dev(&["-L", "eps", "-b"], FIG);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(err(&o), format!("fig2dev: option requires an argument -- 'b'\n{USAGE}"));
    // Driver options are handled before the figure is read.
    let o = fig2dev(&["-L", "svg", "-n", "x", "/no/such.fig"], "");
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(err(&o), "Argument -n unknown to svg driver.\n");
    let o = fig2dev(&["-L", "cgm", "-N", "/no/such.fig"], "");
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(err(&o), "Argument -N unknown to cgm driver.\n");
    // -L keeps 11 characters.
    let o = fig2dev(&["-L", "abcdefghijklmnop"], "");
    assert_eq!(o.status.code(), Some(1));
    assert!(
        err(&o).starts_with("Unknown graphics language abcdefghijk\nKnown languages are:\nbox cgm "),
        "{}",
        err(&o)
    );
}

/// -E takes any name (fig2dev hands it to iconv); -Z, -m and -s read a
/// number prefix as atof/strtod do; -s 0 is 11 points.
#[test]
fn general_option_values_as_fig2dev() {
    for enc in ["KOI8R", "1", "utf8", "whatever"] {
        let o = fig2dev(&["-L", "eps", "-E", enc], FIG);
        assert!(o.status.success(), "{enc}: {}", err(&o));
        assert_eq!(err(&o), "", "{enc}");
    }
    let o = fig2dev(&["-L", "eps", "-m", "2x"], FIG);
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(bbox(&o), "%%BoundingBox: 0 0 147 75");
    let z = |arg: &str| fig2dev(&["-L", "eps", "-Z", arg], FIG);
    assert_eq!(bbox(&z("2in")), bbox(&z("2")));
    assert_eq!(bbox(&z("2w")), "%%BoundingBox: 0 0 144 74");
    // No number: strtod gives 0 without an error, so -Z still counts.
    let o = z("x");
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(err(&o), "");
    let o = fig2dev(&["-L", "eps", "-Z", "x", "-m", "2"], FIG);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(
        err(&o),
        "Must specify only one of -m (magnification) and -Z (max dimension).\n"
    );
    // Infinity or overflow: fig2dev's message (without a newline), and -Z
    // is dropped, so -m may follow.
    let o = fig2dev(&["-L", "eps", "-Z", "inf", "-m", "2"], FIG);
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(err(&o), "Unable to parse dimension specification (-Z inf)");
    assert_eq!(bbox(&o), "%%BoundingBox: 0 0 147 75");
    let o = z("1e999");
    assert_eq!(err(&o), "Unable to parse dimension specification (-Z 1e999)");
    // -s: atof, and 11 points for zero or less (read.c rescales the texts).
    let text = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
4 0 0 50 -1 0 12 0.0000 4 135 450 300 600 plain\\001\n";
    let size = |args: &[&str]| {
        let mut a = vec!["-L", "svg"];
        a.extend_from_slice(args);
        let o = fig2dev(&a, text);
        assert!(o.status.success(), "{args:?}: {}", err(&o));
        let svg = out(&o);
        let at = svg.find("font-size=\"").unwrap() + 11;
        svg[at..].split('"').next().unwrap().to_string()
    };
    assert_eq!(size(&[]), "200");
    assert_eq!(size(&["-s", "0"]), "183.33");
    assert_eq!(size(&["-s", "-3"]), "183.33");
    assert_eq!(size(&["-s", "12pt"]), "200");
}

/// depth_option: junk fills every slot ("Too many -D values!"); "-D +"
/// filters nothing.
#[test]
fn depth_option_as_fig2dev() {
    for junk in ["+a", "+10x", "+5;6"] {
        let o = fig2dev(&["-L", "eps", "-D", junk], FIG);
        assert_eq!(o.status.code(), Some(1), "{junk}");
        assert_eq!(err(&o), "fig2dev: Too many -D values!\n", "{junk}");
    }
    let o = fig2dev(&["-L", "eps", "-D", "+"], FIG);
    assert!(o.status.success(), "{}", err(&o));
    assert_eq!(bbox(&o), "%%BoundingBox: 0 0 74 38");
    let o = fig2dev(&["-L", "mp", "-D", "+"], FIG);
    assert!(o.status.success(), "{}", err(&o));
}

/// read_fig's statuses and messages (fig2dev.c `main`, messages.c
/// `err_msg` with its strerror suffix).
#[test]
fn read_messages_as_fig2dev() {
    let dir = scratch("read");
    let missing = dir.join("missing.fig");
    let missing = missing.to_str().unwrap();
    let o = fig2dev(&["-L", "eps", missing], "");
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(
        err(&o),
        format!("File \"{missing}\" is not accessible: No such file or directory\n")
    );
    let d = dir.to_str().unwrap();
    for lang in ["eps", "mp"] {
        let o = fig2dev(&["-L", lang, d], "");
        assert_eq!(o.status.code(), Some(1));
        assert_eq!(err(&o), format!("Not a regular file: {d}\n"));
    }
    // Status -3 from the Fig 1.3 reader: errno is 0 then.
    let fig13 = "80 2 9 9\n3 10 0 1 1 0 0 0 0 0 0\n10 10 40 0 9999 0\nxyz\n";
    let file = dir.join("bad13.fig");
    std::fs::write(&file, fig13).unwrap();
    let f = file.to_str().unwrap();
    let o = fig2dev(&["-L", "eps", f], "");
    assert_eq!(o.status.code(), Some(1));
    assert!(
        err(&o).starts_with(&format!("File \"{f}\" is not accessible: ")),
        "{}",
        err(&o)
    );
    let o = fig2dev(&["-L", "eps"], fig13);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).starts_with("Input error: "), "{}", err(&o));
    assert_eq!(err(&o).lines().count(), 1, "{}", err(&o));
    std::fs::remove_dir_all(&dir).ok();
}

/// An empty figure: "No objects in Fig file", exit(-1), and the output
/// file fig2dev opened before converting stays empty.
#[test]
fn empty_figure_as_fig2dev() {
    let empty = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n";
    let dir = scratch("empty");
    let input = dir.join("empty.fig");
    std::fs::write(&input, empty).unwrap();
    for name in ["out.eps", "out.png", "out.svg", "out.mp"] {
        let output = dir.join(name);
        std::fs::write(&output, "old").unwrap();
        let o = fig2dev(&[input.to_str().unwrap(), output.to_str().unwrap()], "");
        assert_eq!(o.status.code(), Some(255), "{name}");
        assert_eq!(err(&o), "fig2dev: No objects in Fig file\n", "{name}");
        assert_eq!(std::fs::read(&output).unwrap(), b"", "{name}");
    }
    let o = fig2dev(&["-L", "pdf"], empty);
    assert_eq!(o.status.code(), Some(255));
    assert!(o.stdout.is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

/// genps_option and genbitmaps_option warn about a colour they can't
/// parse and go on without a background.
#[test]
fn bad_background_colour_warns() {
    for lang in ["eps", "pdf", "png"] {
        for colour in ["#144ea0f90c0", "#30g", "no such colour"] {
            let o = fig2dev(&["-L", lang, "-g", colour], FIG);
            assert!(o.status.success(), "{lang} {colour}: {}", err(&o));
            assert_eq!(
                err(&o),
                format!("Can't parse color '{colour}', ignoring background option\n")
            );
        }
    }
    let plain = fig2dev(&["-L", "eps"], FIG);
    let bad = fig2dev(&["-L", "eps", "-g", "#30g"], FIG);
    assert_eq!(plain.stdout, bad.stdout);
}

/// The language guessed from the output suffix reaches the driver as -L
/// does (genmp lists it among its options).
#[test]
fn guessed_language_is_passed_to_the_driver() {
    let dir = scratch("guess");
    let input = dir.join("in.fig");
    std::fs::write(&input, FIG).unwrap();
    let guessed = dir.join("a.mp");
    let o = fig2dev(&[input.to_str().unwrap(), guessed.to_str().unwrap()], "");
    assert!(o.status.success(), "{}", err(&o));
    let explicit = fig2dev(&["-L", "mp", input.to_str().unwrap()], "");
    let guessed = std::fs::read_to_string(&guessed).unwrap();
    assert!(guessed.contains("%  -L mp"), "{guessed}");
    assert_eq!(guessed, out(&explicit));
    std::fs::remove_dir_all(&dir).ok();
}
