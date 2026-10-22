//! "Copy as picture" (plan E14): the figure or the selection as pictures
//! for the system clipboard. The engine makes the bytes with fig2dev's
//! drivers; the front end owns the clipboard and installs a
//! [`PictureClipboard`] that puts them there.

/// The kinds of picture put on the clipboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureKind {
    /// PDF (macOS vector type).
    Pdf,
    /// SVG (Linux vector type).
    Svg,
    /// Enhanced metafile (Windows vector type).
    Emf,
    /// PNG (every platform).
    Png,
}

impl PictureKind {
    /// fig2dev's language for this kind.
    pub fn language(self) -> &'static str {
        match self {
            PictureKind::Pdf => "pdf",
            PictureKind::Svg => "svg",
            PictureKind::Emf => "emf",
            PictureKind::Png => "png",
        }
    }

    /// The vector type and PNG of the platform this was built for: macOS PDF,
    /// Windows EMF (SVG while the EMF driver is not ported), everything else SVG.
    pub fn platform_set() -> [PictureKind; 2] {
        if cfg!(target_os = "macos") {
            [PictureKind::Pdf, PictureKind::Png]
        } else if cfg!(target_os = "windows") {
            [PictureKind::Emf, PictureKind::Png]
        } else {
            [PictureKind::Svg, PictureKind::Png]
        }
    }
}

/// One picture: its kind and file contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureData {
    pub kind: PictureKind,
    pub bytes: Vec<u8>,
}

/// Where "Copy as picture" puts the pictures. Implemented by the front end
/// (the system clipboard); tests use a fake.
pub trait PictureClipboard: Send {
    /// Replace the clipboard's content with all of `items` at once.
    fn set_pictures(&mut self, items: &[PictureData]) -> Result<(), String>;

    /// E7: the cut buffer's objects as Fig text (a custom type another
    /// RustFig window can paste) together with their pictures, replacing the
    /// clipboard's content at once. Default: only the pictures.
    fn set_objects(&mut self, _fig: &str, items: &[PictureData]) -> Result<(), String> {
        self.set_pictures(items)
    }

    /// E7: the Fig text of the custom type, when the clipboard holds one.
    fn get_fig(&mut self) -> Option<String> {
        None
    }

    /// E7: an image (PNG bytes) on the clipboard, if any.
    fn get_image(&mut self) -> Option<Vec<u8>> {
        None
    }
}
