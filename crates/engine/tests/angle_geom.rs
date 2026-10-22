#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Angle Geom (plan D7): each mode maps the pointer to xfig's constrained
//! point while drawing lines, polygons and splines and while moving or
//! adding the end point of an open line.

use rustfig_engine::geom::Point;
use rustfig_engine::model::{ObjectId, ObjectKind};
use rustfig_engine::{AngleGeom, Button, Engine, SnapMode, ToolEvent, ToolKind};
use serde_json::json;

fn engine(mode: AngleGeom) -> Engine {
    let mut e = Engine::new();
    e.indicator.snap = SnapMode::Any;
    e.execute("indicator.set", &json!({"angle_geom": mode})).unwrap();
    e
}

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

fn release(e: &mut Engine, x: f64, y: f64, button: Button) {
    e.tool_event(ToolEvent::Release {
        pos: Point::new(x, y),
        button,
    })
    .unwrap();
}

/// Draw with `tool`: left clicks on all but the last point, a middle click
/// on the last one.
fn draw(e: &mut Engine, tool: ToolKind, pts: &[(f64, f64)]) -> Vec<Point> {
    e.set_tool(tool);
    for (i, (x, y)) in pts.iter().enumerate() {
        mv(e, *x, *y);
        let b = if i + 1 == pts.len() {
            Button::Middle
        } else {
            Button::Left
        };
        press(e, *x, *y, b);
        release(e, *x, *y, b);
    }
    let o = e.doc().objects.last().unwrap();
    o.points().unwrap().to_vec()
}

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

#[test]
fn unconstrained_keeps_the_pointer() {
    let mut e = engine(AngleGeom::Unconstrained);
    let pts = draw(
        &mut e,
        ToolKind::Polyline,
        &[(0.0, 0.0), (500.0, 130.0), (800.0, 900.0)],
    );
    assert_eq!(pts, [p(0.0, 0.0), p(500.0, 130.0), p(800.0, 900.0)]);
}

#[test]
fn manhattan_polyline() {
    let mut e = engine(AngleGeom::Manhattan);
    let pts = draw(
        &mut e,
        ToolKind::Polyline,
        &[(0.0, 0.0), (500.0, 130.0), (530.0, 900.0)],
    );
    assert_eq!(pts, [p(0.0, 0.0), p(500.0, 0.0), p(500.0, 900.0)]);
}

#[test]
fn mountain_polyline() {
    let mut e = engine(AngleGeom::Mountain);
    let pts = draw(
        &mut e,
        ToolKind::Polyline,
        &[(0.0, 0.0), (500.0, -300.0), (900.0, -600.0)],
    );
    // 45 degrees up from the origin, and on up along it.
    assert_eq!(pts, [p(0.0, 0.0), p(500.0, -500.0), p(900.0, -900.0)]);
    // Pointing down: the 135 degree line.
    let mut e = engine(AngleGeom::Mountain);
    let pts = draw(
        &mut e,
        ToolKind::Polyline,
        &[(0.0, 0.0), (500.0, -300.0), (900.0, 100.0)],
    );
    assert_eq!(pts, [p(0.0, 0.0), p(500.0, -500.0), p(1100.0, 100.0)]);
}

#[test]
fn mounthattan_polygon() {
    let mut e = engine(AngleGeom::MountHattan);
    let pts = draw(
        &mut e,
        ToolKind::Polygon,
        &[(0.0, 0.0), (600.0, 100.0), (700.0, 650.0), (100.0, 80.0)],
    );
    assert_eq!(pts, [p(0.0, 0.0), p(600.0, 0.0), p(600.0, 650.0), p(30.0, 80.0)]);
}

#[test]
fn latex_line_spline() {
    let mut e = engine(AngleGeom::LatexLine);
    // Fine (1/16 inch = 75 units) positioning: the pointer snaps to (450, 225), on the 1:2 slope.
    e.indicator.snap = SnapMode::Fine;
    let pts = draw(&mut e, ToolKind::SplineApprox, &[(0.0, 0.0), (440.0, 230.0)]);
    assert_eq!(pts, [p(0.0, 0.0), p(450.0, 225.0)]);
}

#[test]
fn latex_arrow_has_only_arrow_slopes() {
    let mut e = engine(AngleGeom::LatexArrow);
    // Free positioning: 1:5 is no arrow slope, 1:4 is.
    let pts = draw(&mut e, ToolKind::Polyline, &[(0.0, 0.0), (500.0, 100.0)]);
    assert_eq!(pts, [p(0.0, 0.0), p(500.0, 125.0)]);
}

#[test]
fn rubber_band_shows_the_constrained_point() {
    let mut e = engine(AngleGeom::Manhattan);
    e.set_tool(ToolKind::Polyline);
    press(&mut e, 0.0, 0.0, Button::Left);
    mv(&mut e, 400.0, 90.0);
    press(&mut e, 400.0, 90.0, Button::Left);
    mv(&mut e, 410.0, 600.0);
    press(&mut e, 410.0, 600.0, Button::Middle);
    let o = e.doc().objects.last().unwrap();
    assert_eq!(o.points().unwrap(), [p(0.0, 0.0), p(400.0, 0.0), p(400.0, 600.0)]);
}

