#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! E4: colour commands: parsing, editor, recent colours, eyedropper, and
//! user colours surviving a save and reopen in the Fig file.

use rustfig_engine::tools::{Button, ToolEvent};
use rustfig_engine::Engine;
use rustfig_geom::Point;
use serde_json::json;

fn run(e: &mut Engine, id: &str, p: serde_json::Value) -> serde_json::Value {
    e.execute(id, &p).unwrap()
}

#[test]
fn parse_hex_and_x11_names() {
    let mut e = Engine::new();
    assert_eq!(run(&mut e, "color.parse", json!({"text": "#ff8000"}))["rgb"], "#ff8000");
    assert_eq!(run(&mut e, "color.parse", json!({"text": "#f80"}))["rgb"], "#ff8800");
    let r = run(&mut e, "color.parse", json!({"text": "Dark Orange"}));
    assert_eq!(r["rgb"], "#ff8c00");
    let r = run(&mut e, "color.parse", json!({"text": "#00ff00"}));
    assert_eq!(
        (r["h"].as_f64(), r["s"].as_f64(), r["v"].as_f64()),
        (Some(120.0), Some(100.0), Some(100.0))
    );
    assert!(e.execute("color.parse", &json!({"text": "nonsense"})).is_err());
    let r = run(&mut e, "color.from_hsv", json!({"h": 240, "s": 100, "v": 100}));
    assert_eq!(r["rgb"], "#0000ff");
    let r = run(&mut e, "color.names", json!({"prefix": "darko", "limit": 3}));
    assert!(!r["names"].as_array().unwrap().is_empty());
}

#[test]
fn editor_new_edit_delete() {
    let mut e = Engine::new();
    let a = run(&mut e, "color.new", json!({"rgb": "#102030"}))["color"]
        .as_i64()
        .unwrap();
    let b = run(&mut e, "color.new", json!({"rgb": "#102030"}))["color"]
        .as_i64()
        .unwrap();
    assert_eq!((a, b), (32, 33), "new colours get new slots even with equal values");
    run(&mut e, "color.edit", json!({"color": 32, "rgb": "red"}));
    assert_eq!(e.doc().colors[0].rgb.to_hex(), "#ff0000");
    assert!(e.execute("color.edit", &json!({"color": 40, "rgb": "red"})).is_err());
    assert!(e.execute("color.edit", &json!({"color": 5, "rgb": "red"})).is_err());
    // A colour in use is not deleted.
    run(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 600, "height": 600, "pen_color": 33}),
    );
    assert!(e.execute("color.delete", &json!({"color": 33})).is_err());
    run(&mut e, "color.delete", json!({"color": 32}));
    assert_eq!(e.doc().colors.len(), 1);
    // Undo restores the colour.
    run(&mut e, "edit.undo", json!({}));
    assert_eq!(e.doc().colors.len(), 2);
}

#[test]
fn recent_colours_newest_first_unique_capped() {
    let mut e = Engine::new();
    for i in 0..20 {
        run(&mut e, "color.use", json!({"rgb": format!("#0000{i:02x}")}));
    }
    run(&mut e, "color.use", json!({"rgb": "#000005"}));
    let r = run(&mut e, "color.recent", json!({}));
    let l = r["colors"].as_array().unwrap();
    assert_eq!(l.len(), 12);
    assert_eq!(l[0], "#000005");
    assert_eq!(l.iter().filter(|c| *c == "#000005").count(), 1);
    run(&mut e, "color.use", json!({"color": 4}));
    assert_eq!(run(&mut e, "color.recent", json!({}))["colors"][0], "#ff0000");
    assert!(e.prefs_dirty);
}

#[test]
fn eyedropper_takes_the_next_click() {
    let mut e = Engine::new();
    let c = run(&mut e, "color.new", json!({"rgb": "#336699"}))["color"]
        .as_i64()
        .unwrap();
    run(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 1200, "height": 1200, "pen_color": c, "fill_color": 4, "area_fill": 20}),
    );
    run(&mut e, "color.eyedropper", json!({"target": "pen"}));
    let press = |button| ToolEvent::Press {
        pos: Point::new(0.0, 600.0),
        button,
        shift: false,
    };
    e.tool_event(press(Button::Left)).unwrap();
    assert_eq!(i64::from(e.indicator.pen_color), c);
    // One shot: the next click does not pick again.
    e.indicator.pen_color = 0;
    e.tool_event(press(Button::Left)).unwrap();
    assert_eq!(e.indicator.pen_color, 0);
    // The colour under the pointer counts, whatever the target: inside
    // the filled box it is the fill colour (here into the fill), and on
    // the outline the line colour (here into the fill as well).
    run(&mut e, "color.eyedropper", json!({"target": "fill"}));
    e.tool_event(ToolEvent::Press {
        pos: Point::new(600.0, 600.0),
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    assert_eq!(e.indicator.fill_color, 4);
    run(&mut e, "color.eyedropper", json!({"target": "fill"}));
    e.tool_event(press(Button::Left)).unwrap();
    assert_eq!(i64::from(e.indicator.fill_color), c);
    run(&mut e, "color.eyedropper", json!({"target": "pen"}));
    e.tool_event(ToolEvent::Press {
        pos: Point::new(600.0, 600.0),
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    assert_eq!(e.indicator.pen_color, 4, "green fill into the pen colour");
    // Other buttons and Escape cancel; a click on nothing picks nothing.
    run(&mut e, "color.eyedropper", json!({"target": "pen"}));
    e.tool_event(press(Button::Right)).unwrap();
    run(&mut e, "color.eyedropper", json!({"target": "pen"}));
    run(&mut e, "tool.escape", json!({}));
    e.indicator.pen_color = 0;
    e.tool_event(press(Button::Left)).unwrap();
    assert_eq!(e.indicator.pen_color, 0);
    run(&mut e, "color.eyedropper", json!({"target": "pen"}));
    e.tool_event(ToolEvent::Press {
        pos: Point::new(90000.0, 90000.0),
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    assert_eq!(e.indicator.pen_color, 0);
}

#[test]
fn user_colours_survive_save_and_reopen() {
    let mut e = Engine::new();
    let c = run(&mut e, "color.new", json!({"rgb": "#336699"}))["color"]
        .as_i64()
        .unwrap();
    run(
        &mut e,
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 600, "height": 600, "fill_color": c, "area_fill": 20}),
    );
    let dir = std::env::temp_dir().join(format!("rustfig-e4-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("c.fig");
    run(&mut e, "file.save", json!({"path": path.to_string_lossy()}));
    let mut f = Engine::new();
    run(&mut f, "file.open", json!({"path": path.to_string_lossy()}));
    assert_eq!(f.doc().colors.len(), 1);
    assert_eq!(f.doc().colors[0].rgb.to_hex(), "#336699");
    std::fs::remove_dir_all(&dir).ok();
}
