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

/// The clipboard type of the cut buffer's Fig text (E7), which another
/// RustFig window reads back.
pub fn fig_type_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "org.rustfig.fig"
    } else if cfg!(target_os = "windows") {
        "RustFig Fig"
    } else {
        "application/x-rustfig-fig"
    }
}

/// The real clipboard, opened on first use and kept (on X11 the process
/// serves the data until something else is copied).
#[derive(Default)]
pub struct SystemPictureClipboard {
    ctx: Option<ClipboardContext>,
}

impl SystemPictureClipboard {
    fn context(&mut self) -> Result<&ClipboardContext, String> {
        if self.ctx.is_none() {
            self.ctx = Some(ClipboardContext::new().map_err(|e| e.to_string())?);
        }
        self.ctx.as_ref().ok_or_else(|| "no clipboard".to_string())
    }
}

impl PictureClipboard for SystemPictureClipboard {
    fn set_objects(&mut self, fig: &str, items: &[PictureData]) -> Result<(), String> {
        let ctx = self.context()?;
        let mut contents = vec![ClipboardContent::Other(
            fig_type_name().to_string(),
            fig.as_bytes().to_vec(),
        )];
        contents.extend(
            items
                .iter()
                .map(|i| ClipboardContent::Other(type_name(i.kind).to_string(), i.bytes.clone())),
        );
        ctx.set(contents).map_err(|e| e.to_string())
    }

    fn get_fig(&mut self) -> Option<String> {
        let ctx = self.context().ok()?;
        let bytes = ctx.get_buffer(fig_type_name()).ok()?;
        String::from_utf8(bytes).ok()
    }

    fn get_image(&mut self) -> Option<Vec<u8>> {
        let ctx = self.context().ok()?;
        // PNG only: every platform's screenshot and image copy offers it.
        ctx.get_buffer(type_name(PictureKind::Png))
            .ok()
            .filter(|b| !b.is_empty())
    }

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
