//! Crash recovery (plan A6): a running RustFig keeps a copy of an unsaved
//! drawing in a recovery folder, so a crash or a killed process loses at
//! most a few seconds of work. xfig does the same in its emergency save
//! (`SAVE.fig`, `u_error.c`), but only when it notices the crash itself.
//!
//! Each running instance has an id; it writes `ID.rustfig` (the drawing,
//! RustFig's lossless format) and `ID.path` (the file it came from, if any),
//! and holds an exclusive lock on `ID.lock` while it runs. The operating
//! system releases the lock when the process ends, however it ends (even
//! `kill -9`), so a drawing whose lock can be taken belongs to an instance
//! that stopped without cleaning up: it is offered for recovery at the next
//! start, right away. Native only: the web version keeps its drawings in
//! the browser (plan C3).

use crate::EngineError;
use rustfig_format::FileFormat;
use rustfig_model::Document;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

/// The platform's folder for RustFig's recovery files.
pub fn default_dir() -> Option<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    // Scripts and agents (`rustfig --background`) keep their copies apart.
    if let Some(d) = env("RUSTFIG_RECOVERY_DIR") {
        return Some(d);
    }
    let base = if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support/RustFig"))
    } else if cfg!(windows) {
        env("LOCALAPPDATA").map(|d| d.join("RustFig"))
    } else {
        env("XDG_DATA_HOME")
            .or_else(|| env("HOME").map(|h| h.join(".local/share")))
            .map(|d| d.join("rustfig"))
    };
    base.map(|b| b.join("recovery"))
}

/// A drawing left behind by an instance that stopped.
#[derive(Debug, Clone, PartialEq)]
pub struct Orphan {
    pub id: String,
    /// The recovery copy.
    pub file: PathBuf,
    /// The file the drawing was opened from or saved to, if any.
    pub original: Option<String>,
    pub saved: Option<SystemTime>,
}

/// One instance's recovery files.
#[derive(Debug)]
pub struct Recovery {
    dir: PathBuf,
    id: String,
    /// The lock held while this instance runs (none if it couldn't be made:
    /// then other instances may offer this one's copy too early).
    lock: Option<File>,
}

fn io(e: std::io::Error, what: &Path) -> EngineError {
    EngineError::io(format!("{}: {e}", what.display()))
}

impl Recovery {
    /// Recovery files of this process in `dir` (created when needed).
    pub fn new(dir: PathBuf) -> Recovery {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        // The clock alone can give two instances of one process the same
        // id (macOS: microseconds), so a counter makes it unique.
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let id = format!("{}-{nanos:x}-{n}", std::process::id());
        let lock = std::fs::create_dir_all(&dir)
            .ok()
            .and_then(|()| File::create(dir.join(format!("{id}.lock"))).ok())
            .filter(|f| f.try_lock().is_ok());
        Recovery { dir, id, lock }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    fn file(&self, ext: &str) -> PathBuf {
        self.dir.join(format!("{}.{ext}", self.id))
    }

    /// Whether this instance holds its lock.
    pub fn locked(&self) -> bool {
        self.lock.is_some()
    }

    /// Keep a copy of `doc` (opened from `original`).
    pub fn save(&self, doc: &Document, original: Option<&str>) -> Result<(), EngineError> {
        std::fs::create_dir_all(&self.dir).map_err(|e| io(e, &self.dir))?;
        let text = rustfig_format::write(doc, FileFormat::Native).map_err(|e| EngineError::format(e.to_string()))?;
        let f = self.file("rustfig");
        crate::cmd::file::write_atomic(&f.to_string_lossy(), text.as_bytes())?;
        let p = self.file("path");
        match original {
            Some(o) => std::fs::write(&p, o.as_bytes()).map_err(|e| io(e, &p)),
            None => {
                // No original: a stale path file from an earlier drawing goes.
                let _ = std::fs::remove_file(&p); // absent is fine
                Ok(())
            }
        }
    }

    /// The drawing is saved (or discarded): no copy is needed.
    pub fn clear(&self) {
        for ext in ["rustfig", "path"] {
            let _ = std::fs::remove_file(self.file(ext)); // absent is fine
        }
    }

    /// A clean exit: remove everything of this instance.
    pub fn finish(&mut self) {
        self.clear();
        if let Some(l) = self.lock.take() {
            let _ = l.unlock(); // the file goes next; dropping unlocks too
        }
        let _ = std::fs::remove_file(self.file("lock")); // absent is fine
    }

    /// Drawings in the folder left by instances that stopped.
    pub fn orphans(&self) -> Vec<Orphan> {
        orphans(&self.dir, Some(&self.id))
    }
}

/// Whether the instance `id` in `dir` still runs: its lock is held.
fn running(dir: &Path, id: &str) -> bool {
    match File::open(dir.join(format!("{id}.lock"))) {
        Ok(f) => match f.try_lock() {
            Ok(()) => false, // nobody holds it; dropping `f` releases it
            Err(std::fs::TryLockError::WouldBlock) => true,
            Err(std::fs::TryLockError::Error(_)) => false,
        },
        Err(_) => false,
    }
}

/// Drawings in `dir` whose instance stopped (its lock is free).
pub fn orphans(dir: &Path, except: Option<&str>) -> Vec<Orphan> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<Orphan> = rd
        .flatten()
        .filter_map(|e| {
            let file = e.path();
            if file.extension().and_then(|x| x.to_str()) != Some("rustfig") {
                return None;
            }
            let id = file.file_stem()?.to_str()?.to_string();
            if except == Some(id.as_str()) {
                return None;
            }
            if running(dir, &id) {
                return None;
            }
            let original = std::fs::read_to_string(dir.join(format!("{id}.path")))
                .ok()
                .filter(|s| !s.is_empty());
            let saved = e.metadata().and_then(|m| m.modified()).ok();
            Some(Orphan {
                id,
                file,
                original,
                saved,
            })
        })
        .take(64)
        .collect();
    out.sort_by_key(|o| std::cmp::Reverse(o.saved));
    out
}

