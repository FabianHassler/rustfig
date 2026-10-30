//! RustFig text (layer L1): the Fig font tables, font selection, text
//! measurement and glyph outlines from [`figbase::text`] (including the
//! system-font lookup with the built-in fallback), plus xfig's compose
//! sequences for typing text.

mod compose;
pub mod coverage;
pub mod latex_approx;
mod local;
pub mod symbol_tables;

pub use compose::compose;
pub use figbase::text::*;
pub use latex_approx::latex_approx;
// Shadows figbase's `add_font_dirs`: the folders also go to the installed-font list (F7).
pub use local::{
    add_font_dirs, find_local_face, local_faces, local_font_available, local_font_bytes, local_font_covers,
    local_font_sample, measure_local, override_local_faces, shape_local, LocalFace,
};
