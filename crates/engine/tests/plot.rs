#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Plot a curve (plan E12): commands, edit-and-redraw from the comment, and
//! the Plot mode.

use rustfig_engine::{Button, Engine, ToolEvent, ToolKind, UiRequest};
use rustfig_geom::Point;
use rustfig_model::{Object, ObjectId, ObjectKind};
use serde_json::json;

fn members(o: &Object) -> &[Object] {
    match &o.kind {
        ObjectKind::Compound(c) => &c.objects,
        _ => panic!("not a compound"),
    }
}

fn polylines(o: &Object) -> Vec<&rustfig_model::Polyline> {
    members(o)
        .iter()
        .filter_map(|m| match &m.kind {
            ObjectKind::Polyline(p) => Some(p),
            _ => None,
        })
        .collect()
}

fn sine(e: &mut Engine) -> u64 {
    let r = e
        .execute(
            "plot.add",
            &json!({"expr": "sin(x)", "xmin": 0, "xmax": std::f64::consts::TAU, "x": 600, "y": 600, "width": 4800, "height": 3000,
                    "xlabel": "x", "ylabel": "$\\sin x$", "latex": true}),
        )
        .unwrap();
    assert_eq!(r["curves"], 1);
    assert!(r["points"].as_u64().unwrap() >= 400);
    r["id"].as_u64().unwrap()
}

