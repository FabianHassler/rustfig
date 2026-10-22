#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Chop (plan D3): cut an object at its intersections with the axe objects.
//! The left button collects the axe objects, the middle button chops a log
//! object with every axe, and the right button clears the axe list. Pieces
//! keep the attributes of the object they came from; one chop is one undo
//! step.

use rustfig_engine::geom::Point;
use rustfig_engine::model::{Arc, ObjectId, ObjectKind, PolyKind, Polyline};
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

fn polygon(e: &mut Engine, v: &[(f64, f64)]) -> ObjectId {
    e.execute("shape.polygon", &json!({"points": pts(v)})).unwrap();
    e.doc().objects.last().unwrap().id
}

fn circle(e: &mut Engine, cx: f64, cy: f64, r: f64) -> ObjectId {
    e.execute("shape.circle", &json!({"cx": cx, "cy": cy, "r": r})).unwrap();
    e.doc().objects.last().unwrap().id
}

fn arc(e: &mut Engine, v: &[(f64, f64)]) -> ObjectId {
    e.execute("shape.arc", &json!({"points": pts(v)})).unwrap();
    e.doc().objects.last().unwrap().id
}

fn poly(e: &Engine, id: ObjectId) -> Polyline {
    match &e.doc().get(id).unwrap().kind {
        ObjectKind::Polyline(p) => p.clone(),
        k => panic!("not a polyline: {k:?}"),
    }
}

fn arcy(e: &Engine, id: ObjectId) -> Arc {
    match &e.doc().get(id).unwrap().kind {
        ObjectKind::Arc(a) => a.clone(),
        k => panic!("not an arc: {k:?}"),
    }
}

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn snapshot(e: &Engine) -> Value {
    serde_json::to_value(e.doc()).unwrap()
}

/// The ids of the pieces a chop returned.
fn chop(e: &mut Engine, log: ObjectId, axes: &[ObjectId]) -> Result<Value, rustfig_engine::EngineError> {
    let axes: Vec<u64> = axes.iter().map(|a| a.0).collect();
    e.execute("object.chop", &json!({"id": log.0, "axes": axes}))
}

