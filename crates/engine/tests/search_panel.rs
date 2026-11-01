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

const WORDS: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
4 0 0 50 -1 0 12 0.0000 4 150 600 1200 1200 cat\\001\n\
4 0 0 50 -1 0 12 0.0000 4 150 600 1200 2400 concatenate Cat\\001\n\
4 0 0 50 -1 0 12 0.0000 4 150 600 1200 3600 the cat_s and a cat.\\001\n\
4 0 0 50 -1 0 12 0.0000 4 150 600 1200 4800 dog\\001\n\
4 0 0 50 -1 0 12 0.0000 4 150 600 90000 90000 far cat\\001\n";

fn words() -> Engine {
    let mut e = Engine::new();
    e.execute("file.open_data", &json!({"data": WORDS})).unwrap();
    e
}

fn ids(r: &serde_json::Value) -> Vec<u64> {
    r["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect()
}

#[test]
fn whole_word_matches_only_complete_words() {
    let mut e = words();
    let any = e.execute("edit.search_replace", &json!({"find": "cat"})).unwrap();
    assert_eq!(any["matches"], 4); // cat, concatenate, cat_s... and "far cat" (case sensitive: Cat excluded in 2nd, concatenate has cat)
    let whole = e
        .execute("edit.search_replace", &json!({"find": "cat", "whole_word": true}))
        .unwrap();
    // "cat", "the cat_s and a cat." (the last word cat followed by '.'), "far cat"; not concatenate, not cat_s alone.
    assert_eq!(whole["matches"], 3);
    let ci = e
        .execute(
            "edit.search_replace",
            &json!({"find": "cat", "whole_word": true, "case_sensitive": false}),
        )
        .unwrap();
    assert_eq!(ci["matches"], 4); // "concatenate Cat" now matches through its Cat
}

#[test]
fn whole_word_replace_leaves_longer_words() {
    let mut e = words();
    let r = e
        .execute(
            "edit.search_replace",
            &json!({"find": "cat", "replace": "dog", "whole_word": true, "case_sensitive": false}),
        )
        .unwrap();
    assert_eq!(r["replaced"], 4); // cat, Cat, the last cat, far cat
    let texts: Vec<String> = e.execute("edit.search_replace", &json!({"find": ""})).unwrap()["texts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["text"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(texts[0], "dog");
    assert_eq!(texts[1], "concatenate dog");
    assert_eq!(texts[2], "the cat_s and a dog.");
    assert_eq!(texts[4], "far dog");
}

#[test]
fn next_and_previous_walk_the_matches_and_wrap() {
    let mut e = words();
    let p = |dir: &str| json!({"find": "cat", "whole_word": true, "direction": dir});
    let all = ids(&e
        .execute("edit.search_replace", &json!({"find": "cat", "whole_word": true}))
        .unwrap());
    assert_eq!(all.len(), 3);
    e.set_selection(vec![]);
    // Nothing selected: next starts at the first, previous at the last.
    let r = e.execute("edit.search_step", &p("next")).unwrap();
    assert_eq!(
        (
            r["id"].as_u64().unwrap(),
            r["index"].as_u64().unwrap(),
            r["count"].as_u64().unwrap()
        ),
        (all[0], 1, 3)
    );
    assert_eq!(e.selection().len(), 1);
    let r = e.execute("edit.search_step", &p("next")).unwrap();
    assert_eq!(r["id"].as_u64().unwrap(), all[1]);
    let r = e.execute("edit.search_step", &p("next")).unwrap();
    assert_eq!(r["id"].as_u64().unwrap(), all[2]);
    // Wraps to the first.
    let r = e.execute("edit.search_step", &p("next")).unwrap();
    assert_eq!(r["id"].as_u64().unwrap(), all[0]);
    // Previous from the first wraps to the last, then steps back.
    let r = e.execute("edit.search_step", &p("previous")).unwrap();
    assert_eq!(r["id"].as_u64().unwrap(), all[2]);
    let r = e.execute("edit.search_step", &p("previous")).unwrap();
    assert_eq!(r["id"].as_u64().unwrap(), all[1]);
    assert_eq!(r["texts"][0]["text"], "the cat_s and a cat.");
    // No match, bad direction.
    let r = e.execute("edit.search_step", &json!({"find": "zebra"})).unwrap();
    assert_eq!(r["count"], 0);
    assert!(e
        .execute("edit.search_step", &json!({"find": "cat", "direction": "up"}))
        .is_err());
    // Search results list the id of each text, for clicking.
    let r = e.execute("edit.search_replace", &json!({"find": "dog"})).unwrap();
    assert!(r["texts"][0]["id"].as_u64().is_some());
}

#[test]
fn show_object_selects_and_scrolls_it_into_view() {
    let mut e = words();
    e.view.size = (800.0, 600.0);
    let far = ids(&e.execute("edit.search_replace", &json!({"find": "far"})).unwrap())[0];
    let vis = e.view.visible();
    assert!(vis.max.x < 90000.0);
    e.set_selection(vec![]);
    e.execute("edit.show_object", &json!({"id": far})).unwrap();
    assert_eq!(e.selection().iter().map(|i| i.0).collect::<Vec<_>>(), vec![far]);
    let vis = e.view.visible();
    assert!(vis.min.x < 90000.0 && vis.max.x > 90000.0 && vis.min.y < 90000.0 && vis.max.y > 90000.0);
    // An object already in view does not move the view.
    let near = ids(&e.execute("edit.search_replace", &json!({"find": "dog"})).unwrap())[0];
    e.execute("edit.show_object", &json!({"id": far})).unwrap();
    let o = e.view.origin;
    e.execute("edit.show_object", &json!({"id": far})).unwrap();
    assert_eq!(e.view.origin, o);
    e.execute("edit.show_object", &json!({"id": near})).unwrap();
    assert!(e.execute("edit.show_object", &json!({"id": 999999})).is_err());
}
