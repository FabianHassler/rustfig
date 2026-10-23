#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_format::{read_any, read_fig, read_native, write_fig, write_native};
use rustfig_model::{ExportOnSave, ObjectKind, PolyKind, SplineKind, Units};

const SAMPLE: &[u8] = include_bytes!("sample.fig");

#[test]
fn reads_sample() {
    let doc = read_fig(SAMPLE).unwrap();
    assert_eq!(doc.settings.units, Units::Metric);
    assert_eq!(doc.comments, vec!["A whole-figure comment".to_string()]);
    assert_eq!(doc.colors.len(), 1);
    assert_eq!(doc.objects.len(), 6);
    // fig2dev's list order: arcs, compounds, ellipses, lines, splines, texts.
    let ObjectKind::Arc(a) = &doc.objects[0].kind else {
        panic!()
    };
    assert!((a.center.x - 3000.0).abs() < 1e-6);
    let ObjectKind::Compound(c) = &doc.objects[1].kind else {
        panic!()
    };
    assert!(matches!(doc.objects[2].kind, ObjectKind::Ellipse(_)));
    let ObjectKind::Polyline(b) = &doc.objects[3].kind else {
        panic!()
    };
    assert_eq!(b.kind, PolyKind::Box);
    assert_eq!(b.points.len(), 4);
    assert_eq!(doc.objects[3].comments, vec!["a box".to_string()]);
    let ObjectKind::Polyline(l) = &doc.objects[4].kind else {
        panic!()
    };
    assert!(l.forward_arrow.is_some() && l.backward_arrow.is_some());
    assert_eq!(l.points.len(), 3);
    let ObjectKind::Spline(s) = &doc.objects[5].kind else {
        panic!()
    };
    assert_eq!(s.kind, SplineKind::X);
    assert_eq!(s.shapes, vec![0.0, 1.0, -1.0, 0.0]);
    let ObjectKind::Text(t) = &c.objects[0].kind else {
        panic!()
    };
    assert_eq!(t.text, "Hello \\ world");
    let ObjectKind::Text(t) = &c.objects[1].kind else {
        panic!()
    };
    // A lone backslash is dropped (fig2dev reads `\` alone as an escape of
    // nothing); \344 alone is invalid UTF-8 and read as ISO-8859-1.
    assert_eq!(t.text, "multi\nline ä");
}

#[test]
fn fig_roundtrip_is_stable() {
    let doc = read_fig(SAMPLE).unwrap();
    let out = write_fig(&doc);
    let doc2 = read_fig(out.as_bytes()).unwrap();
    assert_eq!(doc.objects.len(), doc2.objects.len());
    let out2 = write_fig(&doc2);
    assert_eq!(out, out2);
}

#[test]
fn export_on_save_comment_roundtrips_and_garbage_stays_a_comment() {
    let mut doc = read_fig(SAMPLE).unwrap();
    doc.export_on_save = Some(ExportOnSave {
        path: "/tmp/figure.svg".to_string(),
        params: serde_json::Map::new(),
    });
    let out = write_fig(&doc);
    assert!(out.contains("# RustFigExportOnSave "), "{out}");
    let back = read_fig(out.as_bytes()).unwrap();
    assert_eq!(back.export_on_save, doc.export_on_save);
    assert_eq!(back.comments, doc.comments, "the setting is not a user comment");

    // A comment that names the marker but isn't valid JSON is the user's
    // text, not a setting (hostile files must not surprise the parser).
    let src = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n\
# RustFigExportOnSave not-json\n\
# a real comment\n\
1200 2\n";
    let parsed = read_fig(src.as_bytes()).unwrap();
    assert!(parsed.export_on_save.is_none());
    assert!(parsed.comments.iter().any(|c| c == "RustFigExportOnSave not-json"));
    assert!(parsed.comments.iter().any(|c| c == "a real comment"));
}

#[test]
fn native_roundtrip() {
    let doc = read_fig(SAMPLE).unwrap();
    let s = write_native(&doc).unwrap();
    let back = read_native(s.as_bytes()).unwrap();
    assert_eq!(doc.objects, back.objects);
    assert_eq!(read_any(s.as_bytes()).unwrap().objects, doc.objects);
}

#[test]
fn latin1_file() {
    let mut src = b"#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n".to_vec();
    src.extend_from_slice(b"4 0 0 50 -1 0 12 0.0000 4 135 1200 1000 7500 caf\xe9 \\351\\001\n");
    let doc = read_fig(&src).unwrap();
    let ObjectKind::Text(t) = &doc.objects[0].kind else {
        panic!()
    };
    assert_eq!(t.text, "café é");
}

