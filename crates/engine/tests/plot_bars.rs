#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Bars, stems and error bars (plan Q4): inline data and files, grouped
//! bars, kept in the figure.

use rustfig_engine::Engine;
use rustfig_model::{Object, ObjectId, ObjectKind, PolyKind, Polyline};
use serde_json::{json, Value};

fn members(o: &Object) -> &[Object] {
    match &o.kind {
        ObjectKind::Compound(c) => &c.objects,
        _ => panic!("not a compound"),
    }
}

fn parts<'a>(o: &'a Object, kind: &str) -> Vec<&'a Polyline> {
    members(o)
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
    let mut q = json!({"x": 0, "y": 0, "width": 4000, "height": 3000});
    for (k, v) in p.as_object().unwrap() {
        q[k] = v.clone();
    }
    e.execute("plot.add", &q)
        .map(|r| ObjectId(r["id"].as_u64().unwrap()))
        .map_err(|e| e.to_string())
}

fn bounds(p: &Polyline) -> (f64, f64, f64, f64) {
    let xs: Vec<f64> = p.points.iter().map(|q| q.x).collect();
    let ys: Vec<f64> = p.points.iter().map(|q| q.y).collect();
    (
        xs.iter().cloned().fold(f64::MAX, f64::min),
        ys.iter().cloned().fold(f64::MAX, f64::min),
        xs.iter().cloned().fold(f64::MIN, f64::max),
        ys.iter().cloned().fold(f64::MIN, f64::max),
    )
}

const DATA: &str = "1 2\n2 4\n3 3\n4 1";

fn bar_plot(e: &mut Engine, extra: Value) -> ObjectId {
    let mut p = json!({"xmin": 0.5, "xmax": 4.5, "ymin": 0, "ymax": 4,
        "curves": [{"data": DATA, "style": "bar", "pen_color": 4}]});
    for (k, v) in extra.as_object().unwrap() {
        p[k] = v.clone();
    }
    add(e, p).unwrap()
}

#[test]
fn bars_stand_on_the_baseline() {
    let mut e = Engine::new();
    let id = bar_plot(&mut e, json!({}));
    let o = e.doc().get(id).unwrap();
    let bars = parts(o, "bar");
    assert_eq!(bars.len(), 4);
    // x 0.5..4.5 over 4000: one unit is 1000; a bar is 0.8 wide, centered.
    let (x0, y0, x1, y1) = bounds(bars[0]);
    assert_eq!((x0, x1), (100.0, 900.0));
    // y 0..4 over 3000: the bar 2 high is 1500, standing on the bottom edge.
    assert_eq!((y0, y1), (1500.0, 3000.0));
    assert_eq!(bars[0].kind, PolyKind::Box);
    assert_eq!(bars[0].style.fill_color.0, 4);
    assert_eq!(bars[0].style.area_fill, 20);
    assert!(parts(o, "curve").is_empty(), "no line through the tops");
    let tops: Vec<f64> = bars.iter().map(|b| bounds(b).1).collect();
    assert_eq!(tops, [1500.0, 0.0, 750.0, 2250.0]);
    // Width and fill density are settings.
    let id = bar_plot(
        &mut e,
        json!({"curves": [{"data": DATA, "style": "bar", "bar_width": 0.5, "fill": {"color": 2, "density": 30}}]}),
    );
    let b = parts(e.doc().get(id).unwrap(), "bar")[0];
    assert_eq!(bounds(b).2 - bounds(b).0, 500.0);
    assert_eq!((b.style.fill_color.0, b.style.area_fill), (2, 30));
    // A baseline: bars hang from it, negative ones too.
    let id = bar_plot(
        &mut e,
        json!({"ymin": -4, "ymax": 4, "curves": [{"points": [[1, -2], [2, 2]], "style": "bar", "baseline": 1}]}),
    );
    let bars = parts(e.doc().get(id).unwrap(), "bar");
    let (_, a0, _, a1) = bounds(bars[0]);
    assert_eq!((a0, a1), (1125.0, 2250.0));
    assert_eq!(a1 - a0, 1125.0, "from y = -2 up to the baseline 1");
}

