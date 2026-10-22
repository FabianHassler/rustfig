#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The measure modes (plan D5): angle, length and area, with xfig's
//! messages, running totals and buttons. Nothing changes in the drawing.

use rustfig_engine::geom::Point;
use rustfig_engine::{Button, Engine, ToolEvent, ToolKind};
use serde_json::{json, Value};

fn run(e: &mut Engine, cmd: &str, p: Value) -> Value {
    e.execute(cmd, &p).unwrap()
}

fn last_id(e: &Engine) -> u64 {
    e.doc().objects.last().unwrap().id.0
}

fn add(e: &mut Engine, cmd: &str, p: Value) -> u64 {
    run(e, cmd, p);
    last_id(e)
}

fn len(e: &mut Engine, id: u64, add: bool) -> Value {
    run(e, "object.measure_length", json!({"id": id, "add": add}))
}

fn area(e: &mut Engine, id: u64, add: bool) -> Value {
    run(e, "object.measure_area", json!({"id": id, "add": add}))
}

fn text(v: &Value) -> &str {
    v["text"].as_str().unwrap()
}

fn near(a: &Value, want: f64, tol: f64) {
    let a = a.as_f64().unwrap();
    assert!((a - want).abs() <= tol, "{a} is not {want} +- {tol}");
}

fn press(e: &mut Engine, x: f64, y: f64, button: Button, shift: bool) {
    e.tool_event(ToolEvent::Press {
        pos: Point::new(x, y),
        button,
        shift,
    })
    .unwrap();
}

#[test]
fn angle_by_three_points() {
    let mut e = Engine::new();
    let r = run(
        &mut e,
        "object.measure_angle",
        json!({"points": [[1200, 0], [0, 0], [0, 1200]]}),
    );
    assert_eq!(text(&r), "Angle defined by three points: -90.00 Degrees");
    let r = run(
        &mut e,
        "object.measure_angle",
        json!({"points": [[0, 1200], [0, 0], [1200, 0]]}),
    );
    assert_eq!(text(&r), "Angle defined by three points: 90.00 Degrees");
    assert_eq!(e.status(), text(&r));
    // A zero-length side has no angle.
    assert!(e
        .execute("object.measure_angle", &json!({"points": [[0, 0], [0, 0], [5, 5]]}))
        .is_err());
    assert_eq!(e.status(), "Can't compute angle");
    assert!(e.execute("object.measure_angle", &json!({"points": [[0, 0]]})).is_err());
}

#[test]
fn angle_at_a_corner_and_save() {
    let mut e = Engine::new();
    let l = add(
        &mut e,
        "shape.polyline",
        json!({"points": [[0, 0], [1200, 0], [1200, 1200]]}),
    );
    let r = run(
        &mut e,
        "object.measure_angle",
        json!({"id": l, "x": 1200, "y": 0, "save": true}),
    );
    assert_eq!(text(&r), "Angle at polyline corner: 90.00 Degrees");
    assert_eq!(e.indicator.rotate_angle, 90.0);
    // The ends of an open line have no corner.
    assert!(e
        .execute("object.measure_angle", &json!({"id": l, "x": 0, "y": 0}))
        .is_err());
    assert_eq!(e.status(), "Can't compute angle at endpoint");
    // Nowhere near a vertex.
    assert!(e
        .execute("object.measure_angle", &json!({"id": l, "x": 600, "y": 0}))
        .is_err());
    // A polygon has a corner at its first point too.
    let g = add(
        &mut e,
        "shape.polygon",
        json!({"points": [[0, 0], [1200, 0], [1200, 1200], [0, 1200]]}),
    );
    let r = run(
        &mut e,
        "object.measure_angle",
        json!({"id": g, "x": 0, "y": 0, "save": false}),
    );
    near(&r["degrees"], 90.0, 1e-9);
    // An arc: the angle at its centre.
    let a = add(
        &mut e,
        "shape.arc",
        json!({"points": [[3000, 2000], [2707, 1293], [2000, 1000]]}),
    );
    let r = run(&mut e, "object.measure_angle", json!({"id": a}));
    assert!(text(&r).starts_with("Angle at arc center: "));
    near(&r["degrees"].as_f64().map(f64::abs).into(), 90.0, 0.5);
}