#[test]
fn version_31_and_resolution() {
    let src = b"#FIG 3.1\nLandscape\nCenter\nInches\n600 2\n3 2 0 1 0 7 0 0 -1 0.000 0 0 0 3\n\t 100 100 200 200 300 100\n\t 1 1 2 2 3 3 4 4 5 5 6 6\n2 1 0 1 0 7 0 0 -1 0.000 0 0 -1 0 0 2\n\t 0 0 50 50\n";
    let doc = read_fig(src).unwrap();
    let ObjectKind::Spline(s) = &doc.objects[1].kind else {
        panic!()
    };
    assert_eq!(s.kind, SplineKind::Interpolated);
    assert_eq!(s.points[1].x, 400.0);
    let ObjectKind::Polyline(p) = &doc.objects[0].kind else {
        panic!()
    };
    assert_eq!(p.points[1].x, 100.0);
}

#[test]
fn errors_have_lines() {
    let e = read_fig(b"#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100\nSingle\n-2\n1200 2\n2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n\t 1 2 x 4\n").unwrap_err();
    assert!(e.message.contains("line 11"), "{e}");
    assert!(read_fig(b"hello").is_err());
    assert!(read_fig(b"#FIG 4.0\nLandscape\n").is_err());
    assert!(read_any(b"garbage").is_err());
}

#[test]
fn objects_are_saved_in_xfig_type_order() {
    // A text listed before a polyline: xfig saves lines before texts.
    let fig = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
4 0 0 50 -1 0 12 0.0000 4 135 405 600 600 Hi\\001\n\
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n\t 0 0 1200 1200\n";
    let mut doc = read_fig(fig.as_bytes()).unwrap();
    // The reader lists lines before texts, as xfig does; swap them.
    doc.objects.swap(0, 1);
    assert!(matches!(doc.objects[0].kind, ObjectKind::Text(_)));
    let out = write_fig(&doc);
    let line = out.find("\n2 ").unwrap();
    let text = out.find("\n4 ").unwrap();
    assert!(line < text, "lines are written before texts:\n{out}");
}

#[test]
fn header_without_centring_line() {
    // As in xfig's library figure simple_DScurves/cllog-400.fig; fig2dev reads it.
    let src = "#FIG 3.2\nPortrait\nMetric\nLetter\n100.00\nSingle\n0\n1200 2\n2 1 0 1 0 -1 50 0 -1 0.000 1 0 7 0 0 2\n\t 1200 710 5847 710\n";
    let doc = rustfig_format::read_fig(src.as_bytes()).unwrap();
    assert!(doc.settings.center);
    assert_eq!(doc.settings.units, rustfig_model::Units::Metric);
    assert_eq!(doc.settings.transparent_color, 0);
    assert_eq!(doc.objects.len(), 1);
    let flush = src.replace("Portrait\n", "Portrait\nFlush left\n");
    assert!(!rustfig_format::read_fig(flush.as_bytes()).unwrap().settings.center);
}

/// fig2dev's -E: the text's encoding, overriding the file's own line.
#[test]
fn text_encodings() {
    use rustfig_format::{read_fig_encoded, TextEncoding};
    let fig = |text: &str| {
        format!("#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n4 0 0 50 -1 0 12 0.0000 4 135 405 0 0 {text}\\001\n")
    };
    let text_of = |src: &[u8], enc| match &read_fig_encoded(src, enc).unwrap().objects[0].kind {
        rustfig_model::ObjectKind::Text(t) => t.text.clone(),
        _ => panic!(),
    };
    // \330 is Ø in ISO-8859-1, Ř in ISO-8859-2.
    let src = fig("\\330 6,4");
    assert_eq!(text_of(src.as_bytes(), TextEncoding::Auto), "Ø 6,4");
    assert_eq!(text_of(src.as_bytes(), TextEncoding::Latin1), "Ø 6,4");
    assert_eq!(text_of(src.as_bytes(), TextEncoding::Latin2), "Ř 6,4");
    // Raw ISO-8859-2 bytes.
    let mut raw = fig("X").into_bytes();
    let at = raw.iter().position(|&b| b == b'X').unwrap();
    raw[at] = 0xB9; // š in ISO-8859-2, ¹ in ISO-8859-1
    assert_eq!(text_of(&raw, TextEncoding::Latin2), "š");
    assert_eq!(text_of(&raw, TextEncoding::Latin1), "¹");
    for (name, enc) in [
        ("0", TextEncoding::Utf8),
        ("utf-8", TextEncoding::Utf8),
        ("1", TextEncoding::Latin1),
        ("ISO-8859-2", TextEncoding::Latin2),
    ] {
        assert_eq!(TextEncoding::from_name(name), Some(enc), "{name}");
    }
    assert_eq!(TextEncoding::from_name("KOI8-R"), None);
}

