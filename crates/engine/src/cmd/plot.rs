//! Plot a curve (plan E12): `plot.add` draws a function, a parametric curve
//! or data from a file as one compound of polylines with axes; the plot's
//! parameters stay in the compound's comment, so `plot.update` and the
//! dialog (`plot.edit`) redraw it. The geometry is in [`crate::plot`].

use super::{always, num_or, opt_str, CmdResult, Command};
use crate::plot::{self, PlotLook, PlotMode, PlotSpec};
use crate::{Engine, EngineError, UiRequest};
use rustfig_geom::{Point, Rect};
use rustfig_model::{ColorRef, Object, ObjectId, ObjectKind};
use serde_json::{json, Value};
use std::sync::Arc;

/// Default frame of a new plot, Fig units (4 by 2.5 inches).
pub const DEFAULT_W: f64 = 4800.0;
pub const DEFAULT_H: f64 = 3000.0;

pub(super) fn commands() -> Vec<Command> {
    vec![
        Command {
            id: "plot.panel",
            label: "Plot curve...",
            menu: "Edit",
            shortcut: "Ctrl+Shift+P",
            params: "{}  — open the Plot dialog for a new plot at the origin (the Plot mode button places it where you drag)",
            mutates: false,
            enabled: always,
            run: |e, _| {
                e.request(UiRequest::PlotDialog { rect: None, edit: None });
                Ok(json!({"requested": true}))
            },
        },
        Command {
            id: "plot.edit",
            label: "Edit plot...",
            menu: "Edit",
            shortcut: "",
            params: "{\"id\"?: plot compound (default: the selected one)}  — open the Plot dialog on an existing plot (Edit mode's left button on a plot does the same)",
            mutates: false,
            enabled: always,
            run: edit,
        },
        Command {
            id: "plot.get",
            label: "Get plot parameters",
            menu: "",
            shortcut: "",
            params: "{\"id\"?}  — the parameters kept in a plot's comment: {\"id\", \"spec\": {…}, \"frame\": [x0, y0, x1, y1]}",
            mutates: false,
            enabled: always,
            run: get,
        },
        Command {
            id: "plot.add",
            label: "Add plot",
            menu: "",
            shortcut: "",
            params: "{\"x\"?: 0, \"y\"?: 0, \"width\"?: 4800, \"height\"?: 3000 (the frame, Fig units), plus the plot's parameters}  — draw a plot as one compound; returns {id, curves, points}. Parameters: mode \"function\" (y = f(x): \"expr\") | \"parametric\" (\"xt\", \"yt\" in t; \"tmin\", \"tmax\") | \"data\" (\"file\" and 1-based columns \"xcol\" (0: row number), \"ycol\"; \"data\": the text itself instead of the file), \"xmin\" \"xmax\" \"ymin\" \"ymax\" (null: fit), \"samples\" (400, refined where the curve bends; no line across poles), \"frame\" (true: the box), \"xticks\" and \"yticks\" ({\"ticks\": true (marks), \"numbers\": true, \"manual\", \"first\", \"step\", \"count\", \"minor\", \"length\", \"dir\": in|out|both, \"format\", \"decimals\"}; ticks sit on the frame's edges even when the box is off), \"axes\" (old all-in-one: false turns frame, ticks and numbers off), \"grid\", \"log_x\", \"log_y\", \"xlabel\", \"ylabel\", \"latex\" (labels are LaTeX special text). Expressions: + - * / ^, parentheses, pi, e, sin cos tan asin acos atan sinh cosh tanh exp ln log log10 log2 sqrt abs floor ceil sign, atan2 min max pow(a,b); line style, depth and font come from the indicator",
            mutates: true,
            enabled: always,
            run: add,
        },
        Command {
            id: "plot.update",
            label: "Redraw plot",
            menu: "",
            shortcut: "",
            params: "{\"id\"?: plot compound (default: the selected one), any of the plot parameters of plot.add to change}  — redraw the plot in its present frame (where the user moved or scaled it) with the changed parameters; returns {id, curves, points}",
            mutates: true,
            enabled: always,
            run: update,
        },
    ]
}

fn look(e: &Engine) -> PlotLook {
    let i = &e.indicator;
    PlotLook {
        curve: i.style(),
        text_color: ColorRef(i.pen_color),
        font: i.font,
        font_size: i.font_size,
        postscript: i.postscript,
        grid_gap: i.dot_gap.clamp(1.0, 100.0),
    }
}

/// The plot compound `id` names (default the selected one).
fn which(e: &Engine, p: &Value) -> Result<ObjectId, EngineError> {
    let id = match p.get("id") {
        None | Some(Value::Null) => match e.selection() {
            [one] => *one,
            _ => return Err(EngineError::bad_params("select one plot or give its \"id\"")),
        },
        Some(v) => ObjectId(
            v.as_u64()
                .ok_or_else(|| EngineError::bad_params("\"id\" must be an object id"))?,
        ),
    };
    match e.doc().get(id) {
        Some(o) if is_plot(o) => Ok(id),
        Some(_) => Err(EngineError::bad_params(format!("object {} is not a plot", id.0))),
        None => Err(EngineError::bad_params(format!("no object {}", id.0))),
    }
}

/// The text of the data file (data mode only): the `data` parameter, or the
/// file, up to [`plot::MAX_DATA_BYTES`].
fn data_text(spec: &PlotSpec, p: &Value) -> Result<Option<String>, EngineError> {
    if spec.mode != PlotMode::Data {
        return Ok(None);
    }
    if let Some(t) = opt_str(p, "data")? {
        if t.len() as u64 > plot::MAX_DATA_BYTES {
            return Err(EngineError::bad_params("the data is too large"));
        }
        return Ok(Some(t.to_string()));
    }
    if spec.file.trim().is_empty() {
        return Err(EngineError::bad_params("give a data \"file\" (or the \"data\" text)"));
    }
    read_file(&spec.file).map(Some)
}

