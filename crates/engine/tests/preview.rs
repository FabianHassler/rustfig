#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! `preview.set` / `preview.clear`: attribute changes shown through the
//! overlay without touching the document, the undo history or the selection.

use rustfig_engine::Engine;
use serde_json::json;

fn with_box() -> (Engine, u64) {
    let mut e = Engine::new();
    let r = e
        .execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 1200, "height": 600}))
        .unwrap();
    let id = r["id"].as_u64().unwrap();
    (e, id)
}

fn fill_of(o: &rustfig_model::Object) -> i32 {
    o.style().unwrap().area_fill
}

#[test]
fn set_and_clear_leave_document_and_history_alone() {
    let (mut e, id) = with_box();
    let before = e.doc().clone();
    let rev = e.revision();
    let sel = e.selection().to_vec();
    e.execute("preview.set", &json!({"ids": [id], "attrs": {"area_fill": 45, "fill_color": 4}}))
        .unwrap();
    assert!(e.preview_active());
    assert_eq!(e.doc(), &before);
    assert_eq!(e.revision(), rev);
    assert_eq!(e.selection(), sel.as_slice());
    let ov = e.overlay();
    assert_eq!(ov.hide.len(), 1);
    assert_eq!(ov.preview.len(), 1);
    assert_eq!(fill_of(&ov.preview[0]), 45);
    assert_ne!(fill_of(e.doc().get(rustfig_model::ObjectId(id)).unwrap()), 45);
    e.execute("preview.clear", &json!({})).unwrap();
    assert!(!e.preview_active());
    let ov = e.overlay();
    assert!(ov.hide.is_empty() && ov.preview.is_empty());
    // One undo step only: the box itself.
    assert!(e.undo());
    assert!(!e.can_undo());
}

#[test]
fn indicator_preview_shows_what_update_would_do() {
    let (mut e, id) = with_box();
    let ind = e.indicator.clone();
    e.execute("preview.set", &json!({"ids": [id], "indicator": {"area_fill": 30}}))
        .unwrap();
    assert_eq!(fill_of(&e.overlay().preview[0]), 30);
    // The indicator itself is untouched.
    assert_eq!(e.indicator.area_fill, ind.area_fill);
}

#[test]
fn preview_ends_with_a_document_change_or_a_mode_change() {
    let (mut e, id) = with_box();
    e.execute("preview.set", &json!({"ids": [id], "attrs": {"area_fill": 20}}))
        .unwrap();
    e.execute("object.set_attributes", &json!({"ids": [id], "thickness": 4}))
        .unwrap();
    assert!(!e.preview_active());
    assert!(e.overlay().preview.is_empty());
    e.execute("preview.set", &json!({"ids": [id], "attrs": {"area_fill": 20}}))
        .unwrap();
    e.execute("tool.box", &json!({})).unwrap();
    assert!(!e.preview_active());
}

#[test]
fn bad_params_are_errors() {
    let (mut e, id) = with_box();
    assert!(e.execute("preview.set", &json!({"ids": [id]})).is_err());
    assert!(e.execute("preview.set", &json!({"ids": [id], "attrs": {"line": "nope"}})).is_err());
    assert!(e.execute("preview.set", &json!({"ids": [424242], "attrs": {"area_fill": 1}})).is_err());
    assert!(!e.preview_active());
}
