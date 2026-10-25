//! xfig's Open and Merge panel (W10-12, `w_file.c`): the file list of a
//! directory under a filename mask, the selected figure's preview with its
//! size, number of objects and comments, and for Merge the Figure Offset X/Y
//! with units. The panel is a view: Open runs `app.open`, Merge runs
//! `file.merge` with the offset and its unit. Desktop only (the browser has
//! no folders).
//
// Ported from xfig src/w_file.c (xfig licence, see LICENSE-xfig).

use crate::App;
use egui::Ui;
use rustfig_engine::file_panel::{self, DirListing, FigurePreview, DEFAULT_MASK};
use rustfig_model::Units;
use serde_json::json;
use std::path::{Path, PathBuf};

/// The units of the Figure Offset menu and the `file.merge` names.
const UNITS: [&str; 3] = ["Inches", "cm", "Fig units"];
/// The preview's box in points.
const PREVIEW: f32 = 220.0;

/// What the panel shows of the selected file.
struct Loaded {
    path: PathBuf,
    result: Result<(FigurePreview, Option<egui::TextureHandle>), String>,
}

/// A listing (or why there is none) and the directory, mask and hidden flag
/// it was made for.
type Listed = (Result<DirListing, String>, (String, String, bool));

/// State of the panel.
#[derive(Default)]
pub struct FilePanel {
    pub open: bool,
    /// Merge (with the Figure Offset) rather than Open.
    pub merge: bool,
    /// The current directory ("" until the panel first lists one).
    pub dir: String,
    /// The Current Dir field while it is edited.
    pub dir_edit: String,
    pub mask: String,
    /// The Filename field.
    pub name: String,
    pub show_hidden: bool,
    pub x: f64,
    pub y: f64,
    pub x_unit: usize,
    pub y_unit: usize,
    /// The listing of the directory, with what it was made for.
    listing: Option<Listed>,
    loaded: Option<Loaded>,
}

impl FilePanel {
    /// Open the panel for Open or Merge. A directory chosen before stays.
    pub fn show(&mut self, merge: bool) {
        self.open = true;
        self.merge = merge;
        if self.mask.is_empty() {
            self.mask = DEFAULT_MASK.to_string();
        }
        self.listing = None;
        self.loaded = None;
    }

    /// The selected path: the Filename field under the current directory.
    pub fn path(&self) -> Option<PathBuf> {
        let name = self.name.trim();
        if name.is_empty() {
            return None;
        }
        let p = Path::new(name);
        Some(if p.is_absolute() {
            p.to_path_buf()
        } else {
            Path::new(&self.dir).join(p)
        })
    }

    /// Select `name` in the list: the Filename field and the preview.
    pub fn select(&mut self, ctx: &egui::Context, units: Units, name: &str) {
        self.name = name.to_string();
        self.load_preview(ctx, units);
    }

    fn load_preview(&mut self, ctx: &egui::Context, units: Units) {
        let Some(path) = self.path() else {
            self.loaded = None;
            return;
        };
        let result = path
            .to_str()
            .ok_or_else(|| "the file name is not text".to_string())
            .and_then(|p| file_panel::preview_figure(units, p).map_err(|e| e.message))
            .map(|fig| {
                let tex = fig.render(PREVIEW as u32 * 2, PREVIEW as u32 * 2).map(|(w, h, rgba)| {
                    let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                    ctx.load_texture("file-panel-preview", img, egui::TextureOptions::LINEAR)
                });
                (fig, tex)
            });
        self.loaded = Some(Loaded { path, result });
    }
}

fn home() -> Option<String> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| h.to_string_lossy().into_owned())
}

/// What the buttons did this frame.
#[derive(Default)]
struct Act {
    /// Open or Merge the Filename.
    go: bool,
    cancel: bool,
    rescan: bool,
    /// Go to this directory.
    dir: Option<String>,
    /// A file was clicked (selected) or double-clicked (go).
    pick: Option<String>,
    /// The system file dialog.
    browse: bool,
}

