//! A RustFig document as fig2dev's objects: the inverse of
//! [`super::to_model`], and the same objects `read.c` returns for the Fig
//! 3.2 text [`crate::write_fig`] writes, without that text.
//!
//! What the conversion does, as the writer and the reader do together:
//! - numbers are rounded as the file prints them (coordinates to integers,
//!   arrowhead sizes to 0.01, style values to 0.001, angles to 0.0001,
//!   arc centres to 0.001), so a figure exports the same through either
//!   path;
//! - thicknesses and corner radii (1/80 inch) and arrowhead thicknesses
//!   are multiplied by the context's `thick_scale` (15 at 1200 ppi);
//! - closed polylines get their first point again at the end, boxes keep
//!   their four corners plus the repeat, arrowheads of closed shapes, of
//!   closed splines and of pie arcs are left out (the file has none);
//! - user colours are numbered `NUM_STD_COLS` + index in the order of the
//!   colour definitions (sorted by number); a user colour that isn't
//!   defined becomes the default colour, as the reader makes it;
//! - texts lose `\u{1}` and stop at a NUL, comments split at newlines;
//! - `Ctx::std_color_used`, `pattern_used` and `arrow_used` are noted as
//!   the reader notes them ([`note_usage`]).
//!
//! Not done here, as the reader's repairs are for files: consecutive equal
//! points are not dropped, invalid objects are not rejected, a text keeps
//! its leading blanks, and the unfilled objects' fill colour is mapped like
//! any other (the reader leaves it as the file has it). [`super::export`]
//! does what the reader does, for the exports (plan D7).

use fig2dev::gendev::{FIG_COLOR_NAMES, MAX_USR_COLS, NUM_STD_COLS};
use fig2dev::object::{self as f, Compound};
use fig2dev::{Ctx, Figure};
use rustfig_geom::Point;
use rustfig_model::{
    Arrow, ColorRef, Document, ObjectKind, Orientation, PolyKind, Style, Units, FIRST_USER_COLOR, LAST_USER_COLOR,
};
use std::cell::Cell;

/// Compounds nested deeper than this are cut off (as the reader refuses
/// them), so that a hostile document can't overflow the stack.
const MAX_DEPTH: usize = crate::MAX_NESTING;

/// One converted object, to be added to a [`Compound`] with
/// [`FigObject::push_into`].
#[derive(Clone, Debug, PartialEq)]
pub enum FigObject {
    Arc(f::Arc),
    Compound(Compound),
    Ellipse(f::Ellipse),
    Line(f::Line),
    Spline(f::Spline),
    Text(f::Text),
}

impl FigObject {
    /// Add the object to the list of its type in `com`.
    pub fn push_into(self, com: &mut Compound) {
        match self {
            FigObject::Arc(a) => com.arcs.push(a),
            FigObject::Compound(c) => com.compounds.push(c),
            FigObject::Ellipse(e) => com.ellipses.push(e),
            FigObject::Line(l) => com.lines.push(l),
            FigObject::Spline(s) => com.splines.push(s),
            FigObject::Text(t) => com.texts.push(t),
        }
    }
}

/// `v` as the file prints it with `prec` decimals and the reader parses it
/// back (0 for a value that isn't finite).
fn q(v: f64, prec: usize) -> f64 {
    if !v.is_finite() {
        return 0.0;
    }
    format!("{v:.prec$}").parse().unwrap_or(0.0)
}

/// A coordinate as an integer file unit.
fn i(v: f64) -> i32 {
    if v.is_finite() {
        v.round().clamp(-2.0e9, 2.0e9) as i32
    } else {
        0
    }
}

/// An angle as the writer prints it: within -7 to 7 as it is, otherwise
/// turned into 0 to 2 pi (0 when not finite).
fn wrap_angle(a: f64) -> f64 {
    if !a.is_finite() {
        0.0
    } else if (-7.0..=7.0).contains(&a) {
        a
    } else {
        a.rem_euclid(std::f64::consts::TAU)
    }
}

fn fpoint(p: Point) -> f::Point {
    f::Point::new(i(p.x), i(p.y))
}

