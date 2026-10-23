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
        json!({"id": 3, "x": 500, "y": 500, "axes": [1, 2, 4], "boundaries": [1, 2]}),
        json!({"id": 1, "x": 500, "y": 500, "axes": [1]}),
        json!({"id": 2, "x": 1e308, "y": "a", "axes": "x", "boundaries": [null]}),
        json!({"id": 4, "x": 1500, "y": 300, "axes": [], "boundaries": []}),
        json!({"ids": [1, 2, 3, 4, 5], "id": 6, "x": 0, "y": 0}),
        json!({"id": 2, "x": 2300, "y": 300, "normal": true, "length": 1e308, "tolerance": -5}),
        json!({"id": 2, "x": 2300, "y": 300, "normal": "yes", "length": -1}),
        json!({"id": 4, "x": 600, "y": 2300, "length": 21, "tolerance": 1e308}),
        json!({"id": 5, "x": 3300, "y": 0, "normal": false, "length": 0}),
        json!({"id": 6, "x": 100, "y": 3000, "tolerance": 1e9}),
        json!({"tangent_length": 1e308, "id": 3, "x": 300, "y": 1200}),
        json!({"tangent_length": -3}),
        json!({"tangent_length": "long"}),
        json!({"id": 2, "x": 1e308, "y": "a", "add": "yes", "points": [[0, 0]], "save": 3, "what": "volume"}),
        json!({"points": [[0, 0], [0, 0], [0, 0]], "save": true}),
        json!({"points": [[1e308, 1e308], [-1e308, 0], [0, 1e308]]}),
        json!({"id": 4, "x": 600, "y": 300, "tolerance": -1, "add": true, "what": "length"}),
        json!({"what": "area", "signed": "x"}),
        json!({"what": "area", "signed": true, "id": 1}),
        json!({"angle_geom": "wavy"}),
        json!({"angle_geom": "latex_arrow", "linkmode": 1e308}),
        json!({"angle_geom": [1e308, null]}),
        json!({"color": 1e308, "rgb": "#12", "text": "x".repeat(500), "h": 1e308, "s": -1e308, "v": "a"}),
        json!({"color": 32, "rgb": "notacolour", "target": "mouth", "prefix": 5, "limit": -1}),
        json!({"color": 543.5, "rgb": "#abc", "h": null, "text": "#ff00zz", "cancel": "yes"}),
        json!({"attr": "wavy", "on": 3, "all": "maybe"}),
        json!({"attr": "line_width", "on": "yes", "all": 7}),
        json!({"name": 5, "ids": [1], "query": [1], "local_font": 7}),
        json!({"name": "\u{0}\u{1}", "ids": [3], "query": "\u{0}", "local_font": ["x"]}),
        json!({"name": null, "ids": [999], "local_font": "Nope", "font": 1e308}),
        json!({"name": "x".repeat(5000), "local_font": "x".repeat(5000)}),
    ]
}

#[test]
fn every_command_with_hostile_params() {
    for c in cmd::all() {
        if c.id == "file.save" || c.id == "file.export" || c.id == "file.digitize" || c.id == "library.save" {
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
fn digitize_with_hostile_params() {
    // Skipped by the sweeps (it writes files): its own hostile run, into a
    // scratch folder, must fail cleanly and leave the document alone.
    let dir = std::env::temp_dir().join(format!("rustfig-digitize-sweep-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let prefix = dir.join("d").to_string_lossy().into_owned();
    let cases = vec![
        json!({}),
        json!({"prefix": ""}),
        json!({"prefix": prefix, "start": -1}),
        json!({"prefix": prefix, "start": 1e300}),
        json!({"prefix": prefix, "start": "x"}),
        json!({"prefix": prefix, "suffix": ""}),
        json!({"prefix": prefix, "suffix": "../x"}),
        json!({"prefix": prefix, "ids": [987654321]}),
        json!({"prefix": prefix, "ids": "all"}),
        json!({"prefix": prefix, "append": 3}),
        json!({"prefix": prefix, "calibration": 5}),
        json!({"prefix": prefix, "calibration": {"points": []}}),
        json!({"prefix": prefix, "calibration": {"points": [{"x": 0}, {"x": 1}]}}),
        json!({"prefix": prefix, "calibration": {"points": [
            {"x": 5, "y": 1, "vx": 0, "vy": 0}, {"x": 5, "y": 9, "vx": 1, "vy": 1}]}}),
        json!({"prefix": prefix, "calibration": {"log_x": true, "points": [
            {"x": 0, "y": 1, "vx": -1, "vy": 0}, {"x": 5, "y": 9, "vx": 1, "vy": 1}]}}),
    ];
    for p in cases {
        let mut e = populated();
        let before = e.doc().clone();
        let _ = e.execute("file.digitize", &p);
        assert_eq!(e.doc(), &before, "{p}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_command_on_empty_document() {
    for c in cmd::all() {
        if c.id == "file.save" || c.id == "file.export" || c.id == "file.digitize" || c.id == "library.save" {
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
            if c.id == "file.save" || c.id == "file.export" || c.id == "file.digitize" || c.id == "library.save" || c.id.starts_with("file.open") {
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
        // A compound left open is saved with the rest of the figure.
        prop_assert_eq!(e2.doc().objects.len(), e.whole_doc().objects.len());
    }
}
