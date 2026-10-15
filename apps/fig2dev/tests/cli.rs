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
