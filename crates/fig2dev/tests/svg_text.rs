#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! SVG text as gensvg.c writes it: Symbol and ZapfDingbats through the
//! Unicode tables, special text as `tspan`s, and `-z`.

use rustfig_fig2dev::{convert, drivers, read::readfp_fig, Ctx};

fn run(font: i32, flags: i32, text: &str, extra: &[(char, Option<&str>)]) -> Result<String, String> {
    let fig = format!(
        "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
4 0 0 50 -1 {font} 12 0.0 {flags} 150 900 600 600 {text}\\001\n"
    );
    let mut ctx = Ctx {
        lang: "svg".into(),
        ..Ctx::default()
    };
    let mut dev = drivers::driver("svg").unwrap();
    for &(c, a) in extra {
        if !ctx.general_option(c, a)? {
            dev.option(&mut ctx, c, a)?;
        }
    }
    ctx.font_size *= ctx.fontmag;
    let objects = readfp_fig(&mut ctx, fig.as_bytes()).map_err(|e| e.to_string())?;
    let out = convert(&mut ctx, dev.as_mut(), &objects)?.output;
    Ok(String::from_utf8(out).unwrap())
}

/// What lies between `<text ...>` and `</text>`.
fn body(svg: &str) -> &str {
    let a = svg.find("<text ").unwrap();
    let a = a + svg[a..].find('>').unwrap() + 1;
    &svg[a..svg.find("</text>").unwrap()]
}

#[test]
fn symbol_codes_become_unicode() {
    let s = run(32, 4, "abAB\\242", &[]).unwrap();
    // alpha beta Alpha Beta; 0242 is the prime.
    assert_eq!(body(&s), "&#945;&#946;&#913;&#914;&#8242;");
}

#[test]
fn dingbat_codes_become_unicode() {
    let s = run(34, 4, "ab", &[]).unwrap();
    assert_eq!(body(&s), "&#10049;&#10050;");
}

#[test]
fn plain_text_is_unchanged() {
    let s = run(0, 4, "a^2 <b> &", &[]).unwrap();
    assert_eq!(body(&s), "a^2 &lt;b&gt; &amp;");
}

#[test]
fn special_text_shifts_with_tspans() {
    let s = run(0, 6, "x^2 y_{ab}z$q$", &[]).unwrap();
    assert_eq!(
        body(&s),
        "x<tspan font-size=\"96\" baseline-shift=\"super\">2</tspan> y<tspan font-size=\"96\" baseline-shift=\"sub\">ab</tspan>zq"
    );
}

#[test]
fn special_text_edge_cases_stay_well_formed() {
    // C leaves the tspan open at the end of the text, and writes a NUL
    // after a final `}`; here both are closed.
    let s = run(0, 6, "e^b", &[]).unwrap();
    assert_eq!(body(&s), "e<tspan font-size=\"96\" baseline-shift=\"super\">b</tspan>");
    let s = run(0, 6, "e^{b}", &[]).unwrap();
    assert_eq!(body(&s), "e<tspan font-size=\"96\" baseline-shift=\"super\">b</tspan>");
    // A trailing `^` is dropped; nesting closes at the innermost `}`.
    let s = run(0, 6, "d^", &[]).unwrap();
    assert_eq!(body(&s), "d");
    let s = run(0, 6, "a^{b_{c}}d", &[]).unwrap();
    assert_eq!(
        body(&s),
        "a<tspan font-size=\"96\" baseline-shift=\"super\">b<tspan font-size=\"96\" baseline-shift=\"sub\">c</tspan>}d</tspan>"
    );
}

#[test]
fn paper_size_sets_width_and_height() {
    let s = run(0, 4, "x", &[('z', Some("A4"))]).unwrap();
    // The figure is landscape: 842 x 595 points, viewBox kept.
    assert!(s.contains("width=\"842pt\" height=\"595pt\""), "{s}");
    let e = run(0, 4, "x", &[('z', Some("nosuch"))]).unwrap_err();
    assert_eq!(e, "Unknown paper size `nosuch'");
}
