//! The current drawing attributes (xfig's indicator panel) and grid/snap modes.

use rustfig_geom::Point;
use rustfig_model::{
    Arrow, CapStyle, ColorRef, JoinStyle, Justify, LineKind, Object, ObjectKind, PolyKind, Style, Units,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The attribute buttons that carry an update flag in Update mode (xfig's
/// `I_UPDATEMASK`, `w_indpanel.h`; `cur_updatemask` in `mode.c`). Only the
/// attributes whose flag is on are copied by Update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateFlag {
    TextJust,
    Font,
    FontSize,
    PenColor,
    FillColor,
    FillStyle,
    LineStyle,
    JoinStyle,
    CapStyle,
    LineWidth,
    Depth,
    TextAngle,
    TextFlags,
    ArrowMode,
    ArrowType,
    ArrowSize,
    BoxRadius,
    ArcType,
}

impl UpdateFlag {
    pub const ALL: [UpdateFlag; 18] = [
        UpdateFlag::TextJust,
        UpdateFlag::Font,
        UpdateFlag::FontSize,
        UpdateFlag::PenColor,
        UpdateFlag::FillColor,
        UpdateFlag::FillStyle,
        UpdateFlag::LineStyle,
        UpdateFlag::JoinStyle,
        UpdateFlag::CapStyle,
        UpdateFlag::LineWidth,
        UpdateFlag::Depth,
        UpdateFlag::TextAngle,
        UpdateFlag::TextFlags,
        UpdateFlag::ArrowMode,
        UpdateFlag::ArrowType,
        UpdateFlag::ArrowSize,
        UpdateFlag::BoxRadius,
        UpdateFlag::ArcType,
    ];

    /// The flag's name as commands take it (`"line_width"`).
    pub fn name(self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default()
    }

    pub fn from_name(n: &str) -> Option<UpdateFlag> {
        UpdateFlag::ALL.iter().copied().find(|f| f.name() == n)
    }
}

/// Grid spacing / point positioning steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SnapMode {
    /// Free positioning.
    Any,
    /// 1/16 inch or 1 mm.
    Fine,
    /// 1/8 inch or 2 mm.
    #[default]
    Small,
    /// 1/4 inch or 5 mm.
    Medium,
    /// 1/2 inch or 1 cm.
    Large,
    /// 1 inch or 2 cm.
    Coarse,
}

impl SnapMode {
    pub const ALL: [SnapMode; 6] = [
        SnapMode::Any,
        SnapMode::Fine,
        SnapMode::Small,
        SnapMode::Medium,
        SnapMode::Large,
        SnapMode::Coarse,
    ];

    /// Step in Fig units (0 for free positioning).
    pub fn step(self, units: Units) -> f64 {
        let (inch, metric) = match self {
            SnapMode::Any => (0.0, 0.0),
            SnapMode::Fine => (75.0, 45.0),
            SnapMode::Small => (150.0, 90.0),
            SnapMode::Medium => (300.0, 225.0),
            SnapMode::Large => (600.0, 450.0),
            SnapMode::Coarse => (1200.0, 900.0),
        };
        match units {
            Units::Inches => inch,
            Units::Metric => metric,
        }
    }

    pub fn label(self, units: Units) -> &'static str {
        match (self, units) {
            (SnapMode::Any, _) => "any",
            (SnapMode::Fine, Units::Inches) => "1/16 in",
            (SnapMode::Small, Units::Inches) => "1/8 in",
            (SnapMode::Medium, Units::Inches) => "1/4 in",
            (SnapMode::Large, Units::Inches) => "1/2 in",
            (SnapMode::Coarse, Units::Inches) => "1 in",
            (SnapMode::Fine, Units::Metric) => "1 mm",
            (SnapMode::Small, Units::Metric) => "2 mm",
            (SnapMode::Medium, Units::Metric) => "5 mm",
            (SnapMode::Large, Units::Metric) => "1 cm",
            (SnapMode::Coarse, Units::Metric) => "2 cm",
        }
    }
}

/// Visible grid (None hides it).
pub type GridMode = Option<SnapMode>;

/// Smart links (xfig's `cur_linkmode`): whether the end points of lines
/// attached to a moved shape follow it, and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LinkMode {
    /// Do not adjust links.
    #[default]
    Off,
    /// Move the attached end point with the box.
    Move,
    /// Move the attached end point and slide the point before it so the
    /// last segment stays axis-aligned (xfig's SMART_SLIDE).
    Slide,
}

impl LinkMode {
    pub const ALL: [LinkMode; 3] = [LinkMode::Off, LinkMode::Move, LinkMode::Slide];
}

