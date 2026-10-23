//! Editing the text being typed (plan E10): a selection (anchor and caret),
//! word jumps, replacing the selection, and the caret under the mouse.
//!
//! RustFig's modernisation of xfig's text mode (`d_text.c`): xfig's keys and
//! buttons stay; selection with the mouse and Shift+arrows, word jumps and
//! the clipboard are added. All positions are in characters (never bytes),
//! so multi-byte text is safe.

use rustfig_geom::{Affine, Point};
use rustfig_model::{Justify, Text};

/// A character index to a byte index (the end of `s` when past it).
pub(crate) fn byte_of(s: &str, chars: usize) -> usize {
    s.char_indices().nth(chars).map(|(b, _)| b).unwrap_or(s.len())
}

fn class(c: char) -> u8 {
    if c.is_whitespace() {
        0
    } else if c.is_alphanumeric() || c == '_' {
        1
    } else {
        2
    }
}

/// The caret one word to the left of `at`: skips spaces, then one word.
pub(crate) fn word_left(text: &str, at: usize) -> usize {
    let cs: Vec<char> = text.chars().collect();
    let mut i = at.min(cs.len());
    while i > 0 && cs.get(i - 1).is_some_and(|c| class(*c) == 0 && *c != '\n') {
        i -= 1;
    }
    let Some(prev) = i.checked_sub(1).and_then(|j| cs.get(j)).copied() else {
        return i;
    };
    if prev == '\n' {
        return i - 1;
    }
    let k = class(prev);
    while i > 0 && cs.get(i - 1).is_some_and(|c| class(*c) == k) {
        i -= 1;
    }
    i
}

/// The caret one word to the right of `at`: skips spaces, then one word.
pub(crate) fn word_right(text: &str, at: usize) -> usize {
    let cs: Vec<char> = text.chars().collect();
    let mut i = at.min(cs.len());
    while cs.get(i).is_some_and(|c| class(*c) == 0 && *c != '\n') {
        i += 1;
    }
    let Some(next) = cs.get(i).copied() else {
        return i;
    };
    if next == '\n' {
        return i + 1;
    }
    let k = class(next);
    while cs.get(i).is_some_and(|c| class(*c) == k) {
        i += 1;
    }
    i
}

/// The word (or run of spaces or punctuation) around `at`, as a range.
pub(crate) fn word_range(text: &str, at: usize) -> (usize, usize) {
    let cs: Vec<char> = text.chars().collect();
    let at = at.min(cs.len());
    let probe = if cs.get(at).is_some_and(|c| *c != '\n') {
        Some(at)
    } else {
        at.checked_sub(1)
    };
    let Some(p) = probe else {
        return (at, at);
    };
    let Some(c) = cs.get(p).copied().filter(|c| *c != '\n') else {
        return (at, at);
    };
    let k = class(c);
    let mut a = p;
    while a > 0 && cs.get(a - 1).is_some_and(|c| class(*c) == k && *c != '\n') {
        a -= 1;
    }
    let mut b = p;
    while cs.get(b).is_some_and(|c| class(*c) == k && *c != '\n') {
        b += 1;
    }
    (a, b)
}

/// The line (between newlines) around `at`, as a range.
pub(crate) fn line_range(text: &str, at: usize) -> (usize, usize) {
    let cs: Vec<char> = text.chars().collect();
    let at = at.min(cs.len());
    let mut a = at;
    while a > 0 && cs.get(a - 1).is_some_and(|c| *c != '\n') {
        a -= 1;
    }
    let mut b = at;
    while cs.get(b).is_some_and(|c| *c != '\n') {
        b += 1;
    }
    (a, b)
}

/// The text and caret being edited.
pub(crate) struct Buf<'a> {
    pub text: &'a mut String,
    pub cursor: &'a mut usize,
    pub anchor: &'a mut usize,
}