#[test]
fn sine_plot_is_one_compound_with_axes() {
    let mut e = Engine::new();
    let id = sine(&mut e);
    assert_eq!(e.doc().objects.len(), 1);
    let o = e.doc().get(ObjectId(id)).unwrap();
    // The frame first, then ticks, numbers, labels and the curve.
    let ObjectKind::Polyline(frame) = &members(o)[0].kind else {
        panic!()
    };
    assert_eq!(frame.kind, rustfig_model::PolyKind::Box);
    assert!(members(o).len() > 15);
    assert!(o.comments[0].starts_with("rustfig-plot:"));
    let texts: Vec<_> = members(o)
        .iter()
        .filter_map(|m| match &m.kind {
            ObjectKind::Text(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t.text == "1" || t.text == "0.5"));
    let special: Vec<_> = texts
        .iter()
        .filter(|t| t.is_special())
        .map(|t| t.text.as_str())
        .collect();
    assert_eq!(special, ["x", "$\\sin x$"]);
    // The metrics are fixed after the command.
    assert!(texts.iter().all(|t| t.length > 0.0 && t.height > 0.0));
    // The curve stays in the frame.
    let curve = polylines(o).into_iter().max_by_key(|p| p.points.len()).unwrap();
    assert!(curve
        .points
        .iter()
        .all(|p| p.x >= 600.0 && p.x <= 5400.0 && p.y >= 600.0 && p.y <= 3600.0));
    // One undo removes the plot.
    assert!(e.undo());
    assert!(e.doc().objects.is_empty());
}

#[test]
fn reciprocal_has_no_line_across_zero() {
    let mut e = Engine::new();
    let r = e
        .execute(
            "plot.add",
            &json!({"expr": "1/x", "xmin": -2, "xmax": 2, "ymin": -4, "ymax": 4, "axes": false}),
        )
        .unwrap();
    assert_eq!(r["curves"], 2);
    let o = e.doc().get(ObjectId(r["id"].as_u64().unwrap())).unwrap();
    // x = 0 maps to the middle of the default 4800 wide frame.
    for p in polylines(o) {
        let left = p.points.iter().all(|q| q.x <= 2400.0);
        let right = p.points.iter().all(|q| q.x >= 2400.0);
        assert!(left || right, "a piece crosses x = 0");
    }
}

#[test]
fn csv_curve_from_a_file_and_from_text() {
    let dir = std::env::temp_dir().join(format!("rustfig-plot-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("d.csv");
    std::fs::write(&f, "t,a,b\n0,0,5\n1,1,3\n2,4,2\n3,9,1\n").unwrap();
    let mut e = Engine::new();
    let r = e
        .execute(
            "plot.add",
            &json!({"mode": "data", "file": f.to_str().unwrap(), "xcol": 1, "ycol": 3, "xmin": null, "xmax": null}),
        )
        .unwrap();
    assert_eq!(r["curves"], 1);
    assert_eq!(r["points"], 4);
    let o = e.doc().get(ObjectId(r["id"].as_u64().unwrap())).unwrap();
    // y falls (column b): the curve's last point is lower on the page (larger y) for smaller values.
    let curve = polylines(o).into_iter().max_by_key(|p| p.points.len()).unwrap();
    assert!(curve.points[3].y > curve.points[0].y);
    // Inline text, row numbers as x.
    let r = e
        .execute(
            "plot.add",
            &json!({"mode": "data", "data": "1 2\n3 4\n", "xcol": 0, "ycol": 2, "xmin": null, "xmax": null}),
        )
        .unwrap();
    assert_eq!(r["points"], 2);
    // Errors.
    let missing = dir.join("nope.csv");
    assert!(e
        .execute("plot.add", &json!({"mode": "data", "file": missing.to_str().unwrap()}))
        .is_err());
    assert!(e
        .execute(
            "plot.add",
            &json!({"mode": "data", "file": f.to_str().unwrap(), "ycol": 9})
        )
        .is_err());
    assert!(e.execute("plot.add", &json!({"mode": "data"})).is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn parametric_circle() {
    let mut e = Engine::new();
    let r = e
        .execute(
            "plot.add",
            &json!({"mode": "parametric", "xt": "cos(t)", "yt": "sin(t)", "xmin": -1.5, "xmax": 1.5, "ymin": -1.5, "ymax": 1.5}),
        )
        .unwrap();
    assert_eq!(r["curves"], 1);
}

#[test]
fn edit_and_redraw_from_the_comment() {
    let mut e = Engine::new();
    let id = sine(&mut e);
    let got = e.execute("plot.get", &json!({"id": id})).unwrap();
    assert_eq!(got["spec"]["expr"], "sin(x)");
    assert_eq!(got["spec"]["xlabel"], "x");
    let before = e.doc().get(ObjectId(id)).unwrap().clone();
    let r = e
        .execute("plot.update", &json!({"id": id, "expr": "cos(x)", "grid": true}))
        .unwrap();
    assert_eq!(r["id"], id);
    let after = e.doc().get(ObjectId(id)).unwrap();
    assert!(after.comments[0].contains("cos(x)"));
    assert!(after.comments[0].contains("\"grid\":true"));
    assert!(members(after).len() > members(&before).len(), "the grid adds lines");
    let frame = |o: &Object| match &members(o)[0].kind {
        ObjectKind::Polyline(p) => p.points.clone(),
        _ => panic!(),
    };
    assert_eq!(frame(&before), frame(after));
    // The object count and id stay.
    assert_eq!(e.doc().objects.len(), 1);
    // Failing updates change nothing.
    let snapshot = e.doc().get(ObjectId(id)).unwrap().clone();
    assert!(e.execute("plot.update", &json!({"id": id, "expr": "sin("})).is_err());
    assert_eq!(e.doc().get(ObjectId(id)).unwrap(), &snapshot);
    // Not a plot.
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10}))
        .unwrap();
    let rect_id = e.doc().objects.last().unwrap().id.0;
    assert!(e.execute("plot.update", &json!({"id": rect_id})).is_err());
    assert!(e.execute("plot.get", &json!({"id": 999_999})).is_err());
}

#[test]
fn edit_follows_a_moved_frame() {
    let mut e = Engine::new();
    let id = sine(&mut e);
    e.set_selection(vec![ObjectId(id)]);
    e.execute("transform.move", &json!({"dx": 1000, "dy": 500})).unwrap();
    let moved = e.doc().get(ObjectId(id)).unwrap().clone();
    let ObjectKind::Polyline(frame) = &members(&moved)[0].kind else {
        panic!()
    };
    assert_eq!(frame.points[0].x, 1600.0);
    e.execute("plot.update", &json!({"id": id, "expr": "x"})).unwrap();
    let after = e.doc().get(ObjectId(id)).unwrap();
    let ObjectKind::Polyline(frame) = &members(after)[0].kind else {
        panic!()
    };
    assert_eq!(frame.points[0].x, 1600.0, "redrawn where the user moved it");
}

fn kinds(o: &Object) -> Vec<String> {
    members(o)
        .iter()
        .filter_map(|m| m.comments.iter().find_map(|c| c.strip_prefix("rustfig-plot-part: ")))
        .map(str::to_string)
        .collect()
}

#[test]
fn every_generated_part_is_marked() {
    let mut e = Engine::new();
    let r = e
        .execute(
            "plot.add",
            &json!({"expr": "sin(x)", "grid": true, "xlabel": "x", "ylabel": "y"}),
        )
        .unwrap();
    let o = e.doc().get(ObjectId(r["id"].as_u64().unwrap())).unwrap();
    assert_eq!(kinds(o).len(), members(o).len(), "no unmarked part");
    for k in ["frame", "grid", "tick", "number", "label", "curve"] {
        assert!(kinds(o).iter().any(|x| x == k), "no {k}");
    }
}

#[test]
fn redraw_replaces_only_the_marked_parts() {
    let mut e = Engine::new();
    let id = sine(&mut e);
    let parts = members(e.doc().get(ObjectId(id)).unwrap()).len();
    // Edit inside the compound: add a box of your own and restyle a tick.
    e.execute("compound.open", &json!({"id": id})).unwrap();
    e.execute("shape.rectangle", &json!({"x": 10, "y": 10, "width": 50, "height": 50}))
        .unwrap();
    e.execute("compound.close", &json!({})).unwrap();
    let o = e.doc().get(ObjectId(id)).unwrap();
    assert!(
        o.comments.iter().any(|c| c.starts_with("rustfig-plot:")),
        "the plot comment stays"
    );
    assert_eq!(members(o).len(), parts + 1);
    // Redraw with another curve: the user's box stays, the parts are new.
    e.execute("plot.update", &json!({"id": id, "expr": "cos(x)", "grid": true}))
        .unwrap();
    let o = e.doc().get(ObjectId(id)).unwrap();
    let own: Vec<_> = members(o)
        .iter()
        .filter(|m| m.comments.iter().all(|c| !c.starts_with("rustfig-plot-part:")))
        .collect();
    assert_eq!(own.len(), 1);
    assert!(matches!(&own[0].kind, ObjectKind::Polyline(p) if p.points.iter().any(|q| q.x == 10.0)));
    assert_eq!(
        kinds(o).iter().filter(|k| *k == "curve").count(),
        1,
        "no duplicate curve"
    );
    assert!(kinds(o).iter().any(|k| k == "grid"));
    assert!(o.comments.iter().filter(|c| c.starts_with("rustfig-plot:")).count() == 1);
}

#[test]
fn breaking_a_plot_leaves_an_ordinary_drawing() {
    let mut e = Engine::new();
    let id = sine(&mut e);
    e.execute("object.ungroup", &json!({"ids": [id]})).unwrap();
    assert!(e.doc().objects.len() > 10);
    for o in &e.doc().objects {
        assert!(
            o.comments.iter().all(|c| !c.starts_with("rustfig-plot")),
            "{:?}",
            o.comments
        );
    }
    assert!(e.execute("plot.update", &json!({"id": id})).is_err());
}

#[test]
fn plot_mode_two_corners_request_the_dialog() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::Plot);
    assert_eq!(e.mouse_hints().left, "first corner");
    let ev = |e: &mut Engine, ev: ToolEvent| e.tool_event(ev).unwrap();
    let press = |x: f64, y: f64, button: Button| ToolEvent::Press {
        pos: Point::new(x, y),
        button,
        shift: false,
    };
    // The first click sets a corner, a rubber band follows, no dialog yet.
    ev(&mut e, press(600.0, 600.0, Button::Left));
    ev(
        &mut e,
        ToolEvent::Release {
            pos: Point::new(600.0, 600.0),
            button: Button::Left,
        },
    );
    ev(
        &mut e,
        ToolEvent::Move {
            pos: Point::new(3000.0, 2400.0),
        },
    );
    assert!(e.take_requests().is_empty(), "one click opens nothing");
    assert_eq!(e.mouse_hints().left, "final point");
    assert_eq!(e.mouse_hints().right, "cancel");
    assert!(!e.overlay().preview.is_empty(), "the rubber band shows");
    // A release after a drag does not finish it either.
    ev(
        &mut e,
        ToolEvent::Release {
            pos: Point::new(3000.0, 2400.0),
            button: Button::Left,
        },
    );
    assert!(e.take_requests().is_empty());
    // The second click opens the dialog for that frame.
    ev(&mut e, press(3000.0, 2400.0, Button::Left));
    let reqs = e.take_requests();
    assert!(
        matches!(
            reqs.as_slice(),
            [UiRequest::PlotDialog { rect: Some(r), edit: None }]
                if (r.width() - 2400.0).abs() < 1e-6 && (r.height() - 1800.0).abs() < 1e-6
        ),
        "{reqs:?}"
    );
    assert!(e.overlay().preview.is_empty());
    // The right button cancels the frame in progress.
    ev(&mut e, press(100.0, 100.0, Button::Left));
    ev(&mut e, press(500.0, 500.0, Button::Right));
    ev(&mut e, press(900.0, 900.0, Button::Left));
    assert!(e.take_requests().is_empty(), "the click after cancel is a first corner");
    ev(&mut e, press(900.0, 900.0, Button::Right));
    // The same point twice is no frame.
    ev(&mut e, press(100.0, 100.0, Button::Left));
    ev(&mut e, press(100.0, 100.0, Button::Left));
    assert!(e.take_requests().is_empty());
    // Nothing is drawn until the dialog runs plot.add.
    assert!(e.doc().objects.is_empty());
    // Nothing is drawn until the dialog runs plot.add.
    assert!(e.doc().objects.is_empty());
    // plot.panel and plot.edit.
    e.execute("plot.panel", &json!({})).unwrap();
    assert_eq!(
        e.take_requests(),
        vec![UiRequest::PlotDialog { rect: None, edit: None }]
    );
    let id = sine(&mut e);
    e.execute("plot.edit", &json!({"id": id})).unwrap();
    assert_eq!(
        e.take_requests(),
        vec![UiRequest::PlotDialog {
            rect: None,
            edit: Some(ObjectId(id))
        }]
    );
    // Edit mode's left button on a plot opens the dialog, not the attribute panel.
    e.set_tool(ToolKind::Edit);
    ev(
        &mut e,
        ToolEvent::Press {
            pos: Point::new(600.0, 600.0),
            button: Button::Left,
            shift: false,
        },
    );
    let reqs = e.take_requests();
    assert!(
        matches!(reqs.as_slice(), [UiRequest::PlotDialog { edit: Some(_), .. }]),
        "{reqs:?}"
    );
}

