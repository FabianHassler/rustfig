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
    assert!(h.query_all_by_label("Edit box").count() >= 1);
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
