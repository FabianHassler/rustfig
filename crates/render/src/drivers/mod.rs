//! Output drivers ported from fig2dev (`old/fig2dev-3.2.9a/fig2dev/dev/`),
//! one module per driver. They share the scene, the export bounds and the
//! scale computed by [`crate::export`].

pub mod bitmaps;
pub mod latex_box;
