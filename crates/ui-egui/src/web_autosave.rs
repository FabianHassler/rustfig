//! Browser autosave (plan C3): the web counterpart of `autosave` (A6).
//! A few seconds after the last change the drawing (Fig 3.2 text) goes to
//! the browser's storage; the next load offers to restore it. The copy goes
//! once the drawing is saved or started anew.
//!
//! This module has no browser code: it talks to a [`Store`], which is
//! `localStorage` on the web (`BrowserStore`, wasm only) and a map in tests.
//! Times are seconds (egui's clock) because `std::time::Instant` panics on
//! wasm.

/// Storage key of the Fig text.
pub const KEY_DATA: &str = "rustfig.autosave.fig";
/// Storage key of the file name the drawing had (absent: unnamed).
pub const KEY_NAME: &str = "rustfig.autosave.name";
/// Wait this long (seconds) after the last change before writing a copy.
pub const DEBOUNCE: f64 = 3.0;
/// How soon (seconds) to look again when nothing is due.
pub const IDLE: f64 = 30.0;
/// Largest copy written, in bytes. Browsers allow about 5 MB per origin for
/// all keys (counted in UTF-16 units) and the settings live there too.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;

/// A key-value store of strings.
pub trait Store {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&mut self, key: &str, value: &str) -> Result<(), String>;
    fn remove(&mut self, key: &str);
}

/// A drawing found in storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    pub data: String,
    pub name: Option<String>,
}

/// What a copy is made of: the Fig text and the file name.
pub type Snapshot = Option<(String, Option<String>)>;

pub struct WebAutosave<S: Store> {
    store: S,
    /// Revision of the copy in storage.
    saved_rev: Option<u64>,
    /// When to write the next copy.
    due: Option<f64>,
    seen_rev: u64,
}

impl<S: Store> WebAutosave<S> {
    pub fn new(store: S) -> Self {
        WebAutosave {
            store,
            saved_rev: None,
            due: None,
            seen_rev: 0,
        }
    }

    /// The copy left by an earlier session. Anything that is not Fig text
    /// (damaged or foreign data) is dropped.
    pub fn find(&mut self) -> Option<Saved> {
        let data = self.store.get(KEY_DATA)?;
        if data.len() > MAX_BYTES || !data.starts_with("#FIG") {
            self.clear();
            return None;
        }
        let name = self.store.get(KEY_NAME).filter(|n| !n.is_empty());
        Some(Saved { data, name })
    }

    /// Remove the copy (the user discarded it, or the drawing was saved).
    pub fn clear(&mut self) {
        self.store.remove(KEY_DATA);
        self.store.remove(KEY_NAME);
        self.saved_rev = None;
        self.due = None;
    }

    /// Once per frame. `modified` and `rev` describe the drawing; `fig`
    /// produces its Fig text and file name when a copy is due. Returns how
    /// many seconds to wait before looking again.
    pub fn tick(&mut self, modified: bool, rev: u64, now: f64, fig: impl FnOnce() -> Snapshot) -> f64 {
        if !modified {
            if self.saved_rev.is_some() {
                self.clear();
            }
            self.due = None;
            return IDLE;
        }
        if rev != self.seen_rev {
            self.seen_rev = rev;
            self.due = Some(now + DEBOUNCE);
        }
        match self.due {
            Some(due) if now >= due => {
                self.due = None;
                if self.saved_rev != Some(rev) {
                    self.write(rev, fig());
                }
                IDLE
            }
            Some(due) => due - now,
            None => IDLE,
        }
    }

    /// Write a copy now if there is unsaved work (after an internal error).
    pub fn emergency(&mut self, modified: bool, rev: u64, fig: impl FnOnce() -> Snapshot) {
        if modified && self.saved_rev != Some(rev) {
            self.write(rev, fig());
        }
    }

    fn write(&mut self, rev: u64, fig: Snapshot) {
        let Some((data, name)) = fig else { return };
        if data.len() > MAX_BYTES {
            // A stale copy would restore an older drawing: better none.
            log::warn!("autosave: the drawing is too large for browser storage");
            self.clear();
            return;
        }
        if let Err(e) = self.store.set(KEY_DATA, &data) {
            log::warn!("autosave: {e}");
            return;
        }
        match name {
            Some(n) => {
                let _ = self.store.set(KEY_NAME, &n); // the name is a nicety
            }
            None => self.store.remove(KEY_NAME),
        }
        self.saved_rev = Some(rev);
    }
}

/// `localStorage` of the page.
#[cfg(target_arch = "wasm32")]
pub struct BrowserStore;

