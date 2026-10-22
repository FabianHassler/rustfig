#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Ctrl+click on the canvas: the shape factor of a spline point (plan S4-6,
//! xfig's `change_sfactor`) and the zoom box (plan E3, xfig's `zoom_selected`).

use rustfig_engine::geom::Point;
use rustfig_engine::model::{ObjectKind, SplineKind};
use rustfig_engine::{Button, Engine, ToolEvent, ToolKind};
use serde_json::json;

fn spline_engine(kind: &str, closed: bool) -> Engine {
    let mut e = Engine::new();
    e.execute(
        "shape.spline",
        &json!({"points": [[0, 0], [1200, 1200], [2400, 0], [3600, 1200]], "kind": kind, "closed": closed}),
    )
    .unwrap();
    e.set_tool(ToolKind::Move);
    e
}

fn shapes(e: &Engine) -> (SplineKind, Vec<f64>) {
    match &e.doc().objects[0].kind {
        ObjectKind::Spline(s) => (s.kind, s.shapes.clone()),
        _ => panic!("not a spline"),
    }
}

fn ctrl(e: &mut Engine, x: f64, y: f64, button: Button, shift: bool) -> Result<(), rustfig_engine::EngineError> {
    e.tool_event(ToolEvent::Ctrl {
        pos: Point::new(x, y),
        button,
        shift,
    })
}

#[test]
fn left_is_more_approximating_and_makes_an_xspline() {
    let mut e = spline_engine("interpolated", false);
    assert_eq!(shapes(&e).0, SplineKind::Interpolated);
    ctrl(&mut e, 1200.0, 1200.0, Button::Left, true).unwrap();
    let (kind, s) = shapes(&e);
    assert_eq!(kind, SplineKind::X);
    assert!((s[1] - (-1.0 + 0.04)).abs() < 1e-9, "{s:?}");
    assert!((s[2] - -1.0).abs() < 1e-9);
    assert!(e.undo());
    assert_eq!(shapes(&e).0, SplineKind::Interpolated);
}

#[test]
fn right_is_more_interpolating_and_stops_at_minus_one() {
    let mut e = spline_engine("approximated", false);
    ctrl(&mut e, 1200.0, 1200.0, Button::Right, true).unwrap();
    assert!((shapes(&e).1[1] - 0.96).abs() < 1e-9);
    for _ in 0..60 {
        ctrl(&mut e, 1200.0, 1200.0, Button::Right, true).unwrap();
    }
    assert!((shapes(&e).1[1] - -1.0).abs() < 1e-9);
}

#[test]
fn left_stops_at_one() {
    let mut e = spline_engine("approximated", true);
    ctrl(&mut e, 0.0, 0.0, Button::Left, true).unwrap();
    assert!((shapes(&e).1[0] - 1.0).abs() < 1e-9);
}

#[test]
fn middle_cycles_the_shapes() {
    let mut e = spline_engine("approximated", false);
    let mut seen = Vec::new();
    for _ in 0..4 {
        ctrl(&mut e, 2400.0, 0.0, Button::Middle, true).unwrap();
        seen.push(shapes(&e).1[2]);
    }
    assert_eq!(seen, vec![-1.0, 0.0, 1.0, -1.0]);
    assert_eq!(shapes(&e).0, SplineKind::X);
}

#[test]
fn end_points_of_an_open_spline_are_refused() {
    let mut e = spline_engine("approximated", false);
    ctrl(&mut e, 0.0, 0.0, Button::Left, true).unwrap();
    assert_eq!(e.status(), "Cannot edit end-points");
    assert_eq!(shapes(&e).0, SplineKind::Approximated);
    ctrl(&mut e, 3600.0, 1200.0, Button::Right, true).unwrap();
    assert_eq!(shapes(&e).0, SplineKind::Approximated);
}

#[test]
fn a_click_off_the_splines_says_so() {
    let mut e = spline_engine("approximated", false);
    ctrl(&mut e, 900.0, 5000.0, Button::Left, true).unwrap();
    assert_eq!(e.status(), "Only spline points can be edited");
}

#[test]
fn drawing_modes_ignore_ctrl_shift() {
    let mut e = spline_engine("approximated", false);
    e.set_tool(ToolKind::Box);
    ctrl(&mut e, 1200.0, 1200.0, Button::Left, true).unwrap();
    assert_eq!(shapes(&e).0, SplineKind::Approximated);
}

