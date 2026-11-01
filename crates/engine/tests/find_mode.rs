//! Find mode: the match set is tool state; criteria fill it, a left click
//! adds or removes an object, `find.apply` acts on all of it, leaving the
//! mode clears it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::tools::{Button, ToolEvent, ToolKind};
use rustfig_engine::Engine;
use rustfig_geom::Point;
use serde_json::{json, Value};

/// Three boxes at x = 0, 2000, 4000 (one red) and a text.
fn engine() -> Engine {
    let mut e = Engine::new();
    for (i, pen) in [0, 4, 0].into_iter().enumerate() {
        e.execute(
            "shape.rectangle",
            &json!({"x": i * 2000, "y": 0, "width": 600, "height": 600, "pen_color": pen}),
        )
        .unwrap();
    }
    e.execute("shape.text", &json!({"x": 0, "y": 2000, "text": "hello", "depth": 40}))
        .unwrap();
    e
}

fn press(e: &mut Engine, x: f64, y: f64, button: Button) {
    e.tool_event(ToolEvent::Press {
        pos: Point::new(x, y),
        button,
        shift: false,
    })
    .unwrap();
}

fn state(e: &mut Engine) -> Value {
    e.execute("find.state", &json!({})).unwrap()
}

fn count(e: &mut Engine) -> u64 {
    state(e)["count"].as_u64().unwrap()
}

#[test]
fn entering_starts_empty_and_criteria_fill_the_set() {
    let mut e = engine();
    e.execute("tool.find", &json!({})).unwrap();
    assert_eq!(e.tool(), ToolKind::Find);
    assert_eq!(count(&mut e), 0);
    assert_eq!(state(&mut e)["mode"], json!(true));
    let r = e.execute("find.set", &json!({"kind": "box"})).unwrap();
    assert_eq!(r["count"], json!(3));
    e.execute("find.set", &json!({"pen_color": 4})).unwrap();
    assert_eq!(count(&mut e), 1);
}

#[test]
fn find_set_without_criteria_is_empty_and_inside_alone_is_none() {
    let mut e = engine();
    e.execute("find.set", &json!({"kind": "box"})).unwrap();
    assert_eq!(count(&mut e), 3);
    e.execute("find.set", &json!({})).unwrap();
    assert_eq!(count(&mut e), 0);
    e.execute("find.set", &json!({"kind": "box"})).unwrap();
    e.execute("find.set", &json!({"inside": true, "text": null})).unwrap();
    assert_eq!(count(&mut e), 0);
    // edit.find_objects keeps meaning "all" without criteria.
    let r = e.execute("edit.find_objects", &json!({})).unwrap();
    assert!(r["count"].as_u64().unwrap() > 3);
}

#[test]
fn find_set_enters_the_mode() {
    let mut e = engine();
    e.execute("find.set", &json!({"kind": "text"})).unwrap();
    assert_eq!(e.tool(), ToolKind::Find);
    assert_eq!(count(&mut e), 1);
}

#[test]
fn left_click_toggles_other_buttons_do_nothing() {
    let mut e = engine();
    e.execute("tool.find", &json!({})).unwrap();
    // The first box's edge, the second's edge.
    press(&mut e, 0.0, 300.0, Button::Left);
    assert_eq!(count(&mut e), 1);
    press(&mut e, 2000.0, 300.0, Button::Left);
    assert_eq!(count(&mut e), 2);
    press(&mut e, 0.0, 300.0, Button::Left);
    assert_eq!(count(&mut e), 1);
    press(&mut e, 4000.0, 300.0, Button::Middle);
    press(&mut e, 4000.0, 300.0, Button::Right);
    assert_eq!(count(&mut e), 1);
    // A click on nothing changes nothing; the drawing is untouched.
    press(&mut e, 9000.0, 9000.0, Button::Left);
    assert_eq!(count(&mut e), 1);
    assert_eq!(e.doc().objects.len(), 4);
}

