#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Differences found against the real xfig (W22-1 .. W22-7): spline shape
//! factor, rotate pivot, draw order after move and scale, radius -1, the
//! clicked circle points, regular polygon and arc-box point order.

use rustfig_engine::geom::Point;
use rustfig_engine::model::{EllipseKind, ObjectKind, PolyKind, SplineKind};
use rustfig_engine::{Button, Engine, SnapMode, ToolEvent, ToolKind};
use serde_json::json;

fn engine() -> Engine {
    let mut e = Engine::new();
    e.indicator.snap = SnapMode::Any;
    e
}

fn press(e: &mut Engine, x: f64, y: f64) {
    e.tool_event(ToolEvent::Press {
        pos: Point::new(x, y),
        button: Button::Left,
        shift: false,
    })
    .unwrap();
}

fn release(e: &mut Engine, x: f64, y: f64) {
    e.tool_event(ToolEvent::Release {
        pos: Point::new(x, y),
        button: Button::Left,
    })
    .unwrap();
}

fn drag(e: &mut Engine, a: (f64, f64), b: (f64, f64)) {
    press(e, a.0, a.1);
    e.tool_event(ToolEvent::Move {
        pos: Point::new(b.0, b.1),
    })
    .unwrap();
    release(e, b.0, b.1);
}

fn pts(e: &Engine) -> Vec<(f64, f64)> {
    e.doc()
        .objects
        .last()
        .unwrap()
        .points()
        .unwrap()
        .iter()
        .map(|p| (p.x, p.y))
        .collect()
}

fn rect(e: &mut Engine, x: i32, y: i32, w: i32, h: i32) {
    e.execute("shape.rectangle", &json!({"x": x, "y": y, "width": w, "height": h}))
        .unwrap();
}

#[test]
fn interpolated_splines_use_minus_half() {
    let mut e = engine();
    e.execute(
        "shape.spline",
        &json!({"points": [[0, 0], [500, 500], [1000, 0], [1500, 500]], "kind": "interpolated"}),
    )
    .unwrap();
    let ObjectKind::Spline(s) = &e.doc().objects[0].kind else {
        panic!()
    };
    assert_eq!(s.kind, SplineKind::Interpolated);
    assert_eq!(s.shapes, vec![0.0, -0.5, -0.5, 0.0]);
    // Line to spline conversion remembers the interpolated kind.
    e.execute("shape.polyline", &json!({"points": [[0, 0], [500, 500], [1000, 0]]}))
        .unwrap();
    let id = e.doc().objects[1].id.0;
    e.execute("object.convert", &json!({"ids": [id]})).unwrap();
    let ObjectKind::Spline(s) = &e.doc().objects[1].kind else {
        panic!()
    };
    assert_eq!(s.shapes, vec![0.0, -0.5, 0.0]);
}

#[test]
fn rotate_turns_about_the_clicked_point() {
    let mut e = engine();
    rect(&mut e, 0, 0, 1000, 1000);
    e.set_tool(ToolKind::RotateCcw);
    press(&mut e, 0.0, 500.0);
    release(&mut e, 0.0, 500.0);
    let c = e.doc().objects[0].geometry_bbox().center();
    // 90 degrees about (0, 500): the centre (500, 500) moves to (0, 0) or (0, 1000).
    assert!(
        c.x.abs() < 1.0 && (c.y.abs() < 1.0 || (c.y - 1000.0).abs() < 1.0),
        "{c:?}"
    );
}

#[test]
fn move_and_scale_put_the_object_last() {
    let mut e = engine();
    rect(&mut e, 0, 0, 1000, 1000);
    rect(&mut e, 3000, 0, 1000, 1000);
    let first = e.doc().objects[0].id;
    e.set_tool(ToolKind::Move);
    drag(&mut e, (0.0, 500.0), (-500.0, 500.0));
    assert_eq!(e.doc().objects.last().unwrap().id, first, "moved object draws on top");
    assert!(e.undo());
    assert_eq!(e.doc().objects[0].id, first);
    e.set_tool(ToolKind::Scale);
    drag(&mut e, (1000.0, 1000.0), (1500.0, 1500.0));
    assert_eq!(e.doc().objects.last().unwrap().id, first, "scaled object draws on top");
    assert!(e.undo());
    assert_eq!(e.doc().objects[0].id, first);
}

