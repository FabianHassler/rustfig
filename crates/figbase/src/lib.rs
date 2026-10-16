//! The code fig2dev and RustFig share (plan B12): Fig geometry (points,
//! transforms, X-splines, arcs, arrowheads, line widths), Fig fonts (tables,
//! metrics, glyph outlines) and colours (the standard Fig colours, area-fill
//! shading, X11 colour names).
//!
//! It sits below every other crate of the workspace and depends on none of
//! them, so that `fig2dev` stays a program of its own.

pub mod color;
pub mod geom;
pub mod text;
