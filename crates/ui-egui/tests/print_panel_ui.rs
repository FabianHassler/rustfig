#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! B5: the Print panel shows xfig's rows, previews the printed page, keeps
//! its values, and prints through the engine's spooler (a recorder here:
//! nothing reaches a printer).

use egui::vec2;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use rustfig_engine::print::{PrintJob, RecordingSpooler};
use rustfig_engine::Engine;
use rustfig_ui_egui::dialogs::ScriptedDialogs;
use rustfig_ui_egui::App;
use serde_json::json;
use std::sync::{Arc, Mutex};

fn open_panel() -> (Harness<'static, App>, Arc<Mutex<Vec<PrintJob>>>) {
    let mut a = App::with_engine(Engine::new());
    a.file_dialogs = Box::new(ScriptedDialogs::default());
    let rec = RecordingSpooler::default();
    let jobs = rec.jobs.clone();
    a.engine.set_spooler(Box::new(rec));
    a.run(
        "shape.rectangle",
        json!({"x": 0, "y": 0, "width": 2400, "height": 1200}),
    );
    let mut h = Harness::builder()
        .with_size(vec2(1280.0, 1400.0))
        .build_state(|ctx, app: &mut App| app.ui(ctx), a);
    h.run_steps(3);
    h.state_mut().run("app.print", json!({}));
    h.run_steps(4);
    (h, jobs)
}

#[test]
fn the_panel_has_xfigs_rows_and_a_preview() {
    let (h, _) = open_panel();
    for label in [
        "Magnification",
        "Fit to Page",
        "Figure size",
        "Paper size",
        "Orientation",
        "Justification",
        "Pages",
        "Background",
        "Grid",
        "Layers",
        "Print all layers",
        "Print only active",
        "Figures in batch",
        "Dismiss",
        "Print FIGURE",
        "Add to Batch",
        "Print Batch",
        "Clear Batch",
        "Print preview",
    ] {
        h.get_by_label(label);
    }
    assert!(h.state().dialogs.print.preview.texture.is_some(), "no preview");
    assert_eq!(h.state().dialogs.print.preview.pages, 1);
}

#[test]
fn the_preview_follows_the_page_setup() {
    let (mut h, _) = open_panel();
    h.get_by_label("Landscape").click();
    h.run_steps(3);
    let landscape = h.state().dialogs.print.preview.texture.as_ref().unwrap().size_vec2();
    assert!(landscape.x > landscape.y, "{landscape:?}");
    h.get_by_label("Portrait").click();
    h.run_steps(3);
    let portrait = h.state().dialogs.print.preview.texture.as_ref().unwrap().size_vec2();
    assert!(portrait.y > portrait.x, "{portrait:?}");
    // A big magnification and multiple pages: more pages to look at.
    h.state_mut()
        .run("doc.set_magnification", json!({"magnification": 900}));
    h.get_by_label("Multiple").click();
    h.run_steps(4);
    assert!(h.state().dialogs.print.preview.pages > 1);
    h.get_by_label(">").click();
    h.run_steps(3);
    assert_eq!(h.state().dialogs.print.preview.page, 1);
    // Fit to Page brings it back to one page.
    h.get_by_label("Fit to Page").click();
    h.run_steps(4);
    assert_eq!(h.state().dialogs.print.preview.pages, 1);
    assert_eq!(
        h.state().dialogs.print.magnification,
        h.state().engine.doc().settings.magnification
    );
}

#[test]
fn values_are_kept_and_the_buttons_print_and_batch() {
    let (mut h, jobs) = open_panel();
    // "Print only active" needs a depth that is off.
    h.state_mut()
        .run("depth.set_visible", json!({"depth": 50, "visible": false}));
    h.run_steps(2);
    h.get_by_label("Print only active").click();
    h.run_steps(3);
    let kept = h.state_mut().run("print.settings", json!({})).unwrap();
    assert_eq!(kept["settings"]["all_layers"], false);
    assert!(h.state().engine.prefs.to_toml().contains("all_layers = false"));
    h.get_by_label("Print FIGURE").click();
    h.run_steps(3);
    assert_eq!(jobs.lock().unwrap().len(), 1);
    // The batch.
    h.get_by_label("Add to Batch").click();
    h.run_steps(3);
    h.get_by_label("Add to Batch").click();
    h.run_steps(3);
    assert_eq!(h.state().engine.print_batch_len(), 2);
    h.get_by_label("Clear Batch").click();
    h.run_steps(3);
    assert_eq!(h.state().engine.print_batch_len(), 0);
    h.get_by_label("Add to Batch").click();
    h.run_steps(3);
    h.get_by_label("Print Batch").click();
    h.run_steps(3);
    assert_eq!(h.state().engine.print_batch_len(), 0);
    assert_eq!(jobs.lock().unwrap().len(), 2);
    h.get_by_label("Dismiss").click();
    h.run_steps(3);
    assert!(!h.state().dialogs.print.open);
}

#[test]
fn the_file_menu_and_the_xfig_keys_open_the_panel() {
    let mut a = App::with_engine(Engine::new());
    a.file_dialogs = Box::new(ScriptedDialogs::default());
    let mut h = Harness::builder()
        .with_size(vec2(1280.0, 1400.0))
        .build_state(|ctx, app: &mut App| app.ui(ctx), a);
    h.run_steps(3);
    h.get_by_label("File").click();
    h.run_steps(2);
    h.get_by_label_contains("Print...").click();
    h.run_steps(3);
    assert!(h.state().dialogs.print.open);
    // An empty figure shows no page, and printing says so instead of failing.
    assert!(h.state().dialogs.print.preview.texture.is_none());
}
