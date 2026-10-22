#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Chop extended (plan D15): boxes, ellipses, splines and compounds as logs,
//! splines and compounds as axes; Trim, Split at point, Cut all and Extend.

use rustfig_engine::geom::Point;
use rustfig_engine::model::{flatten_object, ObjectId, ObjectKind, PolyKind};
use rustfig_engine::{Button, Engine, SnapMode, ToolEvent, ToolKind};
use serde_json::{json, Value};

fn engine() -> Engine {
    let mut e = Engine::new();
    e.indicator.snap = SnapMode::Any;
    e
}

fn last(e: &Engine) -> ObjectId {
    e.doc().objects.last().unwrap().id
}

fn line(e: &mut Engine, v: &[(f64, f64)]) -> ObjectId {
    let pts: Vec<Value> = v.iter().map(|(x, y)| json!([x, y])).collect();
    e.execute("shape.polyline", &json!({"points": pts})).unwrap();
    last(e)
}

fn spline(e: &mut Engine, v: &[(f64, f64)], closed: bool) -> ObjectId {
    let pts: Vec<Value> = v.iter().map(|(x, y)| json!([x, y])).collect();
    e.execute("shape.spline", &json!({"points": pts, "closed": closed, "kind": "x"}))
        .unwrap();
    last(e)
}

fn ids(r: &Value) -> Vec<ObjectId> {
    r["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| ObjectId(v.as_u64().unwrap()))
        .collect()
}

fn chop(e: &mut Engine, log: ObjectId, axes: &[ObjectId]) -> Value {
    let a: Vec<u64> = axes.iter().map(|a| a.0).collect();
    e.execute("object.chop", &json!({"id": log.0, "axes": a})).unwrap()
}

fn snapshot(e: &Engine) -> Value {
    serde_json::to_value(e.doc()).unwrap()
}

fn kind(e: &Engine, id: ObjectId) -> ObjectKind {
    e.doc().get(id).unwrap().kind.clone()
}

fn flat(e: &Engine, id: ObjectId) -> Vec<Point> {
    flatten_object(e.doc().get(id).unwrap()).remove(0).pts
}

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

#[test]
fn a_box_chops_into_polygons() {
    let mut e = engine();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1000, "height": 1000}),
    )
    .unwrap();
    let log = last(&e);
    let axe = line(&mut e, &[(-500.0, 500.0), (1500.0, 500.0)]);
    let before = snapshot(&e);
    let r = chop(&mut e, log, &[axe]);
    let v = ids(&r);
    assert_eq!(v.len(), 2);
    for id in &v {
        let ObjectKind::Polyline(l) = kind(&e, *id) else {
            panic!()
        };
        assert_eq!(l.kind, PolyKind::Polygon);
        assert!(l.points.len() >= 3);
    }
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
}

#[test]
fn an_arc_box_chops_on_its_rounded_outline() {
    let mut e = engine();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1000, "height": 1000, "radius": 40}),
    )
    .unwrap();
    let log = last(&e);
    let axe = line(&mut e, &[(-500.0, 500.0), (1500.0, 500.0)]);
    let v = ids(&chop(&mut e, log, &[axe]));
    assert_eq!(v.len(), 2);
}

#[test]
fn an_ellipse_chops_into_spline_wedges_whose_ends_are_the_cut_points() {
    let mut e = engine();
    e.execute("shape.ellipse", &json!({"cx": 1000, "cy": 1000, "rx": 800, "ry": 400}))
        .unwrap();
    let log = last(&e);
    let axe = line(&mut e, &[(1000.0, 0.0), (1000.0, 2000.0)]);
    let v = ids(&chop(&mut e, log, &[axe]));
    assert_eq!(v.len(), 2);
    let mut tips = Vec::new();
    for id in &v {
        let ObjectKind::Spline(s) = kind(&e, *id) else {
            panic!("not a spline")
        };
        assert!(s.closed);
        // Wedge: the points include the centre as a sharp corner.
        assert!(s.points.contains(&p(1000.0, 1000.0)));
        tips.push(s.points[0]);
        // The curve stays on the ellipse: every flattened point is close to it.
        for q in flat(&e, *id) {
            let r = ((q.x - 1000.0) / 800.0).powi(2) + ((q.y - 1000.0) / 400.0).powi(2);
            let on_edge = (q.x - 1000.0).abs() < 1.0 || r > 0.97 && r < 1.03;
            let on_spoke = (q.x - 1000.0).abs() < 1.0;
            assert!(on_edge || on_spoke || r < 1.03, "{q:?}");
        }
    }
    for t in tips {
        assert!((t.x - 1000.0).abs() < 1.0 && ((t.y - 600.0).abs() < 1.0 || (t.y - 1400.0).abs() < 1.0));
    }
}

