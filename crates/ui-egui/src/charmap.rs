//! The Character Map window (E5, View › Character map…, Ctrl+Shift+M):
//! the characters of the font of the text being typed; a click inserts one
//! at the caret through the `text.insert_char` command (which stores the
//! font's own code for Symbol and ZapfDingbats). Ordinary fonts show Unicode
//! blocks; the Symbol and ZapfDingbats fonts show their own tables. Only
//! the characters the font can draw are listed (its own face or the
//! fallback face), each drawn in that font.

use crate::App;
use rustfig_text::symbol_tables::{self, FONT_DINGBATS, FONT_SYMBOL};
use serde_json::json;

/// A block of Unicode shown for ordinary fonts: name and code point range.
pub const BLOCKS: &[(&str, u32, u32)] = &[
    ("Latin-1", 0xA0, 0xFF),
    ("Latin Extended-A", 0x100, 0x17F),
    ("Greek", 0x370, 0x3FF),
    ("Punctuation", 0x2010, 0x205E),
    ("Super- and subscripts", 0x2070, 0x209F),
    ("Currency", 0x20A0, 0x20BF),
    ("Arrows", 0x2190, 0x21FF),
    ("Mathematical operators", 0x2200, 0x22FF),
    ("Box drawing", 0x2500, 0x257F),
    ("Geometric shapes", 0x25A0, 0x25FF),
];

/// State of the Character Map window.
#[derive(Default)]
pub struct CharMap {
    pub open: bool,
    /// The selected block of `BLOCKS`.
    pub block: usize,
    /// The search field: a character or a hexadecimal code point.
    pub filter: String,
    /// The character last pointed at (shown below the table).
    pub hover: Option<(char, Option<u8>)>,
    /// The characters last listed and what they were listed for (font,
    /// PostScript, block, search): shaping every character each frame is
    /// too slow.
    cache: Option<(ListKey, Vec<Entry>)>,
}

/// What the character list was made for: font, PostScript, block, search.
type ListKey = (i32, bool, usize, String);

/// A listed character and its stored code for Symbol and ZapfDingbats.
type Entry = (char, Option<u8>);

/// The characters to list: the block's or the search's, without those the
/// font draws as the missing-glyph box.
fn drawable(font: i32, ps: bool, block: usize, filter: &str) -> Vec<(char, Option<u8>)> {
    let chars = if filter.trim().is_empty() {
        table(font, block)
    } else {
        found(font, filter)
    };
    let req = rustfig_text::FontRequest::for_fig(font, ps);
    let ok = rustfig_text::coverage::covered(&req, chars.iter().map(|(c, _)| *c));
    chars.into_iter().filter(|(c, _)| ok.contains(c)).collect()
}

/// The characters of `font`'s table: (character, stored code for the
/// Symbol and Dingbats fonts).
fn table(font: i32, block: usize) -> Vec<(char, Option<u8>)> {
    if symbol_tables::table_for(font).is_some() {
        return symbol_tables::glyphs(font)
            .into_iter()
            // The private-use pieces of large brackets have no glyphs.
            .filter(|(_, c)| !('\u{F000}'..='\u{F8FF}').contains(c))
            .map(|(b, c)| (c, Some(b)))
            .collect();
    }
    let (_, lo, hi) = BLOCKS.get(block).copied().unwrap_or(BLOCKS[0]);
    let mut chars: Vec<char> = (lo..=hi).filter_map(char::from_u32).collect();
    // Superscript one to three are Latin-1 characters (U+00B9, U+00B2,
    // U+00B3), not in the superscripts block: list them after ⁰ too.
    if lo == 0x2070 {
        chars.splice(1..1, ['¹', '²', '³']);
    }
    chars.into_iter().map(|c| (c, None)).collect()
}

/// Whether `c` matches the search text: the character itself or its code
/// point in hex ("2211", "U+2211", "0x2211").
fn matches(filter: &str, c: char, code: Option<u8>) -> bool {
    let f = filter.trim();
    if f.is_empty() {
        return true;
    }
    if f.chars().count() == 1 && f.starts_with(c) {
        return true;
    }
    let hex = f
        .trim_start_matches("U+")
        .trim_start_matches("u+")
        .trim_start_matches("0x")
        .to_lowercase();
    // Leading zeros don't count ("U+00e9" is "e9").
    let hex = hex.trim_start_matches('0');
    if hex.is_empty() {
        return false;
    }
    let cp = format!("{:x}", u32::from(c));
    let stored = code.map(|b| format!("{b:x}"));
    cp.starts_with(hex) || stored.is_some_and(|s| s == hex)
}

/// The characters from all tables that match the search, for the font.
fn found(font: i32, filter: &str) -> Vec<(char, Option<u8>)> {
    if symbol_tables::table_for(font).is_some() {
        return table(font, 0)
            .into_iter()
            .filter(|(c, b)| matches(filter, *c, *b))
            .collect();
    }
    (0..BLOCKS.len())
        .flat_map(|b| table(font, b))
        .filter(|(c, b)| matches(filter, *c, *b))
        .collect()
}

