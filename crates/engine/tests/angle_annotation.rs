#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Q9: the measured angle drawn (an arc with its label, a right-angle mark)
//! and a dimension line's label along the line.

use rustfig_engine::tools::{Button, ToolEvent};
use rustfig_engine::{Engine, ToolKind};
use rustfig_geom::Point;
use rustfig_model::{Object, ObjectKind, PolyKind};
use serde_json::{json, Value};

fn members(o: &Object) -> Vec<&Object> {
    match &o.kind {
        ObjectKind::Compound(c) => c.objects.iter().collect(),
        _ => vec![o],
    }
}

fn press(e: &mut Engine, x: f64, y: f64, b: Button, shift: bool) {
    e.tool_event(ToolEvent::Press {
        pos: Point::new(x, y),
        button: b,
        shift,
    })
    .unwrap();
    e.tool_event(ToolEvent::Release {
        pos: Point::new(x, y),
        button: b,
    })
    .ok();
}

#[test]
fn arc_with_label_between_the_sides() {
    let mut e = Engine::new();
    // 60 degrees at the origin: one side along x, the other 60 degrees up (y down on the screen).
    let c = (1200.0 * 0.5, -1200.0 * 0.75_f64.sqrt());
    let r = e
        .execute(
            "object.annotate_angle",
            &json!({"points": [[1200, 0], [0, 0], [c.0, c.1]]}),
        )
        .unwrap();
    assert!((r["degrees"].as_f64().unwrap() - 60.0).abs() < 1e-6);
    let o = &e.doc().objects[0];
    let m = members(o);
    assert_eq!(m.len(), 2);
    let ObjectKind::Arc(a) = &m[0].kind else {
        panic!("no arc")
    };
    let ObjectKind::Text(t) = &m[1].kind else {
        panic!("no text")
    };
    assert_eq!(t.text, "60°");
    // The arc's ends lie on the sides, a third of the side from the vertex, its middle on the bisector.
    assert!(
        a.points.iter().all(|p| (p.x.hypot(p.y) - 400.0).abs() < 1.5),
        "{:?}",
        a.points
    );
    assert!((a.points[0].y).abs() < 1.5 && a.points[0].x > 0.0);
    let bis = (30.0_f64).to_radians();
    assert!((a.points[1].x - 400.0 * bis.cos()).abs() < 1.5 && (a.points[1].y + 400.0 * bis.sin()).abs() < 1.5);
    // The label is beyond the arc, on the bisector.
    assert!(t.origin.x.hypot(t.origin.y) > 400.0);
    // One undo step.
    assert!(e.undo());
    assert!(e.doc().objects.is_empty());
    // Options: radius, label, none.
    e.execute(
        "object.annotate_angle",
        &json!({"points": [[1200, 0], [0, 0], [0, 1200]], "radius": 300, "label": "\\alpha"}),
    )
    .unwrap();
    let m = members(&e.doc().objects[0]);
    assert!(matches!(&m[1].kind, ObjectKind::Text(t) if t.text == "\\alpha"));
    e.execute(
        "object.annotate_angle",
        &json!({"points": [[1200, 0], [0, 0], [0, 1200]], "label": ""}),
    )
    .unwrap();
    assert!(matches!(&e.doc().objects[1].kind, ObjectKind::Arc(_)));
    // A straight angle: a half circle.
    e.execute(
        "object.annotate_angle",
        &json!({"points": [[1200, 0], [0, 0], [-1200, 0]]}),
    )
    .unwrap();
}

#[test]
fn right_angle_mark_needs_a_right_angle() {
    let mut e = Engine::new();
    e.execute(
        "object.annotate_angle",
        &json!({"points": [[1400, 0], [0, 0], [0, 1400]], "mark": "right"}),
    )
    .unwrap();
    let ObjectKind::Polyline(p) = &e.doc().objects[0].kind else {
        panic!()
    };
    assert_eq!(p.kind, PolyKind::Polyline);
    assert_eq!(
        p.points,
        vec![Point::new(200.0, 0.0), Point::new(200.0, 200.0), Point::new(0.0, 200.0)]
    );
    let before = e.doc().objects.len();
    let err = e
        .execute(
            "object.annotate_angle",
            &json!({"points": [[1400, 0], [0, 0], [700, 1400]], "mark": "right"}),
        )
        .unwrap_err();
    assert!(err.message.contains("not a right angle"), "{}", err.message);
    assert_eq!(e.doc().objects.len(), before);
}

#[test]
fn from_a_corner_or_an_arc_and_bad_input() {
    let mut e = Engine::new();
    let r = e
        .execute("shape.polyline", &json!({"points": [[0, 0], [1200, 0], [1200, 1200]]}))
        .unwrap();
    let out = e
        .execute(
            "object.annotate_angle",
            &json!({"id": r["id"], "x": 1200, "y": 0, "mark": "right"}),
        )
        .unwrap();
    assert!((out["degrees"].as_f64().unwrap() - 90.0).abs() < 1e-6);
    // The ends of a line have no angle.
    assert!(e
        .execute("object.annotate_angle", &json!({"id": r["id"], "x": 0, "y": 0}))
        .is_err());
    let a = e
        .execute("shape.arc", &json!({"points": [[1000, 0], [0, 1000], [-1000, 0]]}))
        .unwrap();
    e.execute("object.annotate_angle", &json!({"id": a["id"], "x": 0, "y": 0}))
        .unwrap();
    for bad in [
        json!({"points": [[0, 0], [0, 0], [5, 5]]}),
        json!({"points": [[0, 0], [5, 5]]}),
        json!({"points": [[1, 0], [0, 0], [0, 1]], "mark": "square"}),
        json!({"points": [[1, 0], [0, 0], [0, 1]], "radius": -3}),
        json!({"points": [[1, 0], [0, 0], [0, 1]], "label": 5}),
        json!({"points": [[1e308, 0], [0, 0], [0, 1e308]]}),
    ] {
        let n = e.doc().objects.len();
        let r: Result<Value, _> = e.execute("object.annotate_angle", &bad);
        if r.is_err() {
            assert_eq!(e.doc().objects.len(), n, "{bad}");
        }
    }
}

