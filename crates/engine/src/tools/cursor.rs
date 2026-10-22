//! The canvas cursor per mode (xfig's `w_cursor.c`): a cross-hair for
//! drawing, a hand over what the editing modes pick, the I-beam of the text
//! mode, a mover while objects follow the pointer. The UI maps
//! [`CursorKind`] to its toolkit's cursor set.

use super::{ToolKind, ToolState};
use crate::Engine;

/// xfig's cursors, by what they tell the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorKind {
    /// `arrow_cursor`: no mode, or Select with nothing to do.
    Arrow,
    /// `crosshair_cursor` / `null_cursor`: drawing a shape, placing a point.
    Crosshair,
    /// `pick9_cursor` (a hand): pick an object or one of its points.
    Pick,
    /// `pick15_cursor` (a dotted box): pick a whole object to transform.
    PickObject,
    /// `buster_cursor`: Delete.
    Delete,
    /// `text_cursor`: the text mode.
    Text,
    /// Objects follow the pointer (Move, Copy, Paste).
    Move,
    /// A point or an object is being dragged.
    Grabbing,
}

impl Engine {
    /// The cursor the canvas shows now: by mode, and by what is in progress.
    pub fn cursor_kind(&self) -> CursorKind {
        use ToolKind::*;
        match &self.tool_state {
            ToolState::Paste { .. } => return CursorKind::Move,
            ToolState::MoveObjs { dragging: true, .. } | ToolState::PointDrag { .. } => return CursorKind::Grabbing,
            ToolState::MoveObjs { .. } => return CursorKind::Move,
            _ => {}
        }
        match self.tool {
            None => CursorKind::Arrow,
            Text => CursorKind::Text,
            Delete => CursorKind::Delete,
            Move | Copy | Align | Break | OpenCompound | Convert | FlipH | FlipV | RotateCcw | RotateCw | Scale => {
                CursorKind::PickObject
            }
            Glue | JoinSplit | Chop | Trim | MovePoint | AddPoint | DeletePoint | Update | Edit | Arrow | OpenClose
            | Tangent | AngleMeasure | LengthMeasure | AreaMeasure => CursorKind::Pick,
            _ => CursorKind::Crosshair,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Engine;

    #[test]
    fn the_cursor_follows_the_mode() {
        let mut e = Engine::new();
        let of = |e: &mut Engine, t| {
            e.set_tool(t);
            e.cursor_kind()
        };
        assert_eq!(of(&mut e, ToolKind::None), CursorKind::Arrow);
        assert_eq!(of(&mut e, ToolKind::Box), CursorKind::Crosshair);
        assert_eq!(of(&mut e, ToolKind::Polyline), CursorKind::Crosshair);
        assert_eq!(of(&mut e, ToolKind::Text), CursorKind::Text);
        assert_eq!(of(&mut e, ToolKind::Move), CursorKind::PickObject);
        assert_eq!(of(&mut e, ToolKind::Delete), CursorKind::Delete);
        assert_eq!(of(&mut e, ToolKind::Edit), CursorKind::Pick);
        assert_eq!(of(&mut e, ToolKind::MovePoint), CursorKind::Pick);
    }

    #[test]
    fn every_mode_has_a_cursor_and_busy_states_override() {
        let mut e = Engine::new();
        for t in ToolKind::ALL {
            e.set_tool(t);
            let _ = e.cursor_kind();
        }
        e.set_tool(ToolKind::None);
        let _ = e.execute(
            "shape.rectangle",
            &serde_json::json!({"x": 0, "y": 0, "width": 600, "height": 600}),
        );
        let _ = e.execute("edit.copy", &serde_json::json!({}));
        e.execute("edit.select_all", &serde_json::json!({})).unwrap();
        e.execute("edit.copy", &serde_json::json!({})).unwrap();
        e.execute("edit.paste", &serde_json::json!({})).unwrap();
        assert_eq!(e.cursor_kind(), CursorKind::Move);
    }
}
