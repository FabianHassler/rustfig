#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Plots without a curve: an empty expression, or a curve outside the ranges.

use rustfig_engine::Engine;
use rustfig_model::{Object, ObjectId, ObjectKind};
use serde_json::{json, Value};

fn members(o: &Object) -> &[Object] {
    match &o.kind {
        ObjectKind::Compound(c) => &c.objects,
        _ => panic!("not a compound"),
    }
}

fn curve_parts(o: &Object) -> usize {
    members(o)
        .iter()
        .filter(|m| m.comments.iter().any(|c| c.ends_with(" curve")))
        .count()
}

fn warnings(r: &Value) -> Vec<String> {
    r.get("warnings")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|w| w.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

fn base(expr: &str) -> Value {
    json!({"expr": expr, "xmin": 0, "xmax": 6, "ymin": -1, "ymax": 1,
           "x": 600, "y": 600, "width": 4800, "height": 3000})
}

#[test]
fn empty_expr_draws_frame_only() {
    let mut e = Engine::new();
    let r = e.execute("plot.add", &base("")).unwrap();
    assert_eq!(r["curves"], 0);
    assert!(warnings(&r).is_empty(), "{r}");
    let o = e.doc().get(ObjectId(r["id"].as_u64().unwrap())).unwrap();
    assert_eq!(curve_parts(o), 0);
    let n = members(o).len();
    assert!(n > 5, "frame, ticks and labels expected");

    let mut e2 = Engine::new();
    let r2 = e2.execute("plot.add", &base("sin(x)")).unwrap();
    let o2 = e2.doc().get(ObjectId(r2["id"].as_u64().unwrap())).unwrap();
    assert!(curve_parts(o2) >= 1);
    assert!(members(o2).len() > n);
}

#[test]
fn empty_expr_needs_y_range() {
    let mut e = Engine::new();
    for missing in ["ymin", "ymax"] {
        let mut p = base("");
        p.as_object_mut().unwrap().remove(missing);
        assert!(e.execute("plot.add", &p).is_err(), "without {missing}");
    }
    assert!(e.doc().objects.is_empty());
}

#[test]
fn curve_outside_ranges_warns_then_update_shows_it() {
    let mut e = Engine::new();
    let r = e
        .execute(
            "plot.add",
            &json!({"expr": "x + 100", "xmin": 0, "xmax": 6, "ymin": -1, "ymax": 1,
                    "x": 600, "y": 600, "width": 4800, "height": 3000}),
        )
        .unwrap();
    assert_eq!(r["curves"], 0);
    assert_eq!(warnings(&r), ["nothing of the curve lies inside the ranges"]);
    let id = r["id"].as_u64().unwrap();
    assert_eq!(curve_parts(e.doc().get(ObjectId(id)).unwrap()), 0);

    let r = e
        .execute("plot.update", &json!({"id": id, "ymin": 90, "ymax": 110}))
        .unwrap();
    assert!(r["curves"].as_u64().unwrap() >= 1, "{r}");
    assert!(warnings(&r).is_empty(), "{r}");
    assert!(curve_parts(e.doc().get(ObjectId(id)).unwrap()) >= 1);
}
