//! Keeps a recovery copy of an unsaved drawing (plan A6), using the engine's
//! `recovery` files: a few seconds after the last change, the drawing is
//! written to the recovery folder (the instance's lock there says it is
//! still running); the copy goes once the drawing is saved or the app quits
//! cleanly. Native only.

use rustfig_engine::recovery::Recovery;
use rustfig_engine::Engine;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Wait this long after the last change before writing a copy.
pub const DEBOUNCE: Duration = Duration::from_secs(3);

/// How soon to look again when nothing is due.
const IDLE: Duration = Duration::from_secs(30);

pub struct AutoSave {
    rec: Recovery,
    /// Revision of the copy on disk.
    saved_rev: Option<u64>,
    /// When to write the next copy (set by a change, pushed back by more).
    due: Option<Instant>,
    seen_rev: u64,
}

impl AutoSave {
    pub fn new(dir: PathBuf) -> AutoSave {
        AutoSave {
            rec: Recovery::new(dir),
            saved_rev: None,
            due: None,
            seen_rev: 0,
        }
    }

    pub fn id(&self) -> &str {
        self.rec.id()
    }

    /// Once per frame: write the copy when it is due, clear it when the
    /// drawing has been saved. Returns how soon to look again.
    pub fn tick(&mut self, engine: &Engine, now: Instant) -> Option<Duration> {
        if !engine.is_modified() {
            if self.saved_rev.take().is_some() {
                self.rec.clear();
            }
            self.due = None;
            return Some(IDLE);
        }
        let rev = engine.revision();
        if rev != self.seen_rev {
            self.seen_rev = rev;
            self.due = Some(now + DEBOUNCE);
        }
        match self.due {
            Some(due) if now >= due => {
                self.save(engine);
                Some(IDLE)
            }
            Some(due) => Some(due - now),
            None => Some(IDLE),
        }
    }

    fn save(&mut self, engine: &Engine) {
        self.due = None;
        if self.saved_rev == Some(engine.revision()) {
            return;
        }
        match self.rec.save(engine.doc(), engine.path()) {
            Ok(()) => self.saved_rev = Some(engine.revision()),
            Err(e) => log::warn!("recovery copy: {}", e.message),
        }
    }

    /// Write a copy now (after an internal error), if there is unsaved work.
    pub fn emergency(&mut self, engine: &Engine) {
        if engine.is_modified() {
            self.save(engine);
        }
    }

    /// A clean exit: no copy is needed any more.
    pub fn finish(&mut self) {
        self.rec.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn copies_after_a_pause_and_clears_after_saving() {
        let dir = std::env::temp_dir().join(format!("rustfig-autosave-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut a = AutoSave::new(dir.clone());
        let mut e = Engine::new();
        let t0 = Instant::now();
        a.tick(&e, t0);
        e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
            .unwrap();
        a.tick(&e, t0);
        let copy = dir.join(format!("{}.rustfig", a.id()));
        assert!(!copy.exists(), "not before the pause");
        a.tick(&e, t0 + DEBOUNCE + Duration::from_millis(10));
        assert!(copy.exists(), "after the pause");
        e.set_modified(false);
        a.tick(&e, t0 + DEBOUNCE * 2);
        assert!(!copy.exists(), "gone once saved");
        a.finish();
        assert_eq!(std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
