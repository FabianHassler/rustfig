#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Smart links extended (D14): more target shapes and followers, every
//! geometric operation, copy with links, move point on a box corner, the
//! drag preview and the highlight of attached ends.

use rustfig_engine::geom::Point;
use rustfig_engine::model::{Object, ObjectId, ObjectKind, PolyKind, Polyline};
use rustfig_engine::tools::{Button, ToolEvent};
use rustfig_engine::{Engine, LinkMode};
use serde_json::{json, Value};

fn run(e: &mut Engine, cmd: &str, p: Value) -> Value {
    e.execute(cmd, &p).unwrap()
}

fn id_of(v: &Value) -> u64 {
    v["id"].as_u64().unwrap()
}

fn engine(mode: LinkMode) -> Engine {
    let mut e = Engine::new();
    run(&mut e, "indicator.set", json!({"linkmode": mode}));
    e
}

fn pts(e: &Engine, id: u64) -> Vec<Point> {
    let o = e.doc().get(ObjectId(id)).unwrap();
    o.points()
        .map(<[Point]>::to_vec)
        .unwrap_or_else(|| panic!("no points: {:?}", o.kind))
}

fn line(e: &mut Engine, points: Value) -> u64 {
    id_of(&run(e, "shape.polyline", json!({ "points": points })))
}

fn near(a: Point, x: f64, y: f64) {
    assert!((a.x - x).abs() < 1e-6 && (a.y - y).abs() < 1e-6, "{a:?} != ({x}, {y})");
}

/// Every target kind: a line ends on its outline, the shape moves, the end
/// follows, the far end stays.
#[test]
fn every_target_kind_drags_the_ends() {
    // (shape command, params, a point on the outline)
    let cases: Vec<(&str, Value, (f64, f64))> = vec![
        ("shape.circle", json!({"cx": 0, "cy": 0, "r": 500}), (500.0, 0.0)),
        (
            "shape.ellipse",
            json!({"cx": 0, "cy": 0, "rx": 800, "ry": 400}),
            (0.0, -400.0),
        ),
        (
            "shape.polygon",
            json!({"points": [[0, 0], [1000, 0], [500, 800]]}),
            (500.0, 0.0),
        ),
        (
            "shape.rectangle",
            json!({"x": 0, "y": 0, "width": 1000, "height": 600, "radius": 10}),
            (500.0, 600.0),
        ),
        ("shape.text", json!({"x": 0, "y": 0, "text": "Hello"}), (0.0, 0.0)),
        (
            "shape.spline",
            json!({"points": [[0, 0], [800, 0], [800, 800], [0, 800]], "closed": true, "kind": "interpolated"}),
            (0.0, 0.0),
        ),
    ];
    for (cmd, params, (x, y)) in cases {
        for mode in [LinkMode::Move, LinkMode::Slide] {
            let mut e = engine(mode);
            let shape = id_of(&run(&mut e, cmd, params.clone()));
            // The text box's anchor corner or the shape's outline point; the
            // line comes from far away, so slide has a vertical last segment.
            let (x, y) = match &e.doc().get(ObjectId(shape)).unwrap().kind {
                ObjectKind::Text(t) => (t.corners()[0].x, t.corners()[0].y),
                _ => (x, y),
            };
            let l = line(&mut e, json!([[x + 2000.0, y - 1500.0], [x, y]]));
            let before = e.doc().objects.clone();
            run(&mut e, "transform.move", json!({"ids": [shape], "dx": 100, "dy": 70}));
            let p = pts(&e, l);
            near(p[1], x + 100.0, y + 70.0);
            if mode == LinkMode::Move {
                near(p[0], x + 2000.0, y - 1500.0);
            } else {
                // Slide: the neighbour of the last segment moves along the axis it keeps
                // (the segment is oblique, so its y).
                near(p[0], x + 2000.0, y - 1500.0 + 70.0);
            }
            assert!(e.undo(), "{cmd}");
            assert_eq!(e.doc().objects, before, "{cmd}: one undo step restores everything");
        }
    }
}

#[test]
fn off_follows_nothing_for_any_target() {
    let mut e = engine(LinkMode::Off);
    let c = id_of(&run(&mut e, "shape.circle", json!({"cx": 0, "cy": 0, "r": 500})));
    let l = line(&mut e, json!([[500, 0], [2000, 0]]));
    run(&mut e, "transform.move", json!({"ids": [c], "dx": 100, "dy": 0}));
    near(pts(&e, l)[0], 500.0, 0.0);
    run(&mut e, "transform.rotate", json!({"ids": [c], "angle": 90}));
    near(pts(&e, l)[0], 500.0, 0.0);
}

