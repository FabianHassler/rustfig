//! Snapshot-based undo/redo.

use rustfig_model::{Document, ObjectId};

/// Most undo steps kept.
pub const MAX_UNDO: usize = 200;

type Snap = (Document, Vec<ObjectId>);

#[derive(Default)]
pub(crate) struct History {
    undo: Vec<Snap>,
    redo: Vec<Snap>,
}

impl History {
    pub fn push(&mut self, doc: Document, sel: Vec<ObjectId>) {
        self.undo.push((doc, sel));
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn undo(&mut self, cur: Snap) -> Option<Snap> {
        let prev = self.undo.pop()?;
        self.redo.push(cur);
        Some(prev)
    }
    pub fn redo(&mut self, cur: Snap) -> Option<Snap> {
        let next = self.redo.pop()?;
        self.undo.push(cur);
        Some(next)
    }
}
