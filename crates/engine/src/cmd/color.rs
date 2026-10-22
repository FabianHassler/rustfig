//! Colours (E4): parsing hex and X11 names, the colour editor (new, edit,
//! delete user colours), recently used colours.
//!
//! The editor follows xfig's colour panel (`w_color.c`: Add, Modify, Delete;
//! a colour in use is not deleted, as xfig's `del_color` keeps it).
// Ported from xfig src/w_color.c (xfig licence, see LICENSE-xfig).

use super::{always, num, opt_num, opt_str, string, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_model::{ColorRef, Rgb, FIRST_USER_COLOR, LAST_USER_COLOR};
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "color.parse",
            label: "Parse colour",
            menu: "",
            shortcut: "",
            params: "{\"text\": \"#rrggbb\" | \"#rgb\" | X11 colour name}  — {\"rgb\": \"#rrggbb\", \"r\", \"g\", \"b\", \"h\" (0-360), \"s\", \"v\" (0-100), \"name\"?: the X11 name of exactly that colour}",
            mutates: false,
            enabled: always,
            run: parse,
        },
        Command {
            id: "color.from_hsv",
            label: "Colour from HSV",
            menu: "",
            shortcut: "",
            params: "{\"h\": 0-360, \"s\": 0-100, \"v\": 0-100}  — the same reply as color.parse",
            mutates: false,
            enabled: always,
            run: from_hsv,
        },
        Command {
            id: "color.names",
            label: "X11 colour names",
            menu: "",
            shortcut: "",
            params: "{\"prefix\"?: string, \"limit\"?: 1-1000 (default 50)}  — {\"names\": [...]} the X11 names starting with the prefix (any case)",
            mutates: false,
            enabled: always,
            run: names,
        },
        Command {
            id: "color.new",
            label: "New user colour",
            menu: "",
            shortcut: "",
            params: "{\"rgb\": \"#rrggbb\" | X11 name}  — adds a user colour in the lowest free slot (32-543) even if its value exists; returns {\"color\": number}",
            mutates: true,
            enabled: always,
            run: new_color,
        },
        Command {
            id: "color.edit",
            label: "Change user colour",
            menu: "",
            shortcut: "",
            params: "{\"color\": 32-543, \"rgb\": \"#rrggbb\" | X11 name}  — gives an existing user colour a new value; objects that use it change with it",
            mutates: true,
            enabled: always,
            run: edit,
        },
        Command {
            id: "color.delete",
            label: "Delete user colour",
            menu: "",
            shortcut: "",
            params: "{\"color\": 32-543}  — removes a user colour; refused while an object uses it",
            mutates: true,
            enabled: always,
            run: delete,
        },
        Command {
            id: "color.recent",
            label: "Recent colours",
            menu: "",
            shortcut: "",
            params: "{}  — {\"colors\": [\"#rrggbb\", …]} the colours used lately, newest first (kept in the preferences)",
            mutates: false,
            enabled: always,
            run: recent,
        },
        Command {
            id: "color.use",
            label: "Note colour as used",
            menu: "",
            shortcut: "",
            params: "{\"color\": colour number | \"rgb\": \"#rrggbb\"}  — puts the colour first in the recent colours",
            mutates: false,
            enabled: always,
            run: use_color,
        },
    ]
}

fn rgb_param(p: &Value, key: &str) -> Result<Rgb, EngineError> {
    let s = string(p, key)?;
    if s.chars().count() > 64 {
        return Err(EngineError::bad_params("colour text too long"));
    }
    rustfig_model::x_color(s.trim())
        .ok_or_else(|| EngineError::bad_params(format!("{s:?} is neither #rrggbb nor an X11 colour name")))
}