/// Current attributes applied to new objects, and drawing settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Indicator {
    pub line: LineKind,
    pub thickness: i32,
    pub pen_color: i32,
    pub fill_color: i32,
    /// -1 no fill, 0–40 shade/tint, 41–62 pattern.
    pub area_fill: i32,
    pub depth: i32,
    /// Dash length in 1/80 inch (xfig's `cur_dashlength`).
    pub style_val: f64,
    /// Gap between dots of dotted lines in 1/80 inch (xfig's `cur_dotgap`).
    pub dot_gap: f64,
    pub cap: CapStyle,
    pub join: JoinStyle,
    pub forward_arrow: bool,
    pub backward_arrow: bool,
    pub arrow: Arrow,
    pub font: i32,
    pub font_size: f64,
    pub postscript: bool,
    pub justify: Justify,
    /// Text angle in degrees.
    pub text_angle: f64,
    pub hidden_text: bool,
    pub special_text: bool,
    pub rigid_text: bool,
    /// Arc-box corner radius in 1/80 inch.
    pub box_radius: i32,
    /// Regular polygon sides.
    pub sides: usize,
    /// Rotation step in degrees for the rotate tool.
    pub rotate_angle: f64,
    /// Rotated copies made by one copy & rotate click (xfig `cur_numcopies`, 1..=99).
    pub num_copies: u32,
    /// Dimension line settings (xfig's `cur_dimline_*`).
    pub dimline: crate::dimline::DimlineSettings,
    /// Array copy: columns and rows (xfig `cur_numxcopies`/`cur_numycopies`, 0 = off, 1..=99).
    pub num_x_copies: u32,
    pub num_y_copies: u32,
    pub grid: Option<SnapMode>,
    pub snap: SnapMode,
    /// Align modes for the align command: "none", "left", "center", "right" / "top", "center", "bottom".
    pub align_h: String,
    pub align_v: String,
    /// Arcs are pie wedges rather than open (xfig's "Arc Type").
    pub pie_arc: bool,
    /// Line spacing of the text tool as a multiple of the font's (xfig's
    /// "Text Step").
    pub text_step: f64,
    /// Length of the line the Tangent mode adds, in inches or cm (xfig's
    /// `cur_tangnormlen`, "Tangent/Norm Length": 0 to 20, start 1.0).
    pub tangent_length: f64,
    /// Update flags that are switched off (xfig's `cur_updatemask`; all on
    /// at the start, so the set is empty).
    pub update_off: BTreeSet<UpdateFlag>,
    /// Smart links: line end points attached to a moved shape follow it
    /// (D14: through every geometric operation).
    pub linkmode: LinkMode,
    /// Angle geometry: constrains the next point of lines and splines
    /// (xfig's `cur_anglegeom`).
    pub angle_geom: crate::AngleGeom,
}

impl Default for Indicator {
    fn default() -> Self {
        Indicator {
            line: LineKind::Solid,
            thickness: 1,
            pen_color: 0,
            fill_color: 7,
            area_fill: -1,
            depth: 50,
            // xfig's start values (mode.c, main.c's resource defaults).
            style_val: 4.0,
            dot_gap: 3.0,
            cap: CapStyle::Butt,
            join: JoinStyle::Miter,
            forward_arrow: false,
            backward_arrow: false,
            // xfig's startarrowtype 0: the stick arrow.
            arrow: Arrow {
                kind: 0,
                style: 0,
                ..Arrow::default()
            },
            font: 0,
            font_size: 12.0,
            postscript: true,
            justify: Justify::Left,
            text_angle: 0.0,
            hidden_text: false,
            special_text: false,
            rigid_text: false,
            box_radius: 7,
            sides: 6,
            rotate_angle: 90.0,
            num_copies: 1,
            dimline: crate::dimline::DimlineSettings::default(),
            num_x_copies: 0,
            num_y_copies: 0,
            grid: None,
            // xfig's startposnmode 1: magnet (1/16 inch, 1 mm).
            snap: SnapMode::Fine,
            align_h: "none".into(),
            align_v: "none".into(),
            pie_arc: false,
            text_step: 1.0,
            tangent_length: 1.0,
            update_off: BTreeSet::new(),
            // xfig's `cur_linkmode` starts SMART_OFF (mode.c).
            linkmode: LinkMode::Off,
            // xfig's `cur_anglegeom` starts L_UNCONSTRAINED.
            angle_geom: crate::AngleGeom::Unconstrained,
        }
    }
}

