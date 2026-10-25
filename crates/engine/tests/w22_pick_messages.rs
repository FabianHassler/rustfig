#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! W22: xfig's pick order at equal depth (`u_search.c`) and live messages
//! (`w_msgpanel.c`, `d_text.c`, `w_cmdpanel.c`).

use rustfig_engine::geom::Point;
use rustfig_engine::{Button, Engine, ToolEvent, ToolKind};
use serde_json::json;

fn two_crossing_lines() -> Engine {
    let mut e = Engine::new();
    e.execute(
        "shape.polyline",
        &json!({"points": [[0, 600], [2400, 600]], "depth": 50}),
    )
    .unwrap();
    e.execute(
        "shape.polyline",
        &json!({"points": [[1200, 0], [1200, 1200]], "depth": 50}),
    )
    .unwrap();
    e
}

#[test]
fn equal_depth_picks_the_first_object_shift_the_last() {
    let e = two_crossing_lines();
    let first = e.doc().objects[0].id;
    let at = Point::new(1200.0, 600.0);
    assert_eq!(e.hit_test(at, 30.0), Some(first));
    // Shift on a press that picks (Text mode) takes the last one.
    let mut e = Engine::new();
    for t in ["A", "B"] {
        e.execute("shape.text", &json!({"x": 600, "y": 600, "text": t, "depth": 50}))
            .unwrap();
    }
    e.set_tool(ToolKind::Text);
    let p = Point::new(610.0, 590.0);
    for (shift, want) in [(false, "A"), (true, "B")] {
        e.tool_event(ToolEvent::Press {
            pos: p,
            button: Button::Left,
            shift,
        })
        .unwrap();
        e.tool_event(ToolEvent::Text("!".into())).unwrap();
        e.tool_event(ToolEvent::Key(rustfig_engine::Key::Escape)).unwrap();
        let texts: Vec<String> = e
            .doc()
            .objects
            .iter()
            .filter_map(|o| match &o.kind {
                rustfig_engine::model::ObjectKind::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t.contains(want) && t.contains('!')),
            "{shift}: {texts:?}"
        );
    }
}

#[test]
fn lower_depth_still_wins() {
    let mut e = Engine::new();
    e.execute(
        "shape.polyline",
        &json!({"points": [[0, 600], [2400, 600]], "depth": 50}),
    )
    .unwrap();
    e.execute(
        "shape.polyline",
        &json!({"points": [[1200, 0], [1200, 1200]], "depth": 40}),
    )
    .unwrap();
    let front = e.doc().objects[1].id;
    assert_eq!(e.hit_test(Point::new(1200.0, 600.0), 30.0), Some(front));
}

#[test]
fn box_message_has_a_factor() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::Box);
    let p = |x, y| Point::new(x, y);
    e.tool_event(ToolEvent::Press {
        pos: p(600.0, 600.0),
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    e.tool_event(ToolEvent::Move { pos: p(1800.0, 1200.0) }).unwrap();
    assert!(e.status().contains(", Factor = "), "{}", e.status());
}

#[test]
fn regular_polygon_message_is_a_length() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::RegularPolygon);
    e.tool_event(ToolEvent::Press {
        pos: Point::new(600.0, 600.0),
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    e.tool_event(ToolEvent::Move {
        pos: Point::new(1800.0, 1200.0),
    })
    .unwrap();
    let s = e.status();
    assert!(
        s.starts_with("Length = ") && s.contains("dx = ") && s.ends_with(" deg"),
        "{s}"
    );
}

#[test]
fn text_click_and_delete_all_messages() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::Text);
    e.tool_event(ToolEvent::Press {
        pos: Point::new(600.0, 600.0),
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    assert_eq!(e.status(), "Ready for text input (from keyboard)");
    let mut e = two_crossing_lines();
    e.execute("edit.delete_all", &json!({})).unwrap();
    assert_eq!(e.status(), "Immediate Undo will restore the figure");
}
