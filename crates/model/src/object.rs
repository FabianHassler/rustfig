//! Drawing objects and their attributes.

use rustfig_geom::{arc_from_three_points, flatten_round_rect, Affine, Point, Rect};
use serde::{Deserialize, Serialize};
use std::sync::Arc as Shared;

/// Stable identity of an object within a document session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct ObjectId(pub u64);

/// A Fig colour number: -1 default, 0–31 standard, 32–543 user colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColorRef(pub i32);

impl ColorRef {
    pub const DEFAULT: ColorRef = ColorRef(-1);
    pub const BLACK: ColorRef = ColorRef(0);
    pub const WHITE: ColorRef = ColorRef(7);

    pub fn is_default_or_black(self) -> bool {
        self.0 <= 0
    }
}

impl Default for ColorRef {
    fn default() -> Self {
        ColorRef::BLACK
    }
}

/// Line dash style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LineKind {
    Default,
    #[default]
    Solid,
    Dashed,
    Dotted,
    DashDotted,
    DashDoubleDotted,
    DashTripleDotted,
}

impl LineKind {
    pub const ALL: [LineKind; 6] = [
        LineKind::Solid,
        LineKind::Dashed,
        LineKind::Dotted,
        LineKind::DashDotted,
        LineKind::DashDoubleDotted,
        LineKind::DashTripleDotted,
    ];

    pub fn from_fig(v: i32) -> LineKind {
        match v {
            0 => LineKind::Solid,
            1 => LineKind::Dashed,
            2 => LineKind::Dotted,
            3 => LineKind::DashDotted,
            4 => LineKind::DashDoubleDotted,
            5 => LineKind::DashTripleDotted,
            _ => LineKind::Default,
        }
    }

    pub fn to_fig(self) -> i32 {
        match self {
            LineKind::Default => -1,
            LineKind::Solid => 0,
            LineKind::Dashed => 1,
            LineKind::Dotted => 2,
            LineKind::DashDotted => 3,
            LineKind::DashDoubleDotted => 4,
            LineKind::DashTripleDotted => 5,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            LineKind::Default => "default",
            LineKind::Solid => "solid",
            LineKind::Dashed => "dashed",
            LineKind::Dotted => "dotted",
            LineKind::DashDotted => "dash-dotted",
            LineKind::DashDoubleDotted => "dash-double-dotted",
            LineKind::DashTripleDotted => "dash-triple-dotted",
        }
    }

    pub fn from_name(s: &str) -> Option<LineKind> {
        [LineKind::Default]
            .into_iter()
            .chain(LineKind::ALL)
            .find(|k| k.name() == s)
    }

    /// Dash pattern in Fig units for a given style value (1/80 inch) and
    /// line width. Empty for solid lines. Dots are zero-length dashes, meant
    /// to be drawn with round caps.
    pub fn dash_pattern(self, style_val: f64, width: f64) -> Vec<f64> {
        let unit = rustfig_geom::FIG_PER_STYLE_UNIT;
        let v = if style_val.is_finite() && style_val > 0.0 {
            style_val * unit
        } else {
            4.0 * unit
        };
        let dot = width.max(1.0) * 0.01;
        let gap = v.max(width * 1.5);
        match self {
            LineKind::Default | LineKind::Solid => Vec::new(),
            LineKind::Dashed => vec![v, v],
            LineKind::Dotted => vec![dot, gap],
            LineKind::DashDotted => vec![v, gap / 2.0, dot, gap / 2.0],
            LineKind::DashDoubleDotted => vec![v, gap / 2.0, dot, gap / 2.0, dot, gap / 2.0],
            LineKind::DashTripleDotted => vec![v, gap / 2.0, dot, gap / 2.0, dot, gap / 2.0, dot, gap / 2.0],
        }
    }
}

/// Line cap style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapStyle {
    #[default]
    Butt,
    Round,
    Projecting,
}

