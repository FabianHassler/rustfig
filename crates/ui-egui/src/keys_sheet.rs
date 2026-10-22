//! The shortcut sheet (E8, Help › Keys…): every command with its keys in the
//! standard scheme and in xfig's (Meta/Option and Ctrl mode keys), for the
//! selected system, searchable. Read-only: it draws the command table
//! (`cmd::all`, `menus::UI_COMMANDS`) and the key tables of `menus`.

use crate::menus::{self, UI_COMMANDS, XFIG_MENU_KEYS};
use crate::platform::Os;
use crate::App;
use rustfig_engine::cmd;

/// State of the shortcut sheet.
pub struct KeysSheet {
    pub open: bool,
    pub filter: String,
    /// The system whose keys are shown (macOS writes Cmd and Option).
    pub os: Os,
    /// Leave out commands that have no key in either scheme.
    pub only_keys: bool,
}

impl Default for KeysSheet {
    fn default() -> Self {
        KeysSheet {
            open: false,
            filter: String::new(),
            os: if cfg!(target_os = "macos") { Os::Mac } else { Os::Linux },
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
            standard: menus::entry_keys(id, false, mac),
            xfig: menus::entry_keys(id, true, mac),
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
    egui::Window::new("Keys")
        .open(&mut open)
        .default_size([620.0, 460.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Search");
                let r = ui.text_edit_singleline(&mut st.filter);
                r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Key search"));
                egui::ComboBox::from_label("System")
                    .selected_text(os_name(st.os))
                    .show_ui(ui, |ui| {
                        for o in [Os::Mac, Os::Windows, Os::Linux] {
                            ui.selectable_value(&mut st.os, o, os_name(o));
                        }
                    });
                ui.checkbox(&mut st.only_keys, "Only commands with keys");
            });
            ui.weak(if xfig_on {
                "xfig's keys are on (Edit > Global settings): the xfig column is what works now."
            } else {
                "xfig's keys are off: the standard column is what works now."
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
                        ui.strong("Standard keys");
                        ui.strong("xfig keys");
                        ui.end_row();
                        for r in rows(st.os) {
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
    fn rows_cover_commands_and_both_schemes() {
        let r = rows(Os::Mac);
        assert!(r.len() > cmd::all().len());
        let undo = r.iter().find(|r| r.id == "edit.undo").unwrap();
        assert!(!undo.standard.is_empty());
        assert!(undo.xfig.contains("Option") || !undo.xfig.is_empty());
        assert!(r.iter().any(|r| r.id == "menu.file" && r.xfig.contains("Option")));
        let w = rows(Os::Windows);
        assert!(w.iter().any(|r| r.id == "menu.file" && r.xfig.contains("Alt")));
    }
}
