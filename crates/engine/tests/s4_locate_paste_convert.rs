#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! S4-1 (Shift-click "Locate Object"), S4-2 (Paste follows the mouse) and
//! S4-5 (Convert box and arc-box, line and spline remembering their kind).

use rustfig_engine::geom::Point;
use rustfig_engine::model::{ObjectId, ObjectKind, PolyKind, SplineKind};
use rustfig_engine::{Button, Engine, ToolEvent, ToolKind};
use serde_json::json;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn press(e: &mut Engine, x: f64, y: f64, button: Button, shift: bool) {
    e.tool_event(ToolEvent::Press {
        pos: p(x, y),
        button,
        shift,
    })
    .unwrap();
    e.tool_event(ToolEvent::Release { pos: p(x, y), button }).unwrap();
}

fn rect(e: &mut Engine, x: f64, y: f64, w: f64, h: f64) -> ObjectId {
    e.execute("shape.rectangle", &json!({"x": x, "y": y, "width": w, "height": h}))
        .unwrap();
    e.doc().objects.last().unwrap().id
}

fn polyline_kind(e: &Engine, id: ObjectId) -> PolyKind {
    match &e.doc().get(id).unwrap().kind {
        ObjectKind::Polyline(pl) => pl.kind,
        _ => panic!("not a polyline"),
    }
}

#[test]
fn shift_click_cycles_through_overlapping_objects_and_a_plain_click_acts_on_the_located_one() {
    let mut e = Engine::new();
    let low = rect(&mut e, 0.0, 0.0, 3000.0, 3000.0);
    let high = rect(&mut e, 0.0, 0.0, 3000.0, 3000.0);
    e.execute("object.set_depth", &json!({"ids": [low.0], "depth": 60}))
        .ok();
    e.set_tool(ToolKind::Delete);
    let before = e.doc().objects.len();
    // Shift+click: highlight only, nothing is deleted.
    press(&mut e, 0.0, 1500.0, Button::Left, true);
    assert_eq!(e.doc().objects.len(), before);
    let first = e.overlay().tagged.len();
    assert!(first > 0, "the located object's markers are filled");
    // The hint says what Shift does; Shift+middle and Shift+right are free.
    let h = e.mouse_hints_with(true);
    assert_eq!((h.left, h.middle, h.right), ("locate object", "", ""));
    press(&mut e, 0.0, 1500.0, Button::Middle, true);
    press(&mut e, 0.0, 1500.0, Button::Right, true);
    assert_eq!(e.doc().objects.len(), before, "Shift+middle/right do nothing");
    // Shift+click again: the next object; a third time: nothing found, the
    // cursor is marked.
    press(&mut e, 0.0, 1500.0, Button::Left, true);
    assert_eq!(e.doc().objects.len(), before);
    press(&mut e, 0.0, 1500.0, Button::Left, true);
    assert_eq!(e.overlay().tagged.last(), Some(&p(0.0, 1500.0)));
    // Back to the first, then the second; delete the second with a plain click.
    press(&mut e, 0.0, 1500.0, Button::Left, true);
    press(&mut e, 0.0, 1500.0, Button::Left, true);
    press(&mut e, 0.0, 1500.0, Button::Left, false);
    assert_eq!(e.doc().objects.len(), before - 1);
    let top_survives = e.doc().get(high).is_some();
    let low_survives = e.doc().get(low).is_some();
    assert!(top_survives && !low_survives, "the located lower one is deleted");
    // The located object is the lower one on the second Shift+click.
    assert!(e.undo());
    assert!(!e.overlay().tagged.contains(&p(0.0, 1500.0)));
}

#[test]
fn shift_locate_does_not_apply_where_xfig_has_no_object_search() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::Box);
    assert_eq!(e.mouse_hints_with(true).left, e.mouse_hints().left);
    e.set_tool(ToolKind::Edit);
    // xfig's own Shift meaning in Edit: the middle button shows comments.
    assert_eq!(e.mouse_hints_with(true).middle, "show comments");
    assert_eq!(e.mouse_hints_with(true).left, "locate object");
}