#[test]
fn a_far_end_is_not_attached() {
    let mut e = engine(LinkMode::Move);
    let c = id_of(&run(&mut e, "shape.circle", json!({"cx": 0, "cy": 0, "r": 500})));
    // 100 units off the outline: outside the 45 unit tolerance.
    let l = line(&mut e, json!([[600, 0], [2000, 0]]));
    run(&mut e, "transform.move", json!({"ids": [c], "dx": 100, "dy": 0}));
    near(pts(&e, l)[0], 600.0, 0.0);
}

#[test]
fn compound_and_picture_targets() {
    let mut e = engine(LinkMode::Move);
    let style = e.indicator.style();
    let pic = Object::new(ObjectKind::Polyline(Polyline::new(
        PolyKind::Picture,
        style,
        vec![
            Point::new(0.0, 0.0),
            Point::new(1000.0, 0.0),
            Point::new(1000.0, 600.0),
            Point::new(0.0, 600.0),
        ],
    )));
    let pid = e.add_object(pic).0;
    let l = line(&mut e, json!([[500, 0], [500, -800]]));
    run(&mut e, "transform.move", json!({"ids": [pid], "dx": 50, "dy": 60}));
    near(pts(&e, l)[0], 550.0, 60.0);
    // A compound's interior.
    let r = id_of(&run(
        &mut e,
        "shape.rectangle",
        json!({"x": 3000, "y": 0, "width": 800, "height": 800}),
    ));
    let c2 = id_of(&run(&mut e, "shape.circle", json!({"cx": 3400, "cy": 1500, "r": 200})));
    run(&mut e, "edit.select", json!({"ids": [r, c2]}));
    let g = id_of(&run(&mut e, "object.group", json!({"ids": [r, c2]})));
    let l2 = line(&mut e, json!([[3400, 400], [3400, -900]]));
    run(&mut e, "transform.move", json!({"ids": [g], "dx": 10, "dy": 20}));
    near(pts(&e, l2)[0], 3410.0, 420.0);
}

#[test]
fn open_splines_follow_and_a_line_between_two_shapes_moves_both_ends() {
    let mut e = engine(LinkMode::Move);
    let a = id_of(&run(&mut e, "shape.circle", json!({"cx": 0, "cy": 0, "r": 500})));
    let b = id_of(&run(
        &mut e,
        "shape.rectangle",
        json!({"x": 3000, "y": -300, "width": 600, "height": 600}),
    ));
    let sp = id_of(&run(
        &mut e,
        "shape.spline",
        json!({"points": [[500, 0], [1500, -800], [2500, 800], [3000, 0]], "kind": "interpolated"}),
    ));
    // Both shapes selected: the spline's two ends each follow once.
    run(&mut e, "transform.move", json!({"ids": [a, b], "dx": 100, "dy": 200}));
    let p = pts(&e, sp);
    near(p[0], 600.0, 200.0);
    near(p[3], 3100.0, 200.0);
    near(p[1], 1500.0, -800.0);
    // Only one of them: just that end.
    run(&mut e, "transform.move", json!({"ids": [a], "dx": 10, "dy": 0}));
    let p = pts(&e, sp);
    near(p[0], 610.0, 200.0);
    near(p[3], 3100.0, 200.0);
}

#[test]
fn slide_never_slides_a_line_between_two_shapes() {
    let mut e = engine(LinkMode::Slide);
    let a = id_of(&run(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 500, "height": 500}),
    ));
    let b = id_of(&run(
        &mut e,
        "shape.rectangle",
        json!({"x": 2000, "y": 0, "width": 500, "height": 500}),
    ));
    let l = line(&mut e, json!([[500, 250], [1000, 250], [2000, 250]]));
    run(&mut e, "transform.move", json!({"ids": [a, b], "dx": 0, "dy": 100}));
    let p = pts(&e, l);
    // The middle point is not slid twice, the ends follow once.
    near(p[0], 500.0, 350.0);
    near(p[1], 1000.0, 250.0);
    near(p[2], 2000.0, 350.0);
}

#[test]
fn rotate_flip_scale_transform_the_attached_end() {
    let mut e = engine(LinkMode::Move);
    let r = id_of(&run(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 1000, "height": 1000}),
    ));
    let l = line(&mut e, json!([[3000, 500], [1000, 500]]));
    // Rotate 90 degrees counter-clockwise about the box's centre (500, 500):
    // the point (1000, 500) goes to (500, 0).
    run(&mut e, "transform.rotate", json!({"ids": [r], "angle": 90}));
    near(pts(&e, l)[1], 500.0, 0.0);
    near(pts(&e, l)[0], 3000.0, 500.0);
    // Flip left/right about x = 500: (500, 0) stays.
    run(&mut e, "transform.flip_horizontal", json!({"ids": [r]}));
    near(pts(&e, l)[1], 500.0, 0.0);
    // Scale by 2 about (0, 0).
    run(
        &mut e,
        "transform.scale",
        json!({"ids": [r], "sx": 2, "cx": 0, "cy": 0}),
    );
    near(pts(&e, l)[1], 1000.0, 0.0);
    near(pts(&e, l)[0], 3000.0, 500.0);
    let before = e.doc().objects.clone();
    run(&mut e, "transform.flip_vertical", json!({"ids": [r], "cy": 0}));
    near(pts(&e, l)[1], 1000.0, 0.0);
    assert!(e.undo());
    assert_eq!(e.doc().objects, before, "one undo step");
}

