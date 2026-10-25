#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! W22-19, -21 to -25 and -30: xfig's scale factors, the point-move tie,
//! re-append after edits, point order on flip and rotate, Update's box
//! radius, spline tangents and Convert at a vertex.

use rustfig_engine::geom::Point;
use rustfig_engine::model::{Object, ObjectKind, PolyKind};
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

fn rect(e: &mut Engine, x: i32, y: i32, w: i32, h: i32) {
    e.execute("shape.rectangle", &json!({"x": x, "y": y, "width": w, "height": h}))
        .unwrap();
}

fn pts(o: &Object) -> Vec<Point> {
    o.points().unwrap().to_vec()
}

#[test]
fn box_scale_takes_the_larger_axis_factor() {
    let mut e = Engine::new();
    rect(&mut e, 0, 0, 1200, 600);
    e.set_tool(ToolKind::Scale);
    // Corner (1200, 600) dragged to (1800, 750): x 1.5, y 1.25 -> 1.5.
    click(&mut e, 1200.0, 600.0, Button::Left);
    click(&mut e, 1800.0, 750.0, Button::Left);
    let bb = e.doc().objects[0].geometry_bbox();
    assert_eq!((bb.min, bb.max), (Point::ZERO, Point::new(1800.0, 900.0)));
}

#[test]
fn centre_scale_takes_the_larger_axis_factor_from_the_clicked_vertex() {
    let mut e = Engine::new();
    rect(&mut e, 0, 0, 1200, 600);
    e.set_tool(ToolKind::Scale);
    // Vertex (1200, 600) about the centre (600, 300): to (1800, 750) is
    // x 2.0, y 1.5 -> 2.0.
    click(&mut e, 1200.0, 600.0, Button::Middle);
    click(&mut e, 1800.0, 750.0, Button::Middle);
    let bb = e.doc().objects[0].geometry_bbox();
    assert_eq!(
        (bb.min, bb.max),
        (Point::new(-600.0, -300.0), Point::new(1800.0, 900.0))
    );
}

#[test]
fn middle_move_point_tie_goes_vertical() {
    let mut e = Engine::new();
    e.execute("shape.polyline", &json!({"points": [[0, 0], [1200, 0], [1200, 1200]]}))
        .unwrap();
    e.set_tool(ToolKind::MovePoint);
    click(&mut e, 0.0, 0.0, Button::Middle);
    click(&mut e, 300.0, 300.0, Button::Middle);
    assert_eq!(pts(&e.doc().objects[0])[0], Point::new(0.0, 300.0));
}

#[test]
fn flip_keeps_the_point_order_and_re_appends() {
    let mut e = Engine::new();
    rect(&mut e, 0, 0, 1200, 600);
    rect(&mut e, 5000, 0, 100, 100);
    let before = pts(&e.doc().objects[0]);
    let id = e.doc().objects[0].id;
    e.set_tool(ToolKind::FlipH);
    // About the click's x (0): every point mirrors in place.
    click(&mut e, 0.0, 300.0, Button::Left);
    let last = e.doc().objects.last().unwrap();
    assert_eq!(last.id, id, "xfig's change_line puts the flipped object last");
    let after = pts(last);
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(*b, Point::new(-a.x, a.y));
    }
    // Undo restores the order of the list too.
    e.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(e.doc().objects[0].id, id);
    assert_eq!(pts(&e.doc().objects[0]), before);
}

#[test]
fn rotate_keeps_the_point_order_of_a_box() {
    let mut e = Engine::new();
    rect(&mut e, 0, 0, 1200, 600);
    let before = pts(&e.doc().objects[0]);
    e.execute("transform.rotate", &json!({"angle": -90, "cx": 0, "cy": 0}))
        .unwrap();
    let after = pts(&e.doc().objects[0]);
    // Clockwise by 90 degrees about the origin: (x, y) -> (-y, x).
    for (a, b) in before.iter().zip(&after) {
        assert!((b.x + a.y).abs() < 1e-6 && (b.y - a.x).abs() < 1e-6, "{a:?} -> {b:?}");
    }
}

#[test]
fn flip_and_rotate_write_xfigs_ellipse_fields() {
    let mut e = Engine::new();
    e.execute(
        "shape.ellipse",
        &json!({"cx": 1000, "cy": 1000, "rx": 500, "ry": 300, "angle": 20}),
    )
    .unwrap();
    e.execute("shape.circle", &json!({"cx": 3000, "cy": 1000, "r": 400}))
        .unwrap();
    let ObjectKind::Ellipse(el) = &e.doc().objects[0].kind else {
        panic!()
    };
    let (dir, ang) = (el.direction, el.angle);
    let id = e.doc().objects[0].id;
    e.execute("transform.flip_horizontal", &json!({"ids": [id.0], "cx": 0}))
        .unwrap();
    let ObjectKind::Ellipse(el) = &e.doc().get(id).unwrap().kind else {
        panic!()
    };
    assert_eq!(el.direction, dir ^ 1);
    assert!((el.angle + ang).abs() < 1e-9);
    // A circle turned clockwise by 90 degrees: xfig writes angle 3pi/2.
    let cid = e.doc().objects[1].id;
    e.execute(
        "transform.rotate",
        &json!({"ids": [cid.0], "angle": -90, "cx": 0, "cy": 0}),
    )
    .unwrap();
    let ObjectKind::Ellipse(c) = &e.doc().get(cid).unwrap().kind else {
        panic!()
    };
    assert!((c.angle - 4.7119140625).abs() < 1e-9, "{}", c.angle);
    assert!(c.kind.is_circle());
}