/// The header values and colours of `doc` as a context for fig2dev (1200
/// ppi, the settings the reader takes from the header, user colours as the
/// reader numbers them).
pub fn context(doc: &Document) -> Ctx {
    let mut colors: Vec<_> = doc
        .colors
        .iter()
        .filter(|c| (FIRST_USER_COLOR..=LAST_USER_COLOR).contains(&c.number))
        .collect();
    colors.sort_by_key(|c| c.number);
    colors.dedup_by_key(|c| c.number);
    let list: Vec<(i32, u8, u8, u8)> = colors
        .iter()
        .take(MAX_USR_COLS)
        .map(|c| (c.number, c.rgb.r, c.rgb.g, c.rgb.b))
        .collect();
    let s = &doc.settings;
    let mut ctx = Ctx::for_figure(1200.0, s.units == Units::Metric, &list);
    ctx.landscape = s.orientation == Orientation::Landscape;
    ctx.center = s.center;
    ctx.papersize = s.paper.name().to_string();
    let mag = q(s.magnification, 2) / 100.0;
    let mag = if mag > 0.0 && mag.is_finite() { mag } else { 1.0 };
    ctx.mag = mag;
    ctx.fontmag = mag;
    ctx.multi_page = s.multiple_pages;
    ctx.gif_transparent = transparent_name(&ctx, s.transparent_color);
    ctx
}

/// What the reader makes of the header's transparent colour: a standard
/// colour's name, a user colour as `#rrggbb` (blank padded, as `read.c`
/// formats it), nothing for none, default and background.
fn transparent_name(ctx: &Ctx, number: i32) -> String {
    if let Some(name) = usize::try_from(number).ok().and_then(|n| FIG_COLOR_NAMES.get(n)) {
        return (*name).to_string();
    }
    ctx.user_colors
        .iter()
        .zip(&ctx.user_col_indx)
        .find(|(_, &n)| n == number)
        .map(|(u, _)| format!("#{:2x}{:2x}{:2x}", u.r, u.g, u.b))
        .unwrap_or_default()
}

/// The whole document as fig2dev's figure: the context and the objects,
/// splines as splines (see [`driver_objects`] for what drivers take).
pub fn from_model(doc: &Document) -> Figure {
    let mut ctx = context(doc);
    let mut objects = compound_of(&doc.objects, &ctx, 0);
    objects.comments = comments(&doc.comments);
    note_usage(&mut ctx, &objects);
    Figure { ctx, objects }
}

/// `fig`'s objects as the drivers expect them (as `read.c` returns them for
/// a Fig 3.2 file): the splines converted to lines. fig2dev's messages for
/// a spline that can't be converted go to `fig.ctx`.
pub fn driver_objects(fig: &mut Figure) -> Compound {
    driver_compound(&mut fig.ctx, &fig.objects)
}

/// [`driver_objects`] for any compound built by this module (one object
/// wrapped in a compound, for instance): the repairs of `read.c`'s
/// `sanitize_lineobject` (fig2dev's own [`fig2dev::read::repair_line`]),
/// consecutive equal points dropped, the splines converted to lines. A
/// malformed arc-box or picture box, which the reader rejects, is left as
/// it is.
pub fn driver_compound(ctx: &mut Ctx, com: &Compound) -> Compound {
    let mut objects = com.clone();
    repair_lines(&mut objects);
    fig2dev::trans_spline::splines_to_lines(ctx, &objects)
}

fn repair_lines(c: &mut Compound) {
    for l in &mut c.lines {
        l.points.dedup();
        let saved = l.clone();
        if fig2dev::read::repair_line(l, l.points.len() as i64).is_err() {
            *l = saved;
        }
    }
    c.compounds.iter_mut().for_each(repair_lines);
}

pub(super) fn comments(lines: &[String]) -> Vec<f::Comment> {
    lines
        .iter()
        .flat_map(|c| c.split('\n'))
        .map(|l| f::Comment {
            raw: l.as_bytes().to_vec(),
            text: l.to_string(),
        })
        .collect()
}