/// Angle measure mode: Shift+middle and Shift+right at the last point.
#[test]
fn shift_buttons_draw_the_angle_in_angle_measure_mode() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::AngleMeasure);
    press(&mut e, 1200.0, 0.0, Button::Left, false);
    press(&mut e, 0.0, 0.0, Button::Left, false);
    let h = e.mouse_hints_with(true);
    assert_eq!(
        (h.left, h.middle, h.right),
        ("final point", "draw arc & label", "draw right angle")
    );
    press(&mut e, 0.0, 1200.0, Button::Middle, true);
    assert!(matches!(&e.doc().objects[0].kind, ObjectKind::Compound(_)));
    assert!(!e.tool_busy());
    press(&mut e, 1200.0, 0.0, Button::Left, false);
    press(&mut e, 0.0, 0.0, Button::Left, false);
    press(&mut e, 0.0, 1200.0, Button::Right, true);
    assert!(matches!(&e.doc().objects[1].kind, ObjectKind::Polyline(_)));
    // Plain buttons still measure and cancel and draw nothing.
    press(&mut e, 1200.0, 0.0, Button::Left, false);
    press(&mut e, 0.0, 0.0, Button::Left, false);
    press(&mut e, 0.0, 1200.0, Button::Left, false);
    assert_eq!(e.doc().objects.len(), 2);
    // At a corner of a drawn line: Shift+middle and Shift+right.
    let l = e
        .execute(
            "shape.polyline",
            &json!({"points": [[3000, 0], [4000, 0], [4000, 1000]]}),
        )
        .unwrap();
    assert!(l["id"].is_u64());
    let h = e.mouse_hints_with(true);
    assert_eq!((h.middle, h.right), ("select & draw arc", "select & right angle"));
    press(&mut e, 4000.0, 0.0, Button::Right, true);
    assert_eq!(e.doc().objects.len(), 4);
}

#[test]
fn dimension_label_along_the_line() {
    let mut e = Engine::new();
    let boxed = e
        .execute(
            "shape.dimension_line",
            &json!({"x1": 0, "y1": 1000, "x2": 3000, "y2": 1000}),
        )
        .unwrap();
    let along = e
        .execute(
            "shape.dimension_line",
            &json!({"x1": 0, "y1": 1000, "x2": 3000, "y2": 1000, "label_along": true}),
        )
        .unwrap();
    let get = |e: &Engine, r: &Value| {
        e.doc()
            .get(rustfig_model::ObjectId(r["id"].as_u64().unwrap()))
            .unwrap()
            .clone()
    };
    let (b, a) = (get(&e, &boxed), get(&e, &along));
    let count = |o: &Object, poly: PolyKind| {
        members(o)
            .iter()
            .filter(|m| matches!(&m.kind, ObjectKind::Polyline(p) if p.kind == poly))
            .count()
    };
    assert_eq!(count(&b, PolyKind::Polygon), 1);
    assert_eq!(count(&a, PolyKind::Polygon), 0, "no box to interrupt the line");
    let text = |o: &Object| {
        members(o)
            .iter()
            .find_map(|m| {
                if let ObjectKind::Text(t) = &m.kind {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .unwrap()
    };
    let (tb, ta) = (text(&b), text(&a));
    assert!(
        ta.origin.y < tb.origin.y - 40.0,
        "above the line: {:?} {:?}",
        ta.origin,
        tb.origin
    );
    assert_eq!(ta.text, tb.text);
    assert_eq!(ta.angle, 0.0);
    // Recomputing (after an edit) keeps it along the line: the text stays above, no box comes back.
    e.execute("object.recompute_dimension", &json!({})).unwrap();
    let a2 = get(&e, &along);
    assert_eq!(count(&a2, PolyKind::Polygon), 0);
    assert_eq!(text(&a2).origin, ta.origin);
    // The indicator's setting is the default.
    e.execute("indicator.set", &json!({"dimline": {"label_along": true}}))
        .unwrap();
    let d = e
        .execute("shape.dimension_line", &json!({"x1": 0, "y1": 0, "x2": 900, "y2": 900}))
        .unwrap();
    assert_eq!(count(&get(&e, &d), PolyKind::Polygon), 0);
    let d = e
        .execute(
            "shape.dimension_line",
            &json!({"x1": 0, "y1": 0, "x2": 900, "y2": 900, "label_along": false}),
        )
        .unwrap();
    assert_eq!(count(&get(&e, &d), PolyKind::Polygon), 1);
}
