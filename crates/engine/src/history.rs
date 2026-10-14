//! Snapshot-based undo/redo.
//!
//! Each step stores the document as it was, relative to the state after it:
//! the top-level objects are shared (`Arc`), and only the run of the object
//! list that differs is kept (a splice between a shared prefix and suffix).
//! An edit to one object costs one object; adding or deleting one costs
//! nothing beyond it. The history keeps at most [`MAX_UNDO`] steps and about
//! [`MAX_UNDO_BYTES`] of memory of its own.

use rustfig_model::{Document, Object, ObjectId, ObjectKind};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

/// Most undo steps kept.
pub const MAX_UNDO: usize = 200;
/// About the most memory the undo and redo steps hold of their own.
pub const MAX_UNDO_BYTES: usize = 256 << 20;

type Snap = (Document, Vec<ObjectId>);

/// A document stored relative to another one (its base).
struct Delta {
    /// The document without objects, and with `middle` as its objects.
    rest: Document,
    /// Objects shared with the base's start and end.
    prefix: usize,
    suffix: usize,
}

impl Delta {
    /// `doc` relative to `base`.
    fn new(mut doc: Document, base: &Document) -> Delta {
        let (a, b) = (&doc.objects, &base.objects);
        let prefix = a.iter().zip(b).take_while(|(x, y)| Arc::ptr_eq(x, y)).count();
        let max_suffix = a.len().min(b.len()) - prefix;
        let suffix = a
            .iter()
            .rev()
            .zip(b.iter().rev())
            .take(max_suffix)
            .take_while(|(x, y)| Arc::ptr_eq(x, y))
            .count();
        let end = doc.objects.len() - suffix;
        doc.objects.truncate(end);
        doc.objects.drain(..prefix);
        doc.objects.shrink_to_fit();
        Delta {
            rest: doc,
            prefix,
            suffix,
        }
    }

    /// The document, rebuilt on `base` (the state it was recorded against).
    fn apply(self, base: &Document) -> Document {
        let b = &base.objects;
        // A base changed outside the history (never expected) still gives
        // every object the step kept, and nothing panics.
        let prefix = self.prefix.min(b.len());
        let suffix = self.suffix.min(b.len() - prefix);
        let mut doc = self.rest;
        let middle = std::mem::take(&mut doc.objects);
        let mut objects = Vec::with_capacity(prefix + middle.len() + suffix);
        objects.extend(b.iter().take(prefix).cloned());
        objects.extend(middle);
        objects.extend(b.iter().skip(b.len() - suffix).cloned());
        doc.objects = objects;
        doc
    }

    /// Memory this step holds that its base does not share (an estimate).
    fn bytes(&self, base: &Document) -> usize {
        let doc = &self.rest;
        let lo = self.prefix.min(base.objects.len());
        let hi = base.objects.len().saturating_sub(self.suffix).max(lo);
        let region = base.objects.get(lo..hi).unwrap_or(&[]);
        let shared: HashSet<*const Object> = region.iter().map(Arc::as_ptr).collect();
        std::mem::size_of::<Delta>()
            + doc.objects.capacity() * std::mem::size_of::<Arc<Object>>()
            + doc.comments.iter().map(String::capacity).sum::<usize>()
            + doc
                .objects
                .iter()
                .filter(|o| !shared.contains(&Arc::as_ptr(o)))
                .map(|o| object_bytes(o))
                .sum::<usize>()
    }
}

struct Entry {
    delta: Delta,
    sel: Vec<ObjectId>,
    bytes: usize,
}

impl Entry {
    fn new((doc, sel): Snap, base: &Document) -> Entry {
        let delta = Delta::new(doc, base);
        let bytes = delta.bytes(base);
        Entry { delta, sel, bytes }
    }
}

#[derive(Default)]
pub(crate) struct History {
    /// Oldest first; each step is relative to the one after it, the newest
    /// to the current document.
    undo: VecDeque<Entry>,
    /// Newest undone last; each relative to the one before it, the last to
    /// the current document.
    redo: Vec<Entry>,
    bytes: usize,
}

