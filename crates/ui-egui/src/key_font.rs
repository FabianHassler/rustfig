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

/// Installed families that cover CJK, best first: Noto and Source Han, then the
/// macOS, Windows and Linux system fonts.
const FALLBACK_FAMILIES: &[&str] = &[
    "Noto Sans CJK SC",
    "Noto Sans CJK JP",
    "Noto Sans CJK TC",
    "Noto Sans CJK KR",
    "Noto Sans SC",
    "Noto Sans JP",
    "Source Han Sans",
    "Source Han Sans SC",
    "PingFang SC",
    "Hiragino Sans GB",
    "Hiragino Sans",
    "Microsoft YaHei",
    "Yu Gothic",
    "Meiryo",
    "SimSun",
    "WenQuanYi Zen Hei",
    "WenQuanYi Micro Hei",
    "Droid Sans Fallback",
    "AR PL UMing CN",
    "Arial Unicode MS",
];

/// Symbol and emoji families, tried after the CJK one.
const EMOJI_FAMILIES: &[&str] = &["Noto Sans Symbols 2", "Segoe UI Symbol", "Apple Symbols"];

/// The first of `wanted` that is in `installed` (case-insensitive), as installed.
pub fn pick_family<'a>(installed: &'a [String], wanted: &[&str]) -> Option<&'a str> {
    wanted
        .iter()
        .find_map(|w| installed.iter().find(|f| f.eq_ignore_ascii_case(w)).map(String::as_str))
}

/// Append installed CJK (and symbol) fonts as last fallbacks (native builds only;
/// no font is bundled).
pub fn add_system_fallbacks(fonts: &mut egui::FontDefinitions) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let installed: Vec<String> = rustfig_text::local_faces()
            .into_iter()
            .filter(|f| f.name == f.family)
            .map(|f| f.family)
            .collect();
        for wanted in [FALLBACK_FAMILIES, EMOJI_FAMILIES] {
            let Some(family) = pick_family(&installed, wanted) else {
                continue;
            };
            let Some((bytes, index)) = rustfig_text::local_font_bytes(family) else {
                continue;
            };
            let name = format!("System fallback {family}");
            let mut data = egui::FontData::from_owned(bytes);
            data.index = index;
            fonts.font_data.insert(name.clone(), std::sync::Arc::new(data));
            for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.entry(fam).or_default().push(name.clone());
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = fonts;
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
    add_system_fallbacks(&mut fonts);
    ctx.set_fonts(fonts);
    ctx.data_mut(|d| d.insert_temp(id, true));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_best_installed_family() {
        let installed: Vec<String> = ["DejaVu Sans", "wenquanyi zen hei", "Noto Sans CJK JP"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(pick_family(&installed, FALLBACK_FAMILIES), Some("Noto Sans CJK JP"));
        assert_eq!(
            pick_family(&installed[..2], FALLBACK_FAMILIES),
            Some("wenquanyi zen hei")
        );
        assert_eq!(pick_family(&installed[..1], FALLBACK_FAMILIES), None);
        assert_eq!(pick_family(&[], FALLBACK_FAMILIES), None);
    }

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
