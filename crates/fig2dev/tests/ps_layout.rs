#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The page layout of fig2dev's `ps` language and of `pdf` in page mode
//! (genps.c, genpdf.c): paper sizes, orientation, centring, offsets, pages.

use rustfig_fig2dev::{convert, drivers, read::readfp_fig, Ctx};

const BOX_AT: &str = "2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 1200 1200 3600 1200 3600 2400 1200 2400 1200 1200\n";

fn fig(header_orient: &str, center: &str, paper: &str, multi: &str, body: &str) -> String {
    format!("#FIG 3.2\n{header_orient}\n{center}\nInches\n{paper}\n100.00\n{multi}\n-2\n1200 2\n{body}")
}

fn run(lang: &str, fig: &str, opts: &[(char, Option<&str>)]) -> Result<Vec<u8>, String> {
    let mut ctx = Ctx {
        lang: lang.into(),
        ..Ctx::default()
    };
    let mut dev = drivers::driver(lang).unwrap();
    for &(c, a) in opts {
        if !ctx.general_option(c, a)? {
            dev.option(&mut ctx, c, a)?;
        }
    }
    ctx.font_size *= ctx.fontmag;
    let objects = readfp_fig(&mut ctx, fig.as_bytes()).map_err(|_| "read".to_string())?;
    Ok(convert(&mut ctx, dev.as_mut(), &objects)?.output)
}

fn ps(f: &str, opts: &[(char, Option<&str>)]) -> String {
    String::from_utf8(run("ps", f, opts).unwrap()).unwrap()
}

fn line<'a>(s: &'a str, prefix: &str) -> &'a str {
    s.lines().find(|l| l.starts_with(prefix)).unwrap_or("")
}

/// The first `translate` after the first `%%Page:` line.
fn translate(s: &str) -> String {
    let after = &s[s.find("%%Page: 1").unwrap()..];
    let l = after.lines().find(|l| l.ends_with("translate")).unwrap();
    l.trim_end_matches(" translate").to_string()
}

fn landscape_letter() -> String {
    fig("Landscape", "Center", "Letter", "Single", BOX_AT)
}

#[test]
fn paper_from_the_file_and_from_z() {
    let f = fig("Portrait", "Flush Left", "A4", "Single", BOX_AT);
    let s = ps(&f, &[]);
    assert_eq!(line(&s, "%%BoundingBox"), "%%BoundingBox: 0 0 595 842");
    assert_eq!(line(&s, "%%DocumentPaperSizes"), "%%DocumentPaperSizes: a4");
    assert_eq!(line(&s, "%%Pages"), "%%Pages: 1");
    assert!(s.contains("<</PageSize [595 842]>> setpagedevice"));
    let s = ps(&f, &[('z', Some("legal"))]);
    assert_eq!(line(&s, "%%BoundingBox"), "%%BoundingBox: 0 0 612 1008");
    assert!(s.contains("%%BeginFeature: *PageSize Legal"));
    let s = ps(&f, &[('z', Some("ledger"))]);
    assert_eq!(line(&s, "%%BoundingBox"), "%%BoundingBox: 0 0 792 1224");
}

#[test]
fn orientation_from_the_file_and_options() {
    let s = ps(&landscape_letter(), &[]);
    assert_eq!(line(&s, "%%BoundingBox"), "%%BoundingBox: 0 0 792 612");
    let s = ps(&landscape_letter(), &[('p', Some("x"))]);
    assert_eq!(line(&s, "%%BoundingBox"), "%%BoundingBox: 0 0 612 792");
    let f = fig("Portrait", "Center", "Letter", "Single", BOX_AT);
    let s = ps(&f, &[('l', Some("x"))]);
    assert_eq!(line(&s, "%%BoundingBox"), "%%BoundingBox: 0 0 792 612");
}

#[test]
fn centre_edge_and_offsets() {
    // The box is 2400 x 1200 units = 144 x 72 points at (72, 72) from the
    // top left; letter landscape is 792 x 612 points.
    let flush = fig("Landscape", "Flush Left", "Letter", "Single", BOX_AT);
    // Flush left: the figure's bounds (with the line's width) keep their place.
    let e = translate(&ps(&flush, &[]));
    let c = translate(&ps(&flush, &[('c', None)]));
    assert_ne!(e, c);
    let centred = translate(&ps(&landscape_letter(), &[]));
    assert_eq!(c, centred);
    let edge = translate(&ps(&landscape_letter(), &[('e', None)]));
    assert_eq!(edge, e);
    // Offsets move the figure by points.
    let (ex, ey) = e.split_once(' ').unwrap();
    let moved = translate(&ps(&flush, &[('x', Some("10")), ('y', Some("-5"))]));
    let (mx, my) = moved.split_once(' ').unwrap();
    let d = |a: &str, b: &str| a.parse::<f64>().unwrap() - b.parse::<f64>().unwrap();
    assert!((d(mx, ex) - 10.0).abs() < 0.01 && (d(my, ey) + 5.0).abs() < 0.01);
}

