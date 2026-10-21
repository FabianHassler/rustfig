//! Join and split lines and splines (plan D2).
//!
//! xfig's Join/Split mode (`e_joinsplit.c`): Join connects two open
//! polylines or two open splines at the end points the user picked (picking
//! both ends of one line closes it into a polygon, of one spline into a
//! closed spline); Split cuts a polyline or spline at a vertex or on a
//! segment (a polygon, box or closed spline is opened there).
//!
//! Where xfig is silent or odd, the choices are: the joined object takes
//! xfig's attributes (those of the first object), the arrowheads at the far
//! ends survive and those at the cut or the join go (xfig copies both
//! arrows of the first line, leaving one in the middle); the shape factors
//! of a spline follow their points through reversals, and the new end
//! points get the sharp factor 0, as in `split_spline`/`split_cspline`;
//! xfig's check for the end point of a spline to join also lets the second
//! point through (`prev_point != spline->points`), which the port does not.
//
// Ported from xfig src/e_joinsplit.c (xfig licence, see LICENSE-xfig).

use super::{num, opt_num, opt_str, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_geom::{dist_to_segment, Point};
use rustfig_model::{Object, ObjectId, ObjectKind, PolyKind, Polyline, Spline};
use serde_json::{json, Value};
use std::sync::Arc;

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "object.join",
            label: "Join Lines/Splines",
            menu: "",
            shortcut: "",
            params: "{\"id\", \"end\": first|last, \"other\"?: id (default: the same object), \"other_end\": first|last}  — join two open polylines or two open splines at the chosen end points; the same object with both ends closes it (polygon, closed spline)",
            mutates: true,
            enabled: |_| true,
            run: join,
        },
        Command {
            id: "object.split",
            label: "Split Line/Spline",
            menu: "",
            shortcut: "",
            params: "{\"id\", \"index\"?: vertex, or \"x\", \"y\"?: where to cut (the vertex within \"tolerance\", else the nearest segment)}  — cut a polyline or open spline in two, or open a polygon, box or closed spline",
            mutates: true,
            enabled: |_| true,
            run: split,
        },
    ]
}

/// Where a cut falls: at a vertex, or on the segment that starts at it
/// (`find_endpoints`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hit {
    Vertex(usize),
    Segment(usize),
}

impl Hit {
    /// The vertex after which the object is cut.
    fn left(self) -> usize {
        match self {
            Hit::Vertex(i) | Hit::Segment(i) => i,
        }
    }
}

/// The vertex of `pts` within `tol` of `pos`, else the nearest segment (the
/// closing one too when `closed`; within `tol` if given): xfig's
/// `close_to_vector`, which prefers the end points of a segment.
pub(crate) fn locate(pts: &[Point], closed: bool, pos: Point, tol: Option<f64>) -> Option<Hit> {
    if !pos.is_finite() {
        return None;
    }
    let near = tol.unwrap_or(0.0).max(0.0);
    let vertex = pts
        .iter()
        .enumerate()
        .map(|(i, q)| (q.dist(pos), i))
        .filter(|(d, _)| *d <= near)
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, i)) = vertex {
        return Some(Hit::Vertex(i));
    }
    let n = pts.len();
    let segments = if closed { n } else { n.saturating_sub(1) };
    (0..segments)
        .filter_map(|i| {
            let (a, b) = (pts.get(i)?, pts.get((i + 1) % n)?);
            Some((dist_to_segment(pos, *a, *b), i))
        })
        .filter(|(d, _)| tol.is_none_or(|t| *d <= t.max(0.0)))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, i)| Hit::Segment(i))
}

fn bad(m: &str) -> EngineError {
    EngineError::bad_params(m)
}

fn id_param(p: &Value, key: &str) -> Result<ObjectId, EngineError> {
    match p.get(key) {
        Some(v) => v
            .as_u64()
            .map(ObjectId)
            .ok_or_else(|| bad(&format!("{key:?} must be an object id"))),
        None => Err(bad(&format!("missing object id {key:?}"))),
    }
}