#[test]
fn freehand_is_not_constrained() {
    let mut e = engine(AngleGeom::Manhattan);
    e.set_tool(ToolKind::Polyline);
    press(&mut e, 0.0, 0.0, Button::Middle);
    mv(&mut e, 300.0, 200.0);
    mv(&mut e, 600.0, 300.0);
    press(&mut e, 600.0, 300.0, Button::Middle);
    let pts = e.doc().objects.last().unwrap().points().unwrap().to_vec();
    assert!(pts.contains(&p(300.0, 200.0)), "{pts:?}");
    assert_eq!(pts.last(), Some(&p(600.0, 300.0)));
}

fn line_with_three_points(e: &mut Engine) -> ObjectId {
    let id = e
        .execute("shape.polyline", &json!({"points": [[0, 0], [600, 0], [1200, 300]]}))
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    ObjectId(id)
}

fn pts_of(e: &Engine, id: ObjectId) -> Vec<Point> {
    match &e.doc().get(id).unwrap().kind {
        ObjectKind::Polyline(pl) => pl.points.clone(),
        _ => panic!("not a polyline"),
    }
}

#[test]
fn move_point_constrains_end_points_only() {
    let mut e = engine(AngleGeom::Manhattan);
    let id = line_with_three_points(&mut e);
    e.set_tool(ToolKind::MovePoint);
    // The end point, dragged: kept horizontal or vertical from its neighbour.
    press(&mut e, 1200.0, 300.0, Button::Left);
    mv(&mut e, 1500.0, 80.0);
    mv(&mut e, 1500.0, 700.0);
    release(&mut e, 1500.0, 700.0, Button::Left);
    assert_eq!(pts_of(&e, id), [p(0.0, 0.0), p(600.0, 0.0), p(1500.0, 0.0)]);
    // A point in the middle has two neighbours: free.
    press(&mut e, 600.0, 0.0, Button::Left);
    mv(&mut e, 700.0, 250.0);
    release(&mut e, 700.0, 250.0, Button::Left);
    assert_eq!(pts_of(&e, id), [p(0.0, 0.0), p(700.0, 250.0), p(1500.0, 0.0)]);
}

#[test]
fn move_point_preview_shows_the_constrained_point() {
    let mut e = engine(AngleGeom::Mountain);
    let _id = line_with_three_points(&mut e);
    e.set_tool(ToolKind::MovePoint);
    press(&mut e, 0.0, 0.0, Button::Left);
    mv(&mut e, -200.0, 900.0);
    let ov = e.overlay();
    let o = ov.preview.first().unwrap();
    // From the neighbour (600, 0): 135 degrees, down and to the left.
    assert_eq!(o.points().unwrap()[0], p(600.0 - 900.0, 900.0));
}

#[test]
fn add_point_at_the_end_is_constrained() {
    let mut e = engine(AngleGeom::Manhattan);
    let id = line_with_three_points(&mut e);
    e.set_tool(ToolKind::AddPoint);
    // The middle button adds beyond the nearer end (here the last point).
    press(&mut e, 1190.0, 290.0, Button::Middle);
    mv(&mut e, 1700.0, 250.0);
    mv(&mut e, 1800.0, 400.0);
    release(&mut e, 1800.0, 400.0, Button::Middle);
    assert_eq!(
        pts_of(&e, id),
        [p(0.0, 0.0), p(600.0, 0.0), p(1200.0, 300.0), p(1800.0, 300.0)]
    );
}

#[test]
fn indicator_roundtrip_and_hostile_values() {
    let mut e = Engine::new();
    for m in AngleGeom::ALL {
        e.execute("indicator.set", &json!({"angle_geom": m})).unwrap();
        assert_eq!(e.indicator.angle_geom, m);
    }
    e.execute("indicator.set", &json!({"angle_geom": "latex_arrow"}))
        .unwrap();
    for bad in [json!("wavy"), json!(3), json!(null), json!([]), json!({})] {
        let before = e.indicator.clone();
        assert!(e.execute("indicator.set", &json!({ "angle_geom": bad })).is_err());
        assert_eq!(e.indicator, before);
    }
    e.execute("indicator.reset", &json!({})).unwrap();
    assert_eq!(e.indicator.angle_geom, AngleGeom::Unconstrained);
}

#[test]
fn hostile_pointer_positions_under_every_mode() {
    for mode in AngleGeom::ALL {
        let mut e = engine(mode);
        e.indicator.snap = SnapMode::Fine;
        for tool in [
            ToolKind::Polyline,
            ToolKind::Polygon,
            ToolKind::XSpline,
            ToolKind::MovePoint,
        ] {
            e.set_tool(tool);
            for v in [f64::NAN, f64::INFINITY, 1e300, -1e300, 0.0, 1e9, -1e9 - 1.0] {
                for (x, y) in [(v, 0.0), (0.0, v), (v, v), (5.0, 5.0)] {
                    for b in [Button::Left, Button::Middle] {
                        let _ = e.tool_event(ToolEvent::Move { pos: Point::new(x, y) });
                        let _ = e.tool_event(ToolEvent::Press {
                            pos: Point::new(x, y),
                            button: b,
                            shift: false,
                        });
                        let _ = e.overlay();
                    }
                }
            }
        }
    }
}
