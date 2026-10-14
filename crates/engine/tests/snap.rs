#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! xfig's Snap menu through the tools: the next clicked point snaps to an
//! object feature, Hold keeps the mode, a failed snap ignores the click.

use rustfig_engine::geom::Point;
use rustfig_engine::model::ObjectKind;
use rustfig_engine::{Button, Engine, ToolEvent, ToolKind};
use serde_json::json;

fn click(e: &mut Engine, x: f64, y: f64, button: Button) {
    let pos = Point::new(x, y);
    e.tool_event(ToolEvent::Press {
        pos,
        button,
        shift: false,
    })
    .unwrap();
    e.tool_event(ToolEvent::Release { pos, button }).unwrap();
}

fn snap(e: &mut Engine, mode: &str) {
    e.execute("snap.mode", &json!({ "mode": mode })).unwrap();
}

fn points(e: &Engine, i: usize) -> Vec<Point> {
    e.doc().objects[i].points().unwrap().to_vec()
}

#[test]
fn polyline_to_an_endpoint_is_exact_and_the_mode_ends() {
    let mut e = Engine::new();
    // An off-grid corner.
    e.execute("shape.polyline", &json!({"points": [[1013, 1013], [3000, 1013]]}))
        .unwrap();
    e.set_tool(ToolKind::Polyline);
    click(&mut e, 0.0, 0.0, Button::Left);
    snap(&mut e, "endpoint");
    assert_eq!(e.snap_indicator(), "Endpoint");
    click(&mut e, 1000.0, 1030.0, Button::Left);
    assert_eq!(e.snap_indicator(), "None", "the snap is used up");
    click(&mut e, 0.0, 2400.0, Button::Middle);
    assert_eq!(points(&e, 1)[1], Point::new(1013.0, 1013.0), "not rounded to the grid");
}

#[test]
fn move_ellipse_focus_onto_a_segment_midpoint() {
    // The manual's example.
    let mut e = Engine::new();
    e.execute("shape.polyline", &json!({"points": [[0, 3000], [2400, 3000]]}))
        .unwrap();
    e.execute("shape.ellipse", &json!({"cx": 1200, "cy": 1200, "rx": 500, "ry": 300}))
        .unwrap();
    e.set_tool(ToolKind::Move);
    snap(&mut e, "focus");
    click(&mut e, 1600.0, 1380.0, Button::Left); // on the ellipse near its right focus
    snap(&mut e, "midpoint");
    click(&mut e, 900.0, 3000.0, Button::Left);
    let ObjectKind::Ellipse(el) = &e.doc().objects[1].kind else {
        panic!()
    };
    // Right focus (1600, 1200) moved to (1200, 3000).
    assert_eq!(el.center, Point::new(800.0, 3000.0));
}

#[test]
fn hold_release_and_failures() {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1200, "height": 1200}),
    )
    .unwrap();
    e.set_tool(ToolKind::Polyline);
    e.execute("snap.hold", &json!({})).unwrap();
    snap(&mut e, "endpoint");
    click(&mut e, 50.0, 30.0, Button::Left);
    click(&mut e, 1150.0, 40.0, Button::Left);
    assert_eq!(e.snap_indicator(), "Endpoint", "held");
    // Nothing near: the click is ignored, the line keeps two points.
    click(&mut e, 6000.0, 6000.0, Button::Left);
    assert_eq!(e.status(), "No point found.");
    click(&mut e, 1180.0, 1180.0, Button::Middle);
    assert_eq!(
        points(&e, 1),
        vec![Point::ZERO, Point::new(1200.0, 0.0), Point::new(1200.0, 1200.0)]
    );
    e.execute("snap.release", &json!({})).unwrap();
    assert_eq!(e.snap_indicator(), "None");
    assert!(e.execute("snap.mode", &json!({"mode": "sideways"})).is_err());
}

#[test]
fn intersect_takes_two_objects() {
    let mut e = Engine::new();
    e.execute("shape.polyline", &json!({"points": [[0, 0], [2400, 2400]]}))
        .unwrap();
    e.execute("shape.polyline", &json!({"points": [[0, 2400], [2400, 0]]}))
        .unwrap();
    e.set_tool(ToolKind::Polyline);
    click(&mut e, 0.0, 1200.0 * 3.0, Button::Left);
    snap(&mut e, "intersect");
    click(&mut e, 300.0, 300.0, Button::Left);
    assert_eq!(e.snap_indicator(), "I'sect 2");
    click(&mut e, 2100.0, 300.0, Button::Left);
    click(&mut e, 4000.0, 4000.0, Button::Middle);
    assert_eq!(points(&e, 2)[1], Point::new(1200.0, 1200.0));
}

#[test]
fn tangent_from_the_current_point() {
    let mut e = Engine::new();
    e.execute("shape.circle", &json!({"cx": 0, "cy": 0, "r": 600})).unwrap();
    e.set_tool(ToolKind::Polyline);
    click(&mut e, 2400.0, 0.0, Button::Left);
    snap(&mut e, "tangent");
    click(&mut e, 300.0, -600.0, Button::Left);
    click(&mut e, 300.0, -600.0, Button::Middle);
    let p = points(&e, 1)[1];
    assert!((p.len() - 600.0).abs() <= 1.0, "{p:?} on the circle");
    assert!(p.y < 0.0);
}
