#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! F16: several curves in one plot, each with its own style and markers.

use rustfig_engine::Engine;
use rustfig_model::{Object, ObjectKind};
use serde_json::{json, Value};

fn plot(e: &Engine, id: u64) -> Object {
    let o = e.doc().objects.iter().find(|o| o.id.0 == id).unwrap();
    (**o).clone()
}

/// The generated parts of a plot with the given kind ("curve", "marker").
fn parts<'a>(o: &'a Object, kind: &str) -> Vec<&'a Object> {
    let ObjectKind::Compound(c) = &o.kind else {
        panic!("not a compound")
    };
    c.objects
        .iter()
        .filter(|m| {
            m.comments
                .iter()
                .any(|c| c.trim() == format!("rustfig-plot-part: {kind}"))
        })
        .collect()
}

fn pen(o: &Object) -> i32 {
    match &o.kind {
        ObjectKind::Polyline(p) => p.style.pen_color.0,
        ObjectKind::Ellipse(e) => e.style.pen_color.0,
        _ => panic!("unexpected part"),
    }
}

fn add(e: &mut Engine, p: Value) -> Value {
    e.execute("plot.add", &p).unwrap()
}

#[test]
fn curves_have_their_own_styles_and_share_the_ranges() {
    let mut e = Engine::new();
    let r = add(
        &mut e,
        json!({"xmin": 0, "xmax": 3, "curves": [
            {"expr": "x", "pen_color": 4, "line": "dashed", "thickness": 3},
            {"expr": "x^2", "pen_color": 1}
        ]}),
    );
    assert_eq!(r["warnings"], json!([]));
    let o = plot(&e, r["id"].as_u64().unwrap());
    let curves = parts(&o, "curve");
    assert_eq!(curves.len(), 2);
    let pens: Vec<i32> = curves.iter().map(|c| pen(c)).collect();
    assert_eq!(pens, vec![4, 1]);
    let ObjectKind::Polyline(first) = &curves[0].kind else {
        panic!()
    };
    assert_eq!(first.style.thickness, 3);
    assert_eq!(first.style.line, rustfig_model::LineKind::Dashed);
    // The y range fits both: x^2 reaches 9, so the line x stays in the lower part.
    let spec = e.execute("plot.get", &json!({"id": o.id.0})).unwrap();
    assert_eq!(spec["spec"]["curves"].as_array().unwrap().len(), 2);
}

#[test]
fn markers_alone_from_points_survive_save_and_update() {
    let mut e = Engine::new();
    let r = add(
        &mut e,
        json!({"xmin": 0, "xmax": 4, "ymin": 0, "ymax": 10, "curves": [
            {"data": "1 1\n2 4\n3 9\n", "marker": "circle", "line": "none", "pen_color": 4},
            {"points": [[1, 2], [2, 3]], "marker": "square", "marker_fill": false},
            {"expr": "x", "samples": 4, "marker": "plus"}
        ]}),
    );
    let id = r["id"].as_u64().unwrap();
    assert_eq!(r["markers"], 3 + 2 + 5);
    let o = plot(&e, id);
    assert_eq!(parts(&o, "curve").len(), 2, "the first curve has no line");
    // circles 3, squares 2, plus 2 strokes x 5
    assert_eq!(parts(&o, "marker").len(), 3 + 2 + 10);
    // The inline data is kept as points, on comment lines of their own.
    assert!(o
        .comments
        .iter()
        .any(|c| c.starts_with("rustfig-plot-points: 0 1 1 2 4 3 9")));

    // Through the file.
    let fig = e.execute("file.save_data", &json!({"format": "fig"})).unwrap();
    let mut e2 = Engine::new();
    e2.execute("file.open_data", &json!({"data": fig["data"]})).unwrap();
    let id2 = e2.doc().objects[0].id.0;
    // A wider range redraws from the kept points without the data again.
    let u = e2.execute("plot.update", &json!({"id": id2, "ymax": 20})).unwrap();
    assert_eq!(u["markers"], 10);
    assert_eq!(u["warnings"], json!([]));
    let o2 = plot(&e2, id2);
    assert_eq!(
        o2.comments
            .iter()
            .filter(|c| c.starts_with("rustfig-plot-points:"))
            .count(),
        2
    );
}

#[test]
fn the_plot_style_applies_where_a_curve_sets_none() {
    let mut e = Engine::new();
    let r = add(
        &mut e,
        json!({"expr": "sin(x)", "pen_color": 2, "thickness": 2, "depth": 120}),
    );
    assert_eq!(r["warnings"], json!([]));
    let o = plot(&e, r["id"].as_u64().unwrap());
    let c = parts(&o, "curve");
    assert!(!c.is_empty());
    assert!(c.iter().all(|c| pen(c) == 2));
    assert!(o.depth() == 120);
}

#[test]
fn a_curve_outside_the_ranges_is_named() {
    let mut e = Engine::new();
    let r = add(
        &mut e,
        json!({"xmin": 0, "xmax": 1, "ymin": 0, "ymax": 1, "curves": [{"expr": "x"}, {"expr": "x + 5"}]}),
    );
    assert_eq!(
        r["warnings"],
        json!(["curve 2: nothing of the curve lies inside the ranges"])
    );
}

#[test]
fn bad_curves_are_refused() {
    let mut e = Engine::new();
    for bad in [
        json!({"curves": [{"expr": "x", "line": "wavy"}]}),
        json!({"curves": [{"expr": "x", "pen_color": 100000}]}),
        json!({"curves": [{"expr": "x", "marker": "star"}]}),
        json!({"curves": [{"expr": "x", "marker_size": 0}]}),
        json!({"curves": [{"expr": ""}]}),
        json!({"curves": [{"points": [[1, "a"]]}]}),
        json!({"curves": [{"points": [[0, 1e300]]}]}),
        json!({"line": "wavy"}),
        json!({"depth": -3}),
    ] {
        let before = e.doc().objects.len();
        assert!(e.execute("plot.add", &bad).is_err(), "{bad}");
        assert_eq!(e.doc().objects.len(), before);
    }
    let many: Vec<Value> = (0..51).map(|_| json!({"expr": "x"})).collect();
    assert!(e.execute("plot.add", &json!({"curves": many})).is_err());
}

#[test]
fn one_curve_plots_are_unchanged() {
    let mut e = Engine::new();
    let r = add(&mut e, json!({"expr": "sin(x)"}));
    let o = plot(&e, r["id"].as_u64().unwrap());
    let spec = o.comments.iter().find(|c| c.starts_with("rustfig-plot:")).unwrap();
    assert!(!spec.contains("curves"), "{spec}");
    assert!(!o.comments.iter().any(|c| c.starts_with("rustfig-plot-points:")));
}
