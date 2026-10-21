//! RustFig document model (layer L1).
//!
//! The model mirrors the concepts of the Fig 3.2 format (ellipses,
//! polylines, splines, text, arcs, compounds, user colours) with typed
//! fields, stable object ids and geometry helpers. It knows nothing about
//! files, rendering or the UI.

mod geometry;
mod object;

pub use figbase::color::{area_fill_color, x_color, x_color_names, Rgb, STANDARD_COLORS, STANDARD_COLOR_NAMES};
pub use geometry::{arc_geom, flatten_object, FlatPath};
pub use object::*;

use rustfig_geom::Point;
use serde::{Deserialize, Serialize};
use std::sync::Arc as Shared;

/// Page orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Orientation {
    #[default]
    Landscape,
    Portrait,
}

/// Measurement units of the drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Units {
    #[default]
    Inches,
    Metric,
}

/// Paper sizes known to the Fig format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PaperSize {
    #[default]
    Letter,
    Legal,
    Ledger,
    Tabloid,
    A,
    B,
    C,
    D,
    E,
    A4,
    A3,
    A2,
    A1,
    A0,
    B5,
}

impl PaperSize {
    pub const ALL: [PaperSize; 15] = [
        PaperSize::Letter,
        PaperSize::Legal,
        PaperSize::Ledger,
        PaperSize::Tabloid,
        PaperSize::A,
        PaperSize::B,
        PaperSize::C,
        PaperSize::D,
        PaperSize::E,
        PaperSize::A4,
        PaperSize::A3,
        PaperSize::A2,
        PaperSize::A1,
        PaperSize::A0,
        PaperSize::B5,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PaperSize::Letter => "Letter",
            PaperSize::Legal => "Legal",
            PaperSize::Ledger => "Ledger",
            PaperSize::Tabloid => "Tabloid",
            PaperSize::A => "A",
            PaperSize::B => "B",
            PaperSize::C => "C",
            PaperSize::D => "D",
            PaperSize::E => "E",
            PaperSize::A4 => "A4",
            PaperSize::A3 => "A3",
            PaperSize::A2 => "A2",
            PaperSize::A1 => "A1",
            PaperSize::A0 => "A0",
            PaperSize::B5 => "B5",
        }
    }

    pub fn from_name(s: &str) -> Option<PaperSize> {
        PaperSize::ALL
            .iter()
            .copied()
            .find(|p| p.name().eq_ignore_ascii_case(s))
    }

    /// Portrait size in inches (width, height).
    pub fn inches(self) -> (f64, f64) {
        const MM: f64 = 1.0 / 25.4;
        match self {
            PaperSize::Letter | PaperSize::A => (8.5, 11.0),
            PaperSize::Legal => (8.5, 14.0),
            PaperSize::Ledger | PaperSize::Tabloid | PaperSize::B => (11.0, 17.0),
            PaperSize::C => (17.0, 22.0),
            PaperSize::D => (22.0, 34.0),
            PaperSize::E => (34.0, 44.0),
            PaperSize::A4 => (210.0 * MM, 297.0 * MM),
            PaperSize::A3 => (297.0 * MM, 420.0 * MM),
            PaperSize::A2 => (420.0 * MM, 594.0 * MM),
            PaperSize::A1 => (594.0 * MM, 841.0 * MM),
            PaperSize::A0 => (841.0 * MM, 1189.0 * MM),
            PaperSize::B5 => (176.0 * MM, 250.0 * MM),
        }
    }
}

/// Figure-wide settings stored in the file header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocSettings {
    pub orientation: Orientation,
    /// "Center" (true) or "Flush Left" (false).
    pub center: bool,
    pub units: Units,
    pub paper: PaperSize,
    /// Export and print magnification in percent.
    pub magnification: f64,
    pub multiple_pages: bool,
    /// Transparent colour for GIF export (-3 background, -2 none, -1 default, or a colour number).
    pub transparent_color: i32,
}