impl Buf<'_> {
    fn len(&self) -> usize {
        self.text.chars().count()
    }

    /// The selected range, if any.
    pub fn selection(&self) -> Option<(usize, usize)> {
        let (a, c) = (*self.anchor, *self.cursor);
        (a != c).then(|| (a.min(c), a.max(c)))
    }

    /// Delete the selection; true when there was one.
    pub fn delete_selection(&mut self) -> bool {
        let Some((a, b)) = self.selection() else {
            *self.anchor = *self.cursor;
            return false;
        };
        let (ba, bb) = (byte_of(self.text, a), byte_of(self.text, b));
        self.text.replace_range(ba..bb, "");
        *self.cursor = a;
        *self.anchor = a;
        true
    }

    /// Insert at the caret, replacing the selection.
    pub fn insert(&mut self, s: &str) {
        self.delete_selection();
        let b = byte_of(self.text, *self.cursor);
        self.text.insert_str(b, s);
        *self.cursor += s.chars().count();
        *self.anchor = *self.cursor;
    }

    pub fn selected(&self) -> Option<String> {
        let (a, b) = self.selection()?;
        Some(self.text.chars().skip(a).take(b - a).collect())
    }

    pub fn select_all(&mut self) {
        *self.anchor = 0;
        *self.cursor = self.len();
    }

    /// Move the caret to `to`; `extend` keeps the anchor (Shift).
    pub fn move_to(&mut self, to: usize, extend: bool) {
        *self.cursor = to.min(self.len());
        if !extend {
            *self.anchor = *self.cursor;
        }
    }

    pub fn left(&mut self, extend: bool, word: bool) {
        if !extend {
            if let Some((a, _)) = self.selection() {
                return self.move_to(a, false);
            }
        }
        let to = if word {
            word_left(self.text, *self.cursor)
        } else {
            self.cursor.saturating_sub(1)
        };
        self.move_to(to, extend);
    }

    pub fn right(&mut self, extend: bool, word: bool) {
        if !extend {
            if let Some((_, b)) = self.selection() {
                return self.move_to(b, false);
            }
        }
        let to = if word {
            word_right(self.text, *self.cursor)
        } else {
            *self.cursor + 1
        };
        self.move_to(to, extend);
    }

    pub fn backspace(&mut self, word: bool) {
        if self.delete_selection() {
            return;
        }
        let to = if word {
            word_left(self.text, *self.cursor)
        } else {
            self.cursor.saturating_sub(1)
        };
        *self.anchor = to;
        self.delete_selection();
    }

    pub fn delete(&mut self, word: bool) {
        if self.delete_selection() {
            return;
        }
        let to = if word {
            word_right(self.text, *self.cursor)
        } else {
            (*self.cursor + 1).min(self.len())
        };
        *self.anchor = to;
        self.delete_selection();
    }
}

/// The character index under `pos` (Fig units) in `t`. With `strict` the
/// point must lie on the text (within `tol`), otherwise the nearest
/// character of the nearest line counts.
pub(crate) fn caret_in_text(t: &Text, pos: Point, tol: f64, strict: bool) -> Option<usize> {
    if !pos.x.is_finite() || !pos.y.is_finite() || !t.angle.is_finite() || !t.size.is_finite() {
        return None;
    }
    let local = Affine::rotate(-t.angle).apply_vec(pos - t.origin);
    let em = t.size * rustfig_geom::FIG_PER_POINT;
    let lh = em * rustfig_text::LINE_SPACING;
    if lh <= 0.0 || !lh.is_finite() {
        return None;
    }
    let lines: Vec<&str> = t.text.split('\n').collect();
    let n = lines.len();
    let top = -0.8 * em;
    let bottom = (n - 1) as f64 * lh + 0.2 * em;
    if strict && (local.y < top - tol || local.y > bottom + tol) {
        return None;
    }
    let li = (((local.y - top) / lh).floor().max(0.0) as usize).min(n - 1);
    let line = lines.get(li).copied().unwrap_or("");
    let width = |k: usize| -> f64 {
        let p: String = line.chars().take(k).collect();
        crate::measure_in_font(t, &p).width
    };
    let nc = line.chars().count();
    let lw = width(nc);
    let x0 = match t.justify {
        Justify::Left => 0.0,
        Justify::Center => -lw / 2.0,
        Justify::Right => -lw,
    };
    let rel = local.x - x0;
    if strict && (rel < -tol || rel > lw + tol) {
        return None;
    }
    // First k whose prefix is at least as wide as the point, then the
    // nearer of k and k - 1.
    let (mut lo, mut hi) = (0usize, nc);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if width(mid) >= rel {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    let k = if lo > 0 && (rel - width(lo - 1)).abs() < (width(lo) - rel).abs() {
        lo - 1
    } else {
        lo
    };
    let before: usize = lines.iter().take(li).map(|l| l.chars().count() + 1).sum();
    Some(before + k)
}