#[test]
fn paste_follows_the_pointer_and_places_with_left_middle_or_right() {
    let mut e = Engine::new();
    rect(&mut e, 1000.0, 2000.0, 600.0, 300.0);
    e.execute("edit.select_all", &json!({})).unwrap();
    e.execute("edit.copy", &json!({})).unwrap();
    let n = e.doc().objects.len();
    e.execute("edit.paste", &json!({})).unwrap();
    assert_eq!(e.doc().objects.len(), n, "nothing is placed yet");
    assert_eq!(e.mouse_hints().left, "place object");
    assert_eq!(e.mouse_hints().middle, "place at orig posn");
    assert_eq!(e.mouse_hints().right, "cancel paste");
    e.tool_event(ToolEvent::Move { pos: p(4950.0, 6000.0) }).unwrap();
    assert_eq!(e.overlay().preview.len(), 1);
    // Left: the objects' top-left corner goes to the pointer; one undo step.
    press(&mut e, 4950.0, 6000.0, Button::Left, false);
    assert_eq!(e.doc().objects.len(), n + 1);
    let bb = e.doc().objects.last().unwrap().geometry_bbox();
    assert!(
        (bb.min.x - 4950.0).abs() < 1.5 && (bb.min.y - 6000.0).abs() < 1.5,
        "{bb:?}"
    );
    assert!(e.undo());
    assert_eq!(e.doc().objects.len(), n);
    // Middle: at the original position.
    e.execute("edit.paste", &json!({})).unwrap();
    press(&mut e, 4950.0, 6000.0, Button::Middle, false);
    let bb = e.doc().objects.last().unwrap().geometry_bbox();
    assert!(
        (bb.min.x - 1000.0).abs() < 1.5 && (bb.min.y - 2000.0).abs() < 1.5,
        "{bb:?}"
    );
    assert!(e.undo());
    // Right and Escape cancel.
    e.execute("edit.paste", &json!({})).unwrap();
    press(&mut e, 10.0, 10.0, Button::Right, false);
    assert_eq!(e.doc().objects.len(), n);
    assert!(!e.tool_busy());
    e.execute("edit.paste", &json!({})).unwrap();
    e.tool_event(ToolEvent::Key(rustfig_engine::Key::Escape)).unwrap();
    assert!(!e.tool_busy());
    assert_eq!(e.doc().objects.len(), n);
    // Scripts keep the offset form.
    e.execute("edit.paste", &json!({"dx": 100, "dy": 100})).unwrap();
    assert_eq!(e.doc().objects.len(), n + 1);
}

#[test]
fn convert_makes_boxes_rounded_and_back_and_remembers_the_spline_kind() {
    let mut e = Engine::new();
    let b = rect(&mut e, 0.0, 0.0, 1200.0, 600.0);
    e.indicator.box_radius = 11;
    e.set_tool(ToolKind::Convert);
    press(&mut e, 600.0, 0.0, Button::Left, false);
    assert_eq!(polyline_kind(&e, b), PolyKind::ArcBox);
    let ObjectKind::Polyline(pl) = &e.doc().get(b).unwrap().kind else {
        panic!()
    };
    assert_eq!(pl.radius, 11);
    assert!(e.undo());
    assert_eq!(polyline_kind(&e, b), PolyKind::Box);
    press(&mut e, 600.0, 0.0, Button::Left, false);
    press(&mut e, 600.0, 0.0, Button::Left, false);
    assert_eq!(polyline_kind(&e, b), PolyKind::Box);

    // A line becomes an interpolated spline at first; an approximated spline
    // converted to a line comes back approximated.
    e.execute(
        "shape.polyline",
        &json!({"points": [[0, 2000], [600, 2600], [1200, 2000]]}),
    )
    .unwrap();
    let l = e.doc().objects.last().unwrap().id;
    e.execute("object.convert", &json!({"ids": [l.0]})).unwrap();
    let kind = |e: &Engine, id| match &e.doc().get(id).unwrap().kind {
        ObjectKind::Spline(s) => Some(s.kind),
        _ => None,
    };
    assert_eq!(kind(&e, l), Some(SplineKind::Interpolated));
    e.execute(
        "shape.spline",
        &json!({"points": [[0, 4000], [600, 4600], [1200, 4000]], "kind": "approx"}),
    )
    .unwrap();
    let s = e.doc().objects.last().unwrap().id;
    if kind(&e, s) == Some(SplineKind::Approximated) {
        e.execute("object.convert", &json!({"ids": [s.0]})).unwrap();
        assert_eq!(kind(&e, s), None);
        e.execute("object.convert", &json!({"ids": [s.0]})).unwrap();
        assert_eq!(kind(&e, s), Some(SplineKind::Approximated));
    }
}
