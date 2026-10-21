//! xfig's object edit panel as commands (W10 group 3): geometry rows,
//! text flags, arc type, arrow thickness, comments of a compound.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::Engine;
use rustfig_model::{ObjectKind, SplineKind};
use serde_json::json;

const FIG: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 600 0 600 600 0 600 0 0\n\
1 3 0 1 0 7 50 -1 -1 0.000 1 0.0000 3000 3000 500 500 3000 3000 3500 3500\n\
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 3\n\t 0 0 100 100 200 0\n\
5 1 0 1 0 7 50 -1 -1 0.000 0 1 0 0 1000 0 1500 0 1000 500 500 0\n\
3 2 0 1 0 7 50 -1 -1 0.000 0 0 0 4\n\t 0 400 100 500 200 400 300 500\n\t 0.0 -1.0 -1.0 0.0\n\
4 0 0 50 -1 0 12 0.0000 4 150 600 5000 5000 Hi\\001\n\
6 0 0 600 600\n2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 600 0 600 600 0 600 0 0\n-6\n";

fn engine() -> Engine {
    let mut e = Engine::new();
    e.execute("file.open_data", &json!({"data": FIG})).unwrap();
    e
}

fn obj<'a>(e: &'a Engine, ty: &str) -> &'a rustfig_model::Object {
    e.doc()
        .objects
        .iter()
        .find(|o| o.type_name() == ty)
        .map(|o| &**o)
        .unwrap()
}

fn id_of(e: &Engine, ty: &str) -> u64 {
    obj(e, ty).id.0
}

#[test]
fn box_corners_are_set_in_xfigs_vertex_order() {
    let mut e = engine();
    let id = id_of(&e, "box");
    e.execute(
        "object.set_geometry",
        &json!({"id": id, "points": [[100, 200], [700, 900]]}),
    )
    .unwrap();
    let ObjectKind::Polyline(p) = &obj(&e, "box").kind else {
        panic!()
    };
    let v: Vec<(f64, f64)> = p.points.iter().map(|q| (q.x, q.y)).collect();
    assert_eq!(v, vec![(100.0, 200.0), (700.0, 200.0), (700.0, 900.0), (100.0, 900.0)]);
}

#[test]
fn circle_by_diameter_keeps_start_and_end_like_xfig() {
    let mut e = engine();
    let id = id_of(&e, "circle");
    e.execute(
        "object.set_geometry",
        &json!({"id": id, "points": [[1000, 1000]], "radii": [300, 999]}),
    )
    .unwrap();
    let ObjectKind::Ellipse(el) = &obj(&e, "circle").kind else {
        panic!()
    };
    assert_eq!((el.center.x, el.radii.x, el.radii.y), (1000.0, 300.0, 300.0));
    assert_eq!((el.end.x, el.end.y), (1300.0, 1000.0));
}

#[test]
fn polyline_point_count_cannot_change_and_failure_changes_nothing() {
    let mut e = engine();
    let id = id_of(&e, "polyline");
    let before = obj(&e, "polyline").clone();
    assert!(e
        .execute("object.set_geometry", &json!({"id": id, "points": [[0, 0], [1, 1]]}))
        .is_err());
    assert_eq!(*obj(&e, "polyline"), before);
    e.execute(
        "object.set_geometry",
        &json!({"id": id, "points": [[0, 0], [5, 5], [9, 9]]}),
    )
    .unwrap();
    let ObjectKind::Polyline(p) = &obj(&e, "polyline").kind else {
        panic!()
    };
    assert_eq!(p.points[2].x, 9.0);
}

#[test]
fn collinear_arc_points_are_refused() {
    let mut e = engine();
    let id = id_of(&e, "arc");
    assert!(e
        .execute(
            "object.set_geometry",
            &json!({"id": id, "points": [[0, 0], [1, 1], [2, 2]]})
        )
        .is_err());
    e.execute("object.set_attributes", &json!({"ids": [id], "pie": true}))
        .unwrap();
    let ObjectKind::Arc(a) = &obj(&e, "arc").kind else {
        panic!()
    };
    assert!(a.pie);
}

#[test]
fn spline_shape_factor_turns_an_interpolated_spline_into_an_xspline() {
    let mut e = engine();
    let id = id_of(&e, "spline");
    e.execute(
        "object.set_geometry",
        &json!({"id": id, "shapes": [0.0, 0.3, -1.0, 0.0]}),
    )
    .unwrap();
    let ObjectKind::Spline(s) = &obj(&e, "spline").kind else {
        panic!()
    };
    assert_eq!(s.kind, SplineKind::X);
    assert_eq!(s.shapes, vec![0.0, 0.3, -1.0, 0.0]);
}

