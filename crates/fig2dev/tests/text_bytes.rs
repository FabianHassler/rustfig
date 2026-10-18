#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Text bytes as fig2dev 3.2.9a writes them: the bytes of the input after
//! the `\ooo` escapes, not UTF-8 (the reference outputs were taken from
//! fig2dev 3.2.9a on macOS).

use rustfig_fig2dev::{convert, drivers, read::readfp_fig, Ctx};

fn run(lang: &str, fig: &[u8], enc: Option<&str>) -> Vec<u8> {
    let mut ctx = Ctx {
        lang: lang.into(),
        input_encoding: enc.map(str::to_string),
        ..Ctx::default()
    };
    let mut dev = drivers::driver(lang).unwrap();
    ctx.font_size *= ctx.fontmag;
    let objects = readfp_fig(&mut ctx, fig).unwrap();
    convert(&mut ctx, dev.as_mut(), &objects).unwrap().output
}

fn fig(extra_header: &str, body: &[u8]) -> Vec<u8> {
    let mut v =
        format!("#FIG 3.2\n{extra_header}Landscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n").into_bytes();
    v.extend_from_slice(body);
    v
}

fn has(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

const TEXT: &[u8] = b"4 0 0 50 -1 0 12 0.0 4 135 900 1200 1200 caf\\351\\251\\001\n";

#[test]
fn escapes_are_single_bytes_in_every_text_driver() {
    let f = fig("", TEXT);
    for lang in [
        "latex", "epic", "pict2e", "tikz", "pstricks", "mp", "cgm", "pic", "tpic", "textyl", "ibmgl", "dxf",
    ] {
        let out = run(lang, &f, None);
        assert!(has(&out, b"caf\xe9\xa9"), "{lang}");
        assert!(!has(&out, "é".as_bytes()), "{lang}");
    }
}

#[test]
fn comments_keep_their_bytes() {
    let mut body = b"# caf\xe9\n".to_vec();
    body.extend_from_slice(TEXT);
    let out = run("latex", &fig("", &body), None);
    // The text's own comment comes before the text.
    assert!(has(&out, b"%  caf\xe9 \n"), "{}", String::from_utf8_lossy(&out));
}

#[test]
fn utf8_files_keep_utf8() {
    let f = fig(
        "#encoding: UTF-8\n",
        "4 0 0 50 -1 0 12 0.0 4 135 900 1200 1200 café\\001\n".as_bytes(),
    );
    let out = run("latex", &f, None);
    assert!(has(&out, "café".as_bytes()));
}

#[test]
fn a_declared_latin1_input_is_converted_to_utf8_for_tex() {
    let out = run("latex", &fig("", TEXT), Some("ISO-8859-1"));
    assert!(has(&out, "café©".as_bytes()));
    // Other drivers write the bytes unconverted.
    assert!(has(&run("cgm", &fig("", TEXT), Some("ISO-8859-1")), b"caf\xe9\xa9"));
}

#[test]
fn symbol_text_is_one_char_code_per_byte() {
    let f = fig("", "4 0 0 50 -1 32 12 0.0 4 135 900 1200 1200 a±b\\001\n".as_bytes());
    let out = run("latex", &f, None);
    assert!(has(&out, b"a\\char194\\char177b"), "{}", String::from_utf8_lossy(&out));
}

#[test]
fn unicode_view_stays_unicode() {
    let mut ctx = Ctx::default();
    let c = readfp_fig(&mut ctx, &fig("", TEXT)).unwrap();
    assert_eq!(c.texts[0].cstring, "café©");
    assert_eq!(c.texts[0].raw, b"caf\xe9\xa9");
}

#[test]
fn input_charsets_beyond_latin_go_through_encoding_rs() {
    // 0xe9: KOI8-R "И", windows-1252 "é", ISO-8859-15 "é", ISO-8859-5 "щ".
    let f = fig("", b"4 0 0 50 -1 0 12 0.0 4 135 900 1200 1200 a\\351b\\001\n");
    // (pstricks and mp write the bytes unconverted, as in C.)
    for lang in ["latex", "pict2e", "tikz", "epic"] {
        for (enc, want) in [
            ("KOI8-R", "aИb"),
            ("CP1252", "aéb"),
            ("windows-1252", "aéb"),
            ("ISO-8859-15", "aéb"),
            ("latin9", "aéb"),
            ("ISO-8859-5", "aщb"),
        ] {
            let out = run(lang, &f, Some(enc));
            assert!(has(&out, want.as_bytes()), "{lang} {enc}");
        }
    }
}

#[test]
fn undefined_bytes_stop_the_text_like_iconv() {
    // 0x81 is undefined in CP1252 (iconv rejects it): the text ends there.
    let f = fig("", b"4 0 0 50 -1 0 12 0.0 4 135 900 1200 1200 a\\351\\201b\\001\n");
    let mut ctx = Ctx {
        lang: "latex".into(),
        input_encoding: Some("CP1252".into()),
        ..Ctx::default()
    };
    let mut dev = drivers::driver("latex").unwrap();
    ctx.font_size *= ctx.fontmag;
    let objects = readfp_fig(&mut ctx, &f).unwrap();
    let out = convert(&mut ctx, dev.as_mut(), &objects).unwrap().output;
    assert!(has(&out, "aé}".as_bytes()));
    assert!(ctx.messages.iter().any(|m| m.contains("Illegal byte sequence")));
}

#[test]
fn unknown_input_charset_still_warns_and_keeps_bytes() {
    let f = fig("", b"4 0 0 50 -1 0 12 0.0 4 135 900 1200 1200 a\\351b\\001\n");
    // The reference iconv (macOS) does not know "KOI8R" without the dash.
    let out = run("latex", &f, Some("KOI8R"));
    assert!(has(&out, b"a\xe9b"));
}
