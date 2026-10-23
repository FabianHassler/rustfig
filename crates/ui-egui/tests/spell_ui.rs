#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! F5: the Misspelled words window (headless, with a fake spell checker).
#![cfg(unix)]

use egui::vec2;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use rustfig_engine::Engine;
use rustfig_ui_egui::dialogs::ScriptedDialogs;
use rustfig_ui_egui::App;
use serde_json::json;
use std::os::unix::fs::PermissionsExt;

#[test]
fn spell_check_window_lists_and_corrects() {
    let dir = std::env::temp_dir().join(format!("rustfig-spell-ui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("spell.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\ntr -cs 'a-zA-Z' '\\n' < \"$1\" | grep '^xx' | sort -u\n",
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

    let mut a = App::with_engine(Engine::new());
    a.file_dialogs = Box::new(ScriptedDialogs::default());
    let mut h = Harness::builder()
        .with_size(vec2(1280.0, 860.0))
        .build_state(|ctx, app: &mut App| app.ui(ctx), a);
    h.run_steps(3);
    let e = &mut h.state_mut().engine;
    e.execute("shape.text", &json!({"x": 0, "y": 0, "text": "a xxone b"}))
        .unwrap();
    e.execute(
        "prefs.set",
        &json!({"resource": "spellcheckcommand", "value": format!("{} %f", script.display())}),
    )
    .unwrap();
    h.state_mut().run("app.spell_check", json!({}));
    h.run_steps(3);
    assert_eq!(h.state().dialogs.spell.words, vec!["xxone".to_string()]);
    assert!(h.query_all_by_label("Misspelled words").count() >= 1);
    h.get_by_label("xxone").click();
    h.run_steps(2);
    h.state_mut().dialogs.spell.correction = "one".into();
    h.get_by_label("Correct").click();
    h.run_steps(2);
    let texts: Vec<String> = h
        .state()
        .engine
        .doc()
        .objects
        .iter()
        .filter_map(|o| match &o.kind {
            rustfig_engine::model::ObjectKind::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec!["a one b".to_string()]);
    assert!(h.state().dialogs.spell.words.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