#[test]
fn text_flags_origin_and_compound_comment() {
    let mut e = engine();
    let t = id_of(&e, "text");
    e.execute(
        "object.set_attributes",
        &json!({"ids": [t], "rigid": true, "special": true, "hidden": false}),
    )
    .unwrap();
    e.execute("object.set_geometry", &json!({"id": t, "points": [[111, 222]]}))
        .unwrap();
    let ObjectKind::Text(x) = &obj(&e, "text").kind else {
        panic!()
    };
    assert!(x.flags & 1 != 0 && x.flags & 2 != 0);
    assert_eq!((x.origin.x, x.origin.y), (111.0, 222.0));
    let c = id_of(&e, "compound");
    e.execute("object.set_attributes", &json!({"ids": [c], "comment": "group note"}))
        .unwrap();
    let o = obj(&e, "compound");
    assert_eq!(o.comments, vec!["group note".to_string()]);
    let ObjectKind::Compound(cc) = &o.kind else { panic!() };
    assert!(cc.objects[0].comments.is_empty());
}

#[test]
fn attributes_and_geometry_in_one_undo_step() {
    let mut e = engine();
    let id = id_of(&e, "box");
    let before = obj(&e, "box").clone();
    e.execute(
        "object.set_attributes",
        &json!({"ids": [id], "thickness": 7, "geometry": {"points": [[0, 0], [50, 50]]}}),
    )
    .unwrap();
    assert_ne!(*obj(&e, "box"), before);
    e.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(*obj(&e, "box"), before);
}

fn press(e: &mut Engine, x: f64, y: f64, button: &str) {
    e.execute(
        "tool.event",
        &json!({"type": "press", "x": x, "y": y, "button": button}),
    )
    .unwrap();
    e.execute(
        "tool.event",
        &json!({"type": "release", "x": x, "y": y, "button": button}),
    )
    .unwrap();
}

#[test]
fn edit_mode_buttons_follow_xfig() {
    use rustfig_engine::UiRequest;
    let mut e = engine();
    e.execute("tool.edit", &json!({})).unwrap();
    // Middle button: the whole figure's comments, wherever it is.
    press(&mut e, 9000.0, 9000.0, "middle");
    assert_eq!(e.take_requests(), vec![UiRequest::EditFigureComments]);
    // Right button on an interior spline point: its shape-factor panel.
    let sid = obj(&e, "spline").id;
    press(&mut e, 100.0, 500.0, "right");
    assert_eq!(
        e.take_requests(),
        vec![UiRequest::EditSplinePoint { id: sid, index: 1 }]
    );
    // On an end point or a non-spline point: a message, no panel.
    press(&mut e, 0.0, 400.0, "right");
    assert!(e.take_requests().is_empty());
    assert!(e.status().contains("end-points"));
    press(&mut e, 600.0, 600.0, "right");
    assert!(e.take_requests().is_empty());
    assert!(e.status().contains("Only spline"));
}

#[test]
fn degenerate_geometry_survives_save_and_open() {
    let mut e = engine();
    for ty in ["box", "polyline", "circle", "spline", "arc"] {
        let id = id_of(&e, ty);
        let pts = match ty {
            "polyline" => json!([[0, 0], [0, 0], [0, 0]]),
            "spline" => json!([[0, 0], [0, 0], [0, 0], [0, 0]]),
            "arc" => json!([[0, 0], [0, 0], [0, 0]]),
            "circle" => json!([[0, 0]]),
            _ => json!([[0, 0], [0, 0]]),
        };
        let _ = e.execute(
            "object.set_geometry",
            &json!({"id": id, "points": pts, "radii": [0, 0]}),
        );
    }
    let n = e.doc().objects.len();
    assert!(e
        .execute(
            "object.set_geometry",
            &json!({"id": id_of(&e, "box"), "points": [[5, 5], [5, 9]]})
        )
        .is_err());
    let data = e.execute("file.save_data", &json!({})).unwrap()["data"]
        .as_str()
        .unwrap()
        .to_string();
    let mut e2 = Engine::new();
    e2.execute("file.open_data", &json!({"data": data})).unwrap();
    assert_eq!(e2.doc().objects.len(), n);
}
