//! Context help (W18): F1 while the pointer is over a mode button, the
//! indicator panel (attribute buttons) or the depth panel, or with a dialog
//! open, opens the matching topic of the bundled manual; elsewhere F1 opens
//! the manual's first page. Buttons note that they are hovered while they
//! are drawn; F1 reads that note of the last frame.

use crate::help::TOPICS;
use crate::App;
use rustfig_engine::ToolKind;
use serde_json::json;

/// What the last frame's widgets told about the pointer.
#[derive(Default)]
pub struct ContextHelp {
    /// Topic file of the widget under the pointer (set while drawing).
    pub hovered: Option<&'static str>,
    /// Panels: where they are and their topic.
    pub zones: Vec<(egui::Rect, &'static str)>,
}

/// The manual topic (file name) of a mode: the page with the mode's section.
pub fn mode_topic(t: ToolKind) -> Option<&'static str> {
    let heading = format!("## {}\n", t.label());
    TOPICS.iter().find(|x| x.text.contains(&heading)).map(|x| x.file)
}

/// Topic of the library button.
pub const LIBRARY_TOPIC: &str = "libraries.md";
/// Topic of the indicator panel's attribute buttons.
pub const INDICATOR_TOPIC: &str = "attributes.md";

/// The topic of the open dialog (the first that is open), if any.
pub fn dialog_topic(app: &App) -> Option<&'static str> {
    let d = &app.dialogs;
    let table: [(bool, &'static str); 12] = [
        (d.export.open, "export.md"),
        (d.print.open, "print.md"),
        (d.search.open, "select-move.md"),
        (d.settings_open, "files.md"),
        (d.global_open, "preferences.md"),
        (d.versions_open, "files.md"),
        (d.color.open, "attributes.md"),
        (app.styles.open, "attributes.md"),
        (app.unit_panel_open, "preferences.md"),
        (app.keys_sheet.open, "keys.md"),
        (app.mouse_sheet.open, "mouse.md"),
        (app.library.open, "libraries.md"),
    ];
    table.iter().find(|(open, _)| *open).map(|(_, t)| *t)
}

/// The topic F1 opens: the hovered button's, else the panel under the
/// pointer, else the open dialog's, else none (the manual's first page).
pub fn topic_for(app: &App, pointer: Option<egui::Pos2>) -> Option<&'static str> {
    app.context_help
        .hovered
        .or_else(|| {
            let p = pointer?;
            app.context_help.zones.iter().find(|(r, _)| r.contains(p)).map(|z| z.1)
        })
        .or_else(|| dialog_topic(app))
}

/// Once per frame, before the panels are drawn: handle F1, then forget last
/// frame's notes.
pub fn frame(app: &mut App, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F1)) {
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let topic = topic_for(app, pointer);
        app.run("help.open", json!({ "topic": topic.unwrap_or("") }));
    }
    app.context_help.hovered = None;
    app.context_help.zones.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustfig_engine::Engine;

    #[test]
    fn every_mode_button_has_an_existing_topic() {
        for t in ToolKind::ALL {
            let f = mode_topic(t).unwrap_or_else(|| panic!("no topic for mode {}", t.id()));
            assert!(crate::help::find_topic(f).is_some(), "{f}");
        }
    }

    #[test]
    fn fixed_topics_exist() {
        for f in [LIBRARY_TOPIC, INDICATOR_TOPIC] {
            assert!(crate::help::find_topic(f).is_some(), "{f}");
        }
    }

    #[test]
    fn dialogs_have_topics_and_hover_wins() {
        let mut app = App::with_engine(Engine::new());
        assert_eq!(dialog_topic(&app), None);
        assert_eq!(topic_for(&app, None), None);
        app.dialogs.export.open = true;
        assert_eq!(dialog_topic(&app), Some("export.md"));
        app.dialogs.export.open = false;
        for (set, file) in [(0, "preferences.md"), (1, "keys.md"), (2, "mouse.md")] {
            app.dialogs.global_open = set == 0;
            app.keys_sheet.open = set == 1;
            app.mouse_sheet.open = set == 2;
            let f = dialog_topic(&app).unwrap();
            assert_eq!(f, file);
            assert!(crate::help::find_topic(f).is_some());
        }
        app.mouse_sheet.open = false;
        app.context_help.zones.push((
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0)),
            INDICATOR_TOPIC,
        ));
        assert_eq!(topic_for(&app, Some(egui::pos2(5.0, 5.0))), Some(INDICATOR_TOPIC));
        app.context_help.hovered = Some("chop.md");
        assert_eq!(topic_for(&app, Some(egui::pos2(5.0, 5.0))), Some("chop.md"));
    }
}