#[test]
fn indicator_shows_the_ctrl_functions() {
    let mut e = spline_engine("approximated", false);
    let h = e.mouse_hints_for(true, true);
    assert_eq!(
        (h.left, h.middle, h.right),
        ("More approx", "Cycle shapes", "More interp")
    );
    let h = e.mouse_hints_for(false, true);
    assert_eq!((h.left, h.middle, h.right), ("Zoom area", "Pan to origin", "Unzoom"));
    e.set_tool(ToolKind::Box);
    // xfig: Ctrl+Shift in a drawing mode shows the Shift functions.
    assert_eq!(e.mouse_hints_for(true, true), e.mouse_hints_with(true));
}

#[test]
fn zoom_command_fits_a_box() {
    let mut e = Engine::new();
    e.view.size = (800.0, 400.0);
    e.execute("view.zoom_box", &json!({"x1": 2400, "y1": 1200, "x2": 1200, "y2": 0}))
        .unwrap();
    // 1200 x 1200 Fig units into 800 x 400 px: height limits, 400/1200 px per unit.
    assert!((e.view.px_per_fig() - 400.0 / 1200.0).abs() < 1e-9);
    assert!((e.view.origin.x - 1200.0).abs() < 1e-9 && e.view.origin.y.abs() < 1e-9);
    // A degenerate box changes nothing.
    let before = e.view;
    e.execute("view.zoom_box", &json!({"x1": 5, "y1": 5, "x2": 5, "y2": 900}))
        .unwrap();
    assert_eq!(e.view, before);
}

#[test]
fn ctrl_drag_zooms_into_the_box() {
    let mut e = Engine::new();
    e.view.size = (800.0, 600.0);
    ctrl(&mut e, 0.0, 0.0, Button::Left, false).unwrap();
    assert_eq!(e.mouse_hints().left, "final point");
    e.tool_event(ToolEvent::Move {
        pos: Point::new(1200.0, 900.0),
    })
    .unwrap();
    // The rubber box is in the overlay.
    assert_eq!(e.overlay().guides.len(), 1);
    e.tool_event(ToolEvent::Release {
        pos: Point::new(1200.0, 900.0),
        button: Button::Left,
    })
    .unwrap();
    assert!(!e.tool_busy());
    assert!((e.view.px_per_fig() - 600.0 / 900.0).abs() < 1e-9);
    assert!(e.overlay().guides.is_empty());
}

#[test]
fn ctrl_click_click_zooms_and_right_cancels() {
    let mut e = Engine::new();
    e.view.size = (800.0, 600.0);
    let before = e.view;
    ctrl(&mut e, 0.0, 0.0, Button::Left, false).unwrap();
    e.tool_event(ToolEvent::Release {
        pos: Point::new(0.0, 0.0),
        button: Button::Left,
    })
    .unwrap();
    assert!(e.tool_busy(), "a click leaves the box open");
    e.tool_event(ToolEvent::Press {
        pos: Point::new(600.0, 450.0),
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    assert!(e.view.zoom > before.zoom);
    // Right button cancels a started box.
    let z = e.view;
    ctrl(&mut e, 0.0, 0.0, Button::Left, false).unwrap();
    e.tool_event(ToolEvent::Press {
        pos: Point::new(10.0, 10.0),
        button: Button::Right,
        shift: false,
    })
    .unwrap();
    assert!(!e.tool_busy());
    assert_eq!(e.view, z);
}

#[test]
fn ctrl_right_unzooms_and_middle_pans_to_origin() {
    let mut e = Engine::new();
    e.view.zoom = 4.0;
    e.view.origin = Point::new(500.0, 500.0);
    ctrl(&mut e, 0.0, 0.0, Button::Middle, false).unwrap();
    assert!(e.view.origin.x < 0.0 && e.view.origin.y < 0.0 && e.view.origin.x > -200.0);
    ctrl(&mut e, 0.0, 0.0, Button::Right, false).unwrap();
    assert_eq!(e.view.zoom, 1.0);
}

#[test]
fn no_zoom_box_while_a_tool_is_busy() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::Box);
    e.tool_event(ToolEvent::Press {
        pos: Point::new(0.0, 0.0),
        button: Button::Left,
        shift: false,
    })
    .unwrap();
    ctrl(&mut e, 100.0, 100.0, Button::Left, false).unwrap();
    assert!(e.overlay().guides.len() <= 1);
}
