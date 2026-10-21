#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! E15: an export kept up to date on every save.

use rustfig_engine::Engine;
use serde_json::{json, Value};
use std::path::PathBuf;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustfig-e15-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn engine_with_box() -> Engine {
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    e
}

#[test]
fn save_writes_the_figure_and_the_export() {
    let dir = temp_dir("save");
    let fig = dir.join("drawing.fig");
    let svg = dir.join("drawing.svg");
    let mut e = engine_with_box();

    let set = e
        .execute(
            "file.export_on_save",
            &json!({"path": svg.to_str().unwrap(), "format": "svg", "margin": 1.0}),
        )
        .unwrap();
    assert_eq!(set["export_on_save"], true);
    assert!(e.doc().export_on_save.is_some());

    let r = e.execute("file.save", &json!({"path": fig.to_str().unwrap()})).unwrap();
    assert_eq!(r["path"], fig.to_str().unwrap());
    assert!(r.get("export_on_save_error").is_none(), "{r}");
    assert!(fig.exists(), ".fig saved");
    assert!(svg.exists(), "automatic export written");
    assert!(!e.is_modified());

    // The setting is stored as a comment xfig and fig2dev ignore.
    let fig_text = std::fs::read_to_string(&fig).unwrap();
    assert!(fig_text.contains("# RustFigExportOnSave "), "{fig_text}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn changing_and_saving_updates_the_export() {
    let dir = temp_dir("update");
    let fig = dir.join("drawing.fig");
    let svg = dir.join("drawing.svg");
    let mut e = engine_with_box();
    e.execute(
        "file.export_on_save",
        &json!({"path": svg.to_str().unwrap(), "format": "svg"}),
    )
    .unwrap();
    e.execute("file.save", &json!({"path": fig.to_str().unwrap()})).unwrap();
    let first = std::fs::read_to_string(&svg).unwrap();

    e.execute("shape.circle", &json!({"cx": 1200, "cy": 300, "r": 300}))
        .unwrap();
    e.execute("file.save", &json!({})).unwrap();
    let second = std::fs::read_to_string(&svg).unwrap();
    assert_ne!(first, second, "the export follows the changed figure");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn setting_survives_reopening_in_both_formats() {
    let dir = temp_dir("reopen");
    for ext in ["fig", "rustfig"] {
        let fig = dir.join(format!("drawing.{ext}"));
        let png = dir.join("drawing.png");
        let mut e = engine_with_box();
        e.execute(
            "file.export_on_save",
            &json!({"path": png.to_str().unwrap(), "format": "png", "options": {"smoothing": 4}}),
        )
        .unwrap();
        e.execute("file.save", &json!({"path": fig.to_str().unwrap()})).unwrap();

        let mut reopened = Engine::new();
        reopened
            .execute("file.open", &json!({"path": fig.to_str().unwrap()}))
            .unwrap();
        let setting = reopened.doc().export_on_save.clone().expect("setting survived");
        assert_eq!(setting.path, png.to_str().unwrap());
        assert_eq!(
            setting.params.get("format").and_then(Value::as_str),
            Some("png"),
            "{ext}"
        );
        assert_eq!(
            setting
                .params
                .get("options")
                .and_then(|o| o.get("smoothing"))
                .and_then(Value::as_u64),
            Some(4),
            "{ext}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn failing_export_reports_but_the_save_stands() {
    let dir = temp_dir("failure");
    let fig = dir.join("drawing.fig");
    // A directory that does not exist makes the export's write fail.
    let bad = dir.join("no-such-folder").join("drawing.svg");
    let mut e = engine_with_box();
    e.execute(
        "file.export_on_save",
        &json!({"path": bad.to_str().unwrap(), "format": "svg"}),
    )
    .unwrap();

    let r = e.execute("file.save", &json!({"path": fig.to_str().unwrap()})).unwrap();
    assert!(fig.exists(), ".fig saved even though the export failed");
    assert!(!e.is_modified(), "the save itself succeeded");
    assert_eq!(e.path(), Some(fig.to_str().unwrap()));
    assert!(r.get("export_on_save_error").is_some(), "{r}");
    assert!(e.status().contains("automatic export failed"), "{}", e.status());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn command_can_clear_and_report_the_setting() {
    let mut e = Engine::new();
    let none = e.execute("file.export_on_save", &json!({})).unwrap();
    assert_eq!(none["export_on_save"], false);
    e.execute("file.export_on_save", &json!({"path": "figure.svg", "format": "svg"}))
        .unwrap();
    let some = e.execute("file.export_on_save", &json!({})).unwrap();
    assert_eq!(some["export_on_save"], true);
    assert_eq!(some["path"], "figure.svg");
    let off = e.execute("file.export_on_save", &json!({"off": true})).unwrap();
    assert_eq!(off["export_on_save"], false);
    assert!(e.doc().export_on_save.is_none());
}
