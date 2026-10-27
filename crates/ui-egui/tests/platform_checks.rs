#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! P2: checks of platform behaviour that run on any system: HiDPI scales,
//! non-ASCII file names through the real menus, the three buttons.

use egui::{pos2, vec2};
use egui_kittest::Harness;
use rustfig_engine::Engine;
use rustfig_ui_egui::dialogs::ScriptedDialogs;
use rustfig_ui_egui::App;
use serde_json::json;

fn harness_at(app: App, ppp: f32) -> Harness<'static, App> {
    let mut h = Harness::builder()
        .with_size(vec2(1280.0, 1400.0))
        .with_pixels_per_point(ppp)
        .build_state(|ctx, app: &mut App| app.ui(ctx), app);
    h.run_steps(3);
    h
}

fn click(h: &mut Harness<'static, App>, x: f32, y: f32, button: egui::PointerButton) {
    let pos = pos2(x, y);
    h.hover_at(pos);
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(1);
    }
}

/// The same points on the screen are the same places in the drawing at any
/// scale (1x, Retina 2x, Windows' 125 % / 150 %, a phone-sized 3x).
#[test]
fn hidpi_scales_keep_the_drawing_coordinates() {
    let mut seen: Vec<(f32, Vec<(f64, f64)>)> = Vec::new();
    for ppp in [1.0, 1.25, 1.5, 2.0, 2.5, 3.0, 4.0] {
        let mut a = App::with_engine(Engine::new());
        a.file_dialogs = Box::new(ScriptedDialogs::default());
        let mut h = harness_at(a, ppp);
        h.state_mut().run("tool.polyline", json!({}));
        h.run_steps(2);
        click(&mut h, 400.0, 300.0, egui::PointerButton::Primary);
        click(&mut h, 600.0, 300.0, egui::PointerButton::Primary);
        click(&mut h, 600.0, 500.0, egui::PointerButton::Middle);
        let doc = h.state().engine.doc();
        assert_eq!(doc.objects.len(), 1, "ppp {ppp}");
        let pts: Vec<(f64, f64)> = doc.objects[0].points().unwrap().iter().map(|p| (p.x, p.y)).collect();
        assert_eq!(pts.len(), 3, "ppp {ppp}");
        let errors: Vec<_> = h.state().messages.iter().filter(|m| m.error).collect();
        assert!(errors.is_empty(), "ppp {ppp}: {errors:?}");
        seen.push((ppp, pts));
    }
    let (_, first) = &seen[0];
    for (ppp, pts) in &seen {
        assert_eq!(pts, first, "ppp {ppp} draws elsewhere than ppp 1");
    }
}

/// A canvas bigger than the graphics driver's biggest texture is drawn at a
/// lower resolution, not left blank (found by the HiDPI test above).
#[test]
fn raster_scale_respects_the_texture_limit() {
    use rustfig_ui_egui::canvas::raster_scale;
    assert_eq!(raster_scale(2.0, 1000.0, 8192), 2.0);
    let s = raster_scale(3.0, 2000.0, 4096);
    assert!(s < 3.0 && 2000.0 * s <= 4096.0, "{s}");
    for bad in [f32::NAN, f32::INFINITY, 0.0, -1.0] {
        assert_eq!(raster_scale(bad, 100.0, 2048), 1.0);
    }
    assert!(raster_scale(2.0, 0.0, 2048) > 0.0);
}

/// Opening and saving through the menu with names the systems allow:
/// accents, CJK, emoji, spaces, decomposed accents (what an HFS+ volume or
/// a Mac keyboard may produce), and a long name.
#[test]
fn non_ascii_file_names_open_and_save() {
    let base = std::env::temp_dir().join(format!("rustfig-p2-ü-日本-{}", std::process::id()));
    let names = [
        "Zeichnung ä ö ü ß.fig",
        "図面 日本語.fig",
        "emoji 😀 drawing.fig",
        "decomposed a\u{308}.fig",
        // Windows forbids `"` in file names.
        if cfg!(windows) {
            "with 'quotes' & #hash %percent.fig"
        } else {
            "with 'quotes' & \"double\" #hash %percent.fig"
        },
        &format!("{}.fig", "n".repeat(120)),
    ];
    for name in names {
        let dir = base.join("Ordner mit Leerzeichen é");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name).to_string_lossy().into_owned();
        let mut a = App::with_engine(Engine::new());
        a.file_dialogs = Box::new(ScriptedDialogs::new([Some(path.clone())]));
        a.run("shape.rectangle", json!({"x": 0, "y": 0, "width": 600, "height": 300}));
        let mut h = harness_at(a, 1.0);
        h.state_mut().run("app.save_as", json!({}));
        h.run_steps(2);
        assert!(std::path::Path::new(&path).is_file(), "not saved: {name}");
        assert!(!h.state().engine.is_modified(), "{name}");
        // The title and the file-name box show the name unchanged.
        let shown = h.state().engine.path().unwrap().to_string();
        assert_eq!(shown, path);
        // A second drawing opens it again.
        let mut b = App::with_engine(Engine::new());
        b.file_dialogs = Box::new(ScriptedDialogs::new([Some(path.clone())]));
        let mut hb = harness_at(b, 1.0);
        hb.state_mut().run("app.open", json!({}));
        hb.run_steps(2);
        assert_eq!(hb.state().engine.doc().objects.len(), 1, "{name}");
        // Export next to it (EPS: the name goes into the header).
        let eps = format!("{path}.eps");
        hb.state_mut()
            .engine
            .execute("file.export", &json!({"path": eps, "format": "eps"}))
            .unwrap();
        assert!(std::path::Path::new(&eps).is_file(), "{name}");
    }
    let _ = std::fs::remove_dir_all(&base);
}
