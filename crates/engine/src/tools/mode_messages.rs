//! xfig's sentence for each mode (`mode_switches`, `w_modepanel.c`): the
//! balloon text of the mode button, and what xfig puts in the message panel
//! when the mode is selected.
// Ported from xfig src/w_modepanel.c (xfig licence, see LICENSE-xfig).

use super::ToolKind;

impl ToolKind {
    /// xfig's sentence with its key, e.g. "Rectangular BOX drawing   (b)";
    /// `None` for RustFig's own modes without one.
    pub fn xfig_message(self) -> Option<&'static str> {
        Some(match self {
            ToolKind::CircleRadius => "CIRCLE/ELLIPSE drawing: specify RADII   (c or e)",
            ToolKind::CircleDiameter => "CIRCLE/ELLIPSE drawing: specify DIAMETER(S)   (Shift-c or e)",
            ToolKind::SplineApproxClosed => "CLOSED APPROXIMATED SPLINE drawing: specify control points   (Shift-s)",
            ToolKind::SplineApprox => "APPROXIMATED SPLINE drawing: specify control points   (s)",
            ToolKind::SplineInterpClosed => "CLOSED INTERPOLATED SPLINE drawing: specify control points   (Shift-i)",
            ToolKind::SplineInterp => "INTERPOLATED SPLINE drawing: specify control points   (i)",
            ToolKind::Polygon => "POLYGON drawing   (p)",
            ToolKind::Polyline => "POLYLINE drawing   (l)",
            ToolKind::Box => "Rectangular BOX drawing   (b)",
            ToolKind::ArcBox => "Rectangular BOX drawing with ROUNDED CORNERS   (Shift-b)",
            ToolKind::RegularPolygon => "Regular Polygon   (Shift-p)",
            ToolKind::Arc => "ARC drawing: specify three points on the arc   (r)",
            ToolKind::Picture => "Picture Object   (Ctrl-p)",
            ToolKind::Text => "TEXT input from keyboard   (t)",
            ToolKind::Glue => "GLUE objects into COMPOUND object   (g)",
            ToolKind::Break => "BREAK COMPOUND object   (Shift-g)",
            ToolKind::OpenCompound => "OPEN COMPOUND object   (o)",
            ToolKind::JoinSplit => "Join or Split lines/splines   (j)",
            ToolKind::Chop => "Chop objects   (x)",
            ToolKind::Trim => "TRIM, split and extend lines   (w)",
            ToolKind::Tangent => "Add TANGENT/NORMAL to curve   (n)",
            ToolKind::Plot => "PLOT a function or data   (y)",
            ToolKind::AngleMeasure => "MEASURE angle (specify three points or select object)   (Ctrl-g)",
            ToolKind::LengthMeasure => "Measure LENGTH of polylines, arcs and ellipses   (Ctrl-n)",
            ToolKind::AreaMeasure => "Measure AREA of polygons, arcs and ellipses   (Ctrl-m)",
            ToolKind::Scale => "SCALE objects   (Ctrl-s)",
            ToolKind::Align => "ALIGN objects within a COMPOUND or to CANVAS   (a)",
            ToolKind::MovePoint => "MOVE POINTs   (Shift-m)",
            ToolKind::Move => "MOVE objects   (m)",
            ToolKind::AddPoint => "ADD POINTs to lines, polygons and splines   (Ctrl-a)",
            ToolKind::Copy => "COPY objects  (Ctrl-c)",
            ToolKind::DeletePoint => "DELETE POINTs from lines, polygons and splines   (Shift-d)",
            ToolKind::Delete => "DELETE objects   (d)",
            ToolKind::Update => "UPDATE object <-> current settings   (u)",
            ToolKind::Edit => "CHANGE OBJECT via EDIT panel   (Ctrl-e)",
            ToolKind::FlipV => "FLIP objects up or down   (f)",
            ToolKind::FlipH => "FLIP objects left or right   (Shift-f)",
            ToolKind::RotateCw => "ROTATE objects clockwise   (Ctrl-r)",
            ToolKind::RotateCcw => "ROTATE objects counter-clockwise   (Shift-r)",
            ToolKind::Convert => "CONVERSION between lines, polygons and splines   (v)",
            ToolKind::Arrow => "ADD/DELETE ARROWHEADS   (Shift-a)",
            _ => return None,
        })
    }
}
