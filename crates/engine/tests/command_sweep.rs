#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Every command, with hostile parameters, must return Ok or Err — never panic —
//! and a failed command must leave the document unchanged.

use proptest::prelude::*;
use rustfig_engine::{cmd, Engine};
use serde_json::{json, Value};

fn populated() -> Engine {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1200, "height": 600, "area_fill": 45}),
    )
    .unwrap();
    e.execute("shape.circle", &json!({"cx": 2000, "cy": 300, "r": 300}))
        .unwrap();
    e.execute(
        "shape.polyline",
        &json!({"points": [[0, 900], [600, 1500], [1200, 900]], "forward_arrow": true}),
    )
    .unwrap();
    e.execute(
        "shape.spline",
        &json!({"points": [[0, 2000], [600, 2600], [1200, 2000]], "kind": "x"}),
    )
    .unwrap();
    e.execute("shape.arc", &json!({"points": [[3000, 0], [3300, 300], [3600, 0]]}))
        .unwrap();
    e.execute("shape.text", &json!({"x": 100, "y": 3000, "text": "hello"}))
        .unwrap();
    e.execute("edit.select_all", &json!({})).unwrap();
    e
}

fn hostile_values() -> Vec<Value> {
    vec![
        json!({}),
        json!(null),
        json!([]),
        json!({"ids": [1, 2]}),
        json!({"ids": [999999]}),
        json!({"ids": "x"}),
        json!({"x": 1e308, "y": -1e308, "width": 1e300, "height": 0}),
        json!({"x": "a", "dx": [], "points": [[1]]}),
        json!({"points": [[0, 0], [1, 1], [2, 2]], "pie": true}),
        json!({"path": "/nonexistent/file.fig"}),
        json!({"name": "../../a\u{0}b", "overwrite": 1, "save": []}),
        json!({"name": "x".repeat(9000)}),
        json!({"name": ".."}),
        json!({"data": "#FIG 3.2\ngarbage"}),
        json!({"format": "svg"}),
        json!({"format": "png", "dpi": 1e9}),
        json!({"format": "eps", "options": {"border": 1e308, "Z": "1e-300", "depths": "+999999999999", "font_size": 1e9}}),
        json!({"format": "pdf", "options": {"g": "#ééé", "bogus": [], "title": "\u{0}"}}),
        json!({"format": "png", "options": {"m": 1000, "border": 72000, "font": "Symbol", "N": true}}),
        json!({"format": "svg", "options": "x"}),
        json!({"id": 1, "index": 99999, "x": 0, "y": 0}),
        json!({"x1": -1e9, "y1": -1e9, "x2": 1e9, "y2": 1e9, "dx": 5, "dy": 5, "reference": 1}),
        json!({"x1": -10, "y1": -10, "x2": 4000, "y2": 4000, "id": 3, "end": "start", "x": 1, "y": 1}),
        json!({"reference": 1, "h": "left", "v": "top", "points": [[0, 0], [100, 0], [0, 100]], "kind": "diameter"}),
        json!({"x": 5, "y": 5, "end": 7, "reference": "a"}),
        json!({"id": 3, "index": 0, "x": 5, "y": 5}),
        json!({"zoom": -1, "sx": 0, "angle": 1e20, "depth": -5}),
        json!({"text": "", "find": "", "rgb": "#zzzzzz", "units": "parsecs"}),
        json!({"type": "press", "x": 0, "y": 0}),
        json!({"thickness": "thick", "line": "wavy", "grid": "huge"}),
    ]
}

#[test]
fn every_command_with_hostile_params() {
    for c in cmd::all() {
        if c.id == "file.save" || c.id == "file.export" {
            continue; // would write files
        }
        for p in hostile_values() {
            let mut e = populated();
            let before = e.doc().clone();
            let r = e.execute(c.id, &p);
            if r.is_err() && c.mutates {
                assert_eq!(e.doc(), &before, "{} with {p} failed but changed the document", c.id);
            }
            let _ = e.overlay();
        }
    }
}

#[test]
fn every_command_on_empty_document() {
    for c in cmd::all() {
        if c.id == "file.save" || c.id == "file.export" {
            continue;
        }
        let mut e = Engine::new();
        let _ = e.execute(c.id, &json!({}));
    }
}

#[test]
fn undo_redo_after_every_mutating_command() {
    for c in cmd::all().iter().filter(|c| c.mutates) {
        let mut e = populated();
        let before = e.doc().clone();
        if e.execute(c.id, &json!({})).is_ok() && e.doc() != &before {
            assert!(e.undo(), "{} not undoable", c.id);
            assert_eq!(e.doc().objects, before.objects, "{} undo mismatch", c.id);
            assert!(e.redo());
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn random_command_sequences(seq in proptest::collection::vec((0usize..400, -3000.0f64..3000.0, -3000.0f64..3000.0), 1..20)) {
        let all = cmd::all();
        let mut e = populated();
        for (i, a, b) in seq {
            let c = &all[i % all.len()];
            if c.id == "file.save" || c.id == "file.export" || c.id.starts_with("file.open") {
                continue;
            }
            let p = json!({"dx": a, "dy": b, "x": a, "y": b, "width": b, "height": a, "angle": a,
                "sx": 1.5, "zoom": 2.0, "cx": a, "cy": b, "r": b.abs(), "rx": a, "ry": b, "depth": 40,
                "points": [[a, b], [b, a], [a + 10.0, b - 30.0]], "text": "x", "index": 1, "id": 1,
                "type": "press", "key": "enter"});
            let _ = e.execute(c.id, &p);
        }
        let s = e.execute("file.save_data", &json!({})).unwrap();
        let data = s["data"].as_str().unwrap().to_string();
        let mut e2 = Engine::new();
        e2.execute("file.open_data", &json!({"data": data})).unwrap();
        prop_assert_eq!(e2.doc().objects.len(), e.doc().objects.len());
    }
}
