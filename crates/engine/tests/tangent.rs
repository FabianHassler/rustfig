#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Tangent/normal mode (plan D4): a line through the clicked point of a
//! circle, ellipse, arc, line, polygon, box or spline, along its tangent
//! (left button) or its normal (middle button), one undo step. Its total
//! length is the indicator's Tangent/Norm Length in inches or cm.

use rustfig_engine::geom::Point;
use rustfig_engine::model::{ObjectId, ObjectKind, Units};
use rustfig_engine::{Button, Engine, SnapMode, ToolEvent, ToolKind};
use serde_json::{json, Value};

fn engine() -> Engine {
    let mut e = Engine::new();
    e.indicator.snap = SnapMode::Any;
    e
}

fn last_id(e: &Engine) -> ObjectId {
    e.doc().objects.last().unwrap().id
}

fn points(e: &Engine, id: ObjectId) -> Vec<Point> {
    match &e.doc().get(id).unwrap().kind {
        ObjectKind::Polyline(p) => p.points.clone(),
        k => panic!("not a polyline: {k:?}"),
    }
}

/// Run the command and return the three points of the new line.
fn tangent(e: &mut Engine, id: ObjectId, x: f64, y: f64, normal: bool, extra: Value) -> Vec<Point> {
    let mut p = json!({"id": id.0, "x": x, "y": y, "normal": normal});
    for (k, v) in extra.as_object().unwrap() {
        p[k] = v.clone();
    }
    let r = e.execute("object.tangent_line", &p).unwrap();
    let new = ObjectId(r["id"].as_u64().unwrap());
    points(e, new)
}

/// Direction of the line, as a unit vector.
fn dir(pts: &[Point]) -> (f64, f64) {
    let (a, b) = (pts[0], pts[2]);
    let l = (b.x - a.x).hypot(b.y - a.y);
    ((b.x - a.x) / l, (b.y - a.y) / l)
}

