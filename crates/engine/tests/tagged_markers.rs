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

/// Every mode that waits for an object click shows the markers of the
/// objects it can act on (xfig's `update_markers`), and so does Copy as
/// picture while it waits.
#[test]
fn every_picking_mode_shows_markers() {
    use rustfig_engine::ToolKind;
    let fresh = || {
        let mut e = Engine::new();
        for (c, p) in [
            (
                "shape.polyline",
                json!({"points": [[1200, 1200], [3600, 1200], [3600, 3000]]}),
            ),
            (
                "shape.spline",
                json!({"points": [[1200, 4800], [2400, 6000], [3600, 4800]], "kind": "x"}),
            ),
            (
                "shape.rectangle",
                json!({"x": 6000, "y": 1200, "width": 1200, "height": 600}),
            ),
            (
                "shape.arc",
                json!({"points": [[6000, 6000], [6500, 6500], [7000, 6000]]}),
            ),
            ("shape.circle", json!({"cx": 6000, "cy": 4000, "r": 500})),
            ("shape.circle", json!({"cx": 8000, "cy": 4000, "r": 500})),
        ] {
            e.execute(c, &p).unwrap();
        }
        let ids: Vec<u64> = e.doc().objects.iter().rev().take(2).map(|o| o.id.0).collect();
        e.execute("object.group", &json!({ "ids": ids })).unwrap();
        e.execute("shape.text", &json!({"x": 100, "y": 7000, "text": "hi"}))
            .unwrap();
        e.set_selection(vec![]);
        e
    };
    for &t in ToolKind::ALL.iter() {
        if t.is_drawing() || t == ToolKind::None {
            continue;
        }
        let mut e = fresh();
        e.set_tool(t);
        assert!(!e.overlay().markers.is_empty(), "{t:?} shows no markers");
    }
    // Copy as picture's markers do not need a clipboard to show.
    let mut e = fresh();
    e.set_tool(ToolKind::Polyline);
    assert!(e.overlay().markers.is_empty());
}
