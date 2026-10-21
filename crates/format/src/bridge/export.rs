//! A document as fig2dev's drivers take it, without Fig text (plan D7):
//! what [`fig2dev::read::readfp_fig`] returns for `write_fig(doc)` in its
//! default mode (fig2dev's repairs, as when xfig runs fig2dev on a saved
//! file), and the context it leaves.
//!
//! [`export_objects`] returns `None` wherever the reader would say
//! something about the file (a repair it announces, an object it rejects, a
//! colour that doesn't exist, text it can't read back as written) or the
//! text would be read differently from what the model holds (a carriage
//! return, a NUL, a backslash at the end). The caller then writes the Fig
//! text and reads it, so the messages and failures are fig2dev's. Every
//! check is fig2dev's own function from `fig2dev::read`
//! ([`repair_line`](fig2dev::read::repair_line),
//! [`repair_arc`](fig2dev::read::repair_arc), `*_invalid`,
//! [`apply_font_size`](fig2dev::read::apply_font_size),
//! [`trim_text_start`](fig2dev::read::trim_text_start)), not a copy.

use super::from_model::{
    comments, context, give_children, note_arc, note_ellipse, note_line, note_spline, note_text, object_at, FigObject,
};
use fig2dev::gendev::MAX_USR_COLS;
use fig2dev::object::Compound;
use fig2dev::read as rd;
use fig2dev::Ctx;
use rustfig_model::{Document, FIRST_USER_COLOR, LAST_USER_COLOR};
use std::cell::Cell;

/// The objects for the drivers, and `ctx` set as the reader sets it for
/// the file of `doc`: `ctx` comes with the options applied (and
/// `font_size` multiplied by the magnification), exactly as before
/// reading; on return it has the header's values where no option gave
/// them, the colours and what the objects use.
///
/// `None`: use the text path (see the module doc); `ctx` is then
/// unspecified.
pub fn export_objects(doc: &Document, ctx: &mut Ctx) -> Option<Compound> {
    if ctx.input_encoding.is_some() || !colors_regular(doc) || !comments_ok(&doc.comments) {
        return None;
    }
    let header = context(doc);
    // What the reader takes from the header unless an option said so.
    ctx.user_colors = header.user_colors.clone();
    ctx.user_col_indx = header.user_col_indx.clone();
    ctx.pats_used = false;
    ctx.pattern_used = header.pattern_used;
    ctx.std_color_used = header.std_color_used;
    ctx.arrows_used = false;
    ctx.arrow_used = header.arrow_used;
    ctx.v2_flag = header.v2_flag;
    ctx.v21_flag = header.v21_flag;
    ctx.v30_flag = header.v30_flag;
    ctx.v32_flag = header.v32_flag;
    ctx.ppi = header.ppi;
    ctx.thick_scale = header.thick_scale;
    ctx.metric = header.metric;
    if !ctx.orientspec {
        ctx.landscape = header.landscape;
    }
    if !ctx.centerspec {
        ctx.center = header.center;
    }
    if !ctx.paperspec {
        ctx.papersize = header.papersize.clone();
    }
    if !ctx.magspec {
        ctx.mag = header.mag;
        ctx.fontmag = header.fontmag;
    }
    if !ctx.multispec {
        ctx.multi_page = header.multi_page;
    }
    if !ctx.transspec {
        let n = doc.settings.transparent_color;
        if n < -3 {
            return None;
        }
        ctx.header_transparent = n;
        if !header.gif_transparent.is_empty() {
            ctx.gif_transparent = header.gif_transparent.clone();
        }
    }
    let before = ctx.messages.len();
    let odd = Cell::new(false);
    let mut objects = compound(&doc.objects, ctx, 0, &odd)?;
    if odd.get() || ctx.messages.len() != before {
        return None;
    }
    objects.comments = comments(&doc.comments);
    Some(objects)
}

/// Colour definitions the reader takes as they are: in range, each number
/// once, not more than it keeps.
fn colors_regular(doc: &Document) -> bool {
    let mut numbers: Vec<i32> = doc.colors.iter().map(|c| c.number).collect();
    numbers.sort_unstable();
    numbers.len() <= MAX_USR_COLS
        && numbers.windows(2).all(|w| w[0] != w[1])
        && numbers.iter().all(|n| (FIRST_USER_COLOR..=LAST_USER_COLOR).contains(n))
}

