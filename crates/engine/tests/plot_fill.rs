#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Curves cut at the window and filled areas (plan Q3).

use rustfig_engine::Engine;
use rustfig_model::{Object, ObjectId, ObjectKind, PolyKind, Polyline};
use serde_json::{json, Value};

fn parts<'a>(o: &'a Object, kind: &str) -> Vec<&'a Polyline> {
    let ObjectKind::Compound(c) = &o.kind else { panic!() };
    c.objects
        .iter()
        .filter(|m| {
            m.comments
                .iter()
                .any(|c| c.trim_end().ends_with(&format!("part: {kind}")))
        })
        .filter_map(|m| match &m.kind {
            ObjectKind::Polyline(p) => Some(p),
            _ => None,
        })
        .collect()
}

fn add(e: &mut Engine, p: Value) -> Result<ObjectId, String> {
    let mut q = json!({"xmin": 0, "xmax": std::f64::consts::TAU, "x": 0, "y": 0, "width": 4000, "height": 3000});
    for (k, v) in p.as_object().unwrap() {
        q[k] = v.clone();
    }
    e.execute("plot.add", &q)
        .map(|r| ObjectId(r["id"].as_u64().unwrap()))
        .map_err(|e| e.to_string())
}

fn inside(p: &Polyline) -> bool {
    p.points
        .iter()
        .all(|q| (0.0..=4000.0).contains(&q.x) && (0.0..=3000.0).contains(&q.y))
}

#[test]
fn a_curve_leaving_the_y_range_is_cut_at_the_window() {
    let mut e = Engine::new();
    // 2 sin x between -1 and 1: three pieces, each ending on the window's edge.
    let id = add(&mut e, json!({"expr": "2*sin(x)", "ymin": -1, "ymax": 1})).unwrap();
    let curves = parts(e.doc().get(id).unwrap(), "curve");
    assert_eq!(curves.len(), 3);
    assert!(curves.iter().all(|c| inside(c)));
    let ys = |c: &Polyline| (c.points.first().unwrap().y, c.points.last().unwrap().y);
    assert_eq!(ys(curves[0]).1, 0.0, "cut at the top edge");
    assert_eq!(ys(curves[1]).0, 0.0);
    assert_eq!(ys(curves[1]).1, 3000.0, "and at the bottom edge");
    assert_eq!(ys(curves[2]).0, 3000.0);
}

#[test]
fn a_pole_is_cut_at_the_window_with_no_line_across() {
    let mut e = Engine::new();
    let id = add(&mut e, json!({"expr": "tan(x)", "ymin": -3, "ymax": 3})).unwrap();
    let curves = parts(e.doc().get(id).unwrap(), "curve");
    // Three branches of tan on 0..2pi, each one reaching the window's edges.
    assert!(curves.len() >= 3, "{}", curves.len());
    assert!(curves.iter().all(|c| inside(c)));
    let reach_top = curves.iter().filter(|c| c.points.iter().any(|p| p.y == 0.0)).count();
    let reach_bottom = curves.iter().filter(|c| c.points.iter().any(|p| p.y == 3000.0)).count();
    assert!(reach_top >= 2 && reach_bottom >= 2, "{reach_top} {reach_bottom}");
    // No segment crosses the window from edge to edge.
    for c in &curves {
        assert!(c.points.windows(2).all(|w| (w[0].y - w[1].y).abs() < 2500.0));
    }
}