#[test]
fn multiple_pages() {
    let big = "2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 24000 0 24000 14400 0 14400 0 0\n";
    let f = fig("Portrait", "Flush Left", "A4", "Multiple", big);
    let s = ps(&f, &[]);
    // 1200 x 720 points on 595 x 842 pages.
    assert_eq!(s.matches("\n%%Page: ").count(), 3);
    // fig2dev's %%Pages estimate (2 rows) differs from the pages it writes.
    assert_eq!(line(&s, "%%Pages"), "%%Pages: 6");
    let s = ps(&f, &[('o', None)]);
    assert_eq!(s.matches("\n%%Page: ").count(), 1);
    let f = fig("Portrait", "Flush Left", "A4", "Single", big);
    assert_eq!(ps(&f, &[('M', None)]).matches("\n%%Page: ").count(), 3);
    let overlap = ps(&f, &[('M', None), ('O', None)]);
    assert!(overlap.matches("\n%%Page: ").count() >= 3);
}

#[test]
fn eps_ignores_the_page_options() {
    let f = landscape_letter();
    let plain = run("eps", &f, &[]).unwrap();
    let with = run(
        "eps",
        &f,
        &[('z', Some("a0")), ('l', Some("x")), ('M', None), ('c', None)],
    )
    .unwrap();
    assert_eq!(plain, with);
}

#[test]
fn pdf_page_mode() {
    let f = fig("Portrait", "Center", "A4", "Single", BOX_AT);
    let figure = run("pdf", &f, &[]).unwrap();
    let page = run("pdf", &f, &[('P', None)]).unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    assert!(
        text(&page).contains("/MediaBox [0 0 595 842]"),
        "{}",
        &text(&page)[..400]
    );
    assert!(!text(&figure).contains("/MediaBox [0 0 595 842]"));
    // -z alone implies page mode; -Y sets the version; bad values give 1.5.
    let z = run("pdf", &f, &[('z', Some("a5")), ('Y', Some("7"))]).unwrap();
    assert!(z.starts_with(b"%PDF-1.7"));
    assert!(text(&z).contains("/MediaBox [0 0 420 595]"));
    let bad = run("pdf", &f, &[('Y', Some("9"))]).unwrap();
    assert!(bad.starts_with(b"%PDF-1.5"));
    // Page options before -P are ignored, as in genpdf_option.
    let before = run("pdf", &f, &[('z', Some("a4")), ('l', Some("x"))]).unwrap();
    assert!(text(&before).contains("/MediaBox [0 0 842 595]") || text(&before).contains("/MediaBox [0 0 595 842]"));
}

#[test]
fn pdf_multi_page_has_a_page_per_sheet() {
    let big = "2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 24000 0 24000 14400 0 14400 0 0\n";
    let f = fig("Portrait", "Flush Left", "A4", "Single", big);
    let pdf = run("pdf", &f, &[('P', None), ('M', None)]).unwrap();
    assert!(String::from_utf8_lossy(&pdf).contains("/Count 3"));
}

#[test]
fn unknown_paper_is_fig2devs_message() {
    let f = landscape_letter();
    let r = run("ps", &f, &[('z', Some("foo"))]);
    assert_eq!(r.unwrap_err(), "Unknown paper size `foo'");
    // Also for EPS, where -z itself is ignored but the file's paper counts.
    let f = fig("Portrait", "Center", "Foo", "Single", BOX_AT);
    assert_eq!(run("eps", &f, &[]).unwrap_err(), "Unknown paper size `Foo'");
}

#[test]
fn unknown_options_keep_fig2devs_message() {
    let mut dev = drivers::driver("ps").unwrap();
    let mut ctx = Ctx::default();
    assert_eq!(
        dev.option(&mut ctx, 'k', None).unwrap_err(),
        "Argument -k unknown to ps driver."
    );
}
