#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! EPS previews and the user bounding box (genps.c: `-A`, `-T`, `-C`,
//! `-B`, `-R`) and the bitmap options' messages (genbitmaps.c).

use rustfig_fig2dev::{convert, drivers, read::readfp_fig, Ctx};

const FIG: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 900 0 900 450 0 450 0 0\n";

fn run(lang: &str, opts: &[(char, Option<&str>)]) -> Result<Vec<u8>, String> {
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
    let objects = readfp_fig(&mut ctx, FIG.as_bytes()).map_err(|_| "read".to_string())?;
    Ok(convert(&mut ctx, dev.as_mut(), &objects)?.output)
}

fn text(lang: &str, opts: &[(char, Option<&str>)]) -> String {
    String::from_utf8(run(lang, opts).unwrap()).unwrap()
}

fn line<'a>(s: &'a str, prefix: &str) -> &'a str {
    s.lines().find(|l| l.starts_with(prefix)).unwrap_or("")
}

fn word(b: &[u8], at: usize) -> usize {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize
}

#[test]
fn ascii_preview_is_epsi() {
    let s = text("eps", &[('A', None)]);
    // 900 x 450 Fig units and the frame make 56 x 29 points; the preview is one larger
    // each way, minus one again in the `%%BeginPreview` line.
    assert_eq!(line(&s, "%%BoundingBox"), "%%BoundingBox: 0 0 56 29");
    assert_eq!(line(&s, "%%BeginPreview"), "%%BeginPreview: 56 29 1 29");
    let rows: Vec<&str> = s.lines().filter(|l| l.starts_with("% ")).collect();
    assert_eq!(rows.len(), 29);
    assert!(rows.iter().all(|r| r.len() == 2 + 2 * 7));
    // After the header comments, ahead of the prolog.
    let (c, p, e, pro) = (
        s.find("%%EndComments").unwrap(),
        s.find("%%BeginPreview").unwrap(),
        s.find("%%EndPreview").unwrap(),
        s.find("%%BeginProlog").unwrap(),
    );
    assert!(c < p && p < e && e < pro);
    // The frame is black: the second pixel row from the bottom is set.
    assert!(rows.iter().any(|r| r.contains("FF")));
}

#[test]
fn tiff_preview_has_the_dos_header() {
    for (opts, color) in [(vec![('T', None)], false), (vec![('C', Some("x"))], true)] {
        let b = run("eps", &opts).unwrap();
        assert_eq!(&b[..4], &[0xC5, 0xD0, 0xD3, 0xC6]);
        assert_eq!(word(&b, 4), 30);
        let (eps_len, tif_at, tif_len) = (word(&b, 8), word(&b, 20), word(&b, 24));
        assert_eq!((word(&b, 12), word(&b, 16)), (0, 0));
        assert_eq!(tif_at, 30 + eps_len);
        assert_eq!(&b[28..30], &[0xFF, 0xFF]);
        assert!(b[30..].starts_with(b"%!PS-Adobe-3.0 EPSF-3.0"));
        assert_eq!(&b[tif_at..tif_at + 4], b"II*\0");
        // The trailer follows the TIFF and a newline.
        assert!(b[tif_at + tif_len..].starts_with(b"\n%%Trailer\n"));
        // 57 x 30 pixels (the bounding box plus one each way).
        let short = |at: usize| usize::from(u16::from_le_bytes([b[tif_at + at], b[tif_at + at + 1]]));
        let tag = |t: u16| -> usize {
            let n = short(8);
            (0..n)
                .find(|i| short(10 + 12 * i) == usize::from(t))
                .map(|i| short(10 + 12 * i + 8))
                .unwrap()
        };
        assert_eq!((tag(256), tag(257)), (57, 30));
        assert_eq!(tag(259), if color { 1 } else { 5 });
        assert_eq!(tag(277), if color { 3 } else { 1 });
    }
}

#[test]
fn one_kind_of_preview() {
    let e = run("eps", &[('A', None), ('T', None)]).unwrap_err();
    assert_eq!(e, "Only one type of preview allowed: -A or -T/-C");
    // PDF accepts the options and ignores them.
    assert!(run("pdf", &[('A', None)]).unwrap().starts_with(b"%PDF"));
}

#[test]
fn user_bounding_box() {
    let bb = |o: &[(char, Option<&str>)]| {
        text("eps", o)
            .lines()
            .find(|l| l.starts_with("%%BoundingBox"))
            .unwrap()
            .to_string()
    };
    assert_eq!(bb(&[]), "%%BoundingBox: 0 0 56 29");
    // Relative: width and height in inches, origin offset.
    assert_eq!(bb(&[('R', Some("2 1 0.5 0.25"))]), "%%BoundingBox: 36 18 180 90");
    // Missing numbers: the figure's size.
    assert_eq!(bb(&[('R', Some(""))]), "%%BoundingBox: 0 0 56 29");
    // With the border (-b adds it on both sides).
    assert_eq!(
        bb(&[('R', Some("1 1")), ('b', Some("2"))]),
        "%%BoundingBox: -2 -2 74 74"
    );
    // Not for PostScript pages.
    let ps = text("ps", &[('B', Some("1 1"))]);
    assert!(ps.contains("%%BoundingBox: 0 0 792 612"));
    // The HiRes box would contradict it.
    assert!(!text("eps", &[('R', Some("1 1"))]).contains("HiResBoundingBox"));
}

#[test]
fn bitmap_option_messages() {
    let err = |o: &[(char, Option<&str>)]| run("png", o).unwrap_err();
    assert_eq!(
        err(&[('S', Some(" 3"))]),
        "fig2dev: bad value for -S option:  3, should be 1, 2 or 4"
    );
    // No number: the option does nothing.
    assert!(run("png", &[('S', Some(" abc"))]).is_ok());
    assert!(run("png", &[('S', Some(" 2x"))]).is_ok());
    assert!(err(&[('t', Some(" #ff0000"))]).starts_with("fig2dev: invalid color specification"));
    assert!(run("png", &[('b', Some(" 2.5"))]).is_ok());
}
