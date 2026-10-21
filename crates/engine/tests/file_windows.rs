//! xfig's file windows as commands (W10 group 2): Open and Merge take the
//! Figure Offset, Merge adds one compound, Export refuses an input picture.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::Engine;
use rustfig_model::ObjectKind;
use serde_json::json;

const TWO_BOXES: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
    2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 600 0 600 600 0 600 0 0\n\
    2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 1200 0 1800 0 1800 600 1200 600 1200 0\n";
const ONE_COMPOUND: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
    6 0 0 600 600\n\
    2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 600 0 600 600 0 600 0 0\n\
    -6\n";

#[test]
fn merge_adds_one_compound_moved_by_the_offset() {
    let mut e = Engine::new();
    e.execute(
        "file.merge_data",
        &json!({"data": TWO_BOXES, "offset_x": 100, "offset_y": 50}),
    )
    .unwrap();
    assert_eq!(e.doc().objects.len(), 1);
    let ObjectKind::Compound(c) = &e.doc().objects[0].kind else {
        panic!("not a compound");
    };
    assert_eq!(c.objects.len(), 2);
    let r = e.doc_bounds();
    assert!((r.min.x - 100.0).abs() < 20.0 && (r.min.y - 50.0).abs() < 20.0, "{r:?}");
}

#[test]
fn merge_of_one_compound_is_not_wrapped_again() {
    let mut e = Engine::new();
    e.execute("file.merge_data", &json!({"data": ONE_COMPOUND})).unwrap();
    let ObjectKind::Compound(c) = &e.doc().objects[0].kind else {
        panic!("not a compound");
    };
    assert!(matches!(c.objects[0].kind, ObjectKind::Polyline(_)));
}

#[test]
fn open_takes_the_figure_offset() {
    let dir = std::env::temp_dir().join(format!("w10g2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("a.fig");
    std::fs::write(&f, TWO_BOXES).unwrap();
    let mut e = Engine::new();
    e.execute("file.open", &json!({"path": f.to_str().unwrap()})).unwrap();
    let a = e.doc_bounds();
    e.execute(
        "file.open",
        &json!({"path": f.to_str().unwrap(), "offset_x": 1200, "offset_y": 0}),
    )
    .unwrap();
    let b = e.doc_bounds();
    assert!((b.min.x - a.min.x - 1200.0).abs() < 1.0, "{a:?} {b:?}");
    std::fs::remove_dir_all(&dir).ok();
}