#[test]
fn a_spline_chops_into_splines_that_follow_it() {
    let mut e = engine();
    let log = spline(
        &mut e,
        &[(0.0, 1000.0), (800.0, 0.0), (1600.0, 1000.0), (2400.0, 0.0)],
        false,
    );
    let orig = flat(&e, log);
    let axe = line(&mut e, &[(1000.0, -2000.0), (1000.0, 3000.0)]);
    let before = snapshot(&e);
    let v = ids(&chop(&mut e, log, &[axe]));
    assert!(v.len() >= 2, "{}", v.len());
    for id in &v {
        let ObjectKind::Spline(s) = kind(&e, *id) else { panic!() };
        assert!(!s.closed);
        let (f, l) = (s.points[0], *s.points.last().unwrap());
        // Pieces start or end on the cut line (x = 1000) or at the original ends.
        for end in [f, l] {
            let cut = (end.x - 1000.0).abs() < 1.0;
            let orig_end = end.dist(orig[0]) < 1e-6 || end.dist(*orig.last().unwrap()) < 1e-6;
            assert!(cut || orig_end, "{end:?}");
        }
        // Every flattened piece point lies near the original outline.
        for q in flat(&e, *id) {
            let d = rustfig_engine::geom::dist_to_polyline(q, &orig, false);
            assert!(d < 25.0, "{q:?} is {d} from the original");
        }
    }
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
}

#[test]
fn a_closed_spline_and_a_spline_as_axe() {
    let mut e = engine();
    let log = spline(
        &mut e,
        &[(0.0, 0.0), (1000.0, 0.0), (1000.0, 1000.0), (0.0, 1000.0)],
        true,
    );
    let axe = line(&mut e, &[(-500.0, 500.0), (1500.0, 500.0)]);
    let v = ids(&chop(&mut e, log, &[axe]));
    assert_eq!(v.len(), 2);
    for id in &v {
        let ObjectKind::Spline(s) = kind(&e, *id) else { panic!() };
        assert!(s.closed);
    }
    // A spline as the axe cuts a line.
    let mut e = engine();
    let l = line(&mut e, &[(0.0, 500.0), (3000.0, 500.0)]);
    let sp = spline(&mut e, &[(1000.0, 0.0), (1000.0, 500.0), (1000.0, 1000.0)], false);
    let v = ids(&chop(&mut e, l, &[sp]));
    assert_eq!(v.len(), 2);
}

#[test]
fn arrowheads_stay_only_at_the_original_ends() {
    let mut e = engine();
    e.execute(
        "shape.polyline",
        &json!({"points": [[0, 0], [3000, 0]], "forward_arrow": true, "backward_arrow": true}),
    )
    .unwrap();
    let log = last(&e);
    let a = line(&mut e, &[(1000.0, -500.0), (1000.0, 500.0)]);
    let b = line(&mut e, &[(2000.0, -500.0), (2000.0, 500.0)]);
    let v = ids(&chop(&mut e, log, &[a, b]));
    assert_eq!(v.len(), 3);
    let arrows: Vec<(bool, bool)> = v
        .iter()
        .map(|id| match kind(&e, *id) {
            ObjectKind::Polyline(l) => (l.forward_arrow.is_some(), l.backward_arrow.is_some()),
            _ => panic!(),
        })
        .collect();
    assert_eq!(arrows, vec![(false, true), (false, false), (true, false)]);
}

