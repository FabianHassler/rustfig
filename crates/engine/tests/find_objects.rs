//! W10-39: `edit.find_objects` per criterion, and the actions on the matches
//! (the commands that take `ids`), each one undo step.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::Engine;
use serde_json::{json, Value};

/// A box (red, filled, depth 20), a dashed polygon (depth 60), a thick line
/// with an arrow (depth 30), an ellipse with a pattern, an arc, a spline, a
/// text "Hello World" (font 16, 14 pt, colour 4) and a compound holding a
/// green box and a text "inner".
fn engine() -> Engine {
    let mut e = Engine::new();
    let run = |e: &mut Engine, c: &str, p: Value| e.execute(c, &p).unwrap();
    run(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 600, "height": 300, "pen_color": 4, "fill_color": 2, "area_fill": 20, "depth": 20}),
    );
    run(
        &mut e,
        "shape.polygon",
        json!({"points": [[0, 500], [300, 800], [600, 500]], "line": "dashed", "style_val": 4, "depth": 60}),
    );
    run(
        &mut e,
        "shape.polyline",
        json!({"points": [[0, 1000], [600, 1000]], "forward_arrow": true, "thickness": 4, "depth": 30}),
    );
    run(
        &mut e,
        "shape.ellipse",
        json!({"cx": 2000, "cy": 300, "rx": 300, "ry": 200, "area_fill": 45, "depth": 70}),
    );
    run(
        &mut e,
        "shape.arc",
        json!({"points": [[3000, 0], [3300, 300], [3600, 0]], "depth": 80}),
    );
    run(
        &mut e,
        "shape.spline",
        json!({"points": [[0, 2000], [600, 2600], [1200, 2000]], "kind": "x", "depth": 90}),
    );
    run(
        &mut e,
        "shape.text",
        json!({"x": 100, "y": 3000, "text": "Hello World", "font": 16, "size": 14, "color": 4, "depth": 40}),
    );
    run(
        &mut e,
        "shape.rectangle",
        json!({"x": 5000, "y": 0, "width": 300, "height": 300, "pen_color": 2, "depth": 55}),
    );
    run(
        &mut e,
        "shape.text",
        json!({"x": 5000, "y": 600, "text": "inner", "font": 0, "size": 10, "depth": 55}),
    );
    let n = e.doc().objects.len();
    let ids = [n - 2, n - 1].map(|i| e.doc().objects[i].id.0);
    run(&mut e, "object.group", json!({"ids": ids}));
    e
}