fn compound_of<O: AsRef<rustfig_model::Object>>(objs: &[O], ctx: &Ctx, depth: usize) -> Compound {
    let mut com = Compound::default();
    if depth > MAX_DEPTH {
        return com;
    }
    for o in rustfig_model::xfig_order(objs) {
        object_below(o, ctx, depth).push_into(&mut com);
    }
    com
}

/// Put the objects of `kids` into `inner` (a compound whose corners and
/// comments are set).
pub(super) fn give_children(inner: &mut Compound, kids: Compound) {
    inner.lines = kids.lines;
    inner.ellipses = kids.ellipses;
    inner.splines = kids.splines;
    inner.texts = kids.texts;
    inner.arcs = kids.arcs;
    inner.compounds = kids.compounds;
}

/// One object as fig2dev's, for the context `ctx` (from [`context`]).
pub fn object_to_fig2dev(o: &rustfig_model::Object, ctx: &Ctx) -> FigObject {
    object_below(o, ctx, 0)
}

/// [`object_at`] with the children of a compound.
fn object_below(o: &rustfig_model::Object, ctx: &Ctx, depth: usize) -> FigObject {
    let mut obj = object_at(o, ctx, &Cell::new(false));
    if let (FigObject::Compound(inner), ObjectKind::Compound(c)) = (&mut obj, &o.kind) {
        give_children(inner, compound_of(&c.objects, ctx, depth + 1));
    }
    obj
}

/// One object as fig2dev's; `odd` is set where the Fig file the writer
/// makes would draw a message from the reader (a colour that doesn't exist,
/// an arrowhead it drops), which an export through objects must not hide.
pub(super) fn object_at(o: &rustfig_model::Object, ctx: &Ctx, odd: &Cell<bool>) -> FigObject {
    let c = Conv { ctx, odd };
    let cm = comments(&o.comments);
    match &o.kind {
        ObjectKind::Ellipse(e) => {
            let st = c.style(&e.style);
            FigObject::Ellipse(f::Ellipse {
                type_: e.kind.to_fig(),
                style: st.line,
                thickness: st.thickness,
                pen_color: st.pen,
                fill_color: st.fill,
                depth: st.depth,
                pen: st.pen_style,
                fill_style: st.area_fill,
                style_val: st.style_val,
                direction: e.direction,
                angle: q(wrap_angle(e.angle), 4),
                center: fpoint(e.center),
                radiuses: f::Point::new(i(e.radii.x).saturating_abs(), i(e.radii.y).saturating_abs()),
                start: fpoint(e.start),
                end: fpoint(e.end),
                comments: cm,
            })
        }
        ObjectKind::Polyline(p) => {
            let st = c.style(&p.style);
            let mut points: Vec<f::Point> = p.points.iter().map(|&p| fpoint(p)).collect();
            if p.kind.is_closed() {
                if let Some(&first) = points.first() {
                    points.push(first);
                }
            }
            let open = p.kind == PolyKind::Polyline;
            let pic = (p.kind == PolyKind::Picture).then(|| {
                let (flipped, file) = p
                    .picture
                    .as_ref()
                    .map(|pic| (pic.flipped, pic.file.replace('\n', " ")))
                    .unwrap_or_default();
                Box::new(f::Pic {
                    flipped: i32::from(flipped),
                    file_raw: file.as_bytes().to_vec(),
                    file,
                    ..f::Pic::default()
                })
            });
            FigObject::Line(f::Line {
                type_: p.kind.to_fig(),
                style: st.line,
                thickness: st.thickness,
                pen_color: st.pen,
                fill_color: st.fill,
                depth: st.depth,
                pen: st.pen_style,
                fill_style: st.area_fill,
                style_val: st.style_val,
                for_arrow: c.arrow(open, &p.forward_arrow),
                back_arrow: c.arrow(open, &p.backward_arrow),
                cap_style: p.style.cap.to_fig(),
                join_style: p.style.join.to_fig(),
                radius: p.radius.saturating_mul(c.thick_factor()),
                points,
                pic,
                comments: cm,
            })
        }
        ObjectKind::Spline(s) => {
            let st = c.style(&s.style);
            let points: Vec<f::Point> = s.points.iter().map(|&p| fpoint(p)).collect();
            let controls = (0..points.len())
                .map(|k| f::Control {
                    s: q(s.shapes.get(k).copied().unwrap_or(0.0).clamp(-1.0, 1.0), 3),
                    ..f::Control::default()
                })
                .collect();
            FigObject::Spline(f::Spline {
                type_: s.fig_subtype(),
                style: st.line,
                thickness: st.thickness,
                pen_color: st.pen,
                fill_color: st.fill,
                depth: st.depth,
                pen: st.pen_style,
                fill_style: st.area_fill,
                style_val: st.style_val,
                for_arrow: c.arrow(!s.closed, &s.forward_arrow),
                back_arrow: c.arrow(!s.closed, &s.backward_arrow),
                cap_style: s.style.cap.to_fig(),
                points,
                controls,
                comments: cm,
            })
        }
        ObjectKind::Text(t) => {
            let mut text = t.text.replace('\u{1}', "");
            if let Some(nul) = text.find('\0') {
                text.truncate(nul);
            }
            FigObject::Text(f::Text {
                type_: t.justify.to_fig(),
                font: t.font,
                size: if t.size.is_finite() { t.size } else { 0.0 },
                color: c.color(t.color),
                depth: t.depth,
                angle: q(wrap_angle(t.angle), 4),
                flags: t.flags,
                height: f64::from(i(t.height).max(0)),
                length: f64::from(i(t.length).max(0)),
                base_x: i(t.origin.x),
                base_y: i(t.origin.y),
                pen: t.pen_style,
                raw: text.as_bytes().to_vec(),
                cstring: text,
                comments: cm,
            })
        }
        ObjectKind::Arc(a) => {
            let st = c.style(&a.style);
            let pts = a.points.map(fpoint);
            FigObject::Arc(f::Arc {
                type_: if a.pie { f::T_PIE_WEDGE_ARC } else { f::T_OPEN_ARC },
                style: st.line,
                thickness: st.thickness,
                pen_color: st.pen,
                fill_color: st.fill,
                depth: st.depth,
                pen: st.pen_style,
                fill_style: st.area_fill,
                style_val: st.style_val,
                for_arrow: c.arrow(!a.pie, &a.forward_arrow),
                back_arrow: c.arrow(!a.pie, &a.backward_arrow),
                cap_style: a.style.cap.to_fig(),
                direction: i32::from(a.ccw),
                center: f::DPoint {
                    x: q(a.center.x, 3),
                    y: q(a.center.y, 3),
                },
                point: pts,
                comments: cm,
            })
        }
        ObjectKind::Compound(_) => {
            let mut inner = Compound::default();
            let b = o.geometry_bbox();
            let (lo, hi) = if b.is_empty() {
                (Point::ZERO, Point::ZERO)
            } else {
                (b.min, b.max)
            };
            inner.nwcorner = fpoint(lo);
            inner.secorner = fpoint(hi);
            inner.comments = cm;
            FigObject::Compound(inner)
        }
    }
}