fn end_param(p: &Value, key: &str) -> Result<bool, EngineError> {
    match opt_str(p, key)? {
        Some("first" | "start") => Ok(true),
        Some("last" | "end") => Ok(false),
        _ => Err(bad(&format!("{key:?} must be \"first\" or \"last\""))),
    }
}

/// Points, shape factors and arrows of a line or spline with the joined end
/// last (`reverse_points`, `reverse_sfactors`; the arrows swap with the
/// ends).
struct Run {
    points: Vec<Point>,
    shapes: Vec<f64>,
    /// The arrow at the first point, and at the last.
    first_arrow: Option<rustfig_model::Arrow>,
    last_arrow: Option<rustfig_model::Arrow>,
}

impl Run {
    fn oriented(mut self, joined_end_last: bool) -> Run {
        if !joined_end_last {
            self.points.reverse();
            self.shapes.reverse();
            std::mem::swap(&mut self.first_arrow, &mut self.last_arrow);
        }
        self
    }
}

fn line_run(l: &Polyline) -> Run {
    Run {
        points: l.points.clone(),
        shapes: Vec::new(),
        first_arrow: l.backward_arrow,
        last_arrow: l.forward_arrow,
    }
}

fn spline_run(s: &Spline) -> Result<Run, EngineError> {
    if s.shapes.len() != s.points.len() {
        return Err(bad("the spline has no shape factor for some point"));
    }
    Ok(Run {
        points: s.points.clone(),
        shapes: s.shapes.clone(),
        first_arrow: s.backward_arrow,
        last_arrow: s.forward_arrow,
    })
}

fn open_line(l: &Polyline) -> Result<(), EngineError> {
    if l.kind != PolyKind::Polyline {
        return Err(bad("only lines can be joined, not boxes, polygons or pictures"));
    }
    if l.points.is_empty() {
        return Err(bad("the line has no points"));
    }
    Ok(())
}

/// `join_line2`/`connect_line_points`: the line `l1` with its end (`first1`:
/// the first point) joined to the end of `l2` (`None`: the same line, whose
/// other end is the second one picked). Returns the new line.
pub(crate) fn join_lines(
    l1: &Polyline,
    first1: bool,
    l2: Option<&Polyline>,
    first2: bool,
) -> Result<Polyline, EngineError> {
    open_line(l1)?;
    let Some(l2) = l2 else {
        // Both ends of one line: it becomes a polygon.
        if first1 == first2 {
            return Err(bad("choose the other end of the line to close it"));
        }
        if l1.points.len() < 3 {
            return Err(bad("a line needs three points to become a polygon"));
        }
        let mut poly = l1.clone();
        poly.kind = PolyKind::Polygon;
        poly.forward_arrow = None;
        poly.backward_arrow = None;
        return Ok(poly);
    };
    open_line(l2)?;
    let a = line_run(l1).oriented(!first1);
    let b = line_run(l2).oriented(first2);
    let mut new = l1.clone();
    new.points = a.points;
    new.points.extend(b.points);
    new.backward_arrow = a.first_arrow;
    new.forward_arrow = b.last_arrow;
    Ok(new)
}

fn open_spline(s: &Spline) -> Result<(), EngineError> {
    if s.closed {
        return Err(bad("only open splines can be joined"));
    }
    if s.points.is_empty() {
        return Err(bad("the spline has no points"));
    }
    Ok(())
}

/// `join_spline2`/`connect_spline_points` for splines.
pub(crate) fn join_splines(
    s1: &Spline,
    first1: bool,
    s2: Option<&Spline>,
    first2: bool,
) -> Result<Spline, EngineError> {
    open_spline(s1)?;
    let Some(s2) = s2 else {
        if first1 == first2 {
            return Err(bad("choose the other end of the spline to close it"));
        }
        if s1.points.len() < 3 || s1.shapes.len() != s1.points.len() {
            return Err(bad("a spline needs three points to be closed"));
        }
        // Both end points take the factor of the second point, so the
        // curve is smooth where the ends meet.
        let mut closed = s1.clone();
        closed.closed = true;
        closed.forward_arrow = None;
        closed.backward_arrow = None;
        let second = closed.shapes.get(1).copied().unwrap_or(0.0);
        if let Some(f) = closed.shapes.first_mut() {
            *f = second;
        }
        if let Some(l) = closed.shapes.last_mut() {
            *l = second;
        }
        return Ok(closed);
    };
    open_spline(s2)?;
    let a = spline_run(s1)?.oriented(!first1);
    let b = spline_run(s2)?.oriented(first2);
    let mut new = s1.clone();
    new.points = a.points;
    new.points.extend(b.points);
    new.shapes = a.shapes;
    new.shapes.extend(b.shapes);
    new.backward_arrow = a.first_arrow;
    new.forward_arrow = b.last_arrow;
    Ok(new)
}

