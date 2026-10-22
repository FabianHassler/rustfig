#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The Trim, split and extend mode (plan D16): left trims the clicked piece,
//! middle splits at the point, right extends the nearest line end; the other
//! objects are the boundaries; Shift+click locates.

use rustfig_engine::geom::Point;
use rustfig_engine::{Button, Engine, SnapMode, ToolEvent, ToolKind};
use serde_json::{json, Value};

fn engine() -> Engine {
    let mut e = Engine::new();
    e.indicator.snap = SnapMode::Any;
    e.set_tool(ToolKind::Trim);
    e
}

fn line(e: &mut Engine, v: &[(f64, f64)]) {
    let pts: Vec<Value> = v.iter().map(|(x, y)| json!([x, y])).collect();
    e.execute("shape.polyline", &json!({"points": pts})).unwrap();
}

fn press(e: &mut Engine, x: f64, y: f64, button: Button, shift: bool) {
    let pos = Point::new(x, y);
    e.tool_event(ToolEvent::Press { pos, button, shift }).unwrap();
    e.tool_event(ToolEvent::Release { pos, button }).unwrap();
}

fn count(e: &Engine) -> usize {
    e.doc().objects.len()
}

#[test]
fn the_mode_has_its_command_key_and_hints() {
    let e = engine();
    assert_eq!(ToolKind::Trim.id(), "tool.trim");
    assert_eq!(ToolKind::Trim.shortcut(), "w");
    let h = e.mouse_hints();
    assert_eq!(
        [h.left, h.middle, h.right],
        ["trim piece", "split at point", "extend nearest end"]
    );
    assert!(ToolKind::Trim.locates());
}

#[test]
fn left_trims_the_piece_of_a_line_between_two_crossings() {
    let mut e = engine();
    line(&mut e, &[(0.0, 0.0), (3000.0, 0.0)]);
    line(&mut e, &[(1000.0, -500.0), (1000.0, 500.0)]);
    line(&mut e, &[(2000.0, -500.0), (2000.0, 500.0)]);
    press(&mut e, 1500.0, 0.0, Button::Left, false);
    // Two pieces of the horizontal line and the two crossing lines.
    assert_eq!(count(&e), 4);
    e.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(count(&e), 3, "one undo step");
}

#[test]
fn left_trims_an_arc_and_a_spline() {
    let mut e = engine();
    e.execute("shape.arc", &json!({"points": [[0, 0], [1000, -1000], [2000, 0]]}))
        .unwrap();
    line(&mut e, &[(500.0, -2000.0), (500.0, 500.0)]);
    line(&mut e, &[(1500.0, -2000.0), (1500.0, 500.0)]);
    press(&mut e, 1000.0, -1000.0, Button::Left, false);
    assert!(count(&e) >= 4, "arc pieces remain: {}", count(&e));

    let mut e = engine();
    e.execute(
        "shape.spline",
        &json!({"points": [[0, 0], [1000, 600], [2000, 0], [3000, 600]], "closed": false, "kind": "x"}),
    )
    .unwrap();
    line(&mut e, &[(700.0, -800.0), (700.0, 1500.0)]);
    line(&mut e, &[(2300.0, -800.0), (2300.0, 1500.0)]);
    let before = count(&e);
    press(&mut e, 1500.0, 300.0, Button::Left, false);
    assert!(count(&e) > before, "the spline became pieces: {}", count(&e));
}

#[test]
fn middle_splits_a_line_an_arc_and_a_spline() {
    let mut e = engine();
    line(&mut e, &[(0.0, 0.0), (2000.0, 0.0)]);
    press(&mut e, 1000.0, 0.0, Button::Middle, false);
    assert_eq!(count(&e), 2);

    let mut e = engine();
    e.execute("shape.arc", &json!({"points": [[1500, 0], [1000, 500], [500, 0]]}))
        .unwrap();
    press(&mut e, 1000.0, 500.0, Button::Middle, false);
    assert_eq!(count(&e), 2);

    let mut e = engine();
    e.execute(
        "shape.spline",
        &json!({"points": [[0, 1000], [800, 0], [1600, 1000]], "closed": false, "kind": "x"}),
    )
    .unwrap();
    press(&mut e, 800.0, 300.0, Button::Middle, false);
    assert_eq!(count(&e), 2);
}

#[test]
fn right_extends_the_nearest_end_to_the_boundary() {
    let mut e = engine();
    line(&mut e, &[(0.0, 0.0), (1000.0, 0.0)]);
    line(&mut e, &[(2000.0, -500.0), (2000.0, 500.0)]);
    press(&mut e, 900.0, 0.0, Button::Right, false);
    let o = &e.doc().objects[0];
    let text = serde_json::to_string(o).unwrap();
    assert!(text.contains("2000"), "the end reached the boundary: {text}");
}

#[test]
fn shift_click_only_locates() {
    let mut e = engine();
    line(&mut e, &[(0.0, 0.0), (2000.0, 0.0)]);
    let before = serde_json::to_string(e.doc()).unwrap();
    for b in [Button::Left, Button::Middle, Button::Right] {
        press(&mut e, 1000.0, 0.0, b, true);
    }
    assert_eq!(serde_json::to_string(e.doc()).unwrap(), before);
    assert_eq!(e.mouse_hints_with(true).left, "locate object");
}

#[test]
fn a_click_on_nothing_changes_nothing() {
    let mut e = engine();
    line(&mut e, &[(0.0, 0.0), (2000.0, 0.0)]);
    for b in [Button::Left, Button::Middle, Button::Right] {
        press(&mut e, 1000.0, 5000.0, b, false);
    }
    assert_eq!(count(&e), 1);
}
