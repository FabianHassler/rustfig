#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Join and split (plan D2): `e_joinsplit.c`. Two polylines or open splines
//! join at the end points picked, one object with both ends picked closes,
//! a cut opens or divides; shape factors follow their points; every step is
//! one undo step.

use rustfig_engine::geom::Point;
use rustfig_engine::model::{ObjectId, ObjectKind, PolyKind, Polyline, Spline};
use rustfig_engine::{Button, Engine, SnapMode, ToolEvent, ToolKind};
use serde_json::{json, Value};

fn engine() -> Engine {
    let mut e = Engine::new();
    e.indicator.snap = SnapMode::Any;
    e
}

fn pts(v: &[(f64, f64)]) -> Value {
    json!(v.iter().map(|(x, y)| json!([x, y])).collect::<Vec<_>>())
}

fn line(e: &mut Engine, v: &[(f64, f64)]) -> ObjectId {
    e.execute("shape.polyline", &json!({"points": pts(v)})).unwrap();
    e.doc().objects.last().unwrap().id
}

fn spline(e: &mut Engine, v: &[(f64, f64)], kind: &str) -> ObjectId {
    e.execute("shape.spline", &json!({"points": pts(v), "kind": kind}))
        .unwrap();
    e.doc().objects.last().unwrap().id
}

fn poly(e: &Engine, i: usize) -> Polyline {
    match &e.doc().objects[i].kind {
        ObjectKind::Polyline(p) => p.clone(),
        k => panic!("not a polyline: {k:?}"),
    }
}

fn spl(e: &Engine, i: usize) -> Spline {
    match &e.doc().objects[i].kind {
        ObjectKind::Spline(s) => s.clone(),
        k => panic!("not a spline: {k:?}"),
    }
}

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn snapshot(e: &Engine) -> Value {
    serde_json::to_value(e.doc()).unwrap()
}

fn join(
    e: &mut Engine,
    a: ObjectId,
    ea: &str,
    b: Option<ObjectId>,
    eb: &str,
) -> Result<Value, rustfig_engine::EngineError> {
    let mut params = json!({"id": a.0, "end": ea, "other_end": eb});
    if let Some(b) = b {
        params["other"] = json!(b.0);
    }
    e.execute("object.join", &params)
}

#[test]
fn joins_two_lines_in_every_orientation() {
    // (end of the first, end of the second) -> the points of the result.
    let a = [(0.0, 0.0), (100.0, 0.0), (200.0, 0.0)];
    let b = [(500.0, 0.0), (600.0, 0.0), (700.0, 0.0)];
    type Case = (&'static str, &'static str, [(f64, f64); 6]);
    let cases: [Case; 4] = [
        ("last", "first", [a[0], a[1], a[2], b[0], b[1], b[2]]),
        ("last", "last", [a[0], a[1], a[2], b[2], b[1], b[0]]),
        ("first", "first", [a[2], a[1], a[0], b[0], b[1], b[2]]),
        ("first", "last", [a[2], a[1], a[0], b[2], b[1], b[0]]),
    ];
    for (e1, e2, want) in cases {
        let mut e = engine();
        let (la, lb) = (line(&mut e, &a), line(&mut e, &b));
        let before = snapshot(&e);
        join(&mut e, la, e1, Some(lb), e2).unwrap();
        assert_eq!(e.doc().objects.len(), 1, "{e1} {e2}");
        let got: Vec<Point> = poly(&e, 0).points;
        let want: Vec<Point> = want.iter().map(|(x, y)| p(*x, *y)).collect();
        assert_eq!(got, want, "{e1} {e2}");
        assert_eq!(e.selection().len(), 1);
        // One undo step.
        assert!(e.undo());
        assert_eq!(snapshot(&e), before);
    }
}