#[test]
fn bars_are_cut_at_the_window_and_the_range_fits_them() {
    let mut e = Engine::new();
    let id = bar_plot(&mut e, json!({"ymax": 2.5}));
    let bars = parts(e.doc().get(id).unwrap(), "bar");
    assert!(bars.iter().all(|b| bounds(b).1 >= 0.0), "{bars:?}");
    // Without ranges everything fits: half a bar at each end, the baseline.
    let id = add(
        &mut e,
        json!({"curves": [{"data": "10 5\n11 8\n12 6", "style": "bar"}], "xmin": null, "xmax": null, "ymin": null, "ymax": null}),
    )
    .unwrap();
    let o = e.doc().get(id).unwrap();
    let bars = parts(o, "bar");
    assert_eq!(bars.len(), 3);
    for b in &bars {
        let (x0, y0, x1, y1) = bounds(b);
        assert!(x0 >= 0.0 && x1 <= 4000.0 && y0 >= 0.0 && y1 <= 3000.0);
        assert_eq!(y1, 3000.0, "the baseline 0 is in the range");
    }
}

#[test]
fn grouped_bars_stand_side_by_side() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"xmin": 0.5, "xmax": 4.5, "ymin": 0, "ymax": 4, "curves": [
            {"points": [[1, 1], [2, 2], [3, 3]], "style": "bar", "pen_color": 1},
            {"points": [[1, 2], [2, 3], [3, 1]], "style": "bar", "pen_color": 2}]}),
    )
    .unwrap();
    let o = e.doc().get(id).unwrap();
    let bars = parts(o, "bar");
    assert_eq!(bars.len(), 6);
    let first: Vec<_> = bars.iter().filter(|b| b.style.fill_color.0 == 1).collect();
    let second: Vec<_> = bars.iter().filter(|b| b.style.fill_color.0 == 2).collect();
    for (a, b) in first.iter().zip(&second) {
        let (a, b) = (bounds(a), bounds(b));
        assert!(a.2 <= b.0 + 1e-9, "no overlap: {a:?} {b:?}");
        assert!(((a.2 - a.0) - (b.2 - b.0)).abs() <= 1.0);
        // Together they are centered on x = 1000 * k - 500 and 0.8 wide.
        let center = 0.5 * (a.0 + b.2);
        assert!(((center + 500.0) % 1000.0).abs() < 1e-6 || ((center + 500.0) % 1000.0 - 1000.0).abs() < 1e-6);
        assert!((b.2 - a.0 - 800.0).abs() < 1.0);
    }
}

#[test]
fn stems_with_markers() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"xmin": 0, "xmax": 5, "ymin": -1, "ymax": 3,
               "curves": [{"points": [[1, 2], [2, -1], [3, 1.5], [9, 1]], "style": "stem", "marker": "circle"}]}),
    )
    .unwrap();
    let o = e.doc().get(id).unwrap();
    let stems = parts(o, "stem");
    assert_eq!(stems.len(), 3, "the point at x = 9 is outside");
    for s in &stems {
        assert_eq!(s.points.len(), 2);
        assert_eq!(s.points[0].x, s.points[1].x, "vertical");
    }
    // From the baseline y = 0 (750 from the bottom of 3000 / 4 per unit).
    assert!(stems.iter().all(|s| s.points[0].y == 2250.0));
    assert_eq!(
        parts(o, "marker").len()
            + members(o)
                .iter()
                .filter(|m| matches!(m.kind, ObjectKind::Ellipse(_)))
                .count(),
        3
    );
    assert!(parts(o, "curve").is_empty());
}

