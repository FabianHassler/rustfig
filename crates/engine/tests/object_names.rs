#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! F18: object names kept in the file as `rustfig-name:` comment lines.

use rustfig_engine::Engine;
use serde_json::{json, Value};

fn rect(e: &mut Engine, x: i64) -> u64 {
    e.execute(
        "shape.rectangle",
        &json!({"x": x, "y": 600, "width": 300, "height": 300}),
    )
    .unwrap()["id"]
        .as_u64()
        .unwrap()
}

fn name(e: &mut Engine, id: u64, n: &str) -> Value {
    e.execute("object.set_attributes", &json!({"ids": [id], "name": n}))
        .unwrap()
}

fn listed(e: &mut Engine) -> Vec<Value> {
    e.execute("object.list", &json!({"typeset": false})).unwrap()["objects"]
        .as_array()
        .unwrap()
        .clone()
}

fn x_of(e: &mut Engine, id: u64) -> f64 {
    let l = listed(e);
    l.iter().find(|o| o["id"] == id).unwrap()["bounds"]["x"]
        .as_f64()
        .unwrap()
}

fn reopen(e: &mut Engine) {
    let data = e.execute("file.save_data", &json!({"format": "fig"})).unwrap()["data"]
        .as_str()
        .unwrap()
        .to_string();
    e.execute("file.open_data", &json!({"data": data})).unwrap();
}

#[test]
fn name_is_listed_and_survives_the_file() {
    let mut e = Engine::new();
    let a = rect(&mut e, 100);
    rect(&mut e, 600);
    name(&mut e, a, "  legend ");
    let l = listed(&mut e);
    assert_eq!(l[0]["name"], "legend");
    assert!(l[1].get("name").is_none());
    reopen(&mut e);
    let l = listed(&mut e);
    assert_eq!(l.iter().filter(|o| o["name"] == "legend").count(), 1);
    let id = l.iter().find(|o| o["name"] == "legend").unwrap()["id"]
        .as_u64()
        .unwrap();
    let before = x_of(&mut e, id);
    let r = e
        .execute("transform.move", &json!({"names": ["legend"], "dx": 50, "dy": 0}))
        .unwrap();
    assert!(r.get("warnings").is_none(), "{r}");
    assert_eq!(x_of(&mut e, id), before + 50.0);
}

#[test]
fn names_union_with_ids_and_copies_answer() {
    let mut e = Engine::new();
    let a = rect(&mut e, 100);
    let b = rect(&mut e, 600);
    name(&mut e, a, "box");
    name(&mut e, b, "box");
    let (xa, xb) = (x_of(&mut e, a), x_of(&mut e, b));
    e.execute(
        "transform.move",
        &json!({"ids": [a], "names": ["box", "box"], "dx": 10, "dy": 0}),
    )
    .unwrap();
    assert_eq!(x_of(&mut e, a), xa + 10.0);
    assert_eq!(x_of(&mut e, b), xb + 10.0);
    name(&mut e, a, "");
    e.execute("object.set_attributes", &json!({"names": ["box"], "thickness": 3}))
        .unwrap();
    assert_eq!(x_of(&mut e, a), xa + 10.0);
}

#[test]
fn unknown_name_errors_and_changes_nothing() {
    let mut e = Engine::new();
    let a = rect(&mut e, 100);
    name(&mut e, a, "one");
    let before = e.execute("file.save_data", &json!({"format": "fig"})).unwrap();
    let r = e.execute(
        "transform.move",
        &json!({"ids": [a], "names": ["one", "two"], "dx": 10, "dy": 0}),
    );
    let err = r.unwrap_err().to_string();
    assert!(err.contains("two"), "{err}");
    assert_eq!(before, e.execute("file.save_data", &json!({"format": "fig"})).unwrap());
    assert!(e
        .execute("transform.move", &json!({"names": "one", "dx": 1, "dy": 0}))
        .is_err());
    assert!(e
        .execute("transform.move", &json!({"names": [1], "dx": 1, "dy": 0}))
        .is_err());
}

#[test]
fn removing_the_name_keeps_other_comments() {
    let mut e = Engine::new();
    let a = rect(&mut e, 100);
    e.execute("object.set_attributes", &json!({"ids": [a], "comment": "my note"}))
        .unwrap();
    name(&mut e, a, "first");
    name(&mut e, a, "second");
    let get = |e: &mut Engine| e.execute("object.get", &json!({"ids": [a]})).unwrap()["objects"][0]["comments"].clone();
    assert_eq!(get(&mut e), json!(["rustfig-name: second", "my note"]));
    // Setting the comment again keeps the name.
    e.execute("object.set_attributes", &json!({"ids": [a], "comment": "other"}))
        .unwrap();
    assert_eq!(get(&mut e), json!(["rustfig-name: second", "other"]));
    name(&mut e, a, "");
    assert_eq!(get(&mut e), json!(["other"]));
    assert!(listed(&mut e)[0].get("name").is_none());
}

#[test]
fn bad_names_are_rejected() {
    let mut e = Engine::new();
    let a = rect(&mut e, 100);
    for bad in [json!("a\nb"), json!("x".repeat(101)), json!(5)] {
        assert!(e
            .execute("object.set_attributes", &json!({"ids": [a], "name": bad}))
            .is_err());
    }
    assert!(listed(&mut e)[0].get("name").is_none());
    name(&mut e, a, &"x".repeat(100));
}
