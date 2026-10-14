//! A1 and A9: undo snapshots share unchanged objects, and commands only
//! re-measure what they changed, so editing a large drawing
//! keeps the history small and each step cheap.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::Engine;
use rustfig_geom::Point;
use rustfig_model::{ColorRef, Document, Justify, Object, ObjectKind, PolyKind, Polyline, Style, Text};
use serde_json::json;
use std::sync::Arc;

/// A drawing of `n` objects, some of them texts.
fn large(n: usize) -> Document {
    let mut doc = Document::new();
    for i in 0..n {
        let (x, y) = ((i % 200) as f64 * 300.0, (i / 200) as f64 * 300.0);
        let kind = if i % 10 == 0 {
            ObjectKind::Text(Text {
                justify: Justify::Left,
                color: ColorRef::BLACK,
                depth: 50,
                pen_style: -1,
                font: 0,
                size: 12.0,
                angle: 0.0,
                flags: 4,
                height: 0.0,
                length: 0.0,
                origin: Point::new(x, y),
                text: format!("t{i}"),
            })
        } else {
            ObjectKind::Polyline(Polyline::new(
                PolyKind::Polyline,
                Style::default(),
                vec![
                    Point::new(x, y),
                    Point::new(x + 100.0, y + 200.0),
                    Point::new(x + 200.0, y),
                ],
            ))
        };
        doc.objects.push(Arc::new(Object::new(kind)));
    }
    doc
}

#[test]
fn edits_share_unchanged_objects() {
    let mut e = Engine::new();
    e.replace_document(large(50_000), None);
    let ids: Vec<u64> = e.doc().objects.iter().take(200).map(|o| o.id.0).collect();
    let before = e.doc().objects.clone();
    for id in &ids {
        e.execute("transform.move", &json!({"ids": [id], "dx": 15, "dy": 0}))
            .unwrap();
    }
    // Only the 200 moved objects were copied.
    let copied = e
        .doc()
        .objects
        .iter()
        .zip(&before)
        .filter(|(a, b)| !Arc::ptr_eq(a, b))
        .count();
    assert_eq!(copied, 200);
    let mb = e.undo_bytes() as f64 / 1e6;
    eprintln!("200 one-object edits on 50,000 objects: {mb:.2} MB of undo");
    assert!(mb < 50.0, "200 edits hold {mb:.1} MB of undo");
    for _ in 0..200 {
        assert!(e.undo());
    }
    assert_eq!(e.doc().objects, before);
}

/// A9: texts changed by a command get new metrics; the others are left alone.
#[test]
fn changed_texts_are_measured() {
    let mut e = Engine::new();
    let a = e
        .execute("shape.text", &json!({"x": 0, "y": 0, "text": "Hamburgefonts"}))
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    e.execute("shape.text", &json!({"x": 0, "y": 900, "text": "other"}))
        .unwrap();
    let length = |e: &Engine, i: usize| match &e.doc().objects[i].kind {
        ObjectKind::Text(t) => t.length,
        _ => panic!(),
    };
    let (l0, other) = (length(&e, 0), length(&e, 1));
    let untouched = Arc::clone(&e.doc().objects[1]);
    e.execute("object.set_attributes", &json!({"ids": [a], "font_size": 24}))
        .unwrap();
    assert!((length(&e, 0) - 2.0 * l0).abs() < l0 * 0.1, "{} {l0}", length(&e, 0));
    e.execute("object.set_attributes", &json!({"ids": [a], "text": "H"}))
        .unwrap();
    assert!(length(&e, 0) < l0);
    assert!(Arc::ptr_eq(&untouched, &e.doc().objects[1]));
    assert_eq!(length(&e, 1), other);
}

/// A9: hit testing sees objects changed since the last test, in the same
/// revision (as inside a running command).
#[test]
fn hit_test_follows_changed_objects() {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1200, "height": 1200}),
    )
    .unwrap();
    let id = e.doc().objects[0].id;
    assert_eq!(e.hit_test(Point::new(0.0, 600.0), 30.0), Some(id));
    let rev = e.revision();
    e.doc_mut()
        .get_mut(id)
        .unwrap()
        .transform(&rustfig_geom::Affine::translate(5000.0, 0.0), false);
    assert_eq!(e.revision(), rev);
    assert_eq!(e.hit_test(Point::new(0.0, 600.0), 30.0), None);
    assert_eq!(e.hit_test(Point::new(5000.0, 600.0), 30.0), Some(id));
}
