#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Headless UI tests: the real App driven through egui_kittest.

use egui::{pos2, vec2};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use rustfig_engine::{Engine, ToolKind};
use rustfig_ui_egui::dialogs::ScriptedDialogs;
use rustfig_ui_egui::{App, Layout};
use serde_json::json;

fn harness(app: App) -> Harness<'static, App> {
    let mut h = Harness::builder()
        .with_size(vec2(1280.0, 860.0))
        .build_state(|ctx, app: &mut App| app.ui(ctx), app);
    h.run_steps(3);
    h
}

fn app() -> App {
    let mut a = App::with_engine(Engine::new());
    a.file_dialogs = Box::new(ScriptedDialogs::default());
    a
}

#[test]
fn classic_layout_has_xfig_regions() {
    let h = harness(app());
    for menu in ["File", "Edit", "View", "Snap", "Help"] {
        h.get_by_label(menu);
    }
    h.get_by_label("(untitled)");
    h.get_by_label_contains("Left: select / drag");
    h.get_by_label("1 in = 1.00 in");
    h.get_by_label("Depths");
    h.get_by_label("Drawing");
    h.get_by_label("Editing");
}

#[test]
fn modern_layout_hides_the_classic_strip() {
    let mut a = app();
    a.layout = Layout::Modern;
    let h = harness(a);
    h.get_by_label("File");
    assert!(h.query_by_label_contains("Left: ").is_none());
}

#[test]
fn layout_toggle_command() {
    let mut h = harness(app());
    h.state_mut().run("app.toggle_layout", json!({}));
    h.run_steps(2);
    assert_eq!(h.state().layout, Layout::Modern);
    assert!(h.query_by_label_contains("Left: ").is_none());
}

#[test]
fn every_tool_button_selects_its_tool() {
    let mut h = harness(app());
    for t in ToolKind::ALL {
        h.get_by_label(t.label()).click();
        h.run_steps(2);
        assert_eq!(h.state().engine.tool(), t, "button {}", t.label());
        // The indicator follows the tool.
        let hint = h.state().engine.mouse_hints().left;
        h.get_by_label_contains(&format!("Left: {hint}"));
    }
}

#[test]
fn draw_a_box_with_the_mouse() {
    let mut h = harness(app());
    h.get_by_label("Box").click();
    h.run_steps(2);
    h.hover_at(pos2(400.0, 300.0));
    h.run_steps(1);
    h.drag_at(pos2(400.0, 300.0));
    h.run_steps(1);
    for i in 1..=5 {
        h.hover_at(pos2(400.0 + 40.0 * i as f32, 300.0 + 25.0 * i as f32));
        h.run_steps(1);
    }
    h.drop_at(pos2(600.0, 425.0));
    h.run_steps(2);
    assert_eq!(h.state().engine.doc().objects.len(), 1);
    assert_eq!(h.state().engine.doc().objects[0].type_name(), "box");
}

#[test]
fn unsaved_changes_prompt() {
    let mut h = harness(app());
    h.state_mut()
        .run("shape.rectangle", json!({"x": 0, "y": 0, "width": 600, "height": 300}));
    h.state_mut().run("app.new", json!({}));
    h.run_steps(2);
    h.get_by_label("Save changes?");
    h.get_by_label("Discard").click();
    h.run_steps(2);
    assert!(h.state().engine.doc().objects.is_empty());
}

#[test]
fn messages_reach_the_message_panel_and_history() {
    let mut h = harness(app());
    h.state_mut().error("something went wrong");
    h.run_steps(2);
    h.get_by_label("something went wrong").click();
    h.run_steps(2);
    h.get_by_label("Messages");
}

#[test]
fn dialogs_open() {
    let mut h = harness(app());
    for (cmd, title) in [
        ("app.export", "Export"),
        ("app.search", "Search and Replace"),
        ("app.document_settings", "Document Settings"),
        ("app.commands", "Commands"),
        ("app.about", "About RustFig"),
    ] {
        h.state_mut().run(cmd, json!({}));
        h.run_steps(2);
        assert!(h.query_all_by_label(title).count() >= 1, "{cmd} did not open {title}");
    }
}

fn click_button(
    h: &mut Harness<'static, App>,
    pos: egui::Pos2,
    button: egui::PointerButton,
    modifiers: egui::Modifiers,
) {
    h.hover_at(pos);
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers,
        });
        h.run_steps(1);
    }
}

