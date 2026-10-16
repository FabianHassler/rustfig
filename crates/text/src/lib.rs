//! RustFig text (layer L1): the Fig font tables, font selection, text
//! measurement and glyph outlines from [`figbase::text`] (including the
//! optional craft-fonts collection, [`CRAFT_FONTS`]), plus xfig's compose
//! sequences for typing text.

mod compose;

pub use compose::compose;
pub use figbase::text::*;
