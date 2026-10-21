#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! `file.rename`: the saved file moves on disk; an unsaved figure only gets a name.

use rustfig_engine::Engine;
use serde_json::json;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rustfig-rename-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn saved(dir: &std::path::Path, file: &str) -> (Engine, String) {
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 50}))
        .unwrap();
    let path = dir.join(file).to_string_lossy().into_owned();
    e.execute("file.save", &json!({"path": path})).unwrap();
    (e, path)
}

#[test]
fn bare_name_renames_in_place_and_keeps_extension() {
    let d = scratch("bare");
    let (mut e, old) = saved(&d, "a.fig");
    let r = e.execute("file.rename", &json!({"name": "b"})).unwrap();
    let new = d.join("b.fig").to_string_lossy().into_owned();
    assert_eq!(r["path"], new.as_str());
    assert!(!std::path::Path::new(&old).exists());
    assert!(d.join("b.fig").is_file());
    assert_eq!(e.path(), Some(new.as_str()));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn typed_path_moves_and_existing_target_asks() {
    let d = scratch("move");
    let (mut e, old) = saved(&d, "a.fig");
    std::fs::create_dir_all(d.join("sub")).unwrap();
    let sub = d.join("sub").join("a.fig");
    std::fs::write(&sub, "x").unwrap();
    let target = sub.to_string_lossy().into_owned();
    let err = e.execute("file.rename", &json!({"name": target})).unwrap_err();
    assert_eq!(err.code, "exists");
    assert!(std::path::Path::new(&old).exists());
    assert_eq!(e.path(), Some(old.as_str()));
    e.execute("file.rename", &json!({"name": target, "overwrite": true}))
        .unwrap();
    assert!(!std::path::Path::new(&old).exists());
    assert_ne!(std::fs::read_to_string(&sub).unwrap(), "x");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn rename_with_save_writes_unsaved_changes() {
    let d = scratch("save");
    let (mut e, _) = saved(&d, "a.fig");
    e.execute("shape.circle", &json!({"cx": 500, "cy": 500, "r": 100}))
        .unwrap();
    assert!(e.is_modified());
    e.execute("file.rename", &json!({"name": "c", "save": true})).unwrap();
    assert!(!e.is_modified());
    assert!(!d.join("a.fig").exists());
    assert!(std::fs::read_to_string(d.join("c.fig")).unwrap().contains("500"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn unsaved_figure_only_gets_a_name() {
    let mut e = Engine::new();
    e.execute("file.rename", &json!({"name": "drawing"})).unwrap();
    assert_eq!(e.path(), Some("drawing.fig"));
    assert!(!std::path::Path::new("drawing.fig").exists());
}

#[test]
fn hostile_names_fail_cleanly() {
    let d = scratch("hostile");
    let (mut e, old) = saved(&d, "a.fig");
    for name in [
        json!(""),
        json!("   "),
        json!(".."),
        json!("x/"),
        json!("a\u{0}b"),
        json!("a\nb"),
        json!("x".repeat(10_000)),
        json!(5),
        json!(null),
        json!([]),
        json!("b.rustfig"),
        json!(d.to_string_lossy()),
    ] {
        assert!(e.execute("file.rename", &json!({ "name": name })).is_err(), "{name}");
        assert_eq!(e.path(), Some(old.as_str()));
        assert!(std::path::Path::new(&old).exists());
    }
    assert!(e
        .execute("file.rename", &json!({"name": "q", "overwrite": "yes"}))
        .is_err());
    assert!(e.execute("file.rename", &json!({})).is_err());
    let _ = std::fs::remove_dir_all(&d);
}