fn pieces(r: &Value) -> Vec<ObjectId> {
    r["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| ObjectId(v.as_u64().unwrap()))
        .collect()
}

#[test]
fn chops_a_line_with_a_line() {
    let mut e = engine();
    let log = line(&mut e, &[(0.0, 0.0), (2000.0, 0.0)]);
    let axe = line(&mut e, &[(1000.0, -500.0), (1000.0, 500.0)]);
    let before = snapshot(&e);
    let r = chop(&mut e, log, &[axe]).unwrap();
    let ids = pieces(&r);
    assert_eq!(ids.len(), 2);
    assert_eq!(poly(&e, ids[0]).points, vec![p(0.0, 0.0), p(1000.0, 0.0)]);
    assert_eq!(poly(&e, ids[1]).points, vec![p(1000.0, 0.0), p(2000.0, 0.0)]);
    // The log object is gone, the axe stays; one undo step.
    assert!(e.doc().get(log).is_none());
    assert_eq!(e.doc().objects.len(), 3);
    assert_eq!(e.selection().len(), 2);
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
}

#[test]
fn chops_a_line_with_an_arc_and_an_ellipse() {
    // A semicircular arc (upper half) crosses the line at its two ends.
    let mut e = engine();
    let log = line(&mut e, &[(0.0, 0.0), (2000.0, 0.0)]);
    let axe = arc(&mut e, &[(1500.0, 0.0), (1000.0, 500.0), (500.0, 0.0)]);
    let r = chop(&mut e, log, &[axe]).unwrap();
    let ids = pieces(&r);
    assert_eq!(ids.len(), 3);
    assert_eq!(poly(&e, ids[0]).points, vec![p(0.0, 0.0), p(500.0, 0.0)]);
    assert_eq!(poly(&e, ids[1]).points, vec![p(500.0, 0.0), p(1500.0, 0.0)]);
    assert_eq!(poly(&e, ids[2]).points, vec![p(1500.0, 0.0), p(2000.0, 0.0)]);

    // A circle crossing the line twice does the same.
    let mut e = engine();
    let log = line(&mut e, &[(0.0, 0.0), (2000.0, 0.0)]);
    let axe = circle(&mut e, 1000.0, 0.0, 500.0);
    let r = chop(&mut e, log, &[axe]).unwrap();
    let ids = pieces(&r);
    assert_eq!(ids.len(), 3);
    assert_eq!(poly(&e, ids[1]).points, vec![p(500.0, 0.0), p(1500.0, 0.0)]);
}

#[test]
fn chops_an_arc_with_a_line_and_a_circle_with_a_line() {
    // A semicircle chopped by a vertical line through x=750.
    let mut e = engine();
    let log = arc(&mut e, &[(1500.0, 0.0), (1000.0, 500.0), (500.0, 0.0)]);
    let axe = line(&mut e, &[(750.0, -1000.0), (750.0, 1000.0)]);
    let before = snapshot(&e);
    let r = chop(&mut e, log, &[axe]).unwrap();
    let ids = pieces(&r);
    assert_eq!(ids.len(), 2);
    for id in &ids {
        let a = arcy(&e, *id);
        assert!((a.center.dist(p(1000.0, 0.0))).abs() < 1e-6);
        assert!((a.center.dist(a.points[0]) - 500.0).abs() < 1.0);
        assert!(!a.pie);
    }
    assert!(e.doc().get(log).is_none());
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);

    // A circle chopped by a diameter: two pie wedges.
    let mut e = engine();
    let log = circle(&mut e, 1000.0, 1000.0, 500.0);
    let axe = line(&mut e, &[(1000.0, 0.0), (1000.0, 2000.0)]);
    let r = chop(&mut e, log, &[axe]).unwrap();
    let ids = pieces(&r);
    assert_eq!(ids.len(), 2);
    for id in &ids {
        let a = arcy(&e, *id);
        assert!(a.pie, "circle pieces are pie wedges");
        assert!((a.center.dist(a.points[0]) - 500.0).abs() < 1.0);
    }
}

#[test]
fn a_polygon_needs_two_intersections() {
    let mut e = engine();
    let log = polygon(&mut e, &[(0.0, 0.0), (1000.0, 0.0), (1000.0, 1000.0), (0.0, 1000.0)]);
    let axe = line(&mut e, &[(500.0, 500.0), (1500.0, 500.0)]);
    let before = snapshot(&e);
    let err = chop(&mut e, log, &[axe]).unwrap_err();
    assert!(err.message.contains("two or more"), "{}", err.message);
    assert_eq!(snapshot(&e), before);

    // A line crossing the polygon twice splits it.
    let mut e = engine();
    let log = polygon(&mut e, &[(0.0, 0.0), (1000.0, 0.0), (1000.0, 1000.0), (0.0, 1000.0)]);
    let axe = line(&mut e, &[(-500.0, 500.0), (1500.0, 500.0)]);
    let r = chop(&mut e, log, &[axe]).unwrap();
    let ids = pieces(&r);
    assert_eq!(ids.len(), 2);
    for id in &ids {
        assert_eq!(poly(&e, *id).kind, PolyKind::Polygon);
    }
}

#[test]
fn chop_rejects_bad_log_and_axe_kinds() {
    let mut e = engine();
    let l = line(&mut e, &[(0.0, 0.0), (1000.0, 0.0)]);
    // A text is neither a valid log nor a valid axe (splines, ellipses and
    // compounds are, since D15: see chop_extended.rs).
    e.execute("shape.text", &json!({"x": 0, "y": 3000, "text": "hi"}))
        .unwrap();
    let tx = e.doc().objects.last().unwrap().id;
    let before = snapshot(&e);
    assert!(chop(&mut e, tx, &[l]).is_err());
    assert!(chop(&mut e, l, &[tx]).is_err());
    // Unknown ids.
    assert!(chop(&mut e, ObjectId(99999), &[l]).is_err());
    assert!(chop(&mut e, l, &[ObjectId(99999)]).is_err());
    assert_eq!(snapshot(&e), before);
}

