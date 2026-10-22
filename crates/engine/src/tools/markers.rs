//! Object markers in the editing modes (plan W12).
//!
//! xfig shows small squares on every object the current mode can act on, so
//! one sees what to click: `u_markers.c` (`toggle_markers`,
//! `update_markers`, `toggle_*marker`) draws them for the objects that the
//! mode's mask `cur_objmask` lets through. The mask is the `objmask` column
//! of `mode_switches[]` in `w_modepanel.c` (`new_objmask`), with the `M_*`
//! bits of `object.h`; `set_mode` in `w_modepanel.c` and `update_markers` in
//! `w_indpanel.c` replace the mask of Rotate by `M_ROTATE_ANGLE` when the
//! rotation angle is not 90 or 180 degrees.
//!
//! | mode (xfig `F_*`)             | mask                                              |
//! |-------------------------------|---------------------------------------------------|
//! | drawing modes                 | none (`M_NONE`)                                   |
//! | Text                          | `M_TEXT_NORMAL` (not hidden text)                 |
//! | Glue, Move, Copy, Delete, Update, Edit | `M_ALL`                                  |
//! | Rotate (90, 180 degrees)      | `M_ALL`                                           |
//! | Rotate (other angles)         | `M_ROTATE_ANGLE`: lines, polygons, splines, arcs, text, compounds, ellipses (no boxes) |
//! | Break, Open compound, Align   | `M_COMPOUND`                                      |
//! | Join/Split                    | `M_POLYLINE \| M_SPLINE_O \| M_SPLINE_C`           |
//! | Chop, Trim                    | `M_POLYLINE \| M_ARC \| M_ELLIPSE` (+ splines, compounds: D15) |
//! | Scale, Move point, Flip (both) | `M_NO_TEXT`: everything but text                 |
//! | Add point, Delete point       | `M_VARPTS_OBJECT`: lines, polygons, splines       |
//! | Convert                       | `M_VARPTS_OBJECT \| M_POLYLINE_BOX`                |
//! | Add/delete arrowhead          | `M_OPEN_OBJECT`: lines, open splines, arcs        |
//! | Tangent                       | `M_TANGENT_OBJECT`: lines, polygons, splines, boxes, arcs, ellipses |
//! | Angle measure                 | `M_ANGLEMEAS_OBJECT`: lines, polygons, arcs       |
//! | Length measure                | `M_LENMEAS_OBJECT`: lines, polygons, boxes, arcs, ellipses |
//! | Area measure                  | `M_AREAMEAS_OBJECT`: polygons, boxes, arcs, ellipses |
//!
//! Markers per object (`toggle_*marker`): a box, line, polygon or spline has
//! one at each of its points; an ellipse at its two defining points (`start`,
//! `end`); an arc at its three points; a text at its anchor and one text height above it
//! (the second only when the text has a height of more than a marker); a
//! compound at the four corners of its box. Objects on a hidden depth get
//! none (`active_layer`), nor does a compound with nothing on a visible depth
//! (`any_active_in_compound`). Hidden text gets markers where the mask has
//! `M_TEXT_HIDDEN` (every mask with text but the Text mode's).
//
// Ported from xfig src/u_markers.c, src/w_modepanel.c and src/object.h
// (xfig licence, see LICENSE-xfig).

use super::{ToolKind, ToolState};
use crate::Engine;
use rustfig_geom::{Affine, Point, Rect};
use rustfig_model::{Object, ObjectKind, PolyKind};

/// xfig's `M_*` object mask, reduced to what the model distinguishes
/// (spline open/closed and kind are not told apart by any mode's mask).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ObjMask(u16);

impl ObjMask {
    pub const NONE: ObjMask = ObjMask(0);
    pub const POLYLINE_POLYGON: ObjMask = ObjMask(1);
    pub const POLYLINE_LINE: ObjMask = ObjMask(2);
    /// Boxes, rounded boxes and pictures.
    pub const POLYLINE_BOX: ObjMask = ObjMask(4);
    pub const SPLINE_OPEN: ObjMask = ObjMask(8);
    pub const SPLINE_CLOSED: ObjMask = ObjMask(16);
    pub const TEXT_NORMAL: ObjMask = ObjMask(32);
    pub const TEXT_HIDDEN: ObjMask = ObjMask(64);
    pub const ARC: ObjMask = ObjMask(128);
    pub const ELLIPSE: ObjMask = ObjMask(256);
    pub const COMPOUND: ObjMask = ObjMask(512);

