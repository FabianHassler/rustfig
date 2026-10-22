//! Export line as data (plan S4-8): xfig's Digitize (`w_digitize.c`) the
//! other way round. xfig writes the points a user clicks on a scanned plot
//! to numbered files (`prefix_0001.suffix`, new file per line or appended to
//! one file); RustFig writes the points of chosen polylines and splines, in
//! figure units or, with two calibration points, in the plot's data units.
//
// Ported from xfig src/w_digitize.c (xfig licence, see LICENSE-xfig).

use super::{has_objects, num_or, opt_bool, opt_str, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_geom::Point;
use serde_json::{json, Value};
use std::io::Write;

/// Largest starting number: the name keeps its four digits (xfig's `%04d`).
const MAX_SEQ: f64 = 99_999_999.0;
const MAX_FILES: usize = 10_000;

pub(super) fn commands() -> Vec<Command> {
    vec![Command {
        id: "file.digitize",
        label: "Export line as data...",
        menu: "",
        shortcut: "",
        params: "{\"ids\"?: [id], \"prefix\": string, \"start\"?: int (0), \"suffix\"?: string (\"xy\"; \"csv\" writes comma-separated with a header), \"append\"?: bool (false), \"calibration\"?: {\"points\": [{\"x\",\"y\",\"vx\",\"vy\"}, {…}], \"log_x\"?: bool, \"log_y\"?: bool}, \"dry_run\"?: bool}  — xfig's Digitize reversed: write the points of the selected polylines and splines to prefix_NNNN.suffix (one file per line, numbers counting up from start; append: all lines in the one file, added to it if it exists, blank line between lines). Figure units (1/1200 inch), or data units through the two calibration points (figure x,y of a reference point and its value vx,vy; log axes interpolate in log10). Returns {files, lines, points, next, text?}; dry_run only returns the text",
        mutates: false,
        enabled: has_objects,
        run: digitize,
    }]
}

struct Axis {
    f1: f64,
    f2: f64,
    v1: f64,
    v2: f64,
    log: bool,
}

impl Axis {
    fn new(f1: f64, f2: f64, v1: f64, v2: f64, log: bool, name: &str) -> Result<Axis, EngineError> {
        if ![f1, f2, v1, v2].iter().all(|v| v.is_finite()) {
            return Err(EngineError::bad_params(format!(
                "calibration {name}: numbers must be finite"
            )));
        }
        if (f1 - f2).abs() < 1e-9 {
            return Err(EngineError::bad_params(format!(
                "calibration {name}: the two reference points need different positions"
            )));
        }
        if log && (v1 <= 0.0 || v2 <= 0.0) {
            return Err(EngineError::bad_params(format!(
                "calibration {name}: a logarithmic axis needs positive values"
            )));
        }
        Ok(Axis { f1, f2, v1, v2, log })
    }

    fn map(&self, f: f64) -> f64 {
        let t = (f - self.f1) / (self.f2 - self.f1);
        if self.log {
            let (a, b) = (self.v1.log10(), self.v2.log10());
            10f64.powf(a + t * (b - a))
        } else {
            self.v1 + t * (self.v2 - self.v1)
        }
    }
}

fn point_num(pt: &Value, key: &str) -> Result<f64, EngineError> {
    pt.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| EngineError::bad_params(format!("calibration point needs a number \"{key}\"")))
}

fn calibration(p: &Value) -> Result<Option<(Axis, Axis)>, EngineError> {
    let c = match p.get("calibration") {
        None | Some(Value::Null) => return Ok(None),
        Some(c) => c,
    };
    let pts = c
        .get("points")
        .and_then(Value::as_array)
        .filter(|a| a.len() == 2)
        .ok_or_else(|| EngineError::bad_params("calibration.points must be two reference points"))?;
    let (a, b) = (&pts[0], &pts[1]);
    let flag = |k: &str| c.get(k).and_then(Value::as_bool).unwrap_or(false);
    let x = Axis::new(
        point_num(a, "x")?,
        point_num(b, "x")?,
        point_num(a, "vx")?,
        point_num(b, "vx")?,
        flag("log_x"),
        "x",
    )?;
    let y = Axis::new(
        point_num(a, "y")?,
        point_num(b, "y")?,
        point_num(a, "vy")?,
        point_num(b, "vy")?,
        flag("log_y"),
        "y",
    )?;
    Ok(Some((x, y)))
}

