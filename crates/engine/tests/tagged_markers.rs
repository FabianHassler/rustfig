//! Tagged (selected) objects show filled markers in the editing modes, as
//! xfig's `toggle_*highlight` draws them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::geom::Point;
use rustfig_engine::{Button, Engine, ToolEvent, ToolKind};
use serde_json::json;

fn click(e: &mut Engine, x: f64, y: f64) {
    let pos = Point::new(x, y);
    let button = Button::Left;
    e.tool_event(ToolEvent::Press {
        pos,
        button,
        shift: false,
    })
    .unwrap();
    e.tool_event(ToolEvent::Release { pos, button }).unwrap();
}

#[test]
fn glue_tags_fill_the_markers() {
    let mut e = Engine::new();
    let rect = json!({"x": 0, "y": 0, "width": 1200, "height": 600});
    e.execute("shape.rectangle", &rect).unwrap();
    let far = json!({"x": 3000, "y": 0, "width": 1200, "height": 600});
    e.execute("shape.rectangle", &far).unwrap();
    e.set_selection(Vec::new());
    e.set_tool(ToolKind::Glue);
    assert!(e.overlay().tagged.is_empty());
    click(&mut e, 0.0, 300.0);
    let ov = e.overlay();
    assert!(!ov.tagged.is_empty());
    assert_eq!(
        ov.tagged.len() * 2,
        ov.markers.len(),
        "only the tagged box's are filled"
    );
    assert!(ov.tagged.iter().all(|p| p.x <= 1200.0));
    // Clicking it again untags it.
    click(&mut e, 0.0, 300.0);
    assert!(e.overlay().tagged.is_empty());
    // The selection stays filled in the other editing modes too.
    e.execute("edit.select_all", &json!({})).unwrap();
    e.set_tool(ToolKind::Move);
    let ov = e.overlay();
    assert_eq!(ov.tagged.len(), ov.markers.len());
}

#[test]
fn only_the_selecting_modes_tag() {
    let mut e = Engine::new();
    // Drawing a box with the mouse doesn't tag it.
    e.set_tool(ToolKind::Box);
    let (a, b) = (Point::new(0.0, 0.0), Point::new(1200.0, 600.0));
    let button = Button::Left;
    e.tool_event(ToolEvent::Press {
        pos: a,
        button,
        shift: false,
    })
    .unwrap();
    e.tool_event(ToolEvent::Move { pos: b }).unwrap();
    e.tool_event(ToolEvent::Release { pos: b, button }).unwrap();
    assert_eq!(e.doc().objects.len(), 1);
    assert!(e.selection().is_empty());
    // Nor does updating or editing it.
    e.set_tool(ToolKind::Update);
    click(&mut e, 0.0, 300.0);
    e.set_tool(ToolKind::Edit);
    click(&mut e, 0.0, 300.0);
    assert!(e.selection().is_empty());
    e.set_tool(ToolKind::Glue);
    assert!(e.overlay().tagged.is_empty(), "nothing is tagged when Glue starts");
    // A selection made on purpose survives the other modes.
    click(&mut e, 0.0, 300.0);
    let tagged = e.selection().to_vec();
    assert_eq!(tagged.len(), 1);
    e.set_tool(ToolKind::FlipH);
    click(&mut e, 0.0, 300.0);
    assert_eq!(e.selection(), tagged.as_slice());
    e.set_tool(ToolKind::Copy);
    click(&mut e, 0.0, 300.0);
    click(&mut e, 0.0, 3000.0);
    assert_eq!(e.doc().objects.len(), 2, "copied");
    assert_eq!(e.selection(), tagged.as_slice());
}