fn length(pts: &[Point]) -> f64 {
    (pts[2].x - pts[0].x).hypot(pts[2].y - pts[0].y)
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// Same direction up to sign.
fn parallel(d: (f64, f64), want: (f64, f64)) -> bool {
    (d.0 * want.0 + d.1 * want.1).abs() > 0.9995
}

#[test]
fn circle_tangent_and_normal() {
    let mut e = engine();
    e.execute("shape.circle", &json!({"cx": 3000, "cy": 3000, "r": 1000}))
        .unwrap();
    let c = last_id(&e);
    // At the top of the circle (y grows downwards) the tangent is horizontal.
    let t = tangent(&mut e, c, 3000.0, 2000.0, false, json!({}));
    assert_eq!(t.len(), 3);
    assert_eq!(t[1], Point::new(3000.0, 2000.0), "the middle point is the curve point");
    assert!(parallel(dir(&t), (1.0, 0.0)));
    assert!(close(length(&t), 1200.0, 2.0), "default length 1 inch: {}", length(&t));
    // The normal through the same point is vertical, through the centre.
    let n = tangent(&mut e, c, 3000.0, 2000.0, true, json!({}));
    assert!(parallel(dir(&n), (0.0, 1.0)));
    assert!(close(n[1].x, 3000.0, 0.0) && close(n[1].y, 2000.0, 0.0));
    // A point at 45 degrees (click off the curve by a little: the curve
    // point is on the circle, along the ray from the centre).
    let r = 1000.0_f64;
    let (cx, cy) = (3000.0 + r * 0.5_f64.sqrt(), 3000.0 + r * 0.5_f64.sqrt());
    let t = tangent(&mut e, c, cx + 3.0, cy + 3.0, false, json!({"tolerance": 10}));
    assert!(close(t[1].x, cx, 1.0) && close(t[1].y, cy, 1.0));
    assert!(parallel(dir(&t), (0.5_f64.sqrt(), -0.5_f64.sqrt())));
}

#[test]
fn ellipse_tangent_matches_the_outline() {
    let mut e = engine();
    // An axis-parallel ellipse: at the end of the minor axis the tangent is horizontal.
    e.execute("shape.ellipse", &json!({"cx": 2000, "cy": 2000, "rx": 1500, "ry": 500}))
        .unwrap();
    let id = last_id(&e);
    let t = tangent(&mut e, id, 2000.0, 2500.0, false, json!({}));
    assert!(parallel(dir(&t), (1.0, 0.0)));
    // At the major axis end it is vertical.
    let t = tangent(&mut e, id, 3500.0, 2000.0, false, json!({}));
    assert!(parallel(dir(&t), (0.0, 1.0)));
    // At a general point: the tangent of x^2/a^2 + y^2/b^2 = 1 at (x0, y0) is
    // perpendicular to (x0/a^2, y0/b^2).
    let th = 0.7_f64;
    let (x0, y0) = (1500.0 * th.cos(), 500.0 * th.sin());
    let t = tangent(&mut e, id, 2000.0 + x0, 2000.0 + y0, false, json!({}));
    let g = (x0 / (1500.0 * 1500.0), y0 / (500.0 * 500.0));
    let gl = g.0.hypot(g.1);
    assert!(parallel(dir(&t), (-g.1 / gl, g.0 / gl)));
    // The normal is the gradient direction.
    let n = tangent(&mut e, id, 2000.0 + x0, 2000.0 + y0, true, json!({}));
    assert!(parallel(dir(&n), (g.0 / gl, g.1 / gl)));
}

#[test]
fn rotated_ellipse_tangent_matches_the_drawn_outline() {
    let mut e = engine();
    e.execute(
        "shape.ellipse",
        &json!({"cx": 2000, "cy": 2000, "rx": 1500, "ry": 500, "angle": 30}),
    )
    .unwrap();
    let id = last_id(&e);
    // Sample the drawn outline (the renderer's flattening) and check the
    // line follows it: points of the outline near the curve point are
    // along the tangent.
    let o = e.doc().get(id).unwrap().clone();
    let flat = rustfig_engine::model::flatten_object(&o);
    let pts = &flat[0].pts;
    for k in [3usize, 17, 29, 41] {
        let a = pts[k % (pts.len() - 1)];
        let b = pts[(k + 1) % (pts.len() - 1)];
        let mid = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
        let want = ((b.x - a.x) / a.dist(b), (b.y - a.y) / a.dist(b));
        let t = tangent(&mut e, id, mid.x, mid.y, false, json!({"tolerance": 30}));
        assert!(
            close(t[1].x, mid.x, 20.0) && close(t[1].y, mid.y, 20.0),
            "on the outline at {k}"
        );
        let d = dir(&t);
        assert!(
            (d.0 * want.0 + d.1 * want.1).abs() > 0.99,
            "tangent {d:?} follows the outline {want:?} at {k}"
        );
    }
}

#[test]
fn arc_tangent_is_perpendicular_to_the_radius() {
    let mut e = engine();
    e.execute(
        "shape.arc",
        &json!({"points": [[3000, 2000], [4000, 3000], [3000, 4000]]}),
    )
    .unwrap();
    let id = last_id(&e);
    let a = match &e.doc().get(id).unwrap().kind {
        ObjectKind::Arc(a) => a.clone(),
        k => panic!("{k:?}"),
    };
    // The middle point (4000, 3000) on the circle around (3000, 3000).
    assert!(close(a.center.x, 3000.0, 1.0) && close(a.center.y, 3000.0, 1.0));
    let t = tangent(&mut e, id, 4000.0, 3000.0, false, json!({}));
    assert!(parallel(dir(&t), (0.0, 1.0)));
    let n = tangent(&mut e, id, 4000.0, 3000.0, true, json!({}));
    assert!(parallel(dir(&n), (1.0, 0.0)));
    // Off the arc's span (the left of the circle): no curve point there.
    let r = e.execute("object.tangent_line", &json!({"id": id.0, "x": 2000, "y": 3000}));
    assert!(r.is_err());
}

#[test]
fn line_polygon_and_box_sides() {
    let mut e = engine();
    e.execute(
        "shape.polyline",
        &json!({"points": [[0, 0], [3000, 4000], [6000, 4000]]}),
    )
    .unwrap();
    let l = last_id(&e);
    // On the first side (direction 3:4, a 3-4-5 triangle).
    let t = tangent(&mut e, l, 1500.0, 2000.0, false, json!({}));
    assert!(parallel(dir(&t), (0.6, 0.8)));
    assert_eq!(t[1], Point::new(1500.0, 2000.0));
    let n = tangent(&mut e, l, 1500.0, 2000.0, true, json!({}));
    assert!(parallel(dir(&n), (-0.8, 0.6)));
    // On the second side.
    let t = tangent(&mut e, l, 4500.0, 4003.0, false, json!({"tolerance": 5}));
    assert!(parallel(dir(&t), (1.0, 0.0)));
    assert_eq!(t[1], Point::new(4500.0, 4000.0), "the foot of the click on the side");

    e.execute(
        "shape.rectangle",
        &json!({"x": 10000, "y": 0, "width": 2000, "height": 1000}),
    )
    .unwrap();
    let b = last_id(&e);
    // The closing side (left side of the box) too.
    let t = tangent(&mut e, b, 10000.0, 500.0, false, json!({}));
    assert!(parallel(dir(&t), (0.0, 1.0)));
    let t = tangent(&mut e, b, 11000.0, 1000.0, false, json!({}));
    assert!(parallel(dir(&t), (1.0, 0.0)));

    e.execute(
        "shape.polygon",
        &json!({"points": [[0, 8000], [2000, 8000], [1000, 9000]]}),
    )
    .unwrap();
    let g = last_id(&e);
    let t = tangent(&mut e, g, 500.0, 8500.0, true, json!({}));
    let r = 0.5_f64.sqrt();
    assert!(parallel(dir(&t), (-r, r)), "normal to the closing side of the polygon");
}

#[test]
fn spline_tangent_follows_the_curve() {
    let mut e = engine();
    e.execute(
        "shape.spline",
        &json!({"points": [[0, 3000], [2000, 0], [4000, 3000]], "kind": "x"}),
    )
    .unwrap();
    let id = last_id(&e);
    let o = e.doc().get(id).unwrap().clone();
    let flat = rustfig_engine::model::flatten_object(&o);
    let pts = &flat[0].pts;
    // At the curve's apex the tangent is horizontal; find the top point.
    let top = pts
        .iter()
        .copied()
        .min_by(|a, b| a.y.partial_cmp(&b.y).unwrap())
        .unwrap();
    let t = tangent(&mut e, id, top.x, top.y, false, json!({"tolerance": 20}));
    let d = dir(&t);
    assert!(d.1.abs() < 0.08, "horizontal at the apex: {d:?}");
    // Elsewhere it matches the flattened curve.
    let k = pts.len() / 4;
    let (a, b) = (pts[k], pts[k + 1]);
    let mid = Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let t = tangent(&mut e, id, mid.x, mid.y, false, json!({"tolerance": 20}));
    let want = ((b.x - a.x) / a.dist(b), (b.y - a.y) / a.dist(b));
    assert!(parallel(dir(&t), want) || (dir(&t).0 * want.0 + dir(&t).1 * want.1).abs() > 0.97);
}

#[test]
fn length_follows_the_indicator_and_the_units() {
    let mut e = engine();
    e.execute("shape.polyline", &json!({"points": [[0, 0], [6000, 0]]}))
        .unwrap();
    let l = last_id(&e);
    let t = tangent(&mut e, l, 3000.0, 0.0, false, json!({}));
    assert_eq!(
        t,
        vec![
            Point::new(2400.0, 0.0),
            Point::new(3000.0, 0.0),
            Point::new(3600.0, 0.0)
        ]
    );
    e.execute("indicator.set", &json!({"tangent_length": 2.5})).unwrap();
    let t = tangent(&mut e, l, 3000.0, 0.0, false, json!({}));
    assert!(close(length(&t), 3000.0, 1.0));
    // The command's own length wins.
    let t = tangent(&mut e, l, 3000.0, 0.0, false, json!({"length": 0.5}));
    assert!(close(length(&t), 600.0, 1.0));
    // Metric drawings: centimetres, 450 per cm.
    e.doc_mut().settings.units = Units::Metric;
    let t = tangent(&mut e, l, 3000.0, 0.0, false, json!({"length": 2}));
    assert!(close(length(&t), 900.0, 1.0));
    // The setting is limited to 0..20 like xfig's.
    e.execute("indicator.set", &json!({"tangent_length": 99})).unwrap();
    assert_eq!(e.indicator.tangent_length, 20.0);
    e.execute("indicator.set", &json!({"tangent_length": -1})).unwrap();
    assert_eq!(e.indicator.tangent_length, 0.0);
    assert!(e
        .execute("object.tangent_line", &json!({"id": l.0, "x": 3000, "y": 0}))
        .is_err());
    assert!(e
        .execute(
            "object.tangent_line",
            &json!({"id": l.0, "x": 3000, "y": 0, "length": 21})
        )
        .is_err());
}

#[test]
fn the_line_takes_the_current_attributes() {
    let mut e = engine();
    e.execute("shape.polyline", &json!({"points": [[0, 0], [6000, 0]]}))
        .unwrap();
    let l = last_id(&e);
    e.execute(
        "indicator.set",
        &json!({"thickness": 3, "pen_color": 4, "depth": 70, "forward_arrow": true}),
    )
    .unwrap();
    tangent(&mut e, l, 3000.0, 0.0, false, json!({}));
    let o = e.doc().objects.last().unwrap();
    assert_eq!(o.depth(), 70);
    match &o.kind {
        ObjectKind::Polyline(p) => {
            assert_eq!(p.style.thickness, 3);
            assert_eq!(p.style.pen_color.0, 4);
            assert!(p.forward_arrow.is_some() && p.backward_arrow.is_none());
        }
        k => panic!("{k:?}"),
    }
}

#[test]
fn one_undo_step_and_the_original_stays() {
    let mut e = engine();
    e.execute("shape.circle", &json!({"cx": 3000, "cy": 3000, "r": 1000}))
        .unwrap();
    let c = last_id(&e);
    let before = e.doc().objects.len();
    tangent(&mut e, c, 3000.0, 2000.0, false, json!({}));
    assert_eq!(e.doc().objects.len(), before + 1);
    assert!(e.undo());
    assert_eq!(e.doc().objects.len(), before);
    assert!(e.doc().get(c).is_some());
}

#[test]
fn refused_kinds_and_singularities() {
    let mut e = engine();
    e.execute("shape.text", &json!({"x": 100, "y": 1000, "text": "hello"}))
        .unwrap();
    let t = last_id(&e);
    e.execute("shape.polyline", &json!({"points": [[0, 0], [3000, 0], [3000, 3000]]}))
        .unwrap();
    let l = last_id(&e);
    e.execute("edit.select_all", &json!({})).unwrap();
    e.execute("compound.group", &json!({})).ok();
    let n = e.doc().objects.len();
    let r = e.execute("object.tangent_line", &json!({"id": t.0, "x": 100, "y": 1000}));
    assert!(r.is_err(), "text is refused");
    // A compound is refused too, with the same message.
    if let Some(c) = e
        .doc()
        .objects
        .iter()
        .find(|o| matches!(o.kind, ObjectKind::Compound(_)))
    {
        let r = e.execute("object.tangent_line", &json!({"id": c.id.0, "x": 0, "y": 0}));
        assert!(r.unwrap_err().message.contains("work on"));
    }
    // On the corner of a polyline there is no tangent.
    let mut e2 = engine();
    e2.execute("shape.polyline", &json!({"points": [[0, 0], [3000, 0], [3000, 3000]]}))
        .unwrap();
    let l2 = last_id(&e2);
    let _ = l;
    let r = e2.execute("object.tangent_line", &json!({"id": l2.0, "x": 0, "y": 0}));
    assert_eq!(r.unwrap_err().message, "singularity, can't draw tangent/normal");
    assert_eq!(e2.doc().objects.len(), 1, "nothing was added");
    // Not on the curve at all.
    assert!(e2
        .execute("object.tangent_line", &json!({"id": l2.0, "x": 1500, "y": 2000}))
        .is_err());
    let _ = n;
}

#[test]
fn the_mode_buttons() {
    let mut e = engine();
    e.execute("shape.circle", &json!({"cx": 3000, "cy": 3000, "r": 1000}))
        .unwrap();
    e.execute("shape.text", &json!({"x": 6000, "y": 6000, "text": "hello"}))
        .unwrap();
    e.execute("tool.tangent", &json!({})).unwrap();
    assert_eq!(e.tool(), ToolKind::Tangent);
    let h = e.mouse_hints();
    assert_eq!((h.left, h.middle, h.right), ("add tangent", "add normal", ""));
    let press = |e: &mut Engine, x: f64, y: f64, button: Button| {
        e.tool_event(ToolEvent::Press {
            pos: Point::new(x, y),
            button,
            shift: false,
        })
        .unwrap();
    };
    let n = e.doc().objects.len();
    // Left: tangent.
    press(&mut e, 3000.0, 2000.0, Button::Left);
    assert_eq!(e.doc().objects.len(), n + 1);
    assert_eq!(e.status(), "added tangent");
    let t = points(&e, last_id(&e));
    assert!(parallel(dir(&t), (1.0, 0.0)));
    // Middle: normal.
    press(&mut e, 3000.0, 2000.0, Button::Middle);
    assert_eq!(e.doc().objects.len(), n + 2);
    assert_eq!(e.status(), "added normal");
    let t = points(&e, last_id(&e));
    assert!(parallel(dir(&t), (0.0, 1.0)));
    // Right: nothing. Empty canvas and text: nothing.
    press(&mut e, 3000.0, 2000.0, Button::Right);
    press(&mut e, 9000.0, 9000.0, Button::Left);
    press(&mut e, 6000.0, 6000.0, Button::Left);
    assert_eq!(e.doc().objects.len(), n + 2);
    // Each click is one undo step.
    assert!(e.undo());
    assert_eq!(e.doc().objects.len(), n + 1);
    // The mode's key and id.
    assert_eq!(ToolKind::from_id("tool.tangent"), Some(ToolKind::Tangent));
    assert_eq!(ToolKind::Tangent.shortcut(), "n");
}

#[test]
fn the_search_order_is_xfigs() {
    // An ellipse and a line cross the click: the ellipse wins (xfig looks at
    // ellipses first), whatever the depth order.
    let mut e = engine();
    e.execute("shape.polyline", &json!({"points": [[2000, 3000], [4000, 3000]]}))
        .unwrap();
    e.execute("shape.circle", &json!({"cx": 3000, "cy": 3000, "r": 1000}))
        .unwrap();
    e.execute("shape.polyline", &json!({"points": [[4500, 0], [4500, 100]]}))
        .unwrap();
    let pos = Point::new(2000.0, 3000.0);
    let id = e.tangent_target(pos, 5.0).unwrap();
    assert!(matches!(e.doc().get(id).unwrap().kind, ObjectKind::Ellipse(_)));
}
