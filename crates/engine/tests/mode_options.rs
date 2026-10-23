#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! D10: the remaining mode options (dimension lines, align to canvas, array
//! copy, rotate/flip anchor and copies, one-edge scaling, lengths while
//! drawing, typed coordinates).

use rustfig_engine::model::{ObjectKind, Units};
use rustfig_engine::{Button, Engine, ToolEvent, ToolKind};
use serde_json::{json, Value};

fn press(e: &mut Engine, x: f64, y: f64, button: Button, shift: bool) {
    e.tool_event(ToolEvent::Press {
        pos: rustfig_engine::geom::Point::new(x, y),
        button,
        shift,
    })
    .unwrap();
}

fn release(e: &mut Engine, x: f64, y: f64, button: Button) {
    e.tool_event(ToolEvent::Release {
        pos: rustfig_engine::geom::Point::new(x, y),
        button,
    })
    .unwrap();
}

fn rect(e: &mut Engine, x: f64, y: f64, w: f64, h: f64) -> u64 {
    let r = e
        .execute("shape.rectangle", &json!({"x": x, "y": y, "width": w, "height": h}))
        .unwrap();
    r["id"].as_u64().unwrap()
}

fn bbox(e: &Engine, id: u64) -> (f64, f64, f64, f64) {
    let b = e
        .doc()
        .get(rustfig_engine::model::ObjectId(id))
        .unwrap()
        .geometry_bbox();
    (b.min.x, b.min.y, b.max.x, b.max.y)
}

#[test]
fn dimension_line_by_shift_left_in_polyline_mode() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::Polyline);
    assert_eq!(
        e.mouse_hints_with(true).left,
        "dimension line",
        "listed before the first point"
    );
    press(&mut e, 0.0, 0.0, Button::Left, true);
    // While it is drawn: the second point is final.
    let h = e.mouse_hints();
    assert_eq!(h.left, "final point");
    press(&mut e, 2400.0, 0.0, Button::Left, false);
    assert_eq!(e.doc().objects.len(), 1);
    let o = &e.doc().objects[0];
    assert!(rustfig_engine::dimline::is_dimline(o));
    assert_eq!(o.comments[0], "Dimension line: 2 in");
    let ObjectKind::Compound(c) = &o.kind else { panic!() };
    // The line, the text box, two ticks and the text.
    assert_eq!(c.objects.len(), 5);
    assert!(e.undo());
    assert!(e.doc().objects.is_empty());
}

#[test]
fn dimension_line_recomputes_after_scaling_and_close() {
    let mut e = Engine::new();
    let r = e
        .execute("shape.dimension_line", &json!({"x1": 0, "y1": 0, "x2": 1200, "y2": 0}))
        .unwrap();
    let id = r["id"].as_u64().unwrap();
    e.execute(
        "transform.scale",
        &json!({"ids": [id], "sx": 2.0, "sy": 2.0, "cx": 0, "cy": 0}),
    )
    .unwrap();
    assert_eq!(
        e.doc().get(rustfig_engine::model::ObjectId(id)).unwrap().comments[0],
        "Dimension line: 2 in"
    );
    // Open it, move the end of the main line, close: the length follows (gaps #5).
    e.execute("compound.open", &json!({"id": id})).unwrap();
    let main = e
        .doc()
        .objects
        .iter()
        .find(|o| o.comments.first().is_some_and(|c| c == "main dimension line"))
        .unwrap()
        .id;
    e.execute("point.move", &json!({"id": main.0, "index": 1, "x": 3600, "y": 0}))
        .unwrap();
    e.execute("compound.close", &json!({})).unwrap();
    let o = &e.doc().objects[0];
    assert_eq!(o.comments[0], "Dimension line: 3 in");
}

#[test]
fn dimension_line_rejects_equal_points_and_uses_unit_scale() {
    let mut e = Engine::new();
    assert!(e
        .execute("shape.dimension_line", &json!({"x1": 5, "y1": 5, "x2": 5, "y2": 5}))
        .is_err());
    e.doc_mut().settings.units = Units::Metric;
    e.execute("shape.dimension_line", &json!({"x1": 0, "y1": 0, "x2": 900, "y2": 0}))
        .unwrap();
    assert_eq!(e.doc().objects[0].comments[0], "Dimension line: 2.0 cm");
}

