//! Tool (mode) selection and synthetic tool events.

use super::{always, num, opt_bool, opt_str, CmdResult, Command};
use crate::tools::{Button, Key, ToolEvent, ToolKind};
use crate::{Engine, EngineError};
use rustfig_geom::Point;
use serde_json::{json, Value};

/// Most characters Paste Text inserts at once.
const MAX_PASTE_CHARS: usize = 10_000;

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
        id: "tool.escape",
        label: "Escape",
        menu: "",
        shortcut: "Escape",
        params: "{}  — one level per press: cancel what is in progress (text input is finished), else clear what the mode collected (Chop's axes, the located object, the rotate/flip centre, the selection), else leave the mode; returns {\"done\": cancelled|text finished|axes cleared|cleared|selection cleared|mode left|nothing}",
        mutates: false,
        enabled: always,
        run: |e, _| Ok(json!({"done": e.escape()?})),
    });
    v.push(Command {
        id: "tool.event",
        label: "Send tool event",
        menu: "",
        shortcut: "",
        params: "{\"type\": press|move|release|double_click|key|text, \"x\"?, \"y\"? (Fig units), \"button\"?: left|middle|right (default left), \"shift\"?: bool, \"key\"?: enter|escape|backspace|delete|left|right, \"text\"?: string}",
        mutates: false,
        enabled: always,
        run: event,
    });
    v.push(Command {
        id: "text.paste",
        label: "Paste Text",
        menu: "Edit",
        shortcut: "",
        params: "{\"text\"?: string}  — xfig's Paste Text (its F18/F20 keys are gone from keyboards; while typing the platform paste key does the same): inserts the system clipboard's text (or `text`) at the caret of the text being typed (Text mode), as if typed; returns {\"inserted\": n}",
        mutates: false,
        enabled: |e| e.typing_font().is_some(),
        run: |e, p| {
            let text = match p.get("text") {
                Some(_) => super::string(p, "text")?,
                None => e
                    .clipboard_plain_text()
                    .ok_or_else(|| EngineError::bad_params("the clipboard holds no text"))?,
            };
            if text.chars().count() > MAX_PASTE_CHARS {
                return Err(EngineError::bad_params("the text on the clipboard is too long"));
            }
            Ok(json!({"inserted": e.text_insert_chars(&text)?}))
        },
    });
    v.push(Command {
        id: "text.insert_char",
        label: "Insert Character",
        menu: "",
        shortcut: "",
        params: "{\"text\": string}  — the character map: inserts the characters at the caret of the text being typed (Text mode). In a Symbol or ZapfDingbats text each Unicode character is stored as that font's own code (∑ as 0xE5), so exports draw it; characters the font lacks are skipped; returns {\"inserted\": n}",
        mutates: false,
        enabled: |e| e.typing_font().is_some(),
        run: |e, p| {
            let s = super::string(p, "text")?;
            if s.len() > 64 {
                return Err(EngineError::bad_params("too many characters"));
            }
            Ok(json!({"inserted": e.text_insert_chars(&s)?}))
        },
    });
    v.push(Command {
        id: "tool.point",
        label: "Enter point",
        menu: "",
        shortcut: "",
        params: "{\"text\": \"x,y\" | \"length<angle\", \"button\"?: left|middle|right (default left), \"shift\"?: bool}  — xfig's keyboard point entry: the point typed as text is used as if clicked. x,y in ruler units (inches or cm; fractions as 1-1/2), r before a number makes it relative to the current point (the last point of a line, or where the operation started), a absolute; length<angle is polar from the current point, the angle in degrees (or 2<0.5p in multiples of pi, 2<1r radians)",
        mutates: false,
        enabled: always,
        run: point,
    });
    v.push(Command {
        id: "snap.mode",
        label: "Snap to",
        menu: "",
        shortcut: "",
        params: "{\"mode\": endpoint|midpoint|nearest|focus|diameter|normal|tangent|intersect|none}  — applies to the next clicked point (xfig's Snap menu)",
        mutates: false,
        enabled: always,
        run: |e, p| {
            let name = super::string(p, "mode")?;
            let mode = if name == "none" {
                None
            } else {
                Some(
                    crate::ObjectSnap::from_name(&name)
                        .ok_or_else(|| EngineError::bad_params(format!("unknown snap mode {name:?}")))?,
                )
            };
            e.set_object_snap(mode);
            e.set_status(format!("Snap: {}", e.snap_indicator()));
            Ok(json!({"snap": e.snap_indicator()}))
        },
    });
    v.push(Command {
        id: "snap.hold",
        label: "Hold",
        menu: "",
        shortcut: "",
        params: "{}  — keep the snap mode after it is used",
        mutates: false,
        enabled: always,
        run: |e, _| {
            e.set_snap_held(true);
            Ok(json!({"held": true}))
        },
    });
    v.push(Command {
        id: "snap.release",
        label: "Release",
        menu: "",
        shortcut: "",
        params: "{}  — stop holding and disarm the snap mode",
        mutates: false,
        enabled: always,
        run: |e, _| {
            e.set_snap_held(false);
            Ok(json!({"held": false}))
        },
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
    None,
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
    Trim,
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
    Tangent,
    AngleMeasure,
    LengthMeasure,
    AreaMeasure,
    Plot,
);

fn event(e: &mut Engine, p: &Value) -> CmdResult {
    let kind = opt_str(p, "type")?.unwrap_or("");
    let pos = || -> Result<Point, EngineError> { Ok(Point::new(num(p, "x")?, num(p, "y")?)) };
    let shift = opt_bool(p, "shift")?.unwrap_or(false);
    let button = match opt_str(p, "button")? {
        Some(n) => Button::from_name(n).ok_or_else(|| EngineError::bad_params(format!("unknown button {n:?}")))?,
        None => Button::Left,
    };
    let ev = match kind {
        "press" => ToolEvent::Press {
            pos: pos()?,
            button,
            shift,
        },
        "move" => ToolEvent::Move { pos: pos()? },
        "release" => ToolEvent::Release { pos: pos()?, button },
        "double_click" => ToolEvent::DoubleClick { pos: pos()? },
        "key" => ToolEvent::Key(match opt_str(p, "key")?.unwrap_or("") {
            "enter" | "return" => Key::Enter,
            "escape" => Key::Escape,
            "backspace" => Key::Backspace,
            "delete" => Key::Delete,
            "left" => Key::Left,
            "right" => Key::Right,
            "home" => Key::Home,
            "end" => Key::End,
            o => return Err(EngineError::bad_params(format!("unknown key {o:?}"))),
        }),
        "text" => ToolEvent::Text(super::string(p, "text")?),
        o => return Err(EngineError::bad_params(format!("unknown event type {o:?}"))),
    };
    e.tool_event(ev)?;
    Ok(json!({"tool": e.tool().id()}))
}

/// xfig's keyboard point entry (`w_keyboard.c`): the typed point is fed to
/// the tool as a press there, exactly (no snapping, no angle constraint).
fn point(e: &mut Engine, p: &Value) -> CmdResult {
    let text = super::string(p, "text")?;
    if text.len() > 200 {
        return Err(EngineError::bad_params("the coordinate text is too long"));
    }
    let button = match opt_str(p, "button")? {
        Some(n) => Button::from_name(n).ok_or_else(|| EngineError::bad_params(format!("unknown button {n:?}")))?,
        None => Button::Left,
    };
    let shift = opt_bool(p, "shift")?.unwrap_or(false);
    let at = crate::coords::parse_coordinate(&text, e.current_point(), e.doc().settings.units).map_err(|m| {
        e.set_status(m.clone());
        EngineError::bad_params(m)
    })?;
    e.add_exact_point(at);
    e.tool_event(ToolEvent::Press { pos: at, button, shift })?;
    Ok(json!({"x": at.x, "y": at.y, "tool": e.tool().id()}))
}
