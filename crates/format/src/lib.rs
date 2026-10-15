//! RustFig file formats (layer L2): the Fig format (read 3.0–3.2, write
//! 3.2) and the native `.rustfig` JSON format.

mod fig_read;
mod fig_write;
mod native;

pub use fig_read::{read_fig, read_fig_encoded, TextEncoding, MAX_NESTING, MAX_POINTS, MAX_TEXT_BYTES};
pub use fig_write::write_fig;
pub use native::{read_native, write_native, NATIVE_VERSION};

use rustfig_model::Document;
use std::fmt;

/// Largest file accepted by [`read_any`] (bytes).
pub const MAX_FILE_BYTES: usize = 512 << 20;

/// A parse or serialisation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatError {
    /// 1-based line number, 0 if unknown.
    pub line: usize,
    pub message: String,
}

impl FormatError {
    pub fn new(message: impl Into<String>) -> Self {
        FormatError {
            line: 0,
            message: message.into(),
        }
    }

    pub fn at(line: usize, message: impl Into<String>) -> Self {
        FormatError {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "line {}: {}", self.line, self.message)
        } else {
            f.write_str(&self.message)
        }
    }
}

impl std::error::Error for FormatError {}

/// Document file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    Fig,
    Native,
}

impl FileFormat {
    /// Guess from a file name's extension.
    pub fn from_path(path: &str) -> Option<FileFormat> {
        let lower = path.to_ascii_lowercase();
        if lower.ends_with(".fig") || lower.ends_with(".xfig") {
            Some(FileFormat::Fig)
        } else if lower.ends_with(".rustfig") {
            Some(FileFormat::Native)
        } else {
            None
        }
    }

    /// Guess from file content.
    pub fn sniff(src: &[u8]) -> Option<FileFormat> {
        let head = src.get(..src.len().min(64)).unwrap_or(src);
        let trimmed: Vec<u8> = head.iter().copied().skip_while(u8::is_ascii_whitespace).collect();
        if trimmed.starts_with(b"#FIG") {
            Some(FileFormat::Fig)
        } else if trimmed.starts_with(b"{") {
            Some(FileFormat::Native)
        } else {
            None
        }
    }
}

/// Read a document in any supported format (detected from the content).
pub fn read_any(src: &[u8]) -> Result<Document, FormatError> {
    if src.len() > MAX_FILE_BYTES {
        return Err(FormatError::new("file is too large"));
    }
    match FileFormat::sniff(src) {
        Some(FileFormat::Fig) => read_fig(src),
        Some(FileFormat::Native) => read_native(src),
        None => Err(FormatError::new(
            "unrecognised file format (expected a Fig or RustFig document)",
        )),
    }
}

/// Serialise a document in the given format.
pub fn write(doc: &Document, format: FileFormat) -> Result<String, FormatError> {
    match format {
        FileFormat::Fig => Ok(write_fig(doc)),
        FileFormat::Native => write_native(doc),
    }
}