/// Draw the panel when it is open.
pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.file_panel.open {
        return;
    }
    // List the directory when the panel starts or the mask or directory changed.
    {
        let fp = &app.file_panel;
        let key = (fp.dir.clone(), fp.mask.clone(), fp.show_hidden);
        if fp.listing.as_ref().is_none_or(|(_, k)| *k != key) {
            let mut p = json!({"mask": key.1, "hidden": key.2});
            if !key.0.is_empty() {
                p["dir"] = json!(key.0);
            }
            let r = app
                .engine
                .execute("file.list_dir", &p)
                .map_err(|e| e.message)
                .map(|v| {
                    let strs = |k: &str| -> Vec<String> {
                        v.get(k)
                            .and_then(|a| a.as_array())
                            .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
                            .unwrap_or_default()
                    };
                    DirListing {
                        dir: v.get("dir").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                        dirs: strs("dirs"),
                        files: strs("files"),
                    }
                });
            let fp = &mut app.file_panel;
            if let Ok(l) = &r {
                fp.dir = l.dir.clone();
            }
            let key = (fp.dir.clone(), key.1, key.2);
            fp.listing = Some((r, key));
        }
    }
    let units = app.engine.doc().settings.units;
    let modified = app.engine.is_modified();
    let objects = app.engine.doc().objects.len();
    let mut act = Act::default();
    let mut window_open = true;
    let merge = app.file_panel.merge;
    let title = if merge {
        "Xfig: Merge Figure"
    } else {
        "Xfig: Open Figure"
    };
    let mut fp = std::mem::take(&mut app.file_panel);
    egui::Window::new(title)
        .open(&mut window_open)
        .resizable(false)
        .collapsible(false)
        .default_pos(egui::pos2(160.0, 80.0))
        .show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(300.0);
                    left_column(ui, &mut fp, &mut act, modified, objects);
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_width(PREVIEW + 10.0);
                    preview_column(ui, &fp);
                });
            });
        });
    app.file_panel = fp;
    if !window_open || act.cancel {
        app.file_panel.open = false;
        return;
    }
    if act.rescan {
        app.file_panel.listing = None;
    }
    if let Some(d) = act.dir {
        app.file_panel.dir = d;
        app.file_panel.loaded = None;
    }
    if act.browse {
        // The system's own dialog, for files outside the listed folder.
        if let Some(p) = app.file_dialogs.pick_file("Figures", crate::dialogs::FIGURE_EXTS) {
            let p = PathBuf::from(p);
            if let (Some(dir), Some(name)) = (p.parent(), p.file_name()) {
                app.file_panel.dir = dir.to_string_lossy().into_owned();
                app.file_panel.select(ctx, units, &name.to_string_lossy());
            }
        }
    }
    if let Some(name) = act.pick {
        // The preview is of this file; its size is in the open figure's units.
        app.file_panel.select(ctx, units, &name);
    }
    if act.go {
        go(app);
    }
}

/// Open or Merge the Filename (`file_panel_dismiss` and the Open/Merge
/// callbacks): Open asks about unsaved changes like File > Open.
fn go(app: &mut App) {
    let Some(path) = app.file_panel.path().and_then(|p| p.to_str().map(str::to_string)) else {
        app.error("Enter a file name first".to_string());
        return;
    };
    if !Path::new(&path).is_file() {
        app.error(format!("{path} is not a file"));
        return;
    }
    app.file_panel.open = false;
    if app.file_panel.merge {
        // `file.merge` takes one unit for both offsets: each is converted
        // from its own unit to Fig units here.
        let fp = &app.file_panel;
        let to_fig = |v: f64, u: usize| file_panel::offset_in_fig_units(v, u as u8);
        let (xf, yf) = (to_fig(fp.x, fp.x_unit), to_fig(fp.y, fp.y_unit));
        app.run(
            "file.merge",
            json!({"path": path, "offset_x": xf, "offset_y": yf, "unit": "fig"}),
        );
    } else {
        app.run("app.open", json!({ "path": path }));
    }
}

