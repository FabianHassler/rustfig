#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! More curve kinds (plan Q5): polar, implicit, derivative, integral and
//! field lines; sizes capped, hostile input finishes.

use rustfig_engine::Engine;
use rustfig_model::{Object, ObjectId, ObjectKind, Polyline};
use serde_json::{json, Value};

fn curves(o: &Object) -> Vec<&Polyline> {
    let ObjectKind::Compound(c) = &o.kind else { panic!() };
    c.objects
        .iter()
        .filter(|m| m.comments.iter().any(|c| c.trim_end().ends_with("part: curve")))
        .filter_map(|m| match &m.kind {
            ObjectKind::Polyline(p) => Some(p),
            _ => None,
        })
        .collect()
}

fn add(e: &mut Engine, p: Value) -> Result<ObjectId, String> {
    let mut q = json!({"x": 0, "y": 0, "width": 4000, "height": 4000});
    for (k, v) in p.as_object().unwrap() {
        q[k] = v.clone();
    }
    e.execute("plot.add", &q)
        .map(|r| ObjectId(r["id"].as_u64().unwrap()))
        .map_err(|e| e.to_string())
}

/// Frame units per data unit with x and y both -2..2 on 4000.
const U: f64 = 1000.0;

#[test]
fn polar_curve() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"mode": "polar", "expr": "1+cos(t)", "xmin": -2, "xmax": 2, "ymin": -2, "ymax": 2}),
    )
    .unwrap();
    let cs = curves(e.doc().get(id).unwrap());
    assert_eq!(cs.len(), 1);
    // The cardioid reaches x = 2 at t = 0 and the origin at t = pi.
    let (mut far, mut near) = (0.0f64, f64::MAX);
    for p in &cs[0].points {
        let (x, y) = ((p.x - 2.0 * U) / U, (2.0 * U - p.y) / U);
        far = far.max(x);
        near = near.min(x.hypot(y));
    }
    assert!((far - 2.0).abs() < 0.01, "{far}");
    assert!(near < 0.02, "{near}");
    // theta is accepted, a t range limits it.
    let id = add(
        &mut e,
        json!({"mode": "polar", "expr": "theta", "tmin": 0, "tmax": 3, "xmin": -4, "xmax": 4, "ymin": -4, "ymax": 4}),
    )
    .unwrap();
    assert_eq!(curves(e.doc().get(id).unwrap()).len(), 1);
}

#[test]
fn implicit_circle_and_hyperbola() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"mode": "implicit", "expr": "x^2+y^2=1", "xmin": -2, "xmax": 2, "ymin": -2, "ymax": 2}),
    )
    .unwrap();
    let cs = curves(e.doc().get(id).unwrap());
    assert_eq!(cs.len(), 1, "one chained closed line");
    for p in &cs[0].points {
        let r = ((p.x - 2.0 * U) / U).hypot((2.0 * U - p.y) / U);
        assert!((r - 1.0).abs() < 0.01, "{r}");
    }
    assert_eq!(cs[0].points.first(), cs[0].points.last(), "closed");
    // x*y = 1: two branches. The same without an "=" is f = 0.
    let id = add(
        &mut e,
        json!({"mode": "implicit", "expr": "x*y-1", "xmin": -4, "xmax": 4, "ymin": -4, "ymax": 4}),
    )
    .unwrap();
    assert_eq!(curves(e.doc().get(id).unwrap()).len(), 2);
    // Crossing lines (a saddle in every cell): no panic, a curve.
    let id = add(
        &mut e,
        json!({"mode": "implicit", "expr": "x^2=y^2", "xmin": -2, "xmax": 2, "ymin": -2, "ymax": 2}),
    )
    .unwrap();
    assert!(!curves(e.doc().get(id).unwrap()).is_empty());
    // In a list, kept and redrawn.
    let id = add(
        &mut e,
        json!({"xmin": -2, "xmax": 2, "ymin": -2, "ymax": 2,
               "curves": [{"mode": "implicit", "expr": "x^2+y^2=1"}, {"expr": "x"}]}),
    )
    .unwrap();
    e.execute("plot.update", &json!({"id": id.0, "grid": true})).unwrap();
    assert_eq!(curves(e.doc().get(id).unwrap()).len(), 2);
}