impl Default for DocSettings {
    fn default() -> Self {
        DocSettings {
            orientation: Orientation::Landscape,
            center: true,
            units: Units::Inches,
            paper: PaperSize::Letter,
            magnification: 100.0,
            multiple_pages: false,
            transparent_color: -2,
        }
    }
}

/// A user-defined colour (Fig colour numbers 32–543).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserColor {
    pub number: i32,
    pub rgb: Rgb,
}

/// First user colour number.
pub const FIRST_USER_COLOR: i32 = 32;
/// Last user colour number.
pub const LAST_USER_COLOR: i32 = 543;

/// A whole drawing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Document {
    pub settings: DocSettings,
    /// Whole-figure comments.
    #[serde(default)]
    pub comments: Vec<String>,
    #[serde(default)]
    pub colors: Vec<UserColor>,
    /// Top-level objects, shared so an undo snapshot copies pointers;
    /// change one through [`std::sync::Arc::make_mut`] (or [`Document::object_mut`]).
    pub objects: Vec<std::sync::Arc<Object>>,
    /// Next id handed out by [`Document::alloc_id`].
    #[serde(default)]
    pub next_id: u64,
    /// Compounds opened for editing in place (xfig's "open compound",
    /// `e_compound.c`), outermost first. While any is open, `objects` holds
    /// the members of the innermost one and the levels hold the rest of the
    /// figure; [`Document::close_all`] puts everything back. Never saved:
    /// files always hold the whole figure.
    #[serde(skip)]
    pub open: Vec<OpenLevel>,
}

/// Most compounds open inside each other (compounds nest no deeper in any
/// real figure; the cap keeps hostile input bounded).
pub const MAX_OPEN_LEVELS: usize = 256;

/// One opened compound: the drawing around it, and the compound itself
/// without its members (they are `Document::objects` while it is open).
#[derive(Debug, Clone, PartialEq)]
pub struct OpenLevel {
    /// The drawing one level up, without the opened compound.
    pub parent: Shared<Vec<Shared<Object>>>,
    /// Where the compound sat in `parent`.
    pub index: usize,
    /// The compound with no members (its id and comments).
    pub shell: Object,
    /// xfig's `draw_parent` ("open, keep visible"): the rest of the figure
    /// is drawn gray behind the open compound.
    pub keep_visible: bool,
}

impl Document {
    pub fn new() -> Self {
        Document::default()
    }

    /// Hand out a fresh object id.
    pub fn alloc_id(&mut self) -> ObjectId {
        self.next_id = self.next_id.max(1);
        let id = ObjectId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    /// Give every object (recursively) a fresh, unique id. Call after loading.
    pub fn assign_ids(&mut self) {
        let mut next = 1u64;
        fn walk(objs: &mut [Object], next: &mut u64) {
            for o in objs {
                o.id = ObjectId(*next);
                *next += 1;
                if let ObjectKind::Compound(c) = &mut o.kind {
                    walk(&mut c.objects, next);
                }
            }
        }
        for o in &mut self.objects {
            walk(std::slice::from_mut(std::sync::Arc::make_mut(o)), &mut next);
        }
        self.next_id = next;
    }

    /// Give `obj` and its children fresh ids from this document.
    pub fn reassign_ids(&mut self, obj: &mut Object) {
        obj.id = self.alloc_id();
        if let ObjectKind::Compound(c) = &mut obj.kind {
            for child in &mut c.objects {
                self.reassign_ids(child);
            }
        }
    }

    /// RGB value of a colour number (default/unknown colours are black).
    pub fn rgb(&self, c: ColorRef) -> Rgb {
        match c.0 {
            n if (0..32).contains(&n) => STANDARD_COLORS.get(n as usize).copied().unwrap_or(Rgb::BLACK),
            n if n >= FIRST_USER_COLOR => self
                .colors
                .iter()
                .find(|u| u.number == n)
                .map(|u| u.rgb)
                .unwrap_or(Rgb::BLACK),
            _ => Rgb::BLACK,
        }
    }

    /// Find a colour number for `rgb`, adding a user colour if needed.
    /// Returns `None` when all user colour slots are taken.
    pub fn color_for_rgb(&mut self, rgb: Rgb) -> Option<ColorRef> {
        if let Some(i) = STANDARD_COLORS.iter().position(|c| *c == rgb) {
            return Some(ColorRef(i as i32));
        }
        if let Some(u) = self.colors.iter().find(|u| u.rgb == rgb) {
            return Some(ColorRef(u.number));
        }
        let number = (FIRST_USER_COLOR..=LAST_USER_COLOR).find(|n| !self.colors.iter().any(|u| u.number == *n))?;
        self.colors.push(UserColor { number, rgb });
        Some(ColorRef(number))
    }

    /// Top-level object with the given id.
    pub fn get(&self, id: ObjectId) -> Option<&Object> {
        self.objects.iter().find(|o| o.id == id).map(|o| &**o)
    }

    /// Top-level object with the given id, unshared from undo snapshots.
    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        self.objects
            .iter_mut()
            .find(|o| o.id == id)
            .map(std::sync::Arc::make_mut)
    }