impl History {
    /// Record `doc` (the state before a change) whose following state is `next`.
    pub fn push(&mut self, doc: Document, sel: Vec<ObjectId>, next: &Document) {
        for e in self.redo.drain(..) {
            self.bytes = self.bytes.saturating_sub(e.bytes);
        }
        let e = Entry::new((doc, sel), next);
        self.bytes = self.bytes.saturating_add(e.bytes);
        self.undo.push_back(e);
        // Keep at least the step just taken, however large.
        while self.undo.len() > 1 && (self.undo.len() > MAX_UNDO || self.bytes > MAX_UNDO_BYTES) {
            if let Some(old) = self.undo.pop_front() {
                self.bytes = self.bytes.saturating_sub(old.bytes);
            }
        }
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    /// Step back from `cur`, returning the previous state.
    pub fn undo(&mut self, cur: Snap) -> Option<Snap> {
        let prev = self.undo.pop_back()?;
        self.bytes = self.bytes.saturating_sub(prev.bytes);
        let doc = prev.delta.apply(&cur.0);
        let e = Entry::new(cur, &doc);
        self.bytes = self.bytes.saturating_add(e.bytes);
        self.redo.push(e);
        Some((doc, prev.sel))
    }
    /// Step forward from `cur`, returning the next state.
    pub fn redo(&mut self, cur: Snap) -> Option<Snap> {
        let next = self.redo.pop()?;
        self.bytes = self.bytes.saturating_sub(next.bytes);
        let doc = next.delta.apply(&cur.0);
        let e = Entry::new(cur, &doc);
        self.bytes = self.bytes.saturating_add(e.bytes);
        self.undo.push_back(e);
        Some((doc, next.sel))
    }
    /// Memory the steps hold of their own (an estimate).
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}

/// About the memory one object holds (pictures' pixels are shared, so not counted).
fn object_bytes(o: &Object) -> usize {
    let own = std::mem::size_of::<Object>() + o.comments.iter().map(|c| c.capacity() + 24).sum::<usize>();
    let pt = std::mem::size_of::<rustfig_geom::Point>();
    own + match &o.kind {
        ObjectKind::Polyline(p) => p.points.capacity() * pt + p.picture.as_ref().map_or(0, |pic| pic.file.capacity()),
        ObjectKind::Spline(s) => s.points.capacity() * pt + s.shapes.capacity() * 8,
        ObjectKind::Text(t) => t.text.capacity(),
        ObjectKind::Compound(c) => c.objects.iter().map(object_bytes).sum(),
        ObjectKind::Ellipse(_) | ObjectKind::Arc(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustfig_geom::Point;
    use rustfig_model::{PolyKind, Polyline, Style};

    fn line(x: f64) -> Arc<Object> {
        let p = Polyline::new(
            PolyKind::Polyline,
            Style::default(),
            vec![Point::new(x, 0.0), Point::new(x, 100.0)],
        );
        Arc::new(Object::new(ObjectKind::Polyline(p)))
    }

    fn doc(n: usize) -> Document {
        let mut d = Document::new();
        for i in 0..n {
            d.objects.push(line(i as f64));
        }
        d.assign_ids();
        d
    }

    fn round_trip(before: &Document, after: &Document) -> usize {
        let d = Delta::new(before.clone(), after);
        let bytes = d.bytes(after);
        assert_eq!(&d.apply(after), before);
        bytes
    }

    #[test]
    fn deltas_rebuild_the_document_and_cost_what_changed() {
        let a = doc(1000);
        // Unchanged.
        assert!(round_trip(&a, &a.clone()) < 200);
        // One object changed in place.
        let mut b = a.clone();
        if let Some(o) = b.objects.get_mut(500) {
            Arc::make_mut(o).id = ObjectId(99_999);
        }
        let one = round_trip(&a, &b);
        assert!(one > 100 && one < 1000, "{one}");
        // Added at the end, inserted at the start, deleted in the middle.
        let mut c = a.clone();
        c.objects.push(line(-1.0));
        assert!(round_trip(&a, &c) < 200);
        let mut c = a.clone();
        c.objects.insert(0, line(-1.0));
        assert!(round_trip(&a, &c) < 200);
        let mut c = a.clone();
        c.objects.remove(300);
        assert!(round_trip(&a, &c) < 1000);
        // Everything removed, everything new, reordered.
        assert!(round_trip(&a, &Document::new()) > 1000 * 50);
        assert_eq!(round_trip(&Document::new(), &a), round_trip(&Document::new(), &a));
        let mut r = a.clone();
        r.objects.reverse();
        let reordered = round_trip(&a, &r);
        assert!(reordered < 1000 * 16, "{reordered}");
    }

    #[test]
    fn undo_and_redo_walk_the_chain() {
        let mut h = History::default();
        let s0 = doc(10);
        let mut s1 = s0.clone();
        s1.objects.push(line(50.0));
        let mut s2 = s1.clone();
        if let Some(o) = s2.objects.get_mut(3) {
            Arc::make_mut(o).id = ObjectId(777);
        }
        h.push(s0.clone(), vec![ObjectId(1)], &s1);
        h.push(s1.clone(), vec![ObjectId(2)], &s2);
        let (u1, sel) = h.undo((s2.clone(), vec![])).unwrap();
        assert_eq!((&u1, sel), (&s1, vec![ObjectId(2)]));
        let (u0, _) = h.undo((u1, vec![])).unwrap();
        assert_eq!(u0, s0);
        assert!(h.undo((u0.clone(), vec![])).is_none());
        let (r1, _) = h.redo((u0, vec![])).unwrap();
        assert_eq!(r1, s1);
        let (r2, _) = h.redo((r1, vec![])).unwrap();
        assert_eq!(r2, s2);
    }

    #[test]
    fn history_is_capped_by_steps_and_memory() {
        let mut h = History::default();
        let mut cur = doc(10);
        for _ in 0..MAX_UNDO + 50 {
            let before = cur.clone();
            if let Some(o) = cur.objects.get_mut(0) {
                Arc::make_mut(o);
            }
            h.push(before, Vec::new(), &cur);
        }
        assert_eq!(h.undo.len(), MAX_UNDO);
        // Large steps push out older ones; the newest always stays.
        let big = doc(200_000);
        let one = Entry::new((big.clone(), Vec::new()), &Document::new()).bytes;
        for _ in 0..(MAX_UNDO_BYTES / one + 3) {
            h.push(big.clone(), Vec::new(), &Document::new());
        }
        assert!(h.bytes() <= MAX_UNDO_BYTES, "{}", h.bytes());
        assert!(h.undo.len() < MAX_UNDO);
        let mut h = History::default();
        for _ in 0..2 {
            h.push(doc(2_000_000), Vec::new(), &Document::new());
        }
        assert_eq!(h.undo.len(), 1);
    }
}