fn left_column(ui: &mut Ui, fp: &mut FilePanel, act: &mut Act, modified: bool, objects: usize) {
    egui::Grid::new("file_panel_top").num_columns(2).show(ui, |ui| {
        ui.label("File Status");
        ui.label(if modified { "Modified" } else { "Not modified" });
        ui.end_row();
        ui.label("# of Objects");
        let n = match &fp.loaded {
            Some(Loaded { result: Ok((f, _)), .. }) => f.objects,
            _ => objects,
        };
        ui.label(n.to_string());
        ui.end_row();
        if fp.merge {
            for (label, id, is_x) in [
                ("Figure Offset X", "file_panel_xoff", true),
                ("Y", "file_panel_yoff", false),
            ] {
                ui.label(label);
                ui.horizontal(|ui| {
                    let (v, u) = if is_x {
                        (&mut fp.x, &mut fp.x_unit)
                    } else {
                        (&mut fp.y, &mut fp.y_unit)
                    };
                    let r = ui.add(egui::DragValue::new(v).speed(0.05));
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::DragValue,
                            true,
                            if is_x { "Figure offset X" } else { "Figure offset Y" },
                        )
                    });
                    egui::ComboBox::from_id_salt(id)
                        .selected_text(UNITS.get(*u).copied().unwrap_or("Inches"))
                        .show_ui(ui, |ui| {
                            for (i, name) in UNITS.iter().enumerate() {
                                ui.selectable_value(u, i, *name);
                            }
                        });
                });
                ui.end_row();
            }
        }
    });
    ui.separator();
    ui.label("Filename");
    let r = ui.add(egui::TextEdit::singleline(&mut fp.name).desired_width(f32::INFINITY));
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Filename"));
    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        act.go = true;
    }
    if r.changed() {
        fp.loaded = None;
    }
    ui.label("Fig files");
    let files: Vec<String> = match &fp.listing {
        Some((Ok(l), _)) => l.files.clone(),
        _ => Vec::new(),
    };
    egui::ScrollArea::vertical()
        .id_salt("file_panel_files")
        .max_height(150.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for f in &files {
                let sel = fp.name == *f;
                let r = ui.selectable_label(sel, f);
                if r.clicked() {
                    act.pick = Some(f.clone());
                }
                if r.double_clicked() {
                    act.go = true;
                }
            }
        });
    ui.horizontal(|ui| {
        ui.label("Filename Mask");
        let r = ui.add(egui::TextEdit::singleline(&mut fp.mask).desired_width(f32::INFINITY));
        r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Filename mask"));
    });
    ui.horizontal(|ui| {
        ui.label("Current Dir");
        let r = ui.add(egui::TextEdit::singleline(&mut fp.dir_edit).desired_width(f32::INFINITY));
        r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Current directory"));
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            // Takes effect on Enter, not on every key.
            act.dir = Some(fp.dir_edit.clone());
        } else if !r.has_focus() {
            fp.dir_edit = fp.dir.clone();
        }
    });
    if let Some((Err(e), _)) = &fp.listing {
        ui.colored_label(egui::Color32::from_rgb(200, 0, 0), e);
    }
    ui.label("Directories");
    let dirs: Vec<String> = match &fp.listing {
        Some((Ok(l), _)) => l.dirs.clone(),
        _ => Vec::new(),
    };
    egui::ScrollArea::vertical()
        .id_salt("file_panel_dirs")
        .max_height(90.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for d in &dirs {
                let r = ui.selectable_label(false, d);
                if r.double_clicked() || r.clicked() {
                    let base = PathBuf::from(&fp.dir);
                    let next = if d == ".." {
                        base.parent().map(Path::to_path_buf).unwrap_or(base)
                    } else {
                        base.join(d)
                    };
                    act.dir = Some(next.to_string_lossy().into_owned());
                }
            }
        });
    ui.horizontal(|ui| {
        if ui.button("Home").clicked() {
            act.dir = home();
        }
        if ui.checkbox(&mut fp.show_hidden, "Show Hidden").changed() {
            act.rescan = true;
        }
        if ui.button("Rescan").clicked() {
            act.rescan = true;
        }
        if ui.button("Browse…").clicked() {
            act.browse = true;
        }
    });
    ui.separator();
    ui.horizontal(|ui| {
        let label = if fp.merge { "Merge" } else { "Open" };
        if ui.button(label).clicked() {
            act.go = true;
        }
        if ui.button("Cancel").clicked() {
            act.cancel = true;
        }
    });
}

fn preview_column(ui: &mut Ui, fp: &FilePanel) {
    ui.horizontal(|ui| {
        ui.strong("Preview");
        if let Some(Loaded { result: Ok((f, _)), .. }) = &fp.loaded {
            ui.label(&f.size);
        }
    });
    ui.label(
        fp.loaded
            .as_ref()
            .and_then(|l| l.path.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(PREVIEW, PREVIEW), egui::Sense::hover());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Figure preview"));
    ui.painter().rect_filled(rect, 0.0, egui::Color32::WHITE);
    ui.painter().rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
    match fp.loaded.as_ref().map(|l| &l.result) {
        Some(Ok((_, Some(tex)))) => {
            // Fit the picture in the box, centred.
            let sz = tex.size_vec2();
            let k = (PREVIEW / sz.x)
                .min(PREVIEW / sz.y)
                .min(1.0 / ui.ctx().pixels_per_point().max(1.0) * 2.0);
            let r = egui::Rect::from_center_size(rect.center(), sz * k);
            ui.painter().image(
                tex.id(),
                r,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        Some(Ok((_, None))) => {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "(empty figure)",
                egui::FontId::proportional(12.0),
                egui::Color32::GRAY,
            );
        }
        Some(Err(e)) => {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "cannot read",
                egui::FontId::proportional(12.0),
                egui::Color32::GRAY,
            );
            ui.colored_label(egui::Color32::from_rgb(200, 0, 0), e);
        }
        None => {}
    }
    ui.strong("Figure comments:");
    let comments = match fp.loaded.as_ref().map(|l| &l.result) {
        Some(Ok((f, _))) => f.comments.clone(),
        _ => String::new(),
    };
    egui::ScrollArea::vertical()
        .id_salt("file_panel_comments")
        .max_height(80.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let r = ui.label(egui::RichText::new(&comments).color(egui::Color32::from_rgb(0, 0, 200)));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, "Figure comments"));
        });
}
