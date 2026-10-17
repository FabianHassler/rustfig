//! B0: xfig's export languages through `file.export`, `file.export_data`
//! and `file.export_formats`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::fig2dev::{language_available, languages::LANGUAGES};
use rustfig_engine::Engine;
use serde_json::json;

fn drawing() -> Engine {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 2400, "height": 1200}),
    )
    .unwrap();
    e
}

#[test]
fn formats_list_every_language_in_xfigs_order() {
    let mut e = Engine::new();
    let v = e.execute("file.export_formats", &json!({})).unwrap();
    let formats = v["formats"].as_array().unwrap();
    let ids: Vec<&str> = formats.iter().map(|f| f["id"].as_str().unwrap()).collect();
    let expected: Vec<&str> = LANGUAGES.iter().map(|l| l.id).collect();
    assert_eq!(ids, expected);
    assert_eq!(ids.first(), Some(&"ps"));
    for f in formats {
        let available = f["available"].as_bool().unwrap();
        assert_eq!(available, f["status"] != "planned", "{f}");
        if f["status"] != "available" {
            assert!(f["task"].as_str().is_some(), "{f}");
        }
    }
}

#[test]
fn every_available_language_exports() {
    let mut e = drawing();
    for l in LANGUAGES.iter().filter(|l| language_available(l)) {
        let r = e.execute("file.export_data", &json!({"format": l.id}));
        if l.id == "shape" {
            // fig2dev's shape driver needs the figure's shape comments.
            assert!(r.unwrap_err().message.contains("No shape found"));
            continue;
        }
        let r = r.unwrap();
        if l.id == "pspdf" {
            let files = r["files"].as_array().unwrap();
            assert_eq!(files.len(), 2);
            assert_eq!(files[0]["extension"], "eps");
            assert!(files[1]["base64"].as_str().is_some());
        } else if let Some(files) = r["files"].as_array() {
            // The combined LaTeX languages: graphics and LaTeX part.
            assert!(files.len() >= 2, "{}", l.id);
        } else {
            assert!(r.get("data").or(r.get("base64")).is_some(), "{}", l.id);
        }
    }
}

#[test]
fn latex_box_has_the_figures_size() {
    let mut e = drawing();
    let r = e.execute("file.export_data", &json!({"format": "box"})).unwrap();
    let text = r["data"].as_str().unwrap();
    assert!(text.starts_with("\\makebox["), "{text}");
    assert!(text.contains("in]{\\rule{0in}{"), "{text}");
}

#[test]
fn planned_and_unknown_languages_are_errors() {
    let mut e = drawing();
    let err = e.execute("file.export_data", &json!({"format": "emf"})).unwrap_err();
    assert!(err.message.contains("plan task B9"), "{}", err.message);
    for bad in [
        "",
        "bogus",
        "../../etc/passwd",
        "svg; rm -rf /",
        "ŝvg",
        &"x".repeat(10_000),
    ] {
        assert!(
            e.execute("file.export_data", &json!({"format": bad})).is_err(),
            "{bad:?}"
        );
    }
    assert!(e.execute("file.export_data", &json!({"format": 3})).is_err());
}

