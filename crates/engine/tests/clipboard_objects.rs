#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! E7: the cut buffer on the system clipboard, through a fake clipboard shared
//! by two engines (two RustFig windows). The real clipboard is never touched.

use rustfig_engine::picture::{PictureClipboard, PictureData, PictureKind};
use rustfig_engine::Engine;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct State {
    fig: Option<String>,
    pictures: Vec<PictureData>,
    image: Option<Vec<u8>>,
}

#[derive(Clone, Default)]
struct Fake(Arc<Mutex<State>>);

impl PictureClipboard for Fake {
    fn set_pictures(&mut self, items: &[PictureData]) -> Result<(), String> {
        let mut s = self.0.lock().unwrap();
        s.fig = None;
        s.pictures = items.to_vec();
        Ok(())
    }
    fn set_objects(&mut self, fig: &str, items: &[PictureData]) -> Result<(), String> {
        let mut s = self.0.lock().unwrap();
        s.fig = Some(fig.to_string());
        s.pictures = items.to_vec();
        s.image = None;
        Ok(())
    }
    fn get_fig(&mut self) -> Option<String> {
        self.0.lock().unwrap().fig.clone()
    }
    fn get_image(&mut self) -> Option<Vec<u8>> {
        self.0.lock().unwrap().image.clone()
    }
}

fn engine(fake: &Fake) -> Engine {
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
}

fn copy_first(e: &mut Engine) {
    let id = e.doc().objects[0].id.0;
    e.execute("edit.copy", &json!({"ids": [id]})).unwrap();
}

fn paste_at_once(e: &mut Engine) -> bool {
    e.execute("edit.paste", &json!({"dx": 0, "dy": 0})).is_ok()
}

#[test]
fn copy_fills_cut_buffer_and_system_clipboard() {
    let fake = Fake::default();
    let mut e = engine(&fake);
    draw(&mut e);
    copy_first(&mut e);
    assert_eq!(e.clipboard().len(), 1);
    let s = fake.0.lock().unwrap();
    assert!(s.fig.as_deref().unwrap().starts_with("#FIG 3.2"));
    assert!(s.pictures.iter().any(|p| p.kind == PictureKind::Png));
}

#[test]
fn copy_between_two_windows() {
    let fake = Fake::default();
    let mut a = engine(&fake);
    let mut b = engine(&fake);
    draw(&mut a);
    copy_first(&mut a);
    assert!(b.clipboard().is_empty());
    assert!(paste_at_once(&mut b));
    assert_eq!(b.doc().objects.len(), 1);
    // The same text again is the cut buffer itself; a second paste adds one more.
    assert!(paste_at_once(&mut b));
    assert_eq!(b.doc().objects.len(), 2);
}

#[test]
fn newer_system_copy_wins_over_the_cut_buffer() {
    let fake = Fake::default();
    let mut a = engine(&fake);
    let mut b = engine(&fake);
    draw(&mut a);
    a.execute("shape.circle", &json!({"cx": 3000, "cy": 300, "r": 300}))
        .unwrap();
    let ids: Vec<_> = a.doc().objects.iter().map(|o| o.id.0).collect();
    b.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10}))
        .unwrap();
    copy_first(&mut b);
    a.execute("edit.copy", &json!({"ids": ids})).unwrap();
    assert!(paste_at_once(&mut b));
    assert_eq!(b.doc().objects.len(), 3);
}

#[test]
fn image_on_the_clipboard_pastes_as_a_picture() {
    let fake = Fake::default();
    let mut a = engine(&fake);
    draw(&mut a);
    copy_first(&mut a);
    let png = {
        let s = fake.0.lock().unwrap();
        s.pictures
            .iter()
            .find(|p| p.kind == PictureKind::Png)
            .unwrap()
            .bytes
            .clone()
    };
    {
        let mut s = fake.0.lock().unwrap();
        s.fig = None;
        s.image = Some(png);
    }
    let mut b = engine(&fake);
    assert!(paste_at_once(&mut b));
    assert_eq!(b.doc().objects.len(), 1);
    let o = serde_json::to_string(&*b.doc().objects[0]).unwrap();
    assert!(o.contains("clipboard-"), "{o}");
}

#[test]
fn hostile_clipboard_data_is_refused_cleanly() {
    let fake = Fake::default();
    let mut e = engine(&fake);
    for fig in ["", "garbage\u{0}", "#FIG 3.2\n1 1 1\n", "#FIG 3.2\nLandscape\n"] {
        fake.0.lock().unwrap().fig = Some(fig.to_string());
        assert!(!paste_at_once(&mut e));
        assert!(e.doc().objects.is_empty());
    }
    {
        let mut s = fake.0.lock().unwrap();
        s.fig = None;
        s.image = Some(b"\x89PNG not an image".to_vec());
    }
    assert!(!paste_at_once(&mut e));
    assert!(e.doc().objects.is_empty());
    // The internal cut buffer still works after hostile data.
    fake.0.lock().unwrap().image = None;
    draw(&mut e);
    copy_first(&mut e);
    assert!(paste_at_once(&mut e));
    assert_eq!(e.doc().objects.len(), 2);
}

#[test]
fn without_a_system_clipboard_the_cut_buffer_works() {
    let mut e = Engine::new();
    draw(&mut e);
    copy_first(&mut e);
    assert!(paste_at_once(&mut e));
    assert_eq!(e.doc().objects.len(), 2);
}