#[test]
fn derivative_and_integral_of_a_curve() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"xmin": 0, "xmax": std::f64::consts::TAU, "ymin": -1, "ymax": 1, "curves": [
            {"expr": "sin(x)"},
            {"mode": "derivative", "of": 1},
            {"mode": "integral", "of": 2}]}),
    )
    .unwrap();
    let cs = curves(e.doc().get(id).unwrap());
    assert_eq!(cs.len(), 3);
    // d/dx sin = cos: 1 at x = 0 (the top edge, 0), -1 at x = pi (the bottom, 4000).
    let d = cs[1];
    assert!(d.points.first().unwrap().y < 30.0, "{:?}", d.points.first());
    let mid = d
        .points
        .iter()
        .min_by(|a, b| (a.x - 2000.0).abs().total_cmp(&(b.x - 2000.0).abs()))
        .unwrap();
    assert!(mid.y > 3900.0, "{mid:?}");
    // The integral of that derivative is sin again (from 0): at pi/2 it is 1.
    let i = cs[2];
    let q = i
        .points
        .iter()
        .min_by(|a, b| (a.x - 1000.0).abs().total_cmp(&(b.x - 1000.0).abs()))
        .unwrap();
    assert!(q.y < 60.0, "{q:?}");
    assert!((i.points.last().unwrap().y - 2000.0).abs() < 60.0);
    // Errors: not an earlier curve.
    for c in [
        json!({"mode": "derivative"}),
        json!({"mode": "derivative", "of": 0}),
        json!({"mode": "derivative", "of": 2}),
        json!({"mode": "integral", "of": 9}),
    ] {
        assert!(
            add(&mut e, json!({"curves": [{"expr": "x"}, c.clone()]})).is_err(),
            "{c}"
        );
    }
    assert!(add(&mut e, json!({"mode": "derivative"})).is_err());
}

#[test]
fn field_lines_by_rk4() {
    let mut e = Engine::new();
    // A rotation field: lines are circles around the origin.
    let id = add(
        &mut e,
        json!({"xmin": -2, "xmax": 2, "ymin": -2, "ymax": 2, "curves": [
            {"mode": "field", "xt": "-y", "yt": "x", "starts": [[1, 0], [0.5, 0]], "field_steps": 300, "field_step": 0.05}]}),
    )
    .unwrap();
    let cs = curves(e.doc().get(id).unwrap());
    assert_eq!(cs.len(), 2);
    for (c, r) in cs.iter().zip([1.0, 0.5]) {
        for p in &c.points {
            let d = ((p.x - 2.0 * U) / U).hypot((2.0 * U - p.y) / U);
            assert!((d - r).abs() < 0.01, "{d} against {r}");
        }
        assert!(c.points.len() > 50);
    }
    // A constant field: straight lines, cut at the window.
    let id = add(
        &mut e,
        json!({"xmin": -2, "xmax": 2, "ymin": -2, "ymax": 2, "curves": [
            {"mode": "field", "xt": "1", "yt": "0", "starts": [[0, 1]]}]}),
    )
    .unwrap();
    let c = curves(e.doc().get(id).unwrap())[0].clone();
    assert!(c.points.iter().all(|p| p.y == 1000.0 && (0.0..=4000.0).contains(&p.x)));
    assert_eq!(c.points.first().unwrap().x, 0.0);
    assert_eq!(c.points.last().unwrap().x, 4000.0);
    // Starts are kept in the figure.
    let got = e.execute("plot.get", &json!({"id": id.0})).unwrap();
    assert_eq!(got["spec"]["curves"][0]["starts"], json!([[0.0, 1.0]]));
}