impl Indicator {
    /// The style for a new object.
    pub fn style(&self) -> Style {
        Style {
            line: self.line,
            thickness: self.thickness.clamp(0, 10_000),
            pen_color: ColorRef(self.pen_color),
            fill_color: ColorRef(self.fill_color),
            depth: self.depth.clamp(0, rustfig_model::MAX_DEPTH),
            pen_style: -1,
            area_fill: self.area_fill.clamp(-1, 62),
            style_val: match self.line {
                LineKind::Solid | LineKind::Default => 0.0,
                LineKind::Dotted => self.dot_gap,
                _ => self.style_val,
            },
            cap: self.cap,
            join: self.join,
        }
    }

    /// The arrow the indicator describes (type, style and size).
    pub fn arrow_value(&self) -> Arrow {
        Arrow {
            thickness: f64::from(self.thickness.max(1)),
            ..self.arrow
        }
    }

    pub fn arrows(&self) -> (Option<Arrow>, Option<Arrow>) {
        let a = self.arrow_value();
        (self.forward_arrow.then_some(a), self.backward_arrow.then_some(a))
    }

    /// Whether Update copies this attribute.
    pub fn updates(&self, f: UpdateFlag) -> bool {
        !self.update_off.contains(&f)
    }

    /// Switch one update flag.
    pub fn set_update_flag(&mut self, f: UpdateFlag, on: bool) {
        if on {
            self.update_off.remove(&f);
        } else {
            self.update_off.insert(f);
        }
    }

    /// An arrow slot after Update: the arrow mode decides whether there is
    /// an arrow, the type and size flags which parts of an existing one
    /// change (xfig's `up_arrow`).
    pub fn updated_arrow(&self, old: Option<Arrow>, present: bool) -> Option<Arrow> {
        let new = self.arrow_value();
        let mut a = match old {
            Some(a) => a,
            None if self.updates(UpdateFlag::ArrowMode) && present => return Some(new),
            None => return None,
        };
        if self.updates(UpdateFlag::ArrowMode) && !present {
            return None;
        }
        if self.updates(UpdateFlag::ArrowType) {
            a.kind = new.kind;
            a.style = new.style;
        }
        if self.updates(UpdateFlag::ArrowSize) {
            a.thickness = new.thickness;
            a.width = new.width;
            a.height = new.height;
        }
        Some(a)
    }

    /// Update an object's attributes from the indicator, the flags deciding
    /// which (xfig's `init_update_object`).
    pub fn update_object(&self, o: &mut Object) {
        use rustfig_model::text_flags::POSTSCRIPT;
        use UpdateFlag as F;
        let style = self.style();
        if let ObjectKind::Text(t) = &mut o.kind {
            if self.updates(F::PenColor) {
                t.color = ColorRef(self.pen_color);
            }
            if self.updates(F::Depth) {
                t.depth = style.depth;
            }
            if self.updates(F::Font) {
                t.font = self.font;
            }
            if self.updates(F::FontSize) {
                t.size = self.font_size.clamp(1.0, 2000.0);
            }
            if self.updates(F::TextJust) {
                t.justify = self.justify;
            }
            if self.updates(F::TextAngle) && self.text_angle.is_finite() {
                t.angle = self.text_angle.to_radians();
            }
            // The PostScript/LaTeX flag goes with the font, the others with
            // the flags (xfig: "use I_FONT instead of I_TEXTFLAGS").
            let new = self.text_flags();
            let mut f = t.flags;
            if self.updates(F::TextFlags) {
                f = (f & POSTSCRIPT) | (new & !POSTSCRIPT);
            }
            if self.updates(F::Font) {
                f = (f & !POSTSCRIPT) | (new & POSTSCRIPT);
            }
            t.flags = f;
            return;
        }
        if let ObjectKind::Ellipse(el) = &mut o.kind {
            if self.updates(F::TextAngle) && self.text_angle.is_finite() {
                el.angle = self.text_angle.to_radians();
            }
        }
        if let Some(s) = o.style_mut() {
            if self.updates(F::LineStyle) {
                s.line = style.line;
                s.style_val = style.style_val;
            }
            if self.updates(F::LineWidth) {
                s.thickness = style.thickness;
            }
            if self.updates(F::PenColor) {
                s.pen_color = style.pen_color;
            }
            if self.updates(F::FillColor) {
                s.fill_color = style.fill_color;
            }
            if self.updates(F::FillStyle) {
                s.area_fill = style.area_fill;
            }
            if self.updates(F::Depth) {
                s.depth = style.depth;
            }
            if self.updates(F::CapStyle) {
                s.cap = style.cap;
            }
            if self.updates(F::JoinStyle) {
                s.join = style.join;
            }
        }
        if let ObjectKind::Polyline(pl) = &mut o.kind {
            if pl.kind == PolyKind::ArcBox && self.updates(F::BoxRadius) {
                pl.radius = self.box_radius;
            }
        }
        if let ObjectKind::Arc(a) = &mut o.kind {
            if self.updates(F::ArcType) {
                a.pie = self.pie_arc;
            }
            // xfig's `update_arc`: a pie-wedge arc has no arrowheads.
            if a.pie {
                a.forward_arrow = None;
                a.backward_arrow = None;
            }
        }
        if let Some((f, b)) = o.arrows_mut() {
            *f = self.updated_arrow(*f, self.forward_arrow);
            *b = self.updated_arrow(*b, self.backward_arrow);
        }
    }