#[test]
fn toggle_command_and_errors() {
    let mut e = engine();
    let id = e.doc().objects[0].id.0;
    assert!(
        e.execute("find.toggle", &json!({"id": id})).is_err(),
        "not in Find mode"
    );
    e.execute("tool.find", &json!({})).unwrap();
    assert_eq!(e.execute("find.toggle", &json!({"id": id})).unwrap()["count"], json!(1));
    assert_eq!(e.execute("find.toggle", &json!({"id": id})).unwrap()["count"], json!(0));
    assert!(e.execute("find.toggle", &json!({"id": 999_999})).is_err());
    assert!(e.execute("find.toggle", &json!({"id": -1})).is_err());
    assert!(
        e.execute("find.apply", &json!({"action": "delete"})).is_err(),
        "no matches"
    );
    assert!(e.execute("find.apply", &json!({})).is_err());
}

#[test]
fn actions_are_one_undo_step_and_the_set_follows() {
    let mut e = engine();
    e.execute("find.set", &json!({"kind": "box"})).unwrap();
    let before = e.doc().objects.len();
    // Depth.
    assert!(e
        .execute("find.apply", &json!({"action": "depth", "depth": 1000}))
        .is_err());
    e.execute("find.apply", &json!({"action": "depth", "depth": 7}))
        .unwrap();
    let sevens = e.doc().objects.iter().filter(|o| o.depth() == 7).count();
    assert_eq!(sevens, 3);
    assert_eq!(count(&mut e), 3);
    e.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(e.doc().objects.iter().filter(|o| o.depth() == 7).count(), 0);
    // Update with the indicator's pen colour.
    e.execute("indicator.set", &json!({"pen_color": 2})).unwrap();
    e.execute("find.apply", &json!({"action": "update"})).unwrap();
    assert_eq!(count(&mut e), 3);
    e.execute("edit.undo", &json!({})).unwrap();
    // Glue: the compound is the only match.
    let r = e.execute("find.apply", &json!({"action": "group"})).unwrap();
    assert_eq!(r["count"], json!(3));
    assert_eq!(e.doc().objects.len(), before - 2);
    let s = state(&mut e);
    assert_eq!(s["count"], json!(1));
    assert_eq!(s["items"][0]["type"], json!("compound"));
    assert!(
        e.execute("find.apply", &json!({"action": "group"})).is_err(),
        "needs two"
    );
    // Delete: the set is empty and one undo brings the drawing back.
    e.execute("find.apply", &json!({"action": "delete"})).unwrap();
    assert_eq!(count(&mut e), 0);
    assert_eq!(e.doc().objects.len(), 1);
    e.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(e.doc().objects.len(), 2);
}

#[test]
fn undo_drops_vanished_objects_from_the_set() {
    let mut e = engine();
    e.execute("find.set", &json!({"kind": "text"})).unwrap();
    e.execute("find.apply", &json!({"action": "delete"})).unwrap();
    e.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(count(&mut e), 0);
}

#[test]
fn leaving_clears_the_set() {
    for leave in ["tool.box", "tool.circle_radius", "tool.escape"] {
        let mut e = engine();
        e.execute("find.set", &json!({"kind": "box"})).unwrap();
        assert_eq!(count(&mut e), 3);
        e.execute(leave, &json!({})).unwrap();
        assert_ne!(e.tool(), ToolKind::Find, "{leave}");
        assert_eq!(count(&mut e), 0, "{leave}");
        assert_eq!(state(&mut e)["mode"], json!(false));
        // Coming back starts empty.
        e.execute("tool.find", &json!({})).unwrap();
        assert_eq!(count(&mut e), 0, "{leave}");
    }
}

#[test]
fn mouse_hints_describe_what_the_buttons_do() {
    let mut e = engine();
    e.execute("tool.find", &json!({})).unwrap();
    let h = e.mouse_hints();
    assert_eq!((h.left, h.middle, h.right), ("add/remove object", "", ""));
    let s = e.mouse_hints_with(true);
    assert_eq!((s.left, s.middle, s.right), ("locate object", "", ""));
    // No button in xfig's mode panel: the 40 stay.
    assert_eq!(ToolKind::Find.id(), "tool.find");
}

#[test]
fn shift_left_locates_and_the_next_click_acts_on_the_located_object() {
    let mut e = engine();
    e.execute("tool.find", &json!({})).unwrap();
    e.tool_event(ToolEvent::Press {
        pos: Point::new(0.0, 300.0),
        button: Button::Left,
        shift: true,
    })
    .unwrap();
    assert_eq!(count(&mut e), 0, "Shift+left only locates");
    press(&mut e, 0.0, 300.0, Button::Left);
    assert_eq!(count(&mut e), 1);
}
