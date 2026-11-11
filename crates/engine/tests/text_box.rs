#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Q8: a box behind a text and the vertical anchor of a new text.

use rustfig_engine::Engine;
use rustfig_geom::{Point, Rect};
use rustfig_model::{ColorRef, Object, ObjectKind, PolyKind, Text};
use serde_json::{json, Value};

fn texts(o: &Object, out: &mut Vec<Text>) {
    match &o.kind {
        ObjectKind::Text(t) => out.push(t.clone()),
        ObjectKind::Compound(c) => c.objects.iter().for_each(|o| texts(o, out)),
        _ => {}
    }
}

fn all_texts(e: &Engine) -> Vec<Text> {
    let mut v = Vec::new();
    e.doc().objects.iter().for_each(|o| texts(o, &mut v));
    v
}

fn text(e: &mut Engine, extra: Value) -> Value {
    let mut p = json!({"x": 2000, "y": 1000, "text": "Hxg", "size": 12, "font": 0, "postscript": true});
    p.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    e.execute("shape.text", &p).unwrap()
}

fn bbox(t: &Text) -> Rect {
    Rect::from_points(t.corners())
}

#[test]
fn anchor_puts_the_point_on_the_top_or_the_middle() {
    let mut e = Engine::new();
    text(&mut e, json!({"anchor": "baseline", "justify": "left"}));
    text(&mut e, json!({"anchor": "top", "justify": "left"}));
    text(&mut e, json!({"anchor": "middle", "justify": "left"}));
    let t = all_texts(&e);
    assert_eq!(t[0].origin, Point::new(2000.0, 1000.0));
    let top = bbox(&t[1]);
    assert!((top.min.y - 1000.0).abs() < 1.0, "{top:?}");
    assert_eq!(t[1].origin.x, 2000.0);
    let mid = bbox(&t[2]);
    assert!(((mid.min.y + mid.max.y) / 2.0 - 1000.0).abs() < 1.0, "{mid:?}");
    assert!(t[1].origin.y > t[2].origin.y && t[2].origin.y > t[0].origin.y);
}

#[test]
fn anchor_follows_the_text_angle() {
    let mut e = Engine::new();
    text(&mut e, json!({"anchor": "top", "angle": 90, "justify": "left"}));
    let t = &all_texts(&e)[0];
    // The top of the text is at the point: the baseline is one text height away, along the text's "down".
    let d = t.origin.dist(Point::new(2000.0, 1000.0));
    let h = bbox(&{
        let mut u = t.clone();
        u.angle = 0.0;
        u
    });
    assert!((d - (-(h.min.y - t.origin.y))).abs() < 1.0, "{d} {h:?}");
    // 90 degrees counter-clockwise: the text's down is to the right on the screen.
    assert!(
        t.origin.x > 2000.0 && (t.origin.y - 1000.0).abs() < 1.0,
        "{:?}",
        t.origin
    );
}

#[test]
fn indicator_anchor_and_box_are_the_defaults() {
    let mut e = Engine::new();
    e.execute(
        "indicator.set",
        &json!({"text_anchor": "top", "text_boxed": true, "text_box_margin": 100, "fill_color": 6}),
    )
    .unwrap();
    let r = text(&mut e, json!({"justify": "left"}));
    assert!(r["id"].is_u64());
    let ObjectKind::Compound(c) = &e.doc().objects[0].kind else {
        panic!("not a compound")
    };
    assert_eq!(c.objects.len(), 2);
    let ObjectKind::Polyline(b) = &c.objects[0].kind else {
        panic!()
    };
    let ObjectKind::Text(t) = &c.objects[1].kind else {
        panic!()
    };
    assert_eq!(b.kind, PolyKind::Box);
    assert_eq!(b.style.depth, t.depth + 1);
    assert_eq!(b.style.fill_color, ColorRef(6));
    assert_eq!(b.style.area_fill, 20);
    // The box is the text's box plus the margin, and the top anchor puts the text's top at the point.
    let tb = bbox(t);
    let bb = Rect::from_points(b.points.iter().copied());
    assert!(
        (bb.min.x - (tb.min.x - 100.0)).abs() < 1.0 && (bb.max.x - (tb.max.x + 100.0)).abs() < 1.0,
        "{bb:?} {tb:?}"
    );
    assert!(
        (bb.min.y - 900.0).abs() < 1.0,
        "the text's top is at the point, the margin above it {bb:?}"
    );
    // box: false overrides the indicator.
    let before = e.doc().objects.len();
    text(&mut e, json!({"box": false}));
    assert!(matches!(e.doc().objects[before].kind, ObjectKind::Text(_)));
}

#[test]
fn box_options_lines_and_rotation() {
    let mut e = Engine::new();
    text(
        &mut e,
        json!({"text": "one\ntwo", "box": {"margin": 50, "fill_color": "#ffee00", "thickness": 0, "area_fill": 30}, "justify": "center"}),
    );
    assert_eq!(e.doc().objects.len(), 1, "one box around both lines");
    let ObjectKind::Compound(c) = &e.doc().objects[0].kind else {
        panic!()
    };
    assert_eq!(c.objects.len(), 3);
    let ObjectKind::Polyline(b) = &c.objects[0].kind else {
        panic!()
    };
    assert_eq!((b.style.thickness, b.style.area_fill), (0, 30));
    assert_eq!(e.doc().rgb(b.style.fill_color).to_hex(), "#ffee00");
    let (ObjectKind::Text(t1), ObjectKind::Text(t2)) = (&c.objects[1].kind, &c.objects[2].kind) else {
        panic!()
    };
    let bb = Rect::from_points(b.points.iter().copied());
    for t in [t1, t2] {
        let r = bbox(t);
        assert!(r.min.x >= bb.min.x && r.max.x <= bb.max.x && r.min.y >= bb.min.y && r.max.y <= bb.max.y);
    }
    // A rotated text gets a rotated box (a polygon).
    let mut e = Engine::new();
    text(&mut e, json!({"angle": 30, "box": true}));
    let ObjectKind::Compound(c) = &e.doc().objects[0].kind else {
        panic!()
    };
    let ObjectKind::Polyline(b) = &c.objects[0].kind else {
        panic!()
    };
    assert_eq!(b.kind, PolyKind::Polygon);
    assert_eq!(b.points.len(), 4);
    // One undo step.
    assert!(e.undo());
    assert!(e.doc().objects.is_empty());
}

#[test]
fn bad_anchor_and_box_params() {
    let mut e = Engine::new();
    let p = |extra: Value| {
        let mut p = json!({"x": 0, "y": 0, "text": "a"});
        p.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        p
    };
    let err = e.execute("shape.text", &p(json!({"anchor": "bottom"}))).unwrap_err();
    assert!(err.message.contains("top, middle, baseline"));
    for bad in [
        json!({"box": 3}),
        json!({"box": {"margin": "x"}}),
        json!({"box": {"fill_color": "nocolor"}}),
        json!({"box": {"margin": 1e300}}),
    ] {
        assert!(e.execute("shape.text", &p(bad.clone())).is_err(), "{bad}");
    }
    assert!(e.doc().objects.is_empty());
    // A LaTeX text without TeX is boxed by its stored size.
    e.execute(
        "shape.text",
        &p(json!({"special": true, "text": "$x^2$", "box": true, "anchor": "middle"})),
    )
    .unwrap();
    assert_eq!(e.doc().objects.len(), 1);
}
