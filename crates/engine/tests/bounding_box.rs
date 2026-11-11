#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! F11: the figure's bounding box. Standard is the page; a box of the
//! figure's own is saved as a `# RustFigBoundingBox` comment, edited by
//! commands or by dragging in Bounding box mode, and every export covers it.

use rustfig_engine::Engine;
use serde_json::json;
use std::path::PathBuf;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustfig-f11-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A 1 by 0.5 inch rectangle at (1200, 1200).
fn engine() -> Engine {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 1200, "y": 1200, "width": 1200, "height": 600}),
    )
    .unwrap();
    e
}

fn eps(e: &mut Engine, dir: &std::path::Path, name: &str) -> String {
    let path = dir.join(name);
    e.execute("file.export", &json!({"path": path.to_str().unwrap(), "format": "eps"}))
        .unwrap();
    std::fs::read_to_string(path).unwrap()
}

fn bbox_line(eps: &str) -> &str {
    eps.lines().find(|l| l.starts_with("%%BoundingBox:")).unwrap()
}

#[test]
fn standard_box_is_the_page() {
    let dir = temp_dir("page");
    let mut e = engine();
    assert!(e.doc().bounding_box.is_none());
    // Letter, landscape: 11 by 8.5 inches.
    let out = eps(&mut e, &dir, "page.eps");
    assert_eq!(bbox_line(&out), "%%BoundingBox: 0 0 792 612");
    // Portrait turns the page.
    e.execute("doc.toggle_orientation", &json!({})).unwrap();
    assert_eq!(
        bbox_line(&eps(&mut e, &dir, "portrait.eps")),
        "%%BoundingBox: 0 0 612 792"
    );
    let info = e.execute("doc.info", &json!({})).unwrap();
    assert_eq!(
        info["bounding_box"],
        json!({"standard": true, "x1": 0, "y1": 0, "x2": 10200, "y2": 13200})
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn own_box_sizes_and_clips_every_export() {
    let dir = temp_dir("own");
    let mut e = engine();
    // A 2 by 1 inch box over the left part of the rectangle and beyond.
    let r = e
        .execute(
            "doc.set_bounding_box",
            &json!({"x1": 0, "y1": 1200, "x2": 2400, "y2": 0}),
        )
        .unwrap();
    assert_eq!(r, json!({"standard": false, "x1": 0, "y1": 0, "x2": 2400, "y2": 1200}));
    let out = eps(&mut e, &dir, "own.eps");
    assert_eq!(bbox_line(&out), "%%BoundingBox: 0 0 144 72");
    assert!(out.contains("rectclip"), "objects past the box are cut off");
    // SVG: the page is the box.
    let svg = dir.join("own.svg");
    e.execute("file.export", &json!({"path": svg.to_str().unwrap(), "format": "svg"}))
        .unwrap();
    let svg = std::fs::read_to_string(svg).unwrap();
    assert!(svg.contains("viewBox=\"0 0 2400 1200\""), "{svg}");
    // Undo goes back to the page.
    e.execute("edit.undo", &json!({})).unwrap();
    assert!(e.doc().bounding_box.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_boxes_are_refused() {
    let mut e = engine();
    for p in [
        json!({"x1": 0, "y1": 0, "x2": 0, "y2": 100}),
        json!({"x1": 0, "y1": 0, "x2": 100}),
        json!({"x1": 0, "y1": 0, "x2": 1e300, "y2": 100}),
        json!({"x1": "a", "y1": 0, "x2": 10, "y2": 100}),
    ] {
        assert!(e.execute("doc.set_bounding_box", &p).is_err(), "{p}");
    }
    assert!(e.doc().bounding_box.is_none());
    assert!(Engine::new().execute("doc.bounding_box_content", &json!({})).is_err());
}

#[test]
fn content_then_standard() {
    let mut e = engine();
    let r = e.execute("doc.bounding_box_content", &json!({})).unwrap();
    assert_eq!(r["standard"], false);
    let b = e.doc().bounding_box.unwrap();
    // fig2dev's bounds of the rectangle: half the line width around it.
    assert!(b.x1 <= 1200 && b.x1 > 1180 && b.x2 >= 2400 && b.x2 < 2420, "{b:?}");
    assert!(b.y1 <= 1200 && b.y1 > 1180 && b.y2 >= 1800 && b.y2 < 1820, "{b:?}");
    // The box stays when the drawing grows.
    e.execute(
        "shape.rectangle",
        &json!({"x": 6000, "y": 6000, "width": 100, "height": 100}),
    )
    .unwrap();
    assert_eq!(e.doc().bounding_box, Some(b));
    e.execute("doc.bounding_box_standard", &json!({})).unwrap();
    assert!(e.doc().bounding_box.is_none());
}

#[test]
fn saved_as_the_first_comment_and_read_back() {
    let dir = temp_dir("save");
    let fig = dir.join("box.fig");
    let mut e = engine();
    e.execute("file.save", &json!({"path": fig.to_str().unwrap()})).unwrap();
    let text = std::fs::read_to_string(&fig).unwrap();
    assert!(!text.contains("RustFigBoundingBox"), "standard writes nothing: {text}");

    e.execute(
        "doc.set_bounding_box",
        &json!({"x1": -100, "y1": 50, "x2": 3000, "y2": 2000}),
    )
    .unwrap();
    e.execute("file.save", &json!({"path": fig.to_str().unwrap()})).unwrap();
    let text = std::fs::read_to_string(&fig).unwrap();
    let first_comment = text.lines().find(|l| l.starts_with("# ")).unwrap();
    assert_eq!(first_comment, "# RustFigBoundingBox -100 50 3000 2000");

    let mut f = Engine::new();
    f.execute("file.open", &json!({"path": fig.to_str().unwrap()})).unwrap();
    let b = f.doc().bounding_box.unwrap();
    assert_eq!((b.x1, b.y1, b.x2, b.y2), (-100, 50, 3000, 2000));
    assert!(f.doc().comments.iter().all(|c| !c.contains("RustFigBoundingBox")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dragging_the_frame_is_one_undo_step() {
    let mut e = engine();
    e.execute("tool.bounding_box", &json!({})).unwrap();
    // Point positioning snaps to 1/16 inch (75 Fig units). The page's
    // bottom-right corner (landscape Letter: 13200 by 10200)
    // dragged up and left.
    let ev = |e: &mut Engine, t: &str, x: f64, y: f64| {
        e.execute("tool.event", &json!({"type": t, "x": x, "y": y})).unwrap();
    };
    ev(&mut e, "press", 13200.0, 10200.0);
    ev(&mut e, "move", 6000.0, 4050.0);
    assert_eq!(e.shown_bounding_box().x2, 6000, "the frame follows the pointer");
    assert!(e.doc().bounding_box.is_none(), "nothing committed during the drag");
    ev(&mut e, "release", 6000.0, 4050.0);
    let b = e.doc().bounding_box.unwrap();
    assert_eq!((b.x1, b.y1, b.x2, b.y2), (0, 0, 6000, 4050));
    // Inside the box: the whole box moves.
    ev(&mut e, "press", 3000.0, 2100.0);
    ev(&mut e, "release", 3600.0, 2700.0);
    let b = e.doc().bounding_box.unwrap();
    assert_eq!((b.x1, b.y1, b.x2, b.y2), (600, 600, 6600, 4650));
    e.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(e.doc().bounding_box.unwrap().x1, 0);
    e.execute("edit.undo", &json!({})).unwrap();
    assert!(e.doc().bounding_box.is_none());
}

#[test]
fn copy_as_picture_and_multiple_pages_keep_the_objects_bounds() {
    let mut e = engine();
    e.execute("doc.set_bounding_box", &json!({"x1": 0, "y1": 0, "x2": 100, "y2": 100}))
        .unwrap();
    let doc = e.doc().clone();
    assert!(doc.export_bounds().is_some());
    let mut multi = doc.clone();
    multi.bounding_box = None;
    multi.settings.multiple_pages = true;
    assert_eq!(multi.export_bounds(), None);
}
