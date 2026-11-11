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
            menu: "Tools",
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
            menu: "Tools",
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
            params: "{\"x\"?: 0, \"y\"?: 0, \"width\"?: 4800, \"height\"?: 3000 (the frame, Fig units), plus the plot's parameters}  — draw a plot as one compound; returns {id, curves, points, markers, warnings} (a curve outside the ranges: curves 0 and a warning). Parameters: mode \"function\" (y = f(x): \"expr\"; empty: only frame, ticks and labels, with xmin..ymax given) | \"parametric\" (\"xt\", \"yt\" in t; \"tmin\", \"tmax\") | \"data\" (\"file\" and 1-based columns \"xcol\" (0: row number), \"ycol\"; \"data\": the text itself instead of the file), \"xmin\" \"xmax\" \"ymin\" \"ymax\" (null: fit), \"samples\" (400, refined where the curve bends; no line across poles), \"frame\" (true: the box), \"xticks\" and \"yticks\" ({\"ticks\": true (marks), \"numbers\": true, \"manual\", \"first\", \"step\", \"count\", \"minor\", \"length\", \"dir\": in|out|both, \"format\", \"decimals\", \"scale\": power of ten the numbers are divided by (x10^n at the axis end; not on log axes), \"comma\": decimal comma, \"labels\": [[value, \"text\"]...] (own labels and categories: the ticks stand at these values, the texts as typed), \"rotate\": degrees counterclockwise}; ticks sit on the frame's edges even when the box is off), \"axes\": \"frame\" (ticks and numbers on the box's edges) | \"origin\" (axes crossing at (0, 0) or the nearest edge of the range, arrows and axis labels at the tips, ticks and numbers on one side; no box) | \"none\" (curves only; the old false turns frame, ticks and numbers off), \"axis_arrows\" (true: arrowheads of origin axes), \"grid\" (with \"grid_minor\": lines at the minor ticks too, \"grid_style\": dotted|dashed|solid for the major lines), \"log_x\", \"log_y\", \"xlabel\", \"ylabel\", \"latex\" (the axis labels are LaTeX text with the TeX flag), \"latex_numbers\" (the tick numbers too, in math mode: $-1.5$, $10^{3}$; both for all-LaTeX text). Expressions: + - * / ^, parentheses, pi, e, sin cos tan asin acos atan sinh cosh tanh exp ln log log10 log2 sqrt abs floor ceil sign heaviside gamma lgamma erf erfc j0 j1 y0 y1 i0 i1 k0 k1, atan2 min max pow(a,b) jn(n,x) yn(n,x) in(n,x) kn(n,x); several curves: \"curves\": [{the curve's source as above: \"expr\" | \"xt\", \"yt\", \"tmin\", \"tmax\" | \"file\", \"xcol\", \"ycol\" | \"data\" (text) | \"points\": [[x, y]...]; \"mode\"? (default from the source), \"samples\", \"pen_color\", \"thickness\", \"line\" (a line style or \"none\": markers only), \"style_val\", \"marker\": none|circle|square|triangle|diamond|cross|plus (at the data points or the samples), \"marker_size\" (Fig units, 75), \"marker_fill\" (true)}] (at most 50; the ranges fit them all; inline data and points are kept in the figure, up to 20000 points, so plot.update redraws without them; plot.update with \"curves\" replaces the list); \"depth\" (all parts), \"pen_color\", \"thickness\", \"line\", \"style_val\" (the curves' style where a curve sets none); font comes from the indicator, and so do depth and style when not given",
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
    let id = match super::param(p, "id") {
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

/// The listed curves with the text of their data files read (the file is
/// read again at every redraw; its points are not kept in the figure).
fn read_curve_files(spec: &mut PlotSpec) -> Result<(), EngineError> {
    for c in spec.curves.iter_mut() {
        if c.mode() == PlotMode::Data && c.points.is_empty() && c.data.is_empty() && !c.file.trim().is_empty() {
            c.data = read_file(&c.file)?;
        }
    }
    Ok(())
}

/// The text of the data file (data mode only): the `data` parameter, or the
/// file, up to [`plot::MAX_DATA_BYTES`].
fn data_text(spec: &PlotSpec, p: &Value) -> Result<Option<String>, EngineError> {
    if spec.mode != PlotMode::Data || !spec.curves.is_empty() {
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

/// Notes the plot's parameters as read (serde reads them, not [`super::param`]).
fn read_spec_keys(p: &Value) {
    if let Ok(Value::Object(m)) = serde_json::to_value(PlotSpec::default()) {
        for k in m.keys().map(String::as_str).chain(["axes", "data"]) {
            let _ = super::param(p, k);
        }
    }
}

fn add(e: &mut Engine, p: &Value) -> CmdResult {
    read_spec_keys(p);
    let mut spec = PlotSpec::from_value(p).map_err(bad)?;
    spec.rect = None;
    let (x, y) = (num_or(p, "x", 0.0)?, num_or(p, "y", 0.0)?);
    let (w, h) = (num_or(p, "width", DEFAULT_W)?, num_or(p, "height", DEFAULT_H)?);
    let rect = Rect::new(Point::new(x, y), Point::new(x + w, y + h));
    read_curve_files(&mut spec)?;
    let data = data_text(&spec, p)?;
    let built = plot::build(&spec, rect, data.as_deref(), &look(e)).map_err(bad)?;
    let id = e.add_object(built.object);
    e.set_selection(vec![id]);
    Ok(
        json!({"id": id.0, "curves": built.curves, "points": built.points, "markers": built.markers, "warnings": built.warnings}),
    )
}

fn update(e: &mut Engine, p: &Value) -> CmdResult {
    read_spec_keys(p);
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
    let mut spec = PlotSpec::from_value(&merged).map_err(bad)?;
    read_curve_files(&mut spec)?;
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
                .filter(|c| {
                    let c = c.trim_start();
                    !c.starts_with(plot::PLOT_COMMENT) && !c.starts_with(plot::POINTS_COMMENT)
                })
                .cloned(),
        );
    }
    doc.reassign_ids(&mut obj);
    obj.id = id;
    if let Some(slot) = doc.objects.get_mut(idx) {
        *slot = Arc::new(obj);
    }
    Ok(
        json!({"id": id.0, "curves": built.curves, "points": built.points, "markers": built.markers, "warnings": built.warnings}),
    )
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
