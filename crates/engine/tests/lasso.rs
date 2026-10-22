#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Lasso in every region selection (plan W15): pressing the middle or right
//! button and dragging draws a freehand lasso; a click, move, click keeps
//! xfig's rectangle.

use rustfig_engine::geom::Point;
use rustfig_engine::model::ObjectKind;
use rustfig_engine::{Button, Engine, SnapMode, ToolEvent, ToolKind};
use serde_json::{json, Value};

fn engine() -> Engine {
    let mut e = Engine::new();
    e.indicator.snap = SnapMode::Any;
    e
}

fn ev(e: &mut Engine, ev: ToolEvent) {
    e.tool_event(ev).unwrap();
}

/// Press at the first point, move along the path, release at the last.
fn lasso(e: &mut Engine, button: Button, path: &[(f64, f64)]) {
    let p = |q: &(f64, f64)| Point::new(q.0, q.1);
    ev(
        e,
        ToolEvent::Press {
            pos: p(&path[0]),
            button,
            shift: false,
        },
    );
    for q in &path[1..] {
        ev(e, ToolEvent::Move { pos: p(q) });
    }
    ev(
        e,
        ToolEvent::Release {
            pos: p(&path[path.len() - 1]),
            button,
        },
    );
}

fn click(e: &mut Engine, x: f64, y: f64, button: Button) {
    let pos = Point::new(x, y);
    ev(
        e,
        ToolEvent::Press {
            pos,
            button,
            shift: false,
        },
    );
    ev(e, ToolEvent::Release { pos, button });
}

fn rect(e: &mut Engine, x: i32, y: i32, w: i32, h: i32) {
    e.execute("shape.rectangle", &json!({"x": x, "y": y, "width": w, "height": h}))
        .unwrap();
}

fn snapshot(e: &Engine) -> Value {
    serde_json::to_value(e.doc()).unwrap()
}

/// A square path around (-200,-200)-(2500,1000), as a rectangle drag would
/// cover; it takes boxes 0 and 1 and not 2 (partly inside) or 3 (outside).
const SQUARE: [(f64, f64); 5] = [
    (-200.0, -200.0),
    (2500.0, -200.0),
    (2500.0, 1000.0),
    (-200.0, 1000.0),
    (-200.0, 900.0),
];

/// Boxes 0 and 1 inside the lasso, 2 sticking out of it, 3 far away.
fn scene() -> Engine {
    let mut e = engine();
    rect(&mut e, 0, 0, 1000, 500);
    rect(&mut e, 1500, 0, 500, 500);
    rect(&mut e, 2200, 200, 800, 500);
    rect(&mut e, 5000, 5000, 500, 500);
    e.set_selection(Vec::new());
    e
}

fn thickness(e: &Engine, i: usize) -> i32 {
    match &e.doc().objects[i].kind {
        ObjectKind::Polyline(p) => p.style.thickness,
        _ => panic!("not a polyline"),
    }
}

#[test]
fn glue_tag_region_by_lasso_equals_rectangle() {
    let mut a = scene();
    a.set_tool(ToolKind::Glue);
    lasso(&mut a, Button::Middle, &SQUARE);
    let mut b = scene();
    b.set_tool(ToolKind::Glue);
    click(&mut b, -200.0, -200.0, Button::Middle);
    click(&mut b, 2500.0, 1000.0, Button::Middle);
    assert_eq!(a.selection().len(), 2);
    assert_eq!(a.selection(), b.selection());
    assert!(!a.tool_busy());
}

#[test]
fn glue_lasso_shows_overlay_and_hints() {
    let mut e = scene();
    e.set_tool(ToolKind::Glue);
    assert_eq!(e.mouse_hints().middle, "tag region");
    ev(
        &mut e,
        ToolEvent::Press {
            pos: Point::new(-200.0, -200.0),
            button: Button::Middle,
            shift: false,
        },
    );
    for q in &SQUARE[1..4] {
        ev(
            &mut e,
            ToolEvent::Move {
                pos: Point::new(q.0, q.1),
            },
        );
    }
    let ov = e.overlay();
    assert_eq!(ov.guides.len(), 1);
    assert!(ov.guides[0].len() >= 5, "the freehand path is drawn");
    assert_eq!(e.mouse_hints().middle, "release: close lasso");
}

#[test]
fn objects_partly_inside_are_not_taken() {
    let mut e = scene();
    e.set_tool(ToolKind::Glue);
    // A triangle covering box 0 and the corner of box 1 only.
    lasso(
        &mut e,
        Button::Middle,
        &[(-200.0, -200.0), (1700.0, -200.0), (1700.0, 800.0), (-200.0, 800.0)],
    );
    assert_eq!(e.selection().len(), 1);
}

