//! B0: xfig's export languages through `file.export`, `file.export_data`
//! and `file.export_formats`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::render::languages::LANGUAGES;
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
    for l in LANGUAGES.iter().filter(|l| l.available()) {
        let r = e.execute("file.export_data", &json!({"format": l.id})).unwrap();
        if l.id == "pspdf" {
            let files = r["files"].as_array().unwrap();
            assert_eq!(files.len(), 2);
            assert_eq!(files[0]["extension"], "eps");
            assert!(files[1]["base64"].as_str().is_some());
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
    let err = e.execute("file.export_data", &json!({"format": "tikz"})).unwrap_err();
    assert!(err.message.contains("plan task B1"), "{}", err.message);
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
    let err = e
        .execute("file.export", &json!({"path": path("fig.tikz")}))
        .unwrap_err();
    assert!(err.message.contains("B1"), "{}", err.message);
    assert!(!dir.join("fig.tikz").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}