fn find(e: &mut Engine, p: Value) -> Vec<String> {
    let r = e.execute("edit.find_objects", &p).unwrap();
    assert_eq!(
        r["count"].as_u64().unwrap() as usize,
        r["ids"].as_array().unwrap().len()
    );
    r["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["type"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn kinds() {
    let mut e = engine();
    assert_eq!(find(&mut e, json!({})).len(), 8);
    assert_eq!(find(&mut e, json!({"kind": "line"})), ["polyline"]);
    assert_eq!(find(&mut e, json!({"kind": "box"})), ["box"]);
    assert_eq!(find(&mut e, json!({"kind": "polygon"})), ["polygon"]);
    assert_eq!(find(&mut e, json!({"kind": "spline"})), ["spline"]);
    assert_eq!(find(&mut e, json!({"kind": "arc"})), ["arc"]);
    assert_eq!(find(&mut e, json!({"kind": "ellipse"})), ["ellipse"]);
    assert_eq!(find(&mut e, json!({"kind": "text"})), ["text"]);
    assert_eq!(find(&mut e, json!({"kind": "compound"})), ["compound"]);
    assert!(find(&mut e, json!({"kind": "picture"})).is_empty());
    assert_eq!(find(&mut e, json!({"kind": ["arc", "text"]})).len(), 2);
    assert!(e.execute("edit.find_objects", &json!({"kind": "blob"})).is_err());
}

#[test]
fn colours_fill_depth_width_style_arrows() {
    let mut e = engine();
    assert_eq!(find(&mut e, json!({"pen_color": 4})), ["box", "text"]);
    assert_eq!(find(&mut e, json!({"fill_color": 2})), ["box"]);
    assert_eq!(find(&mut e, json!({"fill": "filled"})), ["box"]);
    assert_eq!(find(&mut e, json!({"fill": "pattern"})), ["ellipse"]);
    assert_eq!(find(&mut e, json!({"fill": "none"})).len(), 4);
    assert_eq!(find(&mut e, json!({"depth": 30})), ["polyline"]);
    assert_eq!(
        find(&mut e, json!({"depth_min": 60, "depth_max": 80})),
        ["polygon", "ellipse", "arc"]
    );
    assert_eq!(find(&mut e, json!({"thickness": 4})), ["polyline"]);
    assert_eq!(find(&mut e, json!({"line": "dashed"})), ["polygon"]);
    assert_eq!(find(&mut e, json!({"arrows": "any"})), ["polyline"]);
    assert_eq!(find(&mut e, json!({"arrows": "none"})).len(), 6);
    assert_eq!(
        find(&mut e, json!({"kind": "box", "pen_color": 4, "depth": 20})),
        ["box"]
    );
    assert!(find(&mut e, json!({"kind": "box", "pen_color": 4, "depth": 21})).is_empty());
}

#[test]
fn text_criteria() {
    let mut e = engine();
    assert_eq!(find(&mut e, json!({"text": "world"})), ["text"]);
    assert!(find(&mut e, json!({"text": "nothing"})).is_empty());
    assert_eq!(find(&mut e, json!({"font": 16})), ["text"]);
    assert_eq!(find(&mut e, json!({"font_size": 14})), ["text"]);
    assert!(find(&mut e, json!({"font_size": 14, "thickness": 1})).is_empty());
}

#[test]
fn compounds_by_default_and_inside() {
    let mut e = engine();
    // Top level only: the members of the compound are not looked at.
    assert!(find(&mut e, json!({"text": "inner"})).is_empty());
    assert!(find(&mut e, json!({"pen_color": 2})).is_empty());
    // Inside: the top-level compound is the match.
    assert_eq!(find(&mut e, json!({"text": "inner", "inside": true})), ["compound"]);
    assert_eq!(find(&mut e, json!({"pen_color": 2, "inside": true})), ["compound"]);
    assert_eq!(find(&mut e, json!({"pen_color": 4, "inside": true})), ["box", "text"]);
    // The compound itself matches by kind and depth.
    assert_eq!(find(&mut e, json!({"kind": "compound", "depth": 55})), ["compound"]);
}

#[test]
fn find_is_read_only() {
    let mut e = engine();
    let before = serde_json::to_string(&e.doc().objects).unwrap();
    e.execute("edit.find_objects", &json!({"kind": "box"})).unwrap();
    assert_eq!(serde_json::to_string(&e.doc().objects).unwrap(), before);
    assert!(!rustfig_engine::cmd::find("edit.find_objects").unwrap().mutates);
}

fn ids(e: &mut Engine, p: Value) -> Vec<u64> {
    let r = e.execute("edit.find_objects", &p).unwrap();
    r["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect()
}

#[test]
fn actions_are_single_undo_steps() {
    let mut e = engine();
    let base = e.doc().objects.len();
    let snapshot = |e: &Engine| serde_json::to_string(&e.doc().objects).unwrap();
    let start = snapshot(&e);

    // Set depth.
    let hits = ids(&mut e, json!({"fill": "none"}));
    assert!(hits.len() > 1);
    e.execute("arrange.set_depth", &json!({"ids": hits, "depth": 7}))
        .unwrap();
    assert_eq!(ids(&mut e, json!({"depth": 7})).len(), 4);
    e.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(snapshot(&e), start);

    // Update with the indicator's attributes.
    e.execute("indicator.set", &json!({"pen_color": 1, "thickness": 3}))
        .ok();
    let hits = ids(&mut e, json!({"kind": ["box", "polygon"]}));
    e.execute("object.update", &json!({"ids": hits})).unwrap();
    e.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(snapshot(&e), start);

    // Glue into a compound.
    let hits = ids(&mut e, json!({"kind": ["box", "polygon", "arc"]}));
    e.execute("object.group", &json!({"ids": hits})).unwrap();
    assert_eq!(e.doc().objects.len(), base - 2);
    e.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(snapshot(&e), start);

    // Delete.
    let hits = ids(&mut e, json!({"kind": "text"}));
    e.execute("edit.delete", &json!({"ids": hits})).unwrap();
    assert_eq!(e.doc().objects.len(), base - 1);
    e.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(snapshot(&e), start);
}

#[test]
fn hostile_criteria_fail_without_change() {
    let mut e = engine();
    for p in [
        json!({"kind": 5}),
        json!({"kind": [1]}),
        json!({"fill": "x"}),
        json!({"depth": "a"}),
        json!({"depth_min": 1e300}),
        json!({"line": "wavy"}),
        json!({"arrows": 3}),
        json!({"text": 4}),
        json!({"inside": "yes"}),
    ] {
        assert!(e.execute("edit.find_objects", &p).is_err(), "{p}");
    }
}
