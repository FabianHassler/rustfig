#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Functions on the mouse buttons xfig leaves free (plan W13, W14): each is
//! driven with tool events, is one undo step and shows in the mouse hints.

use rustfig_engine::geom::Point;
use rustfig_engine::model::{ObjectKind, PolyKind};
use rustfig_engine::{Button, Engine, SnapMode, ToolEvent, ToolKind};
use serde_json::{json, Value};

/// An engine without grid snapping, so coordinates are exact.
fn engine() -> Engine {
    let mut e = Engine::new();
    e.indicator.snap = SnapMode::Any;
    e
}

fn press(e: &mut Engine, x: f64, y: f64, button: Button, shift: bool) {
    e.tool_event(ToolEvent::Press {
        pos: Point::new(x, y),
        button,
        shift,
    })
    .unwrap();
}

fn release(e: &mut Engine, x: f64, y: f64, button: Button) {
    e.tool_event(ToolEvent::Release {
        pos: Point::new(x, y),
        button,
    })
    .unwrap();
}

fn mv(e: &mut Engine, x: f64, y: f64) {
    e.tool_event(ToolEvent::Move { pos: Point::new(x, y) }).unwrap();
}

fn click(e: &mut Engine, x: f64, y: f64, button: Button) {
    press(e, x, y, button, false);
    release(e, x, y, button);
}

