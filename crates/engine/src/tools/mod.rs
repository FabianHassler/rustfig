//! Interactive tools (xfig's drawing and editing modes).
//!
//! A tool receives pointer and key events in Fig coordinates, keeps its
//! in-progress state (the rubber band) and commits its result by running a
//! command, so every edit is undoable and scriptable.
//
// Ported in parts from xfig src/w_modepanel.c (mode keys), the drawing and
// editing modes src/d_*.c and src/e_*.c (button behaviour, `set_mousefun`
// wording) and src/w_canvas.c (snapping) (xfig licence, see LICENSE-xfig).

mod chop;
mod compound;
mod markers;
mod tangent;
use crate::Region;
pub use markers::{mode_mask, object_markers, ObjMask};

use crate::cmd::shape::make_ellipse;
use crate::{Engine, EngineError, UiRequest};
use rustfig_geom::{dist_to_segment, regular_polygon, Affine, Point, Rect};
use rustfig_model::{Arc, EllipseKind, Justify, Object, ObjectId, ObjectKind, PolyKind, Polyline, Spline, SplineKind};
use serde_json::json;

/// The tools, in mode-panel order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolKind {
    /// No mode chosen yet: xfig starts like this ("READY. Select a mode or
    /// load a file"); canvas clicks do nothing. Not a command or a button.
    None,
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
    OpenCompound,
    JoinSplit,
    Chop,
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
    /// Add a tangent or normal line to a curve (xfig's `F_TANGENT`).
    Tangent,
}

impl ToolKind {
    pub const ALL: [ToolKind; 42] = [
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
        ToolKind::OpenCompound,
        ToolKind::JoinSplit,
        ToolKind::Chop,
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
        ToolKind::Tangent,
    ];

    pub fn id(self) -> &'static str {
        match self {
            ToolKind::None => "tool.none",
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
            ToolKind::OpenCompound => "tool.open_compound",
            ToolKind::JoinSplit => "tool.join_split",
            ToolKind::Chop => "tool.chop",
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
            ToolKind::Tangent => "tool.tangent",
        }
    }

    pub fn from_id(id: &str) -> Option<ToolKind> {
        ToolKind::ALL.iter().copied().find(|t| t.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            ToolKind::None => "No mode",
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
            ToolKind::OpenCompound => "Open compound",
            ToolKind::JoinSplit => "Join/split lines and splines",
            ToolKind::Chop => "Chop",
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
            ToolKind::Tangent => "Add tangent/normal to curve",
        }
    }

    /// Single-key accelerators: xfig's mode keys (the tooltips in xfig's
    /// `w_modepanel.c`). xfig's Ctrl-key modes (Scale, Copy, Add point,
    /// Edit, Rotate clockwise, Picture) are in the UI's xfig keymap, since
    /// Ctrl+S, Ctrl+C, … are the platform's on Windows and Linux.
    pub fn shortcut(self) -> &'static str {
        match self {
            ToolKind::CircleRadius => "C",
            ToolKind::EllipseRadii => "E",
            ToolKind::CircleDiameter => "Shift+C",
            ToolKind::EllipseDiameter => "Shift+E",
            ToolKind::SplineApprox => "S",
            ToolKind::SplineApproxClosed => "Shift+S",
            ToolKind::SplineInterp => "I",
            ToolKind::SplineInterpClosed => "Shift+I",
            ToolKind::Polygon => "P",
            ToolKind::Polyline => "L",
            ToolKind::Box => "B",
            ToolKind::ArcBox => "Shift+B",
            ToolKind::RegularPolygon => "Shift+P",
            ToolKind::Arc => "R",
            ToolKind::Text => "T",
            ToolKind::Glue => "G",
            ToolKind::Break => "Shift+G",
            ToolKind::OpenCompound => "O",
            ToolKind::JoinSplit => "J",
            ToolKind::Chop => "x",
            ToolKind::Align => "A",
            ToolKind::Move => "M",
            ToolKind::MovePoint => "Shift+M",
            ToolKind::DeletePoint => "Shift+D",
            ToolKind::Delete => "D",
            ToolKind::Update => "U",
            ToolKind::FlipV => "F",
            ToolKind::FlipH => "Shift+F",
            ToolKind::RotateCcw => "Shift+R",
            ToolKind::Convert => "V",
            ToolKind::Arrow => "Shift+A",
            ToolKind::Tangent => "n",
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
    /// Start of the text being typed (xfig's `*Text*translations`).
    Home,
    /// End of the text being typed.
    End,
}

/// A mouse button as the tools see it. The UI maps two-button mice and
/// trackpads onto these (Alt+left is the middle button, Ctrl+left the right).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Button {
    #[default]
    Left,
    Middle,
    Right,
}

impl Button {
    pub fn from_name(s: &str) -> Option<Button> {
        match s {
            "left" => Some(Button::Left),
            "middle" => Some(Button::Middle),
            "right" => Some(Button::Right),
            _ => None,
        }
    }
}

/// A pointer or key event in Fig coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolEvent {
    Press { pos: Point, button: Button, shift: bool },
    Move { pos: Point },
    Release { pos: Point, button: Button },
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
    /// xfig's object markers of the active editing mode (`markers.rs`).
    pub markers: Vec<Point>,
    /// Join/Split: the end point picked first, drawn as a magenta square of
    /// 20 screen pixels (`draw_join_marker`).
    pub join_marker: Option<Point>,
    pub text: Option<OverlayText>,
}

/// What a rubber band does when it closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Band {
    Select,
    SelectAdd,
    /// Tag the objects inside for gluing (they join the selection).
    Tag,
    Delete,
    /// Update mode, right button (W13): update everything inside.
    UpdateRegion,
    /// Move mode, right button (W14): move everything inside.
    MoveRegion,
    /// Delete point mode, middle button (W14): delete the points inside.
    DeletePoints,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum ToolState {
    #[default]
    Idle,
    Points {
        pts: Vec<Point>,
        cursor: Point,
        /// Points follow the pointer (xfig: middle button).
        freehand: bool,
        /// Arc by centre (xfig: middle button first).
        center: Option<Point>,
    },
    Drag {
        start: Point,
        cursor: Point,
        dragged: bool,
        /// The shape being drawn (the circle modes draw ellipses with the
        /// middle button).
        kind: ToolKind,
        button: Button,
        /// Shift was held when the drag began (centre boxes: a square).
        square: bool,
    },
    /// Circle through three points (right button, W14).
    ThreePoints { pts: Vec<Point>, cursor: Point },
    /// Align mode, right button (W14): the reference object was clicked.
    AlignTo { reference: ObjectId },
    /// Join/Split: the first end point was picked (`join_lines`,
    /// `join_splines`); the next click picks the second.
    Join { id: ObjectId, first: bool, spline: bool },
    /// Chop: the axe (cutting) objects selected so far.
    Chop { axes: Vec<ObjectId> },
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
        /// Horizontal or vertical only (middle button).
        constrain: bool,
    },
    RubberBand {
        start: Point,
        cursor: Point,
        purpose: Band,
        /// Opened by a click; the next click closes it.
        click: bool,
        /// The button that opened it (a click with it closes it).
        button: Button,
        /// The opening button is still down (W15): moving draws a lasso.
        held: bool,
        /// The freehand path since the press (W15), the start first.
        lasso: Vec<Point>,
    },
    PointDrag {
        id: ObjectId,
        index: usize,
        insert: bool,
        start: Point,
        cursor: Point,
        dragging: bool,
        constrain: bool,
    },
    Scale {
        id: ObjectId,
        /// The fixed point (opposite corner, or the centre).
        center: Point,
        start: Point,
        cursor: Point,
        button: Button,
    },
}

/// What each mouse button does right now (xfig's mouse-function indicator).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MouseHints {
    pub left: &'static str,
    pub middle: &'static str,
    pub right: &'static str,
    /// What the buttons do with Shift held, where the mode has such
    /// variants (Chop's Trim, Split and Extend); empty otherwise.
    pub shift: &'static str,
}

/// Pick tolerance in screen pixels.
const PICK_PX: f64 = 6.0;

/// Cap on the points one freehand stroke collects.
const MAX_FREEHAND_POINTS: usize = 10_000;

/// Editable vertices of an object (ellipses: centre and corner).
pub fn edit_points(o: &Object) -> Vec<Point> {
    match &o.kind {
        ObjectKind::Ellipse(e) => vec![e.center, e.center + e.radii],
        ObjectKind::Text(t) => vec![t.origin],
        _ => o.points().map(<[Point]>::to_vec).unwrap_or_default(),
    }
}

/// Keep only the larger of the horizontal and vertical components.
/// How many points of a line, polygon or spline lie inside `r`.
fn region_points(o: &Object, r: &Region) -> Option<usize> {
    if !matches!(o.kind, ObjectKind::Polyline(_) | ObjectKind::Spline(_))
        || !mode_mask(ToolKind::DeletePoint, 90.0).admits(o)
    {
        return None;
    }
    o.points().map(|p| p.iter().filter(|q| r.contains(**q)).count())
}

/// The centre of a regular polygon of `sides` sides that has the side from
/// `a` to `b`, on the side where `b` follows `a` in `regular_polygon`'s order.
fn side_polygon_center(a: Point, b: Point, sides: usize) -> Option<Point> {
    let n = sides.clamp(3, 1000);
    let s = a.dist(b);
    if !s.is_finite() || s < 1e-6 {
        return None;
    }
    let apothem = s / (2.0 * (std::f64::consts::PI / n as f64).tan());
    let mid = a.lerp(b, 0.5);
    let dir = (b - a) * (1.0 / s);
    let normal = Point::new(-dir.y, dir.x);
    [1.0, -1.0].into_iter().map(|k| mid + normal * (apothem * k)).find(|c| {
        regular_polygon(*c, a, n)
            .get(1)
            .is_some_and(|q| q.dist(b) < s * 1e-6 + 1e-6)
    })
}

fn constrain_delta(d: Point) -> Point {
    if d.x.abs() >= d.y.abs() {
        Point::new(d.x, 0.0)
    } else {
        Point::new(0.0, d.y)
    }
}

