#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The depth panel: depths that are off can't be picked or edited, the
//! panel's buttons, and export of the active layers only.

use rustfig_engine::geom::Point;
use rustfig_engine::{Button, Engine, ToolEvent, ToolKind};
use serde_json::{json, Value};

/// A box at depth 10 and a polyline at depth 60, far apart.
fn two_depths() -> Engine {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1200, "height": 600, "depth": 10}),
    )
    .unwrap();
    e.execute(
        "shape.polyline",
        &json!({"points": [[6000, 0], [7200, 0], [7200, 1200]], "depth": 60}),
    )
    .unwrap();
    e
}

fn hide(e: &mut Engine, d: i32) {
    e.execute("depth.set_visible", &json!({"depth": d, "visible": false}))
        .unwrap();
}

fn click(e: &mut Engine, x: f64, y: f64) {
    let pos = Point::new(x, y);
    e.tool_event(ToolEvent::Press {
        pos,
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    e.tool_event(ToolEvent::Release {
        pos,
        button: Button::Left,
    })
    .unwrap();
}

#[test]
fn hidden_objects_cannot_be_picked() {
    let mut e = two_depths();
    hide(&mut e, 60);
    let tol = 50.0;
    assert!(e.hit_test(Point::new(6600.0, 0.0), tol).is_none());
    assert!(e.hit_test(Point::new(600.0, 0.0), tol).is_some());
    // Editing tools ignore it too: delete, move point, add point.
    for tool in [
        ToolKind::Delete,
        ToolKind::MovePoint,
        ToolKind::AddPoint,
        ToolKind::DeletePoint,
    ] {
        e.set_tool(tool);
        click(&mut e, 7200.0, 0.0);
        click(&mut e, 6600.0, 0.0);
        click(&mut e, 7300.0, 300.0);
        assert!(!e.tool_busy(), "{} picked a hidden object", tool.id());
        assert_eq!(e.doc().objects[1].points().unwrap().len(), 3, "{} edited it", tool.id());
    }
    // Select all and the rubber band skip it.
    e.execute("edit.select_all", &json!({})).unwrap();
    assert_eq!(e.selection().len(), 1);
    e.set_tool(ToolKind::Select);
    e.set_selection(vec![]);
    e.tool_event(ToolEvent::Press {
        pos: Point::new(-100.0, -100.0),
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    e.tool_event(ToolEvent::Move {
        pos: Point::new(8000.0, 2000.0),
    })
    .unwrap();
    e.tool_event(ToolEvent::Release {
        pos: Point::new(8000.0, 2000.0),
        button: Button::Left,
    })
    .unwrap();
    assert_eq!(e.selection().len(), 1);
}

#[test]
fn hiding_a_depth_drops_its_objects_from_the_selection() {
    let mut e = two_depths();
    e.execute("edit.select_all", &json!({})).unwrap();
    assert_eq!(e.selection().len(), 2);
    hide(&mut e, 10);
    assert_eq!(e.selection().len(), 1);
}

#[test]
fn all_off_toggle_all_on() {
    let mut e = two_depths();
    e.execute("depth.hide_all", &json!({})).unwrap();
    assert_eq!(e.hidden_depths.len(), 2);
    hide(&mut e, 10);
    e.execute("depth.set_visible", &json!({"depth": 10, "visible": true}))
        .unwrap();
    e.execute("depth.toggle_all", &json!({})).unwrap();
    assert_eq!(e.hidden_depths.iter().copied().collect::<Vec<_>>(), vec![10]);
    e.execute("depth.set_visible", &json!({"depths": [10, 60], "visible": false}))
        .unwrap();
    assert_eq!(e.hidden_depths.len(), 2);
    e.execute("depth.show_all", &json!({})).unwrap();
    assert!(e.hidden_depths.is_empty());
    assert_eq!(e.depth_counts(), vec![(10, 1), (60, 1)]);
    // Bad parameters are refused.
    for bad in [
        json!({}),
        json!({"depths": "ten"}),
        json!({"depths": [f64::MAX, "x"]}),
        json!({"depths": vec![1; 5000]}),
    ] {
        assert!(e.execute("depth.set_visible", &bad).is_err(), "{bad}");
    }
    assert!(e.execute("depth.set_display", &json!({"mode": "pink"})).is_err());
    e.execute("depth.set_display", &json!({"mode": "blank"})).unwrap();
    assert!(!e.gray_hidden_depths);
}

/// Width and height of an exported SVG.
fn svg_size(e: &mut Engine, extra: Value) -> (f64, f64) {
    let mut p = json!({"format": "svg"});
    if let (Some(p), Some(x)) = (p.as_object_mut(), extra.as_object()) {
        p.extend(x.clone());
    }
    let v = e.execute("file.export_data", &p).unwrap();
    let svg = v["data"].as_str().unwrap().to_string();
    let attr = |name: &str| -> f64 {
        // gensvg's header: `\twidth="NNpt" height="NNpt"`.
        let key = if name == "width" { "\twidth=\"" } else { " height=\"" };
        let i = svg.find(key).unwrap() + name.len() + 3;
        let rest = &svg[i..];
        let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap();
        rest[..end].parse().unwrap()
    };
    (attr("width"), attr("height"))
}

#[test]
fn export_active_layers_only() {
    let mut e = two_depths();
    hide(&mut e, 60);
    let whole = svg_size(&mut e, json!({}));
    let active = svg_size(&mut e, json!({"active_layers_only": true}));
    assert!(active.0 < whole.0 / 2.0, "{active:?} vs {whole:?}");
    let framed = svg_size(&mut e, json!({"active_layers_only": true, "whole_figure_bounds": true}));
    assert_eq!(framed, whole);
    // Only the box is drawn: the polyline's points are missing.
    let v = e
        .execute(
            "file.export_data",
            &json!({"format": "svg", "active_layers_only": true}),
        )
        .unwrap();
    assert_eq!(v["data"].as_str().unwrap().matches("<path").count(), 1);
}
