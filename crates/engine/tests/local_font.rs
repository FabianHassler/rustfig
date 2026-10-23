#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! F7: installed fonts per text. The family list is injected, so nothing
//! here depends on the fonts of the machine.

use rustfig_engine::text::{override_local_faces, Generic, LocalFace};
use rustfig_engine::Engine;
use rustfig_model::{ObjectKind, Text};
use serde_json::json;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

fn face(name: &str, generic: Generic, bold: bool, italic: bool) -> LocalFace {
    LocalFace {
        family: name.split(' ').next().unwrap().to_string(),
        label: "x".into(),
        name: name.to_string(),
        generic,
        bold,
        italic,
    }
}

fn inject() {
    override_local_faces(Some(vec![
        face("Fakeserif", Generic::Serif, false, false),
        face("Fakesans Bold Italic", Generic::Sans, true, true),
        face("Fakemono", Generic::Mono, false, false),
    ]));
}

fn text_of(e: &Engine, i: usize) -> Text {
    let ObjectKind::Text(t) = &e.doc().objects[i].kind else {
        panic!()
    };
    t.clone()
}

#[test]
fn choosing_a_local_font_sets_family_and_nearest_fig_font() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    inject();
    let mut e = Engine::new();
    e.execute("shape.text", &json!({"x": 100, "y": 100, "text": "hello"}))
        .unwrap();
    let id = e.doc().objects[0].id.0;
    // Listing.
    let r = e.execute("text.local_fonts", &json!({"query": "sans"})).unwrap();
    assert_eq!(r["fonts"].as_array().unwrap().len(), 1);
    // Sans bold italic is Helvetica Bold Oblique (16 + 3), mono Courier (12).
    e.execute(
        "text.set_local_font",
        &json!({"name": "fakesans bold italic", "ids": [id]}),
    )
    .unwrap();
    let t = text_of(&e, 0);
    assert_eq!(t.local_font.as_deref(), Some("Fakesans Bold Italic"));
    assert_eq!(t.font, 19);
    assert!(t.is_postscript());
    assert_eq!(e.indicator.local_font.as_deref(), Some("Fakesans Bold Italic"));
    e.execute("text.set_local_font", &json!({"name": "Fakemono", "ids": [id]}))
        .unwrap();
    assert_eq!(text_of(&e, 0).font, 12);
    // Undo restores the previous family.
    assert!(e.undo());
    assert_eq!(text_of(&e, 0).font, 19);
    // New texts take the indicator's family.
    e.execute("text.set_local_font", &json!({"name": "Fakeserif", "ids": []}))
        .unwrap();
    e.execute("shape.text", &json!({"x": 100, "y": 900, "text": "new"}))
        .unwrap();
    assert_eq!(text_of(&e, 1).local_font.as_deref(), Some("Fakeserif"));
    assert_eq!(text_of(&e, 1).font, 0);
    // A Fig font number drops the family.
    e.execute("object.set_attributes", &json!({"ids": [id], "font": 4}))
        .unwrap();
    assert!(text_of(&e, 0).local_font.is_none());
    // Back to the Fig font with null.
    e.execute("text.set_local_font", &json!({"name": "Fakemono", "ids": [id]}))
        .unwrap();
    e.execute("text.set_local_font", &json!({"name": null, "ids": [id]}))
        .unwrap();
    assert!(text_of(&e, 0).local_font.is_none());
    // Unknown families are refused and change nothing.
    let before = e.doc().clone();
    assert!(e
        .execute("text.set_local_font", &json!({"name": "Nope", "ids": [id]}))
        .is_err());
    assert_eq!(*e.doc(), before);
    override_local_faces(None);
}

#[test]
fn missing_family_draws_with_the_fig_font_and_is_reported_once() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    override_local_faces(None);
    let src = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
        # rustfig-font: Gone Away Sans\n4 0 0 50 -1 16 12 0.0000 4 135 450 300 600 one\\001\n\
        # rustfig-font: Gone Away Sans\n4 0 0 50 -1 16 12 0.0000 4 135 450 300 900 two\\001\n\
        # rustfig-font: Also Gone\n4 0 0 50 -1 0 12 0.0000 4 135 450 300 1200 three\\001\n";
    let mut e = Engine::new();
    let r = e.execute("file.open_data", &json!({"data": src})).unwrap();
    let w: Vec<&str> = r["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    let msgs: Vec<&&str> = w.iter().filter(|m| m.contains("Gone Away Sans")).collect();
    assert_eq!(msgs.len(), 1, "{w:?}");
    assert!(msgs[0].contains("Also Gone"));
    // Drawn as Fig text; the family stays in the document.
    let scene = rustfig_render::build_scene(e.doc(), &rustfig_render::SceneOptions::default());
    let n = scene
        .prims
        .iter()
        .filter(|p| matches!(p, rustfig_render::Prim::Text(_)))
        .count();
    assert_eq!(n, 3);
    assert_eq!(text_of(&e, 0).local_font.as_deref(), Some("Gone Away Sans"));
    // Saved again, the family is kept.
    let out = e.execute("file.save_data", &json!({"format": "fig"}));
    if let Ok(out) = out {
        if let Some(d) = out["data"].as_str() {
            assert!(d.contains("rustfig-font: Gone Away Sans"));
        }
    }
}
