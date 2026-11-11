#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Tick label format (plan Q2): scale factor, decimal comma, own labels and
//! categories, rotated numbers; on the canvas (PostScript) and as LaTeX.

use rustfig_engine::Engine;
use rustfig_model::{text_flags, Object, ObjectId, ObjectKind, Text};
use serde_json::{json, Value};

fn numbers(o: &Object) -> Vec<&Text> {
    let ObjectKind::Compound(c) = &o.kind else { panic!() };
    c.objects
        .iter()
        .filter(|m| m.comments.iter().any(|c| c.trim_end().ends_with("part: number")))
        .filter_map(|m| match &m.kind {
            ObjectKind::Text(t) => Some(t),
            _ => None,
        })
        .collect()
}

fn add(e: &mut Engine, extra: Value) -> ObjectId {
    let mut p = json!({"expr": "x", "xmin": 0, "xmax": 4000, "ymin": 0, "ymax": 1, "x": 0, "y": 0,
                       "width": 4000, "height": 2500});
    for (k, v) in extra.as_object().unwrap() {
        p[k] = v.clone();
    }
    let r = e.execute("plot.add", &p).unwrap();
    ObjectId(r["id"].as_u64().unwrap())
}

fn texts(e: &Engine, id: ObjectId) -> Vec<String> {
    numbers(e.doc().get(id).unwrap())
        .iter()
        .map(|t| t.text.clone())
        .collect()
}

#[test]
fn scale_factor_divides_the_numbers_and_labels_the_axis_end() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"xticks": {"manual": true, "step": 1000, "scale": 3}, "yticks": {"numbers": false}}),
    );
    let t = texts(&e, id);
    assert_eq!(t, ["0", "1", "2", "3", "4", "\u{d7}10^3"], "{t:?}");
    // The factor stands right of the axis's end.
    let o = e.doc().get(id).unwrap();
    let last = numbers(o).pop().unwrap().origin;
    assert!(last.x > 4000.0);
    // As LaTeX math.
    let id = add(
        &mut e,
        json!({"xticks": {"manual": true, "step": 1000, "scale": 3}, "yticks": {"numbers": false}, "latex_numbers": true}),
    );
    let t = texts(&e, id);
    assert_eq!(t.last().unwrap(), "$\\times10^{3}$");
    assert_eq!(t[1], "$1$");
    assert!(numbers(e.doc().get(id).unwrap())
        .iter()
        .all(|t| t.flags & text_flags::SPECIAL != 0));
    // The y axis carries it above its end; a negative power divides upwards.
    let id = add(
        &mut e,
        json!({"ymin": 0, "ymax": 0.004, "yticks": {"manual": true, "step": 0.001, "scale": -3}, "xticks": {"numbers": false}}),
    );
    let t = texts(&e, id);
    assert_eq!(t, ["0", "1", "2", "3", "4", "\u{d7}10^-3"], "{t:?}");
    let o = e.doc().get(id).unwrap();
    assert!(numbers(o).last().unwrap().origin.y < 0.0);
    // No scale on a logarithmic axis.
    let id = add(
        &mut e,
        json!({"xmin": 1, "xmax": 1000, "log_x": true, "xticks": {"scale": 3}}),
    );
    assert!(!texts(&e, id).iter().any(|t| t.contains('^') && t.contains('\u{d7}')));
}

#[test]
fn decimal_comma_on_the_canvas_and_in_latex() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"xmin": 0, "xmax": 1, "xticks": {"manual": true, "step": 0.5, "comma": true}, "yticks": {"numbers": false}}),
    );
    assert_eq!(texts(&e, id), ["0", "0,5", "1"]);
    let id = add(
        &mut e,
        json!({"xmin": 0, "xmax": 1, "xticks": {"manual": true, "step": 0.5, "comma": true}, "yticks": {"numbers": false},
               "latex_numbers": true}),
    );
    assert_eq!(texts(&e, id), ["$0$", "$0{,}5$", "$1$"]);
    // A fixed number of decimals and a comma together.
    let id = add(
        &mut e,
        json!({"xmin": 0, "xmax": 1, "xticks": {"manual": true, "step": 0.5, "comma": true, "format": "fixed", "decimals": 2},
               "yticks": {"numbers": false}}),
    );
    assert_eq!(texts(&e, id), ["0,00", "0,50", "1,00"]);
}

