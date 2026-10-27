#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! C2: the browser's file I/O (picker, drop, downloads) on scripted dialogs.

use egui::vec2;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use rustfig_engine::Engine;
use rustfig_ui_egui::dialogs::ScriptedDialogs;
use rustfig_ui_egui::web_io::WebPick;
use rustfig_ui_egui::App;
use serde_json::json;

fn harness(app: App) -> Harness<'static, App> {
    let mut h = Harness::builder()
        .with_size(vec2(1280.0, 1400.0))
        .build_state(|ctx, app: &mut App| app.ui(ctx), app);
    h.run_steps(3);
    h
}

fn fig_bytes(rects: u32) -> Vec<u8> {
    let mut e = Engine::new();
    for i in 0..rects {
        e.execute(
            "shape.rectangle",
            &json!({"x": 600 + i * 2000, "y": 600, "width": 1200, "height": 600}),
        )
        .unwrap();
    }
    let r = e.execute("file.save_data", &json!({})).unwrap();
    r["data"].as_str().unwrap().as_bytes().to_vec()
}

fn web_app(picks: Vec<WebPick>) -> (App, ScriptedDialogs) {
    let d = ScriptedDialogs::web(picks);
    let mut a = App::with_engine(Engine::new());
    a.file_dialogs = Box::new(d.clone());
    (a, d)
}

fn count(a: &App) -> usize {
    a.engine.doc().objects.len()
}

#[test]
fn open_edit_save_round_trip() {
    let pick = WebPick {
        name: "Zeichnung ä.fig".into(),
        bytes: fig_bytes(2),
    };
    let (app, dialogs) = web_app(vec![pick]);
    let mut h = harness(app);
    h.state_mut().run("app.open", json!({}));
    h.run_steps(3);
    assert_eq!(count(h.state()), 2);
    assert_eq!(h.state().engine.path(), Some("Zeichnung ä.fig"));
    assert!(!h.state().engine.is_modified());
    // Edit, then save: a download of the same name with the new object in it.
    h.state_mut()
        .run("shape.rectangle", json!({"x": 0, "y": 0, "width": 300, "height": 300}));
    assert!(h.state().engine.is_modified());
    h.state_mut().run("app.save", json!({}));
    h.run_steps(2);
    let downloads = dialogs.downloads();
    assert_eq!(downloads.len(), 1);
    assert_eq!(downloads[0].0, "Zeichnung ä.fig");
    assert!(!h.state().engine.is_modified());
    // Opening the download gives the edited figure back.
    let (app2, _) = web_app(vec![WebPick {
        name: downloads[0].0.clone(),
        bytes: downloads[0].1.clone(),
    }]);
    let mut h2 = harness(app2);
    h2.state_mut().run("app.open", json!({}));
    h2.run_steps(3);
    assert_eq!(count(h2.state()), 3);
}

#[test]
fn save_of_a_new_figure_is_figure_fig_and_save_as_too() {
    let (mut app, dialogs) = web_app(vec![]);
    app.run("shape.rectangle", json!({"x": 0, "y": 0, "width": 300, "height": 300}));
    let mut h = harness(app);
    h.state_mut().run("app.save_as", json!({}));
    h.run_steps(2);
    let d = dialogs.downloads();
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].0, "figure.fig");
    assert!(String::from_utf8_lossy(&d[0].1).starts_with("#FIG 3.2"));
}

#[test]
fn dropped_file_opens_and_asks_about_unsaved_changes() {
    let (mut app, _) = web_app(vec![]);
    app.run("shape.rectangle", json!({"x": 0, "y": 0, "width": 300, "height": 300}));
    let mut h = harness(app);
    assert!(h.state().engine.is_modified());
    h.input_mut().dropped_files.push(egui::DroppedFile {
        name: "dropped.fig".into(),
        bytes: Some(fig_bytes(2).into()),
        ..Default::default()
    });
    h.run_steps(3);
    // The drawing has unsaved changes: the question comes first.
    assert_eq!(count(h.state()), 1);
    assert!(h.state().pending.is_some());
    h.state_mut().engine.set_modified(false);
    h.state_mut().pending = None;
    h.input_mut().dropped_files.push(egui::DroppedFile {
        name: "dropped.fig".into(),
        bytes: Some(fig_bytes(2).into()),
        ..Default::default()
    });
    h.run_steps(3);
    assert_eq!(count(h.state()), 2);
    assert_eq!(h.state().engine.path(), Some("dropped.fig"));
}