/// The three arc points for xfig's arc by centre: `pts` are the first
/// point (which sets the radius), the mid angle and the final angle.
fn arc_from_center(c: Point, pts: &[Point]) -> Option<[Point; 3]> {
    let [a, m, b] = pts else { return None };
    let r = a.dist(c);
    let on = |q: Point| {
        let d = q - c;
        let l = d.len();
        (l > 1e-9 && r > 1e-9).then(|| c + d * (r / l))
    };
    Some([*a, on(*m)?, on(*b)?])
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

    /// The point being drawn from (xfig's `cur_point`): the last point of a
    /// line being entered, the first corner of a drag shape, or where a move
    /// started.
    fn current_point(&self) -> Option<Point> {
        match &self.tool_state {
            ToolState::Points { pts, center, .. } => pts.last().copied().or(*center),
            ToolState::Drag { start, .. }
            | ToolState::MoveObjs { start, .. }
            | ToolState::PointDrag { start, .. }
            | ToolState::Scale { start, .. } => Some(*start),
            _ => None,
        }
    }

    /// The object under a press: at `pos`, or for a snapped press also at
    /// the point the user actually clicked.
    fn pick(&self, pos: Point, tol: f64) -> Option<ObjectId> {
        self.hit_test(pos, tol)
            .or_else(|| self.snap_click.and_then(|c| self.hit_test(c, tol)))
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

    /// The pick tolerance in Fig units (for commands that take a click).
    pub(crate) fn pick_tolerance(&self) -> f64 {
        self.pick_tol()
    }

    /// Feed an event to the active tool.
    pub fn tool_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let r = self.tool_event_inner(ev);
        // Snapped points belong to the operation in progress.
        if !self.tool_busy() {
            self.exact_points.clear();
        }
        r
    }

    fn tool_event_inner(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let r = crate::guard::catch(|| self.dispatch(ev));
        self.snap_click = None;
        match r {
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
        // The Snap menu: a press (not with the right button) moves to the
        // snapped point, or is ignored when nothing was found (xfig's
        // `snap_process` in `w_canvas.c`).
        let ev = match ev {
            ToolEvent::Press { pos, button, shift }
                if button != Button::Right && self.object_snap().is_some() && pos.is_finite() =>
            {
                let tol = self.pick_tol() * 1.5;
                let current = self.current_point();
                match self.apply_object_snap(pos, tol, current) {
                    Some(p) => {
                        self.snap_click = Some(pos);
                        ToolEvent::Press { pos: p, button, shift }
                    }
                    None => return Ok(()),
                }
            }
            ToolEvent::Release { pos, button } => ToolEvent::Release {
                pos: self.release_at.take().unwrap_or(pos),
                button,
            },
            other => other,
        };
        let tool = self.tool;
        if tool == ToolKind::None {
            return Ok(());
        }
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
            ToolKind::Glue => self.glue_event(ev),
            ToolKind::Move | ToolKind::Copy => self.move_event(ev, tool == ToolKind::Copy),
            ToolKind::MovePoint | ToolKind::AddPoint => self.point_event(ev),
            ToolKind::Scale => self.scale_event(ev),
            ToolKind::Delete => self.delete_event(ev),
            ToolKind::OpenCompound => self.open_compound_event(ev),
            ToolKind::JoinSplit => self.join_split_event(ev),
            ToolKind::Chop => self.chop_event(ev),
            ToolKind::Tangent => self.tangent_event(ev),
            _ => self.click_event(ev),
        }
    }

    // ---- point-collecting tools --------------------------------------

    fn points_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let tool = self.tool;
        let tol = self.pick_tol();
        match ev {
            ToolEvent::Press { pos, button, shift } => {
                let p = self.snap(pos);
                // Angle Geom: the point is the constrained one (xfig clicks
                // at the rubber band's end, `cur_x`/`cur_y`).
                let p = if button == Button::Right {
                    p
                } else {
                    self.angle_constrained(p)
                };
                let busy = matches!(self.tool_state, ToolState::Points { .. });
                match button {
                    Button::Right => {
                        if busy {
                            self.tool_state = ToolState::Idle;
                            return Ok(());
                        }
                        // xfig: the right button enters a single point (a dot).
                        if tool == ToolKind::Polyline {
                            if let Some(o) = self.points_object(&[p], None) {
                                return self.commit_object(o);
                            }
                        }
                        Ok(())
                    }
                    Button::Middle => {
                        if busy {
                            if tool == ToolKind::Arc {
                                return Ok(());
                            }
                            // Final point.
                            self.push_point(p, tol);
                            return self.finish_points();
                        }
                        self.tool_state = if tool == ToolKind::Arc {
                            ToolState::Points {
                                pts: Vec::new(),
                                cursor: p,
                                freehand: false,
                                center: Some(p),
                            }
                        } else {
                            ToolState::Points {
                                pts: vec![p],
                                cursor: p,
                                freehand: true,
                                center: None,
                            }
                        };
                        Ok(())
                    }
                    Button::Left => {
                        if let ToolState::Points { pts, freehand, .. } = &mut self.tool_state {
                            if *freehand {
                                return Ok(());
                            }
                            if shift {
                                // xfig: Shift+left removes the last point.
                                pts.pop();
                                if pts.is_empty() {
                                    self.tool_state = ToolState::Idle;
                                }
                                return Ok(());
                            }
                        }
                        self.push_point(p, tol);
                        if tool == ToolKind::Arc {
                            if let ToolState::Points { pts, .. } = &self.tool_state {
                                if pts.len() >= 3 {
                                    return self.finish_points();
                                }
                            }
                        }
                        Ok(())
                    }
                }
            }
            ToolEvent::Move { pos } => {
                let p = self.snap(pos);
                let p = self.angle_constrained(p);
                if let ToolState::Points {
                    pts, cursor, freehand, ..
                } = &mut self.tool_state
                {
                    *cursor = p;
                    if *freehand && pts.last().is_none_or(|l| l.dist(p) > tol) && pts.len() < MAX_FREEHAND_POINTS {
                        pts.push(p);
                    }
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

    /// `p` as constrained by Angle Geom from the last point entered (xfig's
    /// `latex_line` / `constrainedangle_line` locmove procedures of
    /// `init_trace_drawing`): lines, polygons and splines, not freehand, not
    /// arcs, and not for points an object snap placed exactly.
    fn angle_constrained(&self, p: Point) -> Point {
        let mode = self.indicator.angle_geom;
        if mode == crate::AngleGeom::Unconstrained || self.tool == ToolKind::Arc || self.exact_points.contains(&p) {
            return p;
        }
        let ToolState::Points {
            pts,
            freehand: false,
            center: None,
            ..
        } = &self.tool_state
        else {
            return p;
        };
        match pts.last() {
            Some(fix) => crate::constrain_angle(mode, *fix, p, self.indicator.magnet(self.doc().settings.units)),
            None => p,
        }
    }

    /// Add a point to the one being entered, or start a new point list.
    fn push_point(&mut self, p: Point, tol: f64) {
        match &mut self.tool_state {
            ToolState::Points { pts, cursor, .. } => {
                if pts.last().is_none_or(|l| l.dist(p) > tol * 0.5) {
                    pts.push(p);
                }
                *cursor = p;
            }
            _ => {
                self.tool_state = ToolState::Points {
                    pts: vec![p],
                    cursor: p,
                    freehand: false,
                    center: None,
                };
            }
        }
    }

    /// The object a point-collecting tool would create from `pts`.
    fn points_object(&self, pts: &[Point], center: Option<Point>) -> Option<Object> {
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
            let [a, b, c] = match center {
                Some(m) => arc_from_center(m, pts)?,
                None => {
                    let [a, b, c] = pts else { return None };
                    [*a, *b, *c]
                }
            };
            let mut arc = Arc::through(style, a, b, c, self.indicator.pie_arc)?;
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
        let ToolState::Points { mut pts, center, .. } = std::mem::take(&mut self.tool_state) else {
            return Ok(());
        };
        // Drop accidental duplicates from double clicks.
        pts.dedup_by(|a, b| a.dist(*b) < 1e-6);
        match self.points_object(&pts, center) {
            Some(o) => self.commit_object(o),
            None => Ok(()),
        }
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
            ToolEvent::Press { pos, button, shift } => {
                let p = self.snap(pos);
                let circle = matches!(self.tool, ToolKind::CircleRadius | ToolKind::CircleDiameter);
                let picture = self.tool == ToolKind::Picture;
                // xfig leaves the right button free in the circle and
                // picture modes: a circle through three points, a picture
                // that keeps its proportions.
                if button == Button::Right {
                    match self.tool_state.clone() {
                        ToolState::ThreePoints { mut pts, .. } if circle => {
                            pts.push(p);
                            if pts.len() < 3 {
                                self.tool_state = ToolState::ThreePoints { pts, cursor: p };
                                return Ok(());
                            }
                            self.tool_state = ToolState::Idle;
                            return self.finish_three_points(&pts);
                        }
                        ToolState::Idle if circle => {
                            self.tool_state = ToolState::ThreePoints {
                                pts: vec![p],
                                cursor: p,
                            };
                            return Ok(());
                        }
                        ToolState::Idle if picture => {
                            self.tool_state = ToolState::Drag {
                                start: p,
                                cursor: p,
                                dragged: false,
                                kind: ToolKind::Picture,
                                button,
                                square: false,
                            };
                            return Ok(());
                        }
                        ToolState::Drag {
                            start,
                            kind: ToolKind::Picture,
                            button: Button::Right,
                            ..
                        } => {
                            self.tool_state = ToolState::Idle;
                            return self.finish_drag(ToolKind::Picture, start, p, true);
                        }
                        _ => {}
                    }
                }
                if matches!(self.tool_state, ToolState::ThreePoints { .. }) {
                    // Only the right button works until the circle is done.
                    return Ok(());
                }
                if button == Button::Right {
                    self.tool_state = ToolState::Idle;
                    return Ok(());
                }
                if let ToolState::Drag {
                    start,
                    kind,
                    button: b,
                    square,
                    ..
                } = self.tool_state
                {
                    // xfig: an ellipse is finished with either button.
                    let ellipse = matches!(kind, ToolKind::EllipseRadii | ToolKind::EllipseDiameter);
                    if b != button && !ellipse {
                        return Ok(());
                    }
                    self.tool_state = ToolState::Idle;
                    let (a, c) = self.drag_span(kind, b, square, start, p);
                    return self.finish_drag(kind, a, c, false);
                }
                let Some(kind) = self.drag_kind(button) else {
                    return Ok(());
                };
                self.tool_state = ToolState::Drag {
                    start: p,
                    cursor: p,
                    dragged: false,
                    kind,
                    button,
                    square: shift && button == Button::Middle,
                };
                Ok(())
            }
            ToolEvent::Move { pos } => {
                let p = self.snap(pos);
                if let ToolState::ThreePoints { cursor, .. } = &mut self.tool_state {
                    *cursor = p;
                }
                if let ToolState::Drag {
                    start, cursor, dragged, ..
                } = &mut self.tool_state
                {
                    *cursor = p;
                    if start.dist(p) > tol {
                        *dragged = true;
                    }
                }
                Ok(())
            }
            ToolEvent::Release { pos, button } => {
                let p = self.snap(pos);
                if let ToolState::Drag {
                    start,
                    dragged: true,
                    kind,
                    button: b,
                    square,
                    ..
                } = self.tool_state
                {
                    if b == button && start.dist(p) > tol {
                        self.tool_state = ToolState::Idle;
                        let (a, c) = self.drag_span(kind, b, square, start, p);
                        return self.finish_drag(kind, a, c, b == Button::Right);
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// The shape a drag tool draws with `button`. With xfig's buttons the
    /// circle and ellipse modes draw circles with the left button and
    /// ellipses with the middle one, as xfig's ellipse modes do.
    fn drag_kind(&self, button: Button) -> Option<ToolKind> {
        let t = self.tool;
        let by_radius = matches!(t, ToolKind::CircleRadius | ToolKind::EllipseRadii);
        let by_diameter = matches!(t, ToolKind::CircleDiameter | ToolKind::EllipseDiameter);
        match button {
            Button::Left if by_radius => Some(ToolKind::CircleRadius),
            Button::Left if by_diameter => Some(ToolKind::CircleDiameter),
            Button::Left => Some(t),
            Button::Middle if by_radius => Some(ToolKind::EllipseRadii),
            Button::Middle if by_diameter => Some(ToolKind::EllipseDiameter),
            // Boxes and pictures from the centre, a regular polygon by one side.
            Button::Middle
                if matches!(
                    t,
                    ToolKind::Box | ToolKind::ArcBox | ToolKind::Picture | ToolKind::RegularPolygon
                ) =>
            {
                Some(t)
            }
            _ => None,
        }
    }

    fn drag_object(&self, kind: ToolKind, a: Point, b: Point) -> Option<Object> {
        let style = self.indicator.style();
        if a.dist(b) < 1e-6 {
            return None;
        }
        let o = match kind {
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

    /// The two points a drag shape is made from. The middle button draws
    /// a box or picture from its centre (Shift: a square) and a regular
    /// polygon by the ends of one side (W14); otherwise the points are the
    /// ones dragged.
    fn drag_span(&self, kind: ToolKind, button: Button, square: bool, a: Point, b: Point) -> (Point, Point) {
        if button != Button::Middle {
            return (a, b);
        }
        match kind {
            ToolKind::Box | ToolKind::ArcBox | ToolKind::Picture => {
                let mut d = b - a;
                if square {
                    let s = d.x.abs().max(d.y.abs());
                    d = Point::new(if d.x < 0.0 { -s } else { s }, if d.y < 0.0 { -s } else { s });
                }
                (a - d, a + d)
            }
            ToolKind::RegularPolygon => match side_polygon_center(a, b, self.indicator.sides) {
                Some(c) => (c, a),
                None => (a, a),
            },
            _ => (a, b),
        }
    }

    /// A circle through three points (right button in the circle modes).
    fn finish_three_points(&mut self, pts: &[Point]) -> Result<(), EngineError> {
        let kind = if self.tool == ToolKind::CircleDiameter {
            "diameter"
        } else {
            "radius"
        };
        let pts: Vec<[f64; 2]> = pts.iter().map(|p| [p.x, p.y]).collect();
        self.execute("shape.circle3", &json!({"points": pts, "kind": kind}))?;
        Ok(())
    }

    fn finish_drag(&mut self, kind: ToolKind, a: Point, b: Point, keep_aspect: bool) -> Result<(), EngineError> {
        if kind == ToolKind::Picture {
            if a.dist(b) > 1e-6 {
                self.request(UiRequest::PickPicture {
                    rect: Rect::new(a, b),
                    keep_aspect,
                });
            }
            return Ok(());
        }
        match self.drag_object(kind, a, b) {
            Some(o) => self.commit_object(o),
            None => Ok(()),
        }
    }

    // ---- text ----------------------------------------------------------

    fn text_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        match ev {
            ToolEvent::Press { pos, button, .. } => {
                let typing = matches!(self.tool_state, ToolState::Text { .. });
                match button {
                    // xfig: the middle button finishes the text (`d_text.c`).
                    Button::Middle => {
                        if typing {
                            return self.finish_text();
                        }
                        // W14: paste the clipboard's text as a new text.
                        if self.clipboard_text().is_some() {
                            let p = self.snap(pos);
                            self.execute("edit.paste_text", &json!({"x": p.x, "y": p.y}))?;
                        }
                        return Ok(());
                    }
                    // xfig: the right button cancels it (an edited text keeps
                    // its old string).
                    Button::Right => {
                        self.tool_state = ToolState::Idle;
                        return Ok(());
                    }
                    Button::Left => self.finish_text()?,
                }
                // Clicking an existing text edits it.
                let tol = self.pick_tol();
                if let Some(id) = self.pick(pos, tol) {
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
                    text, cursor, origin, ..
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
                    Key::Home => *cursor = 0,
                    Key::End => *cursor = text.chars().count(),
                    Key::Enter => {
                        // Finish this line and start a new text object below it.
                        let origin = *origin;
                        self.finish_text()?;
                        let step = if self.indicator.text_step.is_finite() {
                            self.indicator.text_step.clamp(0.0, 100.0)
                        } else {
                            1.0
                        };
                        let size =
                            self.indicator.font_size * rustfig_geom::FIG_PER_POINT * rustfig_text::LINE_SPACING * step;
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
            ToolEvent::Press { pos, button, shift } => {
                if button != Button::Left {
                    if button == Button::Right {
                        self.tool_state = ToolState::Idle;
                    }
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
                                    constrain: false,
                                };
                                return Ok(());
                            }
                        }
                    }
                }
                match self.pick(pos, tol) {
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
                            constrain: false,
                        };
                    }
                    None => {
                        if !shift {
                            self.set_selection(Vec::new());
                        }
                        self.tool_state = ToolState::RubberBand {
                            start: pos,
                            cursor: pos,
                            purpose: if shift { Band::SelectAdd } else { Band::Select },
                            click: false,
                            button: Button::Left,
                            held: false,
                            lasso: Vec::new(),
                        };
                    }
                }
                Ok(())
            }
            ToolEvent::DoubleClick { pos } => {
                if let Some(id) = self.pick(pos, tol) {
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
                if let ToolState::RubberBand { held: true, lasso, .. } = &mut self.tool_state {
                    // A lasso point per few pixels, capped.
                    let min_step = tol / 3.0;
                    if lasso.len() < MAX_FREEHAND_POINTS && lasso.last().is_none_or(|l| l.dist(pos) >= min_step) {
                        lasso.push(pos);
                    }
                }
                match &mut self.tool_state {
                    ToolState::MoveObjs { cursor, .. }
                    | ToolState::RubberBand { cursor, .. }
                    | ToolState::PointDrag { cursor, .. }
                    | ToolState::Scale { cursor, .. } => *cursor = pos,
                    _ => {}
                }
                Ok(())
            }
            ToolEvent::Release {
                button: Button::Right, ..
            } if !matches!(
                self.tool_state,
                ToolState::RubberBand {
                    button: Button::Right,
                    ..
                }
            ) =>
            {
                Ok(())
            }
            ToolEvent::Release { pos, .. } => match self.tool_state.clone() {
                ToolState::MoveObjs {
                    ids,
                    start,
                    copy,
                    dragging: true,
                    constrain,
                    ..
                } => {
                    if start.dist(pos) > tol {
                        self.tool_state = ToolState::Idle;
                        self.commit_move(&ids, start, pos, copy, constrain)
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
                            constrain,
                        };
                        Ok(())
                    }
                }
                ToolState::RubberBand {
                    start,
                    purpose,
                    click,
                    button,
                    held,
                    lasso,
                    ..
                } => {
                    let dragged = start.dist(pos) > tol || (held && lasso.iter().any(|q| start.dist(*q) > tol));
                    if click && !dragged {
                        // The release of the click that opened the band.
                        if let ToolState::RubberBand { held, lasso, .. } = &mut self.tool_state {
                            *held = false;
                            lasso.clear();
                        }
                        return Ok(());
                    }
                    self.tool_state = ToolState::Idle;
                    if held && lasso.len() >= 2 && button != Button::Left {
                        // Press and drag: the freehand lasso (W15).
                        let mut path = lasso;
                        path.push(pos);
                        return self.finish_band(purpose, Region::Lasso(path), pos);
                    }
                    self.finish_band(purpose, Region::Rect(Rect::new(start, pos)), pos)
                }
                ToolState::PointDrag {
                    id,
                    index,
                    insert,
                    start,
                    dragging: true,
                    constrain,
                    ..
                } => {
                    if start.dist(pos) > tol || insert {
                        self.tool_state = ToolState::Idle;
                        self.commit_point(id, index, insert, start, pos, constrain)
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
                            constrain,
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

    /// Close a rubber band from `start` to `end`.
    fn finish_band(&mut self, purpose: Band, region: Region, end: Point) -> Result<(), EngineError> {
        let inside: Vec<ObjectId> = self.objects_within_region(&region);
        match purpose {
            Band::Select | Band::SelectAdd => {
                let mut sel = if purpose == Band::SelectAdd {
                    self.selection().to_vec()
                } else {
                    Vec::new()
                };
                sel.extend(inside);
                self.set_selection(sel);
            }
            Band::Tag => {
                let mut sel = self.selection().to_vec();
                for id in inside {
                    if !sel.contains(&id) {
                        sel.push(id);
                    }
                }
                self.set_selection(sel);
            }
            Band::UpdateRegion | Band::MoveRegion | Band::DeletePoints => {
                let mask = mode_mask(self.tool, self.indicator.rotate_angle);
                let found = self.objects_in_region(&region, mask);
                let rp = match &region {
                    Region::Rect(r) => json!({"x1": r.min.x, "y1": r.min.y, "x2": r.max.x, "y2": r.max.y}),
                    Region::Lasso(path) => {
                        json!({"polygon": path.iter().map(|p| json!([p.x, p.y])).collect::<Vec<_>>()})
                    }
                };
                match purpose {
                    Band::UpdateRegion => {
                        if found.is_empty() {
                            self.set_status("No objects completely inside the region");
                        } else {
                            self.execute("object.update_region", &rp)?;
                        }
                    }
                    Band::MoveRegion => {
                        if found.is_empty() {
                            self.set_status("No objects completely inside the region");
                        } else {
                            // Place the whole region with the next click.
                            self.set_selection(found.clone());
                            self.tool_state = ToolState::MoveObjs {
                                ids: found,
                                start: end,
                                cursor: end,
                                copy: false,
                                dragging: false,
                                constrain: false,
                            };
                        }
                    }
                    _ => {
                        let has = self
                            .doc()
                            .objects
                            .iter()
                            .any(|o| self.is_visible(o) && region_points(o, &region).is_some_and(|n| n > 0));
                        if has {
                            self.execute("point.delete_region", &rp)?;
                        } else {
                            self.set_status("No point of a line or spline inside the region");
                        }
                    }
                }
            }
            Band::Delete => {
                if !inside.is_empty() {
                    // The command acts on the selection (it must not be empty).
                    self.set_selection(inside.clone());
                    self.execute("edit.delete", &json!({"ids": crate::cmd::ids_value(&inside)}))?;
                }
            }
        }
        Ok(())
    }

    /// The displacement of a move from `start` to `end`.
    fn move_delta(&self, start: Point, end: Point, constrain: bool) -> Point {
        let d = self.snap(end) - self.snap(start);
        if constrain {
            constrain_delta(d)
        } else {
            d
        }
    }

    fn commit_move(
        &mut self,
        ids: &[ObjectId],
        start: Point,
        end: Point,
        copy: bool,
        constrain: bool,
    ) -> Result<(), EngineError> {
        let d = self.move_delta(start, end, constrain);
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

    /// Where a dragged point lands: snapped, and with the middle button
    /// kept horizontal or vertical from its original position.
    fn point_target(
        &self,
        id: ObjectId,
        index: usize,
        insert: bool,
        start: Point,
        pos: Point,
        constrain: bool,
    ) -> Point {
        let p = self.snap(pos);
        if !constrain {
            // Angle Geom applies to the end of an open line or spline
            // (xfig's `move_point_selected`, `init_linepointadding`).
            return match self.angle_anchor(id, index, insert) {
                Some(fix) if !self.exact_points.contains(&p) => crate::constrain_angle(
                    self.indicator.angle_geom,
                    fix,
                    p,
                    self.indicator.magnet(self.doc().settings.units),
                ),
                _ => p,
            };
        }
        let origin = if insert {
            self.snap(start)
        } else {
            self.doc()
                .get(id)
                .and_then(|o| edit_points(o).get(index).copied())
                .unwrap_or(start)
        };
        origin + constrain_delta(p - origin)
    }

    /// The fixed point Angle Geom constrains a dragged point from: the
    /// neighbour of an end point of an open polyline or spline, or the end a
    /// point is added to. Points in the middle or of closed shapes have
    /// none (xfig's `force_noanglegeom`).
    fn angle_anchor(&self, id: ObjectId, index: usize, insert: bool) -> Option<Point> {
        let o = self.doc().get(id)?;
        let pts = match &o.kind {
            ObjectKind::Polyline(pl) if pl.kind == PolyKind::Polyline => &pl.points,
            ObjectKind::Spline(sp) if !sp.closed => &sp.points,
            _ => return None,
        };
        let n = pts.len();
        if insert {
            if index == 0 {
                pts.first().copied()
            } else if index == n {
                pts.last().copied()
            } else {
                None
            }
        } else if n >= 2 && index == 0 {
            pts.get(1).copied()
        } else if n >= 2 && index + 1 == n {
            pts.get(n - 2).copied()
        } else {
            None
        }
    }

    fn commit_point(
        &mut self,
        id: ObjectId,
        index: usize,
        insert: bool,
        start: Point,
        pos: Point,
        constrain: bool,
    ) -> Result<(), EngineError> {
        let p = self.point_target(id, index, insert, start, pos, constrain);
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
        if let ToolEvent::Press { pos, button, .. } = ev {
            let tol = self.pick_tol();
            if matches!(self.tool_state, ToolState::RubberBand { .. }) {
                // Only the button that opened the region closes it.
                if button == Button::Right {
                    return self.region_click(pos, Band::MoveRegion, Button::Right);
                }
                return Ok(());
            }
            if button == Button::Right {
                if self.tool_busy() {
                    self.tool_state = ToolState::Idle;
                } else if !copy {
                    return self.region_click(pos, Band::MoveRegion, Button::Right);
                } else if copy {
                    // xfig: copy the object to the cut buffer.
                    if let Some(id) = self.pick(pos, tol) {
                        self.set_selection(vec![id]);
                        self.execute("edit.copy", &json!({"ids": [id.0]}))?;
                    }
                }
                return Ok(());
            }
            let constrain = button == Button::Middle;
            if let ToolState::MoveObjs {
                ids,
                start,
                dragging: false,
                copy,
                constrain: c,
                ..
            } = self.tool_state.clone()
            {
                if c != constrain {
                    return Ok(());
                }
                self.tool_state = ToolState::Idle;
                return self.commit_move(&ids, start, pos, copy, c);
            }
            if let Some(id) = self.pick(pos, tol) {
                self.set_selection(vec![id]);
                self.tool_state = ToolState::MoveObjs {
                    ids: vec![id],
                    start: pos,
                    cursor: pos,
                    copy,
                    dragging: true,
                    constrain,
                };
            }
            return Ok(());
        }
        self.drag_common(ev)
    }

    // ---- point editing -----------------------------------------------

    fn point_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        if let ToolEvent::Press { pos, button, .. } = ev {
            if button == Button::Right {
                self.tool_state = ToolState::Idle;
                return Ok(());
            }
            let constrain = button == Button::Middle;
            if constrain && self.tool == ToolKind::AddPoint {
                // W14: a point beyond the nearer end of an open line or spline.
                if self.tool_busy() {
                    return Ok(());
                }
                let tol = self.pick_tol();
                if let Some(id) = self.pick(pos, tol) {
                    let ends = match self.doc().get(id).map(|o| &o.kind) {
                        Some(ObjectKind::Polyline(pl)) if pl.kind == PolyKind::Polyline => Some(&pl.points),
                        Some(ObjectKind::Spline(sp)) if !sp.closed => Some(&sp.points),
                        _ => None,
                    };
                    if let Some(pts) = ends {
                        if let (Some(first), Some(last)) = (pts.first(), pts.last()) {
                            let index = if pos.dist(*first) < pos.dist(*last) {
                                0
                            } else {
                                pts.len()
                            };
                            self.tool_state = ToolState::PointDrag {
                                id,
                                index,
                                insert: true,
                                start: pos,
                                cursor: pos,
                                dragging: true,
                                constrain: false,
                            };
                        }
                    }
                }
                return Ok(());
            }
            if let ToolState::PointDrag {
                id,
                index,
                insert,
                start,
                dragging: false,
                constrain: c,
                ..
            } = self.tool_state.clone()
            {
                if c != constrain {
                    return Ok(());
                }
                self.tool_state = ToolState::Idle;
                return self.commit_point(id, index, insert, start, pos, c);
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
                    constrain,
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
            if !self.is_visible(o) {
                continue;
            }
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
        if let ToolEvent::Press { pos, button, .. } = ev {
            if button == Button::Right {
                self.tool_state = ToolState::Idle;
                return Ok(());
            }
            if let ToolState::Scale {
                id,
                center,
                start,
                button: b,
                ..
            } = self.tool_state.clone()
            {
                if b != button {
                    return Ok(());
                }
                self.tool_state = ToolState::Idle;
                return self.commit_scale(id, center, start, pos);
            }
            let tol = self.pick_tol();
            if let Some(id) = self.pick(pos, tol) {
                if let Some(o) = self.doc().get(id) {
                    let bb = o.geometry_bbox();
                    // Left: about the corner opposite the one nearest the
                    // click; middle: about the centre (xfig's scale modes).
                    let center = if button == Button::Middle {
                        bb.center()
                    } else {
                        let corners = bb.corners();
                        let nearest = (0..4)
                            .min_by(|a, b| {
                                let da = corners.get(*a).map_or(f64::MAX, |c| c.dist(pos));
                                let db = corners.get(*b).map_or(f64::MAX, |c| c.dist(pos));
                                da.total_cmp(&db)
                            })
                            .unwrap_or(0);
                        corners.get((nearest + 2) % 4).copied().unwrap_or(bb.center())
                    };
                    self.set_selection(vec![id]);
                    self.tool_state = ToolState::Scale {
                        id,
                        center,
                        start: pos,
                        cursor: pos,
                        button,
                    };
                }
            }
            return Ok(());
        }
        self.drag_common(ev)
    }

    // ---- glue (tag objects, then glue them into a compound) ------------

    fn glue_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        if let ToolEvent::Press { pos, button, .. } = ev {
            let tol = self.pick_tol();
            match button {
                Button::Right => {
                    if self.tool_busy() {
                        self.tool_state = ToolState::Idle;
                    } else if self.selection().len() >= 2 {
                        let ids = crate::cmd::ids_value(self.selection());
                        self.execute("object.group", &json!({ "ids": ids }))?;
                    }
                }
                Button::Middle => self.region_click(pos, Band::Tag, Button::Middle)?,
                Button::Left => {
                    if self.tool_busy() {
                        return Ok(());
                    }
                    match self.pick(pos, tol) {
                        Some(id) => {
                            // Tag or untag.
                            let mut sel = self.selection().to_vec();
                            match sel.iter().position(|s| *s == id) {
                                Some(i) => {
                                    sel.remove(i);
                                }
                                None => sel.push(id),
                            }
                            self.set_selection(sel);
                        }
                        None => {
                            self.tool_state = ToolState::RubberBand {
                                start: pos,
                                cursor: pos,
                                purpose: Band::Tag,
                                click: false,
                                button: Button::Left,
                                held: false,
                                lasso: Vec::new(),
                            };
                        }
                    }
                }
            }
            return Ok(());
        }
        self.drag_common(ev)
    }

    /// xfig's region by two clicks: the first opens a band, the second
    /// (with the same button) closes it.
    fn region_click(&mut self, pos: Point, purpose: Band, button: Button) -> Result<(), EngineError> {
        if let ToolState::RubberBand {
            start,
            purpose: p,
            click: true,
            button: b,
            ..
        } = self.tool_state
        {
            if b != button {
                return Ok(());
            }
            self.tool_state = ToolState::Idle;
            return self.finish_band(p, Region::Rect(Rect::new(start, pos)), pos);
        }
        self.tool_state = ToolState::RubberBand {
            start: pos,
            cursor: pos,
            purpose,
            click: true,
            button,
            held: true,
            lasso: vec![pos],
        };
        Ok(())
    }

    // ---- delete ----------------------------------------------------------

    fn delete_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        if let ToolEvent::Press { pos, button, .. } = ev {
            let tol = self.pick_tol();
            match button {
                Button::Middle => return self.region_click(pos, Band::Delete, Button::Middle),
                Button::Right if self.tool_busy() => self.tool_state = ToolState::Idle,
                Button::Left if self.tool_busy() => {}
                Button::Left | Button::Right => {
                    if let Some(id) = self.pick(pos, tol) {
                        // Right: xfig's "del to cut buf".
                        let cmd = if button == Button::Right {
                            "edit.cut"
                        } else {
                            "edit.delete"
                        };
                        self.set_selection(vec![id]);
                        self.execute(cmd, &json!({ "ids": [id.0] }))?;
                    }
                }
            }
            return Ok(());
        }
        self.drag_common(ev)
    }

    // ---- single-click editing tools ----------------------------------

    fn click_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let ToolEvent::Press { pos, button, shift } = ev else {
            return if self.tool_busy() { self.drag_common(ev) } else { Ok(()) };
        };
        let tol = self.pick_tol();
        let tool = self.tool;
        {
            // Functions on the buttons xfig leaves free (W13, W14).
            match self.tool_state.clone() {
                ToolState::RubberBand {
                    purpose,
                    button: opened_by,
                    ..
                } => {
                    if button == opened_by {
                        return self.region_click(pos, purpose, opened_by);
                    }
                    if button == Button::Right {
                        self.tool_state = ToolState::Idle;
                    }
                    return Ok(());
                }
                ToolState::AlignTo { reference } => {
                    match button {
                        Button::Left => self.tool_state = ToolState::Idle,
                        Button::Right => {
                            if let Some(id) = self.pick(pos, tol).filter(|i| *i != reference) {
                                self.set_selection(vec![id]);
                                self.execute("arrange.align_to", &json!({"reference": reference.0, "ids": [id.0]}))?;
                            }
                        }
                        Button::Middle => {}
                    }
                    return Ok(());
                }
                _ => {}
            }
            match (tool, button) {
                (ToolKind::Update, Button::Right) => return self.region_click(pos, Band::UpdateRegion, button),
                (ToolKind::DeletePoint, Button::Middle) => return self.region_click(pos, Band::DeletePoints, button),
                (ToolKind::Align, Button::Right) => {
                    if let Some(id) = self.pick(pos, tol) {
                        self.set_selection(vec![id]);
                        self.tool_state = ToolState::AlignTo { reference: id };
                    }
                    return Ok(());
                }
                _ => {}
            }
        }
        if tool == ToolKind::DeletePoint {
            if button == Button::Left {
                if let Some((id, index)) = self.find_vertex(pos, tol) {
                    self.execute("point.delete", &json!({"id": id.0, "index": index}))?;
                }
            }
            return Ok(());
        }
        if tool == ToolKind::Edit {
            match button {
                // xfig `e_edit.c`: the middle button edits the figure's
                // comments; with Shift it shows an object's comments.
                Button::Middle if !shift => {
                    self.request(UiRequest::EditFigureComments);
                    return Ok(());
                }
                Button::Middle => {
                    let text = match self.pick(pos, tol).and_then(|id| self.doc().get(id)) {
                        Some(o) if !o.comments.is_empty() => format!("Comments: {}", o.comments.join(" / ")),
                        Some(_) => "Comments: (None)".to_string(),
                        None => return Ok(()),
                    };
                    self.set_status(text);
                    return Ok(());
                }
                Button::Right => {
                    let hit = self.find_vertex(pos, tol);
                    let Some((id, index)) = hit else { return Ok(()) };
                    match self.doc().get(id).map(|o| &o.kind) {
                        Some(ObjectKind::Spline(s)) => {
                            if !s.closed && (index == 0 || index + 1 == s.points.len()) {
                                self.set_status("Cannot edit end-points".to_string());
                            } else {
                                self.request(UiRequest::EditSplinePoint { id, index });
                            }
                        }
                        _ => self.set_status("Only spline points can be edited".to_string()),
                    }
                    return Ok(());
                }
                _ => {}
            }
        }
        let Some(id) = self.pick(pos, tol) else {
            return Ok(());
        };
        let ids = json!([id.0]);
        let is_compound = matches!(self.doc().get(id).map(|o| &o.kind), Some(ObjectKind::Compound(_)));
        let copy = button == Button::Middle;
        match (tool, button) {
            (ToolKind::Break, Button::Left | Button::Middle) if is_compound => {
                self.set_selection(vec![id]);
                self.execute("object.ungroup", &json!({ "ids": ids }))?;
                // Middle: "break and tag" keeps the parts tagged (selected).
                if button == Button::Left {
                    self.set_selection(Vec::new());
                }
            }
            (ToolKind::Break, Button::Right) if is_compound => {
                self.set_selection(vec![id]);
                self.execute("object.break_all", &json!({ "ids": ids }))?;
            }
            (ToolKind::Arrow, Button::Right) => {
                let has = matches!(
                    self.doc().get(id).and_then(Object::arrows),
                    Some((f, b)) if f.is_some() || b.is_some()
                );
                if has {
                    self.set_selection(vec![id]);
                    self.execute("object.reverse_arrows", &json!({ "ids": ids }))?;
                } else {
                    self.set_status("The object has no arrow to reverse");
                }
            }
            (ToolKind::Align, Button::Left) if is_compound => {
                // Align the members of a compound with each other.
                self.set_selection(vec![id]);
                self.execute("object.ungroup", &json!({ "ids": ids }))?;
                let freed = crate::cmd::ids_value(self.selection());
                self.execute("arrange.align", &json!({ "ids": freed }))?;
                self.execute("object.group", &json!({ "ids": freed }))?;
            }
            (ToolKind::Update, Button::Left) => {
                self.set_selection(vec![id]);
                self.execute("object.update", &json!({ "ids": ids }))?;
            }
            (ToolKind::Update, Button::Middle) if !is_compound => {
                // xfig's "update settings": the object's attributes become
                // the current ones.
                if let Some(o) = self.doc().get(id).cloned() {
                    self.indicator.take_from(&o);
                }
            }
            (ToolKind::Edit, Button::Left) => {
                self.set_selection(vec![id]);
                self.request(UiRequest::EditObject { id });
            }
            (ToolKind::FlipV | ToolKind::FlipH, Button::Left | Button::Middle) => {
                // About the clicked point, like xfig; middle flips a copy.
                let cmd = if tool == ToolKind::FlipV {
                    "transform.flip_vertical"
                } else {
                    "transform.flip_horizontal"
                };
                self.set_selection(vec![id]);
                self.execute(cmd, &json!({ "ids": ids, "cx": pos.x, "cy": pos.y, "copy": copy }))?;
            }
            (ToolKind::RotateCcw | ToolKind::RotateCw, Button::Left | Button::Middle) => {
                let mut a = self.indicator.rotate_angle;
                if (tool == ToolKind::RotateCw) != shift {
                    a = -a;
                }
                self.set_selection(vec![id]);
                self.execute("transform.rotate", &json!({ "ids": ids, "angle": a, "copy": copy }))?;
            }
            (ToolKind::Convert, Button::Left) => {
                self.set_selection(vec![id]);
                self.execute("object.convert", &json!({ "ids": ids }))?;
            }
            (ToolKind::Convert, Button::Right) | (ToolKind::OpenClose, Button::Left) => {
                self.set_selection(vec![id]);
                self.execute("object.toggle_closed", &json!({ "ids": ids }))?;
            }
            (ToolKind::Arrow, Button::Left | Button::Middle) => {
                let Some(o) = self.doc().get(id) else { return Ok(()) };
                let Some((fwd, back)) = o.arrows() else { return Ok(()) };
                let pts = rustfig_model::flatten_object(o);
                let Some(path) = pts.first() else { return Ok(()) };
                let (Some(first), Some(last)) = (path.pts.first(), path.pts.last()) else {
                    return Ok(());
                };
                // The forward arrow sits at the last point.
                let at_end = pos.dist(*last) <= pos.dist(*first);
                let present = if at_end { fwd.is_some() } else { back.is_some() };
                // xfig: left adds an arrow, middle removes one.
                let toggle = match button {
                    Button::Left => !present,
                    _ => present,
                };
                if toggle {
                    let cmd = if at_end {
                        "object.toggle_forward_arrow"
                    } else {
                        "object.toggle_backward_arrow"
                    };
                    self.set_selection(vec![id]);
                    self.execute(cmd, &json!({ "ids": ids }))?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    // ---- mouse hints -------------------------------------------------------

    /// The current function of each mouse button, for the mouse indicator.
    /// Must describe what [`Engine::tool_event`] actually does: a test
    /// checks every tool and state. The wording is xfig's
    /// (`set_mousefun` calls in xfig's `d_*.c` and `e_*.c`).
    pub fn mouse_hints(&self) -> MouseHints {
        self.mouse_hints_with(false)
    }

    /// [`Engine::mouse_hints`] while Shift is (or is not) held: the buttons
    /// that do something else with Shift show that instead (xfig's
    /// `draw_shift_mousefun_canvas`). xfig's other Shift labels belong to
    /// functions not ported yet: "Locate Object" (S4-1) and "dimension
    /// line" (D10).
    pub fn mouse_hints_with(&self, shift: bool) -> MouseHints {
        let mut h = self.mouse_hints_plain();
        if self.tool == ToolKind::Chop {
            h.shift = "Shift: left trims the clicked piece, middle splits at the point, right extends a line end";
        }
        if shift {
            let (l, m, r) = self.shift_hints();
            for (slot, s) in [(&mut h.left, l), (&mut h.middle, m), (&mut h.right, r)] {
                if !s.is_empty() {
                    *slot = s;
                }
            }
        }
        h
    }

    /// The Shift meanings of the buttons in the current tool and state;
    /// empty where Shift changes nothing.
    fn shift_hints(&self) -> (&'static str, &'static str, &'static str) {
        match (&self.tool_state, self.tool) {
            (ToolState::Points { freehand: false, .. }, _) => ("del point", "", ""),
            (ToolState::Idle | ToolState::Chop { .. }, ToolKind::Chop) => ("trim piece", "split here", "extend line"),
            (ToolState::Idle, ToolKind::Select) => ("add/remove", "", ""),
            (ToolState::Idle, ToolKind::Box | ToolKind::ArcBox | ToolKind::Picture) => ("", "square at center", ""),
            (ToolState::Idle, ToolKind::Edit) => ("", "show comments", ""),
            (ToolState::Idle, ToolKind::RotateCcw | ToolKind::RotateCw) => ("rotate other way", "copy, other way", ""),
            _ => ("", "", ""),
        }
    }

    fn mouse_hints_plain(&self) -> MouseHints {
        let h = |left, middle, right| MouseHints {
            left,
            middle,
            right,
            shift: "",
        };
        // A label on the button that started the operation.
        let on = |button: Button, label, right| match button {
            Button::Middle => h("", label, right),
            _ => h(label, "", right),
        };
        let tool = self.tool;
        match &self.tool_state {
            ToolState::Points {
                pts, freehand, center, ..
            } => {
                if *freehand {
                    h("", "final point", "cancel")
                } else if tool == ToolKind::Arc {
                    let left = match (center.is_some(), pts.len()) {
                        (true, 0) => "first point",
                        (true, 1) => "direction",
                        (true, _) => "final angle",
                        (false, 0 | 1) => "mid point",
                        (false, _) => "final point",
                    };
                    h(left, "", "cancel")
                } else {
                    h("next point", "final point", "cancel")
                }
            }
            ToolState::Drag { kind, button, .. } => {
                // xfig finishes an ellipse with either button (`d_ellipse.c`).
                match kind {
                    _ if *button == Button::Right => h("", "", "final point"),
                    ToolKind::EllipseRadii => h("Ellipse corner", "Ellipse corner", "cancel"),
                    ToolKind::EllipseDiameter => h("final corner", "final corner", "cancel"),
                    ToolKind::CircleRadius => on(*button, "set radius", "cancel"),
                    _ => on(*button, "final point", "cancel"),
                }
            }
            ToolState::Text { .. } => MouseHints {
                left: "new text",
                middle: "finish text",
                right: "cancel",
                shift: "",
            },
            ToolState::MoveObjs {
                dragging, constrain, ..
            } => {
                let b = if *constrain { Button::Middle } else { Button::Left };
                on(b, if *dragging { "drop" } else { "place object" }, "cancel")
            }
            ToolState::ThreePoints { pts, .. } => h("", "", if pts.len() < 2 { "second point" } else { "final point" }),
            ToolState::AlignTo { .. } => h("cancel", "", "align to first"),
            ToolState::Join { spline, .. } => h(
                if *spline {
                    "Choose next Spline"
                } else {
                    "Choose next Line"
                },
                "",
                "cancel",
            ),
            ToolState::Chop { .. } => h("Select axe object", "Select log object", "Clear axe list"),
            ToolState::RubberBand {
                held: true,
                button: Button::Right,
                lasso,
                ..
            } if lasso.len() >= 2 => h("", "", "release: close lasso"),
            ToolState::RubberBand {
                held: true,
                button: Button::Middle,
                lasso,
                ..
            } if lasso.len() >= 2 => h("", "release: close lasso", "cancel"),
            ToolState::RubberBand {
                click: true,
                button: Button::Right,
                ..
            } => h("", "", "final corner"),
            ToolState::RubberBand { click: true, .. } => h("", "final corner", "cancel"),
            ToolState::RubberBand { .. } => h("drag region", "", "cancel"),
            ToolState::PointDrag {
                dragging,
                constrain,
                insert,
                ..
            } => {
                let b = if *constrain { Button::Middle } else { Button::Left };
                let place = if *insert { "place new point" } else { "new posn" };
                on(b, if *dragging { "drop point" } else { place }, "cancel")
            }
            ToolState::Scale { button, .. } => on(*button, "new posn", "cancel"),
            ToolState::Idle => {
                let glue = if self.selection().len() >= 2 {
                    "compound tagged"
                } else {
                    ""
                };
                let (l, m, r) = match tool {
                    ToolKind::Select => ("select / drag", "", ""),
                    ToolKind::Polyline => ("first point", "freehand", "single point"),
                    ToolKind::Polygon => ("first point", "freehand", ""),
                    t if t.spline_kind().is_some() => ("first point", "freehand", ""),
                    ToolKind::Arc => ("first point", "center point", ""),
                    ToolKind::CircleRadius | ToolKind::EllipseRadii => ("Circle center", "Ellipse center", ""),
                    ToolKind::CircleDiameter | ToolKind::EllipseDiameter => ("Circle diameter", "Ellipse corner", ""),
                    ToolKind::Box | ToolKind::ArcBox | ToolKind::Picture => ("corner point", "", ""),
                    ToolKind::RegularPolygon => ("center point", "", ""),
                    ToolKind::Text => ("position cursor", "", ""),
                    ToolKind::Glue => ("tag object", "tag region (drag: lasso)", glue),
                    ToolKind::Break => ("break compound", "break and tag", ""),
                    ToolKind::OpenCompound => ("open compound", "open, keep visible", ""),
                    ToolKind::JoinSplit => ("Join Lines/Splines", "Split Line/Spline", ""),
                    ToolKind::Chop => ("Select axe object", "Select log object", "Clear axe list"),
                    ToolKind::Scale => ("scale box", "scale about center", ""),
                    ToolKind::Align => ("align compound", "", ""),
                    ToolKind::Move => ("move object", "horiz/vert move", ""),
                    ToolKind::Copy => ("copy object", "horiz/vert copy", "copy to cut buf"),
                    ToolKind::MovePoint => ("move point", "horiz/vert move", ""),
                    ToolKind::AddPoint => ("break/add here", "", ""),
                    ToolKind::DeletePoint => ("delete point", "", ""),
                    ToolKind::Delete => ("delete object", "delete region (drag: lasso)", "del to cut buf"),
                    ToolKind::Update => ("update object", "update settings", ""),
                    ToolKind::Edit => ("edit object", "edit Main comment", "edit point"),
                    ToolKind::FlipV | ToolKind::FlipH => ("flip", "copy & flip", ""),
                    ToolKind::RotateCcw | ToolKind::RotateCw => ("rotate object", "copy & rotate", ""),
                    ToolKind::Convert => ("spline<->line", "", "open<->closed"),
                    ToolKind::OpenClose => ("open/close", "", ""),
                    ToolKind::Arrow => ("add arrow", "delete arrow", ""),
                    ToolKind::Tangent => ("add tangent", "add normal", ""),
                    _ => ("", "", ""),
                };
                // The functions on buttons xfig leaves free (W13, W14).
                let (m, r) = match tool {
                    ToolKind::Update => (m, "update region (drag: lasso)"),
                    ToolKind::Move => (m, "move region (drag: lasso)"),
                    ToolKind::DeletePoint => ("delete in region (drag: lasso)", r),
                    ToolKind::AddPoint => ("add at end", r),
                    ToolKind::Arrow => (m, "reverse arrows"),
                    ToolKind::Break => (m, "break all levels"),
                    ToolKind::Align => (m, "align to first"),
                    ToolKind::CircleRadius | ToolKind::CircleDiameter => (m, "circle by 3 points"),
                    ToolKind::Picture => ("center point", "keep aspect"),
                    ToolKind::Box | ToolKind::ArcBox => ("center point", r),
                    ToolKind::RegularPolygon => ("one side", r),
                    ToolKind::Text if self.clipboard_text().is_some() => ("paste as text", r),
                    _ => (m, r),
                };
                h(l, m, r)
            }
        }
    }

    // ---- overlay -----------------------------------------------------------

    /// What to draw on top of the document for the active tool.
    pub fn overlay(&self) -> Overlay {
        let mut ov = Overlay::default();
        match &self.tool_state {
            ToolState::Idle | ToolState::Join { .. } => {}
            ToolState::Chop { axes } => {
                for id in axes {
                    if let Some(o) = self.doc().get(*id) {
                        let bb = o.geometry_bbox();
                        let mut c = bb.corners().to_vec();
                        if let Some(f) = c.first().copied() {
                            c.push(f);
                        }
                        ov.guides.push(c);
                    }
                }
            }
            ToolState::Points {
                pts, cursor, center, ..
            } => {
                let mut all = pts.clone();
                all.push(*cursor);
                if self.tool == ToolKind::Arc && all.len() < 3 {
                    match center {
                        // Radii from the centre to each point.
                        Some(c) => ov.guides.extend(all.iter().map(|p| vec![*c, *p])),
                        None => ov.guides.push(all.clone()),
                    }
                } else if let Some(o) = self.points_object(&all, *center) {
                    ov.preview.push(o);
                }
                if self.tool.spline_kind().is_some() {
                    ov.guides.push(all.clone());
                }
                ov.handles = pts.clone();
            }
            ToolState::Drag {
                start,
                cursor,
                kind,
                button,
                square,
                ..
            } => {
                let (a, b) = self.drag_span(*kind, *button, *square, *start, *cursor);
                if let Some(o) = self.drag_object(*kind, a, b) {
                    ov.preview.push(o);
                }
            }
            ToolState::ThreePoints { pts, cursor } => {
                let mut all = pts.clone();
                all.push(*cursor);
                if let [a, b, c] = all.as_slice() {
                    if let Some((m, r)) = crate::cmd::circumcircle(*a, *b, *c) {
                        let kind = if self.tool == ToolKind::CircleDiameter {
                            EllipseKind::CircleDiameter
                        } else {
                            EllipseKind::CircleRadius
                        };
                        ov.preview
                            .push(make_ellipse(self.indicator.style(), m, r, r, 0.0, kind));
                    }
                }
                ov.guides.push(all);
                ov.handles = pts.clone();
            }
            ToolState::AlignTo { reference } => {
                if let Some(o) = self.doc().get(*reference) {
                    let bb = o.geometry_bbox();
                    let mut c = bb.corners().to_vec();
                    if let Some(f) = c.first().copied() {
                        c.push(f);
                    }
                    ov.guides.push(c);
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
                constrain,
                ..
            } => {
                let d = self.move_delta(*start, *cursor, *constrain);
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
                if d.len() > 0.0 {
                    // Smart links: every attached line follows while dragging
                    // (a copy leaves the original line where it is: xfig
                    // copies it and stretches the copy's twin).
                    for line in crate::links::preview(self, ids, d.x, d.y) {
                        if !copy {
                            ov.hide.push(line.id);
                        }
                        ov.preview.push(line);
                    }
                }
            }
            ToolState::RubberBand { held: true, lasso, .. } if lasso.len() >= 2 => {
                let mut c = lasso.clone();
                if let Some(f) = c.first().copied() {
                    c.push(f);
                }
                ov.guides.push(c);
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
                start,
                cursor,
                constrain,
                ..
            } => {
                if let Some(o) = self.doc().get(*id) {
                    let p = self.point_target(*id, *index, *insert, *start, *cursor, *constrain);
                    let mut c = o.clone();
                    preview_point(&mut c, *index, *insert, p);
                    ov.handles = edit_points(&c);
                    // Smart links: a box corner dragged stretches the lines
                    // attached to its edges.
                    if let (Some(old), ObjectKind::Polyline(pl)) = (rect_of(o), &c.kind) {
                        if !*insert && pl.kind.is_rect() {
                            let new = Rect::from_points(pl.points.iter().copied());
                            let links = crate::links::collect(self, &[*id]);
                            for line in crate::links::preview_with(self, &links, &crate::links::rect_map(old, new)) {
                                ov.hide.push(line.id);
                                ov.preview.push(line);
                            }
                        }
                    }
                    ov.preview.push(c);
                    ov.hide.push(*id);
                }
            }
            ToolState::Scale {
                id,
                center,
                start,
                cursor,
                ..
            } => {
                if let Some(o) = self.doc().get(*id) {
                    let d0 = start.dist(*center);
                    if d0 > 1e-6 {
                        let f = (cursor.dist(*center) / d0).clamp(1e-3, 1e3);
                        let mut c = o.clone();
                        let t = Affine::about(Affine::scale(f, f), *center);
                        c.transform(&t, true);
                        ov.preview.push(c);
                        // Smart links: the attached lines follow the scaling.
                        let links = crate::links::collect(self, &[*id]);
                        for line in crate::links::preview_with(self, &links, &|q| t.apply(q)) {
                            ov.hide.push(line.id);
                            ov.preview.push(line);
                        }
                        ov.hide.push(*id);
                    }
                    ov.guides.push(vec![*center, *cursor]);
                }
            }
        }
        ov.join_marker = self.join_marker();
        ov.markers = self.mode_markers();
        if ov.handles.is_empty() && matches!(self.tool_state, ToolState::Idle) && self.tool == ToolKind::Select {
            let ids = self.selection().to_vec();
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
        // Smart links: the selected shapes show the line ends attached to
        // them (the ends that will follow).
        if matches!(self.tool_state, ToolState::Idle) && self.tool == ToolKind::Select {
            let ids = self.selection();
            if ids.len() <= 2000 {
                let ends = crate::links::attached_ends(self, ids);
                ov.handles.extend(ends);
            }
        }
        ov
    }
}

/// The box of a box-like polyline (box, arc-box, picture).
fn rect_of(o: &Object) -> Option<Rect> {
    match &o.kind {
        ObjectKind::Polyline(pl) if pl.kind.is_rect() => Some(Rect::from_points(pl.points.iter().copied())),
        _ => None,
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
            button: Button::Left,
            shift: false,
        })
        .unwrap();
        e.tool_event(ToolEvent::Release {
            pos: Point::new(x, y),
            button: Button::Left,
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
            button: Button::Left,
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
            button: Button::Left,
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
    fn home_and_end_move_the_caret() {
        let mut e = Engine::new();
        e.set_tool(ToolKind::Text);
        press(&mut e, 600.0, 600.0);
        e.tool_event(ToolEvent::Text("ell".into())).unwrap();
        e.tool_event(ToolEvent::Key(Key::Home)).unwrap();
        e.tool_event(ToolEvent::Text("H".into())).unwrap();
        e.tool_event(ToolEvent::Key(Key::End)).unwrap();
        e.tool_event(ToolEvent::Text("o".into())).unwrap();
        e.tool_event(ToolEvent::Key(Key::Escape)).unwrap();
        let ObjectKind::Text(t) = &e.doc().objects[0].kind else {
            panic!()
        };
        assert_eq!(t.text, "Hello");
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
            button: Button::Left,
            shift: false,
        })
        .unwrap();
        e.tool_event(ToolEvent::Move {
            pos: Point::new(900.0, 600.0),
        })
        .unwrap();
        e.tool_event(ToolEvent::Release {
            pos: Point::new(900.0, 600.0),
            button: Button::Left,
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
            button: Button::Left,
            shift: false,
        })
        .unwrap();
        e.tool_event(ToolEvent::Move {
            pos: Point::new(1000.0, 1000.0),
        })
        .unwrap();
        e.tool_event(ToolEvent::Release {
            pos: Point::new(1000.0, 1000.0),
            button: Button::Left,
        })
        .unwrap();
        assert_eq!(e.selection().len(), 1);
    }

    fn click(e: &mut Engine, x: f64, y: f64, button: Button) {
        let pos = Point::new(x, y);
        e.tool_event(ToolEvent::Press {
            pos,
            button,
            shift: false,
        })
        .unwrap();
        e.tool_event(ToolEvent::Release { pos, button }).unwrap();
    }

    fn objects_of(e: &Engine) -> Vec<&ObjectKind> {
        e.doc().objects.iter().map(|o| &o.kind).collect()
    }

    #[test]
    fn starts_with_no_mode_like_xfig() {
        let mut e = Engine::new();
        assert_eq!(e.tool(), ToolKind::None);
        assert_eq!(e.status(), "READY. Select a mode or load a file");
        e.execute(
            "shape.rectangle",
            &json!({"x": 0, "y": 0, "width": 1200, "height": 1200}),
        )
        .unwrap();
        for b in [Button::Left, Button::Middle, Button::Right] {
            click(&mut e, 0.0, 0.0, b);
        }
        assert!(!e.tool_busy());
        assert!(e.selection().len() <= 1 && e.doc().objects.len() == 1);
        let h = e.mouse_hints();
        assert!(h.left.is_empty() && h.right.is_empty());
    }

    #[test]
    fn shift_hints_show_what_shift_does() {
        let mut e = Engine::new();
        e.set_tool(ToolKind::Chop);
        let h = e.mouse_hints_with(true);
        assert_eq!((h.left, h.middle, h.right), ("trim piece", "split here", "extend line"));
        assert_eq!(e.mouse_hints_with(false).left, "Select axe object");
        // Shift+left removes the last point of a line being drawn.
        e.set_tool(ToolKind::Polyline);
        click(&mut e, 0.0, 0.0, Button::Left);
        click(&mut e, 600.0, 0.0, Button::Left);
        assert_eq!(e.mouse_hints_with(true).left, "del point");
        assert_eq!(e.mouse_hints_with(true).middle, "final point");
        let pos = Point::new(900.0, 0.0);
        e.tool_event(ToolEvent::Press {
            pos,
            button: Button::Left,
            shift: true,
        })
        .unwrap();
        let ToolState::Points { pts, .. } = &e.tool_state else {
            panic!("still drawing")
        };
        assert_eq!(pts.len(), 1);
        // Modes without Shift meanings keep their labels.
        e.set_tool(ToolKind::Delete);
        assert_eq!(e.mouse_hints_with(true), e.mouse_hints());
    }

    #[test]
    fn mouse_hints_follow_state() {
        let mut e = Engine::new();
        for t in ToolKind::ALL {
            e.set_tool(t);
            let h = e.mouse_hints();
            assert!(!h.left.is_empty(), "{} has no left-button hint", t.id());
        }
        e.set_tool(ToolKind::Polyline);
        assert_eq!(e.mouse_hints().left, "first point");
        assert_eq!(e.mouse_hints().middle, "freehand");
        press(&mut e, 0.0, 0.0);
        assert_eq!(e.mouse_hints().middle, "final point");
        assert_eq!(e.mouse_hints().right, "cancel");
        e.set_tool(ToolKind::Text);
        press(&mut e, 0.0, 0.0);
        assert_eq!(e.mouse_hints().left, "new text");
    }

    /// What a button press can change.
    fn snapshot(e: &Engine) -> String {
        format!(
            "{} {:?} {:?} {} {} {}",
            e.revision(),
            e.selection(),
            e.tool_state,
            serde_json::to_string(&e.indicator).unwrap(),
            e.clipboard().len(),
            e.requests.len()
        )
    }

    /// Documents for the hint check; each has an object at (0, 0).
    fn fixture(n: usize) -> Engine {
        let mut e = Engine::new();
        match n {
            0 | 1 => {
                e.execute(
                    "shape.polyline",
                    &json!({"points": [[0, 0], [1200, 0], [1200, 1200]], "thickness": 3}),
                )
                .unwrap();
                if n == 1 {
                    e.execute("object.toggle_forward_arrow", &json!({})).unwrap();
                    e.execute("object.toggle_backward_arrow", &json!({})).unwrap();
                }
            }
            3 => {
                // A spline whose middle control point is at (0, 0).
                e.execute(
                    "shape.polyline",
                    &json!({"points": [[1200, -1200], [0, 0], [1200, 1200]]}),
                )
                .unwrap();
                e.execute("object.convert", &json!({})).unwrap();
            }
            4 => {
                // A text on the clipboard, for the paste function.
                e.execute("shape.text", &json!({"x": 0, "y": 0, "text": "abc"}))
                    .unwrap();
                e.execute("edit.copy", &json!({})).unwrap();
            }
            5 => {
                // A line with a vertex at (0, 0): Join/Split can split it there.
                e.execute(
                    "shape.polyline",
                    &json!({"points": [[-1200, 0], [0, 0], [1200, 0], [1200, 1200]]}),
                )
                .unwrap();
            }
            6 => {
                // Two lines whose end points are at (0, 0) and (600, 900).
                e.execute("shape.polyline", &json!({"points": [[0, 0], [0, -1200]]}))
                    .unwrap();
                e.execute("shape.polyline", &json!({"points": [[600, 900], [1800, 900]]}))
                    .unwrap();
            }
            7 => {
                // Two crossing lines: an axe through (0, 0) and a log through
                // (600, 900) that intersect, so Chop's middle button cuts.
                e.execute("shape.polyline", &json!({"points": [[0, 0], [0, 1200]]}))
                    .unwrap();
                e.execute("shape.polyline", &json!({"points": [[-600, 900], [1800, 900]]}))
                    .unwrap();
            }
            _ => {
                e.execute(
                    "shape.rectangle",
                    &json!({"x": 0, "y": 0, "width": 1200, "height": 600}),
                )
                .unwrap();
                e.execute("shape.circle", &json!({"cx": 2400, "cy": 2400, "r": 300}))
                    .unwrap();
                e.execute("edit.select_all", &json!({})).unwrap();
                e.execute("object.group", &json!({})).unwrap();
            }
        }
        e
    }

    /// The indicator must tell the truth: in every tool and state,
    /// a button with no label does nothing, and a labelled button does
    /// something in at least one of the fixture documents.
    #[test]
    fn mouse_hints_match_behaviour() {
        use std::collections::BTreeMap;
        const BUTTONS: [Button; 3] = [Button::Left, Button::Middle, Button::Right];
        let p = (0.0, 0.0);
        let q = (600.0, 900.0);
        // (tool, how the state was reached, button, label) -> acted
        let mut acted: BTreeMap<(&str, String, String, &str), bool> = BTreeMap::new();
        {
            for tool in ToolKind::ALL {
                for n in 0..8 {
                    let setup = |prefix: Option<Button>| {
                        let mut e = fixture(n);
                        e.set_tool(tool);
                        if let Some(b) = prefix {
                            click(&mut e, p.0, p.1, b);
                        }
                        e
                    };
                    for prefix in [None, Some(Button::Left), Some(Button::Middle)] {
                        let base = setup(prefix);
                        if prefix.is_some() && !base.tool_busy() {
                            continue;
                        }
                        let at = if prefix.is_some() { q } else { p };
                        let h = base.mouse_hints();
                        for (b, label) in BUTTONS.iter().zip([h.left, h.middle, h.right]) {
                            let mut e = setup(prefix);
                            let before = snapshot(&e);
                            let _ = e.tool_event(ToolEvent::Press {
                                pos: Point::new(at.0, at.1),
                                button: *b,
                                shift: false,
                            });
                            let changed = snapshot(&e) != before;
                            let what = format!("{} after {prefix:?}, {b:?} ({label:?}) in fixture {n}", tool.id());
                            if label.is_empty() {
                                assert!(!changed, "{what}: no label, but the press changed something");
                            } else {
                                let key = (tool.id(), format!("{prefix:?}"), format!("{b:?}"), label);
                                *acted.entry(key).or_default() |= changed;
                            }
                        }
                    }
                }
            }
        }
        let silent: Vec<_> = acted.iter().filter(|(_, a)| !**a).map(|(k, _)| k).collect();
        assert!(silent.is_empty(), "labelled buttons that did nothing: {silent:#?}");
    }

    #[test]
    fn xfig_polyline_buttons() {
        let mut e = Engine::new();
        e.set_tool(ToolKind::Polyline);
        click(&mut e, 0.0, 0.0, Button::Left);
        click(&mut e, 1200.0, 0.0, Button::Left);
        click(&mut e, 1200.0, 1200.0, Button::Middle);
        assert_eq!(e.doc().objects[0].points().unwrap().len(), 3);
        // Right cancels.
        click(&mut e, 0.0, 0.0, Button::Left);
        click(&mut e, 600.0, 0.0, Button::Left);
        click(&mut e, 0.0, 0.0, Button::Right);
        assert_eq!(e.doc().objects.len(), 1);
        assert!(!e.tool_busy());
        // Shift+left removes the last point.
        click(&mut e, 0.0, 0.0, Button::Left);
        click(&mut e, 600.0, 0.0, Button::Left);
        e.tool_event(ToolEvent::Press {
            pos: Point::new(900.0, 0.0),
            button: Button::Left,
            shift: true,
        })
        .unwrap();
        click(&mut e, 0.0, 600.0, Button::Middle);
        assert_eq!(
            e.doc().objects[1].points().unwrap(),
            &[Point::ZERO, Point::new(0.0, 600.0)]
        );
        // Right at rest enters a single point.
        click(&mut e, 2400.0, 2400.0, Button::Right);
        assert_eq!(e.doc().objects[2].points().unwrap().len(), 1);
    }

    #[test]
    fn arc_type_and_text_step() {
        let mut e = Engine::new();
        e.indicator.pie_arc = true;
        e.set_tool(ToolKind::Arc);
        click(&mut e, 0.0, 0.0, Button::Left);
        click(&mut e, 600.0, 600.0, Button::Left);
        click(&mut e, 1200.0, 0.0, Button::Left);
        let ObjectKind::Arc(a) = objects_of(&e)[0] else {
            panic!()
        };
        assert!(a.pie);
        // Text step 2: the next line starts twice as far down.
        e.set_tool(ToolKind::Text);
        let start = |e: &mut Engine, step: f64| {
            e.indicator.text_step = step;
            click(e, 6000.0, 6000.0, Button::Left);
            e.tool_event(ToolEvent::Text("a".into())).unwrap();
            e.tool_event(ToolEvent::Key(Key::Enter)).unwrap();
            let ToolState::Text { origin, .. } = &e.tool_state else {
                panic!()
            };
            let dy = origin.y - 6000.0;
            e.tool_event(ToolEvent::Key(Key::Escape)).unwrap();
            dy
        };
        let one = start(&mut e, 1.0);
        let two = start(&mut e, 2.0);
        assert!((two - 2.0 * one).abs() < 1e-6, "{one} {two}");
    }

    #[test]
    fn xfig_text_buttons() {
        let mut e = Engine::new();
        e.set_tool(ToolKind::Text);
        // Middle finishes.
        click(&mut e, 600.0, 600.0, Button::Left);
        e.tool_event(ToolEvent::Text("Hi".into())).unwrap();
        click(&mut e, 0.0, 0.0, Button::Middle);
        assert_eq!(e.doc().objects.len(), 1);
        assert!(!e.tool_busy());
        // Right cancels: nothing new, and an edited text keeps its string.
        click(&mut e, 3000.0, 3000.0, Button::Left);
        e.tool_event(ToolEvent::Text("gone".into())).unwrap();
        click(&mut e, 0.0, 0.0, Button::Right);
        assert_eq!(e.doc().objects.len(), 1);
        click(&mut e, 650.0, 590.0, Button::Left);
        e.tool_event(ToolEvent::Text("!!".into())).unwrap();
        click(&mut e, 0.0, 0.0, Button::Right);
        let ObjectKind::Text(t) = objects_of(&e)[0] else {
            panic!()
        };
        assert_eq!(t.text, "Hi");
    }

    #[test]
    fn freehand_with_the_middle_button() {
        let mut e = Engine::new();
        e.set_tool(ToolKind::Polyline);
        click(&mut e, 0.0, 0.0, Button::Middle);
        for i in 1..=20 {
            let x = f64::from(i) * 60.0;
            e.tool_event(ToolEvent::Move {
                pos: Point::new(x, (x / 200.0).sin() * 300.0),
            })
            .unwrap();
        }
        click(&mut e, 1300.0, 0.0, Button::Middle);
        assert!(e.doc().objects[0].points().unwrap().len() > 10);
    }

    #[test]
    fn arc_by_center() {
        let mut e = Engine::new();
        e.set_tool(ToolKind::Arc);
        click(&mut e, 0.0, 0.0, Button::Middle);
        click(&mut e, 1200.0, 0.0, Button::Left);
        // Mid and final angles: only the direction from the centre counts.
        click(&mut e, 0.0, 300.0, Button::Left);
        click(&mut e, -5000.0, 0.0, Button::Left);
        let ObjectKind::Arc(a) = objects_of(&e)[0] else {
            panic!()
        };
        for p in a.points {
            assert!((p.len() - 1200.0).abs() < 1e-6, "{p:?} is not on the circle");
        }
        assert!(a.points[1].y > 0.0);
    }

    #[test]
    fn circle_mode_draws_ellipses_with_the_middle_button() {
        let mut e = Engine::new();
        e.set_tool(ToolKind::CircleRadius);
        click(&mut e, 0.0, 0.0, Button::Middle);
        click(&mut e, 1200.0, 600.0, Button::Middle);
        let ObjectKind::Ellipse(el) = objects_of(&e)[0] else {
            panic!()
        };
        assert_eq!(el.radii, Point::new(1200.0, 600.0));
        // xfig finishes an ellipse with either button (`d_ellipse.c`).
        click(&mut e, 0.0, 0.0, Button::Middle);
        click(&mut e, 600.0, 300.0, Button::Left);
        assert_eq!(e.doc().objects.len(), 2);
        assert!(e.undo());
        click(&mut e, 0.0, 0.0, Button::Left);
        click(&mut e, 1200.0, 600.0, Button::Left);
        let ObjectKind::Ellipse(el) = objects_of(&e)[1] else {
            panic!()
        };
        assert_eq!(el.radii.x, el.radii.y);
    }

    #[test]
    fn middle_button_moves_horizontally_or_vertically() {
        let mut e = fixture(0);
        e.set_tool(ToolKind::Move);
        click(&mut e, 0.0, 0.0, Button::Middle);
        click(&mut e, 300.0, 1200.0, Button::Middle);
        assert_eq!(e.doc().objects[0].points().unwrap()[0], Point::new(0.0, 1200.0));
        e.set_tool(ToolKind::MovePoint);
        click(&mut e, 0.0, 1200.0, Button::Middle);
        click(&mut e, 600.0, 1350.0, Button::Middle);
        assert_eq!(e.doc().objects[0].points().unwrap()[0], Point::new(600.0, 1200.0));
    }

    #[test]
    fn copy_and_flip_is_one_undo_step() {
        let mut e = fixture(0);
        e.set_tool(ToolKind::FlipH);
        click(&mut e, 0.0, 0.0, Button::Middle);
        assert_eq!(e.doc().objects.len(), 2);
        // Flipped about the clicked point.
        assert_eq!(e.doc().objects[1].points().unwrap()[1], Point::new(-1200.0, 0.0));
        assert!(e.undo());
        assert_eq!(e.doc().objects.len(), 1);
        e.set_tool(ToolKind::RotateCcw);
        click(&mut e, 0.0, 0.0, Button::Middle);
        assert_eq!(e.doc().objects.len(), 2);
    }

    #[test]
    fn glue_tags_then_glues() {
        let mut e = fixture(0);
        e.execute("shape.circle", &json!({"cx": 3000, "cy": 3000, "r": 100}))
            .unwrap();
        e.execute("shape.circle", &json!({"cx": 5000, "cy": 5000, "r": 100}))
            .unwrap();
        e.set_selection(vec![]);
        e.set_tool(ToolKind::Glue);
        click(&mut e, 0.0, 0.0, Button::Left);
        // Region by two middle clicks.
        click(&mut e, 2800.0, 2800.0, Button::Middle);
        click(&mut e, 3200.0, 3200.0, Button::Middle);
        assert_eq!(e.selection().len(), 2);
        assert_eq!(e.mouse_hints().right, "compound tagged");
        click(&mut e, 0.0, 0.0, Button::Right);
        assert_eq!(e.doc().objects.len(), 2);
        assert!(objects_of(&e)
            .iter()
            .any(|k| matches!(k, ObjectKind::Compound(c) if c.objects.len() == 2)));
    }

    #[test]
    fn delete_region_and_cut() {
        let mut e = fixture(0);
        e.execute("shape.circle", &json!({"cx": 3000, "cy": 3000, "r": 100}))
            .unwrap();
        e.set_tool(ToolKind::Delete);
        click(&mut e, 2800.0, 2800.0, Button::Middle);
        click(&mut e, 3200.0, 3200.0, Button::Middle);
        assert_eq!(e.doc().objects.len(), 1);
        click(&mut e, 0.0, 0.0, Button::Right);
        assert!(e.doc().objects.is_empty());
        assert_eq!(e.clipboard().len(), 1);
    }

    #[test]
    fn update_settings_takes_attributes() {
        let mut e = fixture(1);
        e.set_tool(ToolKind::Update);
        click(&mut e, 0.0, 0.0, Button::Middle);
        assert_eq!(e.indicator.thickness, 3);
        assert!(e.indicator.forward_arrow && e.indicator.backward_arrow);
    }

    #[test]
    fn arrows_add_with_left_and_remove_with_middle() {
        let mut e = fixture(0);
        e.set_tool(ToolKind::Arrow);
        click(&mut e, 1200.0, 1200.0, Button::Left);
        click(&mut e, 1200.0, 1200.0, Button::Left);
        assert!(e.doc().objects[0].arrows().unwrap().0.is_some());
        click(&mut e, 1200.0, 1200.0, Button::Middle);
        assert!(e.doc().objects[0].arrows().unwrap().0.is_none());
    }

    #[test]
    fn scale_about_opposite_corner_and_centre() {
        let mut e = Engine::new();
        e.execute(
            "shape.rectangle",
            &json!({"x": 0, "y": 0, "width": 1200, "height": 1200}),
        )
        .unwrap();
        e.set_tool(ToolKind::Scale);
        click(&mut e, 1200.0, 1200.0, Button::Left);
        click(&mut e, 600.0, 600.0, Button::Left);
        let bb = e.doc().objects[0].geometry_bbox();
        assert_eq!((bb.min, bb.max), (Point::ZERO, Point::new(600.0, 600.0)));
        click(&mut e, 600.0, 600.0, Button::Middle);
        click(&mut e, 900.0, 900.0, Button::Middle);
        let bb = e.doc().objects[0].geometry_bbox();
        assert_eq!((bb.min, bb.max), (Point::new(-300.0, -300.0), Point::new(900.0, 900.0)));
    }

    #[test]
    fn every_tool_survives_random_events() {
        for t in ToolKind::ALL.iter().copied() {
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
                    button: Button::Left,
                    shift: false,
                },
                ToolEvent::Move {
                    pos: Point::new(300.0, 300.0),
                },
                ToolEvent::Release {
                    pos: Point::new(300.0, 300.0),
                    button: Button::Left,
                },
                ToolEvent::Text("ab".into()),
                ToolEvent::Press {
                    pos: Point::new(600.0, 600.0),
                    button: Button::Left,
                    shift: true,
                },
                ToolEvent::Move {
                    pos: Point::new(f64::MAX, -1.0),
                },
                ToolEvent::DoubleClick {
                    pos: Point::new(600.0, 600.0),
                },
                ToolEvent::Press {
                    pos: Point::new(300.0, 0.0),
                    button: Button::Middle,
                    shift: false,
                },
                ToolEvent::Move {
                    pos: Point::new(900.0, 50.0),
                },
                ToolEvent::Release {
                    pos: Point::new(900.0, 50.0),
                    button: Button::Middle,
                },
                ToolEvent::Press {
                    pos: Point::new(f64::NAN, 0.0),
                    button: Button::Middle,
                    shift: true,
                },
                ToolEvent::Key(Key::Enter),
                ToolEvent::Key(Key::Backspace),
                ToolEvent::Press {
                    pos: Point::new(1200.0, 0.0),
                    button: Button::Right,
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
