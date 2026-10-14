//! A1: undo snapshots share unchanged objects, so editing a large drawing
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
