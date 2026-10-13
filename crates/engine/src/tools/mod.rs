//! Interactive tools (xfig's drawing and editing modes).
//!
//! A tool receives pointer and key events in Fig coordinates, keeps its
//! in-progress state (the rubber band) and commits its result by running a
//! command, so every edit is undoable and scriptable.

use crate::cmd::shape::make_ellipse;
use crate::{Engine, EngineError, UiRequest};
use rustfig_geom::{dist_to_segment, regular_polygon, Affine, Point, Rect};
use rustfig_model::{Arc, EllipseKind, Justify, Object, ObjectId, ObjectKind, PolyKind, Polyline, Spline, SplineKind};
use serde_json::json;

/// The tools, in mode-panel order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolKind {
    Select,
    CircleRadius,
    CircleDiameter,
    EllipseRadii,
    EllipseDiameter,
    SplineApproxClosed,
    SplineApprox,
    SplineInterpClosed,
    SplineInterp,
    XSplineClosed,
    XSpline,
    Polygon,
    Polyline,
    Box,
    ArcBox,
    RegularPolygon,
    Arc,
    Picture,
    Text,
    Glue,
    Break,
    Scale,
    Align,
    Move,
    Copy,
    MovePoint,
    AddPoint,
    DeletePoint,
    Delete,
    Update,
    Edit,
    FlipV,
    FlipH,
    RotateCcw,
    RotateCw,
    Convert,
    OpenClose,
    Arrow,
}

impl ToolKind {
    pub const ALL: [ToolKind; 38] = [
        ToolKind::Select,
        ToolKind::CircleRadius,
        ToolKind::CircleDiameter,
        ToolKind::EllipseRadii,
        ToolKind::EllipseDiameter,
        ToolKind::SplineApproxClosed,
        ToolKind::SplineApprox,
        ToolKind::SplineInterpClosed,
        ToolKind::SplineInterp,
        ToolKind::XSplineClosed,
        ToolKind::XSpline,
        ToolKind::Polygon,
        ToolKind::Polyline,
        ToolKind::Box,
        ToolKind::ArcBox,
        ToolKind::RegularPolygon,
        ToolKind::Arc,
        ToolKind::Picture,
        ToolKind::Text,
        ToolKind::Glue,
        ToolKind::Break,
        ToolKind::Scale,
        ToolKind::Align,
        ToolKind::Move,
        ToolKind::Copy,
        ToolKind::MovePoint,
        ToolKind::AddPoint,
        ToolKind::DeletePoint,
        ToolKind::Delete,
        ToolKind::Update,
        ToolKind::Edit,
        ToolKind::FlipV,
        ToolKind::FlipH,
        ToolKind::RotateCcw,
        ToolKind::RotateCw,
        ToolKind::Convert,
        ToolKind::OpenClose,
        ToolKind::Arrow,
    ];

    pub fn id(self) -> &'static str {
        match self {
            ToolKind::Select => "tool.select",
            ToolKind::CircleRadius => "tool.circle_radius",
            ToolKind::CircleDiameter => "tool.circle_diameter",
            ToolKind::EllipseRadii => "tool.ellipse_radii",
            ToolKind::EllipseDiameter => "tool.ellipse_diameter",
            ToolKind::SplineApproxClosed => "tool.closed_spline",
            ToolKind::SplineApprox => "tool.spline",
            ToolKind::SplineInterpClosed => "tool.closed_interp_spline",
            ToolKind::SplineInterp => "tool.interp_spline",
            ToolKind::XSplineClosed => "tool.closed_xspline",
            ToolKind::XSpline => "tool.xspline",
            ToolKind::Polygon => "tool.polygon",
            ToolKind::Polyline => "tool.polyline",
            ToolKind::Box => "tool.box",
            ToolKind::ArcBox => "tool.arc_box",
            ToolKind::RegularPolygon => "tool.regular_polygon",
            ToolKind::Arc => "tool.arc",
            ToolKind::Picture => "tool.picture",
            ToolKind::Text => "tool.text",
            ToolKind::Glue => "tool.glue",
            ToolKind::Break => "tool.break",
            ToolKind::Scale => "tool.scale",
            ToolKind::Align => "tool.align",
            ToolKind::Move => "tool.move",
            ToolKind::Copy => "tool.copy",
            ToolKind::MovePoint => "tool.move_point",
            ToolKind::AddPoint => "tool.add_point",
            ToolKind::DeletePoint => "tool.delete_point",
            ToolKind::Delete => "tool.delete",
            ToolKind::Update => "tool.update",
            ToolKind::Edit => "tool.edit",
            ToolKind::FlipV => "tool.flip_vertical",
            ToolKind::FlipH => "tool.flip_horizontal",
            ToolKind::RotateCcw => "tool.rotate_ccw",
            ToolKind::RotateCw => "tool.rotate_cw",
            ToolKind::Convert => "tool.convert",
            ToolKind::OpenClose => "tool.open_close",
            ToolKind::Arrow => "tool.arrow",
        }
    }

    pub fn from_id(id: &str) -> Option<ToolKind> {
        ToolKind::ALL.iter().copied().find(|t| t.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            ToolKind::Select => "Select",
            ToolKind::CircleRadius => "Circle (radius)",
            ToolKind::CircleDiameter => "Circle (diameter)",
            ToolKind::EllipseRadii => "Ellipse (radii)",
            ToolKind::EllipseDiameter => "Ellipse (diameter)",
            ToolKind::SplineApproxClosed => "Closed approximated spline",
            ToolKind::SplineApprox => "Approximated spline",
            ToolKind::SplineInterpClosed => "Closed interpolated spline",
            ToolKind::SplineInterp => "Interpolated spline",
            ToolKind::XSplineClosed => "Closed X-spline",
            ToolKind::XSpline => "X-spline",
            ToolKind::Polygon => "Polygon",
            ToolKind::Polyline => "Polyline",
            ToolKind::Box => "Box",
            ToolKind::ArcBox => "Arc-box (rounded)",
            ToolKind::RegularPolygon => "Regular polygon",
            ToolKind::Arc => "Arc",
            ToolKind::Picture => "Picture",
            ToolKind::Text => "Text",
            ToolKind::Glue => "Glue into compound",
            ToolKind::Break => "Break compound",
            ToolKind::Scale => "Scale",
            ToolKind::Align => "Align",
            ToolKind::Move => "Move",
            ToolKind::Copy => "Copy",
            ToolKind::MovePoint => "Move point",
            ToolKind::AddPoint => "Add point",
            ToolKind::DeletePoint => "Delete point",
            ToolKind::Delete => "Delete",
            ToolKind::Update => "Update attributes",
            ToolKind::Edit => "Edit attributes",
            ToolKind::FlipV => "Flip up/down",
            ToolKind::FlipH => "Flip left/right",
            ToolKind::RotateCcw => "Rotate counter-clockwise",
            ToolKind::RotateCw => "Rotate clockwise",
            ToolKind::Convert => "Convert line ↔ spline",
            ToolKind::OpenClose => "Open/close",
            ToolKind::Arrow => "Add/remove arrows",
        }
    }

    /// Single-key accelerators, after xfig's.
    pub fn shortcut(self) -> &'static str {
        match self {
            ToolKind::Select => "S",
            ToolKind::CircleRadius => "C",
            ToolKind::EllipseRadii => "E",
            ToolKind::SplineApprox => "I",
            ToolKind::Polygon => "P",
            ToolKind::Polyline => "L",
            ToolKind::Box => "B",
            ToolKind::ArcBox => "Shift+B",
            ToolKind::RegularPolygon => "Shift+P",
            ToolKind::Arc => "R",
            ToolKind::Text => "T",
            ToolKind::Glue => "G",
            ToolKind::Break => "Shift+G",
            ToolKind::Scale => "Shift+S",
            ToolKind::Align => "A",
            ToolKind::Move => "M",
            ToolKind::Copy => "Shift+C",
            ToolKind::MovePoint => "Shift+M",
            ToolKind::AddPoint => "Shift+A",
            ToolKind::DeletePoint => "Shift+D",
            ToolKind::Delete => "D",
            ToolKind::Update => "U",
            ToolKind::Edit => "Shift+E",
            ToolKind::FlipV => "F",
            ToolKind::FlipH => "Shift+F",
            ToolKind::RotateCcw => "Shift+R",
            ToolKind::Convert => "V",
            ToolKind::OpenClose => "O",
            ToolKind::Arrow => "W",
            _ => "",
        }
    }

    pub fn is_drawing(self) -> bool {
        matches!(
            self,
            ToolKind::CircleRadius
                | ToolKind::CircleDiameter
                | ToolKind::EllipseRadii
                | ToolKind::EllipseDiameter
                | ToolKind::SplineApproxClosed
                | ToolKind::SplineApprox
                | ToolKind::SplineInterpClosed
                | ToolKind::SplineInterp
                | ToolKind::XSplineClosed
                | ToolKind::XSpline
                | ToolKind::Polygon
                | ToolKind::Polyline
                | ToolKind::Box
                | ToolKind::ArcBox
                | ToolKind::RegularPolygon
                | ToolKind::Arc
                | ToolKind::Picture
                | ToolKind::Text
        )
    }

    /// Tools that collect a list of points.
    fn spline_kind(self) -> Option<(SplineKind, bool)> {
        match self {
            ToolKind::SplineApproxClosed => Some((SplineKind::Approximated, true)),
            ToolKind::SplineApprox => Some((SplineKind::Approximated, false)),
            ToolKind::SplineInterpClosed => Some((SplineKind::Interpolated, true)),
            ToolKind::SplineInterp => Some((SplineKind::Interpolated, false)),
            ToolKind::XSplineClosed => Some((SplineKind::X, true)),
            ToolKind::XSpline => Some((SplineKind::X, false)),
            _ => None,
        }
    }

    fn collects_points(self) -> bool {
        matches!(self, ToolKind::Polyline | ToolKind::Polygon | ToolKind::Arc) || self.spline_kind().is_some()
    }

    fn is_drag_shape(self) -> bool {
        matches!(
            self,
            ToolKind::CircleRadius
                | ToolKind::CircleDiameter
                | ToolKind::EllipseRadii
                | ToolKind::EllipseDiameter
                | ToolKind::Box
                | ToolKind::ArcBox
                | ToolKind::RegularPolygon
                | ToolKind::Picture
        )
    }
}

