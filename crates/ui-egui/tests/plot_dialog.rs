#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The Plot dialog's controls for the plot extensions (plan Q1-Q5): each
//! one is a field of the dialog that reaches `plot.add`/`plot.update`.

use egui::vec2;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use rustfig_engine::Engine;
use rustfig_ui_egui::dialogs::ScriptedDialogs;
use rustfig_ui_egui::App;
use serde_json::{json, Value};

fn harness() -> Harness<'static, App> {
    let mut a = App::with_engine(Engine::new());
    a.file_dialogs = Box::new(ScriptedDialogs::default());
    let mut h = Harness::builder()
        .with_size(vec2(1280.0, 2400.0))
        .build_state(|ctx, app: &mut App| app.ui(ctx), a);
    h.run_steps(3);
    h
}

/// Open the dialog on a new plot through its command.
fn open(h: &mut Harness<'static, App>) {
    h.state_mut().run("plot.panel", json!({}));
    h.run_steps(3);
    assert!(h.state().plot_dialog.open);
}

/// The kept parameters of the plot just drawn.
fn spec(h: &mut Harness<'static, App>) -> Value {
    let id = h.state().engine.doc().objects[0].id.0;
    h.state_mut().run("plot.get", json!({"id": id})).unwrap()["spec"].clone()
}

#[test]
fn axes_kind_and_grid_controls_reach_the_plot() {
    let mut h = harness();
    open(&mut h);
    h.state_mut().plot_dialog.expr = "sin(x)".into();
    h.run_steps(2);
    h.get_by_label("Crossing at the origin").click();
    h.run_steps(2);
    h.get_by_label("Grid").click();
    h.run_steps(2);
    h.get_by_label("Minor lines").click();
    h.run_steps(2);
    h.get_by_label("dashed").click();
    h.run_steps(2);
    h.get_by_label("Draw plot").click();
    h.run_steps(3);
    assert!(!h.state().plot_dialog.open, "{}", h.state().plot_dialog.error);
    let s = spec(&mut h);
    assert_eq!(s["axes"], "origin");
    assert_eq!(s["grid"], true);
    assert_eq!(s["grid_minor"], true);
    assert_eq!(s["grid_style"], "dashed");
    // Editing the plot shows the same controls.
    let id = h.state().engine.doc().objects[0].id.0;
    h.state_mut().run("plot.edit", json!({"id": id}));
    h.run_steps(3);
    let d = &h.state().plot_dialog;
    assert_eq!(d.axes, rustfig_engine::plot::AxesKind::Origin);
    assert!(d.grid_minor && d.grid_style == "dashed");
}