fn fmt(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        let s = format!("{v:.6}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn line_text(points: &[Point], cal: &Option<(Axis, Axis)>, csv: bool) -> String {
    let mut out = String::new();
    for p in points {
        let (x, y) = match cal {
            Some((ax, ay)) => (ax.map(p.x), ay.map(p.y)),
            None => (p.x, p.y),
        };
        out.push_str(&fmt(x));
        out.push(if csv { ',' } else { ' ' });
        out.push_str(&fmt(y));
        out.push('\n');
    }
    out
}

fn digitize(e: &mut Engine, p: &Value) -> CmdResult {
    let prefix = opt_str(p, "prefix")?.unwrap_or("");
    let dry = opt_bool(p, "dry_run")?.unwrap_or(false);
    if prefix.trim().is_empty() && !dry {
        return Err(EngineError::bad_params("give a file name \"prefix\""));
    }
    let suffix = opt_str(p, "suffix")?.unwrap_or("xy").trim_start_matches('.');
    if suffix.is_empty() || suffix.contains(['/', '\\']) {
        return Err(EngineError::bad_params("suffix must be a file name extension"));
    }
    let start = num_or(p, "start", 0.0)?;
    if !start.is_finite() || !(0.0..=MAX_SEQ).contains(&start) {
        return Err(EngineError::bad_params(format!(
            "start must be a number from 0 to {MAX_SEQ}"
        )));
    }
    let start = start.trunc() as u64;
    let append = opt_bool(p, "append")?.unwrap_or(false);
    let cal = calibration(p)?;
    let csv = suffix.eq_ignore_ascii_case("csv");
    let mut lines: Vec<Vec<Point>> = Vec::new();
    for id in e.target_ids(p)? {
        if let Some(o) = e.doc().get(id) {
            use rustfig_model::{ObjectKind, PolyKind};
            let is_line = match &o.kind {
                ObjectKind::Polyline(l) => matches!(l.kind, PolyKind::Polyline | PolyKind::Polygon),
                ObjectKind::Spline(_) => true,
                _ => false,
            };
            if is_line {
                if let Some(pts) = o.points() {
                    lines.push(pts.to_vec());
                }
            }
        }
    }
    if lines.is_empty() {
        return Err(EngineError::bad_params("select polylines or splines to export"));
    }
    if lines.len() > MAX_FILES {
        return Err(EngineError::bad_params("too many lines"));
    }
    let header = if csv {
        if cal.is_some() {
            "x,y\n"
        } else {
            "x_fig,y_fig\n"
        }
    } else {
        ""
    };
    let name = |seq: u64| format!("{prefix}_{seq:04}.{suffix}");
    let total: usize = lines.iter().map(Vec::len).sum();
    // The files to write: (name, text, append to an existing file).
    let mut files: Vec<(String, String, bool)> = Vec::new();
    if append {
        let mut text = String::new();
        for (i, l) in lines.iter().enumerate() {
            if i > 0 {
                text.push('\n');
            }
            text.push_str(&line_text(l, &cal, csv));
        }
        files.push((name(start), text, true));
    } else {
        for (i, l) in lines.iter().enumerate() {
            let text = format!("{header}{}", line_text(l, &cal, csv));
            files.push((name(start + i as u64), text, false));
        }
    }
    let next = if append { start } else { start + lines.len() as u64 };
    if dry {
        let all: Vec<&str> = files.iter().map(|f| f.1.as_str()).collect();
        return Ok(json!({"files": [], "lines": lines.len(), "points": total, "next": next, "text": all.join("\n")}));
    }
    let mut written = Vec::new();
    for (path, text, app) in &files {
        let mut text = text.clone();
        if *app {
            let exists = std::fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false);
            if exists {
                text.insert(0, '\n');
            } else {
                text.insert_str(0, header);
            }
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|err| EngineError::io(format!("{path}: {err}")))?;
            f.write_all(text.as_bytes())
                .map_err(|err| EngineError::io(format!("{path}: {err}")))?;
        } else {
            std::fs::write(path, text.as_bytes()).map_err(|err| EngineError::io(format!("{path}: {err}")))?;
        }
        written.push(path.clone());
    }
    e.set_status(format!("Wrote {} line(s) to {}", lines.len(), written.join(", ")));
    Ok(json!({"files": written, "lines": lines.len(), "points": total, "next": next}))
}