fn join(e: &mut Engine, p: &Value) -> CmdResult {
    let id1 = id_param(p, "id")?;
    let first1 = end_param(p, "end")?;
    let id2 = match p.get("other") {
        None | Some(Value::Null) => id1,
        Some(_) => id_param(p, "other")?,
    };
    let first2 = end_param(p, "other_end")?;
    let same = id1 == id2;
    let o1 = e
        .doc()
        .get(id1)
        .cloned()
        .ok_or_else(|| EngineError::not_found(format!("no top-level object with id {}", id1.0)))?;
    let o2 = if same {
        None
    } else {
        Some(
            e.doc()
                .get(id2)
                .cloned()
                .ok_or_else(|| EngineError::not_found(format!("no top-level object with id {}", id2.0)))?,
        )
    };
    let kind = match (&o1.kind, o2.as_ref().map(|o| &o.kind)) {
        (ObjectKind::Polyline(l1), None) => ObjectKind::Polyline(join_lines(l1, first1, None, first2)?),
        (ObjectKind::Polyline(l1), Some(ObjectKind::Polyline(l2))) => {
            ObjectKind::Polyline(join_lines(l1, first1, Some(l2), first2)?)
        }
        (ObjectKind::Spline(s1), None) => ObjectKind::Spline(join_splines(s1, first1, None, first2)?),
        (ObjectKind::Spline(s1), Some(ObjectKind::Spline(s2))) => {
            ObjectKind::Spline(join_splines(s1, first1, Some(s2), first2)?)
        }
        _ => return Err(bad("join two lines or two splines")),
    };
    let doc = e.doc_mut();
    doc.objects.retain(|o| o.id != id1 && o.id != id2);
    let mut new = Object::new(kind);
    new.comments = o1.comments.clone();
    new.id = doc.alloc_id();
    let id = new.id;
    doc.objects.push(Arc::new(new));
    e.set_selection(vec![id]);
    Ok(json!({"id": id.0}))
}

/// What a cut produces.
pub(crate) enum Cut {
    /// One object (a polygon, box or closed spline opened).
    One(ObjectKind),
    Two(ObjectKind, ObjectKind),
}

/// `split_line`/`split_polygon`: cut the line, polygon or box at `hit`.
pub(crate) fn split_polyline(l: &Polyline, hit: Hit) -> Result<Cut, EngineError> {
    let n = l.points.len();
    match l.kind {
        PolyKind::Picture => Err(bad("Can't remove points from picture")),
        _ if n < 2 => Err(bad("a single point cannot be split")),
        PolyKind::Polygon | PolyKind::Box | PolyKind::ArcBox => {
            // The loop opens at the cut: it starts at the vertex after it.
            let start = (hit.left() + 1) % n;
            let mut line = l.clone();
            line.kind = PolyKind::Polyline;
            line.points = l.points.iter().cycle().skip(start).take(n).copied().collect();
            line.forward_arrow = None;
            line.backward_arrow = None;
            Ok(Cut::One(ObjectKind::Polyline(line)))
        }
        PolyKind::Polyline => {
            let left = hit.left();
            if matches!(hit, Hit::Vertex(i) if i == 0 || i + 1 >= n) || left + 1 >= n {
                return Err(bad("cannot split at an end point"));
            }
            let mut first = l.clone();
            first.points.truncate(left + 1);
            first.forward_arrow = None;
            let mut second = l.clone();
            second.points = l.points.get(left + 1..).unwrap_or_default().to_vec();
            second.backward_arrow = None;
            Ok(Cut::Two(ObjectKind::Polyline(first), ObjectKind::Polyline(second)))
        }
    }
}