#[test]
fn non_rectangular_lasso_takes_what_is_inside() {
    let mut e = scene();
    e.set_tool(ToolKind::Glue);
    // A triangle: box 0 (0..1000, 0..500) fits, box 1 (1500..2000) does not.
    lasso(
        &mut e,
        Button::Middle,
        &[(-100.0, -100.0), (2000.0, -100.0), (-100.0, 2000.0)],
    );
    assert_eq!(e.selection().len(), 1);
    assert_eq!(e.selection()[0], e.doc().objects[0].id);
}

#[test]
fn delete_region_by_lasso() {
    let mut e = scene();
    e.set_tool(ToolKind::Delete);
    assert_eq!(e.mouse_hints().middle, "delete region");
    let before = snapshot(&e);
    lasso(&mut e, Button::Middle, &SQUARE);
    assert_eq!(e.doc().objects.len(), 2);
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
}

#[test]
fn update_region_by_lasso() {
    let mut e = scene();
    e.execute("indicator.set", &json!({"thickness": 6})).unwrap();
    e.set_tool(ToolKind::Update);
    assert_eq!(e.mouse_hints().right, "update region");
    lasso(&mut e, Button::Right, &SQUARE);
    assert_eq!((thickness(&e, 0), thickness(&e, 1)), (6, 6));
    assert_ne!(thickness(&e, 2), 6);
    assert_ne!(thickness(&e, 3), 6);
}

#[test]
fn move_region_by_lasso_then_place() {
    let mut e = scene();
    e.set_tool(ToolKind::Move);
    assert_eq!(e.mouse_hints().right, "move region");
    lasso(&mut e, Button::Right, &SQUARE);
    assert!(e.tool_busy(), "the region follows the pointer");
    // Move mode doesn't tag the region's objects (only the selecting modes
    // change the selection); its preview shows them.
    assert!(e.selection().is_empty());
    assert_eq!(e.overlay().preview.len(), 2);
    let before = e.doc().objects[0].geometry_bbox().min;
    // The region follows the pointer from the release point.
    click(&mut e, -200.0 + 300.0, 900.0, Button::Left);
    let after = e.doc().objects[0].geometry_bbox().min;
    assert_eq!(after - before, Point::new(300.0, 0.0));
    assert_eq!(e.doc().objects[3].geometry_bbox().min, Point::new(5000.0, 5000.0));
}

#[test]
fn delete_points_by_lasso() {
    let mut e = engine();
    e.execute(
        "shape.polyline",
        &json!({"points": [[0, 0], [500, 0], [1000, 0], [3000, 0], [4000, 0]]}),
    )
    .unwrap();
    e.set_tool(ToolKind::DeletePoint);
    assert_eq!(e.mouse_hints().middle, "delete in region");
    lasso(
        &mut e,
        Button::Middle,
        &[(-100.0, -100.0), (1500.0, -100.0), (1500.0, 100.0), (-100.0, 100.0)],
    );
    assert_eq!(e.doc().objects[0].points().unwrap().len(), 2);
}

#[test]
fn only_active_depths_are_taken() {
    let mut e = scene();
    let id = e.doc().objects[0].id.0;
    e.execute("object.set_attributes", &json!({"ids": [id], "depth": 70}))
        .unwrap();
    e.execute("depth.set_visible", &json!({"depth": 70, "visible": false}))
        .unwrap();
    e.set_tool(ToolKind::Glue);
    lasso(&mut e, Button::Middle, &SQUARE);
    assert_eq!(e.selection().len(), 1);
    assert!(e.selection().iter().all(|s| s.0 != id));
}

#[test]
fn click_move_click_keeps_the_rectangle() {
    let mut e = scene();
    e.set_tool(ToolKind::Glue);
    click(&mut e, -200.0, -200.0, Button::Middle);
    assert!(e.tool_busy());
    assert_eq!(e.mouse_hints().middle, "final corner");
    ev(
        &mut e,
        ToolEvent::Move {
            pos: Point::new(1000.0, 1000.0),
        },
    );
    click(&mut e, 2500.0, 1000.0, Button::Middle);
    assert_eq!(e.selection().len(), 2);
}

#[test]
fn degenerate_lasso_selects_nothing_and_commands_validate() {
    let mut e = scene();
    e.set_tool(ToolKind::Delete);
    lasso(&mut e, Button::Middle, &[(0.0, 0.0), (3000.0, 0.0), (6000.0, 0.0)]);
    assert_eq!(e.doc().objects.len(), 4);
    let before = snapshot(&e);
    for p in [
        json!({"polygon": [[0, 0], [1, 1]]}),
        json!({"polygon": "x"}),
        json!({"polygon": [[0, 0], [1e300, 5], [1, 1]]}),
    ] {
        let _ = e.execute("object.update_region", &p);
        assert_eq!(snapshot(&e), before, "{p}");
    }
    assert!(e
        .execute("object.update_region", &json!({"polygon": [[0, 0], [1, 1]]}))
        .is_err());
}