#[test]
fn the_new_line_has_the_first_lines_attributes_and_the_far_arrows() {
    let mut e = engine();
    let a = line(&mut e, &[(0.0, 0.0), (100.0, 0.0)]);
    e.execute(
        "object.set_attributes",
        &json!({"ids": [a.0], "thickness": 4, "pen_color": 4, "depth": 70, "forward_arrow": true}),
    )
    .unwrap();
    let b = line(&mut e, &[(300.0, 0.0), (400.0, 0.0)]);
    e.execute(
        "object.set_attributes",
        &json!({"ids": [b.0], "backward_arrow": true, "forward_arrow": true}),
    )
    .unwrap();
    // Join the end of a (with its forward arrow) to the start of b.
    join(&mut e, a, "last", Some(b), "first").unwrap();
    let l = poly(&e, 0);
    assert_eq!((l.style.thickness, l.style.depth), (4, 70));
    assert_eq!(l.style.pen_color.0, 4);
    assert!(l.backward_arrow.is_none(), "a had none at its far end");
    assert!(l.forward_arrow.is_some(), "b's far arrow stays");
    assert_eq!(l.points.len(), 4);
}

#[test]
fn both_ends_of_one_line_make_a_polygon() {
    let mut e = engine();
    let a = line(&mut e, &[(0.0, 0.0), (1000.0, 0.0), (1000.0, 800.0)]);
    e.execute("object.toggle_forward_arrow", &json!({"ids": [a.0]}))
        .unwrap();
    assert!(join(&mut e, a, "first", None, "first").is_err(), "the same end twice");
    join(&mut e, a, "first", None, "last").unwrap();
    let l = poly(&e, 0);
    assert_eq!(l.kind, PolyKind::Polygon);
    assert_eq!(l.points.len(), 3, "the model does not repeat the first point");
    assert!(l.forward_arrow.is_none());
    // Two points cannot make a polygon.
    let mut e = engine();
    let a = line(&mut e, &[(0.0, 0.0), (1000.0, 0.0)]);
    let before = snapshot(&e);
    assert!(join(&mut e, a, "first", None, "last").is_err());
    assert_eq!(snapshot(&e), before);
}

#[test]
fn only_open_lines_and_splines_of_one_kind_join() {
    let mut e = engine();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100}))
        .unwrap();
    let bx = e.doc().objects[0].id;
    let l = line(&mut e, &[(0.0, 0.0), (100.0, 0.0)]);
    let s = spline(&mut e, &[(0.0, 0.0), (100.0, 100.0), (200.0, 0.0)], "approximated");
    let c = spline(&mut e, &[(0.0, 0.0), (100.0, 100.0), (200.0, 0.0)], "x");
    e.execute("object.toggle_closed", &json!({"ids": [c.0]})).unwrap();
    let before = snapshot(&e);
    for (a, b) in [(bx, l), (l, bx), (l, s), (s, l), (s, c), (c, c)] {
        assert!(join(&mut e, a, "last", Some(b), "first").is_err(), "{a:?} {b:?}");
        assert_eq!(snapshot(&e), before);
    }
    assert!(join(&mut e, l, "middle", Some(l), "first").is_err());
    assert!(join(&mut e, ObjectId(99999), "first", None, "last").is_err());
    assert!(join(&mut e, l, "first", Some(ObjectId(99999)), "last").is_err());
}

#[test]
fn splines_join_with_their_shape_factors_in_step() {
    let mut e = engine();
    let a = spline(&mut e, &[(0.0, 0.0), (100.0, 100.0), (200.0, 0.0)], "x");
    let b = spline(&mut e, &[(500.0, 0.0), (600.0, 100.0), (700.0, 0.0)], "x");
    // Distinguish every factor.
    e.execute("object.set_geometry", &json!({"id": a.0, "shapes": [0.0, 0.25, 0.0]}))
        .unwrap();
    e.execute("object.set_geometry", &json!({"id": b.0, "shapes": [0.0, -0.5, 0.0]}))
        .unwrap();
    let before = snapshot(&e);
    // First end of a to last end of b: a reversed after b.
    join(&mut e, a, "first", Some(b), "last").unwrap();
    let s = spl(&e, 0);
    assert_eq!(
        s.points,
        vec![
            p(200.0, 0.0),
            p(100.0, 100.0),
            p(0.0, 0.0),
            p(700.0, 0.0),
            p(600.0, 100.0),
            p(500.0, 0.0)
        ]
    );
    assert_eq!(s.shapes, vec![0.0, 0.25, 0.0, 0.0, -0.5, 0.0]);
    assert!(!s.closed);
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
    // Last to first keeps both in order.
    join(&mut e, a, "last", Some(b), "first").unwrap();
    let s = spl(&e, 0);
    assert_eq!(s.points.len(), 6);
    assert_eq!(s.shapes, vec![0.0, 0.25, 0.0, 0.0, -0.5, 0.0]);
}