    pub const TEXT: ObjMask = ObjMask(32 | 64);
    pub const SPLINE: ObjMask = ObjMask(8 | 16);
    pub const POLYLINE: ObjMask = ObjMask(1 | 2 | 4);
    pub const VARPTS_OBJECT: ObjMask = ObjMask(1 | 2 | 8 | 16);
    pub const OPEN_OBJECT: ObjMask = ObjMask(2 | 8 | 128);
    pub const ROTATE_ANGLE: ObjMask = ObjMask(1 | 2 | 8 | 16 | 128 | 32 | 64 | 512 | 256);
    pub const OBJECT: ObjMask = ObjMask(1 | 2 | 4 | 8 | 16 | 32 | 64 | 128 | 256);
    pub const NO_TEXT: ObjMask = ObjMask(1 | 2 | 4 | 8 | 16 | 128 | 256 | 512);
    pub const ALL: ObjMask = ObjMask(1 | 2 | 4 | 8 | 16 | 32 | 64 | 128 | 256 | 512);
    pub const ANGLEMEAS_OBJECT: ObjMask = ObjMask(1 | 2 | 128);
    pub const LENMEAS_OBJECT: ObjMask = ObjMask(1 | 2 | 4 | 128 | 256);
    pub const AREAMEAS_OBJECT: ObjMask = ObjMask(1 | 4 | 128 | 256);
    pub const TANGENT_OBJECT: ObjMask = ObjMask(1 | 2 | 8 | 16 | 4 | 128 | 256);

    pub const fn union(self, o: ObjMask) -> ObjMask {
        ObjMask(self.0 | o.0)
    }

    pub const fn has(self, o: ObjMask) -> bool {
        self.0 & o.0 != 0
    }

    /// Whether the mask lets this object through (`validline_in_mask`,
    /// `validspline_in_mask`, `validtext_in_mask`, `*_in_mask`).
    pub fn admits(self, o: &Object) -> bool {
        match &o.kind {
            ObjectKind::Ellipse(_) => self.has(Self::ELLIPSE),
            ObjectKind::Arc(_) => self.has(Self::ARC),
            ObjectKind::Compound(_) => self.has(Self::COMPOUND),
            ObjectKind::Text(t) => self.has(if t.is_hidden() {
                Self::TEXT_HIDDEN
            } else {
                Self::TEXT_NORMAL
            }),
            ObjectKind::Polyline(p) => self.has(match p.kind {
                PolyKind::Polyline => Self::POLYLINE_LINE,
                PolyKind::Polygon => Self::POLYLINE_POLYGON,
                PolyKind::Box | PolyKind::ArcBox | PolyKind::Picture => Self::POLYLINE_BOX,
            }),
            ObjectKind::Spline(s) => self.has(if s.closed {
                Self::SPLINE_CLOSED
            } else {
                Self::SPLINE_OPEN
            }),
        }
    }
}

/// The mask of a mode (`objmask` of `mode_switches[]`); `rotate_angle` is the
/// rotation step in degrees.
pub fn mode_mask(tool: ToolKind, rotate_angle: f64) -> ObjMask {
    use ToolKind as T;
    match tool {
        T::Text => ObjMask::TEXT_NORMAL,
        T::Glue | T::Move | T::Copy | T::Delete | T::Update | T::Edit => ObjMask::ALL,
        T::RotateCw | T::RotateCcw => {
            let a = rotate_angle.abs();
            if a == 90.0 || a == 180.0 {
                ObjMask::ALL
            } else {
                ObjMask::ROTATE_ANGLE
            }
        }
        T::Break | T::Align | T::OpenCompound => ObjMask::COMPOUND,
        T::JoinSplit => ObjMask::POLYLINE.union(ObjMask::SPLINE),
        // xfig's mask is polylines, arcs and ellipses; RustFig's Chop also
        // takes splines and compounds (plan D15).
        T::Chop | T::Trim => ObjMask::POLYLINE
            .union(ObjMask::ARC)
            .union(ObjMask::ELLIPSE)
            .union(ObjMask::SPLINE)
            .union(ObjMask::COMPOUND),
        T::Scale | T::MovePoint | T::FlipV | T::FlipH => ObjMask::NO_TEXT,
        T::AddPoint | T::DeletePoint => ObjMask::VARPTS_OBJECT,
        T::Convert => ObjMask::VARPTS_OBJECT.union(ObjMask::POLYLINE_BOX),
        T::Arrow => ObjMask::OPEN_OBJECT,
        T::Tangent => ObjMask::TANGENT_OBJECT,
        T::AngleMeasure => ObjMask::ANGLEMEAS_OBJECT,
        T::LengthMeasure => ObjMask::LENMEAS_OBJECT,
        T::AreaMeasure => ObjMask::AREAMEAS_OBJECT,
        // RustFig's own mode: lines, polygons and splines open and close.
        T::OpenClose => ObjMask::VARPTS_OBJECT,
        // Drawing modes, the pick tool and "no mode yet".
        _ => ObjMask::NONE,
    }
}

