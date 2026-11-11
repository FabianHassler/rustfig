#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Plot axes and grid (plan Q1): axes crossing at the origin, no axes, grid
//! with minor lines and a line style; old plots read unchanged.

use rustfig_engine::Engine;
use rustfig_model::{LineKind, Object, ObjectId, ObjectKind, Polyline};
use serde_json::{json, Value};

fn members(o: &Object) -> &[Object] {
    match &o.kind {
        ObjectKind::Compound(c) => &c.objects,
        _ => panic!("not a compound"),
    }
}

fn of_kind<'a>(o: &'a Object, kind: &str) -> Vec<&'a Object> {
    members(o)
        .iter()
        .filter(|m| {
            m.comments
                .iter()
                .any(|c| c.trim_end().ends_with(&format!("part: {kind}")))
        })
        .collect()
}

fn poly(o: &Object) -> &Polyline {
    match &o.kind {
        ObjectKind::Polyline(p) => p,
        _ => panic!("not a polyline"),
    }
}

fn texts(o: &Object, kind: &str) -> Vec<String> {
    of_kind(o, kind)
        .iter()
        .filter_map(|m| match &m.kind {
            ObjectKind::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

fn add(e: &mut Engine, extra: Value) -> ObjectId {
    let mut p = json!({"expr": "x^2", "xmin": -2, "xmax": 2, "ymin": -1, "ymax": 4,
                       "x": 0, "y": 0, "width": 4000, "height": 2500});
    for (k, v) in extra.as_object().unwrap() {
        p[k] = v.clone();
    }
    let r = e.execute("plot.add", &p).unwrap();
    ObjectId(r["id"].as_u64().unwrap())
}

#[test]
fn origin_axes_cross_at_zero_with_arrows_and_labels_at_the_tips() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"axes": "origin", "xlabel": "x", "ylabel": "y", "xticks": {"dir": "out"}, "yticks": {"dir": "out"}}),
    );
    let o = e.doc().get(id).unwrap();
    assert!(of_kind(o, "frame").is_empty(), "no box with origin axes");
    let axes = of_kind(o, "axis");
    assert_eq!(axes.len(), 2);
    let (xa, ya) = (poly(axes[0]), poly(axes[1]));
    // x = 0 is the middle of -2..2, y = 0 one fifth up from -1..4.
    assert_eq!(ya.points[0].x, 2000.0);
    assert_eq!(xa.points[0].y, 2500.0 - 500.0);
    assert!(xa.forward_arrow.is_some() && ya.forward_arrow.is_some());
    assert!(xa.points[1].x > xa.points[0].x, "the arrow at the right tip");
    assert!(ya.points[1].y < ya.points[0].y, "the arrow at the top tip");
    // The labels stand at the tips.
    let labels = of_kind(o, "label");
    assert_eq!(labels.len(), 2);
    let at = |i: usize| match &labels[i].kind {
        ObjectKind::Text(t) => t.origin,
        _ => panic!(),
    };
    assert!(at(0).x > 4000.0 && (at(0).y - xa.points[0].y).abs() < 100.0);
    assert!(at(1).y < 0.0 && at(1).x == 2000.0);
    // Ticks stand on the axes, numbers below and left of them, the y number
    // at the crossing is left out.
    let ticks = of_kind(o, "tick");
    // Each tick crosses its axis: a vertical one the line y = 2000, a
    // horizontal one the line x = 2000 (`out`: it ends on the axis).
    assert!(ticks.iter().all(|t| {
        let p = poly(t);
        let (a, b) = (p.points[0], p.points[1]);
        if a.x == b.x {
            a.y.min(b.y) <= 2000.0 && a.y.max(b.y) >= 2000.0
        } else {
            a.y == b.y && a.x.min(b.x) <= 2000.0 && a.x.max(b.x) >= 2000.0
        }
    }));
    let ynums = of_kind(o, "number")
        .into_iter()
        .filter_map(|m| match &m.kind {
            ObjectKind::Text(t) if t.origin.x < 2000.0 => Some(t.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!ynums.contains(&"0".to_string()), "{ynums:?}");
    assert!(ynums.contains(&"2".to_string()));
    assert!(texts(o, "number").contains(&"-1".to_string()));
}

#[test]
fn origin_axes_stand_at_the_edge_when_zero_is_outside() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"axes": "origin", "xmin": 1, "xmax": 5, "ymin": -4, "ymax": -1}),
    );
    let o = e.doc().get(id).unwrap();
    let axes = of_kind(o, "axis");
    let (xa, ya) = (poly(axes[0]), poly(axes[1]));
    assert_eq!(ya.points[0].x, 0.0, "left edge: 0 is below the x range");
    assert_eq!(xa.points[0].y, 0.0, "top edge: 0 is above the y range");
}

#[test]
fn arrows_can_be_turned_off() {
    let mut e = Engine::new();
    let id = add(&mut e, json!({"axes": "origin", "axis_arrows": false}));
    let o = e.doc().get(id).unwrap();
    assert!(of_kind(o, "axis").iter().all(|a| poly(a).forward_arrow.is_none()));
}

#[test]
fn no_axes_draws_the_curve_alone() {
    let mut e = Engine::new();
    let id = add(&mut e, json!({"axes": "none", "xlabel": "x"}));
    let o = e.doc().get(id).unwrap();
    for k in ["frame", "axis", "tick", "number"] {
        assert!(of_kind(o, k).is_empty(), "{k}");
    }
    assert!(!of_kind(o, "curve").is_empty());
    // The grid does not depend on the axes.
    let id = add(&mut e, json!({"axes": "none", "grid": true}));
    assert!(!of_kind(e.doc().get(id).unwrap(), "grid").is_empty());
}

