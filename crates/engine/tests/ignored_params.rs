#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Parameters a command does not use are reported as warnings; documented
//! ones, aliases and null values are not.

use rustfig_engine::Engine;
use rustfig_model::{ObjectId, ObjectKind};
use serde_json::{json, Value};

fn text_of(e: &Engine, id: u64) -> rustfig_model::Text {
    match &e.doc().get(ObjectId(id)).unwrap().kind {
        ObjectKind::Text(t) => t.clone(),
        _ => panic!("not a text"),
    }
}

fn warnings(r: &Value) -> Vec<String> {
    r.get("warnings")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|w| w.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

fn ignored(r: &Value) -> Vec<String> {
    warnings(r).into_iter().filter(|w| w.contains("ignored")).collect()
}

#[test]
fn text_pen_color_color_and_font_size() {
    let mut e = Engine::new();
    let r = e
        .execute("shape.text", &json!({"x": 0, "y": 0, "text": "a", "pen_color": 4}))
        .unwrap();
    assert!(ignored(&r).is_empty(), "{r}");
    assert_eq!(text_of(&e, r["id"].as_u64().unwrap()).color.0, 4);

    let r = e
        .execute("shape.text", &json!({"x": 0, "y": 0, "text": "b", "color": 2}))
        .unwrap();
    assert!(ignored(&r).is_empty(), "{r}");
    assert_eq!(text_of(&e, r["id"].as_u64().unwrap()).color.0, 2);

    let r = e
        .execute("shape.text", &json!({"x": 0, "y": 0, "text": "c", "font_size": 17}))
        .unwrap();
    assert!(ignored(&r).is_empty(), "{r}");
    assert_eq!(text_of(&e, r["id"].as_u64().unwrap()).size, 17.0);
}

#[test]
fn set_attributes_color_and_size_aliases() {
    let mut e = Engine::new();
    let r = e.execute("shape.text", &json!({"x": 0, "y": 0, "text": "a"})).unwrap();
    let id = r["id"].as_u64().unwrap();
    let r = e
        .execute("object.set_attributes", &json!({"ids": [id], "color": 5, "size": 20}))
        .unwrap();
    assert!(ignored(&r).is_empty(), "{r}");
    let t = text_of(&e, id);
    assert_eq!(t.color.0, 5);
    assert_eq!(t.size, 20.0);
    let r = e
        .execute(
            "object.set_attributes",
            &json!({"ids": [id], "pen_color": 1, "font_size": 12}),
        )
        .unwrap();
    assert!(ignored(&r).is_empty(), "{r}");
    let t = text_of(&e, id);
    assert_eq!((t.color.0, t.size), (1, 12.0));
}

#[test]
fn unknown_key_warns_and_succeeds() {
    let mut e = Engine::new();
    let r = e
        .execute(
            "shape.rectangle",
            &json!({"x": 0, "y": 0, "width": 100, "height": 100, "colour": 4}),
        )
        .unwrap();
    let w = ignored(&r);
    assert_eq!(w.len(), 1, "{r}");
    assert!(
        w[0].contains("\"colour\"") && w[0].contains("shape.rectangle"),
        "{}",
        w[0]
    );
    assert_eq!(e.doc().objects.len(), 1);
}

#[test]
fn documented_keys_give_no_warning() {
    let mut e = Engine::new();
    let r = e
        .execute(
            "shape.rectangle",
            &json!({"x": 0, "y": 0, "width": 100, "height": 100, "radius": 5, "depth": 10,
                    "thickness": 2, "pen_color": 1, "fill_color": 2, "area_fill": 20, "line": "dashed"}),
        )
        .unwrap();
    assert!(ignored(&r).is_empty(), "{r}");
    let r = e
        .execute(
            "shape.text",
            &json!({"x": 0, "y": 0, "text": "t", "size": 12, "font": 3, "postscript": true,
                    "special": false, "justify": "center", "angle": 30, "pen_color": 1, "depth": 40}),
        )
        .unwrap();
    assert!(ignored(&r).is_empty(), "{r}");
}

#[test]
fn null_valued_unknown_key_gives_no_warning() {
    let mut e = Engine::new();
    let r = e
        .execute(
            "shape.rectangle",
            &json!({"x": 0, "y": 0, "width": 100, "height": 100, "colour": null}),
        )
        .unwrap();
    assert!(ignored(&r).is_empty(), "{r}");
}

#[test]
fn plot_add_usual_keys_give_no_warning() {
    let mut e = Engine::new();
    let r = e
        .execute(
            "plot.add",
            &json!({"mode": "function", "expr": "sin(x)", "xmin": 0, "xmax": 6, "ymin": -1, "ymax": 1,
                    "samples": 200, "frame": true, "grid": false, "log_x": false, "log_y": false,
                    "xlabel": "x", "ylabel": "y", "latex": true, "latex_numbers": false,
                    "xticks": {}, "yticks": {}, "x": 600, "y": 600, "width": 4800, "height": 3000}),
        )
        .unwrap();
    assert!(ignored(&r).is_empty(), "{r}");
    assert_eq!(r["curves"], 1);
}

#[test]
fn failed_command_still_errors() {
    let mut e = Engine::new();
    assert!(e
        .execute("shape.rectangle", &json!({"x": 0, "y": 0, "colour": 4}))
        .is_err());
    assert!(e.doc().objects.is_empty());
}