    /// Copy an object's attributes into the indicator (xfig's Update mode,
    /// middle button: "update settings"). The reverse of `object.update`.
    pub fn take_from(&mut self, o: &Object) {
        use UpdateFlag as F;
        let on = |f: F| !self.update_off.contains(&f);
        let (j, p, fc, fs, ls, jn, cp, lw, dp, ta, tf, am, at, asz, br, atp, fnt, fsz) = (
            on(F::TextJust),
            on(F::PenColor),
            on(F::FillColor),
            on(F::FillStyle),
            on(F::LineStyle),
            on(F::JoinStyle),
            on(F::CapStyle),
            on(F::LineWidth),
            on(F::Depth),
            on(F::TextAngle),
            on(F::TextFlags),
            on(F::ArrowMode),
            on(F::ArrowType),
            on(F::ArrowSize),
            on(F::BoxRadius),
            on(F::ArcType),
            on(F::Font),
            on(F::FontSize),
        );
        if let ObjectKind::Text(t) = &o.kind {
            use rustfig_model::text_flags::*;
            if p {
                self.pen_color = t.color.0;
            }
            if dp {
                self.depth = t.depth;
            }
            if fnt {
                self.font = t.font;
                self.postscript = t.flags & POSTSCRIPT != 0;
            }
            if fsz {
                self.font_size = t.size;
            }
            if j {
                self.justify = t.justify;
            }
            if ta {
                self.text_angle = t.angle.to_degrees();
            }
            if tf {
                self.rigid_text = t.flags & RIGID != 0;
                self.special_text = t.flags & SPECIAL != 0;
                self.hidden_text = t.flags & HIDDEN != 0;
            }
            return;
        }
        if let (ObjectKind::Ellipse(el), true) = (&o.kind, ta) {
            self.text_angle = el.angle.to_degrees();
        }
        if let Some(s) = o.style() {
            if ls {
                self.line = s.line;
                if s.style_val > 0.0 {
                    if s.line == LineKind::Dotted {
                        self.dot_gap = s.style_val;
                    } else {
                        self.style_val = s.style_val;
                    }
                }
            }
            if lw {
                self.thickness = s.thickness;
            }
            if p {
                self.pen_color = s.pen_color.0;
            }
            if fc {
                self.fill_color = s.fill_color.0;
            }
            if dp {
                self.depth = s.depth;
            }
            if fs {
                self.area_fill = s.area_fill;
            }
            if cp {
                self.cap = s.cap;
            }
            if jn {
                self.join = s.join;
            }
        }
        if let Some((f, b)) = o.arrows() {
            if am {
                self.forward_arrow = f.is_some();
                self.backward_arrow = b.is_some();
            }
            if let Some(a) = f.or(b) {
                if at {
                    self.arrow.kind = a.kind;
                    self.arrow.style = a.style;
                }
                if asz {
                    self.arrow = Arrow {
                        kind: self.arrow.kind,
                        style: self.arrow.style,
                        ..a
                    };
                }
            }
        }
        if let (ObjectKind::Polyline(p), true) = (&o.kind, br) {
            if p.kind == PolyKind::ArcBox {
                self.box_radius = p.radius;
            }
        }
        if let (ObjectKind::Arc(a), true) = (&o.kind, atp) {
            self.pie_arc = a.pie;
        }
    }

    pub fn text_flags(&self) -> i32 {
        use rustfig_model::text_flags::*;
        let mut f = 0;
        if self.rigid_text {
            f |= RIGID;
        }
        if self.special_text {
            f |= SPECIAL;
        }
        if self.postscript {
            f |= POSTSCRIPT;
        }
        if self.hidden_text {
            f |= HIDDEN;
        }
        f
    }

