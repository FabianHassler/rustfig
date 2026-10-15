//! fig2dev's `mf` driver: not ported yet.

use crate::Driver;

/// The driver for `lang`, once ported.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    let _ = lang;
    None
}
