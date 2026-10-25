#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! S4: the audit's last missing functions: the drawing messages (arc radius,
//! neighbour lengths, scale box size), the crosshair.

use rustfig_engine::geom::Point;
use rustfig_engine::{Button, Engine, ToolEvent, ToolKind};
use serde_json::json;

fn press(e: &mut Engine, x: f64, y: f64, button: Button) {
    e.tool_event(ToolEvent::Press {
        pos: Point::new(x, y),
        button,
        shift: false,
    })
    .unwrap();
}

fn mv(e: &mut Engine, x: f64, y: f64) {
    e.tool_event(ToolEvent::Move { pos: Point::new(x, y) }).unwrap();
}

#[test]
fn moving_a_line_point_reports_both_neighbour_lengths() {
    let mut e = Engine::new();
    e.execute("shape.polyline", &json!({"points": [[0, 0], [1200, 1200], [2400, 0]]}))
        .unwrap();
    e.set_tool(ToolKind::MovePoint);
    press(&mut e, 1200.0, 1200.0, Button::Left);
    mv(&mut e, 1200.0, 600.0);
    let s = e.status().to_string();
    assert!(s.starts_with("Len 1 = "), "{s}");
    assert!(s.contains("dx1 = ") && s.contains("dy2 = "), "{s}");
}

#[test]
fn moving_an_arc_middle_point_reports_the_radius() {
    let mut e = Engine::new();
    e.execute("shape.arc", &json!({"points": [[0, 1200], [1200, 0], [2400, 1200]]}))
        .unwrap();
    e.set_tool(ToolKind::MovePoint);
    press(&mut e, 1200.0, 0.0, Button::Left);
    mv(&mut e, 1200.0, 100.0);
    let s = e.status().to_string();
    assert!(s.contains("Rad = "), "{s}");
}

#[test]
fn scaling_a_box_reports_size_and_factor() {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1200, "height": 1200}),
    )
    .unwrap();
    e.set_tool(ToolKind::Scale);
    press(&mut e, 1200.0, 1200.0, Button::Left);
    mv(&mut e, 2400.0, 2400.0);
    let s = e.status().to_string();
    assert!(s.contains("Width = ") && s.contains("Factor = 2.000"), "{s}");
}

fn key(e: &mut Engine, k: &str) {
    e.execute("tool.event", &json!({"type": "key", "key": k})).unwrap();
}

fn texts(e: &Engine) -> Vec<(String, f64, f64)> {
    e.doc()
        .objects
        .iter()
        .filter_map(|o| match &o.kind {
            rustfig_engine::model::ObjectKind::Text(t) => Some((t.text.clone(), t.size, t.origin.y)),
            _ => None,
        })
        .collect()
}

#[test]
fn control_keys_edit_the_line() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::Text);
    press(&mut e, 600.0, 600.0, Button::Left);
    e.tool_event(ToolEvent::Text("hello world".into())).unwrap();
    for _ in 0..5 {
        key(&mut e, "left");
    }
    key(&mut e, "kill_to_end");
    e.tool_event(ToolEvent::Text("!".into())).unwrap();
    key(&mut e, "home");
    for _ in 0..2 {
        key(&mut e, "right");
    }
    key(&mut e, "kill_to_start");
    key(&mut e, "enter");
    assert_eq!(texts(&e)[0].0, "llo !");
}

#[test]
fn superscript_and_subscript_start_the_next_piece() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::Text);
    press(&mut e, 600.0, 600.0, Button::Left);
    e.tool_event(ToolEvent::Text("x".into())).unwrap();
    key(&mut e, "superscript");
    e.tool_event(ToolEvent::Text("2".into())).unwrap();
    key(&mut e, "subscript");
    e.tool_event(ToolEvent::Text("y".into())).unwrap();
    key(&mut e, "enter");
    let t = texts(&e);
    assert_eq!(t.len(), 3, "{t:?}");
    assert_eq!(t[0].0, "x");
    assert_eq!(t[1].0, "2");
    assert_eq!(t[2].0, "y");
    // The superscript is smaller and higher; stepping back restores both.
    assert!(t[1].1 < t[0].1 && t[1].2 < t[0].2, "{t:?}");
    assert!((t[2].1 - t[0].1).abs() < 1e-6 && (t[2].2 - t[0].2).abs() < 1.0, "{t:?}");
}