#[test]
fn error_bars_from_a_list_and_from_a_column() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"xmin": 0, "xmax": 4, "ymin": 0, "ymax": 4, "curves": [
            {"points": [[1, 1], [2, 2], [3, 3]], "yerr": [0.5, 1, 0.25], "marker": "circle", "line": "none"}]}),
    )
    .unwrap();
    let o = e.doc().get(id).unwrap();
    let eb = parts(o, "errorbar");
    assert_eq!(eb.len(), 9, "a line and two caps per point");
    let line = eb.iter().find(|p| p.points[0].x == p.points[1].x).unwrap();
    // y = 1 +- 0.5 on 3000 / 4 = 750 a unit: 1125 to 1875 from 3000.
    assert_eq!(bounds(line).1, 3000.0 - 1.5 * 750.0);
    assert_eq!(bounds(line).3, 3000.0 - 0.5 * 750.0);
    let caps: Vec<_> = eb.iter().filter(|p| p.points[0].y == p.points[1].y).collect();
    assert_eq!(caps.len(), 6);
    assert!(caps.iter().all(|c| bounds(c).2 - bounds(c).0 == 60.0));
    // One value for all points.
    let id = add(
        &mut e,
        json!({"xmin": 0, "xmax": 4, "ymin": 0, "ymax": 4, "curves": [{"points": [[1, 1], [2, 2]], "yerr": [0.5]}]}),
    )
    .unwrap();
    assert_eq!(parts(e.doc().get(id).unwrap(), "errorbar").len(), 6);
    // Inline data with an error column: kept as a list, redraw works without the text.
    let id = add(
        &mut e,
        json!({"xmin": 0, "xmax": 4, "ymin": 0, "ymax": 4,
               "curves": [{"data": "1 1 0.5\n2 2 0.5\n3 3 1", "yerr_col": 3, "style": "bar"}]}),
    )
    .unwrap();
    let o = e.doc().get(id).unwrap();
    assert_eq!(parts(o, "errorbar").len(), 9);
    assert_eq!(parts(o, "bar").len(), 3);
    assert!(
        o.comments
            .iter()
            .any(|c| c.starts_with("rustfig-plot-yerr: 0 0.5 0.5 1")),
        "{:?}",
        o.comments
    );
    assert!(
        !o.comments[0].contains("yerr\""),
        "the values are not in the main comment"
    );
    e.execute("plot.update", &json!({"id": id.0, "xmax": 5})).unwrap();
    let o = e.doc().get(id).unwrap();
    assert_eq!(parts(o, "errorbar").len(), 9);
    assert_eq!(
        o.comments
            .iter()
            .filter(|c| c.starts_with("rustfig-plot-yerr:"))
            .count(),
        1
    );
    let got = e.execute("plot.get", &json!({"id": id.0})).unwrap();
    assert_eq!(got["spec"]["curves"][0]["yerr"], json!([0.5, 0.5, 1.0]));
    // Error bars cut at the window; caps only where the end is inside.
    let id = add(
        &mut e,
        json!({"xmin": 0, "xmax": 4, "ymin": 0, "ymax": 4, "curves": [{"points": [[1, 3.5]], "yerr": [2]}]}),
    )
    .unwrap();
    let eb = parts(e.doc().get(id).unwrap(), "errorbar");
    assert_eq!(eb.len(), 2, "the line and the lower cap");
    assert_eq!(bounds(eb[0]).1, 0.0);
}

