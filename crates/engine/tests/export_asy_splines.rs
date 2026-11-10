#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Splines reach the Asymptote driver as splines (fig2dev 0.9.14 `-S`).

use rustfig_engine::Engine;
use serde_json::json;

fn asy(mode: Option<&str>) -> String {
    let mut e = Engine::new();
    // An interpolated spline as xfig draws it (shape factor -0.5) and an
    // approximated one.
    e.execute(
        "shape.spline",
        &json!({"points": [[1200, 1200], [2400, 600], [3000, 2400], [4800, 1800]], "kind": "interpolated"}),
    )
    .unwrap();
    e.execute(
        "shape.spline",
        &json!({"points": [[1200, 3600], [2400, 3000], [3000, 4800], [4800, 4200]], "kind": "approximated"}),
    )
    .unwrap();
    let mut p = json!({"format": "asy"});
    if let Some(m) = mode {
        p["options"] = json!({"S": m});
    }
    let r = e.execute("file.export_data", &p).unwrap();
    r["data"].as_str().unwrap().to_string()
}

#[test]
fn splines_are_curves_not_polylines() {
    let out = asy(None);
    assert!(
        out.contains("bspline(new pair[]{(1200,-3600),(2400,-3000),(3000,-4800),(4800,-4200)},false)"),
        "{out}"
    );
    assert!(out.contains("..controls"), "{out}");
    assert!(!out.contains("// Mixed"), "{out}");
    // Not the hundreds of points of a flattened spline.
    assert!(out.matches("--").count() < 20, "{out}");
}

#[test]
fn hobby_and_exact_modes() {
    let hobby = asy(Some("hobby"));
    assert!(
        hobby.contains("(1200,-1200)..(2400,-600)..(3000,-2400)..(4800,-1800)"),
        "{hobby}"
    );
    let exact = asy(Some("exact"));
    assert!(!exact.contains("bspline(new"), "{exact}");
    assert!(exact.matches("..controls").count() > hobby.matches("..controls").count());
}