/// `split_spline`/`split_cspline`: cut the spline at `hit`.
pub(crate) fn split_spline_at(s: &Spline, hit: Hit) -> Result<Cut, EngineError> {
    let n = s.points.len();
    if s.shapes.len() != n {
        return Err(bad("the spline has no shape factor for some point"));
    }
    let left = hit.left();
    if s.closed {
        if n < 3 {
            return Err(bad("the spline has too few points"));
        }
        let start = (left + 1) % n;
        let mut open = s.clone();
        open.closed = false;
        open.points = s.points.iter().cycle().skip(start).take(n).copied().collect();
        open.shapes = s.shapes.iter().cycle().skip(start).take(n).copied().collect();
        // The new end points are sharp.
        if let Some(f) = open.shapes.first_mut() {
            *f = 0.0;
        }
        if let Some(l) = open.shapes.last_mut() {
            *l = 0.0;
        }
        open.forward_arrow = None;
        open.backward_arrow = None;
        return Ok(Cut::One(ObjectKind::Spline(open)));
    }
    // Neither piece may have fewer than two points.
    if left < 1 || left + 3 > n || matches!(hit, Hit::Vertex(i) if i == 0 || i + 1 >= n) {
        return Err(bad("Cutting there would result in a spline with only one point"));
    }
    let mut first = s.clone();
    first.points.truncate(left + 1);
    first.shapes.truncate(left + 1);
    if let Some(l) = first.shapes.last_mut() {
        *l = 0.0;
    }
    first.forward_arrow = None;
    let mut second = s.clone();
    second.points = s.points.get(left + 1..).unwrap_or_default().to_vec();
    second.shapes = s.shapes.get(left + 1..).unwrap_or_default().to_vec();
    if let Some(f) = second.shapes.first_mut() {
        *f = 0.0;
    }
    second.backward_arrow = None;
    Ok(Cut::Two(ObjectKind::Spline(first), ObjectKind::Spline(second)))
}

fn split(e: &mut Engine, p: &Value) -> CmdResult {
    let id = id_param(p, "id")?;
    let o = e
        .doc()
        .get(id)
        .cloned()
        .ok_or_else(|| EngineError::not_found(format!("no top-level object with id {}", id.0)))?;
    let (pts, closed): (&[Point], bool) = match &o.kind {
        ObjectKind::Polyline(l) => (&l.points, l.kind != PolyKind::Polyline),
        ObjectKind::Spline(s) => (&s.points, s.closed),
        _ => return Err(bad("only lines and splines can be split")),
    };
    let hit = match opt_num(p, "index")? {
        Some(i) => {
            if i < 0.0 || i.fract() != 0.0 || i as usize >= pts.len() {
                return Err(bad("\"index\" is not a vertex of the object"));
            }
            Hit::Vertex(i as usize)
        }
        None => {
            let pos = Point::new(num(p, "x")?, num(p, "y")?);
            let tol = opt_num(p, "tolerance")?;
            locate(pts, closed, pos, tol).ok_or_else(|| bad("the point is not on the object"))?
        }
    };
    let cut = match &o.kind {
        ObjectKind::Polyline(l) => split_polyline(l, hit)?,
        ObjectKind::Spline(s) => split_spline_at(s, hit)?,
        _ => return Err(bad("only lines and splines can be split")),
    };
    let doc = e.doc_mut();
    doc.objects.retain(|x| x.id != id);
    let mut ids = Vec::new();
    let kinds = match cut {
        Cut::One(a) => vec![a],
        Cut::Two(a, b) => vec![a, b],
    };
    for kind in kinds {
        let mut new = Object::new(kind);
        new.comments = o.comments.clone();
        new.id = doc.alloc_id();
        ids.push(new.id);
        doc.objects.push(Arc::new(new));
    }
    e.set_selection(ids.clone());
    Ok(json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>()}))
}
