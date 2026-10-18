//! Text bytes as fig2dev's drivers see them: `dev/textconvert.c`.
//!
//! fig2dev keeps a text as the bytes of the input file (after the `\ooo`
//! escapes) and the drivers write those bytes as they are. Only
//! `put_string` (the TeX family) and `genps.c` convert them, from the input
//! encoding (`-E`, or the file's `#encoding: UTF-8` line) to UTF-8, and
//! only when that encoding differs from the locale's. RustFig assumes a
//! UTF-8 locale, so no input encoding, or UTF-8, means "no conversion".
//!
//! - [`Text::raw`](crate::object::Text) / [`Comment::raw`](crate::object::Comment):
//!   the bytes, for every driver that writes text; use [`raw`].
//! - [`Text::cstring`](crate::object::Text) / `Comment::text`: the same
//!   decoded to Unicode by the reader, for consumers that need characters
//!   (bounds, the scene writers, the app's document); use [`unicode`].
//! - [`conv_textstring`], [`conv_textisutf8`], [`convertutf8tolatin1`]:
//!   the conversions of `textconvert.c`, byte for byte.
//
// Ported from fig2dev dev/textconvert.c (xfig licence, see LICENSE-xfig).

use std::borrow::Cow;

use crate::gendev::Ctx;
use crate::object::Text;
use crate::read::Encoding;

/// `conv_spec[conv_text]`: whether the texts need a conversion to UTF-8
/// (decided at the first non-ASCII text, as C does).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConvState {
    #[default]
    Unchecked,
    NotNeeded,
    /// From this encoding to UTF-8.
    Needed(Encoding),
    /// From another charset iconv knows, converted by `encoding_rs`.
    Iconv(&'static encoding_rs::Encoding, bool),
}

/// The `encoding_rs` encoding for an iconv charset name (case-insensitive,
/// with or without dashes): WHATWG labels plus the names iconv accepts
/// that they lack (`KOI8R`, `ISO8859-15`, `latin9`, `CP437`, ...).
/// ISO-8859-1 and -2 are not here: [`Encoding`] converts them.
pub fn iconv_encoding(name: &str) -> Option<&'static encoding_rs::Encoding> {
    let lower = name.trim().to_ascii_lowercase();
    if let Some(e) = encoding_rs::Encoding::for_label(lower.as_bytes()) {
        return Some(e);
    }
    let key: String = lower.chars().filter(|c| !matches!(c, '-' | '_' | ' ' | '.')).collect();
    let label: String = match key.as_str() {
        "cyrillic" => "iso-8859-5".into(),
        "arabic" => "iso-8859-6".into(),
        "greek" => "iso-8859-7".into(),
        "hebrew" => "iso-8859-8".into(),
        "latin3" | "l3" => "iso-8859-3".into(),
        "latin4" | "l4" => "iso-8859-4".into(),
        "latin5" | "l5" => "windows-1254".into(),
        "latin6" | "l6" => "iso-8859-10".into(),
        "latin7" | "l7" => "iso-8859-13".into(),
        "latin8" | "l8" => "iso-8859-14".into(),
        "latin9" | "l9" => "iso-8859-15".into(),
        "latin10" | "l10" => "iso-8859-16".into(),
        "cp866" | "866" => "ibm866".into(),
        "macroman" | "mac" | "macintosh" => "macintosh".into(),
        "utf16" => "utf-16le".into(),
        "sjis" | "shiftjis" => "shift_jis".into(),
        "eucjp" => "euc-jp".into(),
        "euckr" => "euc-kr".into(),
        "iso2022jp" => "iso-2022-jp".into(),
        k => {
            // iconv wants a separator after "iso8859" and "windows".
            let sep = |p: &str| {
                let rest = lower.strip_prefix(p).map(|r| r.trim_start_matches(['-', '_']));
                rest.filter(|r| lower.len() > p.len() + r.len())
            };
            if let Some(n) = sep("iso8859").or_else(|| sep("iso-8859")).or_else(|| sep("iso_8859")) {
                format!("iso-8859-{}", n.trim_start_matches(':').split(':').next().unwrap_or(""))
            } else {
                format!("windows-{}", k.strip_prefix("cp")?)
            }
        }
    };
    encoding_rs::Encoding::for_label(label.as_bytes())
}

/// The bytes of a text as C holds them.
pub fn raw(t: &Text) -> &[u8] {
    &t.raw
}

