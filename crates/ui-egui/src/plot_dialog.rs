//! The Plot dialog (plan E12): what to draw (a function, a parametric curve
//! or data from a file), its ranges and sampling, axes and labels. "Plot"
//! runs `plot.add` (a new plot where the Plot mode's drag placed it) or
//! `plot.update` (a plot being edited); the dialog only collects fields.

use crate::App;
use rustfig_engine::plot::{self, NumFormat, PlotMode, PlotSpec, TickDir, TickSpec};
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
    pub frame_box: bool,
    pub grid: bool,
    pub log_x: bool,
    pub log_y: bool,
    pub xlabel: String,
    pub ylabel: String,
    pub latex: bool,
    pub latex_numbers: bool,
    pub xticks: TickFields,
    pub yticks: TickFields,
    /// The last error, shown in the dialog.
    pub error: String,
}

/// The tick settings of one axis as typed.
#[derive(Debug, Clone)]
pub struct TickFields {
    pub manual: bool,
    pub first: String,
    pub step: String,
    pub count: String,
    pub minor: String,
    pub length: String,
    pub dir: TickDir,
    pub format: NumFormat,
    pub decimals: String,
    pub ticks: bool,
    pub numbers: bool,
}

impl TickFields {
    fn from_spec(t: &TickSpec) -> TickFields {
        TickFields {
            manual: t.manual,
            first: num(t.first),
            step: num(t.step),
            count: t.count.map(|c| c.to_string()).unwrap_or_default(),
            minor: t.minor.map(|c| c.to_string()).unwrap_or_default(),
            length: num(Some(t.length)),
            dir: t.dir,
            format: t.format,
            decimals: t.decimals.to_string(),
            ticks: t.ticks,
            numbers: t.numbers,
        }
    }

    fn params(&self, axis: &str) -> Result<Value, String> {
        let opt_whole = |t: &str, name: &str| -> Result<Value, String> {
            if t.trim().is_empty() {
                Ok(Value::Null)
            } else {
                PlotDialog::whole(t, &format!("{axis} ticks: {name}")).map(|v| json!(v))
            }
        };
        Ok(json!({
            "manual": self.manual,
            "first": PlotDialog::value(&self.first, &format!("{axis} ticks: first tick"))?,
            "step": PlotDialog::value(&self.step, &format!("{axis} ticks: step"))?,
            "count": opt_whole(&self.count, "count")?,
            "minor": opt_whole(&self.minor, "minor ticks")?,
            "length": PlotDialog::value(&self.length, &format!("{axis} ticks: length"))?
                .as_f64().unwrap_or(105.0),
            "dir": self.dir,
            "format": self.format,
            "decimals": PlotDialog::whole(&self.decimals, &format!("{axis} ticks: decimals"))?,
            "ticks": self.ticks,
            "numbers": self.numbers,
        }))
    }
}

fn tick_section(ui: &mut egui::Ui, axis: &str, t: &mut TickFields) {
    egui::CollapsingHeader::new(format!("{axis} axis tick settings"))
        .id_salt(format!("plot_ticks_{axis}"))
        .show(ui, |ui| {
            egui::Grid::new(format!("plot_tick_grid_{axis}"))
                .num_columns(2)
                .min_col_width(LABEL_W)
                .show(ui, |ui| {
                    ui.label("Placement:");
                    ui.checkbox(&mut t.manual, "Manual (empty fields are automatic)");
                    ui.end_row();
                    ui.label("");
                    ui.add_enabled_ui(t.manual, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("first");
                            field(ui, &format!("{axis} tick first"), &mut t.first, 55.0);
                            ui.label("step");
                            field(ui, &format!("{axis} tick step"), &mut t.step, 55.0);
                            ui.label("or count");
                            field(ui, &format!("{axis} tick count"), &mut t.count, 35.0);
                        });
                    });
                    ui.end_row();
                    ui.label("Minor per major:");
                    ui.horizontal(|ui| {
                        field(ui, &format!("{axis} tick minor"), &mut t.minor, 30.0);
                        ui.label("(0-9)   Length:");
                        field(ui, &format!("{axis} tick length"), &mut t.length, 50.0);
                    });
                    ui.end_row();
                    ui.label("Direction:");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut t.dir, TickDir::In, "in");
                        ui.radio_value(&mut t.dir, TickDir::Out, "out");
                        ui.radio_value(&mut t.dir, TickDir::Both, "both");
                    });
                    ui.end_row();
                    ui.label("Number format:");
                    ui.horizontal_wrapped(|ui| {
                        ui.radio_value(&mut t.format, NumFormat::Auto, "automatic");
                        ui.radio_value(&mut t.format, NumFormat::Fixed, "decimals");
                        field(ui, &format!("{axis} tick decimals"), &mut t.decimals, 25.0);
                        ui.radio_value(&mut t.format, NumFormat::Scientific, "scientific");
                        ui.radio_value(&mut t.format, NumFormat::Pi, "multiples of \u{3c0}");
                    });
                    ui.end_row();
                });
            ui.weak("Log axis: the step counts decades; minor ticks are at 2 to n+1 times a decade.");
        });
}

/// Width of the label column of every grid in the dialog.
const LABEL_W: f32 = 100.0;

