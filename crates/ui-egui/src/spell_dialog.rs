//! Edit › Spell check (plan F5; xfig's "Misspelled words" panel,
//! `w_srchrepl.c`): the list of misspelt words, a Correction field, Correct,
//! Recheck and Dismiss. The checking is `text.spell_check`, a correction is
//! `edit.search_replace` (as in xfig); Ignore drops a word from the list.

use crate::App;
use serde_json::{json, Value};

#[derive(Debug, Clone, Default)]
pub struct SpellState {
    pub open: bool,
    pub words: Vec<String>,
    pub selected: Option<usize>,
    pub correction: String,
    pub message: String,
}

/// Run the check and fill the list (the menu entry and Recheck).
pub fn check(app: &mut App) {
    app.dialogs.spell.open = true;
    app.dialogs.spell.selected = None;
    app.dialogs.spell.correction.clear();
    match app.try_run("text.spell_check", &json!({})) {
        Ok(r) => {
            let words: Vec<String> = r
                .get("words")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|w| w.get("word").and_then(Value::as_str).map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let st = &mut app.dialogs.spell;
            st.words = words;
            st.message = r.get("message").and_then(Value::as_str).unwrap_or("").to_string();
        }
        Err(m) => {
            let st = &mut app.dialogs.spell;
            st.words.clear();
            st.message = m.clone();
            app.error(m);
        }
    }
}

/// Replace the selected word by the correction in every text.
fn correct(app: &mut App) {
    let st = &app.dialogs.spell;
    let Some(word) = st.selected.and_then(|i| st.words.get(i)).cloned() else {
        return;
    };
    let correction = st.correction.clone();
    if app
        .run(
            "edit.search_replace",
            json!({"find": word, "replace": correction, "case_sensitive": true}),
        )
        .is_some()
    {
        let st = &mut app.dialogs.spell;
        st.words.retain(|w| *w != word);
        st.selected = None;
        st.correction.clear();
        st.message = format!("\"{word}\" corrected");
    }
}

pub fn dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.spell.open {
        return;
    }
    let mut open = true;
    let (mut recheck, mut do_correct, mut ignore, mut dismiss) = (false, false, false, false);
    let mut picked: Option<usize> = None;
    let st = &mut app.dialogs.spell;
    egui::Window::new("Misspelled words")
        .open(&mut open)
        .collapsible(false)
        .default_size([380.0, 320.0])
        .show(ctx, |ui| {
            ui.label(if st.message.is_empty() {
                "Spell checker"
            } else {
                st.message.as_str()
            });
            ui.separator();
            ui.label("Misspelled words");
            egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                for (i, w) in st.words.iter().enumerate() {
                    if ui.selectable_label(st.selected == Some(i), w).clicked() {
                        picked = Some(i);
                    }
                }
            });
            ui.separator();
            let has = st.selected.is_some();
            ui.horizontal(|ui| {
                ui.add_enabled(has, egui::Label::new("Correction"));
                let r = ui.add_enabled(has, egui::TextEdit::singleline(&mut st.correction));
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && has {
                    do_correct = true;
                }
            });
            ui.horizontal(|ui| {
                if ui.add_enabled(has, egui::Button::new("Correct")).clicked() {
                    do_correct = true;
                }
                if ui
                    .add_enabled(has, egui::Button::new("Ignore"))
                    .on_hover_text("Leave the word as it is and remove it from the list")
                    .clicked()
                {
                    ignore = true;
                }
                if ui.button("Recheck").clicked() {
                    recheck = true;
                }
                if ui.button("Dismiss").clicked() {
                    dismiss = true;
                }
            });
        });
    if let Some(i) = picked {
        let st = &mut app.dialogs.spell;
        st.selected = Some(i);
        st.correction = st.words.get(i).cloned().unwrap_or_default();
    }
    if ignore {
        let st = &mut app.dialogs.spell;
        if let Some(i) = st.selected.take().filter(|i| *i < st.words.len()) {
            st.words.remove(i);
        }
        st.correction.clear();
    }
    if do_correct {
        correct(app);
    }
    if recheck {
        check(app);
    }
    // xfig's Escape and the Dismiss button close the panel.
    if dismiss || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        open = false;
    }
    app.dialogs.spell.open = open;
}