#[test]
fn hostile_parameters_are_errors() {
    let mut e = Engine::new();
    for p in [
        json!({"expr": "x".repeat(100_000)}),
        json!({"expr": "(".repeat(100_000)}),
        json!({"expr": "1/0"}),
        json!({"expr": "sqrt(-1)"}),
        json!({"expr": "x", "xmin": 5, "xmax": 1}),
        json!({"expr": "x", "samples": 0}),
        json!({"expr": "x", "samples": 1e9}),
        json!({"expr": "x", "samples": -3}),
        json!({"expr": "x", "xmin": 1e300, "xmax": 1e301}),
        json!({"expr": "x", "log_x": true, "xmin": -1, "xmax": 1}),
        json!({"expr": "x", "width": 0}),
        json!({"expr": "x", "width": 1e12}),
        json!({"expr": "x", "xlabel": "y".repeat(100_000)}),
        json!({"expr": "x", "xlabel": "y".repeat(990)}),
        json!({"mode": "nope"}),
        json!({"mode": 5}),
        json!({"expr": 5}),
        json!({"expr": "x", "xcol": "a"}),
        json!({"expr": "x", "ymin": "a"}),
    ] {
        let n = e.doc().objects.len();
        let r = e.execute("plot.add", &p);
        assert!(r.is_err(), "{p} should fail");
        assert_eq!(e.doc().objects.len(), n);
    }
}

