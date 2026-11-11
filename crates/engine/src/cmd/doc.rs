//! Document settings, colors and depth visibility.

use super::{always, num, opt_bool, opt_num, string, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_model::{BoundingBox, Orientation, PaperSize, Units};
use serde_json::{json, Value};

/// Largest bounding-box coordinate (Fig units, about 14 km at 1200 per inch).
const MAX_COORD: f64 = 1.0e9;

/// Set the figure's bounding box (`None`: the standard one, the page);
/// the box as the result.
fn set_box(e: &mut Engine, b: Option<BoundingBox>) -> CmdResult {
    if e.doc().bounding_box != b {
        e.doc_mut().bounding_box = b;
    }
    let shown = e.doc().export_box();
    Ok(json!({
        "standard": b.is_none(),
        "x1": shown.x1, "y1": shown.y1, "x2": shown.x2, "y2": shown.y2,
    }))
}

/// Depths 0–999: the most a depth list can sensibly name.
const MAX_DEPTHS: usize = 1000;

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
            id: "doc.set_latex_preamble",
            label: "Set figure LaTeX preamble",
            menu: "",
            shortcut: "",
            params: "{\"preamble\": string (this figure's LaTeX preamble lines, the only ones its texts get besides \\usepackage{color,graphicx}; saved in the file when it has LaTeX texts)}",
            mutates: true,
            enabled: always,
            run: |e, p| {
                let pre = string(p, "preamble")?;
                if pre.len() > crate::latex_preview::MAX_PREAMBLE_BYTES {
                    return Err(EngineError::bad_params("preamble: too long"));
                }
                if pre.contains('\0') {
                    return Err(EngineError::bad_params("preamble: not text"));
                }
                let pre = crate::latex_preview::tidy_preamble(&pre);
                if e.doc().latex_preamble != pre {
                    e.doc_mut().latex_preamble = pre;
                }
                Ok(json!({}))
            },
        },
        Command {
            id: "doc.reapply_latex_preamble",
            label: "Reapply to figure",
            menu: "",
            shortcut: "",
            params: "{}  — replace this figure's LaTeX preamble with the one new figures start with (view.set_latex_preamble)",
            mutates: true,
            enabled: always,
            run: |e, _| {
                let pre = e.new_figure_preamble();
                if e.doc().latex_preamble != pre {
                    e.doc_mut().latex_preamble = pre.clone();
                }
                Ok(json!({"preamble": pre}))
            },
        },
        Command {
            id: "doc.set_bounding_box",
            label: "Set bounding box",
            menu: "",
            shortcut: "",
            params: "{\"x1\", \"y1\", \"x2\", \"y2\": Fig units}  — the figure's own export box (plan F11): every export (File > Export, rustfig-cli convert, printing, rustfig_render) covers exactly this box and cuts off what lies outside; saved in the .fig as a `# RustFigBoundingBox` comment. doc.bounding_box_standard goes back to the page",
            mutates: true,
            enabled: always,
            run: |e, p| {
                let mut v = [0i32; 4];
                for (slot, key) in v.iter_mut().zip(["x1", "y1", "x2", "y2"]) {
                    let n = num(p, key)?;
                    if !n.is_finite() || n.abs() > MAX_COORD {
                        return Err(EngineError::bad_params(format!("\"{key}\" is out of range")));
                    }
                    *slot = n.round() as i32;
                }
                let [x1, y1, x2, y2] = v;
                let b = BoundingBox::new((x1, y1), (x2, y2))
                    .ok_or_else(|| EngineError::bad_params("the bounding box needs a width and a height"))?;
                set_box(e, Some(b))
            },
        },
        Command {
            id: "doc.bounding_box_standard",
            label: "Standard (page)",
            menu: "View",
            shortcut: "",
            params: "{}  — the standard bounding box: exports cover the page (paper size and orientation); nothing is saved in the .fig",
            mutates: true,
            enabled: always,
            run: |e, _| set_box(e, None),
        },
        Command {
            id: "doc.bounding_box_content",
            label: "Fit to drawing",
            menu: "View",
            shortcut: "",
            params: "{}  — the bounding box becomes the drawing's present bounds (as Zoom to fit sees them, typeset LaTeX texts included); it stays fixed when the drawing changes",
            mutates: true,
            enabled: |e| !e.doc().objects.is_empty(),
            run: |e, _| {
                let r = e.fit_bounds();
                if r.is_empty() || !(r.min.is_finite() && r.max.is_finite()) {
                    return Err(EngineError::bad_params("the drawing is empty"));
                }
                let c = |v: f64| v.clamp(-MAX_COORD, MAX_COORD) as i32;
                let b = BoundingBox::new((c(r.min.x.floor()), c(r.min.y.floor())), (c(r.max.x.ceil()), c(r.max.y.ceil())))
                    .ok_or_else(|| EngineError::bad_params("the drawing has no width or height"))?;
                set_box(e, Some(b))
            },
        },
        Command {
            id: "color.add",
            label: "Add user color",
            menu: "",
            shortcut: "",
            params: "{\"rgb\": \"#rrggbb\" | X11 color name}  — the color's number (a standard or user color with that value, else a new user color)",
            mutates: true,
            enabled: always,
            run: |e, p| {
                let s = string(p, "rgb")?;
                let rgb = rustfig_model::x_color(s.trim())
                    .ok_or_else(|| EngineError::bad_params("rgb must be #rrggbb, #rgb or an X11 color name"))?;
                e.prefs.add_recent_color(&rgb.to_hex());
                e.prefs_dirty = true;
                let c = e
                    .doc_mut()
                    .color_for_rgb(rgb)
                    .ok_or_else(|| EngineError::bad_params("all 512 user colors are in use"))?;
                Ok(json!({"color": c.0}))
            },
        },
        Command {
            id: "color.remove_unused",
            label: "Remove unused colors",
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
            label: "Show/hide depths",
            menu: "",
            shortcut: "",
            params: "{\"depth\"?: 0-999, \"depths\"?: [0-999], \"visible\": bool}  — objects on depths that are off can't be picked or edited",
            mutates: false,
            enabled: always,
            run: |e, p| {
                let mut depths = Vec::new();
                if let Some(d) = opt_num(p, "depth")? {
                    depths.push(d);
                }
                if let Some(list) = super::param(p, "depths") {
                    let list = list
                        .as_array()
                        .ok_or_else(|| EngineError::bad_params("depths must be an array of numbers"))?;
                    if list.len() > MAX_DEPTHS {
                        return Err(EngineError::bad_params("too many depths"));
                    }
                    for v in list {
                        depths.push(v.as_f64().ok_or_else(|| EngineError::bad_params("depths must be numbers"))?);
                    }
                }
                if depths.is_empty() {
                    return Err(EngineError::bad_params("give depth or depths"));
                }
                let vis = opt_bool(p, "visible")?.unwrap_or(true);
                for d in depths {
                    if !d.is_finite() {
                        return Err(EngineError::bad_params("depth must be a number"));
                    }
                    let d = (d.round() as i32).clamp(0, rustfig_model::MAX_DEPTH);
                    if vis {
                        e.hidden_depths.remove(&d);
                    } else {
                        e.hidden_depths.insert(d);
                    }
                }
                e.depths_changed();
                Ok(json!({"hidden_depths": e.hidden_depths.iter().collect::<Vec<_>>()}))
            },
        },
        Command {
            id: "depth.show_all",
            label: "All Depths On",
            menu: "View",
            shortcut: "",
            params: "{}",
            mutates: false,
            enabled: always,
            run: |e, _| {
                e.hidden_depths.clear();
                e.depths_changed();
                Ok(json!({}))
            },
        },
        Command {
            id: "depth.hide_all",
            label: "All Depths Off",
            menu: "",
            shortcut: "",
            params: "{}  — every depth in use",
            mutates: false,
            enabled: always,
            run: |e, _| {
                e.hidden_depths = e.doc().all_depths().into_iter().collect();
                e.depths_changed();
                Ok(json!({}))
            },
        },
        Command {
            id: "depth.toggle_all",
            label: "Toggle Depths",
            menu: "",
            shortcut: "",
            params: "{}  — depths in use that are on go off and vice versa",
            mutates: false,
            enabled: always,
            run: |e, _| {
                let used = e.doc().all_depths();
                e.hidden_depths = used.into_iter().filter(|d| !e.hidden_depths.contains(d)).collect();
                e.depths_changed();
                Ok(json!({}))
            },
        },
        Command {
            id: "depth.set_display",
            label: "Display of depths that are off",
            menu: "",
            shortcut: "",
            params: "{\"mode\": gray|blank}  — gray: drawn in gray; blank: not drawn",
            mutates: false,
            enabled: always,
            run: |e, p| {
                e.gray_hidden_depths = match string(p, "mode")?.as_str() {
                    "gray" => true,
                    "blank" => false,
                    o => return Err(EngineError::bad_params(format!("unknown mode {o:?} (gray or blank)"))),
                };
                e.bump_revision();
                Ok(json!({}))
            },
        },
    ]
}

fn info(e: &mut Engine, _: &Value) -> CmdResult {
    let d = e.doc();
    let s = &d.settings;
    let bb = e.doc_bounds();
    let bx = d.export_box();
    Ok(json!({
        "path": e.path(),
        "modified": e.is_modified(),
        "objects": d.objects.len(),
        "leaves": d.leaf_count(),
        "colors": d.colors.len(),
        "depths": d.all_depths(),
        "open_compounds": d.open.len(),
        "hidden_depths": e.hidden_depths.iter().collect::<Vec<_>>(),
        "hidden_depth_display": if e.gray_hidden_depths { "gray" } else { "blank" },
        "orientation": format!("{:?}", s.orientation),
        "units": format!("{:?}", s.units),
        "paper": s.paper.name(),
        "magnification": s.magnification,
        "bounds": if bb.is_empty() { Value::Null } else { json!({"x": bb.min.x, "y": bb.min.y, "width": bb.width(), "height": bb.height()}) },
        "bounding_box": {"standard": d.bounding_box.is_none(), "x1": bx.x1, "y1": bx.y1, "x2": bx.x2, "y2": bx.y2},
        "selection": super::ids_value(e.selection()),
        "tool": e.tool().id(),
    }))
}
