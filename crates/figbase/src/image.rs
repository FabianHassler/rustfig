//! Decoded pictures.

/// Decoded raster image of an imported picture (RGBA8, row-major).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