#[test]
fn slide_after_a_scale_keeps_the_last_segment_axis_aligned() {
    let mut e = engine(LinkMode::Slide);
    let r = id_of(&run(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 1000, "height": 1000}),
    ));
    // A horizontal last segment (y kept): the neighbour moves in y only.
    let l = line(&mut e, json!([[3000, 500], [1000, 500]]));
    run(
        &mut e,
        "transform.scale",
        json!({"ids": [r], "sx": 1, "sy": 2, "cx": 0, "cy": 0}),
    );
    near(pts(&e, l)[1], 1000.0, 1000.0);
    near(pts(&e, l)[0], 3000.0, 1000.0);
}

#[test]
fn nudge_align_and_align_to_follow() {
    let mut e = engine(LinkMode::Move);
    let r = id_of(&run(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 1000, "height": 1000}),
    ));
    let l = line(&mut e, json!([[3000, 500], [1000, 500]]));
    run(
        &mut e,
        "transform.nudge",
        json!({"ids": [r], "dir": "right", "steps": 2}),
    );
    near(pts(&e, l)[1], 1030.0, 500.0);
    run(
        &mut e,
        "transform.nudge",
        json!({"ids": [r], "dir": "up", "distance": 100}),
    );
    near(pts(&e, l)[1], 1030.0, 400.0);
    assert!(e
        .execute("transform.nudge", &json!({"ids": [r], "dir": "sideways"}))
        .is_err());
    // Align to another box's left edge.
    let m = id_of(&run(
        &mut e,
        "shape.rectangle",
        json!({"x": 5000, "y": 5000, "width": 100, "height": 100}),
    ));
    run(
        &mut e,
        "arrange.align_to",
        json!({"reference": m, "ids": [r], "h": "left", "v": "top"}),
    );
    // r was (30..1030, -100..900); now at (5000, 5000): the end moved with it.
    near(pts(&e, l)[1], 6000.0, 5500.0);
    // Align two shapes: the left one moves to the right one's top.
    let c = id_of(&run(&mut e, "shape.circle", json!({"cx": 9000, "cy": 9000, "r": 500})));
    let l2 = line(&mut e, json!([[9500, 9000], [12000, 9000]]));
    run(&mut e, "arrange.align", json!({"ids": [m, c], "v": "top"}));
    // The circle's top (8500) goes to the top of the group (5000): dy = -3500.
    assert!((pts(&e, l2)[0].y - 5500.0).abs() < 0.1, "{:?}", pts(&e, l2)[0]);
}

#[test]
fn copy_copies_the_attached_lines() {
    for mode in [LinkMode::Move, LinkMode::Slide] {
        let mut e = engine(mode);
        let r = id_of(&run(
            &mut e,
            "shape.rectangle",
            json!({"x": 0, "y": 0, "width": 1000, "height": 1000}),
        ));
        let l = line(&mut e, json!([[1000, 500], [3000, 500]]));
        let before = e.doc().objects.clone();
        run(&mut e, "edit.duplicate", json!({"ids": [r], "dx": 0, "dy": 2000}));
        // box, line, copy of the box, copy of the line.
        assert_eq!(e.doc().objects.len(), 4);
        let lines: Vec<Vec<Point>> = e
            .doc()
            .objects
            .iter()
            .filter(|o| matches!(&o.kind, ObjectKind::Polyline(p) if p.kind == PolyKind::Polyline))
            .map(|o| o.points().unwrap().to_vec())
            .collect();
        assert_eq!(lines.len(), 2);
        // One line still ends on the original box, one on the copy.
        assert!(lines.iter().any(|p| (p[0].y - 500.0).abs() < 1e-6));
        assert!(lines.iter().any(|p| (p[0].y - 2500.0).abs() < 1e-6));
        // The moved one is the original (xfig's order).
        assert!((pts(&e, l)[0].y - 2500.0).abs() < 1e-6);
        assert!(e.undo());
        assert_eq!(e.doc().objects, before, "one undo step");
    }
    let mut e = engine(LinkMode::Off);
    let r = id_of(&run(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 1000, "height": 1000}),
    ));
    line(&mut e, json!([[1000, 500], [3000, 500]]));
    run(&mut e, "edit.duplicate", json!({"ids": [r], "dx": 0, "dy": 2000}));
    assert_eq!(e.doc().objects.len(), 3, "off: the line is not copied");
}

