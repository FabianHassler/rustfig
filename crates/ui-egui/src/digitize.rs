//! The "Digitize..." window (plan S4-8): File menu entry in xfig's place
//! (`w_digitize.c`: file name prefix, starting number, suffix, new file or
//! append). RustFig writes the points of the selected polylines and splines
//! through `file.digitize`, optionally in the data units of a calibrated plot.

// Ported from xfig src/w_digitize.c (xfig licence, see LICENSE-xfig).

use crate::App;
use serde_json::{json, Value};

/// Settings of the window; they stay between uses, as xfig's do.
#[derive(Debug, Clone)]
pub struct Digitize {
    pub open: bool,
    pub prefix: String,
    pub start: String,
    pub suffix: String,
    pub append: bool,
    pub calibrate: bool,
    /// Two reference points: figure x, figure y, value x, value y.
    pub points: [[String; 4]; 2],
    pub log_x: bool,
    pub log_y: bool,
}

impl Default for Digitize {
    fn default() -> Self {
        Digitize {
            open: false,
            prefix: "line".to_string(),
            start: "0".to_string(),
            suffix: "xy".to_string(),
            append: false,
            calibrate: false,
            points: Default::default(),
            log_x: false,
            log_y: false,
        }
    }
}

impl Digitize {
    /// xfig's example file name (`%s_%04d.%s`).
    pub fn example(&self) -> String {
        let seq: u64 = self.start.trim().parse().unwrap_or(0);
        format!("{}_{seq:04}.{}", self.prefix, self.suffix.trim_start_matches('.'))
    }

    /// The parameters of `file.digitize`; an error names the field to fix.
    pub fn params(&self, ids: &[u64]) -> Result<Value, String> {
        let start: u64 = self
            .start
            .trim()
            .parse()
            .map_err(|_| "The starting number must be a whole number".to_string())?;
        let mut p = json!({
            "ids": ids,
            "prefix": self.prefix,
            "start": start,
            "suffix": self.suffix,
            "append": self.append,
        });
        if self.calibrate {
            let mut pts = Vec::new();
            for (i, row) in self.points.iter().enumerate() {
                let mut nums = [0.0f64; 4];
                for (n, text) in nums.iter_mut().zip(row) {
                    *n = text
                        .trim()
                        .parse()
                        .map_err(|_| format!("Reference point {} needs four numbers", i + 1))?;
                }
                pts.push(json!({"x": nums[0], "y": nums[1], "vx": nums[2], "vy": nums[3]}));
            }
            p["calibration"] = json!({"points": pts, "log_x": self.log_x, "log_y": self.log_y});
        }
        Ok(p)
    }
}

/// Export with the window's settings. Uses the selection.
pub fn export(app: &mut App) {
    let ids: Vec<u64> = app.engine.selection().iter().map(|i| i.0).collect();
    if ids.is_empty() {
        app.error("Select the polylines or splines to export first");
        return;
    }
    let params = match app.digitize.params(&ids) {
        Ok(p) => p,
        Err(e) => {
            app.error(e);
            return;
        }
    };
    if let Some(r) = app.run("file.digitize", params) {
        if let Some(next) = r["next"].as_u64() {
            if !app.digitize.append {
                app.digitize.start = next.to_string();
            }
        }
    }
}

fn field(ui: &mut egui::Ui, label: &str, text: &mut String, width: f32) {
    let r = ui.add(egui::TextEdit::singleline(text).desired_width(width));
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, label));
}

/// The window, once per frame.
pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.digitize.open {
        return;
    }
    let mut d = app.digitize.clone();
    let mut open = true;
    let mut do_export = false;
    let mut browse = false;
    egui::Window::new("Xfig: Digitize to file(s)")
        .open(&mut open)
        .resizable(false)
        .default_pos(egui::pos2(240.0, 110.0))
        .show(ctx, |ui| {
            crate::dialog_body(ui, |ui| {
                ui.label("Saves the points of the selected polylines and splines.");
                ui.horizontal(|ui| {
                    ui.label("File name prefix:");
                    field(ui, "Digitize prefix", &mut d.prefix, 160.0);
                    if ui.button("Browse...").clicked() {
                        browse = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Starting number:");
                    field(ui, "Digitize starting number", &mut d.start, 60.0);
                    ui.label("Suffix:");
                    field(ui, "Digitize suffix", &mut d.suffix, 50.0);
                });
                ui.horizontal(|ui| {
                    ui.label("Example:");
                    ui.monospace(d.example());
                });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut d.append, false, "New file for each line");
                    ui.radio_value(&mut d.append, true, "Append to one file");
                });
                ui.label("A suffix of csv writes comma separated values with a header; any other, x y per line.");
                ui.separator();
                ui.checkbox(&mut d.calibrate, "Calibrate to data units");
                if d.calibrate {
                    ui.label("Two reference points of the plot: position in the figure and value.");
                    egui::Grid::new("digitize_cal").show(ui, |ui| {
                        ui.label("");
                        for h in ["Figure x", "Figure y", "Value x", "Value y"] {
                            ui.label(h);
                        }
                        ui.end_row();
                        for (i, row) in d.points.iter_mut().enumerate() {
                            ui.label(format!("Point {}", i + 1));
                            for (j, text) in row.iter_mut().enumerate() {
                                field(
                                    ui,
                                    &format!("Digitize point {} {}", i + 1, ["x", "y", "vx", "vy"][j]),
                                    text,
                                    70.0,
                                );
                            }
                            ui.end_row();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut d.log_x, "Logarithmic x");
                        ui.checkbox(&mut d.log_y, "Logarithmic y");
                    });
                } else {
                    ui.label("Without calibration the points are in figure units (1/1200 inch).");
                }
                ui.separator();
                ui.horizontal(|ui| {
                    let native = cfg!(not(target_arch = "wasm32"));
                    let b = ui.add_enabled(native, egui::Button::new("Export"));
                    if b.clicked() {
                        do_export = true;
                    }
                    if ui.button("Done").clicked() {
                        d.open = false;
                    }
                });
            })
        });
    if !open {
        d.open = false;
    }
    if browse {
        if let Some(path) = app.file_dialogs.save_file(&d.example(), "Data", &["xy", "csv", "txt"]) {
            let p = std::path::Path::new(&path);
            if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                d.suffix = ext.to_string();
            }
            // The name is `prefix_NNNN.suffix`: take the number and prefix back.
            let stem = p.with_extension("").to_string_lossy().into_owned();
            match stem.rsplit_once('_') {
                Some((pre, num)) if num.len() == 4 && num.chars().all(|c| c.is_ascii_digit()) => {
                    d.prefix = pre.to_string();
                    d.start = num.trim_start_matches('0').to_string();
                    if d.start.is_empty() {
                        d.start = "0".to_string();
                    }
                }
                _ => d.prefix = stem,
            }
        }
    }
    app.digitize = d;
    if do_export {
        export(app);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn example_follows_xfig() {
        let d = Digitize {
            start: "12".into(),
            ..Digitize::default()
        };
        assert_eq!(d.example(), "line_0012.xy");
    }

    #[test]
    fn params_need_numbers() {
        let mut d = Digitize::default();
        assert!(d.params(&[1]).is_ok());
        d.calibrate = true;
        assert!(d.params(&[1]).is_err());
        d.points = [
            ["0".into(), "0".into(), "0".into(), "0".into()],
            ["10".into(), "10".into(), "1".into(), "2".into()],
        ];
        let p = d.params(&[1]).unwrap();
        assert_eq!(p["calibration"]["points"][1]["vy"], 2.0);
    }
}