    pub fn index_of(&self, id: ObjectId) -> Option<usize> {
        self.objects.iter().position(|o| o.id == id)
    }

    /// Open the top-level compound `id` for editing in place (xfig's
    /// `open_this_compound`): its members become `objects`, the rest of the
    /// drawing waits in [`Document::open`]. `keep_visible` is the middle
    /// button's "open, keep visible". Returns false (nothing changed) when
    /// `id` is not a top-level compound or the nesting cap is reached.
    pub fn open_compound(&mut self, id: ObjectId, keep_visible: bool) -> bool {
        if self.open.len() >= MAX_OPEN_LEVELS {
            return false;
        }
        let Some(index) = self.index_of(id) else {
            return false;
        };
        let mut parent = std::mem::take(&mut self.objects);
        let obj = Shared::unwrap_or_clone(parent.remove(index));
        match obj.kind {
            ObjectKind::Compound(c) => {
                let shell = Object {
                    id: obj.id,
                    comments: obj.comments,
                    kind: ObjectKind::Compound(Compound::default()),
                };
                self.objects = c.objects.into_iter().map(Shared::new).collect();
                self.open.push(OpenLevel {
                    parent: Shared::new(parent),
                    index,
                    shell,
                    keep_visible,
                });
                true
            }
            kind => {
                parent.insert(
                    index,
                    Shared::new(Object {
                        id: obj.id,
                        comments: obj.comments,
                        kind,
                    }),
                );
                self.objects = parent;
                false
            }
        }
    }

    /// Close the innermost open compound (xfig's `close_compound`): the
    /// members go back into the compound, which returns to its place; a
    /// compound left without members is dropped. Returns the compound's id,
    /// or `None` when nothing was open.
    pub fn close_compound(&mut self) -> Option<ObjectId> {
        let level = self.open.pop()?;
        let members: Vec<Object> = std::mem::take(&mut self.objects)
            .into_iter()
            .map(Shared::unwrap_or_clone)
            .collect();
        let mut parent = Shared::unwrap_or_clone(level.parent);
        let id = level.shell.id;
        if !members.is_empty() {
            let mut compound = level.shell;
            compound.kind = ObjectKind::Compound(Compound { objects: members });
            let at = level.index.min(parent.len());
            parent.insert(at, Shared::new(compound));
        }
        self.objects = parent;
        Some(id)
    }

    /// Close every open compound (xfig's `close_all_compounds`).
    pub fn close_all(&mut self) {
        while self.close_compound().is_some() {}
    }

    /// The whole figure with every compound closed (a copy).
    pub fn closed_all(&self) -> Document {
        let mut d = self.clone();
        d.close_all();
        d
    }

