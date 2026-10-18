#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `-G` grid (genps.c `genps_grid`, fig2dev.c `get_args`): drawn
//! by the PostScript, PDF and bitmap languages, not by SVG or the box.

use rustfig_fig2dev::{convert, drivers, read::readfp_fig, Ctx};

const FIG: &str = "#FIG 3.2\nLandscape\nFlush left\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 1200 1200 3600 1200 3600 2400 1200 2400 1200 1200\n";

/// The output and fig2dev's messages.
fn run(lang: &str, grid: Option<&str>) -> (Vec<u8>, Vec<String>) {
    let mut ctx = Ctx {
        lang: lang.into(),
        ..Ctx::default()
    };
    let mut dev = drivers::driver(lang).unwrap();
    if let Some(g) = grid {
        assert!(!ctx.general_option('G', Some(g)).unwrap());
        dev.option(&mut ctx, 'G', Some(g)).unwrap();
    }
    ctx.font_size *= ctx.fontmag;
    let objects = readfp_fig(&mut ctx, FIG.as_bytes()).unwrap();
    let out = convert(&mut ctx, dev.as_mut(), &objects).unwrap().output;
    (out, std::mem::take(&mut ctx.messages))
}

/// The number of grid lines (gray strokes) in an EPS.
fn gray_paths(eps: &[u8]) -> usize {
    String::from_utf8_lossy(eps)
        .matches("0.502 0.502 0.502 setrgbcolor")
        .count()
}

fn lines_of(eps: &[u8]) -> usize {
    // moveto commands: the box has 1, every grid line 1.
    String::from_utf8_lossy(eps)
        .lines()
        .filter(|l| l.ends_with(" m"))
        .count()
}

#[test]
fn drawn_where_genps_draws_it() {
    for lang in ["eps", "ps", "pdf", "pstex", "pdftex", "png", "gif", "ppm", "xpm"] {
        let (plain, _) = run(lang, None);
        let (grid, msgs) = run(lang, Some("0.25:1"));
        assert_ne!(plain, grid, "{lang}");
        assert!(msgs.is_empty(), "{lang}: {msgs:?}");
    }
    for lang in ["svg", "box"] {
        assert_eq!(run(lang, None).0, run(lang, Some("0.25:1")).0, "{lang}");
    }
}

#[test]
fn eps_grid_covers_the_figure_box() {
    let (plain, _) = run("eps", None);
    assert_eq!(gray_paths(&plain), 0);
    let (g, _) = run("eps", Some("0.5:1"));
    // 2 inches by 1 inch plus the box's own move: vertical lines every
    // 0.5 inch from 0 to 2, horizontal ones from 0 to 1 (each tick drawn
    // once, the majors thick).
    assert!(gray_paths(&g) >= 2);
    let n = lines_of(&g) - lines_of(&plain);
    assert!((10..=16).contains(&n), "{n}");
}

#[test]
fn units() {
    // "i" and "in" are inches; "inch" never sets C's multiplier: no grid.
    let base = run("eps", None).0;
    for (spec, drawn) in [
        ("1", true),
        ("1i", true),
        ("1in", true),
        ("1inch", false),
        ("1:2in", true),
        ("5mm:1cm", true),
        ("0.5ft", true),
        ("0.01m", true),
        ("1/4:1", true),
        ("0:1", true),
        ("1:0", true),
        ("0", false),
        ("0:0", false),
        ("-1", false),
    ] {
        let (o, _) = run("eps", Some(spec));
        assert_eq!(o != base, drawn, "-G {spec}");
    }
}

#[test]
fn bad_specs_warn_and_draw_nothing() {
    let base = run("eps", None).0;
    for spec in ["1xyz", "abc", "1:2furlong", "", ":"] {
        let (o, msgs) = run("eps", Some(spec));
        assert_eq!(o, base, "-G {spec:?}");
        assert!(
            msgs.iter().any(|m| m.contains("Can't parse grid spec")),
            "-G {spec:?}: {msgs:?}"
        );
    }
}

#[test]
fn hostile_spacings_do_not_hang() {
    for spec in [
        "1e-30",
        "0.000001",
        "1e38",
        "1e400",
        "nan",
        "inf",
        "1/1e-30",
        "1e-30:1e-30m",
    ] {
        for lang in ["eps", "ps", "png"] {
            let _ = run(lang, Some(spec));
        }
    }
    // Too fine for the figure: no grid, one message.
    let (o, msgs) = run("eps", Some("0.00001"));
    assert_eq!(gray_paths(&o), 0);
    assert!(msgs.iter().any(|m| m.contains("grid spacing is too small")), "{msgs:?}");
}
