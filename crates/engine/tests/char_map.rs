//! E5: the character map inserts into the text being typed; in a Symbol
//! text the character is stored as the font's own code and exports right.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::Engine;
use serde_json::json;

fn typing(font: i32) -> Engine {
    let mut e = Engine::new();
    e.execute("indicator.set", &json!({"font": font, "postscript": true}))
        .unwrap();
    e.execute("tool.text", &json!({})).unwrap();
    e.execute("tool.event", &json!({"type": "press", "x": 1200, "y": 1200}))
        .unwrap();
    e
}

fn data(e: &mut Engine, format: &str) -> String {
    let v = e.execute("file.export_data", &json!({"format": format})).unwrap();
    v["data"].as_str().unwrap_or_default().to_string()
}

#[test]
fn sum_in_symbol_exports_as_the_symbol_code() {
    let mut e = typing(32);
    let r = e.execute("text.insert_char", &json!({"text": "∑≤"})).unwrap();
    assert_eq!(r["inserted"], 2);
    e.execute("tool.escape", &json!({})).unwrap();
    let svg = data(&mut e, "svg");
    assert!(svg.contains("&#8721;") && svg.contains("&#8804;"), "{svg}");
    let eps = data(&mut e, "ps");
    assert!(eps.contains("/Symbol"), "{eps}");
    assert!(eps.contains("\\345"), "{eps}");
}

#[test]
fn other_fonts_keep_the_unicode_character_and_unknown_ones_fail() {
    let mut e = typing(0);
    assert_eq!(
        e.execute("text.insert_char", &json!({"text": "é"})).unwrap()["inserted"],
        1
    );
    let mut s = typing(32);
    assert!(s.execute("text.insert_char", &json!({"text": "é"})).is_err());
    let mut n = Engine::new();
    assert!(n.execute("text.insert_char", &json!({"text": "a"})).is_err());
}
