//! fig2dev's reader (`read.c`, `read1_3.c`): Fig 1.3 to 3.2 into the
//! object model, with fig2dev's checks, repairs and messages.
//!
//! Wave 1 of the driver port fills this in.

use crate::object::Compound;
use crate::Ctx;

/// `readfp_fig`: read a Fig file from `src`. On failure the messages are
/// in `ctx.messages` and the error is fig2dev's status (-1 bad file, -2
/// empty file, -3 not accessible).
pub fn readfp_fig(ctx: &mut Ctx, src: &[u8]) -> Result<Compound, i32> {
    let _ = src;
    ctx.warn("fig2dev: the reader is not ported yet");
    Err(-1)
}
