//! RustFig document model (layer L1).
//!
//! The model mirrors the concepts of the Fig 3.2 format (ellipses,
//! polylines, splines, text, arcs, compounds, user colours) with typed
//! fields, stable object ids and geometry helpers. It knows nothing about
//! files, rendering or the UI.

mod color;
mod geometry;
mod object;
mod xcolors;

pub use color::{area_fill_color, Rgb, STANDARD_COLORS, STANDARD_COLOR_NAMES};
pub use geometry::{arc_geom, flatten_object, FlatPath};
pub use object::*;
pub use xcolors::{x_color, x_color_names};

use rustfig_geom::{Point, Rect};
use serde::{Deserialize, Serialize};

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
    pub objects: Vec<Object>,
    /// Next id handed out by [`Document::alloc_id`].
    #[serde(default)]
    pub next_id: u64,
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
        walk(&mut self.objects, &mut next);
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
        self.objects.iter().find(|o| o.id == id)
    }

    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        self.objects.iter_mut().find(|o| o.id == id)
    }

    pub fn index_of(&self, id: ObjectId) -> Option<usize> {
        self.objects.iter().position(|o| o.id == id)
    }

    /// Bounding box of all objects.
    pub fn bbox(&self) -> Rect {
        self.objects.iter().fold(Rect::EMPTY, |r, o| r.union(&o.bbox()))
    }

    /// Number of leaf objects, counting into compounds.
    pub fn leaf_count(&self) -> usize {
        fn count(objs: &[Object]) -> usize {
            objs.iter()
                .map(|o| match &o.kind {
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
        fn walk<'a>(objs: &'a [Object], f: &mut dyn FnMut(&'a Object)) {
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
        fn walk<'a>(objs: &'a [Object], f: &mut dyn FnMut(&'a Object)) {
            for o in objs {
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
pub fn xfig_order(objs: &[Object]) -> Vec<&Object> {
    let mut v: Vec<&Object> = objs.iter().collect();
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