/// A style's numbers in fig2dev's units.
struct Common {
    line: i32,
    thickness: i32,
    pen: i32,
    fill: i32,
    depth: i32,
    pen_style: i32,
    area_fill: i32,
    style_val: f64,
}

struct Conv<'a> {
    ctx: &'a Ctx,
    odd: &'a Cell<bool>,
}

impl Conv<'_> {
    fn thick_factor(&self) -> i32 {
        fig2dev::round(f64::from(self.ctx.thick_scale))
    }

    /// A colour number as the objects carry it: user colours become
    /// `NUM_STD_COLS` + their index, an undefined one the default.
    fn color(&self, c: ColorRef) -> i32 {
        if c.0 < NUM_STD_COLS as i32 {
            self.odd.set(self.odd.get() || c.0 < f::DEFAULT);
            return c.0.max(-1);
        }
        match self.ctx.user_col_indx.iter().position(|&n| n == c.0) {
            Some(idx) => NUM_STD_COLS as i32 + idx as i32,
            None => {
                self.odd.set(true);
                f::DEFAULT
            }
        }
    }

    fn style(&self, s: &Style) -> Common {
        Common {
            line: s.line.to_fig(),
            thickness: s.thickness.saturating_mul(self.thick_factor()),
            pen: self.color(s.pen_color),
            fill: self.color(s.fill_color),
            depth: s.depth,
            pen_style: s.pen_style,
            area_fill: s.area_fill,
            style_val: q(s.style_val, 3),
        }
    }

    /// The arrowhead, or none where the file has none (`allowed` false).
    fn arrow(&self, allowed: bool, a: &Option<Arrow>) -> Option<f::Arrow> {
        let a = a.as_ref().filter(|_| allowed)?;
        let (thickness, wid, ht) = (q(a.thickness, 2), q(a.width, 2), q(a.height, 2));
        if fig2dev::read::arrow_fault(a.kind, a.style, thickness, wid, ht).is_some() {
            self.odd.set(true);
        }
        Some(f::Arrow {
            type_: a.kind,
            style: a.style,
            thickness: thickness * f64::from(self.ctx.thick_scale),
            wid,
            ht,
        })
    }
}