#[test]
fn data_from_a_file_with_an_error_column() {
    let dir = std::env::temp_dir().join(format!("rustfig-q4-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("bars.csv");
    std::fs::write(&file, "x,y,err\n1,2,0.5\n2,3,0.25\n3,1,0.5\n").unwrap();
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"xmin": 0, "xmax": 4, "ymin": 0, "ymax": 4,
               "curves": [{"file": file.to_str().unwrap(), "style": "bar", "yerr_col": 3}]}),
    )
    .unwrap();
    let o = e.doc().get(id).unwrap();
    assert_eq!(parts(o, "bar").len(), 3);
    assert_eq!(parts(o, "errorbar").len(), 9);
    // Files are read again at every redraw, not kept.
    assert!(!o.comments.iter().any(|c| c.starts_with("rustfig-plot-yerr:")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn old_plots_keep_their_lines_and_a_comment_without_the_new_fields() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"curves": [{"points": [[1, 1], [2, 3]]}], "xmin": 0, "xmax": 3}),
    )
    .unwrap();
    let o = e.doc().get(id).unwrap();
    assert_eq!(parts(o, "curve").len(), 1);
    assert!(parts(o, "bar").is_empty() && parts(o, "errorbar").is_empty());
    for k in ["style", "bar_width", "baseline", "yerr", "err_cap"] {
        assert!(!o.comments[0].contains(k), "{k}: {}", o.comments[0]);
    }
}

#[test]
fn hostile_bar_parameters_are_errors() {
    let mut e = Engine::new();
    let many: Vec<Value> = (0..6000).map(|i| json!([i, 1])).collect();
    let cases = [
        json!({"style": "pie", "points": [[1, 1]]}),
        json!({"style": "bar", "bar_width": 0, "points": [[1, 1]]}),
        json!({"style": "bar", "bar_width": 5, "points": [[1, 1]]}),
        json!({"style": 1, "points": [[1, 1]]}),
        json!({"yerr": [1], "expr": "x"}),
        json!({"yerr_col": 3, "expr": "x"}),
        json!({"yerr_col": 5000, "points": [[1, 1]]}),
        json!({"yerr": [1e300], "points": [[1, 1]]}),
        json!({"yerr": ["a"], "points": [[1, 1]]}),
        json!({"err_cap": -1, "yerr": [1], "points": [[1, 1]]}),
        json!({"baseline": 1e300, "style": "bar", "points": [[1, 1]]}),
        json!({"style": "bar", "points": many.clone()}),
        json!({"style": "stem", "expr": "x", "samples": 20000}),
    ];
    for c in cases {
        let n = e.doc().objects.len();
        assert!(add(&mut e, json!({"curves": [c.clone()]})).is_err(), "{c}");
        assert_eq!(e.doc().objects.len(), n);
    }
    // Bars at the same x, a single point and log axes never panic.
    for (lx, ly) in [(false, false), (true, false), (false, true), (true, true)] {
        let _ = add(
            &mut e,
            json!({"log_x": lx, "log_y": ly, "xmin": 0.5, "xmax": 10, "ymin": 0.5, "ymax": 10, "curves": [
                {"points": [[1, 2], [1, 3], [5, 0.1]], "style": "bar", "yerr": [5, 1, 0.01]},
                {"points": [[3, 2]], "style": "stem", "baseline": -5}]}),
        );
    }
}

mod fuzz {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Random bars, stems and errors: an error or a plot, never a panic.
        #[test]
        fn random_series(xs in proptest::collection::vec(-1e6f64..1e6, 0..30),
                         ys in proptest::collection::vec(-1e6f64..1e6, 0..30),
                         es in proptest::collection::vec(-1e6f64..1e6, 0..30),
                         style in 0u8..3, w in -1.0f64..2.0, base in -1e9f64..1e9,
                         lx in any::<bool>(), ly in any::<bool>(), cols in 0usize..6) {
            let pts: Vec<Value> = xs.iter().zip(&ys).map(|(x, y)| json!([x, y])).collect();
            let style = ["line", "bar", "stem"][style as usize];
            let mut e = Engine::new();
            let _ = e.execute("plot.add", &json!({"log_x": lx, "log_y": ly, "curves": [
                {"points": pts, "style": style, "bar_width": w, "baseline": base, "yerr": es},
                {"data": "1 2 3\n4 5 6\nx y z\n7 8 9", "xcol": cols, "ycol": cols, "yerr_col": cols, "style": style}]}));
        }
    }
}
