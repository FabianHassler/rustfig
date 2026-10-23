//! The Plot dialog (plan E12): what to draw (a function, a parametric curve
//! or data from a file), its ranges and sampling, axes and labels. "Plot"
//! runs `plot.add` (a new plot where the Plot mode's drag placed it) or
//! `plot.update` (a plot being edited); the dialog only collects fields.

use crate::App;
use rustfig_engine::plot::{self, PlotMode, PlotSpec};
use serde_json::{json, Value};

/// The dialog's fields as text; they stay between uses.
#[derive(Debug, Clone)]
pub struct PlotDialog {
    pub open: bool,
    /// The plot compound being edited (`None`: a new plot).
    pub edit: Option<u64>,
    /// Where a new plot goes: x, y, width, height (Fig units).
    pub frame: Option<[f64; 4]>,
    pub mode: PlotMode,
    pub expr: String,
    pub xt: String,
    pub yt: String,
    pub file: String,
    pub xcol: String,
    pub ycol: String,
    pub xmin: String,
    pub xmax: String,
    pub ymin: String,
    pub ymax: String,
    pub tmin: String,
    pub tmax: String,
    pub samples: String,
    pub axes: bool,
    pub grid: bool,
    pub log_x: bool,
    pub log_y: bool,
    pub xlabel: String,
    pub ylabel: String,
    pub latex: bool,
    /// The last error, shown in the dialog.
    pub error: String,
}