#[cfg(not(target_arch = "wasm32"))]
fn read_file(path: &str) -> Result<String, EngineError> {
    use std::io::Read;
    let path = std::path::Path::new(path.trim());
    let f = std::fs::File::open(path).map_err(|e| EngineError::io(format!("{}: {e}", path.display())))?;
    let mut buf = Vec::new();
    // One byte more than the cap tells a file that is too large.
    f.take(plot::MAX_DATA_BYTES + 1)
        .read_to_end(&mut buf)
        .map_err(|e| EngineError::io(format!("{}: {e}", path.display())))?;
    if buf.len() as u64 > plot::MAX_DATA_BYTES {
        return Err(EngineError::bad_params(format!(
            "{} is larger than {} MB",
            path.display(),
            plot::MAX_DATA_BYTES / (1024 * 1024)
        )));
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

#[cfg(target_arch = "wasm32")]
fn read_file(_: &str) -> Result<String, EngineError> {
    Err(EngineError::bad_params(
        "reading a data file is not available on the web: give the text as \"data\"",
    ))
}

fn bad(m: String) -> EngineError {
    EngineError::bad_params(m)
}

fn add(e: &mut Engine, p: &Value) -> CmdResult {
    let mut spec = PlotSpec::from_value(p).map_err(bad)?;
    spec.rect = None;
    let (x, y) = (num_or(p, "x", 0.0)?, num_or(p, "y", 0.0)?);
    let (w, h) = (num_or(p, "width", DEFAULT_W)?, num_or(p, "height", DEFAULT_H)?);
    let rect = Rect::new(Point::new(x, y), Point::new(x + w, y + h));
    let data = data_text(&spec, p)?;
    let built = plot::build(&spec, rect, data.as_deref(), &look(e)).map_err(bad)?;
    let id = e.add_object(built.object);
    e.set_selection(vec![id]);
    Ok(json!({"id": id.0, "curves": built.curves, "points": built.points}))
}

fn update(e: &mut Engine, p: &Value) -> CmdResult {
    let id = which(e, p)?;
    let (old, frame) = {
        let o = e
            .doc()
            .get(id)
            .ok_or_else(|| EngineError::bad_params("the plot is gone"))?;
        let spec = PlotSpec::of_object(o).ok_or_else(|| EngineError::bad_params("not a plot"))?;
        let frame = plot::frame_of(o, &spec).ok_or_else(|| EngineError::bad_params("the plot has lost its frame"))?;
        (spec, frame)
    };
    // The changed parameters over the kept ones.
    let mut merged = serde_json::to_value(&old).map_err(|e| EngineError::internal(e.to_string()))?;
    if let (Some(m), Some(given)) = (merged.as_object_mut(), p.as_object()) {
        for (k, v) in given {
            if k != "id" && k != "data" && k != "rect" {
                m.insert(k.clone(), v.clone());
            }
        }
    }
    let spec = PlotSpec::from_value(&merged).map_err(bad)?;
    let data = data_text(&spec, p)?;
    let built = plot::build(&spec, frame, data.as_deref(), &look(e)).map_err(bad)?;
    let mut obj = built.object;
    let doc = e.doc_mut();
    let idx = doc
        .index_of(id)
        .ok_or_else(|| EngineError::bad_params("the plot is gone"))?;
    // Only the generated parts are replaced: what the user added to the
    // plot (in an open compound) and left unmarked stays, and so do other
    // comments of the compound.
    if let Some(Object {
        kind: ObjectKind::Compound(old),
        comments,
        ..
    }) = doc.objects.get(idx).map(|o| &**o)
    {
        if let ObjectKind::Compound(new) = &mut obj.kind {
            new.objects
                .extend(old.objects.iter().filter(|m| !plot::is_part(m)).cloned());
        }
        obj.comments.extend(
            comments
                .iter()
                .filter(|c| !c.trim_start().starts_with(plot::PLOT_COMMENT))
                .cloned(),
        );
    }
    doc.reassign_ids(&mut obj);
    obj.id = id;
    if let Some(slot) = doc.objects.get_mut(idx) {
        *slot = Arc::new(obj);
    }
    Ok(json!({"id": id.0, "curves": built.curves, "points": built.points}))
}

fn get(e: &mut Engine, p: &Value) -> CmdResult {
    let id = which(e, p)?;
    let o = e
        .doc()
        .get(id)
        .ok_or_else(|| EngineError::bad_params("the plot is gone"))?;
    let spec = PlotSpec::of_object(o).ok_or_else(|| EngineError::bad_params("not a plot"))?;
    let frame = plot::frame_of(o, &spec).map(|r| [r.min.x, r.min.y, r.max.x, r.max.y]);
    Ok(json!({"id": id.0, "spec": spec, "frame": frame}))
}

fn edit(e: &mut Engine, p: &Value) -> CmdResult {
    let id = which(e, p)?;
    e.request(UiRequest::PlotDialog {
        rect: None,
        edit: Some(id),
    });
    Ok(json!({"requested": true, "id": id.0}))
}

/// Whether the object is a plot made by `plot.add`.
pub fn is_plot(o: &rustfig_model::Object) -> bool {
    matches!(o.kind, ObjectKind::Compound(_)) && PlotSpec::of_object(o).is_some()
}
