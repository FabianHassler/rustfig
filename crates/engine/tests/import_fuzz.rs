#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Opening or merging arbitrary data must never panic.

use proptest::prelude::*;
use rustfig_engine::Engine;
use serde_json::json;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn open_and_merge_random(data in "(#FIG 3\\.2\n)?[ -~\n]{0,600}") {
        let mut e = Engine::new();
        let _ = e.execute("file.open_data", &json!({"data": data}));
        let _ = e.execute("file.merge_data", &json!({"data": data}));
        let _ = e.execute("file.export_data", &json!({"format": "svg"}));
    }

    #[test]
    fn raw_object_json(s in "[ -~]{0,300}") {
        let mut e = Engine::new();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            let _ = e.execute("object.add", &json!({"object": v}));
        }
        let _ = rustfig_engine::import::decode_picture(s.as_bytes());
    }
}

#[test]
fn exports_of_every_format() {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1200, "height": 600, "area_fill": 50}),
    )
    .unwrap();
    e.execute("shape.text", &json!({"x": 100, "y": 300, "text": "Grüße (x)"}))
        .unwrap();
    for f in ["svg", "eps", "pdf", "png"] {
        let r = e.execute("file.export_data", &json!({"format": f})).unwrap();
        assert!(r.get("data").or(r.get("base64")).is_some(), "{f}");
    }
}
