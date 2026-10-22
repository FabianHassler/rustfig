//! The colour eyedropper (E4): a one-shot pick state. After `color.eyedropper`
//! the next canvas click takes the pen or fill colour of the object under it
//! as the current colour.

use crate::tools::Button;
use crate::Engine;
use rustfig_geom::Point;
use rustfig_model::{ColorRef, Object, ObjectKind};

/// Which current colour the click sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eyedropper {
    Pen,
    Fill,
}

/// The colour number seen at `p` on `o` (within `tol`): the fill colour
/// inside a filled area, else the line (pen) colour; text gives its colour.
/// A compound gives the member under the point.
fn color_at(o: &Object, p: Point, tol: f64) -> Option<i32> {
    match &o.kind {
        ObjectKind::Text(t) => Some(t.color.0),
        ObjectKind::Compound(c) => c
            .objects
            .iter()
            .rev()
            .find(|m| m.distance(p) <= tol)
            .and_then(|m| color_at(m, p, tol)),
        _ => {
            let s = o.style()?;
            if s.is_filled() {
                // On the outline: the pen colour; inside it: the fill.
                let mut line = o.clone();
                if let Some(ls) = line.style_mut() {
                    ls.area_fill = -1;
                }
                if line.distance(p) > tol {
                    return Some(s.fill_color.0);
                }
            }
            Some(s.pen_color.0)
        }
    }
}

impl Engine {
    /// A canvas press while the eyedropper is waiting. Always consumes the
    /// press: the left button picks, the others cancel.
    pub(crate) fn eyedropper_press(&mut self, target: Eyedropper, pos: Point, button: Button) {
        self.eyedropper = None;
        if button != Button::Left || !pos.is_finite() {
            self.set_status("Eyedropper cancelled.");
            return;
        }
        let tol = self.pick_tolerance();
        // The colour under the pointer, whichever part of the object it
        // is; the eyedropper's target only says where it goes.
        let found = self
            .hit_test(pos, tol)
            .and_then(|id| self.doc().get(id))
            .and_then(|o| color_at(o, pos, tol));
        let Some(c) = found else {
            self.set_status("No object there to take a colour from.");
            return;
        };
        let is_pen = target == Eyedropper::Pen;
        if is_pen {
            self.indicator.pen_color = c;
        } else {
            self.indicator.fill_color = c;
        }
        let rgb = self.doc().rgb(ColorRef(c));
        self.prefs.add_recent_color(&rgb.to_hex());
        self.prefs_dirty = true;
        self.set_status(format!(
            "{} colour taken: {}",
            if is_pen { "Pen" } else { "Fill" },
            rgb.to_hex()
        ));
    }
}