#[test]
fn a_compound_chops_every_member() {
    let mut e = engine();
    let l1 = line(&mut e, &[(0.0, 0.0), (2000.0, 0.0)]);
    let l2 = line(&mut e, &[(0.0, 400.0), (2000.0, 400.0)]);
    e.set_selection(vec![l1, l2]);
    e.execute("object.group", &json!({})).unwrap();
    let comp = last(&e);
    let axe = line(&mut e, &[(1000.0, -500.0), (1000.0, 900.0)]);
    let before = snapshot(&e);
    let r = chop(&mut e, comp, &[axe]);
    assert_eq!(r["pieces"], 2);
    let ObjectKind::Compound(c) = kind(&e, comp) else {
        panic!()
    };
    assert_eq!(c.objects.len(), 4);
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
    // A compound as axe contributes its members.
    let mut e = engine();
    let log = line(&mut e, &[(0.0, 200.0), (2000.0, 200.0)]);
    let m1 = line(&mut e, &[(500.0, 0.0), (500.0, 400.0)]);
    let m2 = line(&mut e, &[(1500.0, 0.0), (1500.0, 400.0)]);
    e.set_selection(vec![m1, m2]);
    e.execute("object.group", &json!({})).unwrap();
    let comp = last(&e);
    assert_eq!(ids(&chop(&mut e, log, &[comp])).len(), 3);
}

// ---- Trim ------------------------------------------------------------------

#[test]
fn trim_removes_the_clicked_piece() {
    let mut e = engine();
    let log = line(&mut e, &[(0.0, 0.0), (3000.0, 0.0)]);
    let a = line(&mut e, &[(1000.0, -500.0), (1000.0, 500.0)]);
    let b = line(&mut e, &[(2000.0, -500.0), (2000.0, 500.0)]);
    let before = snapshot(&e);
    let r = e
        .execute(
            "object.trim",
            &json!({"id": log.0, "x": 1500, "y": 0, "axes": [a.0, b.0]}),
        )
        .unwrap();
    let v = ids(&r);
    assert_eq!(v.len(), 2);
    let ObjectKind::Polyline(l0) = kind(&e, v[0]) else {
        panic!()
    };
    let ObjectKind::Polyline(l1) = kind(&e, v[1]) else {
        panic!()
    };
    assert_eq!(l0.points, vec![p(0.0, 0.0), p(1000.0, 0.0)]);
    assert_eq!(l1.points, vec![p(2000.0, 0.0), p(3000.0, 0.0)]);
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
}

#[test]
fn trim_defaults_to_every_other_object_and_opens_circles() {
    let mut e = engine();
    e.execute("shape.circle", &json!({"cx": 1000, "cy": 1000, "r": 500}))
        .unwrap();
    let c = last(&e);
    line(&mut e, &[(1000.0, 0.0), (1000.0, 2000.0)]);
    let r = e
        .execute("object.trim", &json!({"id": c.0, "x": 1500, "y": 1000}))
        .unwrap();
    let v = ids(&r);
    assert_eq!(v.len(), 1);
    let ObjectKind::Arc(a) = kind(&e, v[0]) else { panic!() };
    assert!(!a.pie);
    // The kept half is the left one.
    assert!(a.points[1].x < 1000.0);
    // A closed polygon with one intersection is left alone.
    let mut e = engine();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1000, "height": 1000}),
    )
    .unwrap();
    let bx = last(&e);
    line(&mut e, &[(500.0, 500.0), (1500.0, 500.0)]);
    let before = snapshot(&e);
    let r = e
        .execute("object.trim", &json!({"id": bx.0, "x": 0, "y": 500}))
        .unwrap();
    assert!(ids(&r).is_empty());
    assert_eq!(snapshot(&e), before);
}

// ---- Split -----------------------------------------------------------------

#[test]
fn split_at_cuts_where_clicked() {
    let mut e = engine();
    let l = line(&mut e, &[(0.0, 0.0), (1000.0, 0.0), (1000.0, 1000.0)]);
    let before = snapshot(&e);
    let r = e
        .execute("object.split_at", &json!({"id": l.0, "x": 1000, "y": 400}))
        .unwrap();
    let v = ids(&r);
    assert_eq!(v.len(), 2);
    let ObjectKind::Polyline(a) = kind(&e, v[0]) else {
        panic!()
    };
    let ObjectKind::Polyline(b) = kind(&e, v[1]) else {
        panic!()
    };
    assert_eq!(a.points, vec![p(0.0, 0.0), p(1000.0, 0.0), p(1000.0, 400.0)]);
    assert_eq!(b.points, vec![p(1000.0, 400.0), p(1000.0, 1000.0)]);
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
    // At an end point: refused.
    assert!(e
        .execute("object.split_at", &json!({"id": l.0, "x": 0, "y": -50}))
        .is_err());
    assert_eq!(snapshot(&e), before);
}