#[test]
fn a_chop_that_does_not_intersect_changes_nothing() {
    let mut e = engine();
    let log = line(&mut e, &[(0.0, 0.0), (1000.0, 0.0)]);
    let axe = line(&mut e, &[(0.0, 500.0), (1000.0, 500.0)]);
    let before = snapshot(&e);
    let r = chop(&mut e, log, &[axe]).unwrap();
    assert_eq!(pieces(&r).len(), 0);
    assert_eq!(snapshot(&e), before);
}

#[test]
fn the_log_cannot_chop_itself() {
    let mut e = engine();
    let log = line(&mut e, &[(0.0, 0.0), (1000.0, 0.0)]);
    let before = snapshot(&e);
    let err = chop(&mut e, log, &[log]).unwrap_err();
    assert!(err.message.contains("itself"), "{}", err.message);
    assert_eq!(snapshot(&e), before);
}

#[test]
fn a_missing_axe_list_is_an_error() {
    let mut e = engine();
    let log = line(&mut e, &[(0.0, 0.0), (1000.0, 0.0)]);
    assert!(e.execute("object.chop", &json!({"id": log.0})).is_err());
    assert!(e.execute("object.chop", &json!({"id": log.0, "axes": []})).is_err());
    assert!(e.execute("object.chop", &json!({"axes": [log.0]})).is_err());
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
fn the_mode_collects_axes_chops_a_log_and_clears() {
    let mut e = engine();
    line(&mut e, &[(0.0, 0.0), (2000.0, 0.0)]);
    line(&mut e, &[(1000.0, -500.0), (1000.0, 500.0)]);
    e.set_tool(ToolKind::Chop);
    assert_eq!(hints(&e), ["Select axe object", "Select log object", "Clear axe list"]);
    // Markers on every line, arc and ellipse.
    assert_eq!(e.overlay().markers.len(), 4);
    // Left click on the axe (away from the log) selects it.
    press(&mut e, 1000.0, 500.0, Button::Left);
    assert!(e.status().contains("Axe object 1"), "{}", e.status());
    // The axe is highlighted in the axe colour; with the pointer on the
    // other line, that one is highlighted as the log.
    assert_eq!(e.overlay().chop_axes.len(), 1);
    assert_eq!(e.overlay().markers.len(), 4, "the markers stay while picking");
    assert!(e.overlay().chop_log.is_none());
    e.tool_event(ToolEvent::Move { pos: p(1500.0, 0.0) }).unwrap();
    let log = e.overlay().chop_log.unwrap();
    assert_eq!(log.markers, vec![p(0.0, 0.0), p(2000.0, 0.0)]);
    e.tool_event(ToolEvent::Move { pos: p(1000.0, 400.0) }).unwrap();
    assert!(e.overlay().chop_log.is_none(), "an axe is not its own log");
    // Clicking the same axe again removes it.
    press(&mut e, 1000.0, 500.0, Button::Left);
    assert!(e.status().contains("removed"), "{}", e.status());
    assert!(e.overlay().chop_axes.is_empty());
    // Re-select, then middle-click the log to chop it.
    press(&mut e, 1000.0, 500.0, Button::Left);
    let before = snapshot(&e);
    press(&mut e, 500.0, 0.0, Button::Middle);
    assert_eq!(e.doc().objects.len(), 3);
    assert!(e.status().contains("chopped into 2"), "{}", e.status());
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
    // Right click clears the axe list.
    press(&mut e, 1000.0, 500.0, Button::Left);
    press(&mut e, 1000.0, 500.0, Button::Right);
    assert!(e.status().contains("cleared"), "{}", e.status());
    assert!(e.overlay().guides.is_empty());
}

#[test]
fn the_mode_requires_an_axe_before_chopping() {
    let mut e = engine();
    line(&mut e, &[(0.0, 0.0), (2000.0, 0.0)]);
    e.set_tool(ToolKind::Chop);
    let before = snapshot(&e);
    press(&mut e, 500.0, 0.0, Button::Middle);
    assert_eq!(snapshot(&e), before);
    assert!(e.status().contains("axe"), "{}", e.status());
}
