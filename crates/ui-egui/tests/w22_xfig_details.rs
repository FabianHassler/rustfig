#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! W22-8 .. W22-15: details where RustFig differed from the real xfig
//! (`docs/xfig-compare.md`): the mode messages, the Glue hint, the depth
//! pop-up, the order of the edit panel, the mode panel's Trim button, the
//! message after opening a file and the indicator labels.

use egui::vec2;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use rustfig_engine::{Engine, ToolKind};
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

fn app() -> App {
    let mut a = App::with_engine(Engine::new());
    a.file_dialogs = Box::new(ScriptedDialogs::default());
    a
}

/// W22-8: selecting a mode puts xfig's sentence with its key in the message
/// panel.
#[test]
fn mode_selection_shows_xfigs_sentence() {
    let table = [
        (ToolKind::Box, "Rectangular BOX drawing   (b)"),
        (ToolKind::Polyline, "POLYLINE drawing   (l)"),
        (ToolKind::Move, "MOVE objects   (m)"),
        (ToolKind::Glue, "GLUE objects into COMPOUND object   (g)"),
        (
            ToolKind::CircleDiameter,
            "CIRCLE/ELLIPSE drawing: specify DIAMETER(S)   (Shift-c or e)",
        ),
        (ToolKind::Delete, "DELETE objects   (d)"),
    ];
    let mut h = harness(app());
    for (tool, text) in table {
        h.state_mut().run(tool.id(), json!({}));
        h.run_steps(2);
        assert_eq!(
            h.state().message.as_ref().map(|m| m.text.as_str()),
            Some(text),
            "{tool:?}"
        );
        h.get_by_label(text);
    }
}

/// Every mode that has an xfig sentence puts it in the message.
#[test]
fn every_mode_with_a_sentence_shows_it() {
    let mut e = Engine::new();
    for t in ToolKind::ALL {
        if let Some(text) = t.xfig_message() {
            e.set_tool(t);
            assert_eq!(e.status(), text, "{t:?}");
        }
    }
}

/// W22-9: Glue's right button reads "compound tagged" from the start.
#[test]
fn glue_hint_from_the_start() {
    let mut e = Engine::new();
    e.set_tool(ToolKind::Glue);
    assert_eq!(e.mouse_hints().right, "compound tagged");
}

/// W22-10: the Depth button opens xfig's number pop-up; Set applies.
#[test]
fn depth_button_opens_a_popup_that_sets() {
    let mut h = harness(app());
    h.state_mut().run("tool.box", json!({}));
    h.run_steps(2);
    h.get_by_label("Depth: 50").click();
    h.run_steps(3);
    h.get_by_label("Value");
    h.get_by_label("Cancel");
    // Cancel leaves the depth alone.
    h.get_by_label("Cancel").click();
    h.run_steps(3);
    assert_eq!(h.state().engine.indicator.depth, 50);
    assert!(h.query_by_label("Value").is_none());
    h.get_by_label("Depth: 50").click();
    h.run_steps(3);
    h.get_by_label("Set").click();
    h.run_steps(3);
    assert_eq!(h.state().engine.indicator.depth, 50);
    assert!(h.query_by_label("Value").is_none());
}

/// W22-12: Trim is the last editing button; xfig's buttons keep their
/// positions.
#[test]
fn trim_button_comes_last_in_the_editing_modes() {
    let order = rustfig_ui_egui::panels::editing_tools();
    assert_eq!(order.first(), Some(&ToolKind::Glue));
    assert_eq!(order.get(4), Some(&ToolKind::Chop));
    assert_eq!(order.get(5), Some(&ToolKind::Scale));
    assert_eq!(order.last(), Some(&ToolKind::Trim));
}

/// W22-14: after opening a file the message names the file, not its path.
#[test]
fn open_message_names_the_file_only() {
    let dir = std::env::temp_dir().join("rustfig_w22_open");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("fig one.rustfig");
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100}))
        .unwrap();
    e.execute("file.save", &json!({"path": path.to_string_lossy()}))
        .unwrap();
    let mut h = harness(app());
    h.state_mut().run("file.open", json!({"path": path.to_string_lossy()}));
    h.run_steps(2);
    let text = h.state().message.as_ref().unwrap().text.clone();
    assert!(text.contains("fig one.rustfig"), "{text}");
    assert!(!text.contains(&*dir.to_string_lossy()), "{text}");
    std::fs::remove_dir_all(&dir).ok();
}

/// W22-15: indicator labels as xfig: "hidden=off", the size with two decimals.
#[test]
fn indicator_labels_as_xfig() {
    let mut h = harness(app());
    h.state_mut().run("tool.text", json!({}));
    h.run_steps(2);
    h.get_by_label("Text Size: 12.00");
    h.get_by_label("Text Flags: hidden=off");
    // The right button shows the next flag.
    h.get_by_label("Text Flags: hidden=off").click_secondary();
    h.run_steps(2);
    h.get_by_label("Text Flags: TeX=off");
}

/// W22-11: Comments is the first row of the edit panel.
#[test]
fn edit_panel_starts_with_comments() {
    let mut h = harness(app());
    h.state_mut()
        .run("shape.rectangle", json!({"x": 0, "y": 0, "width": 600, "height": 300}));
    h.run_steps(2);
    {
        let app = h.state_mut();
        let id = app.engine.selection().first().copied().unwrap();
        app.dialogs.open_edit(&app.engine, id);
    }
    h.run_steps(3);
    let comments = h.get_by_label("Comments").rect().top();
    let depth = h.get_by_label("Depth").rect().top();
    assert!(comments < depth, "{comments} {depth}");
}
