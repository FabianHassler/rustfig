#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! W10-23, W10-31, W10-32, W10-33: the Global settings rows with an effect,
//! the Arrow Size panel's modes, Integer zoom, and the indicator choices
//! xfig has beyond RustFig's first set (seven align choices, isometric grids).

use rustfig_engine::geom::Point;
use rustfig_engine::model::ObjectKind;
use rustfig_engine::{Button, Engine, ToolEvent, ToolKind};
use serde_json::json;

fn rect(e: &mut Engine, x: f64, y: f64, w: f64, h: f64) -> u64 {
    let r = e
        .execute("shape.rectangle", &json!({"x": x, "y": y, "width": w, "height": h}))
        .unwrap();
    r["id"].as_u64().unwrap()
}

fn bbox(e: &Engine, id: u64) -> (f64, f64, f64, f64) {
    let o = e.doc().objects.iter().find(|o| o.id.0 == id).unwrap();
    let b = o.geometry_bbox();
    (b.min.x, b.min.y, b.max.x, b.max.y)
}

fn pref(e: &mut Engine, name: &str, value: serde_json::Value) {
    e.execute("prefs.set", &json!({"resource": name, "value": value}))
        .unwrap();
}

// --- W10-33: align ---

fn three(e: &mut Engine) -> [u64; 3] {
    [
        rect(e, 0.0, 0.0, 100.0, 100.0),
        rect(e, 150.0, 0.0, 300.0, 100.0),
        rect(e, 1000.0, 0.0, 200.0, 100.0),
    ]
}

#[test]
fn align_distribute_centres_spaces_the_centres_evenly() {
    let mut e = Engine::new();
    let ids = three(&mut e);
    e.execute(
        "arrange.align",
        &json!({"ids": ids, "h": "distribute_centres", "v": "none"}),
    )
    .unwrap();
    let c: Vec<f64> = ids
        .iter()
        .map(|i| {
            let b = bbox(&e, *i);
            (b.0 + b.2) / 2.0
        })
        .collect();
    // The outer two stay, the middle one sits half way between them.
    assert!((c[0] - 50.0).abs() < 1e-6 && (c[2] - 1100.0).abs() < 1e-6);
    assert!((c[1] - 575.0).abs() < 1e-6, "{c:?}");
}

#[test]
fn align_distribute_edges_makes_equal_gaps() {
    let mut e = Engine::new();
    let ids = three(&mut e);
    e.execute("arrange.align", &json!({"ids": ids, "h": "distribute_edges"}))
        .unwrap();
    let (a, b, c) = (bbox(&e, ids[0]), bbox(&e, ids[1]), bbox(&e, ids[2]));
    assert!(((b.0 - a.2) - (c.0 - b.2)).abs() < 1e-6);
    assert!((a.0).abs() < 1e-6 && (c.2 - 1200.0).abs() < 1e-6);
}

#[test]
fn align_abut_puts_the_objects_edge_to_edge() {
    let mut e = Engine::new();
    let ids = three(&mut e);
    e.execute("arrange.align", &json!({"ids": ids, "h": "abut"})).unwrap();
    let (a, b, c) = (bbox(&e, ids[0]), bbox(&e, ids[1]), bbox(&e, ids[2]));
    assert!(
        (b.0 - a.2).abs() < 1e-6 && (c.0 - b.2).abs() < 1e-6,
        "{a:?} {b:?} {c:?}"
    );
}

#[test]
fn align_indicator_takes_all_seven_choices() {
    let mut e = Engine::new();
    for h in [
        "none",
        "left",
        "center",
        "right",
        "distribute_centres",
        "distribute_edges",
        "abut",
    ] {
        e.execute("indicator.set", &json!({"align_h": h})).unwrap();
        assert_eq!(e.indicator.align_h, h);
    }
    // Against the canvas distributing and abutting do nothing (xfig says so).
    let id = rect(&mut e, 100.0, 100.0, 100.0, 100.0);
    e.execute("arrange.align_canvas", &json!({"h": "abut", "v": "distribute_edges"}))
        .unwrap();
    assert_eq!(bbox(&e, id).0, 100.0);
}

// --- W10-33: isometric grids ---

#[test]
fn isometric_grid_snaps_to_the_triangular_lattice() {
    let mut e = Engine::new();
    e.execute(
        "indicator.set",
        &json!({"snap": "medium", "grid": "medium", "grid_iso": true}),
    )
    .unwrap();
    let s = 300.0_f64;
    let c30 = 0.75_f64.sqrt();
    let p = e.snap(Point::new(s * c30 * 2.0 + 20.0, 40.0));
    // Even column: y on multiples of the spacing.
    assert!((p.x - (s * c30 * 2.0).round()).abs() <= 1.0, "{p:?}");
    assert!((p.y % s).abs() < 1.0, "{p:?}");
    // Odd column: y on the half spacing.
    let q = e.snap(Point::new(s * c30 + 10.0, 10.0));
    assert!((q.y.rem_euclid(s) - s / 2.0).abs() < 1.0, "{q:?}");
    // Square grid again.
    e.execute("indicator.set", &json!({"grid_iso": false})).unwrap();
    assert_eq!(e.snap(Point::new(310.0, 40.0)), Point::new(300.0, 0.0));
}