/// A bold group heading.
fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).strong());
    ui.separator();
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
            frame_box: true,
            grid: false,
            log_x: false,
            log_y: false,
            xlabel: String::new(),
            ylabel: String::new(),
            latex: false,
            latex_numbers: false,
            xticks: TickFields::from_spec(&TickSpec::default()),
            yticks: TickFields::from_spec(&TickSpec::default()),
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
        self.frame_box = s.frame;
        self.grid = s.grid;
        self.log_x = s.log_x;
        self.log_y = s.log_y;
        self.xlabel = s.xlabel.clone();
        self.ylabel = s.ylabel.clone();
        self.latex = s.latex;
        self.latex_numbers = s.latex_numbers;
        self.xticks = TickFields::from_spec(&s.xticks);
        self.yticks = TickFields::from_spec(&s.yticks);
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
            "frame": self.frame_box,
            "grid": self.grid,
            "log_x": self.log_x,
            "log_y": self.log_y,
            "xlabel": self.xlabel,
            "ylabel": self.ylabel,
            "latex": self.latex,
            "latex_numbers": self.latex_numbers,
            "xticks": self.xticks.params("x")?,
            "yticks": self.yticks.params("y")?,
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

/// A checkbox without text, found by its accessible label.
fn labelled_check(ui: &mut egui::Ui, on: &mut bool, label: &str) {
    let r = ui.checkbox(on, "");
    r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *on, label));
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
            crate::dialog_body(ui, |ui| {
                heading(ui, "Function");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut d.mode, PlotMode::Function, "y = f(x)");
                    ui.radio_value(&mut d.mode, PlotMode::Parametric, "x(t), y(t)");
                    // Files are read by path: not on the web.
                    ui.add_enabled_ui(cfg!(not(target_arch = "wasm32")), |ui| {
                        ui.radio_value(&mut d.mode, PlotMode::Data, "Data file");
                    });
                });
                egui::Grid::new("plot_function")
                    .num_columns(2)
                    .min_col_width(LABEL_W)
                    .show(ui, |ui| {
                        match d.mode {
                            PlotMode::Function => {
                                ui.label("y =");
                                field(ui, "Plot expression", &mut d.expr, 260.0);
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
                            }
                            PlotMode::Data => {
                                ui.label("File:");
                                ui.horizontal(|ui| {
                                    field(ui, "Plot data file", &mut d.file, 190.0);
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
                            }
                        }
                        if d.mode != PlotMode::Data {
                            ui.label("Samples:");
                            field(ui, "Plot samples", &mut d.samples, 70.0);
                            ui.end_row();
                        }
                    });

                heading(ui, "Range");
                egui::Grid::new("plot_range")
                    .num_columns(3)
                    .min_col_width(LABEL_W)
                    .show(ui, |ui| {
                        ui.label("x from / to:");
                        ui.horizontal(|ui| {
                            field(ui, "Plot x from", &mut d.xmin, 70.0);
                            field(ui, "Plot x to", &mut d.xmax, 70.0);
                        });
                        ui.checkbox(&mut d.log_x, "Logarithmic");
                        ui.end_row();
                        ui.label("y from / to:");
                        ui.horizontal(|ui| {
                            field(ui, "Plot y from", &mut d.ymin, 70.0);
                            field(ui, "Plot y to", &mut d.ymax, 70.0);
                        });
                        ui.checkbox(&mut d.log_y, "Logarithmic");
                        ui.end_row();
                    });
                ui.weak("Empty fields fit the data (a function: x from -10 to 10).");

                heading(ui, "Axes");
                egui::Grid::new("plot_axes")
                    .num_columns(3)
                    .min_col_width(LABEL_W)
                    .show(ui, |ui| {
                        ui.label("");
                        ui.label("Ticks");
                        ui.label("Numbers");
                        ui.end_row();
                        ui.label("x axis");
                        labelled_check(ui, &mut d.xticks.ticks, "Plot x ticks");
                        labelled_check(ui, &mut d.xticks.numbers, "Plot x numbers");
                        ui.end_row();
                        ui.label("y axis");
                        labelled_check(ui, &mut d.yticks.ticks, "Plot y ticks");
                        labelled_check(ui, &mut d.yticks.numbers, "Plot y numbers");
                        ui.end_row();
                    });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut d.frame_box, "Box");
                    ui.checkbox(&mut d.grid, "Grid");
                });
                tick_section(ui, "x", &mut d.xticks);
                tick_section(ui, "y", &mut d.yticks);

                heading(ui, "Labels");
                egui::Grid::new("plot_labels")
                    .num_columns(2)
                    .min_col_width(LABEL_W)
                    .show(ui, |ui| {
                        ui.label("x label:");
                        field(ui, "Plot x label", &mut d.xlabel, 260.0);
                        ui.end_row();
                        ui.label("y label:");
                        field(ui, "Plot y label", &mut d.ylabel, 260.0);
                        ui.end_row();
                        ui.label("LaTeX:");
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut d.latex, "labels");
                            ui.checkbox(&mut d.latex_numbers, "numbers");
                        });
                        ui.end_row();
                    });
                ui.add_space(4.0);
                ui.weak("Line style, depth and font are the current ones in the attribute panel.");
                if !d.error.is_empty() {
                    ui.colored_label(ui.visuals().error_fg_color, &d.error);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Draw plot").clicked() {
                        go = true;
                    }
                    if ui.button("Cancel").clicked() {
                        d.open = false;
                    }
                });
            })
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
