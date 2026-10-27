//! Command search (W18, Help › Command search…, Ctrl+Shift+K / Cmd+Shift+K):
//! a small window with a text field and the commands that match what is
//! typed, by label, id or menu path, each with its key; disabled ones are
//! greyed. Arrow keys move, Enter runs the command through `app.run`, so the
//! UI's own commands work too. Ctrl+K itself is xfig's keyboard panel
//! (`app.keyboard_entry`), so the search took Shift with it.
//! The matching is a pure function ([`matches`]); the window draws it.

use crate::menus::{self, XfigItem, UI_COMMANDS, XFIG_MENUS};
use crate::platform::Os;
use crate::App;
use rustfig_engine::cmd;
use serde_json::json;

/// Most rows shown at once.
const MAX_ROWS: usize = 40;

/// One searchable command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub label: String,
    /// Menu path (`"Edit"`, `"Object/Arrange"`), empty if not in a menu.
    pub menu: String,
    /// The keys as written for the system (system's, then xfig's).
    pub keys: String,
}

/// State of the search window.
#[derive(Default)]
pub struct CommandSearch {
    pub open: bool,
    pub query: String,
    /// Index into the current match list.
    pub selected: usize,
    /// The window was just opened: focus the field.
    focus: bool,
}

impl CommandSearch {
    pub fn show(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.focus = true;
    }
}

/// The menu path of a UI command in xfig's menu layout (engine commands
/// carry theirs).
fn ui_menu_path(id: &str) -> String {
    fn walk(items: &[XfigItem], id: &str, path: &str) -> Option<String> {
        for it in items {
            match it {
                XfigItem::Cmd(_, c) | XfigItem::Own(c) if *c == id => return Some(path.to_string()),
                XfigItem::Sub(name, sub) => {
                    if let Some(p) = walk(sub, id, &format!("{path}/{name}")) {
                        return Some(p);
                    }
                }
                _ => {}
            }
        }
        None
    }
    XFIG_MENUS
        .iter()
        .find_map(|(m, items)| walk(items, id, m))
        .unwrap_or_default()
}

/// Every command: the engine's, then the UI's own.
pub fn entries(os: Os) -> Vec<Entry> {
    let mac = os == Os::Mac;
    let keys = |id: &str| {
        let (a, b) = menus::entry_key_pair(id, mac);
        match (a.is_empty(), b.is_empty()) {
            (false, false) => format!("{a}  ({b})"),
            (false, true) => a,
            (true, _) => b,
        }
    };
    let engine = cmd::all().iter().map(|c| (c.id, c.label, c.menu.to_string()));
    let ui = UI_COMMANDS.iter().map(|c| (c.id, c.label, ui_menu_path(c.id)));
    engine
        .chain(ui)
        .map(|(id, label, menu)| Entry {
            id: id.to_string(),
            label: label.to_string(),
            menu,
            keys: keys(id),
        })
        .collect()
}

/// Indices of the entries matching `query`, best first: every word of the
/// query must occur in the label, id or menu path (case ignored); labels
/// that start with the query come first, then labels that contain it, then
/// the rest, each group in table order. An empty query matches everything.
pub fn matches(entries: &[Entry], query: &str) -> Vec<usize> {
    let q = query.trim().to_lowercase();
    let words: Vec<&str> = q.split_whitespace().collect();
    let mut ranked: Vec<(u8, usize)> = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        let label = e.label.to_lowercase();
        let hay = format!("{label} {} {}", e.id.to_lowercase(), e.menu.to_lowercase());
        if !words.iter().all(|w| hay.contains(w)) {
            continue;
        }
        let rank = if q.is_empty() || label.starts_with(&q) {
            0
        } else if label.contains(&q) {
            1
        } else {
            2
        };
        ranked.push((rank, i));
    }
    ranked.sort_by_key(|(r, i)| (*r, *i));
    ranked.into_iter().map(|(_, i)| i).collect()
}

