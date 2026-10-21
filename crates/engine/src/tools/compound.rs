//! The Open compound and Join/Split modes (plan D1, D2): the mouse side of
//! `e_compound.c` and `e_joinsplit.c`. The work is done by the commands
//! `compound.open`, `object.join` and `object.split`.
//
// Ported from xfig src/e_compound.c and src/e_joinsplit.c (xfig licence, see LICENSE-xfig).

use super::{markers::ObjMask, Button, ToolEvent, ToolKind, ToolState};
use crate::{Engine, EngineError};
use rustfig_geom::Point;
use rustfig_model::{Object, ObjectId, ObjectKind, PolyKind};
use serde_json::json;

/// The end point of a line or spline under the pointer.
struct EndHit {
    id: ObjectId,
    /// The first point of the object (else the last).
    first: bool,
    spline: bool,
}

impl Engine {
    /// Open compound (`init_open_compound`): left opens the compound, middle
    /// opens it and keeps the rest of the figure visible (gray). The modern
    /// scheme has no middle button: Shift+left keeps it visible.
    pub(super) fn open_compound_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let ToolEvent::Press { pos, button, shift } = ev else {
            return Ok(());
        };
        let keep_visible = match button {
            Button::Left => !self.xfig_buttons() && shift,
            Button::Middle => true,
            Button::Right => return Ok(()),
        };
        let tol = self.pick_tol();
        let mask = ObjMask::COMPOUND;
        if let Some(id) = self.hit_test_where(pos, tol, &|o| mask.admits(o)) {
            self.execute("compound.open", &json!({"id": id.0, "keep_visible": keep_visible}))?;
        }
        Ok(())
    }

    /// The end point of an open line or spline within `tol` of `pos`;
    /// `only` limits the search to lines (`Some(false)`) or splines.
    fn end_point_at(&self, pos: Point, tol: f64, only: Option<bool>) -> Option<EndHit> {
        let mask = match only {
            Some(false) => ObjMask::POLYLINE_LINE,
            Some(true) => ObjMask::SPLINE_OPEN,
            None => ObjMask::POLYLINE_LINE.union(ObjMask::SPLINE_OPEN),
        };
        let mut best: Option<(f64, EndHit)> = None;
        for o in &self.doc().objects {
            if !mask.admits(o) || !self.is_visible(o) {
                continue;
            }
            let (pts, spline) = match &o.kind {
                ObjectKind::Polyline(l) => (&l.points, false),
                ObjectKind::Spline(s) => (&s.points, true),
                _ => continue,
            };
            let (Some(first), Some(last)) = (pts.first(), pts.last()) else {
                continue;
            };
            for (p, is_first) in [(first, true), (last, false)] {
                let d = p.dist(pos);
                if d <= tol && best.as_ref().is_none_or(|(bd, _)| d < *bd) {
                    best = Some((
                        d,
                        EndHit {
                            id: o.id,
                            first: is_first,
                            spline,
                        },
                    ));
                }
            }
        }
        best.map(|(_, h)| h)
    }

    /// Join/Split. Left picks an end point (`init_join`) and then the end
    /// point of the second line or spline (`join_line2`, `join_spline2`);
    /// middle splits at the pointer (`init_split`); right cancels a join.
    /// The modern scheme has no middle button: right splits when nothing is
    /// being joined (Escape cancels a join).
    pub(super) fn join_split_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let ToolEvent::Press { pos, button, .. } = ev else {
            return Ok(());
        };
        let tol = self.pick_tol();
        let xfig = self.xfig_buttons();
        if let ToolState::Join { id, first, spline } = self.tool_state {
            match button {
                Button::Left => {
                    let Some(hit) = self.end_point_at(pos, tol, Some(spline)) else {
                        return Ok(());
                    };
                    // The same point twice does nothing; both ends of one
                    // object close it.
                    if hit.id == id && hit.first == first {
                        return Ok(());
                    }
                    let end = |f: bool| if f { "first" } else { "last" };
                    let mut params = json!({"id": id.0, "end": end(first), "other_end": end(hit.first)});
                    if hit.id != id {
                        params["other"] = json!(hit.id.0);
                    }
                    self.tool_state = ToolState::Idle;
                    if let Err(e) = self.execute("object.join", &params) {
                        self.set_status(e.message);
                    }
                }
                Button::Right => self.tool_state = ToolState::Idle,
                Button::Middle => {}
            }
            return Ok(());
        }
        let split = button == Button::Middle || (!xfig && button == Button::Right);
        if button == Button::Left {
            let Some(hit) = self.end_point_at(pos, tol, None) else {
                return Ok(());
            };
            self.tool_state = ToolState::Join {
                id: hit.id,
                first: hit.first,
                spline: hit.spline,
            };
        } else if split {
            let mask = ObjMask::POLYLINE.union(ObjMask::SPLINE);
            let Some(id) = self.hit_test_where(pos, tol, &|o: &Object| mask.admits(o)) else {
                return Ok(());
            };
            let params = json!({"id": id.0, "x": pos.x, "y": pos.y, "tolerance": tol});
            if let Err(e) = self.execute("object.split", &params) {
                // xfig: put_msg and beep; the figure stays as it is.
                self.set_status(e.message);
            }
        }
        Ok(())
    }

    /// The end point a join started at, for the marker (`draw_join_marker`).
    pub(super) fn join_marker(&self) -> Option<Point> {
        let ToolState::Join { id, first, .. } = self.tool_state else {
            return None;
        };
        let pts = match &self.doc().get(id)?.kind {
            ObjectKind::Polyline(l) if l.kind == PolyKind::Polyline => &l.points,
            ObjectKind::Spline(s) => &s.points,
            _ => return None,
        };
        if first {
            pts.first().copied()
        } else {
            pts.last().copied()
        }
    }

    /// Whether the tool is Join/Split and has picked the first end.
    pub(super) fn joining(&self) -> bool {
        self.tool == ToolKind::JoinSplit && matches!(self.tool_state, ToolState::Join { .. })
    }
}