#[test]
fn move_point_on_a_box_corner_resizes_with_the_lines() {
    let mut e = engine(LinkMode::Move);
    let r = id_of(&run(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 1000, "height": 1000}),
    ));
    let top = line(&mut e, json!([[500, 0], [500, -800]]));
    let right = line(&mut e, json!([[3000, 500], [1000, 500]]));
    // Corner 2 is (1000, 1000), the opposite of (0, 0): stretch to (2000, 3000).
    let before = e.doc().objects.clone();
    run(&mut e, "point.move", json!({"id": r, "index": 2, "x": 2000, "y": 3000}));
    // The top edge's attachment keeps its relative place; the right edge's
    // follows the edge.
    near(pts(&e, top)[0], 1000.0, 0.0);
    near(pts(&e, right)[1], 2000.0, 1500.0);
    near(pts(&e, right)[0], 3000.0, 500.0);
    assert!(e.undo());
    assert_eq!(e.doc().objects, before, "one undo step");
    // Off: only the box changes.
    run(&mut e, "indicator.set", json!({"linkmode": "off"}));
    run(&mut e, "point.move", json!({"id": r, "index": 2, "x": 2000, "y": 3000}));
    near(pts(&e, right)[1], 1000.0, 500.0);
}

#[test]
fn the_drag_preview_shows_every_follower_and_the_ends_are_highlighted() {
    let mut e = engine(LinkMode::Move);
    let a = id_of(&run(&mut e, "shape.circle", json!({"cx": 0, "cy": 0, "r": 500})));
    let b = id_of(&run(
        &mut e,
        "shape.rectangle",
        json!({"x": 3000, "y": -300, "width": 600, "height": 600}),
    ));
    let sp = id_of(&run(
        &mut e,
        "shape.spline",
        json!({"points": [[500, 0], [1500, -800], [3000, 0]], "kind": "interpolated"}),
    ));
    let l = line(&mut e, json!([[3600, 0], [5000, 0]]));
    // Highlight: select both shapes with the Select tool; three ends are attached.
    run(&mut e, "tool.select", json!({}));
    run(&mut e, "edit.select", json!({"ids": [a, b]}));
    let ends = e.overlay().handles;
    for q in [Point::new(500.0, 0.0), Point::new(3000.0, 0.0), Point::new(3600.0, 0.0)] {
        assert!(ends.iter().any(|h| h.dist(q) < 1e-6), "{q:?} highlighted");
    }
    // Drag with the Move tool: pick the circle's outline, move.
    e.indicator.snap = rustfig_engine::SnapMode::Any;
    run(&mut e, "edit.select", json!({"ids": []}));
    run(&mut e, "tool.move", json!({}));
    let at = Point::new(0.0, -500.0);
    e.tool_event(ToolEvent::Press {
        pos: at,
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    e.tool_event(ToolEvent::Release {
        pos: at,
        button: Button::Left,
    })
    .unwrap();
    e.tool_event(ToolEvent::Move {
        pos: Point::new(100.0, -400.0),
    })
    .unwrap();
    let ov = e.overlay();
    let o = ov
        .preview
        .iter()
        .find(|o| o.id.0 == sp)
        .expect("the spline follows while dragging");
    near(o.points().unwrap()[0], 600.0, 100.0);
    assert!(ov.hide.iter().any(|i| i.0 == sp));
    assert!(!ov.preview.iter().any(|o| o.id.0 == l), "an unrelated line stays put");
}

#[test]
fn dragging_a_box_corner_previews_the_stretched_lines() {
    let mut e = engine(LinkMode::Move);
    e.indicator.snap = rustfig_engine::SnapMode::Any;
    let r = id_of(&run(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 1000, "height": 1000}),
    ));
    let right = line(&mut e, json!([[3000, 500], [1000, 500]]));
    run(&mut e, "tool.move_point", json!({}));
    let at = Point::new(1000.0, 1000.0);
    e.tool_event(ToolEvent::Press {
        pos: at,
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    e.tool_event(ToolEvent::Move {
        pos: Point::new(2000.0, 3000.0),
    })
    .unwrap();
    let ov = e.overlay();
    let o = ov
        .preview
        .iter()
        .find(|o| o.id.0 == right)
        .expect("attached line previewed");
    near(o.points().unwrap()[1], 2000.0, 1500.0);
    assert!(ov.hide.iter().any(|i| i.0 == right) && ov.hide.iter().any(|i| i.0 == r));
    e.tool_event(ToolEvent::Release {
        pos: Point::new(2000.0, 3000.0),
        button: Button::Left,
    })
    .unwrap();
    near(pts(&e, right)[1], 2000.0, 1500.0);
}
