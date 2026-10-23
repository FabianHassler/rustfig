#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! F5: spell check of the texts through `spellcheckcommand` (a fake checker).
#![cfg(unix)]

use rustfig_engine::Engine;
use serde_json::json;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rustfig-spell-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A checker that prints every word starting with "xx" once per line.
fn fake(dir: &std::path::Path) -> String {
    let p = dir.join("spell.sh");
    std::fs::write(&p, "#!/bin/sh\ntr -cs 'a-zA-Z' '\\n' < \"$1\" | grep '^xx' | sort -u\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    format!("{} %f", p.display())
}

fn engine() -> Engine {
    let mut e = Engine::new();
    e.execute("shape.text", &json!({"x": 0, "y": 0, "text": "good xxone words"}))
        .unwrap();
    e.execute("shape.text", &json!({"x": 0, "y": 900, "text": "xxtwo and xxone"}))
        .unwrap();
    e
}

fn set_command(e: &mut Engine, v: &str) {
    e.execute("prefs.set", &json!({"resource": "spellcheckcommand", "value": v}))
        .unwrap();
}

#[test]
fn lists_misspelt_words_with_their_objects() {
    let dir = scratch("list");
    let mut e = engine();
    set_command(&mut e, &fake(&dir));
    let r = e.execute("text.spell_check", &json!({})).unwrap();
    let words = r["words"].as_array().unwrap();
    assert_eq!(words.len(), 2, "{r}");
    let one = words.iter().find(|w| w["word"] == "xxone").unwrap();
    assert_eq!(one["objects"].as_array().unwrap().len(), 2);
    assert_eq!(r["message"], "2 misspelled words found");
    // Correcting is the existing replace.
    e.execute("edit.search_replace", &json!({"find": "xxone", "replace": "one"}))
        .unwrap();
    let r = e.execute("text.spell_check", &json!({})).unwrap();
    assert_eq!(r["words"].as_array().unwrap().len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_misspelt_words_and_missing_checker() {
    let dir = scratch("none");
    let mut e = Engine::new();
    e.execute("shape.text", &json!({"x": 0, "y": 0, "text": "fine"}))
        .unwrap();
    set_command(&mut e, &fake(&dir));
    let r = e.execute("text.spell_check", &json!({})).unwrap();
    assert_eq!(r["message"], "No misspelled words found");
    set_command(&mut e, "rustfig-no-such-checker %f");
    let err = e.execute("text.spell_check", &json!({})).unwrap_err();
    assert!(
        err.message.contains("failed") || err.message.contains("No spell checker"),
        "{}",
        err.message
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failing_checker_is_an_error() {
    let mut e = engine();
    set_command(&mut e, "exit 3 # %f");
    let err = e.execute("text.spell_check", &json!({})).unwrap_err();
    assert!(err.message.contains("failed"), "{}", err.message);
}
