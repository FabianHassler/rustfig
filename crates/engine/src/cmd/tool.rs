//! Tool (mode) selection and synthetic tool events.

use super::{always, num, opt_bool, opt_str, CmdResult, Command};
use crate::tools::{Key, ToolEvent, ToolKind};
use crate::{Engine, EngineError};
use rustfig_geom::Point;
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    let mut v: Vec<Command> = ToolKind::ALL
        .iter()
        .map(|t| Command {
            id: t.id(),
            label: t.label(),
            menu: if t.is_drawing() { "Tools/Draw" } else { "Tools/Edit" },
            shortcut: t.shortcut(),
            params: "{}",
            mutates: false,
            enabled: always,
            run: run_for(*t),
        })
        .collect();
    v.push(Command {
        id: "tool.event",
        label: "Send tool event",
        menu: "",
        shortcut: "",
        params: "{\"type\": press|move|release|double_click|key|text, \"x\"?, \"y\"? (Fig units), \"button\"?: left|right, \"shift\"?: bool, \"key\"?: enter|escape|backspace|delete|left|right, \"text\"?: string}",
        mutates: false,
        enabled: always,
        run: event,
    });
    v
}

fn set(e: &mut Engine, kind: ToolKind) -> CmdResult {
    e.set_tool(kind);
    Ok(json!({"tool": kind.id()}))
}

/// One `run` function per tool (commands take plain function pointers).
macro_rules! tool_runs {
    ($($k:ident),* $(,)?) => {
        fn run_for(kind: ToolKind) -> fn(&mut Engine, &Value) -> CmdResult {
            match kind {
                $(ToolKind::$k => {
                    fn f(e: &mut Engine, _: &Value) -> CmdResult {
                        set(e, ToolKind::$k)
                    }
                    f
                })*
            }
        }
    };
}

tool_runs!(
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
);

fn event(e: &mut Engine, p: &Value) -> CmdResult {
    let kind = opt_str(p, "type")?.unwrap_or("");
    let pos = || -> Result<Point, EngineError> { Ok(Point::new(num(p, "x")?, num(p, "y")?)) };
    let shift = opt_bool(p, "shift")?.unwrap_or(false);
    let right = opt_str(p, "button")? == Some("right");
    let ev = match kind {
        "press" => ToolEvent::Press {
            pos: pos()?,
            right,
            shift,
        },
        "move" => ToolEvent::Move { pos: pos()? },
        "release" => ToolEvent::Release { pos: pos()?, right },
        "double_click" => ToolEvent::DoubleClick { pos: pos()? },
        "key" => ToolEvent::Key(match opt_str(p, "key")?.unwrap_or("") {
            "enter" | "return" => Key::Enter,
            "escape" => Key::Escape,
            "backspace" => Key::Backspace,
            "delete" => Key::Delete,
            "left" => Key::Left,
            "right" => Key::Right,
            o => return Err(EngineError::bad_params(format!("unknown key {o:?}"))),
        }),
        "text" => ToolEvent::Text(super::string(p, "text")?),
        o => return Err(EngineError::bad_params(format!("unknown event type {o:?}"))),
    };
    e.tool_event(ev)?;
    Ok(json!({"tool": e.tool().id()}))
}