/// RGB to hue (0-360), saturation and value (0-100).
pub fn to_hsv(c: Rgb) -> (f64, f64, f64) {
    let (r, g, b) = (f64::from(c.r) / 255.0, f64::from(c.g) / 255.0, f64::from(c.b) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max == 0.0 { 0.0 } else { d / max };
    (h, s * 100.0, max * 100.0)
}

/// Hue (0-360), saturation and value (0-100) to RGB.
pub fn from_hsv_values(h: f64, s: f64, v: f64) -> Rgb {
    let h = if h.is_finite() { h.rem_euclid(360.0) / 60.0 } else { 0.0 };
    let s = (s / 100.0).clamp(0.0, 1.0);
    let v = (v / 100.0).clamp(0.0, 1.0);
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let byte = |f: f64| ((f + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb::new(byte(r), byte(g), byte(b))
}

fn describe(c: Rgb) -> Value {
    let (h, s, v) = to_hsv(c);
    let name = rustfig_model::x_color_names().find(|n| rustfig_model::x_color(n) == Some(c));
    let mut m =
        json!({"rgb": c.to_hex(), "r": c.r, "g": c.g, "b": c.b, "h": h.round(), "s": s.round(), "v": v.round()});
    if let (Some(name), Some(o)) = (name, m.as_object_mut()) {
        o.insert("name".into(), json!(name));
    }
    m
}

fn parse(_: &mut Engine, p: &Value) -> CmdResult {
    Ok(describe(rgb_param(p, "text")?))
}

fn from_hsv(_: &mut Engine, p: &Value) -> CmdResult {
    Ok(describe(from_hsv_values(num(p, "h")?, num(p, "s")?, num(p, "v")?)))
}

fn names(_: &mut Engine, p: &Value) -> CmdResult {
    let prefix = opt_str(p, "prefix")?.unwrap_or("").to_ascii_lowercase();
    let limit = opt_num(p, "limit")?.unwrap_or(50.0).clamp(1.0, 1000.0) as usize;
    let list: Vec<&str> = rustfig_model::x_color_names()
        .filter(|n| n.to_ascii_lowercase().starts_with(&prefix))
        .take(limit)
        .collect();
    Ok(json!({ "names": list }))
}

fn user_number(p: &Value) -> Result<i32, EngineError> {
    let n = num(p, "color")?;
    if n.fract() != 0.0 || n < f64::from(FIRST_USER_COLOR) || n > f64::from(LAST_USER_COLOR) {
        return Err(EngineError::bad_params(format!(
            "color must be a user colour number {FIRST_USER_COLOR}-{LAST_USER_COLOR}"
        )));
    }
    Ok(n as i32)
}

fn note_recent(e: &mut Engine, rgb: Rgb) {
    e.prefs.add_recent_color(&rgb.to_hex());
    e.prefs_dirty = true;
}

fn new_color(e: &mut Engine, p: &Value) -> CmdResult {
    let rgb = rgb_param(p, "rgb")?;
    let c = e
        .doc_mut()
        .new_user_color(rgb)
        .ok_or_else(|| EngineError::bad_params("all 512 user colours are in use"))?;
    note_recent(e, rgb);
    Ok(json!({"color": c.0}))
}

fn edit(e: &mut Engine, p: &Value) -> CmdResult {
    let n = user_number(p)?;
    let rgb = rgb_param(p, "rgb")?;
    let u = e
        .doc_mut()
        .colors
        .iter_mut()
        .find(|u| u.number == n)
        .ok_or_else(|| EngineError::bad_params(format!("no user colour {n}")))?;
    u.rgb = rgb;
    note_recent(e, rgb);
    Ok(json!({"color": n}))
}

fn delete(e: &mut Engine, p: &Value) -> CmdResult {
    let n = user_number(p)?;
    if !e.doc().colors.iter().any(|u| u.number == n) {
        return Err(EngineError::bad_params(format!("no user colour {n}")));
    }
    if e.doc().used_colors().contains(&n) {
        return Err(EngineError::bad_params(format!("colour {n} is used by an object")));
    }
    e.doc_mut().colors.retain(|u| u.number != n);
    Ok(json!({"color": n}))
}

fn recent(e: &mut Engine, _: &Value) -> CmdResult {
    Ok(json!({"colors": e.prefs.rustfig.recent_colors}))
}

fn use_color(e: &mut Engine, p: &Value) -> CmdResult {
    let rgb = if p.get("rgb").is_some() {
        rgb_param(p, "rgb")?
    } else {
        let n = num(p, "color")?;
        if n.fract() != 0.0 || !(0.0..=f64::from(LAST_USER_COLOR)).contains(&n) {
            return Err(EngineError::bad_params("color must be a colour number 0-543"));
        }
        let n = n as i32;
        if n >= FIRST_USER_COLOR && !e.doc().colors.iter().any(|u| u.number == n) {
            return Err(EngineError::bad_params(format!("no user colour {n}")));
        }
        e.doc().rgb(ColorRef(n))
    };
    note_recent(e, rgb);
    Ok(json!({"rgb": rgb.to_hex()}))
}