#[test]
fn frame_is_the_default_and_the_kind_is_kept_by_redraw() {
    let mut e = Engine::new();
    let id = add(&mut e, json!({}));
    let o = e.doc().get(id).unwrap();
    assert_eq!(of_kind(o, "frame").len(), 1);
    assert!(!o.comments[0].contains("axes"), "{}", o.comments[0]);
    e.execute("plot.update", &json!({"id": id.0, "axes": "origin"}))
        .unwrap();
    let o = e.doc().get(id).unwrap();
    assert_eq!(of_kind(o, "axis").len(), 2);
    assert!(o.comments[0].contains("\"axes\":\"origin\""));
    let got = e.execute("plot.get", &json!({"id": id.0})).unwrap();
    assert_eq!(got["spec"]["axes"], "origin");
    // Back to the frame.
    e.execute("plot.update", &json!({"id": id.0, "axes": "frame"})).unwrap();
    let o = e.doc().get(id).unwrap();
    assert_eq!(of_kind(o, "frame").len(), 1);
    assert!(of_kind(o, "axis").is_empty());
    assert!(!o.comments[0].contains("axes"));
}

#[test]
fn old_comments_read_unchanged() {
    use rustfig_engine::plot::{AxesKind, PlotSpec, PLOT_COMMENT};
    for (json, frame) in [("{}", true), ("{\"axes\":true}", true), ("{\"axes\":false}", false)] {
        let s = PlotSpec::from_comment(&format!("{PLOT_COMMENT} {json}")).unwrap();
        assert_eq!(s.axes_kind(), AxesKind::Frame);
        assert_eq!(s.frame, frame);
        assert!(!s.grid_minor && s.grid_style.is_none() && s.axis_arrows);
    }
    let mut e = Engine::new();
    let r = e.execute("plot.add", &json!({"expr": "x", "axes": false})).unwrap();
    let o = e.doc().get(ObjectId(r["id"].as_u64().unwrap())).unwrap();
    assert!(of_kind(o, "frame").is_empty() && of_kind(o, "tick").is_empty());
    assert!(e
        .execute("plot.add", &json!({"expr": "x", "axes": "sideways"}))
        .is_err());
}

#[test]
fn grid_has_minor_lines_and_a_line_style() {
    let mut e = Engine::new();
    let plain = add(&mut e, json!({"grid": true}));
    let n_major = of_kind(e.doc().get(plain).unwrap(), "grid").len();
    assert!(n_major > 4);
    let dashed = add(&mut e, json!({"grid": true, "grid_style": "dashed"}));
    let o = e.doc().get(dashed).unwrap();
    assert_eq!(of_kind(o, "grid").len(), n_major);
    assert!(of_kind(o, "grid")
        .iter()
        .all(|g| poly(g).style.line == LineKind::Dashed));
    let plain_o = e.doc().get(plain).unwrap();
    assert!(of_kind(plain_o, "grid")
        .iter()
        .all(|g| poly(g).style.line == LineKind::Dotted));

    let minor = add(&mut e, json!({"grid": true, "grid_minor": true}));
    let o = e.doc().get(minor).unwrap();
    let lines = of_kind(o, "grid");
    assert!(lines.len() > 3 * n_major, "{} against {n_major}", lines.len());
    // Minor lines are dotted at a wider gap than the major ones.
    let gaps: Vec<f64> = lines.iter().map(|g| poly(g).style.style_val).collect();
    let (lo, hi) = (
        gaps.iter().cloned().fold(f64::MAX, f64::min),
        gaps.iter().cloned().fold(0.0, f64::max),
    );
    assert!(hi >= 2.0 * lo - 1e-9 && hi > lo);
    // Without "grid" nothing is drawn, whatever else is set.
    let none = add(&mut e, json!({"grid_minor": true, "grid_style": "dashed"}));
    assert!(of_kind(e.doc().get(none).unwrap(), "grid").is_empty());
    assert!(e
        .execute("plot.add", &json!({"expr": "x", "grid": true, "grid_style": "wavy"}))
        .is_err());
}

#[test]
fn hostile_axes_and_grid_values_are_errors_or_plots() {
    let mut e = Engine::new();
    for p in [
        json!({"axes": 5}),
        json!({"axes": null, "grid": "yes"}),
        json!({"axes": ["origin"]}),
        json!({"axis_arrows": "no"}),
        json!({"grid_minor": 1}),
        json!({"grid_style": 7}),
        json!({"grid_style": "\u{0}".repeat(400)}),
    ] {
        let n = e.doc().objects.len();
        let mut q = json!({"expr": "x"});
        for (k, v) in p.as_object().unwrap() {
            q[k] = v.clone();
        }
        let _ = e.execute("plot.add", &q);
        assert!(e.doc().objects.len() <= n + 1);
    }
    // Origin axes with a logarithmic axis and a degenerate frame never panic.
    for (lx, ly) in [(false, false), (true, false), (false, true), (true, true)] {
        let _ = e.execute(
            "plot.add",
            &json!({"expr": "x", "xmin": 0.1, "xmax": 10, "ymin": 0.1, "ymax": 10, "log_x": lx, "log_y": ly,
                    "axes": "origin", "grid": true, "grid_minor": true}),
        );
    }
}