#[test]
fn text_edit_and_break_re_append() {
    let mut e = Engine::new();
    e.execute("shape.text", &json!({"x": 0, "y": 0, "text": "a"})).unwrap();
    e.execute("shape.text", &json!({"x": 0, "y": 600, "text": "b"}))
        .unwrap();
    let first = e.doc().objects[0].id;
    e.execute(
        "object.set_attributes",
        &json!({"ids": [first.0], "text": "c", "reappend": true}),
    )
    .unwrap();
    assert_eq!(e.doc().objects.last().unwrap().id, first);
    // Break: the parts go to the end.
    rect(&mut e, 0, 2000, 100, 100);
    rect(&mut e, 500, 2000, 100, 100);
    let n = e.doc().objects.len();
    let ids: Vec<u64> = e.doc().objects[n - 2..].iter().map(|o| o.id.0).collect();
    e.execute("object.group", &json!({"ids": ids})).unwrap();
    rect(&mut e, 9000, 0, 100, 100);
    e.set_tool(ToolKind::Break);
    click(&mut e, 0.0, 2050.0, Button::Left);
    let objs = &e.doc().objects;
    let k = objs.len();
    // The two freed boxes now follow the rectangle drawn after the group.
    assert_eq!(pts(&objs[k - 3])[0], Point::new(9000.0, 0.0));
    assert_eq!(pts(&objs[k - 2])[0], Point::new(0.0, 2000.0));
    assert_eq!(pts(&objs[k - 1])[0], Point::new(500.0, 2000.0));
}

#[test]
fn update_turns_a_plain_box_into_an_arc_box() {
    let mut e = Engine::new();
    rect(&mut e, 0, 0, 1200, 600);
    e.set_tool(ToolKind::Update);
    click(&mut e, 0.0, 300.0, Button::Left);
    let ObjectKind::Polyline(p) = &e.doc().objects[0].kind else {
        panic!()
    };
    assert_eq!(p.kind, PolyKind::ArcBox);
    assert_eq!(p.radius, e.indicator.box_radius);
}

#[test]
fn spline_tangent_at_the_lowest_point_is_horizontal() {
    let mut e = Engine::new();
    e.execute(
        "shape.spline",
        &json!({"points": [[0, 0], [2000, 3000], [4000, 0]], "kind": "approximated"}),
    )
    .unwrap();
    let id = e.doc().objects[0].id;
    let o = e.doc().get(id).unwrap().clone();
    let flat = rustfig_engine::model::flatten_object(&o);
    let low = flat[0]
        .pts
        .iter()
        .copied()
        .max_by(|a, b| a.y.partial_cmp(&b.y).unwrap())
        .unwrap();
    let r = e
        .execute(
            "object.tangent_line",
            &json!({"id": id.0, "x": low.x, "y": low.y, "tolerance": 20}),
        )
        .unwrap();
    let p = r["points"].as_array().unwrap();
    let (a, b) = (p[0].as_array().unwrap(), p[2].as_array().unwrap());
    let (dx, dy) = (
        b[0].as_f64().unwrap() - a[0].as_f64().unwrap(),
        b[1].as_f64().unwrap() - a[1].as_f64().unwrap(),
    );
    assert!(dy.abs() < 0.03 * dx.abs(), "tangent {dx} {dy}");
}

#[test]
fn convert_right_needs_a_vertex_and_opens_there() {
    let mut e = Engine::new();
    e.execute(
        "shape.polygon",
        &json!({"points": [[0, 0], [1200, 0], [1200, 1200], [0, 1200]]}),
    )
    .unwrap();
    e.set_tool(ToolKind::Convert);
    // On an edge, not at a vertex: nothing happens.
    click(&mut e, 600.0, 0.0, Button::Right);
    let ObjectKind::Polyline(p) = &e.doc().objects[0].kind else {
        panic!()
    };
    assert_eq!(p.kind, PolyKind::Polygon);
    // At the third vertex: the open line starts there and ends before it.
    click(&mut e, 1200.0, 1200.0, Button::Right);
    let ObjectKind::Polyline(p) = &e.doc().objects[0].kind else {
        panic!()
    };
    assert_eq!(p.kind, PolyKind::Polyline);
    assert_eq!(
        p.points,
        vec![
            Point::new(1200.0, 1200.0),
            Point::new(0.0, 1200.0),
            Point::new(0.0, 0.0),
            Point::new(1200.0, 0.0)
        ]
    );
    // Closing again, then the first vertex: xfig starts at the second.
    click(&mut e, 0.0, 0.0, Button::Right);
    click(&mut e, 1200.0, 1200.0, Button::Right);
    let ObjectKind::Polyline(p) = &e.doc().objects[0].kind else {
        panic!()
    };
    assert_eq!(p.kind, PolyKind::Polyline);
    assert_eq!(p.points[0], Point::new(0.0, 1200.0));
    assert_eq!(p.points[3], Point::new(1200.0, 1200.0));
}