#[test]
fn open_merge_panel_commands() {
    let dir = std::env::temp_dir().join(format!("rustfig-s4-panel-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let mut src = Engine::new();
    src.execute("doc.set_comments", &json!({"comments": "first line\nsecond line"}))
        .ok();
    src.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 2400, "height": 1200}),
    )
    .unwrap();
    let fig = dir.join("box.fig");
    src.execute("file.save", &json!({"path": fig.to_str().unwrap()}))
        .unwrap();
    std::fs::write(dir.join("notes.txt"), b"x").unwrap();
    std::fs::write(dir.join(".hidden.fig"), b"x").unwrap();

    let mut e = Engine::new();
    let l = e
        .execute("file.list_dir", &json!({"dir": dir.to_str().unwrap()}))
        .unwrap();
    assert_eq!(l["files"], json!(["box.fig"]), "{l}");
    assert_eq!(l["dirs"], json!(["..", "sub"]), "{l}");
    let all = e
        .execute(
            "file.list_dir",
            &json!({"dir": dir.to_str().unwrap(), "mask": "*", "hidden": true}),
        )
        .unwrap();
    assert_eq!(all["files"].as_array().unwrap().len(), 3, "{all}");
    assert!(e.execute("file.list_dir", &json!({"dir": "/no/such/folder"})).is_err());

    let p = e
        .execute("file.preview", &json!({"path": fig.to_str().unwrap()}))
        .unwrap();
    assert_eq!(p["objects"], 1);
    assert_eq!(p["size"], "2.0x1.0 in", "{p}");
    assert_eq!(p["comments"], "first line\nsecond line", "{p}");
    let pv = rustfig_engine::file_panel::preview_figure(e.doc().settings.units, fig.to_str().unwrap()).unwrap();
    let (w, h, rgba) = pv.render(100, 100).unwrap();
    assert_eq!(rgba.len(), (w * h * 4) as usize);
    assert!(w > h, "the figure is wider than tall");
    assert!(rgba.chunks(4).any(|p| p[0] < 128), "something is drawn");
    assert!(e
        .execute(
            "file.preview",
            &json!({"path": dir.join("notes.txt").to_str().unwrap()})
        )
        .is_err());

    // Merge with a Figure Offset in inches.
    e.execute(
        "file.merge",
        &json!({"path": fig.to_str().unwrap(), "offset_x": 1, "offset_y": 2, "unit": "inches"}),
    )
    .unwrap();
    let b = e.doc_bounds();
    assert!(
        (b.min.x - 1200.0).abs() < 100.0 && (b.min.y - 2400.0).abs() < 100.0,
        "{b:?}"
    );
    assert!(e
        .execute("file.merge", &json!({"path": fig.to_str().unwrap(), "unit": "furlong"}))
        .is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn crosshair_follows_the_pointer_while_an_action_is_on() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::Polyline);
    press(&mut e, 100.0, 100.0, Button::Left);
    mv(&mut e, 500.0, 300.0);
    assert_eq!(e.overlay().crosshair, None, "off by default");
    e.execute("view.toggle_crosshair", &json!({})).unwrap();
    // The pointer after snapping to the grid.
    let c = e.overlay().crosshair.unwrap();
    assert!((c.x - 500.0).abs() < 100.0 && (c.y - 300.0).abs() < 100.0, "{c:?}");
    e.execute("view.toggle_crosshair", &json!({"on": false})).unwrap();
    assert_eq!(e.overlay().crosshair, None);
}