/// Whether the reader keeps these comment lines as the model has them: not
/// a carriage return before the newline (it is dropped), no NUL (refused),
/// not too many.
fn comments_ok(lines: &[String]) -> bool {
    let all: Vec<&str> = lines.iter().flat_map(|c| c.split('\n')).collect();
    all.len() <= rd::MAXCOMMENTS && !all.iter().any(|l| l.contains(['\r', '\0']))
}

fn compound<O: AsRef<rustfig_model::Object>>(
    objs: &[O],
    ctx: &mut Ctx,
    depth: usize,
    odd: &Cell<bool>,
) -> Option<Compound> {
    if depth >= rd::MAX_NESTING {
        return None;
    }
    let mut com = Compound::default();
    for o in rustfig_model::xfig_order(objs) {
        let mut obj = object_at(o, ctx, odd);
        // (xfig leaves out an ellipse with a radius of 0, comments and all.)
        if matches!(&obj, FigObject::Ellipse(e) if e.radiuses.x == 0 || e.radiuses.y == 0) {
            continue;
        }
        if !comments_ok(&o.comments) {
            return None;
        }
        // (The conversion cuts a text at a NUL; the file would be refused.)
        if matches!(&o.kind, rustfig_model::ObjectKind::Text(t) if t.text.contains('\0')) {
            return None;
        }
        if let (FigObject::Compound(inner), rustfig_model::ObjectKind::Compound(c)) = (&mut obj, &o.kind) {
            give_children(inner, compound(&c.objects, ctx, depth + 1, odd)?);
        }
        fix(obj, ctx)?.push_into(&mut com);
    }
    Some(com)
}

/// The reader's checks and repairs of one converted object, noting what it
/// uses in `ctx` as the reader does (before it repairs).
fn fix(obj: FigObject, ctx: &mut Ctx) -> Option<FigObject> {
    Some(match obj {
        FigObject::Arc(mut a) => {
            if rd::arc_invalid(&a) || rd::repair_arc(&mut a).is_err() {
                return None;
            }
            note_arc(ctx, &a);
            FigObject::Arc(a)
        }
        FigObject::Ellipse(e) => {
            if rd::ellipse_invalid(&e, false) {
                return None;
            }
            note_ellipse(ctx, &e);
            FigObject::Ellipse(e)
        }
        FigObject::Line(mut l) => {
            if rd::line_invalid(&l) || l.points.is_empty() {
                return None;
            }
            // The reader skips blanks before a picture's name and ends it
            // at the end of the line.
            if let Some(p) = &l.pic {
                if p.file.starts_with(|c: char| c.is_ascii_whitespace()) || p.file.contains(['\r', '\0']) {
                    return None;
                }
            }
            note_line(ctx, &l);
            l.points.dedup();
            let n = l.points.len() as i64;
            match rd::repair_line(&mut l, n) {
                Ok(notes) if notes.is_empty() => {}
                _ => return None,
            }
            FigObject::Line(l)
        }
        FigObject::Spline(s) => {
            if rd::spline_invalid(&s) || s.points.len() < 2 {
                return None;
            }
            // Fig 3.2 splines are lines for the drivers, as the reader
            // makes them (a spline that can't be is a failure there).
            let bad = s.points.iter().all(|p| Some(p) == s.points.first())
                || s.controls.len() != s.points.len()
                || s.controls.iter().any(|c| !c.s.is_finite());
            if bad {
                return None;
            }
            note_spline(ctx, &s);
            FigObject::Line(fig2dev::trans_spline::create_line_with_spline(ctx, &s)?)
        }
        FigObject::Text(mut t) => {
            let text = t.cstring.clone();
            if text.contains(['\r', '\0']) || (text.contains('\\') && (text.ends_with('\\') || text.contains("001"))) {
                return None;
            }
            let trimmed = rd::trim_text_start(&text);
            t.cstring = trimmed.to_string();
            t.raw = trimmed.as_bytes().to_vec();
            rd::apply_font_size(ctx.font_size, &mut t);
            if rd::text_invalid(&t) {
                return None;
            }
            note_text(ctx, &t);
            FigObject::Text(t)
        }
        c @ FigObject::Compound(_) => c,
    })
}
