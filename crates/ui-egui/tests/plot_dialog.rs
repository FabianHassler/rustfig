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

#[test]
fn tick_format_controls_reach_the_plot() {
    let mut h = harness();
    open(&mut h);
    {
        let d = &mut h.state_mut().plot_dialog;
        d.expr = "x".into();
        d.xmin = "0".into();
        d.xmax = "3000".into();
        d.xticks.scale = "3".into();
        d.xticks.comma = true;
        d.xticks.rotate = "45".into();
        d.xticks.labels = "1000 one\n2000 two words".into();
        d.yticks.labels = "pi/2 half pi".into();
    }
    h.run_steps(2);
    h.get_by_label("Draw plot").click();
    h.run_steps(3);
    assert!(!h.state().plot_dialog.open, "{}", h.state().plot_dialog.error);
    let s = spec(&mut h);
    assert_eq!(s["xticks"]["scale"], 3);
    assert_eq!(s["xticks"]["comma"], true);
    assert_eq!(s["xticks"]["rotate"], 45.0);
    assert_eq!(s["xticks"]["labels"], json!([[1000.0, "one"], [2000.0, "two words"]]));
    assert_eq!(s["yticks"]["labels"][0][0], std::f64::consts::FRAC_PI_2);
    // Editing shows them again, a bad value is an error in the dialog.
    let id = h.state().engine.doc().objects[0].id.0;
    h.state_mut().run("plot.edit", json!({"id": id}));
    h.run_steps(3);
    let d = &h.state().plot_dialog;
    assert_eq!(d.xticks.scale, "3");
    assert_eq!(d.xticks.labels, "1000 one\n2000 two words");
    h.state_mut().plot_dialog.xticks.scale = "x".into();
    h.run_steps(2);
    h.get_by_label("Draw plot").click();
    h.run_steps(3);
    assert!(h.state().plot_dialog.open && h.state().plot_dialog.error.contains("scale"));
}

#[test]
fn fill_controls_reach_the_plot() {
    let mut h = harness();
    open(&mut h);
    {
        let d = &mut h.state_mut().plot_dialog;
        d.expr = "sin(x)".into();
        d.ymin = "-1".into();
        d.ymax = "1".into();
    }
    h.run_steps(2);
    h.get_by_label("Curve options").click();
    h.run_steps(2);
    h.get_by_label("to the line y =").click();
    h.run_steps(2);
    h.get_by_label("Draw plot").click();
    h.run_steps(3);
    assert!(!h.state().plot_dialog.open, "{}", h.state().plot_dialog.error);
    let s = spec(&mut h);
    assert_eq!(s["curves"][0]["fill"]["y"], 0.0);
    assert_eq!(s["curves"][0]["expr"], "sin(x)");
    // The area is drawn.
    let o = &h.state().engine.doc().objects[0];
    let rustfig_model::ObjectKind::Compound(c) = &o.kind else {
        panic!()
    };
    assert!(c
        .objects
        .iter()
        .any(|m| m.comments.iter().any(|c| c.ends_with("part: fill"))));
    // Editing and switching it off removes the area.
    let id = o.id.0;
    h.state_mut().run("plot.edit", json!({"id": id}));
    h.run_steps(3);
    assert_eq!(
        h.state().plot_dialog.fill.mode,
        rustfig_ui_egui::plot_dialog::FillMode::Baseline
    );
    h.state_mut().plot_dialog.fill.mode = rustfig_ui_egui::plot_dialog::FillMode::None;
    h.get_by_label("Draw plot").click();
    h.run_steps(3);
    let s = spec(&mut h);
    assert!(s["curves"][0]["fill"].is_null());
}

#[test]
fn fill_up_to_a_second_curve_added_in_the_dialog() {
    let mut h = harness();
    open(&mut h);
    {
        let d = &mut h.state_mut().plot_dialog;
        d.expr = "sin(x)".into();
        d.ymin = "-1".into();
        d.ymax = "1".into();
        d.fill.mode = rustfig_ui_egui::plot_dialog::FillMode::Curve;
    }
    h.run_steps(2);
    h.get_by_label("Curve options").click();
    h.run_steps(2);
    h.get_by_label("Add curve y = f(x)").click();
    h.run_steps(3);
    assert_eq!(h.state().plot_dialog.curves.len(), 2);
    h.get_by_label("Remove").click();
    h.run_steps(3);
    assert_eq!(h.state().plot_dialog.curves.len(), 1);
    h.get_by_label("Add curve y = f(x)").click();
    h.run_steps(3);
    h.get_by_label("Draw plot").click();
    h.run_steps(3);
    assert!(!h.state().plot_dialog.open, "{}", h.state().plot_dialog.error);
    let s = spec(&mut h);
    assert_eq!(s["curves"].as_array().unwrap().len(), 2);
    assert_eq!(s["curves"][1]["expr"], "cos(x)");
    assert_eq!(s["curves"][0]["fill"]["curve"], 2);
}

#[test]
fn bar_and_error_controls_reach_the_plot() {
    let mut h = harness();
    open(&mut h);
    let dir = std::env::temp_dir().join(format!("rustfig-q4-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("d.csv");
    std::fs::write(&file, "1,2,0.5\n2,3,0.5\n3,1,0.25\n").unwrap();
    {
        let d = &mut h.state_mut().plot_dialog;
        d.mode = rustfig_engine::plot::PlotMode::Data;
        d.file = file.to_string_lossy().into_owned();
        d.xcol = "1".into();
        d.ycol = "2".into();
        d.xmin = "0".into();
        d.xmax = "4".into();
        d.ymin = "0".into();
        d.ymax = "4".into();
        d.yerr_col = "3".into();
    }
    h.run_steps(2);
    h.get_by_label("Curve options").click();
    h.run_steps(2);
    h.get_by_label("bars").click();
    h.run_steps(2);
    h.get_by_label("Draw plot").click();
    h.run_steps(3);
    assert!(!h.state().plot_dialog.open, "{}", h.state().plot_dialog.error);
    let s = spec(&mut h);
    assert_eq!(s["curves"][0]["style"], "bar");
    assert_eq!(s["curves"][0]["yerr_col"], 3);
    let o = &h.state().engine.doc().objects[0];
    let rustfig_model::ObjectKind::Compound(c) = &o.kind else {
        panic!()
    };
    let count = |k: &str| {
        c.objects
            .iter()
            .filter(|m| m.comments.iter().any(|c| c.ends_with(&format!("part: {k}"))))
            .count()
    };
    assert_eq!(count("bar"), 3);
    assert_eq!(count("errorbar"), 9);
    // A bad column is an error in the dialog.
    let id = h.state().engine.doc().objects[0].id.0;
    h.state_mut().run("plot.edit", json!({"id": id}));
    h.run_steps(3);
    h.state_mut().plot_dialog.yerr_col = "x".into();
    h.run_steps(2);
    h.get_by_label("Draw plot").click();
    h.run_steps(3);
    assert!(h.state().plot_dialog.open && h.state().plot_dialog.error.contains("error column"));
    let _ = std::fs::remove_dir_all(&dir);
}