#[test]
fn old_versions_are_recognised() {
    use rustfig_format::FileFormat;
    for src in [&b"#FIG 2.1\n80 2\n"[..], b"#FIG 1.4\n", b"1200 2 9 9\n", b"#FIG 3.2\n"] {
        assert_eq!(FileFormat::sniff(src), Some(FileFormat::Fig));
    }
    assert_eq!(FileFormat::sniff(b"{\"objects\": []}"), Some(FileFormat::Native));
    assert_eq!(FileFormat::sniff(b""), None);
    assert_eq!(FileFormat::sniff(b"hello"), None);
}

const HEAD: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n";

/// Hostile Fig input returns errors (or a repaired figure), never panics or
/// allocates what the counts say.
#[test]
fn hostile_fig_input() {
    for body in [
        "2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2000000000\n\t 1 2 3 4\n",
        "3 2 0 1 0 7 50 -1 -1 0.000 0 0 0 2000000000\n\t 1 2 3 4\n",
        "2 1 0 1 0 7 50 -1 -1 nan 0 0 -1 0 0 2\n\t 1 2 3 4\n",
        "1 3 0 1 0 7 50 -1 -1 0.000 1 nan 100 100 nan 1 1 1 1 1\n",
        "4 0 0 50 -1 0 nan 0.0000 4 135 405 600 600 Hi\\001\n",
        "0 32 #zzzzzz\n",
        "2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n\t 99999999999999999999 1 3 4\n",
        "\0\0\0\n",
    ] {
        let _ = read_fig(format!("{HEAD}{body}").as_bytes());
    }
    let deep = format!("{HEAD}{}", "6 0 0 1 1\n".repeat(rustfig_format::MAX_NESTING + 50));
    assert!(read_fig(deep.as_bytes()).is_err());
}

/// What the reader deletes or fixes is reported as messages.
#[test]
fn reader_messages() {
    let src = format!("{HEAD}2 3 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n\t 0 0 600 0\n");
    let (doc, messages) =
        rustfig_format::read_fig_messages(src.as_bytes(), rustfig_format::TextEncoding::Auto).unwrap();
    // Like xfig, the reader deletes a polygon of fewer than 3 points.
    assert_eq!(doc.objects.len(), 0);
    assert!(messages.iter().any(|m| m.contains("Deleting polygon")), "{messages:?}");
    let ok = rustfig_format::read_fig_messages(SAMPLE, rustfig_format::TextEncoding::Auto).unwrap();
    assert!(ok.1.is_empty(), "{:?}", ok.1);
}

/// A text keeps its leading blanks beyond the separating one (fig2dev's own
/// reader drops them all).
#[test]
fn text_keeps_leading_blanks() {
    let src = format!("{HEAD}4 0 0 50 -1 0 12 0.0000 4 135 405 600 600   two\\001\n");
    let doc = read_fig(src.as_bytes()).unwrap();
    let ObjectKind::Text(t) = &doc.objects[0].kind else {
        panic!()
    };
    assert_eq!(t.text, "  two");
}

/// F7: an installed font of a text is a `rustfig-font:` comment line in the
/// Fig file (xfig and fig2dev ignore it); other comments stay, and the
/// text's Fig font number is the nearest Fig font.
#[test]
fn local_font_is_a_text_comment_in_fig() {
    let src = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
        # first note\n# rustfig-font: Minion Pro Semibold\n# second note\n\
        4 0 0 50 -1 16 12 0.0000 4 135 450 300 600 hello\\001\n";
    let doc = read_fig(src.as_bytes()).unwrap();
    let ObjectKind::Text(t) = &doc.objects[0].kind else {
        panic!()
    };
    assert_eq!(t.local_font.as_deref(), Some("Minion Pro Semibold"));
    assert_eq!(t.font, 16);
    assert_eq!(doc.objects[0].comments, ["first note", "second note"]);
    // Written back the line is there once, and a second read agrees.
    let out = write_fig(&doc);
    assert_eq!(out.matches("# rustfig-font: Minion Pro Semibold\n").count(), 1);
    assert!(
        out.contains("# first note\n# second note\n")
            || out.contains("# rustfig-font: Minion Pro Semibold\n# first note\n# second note\n")
    );
    assert_eq!(read_fig(out.as_bytes()).unwrap(), doc);
    // A text without the line is a plain Fig text.
    let plain = src.replace("# rustfig-font: Minion Pro Semibold\n", "");
    let doc2 = read_fig(plain.as_bytes()).unwrap();
    let ObjectKind::Text(t2) = &doc2.objects[0].kind else {
        panic!()
    };
    assert!(t2.local_font.is_none());
    // The native format keeps the family too.
    assert_eq!(read_native(write_native(&doc).unwrap().as_bytes()).unwrap(), doc);
}