#[test]
fn fill_between_sin_and_cos() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"ymin": -1, "ymax": 1, "curves": [
            {"expr": "sin(x)", "pen_color": 1, "fill": {"curve": 2, "color": 11, "density": 30}},
            {"expr": "cos(x)", "pen_color": 2}]}),
    )
    .unwrap();
    let o = e.doc().get(id).unwrap();
    let fills = parts(o, "fill");
    assert_eq!(fills.len(), 1);
    let f = fills[0];
    assert_eq!(f.kind, PolyKind::Polygon);
    assert_eq!(
        (f.style.fill_color.0, f.style.area_fill, f.style.thickness),
        (11, 30, 0)
    );
    assert!(inside(f));
    // Behind the curves and the grid.
    let curve_depth = parts(o, "curve")[0].style.depth;
    assert!(f.style.depth > curve_depth);
    // Both curves' ends are in it: sin(0) = 0 -> mid height, cos(0) = 1 -> top.
    let at = |x: f64, y: f64| f.points.iter().any(|p| (p.x - x).abs() < 1.0 && (p.y - y).abs() < 1.0);
    assert!(at(0.0, 1500.0) && at(0.0, 0.0), "{:?}", &f.points[..3]);
    // The curves are still drawn.
    assert_eq!(parts(o, "curve").len(), 2);
    // The fill is kept and redrawn.
    let got = e.execute("plot.get", &json!({"id": id.0})).unwrap();
    assert_eq!(got["spec"]["curves"][0]["fill"]["curve"], 2);
    e.execute("plot.update", &json!({"id": id.0, "grid": true})).unwrap();
    assert_eq!(parts(e.doc().get(id).unwrap(), "fill").len(), 1);
}

#[test]
fn fill_to_the_baseline() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"expr": "sin(x)", "ymin": -1, "ymax": 1, "curves": [{"expr": "sin(x)", "pen_color": 4, "fill": {}}]}),
    )
    .unwrap();
    let o = e.doc().get(id).unwrap();
    let f = parts(o, "fill")[0];
    // The default baseline y = 0 is the middle; the color is the curve's.
    assert!(f.points.iter().filter(|p| p.y == 1500.0).count() >= 2);
    assert_eq!((f.style.fill_color.0, f.style.area_fill), (4, 20));
    // A baseline beyond the window sits on its edge.
    let id = add(
        &mut e,
        json!({"ymin": -1, "ymax": 1, "curves": [{"expr": "sin(x)", "fill": {"y": -50}}]}),
    )
    .unwrap();
    let f = parts(e.doc().get(id).unwrap(), "fill")[0];
    assert!(f.points.iter().any(|p| p.y == 3000.0));
    assert!(inside(f));
    // A curve cut at the window gives a polygon per piece.
    let id = add(
        &mut e,
        json!({"ymin": -1, "ymax": 1, "curves": [{"expr": "2*sin(x)", "fill": {"y": 0}}]}),
    )
    .unwrap();
    let o = e.doc().get(id).unwrap();
    assert_eq!(parts(o, "fill").len(), 3);
    assert!(parts(o, "fill").iter().all(|f| inside(f)));
}

#[test]
fn old_plots_have_no_fill_and_bad_fills_are_errors() {
    let mut e = Engine::new();
    let id = add(&mut e, json!({"expr": "sin(x)"})).unwrap();
    assert!(parts(e.doc().get(id).unwrap(), "fill").is_empty());
    assert!(!e.doc().get(id).unwrap().comments[0].contains("fill"));
    let two = |fill: Value| json!({"curves": [{"expr": "x", "fill": fill}, {"expr": "x*x"}], "ymin": 0, "ymax": 5});
    for fill in [
        json!({"curve": 9}),
        json!({"curve": 0}),
        json!({"curve": 1}),
        json!({"density": 99}),
        json!({"density": -1}),
        json!({"y": 1e300}),
        json!({"color": 9999}),
        json!({"curve": "two"}),
        json!("full"),
    ] {
        let n = e.doc().objects.len();
        assert!(add(&mut e, two(fill.clone())).is_err(), "{fill}");
        assert_eq!(e.doc().objects.len(), n);
    }
    // A single curve cannot fill up to another.
    assert!(add(&mut e, json!({"curves": [{"expr": "x", "fill": {"curve": 2}}]})).is_err());
    // A fill of a curve outside the window warns instead of failing.
    let r = e
        .execute(
            "plot.add",
            &json!({"xmin": 0, "xmax": 1, "ymin": 10, "ymax": 11, "curves": [{"expr": "x", "fill": {"y": 0}}]}),
        )
        .unwrap();
    assert!(!r["warnings"].as_array().unwrap().is_empty());
}