#[test]
fn both_ends_of_one_spline_close_it_smoothly() {
    let mut e = engine();
    let a = spline(
        &mut e,
        &[(0.0, 0.0), (100.0, 100.0), (200.0, 0.0), (100.0, -100.0)],
        "x",
    );
    e.execute(
        "object.set_geometry",
        &json!({"id": a.0, "shapes": [0.0, 0.5, -0.5, 0.0]}),
    )
    .unwrap();
    join(&mut e, a, "last", None, "first").unwrap();
    let s = spl(&e, 0);
    assert!(s.closed);
    assert_eq!(s.points.len(), 4, "no duplicate point");
    assert_eq!(
        s.shapes,
        vec![0.5, 0.5, -0.5, 0.5],
        "the ends take the second point's factor"
    );
    assert!(s.forward_arrow.is_none() && s.backward_arrow.is_none());
}

#[test]
fn splits_a_line_at_a_vertex_or_on_a_segment() {
    // At the vertex: it stays with the first piece; the segment after it is gone.
    let mut e = engine();
    let a = line(&mut e, &[(0.0, 0.0), (100.0, 0.0), (200.0, 0.0), (300.0, 0.0)]);
    let before = snapshot(&e);
    e.execute(
        "object.split",
        &json!({"id": a.0, "x": 102.0, "y": 3.0, "tolerance": 10.0}),
    )
    .unwrap();
    assert_eq!(e.doc().objects.len(), 2);
    assert_eq!(poly(&e, 0).points, vec![p(0.0, 0.0), p(100.0, 0.0)]);
    assert_eq!(poly(&e, 1).points, vec![p(200.0, 0.0), p(300.0, 0.0)]);
    assert_eq!(e.selection().len(), 2);
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
    // On a segment, away from the vertices: cut after the segment's start.
    e.execute(
        "object.split",
        &json!({"id": a.0, "x": 150.0, "y": 1.0, "tolerance": 10.0}),
    )
    .unwrap();
    assert_eq!(poly(&e, 0).points, vec![p(0.0, 0.0), p(100.0, 0.0)]);
    assert_eq!(poly(&e, 1).points, vec![p(200.0, 0.0), p(300.0, 0.0)]);
    assert!(e.undo());
    // By vertex index.
    e.execute("object.split", &json!({"id": a.0, "index": 2})).unwrap();
    assert_eq!(poly(&e, 0).points.len(), 3);
    assert_eq!(poly(&e, 1).points, vec![p(300.0, 0.0)]);
}

#[test]
fn the_ends_of_a_line_cannot_be_split_and_arrows_stay_at_the_far_ends() {
    let mut e = engine();
    let a = line(&mut e, &[(0.0, 0.0), (100.0, 0.0), (200.0, 0.0), (300.0, 0.0)]);
    e.execute("object.toggle_forward_arrow", &json!({"ids": [a.0]}))
        .unwrap();
    e.execute("object.toggle_backward_arrow", &json!({"ids": [a.0]}))
        .unwrap();
    let before = snapshot(&e);
    for bad in [
        json!({"id": a.0, "index": 0}),
        json!({"id": a.0, "index": 3}),
        json!({"id": a.0, "index": 9}),
        json!({"id": a.0}),
        json!({"id": a.0, "x": 500.0, "y": 50.0, "tolerance": 5.0}),
    ] {
        assert!(e.execute("object.split", &bad).is_err(), "{bad}");
        assert_eq!(snapshot(&e), before);
    }
    e.execute("object.split", &json!({"id": a.0, "index": 1})).unwrap();
    let (l1, l2) = (poly(&e, 0), poly(&e, 1));
    assert!(l1.backward_arrow.is_some() && l1.forward_arrow.is_none());
    assert!(l2.forward_arrow.is_some() && l2.backward_arrow.is_none());
}

