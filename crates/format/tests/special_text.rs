#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_format::{read_fig, read_native, write_fig, write_native};
use rustfig_model::ObjectKind;

const HEAD: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n";

#[test]
fn special_text_flag_roundtrips_raw() {
    let src = format!(
        "{HEAD}4 0 0 50 -1 0 12 0.0000 2 135 450 300 900 $\\\\alpha^2_{{i}}$\\001\n\
         4 0 0 50 -1 0 12 0.0000 6 135 450 300 1200 $x$\\001\n"
    );
    let doc = read_fig(src.as_bytes()).unwrap();
    let texts: Vec<_> = doc
        .objects
        .iter()
        .filter_map(|o| match &o.kind {
            ObjectKind::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(texts.len(), 2);
    let special = texts.iter().find(|t| t.text.contains("alpha")).unwrap();
    assert!(special.is_special() && !special.is_postscript());
    assert_eq!(special.text, "$\\alpha^2_{i}$");
    let both = texts.iter().find(|t| t.text == "$x$").unwrap();
    assert!(both.is_special() && both.is_postscript());

    let out = write_fig(&doc);
    let again = read_fig(out.as_bytes()).unwrap();
    assert_eq!(again.objects.len(), 2);
    for (a, b) in doc.objects.iter().zip(again.objects.iter()) {
        let (ObjectKind::Text(a), ObjectKind::Text(b)) = (&a.kind, &b.kind) else {
            panic!()
        };
        assert_eq!((a.flags, &a.text), (b.flags, &b.text));
    }
    let s = String::from_utf8_lossy(out.as_bytes());
    assert!(s.contains("$\\\\alpha^2_{i}$\\001"), "{s}");
    let native = read_native(write_native(&doc).unwrap().as_bytes()).unwrap();
    assert_eq!(native.objects, doc.objects);
}
