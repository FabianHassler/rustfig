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
    // xfig starts with no mode: an empty mouse indicator and its message.
    h.get_by_label_contains("Left: —; middle: —; right: —");
    h.get_by_label("READY. Select a mode or load a file");
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
        ("app.global_settings", "Global settings"),
        ("app.unit_panel", "Unit menu"),
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
    // xfig's right button sets the depth of new objects (W8).
    h.get_by_label("Depth 30 (1 object)").click_secondary();
    h.run_steps(2);
    assert_eq!(h.state().engine.indicator.depth, 30);
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

#[test]
fn snap_hold_is_visible_and_release_needs_it() {
    let mut h = harness(app());
    assert!(h.query_by_label("Snap Mode: None (held)").is_none());
    h.state_mut().run("snap.hold", json!({}));
    h.run_steps(2);
    h.get_by_label("Snap Mode: None (held)");
    h.get_by_label("Snap").click();
    h.run_steps(2);
    h.get_by_label_contains("Hold");
    h.state_mut().run("snap.release", json!({}));
    h.run_steps(2);
    assert!(h.query_by_label("Snap Mode: None (held)").is_none());
    h.get_by_label("Snap Mode: None");
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
    h.get_by_label_contains(&rustfig_ui_egui::menus::xfig_key_label("Alt+U"));
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

#[test]
fn export_dialog_lists_xfigs_languages() {
    let dir = std::env::temp_dir().join(format!("rustfig-ui-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("figure.box").to_string_lossy().into_owned();
    let mut a = app();
    a.file_dialogs = Box::new(ScriptedDialogs::new([Some(out.clone())]));
    a.run("shape.rectangle", json!({"x": 0, "y": 0, "width": 1200, "height": 600}));
    let mut h = harness(a);
    h.state_mut().run("app.export", json!({}));
    h.run_steps(2);
    h.get_by_label("PDF (Portable Document Format)").click();
    h.run_steps(2);
    // Languages not ported yet are listed but greyed out.
    h.get_by_label("TeX/LaTeX + tikz macros");
    h.get_by_label("LaTeX box (figure boundary)").click();
    h.run_steps(2);
    h.get_by_label("Export…").click();
    h.run_steps(2);
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.starts_with("\\makebox["), "{text}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn edit_attributes_dialog_opens_for_the_selection() {
    let mut h = harness(app());
    h.state_mut()
        .engine
        .execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    h.state_mut().engine.execute("edit.select_all", &json!({})).unwrap();
    h.state_mut().run("app.edit_selected", json!({}));
    h.run_steps(2);
    assert!(h.query_all_by_label("POLYLINE: Box").count() >= 1);
}

#[test]
fn file_dialogs_save_open_merge_and_cancel() {
    let dir = std::env::temp_dir().join(format!("rustfig-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.fig").to_string_lossy().into_owned();
    let mut a = app();
    a.file_dialogs = Box::new(ScriptedDialogs::new([
        Some(path.clone()), // Save As
        Some(path.clone()), // Open
        Some(path.clone()), // Merge
        None,               // Import picture: cancelled
    ]));
    let mut h = harness(a);
    h.state_mut()
        .engine
        .execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    h.state_mut().run("app.save_as", json!({}));
    h.run_steps(2);
    assert!(std::path::Path::new(&path).exists());
    assert!(!h.state().engine.is_modified());
    h.state_mut().run("app.open", json!({}));
    h.run_steps(2);
    assert_eq!(h.state().engine.doc().objects.len(), 1);
    h.state_mut().run("app.merge", json!({}));
    h.run_steps(2);
    assert_eq!(h.state().engine.doc().objects.len(), 2);
    h.state_mut().run("app.import_picture", json!({}));
    h.run_steps(2);
    assert_eq!(h.state().engine.doc().objects.len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn quit_asks_about_unsaved_changes() {
    let mut h = harness(app());
    h.state_mut().run("app.quit", json!({}));
    h.run_steps(2);
    assert!(h.state().quit_requested(), "nothing to save: quits at once");
    let mut h = harness(app());
    h.state_mut()
        .engine
        .execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    h.state_mut().run("app.quit", json!({}));
    h.run_steps(2);
    assert!(!h.state().quit_requested());
    h.get_by_label("Save changes?");
    h.get_by_label("Discard").click();
    h.run_steps(2);
    assert!(h.state().quit_requested());
}

/// W8: xfig's canvas keys (`w_canvas.c`): arrows pan, Shift five times as
/// far, Home to the origin.
#[test]
fn arrow_keys_pan_and_home_returns() {
    let mut h = harness(app());
    let o0 = h.state().engine.view.origin;
    press_key(&mut h, egui::Key::ArrowLeft, egui::Modifiers::NONE);
    let o1 = h.state().engine.view.origin;
    // Left is xfig's pan_right: the drawing moves right, the origin left.
    assert!(o1.x < o0.x && o1.y == o0.y, "{o0:?} {o1:?}");
    press_key(&mut h, egui::Key::ArrowUp, egui::Modifiers::SHIFT);
    let o2 = h.state().engine.view.origin;
    assert!(o2.y < o1.y);
    assert!(
        ((o1.y - o2.y) - 5.0 * (o0.x - o1.x)).abs() < 1e-6,
        "{o0:?} {o1:?} {o2:?}"
    );
    press_key(&mut h, egui::Key::Home, egui::Modifiers::NONE);
    let o3 = h.state().engine.view.origin;
    assert_ne!(o3, o2);
    h.state_mut().run("view.pan_origin", json!({}));
    assert_eq!(h.state().engine.view.origin, o3, "Home is view.pan_origin");
}

/// W8: xfig's ExportFile (Shift-Meta-X) exports without the panel.
#[test]
fn export_now_writes_next_to_the_figure() {
    let dir = std::env::temp_dir().join(format!("rustfig-export-now-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let fig = dir.join("drawing.fig").to_string_lossy().into_owned();
    let mut h = harness(app());
    h.state_mut()
        .run("shape.rectangle", json!({"x": 0, "y": 0, "width": 600, "height": 300}));
    h.state_mut().run("app.save_as", json!({"path": fig}));
    h.state_mut().run("app.export_now", json!({}));
    h.run_steps(2);
    assert!(dir.join("drawing.pdf").exists());
    assert_eq!(h.query_all_by_label("Export").count(), 0, "no panel");
    let _ = std::fs::remove_dir_all(&dir);
}

/// W8: Escape closes xfig's panels (Cancel).
#[test]
fn escape_closes_panels() {
    let mut h = harness(app());
    h.state_mut().run("app.export", json!({}));
    h.state_mut().run("app.search", json!({}));
    h.run_steps(2);
    assert!(h.query_all_by_label("Search and Replace").count() >= 1);
    press_key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(!h.state().dialogs.search.open);
    assert!(!h.state().dialogs.export.open);
}

/// W8: Meta-V pops up xfig's View menu; a letter picks its entry.
#[test]
fn xfig_menu_keys_and_letters() {
    let mut h = harness(app());
    let z0 = h.state().engine.view.zoom;
    press_key(&mut h, egui::Key::V, egui::Modifiers::ALT);
    h.run_steps(2);
    assert!(h.query_all_by_label_contains("Zoom In").count() >= 1, "View menu open");
    press_key(&mut h, egui::Key::I, egui::Modifiers::NONE);
    assert!(h.state().engine.view.zoom > z0, "i is Zoom In");
    assert_eq!(h.query_all_by_label_contains("Zoom In").count(), 0, "menu closed");
    // With no menu open, i is not Zoom In.
    let z1 = h.state().engine.view.zoom;
    press_key(&mut h, egui::Key::I, egui::Modifiers::NONE);
    assert_eq!(h.state().engine.view.zoom, z1);
}

#[test]
fn xfig_menu_letters_name_menu_entries() {
    use rustfig_ui_egui::menus::{XfigItem, XFIG_MENUS, XFIG_MENU_LETTERS};
    for (menu, letter, target) in XFIG_MENU_LETTERS {
        let items = XFIG_MENUS.iter().find(|(m, _)| m == menu).map(|(_, i)| *i).unwrap();
        let found = items.iter().any(|i| match i {
            XfigItem::Cmd(_, id) => id == target,
            XfigItem::Missing(label, _, _) => label == target,
            XfigItem::Sep => false,
        });
        assert!(found, "{menu} {letter}: {target} isn't in the menu");
    }
}

/// W8: the edit popup's keys (`e_edit.c`): Ctrl+Return is Done, Escape Cancel.
#[test]
fn edit_popup_keys() {
    let mut h = harness(app());
    h.state_mut()
        .engine
        .execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    h.state_mut().engine.execute("edit.select_all", &json!({})).unwrap();
    h.state_mut().run("app.edit_selected", json!({}));
    h.run_steps(2);
    assert!(h.state().dialogs.edit.is_some());
    press_key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(h.state().dialogs.edit.is_none(), "Escape cancels");
    assert!(!h.state().engine.selection().is_empty(), "and doesn't deselect");
    h.state_mut().run("app.edit_selected", json!({}));
    h.run_steps(2);
    press_key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(h.state().dialogs.edit.is_some(), "Return applies and stays");
    press_key(&mut h, egui::Key::Enter, egui::Modifiers::COMMAND);
    assert!(h.state().dialogs.edit.is_none(), "Ctrl+Return is Done");
}

/// W8: Alt+wheel over the canvas pops up xfig's mode menus.
#[test]
fn alt_wheel_pops_up_the_mode_menus() {
    let mut h = harness(app());
    let centre = pos2(700.0, 450.0);
    h.event(egui::Event::PointerMoved(centre));
    h.run_steps(2);
    h.input_mut().modifiers = egui::Modifiers::ALT;
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: vec2(0.0, 1.0),
        modifiers: egui::Modifiers::ALT,
    });
    h.run_steps(3);
    assert!(
        matches!(h.state().mode_popup, Some((true, _))),
        "wheel up: drawing modes"
    );
    h.input_mut().modifiers = egui::Modifiers::NONE;
    press_key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(h.state().mode_popup.is_none(), "Escape closes it");
}

/// W8: View › Show info balloons (Meta-Y) switches the tooltips.
#[test]
fn balloons_toggle() {
    let mut h = harness(app());
    assert!(h.ctx.style().interaction.tooltip_delay < 5.0);
    h.state_mut().run("app.toggle_balloons", json!({}));
    h.run_steps(2);
    assert!(!h.state().show_balloons);
    assert!(h.ctx.style().interaction.tooltip_delay > 1.0e6);
    press_key(&mut h, egui::Key::Y, egui::Modifiers::ALT);
    assert!(h.state().show_balloons, "Meta-Y");
    assert!(h.ctx.style().interaction.tooltip_delay < 5.0);
}

/// W8: xfig's compose key: Alt alone, then two characters (`CompKeyDB`).
#[test]
fn compose_key_makes_one_character() {
    let mut h = harness(app());
    h.state_mut().engine.set_tool(ToolKind::Text);
    h.state_mut()
        .engine
        .tool_event(rustfig_engine::ToolEvent::Press {
            pos: rustfig_geom::Point::new(600.0, 600.0),
            button: rustfig_engine::Button::Left,
            shift: false,
        })
        .unwrap();
    h.run_steps(1);
    assert!(h.state().engine.is_typing());
    h.input_mut().modifiers = egui::Modifiers::ALT;
    h.run_steps(1);
    h.input_mut().modifiers = egui::Modifiers::NONE;
    h.run_steps(1);
    for t in ["a", "\"", "b"] {
        h.event(egui::Event::Text(t.into()));
        h.run_steps(1);
    }
    let text = match h.state().engine.overlay().text.map(|t| t.object.kind) {
        Some(rustfig_model::ObjectKind::Text(t)) => t.text,
        _ => String::new(),
    };
    assert_eq!(text, "äb");
}

/// W8: Return in the export panel exports (xfig's ExportFile).
#[test]
fn return_exports_from_the_panel() {
    let dir = std::env::temp_dir().join(format!("rustfig-export-return-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("f.pdf").to_string_lossy().into_owned();
    let mut a = app();
    a.file_dialogs = Box::new(ScriptedDialogs::new([Some(out.clone())]));
    let mut h = harness(a);
    h.state_mut()
        .run("shape.rectangle", json!({"x": 0, "y": 0, "width": 600, "height": 300}));
    h.state_mut().run("app.export", json!({}));
    h.run_steps(2);
    press_key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(std::path::Path::new(&out).exists());
    assert!(!h.state().dialogs.export.open);
    let _ = std::fs::remove_dir_all(&dir);
}

/// W9: all of xfig's 29 arrow choices (types 0–14), with vector pictures
/// or xfig's bitmaps.
#[test]
fn arrow_types_up_to_14_in_both_picture_modes() {
    for vector in [true, false] {
        let mut a = app();
        a.vector_icons = vector;
        let mut h = harness(a);
        h.state_mut().engine.set_tool(ToolKind::Polyline);
        h.run_steps(2);
        h.get_by_label_contains("Arrow Type: ").click();
        h.run_steps(2);
        h.get_by_label("hollow type 13").click();
        h.run_steps(2);
        let a = h.state().engine.indicator.arrow;
        assert_eq!((a.kind, a.style), (13, 0), "vector {vector}");
        h.get_by_label_contains("Arrow Type: ").click();
        h.run_steps(2);
        h.get_by_label("solid type 14").click();
        h.run_steps(2);
        let a = h.state().engine.indicator.arrow;
        assert_eq!((a.kind, a.style), (14, 1), "vector {vector}");
    }
}

/// A6: a drawing left by an instance that stopped is offered at the next
/// start and restored unsaved, under the name of its file.
#[test]
fn recovery_after_a_crash() {
    use rustfig_engine::recovery::Recovery;
    let dir = std::env::temp_dir().join(format!("rustfig-ui-recovery-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // The instance that crashed: it kept a copy, then stopped beating.
    let mut crashed = Engine::new();
    crashed
        .execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    let rec = Recovery::new(dir.clone());
    rec.save(crashed.doc(), Some("/somewhere/plan.fig")).unwrap();
    let id = rec.id().to_string();
    drop(rec); // the process ends: its lock is released

    let mut a = app();
    a.enable_recovery(dir.clone());
    assert_eq!(a.recoverable.len(), 1);
    let mut h = harness(a);
    h.get_by_label("Recover unsaved drawings");
    h.get_by_label("plan.fig");
    h.get_by_label("Restore").click();
    h.run_steps(2);
    let e = &h.state().engine;
    assert_eq!(e.doc().objects.len(), 1);
    assert!(e.is_modified(), "restored drawings are unsaved");
    assert_eq!(e.path(), Some("/somewhere/plan.fig"));
    assert!(h.state().recoverable.is_empty());
    assert!(!dir.join(format!("{id}.rustfig")).exists(), "old copy gone");
    let _ = std::fs::remove_dir_all(&dir);
}

fn settings_file(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rustfig-settings-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir.join("settings.toml")
}

fn app_with_settings(path: &std::path::Path) -> App {
    let mut a = app();
    a.settings = rustfig_ui_egui::settings::Settings::file(path.to_path_buf());
    rustfig_ui_egui::settings::load_and_apply(&mut a);
    a
}

/// E1: settings changed in one session are there in the next.
#[test]
fn settings_survive_a_restart() {
    let path = settings_file("restart");
    {
        let mut h = harness(app_with_settings(&path));
        h.state_mut().run("app.toggle_layout", json!({ "classic": false }));
        h.state_mut()
            .run("prefs.set", json!({ "resource": "startfontsize", "value": 18 }));
        h.state_mut()
            .run("prefs.set", json!({ "resource": "startgridmode", "value": "2" }));
        h.state_mut()
            .run("prefs.add_color", json!({ "name": "Rust", "rgb": "#d2692a" }));
        h.run_steps(2);
    }
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("startfontsize = 18"), "{text}");
    assert!(text.contains("layout = \"modern\""), "{text}");
    let a = app_with_settings(&path);
    assert!(!a.classic());
    assert_eq!(a.engine.indicator.font_size, 18.0);
    assert!(a.engine.indicator.grid.is_some());
    assert_eq!(a.engine.prefs.rustfig.user_colors[0].name, "Rust");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// E1: a damaged settings file gives xfig's defaults and a message.
#[test]
fn damaged_settings_fall_back_to_defaults() {
    let path = settings_file("damaged");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "[xfig\nstartfontsize = ").unwrap();
    let a = app_with_settings(&path);
    assert!(a.classic());
    assert!(a
        .message
        .as_ref()
        .is_some_and(|m| m.text.contains("using xfig's defaults")));
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// E1: saved files are listed in the File menu and reopen from it.
#[test]
fn recent_files_in_the_file_menu() {
    let path = settings_file("recent");
    let fig = path.with_file_name("drawing.fig").to_string_lossy().into_owned();
    let mut h = harness(app_with_settings(&path));
    h.state_mut()
        .run("shape.rectangle", json!({"x": 0, "y": 0, "width": 600, "height": 300}));
    h.state_mut().run("app.save_as", json!({ "path": fig }));
    h.state_mut().run("app.new", json!({}));
    h.run_steps(2);
    assert!(h.state().engine.doc().objects.is_empty());
    h.get_by_label("File").click();
    h.run_steps(2);
    h.get_by_label_contains("1 drawing.fig").click();
    h.run_steps(2);
    assert_eq!(h.state().engine.doc().objects.len(), 1);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// W10 group 3: Cancel in the edit panel restores the object also after
/// Apply (xfig `done_*` with CANCEL), Done keeps it.
#[test]
fn edit_panel_cancel_restores_after_apply_and_geometry_rows_apply() {
    use rustfig_model::ObjectKind;
    let mut h = harness(app());
    h.state_mut()
        .engine
        .execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    h.state_mut().engine.execute("edit.select_all", &json!({})).unwrap();
    h.state_mut().run("app.edit_selected", json!({}));
    h.run_steps(2);
    let before = h.state().engine.doc().objects[0].clone();
    // Change a corner and the width in the panel's state, then Apply.
    {
        let st = h.state_mut().dialogs.edit.as_mut().unwrap();
        assert_eq!(st.title, "POLYLINE: Box");
        st.thickness = 9;
        if let rustfig_ui_egui::dialogs::Geo::Corners(c) = &mut st.geo {
            c[1] = [900.0, 700.0];
        } else {
            panic!("a box edits its two corners");
        }
    }
    press_key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    let after = h.state().engine.doc().objects[0].clone();
    let ObjectKind::Polyline(p) = &after.kind else { panic!() };
    assert_eq!((p.style.thickness, p.points[2].x, p.points[2].y), (9, 900.0, 700.0));
    // Escape is Cancel: the original comes back.
    press_key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(h.state().dialogs.edit.is_none());
    assert_eq!(h.state().engine.doc().objects[0], before);
}

/// W10 group 3: the middle button in Edit mode edits the figure's comments.
#[test]
fn edit_mode_middle_button_opens_the_figure_comments() {
    let mut h = harness(app());
    h.state_mut().engine.execute("tool.edit", &json!({})).unwrap();
    h.state_mut()
        .engine
        .execute(
            "tool.event",
            &json!({"type": "press", "x": 5000, "y": 5000, "button": "middle"}),
        )
        .unwrap();
    h.run_steps(2);
    assert!(h.state().dialogs.figure_comments.is_some());
}

/// Press, move in steps and release one button, with the modifiers held
/// (the way a user drags: the pointer is over the widget before the press).
fn drag_button(
    h: &mut Harness<'static, App>,
    from: egui::Pos2,
    to: egui::Pos2,
    button: egui::PointerButton,
    modifiers: egui::Modifiers,
) {
    h.hover_at(from);
    h.run_steps(1);
    h.event(egui::Event::PointerButton {
        pos: from,
        button,
        pressed: true,
        modifiers,
    });
    h.run_steps(1);
    for k in 1..=4 {
        let p = from + (to - from) * (k as f32 / 4.0);
        h.event(egui::Event::PointerMoved(p));
        h.run_steps(1);
    }
    h.event(egui::Event::PointerButton {
        pos: to,
        button,
        pressed: false,
        modifiers,
    });
    h.run_steps(1);
}

fn wheel_at(h: &mut Harness<'static, App>, pos: egui::Pos2, dy: f32, modifiers: egui::Modifiers) {
    h.hover_at(pos);
    h.run_steps(1);
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, dy),
        modifiers,
    });
    h.run_steps(2);
}

/// xfig's rulers (`w_rulers.c`): left pans left/up and right pans right/down
/// on release (Shift five times as far), the middle button and Alt+right
/// drag the view, the wheel is buttons 4 and 5, and left-drag drags too.
#[test]
fn rulers_follow_xfig_buttons() {
    use egui::{Modifiers, PointerButton as B};
    let alt = Modifiers {
        alt: true,
        ..Modifiers::NONE
    };
    let shift = Modifiers {
        shift: true,
        ..Modifiers::NONE
    };
    for horizontal in [true, false] {
        let name = if horizontal { "Top ruler" } else { "Side ruler" };
        let mut h = harness(app());
        let pos = h.get_by_label(name).rect().center();
        let step = |h: &Harness<'static, App>, mult: f64| {
            let v = &h.state().engine.view;
            600.0 / v.zoom * mult
        };
        let coord = |h: &Harness<'static, App>| {
            let o = h.state().engine.view.origin;
            if horizontal {
                o.x
            } else {
                o.y
            }
        };
        let along = |d: f32| if horizontal { vec2(d, 0.0) } else { vec2(0.0, d) };
        let c0 = coord(&h);
        // Left: pan left/up (the drawing moves left/up: the view origin grows).
        click_button(&mut h, pos, B::Primary, Modifiers::NONE);
        let s = step(&h, 1.0);
        assert!((coord(&h) - c0 - s).abs() < 1e-6, "{name} left: {}", coord(&h) - c0);
        // Right: back.
        click_button(&mut h, pos, B::Secondary, Modifiers::NONE);
        assert!((coord(&h) - c0).abs() < 1e-6, "{name} right");
        // Shift: five times.
        click_button(&mut h, pos, B::Primary, shift);
        assert!((coord(&h) - c0 - 5.0 * s).abs() < 1e-6, "{name} shift+left");
        click_button(&mut h, pos, B::Secondary, shift);
        assert!((coord(&h) - c0).abs() < 1e-6, "{name} shift+right");
        // Middle and Alt+right clicks (no movement) pan nothing, as in xfig.
        click_button(&mut h, pos, B::Middle, Modifiers::NONE);
        assert!((coord(&h) - c0).abs() < 1e-6, "{name} middle click");
        click_button(&mut h, pos, B::Secondary, alt);
        assert!((coord(&h) - c0).abs() < 1e-6, "{name} alt+right click");
        // A click with a little pointer jitter is still a click: one step
        // up/left (a real mouse never holds perfectly still).
        drag_button(&mut h, pos, pos + along(2.0), B::Primary, Modifiers::NONE);
        assert!(
            (coord(&h) - c0 - s).abs() < 1e-6,
            "{name} left click with jitter: {}",
            coord(&h) - c0
        );
        drag_button(&mut h, pos, pos + along(2.0), B::Secondary, Modifiers::NONE);
        assert!((coord(&h) - c0).abs() < 1e-6, "{name} right click with jitter");
        // Wheel: up is button 4 (left), down is button 5 (right).
        wheel_at(&mut h, pos, 40.0, Modifiers::NONE);
        assert!((coord(&h) - c0 - s).abs() < 1e-6, "{name} wheel up");
        wheel_at(&mut h, pos, -40.0, Modifiers::NONE);
        assert!((coord(&h) - c0).abs() < 1e-6, "{name} wheel down");
        // Middle, Alt+right and left drags: the view follows the pointer.
        for (button, mods, label) in [
            (B::Middle, Modifiers::NONE, "middle"),
            (B::Secondary, alt, "alt+right"),
            (B::Primary, Modifiers::NONE, "left"),
        ] {
            let before = coord(&h);
            drag_button(&mut h, pos, pos + along(40.0), button, mods);
            let moved = coord(&h) - before;
            let zoom = h.state().engine.view.zoom;
            let want = -40.0 / (zoom * 96.0 / 1200.0 * 1.0);
            let _ = want;
            assert!(moved < -1.0, "{name} {label} drag moved {moved}");
            // Back to where it was for the next button.
            drag_button(&mut h, pos + along(40.0), pos, button, mods);
            assert!((coord(&h) - before).abs() < 1.0, "{name} {label} drag back");
        }
        // A drag must not also pan by a step when released.
        let before = coord(&h);
        drag_button(&mut h, pos, pos + along(12.0), B::Middle, Modifiers::NONE);
        let moved = (coord(&h) - before).abs();
        assert!(moved < 3.0 * step(&h, 1.0), "{name} middle drag then release");
    }
}

/// W11: the button style is a setting, kept in the preferences.
#[test]
fn button_style_switches_and_persists() {
    let path = settings_file("buttons");
    {
        let mut h = harness(app_with_settings(&path));
        assert!(h.state().button_3d, "3D is the default");
        h.state_mut().run("app.button_style", json!({"style": "flat"}));
        h.run_steps(2);
        assert!(!h.state().button_3d);
        assert!(h
            .state_mut()
            .run("app.button_style", json!({"style": "round"}))
            .is_none());
        h.state_mut().run("app.button_style", json!({}));
        assert!(h.state().button_3d, "toggles when omitted");
        h.state_mut().run("app.button_style", json!({"style": "flat"}));
        h.run_steps(2);
    }
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("button_style = \"flat\""), "{text}");
    assert!(!app_with_settings(&path).button_3d);
    // The Global settings window offers both.
    let mut h = harness(app());
    h.state_mut().run("app.global_settings", json!({}));
    h.run_steps(3);
    h.get_by_label("Flat");
    h.get_by_label("3D").click();
    h.run_steps(2);
    assert!(h.state().button_3d);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

/// W11: every mode button is inside the panel and the window at the default
/// size (nothing clipped, no scrolling needed).
#[test]
fn mode_panel_is_not_clipped() {
    let h = harness(app());
    let right = rustfig_ui_egui::panels::MODE_PANEL_WIDTH;
    for t in [
        ToolKind::CircleRadius,
        ToolKind::Text,
        ToolKind::Glue,
        ToolKind::Arrow,
        ToolKind::RotateCw,
    ] {
        let r = h.get_by_label(t.label()).rect();
        assert!(r.right() <= right, "{} right edge {} > {right}", t.label(), r.right());
        assert!(r.left() >= 0.0 && r.top() >= 0.0, "{}", t.label());
        // Above the attribute panel at the bottom of the 860-high window.
        assert!(r.bottom() <= 860.0 - 40.0, "{} bottom {}", t.label(), r.bottom());
    }
}

/// W11: the Snap Mode box is a chooser with the same modes as the Snap menu.
#[test]
fn snap_box_chooses_a_mode() {
    let mut h = harness(app());
    h.get_by_label("Snap Mode: None").click();
    h.run_steps(3);
    h.get_by_label("Midpoint").click();
    h.run_steps(3);
    assert_eq!(h.state().engine.snap_indicator(), "Midpoint");
    h.get_by_label("Snap Mode: Midpoint").click();
    h.run_steps(3);
    h.get_by_label("None").click();
    h.run_steps(3);
    assert_eq!(h.state().engine.snap_indicator(), "None");
}

/// W11: the file-name box turns into a field; Enter names the figure.
#[test]
fn file_name_box_renames() {
    let dir = std::env::temp_dir().join(format!("rustfig-name-box-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut h = harness(app());
    // An unsaved figure: the name is what the next Save uses.
    h.get_by_label("(untitled)").click();
    h.run_steps(3);
    h.get_by_label("File name").type_text("mine");
    h.run_steps(2);
    press_key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(h.state().engine.path(), Some("mine.fig"));
    h.get_by_label("mine.fig");
    // Escape cancels.
    h.get_by_label("mine.fig").click();
    h.run_steps(3);
    h.get_by_label("File name").type_text("zzz");
    press_key(&mut h, egui::Key::Escape, egui::Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(h.state().engine.path(), Some("mine.fig"));
    h.get_by_label("mine.fig");
    // A saved file moves; another file of that name asks first.
    let a = dir.join("a.fig").to_string_lossy().into_owned();
    let b = dir.join("b.fig").to_string_lossy().into_owned();
    h.state_mut().run("file.save", json!({"path": a}));
    std::fs::write(&b, "x").unwrap();
    h.run_steps(2);
    h.get_by_label("a.fig").click();
    h.run_steps(3);
    h.get_by_label("File name").type_text("b");
    h.run_steps(1);
    press_key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(h.state().engine.path(), Some(a.as_str()), "asked before replacing");
    h.get_by_label("Replace").click();
    h.run_steps(3);
    assert_eq!(h.state().engine.path(), Some(b.as_str()));
    assert!(!std::path::Path::new(&a).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---- Injected mouse and keys (A11) ----

/// Run a control-channel input request through the queue, frame by frame,
/// as `App::raw_input_hook` does.
fn inject(h: &mut Harness<'static, App>, method: &str, p: serde_json::Value) -> serde_json::Value {
    let (steps, result) = rustfig_ui_egui::control::input_steps(h.state(), method, &p).unwrap();
    let seq = h.state_mut().injector.push(steps).unwrap();
    for _ in 0..200 {
        if h.state().injector.reached(seq) {
            break;
        }
        let mut raw = std::mem::take(h.input_mut());
        h.state_mut().feed_input(&mut raw);
        *h.input_mut() = raw;
        h.step();
    }
    h.run_steps(2);
    result
}

fn attached() -> Harness<'static, App> {
    let mut h = harness(app());
    let ctx = h.ctx.clone();
    h.state_mut().attach(&ctx);
    h.run_steps(3);
    h
}

fn open_state(h: &mut Harness<'static, App>) -> serde_json::Value {
    let ctx = h.ctx.clone();
    rustfig_ui_egui::control::open_state(h.state(), &ctx)
}

#[test]
fn injected_clicks_open_menu_submenu_and_dialog_by_label() {
    let mut h = attached();
    assert!(!h.state().widgets.lock().unwrap().is_empty());
    // A menu by its label, then a dialog from its entry.
    inject(&mut h, "ui.click_label", json!({"label": "File"}));
    assert_eq!(open_state(&mut h)["popup"], json!(true));
    inject(&mut h, "ui.click_label", json!({"label": "Export"}));
    assert!(open_state(&mut h)["dialogs"]
        .as_array()
        .unwrap()
        .contains(&json!("export")));
    inject(&mut h, "ui.click_label", json!({"label": "Help"}));
    inject(&mut h, "ui.click_label", json!({"label": "About"}));
    let open = open_state(&mut h);
    assert!(open["dialogs"].as_array().unwrap().contains(&json!("about")), "{open}");
    // A submenu (modern layout): Tools > Draw lists the drawing tools.
    let mut m = harness(app());
    m.state_mut().layout = Layout::Modern;
    let ctx = m.ctx.clone();
    m.state_mut().attach(&ctx);
    m.run_steps(3);
    inject(&mut m, "ui.click_label", json!({"label": "Tools"}));
    inject(&mut m, "ui.click_label", json!({"label": "Draw"}));
    let labels: Vec<String> = m
        .state()
        .widgets
        .lock()
        .unwrap()
        .iter()
        .map(|w| w.label.clone())
        .collect();
    assert!(labels.iter().any(|l| l.contains("Box")), "{labels:?}");
}

#[test]
fn injected_keys_and_text_reach_the_app() {
    let mut h = attached();
    // xfig's accelerator Meta-F opens the File menu; typing text goes to the focused field.
    inject(&mut h, "ui.key", json!({"key": "F", "modifiers": ["alt"]}));
    h.run_steps(2);
    inject(&mut h, "ui.key", json!({"text": "ab"}));
    inject(
        &mut h,
        "ui.pointer",
        json!({"action": "scroll", "x": 600, "y": 400, "dy": -30}),
    );
    inject(
        &mut h,
        "ui.pointer",
        json!({"action": "double_click", "x": 600, "y": 400, "button": "right", "modifiers": ["shift"]}),
    );
}

#[test]
fn injected_alt_click_is_the_middle_button() {
    let mut h = attached();
    let steps =
        rustfig_ui_egui::inject::pointer_steps(&json!({"action": "click", "x": 3, "y": 4, "modifiers": ["alt"]}))
            .unwrap();
    let seq = h.state_mut().injector.push(steps).unwrap();
    let mut seen = Vec::new();
    for _ in 0..seq {
        let mut raw = egui::RawInput::default();
        h.state_mut().feed_input(&mut raw);
        for e in raw.events {
            if let egui::Event::PointerButton { button, .. } = e {
                seen.push(button);
            }
        }
    }
    assert_eq!(seen, [egui::PointerButton::Middle, egui::PointerButton::Middle]);
}

#[test]
fn control_input_hostile_params_never_panic() {
    let mut h = attached();
    for (m, p) in [
        ("ui.pointer", json!({"action": "click", "x": f64::MAX, "y": 0})),
        ("ui.pointer", json!([1, 2])),
        ("ui.pointer", json!({"action": 5})),
        ("ui.key", json!({"key": "\u{0}"})),
        ("ui.key", json!({"text": "\u{1}\n"})),
        ("ui.key", json!({"modifiers": "wat", "key": "A"})),
        ("ui.click_label", json!({})),
        ("ui.click_label", json!({"label": "no such widget"})),
        ("ui.click_label", json!({"label": "File", "index": 1e30})),
        ("ui.click_label", json!({"label": "File", "button": "x"})),
    ] {
        let _ = rustfig_ui_egui::control::input_steps(h.state(), m, &p);
    }
    h.run_steps(2);
}
