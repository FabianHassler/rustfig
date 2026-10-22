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

/// (pen, fill) colour numbers of an object; a compound gives its first
/// member that has them. Text has a pen colour only.
fn colors_of(o: &Object) -> Option<(i32, Option<i32>)> {
    if let Some(s) = o.style() {
        return Some((s.pen_color.0, Some(s.fill_color.0)));
    }
    match &o.kind {
        ObjectKind::Text(t) => Some((t.color.0, None)),
        ObjectKind::Compound(c) => c.objects.iter().find_map(colors_of),
        _ => None,
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
        let found = self
            .hit_test(pos, tol)
            .and_then(|id| self.doc().get(id))
            .and_then(colors_of);
        let Some((pen, fill)) = found else {
            self.set_status("No object there to take a colour from.");
            return;
        };
        let (is_pen, c) = match (target, fill) {
            (Eyedropper::Fill, Some(f)) => (false, f),
            (Eyedropper::Fill, None) => {
                self.set_status("That object has no fill colour.");
                return;
            }
            (Eyedropper::Pen, _) => (true, pen),
        };
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