#[test]
fn start_grid_mode_has_nine_choices() {
    let mut e = Engine::new();
    pref(&mut e, "startgridmode", json!(7));
    assert!(e.indicator.grid_iso && e.indicator.grid.is_some());
    pref(&mut e, "startgridmode", json!(2));
    assert!(!e.indicator.grid_iso && e.indicator.grid.is_some());
    pref(&mut e, "startgridmode", json!(0));
    assert!(e.indicator.grid.is_none() && !e.indicator.grid_iso);
    e.execute("indicator.set", &json!({"grid": "large", "grid_iso": true}))
        .unwrap();
    e.execute("prefs.use_current_attributes", &json!({})).unwrap();
    assert_eq!(e.prefs.int("startgridmode"), Some(7));
}

// --- W10-31: arrow size ---

fn arrow_of_new_line(e: &mut Engine) -> rustfig_engine::model::Arrow {
    e.execute("indicator.set", &json!({"forward_arrow": true})).unwrap();
    e.execute("shape.polyline", &json!({"points": [[0, 0], [1200, 0]]}))
        .unwrap();
    let o = e.doc().objects.last().unwrap();
    match &o.kind {
        ObjectKind::Polyline(l) => l.forward_arrow.unwrap(),
        _ => panic!("not a line"),
    }
}

#[test]
fn arrow_size_multiple_of_line_width_is_the_start_state() {
    let mut e = Engine::new();
    e.execute(
        "indicator.set",
        &json!({"thickness": 3, "arrow": {"kind": 1, "style": 1, "thickness": 1.0, "width": 60.0, "height": 120.0}}),
    )
    .unwrap();
    let a = arrow_of_new_line(&mut e);
    // xfig: 1, 4, 8 times the line width (in 1/80 inch, 15 Fig units each).
    assert_eq!(
        (a.thickness, a.width, a.height),
        (3.0, 4.0 * 3.0 * 15.0, 8.0 * 3.0 * 15.0)
    );
}

#[test]
fn arrow_size_absolute_and_multiple_values_are_both_settable() {
    let mut e = Engine::new();
    e.execute(
        "indicator.set",
        &json!({"thickness": 2, "arrow_absolute": true,
                "arrow": {"kind": 1, "style": 1, "thickness": 2.5, "width": 150.0, "height": 300.0}}),
    )
    .unwrap();
    let a = arrow_of_new_line(&mut e);
    assert_eq!((a.thickness, a.width, a.height), (2.5, 150.0, 300.0));
    e.execute(
        "indicator.set",
        &json!({"arrow_absolute": false, "arrow_mult": [2.0, 5.0, 6.0]}),
    )
    .unwrap();
    let b = arrow_of_new_line(&mut e);
    assert_eq!(
        (b.thickness, b.width, b.height),
        (4.0, 5.0 * 2.0 * 15.0, 6.0 * 2.0 * 15.0)
    );
    // Out of range values are brought into 0.1 to 10000.
    e.execute("indicator.set", &json!({"arrow_mult": [0.0, 20000.0, 1.0]}))
        .unwrap();
    assert_eq!(e.indicator.arrow_mult, (0.1, 10_000.0, 1.0));
}

#[test]
fn arrow_size_settings_are_kept_in_the_preferences() {
    let mut e = Engine::new();
    e.execute(
        "indicator.set",
        &json!({"arrow_absolute": true, "arrow_mult": [2.0, 3.0, 4.0]}),
    )
    .unwrap();
    e.execute("prefs.use_current_attributes", &json!({})).unwrap();
    let text = e.prefs.to_toml();
    let (p, _) = rustfig_engine::prefs::Prefs::parse(&text).unwrap();
    let mut e2 = Engine::new();
    p.apply(&mut e2);
    assert!(e2.indicator.arrow_absolute);
    assert_eq!(e2.indicator.arrow_mult, (2.0, 3.0, 4.0));
}

// --- W10-32: integer zoom ---

#[test]
fn integer_zoom_rounds_set_and_truncates_fit() {
    let mut e = Engine::new();
    e.execute("view.zoom_set", &json!({"zoom": 2.6})).unwrap();
    assert!((e.view.zoom - 2.6).abs() < 1e-9);
    e.execute("view.set_integer_zoom", &json!({"on": true})).unwrap();
    e.execute("view.zoom_set", &json!({"zoom": 2.6})).unwrap();
    assert_eq!(e.view.zoom, 3.0);
    e.execute("view.zoom_set", &json!({"zoom": 1.5})).unwrap();
    assert_eq!(e.view.zoom, 1.5, "below 1.99 xfig keeps the fraction");
    rect(&mut e, 0.0, 0.0, 300.0, 300.0);
    e.execute("view.zoom_fit", &json!({})).unwrap();
    assert_eq!(e.view.zoom.fract(), 0.0);
    e.execute("view.set_integer_zoom", &json!({})).unwrap();
    assert!(!e.view.integer_zoom, "toggles when omitted");
}