#[test]
fn align_to_canvas_with_the_middle_button() {
    let mut e = Engine::new();
    let a = rect(&mut e, 600.0, 600.0, 1200.0, 600.0);
    e.execute("indicator.set", &json!({"align_h": "left", "align_v": "top"}))
        .unwrap();
    e.set_tool(ToolKind::Align);
    assert_eq!(e.mouse_hints().middle, "align canvas");
    press(&mut e, 10000.0, 10000.0, Button::Middle, false);
    let (x0, y0, ..) = bbox(&e, a);
    assert!(x0.abs() < 1e-6 && y0.abs() < 1e-6, "{x0} {y0}");
    // Centered on the page (Letter, landscape: 11 x 8.5 inches).
    e.execute("arrange.align_canvas", &json!({"h": "center", "v": "center"}))
        .unwrap();
    let (x0, y0, x1, y1) = bbox(&e, a);
    assert!(((x0 + x1) / 2.0 - 5.5 * 1200.0).abs() < 1.0, "{x0} {x1}");
    assert!(((y0 + y1) / 2.0 - 4.25 * 1200.0).abs() < 1.0, "{y0} {y1}");
    assert!(e.execute("arrange.align_canvas", &json!({"h": "bogus"})).is_err());
}

#[test]
fn array_copy_with_the_middle_button() {
    let mut e = Engine::new();
    rect(&mut e, 0.0, 0.0, 100.0, 100.0);
    e.execute(
        "indicator.set",
        &json!({"snap": "any", "num_x_copies": 3, "num_y_copies": 2}),
    )
    .unwrap();
    e.set_tool(ToolKind::Copy);
    press(&mut e, 0.0, 50.0, Button::Left, false);
    release(&mut e, 0.0, 50.0, Button::Left);
    let h = e.mouse_hints();
    assert_eq!((h.left, h.middle), ("place object", "array placement"));
    press(&mut e, 200.0, 250.0, Button::Middle, false);
    // The original and a 3 x 2 grid without its first cell: 6 in all.
    assert_eq!(e.doc().objects.len(), 6);
    let xs: Vec<f64> = e.doc().objects.iter().map(|o| o.geometry_bbox().min.x).collect();
    assert!(xs.contains(&400.0) && xs.contains(&200.0), "{xs:?}");
    // The left button still copies one.
    let mut e2 = Engine::new();
    rect(&mut e2, 0.0, 0.0, 100.0, 100.0);
    e2.execute("indicator.set", &json!({"snap": "any", "num_x_copies": 3}))
        .unwrap();
    e2.set_tool(ToolKind::Copy);
    press(&mut e2, 0.0, 50.0, Button::Left, false);
    release(&mut e2, 0.0, 50.0, Button::Left);
    press(&mut e2, 200.0, 50.0, Button::Left, false);
    assert_eq!(e2.doc().objects.len(), 2);
}

#[test]
fn rotate_about_a_set_center_with_copies() {
    let mut e = Engine::new();
    let a = rect(&mut e, 1000.0, 0.0, 200.0, 200.0);
    e.execute("indicator.set", &json!({"rotate_angle": 90.0, "num_copies": 3}))
        .unwrap();
    e.set_tool(ToolKind::RotateCcw);
    assert_eq!(e.mouse_hints().right, "set center");
    press(&mut e, 0.0, 0.0, Button::Right, false);
    assert_eq!(e.mouse_hints().right, "unset center");
    assert!(!e.overlay().handles.is_empty(), "the centre is marked");
    // Copy & rotate: three copies at 90, 180 and 270 degrees about (0, 0).
    press(&mut e, 1000.0, 100.0, Button::Middle, false);
    assert_eq!(e.doc().objects.len(), 4);
    let (x0, y0, x1, y1) = bbox(&e, a);
    assert_eq!((x0, y0, x1, y1), (1000.0, 0.0, 1200.0, 200.0), "the original stays");
    let mut centres: Vec<(i64, i64)> = e
        .doc()
        .objects
        .iter()
        .map(|o| {
            let b = o.geometry_bbox();
            (
                ((b.min.x + b.max.x) / 2.0).round() as i64,
                ((b.min.y + b.max.y) / 2.0).round() as i64,
            )
        })
        .collect();
    centres.sort();
    assert_eq!(centres, vec![(-1100, -100), (-100, 1100), (100, -1100), (1100, 100)]);
    // The right button again unsets it; changing the mode erases it too.
    press(&mut e, 5.0, 5.0, Button::Right, false);
    assert_eq!(e.mouse_hints().right, "set center");
}