mod fuzz {
    use super::*;
    use proptest::prelude::*;

    const TOKENS: &[&str] = &[
        "x", "t", "pi", "e", "1", "0", "2.5", "1e308", "1e-308", "sin", "cos", "tan", "exp", "log", "sqrt", "abs",
        "atan2", "max", "pow", "floor", "(", ")", ",", "+", "-", "*", "/", "^", "**", " ", ".", "\u{d7}", "\u{2212}",
    ];

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// Hostile expressions: an error or a plot, never a panic or a hang.
        #[test]
        fn token_soup_expressions(toks in proptest::collection::vec(proptest::sample::select(TOKENS), 0..60),
                                  log in any::<bool>(), mode in 0u8..2) {
            let expr: String = toks.concat();
            let mut e = Engine::new();
            let p = if mode == 0 {
                json!({"expr": expr, "log_x": log, "log_y": log, "xmin": 0.001, "xmax": 50, "samples": 300})
            } else {
                json!({"mode": "parametric", "xt": expr, "yt": expr, "log_x": log})
            };
            let _ = e.execute("plot.add", &p);
        }

        #[test]
        fn random_text_expressions(expr in "[ -~\u{d7}\u{f7}\u{2212}]{0,120}") {
            let mut e = Engine::new();
            let _ = e.execute("plot.add", &json!({"expr": expr, "samples": 200}));
        }

