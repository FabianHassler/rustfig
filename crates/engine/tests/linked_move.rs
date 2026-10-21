#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Smart links (D8): moving a box drags the end points of lines attached to
//! its perimeter, in xfig's off / move / slide modes.

use rustfig_engine::geom::Point;
use rustfig_engine::model::ObjectKind;
use rustfig_engine::{Engine, LinkMode};
use serde_json::json;

/// A box 0..1200 × 0..600 and two lines: one ending on its top edge, one
/// ending on its right edge. Returns (box, top line, right line).
fn box_and_lines() -> (Engine, u64, u64, u64) {
    let mut e = Engine::new();
    let box_id = e
        .execute(
            "shape.rectangle",
            &json!({"x": 0, "y": 0, "width": 1200, "height": 600}),
        )
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    // First point (600, 0) sits on the top edge.
    let top = e
        .execute("shape.polyline", &json!({"points": [[600, 0], [600, -300]]}))
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    // Last point (1200, 300) sits on the right edge.
    let right = e
        .execute("shape.polyline", &json!({"points": [[1500, 300], [1200, 300]]}))
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    (e, box_id, top, right)
}

fn points(e: &Engine, id: u64) -> Vec<Point> {
    let o = e.doc().get(rustfig_engine::model::ObjectId(id)).unwrap();
    match &o.kind {
        ObjectKind::Polyline(p) => p.points.clone(),
        other => panic!("expected a polyline, got {other:?}"),
    }
}

fn set_linkmode(e: &mut Engine, m: LinkMode) {
    e.execute("indicator.set", &json!({"linkmode": m})).unwrap();
}

#[test]
fn move_drags_attached_line_ends() {
    let (mut e, box_id, top, right) = box_and_lines();
    set_linkmode(&mut e, LinkMode::Move);
    e.execute("transform.move", &json!({"ids": [box_id], "dx": 100, "dy": 200}))
        .unwrap();
    // Only the attached end point follows; the far end stays.
    assert_eq!(points(&e, top), [Point::new(700.0, 200.0), Point::new(600.0, -300.0)]);
    assert_eq!(
        points(&e, right),
        [Point::new(1500.0, 300.0), Point::new(1300.0, 500.0)]
    );
}

#[test]
fn off_leaves_lines_alone() {
    let (mut e, box_id, top, right) = box_and_lines();
    set_linkmode(&mut e, LinkMode::Off);
    e.execute("transform.move", &json!({"ids": [box_id], "dx": 100, "dy": 200}))
        .unwrap();
    assert_eq!(points(&e, top), [Point::new(600.0, 0.0), Point::new(600.0, -300.0)]);
    assert_eq!(
        points(&e, right),
        [Point::new(1500.0, 300.0), Point::new(1200.0, 300.0)]
    );
}

#[test]
fn slide_keeps_the_last_segment_axis_aligned() {
    let (mut e, box_id, top, right) = box_and_lines();
    set_linkmode(&mut e, LinkMode::Slide);
    e.execute("transform.move", &json!({"ids": [box_id], "dx": 100, "dy": 200}))
        .unwrap();
    // The vertical top line slides sideways: its second point gains dx only.
    assert_eq!(points(&e, top), [Point::new(700.0, 200.0), Point::new(700.0, -300.0)]);
    // The horizontal right line slides up/down: its first point gains dy only.
    assert_eq!(
        points(&e, right),
        [Point::new(1500.0, 500.0), Point::new(1300.0, 500.0)]
    );
}

/// Moving a compound drags line ends that sit anywhere inside it (xfig's
/// `get_interior_links`).
#[test]
fn compound_drags_lines_inside_it() {
    let mut e = Engine::new();
    let box_style = e.indicator.style();
    let box_obj = rustfig_engine::model::Object::new(ObjectKind::Polyline(rustfig_engine::model::Polyline::rect(
        box_style,
        Point::new(0.0, 0.0),
        Point::new(1200.0, 600.0),
    )));
    let compound = rustfig_engine::model::Object::new(ObjectKind::Compound(rustfig_engine::model::Compound {
        objects: vec![box_obj],
    }));
    let compound_id = e.add_object(compound);
    let line = e
        .execute("shape.polyline", &json!({"points": [[300, 200], [300, -300]]}))
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    set_linkmode(&mut e, LinkMode::Move);
    e.execute("transform.move", &json!({"ids": [compound_id.0], "dx": 100, "dy": 200}))
        .unwrap();
    // The line's first point (300, 200) is inside the compound's box, so it
    // follows; the far end stays.
    assert_eq!(points(&e, line), [Point::new(400.0, 400.0), Point::new(300.0, -300.0)]);
}

#[test]
fn moving_a_line_alone_does_not_link() {
    let (mut e, _box_id, top, _right) = box_and_lines();
    set_linkmode(&mut e, LinkMode::Move);
    e.execute("transform.move", &json!({"ids": [top], "dx": 50, "dy": 60}))
        .unwrap();
    // A polyline is not a box, so its points move as one object, once.
    assert_eq!(points(&e, top), [Point::new(650.0, 60.0), Point::new(650.0, -240.0)]);
}
