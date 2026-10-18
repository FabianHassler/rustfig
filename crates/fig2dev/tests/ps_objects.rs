#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Objects as genps draws them: hidden texts skipped, no arrowheads at
//! thickness 0 or on pie wedges, single-point polylines, polylines with
//! coincident ends closed, patterns filled with the nonzero rule.

use rustfig_fig2dev::{convert, drivers, read::readfp_fig, Ctx};

fn run(lang: &str, body: &str) -> String {
    let fig = format!("#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n{body}");
    let mut ctx = Ctx {
        lang: lang.into(),
        ..Ctx::default()
    };
    let mut dev = drivers::driver(lang).unwrap();
    ctx.font_size *= ctx.fontmag;
    let objects = readfp_fig(&mut ctx, fig.as_bytes()).unwrap();
    String::from_utf8_lossy(&convert(&mut ctx, dev.as_mut(), &objects).unwrap().output).into_owned()
}

#[test]
fn hidden_text_is_skipped_in_eps() {
    let body =
        "4 0 4 50 -1 0 20 0.0 12 300 900 600 600 secret\\001\n4 0 0 50 -1 0 20 0.0 4 300 900 600 1400 shown\\001\n";
    let eps = run("eps", body);
    assert!(!eps.contains("secret") && eps.contains("(shown)"));
}

#[test]
fn arrows_need_a_line_thickness_and_an_open_arc() {
    let arrow = "\t1 1 1.00 120.00 180.00\n";
    let line = |t: u32| format!("2 1 0 {t} 0 7 50 -1 -1 0.000 0 0 -1 1 0 2\n{arrow}\t 600 600 3000 600\n");
    let (thin, none) = (run("eps", &line(1)), run("eps", &line(0)));
    assert!(thin.matches("stroke").count() > none.matches("stroke").count());
    let pie =
        |kind: u32| format!("5 {kind} 0 3 0 7 50 -1 -1 0.000 0 0 1 0 1800 1800 1200 1800 1800 1200 2400 1800\n{arrow}");
    let (open, wedge) = (run("eps", &pie(1)), run("eps", &pie(2)));
    assert!(open.matches("stroke").count() > wedge.matches("stroke").count());
}

#[test]
fn single_point_polyline_is_a_segment_and_coincident_ends_close() {
    let dot = run("eps", "2 1 0 20 0 7 50 -1 -1 0.000 0 0 -1 0 0 1\n\t 600 600\n");
    assert!(dot.contains("stroke"), "{dot}");
    let open = "2 1 0 20 0 7 50 -1 -1 0.000 0 0 -1 0 0 4\n\t 600 600 2400 600 1500 2000 600 600\n";
    let closed = run("eps", open);
    let svg = run("svg", open);
    assert!(closed.contains("closepath") && !svg.contains(" Z"));
}

#[test]
fn patterns_fill_nonzero_in_eps_and_shades_even_odd() {
    let star = |fill: u32| {
        format!("2 3 0 2 0 4 50 -1 {fill} 0.000 0 0 -1 0 0 6\n\t 1800 600 2600 2800 600 1400 3000 1400 1000 2800 1800 600\n")
    };
    let pat = run("eps", &star(42));
    assert!(pat.contains("gsave fill grestore gsave clip") && !pat.contains("eoclip"));
    assert!(run("eps", &star(20)).contains("eofill"));
}
