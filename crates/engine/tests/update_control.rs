#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Update Control (plan W10-1): the update flags choose which attributes
//! Update, update region and update settings copy (xfig's `cur_updatemask`).

use rustfig_engine::geom::Point;
use rustfig_engine::model::ObjectKind;
use rustfig_engine::{Button, Engine, SnapMode, ToolEvent, ToolKind, UpdateFlag};
use serde_json::json;

fn engine() -> Engine {
    let mut e = Engine::new();
    e.indicator.snap = SnapMode::Any;
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1000, "height": 500}),
    )
    .unwrap();
    // The indicator now differs from the rectangle in fill, width and pen.
    e.execute(
        "indicator.set",
        &json!({"area_fill": 20, "fill_color": 4, "thickness": 6, "pen_color": 2}),
    )
    .unwrap();
    e
}

fn style(e: &Engine) -> (i32, i32, i32, i32) {
    match &e.doc().objects[0].kind {
        ObjectKind::Polyline(p) => (
            p.style.area_fill,
            p.style.fill_color.0,
            p.style.thickness,
            p.style.pen_color.0,
        ),
        _ => panic!("not a polyline"),
    }
}

fn only(e: &mut Engine, flag: &str) {
    e.execute("indicator.update_flags", &json!({"all": false})).unwrap();
    e.execute("indicator.set_update_flag", &json!({"attr": flag, "on": true}))
        .unwrap();
}

#[test]
fn all_flags_start_on() {
    let e = Engine::new();
    assert!(UpdateFlag::ALL.iter().all(|f| e.indicator.updates(*f)));
}

#[test]
fn update_with_only_fill_style_changes_only_the_fill_style() {
    let mut e = engine();
    let (fill0, color0, width0, pen0) = style(&e);
    only(&mut e, "fill_style");
    let ids: Vec<u64> = e.doc().objects.iter().map(|o| o.id.0).collect();
    e.execute("object.update", &json!({"ids": ids})).unwrap();
    assert_eq!(style(&e), (20, color0, width0, pen0));
    assert_ne!(fill0, 20);
    assert!(e.undo());
    assert_eq!(style(&e).0, fill0);
}

#[test]
fn update_region_honours_the_flags() {
    let mut e = engine();
    let (_, color0, _, pen0) = style(&e);
    only(&mut e, "line_width");
    e.execute(
        "object.update_region",
        &json!({"x1": -100, "y1": -100, "x2": 2000, "y2": 2000}),
    )
    .unwrap();
    let (fill, color, width, pen) = style(&e);
    assert_eq!((width, color, pen), (6, color0, pen0));
    assert_eq!(fill, -1);
}

#[test]
fn all_cleared_changes_nothing_and_toggle_inverts() {
    let mut e = engine();
    let before = style(&e);
    e.execute("indicator.update_flags", &json!({"all": false})).unwrap();
    e.execute("object.update", &json!({"ids": [e.doc().objects[0].id.0]}))
        .unwrap();
    assert_eq!(style(&e), before);
    e.execute("indicator.set_update_flag", &json!({"attr": "depth", "on": true}))
        .unwrap();
    let r = e.execute("indicator.update_flags", &json!({"all": "toggle"})).unwrap();
    assert_eq!(r["on"].as_array().unwrap().len(), UpdateFlag::ALL.len() - 1);
    assert!(!e.indicator.updates(UpdateFlag::Depth));
    e.execute("indicator.update_flags", &json!({"all": true})).unwrap();
    assert!(UpdateFlag::ALL.iter().all(|f| e.indicator.updates(*f)));
    // Without "on" a flag toggles.
    e.execute("indicator.set_update_flag", &json!({"attr": "font"}))
        .unwrap();
    assert!(!e.indicator.updates(UpdateFlag::Font));
    assert!(e
        .execute("indicator.set_update_flag", &json!({"attr": "nope"}))
        .is_err());
}

#[test]
fn update_settings_honours_the_flags() {
    let mut e = engine();
    e.execute(
        "indicator.set",
        &json!({"thickness": 1, "pen_color": 0, "area_fill": -1}),
    )
    .unwrap();
    // A second object with its own attributes.
    e.execute(
        "shape.rectangle",
        &json!({"x": 3000, "y": 0, "width": 500, "height": 500}),
    )
    .unwrap();
    e.execute(
        "object.set_attributes",
        &json!({"ids": [e.doc().objects[1].id.0], "thickness": 5, "pen_color": 1}),
    )
    .unwrap();
    only(&mut e, "pen_color");
    e.set_tool(ToolKind::Update);
    let (x, y) = (3250.0, 0.0);
    for ev in [
        ToolEvent::Press {
            pos: Point::new(x, y),
            button: Button::Middle,
            shift: false,
        },
        ToolEvent::Release {
            pos: Point::new(x, y),
            button: Button::Middle,
        },
    ] {
        e.tool_event(ev).unwrap();
    }
    assert_eq!(e.indicator.pen_color, 1);
    assert_eq!(e.indicator.thickness, 1, "line width flag is off");
}
