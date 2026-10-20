//! Bounding boxes as fig2dev computes them (plan D2): the objects go
//! through the bridge ([`super::from_model`]) and `fig2dev::bound` measures
//! them, so the box RustFig fits to, selects by and reports equals the
//! frame fig2dev's drivers export (line widths, arrowheads, text extents and
//! C's truncations included).

use fig2dev::bound::{compound_bound, Bounds};
use fig2dev::object::Compound;
use fig2dev::Ctx;
use rustfig_geom::{Point, Rect};
use rustfig_model::{Document, Object};

use super::from_model::{context, driver_compound, object_to_fig2dev};

/// A fig2dev box as a rectangle (empty when fig2dev found nothing).
fn rect(b: Bounds) -> Rect {
    if b.0 > b.2 || b.1 > b.3 {
        return Rect::EMPTY;
    }
    Rect::new(
        Point::new(f64::from(b.0), f64::from(b.1)),
        Point::new(f64::from(b.2), f64::from(b.3)),
    )
}

/// The bounds fig2dev gives `o` in the context `ctx` (from
/// [`super::from_model::context`] of its document), texts included.
pub fn object_bounds(o: &Object, ctx: &mut Ctx) -> Rect {
    let mut com = Compound::default();
    object_to_fig2dev(o, ctx).push_into(&mut com);
    let com = driver_compound(ctx, &com);
    rect(compound_bound(ctx, &com, true))
}

/// The bounds fig2dev gives the whole of `doc`.
pub fn document_bounds(doc: &Document) -> Rect {
    let mut ctx = context(doc);
    doc.objects
        .iter()
        .fold(Rect::EMPTY, |r, o| r.union(&object_bounds(o, &mut ctx)))
}
