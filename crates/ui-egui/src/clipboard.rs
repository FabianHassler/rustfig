//! The system clipboard for "Copy as picture" (plan E14): the figure as the
//! platform's vector type plus PNG, so it pastes as vectors into Keynote,
//! PowerPoint, Word and Inkscape. Native only; the engine reports "not
//! available" without one (wasm, MCP, tests).

use clipboard_rs::{Clipboard, ClipboardContent, ClipboardContext};
use rustfig_engine::picture::{PictureClipboard, PictureData, PictureKind};

/// The clipboard type name of `kind` on this platform.
pub fn type_name(kind: PictureKind) -> &'static str {
    match kind {
        PictureKind::Png => {
            if cfg!(target_os = "macos") {
                "public.png"
            } else if cfg!(target_os = "windows") {
                "PNG"
            } else {
                "image/png"
            }
        }
        PictureKind::Pdf => "com.adobe.pdf",
        PictureKind::Svg => "image/svg+xml",
        // Registered format; Office reads CF_ENHMETAFILE handles, which
        // clipboard-rs cannot set (see docs/architecture.md).
        PictureKind::Emf => "image/x-emf",
    }
}

/// The real clipboard, opened on first use and kept (on X11 the process
/// serves the data until something else is copied).
#[derive(Default)]
pub struct SystemPictureClipboard {
    ctx: Option<ClipboardContext>,
}

impl PictureClipboard for SystemPictureClipboard {
    fn set_pictures(&mut self, items: &[PictureData]) -> Result<(), String> {
        if self.ctx.is_none() {
            self.ctx = Some(ClipboardContext::new().map_err(|e| e.to_string())?);
        }
        let ctx = self.ctx.as_ref().ok_or("no clipboard")?;
        let contents = items
            .iter()
            .map(|i| ClipboardContent::Other(type_name(i.kind).to_string(), i.bytes.clone()))
            .collect();
        ctx.set(contents).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_names_are_distinct_and_set() {
        let kinds = [PictureKind::Pdf, PictureKind::Svg, PictureKind::Emf, PictureKind::Png];
        let names: std::collections::BTreeSet<_> = kinds.iter().map(|k| type_name(*k)).collect();
        assert_eq!(names.len(), kinds.len());
    }
}