#[test]
fn polyline_with_xfig_buttons() {
    let mut h = harness(app());
    h.get_by_label("Polyline").click();
    h.run_steps(2);
    let none = egui::Modifiers::NONE;
    click_button(&mut h, pos2(400.0, 300.0), egui::PointerButton::Primary, none);
    h.get_by_label_contains("middle: final point");
    click_button(&mut h, pos2(500.0, 300.0), egui::PointerButton::Primary, none);
    click_button(&mut h, pos2(500.0, 400.0), egui::PointerButton::Middle, none);
    let doc = h.state().engine.doc();
    assert_eq!(doc.objects.len(), 1);
    assert_eq!(doc.objects[0].points().unwrap().len(), 3);
    // Alt+click stands in for the middle button on a two-button mouse.
    click_button(&mut h, pos2(400.0, 500.0), egui::PointerButton::Primary, none);
    click_button(
        &mut h,
        pos2(500.0, 500.0),
        egui::PointerButton::Primary,
        egui::Modifiers::ALT,
    );
    assert_eq!(h.state().engine.doc().objects.len(), 2);
    // The right button cancels.
    click_button(&mut h, pos2(400.0, 600.0), egui::PointerButton::Primary, none);
    click_button(&mut h, pos2(500.0, 600.0), egui::PointerButton::Secondary, none);
    assert_eq!(h.state().engine.doc().objects.len(), 2);
    assert!(!h.state().engine.tool_busy());
}

#[test]
fn mouse_buttons_menu_switches_scheme() {
    let mut h = harness(app());
    h.get_by_label_contains("middle: —");
    h.get_by_label("View").click();
    h.run_steps(2);
    h.get_by_label_contains("xfig mouse buttons").click();
    h.run_steps(2);
    assert_eq!(h.state().engine.mouse_scheme(), rustfig_engine::MouseScheme::Modern);
    h.get_by_label_contains("middle: pan");
}

#[test]
fn two_button_mapping() {
    use egui::{Modifiers, PointerButton};
    use rustfig_engine::Button;
    use rustfig_ui_egui::canvas::tool_button;
    assert_eq!(tool_button(PointerButton::Primary, Modifiers::NONE), Some(Button::Left));
    assert_eq!(
        tool_button(PointerButton::Primary, Modifiers::ALT),
        Some(Button::Middle)
    );
    assert_eq!(
        tool_button(PointerButton::Primary, Modifiers::CTRL),
        Some(Button::Right)
    );
    assert_eq!(
        tool_button(PointerButton::Middle, Modifiers::NONE),
        Some(Button::Middle)
    );
    assert_eq!(
        tool_button(PointerButton::Secondary, Modifiers::NONE),
        Some(Button::Right)
    );
}

fn with_depths() -> App {
    let mut a = app();
    for (y, d) in [(0, 10), (900, 20), (1800, 20), (2700, 30)] {
        a.engine
            .execute(
                "shape.rectangle",
                &json!({"x": 0, "y": y, "width": 600, "height": 300, "depth": d}),
            )
            .unwrap();
    }
    a
}

#[test]
fn depth_panel_rows_buttons_and_display() {
    let mut h = harness(with_depths());
    h.get_by_label("Depth 20 (2 objects)");
    h.get_by_label("Depth 10 (1 object)").click();
    h.run_steps(2);
    assert_eq!(
        h.state().engine.hidden_depths.iter().copied().collect::<Vec<_>>(),
        vec![10]
    );
    h.get_by_label("Toggle").click();
    h.run_steps(2);
    assert_eq!(
        h.state().engine.hidden_depths.iter().copied().collect::<Vec<_>>(),
        vec![20, 30]
    );
    h.get_by_label("All On").click();
    h.run_steps(2);
    assert!(h.state().engine.hidden_depths.is_empty());
    h.get_by_label("All Off").click();
    h.run_steps(2);
    assert_eq!(h.state().engine.hidden_depths.len(), 3);
    h.get_by_label("Blank").click();
    h.run_steps(2);
    assert!(!h.state().engine.gray_hidden_depths);
}

#[test]
fn dragging_across_depth_checkboxes_switches_them_all() {
    let mut h = harness(with_depths());
    let first = h.get_by_label("Depth 10 (1 object)").rect().center();
    let last = h.get_by_label("Depth 30 (1 object)").rect().center();
    h.hover_at(first);
    h.run_steps(1);
    h.event(egui::Event::PointerButton {
        pos: first,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(1);
    for i in 1..=6 {
        h.hover_at(first.lerp(last, i as f32 / 6.0));
        h.run_steps(1);
    }
    h.event(egui::Event::PointerButton {
        pos: last,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(2);
    // The first checkbox was on, so the sweep turned every one off, once.
    assert_eq!(h.state().engine.hidden_depths.len(), 3);
}

#[test]
fn a_mouse_click_on_a_depth_toggles_it_once() {
    let mut h = harness(with_depths());
    let at = h.get_by_label("Depth 20 (2 objects)").rect().center();
    click_button(&mut h, at, egui::PointerButton::Primary, egui::Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(
        h.state().engine.hidden_depths.iter().copied().collect::<Vec<_>>(),
        vec![20]
    );
    click_button(&mut h, at, egui::PointerButton::Primary, egui::Modifiers::NONE);
    h.run_steps(2);
    assert!(h.state().engine.hidden_depths.is_empty());
}