#[test]
fn a_polygon_or_box_opens_into_a_polyline_starting_after_the_cut() {
    let mut e = engine();
    e.execute(
        "shape.polygon",
        &json!({"points": pts(&[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)])}),
    )
    .unwrap();
    let before = snapshot(&e);
    let id = e.doc().objects[0].id;
    // Cut at vertex 1: the loop starts at the vertex after it, ends on it.
    e.execute("object.split", &json!({"id": id.0, "index": 1})).unwrap();
    let l = poly(&e, 0);
    assert_eq!(l.kind, PolyKind::Polyline);
    assert_eq!(
        l.points,
        vec![p(100.0, 100.0), p(0.0, 100.0), p(0.0, 0.0), p(100.0, 0.0)]
    );
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
    // Cut on the closing segment: the polygon's own order.
    e.execute(
        "object.split",
        &json!({"id": id.0, "x": 0.0, "y": 50.0, "tolerance": 5.0}),
    )
    .unwrap();
    assert_eq!(
        poly(&e, 0).points,
        vec![p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0), p(0.0, 100.0)]
    );
    // A box becomes a four-point polyline.
    let mut e = engine();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100}))
        .unwrap();
    let id = e.doc().objects[0].id;
    e.execute("object.split", &json!({"id": id.0, "index": 0})).unwrap();
    let l = poly(&e, 0);
    assert_eq!((l.kind, l.points.len()), (PolyKind::Polyline, 4));
    assert_eq!(l.points.last(), Some(&p(0.0, 0.0)), "ends on the clicked corner");
}

#[test]
fn pictures_and_other_objects_cannot_be_split() {
    let mut e = engine();
    e.execute("shape.circle", &json!({"cx": 0, "cy": 0, "r": 100})).unwrap();
    e.execute("shape.text", &json!({"x": 0, "y": 500, "text": "t"}))
        .unwrap();
    let before = snapshot(&e);
    for o in e.doc().objects.clone() {
        assert!(e.execute("object.split", &json!({"id": o.id.0, "index": 0})).is_err());
    }
    assert!(e.execute("object.split", &json!({"id": 99999, "index": 0})).is_err());
    assert!(e.execute("object.split", &json!({"id": "a", "index": 0})).is_err());
    assert_eq!(snapshot(&e), before);
}

#[test]
fn splits_an_open_spline_with_sharp_new_ends() {
    let mut e = engine();
    let a = spline(
        &mut e,
        &[(0.0, 0.0), (100.0, 100.0), (200.0, 0.0), (300.0, 100.0), (400.0, 0.0)],
        "x",
    );
    e.execute(
        "object.set_geometry",
        &json!({"id": a.0, "shapes": [0.0, 0.5, 0.75, -0.5, 0.0]}),
    )
    .unwrap();
    let before = snapshot(&e);
    e.execute("object.split", &json!({"id": a.0, "index": 2})).unwrap();
    let (s1, s2) = (spl(&e, 0), spl(&e, 1));
    assert_eq!(s1.points, vec![p(0.0, 0.0), p(100.0, 100.0), p(200.0, 0.0)]);
    assert_eq!(s1.shapes, vec![0.0, 0.5, 0.0], "the cut end is sharp");
    assert_eq!(s2.points, vec![p(300.0, 100.0), p(400.0, 0.0)]);
    assert_eq!(s2.shapes, vec![0.0, 0.0]);
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
    // No piece may have one point: only vertex 1 and 2 (and the segments
    // from 1 to 2 and 2 to 3) are allowed on five points.
    for bad in [0, 3, 4] {
        assert!(
            e.execute("object.split", &json!({"id": a.0, "index": bad})).is_err(),
            "{bad}"
        );
    }
    e.execute(
        "object.split",
        &json!({"id": a.0, "x": 150.0, "y": 50.0, "tolerance": 100.0}),
    )
    .unwrap();
    assert_eq!(spl(&e, 0).points.len(), 2);
    assert_eq!(spl(&e, 1).points.len(), 3);
}

