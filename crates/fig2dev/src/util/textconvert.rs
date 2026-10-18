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
            None => {
                ctx.warn(format!("Unable to convert from {n} to UTF-8 character set."));
                ConvState::NotNeeded
            }
        },
    };
    ctx.text_conv = st;
    st
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
        assert_eq!(conv_textstring(&mut ctx, b"a\xe9").as_ref(), b"a\xe9");
        assert_eq!(ctx.messages.len(), 1);
    }
}
