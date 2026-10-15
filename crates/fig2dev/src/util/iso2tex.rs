//! ISO-8859-1 and ISO-8859-2 characters as TeX sequences (`ISO1toTeX`,
//! `ISO2toTeX`). The tables cover the upper half of the code page, from
//! 0xA0 (octal 240) to 0xFF; texts reach the drivers as Unicode, so
//! [`iso1_to_tex`] and [`iso2_to_tex`] look a character up by its code
//! point.
//
// Ported from fig2dev iso2tex.c (xfig licence, see LICENSE-xfig).
//
// One fix: C's entry for 0xA9 in ISO2toTeX is "\v{S}", a vertical tab
// followed by "{S}"; here it is TeX's caron, `\v{S}`.

/// The first code covered by the tables (octal 240).
pub const ISO_TABLE_START: u32 = 0xA0;

/// ISO-8859-2's upper half (0xA0..=0xFF) in Unicode.
pub const ISO2_UNICODE: [char; 96] = [
    '\u{00a0}', '\u{0104}', '\u{02d8}', '\u{0141}', '\u{00a4}', '\u{013d}', '\u{015a}', '\u{00a7}', //
    '\u{00a8}', '\u{0160}', '\u{015e}', '\u{0164}', '\u{0179}', '\u{00ad}', '\u{017d}', '\u{017b}', //
    '\u{00b0}', '\u{0105}', '\u{02db}', '\u{0142}', '\u{00b4}', '\u{013e}', '\u{015b}', '\u{02c7}', //
    '\u{00b8}', '\u{0161}', '\u{015f}', '\u{0165}', '\u{017a}', '\u{02dd}', '\u{017e}', '\u{017c}', //
    '\u{0154}', '\u{00c1}', '\u{00c2}', '\u{0102}', '\u{00c4}', '\u{0139}', '\u{0106}', '\u{00c7}', //
    '\u{010c}', '\u{00c9}', '\u{0118}', '\u{00cb}', '\u{011a}', '\u{00cd}', '\u{00ce}', '\u{010e}', //
    '\u{0110}', '\u{0143}', '\u{0147}', '\u{00d3}', '\u{00d4}', '\u{0150}', '\u{00d6}', '\u{00d7}', //
    '\u{0158}', '\u{016e}', '\u{00da}', '\u{0170}', '\u{00dc}', '\u{00dd}', '\u{0162}', '\u{00df}', //
    '\u{0155}', '\u{00e1}', '\u{00e2}', '\u{0103}', '\u{00e4}', '\u{013a}', '\u{0107}', '\u{00e7}', //
    '\u{010d}', '\u{00e9}', '\u{0119}', '\u{00eb}', '\u{011b}', '\u{00ed}', '\u{00ee}', '\u{010f}', //
    '\u{0111}', '\u{0144}', '\u{0148}', '\u{00f3}', '\u{00f4}', '\u{0151}', '\u{00f6}', '\u{00f7}', //
    '\u{0159}', '\u{016f}', '\u{00fa}', '\u{0171}', '\u{00fc}', '\u{00fd}', '\u{0163}', '\u{02d9}', //
];

/// The TeX sequence for a character of ISO-8859-1's upper half
/// (U+00A0..=U+00FF), `None` for others.
pub fn iso1_to_tex(c: char) -> Option<&'static str> {
    let i = (c as u32).checked_sub(ISO_TABLE_START)?;
    ISO1_TO_TEX.get(usize::try_from(i).ok()?).copied()
}

/// The ISO-8859-2 code of `c`, if it is in the upper half of the code page.
pub fn unicode_to_iso2(c: char) -> Option<u8> {
    let i = ISO2_UNICODE.iter().position(|&u| u == c)?;
    u8::try_from(i + ISO_TABLE_START as usize).ok()
}

/// The TeX sequence for a character of ISO-8859-2's upper half, `None`
/// for others.
pub fn iso2_to_tex(c: char) -> Option<&'static str> {
    let i = ISO2_UNICODE.iter().position(|&u| u == c)?;
    ISO2_TO_TEX.get(i).copied()
}