#[test]
fn lengths_and_the_running_total() {
    let mut e = Engine::new();
    let b = add(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 2400, "height": 1200}),
    );
    let l = add(&mut e, "shape.polyline", json!({"points": [[0, 0], [2400, 0]]}));
    let r = len(&mut e, b, false);
    assert_eq!(text(&r), "Length of box is 6 in");
    assert!(r["accumulated"].is_null());
    let r = len(&mut e, b, true);
    assert_eq!(text(&r), "Length of box is 6 in, accumulated 6 in");
    let r = len(&mut e, l, true);
    assert_eq!(text(&r), "Length of polyline is 2 in, accumulated 8 in");
    run(&mut e, "object.measure_reset", json!({"what": "length"}));
    assert_eq!(e.status(), "length reset to 0");
    let r = len(&mut e, l, true);
    assert_eq!(text(&r), "Length of polyline is 2 in, accumulated 2 in");
    // Tenths of an inch instead of fractions.
    e.unit_scale.decimal_inches = true;
    assert_eq!(text(&len(&mut e, b, false)), "Length of box is 6.0 in");
    e.unit_scale.decimal_inches = false;
    let g = add(
        &mut e,
        "shape.polygon",
        json!({"points": [[0, 0], [1200, 0], [1200, 1200]]}),
    );
    near(&len(&mut e, g, false)["length"], 4097.06, 0.01);
    // Fractions of 1/64: 1.5 in and 1-1/4 in.
    let m = add(&mut e, "shape.polyline", json!({"points": [[0, 0], [1800, 0]]}));
    assert_eq!(text(&len(&mut e, m, false)), "Length of polyline is 1-1/2 in");
    let m = add(&mut e, "shape.polyline", json!({"points": [[0, 0], [1500, 0]]}));
    assert_eq!(text(&len(&mut e, m, false)), "Length of polyline is 1-1/4 in");
}

#[test]
fn lengths_of_curves_and_metric_units() {
    let mut e = Engine::new();
    let c = add(&mut e, "shape.circle", json!({"cx": 3000, "cy": 3000, "r": 1000}));
    near(&len(&mut e, c, false)["length"], 6283.19, 0.01);
    assert_eq!(len(&mut e, c, false)["kind"], "circle");
    let el = add(
        &mut e,
        "shape.ellipse",
        json!({"cx": 0, "cy": 0, "rx": 1200, "ry": 600}),
    );
    let r = len(&mut e, el, false);
    assert_eq!(r["kind"], "ellipse");
    near(&r["length"], 5813.0, 1.0);
    let a = add(
        &mut e,
        "shape.arc",
        json!({"points": [[3000, 2000], [2707, 1293], [2000, 1000]]}),
    );
    let r = len(&mut e, a, false);
    assert_eq!(r["kind"], "arc");
    near(&r["length"], 1571.0, 8.0);
    // An arc box gives up the corners' circle.
    let ab = add(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 2400, "height": 1200, "radius": 8}),
    );
    let r = len(&mut e, ab, false);
    assert_eq!(r["kind"], "arcbox");
    near(&r["length"], 7200.0 - std::f64::consts::PI * 240.0, 1e-6);
    // Metric: 450 per cm.
    run(&mut e, "doc.set_units", json!({"units": "metric"}));
    let b = add(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 2400, "height": 1200}),
    );
    assert_eq!(text(&len(&mut e, b, false)), "Length of box is 16.0 cm");
    // The Unit/Scale setting scales the figure.
    e.unit_scale.factor = 2.0;
    e.unit_scale.unit = "m".into();
    assert_eq!(text(&len(&mut e, b, false)), "Length of box is 32.0 m");
}