#[test]
fn split_at_arcs_splines_and_closed_figures() {
    let mut e = engine();
    e.execute("shape.arc", &json!({"points": [[1500, 0], [1000, 500], [500, 0]]}))
        .unwrap();
    let a = last(&e);
    let v = ids(&e
        .execute("object.split_at", &json!({"id": a.0, "x": 1000, "y": 600}))
        .unwrap());
    assert_eq!(v.len(), 2);
    let mut e = engine();
    let s = spline(&mut e, &[(0.0, 1000.0), (800.0, 0.0), (1600.0, 1000.0)], false);
    let v = ids(&e
        .execute("object.split_at", &json!({"id": s.0, "x": 800, "y": 300}))
        .unwrap());
    assert_eq!(v.len(), 2);
    let ObjectKind::Spline(s0) = kind(&e, v[0]) else {
        panic!()
    };
    let ObjectKind::Spline(s1) = kind(&e, v[1]) else {
        panic!()
    };
    assert_eq!(s0.points.last(), s1.points.first());
    // A box opens into one polyline.
    let mut e = engine();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1000, "height": 1000}),
    )
    .unwrap();
    let b = last(&e);
    let v = ids(&e
        .execute("object.split_at", &json!({"id": b.0, "x": 500, "y": 0}))
        .unwrap());
    assert_eq!(v.len(), 1);
    let ObjectKind::Polyline(l) = kind(&e, v[0]) else {
        panic!()
    };
    assert_eq!(l.kind, PolyKind::Polyline);
    assert_eq!(l.points.first(), l.points.last());
    // A circle needs two cut points.
    let mut e = engine();
    e.execute("shape.circle", &json!({"cx": 0, "cy": 0, "r": 500})).unwrap();
    let c = last(&e);
    assert!(e
        .execute("object.split_at", &json!({"id": c.0, "x": 500, "y": 0}))
        .is_err());
}

// ---- Cut all ---------------------------------------------------------------

#[test]
fn cut_all_cuts_every_selected_object_in_one_undo_step() {
    let mut e = engine();
    let a = line(&mut e, &[(0.0, 500.0), (2000.0, 500.0)]);
    let b = line(&mut e, &[(1000.0, 0.0), (1000.0, 1000.0)]);
    e.execute("shape.circle", &json!({"cx": 1000, "cy": 500, "r": 300}))
        .unwrap();
    let c = last(&e);
    let before = snapshot(&e);
    let r = e.execute("object.cut_all", &json!({"ids": [a.0, b.0, c.0]})).unwrap();
    assert_eq!(r["cut"], 3);
    // a: 4 pieces (b at 1000 and the circle at 700 and 1300), b: 4, c: 4 arcs.
    assert_eq!(ids(&r).len(), 12);
    assert_eq!(e.doc().objects.len(), 12);
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
    // Selected objects only: a lone object is an error.
    assert!(e.execute("object.cut_all", &json!({"ids": [a.0]})).is_err());
}

// ---- Extend ----------------------------------------------------------------

#[test]
fn extend_lengthens_a_line_to_the_nearest_boundary() {
    let mut e = engine();
    let l = line(&mut e, &[(0.0, 0.0), (1000.0, 0.0)]);
    let far = line(&mut e, &[(3000.0, -500.0), (3000.0, 500.0)]);
    let near = line(&mut e, &[(2000.0, -500.0), (2000.0, 500.0)]);
    let before = snapshot(&e);
    let r = e
        .execute("object.extend", &json!({"id": l.0, "x": 900, "y": 0}))
        .unwrap();
    assert_eq!(r["x"], 2000.0);
    let ObjectKind::Polyline(pl) = kind(&e, l) else {
        panic!()
    };
    assert_eq!(pl.points, vec![p(0.0, 0.0), p(2000.0, 0.0)]);
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
    // Naming the boundary picks it.
    e.execute(
        "object.extend",
        &json!({"id": l.0, "x": 900, "y": 0, "boundaries": [far.0]}),
    )
    .unwrap();
    let ObjectKind::Polyline(pl) = kind(&e, l) else {
        panic!()
    };
    assert_eq!(pl.points.last(), Some(&p(3000.0, 0.0)));
    let _ = near;
    // The start end extends backwards; with nothing in the way it fails.
    let mut e = engine();
    let l = line(&mut e, &[(1000.0, 0.0), (2000.0, 0.0)]);
    line(&mut e, &[(500.0, -500.0), (500.0, 500.0)]);
    e.execute("object.extend", &json!({"id": l.0, "x": 1000, "y": 0}))
        .unwrap();
    let ObjectKind::Polyline(pl) = kind(&e, l) else {
        panic!()
    };
    assert_eq!(pl.points.first(), Some(&p(500.0, 0.0)));
    let before = snapshot(&e);
    assert!(e
        .execute("object.extend", &json!({"id": l.0, "x": 2000, "y": 0}))
        .is_err());
    assert_eq!(snapshot(&e), before);
}