/// The text decoded to Unicode.
pub fn unicode(t: &Text) -> &str {
    &t.cstring
}

/// The bytes as a `String` with one character per byte (U+0000..=U+00FF),
/// for drivers whose code works on `str` but must write the bytes back
/// unchanged: pair with [`from_byte_view`].
pub fn byte_view(b: &[u8]) -> String {
    b.iter().map(|&b| char::from(b)).collect()
}

/// The bytes of a [`byte_view`] (characters beyond U+00FF, which a driver
/// added itself, are written as UTF-8).
pub fn from_byte_view(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for c in s.chars() {
        match u8::try_from(u32::from(c)) {
            Ok(b) => out.push(b),
            Err(_) => out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes()),
        }
    }
    out
}

/// `conv_non_ascii`: the offset of the first byte beyond ASCII.
pub fn conv_non_ascii(s: &[u8]) -> Option<usize> {
    s.iter().position(|b| !b.is_ascii())
}

/// `check_conversion` for the text conversion (output UTF-8), lazily.
fn state(ctx: &mut Ctx) -> ConvState {
    if ctx.text_conv != ConvState::Unchecked {
        return ctx.text_conv;
    }
    let st = match ctx.input_encoding.clone() {
        // The locale's charset (UTF-8) is the input's.
        None => ConvState::NotNeeded,
        Some(n) if n.eq_ignore_ascii_case("UTF-8") => ConvState::NotNeeded,
        Some(n) => match Encoding::from_name(&n) {
            Some(e) => ConvState::Needed(e),
            None => match iconv_encoding(&n) {
                Some(e) => ConvState::Iconv(e, c1_identity(&n)),
                None => {
                    ctx.warn(format!("Unable to convert from {n} to UTF-8 character set."));
                    ConvState::NotNeeded
                }
            },
        },
    };
    ctx.text_conv = st;
    st
}

/// `convert` for an `encoding_rs` charset: like iconv, it stops at the
/// first invalid sequence, keeps what was converted so far and reports
/// "Error converting string ...: Illegal byte sequence".
fn iconv_convert(ctx: &mut Ctx, e: &'static encoding_rs::Encoding, c1: bool, s: &[u8]) -> Vec<u8> {
    if c1 {
        // ISO-8859-9 and TIS-620 are the WHATWG code pages with 0x80..=0x9f
        // as C1 controls, as in the standards (single-byte: per byte).
        let mut out = String::new();
        for &b in s {
            if (0x80..=0x9f).contains(&b) {
                out.push(char::from(b));
            } else {
                let one = [b];
                let (t, bad) = e.decode_without_bom_handling(&one);
                if bad {
                    ctx.warn(format!(
                        "Error converting string {}: Illegal byte sequence",
                        byte_view(s)
                    ));
                    break;
                }
                out.push_str(&t);
            }
        }
        return out.into_bytes();
    }
    let mut dec = e.new_decoder_without_bom_handling();
    let mut out = String::with_capacity(
        dec.max_utf8_buffer_length_without_replacement(s.len())
            .unwrap_or(s.len() * 4),
    );
    let (res, _) = dec.decode_to_string_without_replacement(s, &mut out, true);
    let mut bad = matches!(res, encoding_rs::DecoderResult::Malformed(..));
    // WHATWG maps the bytes the Windows code pages leave undefined to the
    // C1 controls; iconv rejects them.
    if !bad && e.name().starts_with("windows-125") {
        if let Some(i) = s.iter().position(|&b| b >= 0x80 && decodes_to_c1(e, b)) {
            out.truncate(e.decode_without_bom_handling(s.get(..i).unwrap_or_default()).0.len());
            bad = true;
        }
    }
    if bad {
        ctx.warn(format!(
            "Error converting string {}: Illegal byte sequence",
            byte_view(s)
        ));
    }
    out.into_bytes()
}

/// Charsets whose 0x80..=0x9f are C1 controls (WHATWG's replacements add graphics).
fn c1_identity(name: &str) -> bool {
    let key: String = name
        .chars()
        .filter(|c| !matches!(c, '-' | '_' | ' ' | '.'))
        .map(|c| c.to_ascii_lowercase())
        .collect();
    matches!(key.as_str(), "iso88599" | "latin5" | "l5" | "tis620")
}

fn decodes_to_c1(e: &'static encoding_rs::Encoding, b: u8) -> bool {
    let buf = [b];
    let (t, _) = e.decode_without_bom_handling(&buf);
    t.chars().next().is_some_and(|c| ('\u{80}'..='\u{9f}').contains(&c))
}