pub const ISO1_TO_TEX: [&str; 96] = [
    r#"{}"#,
    r#"{!`}"#,
    r#"{}"#,
    r#"\pounds{}"#,
    r#"{}"#,
    r#"{}"#,
    r#"{}"#,
    r#"\S{}"#,
    r#"\"{}"#,
    r#"\copyright{}"#,
    r#"\b{a}"#,
    r#"\mbox{$\ll$}"#,
    r#"{--}"#,
    r#"{-}"#,
    r#"{}"#,
    r#"{}"#,
    r#"{\lower.2ex\hbox{\char\'27}}"#,
    r#"\mbox{$\pm$}"#,
    r#"\mbox{$\mathsurround 0pt{}^2$}"#,
    r#"\mbox{$\mathsurround 0pt{}^3$}"#,
    r#"\'{}"#,
    r#"\mbox{$\mu$}"#,
    r#"\P{}"#,
    r#"\mbox{$\cdot$}"#,
    r#""#,
    r#"\mbox{$\mathsurround 0pt{}^1$}"#,
    r#"\b{o}"#,
    r#"\mbox{$\gg$}"#,
    r#"\mbox{$1\over 4$}"#,
    r#"\mbox{$1\over 2$}"#,
    r#"\mbox{$3\over 4$}"#,
    r#"{?`}"#,
    r#"\`A"#,
    r#"\'A"#,
    r#"\^A"#,
    r#"\~A"#,
    r#"\"A"#,
    r#"\AA{}"#,
    r#"\AE{}"#,
    r#"\c C"#,
    r#"\`E"#,
    r#"\'E"#,
    r#"\^E"#,
    r#"\"E"#,
    r#"\`I"#,
    r#"\'I"#,
    r#"\^I"#,
    r#"\"I"#,
    r#"{\rlap{\raise.3ex\hbox{--}}D}"#,
    r#"\~N"#,
    r#"\`O"#,
    r#"\'O"#,
    r#"\^O"#,
    r#"\~O"#,
    r#"\"O"#,
    r#"\mbox{$\times$}"#,
    r#"\O{}"#,
    r#"\`U"#,
    r#"\'U"#,
    r#"\^U"#,
    r#"\"U"#,
    r#"\'Y"#,
    r#"{}"#,
    r#"\ss{}"#,
    r#"\`a"#,
    r#"\'a"#,
    r#"\^a"#,
    r#"\~a"#,
    r#"\"a"#,
    r#"\aa{}"#,
    r#"\ae{}"#,
    r#"\c c"#,
    r#"\`e"#,
    r#"\'e"#,
    r#"\^e"#,
    r#"\"e"#,
    r#"\`\i{}"#,
    r#"\'\i{}"#,
    r#"\^\i{}"#,
    r#"\"\i{}"#,
    r#"\mbox{$\partial$}"#,
    r#"\~n"#,
    r#"\`o"#,
    r#"\'o"#,
    r#"\^o"#,
    r#"\~o"#,
    r#"\"o"#,
    r#"\mbox{$\div$}"#,
    r#"\o{}"#,
    r#"\`u"#,
    r#"\'u"#,
    r#"\^u"#,
    r#"\"u"#,
    r#"\'y"#,
    r#"{}"#,
    r#"\"y"#,
];
pub const ISO2_TO_TEX: [&str; 96] = [
    r#"{}"#,
    r#"\k{A}"#,
    r#"\u{}"#,
    r#"{\L}"#,
    r#"{}"#,
    r#"\v{L}"#,
    r#"\'S"#,
    r#"\S{}"#,
    r#"\"{}"#,
    r#"\v{S}"#,
    r#"\c{S}"#,
    r#"\v{T}"#,
    r#"\'Z"#,
    r#"{-}"#,
    r#"\v{Z}"#,
    r#"\.Z"#,
    r#"{\lower.2ex\hbox{\char\'27}}"#,
    r#"\k{a}"#,
    r#"\k{}"#,
    r#"{\l}"#,
    r#"\'{}"#,
    r#"\v{l}"#,
    r#"\'s"#,
    r#"\v{}"#,
    r#"\o{}"#,
    r#"\v{s}"#,
    r#"\c{s}"#,
    r#"\v{t}"#,
    r#"\'z"#,
    r#"\H{}"#,
    r#"\v{z}"#,
    r#"\.z"#,
    r#"\'R"#,
    r#"\'A"#,
    r#"\^A"#,
    r#"\u{A}"#,
    r#"\"A"#,
    r#"\'L"#,
    r#"\'C"#,
    r#"\c{C}"#,
    r#"\v{C}"#,
    r#"\'E"#,
    r#"\k{E}"#,
    r#"\"E"#,
    r#"\v{E}"#,
    r#"\'I"#,
    r#"\^I"#,
    r#"\v{D}"#,
    r#"D"#,
    r#"\'N"#,
    r#"\v{N}"#,
    r#"\'O"#,
    r#"\^O"#,
    r#"\H{O}"#,
    r#"\"O"#,
    r#"\mbox{$\times$}"#,
    r#"\v{R}"#,
    r#"\v{U}"#,
    r#"\'U"#,
    r#"\H{U}"#,
    r#"\"U"#,
    r#"\'Y"#,
    r#"\c{T}"#,
    r#"\ss{}"#,
    r#"\'r"#,
    r#"\'a"#,
    r#"\^a"#,
    r#"\u{a}"#,
    r#"\"a"#,
    r#"\'l"#,
    r#"\'c"#,
    r#"\c{c}"#,
    r#"\v{c}"#,
    r#"\'e"#,
    r#"\k{e}"#,
    r#"\"e"#,
    r#"\v{e}"#,
    r#"\'\i{}"#,
    r#"\^\i{}"#,
    r#"\v{d}"#,
    r#"d"#,
    r#"\'n"#,
    r#"\v{n}"#,
    r#"\'o"#,
    r#"\^o"#,
    r#"\H{o}"#,
    r#"\"o"#,
    r#"\mbox{$\div$}"#,
    r#"\v{r}"#,
    r#"\v{u}"#,
    r#"\'u"#,
    r#"\H{u}"#,
    r#"\"u"#,
    r#"\'y"#,
    r#"\c{t}"#,
    r#"\.{}"#,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables() {
        assert_eq!(ISO1_TO_TEX.len(), 96);
        assert_eq!(ISO2_TO_TEX.len(), 96);
        assert_eq!(iso1_to_tex('£'), Some(r"\pounds{}"));
        assert_eq!(iso1_to_tex('ü'), Some(r#"\"u"#));
        assert_eq!(iso1_to_tex('ß'), Some(r"\ss{}"));
        assert_eq!(iso1_to_tex('ÿ'), Some(r#"\"y"#));
        assert_eq!(iso1_to_tex('a'), None);
        assert_eq!(iso1_to_tex('α'), None);
        assert_eq!(iso2_to_tex('Š'), Some(r"\v{S}"));
        assert_eq!(iso2_to_tex('ł'), Some(r"{\l}"));
        assert_eq!(iso2_to_tex('˙'), Some(r"\.{}"));
        assert_eq!(unicode_to_iso2('ř'), Some(0xF8));
        assert_eq!(iso2_to_tex('£'), None);
    }
}