// ---- the mode --------------------------------------------------------------

fn press(e: &mut Engine, x: f64, y: f64, button: Button, shift: bool) {
    e.tool_event(ToolEvent::Press {
        pos: p(x, y),
        button,
        shift,
    })
    .unwrap();
    e.tool_event(ToolEvent::Release { pos: p(x, y), button }).unwrap();
}

#[test]
fn shift_buttons_trim_split_and_extend_in_chop_mode() {
    let mut e = engine();
    e.set_tool(ToolKind::Chop);
    assert!(e.mouse_hints().shift.contains("trims"));
    let log = line(&mut e, &[(0.0, 0.0), (3000.0, 0.0)]);
    line(&mut e, &[(1000.0, -500.0), (1000.0, 500.0)]);
    line(&mut e, &[(2000.0, -500.0), (2000.0, 500.0)]);
    // Shift+left: trim the middle piece (no axe selected: every other object).
    let before = snapshot(&e);
    press(&mut e, 1500.0, 0.0, Button::Left, true);
    assert_eq!(e.doc().objects.len(), 4);
    assert!(e.doc().get(log).is_none());
    assert!(e.undo());
    assert_eq!(snapshot(&e), before);
    // Shift+middle: split at the point.
    press(&mut e, 500.0, 0.0, Button::Middle, true);
    assert_eq!(e.doc().objects.len(), 4);
    assert!(e.undo());
    // Shift+right: extend the end nearest the click to the next boundary.
    let l2 = line(&mut e, &[(0.0, 1500.0), (500.0, 1500.0)]);
    line(&mut e, &[(800.0, 1000.0), (800.0, 2000.0)]);
    press(&mut e, 450.0, 1500.0, Button::Right, true);
    let ObjectKind::Polyline(pl) = kind(&e, l2) else {
        panic!()
    };
    assert_eq!(pl.points.last(), Some(&p(800.0, 1500.0)));
    // Plain buttons keep xfig's meaning.
    press(&mut e, 1000.0, 500.0, Button::Left, false);
    assert!(e.status().contains("Axe object 1"), "{}", e.status());
}

#[test]
fn a_cut_at_an_end_point_leaves_no_one_point_piece() {
    // Found by command_sweep: after this sequence Cut all made a spline piece
    // whose points all coincide, which the Fig reader refuses.
    let mut e = Engine::new();
    for (id, p) in [
        ("shape.rectangle", json!({"x": 0, "y": 0, "width": 1200, "height": 600})),
        ("shape.circle", json!({"cx": 2000, "cy": 300, "r": 300})),
        (
            "shape.polyline",
            json!({"points": [[0, 900], [600, 1500], [1200, 900]]}),
        ),
        (
            "shape.spline",
            json!({"points": [[0, 2000], [600, 2600], [1200, 2000]], "kind": "x"}),
        ),
        ("shape.arc", json!({"points": [[3000, 0], [3300, 300], [3600, 0]]})),
        ("edit.select_all", json!({})),
        ("point.move", json!({"id": 1, "index": 1, "x": -74.0, "y": 2229.8})),
        (
            "transform.scale",
            json!({"sx": 1.5, "x": 0.0, "y": -428.3, "cx": 0.0, "cy": -428.3}),
        ),
        ("object.cut_all", json!({})),
    ] {
        let _ = e.execute(id, &p);
    }
    for o in &e.doc().objects {
        let pts = flat(&e, o.id);
        assert!(pts.iter().any(|q| q.dist(pts[0]) > 0.5), "one-point piece {:?}", o.kind);
    }
    let data = e.execute("file.save_data", &json!({})).unwrap();
    let mut e2 = Engine::new();
    e2.execute("file.open_data", &json!({"data": data["data"]})).unwrap();
}