// --- W10-23: Global settings rows ---

#[test]
fn recent_files_count_follows_the_setting() {
    let mut e = Engine::new();
    for i in 0..9 {
        e.prefs.add_recent(&format!("/tmp/f{i}.fig"));
    }
    assert_eq!(e.prefs.rustfig.recent_files.len(), 9);
    pref(&mut e, "max_recent_files", json!(3));
    assert_eq!(e.prefs.rustfig.recent_files.len(), 3);
    e.prefs.add_recent("/tmp/new.fig");
    assert_eq!(e.prefs.rustfig.recent_files.len(), 3);
    assert_eq!(e.prefs.rustfig.recent_files[0], "/tmp/new.fig");
    pref(&mut e, "max_recent_files", json!(0));
    assert!(e.prefs.rustfig.recent_files.is_empty());
    pref(&mut e, "max_recent_files", json!(50));
    for i in 0..20 {
        e.prefs.add_recent(&format!("/tmp/g{i}.fig"));
    }
    assert_eq!(e.prefs.rustfig.recent_files.len(), 9, "never more than 9");
}

#[test]
fn negative_coordinates_can_be_switched_off() {
    let mut e = Engine::new();
    assert_eq!(e.snap(Point::new(-300.0, -300.0)), Point::new(-300.0, -300.0));
    pref(&mut e, "allownegcoords", json!(false));
    assert_eq!(e.snap(Point::new(-300.0, 450.0)), Point::new(0.0, 450.0));
    // The view moves to the origin when it was left of it.
    let mut e = Engine::new();
    assert!(e.view.origin.x < 0.0);
    pref(&mut e, "allownegcoords", json!(false));
    assert!(e.view.origin.x >= 0.0 && e.view.origin.y >= 0.0);
}

#[test]
fn freehand_resolution_sets_the_point_spacing() {
    let count = |res: i64| {
        let mut e = Engine::new();
        pref(&mut e, "freehand_resolution", json!(res));
        e.execute("indicator.set", &json!({"snap": "any"})).unwrap();
        e.set_tool(ToolKind::Polyline);
        let press = |e: &mut Engine, x: f64, button| {
            e.tool_event(ToolEvent::Press {
                pos: Point::new(x, 0.0),
                button,
                shift: false,
            })
            .unwrap();
        };
        press(&mut e, 0.0, Button::Middle);
        for i in 1..=100 {
            e.tool_event(ToolEvent::Move {
                pos: Point::new(f64::from(i) * 10.0, 0.0),
            })
            .unwrap();
        }
        press(&mut e, 1010.0, Button::Middle);
        e.doc().objects[0].points().unwrap().len()
    };
    let fine = count(10);
    let coarse = count(200);
    assert!(fine > coarse * 3, "{fine} vs {coarse}");
}

#[test]
fn vertex_numbers_axis_lines_and_tracking_are_preferences() {
    let mut e = Engine::new();
    e.execute(
        "shape.polyline",
        &json!({"points": [[0, 0], [100, 0], [100, 100], [0, 100]]}),
    )
    .unwrap();
    assert!(!e.show_vertex_numbers());
    e.execute("view.toggle_vertex_numbers", &json!({})).unwrap();
    assert!(e.show_vertex_numbers());
    assert_eq!(e.vertex_numbers().len(), 4);
    assert_eq!(e.prefs.bool("shownums"), Some(true), "kept in the preferences");
    pref(&mut e, "showaxislines", json!(false));
    pref(&mut e, "trackCursor", json!(false));
    assert!(!e.show_axis_lines() && !e.track_mouse());
    let text = e.prefs.to_toml();
    let (p, _) = rustfig_engine::prefs::Prefs::parse(&text).unwrap();
    assert_eq!(p.bool("showaxislines"), Some(false));
    assert_eq!(p.bool("trackCursor"), Some(false));
}

#[test]
fn helper_commands_are_resources() {
    let mut e = Engine::new();
    for (name, v) in [
        ("image_editor", "gimp %f"),
        ("spellcheckcommand", "aspell check %f"),
        ("browser", "firefox %f"),
        ("pdf_viewer", "evince %f"),
    ] {
        pref(&mut e, name, json!(v));
        assert_eq!(e.prefs.str(name).as_deref(), Some(v));
    }
    pref(&mut e, "max_image_colors", json!(64));
    assert_eq!(e.prefs.int("max_image_colors"), Some(64));
}
