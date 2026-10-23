//! OS calls (E4b, B5b): the only crate of the workspace that may contain `unsafe`
//! (`deny` crate-wide, `allow` on the few OS-call functions, each with a SAFETY
//! comment). Its API is safe and knows nothing of egui.
//!
//! - [`screen`]: pick a colour from the screen.
//! - [`print`]: the platform's native print dialog for a PDF.

pub mod print;
pub mod screen;