/// Keys tools react to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Enter,
    Escape,
    Backspace,
    Delete,
    Left,
    Right,
}

/// A pointer or key event in Fig coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolEvent {
    Press { pos: Point, right: bool, shift: bool },
    Move { pos: Point },
    Release { pos: Point, right: bool },
    DoubleClick { pos: Point },
    Key(Key),
    Text(String),
}

/// The text being typed, for the UI to draw with a cursor.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayText {
    pub object: Object,
    /// Cursor position in characters.
    pub cursor: usize,
}

/// What the UI draws on top of the document while a tool is busy.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Overlay {
    /// Preview objects (drawn as outlines).
    pub preview: Vec<Object>,
    /// Extra guide lines (open polylines).
    pub guides: Vec<Vec<Point>>,
    /// Objects hidden while their preview is shown.
    pub hide: Vec<ObjectId>,
    /// Editable vertices to mark.
    pub handles: Vec<Point>,
    pub text: Option<OverlayText>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum ToolState {
    #[default]
    Idle,
    Points {
        pts: Vec<Point>,
        cursor: Point,
    },
    Drag {
        start: Point,
        cursor: Point,
        dragged: bool,
    },
    Text {
        origin: Point,
        text: String,
        cursor: usize,
        editing: Option<ObjectId>,
    },
    MoveObjs {
        ids: Vec<ObjectId>,
        start: Point,
        cursor: Point,
        copy: bool,
        /// Started by a press-drag (commit on release) rather than a click.
        dragging: bool,
    },
    RubberBand {
        start: Point,
        cursor: Point,
        add: bool,
    },
    PointDrag {
        id: ObjectId,
        index: usize,
        insert: bool,
        start: Point,
        cursor: Point,
        dragging: bool,
    },
    Scale {
        id: ObjectId,
        center: Point,
        start: Point,
        cursor: Point,
    },
}

/// What each mouse button does right now (xfig's mouse-function indicator).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MouseHints {
    pub left: &'static str,
    pub middle: &'static str,
    pub right: &'static str,
    /// Keyboard input goes to the tool (the text tool is typing).
    pub keyboard: bool,
}

/// Pick tolerance in screen pixels.
const PICK_PX: f64 = 6.0;

/// Editable vertices of an object (ellipses: centre and corner).
pub fn edit_points(o: &Object) -> Vec<Point> {
    match &o.kind {
        ObjectKind::Ellipse(e) => vec![e.center, e.center + e.radii],
        ObjectKind::Text(t) => vec![t.origin],
        _ => o.points().map(<[Point]>::to_vec).unwrap_or_default(),
    }
}

impl Engine {
    /// Switch tools, abandoning any work in progress.
    pub fn set_tool(&mut self, kind: ToolKind) {
        if let Err(e) = self.finish_text() {
            self.set_status(e.message);
        }
        self.tool_state = ToolState::Idle;
        self.tool = kind;
        self.set_status(kind.label());
    }

    /// Whether a tool has work in progress (keys go to the tool).
    pub fn tool_busy(&self) -> bool {
        !matches!(self.tool_state, ToolState::Idle)
    }

    /// Whether the text tool is collecting typed characters.
    pub fn is_typing(&self) -> bool {
        matches!(self.tool_state, ToolState::Text { .. })
    }

    fn pick_tol(&self) -> f64 {
        PICK_PX / self.view.px_per_fig().max(1e-9)
    }