#[test]
fn flip_about_a_set_anchor() {
    let mut e = Engine::new();
    let a = rect(&mut e, 100.0, 0.0, 200.0, 200.0);
    e.set_tool(ToolKind::FlipH);
    assert_eq!(e.mouse_hints().right, "set anchor");
    press(&mut e, 0.0, 500.0, Button::Right, false);
    press(&mut e, 300.0, 100.0, Button::Left, false);
    let (x0, _, x1, _) = bbox(&e, a);
    assert_eq!((x0, x1), (-300.0, -100.0), "mirrored about x = 0, not the click");
    e.set_tool(ToolKind::FlipV);
    assert_eq!(e.mouse_hints().right, "set anchor", "a new mode forgets the anchor");
}

#[test]
fn scale_one_edge() {
    let mut e = Engine::new();
    let a = rect(&mut e, 0.0, 0.0, 1000.0, 500.0);
    e.set_tool(ToolKind::Scale);
    // The middle of the right edge: only the width changes, about the left edge.
    press(&mut e, 1000.0, 250.0, Button::Left, false);
    press(&mut e, 1500.0, 250.0, Button::Left, false);
    assert_eq!(bbox(&e, a), (0.0, 0.0, 1500.0, 500.0));
    // The middle of the bottom edge: only the height.
    press(&mut e, 750.0, 500.0, Button::Left, false);
    press(&mut e, 750.0, 1000.0, Button::Left, false);
    assert_eq!(bbox(&e, a), (0.0, 0.0, 1500.0, 1000.0));
    // A corner: both, in proportion.
    press(&mut e, 1500.0, 1000.0, Button::Left, false);
    press(&mut e, 3000.0, 2000.0, Button::Left, false);
    assert_eq!(bbox(&e, a), (0.0, 0.0, 3000.0, 2000.0));
}

#[test]
fn lengths_in_the_message_panel_and_on_the_canvas() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::Polyline);
    press(&mut e, 0.0, 0.0, Button::Left, false);
    e.tool_event(ToolEvent::Move {
        pos: rustfig_engine::geom::Point::new(1200.0, 0.0),
    })
    .unwrap();
    let s = e.status().to_string();
    assert!(
        s.contains("Length = 1 in") && s.contains("dx = 1 in") && s.contains("dy = 0 in"),
        "{s}"
    );
    assert!(s.contains("(0.0 deg)"), "{s}");
    assert!(e.overlay().lengths.is_empty(), "off by default");
    e.execute("view.toggle_lengths", &json!({})).unwrap();
    e.tool_event(ToolEvent::Move {
        pos: rustfig_engine::geom::Point::new(1200.0, 600.0),
    })
    .unwrap();
    let ov = e.overlay();
    let labels: Vec<&str> = ov.lengths.iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(labels, vec!["1.000", "0.500"]);
    assert!(e.status().contains("Length = 1.1 in"), "{}", e.status());
}

#[test]
fn typed_coordinates_for_the_next_point() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::Polyline);
    e.execute("tool.point", &json!({"text": "1, 1"})).unwrap();
    e.execute("tool.point", &json!({"text": "r2 0"})).unwrap();
    e.execute("tool.point", &json!({"text": "1<90"})).unwrap();
    e.tool_event(ToolEvent::Key(rustfig_engine::Key::Enter)).unwrap();
    let ObjectKind::Polyline(p) = &e.doc().objects[0].kind else {
        panic!()
    };
    let pts: Vec<(f64, f64)> = p.points.iter().map(|q| (q.x, q.y)).collect();
    assert_eq!(pts, vec![(1200.0, 1200.0), (3600.0, 1200.0), (3600.0, 2400.0)]);
    // Relative without a current point, and nonsense, are errors.
    let mut e2 = Engine::new();
    e2.set_tool(ToolKind::Polyline);
    assert!(e2.execute("tool.point", &json!({"text": "r1,1"})).is_err());
    assert!(e2.execute("tool.point", &json!({"text": "banana"})).is_err());
    assert!(e2.execute("tool.point", &json!({"text": Value::Null})).is_err());
}
