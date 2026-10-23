//! The shortcut sheet (E8, Help › Keys…): every command with its keys in the
//! system's scheme and in xfig's (Meta/Option and Ctrl mode keys), for the
//! system RustFig runs on (the browser's on the web), searchable. Both
//! schemes work; the key tables of all three systems are tested, only the
//! current one is shown. Read-only: it draws the command table
//! (`cmd::all`, `menus::UI_COMMANDS`) and the key tables of `menus`.

use crate::menus::{self, UI_COMMANDS, XFIG_MENU_KEYS};
use crate::platform::Os;
use crate::App;
use rustfig_engine::cmd;

/// State of the shortcut sheet.
pub struct KeysSheet {
    pub open: bool,
    pub filter: String,
    /// Leave out commands that have no key in either scheme.
    pub only_keys: bool,
}

impl Default for KeysSheet {
    fn default() -> Self {
        KeysSheet {
            open: false,
            filter: String::new(),
            only_keys: true,
        }
    }
}

/// One row: label, id, standard keys, xfig keys.
pub struct Row {
    pub label: String,
    pub id: String,
    pub standard: String,
    pub xfig: String,
}

fn os_name(os: Os) -> &'static str {
    match os {
        Os::Mac => "macOS",
        Os::Windows => "Windows",
        Os::Linux => "Linux",
    }
}

/// The rows for `os`, in table order (commands, then the UI commands, then
/// xfig's menu keys).
pub fn rows(os: Os) -> Vec<Row> {
    let mac = os == Os::Mac;
    let mut v: Vec<Row> = cmd::all()
        .iter()
        .map(|c| (c.id, c.label))
        .chain(UI_COMMANDS.iter().map(|c| (c.id, c.label)))
        .map(|(id, label)| Row {
            label: label.to_string(),
            id: id.to_string(),
            standard: menus::entry_key_pair(id, mac).0,
            xfig: menus::entry_key_pair(id, mac).1,
        })
        .collect();
    for (spec, menu) in XFIG_MENU_KEYS {
        v.push(Row {
            label: format!("Open the {menu} menu"),
            id: format!("menu.{}", menu.to_lowercase()),
            standard: String::new(),
            xfig: menus::key_label_for(spec, mac),
        });
    }
    v
}

/// Show the window when it is open.
pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.keys_sheet.open {
        return;
    }
    let mut open = true;
    let mut st = std::mem::take(&mut app.keys_sheet);
    let xfig_on = app.xfig_keys;
    let os = Os::current(ctx);
    egui::Window::new("Keys")
        .open(&mut open)
        .default_size([620.0, 460.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Search");
                let r = ui.text_edit_singleline(&mut st.filter);
                r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Key search"));
                ui.checkbox(&mut st.only_keys, "Only commands with keys");
            });
            ui.label(format!(
                "Keys for {}. xfig's keys and the system's keys both work.",
                os_name(os)
            ));
            ui.weak(match (xfig_on, os) {
                (false, _) => "xfig's keys are off (Edit > Global settings): only the system's keys work now.",
                (true, Os::Mac) => "xfig's keys can be turned off in Edit > Global settings.",
                (true, _) => {
                    "Where both want the same Ctrl key, xfig's meaning wins while xfig's keys are on \
                     (they can be turned off in Edit > Global settings)."
                }
            });
            ui.separator();
            let f = st.filter.trim().to_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("keys_grid")
                    .striped(true)
                    .num_columns(4)
                    .show(ui, |ui| {
                        ui.strong("Command");
                        ui.strong("Id");
                        ui.strong(format!("{} keys", os_name(os)));
                        ui.strong("xfig keys");
                        ui.end_row();
                        for r in rows(os) {
                            if st.only_keys && r.standard.is_empty() && r.xfig.is_empty() {
                                continue;
                            }
                            let hay = format!("{} {} {} {}", r.label, r.id, r.standard, r.xfig).to_lowercase();
                            if !f.is_empty() && !hay.contains(&f) {
                                continue;
                            }
                            ui.label(&r.label);
                            ui.monospace(&r.id);
                            ui.label(&r.standard);
                            ui.label(&r.xfig);
                            ui.end_row();
                        }
                    });
            });
        });
    st.open = open;
    app.keys_sheet = st;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_system_has_its_keys_and_punctuation_is_shown() {
        for os in [Os::Mac, Os::Windows, Os::Linux] {
            let r = rows(os);
            for (_, id) in menus::XFIG_KEYS {
                let row = r.iter().find(|r| r.id == *id).unwrap();
                assert!(!row.xfig.is_empty(), "{id} has no xfig key on {os:?}");
            }
            for row in &r {
                for k in [&row.standard, &row.xfig] {
                    let words = ["Ctrl+", "Alt+", "Shift+"].iter().any(|w| k.contains(w));
                    let symbols = "\u{2318}\u{2325}\u{2303}\u{21e7}".chars().any(|c| k.contains(c));
                    assert!(if os == Os::Mac { !words } else { !symbols }, "{k:?} {os:?}");
                }
            }
            let quote = r.iter().find(|r| r.id == "view.toggle_grid").unwrap();
            assert!(quote.standard.ends_with('\''), "{}", quote.standard);
        }
    }

    #[test]
    fn system_column_only_where_it_differs_from_xfig() {
        for os in [Os::Mac, Os::Windows, Os::Linux] {
            let r = rows(os);
            for row in &r {
                assert!(row.standard.is_empty() || row.standard != row.xfig, "{}", row.id);
            }
            let circle = r.iter().find(|r| r.id == "tool.circle_radius").unwrap();
            assert_eq!(circle.xfig, "C");
            assert!(circle.standard.is_empty());
        }
        let undo = rows(Os::Mac).into_iter().find(|r| r.id == "edit.undo").unwrap();
        assert!(!undo.xfig.contains("Meta") && !undo.xfig.contains("Alt"));
    }

    #[test]
    fn rows_cover_commands_and_both_schemes() {
        let r = rows(Os::Mac);
        assert!(r.len() > cmd::all().len());
        let undo = r.iter().find(|r| r.id == "edit.undo").unwrap();
        assert!(!undo.standard.is_empty());
        assert!(undo.xfig.contains('⌥'));
        assert!(undo.standard.contains('⌘'));
        assert!(r.iter().any(|r| r.id == "menu.file" && r.xfig.contains('⌥')));
        let w = rows(Os::Windows);
        assert!(w.iter().any(|r| r.id == "menu.file" && r.xfig.contains("Alt")));
    }
}