#[test]
fn files_pick_the_language_from_their_name() {
    let dir = std::env::temp_dir().join(format!("rustfig-export-languages-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut e = drawing();
    let path = |name: &str| dir.join(name).to_string_lossy().into_owned();

    let r = e.execute("file.export", &json!({"path": path("fig.box")})).unwrap();
    assert_eq!(r["format"], "box");
    let r = e
        .execute("file.export", &json!({"path": path("both.eps"), "format": "pspdf"}))
        .unwrap();
    assert_eq!(r["paths"].as_array().unwrap().len(), 2);
    assert!(dir.join("both.eps").is_file() && dir.join("both.pdf").is_file());

    let err = e.execute("file.export", &json!({"path": path("fig.tex")})).unwrap_err();
    assert!(err.message.contains("format"), "{}", err.message);
    let err = e.execute("file.export", &json!({"path": path("fig.emf")})).unwrap_err();
    assert!(err.message.contains("B9"), "{}", err.message);
    assert!(!dir.join("fig.emf").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

fn eps(e: &mut Engine, options: serde_json::Value) -> String {
    let r = e
        .execute("file.export_data", &json!({"format": "eps", "options": options}))
        .unwrap();
    r["data"].as_str().unwrap().to_string()
}

fn bbox(eps: &str) -> (i64, i64) {
    let line = eps.lines().find(|l| l.starts_with("%%BoundingBox:")).unwrap();
    let v: Vec<i64> = line.split_whitespace().skip(1).map(|n| n.parse().unwrap()).collect();
    (v[2], v[3])
}

#[test]
fn fig2dev_options_change_the_output() {
    let mut e = drawing();
    let plain = eps(&mut e, json!({}));
    let (w, h) = bbox(&plain);
    // -b: a border of 10 points on every side.
    let (bw, bh) = bbox(&eps(&mut e, json!({"border": 10})));
    assert_eq!((bw - w, bh - h), (20, 20));
    // -m 2 doubles the figure; -Z 4w makes it 4 inches wide (padding included).
    let (mw, _) = bbox(&eps(&mut e, json!({"m": 2})));
    assert!((mw - 2 * w).abs() <= 2, "{mw} {w}");
    let (zw, _) = bbox(&eps(&mut e, json!({"max_size": "4w"})));
    assert!((zw - 288).abs() <= 1, "{zw}");
    // -n, -g, -N.
    assert!(eps(&mut e, json!({"title": "Figure 1"})).contains("%%Title: Figure 1"));
    let bg = eps(&mut e, json!({"background": "red"}));
    let gray = eps(&mut e, json!({"background": "red", "N": true}));
    assert_ne!(bg, gray);
    assert_ne!(bg, plain);
    // Options may also be given by letter, with a dash.
    assert_eq!(eps(&mut e, json!({"-b": 10})), eps(&mut e, json!({"border": 10})));
}

#[test]
fn depth_options_select_objects_and_bounds() {
    let mut e = drawing();
    let r = e
        .execute(
            "shape.rectangle",
            &json!({"x": 6000, "y": 0, "width": 1200, "height": 1200}),
        )
        .unwrap();
    let id = r["id"].as_u64().unwrap();
    e.execute("arrange.set_depth", &json!({"ids": [id], "depth": 10}))
        .unwrap();
    let all = eps(&mut e, json!({}));
    let only_50 = eps(&mut e, json!({"depths": "+50"}));
    let only_50_adjusted = eps(&mut e, json!({"depths": "+50", "K": true}));
    // Without -K the page keeps the whole figure; with it, only depth 50 counts.
    assert_ne!(only_50, all);
    assert_eq!(bbox(&all), bbox(&only_50));
    assert!(bbox(&only_50_adjusted).0 < bbox(&all).0 / 2);
    assert_eq!(
        bbox(&eps(&mut e, json!({"depths": "-10", "K": true}))),
        bbox(&only_50_adjusted)
    );
}

#[test]
fn bad_options_are_refused() {
    let mut e = drawing();
    let err = |e: &mut Engine, format: &str, options: serde_json::Value| {
        e.execute("file.export_data", &json!({"format": format, "options": options}))
            .unwrap_err()
            .message
    };
    assert!(err(&mut e, "eps", json!({"nonsense": 1})).contains("no option"));
    // Options not ported yet: fig2dev's driver warns and goes on.
    let r = e
        .execute(
            "file.export_data",
            &json!({"format": "eps", "options": {"ascii_preview": true}}),
        )
        .unwrap();
    assert!(r["warnings"][0].as_str().unwrap().contains("B4"), "{r}");
    assert!(err(&mut e, "svg", json!({"title": "x"})).contains("no option"));
    assert!(err(&mut e, "eps", json!({"border": "wide"})).contains("number"));
    // fig2dev takes -m and -Z together; -Z sizes the figure.
    assert!(e
        .execute("file.export_data", &json!({"format": "eps", "options": [1]}))
        .is_err());
}

#[test]
fn formats_list_each_languages_options() {
    let mut e = Engine::new();
    let v = e.execute("file.export_formats", &json!({})).unwrap();
    let eps = v["formats"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "eps")
        .unwrap();
    let keys: Vec<&str> = eps["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["key"].as_str().unwrap())
        .collect();
    assert!(keys.contains(&"border") && keys.contains(&"depths") && keys.contains(&"bbox"));
    let bbox = eps["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["key"] == "bbox")
        .unwrap();
    assert_eq!(bbox["available"], false);
    assert_eq!(bbox["task"], "B4");
}
