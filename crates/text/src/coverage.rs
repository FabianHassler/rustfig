//! Which characters a Fig font can draw: the font's own face or the
//! fallback face that [`figbase::text::shape`] uses for missing glyphs.
//! A character neither has is drawn as the "missing glyph" box.

use figbase::text::{shape, FontRequest, PathOp};

/// A code point no font has (plane 16 private use).
const MISSING: &str = "\u{10FFFD}";

/// Whether `c` comes out as a real glyph (not the missing-glyph box) in the
/// font of `req`. Spaces and other blank characters count as present.
pub fn covers(req: &FontRequest, c: char) -> bool {
    let missing = outline(req, MISSING);
    covered_by(req, c, &missing)
}

/// [`covers`] for many characters, sharing the missing-glyph outline.
pub fn covered(req: &FontRequest, chars: impl IntoIterator<Item = char>) -> Vec<char> {
    let missing = outline(req, MISSING);
    chars.into_iter().filter(|&c| covered_by(req, c, &missing)).collect()
}

fn covered_by(req: &FontRequest, c: char, missing: &[PathOp]) -> bool {
    if c.is_control() {
        return false;
    }
    let mut buf = [0u8; 4];
    let ops = outline(req, c.encode_utf8(&mut buf));
    // A face without a missing-glyph outline draws nothing for missing
    // characters: then only characters with an outline count, plus spaces.
    if missing.is_empty() {
        return !ops.is_empty() || c.is_whitespace();
    }
    ops != missing
}

fn outline(req: &FontRequest, s: &str) -> Vec<PathOp> {
    shape(req, 100.0, s).lines.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin_letters_are_covered_private_use_is_not() {
        // Installed fonts may cover the private-use area (Apple logo): use the built-in ones.
        figbase::text::set_builtin_fonts(true);
        let req = FontRequest::for_fig(0, true);
        assert!(covers(&req, 'A'));
        assert!(covers(&req, 'é'));
        assert!(covers(&req, ' '));
        assert!(!covers(&req, '\u{F8FF}'));
        assert!(!covers(&req, '\u{E000}'));
        assert_eq!(covered(&req, ['A', '\u{E000}']), vec!['A']);
    }
}