    /// Feed an event to the active tool.
    pub fn tool_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        match crate::guard::catch(|| self.dispatch(ev)) {
            Ok(r) => {
                if r.is_err() {
                    self.tool_state = ToolState::Idle;
                }
                r
            }
            Err(msg) => {
                self.tool_state = ToolState::Idle;
                Err(EngineError::internal(msg))
            }
        }
    }

    fn dispatch(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        if let ToolEvent::Key(Key::Escape) = ev {
            if matches!(self.tool_state, ToolState::Text { .. }) {
                return self.finish_text();
            }
            self.tool_state = ToolState::Idle;
            return Ok(());
        }
        let tool = self.tool;
        if tool == ToolKind::Text {
            return self.text_event(ev);
        }
        if tool.collects_points() {
            return self.points_event(ev);
        }
        if tool.is_drag_shape() {
            return self.drag_event(ev);
        }
        match tool {
            ToolKind::Select => self.select_event(ev),
            ToolKind::Glue => self.band_event(ev, true),
            ToolKind::Move | ToolKind::Copy => self.move_event(ev, tool == ToolKind::Copy),
            ToolKind::MovePoint | ToolKind::AddPoint => self.point_event(ev),
            ToolKind::Scale => self.scale_event(ev),
            _ => self.click_event(ev),
        }
    }

    // ---- point-collecting tools --------------------------------------

    fn points_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let tool = self.tool;
        let tol = self.pick_tol();
        match ev {
            ToolEvent::Press { pos, right, .. } => {
                let p = self.snap(pos);
                if right {
                    return self.finish_points();
                }
                match &mut self.tool_state {
                    ToolState::Points { pts, cursor } => {
                        if pts.last().is_none_or(|l| l.dist(p) > tol * 0.5) {
                            pts.push(p);
                        }
                        *cursor = p;
                    }
                    _ => {
                        self.tool_state = ToolState::Points {
                            pts: vec![p],
                            cursor: p,
                        };
                    }
                }
                if tool == ToolKind::Arc {
                    if let ToolState::Points { pts, .. } = &self.tool_state {
                        if pts.len() >= 3 {
                            return self.finish_points();
                        }
                    }
                }
                Ok(())
            }
            ToolEvent::Move { pos } => {
                let p = self.snap(pos);
                if let ToolState::Points { cursor, .. } = &mut self.tool_state {
                    *cursor = p;
                }
                Ok(())
            }
            ToolEvent::DoubleClick { .. } | ToolEvent::Key(Key::Enter) => self.finish_points(),
            ToolEvent::Key(Key::Backspace) | ToolEvent::Key(Key::Delete) => {
                if let ToolState::Points { pts, .. } = &mut self.tool_state {
                    pts.pop();
                    if pts.is_empty() {
                        self.tool_state = ToolState::Idle;
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// The object a point-collecting tool would create from `pts`.
    fn points_object(&self, pts: &[Point]) -> Option<Object> {
        let style = self.indicator.style();
        let (fa, ba) = self.indicator.arrows();
        let tool = self.tool;
        let kind = if let Some((sk, closed)) = tool.spline_kind() {
            if pts.len() < if closed { 3 } else { 2 } {
                return None;
            }
            let mut s = Spline::new(sk, closed, style, pts.to_vec());
            if !closed {
                s.forward_arrow = fa;
                s.backward_arrow = ba;
            }
            ObjectKind::Spline(s)
        } else if tool == ToolKind::Arc {
            let [a, b, c] = pts else { return None };
            let mut arc = Arc::through(style, *a, *b, *c, false)?;
            arc.forward_arrow = fa;
            arc.backward_arrow = ba;
            ObjectKind::Arc(arc)
        } else if tool == ToolKind::Polygon {
            if pts.len() < 3 {
                return None;
            }
            ObjectKind::Polyline(Polyline::new(PolyKind::Polygon, style, pts.to_vec()))
        } else {
            if pts.is_empty() {
                return None;
            }
            let mut pl = Polyline::new(PolyKind::Polyline, style, pts.to_vec());
            pl.forward_arrow = fa;
            pl.backward_arrow = ba;
            ObjectKind::Polyline(pl)
        };
        Some(Object::new(kind))
    }

    fn finish_points(&mut self) -> Result<(), EngineError> {
        let ToolState::Points { pts, .. } = std::mem::take(&mut self.tool_state) else {
            return Ok(());
        };
        let mut pts = pts;
        // Drop accidental duplicates from double clicks.
        pts.dedup_by(|a, b| a.dist(*b) < 1e-6);
        let Some(obj) = self.points_object(&pts) else {
            // Too few points: a single click with the polyline tool is a dot.
            if self.tool == ToolKind::Polyline && pts.len() == 1 {
                if let Some(o) = self.points_object(&pts) {
                    return self.commit_object(o);
                }
            }
            return Ok(());
        };
        self.commit_object(obj)
    }

    fn commit_object(&mut self, obj: Object) -> Result<(), EngineError> {
        let v = serde_json::to_value(&obj).map_err(|e| EngineError::internal(e.to_string()))?;
        self.execute("object.add", &json!({ "object": v }))?;
        Ok(())
    }

    // ---- two-point (drag) tools --------------------------------------

    fn drag_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let tol = self.pick_tol();
        match ev {
            ToolEvent::Press { pos, right, .. } => {
                if right {
                    self.tool_state = ToolState::Idle;
                    return Ok(());
                }
                let p = self.snap(pos);
                match &self.tool_state {
                    ToolState::Drag { start, .. } => {
                        let start = *start;
                        self.tool_state = ToolState::Idle;
                        self.finish_drag(start, p)
                    }
                    _ => {
                        self.tool_state = ToolState::Drag {
                            start: p,
                            cursor: p,
                            dragged: false,
                        };
                        Ok(())
                    }
                }
            }
            ToolEvent::Move { pos } => {
                let p = self.snap(pos);
                if let ToolState::Drag { start, cursor, dragged } = &mut self.tool_state {
                    *cursor = p;
                    if start.dist(p) > tol {
                        *dragged = true;
                    }
                }
                Ok(())
            }
            ToolEvent::Release { pos, .. } => {
                let p = self.snap(pos);
                if let ToolState::Drag {
                    start, dragged: true, ..
                } = self.tool_state
                {
                    if start.dist(p) > tol {
                        self.tool_state = ToolState::Idle;
                        return self.finish_drag(start, p);
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn drag_object(&self, a: Point, b: Point) -> Option<Object> {
        let style = self.indicator.style();
        if a.dist(b) < 1e-6 {
            return None;
        }
        let o = match self.tool {
            ToolKind::CircleRadius => make_ellipse(style, a, a.dist(b), a.dist(b), 0.0, EllipseKind::CircleRadius),
            ToolKind::CircleDiameter => {
                let r = a.dist(b) / 2.0;
                make_ellipse(style, a.lerp(b, 0.5), r, r, 0.0, EllipseKind::CircleDiameter)
            }
            ToolKind::EllipseRadii => {
                let d = b - a;
                make_ellipse(style, a, d.x.abs(), d.y.abs(), 0.0, EllipseKind::EllipseRadii)
            }
            ToolKind::EllipseDiameter => {
                let d = b - a;
                make_ellipse(
                    style,
                    a.lerp(b, 0.5),
                    d.x.abs() / 2.0,
                    d.y.abs() / 2.0,
                    0.0,
                    EllipseKind::EllipseDiameter,
                )
            }
            ToolKind::Box | ToolKind::Picture => Object::new(ObjectKind::Polyline(Polyline::rect(style, a, b))),
            ToolKind::ArcBox => {
                let mut p = Polyline::rect(style, a, b);
                p.kind = PolyKind::ArcBox;
                p.radius = self.indicator.box_radius;
                Object::new(ObjectKind::Polyline(p))
            }
            ToolKind::RegularPolygon => Object::new(ObjectKind::Polyline(Polyline::new(
                PolyKind::Polygon,
                style,
                regular_polygon(a, b, self.indicator.sides),
            ))),
            _ => return None,
        };
        Some(o)
    }

    fn finish_drag(&mut self, a: Point, b: Point) -> Result<(), EngineError> {
        if self.tool == ToolKind::Picture {
            if a.dist(b) > 1e-6 {
                self.request(UiRequest::PickPicture { rect: Rect::new(a, b) });
            }
            return Ok(());
        }
        match self.drag_object(a, b) {
            Some(o) => self.commit_object(o),
            None => Ok(()),
        }
    }

    // ---- text ----------------------------------------------------------

    fn text_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        match ev {
            ToolEvent::Press { pos, right, .. } => {
                self.finish_text()?;
                if right {
                    return Ok(());
                }
                // Clicking an existing text edits it.
                let tol = self.pick_tol();
                if let Some(id) = self.hit_test(pos, tol) {
                    if let Some(Object {
                        kind: ObjectKind::Text(t),
                        ..
                    }) = self.doc().get(id)
                    {
                        self.tool_state = ToolState::Text {
                            origin: t.origin,
                            text: t.text.clone(),
                            cursor: t.text.chars().count(),
                            editing: Some(id),
                        };
                        return Ok(());
                    }
                }
                self.tool_state = ToolState::Text {
                    origin: self.snap(pos),
                    text: String::new(),
                    cursor: 0,
                    editing: None,
                };
                Ok(())
            }
            ToolEvent::Text(s) => {
                if let ToolState::Text { text, cursor, .. } = &mut self.tool_state {
                    let clean: String = s.chars().filter(|c| !c.is_control() || *c == '\t').collect();
                    if text.len() + clean.len() > rustfig_format::MAX_TEXT_BYTES {
                        return Err(EngineError::bad_params("text is too long"));
                    }
                    let byte = char_to_byte(text, *cursor);
                    text.insert_str(byte, &clean);
                    *cursor += clean.chars().count();
                }
                Ok(())
            }
            ToolEvent::Key(k) => {
                let ToolState::Text {
                    text,
                    cursor,
                    origin,
                    editing,
                } = &mut self.tool_state
                else {
                    return Ok(());
                };
                match k {
                    Key::Backspace => {
                        if *cursor > 0 {
                            let b = char_to_byte(text, *cursor - 1);
                            text.remove(b);
                            *cursor -= 1;
                        }
                    }
                    Key::Delete => {
                        if *cursor < text.chars().count() {
                            let b = char_to_byte(text, *cursor);
                            text.remove(b);
                        }
                    }
                    Key::Left => *cursor = cursor.saturating_sub(1),
                    Key::Right => *cursor = (*cursor + 1).min(text.chars().count()),
                    Key::Enter => {
                        // Finish this line and start a new text object below it.
                        let origin = *origin;
                        let _ = editing;
                        self.finish_text()?;
                        let size = self.indicator.font_size * rustfig_geom::FIG_PER_POINT * rustfig_text::LINE_SPACING;
                        let a = self.indicator.text_angle.to_radians();
                        let next = origin + Affine::rotate(a).apply_vec(Point::new(0.0, size));
                        self.tool_state = ToolState::Text {
                            origin: next,
                            text: String::new(),
                            cursor: 0,
                            editing: None,
                        };
                    }
                    Key::Escape => {}
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Commit the text being typed, if any.
    pub fn finish_text(&mut self) -> Result<(), EngineError> {
        let ToolState::Text {
            origin, text, editing, ..
        } = std::mem::take(&mut self.tool_state)
        else {
            return Ok(());
        };
        match editing {
            Some(id) => {
                if text.trim().is_empty() {
                    self.execute("edit.delete", &json!({"ids": [id.0]}))?;
                } else {
                    let unchanged = matches!(
                        self.doc().get(id).map(|o| &o.kind),
                        Some(ObjectKind::Text(t)) if t.text == text
                    );
                    if !unchanged {
                        self.execute("object.set_attributes", &json!({"ids": [id.0], "text": text}))?;
                    }
                }
            }
            None => {
                if !text.is_empty() {
                    self.execute("shape.text", &json!({"x": origin.x, "y": origin.y, "text": text}))?;
                }
            }
        }
        Ok(())
    }

    // ---- selection -------------------------------------------------------

    fn select_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let tol = self.pick_tol();
        match ev {
            ToolEvent::Press { pos, right, shift } => {
                if right {
                    self.tool_state = ToolState::Idle;
                    return Ok(());
                }
                // Vertex handles of a single selected object.
                if let [id] = self.selection() {
                    let id = *id;
                    if let Some(o) = self.doc().get(id) {
                        if !matches!(o.kind, ObjectKind::Text(_) | ObjectKind::Compound(_)) {
                            if let Some(index) = nearest_vertex(&edit_points(o), pos, tol) {
                                self.tool_state = ToolState::PointDrag {
                                    id,
                                    index,
                                    insert: false,
                                    start: pos,
                                    cursor: pos,
                                    dragging: true,
                                };
                                return Ok(());
                            }
                        }
                    }
                }
                match self.hit_test(pos, tol) {
                    Some(id) => {
                        let mut sel = self.selection().to_vec();
                        if shift {
                            if let Some(i) = sel.iter().position(|s| *s == id) {
                                sel.remove(i);
                                self.set_selection(sel);
                                return Ok(());
                            }
                            sel.push(id);
                        } else if !sel.contains(&id) {
                            sel = vec![id];
                        }
                        self.set_selection(sel.clone());
                        self.tool_state = ToolState::MoveObjs {
                            ids: sel,
                            start: pos,
                            cursor: pos,
                            copy: false,
                            dragging: true,
                        };
                    }
                    None => {
                        if !shift {
                            self.set_selection(Vec::new());
                        }
                        self.tool_state = ToolState::RubberBand {
                            start: pos,
                            cursor: pos,
                            add: shift,
                        };
                    }
                }
                Ok(())
            }
            ToolEvent::DoubleClick { pos } => {
                if let Some(id) = self.hit_test(pos, tol) {
                    self.tool_state = ToolState::Idle;
                    self.set_selection(vec![id]);
                    self.request(UiRequest::EditObject { id });
                }
                Ok(())
            }
            ToolEvent::Key(Key::Delete) | ToolEvent::Key(Key::Backspace) => {
                if !self.selection().is_empty() {
                    self.execute("edit.delete", &json!({}))?;
                }
                Ok(())
            }
            other => self.drag_common(other),
        }
    }

    /// Move, release and rubber-band handling shared by several tools.
    fn drag_common(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let tol = self.pick_tol();
        match ev {
            ToolEvent::Move { pos } => {
                match &mut self.tool_state {
                    ToolState::MoveObjs { cursor, .. }
                    | ToolState::RubberBand { cursor, .. }
                    | ToolState::PointDrag { cursor, .. }
                    | ToolState::Scale { cursor, .. } => *cursor = pos,
                    _ => {}
                }
                Ok(())
            }
            ToolEvent::Release { pos, .. } => match self.tool_state.clone() {
                ToolState::MoveObjs {
                    ids,
                    start,
                    copy,
                    dragging: true,
                    ..
                } => {
                    if start.dist(pos) > tol {
                        self.tool_state = ToolState::Idle;
                        self.commit_move(&ids, start, pos, copy)
                    } else if self.tool == ToolKind::Select {
                        self.tool_state = ToolState::Idle;
                        Ok(())
                    } else {
                        // A click: keep following the pointer until the next click.
                        self.tool_state = ToolState::MoveObjs {
                            ids,
                            start,
                            cursor: pos,
                            copy,
                            dragging: false,
                        };
                        Ok(())
                    }
                }
                ToolState::RubberBand { start, add, .. } => {
                    self.tool_state = ToolState::Idle;
                    let r = Rect::new(start, pos);
                    let inside: Vec<ObjectId> = self
                        .doc()
                        .objects
                        .iter()
                        .filter(|o| self.is_visible(o) && r.contains_rect(&o.bbox()))
                        .map(|o| o.id)
                        .collect();
                    if self.tool == ToolKind::Glue {
                        if inside.len() >= 2 {
                            self.execute("object.group", &json!({"ids": crate::cmd::ids_value(&inside)}))?;
                        }
                        return Ok(());
                    }
                    let mut sel = if add { self.selection().to_vec() } else { Vec::new() };
                    sel.extend(inside);
                    self.set_selection(sel);
                    Ok(())
                }
                ToolState::PointDrag {
                    id,
                    index,
                    insert,
                    start,
                    dragging: true,
                    ..
                } => {
                    if start.dist(pos) > tol || insert {
                        self.tool_state = ToolState::Idle;
                        self.commit_point(id, index, insert, pos)
                    } else if self.tool == ToolKind::Select {
                        self.tool_state = ToolState::Idle;
                        Ok(())
                    } else {
                        self.tool_state = ToolState::PointDrag {
                            id,
                            index,
                            insert,
                            start,
                            cursor: pos,
                            dragging: false,
                        };
                        Ok(())
                    }
                }
                ToolState::Scale { id, center, start, .. } => {
                    if start.dist(pos) > tol {
                        self.tool_state = ToolState::Idle;
                        return self.commit_scale(id, center, start, pos);
                    }
                    Ok(())
                }
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }

    fn commit_move(&mut self, ids: &[ObjectId], start: Point, end: Point, copy: bool) -> Result<(), EngineError> {
        let d = self.snap(end) - self.snap(start);
        if d.len() < 1e-9 && !copy {
            return Ok(());
        }
        let ids = crate::cmd::ids_value(ids);
        if copy {
            self.execute("edit.duplicate", &json!({"ids": ids, "dx": d.x, "dy": d.y}))?;
        } else {
            self.execute("transform.move", &json!({"ids": ids, "dx": d.x, "dy": d.y}))?;
        }
        Ok(())
    }

    fn commit_point(&mut self, id: ObjectId, index: usize, insert: bool, pos: Point) -> Result<(), EngineError> {
        let p = self.snap(pos);
        let cmd = if insert { "point.add" } else { "point.move" };
        self.execute(cmd, &json!({"id": id.0, "index": index, "x": p.x, "y": p.y}))?;
        Ok(())
    }

    fn commit_scale(&mut self, id: ObjectId, center: Point, start: Point, end: Point) -> Result<(), EngineError> {
        let d0 = start.dist(center);
        if d0 < 1e-6 {
            return Ok(());
        }
        let f = end.dist(center) / d0;
        if !(1e-3..=1e3).contains(&f) {
            return Err(EngineError::bad_params("scale factor out of range"));
        }
        self.execute(
            "transform.scale",
            &json!({"ids": [id.0], "sx": f, "sy": f, "cx": center.x, "cy": center.y}),
        )?;
        Ok(())
    }

    // ---- move / copy (click-move-click or drag) ----------------------

    fn move_event(&mut self, ev: ToolEvent, copy: bool) -> Result<(), EngineError> {
        if let ToolEvent::Press { pos, right, .. } = ev {
            if right {
                self.tool_state = ToolState::Idle;
                return Ok(());
            }
            if let ToolState::MoveObjs {
                ids,
                start,
                dragging: false,
                copy,
                ..
            } = self.tool_state.clone()
            {
                self.tool_state = ToolState::Idle;
                return self.commit_move(&ids, start, pos, copy);
            }
            let tol = self.pick_tol();
            if let Some(id) = self.hit_test(pos, tol) {
                self.set_selection(vec![id]);
                self.tool_state = ToolState::MoveObjs {
                    ids: vec![id],
                    start: pos,
                    cursor: pos,
                    copy,
                    dragging: true,
                };
            }
            return Ok(());
        }
        self.drag_common(ev)
    }

    // ---- point editing -----------------------------------------------

    fn point_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        if let ToolEvent::Press { pos, right, .. } = ev {
            if right {
                self.tool_state = ToolState::Idle;
                return Ok(());
            }
            if let ToolState::PointDrag {
                id,
                index,
                insert,
                dragging: false,
                ..
            } = self.tool_state.clone()
            {
                self.tool_state = ToolState::Idle;
                return self.commit_point(id, index, insert, pos);
            }
            let tol = self.pick_tol();
            let found = if self.tool == ToolKind::MovePoint {
                self.find_vertex(pos, tol).map(|(id, i)| (id, i, false))
            } else {
                self.find_segment(pos, tol).map(|(id, i)| (id, i, true))
            };
            if let Some((id, index, insert)) = found {
                self.tool_state = ToolState::PointDrag {
                    id,
                    index,
                    insert,
                    start: pos,
                    cursor: pos,
                    dragging: true,
                };
            }
            return Ok(());
        }
        self.drag_common(ev)
    }

    /// The nearest editable vertex of any top-level object.
    fn find_vertex(&self, p: Point, tol: f64) -> Option<(ObjectId, usize)> {
        let mut best: Option<(f64, ObjectId, usize)> = None;
        for o in &self.doc().objects {
            if !self.is_visible(o) || matches!(o.kind, ObjectKind::Text(_) | ObjectKind::Compound(_)) {
                continue;
            }
            for (i, v) in edit_points(o).iter().enumerate() {
                let d = v.dist(p);
                if d <= tol && best.is_none_or(|(bd, _, _)| d < bd) {
                    best = Some((d, o.id, i));
                }
            }
        }
        best.map(|(_, id, i)| (id, i))
    }

    /// The control-polygon segment nearest to `p`; returns the insert index.
    fn find_segment(&self, p: Point, tol: f64) -> Option<(ObjectId, usize)> {
        let mut best: Option<(f64, ObjectId, usize)> = None;
        for o in &self.doc().objects {
            let closed = match &o.kind {
                ObjectKind::Polyline(pl) if pl.kind == PolyKind::Polyline => false,
                ObjectKind::Polyline(pl) if pl.kind == PolyKind::Polygon => true,
                ObjectKind::Spline(s) => s.closed,
                _ => continue,
            };
            let Some(pts) = o.points() else { continue };
            let n = pts.len();
            let segs = if closed { n } else { n.saturating_sub(1) };
            for i in 0..segs {
                let (Some(a), Some(b)) = (pts.get(i), pts.get((i + 1) % n)) else {
                    continue;
                };
                let d = dist_to_segment(p, *a, *b);
                if d <= tol * 2.0 && best.is_none_or(|(bd, _, _)| d < bd) {
                    best = Some((d, o.id, i + 1));
                }
            }
        }
        best.map(|(_, id, i)| (id, i))
    }

    // ---- scale -----------------------------------------------------------

    fn scale_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        if let ToolEvent::Press { pos, right, .. } = ev {
            if right {
                self.tool_state = ToolState::Idle;
                return Ok(());
            }
            if let ToolState::Scale { id, center, start, .. } = self.tool_state.clone() {
                self.tool_state = ToolState::Idle;
                return self.commit_scale(id, center, start, pos);
            }
            let tol = self.pick_tol();
            if let Some(id) = self.hit_test(pos, tol) {
                if let Some(o) = self.doc().get(id) {
                    let center = o.geometry_bbox().center();
                    self.set_selection(vec![id]);
                    self.tool_state = ToolState::Scale {
                        id,
                        center,
                        start: pos,
                        cursor: pos,
                    };
                }
            }
            return Ok(());
        }
        self.drag_common(ev)
    }

    fn band_event(&mut self, ev: ToolEvent, _glue: bool) -> Result<(), EngineError> {
        if let ToolEvent::Press { pos, right, .. } = ev {
            if right {
                self.tool_state = ToolState::Idle;
                return Ok(());
            }
            self.tool_state = ToolState::RubberBand {
                start: pos,
                cursor: pos,
                add: false,
            };
            return Ok(());
        }
        self.drag_common(ev)
    }

    // ---- single-click editing tools ----------------------------------

    fn click_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let ToolEvent::Press { pos, right, shift } = ev else {
            return Ok(());
        };
        if right {
            return Ok(());
        }
        let tol = self.pick_tol();
        if self.tool == ToolKind::DeletePoint {
            if let Some((id, index)) = self.find_vertex(pos, tol) {
                self.execute("point.delete", &json!({"id": id.0, "index": index}))?;
            }
            return Ok(());
        }
        let Some(id) = self.hit_test(pos, tol) else {
            return Ok(());
        };
        let ids = json!([id.0]);
        match self.tool {
            ToolKind::Delete => {
                self.execute("edit.delete", &json!({ "ids": ids }))?;
            }
            ToolKind::Update => {
                self.execute("object.update", &json!({ "ids": ids }))?;
            }
            ToolKind::Edit => {
                self.set_selection(vec![id]);
                self.request(UiRequest::EditObject { id });
            }
            ToolKind::FlipV => {
                self.execute("transform.flip_vertical", &json!({ "ids": ids }))?;
            }
            ToolKind::FlipH => {
                self.execute("transform.flip_horizontal", &json!({ "ids": ids }))?;
            }
            ToolKind::RotateCcw | ToolKind::RotateCw => {
                let mut a = self.indicator.rotate_angle;
                if (self.tool == ToolKind::RotateCw) != shift {
                    a = -a;
                }
                self.execute("transform.rotate", &json!({ "ids": ids, "angle": a }))?;
            }
            ToolKind::Break => {
                if matches!(self.doc().get(id).map(|o| &o.kind), Some(ObjectKind::Compound(_))) {
                    self.execute("object.ungroup", &json!({ "ids": ids }))?;
                }
            }
            ToolKind::Align => {
                // Align the members of a compound with each other.
                if matches!(self.doc().get(id).map(|o| &o.kind), Some(ObjectKind::Compound(_))) {
                    self.execute("object.ungroup", &json!({ "ids": ids }))?;
                    let freed = crate::cmd::ids_value(self.selection());
                    self.execute("arrange.align", &json!({ "ids": freed }))?;
                    self.execute("object.group", &json!({ "ids": freed }))?;
                }
            }
            ToolKind::Convert => {
                self.execute("object.convert", &json!({ "ids": ids }))?;
            }
            ToolKind::OpenClose => {
                self.execute("object.toggle_closed", &json!({ "ids": ids }))?;
            }
            ToolKind::Arrow => {
                let Some(o) = self.doc().get(id) else { return Ok(()) };
                let pts = rustfig_model::flatten_object(o);
                let Some(path) = pts.first() else { return Ok(()) };
                let (Some(first), Some(last)) = (path.pts.first(), path.pts.last()) else {
                    return Ok(());
                };
                let cmd = if pos.dist(*last) <= pos.dist(*first) {
                    "object.toggle_forward_arrow"
                } else {
                    "object.toggle_backward_arrow"
                };
                self.execute(cmd, &json!({ "ids": ids }))?;
            }
            _ => {}
        }
        Ok(())
    }

    // ---- mouse hints -------------------------------------------------------

    /// The current function of each mouse button, for the mouse indicator.
    /// Must describe what [`Engine::tool_event`] actually does.
    pub fn mouse_hints(&self) -> MouseHints {
        let pan = "pan";
        let h = |left, right| MouseHints {
            left,
            middle: pan,
            right,
            keyboard: false,
        };
        let tool = self.tool;
        match &self.tool_state {
            ToolState::Points { pts, .. } => {
                if tool == ToolKind::Arc {
                    h(if pts.len() < 2 { "second point" } else { "last point" }, "cancel")
                } else {
                    h("next point", "finish")
                }
            }
            ToolState::Drag { .. } => {
                let left = match tool {
                    ToolKind::CircleRadius => "set radius",
                    ToolKind::CircleDiameter => "set diameter",
                    ToolKind::EllipseRadii => "set radii",
                    ToolKind::RegularPolygon => "set size",
                    _ => "final corner",
                };
                h(left, "cancel")
            }
            ToolState::Text { .. } => MouseHints {
                left: "new text",
                middle: pan,
                right: "finish text",
                keyboard: true,
            },
            ToolState::MoveObjs { dragging: true, .. } => h("drop", ""),
            ToolState::MoveObjs { .. } => h("place object", "cancel"),
            ToolState::RubberBand { .. } => h("drag region", ""),
            ToolState::PointDrag { dragging: true, .. } => h("drop point", ""),
            ToolState::PointDrag { .. } => h("place point", "cancel"),
            ToolState::Scale { .. } => h("set size", "cancel"),
            ToolState::Idle => {
                let left = match tool {
                    ToolKind::Select => "select / drag",
                    ToolKind::Polyline | ToolKind::Polygon | ToolKind::Arc => "first point",
                    t if t.spline_kind().is_some() => "first point",
                    ToolKind::CircleRadius | ToolKind::EllipseRadii | ToolKind::RegularPolygon => "center",
                    ToolKind::CircleDiameter => "diameter start",
                    ToolKind::EllipseDiameter | ToolKind::Box | ToolKind::ArcBox | ToolKind::Picture => "first corner",
                    ToolKind::Text => "place or edit text",
                    ToolKind::Glue => "region to glue",
                    ToolKind::Break => "break compound",
                    ToolKind::Scale => "pick object",
                    ToolKind::Align => "align compound",
                    ToolKind::Move => "pick object",
                    ToolKind::Copy => "copy object",
                    ToolKind::MovePoint => "pick point",
                    ToolKind::AddPoint => "add point",
                    ToolKind::DeletePoint => "delete point",
                    ToolKind::Delete => "delete object",
                    ToolKind::Update => "update object",
                    ToolKind::Edit => "edit object",
                    ToolKind::FlipV => "flip up/down",
                    ToolKind::FlipH => "flip left/right",
                    ToolKind::RotateCcw => "rotate ccw",
                    ToolKind::RotateCw => "rotate cw",
                    ToolKind::Convert => "convert",
                    ToolKind::OpenClose => "open/close",
                    ToolKind::Arrow => "toggle arrow",
                    _ => "",
                };
                h(left, "")
            }
        }
    }

    // ---- overlay -----------------------------------------------------------

    /// What to draw on top of the document for the active tool.
    pub fn overlay(&self) -> Overlay {
        let mut ov = Overlay::default();
        match &self.tool_state {
            ToolState::Idle => {}
            ToolState::Points { pts, cursor } => {
                let mut all = pts.clone();
                all.push(*cursor);
                if self.tool == ToolKind::Arc && all.len() < 3 {
                    ov.guides.push(all.clone());
                } else if let Some(o) = self.points_object(&all) {
                    ov.preview.push(o);
                }
                if self.tool.spline_kind().is_some() {
                    ov.guides.push(all.clone());
                }
                ov.handles = pts.clone();
            }
            ToolState::Drag { start, cursor, .. } => {
                if let Some(o) = self.drag_object(*start, *cursor) {
                    ov.preview.push(o);
                }
            }
            ToolState::Text {
                origin, text, cursor, ..
            } => {
                let mut t = crate::cmd::shape::make_text(self, *origin, text.clone());
                if let ToolState::Text { editing: Some(id), .. } = &self.tool_state {
                    if let Some(Object {
                        kind: ObjectKind::Text(orig),
                        ..
                    }) = self.doc().get(*id)
                    {
                        t = orig.clone();
                        t.text = text.clone();
                    }
                    ov.hide.push(*id);
                }
                crate::fix_text_metrics(&mut t);
                ov.text = Some(OverlayText {
                    object: Object::new(ObjectKind::Text(t)),
                    cursor: *cursor,
                });
            }
            ToolState::MoveObjs {
                ids,
                start,
                cursor,
                copy,
                ..
            } => {
                let d = self.snap(*cursor) - self.snap(*start);
                for id in ids {
                    if let Some(o) = self.doc().get(*id) {
                        let mut c = o.clone();
                        c.translate(d.x, d.y);
                        ov.preview.push(c);
                    }
                }
                if !copy && d.len() > 0.0 {
                    ov.hide.clone_from(ids);
                }
            }
            ToolState::RubberBand { start, cursor, .. } => {
                let r = Rect::new(*start, *cursor);
                let mut c = r.corners().to_vec();
                if let Some(f) = c.first().copied() {
                    c.push(f);
                }
                ov.guides.push(c);
            }
            ToolState::PointDrag {
                id,
                index,
                insert,
                cursor,
                ..
            } => {
                if let Some(o) = self.doc().get(*id) {
                    let p = self.snap(*cursor);
                    let mut c = o.clone();
                    preview_point(&mut c, *index, *insert, p);
                    ov.handles = edit_points(&c);
                    ov.preview.push(c);
                    ov.hide.push(*id);
                }
            }
            ToolState::Scale {
                id,
                center,
                start,
                cursor,
            } => {
                if let Some(o) = self.doc().get(*id) {
                    let d0 = start.dist(*center);
                    if d0 > 1e-6 {
                        let f = (cursor.dist(*center) / d0).clamp(1e-3, 1e3);
                        let mut c = o.clone();
                        c.transform(&Affine::about(Affine::scale(f, f), *center), true);
                        ov.preview.push(c);
                        ov.hide.push(*id);
                    }
                    ov.guides.push(vec![*center, *cursor]);
                }
            }
        }
        if ov.handles.is_empty() && matches!(self.tool_state, ToolState::Idle) {
            let show = matches!(
                self.tool,
                ToolKind::Select | ToolKind::MovePoint | ToolKind::DeletePoint | ToolKind::AddPoint
            );
            if show {
                let ids: Vec<ObjectId> = if self.tool == ToolKind::Select {
                    self.selection().to_vec()
                } else {
                    self.doc().objects.iter().map(|o| o.id).collect()
                };
                if ids.len() <= 2000 {
                    for id in ids {
                        if let Some(o) = self.doc().get(id) {
                            if !matches!(o.kind, ObjectKind::Compound(_) | ObjectKind::Text(_)) {
                                ov.handles.extend(edit_points(o));
                            }
                        }
                    }
                }
            }
        }
        ov
    }
}

/// Apply a pending point edit to a copy, for previewing.
fn preview_point(o: &mut Object, index: usize, insert: bool, p: Point) {
    match &mut o.kind {
        ObjectKind::Polyline(pl) if insert => {
            let i = index.min(pl.points.len());
            pl.points.insert(i, p);
        }
        ObjectKind::Spline(s) if insert => {
            let i = index.min(s.points.len());
            s.points.insert(i, p);
            let i = i.min(s.shapes.len());
            s.shapes.insert(i, s.kind.default_shape());
        }
        ObjectKind::Polyline(pl) if pl.kind.is_rect() => {
            if let Some(opp) = pl.points.get((index + 2) % 4).copied() {
                pl.points = Rect::new(opp, p).corners().to_vec();
            }
        }
        ObjectKind::Polyline(pl) => {
            if let Some(q) = pl.points.get_mut(index) {
                *q = p;
            }
        }
        ObjectKind::Spline(s) => {
            if let Some(q) = s.points.get_mut(index) {
                *q = p;
            }
        }
        ObjectKind::Arc(a) => {
            if let Some(q) = a.points.get_mut(index) {
                *q = p;
            }
            a.refit();
        }
        ObjectKind::Ellipse(e) => match index {
            0 => e.center = p,
            _ => {
                let d = p - e.center;
                e.radii = if e.kind.is_circle() {
                    Point::new(d.len(), d.len())
                } else {
                    Point::new(d.x.abs(), d.y.abs())
                };
            }
        },
        _ => {}
    }
}

fn nearest_vertex(pts: &[Point], p: Point, tol: f64) -> Option<usize> {
    let mut best: Option<(f64, usize)> = None;
    for (i, v) in pts.iter().enumerate() {
        let d = v.dist(p);
        if d <= tol && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, i));
        }
    }
    best.map(|(_, i)| i)
}

fn char_to_byte(s: &str, chars: usize) -> usize {
    s.char_indices().nth(chars).map(|(b, _)| b).unwrap_or(s.len())
}

/// Text justification name.
pub fn justify_name(j: Justify) -> &'static str {
    match j {
        Justify::Left => "left",
        Justify::Center => "center",
        Justify::Right => "right",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(e: &mut Engine, x: f64, y: f64) {
        e.tool_event(ToolEvent::Press {
            pos: Point::new(x, y),
            right: false,
            shift: false,
        })
        .unwrap();
        e.tool_event(ToolEvent::Release {
            pos: Point::new(x, y),
            right: false,
        })
        .unwrap();
    }

    #[test]
    fn draw_polyline_by_clicks() {
        let mut e = Engine::new();
        e.set_tool(ToolKind::Polyline);
        press(&mut e, 0.0, 0.0);
        press(&mut e, 1200.0, 0.0);
        press(&mut e, 1200.0, 1200.0);
        e.tool_event(ToolEvent::DoubleClick {
            pos: Point::new(1200.0, 1200.0),
        })
        .unwrap();
        assert_eq!(e.doc().objects.len(), 1);
        assert_eq!(e.doc().objects[0].points().unwrap().len(), 3);
        assert!(!e.tool_busy());
    }

    #[test]
    fn draw_box_by_drag_and_by_clicks() {
        let mut e = Engine::new();
        e.set_tool(ToolKind::Box);
        e.tool_event(ToolEvent::Press {
            pos: Point::new(0.0, 0.0),
            right: false,
            shift: false,
        })
        .unwrap();
        e.tool_event(ToolEvent::Move {
            pos: Point::new(600.0, 300.0),
        })
        .unwrap();
        assert_eq!(e.overlay().preview.len(), 1);
        e.tool_event(ToolEvent::Release {
            pos: Point::new(600.0, 300.0),
            right: false,
        })
        .unwrap();
        assert_eq!(e.doc().objects.len(), 1);
        press(&mut e, 0.0, 0.0);
        press(&mut e, 300.0, 300.0);
        assert_eq!(e.doc().objects.len(), 2);
    }

    #[test]
    fn type_text() {
        let mut e = Engine::new();
        e.set_tool(ToolKind::Text);
        press(&mut e, 600.0, 600.0);
        e.tool_event(ToolEvent::Text("Hellx".into())).unwrap();
        e.tool_event(ToolEvent::Key(Key::Backspace)).unwrap();
        e.tool_event(ToolEvent::Text("o".into())).unwrap();
        assert!(e.overlay().text.is_some());
        e.tool_event(ToolEvent::Key(Key::Escape)).unwrap();
        let ObjectKind::Text(t) = &e.doc().objects[0].kind else {
            panic!()
        };
        assert_eq!(t.text, "Hello");
        assert!(t.length > 0.0);
        // Click it again to edit.
        press(&mut e, 650.0, 590.0);
        e.tool_event(ToolEvent::Text("!".into())).unwrap();
        e.set_tool(ToolKind::Select);
        let ObjectKind::Text(t) = &e.doc().objects[0].kind else {
            panic!()
        };
        assert_eq!(t.text, "Hello!");
        assert_eq!(e.doc().objects.len(), 1);
    }

    #[test]
    fn select_and_drag_moves() {
        let mut e = Engine::new();
        e.execute(
            "shape.rectangle",
            &json!({"x": 0, "y": 0, "width": 1200, "height": 1200, "area_fill": 20}),
        )
        .unwrap();
        e.set_tool(ToolKind::Select);
        e.set_selection(vec![]);
        e.tool_event(ToolEvent::Press {
            pos: Point::new(600.0, 600.0),
            right: false,
            shift: false,
        })
        .unwrap();
        e.tool_event(ToolEvent::Move {
            pos: Point::new(900.0, 600.0),
        })
        .unwrap();
        e.tool_event(ToolEvent::Release {
            pos: Point::new(900.0, 600.0),
            right: false,
        })
        .unwrap();
        let bb = e.doc().objects[0].geometry_bbox();
        assert_eq!(bb.min.x, 300.0);
        assert!(e.undo());
        assert_eq!(e.doc().objects[0].geometry_bbox().min.x, 0.0);
    }

    #[test]
    fn rubber_band_selects() {
        let mut e = Engine::new();
        e.execute("shape.circle", &json!({"cx": 300, "cy": 300, "r": 100}))
            .unwrap();
        e.execute("shape.circle", &json!({"cx": 3000, "cy": 3000, "r": 100}))
            .unwrap();
        e.set_tool(ToolKind::Select);
        e.set_selection(vec![]);
        e.tool_event(ToolEvent::Press {
            pos: Point::new(0.0, 0.0),
            right: false,
            shift: false,
        })
        .unwrap();
        e.tool_event(ToolEvent::Move {
            pos: Point::new(1000.0, 1000.0),
        })
        .unwrap();
        e.tool_event(ToolEvent::Release {
            pos: Point::new(1000.0, 1000.0),
            right: false,
        })
        .unwrap();
        assert_eq!(e.selection().len(), 1);
    }

    #[test]
    fn mouse_hints_follow_state() {
        let mut e = Engine::new();
        for t in ToolKind::ALL {
            e.set_tool(t);
            let h = e.mouse_hints();
            assert!(!h.left.is_empty(), "{} has no left-button hint", t.id());
            assert_eq!(h.middle, "pan");
        }
        e.set_tool(ToolKind::Polyline);
        assert_eq!(e.mouse_hints().left, "first point");
        press(&mut e, 0.0, 0.0);
        assert_eq!(e.mouse_hints().right, "finish");
        e.set_tool(ToolKind::Text);
        press(&mut e, 0.0, 0.0);
        assert!(e.mouse_hints().keyboard);
        e.set_tool(ToolKind::Box);
        e.tool_event(ToolEvent::Press {
            pos: Point::ZERO,
            right: false,
            shift: false,
        })
        .unwrap();
        assert_eq!(e.mouse_hints().right, "cancel");
        e.tool_event(ToolEvent::Press {
            pos: Point::ZERO,
            right: true,
            shift: false,
        })
        .unwrap();
        assert!(!e.tool_busy(), "right button must cancel as the hint says");
    }

    #[test]
    fn every_tool_survives_random_events() {
        for t in ToolKind::ALL {
            let mut e = Engine::new();
            e.execute(
                "shape.rectangle",
                &json!({"x": 0, "y": 0, "width": 1200, "height": 600}),
            )
            .unwrap();
            e.execute("shape.polyline", &json!({"points": [[0, 0], [600, 600], [1200, 0]]}))
                .unwrap();
            e.set_tool(t);
            let evs = [
                ToolEvent::Press {
                    pos: Point::new(0.0, 0.0),
                    right: false,
                    shift: false,
                },
                ToolEvent::Move {
                    pos: Point::new(300.0, 300.0),
                },
                ToolEvent::Release {
                    pos: Point::new(300.0, 300.0),
                    right: false,
                },
                ToolEvent::Text("ab".into()),
                ToolEvent::Press {
                    pos: Point::new(600.0, 600.0),
                    right: false,
                    shift: true,
                },
                ToolEvent::Move {
                    pos: Point::new(f64::MAX, -1.0),
                },
                ToolEvent::DoubleClick {
                    pos: Point::new(600.0, 600.0),
                },
                ToolEvent::Key(Key::Enter),
                ToolEvent::Key(Key::Backspace),
                ToolEvent::Press {
                    pos: Point::new(1200.0, 0.0),
                    right: true,
                    shift: false,
                },
                ToolEvent::Key(Key::Escape),
            ];
            for ev in evs {
                let _ = e.tool_event(ev);
                let _ = e.overlay();
            }
        }
    }
}
