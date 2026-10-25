//! OS calls (E4b, B5b): the only crate of the workspace that may contain `unsafe`
//! (`deny` crate-wide, `allow` on the few OS-call functions, each with a SAFETY
//! comment). Its API is safe and knows nothing of egui.
//!
//! - [`screen`]: pick a colour from the screen.
//! - [`print`]: the platform's native print dialog for a PDF.
//! - [`capture`]: screen capture of a window or an area into PNG bytes (D11).
//! - [`open_docs`]: files the system asks the program to open (macOS Apple Event `odoc`).

pub mod capture;
pub mod open_docs;
pub mod print;
pub mod screen;
