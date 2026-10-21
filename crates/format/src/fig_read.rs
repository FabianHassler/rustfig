//! Reader for the Fig format, versions 1.3 to 3.2 (plan D3): fig2dev's
//! reader ([`fig2dev::read::readfp_fig`], with its checks, repairs and
//! messages) and the bridge's [`crate::bridge::to_document`].

use crate::bridge::to_document;
use crate::fig_write::restore_export_on_save;
use crate::FormatError;
use fig2dev::Ctx;
use rustfig_model::Document;

/// Deepest compound nesting accepted (fig2dev's reader's limit).
pub const MAX_NESTING: usize = fig2dev::read::MAX_NESTING;
/// Largest text string RustFig's commands accept, in bytes.
pub const MAX_TEXT_BYTES: usize = 1 << 20;

/// The encoding of a Fig file's text (fig2dev's `-E`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextEncoding {
    /// UTF-8 if the file says `#encoding: UTF-8` (or its bytes are UTF-8),
    /// ISO-8859-1 otherwise, as the Fig 3.2 format prescribes.
    #[default]
    Auto,
    Utf8,
    Latin1,
    Latin2,
}

impl TextEncoding {
    /// fig2dev's names for `-E`: `0`, `utf8`, `UTF8`, `utf-8` (and any
    /// case of `UTF-8`), `1` or `ISO-8859-1`, `2` or `ISO-8859-2`.
    pub fn from_name(s: &str) -> Option<TextEncoding> {
        let n = s.trim().to_ascii_lowercase().replace('_', "-");
        match n.as_str() {
            "0" | "utf8" | "utf-8" => Some(TextEncoding::Utf8),
            "1" | "iso-8859-1" | "iso8859-1" | "latin1" | "latin-1" => Some(TextEncoding::Latin1),
            "2" | "iso-8859-2" | "iso8859-2" | "latin2" | "latin-2" => Some(TextEncoding::Latin2),
            _ => None,
        }
    }

    /// The name fig2dev's [`Ctx::input_encoding`] takes.
    fn ctx_name(self) -> Option<&'static str> {
        match self {
            TextEncoding::Auto => None,
            TextEncoding::Utf8 => Some("UTF-8"),
            TextEncoding::Latin1 => Some("ISO-8859-1"),
            TextEncoding::Latin2 => Some("ISO-8859-2"),
        }
    }
}

/// Parse a Fig file (versions 1.3 to 3.2).
pub fn read_fig(src: &[u8]) -> Result<Document, FormatError> {
    read_fig_encoded(src, TextEncoding::Auto)
}

/// Parse a Fig file whose text is in `enc` (fig2dev's `-E`).
pub fn read_fig_encoded(src: &[u8], enc: TextEncoding) -> Result<Document, FormatError> {
    read_fig_messages(src, enc).map(|(doc, _)| doc)
}

/// Parse a Fig file; the document and fig2dev's messages (what it repaired
/// or skipped, one per entry).
pub fn read_fig_messages(src: &[u8], enc: TextEncoding) -> Result<(Document, Vec<String>), FormatError> {
    if src.len() > crate::MAX_FILE_BYTES {
        return Err(FormatError::new("file is too large"));
    }
    let mut ctx = Ctx {
        input_encoding: enc.ctx_name().map(String::from),
        // Read as xfig does for an editor: X-splines stay splines, texts keep their blanks.
        editing: true,
        // xfig's default when the file has no centring line.
        center: true,
        ..Ctx::default()
    };
    match fig2dev::read::readfp_fig(&mut ctx, src) {
        Ok(objects) => {
            let mut doc = to_document(&ctx, &objects);
            restore_export_on_save(&mut doc);
            Ok((doc, std::mem::take(&mut ctx.messages)))
        }
        Err(_) if ctx.messages.is_empty() => Err(FormatError::new("not a Fig file")),
        Err(_) => Err(FormatError::new(ctx.messages.join("\n"))),
    }
}
