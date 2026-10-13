//! The native `.rustfig` format: the document model as JSON.

use crate::FormatError;
use rustfig_model::Document;
use serde::{Deserialize, Serialize};

/// Current native format version.
pub const NATIVE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Envelope {
    format: String,
    version: u32,
    document: Document,
}

/// Serialise a document in the native format.
pub fn write_native(doc: &Document) -> Result<String, FormatError> {
    let env = Envelope {
        format: "rustfig".into(),
        version: NATIVE_VERSION,
        document: doc.clone(),
    };
    serde_json::to_string_pretty(&env).map_err(|e| FormatError::new(format!("cannot serialise: {e}")))
}

/// Parse a native document.
pub fn read_native(src: &[u8]) -> Result<Document, FormatError> {
    let env: Envelope = serde_json::from_slice(src).map_err(|e| FormatError::at(e.line(), e.to_string()))?;
    if env.format != "rustfig" {
        return Err(FormatError::new("not a RustFig document"));
    }
    if env.version > NATIVE_VERSION {
        return Err(FormatError::new(format!(
            "document version {} is newer than this RustFig supports ({NATIVE_VERSION})",
            env.version
        )));
    }
    let mut doc = env.document;
    doc.assign_ids();
    Ok(doc)
}
