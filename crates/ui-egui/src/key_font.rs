//! The key-symbol font: a 2 KB subset of Noto fonts (SIL OFL 1.1, see
//! `assets/fonts/OFL.txt` and `ASSETS.md`) with ⌘ ⇧ ⌥ ⌃ ⌫ ⎋ ↩ ⇥, installed as the
//! last fallback of egui's proportional and monospace families so menus show
//! ⇧⌘Z and ⌥U in every build, whatever fonts the system has.

/// Name of the font in egui's font data.
pub const KEY_FONT_NAME: &str = "RustFig Key Symbols";

static KEY_FONT: &[u8] = include_bytes!("../../../assets/fonts/rustfig-key-symbols.ttf");

/// The key symbols the font covers.
pub const KEY_SYMBOLS: &str = "⌘⇧⌥⌃⌫⎋↩⇥";

/// Add the font as last fallback to `fonts` (once).
pub fn add_to(fonts: &mut egui::FontDefinitions) {
    if fonts.font_data.contains_key(KEY_FONT_NAME) {
        return;
    }
    fonts.font_data.insert(
        KEY_FONT_NAME.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(KEY_FONT)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts.families.entry(family).or_default().push(KEY_FONT_NAME.to_owned());
    }
}

/// Install egui's default fonts plus the key-symbol font into `ctx`; does nothing the
/// second time. (The app sets no other fonts; `ctx.fonts` can't be read before the first
/// frame, so the definitions start from the defaults.)
pub fn install(ctx: &egui::Context) {
    let id = egui::Id::new("rustfig_key_font_installed");
    if ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false) {
        return;
    }
    let mut fonts = egui::FontDefinitions::default();
    add_to(&mut fonts);
    ctx.set_fonts(fonts);
    ctx.data_mut(|d| d.insert_temp(id, true));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_font_has_every_key_symbol() {
        let face = ttf_parser::Face::parse(KEY_FONT, 0).expect("font parses");
        for c in KEY_SYMBOLS.chars() {
            assert!(face.glyph_index(c).is_some(), "no glyph for {c:?}");
        }
    }

    #[test]
    fn egui_draws_the_key_symbols_in_both_families() {
        let ctx = egui::Context::default();
        install(&ctx);
        let _ = ctx.run(egui::RawInput::default(), |_| {});
        for font in [egui::FontId::proportional(13.0), egui::FontId::monospace(13.0)] {
            for c in KEY_SYMBOLS.chars() {
                assert!(
                    ctx.fonts_mut(|f| f.has_glyph(&font, c)),
                    "no glyph for {c:?} in {font:?}"
                );
            }
        }
    }
}