/// Remove an orphan's files.
pub fn discard(dir: &Path, id: &str) {
    for ext in ["rustfig", "path", "lock"] {
        let _ = std::fs::remove_file(dir.join(format!("{id}.{ext}"))); // absent is fine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustfig_geom::Point;
    use rustfig_model::{Object, ObjectKind, PolyKind, Polyline, Style};
    use std::sync::Arc;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rustfig-recovery-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn doc() -> Document {
        let mut d = Document::new();
        d.objects.push(Arc::new(Object::new(ObjectKind::Polyline(Polyline::new(
            PolyKind::Box,
            Style::default(),
            vec![Point::new(0.0, 0.0), Point::new(600.0, 0.0), Point::new(600.0, 300.0)],
        )))));
        d.assign_ids();
        d
    }

    #[test]
    fn a_stopped_instance_leaves_an_orphan() {
        let dir = tmp("orphan");
        let a = Recovery::new(dir.clone());
        assert!(a.locked());
        a.save(&doc(), Some("/x/fig.fig")).unwrap();
        let b = Recovery::new(dir.clone());
        // A still runs: nothing to recover.
        assert!(b.orphans().is_empty());
        // A stops without cleaning up (as a crash or kill -9: the system
        // releases its lock).
        drop(a);
        // A process another test spawns at this moment can hold a copy of
        // A's lock until it has started (fork, then exec), so wait a little.
        let mut found = b.orphans();
        for _ in 0..50 {
            if !found.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
            found = b.orphans();
        }
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].original.as_deref(), Some("/x/fig.fig"));
        let text = std::fs::read_to_string(&found[0].file).unwrap();
        let back = rustfig_format::read_any(text.as_bytes()).unwrap();
        assert_eq!(back.objects.len(), 1);
        discard(&dir, &found[0].id);
        assert!(b.orphans().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clear_and_finish_remove_the_copy() {
        let dir = tmp("clear");
        let mut a = Recovery::new(dir.clone());
        a.save(&doc(), None).unwrap();
        a.clear();
        assert!(orphans(&dir, None).is_empty());
        a.save(&doc(), None).unwrap();
        a.finish();
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_dir_is_named_after_rustfig() {
        if let Some(d) = default_dir() {
            assert!(d.to_string_lossy().to_lowercase().contains("rustfig"));
            assert!(d.ends_with("recovery"));
        }
    }
}
