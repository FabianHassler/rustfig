#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! W10-16, W10-20: the picture edit panel's commands.

use rustfig_engine::cmd::picture::{picture_type, resize_corner, rotation_of, with_rotation, Resize};
use rustfig_engine::Engine;
use rustfig_geom::Point;
use serde_json::{json, Value};

fn xpm(name: &str, w: usize, h: usize) -> String {
    let row = "a".repeat(w);
    let rows: Vec<String> = (0..h).map(|_| format!("\"{row}\"")).collect();
    let text = format!(
        "/* XPM */\nstatic char *a[] = {{\n\"{w} {h} 2 1\",\n\"a c #000000\",\n\"b c None\",\n{}}};\n",
        rows.join(",\n")
    );
    let path = std::env::temp_dir().join(format!("picture_panel_{}_{name}.xpm", std::process::id()));
    std::fs::write(&path, text).unwrap();
    path.to_string_lossy().into_owned()
}

fn with_picture(name: &str, w: usize, h: usize, box_w: i32, box_h: i32) -> (Engine, u64, String) {
    let path = xpm(name, w, h);
    let mut e = Engine::new();
    let r = e
        .execute(
            "shape.picture",
            &json!({"path": path, "x": 100, "y": 100, "width": box_w, "height": box_h}),
        )
        .unwrap();
    let id = r["id"].as_u64().unwrap();
    (e, id, path)
}

fn corner(e: &mut Engine, id: u64) -> (f64, f64) {
    let r = e.execute("picture.info", &json!({"id": id})).unwrap();
    assert!(r["width"].as_u64().unwrap() > 0);
    let o = e.doc().get(rustfig_model::ObjectId(id)).unwrap();
    let p = o.points().unwrap();
    (p[2].x, p[2].y)
}

#[test]
fn size_buttons_follow_xfig() {
    // 8x4 pixels: original size 120 x 60 units, h/w ratio 0.5.
    let (mut e, id, path) = with_picture("size", 8, 4, 1000, 1000);
    e.execute("picture.resize", &json!({"id": id, "mode": "original"}))
        .unwrap();
    assert_eq!(corner(&mut e, id), (220.0, 160.0));
    e.execute("picture.resize", &json!({"id": id, "mode": "scale", "percent": 200}))
        .unwrap();
    assert_eq!(corner(&mut e, id), (340.0, 220.0));
    // A square box: shrink keeps the width, the height becomes width * ratio.
    e.execute("picture.resize", &json!({"id": id, "mode": "original"}))
        .unwrap();
    e.execute(
        "object.set_geometry",
        &json!({"id": id, "points": [[100, 100], [1100, 1100]]}),
    )
    .unwrap();
    e.execute("picture.resize", &json!({"id": id, "mode": "shrink"}))
        .unwrap();
    assert_eq!(corner(&mut e, id), (1100.0, 600.0));
    // Expand from a box wider than the ratio: the height grows.
    e.execute("picture.resize", &json!({"id": id, "mode": "expand"}))
        .unwrap();
    assert_eq!(corner(&mut e, id), (1100.0, 600.0));
    e.execute("edit.undo", &json!({})).unwrap();
    assert!(e
        .execute("picture.resize", &json!({"id": id, "mode": "bogus"}))
        .is_err());
    std::fs::remove_file(path).ok();
}

#[test]
fn rotation_and_pure_helpers() {
    let a = Point::new(0.0, 0.0);
    let b = Point::new(100.0, 50.0);
    for deg in [0, 90, 180, 270] {
        let q = with_rotation(a, b, deg).unwrap();
        assert_eq!(rotation_of(a, q), deg);
    }
    assert!(with_rotation(a, b, 45).is_none());
    assert!(resize_corner(Resize::Original, a, Point::new(0.0, 5.0), false, 4, 2).is_none());
    assert!(resize_corner(Resize::Original, a, b, false, 0, 0).is_none());
    assert_eq!(picture_type("/no/such/file.XBM"), "XBM");

    let (mut e, id, path) = with_picture("rot", 4, 2, 400, 200);
    e.execute("picture.rotate", &json!({"id": id, "rotation": 180}))
        .unwrap();
    let r: Value = e.execute("picture.info", &json!({"id": id})).unwrap();
    assert_eq!(r["rotation"], 180);
    assert!(e.execute("picture.rotate", &json!({"id": id, "rotation": 45})).is_err());
    std::fs::remove_file(path).ok();
}

#[test]
fn reread_and_relink() {
    let (mut e, id, path) = with_picture("relink_a", 4, 2, 400, 200);
    let other = xpm("relink_b", 6, 3);
    // Missing file: info says so, edit_image refuses, relink repairs it.
    std::fs::remove_file(&path).unwrap();
    let r = e.execute("picture.info", &json!({"id": id})).unwrap();
    assert_eq!(r["exists"], false);
    assert!(e.execute("picture.edit_image", &json!({"id": id})).is_err());
    assert!(
        e.execute("picture.set_file", &json!({"id": id})).is_err(),
        "reread of a missing file"
    );
    e.execute("picture.set_file", &json!({"id": id, "path": other}))
        .unwrap();
    let r = e.execute("picture.info", &json!({"id": id})).unwrap();
    assert_eq!(
        (r["exists"].clone(), r["width"].clone(), r["file"].clone()),
        (json!(true), json!(6), json!(other))
    );
    assert_eq!(r["transparent"], false);
    e.execute("picture.set_file", &json!({"id": id})).unwrap();
    e.execute("edit.undo", &json!({})).unwrap();
    e.execute("edit.undo", &json!({})).unwrap();
    let r = e.execute("picture.info", &json!({"id": id})).unwrap();
    assert_eq!(r["file"], json!(path), "undo restores the old file name");
    std::fs::remove_file(other).ok();
}

#[test]
fn not_a_picture_is_an_error() {
    let mut e = Engine::new();
    let r = e
        .execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100}))
        .unwrap();
    let id = r["id"].as_u64().unwrap();
    for c in ["picture.info", "picture.set_file", "picture.edit_image"] {
        assert!(e.execute(c, &json!({"id": id})).is_err(), "{c}");
    }
}