#[cfg(target_arch = "wasm32")]
impl BrowserStore {
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window().and_then(|w| w.local_storage().ok().flatten())
    }
}

#[cfg(target_arch = "wasm32")]
impl Store for BrowserStore {
    fn get(&self, key: &str) -> Option<String> {
        Self::storage()?.get_item(key).ok().flatten()
    }
    fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        Self::storage()
            .ok_or_else(|| "no browser storage".to_string())?
            .set_item(key, value)
            .map_err(|_| "the browser refused to store the copy (full or blocked)".to_string())
    }
    fn remove(&mut self, key: &str) {
        if let Some(s) = Self::storage() {
            let _ = s.remove_item(key); // nothing to do if the browser refuses
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Mem {
        map: HashMap<String, String>,
        full: bool,
    }
    impl Store for Mem {
        fn get(&self, k: &str) -> Option<String> {
            self.map.get(k).cloned()
        }
        fn set(&mut self, k: &str, v: &str) -> Result<(), String> {
            if self.full {
                return Err("full".into());
            }
            self.map.insert(k.into(), v.into());
            Ok(())
        }
        fn remove(&mut self, k: &str) {
            self.map.remove(k);
        }
    }

    fn fig(s: &str) -> impl FnOnce() -> Snapshot + '_ {
        move || Some((format!("#FIG 3.2\n{s}"), Some("a.fig".into())))
    }

    #[test]
    fn saves_after_the_pause_and_throttles() {
        let mut a = WebAutosave::new(Mem::default());
        assert!(a.tick(true, 1, 0.0, fig("1")) <= DEBOUNCE);
        assert!(a.store.get(KEY_DATA).is_none(), "not before the pause");
        // More changes push the copy back.
        a.tick(true, 2, 2.0, fig("2"));
        a.tick(true, 2, 4.0, fig("x"));
        assert!(a.store.get(KEY_DATA).is_none());
        a.tick(true, 2, 5.1, fig("2"));
        assert_eq!(a.find().map(|s| s.data), Some("#FIG 3.2\n2".to_string()));
        assert_eq!(a.store.get(KEY_NAME).as_deref(), Some("a.fig"));
        // Unchanged revision: no second write.
        a.store.remove(KEY_DATA);
        a.tick(true, 2, 60.0, fig("2"));
        assert!(a.store.get(KEY_DATA).is_none());
    }

    #[test]
    fn clears_once_saved_or_new() {
        let mut a = WebAutosave::new(Mem::default());
        a.tick(true, 1, 0.0, fig("1"));
        a.tick(true, 1, 4.0, fig("1"));
        assert!(a.store.get(KEY_DATA).is_some());
        a.tick(false, 1, 5.0, fig("1"));
        assert!(a.store.get(KEY_DATA).is_none() && a.store.get(KEY_NAME).is_none());
    }

    #[test]
    fn an_old_copy_survives_until_work_replaces_it() {
        let mut m = Mem::default();
        m.map.insert(KEY_DATA.into(), "#FIG 3.2\nold".into());
        let mut a = WebAutosave::new(m);
        a.tick(false, 0, 0.0, fig("x"));
        assert!(a.find().is_some(), "an unmodified new session keeps the offer");
        a.clear();
        assert!(a.find().is_none());
    }

    #[test]
    fn corrupt_or_oversized_data_is_ignored() {
        let mut m = Mem::default();
        m.map.insert(KEY_DATA.into(), "garbage \u{0}\u{ffff}".into());
        let mut a = WebAutosave::new(m);
        assert!(a.find().is_none());
        assert!(a.store.get(KEY_DATA).is_none(), "and removed");
        a.store
            .map
            .insert(KEY_DATA.into(), format!("#FIG {}", "x".repeat(MAX_BYTES)));
        assert!(a.find().is_none());
    }

    #[test]
    fn size_cap_and_full_storage() {
        let mut a = WebAutosave::new(Mem::default());
        a.tick(true, 1, 0.0, fig("1"));
        a.tick(true, 1, 4.0, fig("1"));
        assert!(a.store.get(KEY_DATA).is_some());
        // The drawing grows past the cap: the stale copy is removed.
        a.tick(true, 2, 5.0, fig("2"));
        a.tick(true, 2, 9.0, || Some(("#FIG ".repeat(MAX_BYTES), None)));
        assert!(a.store.get(KEY_DATA).is_none());
        // A refusing store must not panic or claim success.
        let mut b = WebAutosave::new(Mem {
            full: true,
            ..Mem::default()
        });
        b.emergency(true, 3, fig("3"));
        assert!(b.saved_rev.is_none());
    }
}