/// `conv_textstring`: the text converted from the input encoding to UTF-8,
/// or as it is when no conversion is needed.
pub fn conv_textstring<'a>(ctx: &mut Ctx, s: &'a [u8]) -> Cow<'a, [u8]> {
    if conv_non_ascii(s).is_none() {
        return Cow::Borrowed(s);
    }
    match state(ctx) {
        ConvState::Needed(Encoding::Utf8) | ConvState::NotNeeded | ConvState::Unchecked => Cow::Borrowed(s),
        ConvState::Needed(e) => Cow::Owned(e.decode(s).into_bytes()),
        ConvState::Iconv(e, c1) => Cow::Owned(iconv_convert(ctx, e, c1, s)),
    }
}

/// `conv_textisutf8`: the offset of the first non-ASCII byte, if there is
/// one and a conversion is needed (C's inverted logic: the string is then
/// taken to be UTF-8).
pub fn conv_textisutf8(ctx: &mut Ctx, s: &[u8]) -> Option<usize> {
    let at = conv_non_ascii(s)?;
    match state(ctx) {
        ConvState::NotNeeded => None,
        _ => Some(at),
    }
}

/// `convertutf8tolatin1` on `s` (in C, from the first non-ASCII byte on,
/// in place), with its quirks: only the lead bytes `0xc0..=0xc3` (and
/// bytes `0xe0..=0xef` that fail the next test) pass the mask `0xd0`, any
/// other run of non-ASCII bytes is dropped together with the byte after the
/// run, and a lead byte needs a following non-NUL byte. Where C would read
/// beyond the terminating NUL, the string ends.
pub fn convertutf8tolatin1(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let mut c = 0;
    while c < s.len() && at(c) != 0 {
        let b = at(c);
        if b & 0x80 == 0 {
            out.push(b);
        } else if b & 0xd0 == 0xc0 && (b & 0x3f) < 4 && at(c + 1) != 0 {
            c += 1;
            out.push((b & 0x3f) << 6 | (at(c) & 0x3f));
        } else {
            while at(c) != 0 && at(c) & 0x80 != 0 {
                c += 1;
            }
            if at(c) == 0 {
                break;
            }
        }
        c += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_to_latin1_as_c() {
        assert_eq!(convertutf8tolatin1("a±b".as_bytes()), b"a\xb1b");
        assert_eq!(convertutf8tolatin1("a\u{1}Ā".as_bytes()), b"a\x01");
        // Latin-1 bytes (not UTF-8): the run is dropped with the byte after it.
        assert_eq!(convertutf8tolatin1(b"a\xe9bc"), b"ac");
        assert_eq!(convertutf8tolatin1(b"a\xe9"), b"a");
    }

    #[test]
    fn conversion_follows_the_input_encoding() {
        let mut ctx = Ctx::default();
        assert_eq!(conv_textstring(&mut ctx, b"a\xe9").as_ref(), b"a\xe9");
        assert_eq!(conv_textisutf8(&mut ctx, b"a\xe9"), None);
        let mut ctx = Ctx {
            input_encoding: Some("ISO-8859-1".into()),
            ..Ctx::default()
        };
        assert_eq!(conv_textstring(&mut ctx, b"a\xe9").as_ref(), "aé".as_bytes());
        assert_eq!(conv_textisutf8(&mut ctx, b"a\xe9"), Some(1));
        let mut ctx = Ctx {
            input_encoding: Some("KOI8-R".into()),
            ..Ctx::default()
        };
        assert_eq!(conv_textstring(&mut ctx, b"a\xe9").as_ref(), "aИ".as_bytes());
        assert!(ctx.messages.is_empty());
        let mut ctx = Ctx {
            input_encoding: Some("no-such-charset".into()),
            ..Ctx::default()
        };
        assert_eq!(conv_textstring(&mut ctx, b"a\xe9").as_ref(), b"a\xe9");
        assert_eq!(ctx.messages.len(), 1);
        // The reference iconv (macOS) takes KOI8-R but not KOI8R.
        assert!(iconv_encoding("koi8-r").is_some() && iconv_encoding("KOI8R").is_none());
        assert_eq!(iconv_encoding("latin9"), iconv_encoding("ISO-8859-15"));
        assert_eq!(iconv_encoding("CP1251"), iconv_encoding("windows-1251"));
    }
}
