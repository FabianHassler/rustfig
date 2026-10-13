#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_format::{read_any, read_fig, read_native, write_fig, write_native};
use rustfig_model::{ObjectKind, PolyKind, SplineKind, Units};

const SAMPLE: &[u8] = include_bytes!("sample.fig");

#[test]
fn reads_sample() {
    let doc = read_fig(SAMPLE).unwrap();
    assert_eq!(doc.settings.units, Units::Metric);
    assert_eq!(doc.comments, vec!["A whole-figure comment".to_string()]);
    assert_eq!(doc.colors.len(), 1);
    assert_eq!(doc.objects.len(), 6);
    let ObjectKind::Polyline(b) = &doc.objects[0].kind else {
        panic!()
    };
    assert_eq!(b.kind, PolyKind::Box);
    assert_eq!(b.points.len(), 4);
    assert_eq!(doc.objects[0].comments, vec!["a box".to_string()]);
    let ObjectKind::Polyline(l) = &doc.objects[1].kind else {
        panic!()
    };
    assert!(l.forward_arrow.is_some() && l.backward_arrow.is_some());
    assert_eq!(l.points.len(), 3);
    let ObjectKind::Spline(s) = &doc.objects[3].kind else {
        panic!()
    };
    assert_eq!(s.kind, SplineKind::X);
    assert_eq!(s.shapes, vec![0.0, 1.0, -1.0, 0.0]);
    let ObjectKind::Arc(a) = &doc.objects[4].kind else {
        panic!()
    };
    assert!((a.center.x - 3000.0).abs() < 1e-6);
    let ObjectKind::Compound(c) = &doc.objects[5].kind else {
        panic!()
    };
    let ObjectKind::Text(t) = &c.objects[0].kind else {
        panic!()
    };
    assert_eq!(t.text, "Hello \\ world");
    let ObjectKind::Text(t) = &c.objects[1].kind else {
        panic!()
    };
    // Octal escapes in a UTF-8 file are raw bytes; \344 alone is invalid UTF-8.
    assert!(t.text.starts_with("multi\\\nline "));
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
    let ObjectKind::Spline(s) = &doc.objects[0].kind else {
        panic!()
    };
    assert_eq!(s.kind, SplineKind::Interpolated);
    assert_eq!(s.points[1].x, 400.0);
    let ObjectKind::Polyline(p) = &doc.objects[1].kind else {
        panic!()
    };
    assert_eq!(p.points[1].x, 100.0);
}

#[test]
fn errors_have_lines() {
    let e = read_fig(b"#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100\nSingle\n-2\n1200 2\n2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n\t 1 2 x 4\n").unwrap_err();
    assert_eq!(e.line, 11);
    assert!(read_fig(b"hello").is_err());
    assert!(read_fig(b"#FIG 2.1\n80 2\n").is_err());
    assert!(read_any(b"garbage").is_err());
}