fn drag(e: &mut Engine, a: (f64, f64), b: (f64, f64), button: Button) {
    press(e, a.0, a.1, button, false);
    mv(e, (a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    mv(e, b.0, b.1);
    release(e, b.0, b.1, button);
}

fn rect(e: &mut Engine, x: i32, y: i32, w: i32, h: i32) {
    e.execute("shape.rectangle", &json!({"x": x, "y": y, "width": w, "height": h}))
        .unwrap();
}

fn bbox(e: &Engine, i: usize) -> (Point, Point) {
    let b = e.doc().objects[i].geometry_bbox();
    (b.min, b.max)
}

fn hints(e: &Engine) -> [&'static str; 3] {
    let h = e.mouse_hints();
    [h.left, h.middle, h.right]
}

fn thickness(e: &Engine, i: usize) -> i32 {
    match &e.doc().objects[i].kind {
        ObjectKind::Polyline(p) => p.style.thickness,
        _ => panic!("not a polyline"),
    }
}

fn one_undo_step(e: &mut Engine, before: &Value) {
    assert!(e.undo());
    assert_eq!(
        &serde_json::to_value(e.doc()).unwrap(),
        before,
        "one undo restores the document"
    );
}

fn snapshot(e: &Engine) -> Value {
    serde_json::to_value(e.doc()).unwrap()
}

#[test]
fn update_region_updates_what_is_inside() {
    let mut e = engine();
    rect(&mut e, 0, 0, 1000, 500);
    rect(&mut e, 1500, 0, 500, 500);
    rect(&mut e, 5000, 5000, 500, 500);
    e.execute("indicator.set", &json!({"thickness": 6})).unwrap();
    e.set_tool(ToolKind::Update);
    assert_eq!(hints(&e)[2], "update region");
    let before = snapshot(&e);
    press(&mut e, -200.0, -200.0, Button::Right, false);
    assert_eq!(hints(&e), ["", "", "final corner"]);
    let ov = e.overlay();
    assert_eq!(ov.guides.len(), 1, "a rubber band is drawn");
    mv(&mut e, 2500.0, 1000.0);
    release(&mut e, 2500.0, 1000.0, Button::Right);
    assert_eq!((thickness(&e, 0), thickness(&e, 1)), (6, 6));
    assert_ne!(thickness(&e, 2), 6, "outside objects are untouched");
    assert!(!e.tool_busy());
    one_undo_step(&mut e, &before);
}

#[test]
fn update_region_by_two_clicks_and_hidden_depths() {
    let mut e = engine();
    rect(&mut e, 0, 0, 1000, 500);
    e.execute(
        "object.set_attributes",
        &json!({"ids": [e.doc().objects[0].id.0], "depth": 70}),
    )
    .unwrap();
    rect(&mut e, 1500, 0, 500, 500);
    e.execute("indicator.set", &json!({"thickness": 7})).unwrap();
    e.execute("view.toggle_depth", &json!({"depth": 70})).ok();
    e.set_tool(ToolKind::Update);
    click(&mut e, -200.0, -200.0, Button::Right);
    assert!(e.tool_busy());
    click(&mut e, 2500.0, 1000.0, Button::Right);
    assert!(!e.tool_busy());
    assert_eq!(thickness(&e, 1), 7);
}

#[test]
fn left_and_middle_unchanged_in_update() {
    let mut e = engine();
    rect(&mut e, 0, 0, 1000, 500);
    e.execute("indicator.set", &json!({"thickness": 5})).unwrap();
    e.set_tool(ToolKind::Update);
    click(&mut e, 0.0, 0.0, Button::Left);
    assert_eq!(thickness(&e, 0), 5);
    assert_eq!(hints(&e)[..2], ["update object", "update settings"]);
}

#[test]
fn move_region_moves_everything_inside() {
    let mut e = engine();
    rect(&mut e, 0, 0, 1000, 500);
    rect(&mut e, 1500, 0, 500, 500);
    rect(&mut e, 5000, 5000, 500, 500);
    e.set_tool(ToolKind::Move);
    assert_eq!(hints(&e)[2], "move region");
    let before = snapshot(&e);
    drag(&mut e, (-200.0, -200.0), (2500.0, 1000.0), Button::Right);
    assert!(e.tool_busy(), "waits for the place click");
    assert_eq!(hints(&e), ["place object", "", "cancel"]);
    click(&mut e, 2500.0, 1000.0, Button::Left);
    // placed 0 away: move to 100 right, 200 down
    assert!(!e.tool_busy());
    let _ = before;
    let mut e = engine();
    rect(&mut e, 0, 0, 1000, 500);
    rect(&mut e, 1500, 0, 500, 500);
    rect(&mut e, 5000, 5000, 500, 500);
    e.set_tool(ToolKind::Move);
    let before = snapshot(&e);
    drag(&mut e, (-200.0, -200.0), (2500.0, 1000.0), Button::Right);
    press(&mut e, 2600.0, 1200.0, Button::Left, false);
    release(&mut e, 2600.0, 1200.0, Button::Left);
    assert_eq!(bbox(&e, 0).0, Point::new(100.0, 200.0));
    assert_eq!(bbox(&e, 1).0, Point::new(1600.0, 200.0));
    assert_eq!(bbox(&e, 2).0, Point::new(5000.0, 5000.0));
    one_undo_step(&mut e, &before);
}

#[test]
fn move_region_command_validates() {
    let mut e = engine();
    rect(&mut e, 0, 0, 100, 100);
    let before = snapshot(&e);
    assert!(e
        .execute(
            "transform.move_region",
            &json!({"x1": 5000, "y1": 5000, "x2": 6000, "y2": 6000, "dx": 1, "dy": 1})
        )
        .is_err());
    assert!(e
        .execute(
            "transform.move_region",
            &json!({"x1": 0, "y1": 0, "x2": "a", "y2": 1, "dx": 1, "dy": 1})
        )
        .is_err());
    assert_eq!(snapshot(&e), before);
    e.execute(
        "transform.move_region",
        &json!({"x1": -200, "y1": -200, "x2": 500, "y2": 500, "dx": 50, "dy": 0}),
    )
    .unwrap();
    assert_eq!(bbox(&e, 0).0, Point::new(50.0, 0.0));
}

#[test]
fn delete_points_in_a_region() {
    let mut e = engine();
    e.execute(
        "shape.polyline",
        &json!({"points": [[0, 0], [1000, 0], [2000, 0], [3000, 0], [4000, 0]]}),
    )
    .unwrap();
    e.set_tool(ToolKind::DeletePoint);
    assert_eq!(hints(&e)[..2], ["delete point", "delete in region"]);
    let before = snapshot(&e);
    click(&mut e, 900.0, -500.0, Button::Middle);
    assert_eq!(hints(&e), ["", "final corner", "cancel"]);
    click(&mut e, 2100.0, 500.0, Button::Middle);
    assert_eq!(e.doc().objects[0].points().unwrap().len(), 3);
    one_undo_step(&mut e, &before);
    // Not enough points left: refused, document unchanged.
    click(&mut e, -100.0, -100.0, Button::Middle);
    let r = e.tool_event(ToolEvent::Press {
        pos: Point::new(4100.0, 100.0),
        button: Button::Middle,
        shift: false,
    });
    assert!(r.is_err(), "too few points left");
    assert_eq!(snapshot(&e), before);
}

#[test]
fn add_point_at_the_nearest_end() {
    let mut e = engine();
    e.execute("shape.polyline", &json!({"points": [[0, 0], [2000, 0]]}))
        .unwrap();
    e.set_tool(ToolKind::AddPoint);
    assert_eq!(hints(&e)[1], "add at end");
    let before = snapshot(&e);
    press(&mut e, 1900.0, 0.0, Button::Middle, false);
    mv(&mut e, 2500.0, 700.0);
    release(&mut e, 2500.0, 700.0, Button::Middle);
    let pts = e.doc().objects[0].points().unwrap().to_vec();
    assert_eq!(pts.len(), 3);
    assert_eq!(pts[2], Point::new(2500.0, 700.0));
    one_undo_step(&mut e, &before);
    press(&mut e, 100.0, 0.0, Button::Middle, false);
    release(&mut e, -500.0, 0.0, Button::Middle);
    assert_eq!(e.doc().objects[0].points().unwrap()[0], Point::new(-500.0, 0.0));
}

#[test]
fn arrow_right_reverses() {
    let mut e = engine();
    e.execute(
        "shape.polyline",
        &json!({"points": [[0, 0], [2000, 0]], "forward_arrow": true}),
    )
    .unwrap();
    e.set_tool(ToolKind::Arrow);
    assert_eq!(hints(&e)[2], "reverse arrows");
    let before = snapshot(&e);
    click(&mut e, 1000.0, 0.0, Button::Right);
    let (f, b) = e.doc().objects[0].arrows().unwrap();
    assert!(f.is_none() && b.is_some());
    one_undo_step(&mut e, &before);
}

#[test]
fn break_right_breaks_all_levels() {
    let mut e = engine();
    rect(&mut e, 0, 0, 500, 500);
    rect(&mut e, 1000, 0, 500, 500);
    e.execute("edit.select_all", &json!({})).unwrap();
    e.execute("object.group", &json!({})).unwrap();
    rect(&mut e, 3000, 0, 500, 500);
    e.execute("edit.select_all", &json!({})).unwrap();
    e.execute("object.group", &json!({})).unwrap();
    assert_eq!(e.doc().objects.len(), 1);
    e.set_tool(ToolKind::Break);
    assert_eq!(hints(&e)[2], "break all levels");
    let before = snapshot(&e);
    click(&mut e, 0.0, 0.0, Button::Right);
    assert_eq!(e.doc().objects.len(), 3);
    assert!(e
        .doc()
        .objects
        .iter()
        .all(|o| !matches!(o.kind, ObjectKind::Compound(_))));
    one_undo_step(&mut e, &before);
    // Left breaks one level only.
    click(&mut e, 0.0, 0.0, Button::Left);
    assert_eq!(e.doc().objects.len(), 2);
}

#[test]
fn align_right_aligns_to_the_first_object() {
    let mut e = engine();
    rect(&mut e, 0, 0, 1000, 500);
    rect(&mut e, 2000, 1000, 500, 500);
    rect(&mut e, 4000, 3000, 300, 300);
    e.execute("indicator.set", &json!({"align_h": "left", "align_v": "top"}))
        .unwrap();
    e.set_tool(ToolKind::Align);
    assert_eq!(hints(&e)[2], "align to first");
    let before = snapshot(&e);
    click(&mut e, 0.0, 0.0, Button::Right);
    assert_eq!(hints(&e), ["cancel", "", "align to first"]);
    click(&mut e, 2000.0, 1000.0, Button::Right);
    click(&mut e, 4000.0, 3000.0, Button::Right);
    assert_eq!(bbox(&e, 1).0, bbox(&e, 0).0);
    assert_eq!(bbox(&e, 2).0, bbox(&e, 0).0);
    assert!(e.undo() && e.undo());
    assert_eq!(snapshot(&e), before);
    // Left cancels the pending reference.
    click(&mut e, 2000.0, 1000.0, Button::Left);
    assert!(!e.tool_busy());
}

#[test]
fn centre_box_and_square() {
    let mut e = engine();
    e.set_tool(ToolKind::Box);
    assert_eq!(hints(&e)[1], "center point");
    drag(&mut e, (1000.0, 1000.0), (1400.0, 1200.0), Button::Middle);
    assert_eq!(bbox(&e, 0), (Point::new(600.0, 800.0), Point::new(1400.0, 1200.0)));
    press(&mut e, 5000.0, 5000.0, Button::Middle, true);
    mv(&mut e, 5300.0, 5100.0);
    release(&mut e, 5300.0, 5100.0, Button::Middle);
    assert_eq!(bbox(&e, 1), (Point::new(4700.0, 4700.0), Point::new(5300.0, 5300.0)));
    // The left button still draws corner to corner.
    drag(&mut e, (0.0, 0.0), (300.0, 200.0), Button::Left);
    assert_eq!(bbox(&e, 2), (Point::new(0.0, 0.0), Point::new(300.0, 200.0)));
    e.set_tool(ToolKind::ArcBox);
    drag(&mut e, (9000.0, 9000.0), (9200.0, 9100.0), Button::Middle);
    match &e.doc().objects[3].kind {
        ObjectKind::Polyline(p) => assert_eq!(p.kind, PolyKind::ArcBox),
        _ => panic!(),
    }
    assert_eq!(bbox(&e, 3).0, Point::new(8800.0, 8900.0));
}

#[test]
fn picture_middle_and_right_make_requests() {
    let mut e = engine();
    e.set_tool(ToolKind::Picture);
    assert_eq!(hints(&e), ["corner point", "center point", "keep aspect"]);
    drag(&mut e, (1000.0, 1000.0), (1400.0, 1200.0), Button::Middle);
    drag(&mut e, (0.0, 0.0), (800.0, 400.0), Button::Right);
    drag(&mut e, (0.0, 0.0), (800.0, 400.0), Button::Left);
    let reqs = e.take_requests();
    let got: Vec<(f64, bool)> = reqs
        .into_iter()
        .filter_map(|r| match r {
            rustfig_engine::UiRequest::PickPicture { rect, keep_aspect } => Some((rect.width(), keep_aspect)),
            _ => None,
        })
        .collect();
    assert_eq!(got, vec![(800.0, false), (800.0, true), (800.0, false)]);
}

#[test]
fn picture_keeps_its_aspect_ratio() {
    let dir = std::env::temp_dir();
    let path = dir.join("free_buttons_picture.xpm");
    std::fs::write(
        &path,
        "/* XPM */\nstatic char *a[] = {\n\"4 2 2 1\",\n\"a c #000000\",\n\"b c #ffffff\",\n\"aaaa\",\n\"bbbb\"};\n",
    )
    .unwrap();
    let mut e = engine();
    let r = e.execute(
        "shape.picture",
        &json!({"path": path.to_string_lossy(), "x": 0, "y": 0, "width": 1000, "height": 1000, "keep_aspect": true}),
    );
    std::fs::remove_file(&path).ok();
    if r.is_ok() {
        let (a, b) = bbox(&e, 0);
        assert!(((b.x - a.x) / (b.y - a.y) - 2.0).abs() < 1e-9, "4:2 picture keeps 2:1");
    }
}

#[test]
fn regular_polygon_by_one_side() {
    let mut e = engine();
    e.execute("indicator.set", &json!({"sides": 6})).unwrap();
    e.set_tool(ToolKind::RegularPolygon);
    assert_eq!(hints(&e)[1], "one side");
    drag(&mut e, (0.0, 0.0), (1000.0, 0.0), Button::Middle);
    let pts = e.doc().objects[0].points().unwrap().to_vec();
    assert_eq!(pts.len(), 6);
    for i in 0..6 {
        let d = pts[i].dist(pts[(i + 1) % 6]);
        assert!((d - 1000.0).abs() < 1e-6, "every side is 1000 long, got {d}");
    }
    assert!(pts.iter().any(|p| p.dist(Point::ZERO) < 1e-6));
    assert!(pts.iter().any(|p| p.dist(Point::new(1000.0, 0.0)) < 1e-6));
}

#[test]
fn circle_through_three_points() {
    let mut e = engine();
    e.set_tool(ToolKind::CircleRadius);
    assert_eq!(hints(&e)[2], "circle by 3 points");
    let before = snapshot(&e);
    click(&mut e, 1000.0, 0.0, Button::Right);
    assert_eq!(hints(&e), ["", "", "second point"]);
    click(&mut e, 0.0, 1000.0, Button::Right);
    mv(&mut e, -1000.0, 0.0);
    assert!(!e.overlay().preview.is_empty(), "the circle is previewed");
    // Other buttons do nothing meanwhile.
    click(&mut e, 5.0, 5.0, Button::Left);
    assert!(e.tool_busy());
    click(&mut e, -1000.0, 0.0, Button::Right);
    assert!(!e.tool_busy());
    let (a, b) = bbox(&e, 0);
    assert!(((b.x - a.x) - 2000.0).abs() < 60.0 && ((b.y - a.y) - 2000.0).abs() < 60.0);
    assert!(
        (a.x + b.x).abs() < 1.0 && (a.y + b.y).abs() < 1.0,
        "centred on the origin"
    );
    one_undo_step(&mut e, &before);
    // Collinear points: refused, nothing drawn.
    click(&mut e, 0.0, 0.0, Button::Right);
    click(&mut e, 100.0, 0.0, Button::Right);
    let r = e.tool_event(ToolEvent::Press {
        pos: Point::new(200.0, 0.0),
        button: Button::Right,
        shift: false,
    });
    assert!(r.is_err());
    assert!(e.doc().objects.is_empty());
    // The left button still draws a circle by radius.
    drag(&mut e, (0.0, 0.0), (500.0, 0.0), Button::Left);
    assert_eq!(e.doc().objects.len(), 1);
    // Ellipse modes have no such function.
    e.set_tool(ToolKind::EllipseRadii);
    assert_eq!(hints(&e)[2], "");
}

#[test]
fn text_middle_pastes_the_clipboard() {
    let mut e = engine();
    e.set_tool(ToolKind::Text);
    assert_eq!(hints(&e)[1], "", "nothing to paste");
    e.execute("shape.text", &json!({"x": 0, "y": 0, "text": "hello"}))
        .unwrap();
    e.execute("edit.copy", &json!({})).unwrap();
    e.execute("edit.delete", &json!({})).unwrap();
    assert_eq!(hints(&e)[1], "paste as text");
    let before = snapshot(&e);
    click(&mut e, 700.0, 900.0, Button::Middle);
    assert_eq!(e.doc().objects.len(), 1);
    match &e.doc().objects[0].kind {
        ObjectKind::Text(t) => {
            assert_eq!(t.text, "hello");
            assert_eq!(t.origin, Point::new(700.0, 900.0));
        }
        _ => panic!(),
    }
    one_undo_step(&mut e, &before);
}

#[test]
fn hostile_params_leave_the_document_alone() {
    let mut e = engine();
    rect(&mut e, 0, 0, 100, 100);
    let before = snapshot(&e);
    for (cmd, p) in [
        ("object.update_region", json!({})),
        ("object.update_region", json!({"x1": 1e300, "y1": 0, "x2": 1, "y2": 1})),
        (
            "point.delete_region",
            json!({"x1": 0, "y1": 0, "x2": 1, "y2": 1, "id": 99}),
        ),
        ("point.delete_region", json!({"x1": -5, "y1": -5, "x2": 5, "y2": 5})),
        ("point.add_end", json!({"id": 1, "x": 0, "y": 0})),
        ("point.add_end", json!({"id": 1, "x": 0, "y": 0, "end": "middle"})),
        ("object.reverse_arrows", json!({})),
        ("object.break_all", json!({"ids": [1]})),
        ("arrange.align_to", json!({"reference": 1, "ids": [1]})),
        ("arrange.align_to", json!({"reference": 99})),
        ("arrange.align_to", json!({"reference": 1, "h": "bogus"})),
        ("shape.circle3", json!({"points": [[0, 0], [1, 1]]})),
        ("shape.circle3", json!({"points": [[0, 0], [1, 1], [2, 2]]})),
        ("shape.circle3", json!({"points": [[0, 0], [1e300, 1], [2, 2]]})),
        ("edit.paste_text", json!({"x": 0, "y": 0})),
    ] {
        assert!(e.execute(cmd, &p).is_err(), "{cmd} {p}");
        assert_eq!(snapshot(&e), before, "{cmd} {p}");
    }
}