    /// The point positioning step as an integer, the "magnet" of the LaTeX
    /// angle modes (xfig: 1 when positioning is free, else
    /// `max(1, point_spacing())`).
    pub fn magnet(&self, units: Units) -> i64 {
        let step = self.snap.step(units);
        if step.is_finite() && step >= 1.0 {
            step as i64
        } else {
            1
        }
    }

    /// Snap a point to the positioning grid.
    pub fn snap_point(&self, p: Point, units: Units) -> Point {
        let step = self.snap.step(units);
        if step <= 0.0 || !p.is_finite() {
            return p;
        }
        Point::new((p.x / step).round() * step, (p.y / step).round() * step)
    }
}

/// xfig's arrow type choices (`arrowtype_choices` in `w_indpanel.c`): 0 is
/// the stick arrow, then each type 1–14 hollow and filled. Returns (kind,
/// style); the `startarrowtype` resource is such a choice.
pub fn arrow_choice(n: i32) -> (i32, i32) {
    if n <= 0 {
        (0, 0)
    } else {
        let n = n.min(28);
        ((n + 1) / 2, if n % 2 == 1 { 0 } else { 1 })
    }
}

/// The choice of arrow `kind` and `style` (the inverse of [`arrow_choice`]).
pub fn arrow_choice_of(kind: i32, style: i32) -> i32 {
    if kind <= 0 {
        0
    } else {
        kind * 2 - 1 + i32::from(style == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping() {
        // xfig's default: magnet, 1/16 inch.
        let ind = Indicator::default();
        assert_eq!(ind.snap, SnapMode::Fine);
        let p = ind.snap_point(Point::new(160.0, 220.0), Units::Inches);
        assert_eq!(p, Point::new(150.0, 225.0));
        let eighth = Indicator {
            snap: SnapMode::Small,
            ..Indicator::default()
        };
        let p = eighth.snap_point(Point::new(160.0, 220.0), Units::Inches);
        assert_eq!(p, Point::new(150.0, 150.0));
        let any = Indicator {
            snap: SnapMode::Any,
            ..Indicator::default()
        };
        assert_eq!(
            any.snap_point(Point::new(1.5, 2.5), Units::Inches),
            Point::new(1.5, 2.5)
        );
    }

    /// The start values are xfig's (`mode.c`, `main.c`'s resource defaults,
    /// `object.h`): gap 23.
    #[test]
    fn start_values_are_xfigs() {
        let i = Indicator::default();
        assert_eq!(i.snap, SnapMode::Fine, "startposnmode 1 (magnet)");
        assert_eq!(i.grid, None, "startgridmode 0");
        assert_eq!((i.arrow.kind, i.arrow.style), (0, 0), "startarrowtype 0");
        assert_eq!((i.arrow.width, i.arrow.height), (60.0, 120.0), "DEF_ARROW_WID/HT 4, 8");
        assert_eq!((i.style_val, i.dot_gap), (4.0, 3.0), "DEF_DASHLENGTH, DEF_DOTGAP");
        assert_eq!(i.text_step, 1.0, "cur_textstep (the man page's 1.2 is wrong)");
        assert_eq!((i.font_size, i.thickness, i.depth, i.box_radius), (12.0, 1, 50, 7));
        assert_eq!((i.area_fill, i.pen_color, i.fill_color), (-1, 0, 7));
        assert_eq!((i.align_h.as_str(), i.align_v.as_str()), ("none", "none"));
        assert_eq!((i.sides, i.rotate_angle), (6, 90.0));
        assert_eq!(i.linkmode, LinkMode::Off, "cur_linkmode 0 (SMART_OFF)");
        assert_eq!(
            i.angle_geom,
            crate::AngleGeom::Unconstrained,
            "cur_anglegeom L_UNCONSTRAINED"
        );
    }

    #[test]
    fn dotted_lines_use_the_dot_gap() {
        let i = Indicator {
            line: LineKind::Dotted,
            ..Indicator::default()
        };
        assert_eq!(i.style().style_val, 3.0);
        let d = Indicator {
            line: LineKind::Dashed,
            ..Indicator::default()
        };
        assert_eq!(d.style().style_val, 4.0);
    }

    #[test]
    fn json_partial_update() {
        let v = serde_json::json!({"thickness": 3, "line": "dashed"});
        let ind: Indicator = serde_json::from_value(v).unwrap();
        assert_eq!(ind.thickness, 3);
        assert_eq!(ind.line, LineKind::Dashed);
        assert_eq!(ind.font_size, 12.0);
    }
}