impl CapStyle {
    pub fn from_fig(v: i32) -> CapStyle {
        match v {
            1 => CapStyle::Round,
            2 => CapStyle::Projecting,
            _ => CapStyle::Butt,
        }
    }
    pub fn to_fig(self) -> i32 {
        self as i32
    }
}

/// Line join style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JoinStyle {
    #[default]
    Miter,
    Round,
    Bevel,
}

impl JoinStyle {
    pub fn from_fig(v: i32) -> JoinStyle {
        match v {
            1 => JoinStyle::Round,
            2 => JoinStyle::Bevel,
            _ => JoinStyle::Miter,
        }
    }
    pub fn to_fig(self) -> i32 {
        self as i32
    }
}

/// Graphic attributes shared by ellipses, polylines, splines and arcs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub line: LineKind,
    /// Line thickness in 1/80 inch.
    pub thickness: i32,
    pub pen_color: ColorRef,
    pub fill_color: ColorRef,
    /// 0–999, larger is further back.
    pub depth: i32,
    pub pen_style: i32,
    /// -1 no fill, 0–40 shades/tints, 41–62 patterns.
    pub area_fill: i32,
    /// Dash length / dot gap in 1/80 inch.
    pub style_val: f64,
    pub cap: CapStyle,
    pub join: JoinStyle,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            line: LineKind::Solid,
            thickness: 1,
            pen_color: ColorRef::BLACK,
            fill_color: ColorRef::WHITE,
            depth: 50,
            pen_style: -1,
            area_fill: -1,
            style_val: 0.0,
            cap: CapStyle::Butt,
            join: JoinStyle::Miter,
        }
    }
}

impl Style {
    pub fn is_filled(&self) -> bool {
        self.area_fill >= 0
    }
}

/// Maximum object depth.
pub const MAX_DEPTH: i32 = 999;

/// An arrowhead specification.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Arrow {
    /// 0 stick, 1 triangle, 2 indented, 3 pointed, 4+ extended shapes.
    pub kind: i32,
    /// 0 hollow (white), 1 filled with pen colour.
    pub style: i32,
    /// Outline thickness in 1/80 inch.
    pub thickness: f64,
    /// Width in Fig units.
    pub width: f64,
    /// Length in Fig units.
    pub height: f64,
}

impl Default for Arrow {
    fn default() -> Self {
        Arrow {
            kind: 1,
            style: 1,
            thickness: 1.0,
            width: 60.0,
            height: 120.0,
        }
    }
}

/// Ellipse sub-types (how it was entered).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EllipseKind {
    EllipseRadii,
    EllipseDiameter,
    CircleRadius,
    CircleDiameter,
}

