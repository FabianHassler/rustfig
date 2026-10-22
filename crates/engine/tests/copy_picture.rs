#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Copy as picture (E14) through a fake clipboard: the real one is never touched.

use rustfig_engine::picture::{PictureClipboard, PictureData, PictureKind};
use rustfig_engine::Engine;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Fake {
    got: Arc<Mutex<Vec<PictureData>>>,
    fail: bool,
}

impl PictureClipboard for Fake {
    fn set_pictures(&mut self, items: &[PictureData]) -> Result<(), String> {
        if self.fail {
            return Err("clipboard busy".into());
        }
        *self.got.lock().unwrap() = items.to_vec();
        Ok(())
    }
}

fn engine_with(fake: &Fake) -> Engine {
    let mut e = Engine::new();
    e.set_picture_clipboard(Some(Box::new(fake.clone())));
    e
}

fn draw(e: &mut Engine) {
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1200, "height": 600}),
    )
    .unwrap();
    e.execute("shape.circle", &json!({"cx": 3000, "cy": 300, "r": 300}))
        .unwrap();
}

#[test]
fn clipboard_gets_the_platform_vector_type_and_png() {
    let fake = Fake::default();
    let mut e = engine_with(&fake);
    draw(&mut e);
    e.execute("edit.deselect_all", &json!({})).ok();
    let r = e.execute("edit.copy_picture", &json!({})).unwrap();
    let got = fake.got.lock().unwrap().clone();
    let kinds: Vec<_> = got.iter().map(|i| i.kind).collect();
    // Windows gets SVG for EMF only when fig2dev lacks the EMF driver.
    let emf = rustfig_engine::fig2dev::driver_available("emf");
    let want: Vec<_> = PictureKind::platform_set()
        .into_iter()
        .map(|k| {
            if k == PictureKind::Emf && !emf {
                PictureKind::Svg
            } else {
                k
            }
        })
        .collect();
    assert_eq!(kinds, want);
    for i in &got {
        assert!(!i.bytes.is_empty());
        match i.kind {
            PictureKind::Pdf => assert!(i.bytes.starts_with(b"%PDF")),
            PictureKind::Png => assert!(i.bytes.starts_with(b"\x89PNG")),
            PictureKind::Svg => assert!(String::from_utf8_lossy(&i.bytes).contains("<svg")),
            // EMF files start with the EMR_HEADER record type 1.
            PictureKind::Emf => assert!(i.bytes.starts_with(&[1, 0, 0, 0])),
        }
    }
    assert!(r["formats"].as_array().unwrap().len() == 2);
}

#[test]
fn selection_only_is_smaller_than_the_figure() {
    let fake = Fake::default();
    let mut e = engine_with(&fake);
    draw(&mut e);
    e.execute("edit.select_all", &json!({})).unwrap();
    e.execute("edit.copy_picture", &json!({})).unwrap();
    let all = fake.got.lock().unwrap().last().unwrap().bytes.len();
    let ids: Vec<Value> = e.doc().objects.iter().take(1).map(|o| json!(o.id.0)).collect();
    e.execute("edit.copy_picture", &json!({"ids": ids})).unwrap();
    let one = fake.got.lock().unwrap().last().unwrap().bytes.len();
    assert_ne!(all, one);
}

#[test]
fn empty_figure_and_clipboard_errors_are_messages() {
    let fake = Fake::default();
    let mut e = engine_with(&fake);
    let err = e.execute("edit.copy_picture", &json!({})).unwrap_err();
    assert!(err.to_string().contains("empty"), "{err}");
    assert!(fake.got.lock().unwrap().is_empty());

    let mut bad = engine_with(&Fake {
        fail: true,
        ..Fake::default()
    });
    draw(&mut bad);
    let err = bad.execute("edit.copy_picture", &json!({})).unwrap_err();
    assert!(err.to_string().contains("clipboard busy"), "{err}");

    let mut none = Engine::new();
    draw(&mut none);
    let err = none.execute("edit.copy_picture", &json!({})).unwrap_err();
    assert!(err.to_string().contains("not available"), "{err}");
}

#[test]
fn hostile_params_fail_and_leave_the_document_alone() {
    for p in [
        json!({"ids": [999999]}),
        json!({"ids": "x"}),
        json!({"magnification": -1}),
        json!({"magnification": "big"}),
        json!({"dpi": 1e9}),
        json!({"background": "#zz"}),
        json!({"margin": 1e308}),
        json!({"options": "x"}),
    ] {
        let fake = Fake::default();
        let mut e = engine_with(&fake);
        draw(&mut e);
        let before = e.doc().clone();
        assert!(e.execute("edit.copy_picture", &p).is_err(), "{p}");
        assert_eq!(e.doc(), &before);
        assert!(fake.got.lock().unwrap().is_empty());
    }
}

/// Edit > Copy as Picture waits for its objects (xfig has no selection):
/// left an object, middle a region, right the whole figure; Escape cancels.
#[test]
fn copy_as_picture_picks_after_the_command() {
    use rustfig_engine::geom::Point;
    use rustfig_engine::{Button, ToolEvent};
    let fake = Fake::default();
    let mut e = engine_with(&fake);
    draw(&mut e);
    let press = |e: &mut Engine, x: f64, y: f64, button| {
        let pos = Point::new(x, y);
        e.tool_event(ToolEvent::Press {
            pos,
            button,
            shift: false,
        })
        .unwrap();
        e.tool_event(ToolEvent::Release { pos, button }).unwrap();
    };
    let copied = |fake: &Fake| fake.got.lock().unwrap().last().map(|d| d.bytes.len());
    // Right: the whole figure.
    e.execute("edit.copy_picture_pick", &json!({})).unwrap();
    assert_eq!(e.mouse_hints().right, "whole figure");
    press(&mut e, 0.0, 0.0, Button::Right);
    let whole = copied(&fake).unwrap();
    // Left: the object under the pointer (the box's edge).
    fake.got.lock().unwrap().clear();
    e.execute("edit.copy_picture_pick", &json!({})).unwrap();
    press(&mut e, 0.0, 300.0, Button::Left);
    let one = copied(&fake).unwrap();
    assert_ne!(whole, one);
    // Middle: a region by two clicks around the circle.
    fake.got.lock().unwrap().clear();
    e.execute("edit.copy_picture_pick", &json!({})).unwrap();
    press(&mut e, 2500.0, -200.0, Button::Middle);
    press(&mut e, 3500.0, 800.0, Button::Middle);
    assert!(copied(&fake).is_some(), "the circle was copied");
    // Escape cancels; the next click does nothing special.
    fake.got.lock().unwrap().clear();
    e.execute("edit.copy_picture_pick", &json!({})).unwrap();
    e.execute("tool.escape", &json!({})).unwrap();
    assert_ne!(e.mouse_hints().right, "whole figure");
    assert!(copied(&fake).is_none());
}