#[test]
fn new_lines_have_radius_minus_one_and_arc_boxes_the_indicator_radius() {
    let mut e = engine();
    e.set_tool(ToolKind::Box);
    drag(&mut e, (0.0, 0.0), (1000.0, 500.0));
    let ObjectKind::Polyline(p) = &e.doc().objects[0].kind else {
        panic!()
    };
    assert_eq!((p.kind, p.radius), (PolyKind::Box, -1));
    e.set_tool(ToolKind::ArcBox);
    drag(&mut e, (0.0, 0.0), (1000.0, 500.0));
    let ObjectKind::Polyline(p) = &e.doc().objects[1].kind else {
        panic!()
    };
    assert_eq!((p.kind, p.radius), (PolyKind::ArcBox, 7));
    e.execute("shape.polyline", &json!({"points": [[0, 0], [500, 500]]}))
        .unwrap();
    let ObjectKind::Polyline(p) = &e.doc().objects[2].kind else {
        panic!()
    };
    assert_eq!(p.radius, -1);
}

#[test]
fn circles_keep_the_clicked_points() {
    let mut e = engine();
    e.set_tool(ToolKind::CircleRadius);
    drag(&mut e, (1000.0, 1000.0), (1300.0, 1400.0));
    let ObjectKind::Ellipse(c) = &e.doc().objects[0].kind else {
        panic!()
    };
    assert_eq!(c.kind, EllipseKind::CircleRadius);
    assert_eq!((c.center, c.radii.x), (Point::new(1000.0, 1000.0), 500.0));
    assert_eq!(
        (c.start, c.end),
        (Point::new(1000.0, 1000.0), Point::new(1300.0, 1400.0))
    );
    e.set_tool(ToolKind::CircleDiameter);
    drag(&mut e, (0.0, 0.0), (21975.0, 8.0));
    let ObjectKind::Ellipse(c) = &e.doc().objects[1].kind else {
        panic!()
    };
    assert_eq!(c.center, Point::new(10987.0, 4.0), "xfig truncates the centre");
    assert_eq!((c.start, c.end), (Point::new(0.0, 0.0), Point::new(21975.0, 8.0)));
}

#[test]
fn regular_polygon_starts_at_the_last_click_and_runs_as_xfig() {
    let mut e = engine();
    e.execute("indicator.set", &json!({"sides": 4})).unwrap();
    e.set_tool(ToolKind::RegularPolygon);
    drag(&mut e, (0.0, 0.0), (1000.0, 0.0));
    assert_eq!(
        pts(&e),
        vec![(1000.0, 0.0), (0.0, -1000.0), (-1000.0, 0.0), (0.0, 1000.0)]
    );
    let ObjectKind::Polyline(p) = &e.doc().objects[0].kind else {
        panic!()
    };
    assert_eq!(p.radius, 0, "create_regpoly sets radius 0");
}

#[test]
fn arc_box_points_start_at_the_second_corner() {
    let mut e = engine();
    e.set_tool(ToolKind::ArcBox);
    drag(&mut e, (0.0, 0.0), (1000.0, 500.0));
    assert_eq!(pts(&e), vec![(1000.0, 500.0), (1000.0, 0.0), (0.0, 0.0), (0.0, 500.0)]);
    e.set_tool(ToolKind::Box);
    drag(&mut e, (0.0, 0.0), (1000.0, 500.0));
    assert_eq!(pts(&e), vec![(0.0, 0.0), (1000.0, 0.0), (1000.0, 500.0), (0.0, 500.0)]);
}
