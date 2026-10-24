//! The browser's stand-ins for files (C2): Merge from bytes, and pictures kept
//! in memory by file name.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::{base64, Engine};
use rustfig_model::{ObjectKind, Picture};
use serde_json::json;

fn png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(w, h, image::Rgba([200, 30, 30, 255]));
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

const BOX: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
    2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 600 0 600 600 0 600 0 0\n";

#[test]
fn merge_from_base64_bytes() {
    let mut e = Engine::new();
    e.execute("file.merge_data", &json!({"base64": base64(BOX.as_bytes())}))
        .unwrap();
    assert_eq!(e.doc().objects.len(), 1);
    assert!(e.execute("file.merge_data", &json!({"base64": 5})).is_err());
    assert!(e.execute("file.merge_data", &json!({})).is_err());
    assert_eq!(e.doc().objects.len(), 1);
}

#[test]
fn stored_picture_loads_by_file_name_and_survives_a_merge() {
    let mut e = Engine::new();
    let name = "web-store-test-picture.png";
    e.execute(
        "picture.store",
        &json!({"name": format!("C:\\x\\{name}"), "base64": base64(&png(7, 5))}),
    )
    .unwrap();
    // Picture mode: the file name is all the figure keeps.
    let r = e
        .execute("shape.picture", &json!({"path": name, "x": 0, "y": 0}))
        .unwrap();
    let id = r["id"].as_u64().unwrap();
    let info = e.execute("picture.info", &json!({"id": id})).unwrap();
    assert_eq!(info["width"], 7);
    assert_eq!(info["exists"], true);
    assert_eq!(info["type"], "PNG");
    // A figure naming the picture finds it when merged.
    let fig = format!(
        "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
         2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5\n\t0 {name}\n\t 0 0 600 0 600 600 0 600 0 0\n"
    );
    e.execute("file.merge_data", &json!({"data": fig})).unwrap();
    fn find(o: &rustfig_model::Object) -> Option<&Picture> {
        match &o.kind {
            ObjectKind::Polyline(p) => p.picture.as_ref(),
            ObjectKind::Compound(c) => c.objects.iter().find_map(find),
            _ => None,
        }
    }
    let pic = find(e.doc().objects.last().unwrap()).unwrap();
    assert_eq!(pic.image.as_ref().unwrap().width, 7);
}

#[test]
fn picture_store_refuses_nonsense() {
    let mut e = Engine::new();
    assert!(e
        .execute("picture.store", &json!({"name": "", "base64": "AAAA"}))
        .is_err());
    assert!(e
        .execute("picture.store", &json!({"name": "a.png", "base64": ""}))
        .is_err());
    assert!(e.execute("picture.store", &json!({"name": "a.png"})).is_err());
    // Bytes that are no picture are kept but do not load.
    e.execute(
        "picture.store",
        &json!({"name": "web-store-bad.png", "base64": base64(b"hello")}),
    )
    .unwrap();
    assert!(e
        .execute("shape.picture", &json!({"path": "web-store-bad.png", "x": 0, "y": 0}))
        .is_err());
}
