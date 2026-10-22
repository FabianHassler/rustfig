#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! S4-3: saving into another folder rewrites relative picture names
//! (xfig's `relative_path`); absolute names stay.

use rustfig_engine::Engine;
use serde_json::json;

const XPM: &str = "/* XPM */\nstatic char *a[] = {\n\"2 2 2 1\",\n\"a c #000000\",\n\"b c None\",\n\"aa\",\n\"aa\"};\n";

#[test]
fn save_as_into_another_folder_keeps_pictures_loading() {
    let d = std::env::temp_dir().join(format!("rustfig-relpic-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("a/pics")).unwrap();
    std::fs::create_dir_all(d.join("b/sub")).unwrap();
    let pic = d.join("a/pics/p.xpm");
    std::fs::write(&pic, XPM).unwrap();
    // A figure in a/ whose picture is named relative to it.
    let mut e = Engine::new();
    e.execute(
        "shape.picture",
        &json!({"path": pic.to_string_lossy(), "x": 0, "y": 0, "width": 500, "height": 500}),
    )
    .unwrap();
    let f = d.join("a/f.fig");
    e.execute("file.save", &json!({"path": f.to_string_lossy()})).unwrap();
    let text = std::fs::read_to_string(&f)
        .unwrap()
        .replace(&*pic.to_string_lossy(), "pics/p.xpm");
    std::fs::write(&f, text).unwrap();

    let mut e = Engine::new();
    e.execute("file.open", &json!({"path": f.to_string_lossy()})).unwrap();
    let g = d.join("b/sub/g.fig");
    e.execute("file.save", &json!({"path": g.to_string_lossy()})).unwrap();
    let saved = std::fs::read_to_string(&g).unwrap();
    assert!(saved.contains("../../a/pics/p.xpm"), "{saved}");
    // The figure in memory follows, and the saved file loads its picture.
    let mut e2 = Engine::new();
    e2.execute("file.open", &json!({"path": g.to_string_lossy()})).unwrap();
    let ids: Vec<u64> = e2.doc().objects.iter().map(|o| o.id.0).collect();
    let info = e2.execute("picture.info", &json!({"id": ids[0]})).unwrap();
    assert!(info["width"].as_u64().unwrap() > 0);

    // Absolute names stay as they are.
    let abs = d.join("b/h.fig");
    let mut e3 = Engine::new();
    e3.execute(
        "shape.picture",
        &json!({"path": pic.to_string_lossy(), "x": 0, "y": 0, "width": 500, "height": 500}),
    )
    .unwrap();
    e3.execute("file.save", &json!({"path": f.to_string_lossy()})).unwrap();
    e3.execute("file.save", &json!({"path": abs.to_string_lossy()}))
        .unwrap();
    assert!(std::fs::read_to_string(&abs).unwrap().contains(&*pic.to_string_lossy()));
    let _ = std::fs::remove_dir_all(&d);
}