#[test]
fn a_closed_spline_opens_at_the_cut() {
    let mut e = engine();
    let a = spline(
        &mut e,
        &[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
        "approximated",
    );
    e.execute("object.toggle_closed", &json!({"ids": [a.0]})).unwrap();
    e.execute(
        "object.set_geometry",
        &json!({"id": a.0, "shapes": [1.0, 0.5, 0.25, -1.0]}),
    )
    .unwrap();
    assert!(spl(&e, 0).closed);
    let before = snapshot(&e);
    // Cut at vertex 1: starts at vertex 2, ends at vertex 1.
    e.execute("object.split", &json!({"id": a.0, "index": 1})).unwrap();
    let s = spl(&e, 0);
    assert!(!s.closed);
    assert_eq!(
        s.points,
        vec![p(100.0, 100.0), p(0.0, 100.0), p(0.0, 0.0), p(100.0, 0.0)]
    );
    assert_eq!(s.shapes, vec![0.0, -1.0, 1.0, 0.0]);
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
    // The last vertex: the new spline starts at the first point.
    e.execute("object.split", &json!({"id": a.0, "index": 3})).unwrap();
    assert_eq!(spl(&e, 0).points[0], p(0.0, 0.0));
}

#[test]
fn split_then_join_gives_the_line_back() {
    let mut e = engine();
    let a = line(&mut e, &[(0.0, 0.0), (100.0, 0.0), (200.0, 0.0), (300.0, 0.0)]);
    let orig = poly(&e, 0).points;
    e.execute("object.split", &json!({"id": a.0, "index": 1})).unwrap();
    let ids: Vec<ObjectId> = e.doc().objects.iter().map(|o| o.id).collect();
    join(&mut e, ids[0], "last", Some(ids[1]), "first").unwrap();
    let joined = poly(&e, 0).points;
    // The segment between the pieces was lost by the split; the join draws it again.
    assert_eq!(joined, orig);
}

// ---- the mode ------------------------------------------------------------

fn press(e: &mut Engine, x: f64, y: f64, button: Button) {
    e.tool_event(ToolEvent::Press {
        pos: p(x, y),
        button,
        shift: false,
    })
    .unwrap();
    e.tool_event(ToolEvent::Release { pos: p(x, y), button }).unwrap();
}

fn hints(e: &Engine) -> [&'static str; 3] {
    let h = e.mouse_hints();
    [h.left, h.middle, h.right]
}

#[test]
fn the_mode_joins_with_two_clicks_and_cancels_with_the_right_button() {
    let mut e = engine();
    line(&mut e, &[(0.0, 0.0), (1000.0, 0.0)]);
    line(&mut e, &[(3000.0, 0.0), (4000.0, 0.0)]);
    e.set_tool(ToolKind::JoinSplit);
    assert_eq!(hints(&e), ["Join Lines/Splines", "Split Line/Spline", ""]);
    // Markers on lines, splines and boxes (the whole `M_POLYLINE`).
    assert_eq!(e.overlay().markers.len(), 4);
    // A click away from an end point does nothing.
    press(&mut e, 500.0, 0.0, Button::Left);
    assert!(!e.tool_busy());
    // The first end point: marker, new hints, only lines marked.
    press(&mut e, 1000.0, 0.0, Button::Left);
    assert!(e.tool_busy());
    assert_eq!(hints(&e), ["Choose next Line", "", "cancel"]);
    assert_eq!(e.overlay().join_marker, Some(p(1000.0, 0.0)));
    assert_eq!(e.overlay().markers.len(), 4);
    // Cancel.
    press(&mut e, 0.0, 0.0, Button::Right);
    assert!(!e.tool_busy());
    assert_eq!(e.overlay().join_marker, None);
    // Again, and pick the same point (ignored) and then the second line.
    press(&mut e, 1000.0, 0.0, Button::Left);
    press(&mut e, 1000.0, 0.0, Button::Left);
    assert!(e.tool_busy());
    // Not an end point of a line: ignored.
    press(&mut e, 3500.0, 0.0, Button::Left);
    assert!(e.tool_busy());
    let before = snapshot(&e);
    press(&mut e, 3000.0, 0.0, Button::Left);
    assert!(!e.tool_busy());
    assert_eq!(e.doc().objects.len(), 1);
    assert_eq!(poly(&e, 0).points.len(), 4);
    // The mode continues; one undo step.
    assert_eq!(e.tool(), ToolKind::JoinSplit);
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
}

#[test]
fn the_mode_closes_a_line_by_both_ends_and_joins_splines() {
    let mut e = engine();
    line(&mut e, &[(0.0, 0.0), (1000.0, 0.0), (1000.0, 1000.0)]);
    e.set_tool(ToolKind::JoinSplit);
    press(&mut e, 0.0, 0.0, Button::Left);
    press(&mut e, 1000.0, 1000.0, Button::Left);
    assert_eq!(poly(&e, 0).kind, PolyKind::Polygon);
    // Splines: the second step only takes splines.
    let mut e = engine();
    spline(&mut e, &[(0.0, 0.0), (500.0, 500.0), (1000.0, 0.0)], "approximated");
    line(&mut e, &[(2000.0, 0.0), (3000.0, 0.0)]);
    spline(
        &mut e,
        &[(2000.0, 2000.0), (2500.0, 2500.0), (3000.0, 2000.0)],
        "approximated",
    );
    e.set_tool(ToolKind::JoinSplit);
    press(&mut e, 1000.0, 0.0, Button::Left);
    assert_eq!(hints(&e)[0], "Choose next Spline");
    // The line's end is not a candidate.
    press(&mut e, 2000.0, 0.0, Button::Left);
    assert_eq!(e.doc().objects.len(), 3);
    press(&mut e, 2000.0, 2000.0, Button::Left);
    assert_eq!(e.doc().objects.len(), 2);
    let s = e
        .doc()
        .objects
        .iter()
        .find_map(|o| match &o.kind {
            ObjectKind::Spline(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(s.points.len(), 6);
}

#[test]
fn the_middle_button_splits_and_a_failure_changes_nothing() {
    let mut e = engine();
    line(&mut e, &[(0.0, 0.0), (1000.0, 0.0), (2000.0, 0.0)]);
    e.set_tool(ToolKind::JoinSplit);
    let before = snapshot(&e);
    // On the first point: cannot.
    press(&mut e, 0.0, 0.0, Button::Middle);
    assert_eq!(snapshot(&e), before);
    assert!(e.status().contains("end point"), "{}", e.status());
    // Away from the object: nothing.
    press(&mut e, 5000.0, 5000.0, Button::Middle);
    assert_eq!(snapshot(&e), before);
    press(&mut e, 1000.0, 0.0, Button::Middle);
    assert_eq!(e.doc().objects.len(), 2);
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
    // The modern scheme splits with the right button.
    e.set_mouse_scheme(rustfig_engine::MouseScheme::Modern);
    e.set_tool(ToolKind::JoinSplit);
    assert_eq!(hints(&e)[2], "Split Line/Spline");
    press(&mut e, 1000.0, 0.0, Button::Right);
    assert_eq!(e.doc().objects.len(), 2);
}

#[test]
fn the_mode_works_inside_an_open_compound() {
    let mut e = engine();
    line(&mut e, &[(0.0, 0.0), (1000.0, 0.0)]);
    line(&mut e, &[(3000.0, 0.0), (4000.0, 0.0)]);
    e.execute("edit.select_all", &json!({})).unwrap();
    e.execute("object.group", &json!({})).unwrap();
    let c = e.doc().objects[0].id;
    e.execute("compound.open", &json!({"id": c.0})).unwrap();
    e.set_tool(ToolKind::JoinSplit);
    press(&mut e, 1000.0, 0.0, Button::Left);
    press(&mut e, 3000.0, 0.0, Button::Left);
    assert_eq!(e.doc().objects.len(), 1);
    e.execute("compound.close", &json!({})).unwrap();
    let ObjectKind::Compound(cmp) = &e.doc().objects[0].kind else {
        panic!("compound")
    };
    assert_eq!(cmp.objects.len(), 1, "the joined line is in the compound");
}