/// At most this many objects get markers (a view full of tiny objects).
const MAX_MARKED: usize = 5000;
/// At most this many markers in all.
const MAX_MARKERS: usize = 40_000;

/// The markers of one object (`toggle_*marker`).
pub fn object_markers(o: &Object, bbox: &Rect, out: &mut Vec<Point>) {
    match &o.kind {
        ObjectKind::Ellipse(e) => out.extend([e.start, e.end]),
        ObjectKind::Arc(a) => out.extend(a.points),
        ObjectKind::Polyline(p) => out.extend(p.points.iter().copied()),
        ObjectKind::Spline(s) => out.extend(s.points.iter().copied()),
        ObjectKind::Text(t) => {
            out.push(t.origin);
            // The second marker sits one text height above the anchor (along
            // the text's up direction), as seen in xfig 3.2.9a; skipped when it
            // would lie on the first (a string of spaces has no height).
            let h = if t.height.is_finite() { t.height.max(0.0) } else { 0.0 };
            if h > 5.0 * 15.0 && !t.text.trim().is_empty() {
                let rot = Affine::rotate(if t.angle.is_finite() { t.angle } else { 0.0 });
                out.push(t.origin + rot.apply_vec(Point::new(0.0, -h)));
            }
        }
        ObjectKind::Compound(_) => {
            let (a, b) = (bbox.min, bbox.max);
            out.extend([a, Point::new(b.x, a.y), b, Point::new(a.x, b.y)]);
        }
    }
}

impl Engine {
    /// The object mask of the active tool (see the table above).
    pub fn marker_mask(&self) -> ObjMask {
        // Join/Split, after the first end point: only what can be joined
        // to it (`join_lines`, `join_splines`: `update_markers`).
        if let ToolState::Join { spline, .. } = self.tool_state {
            return if spline {
                ObjMask::SPLINE_OPEN
            } else {
                ObjMask::POLYLINE_LINE
            };
        }
        mode_mask(self.tool(), self.indicator.rotate_angle)
    }

    /// Whether the object gets markers in the active mode: the mask admits it
    /// and it is on a visible depth.
    pub fn has_marker(&self, o: &Object) -> bool {
        self.marker_mask().admits(o) && self.is_visible(o)
    }

    /// Markers of the objects the active mode can act on, culled to the
    /// visible part of the canvas. Empty while a tool is busy (xfig clears
    /// them, `update_markers(0)`, while an object is being dragged).
    pub fn mode_markers(&self) -> Vec<Point> {
        self.mode_markers_tagged().0
    }

