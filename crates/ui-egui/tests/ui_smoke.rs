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
    h.get_by_label("Units: 1in = 1.00in");
    h.get_by_label("Top ruler");
    h.get_by_label("Side ruler");
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
    // The mode panel has xfig's modes; RustFig's other tools are in the
    // Tools menu.
    let menu_only = [
        ToolKind::Select,
        ToolKind::OpenClose,
        ToolKind::XSpline,
        ToolKind::XSplineClosed,
        ToolKind::EllipseRadii,
        ToolKind::EllipseDiameter,
    ];
    for t in ToolKind::ALL.into_iter().filter(|t| !menu_only.contains(t)) {
        // The mode panel scrolls in a small window, as a user would.
        h.get_by_label(t.label()).scroll_to_me();
        // Scrolling is animated: let it settle.
        h.run_steps(30);
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
fn global_settings_switch_the_mouse_scheme() {
    let mut h = harness(app());
    h.get_by_label_contains("middle: —");
    // xfig's Edit › Global settings holds RustFig's preferences.
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label_contains("Global settings...").click();
    h.run_steps(4);
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

#[test]
fn attribute_panel_shows_the_modes_attributes() {
    let mut h = harness(app());
    let select = |h: &mut Harness<'static, App>, tool: ToolKind| {
        h.state_mut().engine.set_tool(tool);
        h.run_steps(2);
    };
    select(&mut h, ToolKind::Polyline);
    h.get_by_label("Line Width: 1");
    h.get_by_label("Arrow Mode: none");
    h.get_by_label_contains("Zoom: ");
    assert!(h.query_by_label_contains("Text Size").is_none());
    select(&mut h, ToolKind::Text);
    h.get_by_label("Text Size: 12");
    h.get_by_label("Text Step: 1.0");
    assert!(h.query_by_label_contains("Line Style").is_none());
    // Delete has only Zoom and Grid Mode, as in xfig.
    select(&mut h, ToolKind::Delete);
    assert!(h.query_by_label_contains("Point Posn").is_none());
    h.get_by_label_contains("Grid Mode: ");
}

#[test]
fn attribute_buttons_step_and_choose() {
    let mut h = harness(app());
    h.state_mut().engine.set_tool(ToolKind::Polyline);
    h.run_steps(2);
    let at = h.get_by_label("Line Width: 1").rect().center();
    // Right button: next/increase; middle: previous/decrease.
    click_button(&mut h, at, egui::PointerButton::Secondary, egui::Modifiers::NONE);
    click_button(&mut h, at, egui::PointerButton::Secondary, egui::Modifiers::NONE);
    assert_eq!(h.state().engine.indicator.thickness, 3);
    click_button(&mut h, at, egui::PointerButton::Middle, egui::Modifiers::NONE);
    assert_eq!(h.state().engine.indicator.thickness, 2);
    // Hovering shows xfig's button functions in the mouse indicator.
    h.get_by_label_contains("Left: Menu; middle: Dec/Prev; right: Inc/Next");
    // Left click opens the choice panel; pick a line style.
    h.get_by_label("Line Style: solid").click();
    h.run_steps(2);
    h.get_by_label("dashed").click();
    h.run_steps(2);
    assert_eq!(h.state().engine.indicator.line, rustfig_model::LineKind::Dashed);
    h.get_by_label_contains("Arrow Type: ").click();
    h.run_steps(2);
    h.get_by_label("solid type 1").click();
    h.run_steps(2);
    let a = h.state().engine.indicator.arrow;
    assert_eq!((a.kind, a.style), (1, 1));
}

#[test]
fn rulers_pan_and_units_box() {
    let mut h = harness(app());
    let x0 = h.state().engine.view.origin.x;
    let top = h.get_by_label("Top ruler").rect().center();
    // Left: pan left by 1/2 inch of the drawing (at this zoom).
    click_button(&mut h, top, egui::PointerButton::Primary, egui::Modifiers::NONE);
    let zoom = h.state().engine.view.zoom;
    let moved = h.state().engine.view.origin.x - x0;
    assert!((moved - 600.0 / zoom).abs() < 1e-6, "moved {moved}");
    h.get_by_label_contains("Left: Pan Left; middle: Drag; right: Pan Right");
    click_button(&mut h, top, egui::PointerButton::Secondary, egui::Modifiers::NONE);
    assert!((h.state().engine.view.origin.x - x0).abs() < 1e-6);
    // Units box: right opens the Unit/Scale panel.
    let ub = h.get_by_label("Units: 1in = 1.00in").rect().center();
    click_button(&mut h, ub, egui::PointerButton::Secondary, egui::Modifiers::NONE);
    // Let the new window settle (egui sizes it in its first frame).
    h.run_steps(3);
    let metric = h.get_by_label("Metric (cm)").rect().center();
    click_button(&mut h, metric, egui::PointerButton::Primary, egui::Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(h.state().engine.doc().settings.units, rustfig_model::Units::Metric);
    h.get_by_label("Units: 1cm = 1.00cm");
}

#[test]
fn snap_menu_arms_a_snap_mode() {
    let mut h = harness(app());
    h.get_by_label("Snap Mode: None");
    h.get_by_label("Snap").click();
    h.run_steps(2);
    h.get_by_label("Endpoint").click();
    h.run_steps(2);
    h.get_by_label("Snap Mode: Endpoint");
    h.get_by_label("Snap").click();
    h.run_steps(2);
    h.get_by_label("Hold").click();
    h.run_steps(2);
    h.get_by_label("Snap Mode: Endpoint (held)");
}

fn press_key(h: &mut Harness<'static, App>, key: egui::Key, modifiers: egui::Modifiers) {
    h.event(egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    });
    h.run_steps(2);
}

#[test]
fn xfig_keys_select_modes_and_run_menu_entries() {
    let mut h = harness(app());
    press_key(&mut h, egui::Key::S, egui::Modifiers::NONE);
    assert_eq!(h.state().engine.tool(), ToolKind::SplineApprox, "xfig: s = spline");
    press_key(&mut h, egui::Key::A, egui::Modifiers::SHIFT);
    assert_eq!(h.state().engine.tool(), ToolKind::Arrow, "xfig: Shift-a = arrows");
    // xfig's Ctrl mode keys use the Control key itself.
    let ctrl = egui::Modifiers {
        ctrl: true,
        ..egui::Modifiers::NONE
    };
    press_key(&mut h, egui::Key::S, ctrl);
    assert_eq!(h.state().engine.tool(), ToolKind::Scale, "xfig: Ctrl-s = scale");
    // Meta-U (Alt/Option) undoes.
    h.state_mut()
        .run("shape.rectangle", json!({"x": 0, "y": 0, "width": 600, "height": 300}));
    press_key(&mut h, egui::Key::U, egui::Modifiers::ALT);
    assert!(h.state().engine.doc().objects.is_empty(), "Meta-U undoes");
    // The menus show xfig's accelerators.
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label_contains("Meta-U");
    // Turned off, Ctrl-s no longer picks Scale.
    h.state_mut().run("app.toggle_xfig_keys", json!({"on": false}));
    h.state_mut().engine.set_tool(ToolKind::Select);
    press_key(&mut h, egui::Key::S, ctrl);
    assert_ne!(h.state().engine.tool(), ToolKind::Scale);
}

#[test]
fn classic_menus_are_xfigs() {
    let mut h = harness(app());
    // xfig has no Object or Tools menu.
    assert!(h.query_by_label("Object").is_none());
    assert!(h.query_by_label("Tools").is_none());
    h.get_by_label("File").click();
    h.run_steps(2);
    // (Labels include xfig's accelerator, e.g. "Open... Meta-O".)
    for entry in ["Open...", "Merge...", "Save As...", "Export...", "Exit"] {
        h.get_by_label_contains(entry);
    }
    // Entries RustFig doesn't have yet are there, greyed out.
    use egui_kittest::kittest::NodeT;
    assert!(h.get_by_label_contains("Print...").accesskit_node().is_disabled());
}
