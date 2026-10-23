#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! `file.open_data` from bytes (C2: the browser's Open and drag-and-drop).

use rustfig_engine::{base64, base64_decode, Engine};
use serde_json::json;

#[test]
fn base64_round_trips_every_length() {
    for n in 0..40u8 {
        let data: Vec<u8> = (0..n).map(|i| i.wrapping_mul(37).wrapping_add(11)).collect();
        assert_eq!(base64_decode(&base64(&data)), data);
    }
}

#[test]
fn open_data_takes_base64_bytes() {
    let mut a = Engine::new();
    a.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    let text = a.execute("file.save_data", &json!({})).unwrap()["data"]
        .as_str()
        .unwrap()
        .to_string();
    let mut b = Engine::new();
    let r = b
        .execute(
            "file.open_data",
            &json!({"base64": base64(text.as_bytes()), "name": "ä ö.fig"}),
        )
        .unwrap();
    assert_eq!(r["objects"], 1);
    assert_eq!(b.path(), Some("ä ö.fig"));
}

#[test]
fn open_data_rejects_garbage_without_changing_the_document() {
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    for p in [
        json!({"base64": "////"}),
        json!({"base64": ""}),
        json!({"base64": 5}),
        json!({}),
    ] {
        assert!(e.execute("file.open_data", &p).is_err(), "{p}");
    }
    assert_eq!(e.doc().objects.len(), 1);
}
