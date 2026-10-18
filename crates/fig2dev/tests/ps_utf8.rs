#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Text beyond Latin-1 in PostScript (genps.c: Xfig-UTF8-H CMap, `composefont`
//! `-UTF-8` fonts) and PDF (Type 3 fonts with a ToUnicode map).

use rustfig_fig2dev::{convert, drivers, read::readfp_fig, Ctx};

fn run(lang: &str, text: &str, extra: &[(char, Option<&str>)]) -> Vec<u8> {
    let fig = format!(
        "#FIG 3.2\n#encoding: UTF-8\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
4 0 0 50 -1 0 14 0.0 4 150 900 600 600 {text}\\001\n"
    );
    let mut ctx = Ctx {
        lang: lang.into(),
        ..Ctx::default()
    };
    let mut dev = drivers::driver(lang).unwrap();
    for &(c, a) in extra {
        if !ctx.general_option(c, a).unwrap() {
            dev.option(&mut ctx, c, a).unwrap();
        }
    }
    ctx.font_size *= ctx.fontmag;
    let objects = readfp_fig(&mut ctx, fig.as_bytes()).unwrap();
    convert(&mut ctx, dev.as_mut(), &objects).unwrap().output
}

#[test]
fn greek_in_eps_uses_the_cmap_and_a_composite_font() {
    let s = String::from_utf8(run("eps", "αβγ (x)", &[])).unwrap();
    assert!(s.contains("%%DocumentNeededResources: ProcSet (CIDInit)\n%%+ CMap (Xfig-UTF8-H)\n"));
    assert!(s.contains("/CMapName /Xfig-AGL-UTF8-H def"));
    assert!(s.contains("/Times-Roman-UTF-8\n/Xfig-AGL-UTF8-H /CMap findresource\n[/Times-Roman ] composefont pop\n"));
    assert!(s.contains("/Times-Roman-UTF-8 findfont"));
    assert!(s.contains("(αβγ \\(x\\)) show"));
}

#[test]
fn latin1_text_in_eps_stays_on_the_iso_fonts() {
    let s = String::from_utf8(run("eps", "café", &[])).unwrap();
    assert!(!s.contains("CMap") && !s.contains("composefont"));
    assert!(s.contains("(caf\\351) show"));
}

#[test]
fn greek_in_pdf_is_a_type3_font_with_unicode_map() {
    let b = run("pdf", "αβγ", &[]);
    let s = String::from_utf8_lossy(&b);
    assert!(s.contains("/Subtype /Type3") && s.contains("/ToUnicode") && s.contains("/uni03B1"));
    let l = run("pdf", "café", &[]);
    assert!(!String::from_utf8_lossy(&l).contains("Type3"));
}
