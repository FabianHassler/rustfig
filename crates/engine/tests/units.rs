#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! xfig's Unit/Scale settings: the units box label and `view.set_scale`.

use rustfig_engine::Engine;
use serde_json::json;

#[test]
fn units_box_label_follows_units_and_scale() {
    let mut e = Engine::new();
    assert_eq!(e.unit_label(), "1in = 1.00in");
    e.execute("doc.set_units", &json!({"units": "metric"})).unwrap();
    assert_eq!(e.unit_label(), "1cm = 1.00cm");
    e.execute("view.set_scale", &json!({"unit": "m", "scale": 2.0}))
        .unwrap();
    assert_eq!(e.unit_label(), "1cm = 2.00m");
    for bad in [
        json!({"scale": 0}),
        json!({"scale": -3}),
        json!({"scale": f64::MAX}),
        json!({"unit": "a very long unit name"}),
        json!({"unit": "a\nb"}),
        json!({"decimal": "yes"}),
    ] {
        assert!(e.execute("view.set_scale", &bad).is_err(), "{bad}");
    }
    assert_eq!(e.unit_label(), "1cm = 2.00m");
}

/// Setting one preference applies that one only: the attribute panel and
/// the zoom keep what the user set, even with start values stored.
#[test]
fn setting_one_preference_keeps_the_others() {
    let mut e = rustfig_engine::Engine::new();
    e.execute(
        "prefs.set",
        &serde_json::json!({"resource": "startfontsize", "value": 20}),
    )
    .unwrap();
    e.execute("prefs.set", &serde_json::json!({"resource": "zoom", "value": 2.0}))
        .unwrap();
    e.indicator.font_size = 33.0;
    e.view.zoom = 0.5;
    e.execute("prefs.set", &serde_json::json!({"resource": "inches", "value": false}))
        .unwrap();
    assert_eq!(e.indicator.font_size, 33.0);
    assert_eq!(e.view.zoom, 0.5);
    assert_eq!(
        e.doc().settings.units,
        rustfig_model::Units::Metric,
        "the set one applies"
    );
}