#[test]
fn export_is_a_download_and_the_embedded_figure_opens_again() {
    let (mut app, dialogs) = web_app(vec![]);
    app.run("shape.rectangle", json!({"x": 0, "y": 0, "width": 1200, "height": 600}));
    app.run(
        "shape.rectangle",
        json!({"x": 2000, "y": 0, "width": 1200, "height": 600}),
    );
    let mut h = harness(app);
    h.state_mut().run("app.export", json!({}));
    h.run_steps(2);
    h.get_by_label("Export…").click();
    h.run_steps(3);
    let d = dialogs.downloads();
    assert_eq!(d.len(), 1, "one download per exported file");
    assert!(d[0].0.starts_with("figure."), "named after the figure: {}", d[0].0);
    assert!(!d[0].1.is_empty());
    // A PDF export with the figure embedded opens as a .fig of its own name.
    let r = h
        .state_mut()
        .engine
        .execute("file.export_data", &json!({"format": "pdf", "embed": true}))
        .unwrap();
    let pdf = rustfig_engine::base64_decode(r["base64"].as_str().unwrap());
    let (app2, _) = web_app(vec![WebPick {
        name: "my drawing.pdf".into(),
        bytes: pdf,
    }]);
    let mut h2 = harness(app2);
    h2.state_mut().run("app.open", json!({}));
    h2.run_steps(3);
    assert_eq!(count(h2.state()), 2);
    assert_eq!(h2.state().engine.path(), Some("my drawing.fig"));
    assert!(h2.state().engine.is_modified());
}

#[test]
fn hostile_picks_are_errors_not_panics() {
    for bytes in [vec![], vec![0xff, 0xfe, 0, 1, 2], b"#FIG 3.2\nnonsense".to_vec()] {
        let (app, _) = web_app(vec![WebPick {
            name: "../../x\n.fig".into(),
            bytes,
        }]);
        let mut h = harness(app);
        h.state_mut().run("app.open", json!({}));
        h.run_steps(3);
        // The same bytes as a merge or a picture are errors too.
        h.state_mut().run("app.merge", json!({}));
        h.state_mut().run("app.import_picture", json!({}));
        h.run_steps(3);
    }
}

#[test]
fn merge_through_the_browser_picker_adds_a_compound() {
    let (app, _) = web_app(vec![WebPick {
        name: "more.fig".into(),
        bytes: fig_bytes(2),
    }]);
    let mut h = harness(app);
    h.state_mut()
        .run("shape.rectangle", json!({"x": 0, "y": 0, "width": 300, "height": 300}));
    h.state_mut().run("app.merge", json!({}));
    h.run_steps(3);
    // The rectangle and one compound holding the two merged ones; the file keeps its name.
    assert_eq!(count(h.state()), 2);
    assert!(h.state().engine.is_modified());
    assert_eq!(h.state().engine.path(), None);
}

/// A 1 x 1 PNG.
const PNG_1X1: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

#[test]
fn picture_through_the_browser_picker_is_kept_by_name() {
    let (app, _) = web_app(vec![WebPick {
        name: "C:\\fakepath\\web-ui-test-pixel.png".into(),
        bytes: rustfig_engine::base64_decode(PNG_1X1),
    }]);
    let mut h = harness(app);
    h.state_mut().run("app.import_picture", json!({}));
    h.run_steps(3);
    assert_eq!(count(h.state()), 1);
    let id = h.state().engine.doc().objects[0].id.0;
    let r = h
        .state_mut()
        .engine
        .execute("picture.info", &json!({"id": id}))
        .unwrap();
    assert_eq!(r["width"], 1);
    assert_eq!(r["exists"], true);
    // The figure keeps the file name only.
    let saved = h.state_mut().engine.execute("file.save_data", &json!({})).unwrap();
    assert!(saved["data"].as_str().unwrap().contains("web-ui-test-pixel.png"));
}
