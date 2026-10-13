//! Last line of defence against bugs: run a closure and turn a panic into
//! an error message instead of aborting (and losing unsaved work).
//!
//! This does not replace `Result`: on wasm a panic aborts regardless.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// Run `f`, returning `Err(message)` if it panics.
pub fn catch<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|e| {
        if let Some(s) = e.downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        }
    })
}

#[cfg(test)]
mod tests {
    #[test]
    #[allow(clippy::panic)]
    fn catches_panics() {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let r: Result<(), String> = super::catch(|| panic!("boom"));
        std::panic::set_hook(prev);
        assert_eq!(r, Err("boom".to_string()));
        assert_eq!(super::catch(|| 5), Ok(5));
    }
}
