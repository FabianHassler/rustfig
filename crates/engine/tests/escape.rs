#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Escape, one level per press: cancel, clear, leave the mode.

use rustfig_engine::geom::Point;
use rustfig_engine::{Button, Engine, Key, ToolEvent, ToolKind};
use serde_json::json;

fn click(e: &mut Engine, x: f64, y: f64) {
    let pos = Point::new(x, y);
    e.tool_event(ToolEvent::Press {
        pos,
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    e.tool_event(ToolEvent::Release {
        pos,
        button: Button::Left,
    })
    .unwrap();
}

fn escape(e: &mut Engine) -> String {
    let r = e.execute("tool.escape", &json!({})).unwrap();
    r["done"].as_str().unwrap().to_string()
}

#[test]
fn escape_cancels_then_clears_then_leaves_the_mode() {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1200, "height": 600}),
    )
    .unwrap();
    e.execute("edit.select_all", &json!({})).unwrap();
    e.set_tool(ToolKind::Polyline);
    click(&mut e, 0.0, 2000.0);
    click(&mut e, 600.0, 2000.0);
    assert!(e.tool_busy());
    let n = e.doc().objects.len();
    assert_eq!(escape(&mut e), "cancelled");
    assert!(!e.tool_busy());
    assert_eq!(e.doc().objects.len(), n, "the line in progress is dropped");
    assert_eq!(escape(&mut e), "selection cleared");
    assert!(e.selection().is_empty());
    assert_eq!(escape(&mut e), "mode left");
    assert_eq!(e.tool(), ToolKind::None);
    assert_eq!(escape(&mut e), "nothing");
}

#[test]
fn backspace_removes_the_last_point() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::Polyline);
    click(&mut e, 0.0, 0.0);
    click(&mut e, 600.0, 0.0);
    e.tool_event(ToolEvent::Key(Key::Backspace)).unwrap();
    assert!(e.tool_busy(), "one point is left");
    e.tool_event(ToolEvent::Key(Key::Backspace)).unwrap();
    assert!(!e.tool_busy());
}

#[test]
fn escape_clears_the_chop_axes_before_leaving() {
    let mut e = Engine::new();
    e.execute("shape.polyline", &json!({"points": [[0, 0], [1200, 0]]}))
        .unwrap();
    e.set_selection(Vec::new());
    e.set_tool(ToolKind::Chop);
    click(&mut e, 600.0, 0.0);
    assert_eq!(escape(&mut e), "axes cleared");
    assert_eq!(escape(&mut e), "mode left");
}

#[test]
fn escape_is_the_key() {
    let esc = rustfig_engine::cmd::find("tool.escape").unwrap();
    assert_eq!(esc.shortcut, "Escape");
}