#[test]
fn areas_with_sign_and_total() {
    let mut e = Engine::new();
    let b = add(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 2400, "height": 1200}),
    );
    let g = add(
        &mut e,
        "shape.polygon",
        json!({"points": [[0, 0], [1200, 0], [1200, 1200], [0, 1200]]}),
    );
    assert_eq!(text(&area(&mut e, b, false)), "Area of box is 2.0 square in");
    let r = area(&mut e, g, true);
    assert_eq!(text(&r), "Area of polygon is 1.0 square in, accumulated 1.0 square in");
    let r = area(&mut e, b, true);
    assert_eq!(text(&r), "Area of box is 2.0 square in, accumulated 3.0 square in");
    // Signed: this polygon turns clockwise on the screen.
    run(&mut e, "object.measure_reset", json!({"what": "area", "signed": true}));
    assert_eq!(e.status(), "signed area reset to 0");
    let r = area(&mut e, g, true);
    assert_eq!(r["signed"], true);
    near(&r["area"], -1440000.0, 1e-6);
    assert_eq!(
        text(&r),
        "Area of polygon is -1.0 square in, accumulated -1.0 square in"
    );
    run(&mut e, "object.measure_reset", json!({"what": "area"}));
    assert_eq!(e.status(), "area reset to 0");
    near(&area(&mut e, g, false)["area"], 1440000.0, 1e-6);
    // Curves.
    let c = add(&mut e, "shape.circle", json!({"cx": 0, "cy": 0, "r": 1000}));
    assert_eq!(text(&area(&mut e, c, false)), "Area of ellipse is 2.2 square in");
    let pie = add(
        &mut e,
        "shape.arc",
        json!({"points": [[3000, 2000], [2707, 1293], [2000, 1000]], "pie": true}),
    );
    let r = area(&mut e, pie, false);
    assert_eq!(r["kind"], "sector");
    near(&r["area"], 785398.0, 15000.0);
    let open = add(
        &mut e,
        "shape.arc",
        json!({"points": [[3000, 2000], [2707, 1293], [2000, 1000]]}),
    );
    let r = area(&mut e, open, false);
    assert_eq!(r["kind"], "open arc");
    near(&r["area"], 285398.0, 15000.0);
    let ab = add(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 2400, "height": 1200, "radius": 8}),
    );
    let r = area(&mut e, ab, false);
    assert_eq!(r["kind"], "arcbox");
    near(&r["area"], 2880000.0 - std::f64::consts::PI * 120.0 * 120.0, 1e-6);
    // Metric: three decimals.
    run(&mut e, "doc.set_units", json!({"units": "metric"}));
    assert_eq!(text(&area(&mut e, b, false)), "Area of box is 14.222 square cm");
}

#[test]
fn refused_kinds() {
    let mut e = Engine::new();
    let t = add(&mut e, "shape.text", json!({"x": 0, "y": 0, "text": "hi"}));
    let s = add(
        &mut e,
        "shape.spline",
        json!({"points": [[0, 0], [500, 500], [1000, 0]]}),
    );
    let l = add(&mut e, "shape.polyline", json!({"points": [[0, 0], [2400, 0]]}));
    for id in [t, s] {
        assert!(e.execute("object.measure_length", &json!({"id": id})).is_err());
        assert_eq!(e.status(), "Sorry, can't measure length of this object");
        assert!(e.execute("object.measure_area", &json!({"id": id})).is_err());
        assert_eq!(e.status(), "Sorry, can't measure area of this object");
    }
    // An open line has a length but no area.
    assert!(e.execute("object.measure_area", &json!({"id": l})).is_err());
    assert!(e.execute("object.measure_length", &json!({"id": 9999})).is_err());
    assert!(e.execute("object.measure_reset", &json!({"what": "volume"})).is_err());
    assert!(e
        .execute("object.measure_angle", &json!({"id": t, "x": 0, "y": 0}))
        .is_err());
}

#[test]
fn the_buttons_and_hints() {
    let mut e = Engine::new();
    let b = add(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 2400, "height": 1200}),
    );
    let l = add(
        &mut e,
        "shape.polyline",
        json!({"points": [[0, 3000], [1200, 3000], [1200, 4200]]}),
    );
    run(&mut e, "tool.length_measure", json!({}));
    assert_eq!(e.tool(), ToolKind::LengthMeasure);
    let h = e.mouse_hints();
    assert_eq!(
        (h.left, h.middle, h.right),
        ("select object", "select & add", "reset to 0")
    );
    let before = e.doc().objects.clone();
    let rev = e.revision();
    let sel = e.selection().to_vec();
    press(&mut e, 1200.0, 0.0, Button::Left, false);
    assert_eq!(e.status(), "Length of box is 6 in");
    press(&mut e, 1200.0, 0.0, Button::Middle, false);
    assert_eq!(e.status(), "Length of box is 6 in, accumulated 6 in");
    press(&mut e, 600.0, 3000.0, Button::Middle, false);
    assert_eq!(e.status(), "Length of polyline is 2 in, accumulated 8 in");
    press(&mut e, 600.0, 3000.0, Button::Middle, true);
    assert_eq!(
        e.status(),
        "Length of polyline is 2 in, accumulated 8 in",
        "Shift only locates"
    );
    press(&mut e, 0.0, 0.0, Button::Right, false);
    assert_eq!(e.status(), "length reset to 0");
    press(&mut e, 9000.0, 9000.0, Button::Left, false);
    // Nothing changed: same objects, no undo step, selection as it was.
    assert_eq!(e.doc().objects, before);
    assert_eq!(e.revision(), rev);
    assert_eq!(e.selection(), sel.as_slice());
    e.set_selection(vec![rustfig_engine::model::ObjectId(b)]);
    press(&mut e, 600.0, 3000.0, Button::Left, false);
    assert_eq!(e.selection().len(), 1);
    let _ = l;

    // Area: Shift+right resets to signed.
    run(&mut e, "tool.area_measure", json!({}));
    let h = e.mouse_hints();
    assert_eq!(
        (h.left, h.middle, h.right),
        ("select object", "select & add", "reset to 0")
    );
    assert_eq!(e.mouse_hints_with(true).right, "reset to +-0");
    press(&mut e, 1200.0, 0.0, Button::Left, false);
    assert_eq!(e.status(), "Area of box is 2.0 square in");
    // The open line is not in the area mode's mask.
    press(&mut e, 600.0, 3000.0, Button::Left, false);
    assert_eq!(e.status(), "Area of box is 2.0 square in");
    press(&mut e, 0.0, 0.0, Button::Right, true);
    assert_eq!(e.status(), "signed area reset to 0");
    press(&mut e, 0.0, 0.0, Button::Right, false);
    assert_eq!(e.status(), "area reset to 0");
}