        /// Hostile data text and columns.
        #[test]
        fn random_data(data in "[ -~\n\t]{0,400}", xcol in 0u32..6, ycol in 0u32..6) {
            let mut e = Engine::new();
            let _ = e.execute("plot.add", &json!({"mode": "data", "data": data, "xcol": xcol, "ycol": ycol,
                "xmin": null, "xmax": null, "log_x": xcol % 2 == 0, "log_y": ycol % 2 == 0}));
        }

        /// Hostile numbers in the ranges, sizes and sample counts.
        #[test]
        fn random_numbers(a in -1e12f64..1e12, b in -1e12f64..1e12, w in -1e6f64..1e6, n in -5i64..30_000) {
            let mut e = Engine::new();
            let _ = e.execute("plot.add", &json!({"expr": "sin(x)/x", "xmin": a, "xmax": b, "ymin": b, "ymax": a,
                "width": w, "height": a, "samples": n}));
        }
    }
}

#[test]
fn the_largest_expression_with_the_most_samples_finishes() {
    let start = std::time::Instant::now();
    let mut e = Engine::new();
    let expr = format!("sin(x){}", "+cos(x)*1/x".repeat(60));
    let _ = e.execute(
        "plot.add",
        &json!({"expr": expr, "xmin": -1, "xmax": 1, "samples": 20_000}),
    );
    assert!(start.elapsed().as_secs() < 60, "took {:?}", start.elapsed());
    // Over the node cap: an error.
    let big = format!("x{}", "+x".repeat(1_100));
    assert!(e.execute("plot.add", &json!({"expr": big})).is_err());
}