fn num(v: Option<f64>) -> String {
    v.map(|v| {
        let s = format!("{v:.9}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    })
    .unwrap_or_default()
}

impl Default for PlotDialog {
    fn default() -> Self {
        let mut d = PlotDialog {
            open: false,
            edit: None,
            frame: None,
            mode: PlotMode::Function,
            expr: String::new(),
            xt: String::new(),
            yt: String::new(),
            file: String::new(),
            xcol: String::new(),
            ycol: String::new(),
            xmin: String::new(),
            xmax: String::new(),
            ymin: String::new(),
            ymax: String::new(),
            tmin: String::new(),
            tmax: String::new(),
            samples: String::new(),
            axes: true,
            grid: false,
            log_x: false,
            log_y: false,
            xlabel: String::new(),
            ylabel: String::new(),
            latex: false,
            error: String::new(),
        };
        d.load(&PlotSpec::default());
        d
    }
}

impl PlotDialog {
    fn load(&mut self, s: &PlotSpec) {
        self.mode = s.mode;
        self.expr = s.expr.clone();
        self.xt = s.xt.clone();
        self.yt = s.yt.clone();
        self.file = s.file.clone();
        self.xcol = s.xcol.to_string();
        self.ycol = s.ycol.to_string();
        self.xmin = num(s.xmin);
        self.xmax = num(s.xmax);
        self.ymin = num(s.ymin);
        self.ymax = num(s.ymax);
        self.tmin = num(s.tmin);
        self.tmax = num(s.tmax);
        self.samples = s.samples.to_string();
        self.axes = s.axes;
        self.grid = s.grid;
        self.log_x = s.log_x;
        self.log_y = s.log_y;
        self.xlabel = s.xlabel.clone();
        self.ylabel = s.ylabel.clone();
        self.latex = s.latex;
        self.error.clear();
    }

    /// Open for a new plot in `frame` (x, y, width, height), keeping the
    /// fields of the last use.
    pub fn open_new(&mut self, frame: Option<[f64; 4]>) {
        self.open = true;
        self.edit = None;
        self.frame = frame;
        self.error.clear();
    }

    /// Open on the plot `id` with its kept parameters.
    pub fn open_edit(&mut self, app_result: &Value, id: u64) {
        if let Some(spec) = app_result.get("spec").and_then(|v| PlotSpec::from_value(v).ok()) {
            self.load(&spec);
            self.open = true;
            self.edit = Some(id);
            self.frame = None;
        }
    }

    /// A range field: empty fits the data; an expression such as `2*pi` works.
    fn value(text: &str, name: &str) -> Result<Value, String> {
        let t = text.trim();
        if t.is_empty() {
            return Ok(Value::Null);
        }
        let f = plot::parse(t, &[]).map_err(|e| format!("{name}: {e}"))?;
        let v = f.eval(&[]);
        if v.is_finite() {
            Ok(json!(v))
        } else {
            Err(format!("{name} is not a number"))
        }
    }

    fn whole(text: &str, name: &str) -> Result<u64, String> {
        text.trim()
            .parse()
            .map_err(|_| format!("{name} must be a whole number"))
    }

    /// The parameters of `plot.add` or `plot.update`; an error names the
    /// field to fix.
    pub fn params(&self) -> Result<Value, String> {
        let mut p = json!({
            "mode": self.mode,
            "expr": self.expr,
            "xt": self.xt,
            "yt": self.yt,
            "file": self.file,
            "xcol": Self::whole(&self.xcol, "The x column")?,
            "ycol": Self::whole(&self.ycol, "The y column")?,
            "xmin": Self::value(&self.xmin, "x from")?,
            "xmax": Self::value(&self.xmax, "x to")?,
            "ymin": Self::value(&self.ymin, "y from")?,
            "ymax": Self::value(&self.ymax, "y to")?,
            "tmin": Self::value(&self.tmin, "t from")?,
            "tmax": Self::value(&self.tmax, "t to")?,
            "samples": Self::whole(&self.samples, "Samples")?,
            "axes": self.axes,
            "grid": self.grid,
            "log_x": self.log_x,
            "log_y": self.log_y,
            "xlabel": self.xlabel,
            "ylabel": self.ylabel,
            "latex": self.latex,
        });
        if let (Some(o), Some(id), _) = (p.as_object_mut(), self.edit, ()) {
            o.insert("id".into(), json!(id));
        } else if let (Some(o), Some([x, y, w, h])) = (p.as_object_mut(), self.frame) {
            o.insert("x".into(), json!(x));
            o.insert("y".into(), json!(y));
            o.insert("width".into(), json!(w));
            o.insert("height".into(), json!(h));
        }
        Ok(p)
    }
}

fn field(ui: &mut egui::Ui, label: &str, text: &mut String, width: f32) {
    let r = ui.add(egui::TextEdit::singleline(text).desired_width(width));
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, label));
}

/// The window, once per frame.
pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.plot_dialog.open {
        return;
    }
    let mut d = app.plot_dialog.clone();
    let mut open = true;
    let mut go = false;
    let mut browse = false;
    let title = if d.edit.is_some() { "Edit plot" } else { "Plot curve" };
    egui::Window::new(title)
        .open(&mut open)
        .resizable(false)
        .default_pos(egui::pos2(240.0, 90.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut d.mode, PlotMode::Function, "y = f(x)");
                ui.radio_value(&mut d.mode, PlotMode::Parametric, "x(t), y(t)");
                // Files are read by path: not on the web.
                ui.add_enabled_ui(cfg!(not(target_arch = "wasm32")), |ui| {
                    ui.radio_value(&mut d.mode, PlotMode::Data, "Data file");
                });
            });
            egui::Grid::new("plot_fields").num_columns(2).show(ui, |ui| {
                match d.mode {
                    PlotMode::Function => {
                        ui.label("y =");
                        field(ui, "Plot expression", &mut d.expr, 260.0);
                        ui.end_row();
                        ui.label("x from / to:");
                        ui.horizontal(|ui| {
                            field(ui, "Plot x from", &mut d.xmin, 70.0);
                            field(ui, "Plot x to", &mut d.xmax, 70.0);
                        });
                        ui.end_row();
                    }
                    PlotMode::Parametric => {
                        ui.label("x(t) =");
                        field(ui, "Plot x(t)", &mut d.xt, 260.0);
                        ui.end_row();
                        ui.label("y(t) =");
                        field(ui, "Plot y(t)", &mut d.yt, 260.0);
                        ui.end_row();
                        ui.label("t from / to:");
                        ui.horizontal(|ui| {
                            field(ui, "Plot t from", &mut d.tmin, 70.0);
                            field(ui, "Plot t to", &mut d.tmax, 70.0);
                            ui.weak("(default 0 to 2*pi)");
                        });
                        ui.end_row();
                        ui.label("x from / to:");
                        ui.horizontal(|ui| {
                            field(ui, "Plot x from", &mut d.xmin, 70.0);
                            field(ui, "Plot x to", &mut d.xmax, 70.0);
                            ui.weak("(empty: fit)");
                        });
                        ui.end_row();
                    }
                    PlotMode::Data => {
                        ui.label("File:");
                        ui.horizontal(|ui| {
                            field(ui, "Plot data file", &mut d.file, 200.0);
                            if ui.button("Browse...").clicked() {
                                browse = true;
                            }
                        });
                        ui.end_row();
                        ui.label("Columns x / y:");
                        ui.horizontal(|ui| {
                            field(ui, "Plot x column", &mut d.xcol, 40.0);
                            field(ui, "Plot y column", &mut d.ycol, 40.0);
                            ui.weak("(from 1; x 0: row number)");
                        });
                        ui.end_row();
                        ui.label("x from / to:");
                        ui.horizontal(|ui| {
                            field(ui, "Plot x from", &mut d.xmin, 70.0);
                            field(ui, "Plot x to", &mut d.xmax, 70.0);
                            ui.weak("(empty: fit)");
                        });
                        ui.end_row();
                    }
                }
                ui.label("y from / to:");
                ui.horizontal(|ui| {
                    field(ui, "Plot y from", &mut d.ymin, 70.0);
                    field(ui, "Plot y to", &mut d.ymax, 70.0);
                    ui.weak("(empty: fit)");
                });
                ui.end_row();
                if d.mode != PlotMode::Data {
                    ui.label("Samples:");
                    field(ui, "Plot samples", &mut d.samples, 70.0);
                    ui.end_row();
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.checkbox(&mut d.axes, "Axes with ticks and numbers");
                ui.checkbox(&mut d.grid, "Grid");
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut d.log_x, "Logarithmic x");
                ui.checkbox(&mut d.log_y, "Logarithmic y");
            });
            egui::Grid::new("plot_labels").num_columns(2).show(ui, |ui| {
                ui.label("x label:");
                field(ui, "Plot x label", &mut d.xlabel, 220.0);
                ui.end_row();
                ui.label("y label:");
                field(ui, "Plot y label", &mut d.ylabel, 220.0);
                ui.end_row();
            });
            ui.checkbox(&mut d.latex, "Labels are LaTeX (special text)");
            ui.weak("Line style, depth and font are the current ones in the attribute panel.");
            if !d.error.is_empty() {
                ui.colored_label(ui.visuals().error_fg_color, &d.error);
            }
            ui.horizontal(|ui| {
                if ui.button("Draw plot").clicked() {
                    go = true;
                }
                if ui.button("Cancel").clicked() {
                    d.open = false;
                }
            });
        });
    if !open {
        d.open = false;
    }
    if browse {
        if let Some(path) = app.file_dialogs.pick_file("Data", &["csv", "txt", "dat", "xy", "tsv"]) {
            d.file = path;
        }
    }
    if go {
        d.error.clear();
        match d.params() {
            Ok(p) => {
                let cmd = if d.edit.is_some() { "plot.update" } else { "plot.add" };
                match app.try_run(cmd, &p) {
                    Ok(_) => d.open = false,
                    Err(e) => d.error = e,
                }
            }
            Err(e) => d.error = e,
        }
    }
    app.plot_dialog = d;
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn params_follow_the_fields() {
        let mut d = PlotDialog::default();
        d.xmax = "2*pi".into();
        d.ymin = String::new();
        let p = d.params().unwrap();
        assert!((p["xmax"].as_f64().unwrap() - std::f64::consts::TAU).abs() < 1e-9);
        assert!(p["ymin"].is_null());
        assert_eq!(p["samples"], 400);
        d.samples = "x".into();
        assert!(d.params().is_err());
        d.samples = "10".into();
        d.xmin = "1/0".into();
        assert!(d.params().is_err());
    }
}