#[test]
fn the_angle_tool() {
    let mut e = Engine::new();
    let l = add(
        &mut e,
        "shape.polyline",
        json!({"points": [[0, 0], [1200, 0], [1200, 1200]]}),
    );
    run(&mut e, "tool.angle_measure", json!({}));
    let h = e.mouse_hints();
    assert_eq!(
        (h.left, h.middle, h.right),
        ("first point", "select & save", "select object")
    );
    let before = e.doc().objects.clone();
    let rev = e.revision();
    // Middle on the corner: measure and save.
    press(&mut e, 1200.0, 0.0, Button::Middle, false);
    assert_eq!(e.status(), "Angle at polyline corner: 90.00 Degrees");
    e.indicator.rotate_angle = 45.0;
    press(&mut e, 1200.0, 0.0, Button::Right, false);
    assert_eq!(e.indicator.rotate_angle, 45.0, "right does not save");
    press(&mut e, 1200.0, 0.0, Button::Middle, false);
    assert_eq!(e.indicator.rotate_angle, 90.0);
    // Three points.
    press(&mut e, 5000.0, 0.0, Button::Left, false);
    let h = e.mouse_hints();
    assert_eq!((h.left, h.middle, h.right), ("angle tip", "", "cancel"));
    e.tool_event(ToolEvent::Move {
        pos: Point::new(5000.0, 500.0),
    })
    .unwrap();
    assert_eq!(e.overlay().guides[0].len(), 2);
    press(&mut e, 5000.0, 0.0, Button::Left, false); // the same point: ignored
    press(&mut e, 4000.0, 0.0, Button::Left, false);
    let h = e.mouse_hints();
    assert_eq!((h.left, h.middle, h.right), ("final point", "save angle", "cancel"));
    assert_eq!(e.overlay().guides[0].len(), 3);
    press(&mut e, 4000.0, 1000.0, Button::Left, false);
    assert_eq!(e.status(), "Angle defined by three points: -90.00 Degrees");
    assert_eq!(e.indicator.rotate_angle, 90.0, "left does not save");
    assert_eq!(e.mouse_hints().left, "first point");
    // Middle on the last point saves; right cancels.
    press(&mut e, 5000.0, 0.0, Button::Left, false);
    press(&mut e, 4000.0, 0.0, Button::Left, false);
    press(&mut e, 4000.0, 1000.0, Button::Middle, false);
    assert_eq!(e.indicator.rotate_angle, 90.0);
    press(&mut e, 5000.0, 0.0, Button::Left, false);
    press(&mut e, 4000.0, 0.0, Button::Left, false);
    press(&mut e, 4000.0, 0.0, Button::Right, false);
    assert!(e.overlay().guides.is_empty());
    assert_eq!(e.mouse_hints().left, "first point");
    assert_eq!(e.doc().objects, before);
    assert_eq!(e.revision(), rev);
    let _ = l;
}

#[test]
fn keys_and_ids() {
    assert_eq!(ToolKind::from_id("tool.angle_measure"), Some(ToolKind::AngleMeasure));
    assert_eq!(ToolKind::from_id("tool.length_measure"), Some(ToolKind::LengthMeasure));
    assert_eq!(ToolKind::from_id("tool.area_measure"), Some(ToolKind::AreaMeasure));
}