/// Note the standard colours, fill patterns and arrowheads `objects` use
/// in `ctx`, as the reader does (`fix_and_note_color`, `note_fill`,
/// `make_arrow`); drivers write only what is noted.
pub fn note_usage(ctx: &mut Ctx, objects: &Compound) {
    objects.arcs.iter().for_each(|a| note_arc(ctx, a));
    objects.ellipses.iter().for_each(|e| note_ellipse(ctx, e));
    objects.lines.iter().for_each(|l| note_line(ctx, l));
    objects.splines.iter().for_each(|s| note_spline(ctx, s));
    objects.texts.iter().for_each(|t| note_text(ctx, t));
    objects.compounds.iter().for_each(|c| note_usage(ctx, c));
}

fn note_color(ctx: &mut Ctx, c: i32) {
    if let Some(u) = usize::try_from(c).ok().and_then(|i| ctx.std_color_used.get_mut(i)) {
        *u = true;
    }
}

fn note_fill(ctx: &mut Ctx, style: i32, c: i32) {
    if style == f::UNFILLED {
        return;
    }
    note_color(ctx, c);
    if style >= f::NUMSHADES + f::NUMTINTS {
        if let Some(p) = usize::try_from(style - f::NUMSHADES - f::NUMTINTS)
            .ok()
            .and_then(|i| ctx.pattern_used.get_mut(i))
        {
            *p = true;
        }
        ctx.pats_used = true;
    }
}

fn note_arrows(ctx: &mut Ctx, list: [&Option<f::Arrow>; 2]) {
    for a in list.into_iter().flatten() {
        ctx.arrows_used = true;
        if let Some(u) = usize::try_from(2 * a.type_ + a.style)
            .ok()
            .and_then(|i| ctx.arrow_used.get_mut(i))
        {
            *u = true;
        }
    }
}

/// What the reader notes for one arc.
pub(super) fn note_arc(ctx: &mut Ctx, a: &f::Arc) {
    note_color(ctx, a.pen_color);
    note_fill(ctx, a.fill_style, a.fill_color);
    note_arrows(ctx, [&a.for_arrow, &a.back_arrow]);
}

/// What the reader notes for one ellipse.
pub(super) fn note_ellipse(ctx: &mut Ctx, e: &f::Ellipse) {
    note_color(ctx, e.pen_color);
    note_fill(ctx, e.fill_style, e.fill_color);
}

/// What the reader notes for one line (as read, before its repairs: the
/// fill of a short polyline counts although the repair removes it).
pub(super) fn note_line(ctx: &mut Ctx, l: &f::Line) {
    note_color(ctx, l.pen_color);
    note_fill(ctx, l.fill_style, l.fill_color);
    note_arrows(ctx, [&l.for_arrow, &l.back_arrow]);
}

/// What the reader notes for one spline.
pub(super) fn note_spline(ctx: &mut Ctx, s: &f::Spline) {
    note_color(ctx, s.pen_color);
    note_fill(ctx, s.fill_style, s.fill_color);
    note_arrows(ctx, [&s.for_arrow, &s.back_arrow]);
}

/// What the reader notes for one text.
pub(super) fn note_text(ctx: &mut Ctx, t: &f::Text) {
    note_color(ctx, t.color);
}
