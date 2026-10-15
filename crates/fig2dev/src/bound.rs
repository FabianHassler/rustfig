//! fig2dev's bounds (`bound.c`): bounding boxes of objects and compounds,
//! arrowhead shapes and the arc-arrow angles the drivers share.
//!
//! Wave 1 of the driver port fills this in.

use crate::object::Compound;
use crate::Ctx;

/// `compound_bound`: the bounds of the figure, with or without texts,
/// honouring `-D` with `-K` (`ctx.adjust_boundingbox`).
pub fn compound_bound(ctx: &Ctx, compound: &Compound, include_text: bool) -> (i32, i32, i32, i32) {
    let _ = (ctx, compound, include_text);
    (0, 0, 0, 0)
}