    /// [`Engine::mode_markers`], and separately the markers of the tagged
    /// (selected) objects among them, which xfig fills
    /// (`toggle_*highlight`, drawn when `tagged` is set).
    pub fn mode_markers_tagged(&self) -> (Vec<Point>, Vec<Point>) {
        let mask = self.marker_mask();
        let mut out = Vec::new();
        let mut tagged = Vec::new();
        // Picking states (Join's second end, Chop's axes and log, Align's
        // reference) keep the markers: the next click needs them.
        let picking = self.joining() || matches!(self.tool_state, ToolState::Chop { .. } | ToolState::AlignTo { .. });
        if mask == ObjMask::NONE || (self.tool_busy() && !picking) {
            return (out, tagged);
        }
        let selected: std::collections::HashSet<_> = self.selection().iter().copied().collect();
        let view = self.view.visible();
        let cull = !view.is_empty() && self.view.size.0 > 0.0 && self.view.size.1 > 0.0;
        let mut marked = 0usize;
        for (o, bbox, _) in self.object_bboxes().iter() {
            if cull
                && (bbox.max.x < view.min.x
                    || bbox.min.x > view.max.x
                    || bbox.max.y < view.min.y
                    || bbox.min.y > view.max.y)
            {
                continue;
            }
            if !mask.admits(o) || !self.is_visible(o) {
                continue;
            }
            let first = out.len();
            object_markers(o, bbox, &mut out);
            if selected.contains(&o.id) {
                tagged.extend(out.get(first..).unwrap_or_default());
            }
            marked += 1;
            if marked >= MAX_MARKED || out.len() >= MAX_MARKERS {
                break;
            }
        }
        out.retain(|p| p.is_finite());
        tagged.retain(|p| p.is_finite());
        (out, tagged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::shape::{make_ellipse, make_text};
    use rustfig_model::{text_flags, Arc, Compound, EllipseKind, PolyKind, Polyline, Spline, SplineKind, Style};

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    /// One object of every kind the masks tell apart, on a fresh engine.
    /// Returns the engine and the names in document order.
    fn sample() -> (Engine, Vec<&'static str>) {
        let mut e = Engine::new();
        let st = Style::default();
        let pts = vec![p(0.0, 0.0), p(900.0, 0.0), p(900.0, 600.0)];
        let mut names = Vec::new();
        let mut add = |e: &mut Engine, n: &'static str, o: Object| {
            e.add_object(o);
            names.push(n);
        };
        let line =
            |k: PolyKind, pts: Vec<Point>| Object::new(ObjectKind::Polyline(Polyline::new(k, Style::default(), pts)));
        add(&mut e, "line", line(PolyKind::Polyline, pts.clone()));
        add(&mut e, "polygon", line(PolyKind::Polygon, pts.clone()));
        add(
            &mut e,
            "box",
            line(
                PolyKind::Box,
                Rect::new(p(0.0, 0.0), p(300.0, 300.0)).corners().to_vec(),
            ),
        );
        add(
            &mut e,
            "arcbox",
            line(
                PolyKind::ArcBox,
                Rect::new(p(0.0, 0.0), p(300.0, 300.0)).corners().to_vec(),
            ),
        );
        add(
            &mut e,
            "picture",
            line(
                PolyKind::Picture,
                Rect::new(p(0.0, 0.0), p(300.0, 300.0)).corners().to_vec(),
            ),
        );
        let sp = |k, c| Object::new(ObjectKind::Spline(Spline::new(k, c, Style::default(), pts.clone())));
        add(&mut e, "spline_open", sp(SplineKind::Approximated, false));
        add(&mut e, "spline_closed", sp(SplineKind::Interpolated, true));
        add(
            &mut e,
            "ellipse",
            make_ellipse(st, p(600.0, 600.0), 300.0, 200.0, 0.0, EllipseKind::EllipseRadii),
        );
        if let Some(a) = Arc::through(st, p(0.0, 0.0), p(300.0, -300.0), p(600.0, 0.0), false) {
            add(&mut e, "arc", Object::new(ObjectKind::Arc(a)));
        }
        let mut t = make_text(&e, p(100.0, 900.0), "hello world".into());
        let mut hidden = t.clone();
        hidden.flags |= text_flags::HIDDEN;
        t.flags &= !text_flags::HIDDEN;
        add(&mut e, "text", Object::new(ObjectKind::Text(t)));
        add(&mut e, "text_hidden", Object::new(ObjectKind::Text(hidden)));
        let inner = line(PolyKind::Polyline, pts);
        add(
            &mut e,
            "compound",
            Object::new(ObjectKind::Compound(Compound { objects: vec![inner] })),
        );
        (e, names)
    }

    fn marked(tool: ToolKind, angle: f64) -> Vec<&'static str> {
        let (mut e, names) = sample();
        e.indicator.rotate_angle = angle;
        e.set_tool(tool);
        let objs = e.doc().objects.clone();
        names
            .into_iter()
            .zip(objs)
            .filter(|(_, o)| e.has_marker(o))
            .map(|(n, _)| n)
            .collect()
    }

    const ALL: [&str; 12] = [
        "line",
        "polygon",
        "box",
        "arcbox",
        "picture",
        "spline_open",
        "spline_closed",
        "ellipse",
        "arc",
        "text",
        "text_hidden",
        "compound",
    ];