#[test]
fn own_labels_and_categories() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"xmin": 0.5, "xmax": 3.5, "xticks": {"labels": [[1, "Mon"], [2, "Tue"], [3, "Wed"], [9, "off the range"]]},
               "yticks": {"numbers": false, "ticks": false}}),
    );
    assert_eq!(texts(&e, id), ["Mon", "Tue", "Wed"]);
    // The ticks stand at the given values, nowhere else.
    let o = e.doc().get(id).unwrap();
    let ObjectKind::Compound(c) = &o.kind else { panic!() };
    let ticks = c
        .objects
        .iter()
        .filter(|m| m.comments.iter().any(|c| c.trim_end().ends_with("part: tick")))
        .count();
    assert_eq!(ticks, 3);
    // Own LaTeX labels are used as typed (special text).
    let id = add(
        &mut e,
        json!({"xmin": 0.5, "xmax": 3.5, "latex_numbers": true,
               "xticks": {"labels": [[1, "$\\alpha$"], [2, "b"]]}, "yticks": {"numbers": false}}),
    );
    assert_eq!(texts(&e, id), ["$\\alpha$", "b"]);
    // Kept in the figure, and a redraw keeps them.
    let got = e.execute("plot.get", &json!({"id": id.0})).unwrap();
    assert_eq!(got["spec"]["xticks"]["labels"][0], json!([1.0, "$\\alpha$"]));
    e.execute("plot.update", &json!({"id": id.0, "grid": true})).unwrap();
    assert_eq!(texts(&e, id), ["$\\alpha$", "b"]);
    // On a log axis the value is the data value.
    let id = add(
        &mut e,
        json!({"xmin": 1, "xmax": 1000, "log_x": true, "xticks": {"labels": [[10, "ten"], [100, "hundred"], [-5, "bad"]]},
               "yticks": {"numbers": false}}),
    );
    assert_eq!(texts(&e, id), ["ten", "hundred"]);
    // numbers off: the ticks stay, the texts go.
    let id = add(
        &mut e,
        json!({"xticks": {"labels": [[1000, "a"]], "numbers": false}, "yticks": {"numbers": false}}),
    );
    assert!(texts(&e, id).is_empty());
}

#[test]
fn rotated_numbers() {
    let mut e = Engine::new();
    let id = add(
        &mut e,
        json!({"xticks": {"manual": true, "step": 1000, "rotate": 45}, "yticks": {"numbers": false}}),
    );
    let o = e.doc().get(id).unwrap();
    let ts = numbers(o);
    assert_eq!(ts.len(), 5);
    for t in &ts {
        assert!((t.angle - 45f64.to_radians()).abs() < 1e-9);
        assert_eq!(t.justify, rustfig_model::Justify::Right, "the text ends under the tick");
    }
    // The y numbers turned vertical.
    let id = add(&mut e, json!({"yticks": {"rotate": 90}, "xticks": {"numbers": false}}));
    let o = e.doc().get(id).unwrap();
    assert!(numbers(o)
        .iter()
        .all(|t| (t.angle - std::f64::consts::FRAC_PI_2).abs() < 1e-9));
    // Clockwise starts under the tick.
    let id = add(&mut e, json!({"xticks": {"rotate": -30}, "yticks": {"numbers": false}}));
    assert!(numbers(e.doc().get(id).unwrap())
        .iter()
        .all(|t| t.justify == rustfig_model::Justify::Left && t.angle < 0.0));
    // Without a rotation the numbers stay centered and flat.
    let id = add(&mut e, json!({"yticks": {"numbers": false}}));
    assert!(numbers(e.doc().get(id).unwrap())
        .iter()
        .all(|t| t.angle == 0.0 && t.justify == rustfig_model::Justify::Center));
}

#[test]
fn old_ticks_read_unchanged_and_the_comment_stays_short() {
    use rustfig_engine::plot::{PlotSpec, PLOT_COMMENT};
    let old = format!(
        "{PLOT_COMMENT} {{\"xticks\":{{\"manual\":true,\"step\":2,\"minor\":1,\"length\":105.0,\"dir\":\"in\",\"format\":\"auto\",\"decimals\":2,\"ticks\":true,\"numbers\":true}}}}"
    );
    let s = PlotSpec::from_comment(&old).unwrap();
    assert!(s.xticks.scale.is_none() && !s.xticks.comma && s.xticks.labels.is_empty() && s.xticks.rotate == 0.0);
    assert_eq!(s.xticks.step, Some(2.0));
    // Only what differs is written.
    let mut s = PlotSpec::default();
    s.xticks.comma = true;
    s.xticks.scale = Some(3);
    let c = s.comment().unwrap();
    assert!(
        c.contains("\"comma\":true") && c.contains("\"scale\":3") && !c.contains("decimals"),
        "{c}"
    );
    assert_eq!(PlotSpec::from_comment(&c).unwrap().xticks, s.xticks);
}

#[test]
fn hostile_tick_formats_are_errors() {
    let mut e = Engine::new();
    let many: Vec<Value> = (0..300).map(|i| json!([i, "x"])).collect();
    for t in [
        json!({"scale": 400}),
        json!({"scale": -400}),
        json!({"scale": 1.5}),
        json!({"rotate": 1e9}),
        json!({"rotate": "left"}),
        json!({"labels": many}),
        json!({"labels": [[1e300, "x"]]}),
        json!({"labels": [["a", "x"]]}),
        json!({"labels": [[1, 2]]}),
        json!({"labels": [[1, "y".repeat(5000)]]}),
        json!({"labels": "abc"}),
        json!({"comma": "yes"}),
    ] {
        let n = e.doc().objects.len();
        let r = e.execute("plot.add", &json!({"expr": "x", "xticks": t}));
        assert!(r.is_err(), "{t} should fail");
        assert_eq!(e.doc().objects.len(), n);
    }
}
