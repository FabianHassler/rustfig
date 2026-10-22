#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! S4-8: Export line as data: figure and calibrated units, numbered files, append.

use rustfig_engine::Engine;
use serde_json::json;

fn engine() -> (Engine, Vec<u64>) {
    let mut e = Engine::new();
    let a = e
        .execute("shape.polyline", &json!({"points": [[0, 0], [600, 1200], [1200, 600]]}))
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    let b = e
        .execute("shape.polyline", &json!({"points": [[0, 100], [100, 200]]}))
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    (e, vec![a, b])
}

fn scratch(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rustfig-digitize-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn figure_units_numbered_files() {
    let d = scratch("num");
    let (mut e, ids) = engine();
    let prefix = d.join("plot").to_string_lossy().into_owned();
    let r = e
        .execute("file.digitize", &json!({"ids": ids, "prefix": prefix, "start": 7}))
        .unwrap();
    assert_eq!(r["next"], 9);
    assert_eq!(
        std::fs::read_to_string(d.join("plot_0007.xy")).unwrap(),
        "0 0\n600 1200\n1200 600\n"
    );
    assert_eq!(
        std::fs::read_to_string(d.join("plot_0008.xy")).unwrap(),
        "0 100\n100 200\n"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn calibrated_csv_in_data_units() {
    let d = scratch("cal");
    let (mut e, ids) = engine();
    let prefix = d.join("c").to_string_lossy().into_owned();
    // x: 0..1200 is 0..12; y: 1200 is 0, 0 is 10 (the plot's y axis points up).
    let cal = json!({"points": [
        {"x": 0, "y": 1200, "vx": 0, "vy": 0},
        {"x": 1200, "y": 0, "vx": 12, "vy": 10}]});
    e.execute(
        "file.digitize",
        &json!({"ids": [ids[0]], "prefix": prefix, "suffix": "csv", "calibration": cal}),
    )
    .unwrap();
    let t = std::fs::read_to_string(d.join("c_0000.csv")).unwrap();
    assert_eq!(t, "x,y\n0,10\n6,0\n12,5\n");
    // Logarithmic y axis: halfway between 1 and 100 is 10.
    let cal = json!({"log_y": true, "points": [
        {"x": 0, "y": 0, "vx": 0, "vy": 1}, {"x": 1200, "y": 1200, "vx": 1, "vy": 100}]});
    let r = e
        .execute(
            "file.digitize",
            &json!({"ids": [ids[0]], "prefix": prefix, "calibration": cal, "dry_run": true}),
        )
        .unwrap();
    assert!(r["text"].as_str().unwrap().contains("0.5 100\n"), "{r}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn append_mode_adds_to_one_file() {
    let d = scratch("app");
    let (mut e, ids) = engine();
    let prefix = d.join("a").to_string_lossy().into_owned();
    e.execute("file.digitize", &json!({"ids": ids, "prefix": prefix, "append": true}))
        .unwrap();
    e.execute(
        "file.digitize",
        &json!({"ids": [ids[1]], "prefix": prefix, "append": true}),
    )
    .unwrap();
    let t = std::fs::read_to_string(d.join("a_0000.xy")).unwrap();
    assert_eq!(t, "0 0\n600 1200\n1200 600\n\n0 100\n100 200\n\n0 100\n100 200\n");
    assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn only_lines_are_exported() {
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10}))
        .unwrap();
    assert!(e
        .execute("file.digitize", &json!({"prefix": "x", "dry_run": true}))
        .is_err());
}