    #[test]
    fn table_mode_to_objects() {
        use ToolKind as T;
        let no_text: Vec<&str> = ALL.iter().copied().filter(|n| !n.starts_with("text")).collect();
        let varpts = vec!["line", "polygon", "spline_open", "spline_closed"];
        let boxes = ["box", "arcbox", "picture"];
        let cases: Vec<(ToolKind, Vec<&str>)> = vec![
            (T::None, vec![]),
            (T::Box, vec![]),
            (T::Polyline, vec![]),
            (T::Text, vec!["text"]),
            (T::Glue, ALL.to_vec()),
            (T::Move, ALL.to_vec()),
            (T::Copy, ALL.to_vec()),
            (T::Delete, ALL.to_vec()),
            (T::Update, ALL.to_vec()),
            (T::Edit, ALL.to_vec()),
            (T::Break, vec!["compound"]),
            (T::OpenCompound, vec!["compound"]),
            (
                T::JoinSplit,
                vec![
                    "line",
                    "polygon",
                    "box",
                    "arcbox",
                    "picture",
                    "spline_open",
                    "spline_closed",
                ],
            ),
            (
                T::Chop,
                vec![
                    "line",
                    "polygon",
                    "box",
                    "arcbox",
                    "picture",
                    "spline_open",
                    "spline_closed",
                    "ellipse",
                    "arc",
                    "compound",
                ],
            ),
            (T::Align, vec!["compound"]),
            (T::Scale, no_text.clone()),
            (T::MovePoint, no_text.clone()),
            (T::FlipV, no_text.clone()),
            (T::FlipH, no_text),
            (T::AddPoint, varpts.clone()),
            (T::DeletePoint, varpts.clone()),
            (
                T::Convert,
                ALL.iter()
                    .copied()
                    .filter(|n| varpts.contains(n) || boxes.contains(n))
                    .collect(),
            ),
            (T::Arrow, vec!["line", "spline_open", "arc"]),
        ];
        for (tool, want) in cases {
            assert_eq!(marked(tool, 90.0), want, "{tool:?}");
        }
        assert_eq!(marked(T::Trim, 90.0), marked(T::Chop, 90.0));
        // Rotate: every object at 90/180 degrees, no boxes otherwise.
        for t in [T::RotateCw, T::RotateCcw] {
            assert_eq!(marked(t, 90.0), ALL.to_vec());
            assert_eq!(marked(t, -180.0), ALL.to_vec());
            let other: Vec<&str> = ALL.iter().copied().filter(|n| !boxes.contains(n)).collect();
            assert_eq!(marked(t, 45.0), other);
        }
    }

    #[test]
    fn markers_per_object() {
        let (mut e, names) = sample();
        e.set_tool(ToolKind::Move);
        let objs = e.doc().objects.clone();
        let count = |n: &str| {
            let i = names.iter().position(|x| *x == n).unwrap();
            let mut v = Vec::new();
            object_markers(&objs[i], &objs[i].approx_bbox(), &mut v);
            v.len()
        };
        assert_eq!(count("line"), 3);
        assert_eq!(count("polygon"), 3);
        assert_eq!(count("box"), 4);
        assert_eq!(count("spline_open"), 3);
        assert_eq!(count("ellipse"), 2);
        assert_eq!(count("arc"), 3);
        assert_eq!(count("text"), 2);
        assert_eq!(count("compound"), 4);
        let total: usize = names.iter().map(|n| count(n)).sum();
        assert_eq!(e.mode_markers().len(), total);
    }

    #[test]
    fn hidden_depth_has_no_markers_and_busy_tool_none() {
        let (mut e, _) = sample();
        e.set_tool(ToolKind::Delete);
        let n = e.mode_markers().len();
        assert!(n > 0);
        e.hidden_depths.insert(50);
        assert!(e.mode_markers().is_empty());
        assert!(e.overlay().markers.is_empty());
        e.hidden_depths.clear();
        assert_eq!(e.overlay().markers.len(), n);
        // Drawing modes show none.
        e.set_tool(ToolKind::Box);
        assert!(e.overlay().markers.is_empty());
    }

    #[test]
    fn culled_to_the_view() {
        let (mut e, _) = sample();
        e.set_tool(ToolKind::Move);
        e.view.size = (100.0, 100.0);
        e.view.origin = p(1.0e6, 1.0e6);
        assert!(e.mode_markers().is_empty());
        e.view.origin = p(-100.0, -100.0);
        e.view.size = (4000.0, 4000.0);
        assert!(!e.mode_markers().is_empty());
    }
}