    /// Distinct depths of the whole figure, also outside the open compounds.
    pub fn all_depths(&self) -> Vec<i32> {
        let mut v = self.depths();
        for level in &self.open {
            let mut d = Document::new();
            d.objects = level.parent.as_ref().clone();
            v.extend(d.depths());
        }
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Number of leaf objects, counting into compounds.
    pub fn leaf_count(&self) -> usize {
        fn count<O: AsRef<Object>>(objs: &[O]) -> usize {
            objs.iter()
                .map(|o| match &o.as_ref().kind {
                    ObjectKind::Compound(c) => count(&c.objects),
                    _ => 1,
                })
                .sum()
        }
        count(&self.objects)
    }

    /// Visit every leaf object (depth-first, file order).
    /// Leaves in xfig's list order: at every level, objects by type
    /// ([`Object::xfig_type_rank`]), keeping their order within a type.
    /// A stable sort of these by depth gives xfig's drawing order.
    pub fn for_each_leaf_xfig_order<'a>(&'a self, f: &mut dyn FnMut(&'a Object)) {
        fn walk<'a, O: AsRef<Object>>(objs: &'a [O], f: &mut dyn FnMut(&'a Object)) {
            for o in xfig_order(objs) {
                match &o.kind {
                    ObjectKind::Compound(c) => walk(&c.objects, f),
                    _ => f(o),
                }
            }
        }
        walk(&self.objects, f);
    }

    pub fn for_each_leaf<'a>(&'a self, f: &mut dyn FnMut(&'a Object)) {
        fn walk<'a, O: AsRef<Object>>(objs: &'a [O], f: &mut dyn FnMut(&'a Object)) {
            for o in objs {
                let o = o.as_ref();
                match &o.kind {
                    ObjectKind::Compound(c) => walk(&c.objects, f),
                    _ => f(o),
                }
            }
        }
        walk(&self.objects, f);
    }

    /// Distinct depths used by leaf objects, ascending.
    pub fn depths(&self) -> Vec<i32> {
        let mut v = Vec::new();
        self.for_each_leaf(&mut |o| v.push(o.depth()));
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Drop user colours no object refers to.
    pub fn remove_unused_colors(&mut self) {
        let mut used = std::collections::BTreeSet::new();
        self.for_each_leaf(&mut |o| {
            if let Some(s) = o.style() {
                used.insert(s.pen_color.0);
                used.insert(s.fill_color.0);
            }
            if let ObjectKind::Text(t) = &o.kind {
                used.insert(t.color.0);
            }
        });
        self.colors.retain(|c| used.contains(&c.number));
    }

    /// Translate a point from Fig units to the given units (inches or cm).
    pub fn to_units(&self, v: f64) -> f64 {
        match self.settings.units {
            Units::Inches => v / rustfig_geom::FIG_PER_INCH,
            Units::Metric => v / rustfig_geom::FIG_PER_CM,
        }
    }

    pub fn unit_name(&self) -> &'static str {
        match self.settings.units {
            Units::Inches => "in",
            Units::Metric => "cm",
        }
    }
}

/// Convenience to build a point.
pub fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// Objects in xfig's list order: by type, keeping the order within a type.
pub fn xfig_order<O: AsRef<Object>>(objs: &[O]) -> Vec<&Object> {
    let mut v: Vec<&Object> = objs.iter().map(AsRef::as_ref).collect();
    v.sort_by_key(|o| o.xfig_type_rank());
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut d = Document::new();
        let a = d.alloc_id();
        let b = d.alloc_id();
        assert_ne!(a, b);
    }

    #[test]
    fn user_colors() {
        let mut d = Document::new();
        assert_eq!(d.color_for_rgb(Rgb::new(255, 0, 0)), Some(ColorRef(4)));
        let c = d.color_for_rgb(Rgb::new(1, 2, 3)).unwrap();
        assert_eq!(c, ColorRef(32));
        assert_eq!(d.rgb(c), Rgb::new(1, 2, 3));
        assert_eq!(d.color_for_rgb(Rgb::new(1, 2, 3)), Some(c));
        for i in 0..600u32 {
            let _ = d.color_for_rgb(Rgb::new((i % 256) as u8, (i / 256) as u8, 77));
        }
        assert!(d.colors.len() <= 512);
    }

    #[test]
    fn paper_names_roundtrip() {
        for p in PaperSize::ALL {
            assert_eq!(PaperSize::from_name(p.name()), Some(p));
        }
    }
}