#[test]
fn hostile_kinds_are_errors_and_finish() {
    let mut e = Engine::new();
    let many: Vec<Value> = (0..100).map(|i| json!([i, i])).collect();
    for c in [
        json!({"mode": "field", "xt": "x", "yt": "y"}),
        json!({"mode": "field", "xt": "x", "yt": "y", "starts": many}),
        json!({"mode": "field", "xt": "x", "yt": "y", "starts": [[1, 1]], "field_steps": 0}),
        json!({"mode": "field", "xt": "x", "yt": "y", "starts": [[1, 1]], "field_steps": 100000}),
        json!({"mode": "field", "xt": "x", "yt": "y", "starts": [[1e300, 1]]}),
        json!({"mode": "field", "xt": "x", "yt": "y", "starts": [[1, 1]], "field_step": -1}),
        json!({"mode": "field", "xt": "x", "yt": "y", "starts": [[1, 1]], "field_step": 0}),
        json!({"mode": "implicit", "expr": "x=y=1"}),
        json!({"mode": "implicit", "expr": "="}),
        json!({"mode": "implicit", "expr": ""}),
        json!({"mode": "polar", "expr": ""}),
        json!({"mode": "polar", "expr": "t", "tmin": 3, "tmax": 1}),
    ] {
        assert!(
            add(
                &mut e,
                json!({"curves": [c.clone()], "xmin": -2, "xmax": 2, "ymin": -2, "ymax": 2})
            )
            .is_err(),
            "{c}"
        );
    }
    // Field lines on a logarithmic axis are refused.
    assert!(add(
        &mut e,
        json!({"log_x": true, "xmin": 1, "xmax": 10, "ymin": 0, "ymax": 1,
               "curves": [{"mode": "field", "xt": "1", "yt": "1", "starts": [[2, 0.5]]}]})
    )
    .is_err());
    // The biggest expression on the biggest grid finishes; the field's work is capped.
    let start = std::time::Instant::now();
    let big = format!("x{}", "+sin(x*y)*cos(y)".repeat(100));
    let _ = add(
        &mut e,
        json!({"mode": "implicit", "expr": big, "samples": 20000, "xmin": -2, "xmax": 2, "ymin": -2, "ymax": 2}),
    );
    let _ = add(
        &mut e,
        json!({"xmin": -2, "xmax": 2, "ymin": -2, "ymax": 2, "curves": [
            {"mode": "field", "xt": big, "yt": big, "starts": many_starts(), "field_steps": 5000, "field_step": 0.001}]}),
    );
    assert!(start.elapsed().as_secs() < 120, "took {:?}", start.elapsed());
    // Logarithmic axes with implicit and polar curves do not panic.
    for (lx, ly) in [(true, false), (false, true), (true, true)] {
        let _ = add(
            &mut e,
            json!({"log_x": lx, "log_y": ly, "xmin": 0.1, "xmax": 10, "ymin": 0.1, "ymax": 10, "curves": [
                {"mode": "implicit", "expr": "x*y=1"}, {"mode": "polar", "expr": "t"}, {"mode": "derivative", "of": 1}]}),
        );
    }
}

fn many_starts() -> Vec<Value> {
    (0..60).map(|i| json!([i as f64 / 30.0 - 1.0, 0.1])).collect()
}

mod fuzz {
    use super::*;
    use proptest::prelude::*;

    const TOKENS: &[&str] = &[
        "x", "y", "t", "pi", "1", "0", "2.5", "1e308", "sin", "cos", "sqrt", "abs", "exp", "(", ")", ",", "+", "-",
        "*", "/", "^", "=", " ",
    ];

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn token_soup_kinds(toks in proptest::collection::vec(proptest::sample::select(TOKENS), 0..40),
                            which in 0u8..4, lx in any::<bool>(), steps in 0usize..50) {
            let expr: String = toks.concat();
            let c = match which {
                0 => json!({"mode": "implicit", "expr": expr, "samples": 40}),
                1 => json!({"mode": "polar", "expr": expr, "samples": 50}),
                2 => json!({"mode": "field", "xt": expr, "yt": expr, "starts": [[0.5, 0.5], [-1, 2]], "field_steps": steps}),
                _ => json!({"mode": "integral", "of": 1}),
            };
            let mut e = Engine::new();
            let _ = e.execute("plot.add", &json!({"log_x": lx, "xmin": 0.01, "xmax": 5, "ymin": -3, "ymax": 3,
                "curves": [{"expr": expr}, c]}));
        }
    }
}
