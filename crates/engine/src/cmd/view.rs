//! View commands: zoom, pan, grid, snap, page border.

use super::{always, num, num_or, opt_bool, opt_num, opt_str, CmdResult, Command};
use crate::{Engine, EngineError, SnapMode};
use rustfig_geom::Point;
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "view.zoom_in",
            label: "Zoom In",
            menu: "View",
            shortcut: "Ctrl+=",
            params: "{}",
            mutates: false,
            enabled: always,
            run: |e, _| zoom_by(e, 2.0),
        },
        Command {
            id: "view.zoom_out",
            label: "Zoom Out",
            menu: "View",
            shortcut: "Ctrl+-",
            params: "{}",
            mutates: false,
            enabled: always,
            run: |e, _| zoom_by(e, 0.5),
        },
        Command {
            id: "view.zoom_fit",
            label: "Zoom to Fit",
            menu: "View",
            shortcut: "Ctrl+0",
            params: "{}",
            mutates: false,
            enabled: always,
            run: |e, _| {
                let bb = e.doc_bounds();
                if !bb.is_empty() {
                    e.view.fit(&bb);
                    e.view.apply_integer_zoom(true);
                }
                Ok(json!({"zoom": e.view.zoom}))
            },
        },
        Command {
            id: "view.zoom_actual",
            label: "Actual Size",
            menu: "View",
            shortcut: "Ctrl+1",
            params: "{}",
            mutates: false,
            enabled: always,
            run: |e, _| {
                let c = Point::new(e.view.size.0 / 2.0, e.view.size.1 / 2.0);
                e.view.zoom_about(1.0, c);
                Ok(json!({"zoom": 1.0}))
            },
        },
        Command {
            id: "view.zoom_box",
            label: "Zoom to box",
            menu: "",
            shortcut: "",
            params: "{\"x1\", \"y1\", \"x2\", \"y2\": two opposite corners of the box (Fig units)}  â zoom so that the box fills the canvas, its top-left corner at the canvas's top-left (xfig's zoom box: Ctrl+left click, then drag or click the opposite corner; on macOS Cmd+click). A box with no width or no height is ignored",
            mutates: false,
            enabled: always,
            run: |e, p| {
                let a = Point::new(num(p, "x1")?, num(p, "y1")?);
                let b = Point::new(num(p, "x2")?, num(p, "y2")?);
                if !(a.is_finite() && b.is_finite()) {
                    return Err(EngineError::bad_params("box corners must be finite"));
                }
                let (w, h) = ((a.x - b.x).abs(), (a.y - b.y).abs());
                if w >= 1.0 && h >= 1.0 {
                    let px = (e.view.size.0.max(10.0) / w).min(e.view.size.1.max(10.0) / h);
                    e.view.zoom = (px / crate::view::BASE_PX_PER_FIG).clamp(crate::view::MIN_ZOOM, crate::view::MAX_ZOOM);
                    e.view.origin = Point::new(a.x.min(b.x), a.y.min(b.y));
                }
                Ok(json!({"zoom": e.view.zoom}))
            },
        },
        Command {
            id: "view.zoom_set",
            label: "Set zoom",
            menu: "",
            shortcut: "",
            params: "{\"zoom\": factor (1 = 80 px/inch), \"x\"?, \"y\"?: screen anchor}",
            mutates: false,
            enabled: always,
            run: |e, p| {
                let z = num(p, "zoom")?;
                if z <= 0.0 {
                    return Err(EngineError::bad_params("zoom must be positive"));
                }
                let a = Point::new(
                    num_or(p, "x", e.view.size.0 / 2.0)?,
                    num_or(p, "y", e.view.size.1 / 2.0)?,
                );
                let z = if e.view.integer_zoom && z > 1.99 { z.round() } else { z };
                e.view.zoom_about(z, a);
                Ok(json!({"zoom": e.view.zoom}))
            },
        },
        Command {
            id: "view.set_integer_zoom",
            label: "Integer zoom",
            menu: "",
            shortcut: "",
            params: "{\"on\"?: bool (default: toggle)}  — xfig's Integer zoom check box in the Zoom pop-up: zoom values above 1 are whole numbers (zoom_set rounds, zoom_fit truncates)",
            mutates: false,
            enabled: always,
            run: |e, p| {
                e.view.integer_zoom = opt_bool(p, "on")?.unwrap_or(!e.view.integer_zoom);
                Ok(json!({"integer_zoom": e.view.integer_zoom}))
            },
        },
        Command {
            id: "view.pan",
            label: "Pan",
            menu: "",
            shortcut: "",
            params: "{\"dx\", \"dy\"}  â screen pixels",
            mutates: false,
            enabled: always,
            run: |e, p| {
                e.view.pan_pixels(num(p, "dx")?, num(p, "dy")?);
                Ok(json!({}))
            },
        },
        Command {
            id: "view.set_scale",
            label: "Unit/Scale settings",
            menu: "",
            shortcut: "",
            params: "{\"decimal\"?: bool (imperial rulers in tenths), \"unit\"?: figure unit name (empty: the ruler unit), \"scale\"?: figure units per ruler unit}",
            mutates: false,
            enabled: always,
            run: |e, p| {
                let decimal = opt_bool(p, "decimal")?;
                let unit = opt_str(p, "unit")?;
                let scale = opt_num(p, "scale")?;
                if let Some(u) = unit {
                    if u.chars().count() > crate::view::MAX_UNIT_NAME || u.chars().any(char::is_control) {
                        return Err(EngineError::bad_params("unit name too long or not printable"));
                    }
                }
                if let Some(s) = scale {
                    if !(s.is_finite() && (1e-6..=1e6).contains(&s)) {
                        return Err(EngineError::bad_params("scale must be between 0.000001 and 1000000"));
                    }
                }
                if let Some(d) = decimal {
                    e.unit_scale.decimal_inches = d;
                }
                if let Some(u) = unit {
                    e.unit_scale.unit = u.trim().to_string();
                }
                if let Some(s) = scale {
                    e.unit_scale.factor = s;
                }
                Ok(json!({"label": e.unit_label()}))
            },
        },
        Command {
            id: "view.pan_origin",
            label: "Pan to Origin",
            menu: "View",
            shortcut: "",
            params: "{}",
            mutates: false,
            enabled: always,
            run: |e, _| {
                let s = e.view.px_per_fig();
                e.view.origin = Point::new(-40.0 / s, -40.0 / s);
                Ok(json!({}))
            },
        },
        Command {
            id: "view.set_size",
            label: "Set canvas size",
            menu: "",
            shortcut: "",
            params: "{\"width\", \"height\"}  â pixels (set by the UI)",
            mutates: false,
            enabled: always,
            run: |e, p| {
                let (w, h) = (num(p, "width")?, num(p, "height")?);
                e.view.size = (w.clamp(1.0, 100_000.0), h.clamp(1.0, 100_000.0));
                Ok(json!({}))
            },
        },
        Command {
            id: "view.toggle_grid",
            label: "Show Grid",
            menu: "View",
            shortcut: "Ctrl+'",
            params: "{}",
            mutates: false,
            enabled: always,
            run: |e, _| {
                e.indicator.grid = match e.indicator.grid {
                    Some(_) => None,
                    None => Some(match e.indicator.snap {
                        SnapMode::Any => SnapMode::Medium,
                        s => s,
                    }),
                };
                Ok(json!({"grid": e.indicator.grid.is_some()}))
            },
        },
        Command {
            id: "view.set_grid",
            label: "Set grid",
            menu: "",
            shortcut: "",
            params: "{\"grid\": none|fine|small|medium|large|coarse}",
            mutates: false,
            enabled: always,
            run: |e, p| {
                e.indicator.grid = match opt_str(p, "grid")?.unwrap_or("none") {
                    "none" | "off" => None,
                    s => Some(parse_mode(s)?),
                };
                Ok(json!({}))
            },
        },
        Command {
            id: "view.set_snap",
            label: "Set point positioning",
            menu: "",
            shortcut: "",
            params: "{\"snap\": any|fine|small|medium|large|coarse}",
            mutates: false,
            enabled: always,
            run: |e, p| {
                e.indicator.snap = parse_mode(opt_str(p, "snap")?.unwrap_or("any"))?;
                Ok(json!({}))
            },
        },
        Command {
            id: "view.toggle_lengths",
            label: "Show Lengths",
            menu: "",
            shortcut: "",
            params: "{\"on\"?: bool}  â while drawing, show the dx / dy triangle of the segment with its two sides' lengths (xfig's Show Lengths; the message panel always shows length, dx, dy and angle)",
            mutates: false,
            enabled: always,
            run: |e, p| {
                e.show_lengths = super::opt_bool(p, "on")?.unwrap_or(!e.show_lengths);
                Ok(json!({"show_lengths": e.show_lengths}))
            },
        },
        Command {
            id: "view.toggle_vertex_numbers",
            label: "Show vertex numbers",
            menu: "",
            shortcut: "",
            params: "{\"on\"?: bool}  â number the points of lines, polygons and splines on the canvas (xfig's Show vertex numbers; resource shownums)",
            mutates: false,
            enabled: always,
            run: |e, p| {
                let on = super::opt_bool(p, "on")?.unwrap_or(!e.show_vertex_numbers());
                e.prefs
                    .set("shownums", toml::Value::Boolean(on))
                    .map_err(EngineError::bad_params)?;
                e.prefs_dirty = true;
                Ok(json!({"show_vertex_numbers": on}))
            },
        },
        Command {
            id: "view.toggle_page_border",
            label: "Show Page Border",
            menu: "View",
            shortcut: "Ctrl+B",
            params: "{}",
            mutates: false,
            enabled: always,
            run: |e, _| {
                e.show_page_border = !e.show_page_border;
                Ok(json!({"page_border": e.show_page_border}))
            },
        },
    ]
}

fn parse_mode(s: &str) -> Result<SnapMode, EngineError> {
    serde_json::from_value(Value::String(s.to_string()))
        .map_err(|_| EngineError::bad_params(format!("unknown positioning mode {s:?}")))
}

fn zoom_by(e: &mut Engine, f: f64) -> CmdResult {
    let c = Point::new(e.view.size.0 / 2.0, e.view.size.1 / 2.0);
    let z = e.view.zoom * f;
    e.view.zoom_about(z, c);
    Ok(json!({"zoom": e.view.zoom}))
}