fn font_name(font: i32) -> Option<&'static str> {
    match font {
        FONT_SYMBOL => Some("Symbol"),
        FONT_DINGBATS => Some("ZapfDingbats"),
        _ => None,
    }
}

/// Show the window when it is open.
pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.charmap.open {
        return;
    }
    let typing_font = app.engine.typing_font_postscript();
    let (font, ps) = typing_font.unwrap_or((app.engine.indicator.font, app.engine.indicator.postscript));
    let typing = typing_font.is_some();
    let mut open = true;
    let mut insert: Option<char> = None;
    let mut block = app.charmap.block.min(BLOCKS.len() - 1);
    let mut filter = std::mem::take(&mut app.charmap.filter);
    let mut hover = None;
    let mut cache = app.charmap.cache.take();
    egui::Window::new("Character Map")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            match font_name(font) {
                Some(n) => {
                    ui.label(format!("{n} font: the characters are stored as its own codes."));
                }
                None => {
                    egui::ComboBox::from_label("Block")
                        .selected_text(BLOCKS.get(block).map_or("", |b| b.0))
                        .show_ui(ui, |ui| {
                            for (i, (n, _, _)) in BLOCKS.iter().enumerate() {
                                ui.selectable_value(&mut block, i, *n);
                            }
                        });
                }
            }
            ui.horizontal(|ui| {
                ui.label("Search");
                let r = ui.text_edit_singleline(&mut filter);
                r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Character search"));
                ui.weak("a character or its code, e.g. 2211");
            });
            let key = (font, ps, block, filter.clone());
            let chars = match &cache {
                Some((k, chars)) if *k == key => chars.clone(),
                _ => {
                    let chars = drawable(font, ps, block, &filter);
                    cache = Some((key, chars.clone()));
                    chars
                }
            };
            egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                ui.horizontal_wrapped(|ui| {
                    ui.set_max_width(12.0 * 30.0);
                    for (c, code) in &chars {
                        let text = c.to_string();
                        let ink = ui.visuals().text_color();
                        let b = match crate::font_chooser::sample_image(ui.ctx(), font, ps, &text, 16.0) {
                            Some(img) => egui::Button::image(img.tint(ink)),
                            None => egui::Button::new(text),
                        }
                        .min_size(egui::vec2(28.0, 28.0));
                        let r = ui.add_enabled(typing, b);
                        let label = format!("Insert {c}");
                        r.widget_info(move || egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label.clone()));
                        if r.hovered() {
                            hover = Some((*c, *code));
                        }
                        if r.clicked() {
                            insert = Some(*c);
                        }
                    }
                    if chars.is_empty() {
                        ui.label("No such character.");
                    }
                });
            });
            ui.separator();
            match app.charmap.hover {
                Some((c, Some(b))) => ui.label(format!("U+{:04X}, code {b:#04X} (octal \\{b:03o})", u32::from(c))),
                Some((c, None)) => ui.label(format!("U+{:04X}", u32::from(c))),
                None => ui.label(" "),
            };
            if !typing {
                ui.weak("Choose Text mode and click in the drawing to start typing; then click a character.");
            }
        });
    app.charmap.cache = cache;
    app.charmap.open = open;
    app.charmap.block = block;
    app.charmap.filter = filter;
    if hover.is_some() {
        app.charmap.hover = hover;
    }
    if let Some(c) = insert {
        app.run("text.insert_char", json!({"text": c.to_string()}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_table_has_sum_and_search_finds_it() {
        let t = table(FONT_SYMBOL, 0);
        assert!(t.contains(&('∑', Some(0xE5))));
        assert!(found(FONT_SYMBOL, "2211").contains(&('∑', Some(0xE5))));
        assert!(found(FONT_SYMBOL, "∑").contains(&('∑', Some(0xE5))));
        assert!(found(0, "U+00e9").iter().any(|(c, _)| *c == 'é'));
        assert!(found(0, "zzzz").is_empty());
    }

    /// Characters the font can't draw (the missing-glyph box) aren't listed.
    #[test]
    fn only_drawable_characters_are_listed() {
        let all: usize = (0..BLOCKS.len()).map(|b| table(0, b).len()).sum();
        let shown: usize = (0..BLOCKS.len()).map(|b| drawable(0, true, b, "").len()).sum();
        assert!(shown > 0 && shown <= all);
        assert!(drawable(0, true, 0, "").iter().any(|(c, _)| *c == 'é'));
        assert!(drawable(0, true, 0, "E000").is_empty());
    }

    /// The superscripts block lists ¹²³ (Latin-1) with ⁰ and ⁴ to ⁹.
    #[test]
    fn superscripts_one_to_three_are_listed() {
        let b = BLOCKS.iter().position(|(n, ..)| n.starts_with("Super")).unwrap();
        let s: String = table(0, b).iter().map(|(c, _)| *c).take(6).collect();
        assert!(s.starts_with("⁰¹²³"), "{s}");
    }
}
