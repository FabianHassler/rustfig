#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! W10-4, W10-10, W10-11: the Export panel shows the rows xfig shows for
//! the language, previews the result, keeps its values and resets them.

use egui::vec2;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use rustfig_engine::Engine;
use rustfig_ui_egui::dialogs::ScriptedDialogs;
use rustfig_ui_egui::App;
use serde_json::json;

fn harness(app: App) -> Harness<'static, App> {
    let mut h = Harness::builder()
        .with_size(vec2(1280.0, 1400.0))
        .build_state(|ctx, app: &mut App| app.ui(ctx), app);
    h.run_steps(3);
    h
}

fn language(h: &mut Harness<'static, App>, id: &str) {
    let lang = rustfig_engine::fig2dev::languages::language(id).unwrap();
    h.state_mut().dialogs.export.format = lang.id;
    h.run_steps(3);
}

fn count(h: &Harness<'static, App>, label: &str) -> usize {
    h.query_all_by_label(label).count()
}

#[test]
fn rows_follow_the_language() {
    let mut a = App::with_engine(Engine::new());
    a.file_dialogs = Box::new(ScriptedDialogs::default());
    a.run(
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 2400, "height": 1200}),
    );
    let mut h = harness(a);
    h.state_mut().run("app.export", json!({}));
    h.run_steps(3);
    language(&mut h, "pdf");
    h.get_by_label("Fit to Page");
    h.get_by_label("Border (points)");
    h.get_by_label("Paper size");
    assert_eq!(count(&h, "GIF transparent color"), 0);
    assert_eq!(count(&h, "EPS/PS preview"), 0);
    language(&mut h, "eps");
    h.get_by_label("EPS/PS preview");
    assert_eq!(count(&h, "Paper size"), 0);
    language(&mut h, "gif");
    h.get_by_label("GIF transparent color");
    h.get_by_label("Border (pixels)");
    h.get_by_label("Resolution");
    assert_eq!(count(&h, "Image quality"), 0);
    language(&mut h, "jpeg");
    h.get_by_label("Image quality");
    language(&mut h, "svg");
    assert_eq!(count(&h, "Border (points)"), 0);
    h.get_by_label("Figure size");
}

#[test]
fn preview_keep_reset_and_fit() {
    let mut a = App::with_engine(Engine::new());
    a.file_dialogs = Box::new(ScriptedDialogs::default());
    a.run(
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 2400, "height": 1200}),
    );
    let mut h = harness(a);
    h.state_mut().run("app.export", json!({}));
    h.run_steps(3);
    language(&mut h, "png");
    assert!(h.state().dialogs.export.preview.texture.is_some(), "no preview");
    // Values are kept per language, in the settings and not the drawing.
    h.state_mut().dialogs.export.settings.border = 7.0;
    h.run_steps(3);
    let kept = h.state_mut().run("export.settings", json!({"format": "png"})).unwrap();
    assert_eq!(kept["settings"]["border"], 7.0);
    assert!(h.state().engine.prefs.to_toml().contains("[rustfig.export.png]"));
    // They go back to xfig's with the reset button.
    h.get_by_label("Reset to xfig defaults").click();
    h.run_steps(3);
    assert_eq!(h.state().dialogs.export.settings.border, 0.0);
    let kept = h.state_mut().run("export.settings", json!({"format": "png"})).unwrap();
    assert_eq!(kept["settings"]["border"], 0.0);
    // Fit to Page sets the figure's magnification.
    h.get_by_label("Fit to Page").click();
    h.run_steps(3);
    assert!(h.state().dialogs.export.magnification > 150.0);
    assert_eq!(
        h.state().dialogs.export.magnification,
        h.state().engine.doc().settings.magnification
    );
}