impl EllipseKind {
    pub fn from_fig(v: i32) -> EllipseKind {
        match v {
            2 => EllipseKind::EllipseDiameter,
            3 => EllipseKind::CircleRadius,
            4 => EllipseKind::CircleDiameter,
            _ => EllipseKind::EllipseRadii,
        }
    }
    pub fn to_fig(self) -> i32 {
        match self {
            EllipseKind::EllipseRadii => 1,
            EllipseKind::EllipseDiameter => 2,
            EllipseKind::CircleRadius => 3,
            EllipseKind::CircleDiameter => 4,
        }
    }
    pub fn is_circle(self) -> bool {
        matches!(self, EllipseKind::CircleRadius | EllipseKind::CircleDiameter)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ellipse {
    pub kind: EllipseKind,
    pub style: Style,
    pub direction: i32,
    /// Angle of the x axis, radians counter-clockwise.
    pub angle: f64,
    pub center: Point,
    pub radii: Point,
    pub start: Point,
    pub end: Point,
}

/// Polyline sub-types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolyKind {
    Polyline,
    Box,
    Polygon,
    ArcBox,
    Picture,
}

impl PolyKind {
    pub fn from_fig(v: i32) -> PolyKind {
        match v {
            2 => PolyKind::Box,
            3 => PolyKind::Polygon,
            4 => PolyKind::ArcBox,
            5 => PolyKind::Picture,
            _ => PolyKind::Polyline,
        }
    }
    pub fn to_fig(self) -> i32 {
        match self {
            PolyKind::Polyline => 1,
            PolyKind::Box => 2,
            PolyKind::Polygon => 3,
            PolyKind::ArcBox => 4,
            PolyKind::Picture => 5,
        }
    }
    pub fn is_closed(self) -> bool {
        !matches!(self, PolyKind::Polyline)
    }
    /// Box-like kinds whose points form an axis-aligned rectangle.
    pub fn is_rect(self) -> bool {
        matches!(self, PolyKind::Box | PolyKind::ArcBox | PolyKind::Picture)
    }
}

/// Decoded raster image of an imported picture (RGBA8, row-major).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// An imported picture inside a picture polyline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Picture {
    pub flipped: bool,
    pub file: String,
    /// Decoded pixels, if the file could be loaded. Not saved.
    #[serde(skip)]
    pub image: Option<Shared<RasterImage>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polyline {
    pub kind: PolyKind,
    pub style: Style,
    /// Corner radius of arc-boxes in 1/80 inch.
    pub radius: i32,
    pub forward_arrow: Option<Arrow>,
    pub backward_arrow: Option<Arrow>,
    /// Vertices. Closed kinds do not repeat the first point.
    pub points: Vec<Point>,
    pub picture: Option<Picture>,
}

impl Polyline {
    pub fn new(kind: PolyKind, style: Style, points: Vec<Point>) -> Polyline {
        Polyline {
            kind,
            style,
            radius: 7,
            forward_arrow: None,
            backward_arrow: None,
            points,
            picture: None,
        }
    }

    /// A box from two opposite corners.
    pub fn rect(style: Style, a: Point, b: Point) -> Polyline {
        let r = Rect::new(a, b);
        Polyline::new(PolyKind::Box, style, r.corners().to_vec())
    }
}

/// Spline sub-types (the curve model of each point lives in `shapes`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplineKind {
    Approximated,
    Interpolated,
    X,
}