/// Show the window when it is open.
pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.command_search.open {
        return;
    }
    let mut st = std::mem::take(&mut app.command_search);
    let all = entries(Os::current(ctx));
    let mut hits = matches(&all, &st.query);
    hits.truncate(MAX_ROWS);
    let mut run: Option<String> = None;
    let mut open = true;
    ctx.input_mut(|i| {
        if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
            st.selected += 1;
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
            st.selected = st.selected.saturating_sub(1);
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
            open = false;
        }
    });
    st.selected = st.selected.min(hits.len().saturating_sub(1));
    let enter = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
    egui::Window::new("Command search")
        .open(&mut open)
        .default_width(460.0)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
        .show(ctx, |ui| {
            crate::dialog_body(ui, |ui| {
                let r = ui.add(
                    egui::TextEdit::singleline(&mut st.query)
                        .hint_text("Type a command")
                        .desired_width(f32::INFINITY),
                );
                r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Command search"));
                if st.focus {
                    r.request_focus();
                    st.focus = false;
                }
                if r.changed() {
                    st.selected = 0;
                }
                ui.separator();
                egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                    if hits.is_empty() {
                        ui.weak("No command matches.");
                    }
                    for (row, &i) in hits.iter().enumerate() {
                        let Some(e) = all.get(i) else { continue };
                        let enabled = app.enabled(&e.id);
                        let text = if e.keys.is_empty() {
                            e.label.clone()
                        } else {
                            format!("{}    {}", e.label, e.keys)
                        };
                        let mut rt = egui::RichText::new(text);
                        if !enabled {
                            rt = rt.weak();
                        }
                        let tip = if e.menu.is_empty() {
                            e.id.clone()
                        } else {
                            format!("{}  ({})", e.id, e.menu)
                        };
                        let resp = ui.selectable_label(row == st.selected, rt).on_hover_text(tip);
                        resp.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Button,
                                enabled,
                                row == st.selected,
                                e.label.clone(),
                            )
                        });
                        if row == st.selected {
                            resp.scroll_to_me(None);
                        }
                        if resp.clicked() && enabled {
                            run = Some(e.id.clone());
                        }
                    }
                });
                if enter {
                    if let Some(e) = hits.get(st.selected).and_then(|&i| all.get(i)) {
                        if app.enabled(&e.id) {
                            run = Some(e.id.clone());
                        } else {
                            app.info(format!("{} is not available now", e.label));
                        }
                    }
                }
            })
        });
    st.open = open && run.is_none();
    app.command_search = st;
    if let Some(id) = run {
        app.run(&id, json!({}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(id: &str, label: &str, menu: &str) -> Entry {
        Entry {
            id: id.into(),
            label: label.into(),
            menu: menu.into(),
            keys: String::new(),
        }
    }

    #[test]
    fn matches_label_id_and_menu_path_best_first() {
        let v = vec![
            e("edit.undo", "Undo", "Edit"),
            e("file.undo_all", "Revert", "File"),
            e("object.rotate", "Rotate", "Object/Arrange"),
            e("x.y", "Redo undone", ""),
        ];
        assert_eq!(matches(&v, "undo"), vec![0, 3, 1]);
        assert_eq!(matches(&v, "ARRANGE rot"), vec![2]);
        assert_eq!(matches(&v, "edit.u"), vec![0]);
        assert_eq!(matches(&v, "  "), vec![0, 1, 2, 3]);
        assert!(matches(&v, "zzz").is_empty());
    }

    #[test]
    fn real_commands_are_found_with_their_keys() {
        for os in [Os::Mac, Os::Windows, Os::Linux] {
            let all = entries(os);
            let hit = matches(&all, "undo");
            let first = &all[hit[0]];
            assert_eq!(first.id, "edit.undo");
            assert!(!first.keys.is_empty());
            // UI commands are searchable, by menu path too.
            let hit = matches(&all, "help command search");
            assert!(hit.iter().any(|&i| all[i].id == "help.command_search"));
            let me = all.iter().find(|x| x.id == "help.command_search").unwrap();
            assert_eq!(me.menu, "Help");
        }
    }

    #[test]
    fn the_search_key_is_free() {
        let s = menus::find_ui("help.command_search").unwrap().shortcut;
        assert_eq!(s, "Ctrl+Shift+K");
        let others = cmd::all().iter().map(|c| c.shortcut).chain(
            UI_COMMANDS
                .iter()
                .filter(|c| c.id != "help.command_search")
                .map(|c| c.shortcut),
        );
        assert!(others.into_iter().all(|o| o != s));
    }
}
