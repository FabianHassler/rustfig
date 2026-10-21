//! xfig's Search and Replace panel as a command (W10 group 4): the list of
//! matches with positions, empty search matching every text.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::Engine;
use serde_json::json;

const FIG: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
4 0 0 50 -1 0 12 0.0000 4 150 600 1200 2400 Hello world\\001\n\
4 0 0 50 -1 0 12 0.0000 4 150 600 600 600 other\\001\n";

fn engine() -> Engine {
    let mut e = Engine::new();
    e.execute("file.open_data", &json!({"data": FIG})).unwrap();
    e
}

#[test]
fn search_lists_matches_with_positions() {
    let mut e = engine();
    let r = e
        .execute(
            "edit.search_replace",
            &json!({"find": "WORLD", "case_sensitive": false}),
        )
        .unwrap();
    assert_eq!(r["matches"], 1);
    assert_eq!(r["texts"][0]["x"].as_f64(), Some(1200.0));
    assert_eq!(r["texts"][0]["text"], "Hello world");
    let r = e
        .execute("edit.search_replace", &json!({"find": "WORLD", "case_sensitive": true}))
        .unwrap();
    assert_eq!(r["matches"], 0);
}

#[test]
fn empty_search_matches_every_text_but_cannot_replace() {
    let mut e = engine();
    let r = e.execute("edit.search_replace", &json!({"find": ""})).unwrap();
    assert_eq!(r["matches"], 2);
    assert!(e
        .execute("edit.search_replace", &json!({"find": "", "replace": "x"}))
        .is_err());
}