impl SplineKind {
    pub fn default_shape(self) -> f64 {
        match self {
            SplineKind::Approximated => 1.0,
            SplineKind::Interpolated => -1.0,
            SplineKind::X => 0.5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spline {
    pub kind: SplineKind,
    pub closed: bool,
    pub style: Style,
    pub forward_arrow: Option<Arrow>,
    pub backward_arrow: Option<Arrow>,
    pub points: Vec<Point>,
    /// One shape factor per point, in [-1, 1].
    pub shapes: Vec<f64>,
}

impl Spline {
    pub fn new(kind: SplineKind, closed: bool, style: Style, points: Vec<Point>) -> Spline {
        let n = points.len();
        let mut shapes = vec![kind.default_shape(); n];
        if !closed {
            if let Some(s) = shapes.first_mut() {
                *s = 0.0;
            }
            if let Some(s) = shapes.last_mut() {
                *s = 0.0;
            }
        }
        Spline {
            kind,
            closed,
            style,
            forward_arrow: None,
            backward_arrow: None,
            points,
            shapes,
        }
    }

    pub fn fig_subtype(&self) -> i32 {
        let base = match self.kind {
            SplineKind::Approximated => 0,
            SplineKind::Interpolated => 2,
            SplineKind::X => 4,
        };
        base + i32::from(self.closed)
    }
}

/// Text justification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Justify {
    #[default]
    Left,
    Center,
    Right,
}

impl Justify {
    pub fn from_fig(v: i32) -> Justify {
        match v {
            1 => Justify::Center,
            2 => Justify::Right,
            _ => Justify::Left,
        }
    }
    pub fn to_fig(self) -> i32 {
        self as i32
    }
    pub fn swapped(self) -> Justify {
        match self {
            Justify::Left => Justify::Right,
            Justify::Right => Justify::Left,
            Justify::Center => Justify::Center,
        }
    }
}

/// Text flag bits.
pub mod text_flags {
    pub const RIGID: i32 = 1;
    pub const SPECIAL: i32 = 2;
    pub const POSTSCRIPT: i32 = 4;
    pub const HIDDEN: i32 = 8;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Text {
    pub justify: Justify,
    pub color: ColorRef,
    pub depth: i32,
    pub pen_style: i32,
    /// Font number; meaning depends on the PostScript flag.
    pub font: i32,
    /// Size in points.
    pub size: f64,
    /// Radians, counter-clockwise.
    pub angle: f64,
    pub flags: i32,
    /// Height in Fig units (ascent of the string).
    pub height: f64,
    /// Length in Fig units (advance width of the longest line).
    pub length: f64,
    /// Baseline anchor; meaning depends on `justify`.
    pub origin: Point,
    pub text: String,
}

impl Text {
    pub fn is_postscript(&self) -> bool {
        self.flags & text_flags::POSTSCRIPT != 0
    }
    pub fn is_hidden(&self) -> bool {
        self.flags & text_flags::HIDDEN != 0
    }
    pub fn is_special(&self) -> bool {
        self.flags & text_flags::SPECIAL != 0
    }
    pub fn is_rigid(&self) -> bool {
        self.flags & text_flags::RIGID != 0
    }

    /// The four corners of the text box (baseline-left first), from the
    /// stored `length` and `height`.
    pub fn corners(&self) -> [Point; 4] {
        let l = if self.length.is_finite() {
            self.length.max(0.0)
        } else {
            0.0
        };
        let h = if self.height.is_finite() {
            self.height.max(0.0)
        } else {
            0.0
        };
        let x0 = match self.justify {
            Justify::Left => 0.0,
            Justify::Center => -l / 2.0,
            Justify::Right => -l,
        };
        let rot = Affine::rotate(if self.angle.is_finite() { self.angle } else { 0.0 });
        let lines = self.text.lines().count().max(1) as f64;
        let line_gap = self.size.max(0.0) * rustfig_geom::FIG_PER_POINT * 1.2;
        let descent = h * 0.25 + (lines - 1.0) * line_gap;
        [
            Point::new(x0, descent),
            Point::new(x0 + l, descent),
            Point::new(x0 + l, -h),
            Point::new(x0, -h),
        ]
        .map(|p| self.origin + rot.apply_vec(p))
    }
}

/// Circular arc through three points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Arc {
    /// Pie wedge (closed) instead of an open arc.
    pub pie: bool,
    pub style: Style,
    /// Counter-clockwise on screen (Fig direction 1).
    pub ccw: bool,
    pub forward_arrow: Option<Arrow>,
    pub backward_arrow: Option<Arrow>,
    pub center: Point,
    pub points: [Point; 3],
}

impl Arc {
    /// Build an arc through three points; `None` if they are collinear.
    pub fn through(style: Style, p1: Point, p2: Point, p3: Point, pie: bool) -> Option<Arc> {
        let g = arc_from_three_points(p1, p2, p3)?;
        Some(Arc {
            pie,
            style,
            ccw: g.counter_clockwise(),
            forward_arrow: None,
            backward_arrow: None,
            center: g.center,
            points: [p1, p2, p3],
        })
    }

    /// Recompute centre and direction from the three points.
    pub fn refit(&mut self) {
        if let Some(g) = arc_from_three_points(self.points[0], self.points[1], self.points[2]) {
            self.center = g.center;
            self.ccw = g.counter_clockwise();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Compound {
    pub objects: Vec<Object>,
}

/// The kinds of drawing objects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ObjectKind {
    Ellipse(Ellipse),
    Polyline(Polyline),
    Spline(Spline),
    Text(Text),
    Arc(Arc),
    Compound(Compound),
}

/// A drawing object with its identity and attached comments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Object {
    #[serde(default)]
    pub id: ObjectId,
    #[serde(default)]
    pub comments: Vec<String>,
    #[serde(flatten)]
    pub kind: ObjectKind,
}

impl Object {
    pub fn new(kind: ObjectKind) -> Object {
        Object {
            id: ObjectId(0),
            comments: Vec::new(),
            kind,
        }
    }

    /// Short type name ("polyline", "box", "text", ...).
    pub fn type_name(&self) -> &'static str {
        match &self.kind {
            ObjectKind::Ellipse(e) if e.kind.is_circle() => "circle",
            ObjectKind::Ellipse(_) => "ellipse",
            ObjectKind::Polyline(p) => match p.kind {
                PolyKind::Polyline => "polyline",
                PolyKind::Box => "box",
                PolyKind::Polygon => "polygon",
                PolyKind::ArcBox => "arc-box",
                PolyKind::Picture => "picture",
            },
            ObjectKind::Spline(s) if s.closed => "closed spline",
            ObjectKind::Spline(_) => "spline",
            ObjectKind::Text(_) => "text",
            ObjectKind::Arc(_) => "arc",
            ObjectKind::Compound(_) => "compound",
        }
    }

    pub fn style(&self) -> Option<&Style> {
        match &self.kind {
            ObjectKind::Ellipse(e) => Some(&e.style),
            ObjectKind::Polyline(p) => Some(&p.style),
            ObjectKind::Spline(s) => Some(&s.style),
            ObjectKind::Arc(a) => Some(&a.style),
            ObjectKind::Text(_) | ObjectKind::Compound(_) => None,
        }
    }

    pub fn style_mut(&mut self) -> Option<&mut Style> {
        match &mut self.kind {
            ObjectKind::Ellipse(e) => Some(&mut e.style),
            ObjectKind::Polyline(p) => Some(&mut p.style),
            ObjectKind::Spline(s) => Some(&mut s.style),
            ObjectKind::Arc(a) => Some(&mut a.style),
            ObjectKind::Text(_) | ObjectKind::Compound(_) => None,
        }
    }

    /// Depth (the minimum depth of the children for compounds).
    pub fn depth(&self) -> i32 {
        match &self.kind {
            ObjectKind::Text(t) => t.depth,
            ObjectKind::Compound(c) => c.objects.iter().map(Object::depth).min().unwrap_or(0),
            _ => self.style().map(|s| s.depth).unwrap_or(0),
        }
    }

    /// Set the depth (shifting all children of a compound by the same amount).
    pub fn set_depth(&mut self, depth: i32) {
        let depth = depth.clamp(0, MAX_DEPTH);
        match &mut self.kind {
            ObjectKind::Text(t) => t.depth = depth,
            ObjectKind::Compound(c) => {
                let cur = c.objects.iter().map(Object::depth).min().unwrap_or(0);
                let delta = depth - cur;
                for o in &mut c.objects {
                    let d = o.depth() + delta;
                    o.set_depth(d);
                }
            }
            _ => {
                if let Some(s) = self.style_mut() {
                    s.depth = depth;
                }
            }
        }
    }

    pub fn arrows_mut(&mut self) -> Option<(&mut Option<Arrow>, &mut Option<Arrow>)> {
        match &mut self.kind {
            ObjectKind::Polyline(p) if p.kind == PolyKind::Polyline => {
                Some((&mut p.forward_arrow, &mut p.backward_arrow))
            }
            ObjectKind::Spline(s) if !s.closed => Some((&mut s.forward_arrow, &mut s.backward_arrow)),
            ObjectKind::Arc(a) if !a.pie => Some((&mut a.forward_arrow, &mut a.backward_arrow)),
            _ => None,
        }
    }

    /// Editable vertices, if the object has any.
    pub fn points(&self) -> Option<&[Point]> {
        match &self.kind {
            ObjectKind::Polyline(p) => Some(&p.points),
            ObjectKind::Spline(s) => Some(&s.points),
            ObjectKind::Arc(a) => Some(&a.points),
            _ => None,
        }
    }

    /// Bounding box including line width and arrowheads (approximately).
    pub fn bbox(&self) -> Rect {
        match &self.kind {
            ObjectKind::Compound(c) => c.objects.iter().fold(Rect::EMPTY, |r, o| r.union(&o.bbox())),
            ObjectKind::Text(t) => Rect::from_points(t.corners()),
            _ => {
                let mut r = Rect::EMPTY;
                for p in crate::flatten_object(self) {
                    for q in p.pts {
                        r.add_point(q);
                    }
                }
                let w = self
                    .style()
                    .map(|s| s.thickness.max(0) as f64 * rustfig_geom::FIG_PER_THICKNESS / 2.0)
                    .unwrap_or(0.0);
                r.expand(w)
            }
        }
    }

    /// Bounding box of the geometry only (no line width): what alignment uses.
    pub fn geometry_bbox(&self) -> Rect {
        match &self.kind {
            ObjectKind::Compound(c) => c.objects.iter().fold(Rect::EMPTY, |r, o| r.union(&o.geometry_bbox())),
            ObjectKind::Text(t) => Rect::from_points(t.corners()),
            _ => {
                let mut r = Rect::EMPTY;
                for p in crate::flatten_object(self) {
                    for q in p.pts {
                        r.add_point(q);
                    }
                }
                r
            }
        }
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.transform(&Affine::translate(dx, dy), false);
    }

    /// Apply an affine transform to the geometry. `scale_text` also scales
    /// font sizes of non-rigid texts.
    pub fn transform(&mut self, t: &Affine, scale_text: bool) {
        let f = |p: &mut Point| *p = t.apply(*p);
        match &mut self.kind {
            ObjectKind::Ellipse(e) => {
                let (rx, ry, angle) = transform_ellipse(t, e.radii.x, e.radii.y, e.angle);
                e.center = t.apply(e.center);
                f(&mut e.start);
                f(&mut e.end);
                e.radii = Point::new(rx, ry);
                e.angle = angle;
                if e.kind.is_circle() && (rx - ry).abs() > 0.5 {
                    e.kind = EllipseKind::EllipseRadii;
                }
                if e.kind.is_circle() {
                    e.angle = 0.0;
                }
            }
            ObjectKind::Polyline(p) => {
                if p.kind == PolyKind::ArcBox && !preserves_axes(t) {
                    let r = Rect::from_points(p.points.iter().copied());
                    p.points = flatten_round_rect(&r, p.radius as f64 * rustfig_geom::FIG_PER_STYLE_UNIT);
                    p.kind = PolyKind::Polygon;
                }
                p.points.iter_mut().for_each(f);
                if p.kind.is_rect() {
                    if preserves_axes(t) {
                        let r = Rect::from_points(p.points.iter().copied());
                        if p.kind == PolyKind::Picture {
                            if let Some(pic) = &mut p.picture {
                                if t.is_mirroring() {
                                    pic.flipped = !pic.flipped;
                                }
                            }
                        }
                        p.points = r.corners().to_vec();
                    } else if p.kind == PolyKind::Box {
                        p.kind = PolyKind::Polygon;
                    }
                }
            }
            ObjectKind::Spline(s) => s.points.iter_mut().for_each(f),
            ObjectKind::Arc(a) => {
                a.points.iter_mut().for_each(f);
                a.center = t.apply(a.center);
                a.refit();
            }
            ObjectKind::Text(tx) => {
                tx.origin = t.apply(tx.origin);
                let dir = Point::new(tx.angle.cos(), -tx.angle.sin());
                let nd = t.apply_vec(dir);
                let mut angle = (-nd.y).atan2(nd.x);
                if t.is_mirroring() {
                    // Keep text readable: mirror the anchor, not the glyphs.
                    let readable = -tx.angle;
                    if (angle - readable).cos() < 0.0 {
                        angle += std::f64::consts::PI;
                        tx.justify = tx.justify.swapped();
                    }
                }
                if angle.abs() < 1e-9 {
                    angle = 0.0;
                }
                tx.angle = angle.rem_euclid(std::f64::consts::TAU);
                if tx.angle > std::f64::consts::TAU - 1e-9 {
                    tx.angle = 0.0;
                }
                let s = t.mean_scale();
                if scale_text && !tx.is_rigid() && s.is_finite() && s > 0.0 {
                    tx.size = (tx.size * s).clamp(1.0, 2000.0);
                    tx.height *= s;
                    tx.length *= s;
                }
            }
            ObjectKind::Compound(c) => {
                for o in &mut c.objects {
                    o.transform(t, scale_text);
                }
            }
        }
    }

    /// Distance from `p` to the object's outline; 0 inside filled shapes
    /// and inside text boxes.
    pub fn distance(&self, p: Point) -> f64 {
        match &self.kind {
            ObjectKind::Compound(c) => c.objects.iter().map(|o| o.distance(p)).fold(f64::INFINITY, f64::min),
            ObjectKind::Text(t) => {
                let c = t.corners();
                if rustfig_geom::point_in_polygon(p, &c) {
                    0.0
                } else {
                    rustfig_geom::dist_to_polyline(p, &c, true)
                }
            }
            ObjectKind::Polyline(pl) if pl.kind == PolyKind::Picture => {
                if rustfig_geom::point_in_polygon(p, &pl.points) {
                    0.0
                } else {
                    rustfig_geom::dist_to_polyline(p, &pl.points, true)
                }
            }
            _ => {
                let filled = self.style().map(|s| s.is_filled()).unwrap_or(false);
                let mut best = f64::INFINITY;
                for fp in crate::flatten_object(self) {
                    if filled && fp.closed && rustfig_geom::point_in_polygon(p, &fp.pts) {
                        return 0.0;
                    }
                    best = best.min(rustfig_geom::dist_to_polyline(p, &fp.pts, fp.closed));
                }
                best
            }
        }
    }
}

/// True for transforms that keep axis-aligned rectangles axis-aligned.
fn preserves_axes(t: &Affine) -> bool {
    (t.b.abs() < 1e-9 && t.c.abs() < 1e-9) || (t.a.abs() < 1e-9 && t.d.abs() < 1e-9)
}

/// Transform an ellipse given by radii and angle; returns new radii and angle
/// (via the singular value decomposition of the 2x2 linear part).
fn transform_ellipse(t: &Affine, rx: f64, ry: f64, angle: f64) -> (f64, f64, f64) {
    // M = L * R(angle) * diag(rx, ry), columns are the transformed semi-axes.
    let r = Affine::rotate(angle);
    let u = t.apply_vec(r.apply_vec(Point::new(rx, 0.0)));
    let v = t.apply_vec(r.apply_vec(Point::new(0.0, ry)));
    let (a, b, c, d) = (u.x, v.x, u.y, v.y);
    // SVD of [a b; c d].
    let e = (a + d) / 2.0;
    let f = (a - d) / 2.0;
    let g = (c + b) / 2.0;
    let h = (c - b) / 2.0;
    let q = (e * e + h * h).sqrt();
    let w = (f * f + g * g).sqrt();
    let sx = q + w;
    let sy = (q - w).abs();
    let a1 = g.atan2(f);
    let a2 = h.atan2(e);
    let theta = (a2 - a1) / 2.0;
    let phi = (a2 + a1) / 2.0;
    let _ = theta;
    // `phi` is the rotation of the major axis in y-down maths coordinates.
    let new_angle = -phi;
    if !(sx.is_finite() && sy.is_finite() && new_angle.is_finite()) {
        return (rx, ry, angle);
    }
    let na = new_angle.rem_euclid(std::f64::consts::PI);
    (sx, sy, if na.abs() < 1e-9 { 0.0 } else { na })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ellipse(rx: f64, ry: f64, angle: f64) -> Object {
        Object::new(ObjectKind::Ellipse(Ellipse {
            kind: EllipseKind::EllipseRadii,
            style: Style::default(),
            direction: 1,
            angle,
            center: Point::new(100.0, 100.0),
            radii: Point::new(rx, ry),
            start: Point::new(100.0, 100.0),
            end: Point::new(100.0 + rx, 100.0 + ry),
        }))
    }

    #[test]
    fn ellipse_scale_and_rotate() {
        let mut e = ellipse(200.0, 100.0, 0.0);
        e.transform(&Affine::scale(2.0, 1.0), false);
        let ObjectKind::Ellipse(el) = &e.kind else { panic!() };
        assert!((el.radii.x - 400.0).abs() < 1e-6 && (el.radii.y - 100.0).abs() < 1e-6);
        assert!(el.angle.abs() < 1e-9);

        let mut e = ellipse(200.0, 100.0, 0.0);
        let before = e.geometry_bbox();
        e.transform(
            &Affine::about(Affine::rotate(std::f64::consts::FRAC_PI_2), Point::new(100.0, 100.0)),
            false,
        );
        let after = e.geometry_bbox();
        assert!((before.width() - after.height()).abs() < 2.0);
        assert!((before.height() - after.width()).abs() < 2.0);
    }

    #[test]
    fn box_rotated_becomes_polygon() {
        let mut o = Object::new(ObjectKind::Polyline(Polyline::rect(
            Style::default(),
            Point::new(0.0, 0.0),
            Point::new(100.0, 50.0),
        )));
        o.transform(&Affine::rotate(std::f64::consts::FRAC_PI_2), false);
        let ObjectKind::Polyline(p) = &o.kind else { panic!() };
        assert_eq!(p.kind, PolyKind::Box);
        o.transform(&Affine::rotate(0.3), false);
        let ObjectKind::Polyline(p) = &o.kind else { panic!() };
        assert_eq!(p.kind, PolyKind::Polygon);
    }

    #[test]
    fn text_flip_keeps_readable() {
        let mut o = Object::new(ObjectKind::Text(Text {
            justify: Justify::Left,
            color: ColorRef::BLACK,
            depth: 50,
            pen_style: -1,
            font: 0,
            size: 12.0,
            angle: 0.0,
            flags: text_flags::POSTSCRIPT,
            height: 100.0,
            length: 300.0,
            origin: Point::new(100.0, 100.0),
            text: "hi".into(),
        }));
        o.transform(&Affine::scale(-1.0, 1.0), false);
        let ObjectKind::Text(t) = &o.kind else { panic!() };
        assert_eq!(t.justify, Justify::Right);
        assert!(t.angle.abs() < 1e-9);
        assert_eq!(t.origin, Point::new(-100.0, 100.0));
        o.transform(&Affine::scale(1.0, -1.0), false);
        let ObjectKind::Text(t) = &o.kind else { panic!() };
        assert_eq!(t.justify, Justify::Right);
        assert!(t.angle.abs() < 1e-9);
    }

    #[test]
    fn serde_roundtrip() {
        let o = ellipse(10.0, 20.0, 0.5);
        let s = serde_json::to_string(&o).unwrap();
        let back: Object = serde_json::from_str(&s).unwrap();
        assert_eq!(o, back);
    }

    #[test]
    fn depth_of_compound() {
        let mut a = ellipse(1.0, 1.0, 0.0);
        a.set_depth(30);
        let mut b = ellipse(1.0, 1.0, 0.0);
        b.set_depth(60);
        let mut c = Object::new(ObjectKind::Compound(Compound { objects: vec![a, b] }));
        assert_eq!(c.depth(), 30);
        c.set_depth(10);
        let ObjectKind::Compound(cc) = &c.kind else { panic!() };
        assert_eq!(cc.objects[0].depth(), 10);
        assert_eq!(cc.objects[1].depth(), 40);
    }
}
