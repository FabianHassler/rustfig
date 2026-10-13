//! Document settings, colours and depth visibility.

use super::{always, num, opt_bool, string, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_model::{Orientation, PaperSize, Rgb, Units};
use serde_json::{json, Value};

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "doc.info",
            label: "Document info",
            menu: "",
            shortcut: "",
            params: "{}",
            mutates: false,
            enabled: always,
            run: info,
        },
        Command {
            id: "doc.set_units",
            label: "Set units",
            menu: "",
            shortcut: "",
            params: "{\"units\": inches|metric}",
            mutates: true,
            enabled: always,
            run: |e, p| {
                let u = match string(p, "units")?.as_str() {
                    "inches" | "in" => Units::Inches,
                    "metric" | "cm" => Units::Metric,
                    o => return Err(EngineError::bad_params(format!("unknown units {o:?}"))),
                };
                e.doc_mut().settings.units = u;
                Ok(json!({}))
            },
        },
        Command {
            id: "doc.set_paper",
            label: "Set paper size",
            menu: "",
            shortcut: "",
            params: "{\"paper\": Letter|Legal|Ledger|Tabloid|A|B|C|D|E|A4|A3|A2|A1|A0|B5}",
            mutates: true,
            enabled: always,
            run: |e, p| {
                let name = string(p, "paper")?;
                let paper = PaperSize::from_name(&name)
                    .ok_or_else(|| EngineError::bad_params(format!("unknown paper size {name:?}")))?;
                e.doc_mut().settings.paper = paper;
                Ok(json!({}))
            },
        },
        Command {
            id: "doc.toggle_orientation",
            label: "Portrait/Landscape",
            menu: "View",
            shortcut: "",
            params: "{}",
            mutates: true,
            enabled: always,
            run: |e, _| {
                let s = &mut e.doc_mut().settings;
                s.orientation = match s.orientation {
                    Orientation::Landscape => Orientation::Portrait,
                    Orientation::Portrait => Orientation::Landscape,
                };
                Ok(json!({}))
            },
        },
        Command {
            id: "doc.set_magnification",
            label: "Set export magnification",
            menu: "",
            shortcut: "",
            params: "{\"magnification\": percent}",
            mutates: true,
            enabled: always,
            run: |e, p| {
                let m = num(p, "magnification")?;
                if !(0.1..=100_000.0).contains(&m) {
                    return Err(EngineError::bad_params("magnification must be between 0.1 and 100000"));
                }
                e.doc_mut().settings.magnification = m;
                Ok(json!({}))
            },
        },
        Command {
            id: "doc.set_comments",
            label: "Set figure comments",
            menu: "",
            shortcut: "",
            params: "{\"comments\": string}",
            mutates: true,
            enabled: always,
            run: |e, p| {
                let c = string(p, "comments")?;
                e.doc_mut().comments = c.lines().map(str::to_string).collect();
                Ok(json!({}))
            },
        },
        Command {
            id: "color.add",
            label: "Add user colour",
            menu: "",
            shortcut: "",
            params: "{\"rgb\": \"#rrggbb\"}  — returns the colour number",
            mutates: true,
            enabled: always,
            run: |e, p| {
                let s = string(p, "rgb")?;
                let rgb = Rgb::from_hex(&s).ok_or_else(|| EngineError::bad_params("rgb must be #rrggbb"))?;
                let c = e
                    .doc_mut()
                    .color_for_rgb(rgb)
                    .ok_or_else(|| EngineError::bad_params("all 512 user colours are in use"))?;
                Ok(json!({"color": c.0}))
            },
        },
        Command {
            id: "color.remove_unused",
            label: "Remove unused colours",
            menu: "",
            shortcut: "",
            params: "{}",
            mutates: true,
            enabled: always,
            run: |e, _| {
                let before = e.doc().colors.len();
                e.doc_mut().remove_unused_colors();
                Ok(json!({"removed": before - e.doc().colors.len()}))
            },
        },
        Command {
            id: "depth.set_visible",
            label: "Show/hide depth",
            menu: "",
            shortcut: "",
            params: "{\"depth\": 0-999, \"visible\": bool}",
            mutates: false,
            enabled: always,
            run: |e, p| {
                let d = num(p, "depth")?.round() as i32;
                let vis = opt_bool(p, "visible")?.unwrap_or(true);
                if vis {
                    e.hidden_depths.remove(&d);
                } else {
                    e.hidden_depths.insert(d);
                }
                e.bump_revision();
                Ok(json!({}))
            },
        },
        Command {
            id: "depth.show_all",
            label: "Show All Depths",
            menu: "View",
            shortcut: "",
            params: "{}",
            mutates: false,
            enabled: always,
            run: |e, _| {
                e.hidden_depths.clear();
                e.bump_revision();
                Ok(json!({}))
            },
        },
    ]
}

fn info(e: &mut Engine, _: &Value) -> CmdResult {
    let d = e.doc();
    let s = &d.settings;
    let bb = d.bbox();
    Ok(json!({
        "path": e.path(),
        "modified": e.is_modified(),
        "objects": d.objects.len(),
        "leaves": d.leaf_count(),
        "colors": d.colors.len(),
        "depths": d.depths(),
        "hidden_depths": e.hidden_depths.iter().collect::<Vec<_>>(),
        "orientation": format!("{:?}", s.orientation),
        "units": format!("{:?}", s.units),
        "paper": s.paper.name(),
        "magnification": s.magnification,
        "bounds": if bb.is_empty() { Value::Null } else { json!({"x": bb.min.x, "y": bb.min.y, "width": bb.width(), "height": bb.height()}) },
        "selection": super::ids_value(e.selection()),
        "tool": e.tool().id(),
    }))
}
