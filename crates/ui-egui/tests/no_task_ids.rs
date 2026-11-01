#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Users don't know the plan's task ids (E9, B4, W10-25, ...): no text the
//! app shows may contain one. Scans what is reachable as data: command
//! labels and parameter docs (engine and UI commands), export languages and
//! their options, the resources, and the manual's pages.

use rustfig_engine::fig2dev::languages::LANGUAGES;
use rustfig_engine::fig2dev::options::options_for;

/// Words that look like a task id but are not one: paper sizes, function
/// keys, the PPM magic number.
fn legit(w: &str) -> bool {
    matches!(w, "A0" | "A1" | "A2" | "A3" | "A4" | "B5" | "P6" | "P3" | "S1" | "S2")
        || (w.starts_with('F') && w[1..].parse::<u32>().is_ok_and(|n| (1..=20).contains(&n)))
}

/// The task ids in `text`: a letter of A-G, P, S, W with one or two digits
/// (and maybe a lower-case letter), or `W10-14`, as a whole word.
fn task_ids(text: &str) -> Vec<String> {
    let b: Vec<char> = text.chars().collect();
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if i > 0 && (word(b[i - 1])) || !"ABCDEFGPSW".contains(b[i]) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        let digits = j - i - 1;
        if digits == 0 || digits > 2 {
            i += 1;
            continue;
        }
        if j < b.len() && b[j].is_ascii_lowercase() && !(j + 1 < b.len() && word(b[j + 1])) {
            j += 1;
        }
        if j < b.len() && b[j] == '-' && b[j + 1..].first().is_some_and(char::is_ascii_digit) {
            j += 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
        }
        if j >= b.len() || !word(b[j]) {
            let w: String = b[i..j].iter().collect();
            if !legit(&w) {
                out.push(w);
            }
        }
        i = j.max(i + 1);
    }
    out
}

fn check(what: &str, text: &str, bad: &mut Vec<String>) {
    for id in task_ids(text) {
        bad.push(format!(
            "{what}: {id} in {:?}",
            text.chars().take(90).collect::<String>()
        ));
    }
}

#[test]
fn scanner_finds_ids_and_skips_look_alikes() {
    assert_eq!(task_ids("the window (E9) and W10-25: B4."), ["E9", "W10-25", "B4"]);
    assert_eq!(task_ids("paper A4, key F12, B5, E9a"), ["E9a"]);
    assert!(task_ids("SHA256 A100 xE9 E9xy").is_empty());
}

#[test]
fn no_task_ids_in_shown_text() {
    let mut bad = Vec::new();
    for c in rustfig_engine::cmd::all() {
        check(c.id, c.label, &mut bad);
        check(c.id, c.params, &mut bad);
    }
    for c in rustfig_ui_egui::menus::UI_COMMANDS {
        check(c.id, c.label, &mut bad);
        check(c.id, c.params, &mut bad);
    }
    for l in LANGUAGES {
        check(l.id, l.description, &mut bad);
        for o in options_for(l.id) {
            check(o.key, o.help, &mut bad);
        }
    }
    for r in rustfig_engine::prefs::RESOURCES {
        check(r.name, r.option, &mut bad);
    }
    for t in rustfig_ui_egui::help::TOPICS {
        for line in t.text.lines() {
            check(t.file, line, &mut bad);
        }
    }
    assert!(bad.is_empty(), "task ids in user-visible text:\n{}", bad.join("\n"));
}

#[test]
fn every_resource_in_the_settings_windows_is_applied() {
    use rustfig_engine::prefs::{Status, RESOURCES};
    let shown: Vec<_> = RESOURCES.iter().filter(|r| r.in_ui()).collect();
    assert!(shown.len() > 30);
    assert!(shown.iter().all(|r| r.status == Status::Applied));
    // What Global settings lists on its Advanced tab is exactly those.
    let listed: Vec<String> = rustfig_ui_egui::dialogs::global_settings_matches("")
        .into_iter()
        .filter(|(tab, _)| *tab == "Advanced")
        .map(|(_, n)| n)
        .collect();
    assert_eq!(listed.len(), shown.len());
    for n in ["axislines", "browser", "pdf_viewer", "max_image_colors", "grid_unit"] {
        assert!(!listed.iter().any(|l| l == n), "{n} is shown");
    }
}
