//! Dialog windows: edit object, export, search/replace, document settings,
//! command list, about, unsaved changes.
//
// Ported from xfig src/e_edit.c (edit panel: compound, picture and point
// rows, live apply) (xfig licence, see LICENSE-xfig).

use crate::attributes::{arrow_menu, cap_menu, join_menu, line_menu};
use crate::panels::{color_combo, fill_combo_for, font_combo, limit_fill, plain_fill_color};
use crate::{menus, App};
use rustfig_engine::fig2dev::languages::{language, Kind, LANGUAGES};
use rustfig_engine::Engine;
use rustfig_model::{CapStyle, JoinStyle, Justify, LineKind, ObjectId, ObjectKind, PaperSize, PolyKind, SplineKind};
use serde_json::{json, Value};

/// The position rows of xfig's edit panel for one object (`e_edit.c`):
/// what is edited depends on the kind.
#[derive(Debug, Clone, PartialEq)]
pub enum Geo {
    None,
    /// Box, arc-box, picture: first and opposite corner.
    Corners([[f64; 2]; 2]),
    /// Polyline, polygon, spline: every point (their number is fixed).
    Points(Vec<[f64; 2]>),
    /// Arc: three points.
    Arc([[f64; 2]; 3]),
    /// Ellipse or circle: centre, radii (a circle has one radius, in `x`)
    /// and, for an ellipse, the angle in degrees.
    Ellipse {
        center: [f64; 2],
        radii: [f64; 2],
        circle: bool,
        angle: f64,
    },
    /// Text: origin.
    Origin([f64; 2]),
    /// Compound: top-left and bottom-right corner of its bounds (xfig's
    /// `nwcorner`, `secorner`).
    Compound {
        nw: [f64; 2],
        se: [f64; 2],
    },
}

/// One arrow's rows: type choice, thickness, width, length (xfig shows
/// width and length in 1/80 inch).
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowRows {
    pub on: bool,
    pub choice: i32,
    pub thick: f64,
    pub width: f64,
    pub length: f64,
}

impl ArrowRows {
    fn of(a: &Option<rustfig_model::Arrow>) -> ArrowRows {
        match a {
            Some(a) => ArrowRows {
                on: true,
                choice: rustfig_engine::arrow_choice_of(a.kind, a.style),
                thick: a.thickness,
                width: a.width / rustfig_geom::FIG_PER_STYLE_UNIT,
                length: a.height / rustfig_geom::FIG_PER_STYLE_UNIT,
            },
            None => ArrowRows {
                on: false,
                choice: 1,
                thick: 1.0,
                width: 4.0,
                length: 8.0,
            },
        }
    }

    fn spec(&self) -> Value {
        let (kind, style) = rustfig_engine::arrow_choice(self.choice);
        json!({"kind": kind, "style": style, "thickness": self.thick,
            "width": self.width * rustfig_geom::FIG_PER_STYLE_UNIT,
            "height": self.length * rustfig_geom::FIG_PER_STYLE_UNIT})
    }
}

/// Attribute editor state for one object.
/// A texture handle that can sit in a `Debug` state.
#[derive(Clone)]
pub struct Thumb(pub egui::TextureHandle);

impl std::fmt::Debug for Thumb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Thumb")
    }
}

#[derive(Debug, Clone)]
pub struct EditState {
    pub id: ObjectId,
    pub kind: String,
    /// xfig's panel title, e.g. "POLYLINE: Box".
    pub title: String,
    pub is_text: bool,
    pub is_picture: bool,
    pub is_compound: bool,
    pub has_style: bool,
    pub has_arrows: bool,
    pub is_arcbox: bool,
    pub is_arc: bool,
    pub pie: bool,
    pub line: LineKind,
    pub thickness: i32,
    pub pen: i32,
    pub fill: i32,
    pub area: i32,
    pub depth: i32,
    pub style_val: f64,
    pub cap: CapStyle,
    pub join: JoinStyle,
    pub fwd: ArrowRows,
    pub back: ArrowRows,
    pub radius: i32,
    pub text: String,
    pub font: i32,
    pub postscript: bool,
    pub size: f64,
    pub justify: Justify,
    pub angle: f64,
    pub rigid: bool,
    pub special: bool,
    pub hidden: bool,
    pub comment: String,
    pub geo: Geo,
    /// Show coordinates in Fig units instead of the figure's units.
    pub fig_units: bool,
    /// Picture rows.
    pub picture_file: String,
    pub flipped: bool,
    /// The file name the object has now (the typed name differs until Reread).
    pub picture_saved_file: String,
    /// The picture's file facts and pixels (W10-16).
    pub picture_info: Option<rustfig_engine::cmd::picture::PictureInfo>,
    pub picture_image: Option<std::sync::Arc<rustfig_model::RasterImage>>,
    /// Thumbnail texture, made on the first frame (and after a reread).
    pub picture_thumb: Option<Thumb>,
    /// "Scale by %" entry.
    pub picture_percent: f64,
    /// Compound rows: member count.
    pub members: usize,
    /// The panel is live (W10-18): every change is applied at once as one
    /// undo step (`live`: that step is on the history, made at engine
    /// revision `rev`); the next change replaces it, Cancel undoes it.
    pub live: bool,
    pub rev: u64,
    /// The parameters last applied (or the object's own at the start).
    pub last: Option<Value>,
    /// The last application succeeded (Done keeps the panel open if not).
    pub valid: bool,
    /// Picture files loaded (Reread, Relink, a typed or dropped name): each
    /// is an undo step of its own below the live one, undone by Cancel.
    pub file_steps: u32,
    /// Compound rows: Collapse Depths was pressed; smallest and largest
    /// depth inside; the texts directly in it.
    pub collapse: bool,
    pub min_depth: i32,
    pub max_depth: i32,
    pub texts: Vec<String>,
    /// Point table: X-spline shape factors (one per point), the fewest
    /// points the object may keep, and the paste box.
    pub xspline: bool,
    pub shapes: Vec<f64>,
    pub min_points: usize,
    pub paste: String,
    pub paste_error: String,
}

/// The figure's comments (xfig's "Whole Figure" panel, middle button in
/// Edit mode).
#[derive(Debug, Clone, Default)]
pub struct FigureComments {
    pub text: String,
}

/// xfig's "Edit the behavior of the control point" panel (right button in
/// Edit mode): the shape factor of one spline point.
#[derive(Debug, Clone)]
pub struct SplinePointEdit {
    pub id: ObjectId,
    pub index: usize,
    pub kind: SplineKind,
    pub shapes: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct ExportState {
    pub open: bool,
    pub format: &'static str,
    pub magnification: f64,
    pub margin: f64,
    pub dpi: f64,
    pub white_background: bool,
    /// Keep this export up to date on every save (plan E15).
    pub export_on_save: bool,
    /// Set once from the document when the panel opens, so typing in the
    /// panel is not overwritten.
    pub export_on_save_synced: bool,
    /// Leave out depths that are off (xfig's "active layers only").
    pub active_layers_only: bool,
    /// With active layers only: size the page by the whole figure.
    pub whole_figure_bounds: bool,
    /// Bitmap smoothing (fig2dev's `-S`: 1 none, 2 some, 4 more); RustFig
    /// starts with 4, the closest to the antialiased canvas (xfig: none).
    pub smoothing: Option<u32>,
    /// JPEG quality (fig2dev's `-q`).
    pub jpeg_quality: u32,
}

impl Default for ExportState {
    fn default() -> Self {
        ExportState {
            open: false,
            format: "pdf",
            magnification: 100.0,
            margin: 0.0,
            dpi: 150.0,
            white_background: false,
            export_on_save: false,
            export_on_save_synced: false,
            active_layers_only: false,
            whole_figure_bounds: false,
            smoothing: Some(4),
            jpeg_quality: 75,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SearchState {
    pub open: bool,
    pub find: String,
    pub replace: String,
    pub case_sensitive: bool,
    /// xfig's results list ("[x=.. y=..] text" per match) and its message line.
    pub results: Vec<String>,
    pub message: String,
    /// Ids of the texts found (for "Update settings" and "Replace").
    pub found: Vec<u64>,
    /// "Replace" with an empty string was asked once (xfig asks twice).
    pub empty_warned: bool,
}

#[derive(Default)]
pub struct Dialogs {
    pub edit: Option<EditState>,
    pub figure_comments: Option<FigureComments>,
    pub spline_point: Option<SplinePointEdit>,
    pub export: ExportState,
    pub search: SearchState,
    pub settings_open: bool,
    pub commands_open: bool,
    pub about_open: bool,
    /// The bundled manual (`help.open`).
    pub help: crate::help::HelpState,
    pub messages_open: bool,
    /// Edit › Global settings (RustFig's preferences).
    pub global_open: bool,
    /// The xfig resources window (E1).
    pub resources_open: bool,
    /// A new file name that another file already has: asking before replacing.
    pub rename_confirm: Option<String>,
    /// xfig's query when saving inside an open compound: the command (Save
    /// or Save As) waiting for "Save All", "Save Part" or "Cancel".
    pub save_scope: Option<&'static str>,
    resource_filter: String,
    /// Values being typed into the resources window, by resource.
    resource_edits: std::collections::BTreeMap<&'static str, String>,
    /// The user colour being added: name and colour.
    new_color: (String, [u8; 3]),
    command_filter: String,
}

impl EditState {
    /// Take the picture's file facts and pixels from the object.
    pub fn refresh_picture(&mut self, pic: &rustfig_model::Picture) {
        self.picture_saved_file = pic.file.clone();
        self.picture_info = Some(rustfig_engine::cmd::picture::info(pic));
        self.picture_image = pic.image.clone();
        self.picture_thumb = None;
    }
}

impl Dialogs {
    pub fn open_edit(&mut self, engine: &Engine, id: ObjectId) {
        let Some(o) = engine.doc().get(id) else { return };
        let xy = |p: &rustfig_geom::Point| [p.x, p.y];
        let mut st = EditState {
            id,
            kind: o.type_name().to_string(),
            title: String::new(),
            is_text: false,
            is_picture: false,
            is_compound: false,
            has_style: false,
            has_arrows: false,
            is_arcbox: false,
            is_arc: false,
            pie: false,
            line: LineKind::Solid,
            thickness: 1,
            pen: 0,
            fill: 7,
            area: -1,
            depth: o.depth(),
            style_val: 4.0,
            cap: CapStyle::Butt,
            join: JoinStyle::Miter,
            fwd: ArrowRows::of(&None),
            back: ArrowRows::of(&None),
            radius: 7,
            text: String::new(),
            font: 0,
            postscript: true,
            size: 12.0,
            justify: Justify::Left,
            angle: 0.0,
            rigid: false,
            special: false,
            hidden: false,
            comment: o.comments.join("\n"),
            geo: Geo::None,
            fig_units: false,
            picture_file: String::new(),
            flipped: false,
            picture_saved_file: String::new(),
            picture_info: None,
            picture_image: None,
            picture_thumb: None,
            picture_percent: 100.0,
            members: 0,
            live: false,
            rev: 0,
            last: None,
            valid: true,
            file_steps: 0,
            collapse: false,
            min_depth: 0,
            max_depth: 0,
            texts: Vec::new(),
            xspline: false,
            shapes: Vec::new(),
            min_points: 1,
            paste: String::new(),
            paste_error: String::new(),
        };
        if let Some(s) = o.style() {
            st.has_style = true;
            st.line = s.line;
            st.thickness = s.thickness;
            st.pen = s.pen_color.0;
            st.fill = s.fill_color.0;
            st.area = s.area_fill;
            st.style_val = s.style_val;
            st.cap = s.cap;
            st.join = s.join;
        }
        let mut o2 = o.clone();
        if let Some((f, b)) = o2.arrows_mut() {
            st.has_arrows = true;
            st.fwd = ArrowRows::of(f);
            st.back = ArrowRows::of(b);
        }
        st.title = match &o.kind {
            ObjectKind::Text(t) => {
                st.is_text = true;
                st.text = t.text.clone();
                st.font = t.font;
                st.postscript = t.is_postscript();
                st.size = t.size;
                st.justify = t.justify;
                st.angle = t.angle.to_degrees();
                st.pen = t.color.0;
                st.rigid = t.flags & rustfig_model::text_flags::RIGID != 0;
                st.special = t.is_special();
                st.hidden = t.is_hidden();
                st.geo = Geo::Origin(xy(&t.origin));
                "TEXT".to_string()
            }
            ObjectKind::Polyline(p) => {
                if p.kind.is_rect() {
                    if let (Some(a), Some(b)) = (p.points.first(), p.points.get(2)) {
                        st.geo = Geo::Corners([xy(a), xy(b)]);
                    }
                } else {
                    st.geo = Geo::Points(p.points.iter().map(xy).collect());
                    st.min_points = if p.kind == PolyKind::Polygon { 3 } else { 1 };
                }
                if p.kind == PolyKind::ArcBox {
                    st.is_arcbox = true;
                    st.radius = p.radius;
                }
                if p.kind == PolyKind::Picture {
                    st.is_picture = true;
                    if let Some(pic) = &p.picture {
                        st.picture_file = pic.file.clone();
                        st.flipped = pic.flipped;
                        st.refresh_picture(pic);
                    }
                }
                let sub = match p.kind {
                    PolyKind::Polyline => "Polyline",
                    PolyKind::Box => "Box",
                    PolyKind::Polygon => "Polygon",
                    PolyKind::ArcBox => "ArcBox",
                    PolyKind::Picture => "Picture Object",
                };
                format!("POLYLINE: {sub}")
            }
            ObjectKind::Ellipse(e) => {
                st.geo = Geo::Ellipse {
                    center: xy(&e.center),
                    radii: xy(&e.radii),
                    circle: e.kind.is_circle(),
                    angle: e.angle.to_degrees(),
                };
                use rustfig_model::EllipseKind as K;
                match e.kind {
                    K::EllipseRadii => "ELLIPSE: specified by radii",
                    K::EllipseDiameter => "ELLIPSE: specified by diameters",
                    K::CircleRadius => "CIRCLE: specified by radius",
                    K::CircleDiameter => "CIRCLE: specified by diameter",
                }
                .to_string()
            }
            ObjectKind::Arc(a) => {
                st.is_arc = true;
                st.pie = a.pie;
                st.geo = Geo::Arc([xy(&a.points[0]), xy(&a.points[1]), xy(&a.points[2])]);
                "ARC: Specified by 3 points".to_string()
            }
            ObjectKind::Spline(s) => {
                st.geo = Geo::Points(s.points.iter().map(xy).collect());
                st.min_points = if s.closed { 3 } else { 2 };
                st.xspline = s.kind == SplineKind::X;
                st.shapes = (0..s.points.len())
                    .map(|i| s.shapes.get(i).copied().unwrap_or_else(|| s.kind.default_shape()))
                    .collect();
                let name = match (s.kind, s.closed) {
                    (SplineKind::Approximated, false) => "Open approximated spline",
                    (SplineKind::Approximated, true) => "Closed approximated spline",
                    (SplineKind::Interpolated, false) => "Open interpolated spline",
                    (SplineKind::Interpolated, true) => "Closed interpolated spline",
                    (SplineKind::X, false) => "X-Spline open",
                    (SplineKind::X, true) => "X-Spline closed",
                };
                format!("SPLINE: {name}")
            }
            ObjectKind::Compound(c) => {
                st.is_compound = true;
                st.members = c.objects.len();
                let mut depths = Vec::new();
                leaf_depths(&o.kind, &mut depths);
                st.min_depth = depths.iter().copied().min().unwrap_or(0);
                st.max_depth = depths.iter().copied().max().unwrap_or(0);
                st.texts = c
                    .objects
                    .iter()
                    .filter_map(|m| match &m.kind {
                        ObjectKind::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect();
                if let Some(b) = engine.object_fig_bounds(id).filter(|b| !b.is_empty()) {
                    st.geo = Geo::Compound {
                        nw: [b.min.x, b.min.y],
                        se: [b.max.x, b.max.y],
                    };
                }
                "COMPOUND".to_string()
            }
        };
        // xfig: arrows only on open splines/polylines/arcs, none on a
        // single-point line; closed approximated/interpolated splines and
        // polygons/boxes have none.
        if let ObjectKind::Spline(s) = &o.kind {
            if s.closed && s.kind != SplineKind::X {
                st.has_arrows = false;
            }
        }
        st.last = Some(Self::edit_params(&st));
        self.edit = Some(st);
    }

    /// The edit panel's parameters for `object.set_attributes`.
    pub fn edit_params(st: &EditState) -> Value {
        let mut p = json!({
            "ids": [st.id.0],
            "comment": st.comment,
        });
        // A compound's depths change only with Collapse Depths.
        if !st.is_compound || st.collapse {
            p["depth"] = json!(st.depth);
        }
        if st.is_compound && !st.texts.is_empty() {
            p["texts"] = json!(st.texts);
        }
        if st.is_text {
            p["text"] = json!(st.text);
            p["font"] = json!(st.font);
            p["postscript"] = json!(st.postscript);
            p["font_size"] = json!(st.size);
            p["justify"] = json!(match st.justify {
                Justify::Left => "left",
                Justify::Center => "center",
                Justify::Right => "right",
            });
            p["angle"] = json!(st.angle);
            p["pen_color"] = json!(st.pen);
            p["rigid"] = json!(st.rigid);
            p["special"] = json!(st.special);
            p["hidden"] = json!(st.hidden);
        }
        if st.has_style && !st.is_picture {
            p["line"] = json!(st.line.name());
            p["thickness"] = json!(st.thickness);
            p["style_val"] = json!(st.style_val);
            p["pen_color"] = json!(st.pen);
            p["fill_color"] = json!(st.fill);
            p["area_fill"] = json!(limit_fill(st.area, st.fill));
            p["cap"] = json!(format!("{:?}", st.cap).to_lowercase());
            p["join"] = json!(format!("{:?}", st.join).to_lowercase());
        }
        if st.is_picture {
            p["flipped"] = json!(st.flipped);
            if st.picture_info.as_ref().is_some_and(|i| i.kind == "XBM") {
                p["pen_color"] = json!(st.pen);
            }
        }
        if st.has_arrows {
            p["forward_arrow"] = json!(st.fwd.on);
            p["backward_arrow"] = json!(st.back.on);
            p["forward_arrow_spec"] = st.fwd.spec();
            p["backward_arrow_spec"] = st.back.spec();
        }
        if st.is_arcbox {
            p["radius"] = json!(st.radius);
        }
        if st.is_arc {
            p["pie"] = json!(st.pie);
        }
        match &st.geo {
            Geo::None => {}
            Geo::Corners(c) => p["geometry"] = json!({"points": c}),
            Geo::Points(v) => {
                let mut g = json!({"points": v, "resize": true});
                if st.xspline {
                    g["shapes"] = json!(st.shapes);
                }
                p["geometry"] = g;
            }
            Geo::Compound { nw, se } => p["geometry"] = json!({"points": [nw, se]}),
            Geo::Arc(a) => p["geometry"] = json!({"points": a}),
            Geo::Ellipse {
                center,
                radii,
                circle,
                angle,
            } => {
                let mut g = json!({"points": [center], "radii": [radii[0], if *circle { radii[0] } else { radii[1] }]});
                if !*circle {
                    g["angle"] = json!(angle);
                }
                p["geometry"] = g;
            }
            Geo::Origin(o) => p["geometry"] = json!({"points": [o]}),
        }
        p
    }
}

/// Depths of the leaves of an object (a compound's depths, recursively).
fn leaf_depths(kind: &ObjectKind, out: &mut Vec<i32>) {
    if let ObjectKind::Compound(c) = kind {
        for m in &c.objects {
            match &m.kind {
                ObjectKind::Compound(_) => leaf_depths(&m.kind, out),
                _ => out.push(m.depth()),
            }
        }
    }
}

/// Points typed or pasted as text: pairs of numbers separated by spaces,
/// commas, semicolons or brackets, one point per pair (W10-19). The values
/// are in the panel's units; `scale` Fig units each.
pub fn parse_points(text: &str, scale: f64) -> Result<Vec<[f64; 2]>, String> {
    let mut nums = Vec::new();
    for tok in text.split(|c: char| c.is_whitespace() || ",;()[]".contains(c)) {
        if tok.is_empty() {
            continue;
        }
        let v: f64 = tok.parse().map_err(|_| format!("\"{tok}\" is not a number"))?;
        if !v.is_finite() || (v * scale).abs() > 1.0e7 {
            return Err(format!("{tok} is out of range"));
        }
        nums.push(v * scale);
        if nums.len() > 2 * MAX_TABLE_POINTS {
            return Err("too many points".to_string());
        }
    }
    if nums.is_empty() {
        return Err("no points".to_string());
    }
    if nums.len() % 2 != 0 {
        return Err("a point needs an x and a y".to_string());
    }
    Ok(nums.as_chunks::<2>().0.to_vec())
}

/// Most points the table takes (a pasted list longer than this is refused).
pub const MAX_TABLE_POINTS: usize = 100_000;

/// What a line's one style value v means (1/80 inch; the Fig format keeps
/// one number per object, fig2dev derives the rest, `genps.c` `set_style`).
pub const LINE_STYLE_VALUE_HELP: &str = "A Fig file keeps one number per line, in 1/80 inch; the rest of the pattern follows from it:\n\
    Dashed: dash v, gap v\n\
    Dotted: dot, gap v\n\
    Dash-dotted: dash v, gap v/2, dot, gap v/2\n\
    Dash-double-dotted: dash v, gaps 0.45v, 0.33v, 0.45v between the dots\n\
    Dash-triple-dotted: dash v, gaps 0.4v, 0.3v, 0.3v, 0.4v between the dots\n\
    The attribute panel's Line Style keeps two defaults: the dash length for the dashed styles, the dot gap for Dotted.";

/// Extensions offered when picking a figure.
pub const FIGURE_EXTS: &[&str] = &["fig", "xfig", "rustfig"];
/// Extensions offered when picking a picture: xfig's picture types
/// (`f_picobj.c`: EPS/PS, PDF, GIF, PCX, PPM, TIFF, XBM, JPEG, PNG, XPM),
/// BMP, and compressed files (`.gz`, `.Z`), which fig2dev's reader unpacks.
pub const PICTURE_EXTS: &[&str] = &[
    "eps", "epsi", "epsf", "ps", "pdf", "png", "jpg", "jpeg", "gif", "pcx", "ppm", "pgm", "pbm", "pnm", "tif", "tiff",
    "xbm", "xpm", "bmp", "gz", "Z",
];

/// File pickers. The app uses [`NativeDialogs`]; tests substitute a script.
pub trait FileDialogs {
    /// Choose an existing file (`None` when cancelled or unavailable).
    fn pick_file(&self, what: &str, exts: &[&str]) -> Option<String>;
    /// Choose where to save.
    fn save_file(&self, default_name: &str, what: &str, exts: &[&str]) -> Option<String>;
}

/// The operating system's file dialogs (none on the web yet).
pub struct NativeDialogs;

impl FileDialogs for NativeDialogs {
    fn pick_file(&self, what: &str, exts: &[&str]) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            rfd::FileDialog::new()
                .add_filter(what, exts)
                .pick_file()
                .and_then(|p| p.to_str().map(str::to_string))
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (what, exts);
            None
        }
    }

    fn save_file(&self, default_name: &str, what: &str, exts: &[&str]) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            rfd::FileDialog::new()
                .set_file_name(default_name)
                .add_filter(what, exts)
                .save_file()
                .and_then(|p| p.to_str().map(str::to_string))
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (default_name, what, exts);
            None
        }
    }
}

/// Dialogs that answer from a script, in order (for tests).
#[derive(Default)]
pub struct ScriptedDialogs {
    answers: std::cell::RefCell<std::collections::VecDeque<Option<String>>>,
}

impl ScriptedDialogs {
    pub fn new(answers: impl IntoIterator<Item = Option<String>>) -> Self {
        ScriptedDialogs {
            answers: std::cell::RefCell::new(answers.into_iter().collect()),
        }
    }

    fn next(&self) -> Option<String> {
        self.answers.borrow_mut().pop_front().flatten()
    }
}

impl FileDialogs for ScriptedDialogs {
    fn pick_file(&self, _what: &str, _exts: &[&str]) -> Option<String> {
        self.next()
    }
    fn save_file(&self, _default_name: &str, _what: &str, _exts: &[&str]) -> Option<String> {
        self.next()
    }
}

/// xfig's Escape: close the panels (Cancel), when no text field has the
/// keyboard (there Escape just leaves the field). Runs before the shortcuts,
/// so an open panel takes Escape from Edit › Select none.
pub(crate) fn escape_closes_panels(app: &mut App, ctx: &egui::Context) {
    if ctx.wants_keyboard_input() || app.engine.is_typing() || app.engine.tool_busy() {
        return;
    }
    let d = &mut app.dialogs;
    let any = d.export.open
        || d.search.open
        || d.settings_open
        || d.commands_open
        || d.about_open
        || d.help.open
        || d.global_open
        || d.edit.is_some()
        || d.figure_comments.is_some()
        || d.spline_point.is_some()
        || d.messages_open
        || app.unit_panel_open;
    if any && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        // The edit popup's Escape is Cancel (`e_edit.c`): the object is
        // restored, also after Apply.
        cancel_edit(app);
        let d = &mut app.dialogs;
        d.figure_comments = None;
        d.spline_point = None;
        d.export.open = false;
        d.search.open = false;
        d.settings_open = false;
        d.commands_open = false;
        d.about_open = false;
        d.help.open = false;
        d.global_open = false;
        d.messages_open = false;
        app.unit_panel_open = false;
    }
}

/// Drawings a stopped RustFig left unsaved (A6): restore or discard each.
fn recovery_dialog(app: &mut App, ctx: &egui::Context) {
    if app.recoverable.is_empty() {
        return;
    }
    let mut open = true;
    let mut action: Option<(usize, bool)> = None;
    egui::Window::new("Recover unsaved drawings")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label("RustFig stopped without saving these drawings. Restore one to keep working on it, then save it.");
            ui.add_space(4.0);
            egui::Grid::new("recoverable").num_columns(3).show(ui, |ui| {
                for (i, (_, original)) in app.recoverable.iter().enumerate() {
                    let name = original
                        .as_deref()
                        .and_then(|o| std::path::Path::new(o).file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("Untitled");
                    ui.label(name);
                    if ui.button("Restore").clicked() {
                        action = Some((i, true));
                    }
                    if ui.button("Discard").clicked() {
                        action = Some((i, false));
                    }
                    ui.end_row();
                }
            });
        });
    if let Some((i, restore)) = action {
        if let Some((id, _)) = app.recoverable.get(i).cloned() {
            if restore && app.engine.is_modified() {
                app.error("Save or close the current drawing before restoring another");
                return;
            }
            let ok = !restore || app.run("file.recover", json!({ "id": id })).is_some();
            if ok {
                // The restored drawing is this instance's now (its own
                // copy follows); the old files go.
                app.run("file.discard_recovery", json!({ "id": id }));
                app.recoverable.remove(i);
            }
        }
    }
    if !open {
        // Later: the files stay and are offered again at the next start.
        app.recoverable.clear();
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    recovery_dialog(app, ctx);
    figure_comments_dialog(app, ctx);
    spline_point_dialog(app, ctx);
    resources_dialog(app, ctx);
    edit_dialog(app, ctx);
    export_dialog(app, ctx);
    search_dialog(app, ctx);
    settings_dialog(app, ctx);
    commands_dialog(app, ctx);
    about_dialog(app, ctx);
    crate::help::help_dialog(app, ctx);
    messages_dialog(app, ctx);
    global_dialog(app, ctx);
    unsaved_dialog(app, ctx);
    rename_dialog(app, ctx);
    save_scope_dialog(app, ctx);
}

/// Rename the figure's file (`file.rename`): the saved file moves on disk,
/// an unsaved figure just gets the name the next Save uses; with unsaved
/// changes in a saved figure it is saved under the new name. A name another
/// file has asks before replacing it.
pub fn rename_file(app: &mut App, name: &str, overwrite: bool) {
    let save = app.engine.is_modified() && app.engine.path().is_some();
    let params = json!({"name": name, "overwrite": overwrite, "save": save});
    match app.engine.execute("file.rename", &params) {
        Ok(_) => {
            let s = app.engine.status().to_string();
            if !s.is_empty() {
                app.info(s);
                app.engine.set_status("");
            }
        }
        Err(e) if e.code == "exists" => app.dialogs.rename_confirm = Some(name.to_string()),
        Err(e) => app.error(e.message),
    }
}

fn rename_dialog(app: &mut App, ctx: &egui::Context) {
    let Some(name) = app.dialogs.rename_confirm.clone() else {
        return;
    };
    let mut choice = None;
    egui::Modal::new(egui::Id::new("rename_exists")).show(ctx, |ui| {
        ui.heading("Replace file?");
        ui.label(format!("A file named {name:?} already exists. Replace it?"));
        ui.horizontal(|ui| {
            if ui.button("Replace").clicked() {
                choice = Some(true);
            }
            if ui.button("Cancel").clicked() {
                choice = Some(false);
            }
        });
    });
    match choice {
        Some(true) => {
            app.dialogs.rename_confirm = None;
            rename_file(app, &name, true);
        }
        Some(false) => app.dialogs.rename_confirm = None,
        None => {}
    }
}

/// xfig's query when saving with a compound open (`do_save`, `w_file.c`;
/// `popup_query` with QUERY_ALLPARTCAN, `w_util.c`): save only the visible
/// part or all of the figure. "Save All" closes the compounds first.
fn save_scope_dialog(app: &mut App, ctx: &egui::Context) {
    let Some(id) = app.dialogs.save_scope else { return };
    let mut choice = None;
    egui::Modal::new(egui::Id::new("save_scope")).show(ctx, |ui| {
        ui.heading("Xfig: Query");
        ui.label("You have opened a compound. You may save just\nthe visible part or all of the figure.");
        ui.horizontal(|ui| {
            if ui.button("Save All").clicked() {
                choice = Some(Some(false));
            }
            if ui.button("Save Part").clicked() {
                choice = Some(Some(true));
            }
            if ui.button("Cancel").clicked() {
                choice = Some(None);
            }
        });
    });
    let Some(choice) = choice else { return };
    app.dialogs.save_scope = None;
    let then = app.after_save.take();
    let Some(part) = choice else { return };
    if !part {
        // xfig: `close_all_compounds` before saving.
        if let Err(e) = app.engine.finish_text() {
            app.error(e.message);
        }
        app.run("compound.close_all", json!({}));
    }
    app.run(id, json!({ "part": part }));
    // As xfig's `query_save`: quit (or open, new) once the figure is saved.
    if let Some(action) = then {
        if !app.engine.is_modified() {
            app.do_pending(action);
        }
    }
}

/// xfig's Global settings panel holds its preferences; RustFig's is the
/// keyboard choice.
fn global_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.global_open {
        return;
    }
    let mut open = true;
    egui::Window::new("Global settings")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            for (id, label, on) in [
                ("app.toggle_xfig_keys", "xfig keyboard shortcuts", app.xfig_keys),
                // xfig's Global Settings panel repeats the View menu's switches.
                (
                    "view.toggle_page_border",
                    "Show page borders",
                    app.engine.show_page_border,
                ),
                ("app.toggle_depths", "Show depth manager", app.show_depths),
                ("app.toggle_balloons", "Show info balloons", app.show_balloons),
            ] {
                let mut v = on;
                if ui.checkbox(&mut v, label).changed() {
                    app.run(id, serde_json::json!({}));
                }
            }
            ui.horizontal(|ui| {
                ui.label("Balloon delay (ms):");
                let mut ms = app
                    .engine
                    .prefs
                    .xfig
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("balloon_delay"))
                    .and_then(|(_, v)| v.as_integer())
                    .unwrap_or(500);
                if ui.add(egui::DragValue::new(&mut ms).range(0..=100_000)).changed() {
                    app.run(
                        "prefs.set",
                        serde_json::json!({ "resource": "balloon_delay", "value": ms.to_string() }),
                    );
                }
            });
            ui.horizontal(|ui| {
                ui.label("Button style:");
                for (style, label) in [("3d", "3D"), ("flat", "Flat")] {
                    if ui.radio(app.button_3d == (style == "3d"), label).clicked() {
                        app.run("app.button_style", serde_json::json!({ "style": style }));
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Units and page…").clicked() {
                    app.run("app.document_settings", serde_json::json!({}));
                }
                if ui
                    .button("xfig resources…")
                    .on_hover_text("All of xfig's settings (its X resources and command-line options)")
                    .clicked()
                {
                    app.dialogs.resources_open = true;
                }
            });
            if ui
                .button("Use current attributes as defaults")
                .on_hover_text(
                    "The attribute panel's values become the start values (startfontsize, startlinewidth, …)",
                )
                .clicked()
            {
                app.run("prefs.use_current_attributes", serde_json::json!({}));
            }
            ui.separator();
            user_colors_editor(app, ui);
            ui.separator();
            ui.label(
                egui::RichText::new(format!("Saved in {}", app.settings.location()))
                    .small()
                    .weak(),
            );
        });
    app.dialogs.global_open &= open;
}

/// The colours kept for every drawing (E1): offered in every colour menu.
fn user_colors_editor(app: &mut App, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("Your colours").strong());
    let colors = app.engine.prefs.rustfig.user_colors.clone();
    if colors.is_empty() {
        ui.label(egui::RichText::new("None yet: add colours to have them in every drawing.").weak());
    }
    let mut remove = None;
    egui::Grid::new("user_colors").num_columns(3).show(ui, |ui| {
        for c in &colors {
            let rgb = rustfig_model::Rgb::from_hex(&c.rgb).unwrap_or(rustfig_model::Rgb::BLACK);
            let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 14.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(r, 2.0, egui::Color32::from_rgb(rgb.r, rgb.g, rgb.b));
            ui.label(format!("{}  {}", c.name, c.rgb));
            if ui.small_button("Remove").clicked() {
                remove = Some(c.name.clone());
            }
            ui.end_row();
        }
    });
    if let Some(name) = remove {
        app.run("prefs.remove_color", json!({ "name": name }));
    }
    ui.horizontal(|ui| {
        let (name, rgb) = &mut app.dialogs.new_color;
        ui.color_edit_button_srgb(rgb);
        ui.add(egui::TextEdit::singleline(name).hint_text("name").desired_width(120.0));
        let hex = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
        let ready = !name.trim().is_empty();
        if ui.add_enabled(ready, egui::Button::new("Add colour")).clicked() {
            let p = json!({ "name": name.trim(), "rgb": hex });
            app.dialogs.new_color.0.clear();
            app.run("prefs.add_color", p);
        }
    });
}

/// Every xfig resource with its value, default, option and what RustFig
/// does with it; values can be edited (E1).
fn resources_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.resources_open {
        return;
    }
    let mut open = true;
    let mut commit: Option<(&'static str, Option<String>)> = None;
    let values = app.engine.prefs.xfig.clone();
    egui::Window::new("xfig resources")
        .open(&mut open)
        .default_size([620.0, 460.0])
        .show(ctx, |ui| {
            ui.label("xfig's settings under xfig's names. A value set here is kept for every start; xfig's command-line options override it for one run. Empty means xfig's default.");
            ui.add(egui::TextEdit::singleline(&mut app.dialogs.resource_filter).hint_text("filter"));
            let filter = app.dialogs.resource_filter.to_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("resources").striped(true).num_columns(4).show(ui, |ui| {
                    ui.strong("Resource");
                    ui.strong("Value");
                    ui.strong("Option");
                    ui.strong("In RustFig");
                    ui.end_row();
                    for r in rustfig_engine::prefs::RESOURCES {
                        if !filter.is_empty() && !r.name.to_lowercase().contains(&filter) && !r.option.contains(&filter) {
                            continue;
                        }
                        ui.label(r.name);
                        let current = values
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case(r.name))
                            .map(|(_, v)| match v {
                                toml_str if toml_str.is_str() => toml_str.as_str().unwrap_or_default().to_string(),
                                other => other.to_string(),
                            })
                            .unwrap_or_default();
                        let text = app.dialogs.resource_edits.entry(r.name).or_insert_with(|| current.clone());
                        let resp = ui.add(
                            egui::TextEdit::singleline(text)
                                .hint_text(r.default)
                                .desired_width(140.0),
                        );
                        if resp.lost_focus() && *text != current {
                            let t = text.trim().to_string();
                            commit = Some((r.name, (!t.is_empty()).then_some(t)));
                        }
                        ui.label(egui::RichText::new(r.option).small());
                        let status = match r.status {
                            rustfig_engine::prefs::Status::Applied => "applied".to_string(),
                            rustfig_engine::prefs::Status::Planned(t) => format!("kept; in use with {t}"),
                        };
                        ui.label(egui::RichText::new(status).small());
                        ui.end_row();
                    }
                });
            });
        });
    if let Some((name, value)) = commit {
        let v = match value {
            Some(t) => json!(t),
            None => serde_json::Value::Null,
        };
        if app.run("prefs.set", json!({ "resource": name, "value": v })).is_none() {
            // Refused: show the stored value again.
            app.dialogs.resource_edits.remove(name);
        }
    }
    if !open {
        app.dialogs.resources_open = false;
        app.dialogs.resource_edits.clear();
    }
}

impl Dialogs {
    /// The middle button in Edit mode: the whole figure's comments.
    pub fn open_figure_comments(&mut self, engine: &Engine) {
        self.figure_comments = Some(FigureComments {
            text: engine.doc().comments.join("\n"),
        });
    }

    /// The right button in Edit mode on a spline's control point.
    pub fn open_spline_point(&mut self, engine: &Engine, id: ObjectId, index: usize) {
        if let Some(ObjectKind::Spline(s)) = engine.doc().get(id).map(|o| &o.kind) {
            if index < s.points.len() {
                self.spline_point = Some(SplinePointEdit {
                    id,
                    index,
                    kind: s.kind,
                    shapes: s.shapes.clone(),
                });
            }
        }
    }
}

fn figure_comments_dialog(app: &mut App, ctx: &egui::Context) {
    let Some(mut st) = app.dialogs.figure_comments.take() else {
        return;
    };
    let (mut done, mut cancel) = (false, false);
    let mut open = true;
    // xfig: "Whole Figure", Done and Cancel only (no Apply).
    egui::Window::new("Whole Figure")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label("Comments");
            ui.add(
                egui::TextEdit::multiline(&mut st.text)
                    .desired_rows(4)
                    .desired_width(300.0),
            );
            ui.horizontal(|ui| {
                done = ui.button("Done").clicked();
                cancel = ui.button("Cancel").clicked();
            });
        });
    if done {
        app.run("doc.set_comments", json!({"comments": st.text}));
    } else if open && !cancel {
        app.dialogs.figure_comments = Some(st);
    }
}

fn spline_point_dialog(app: &mut App, ctx: &egui::Context) {
    let Some(mut st) = app.dialogs.spline_point.take() else {
        return;
    };
    let (mut done, mut cancel) = (false, false);
    let mut open = true;
    egui::Window::new("Edit spline point")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label("Edit the behavior\nof the control point");
            if let Some(v) = st.shapes.get_mut(st.index) {
                ui.add(egui::Slider::new(v, -1.0..=1.0).text("shape factor"));
                ui.horizontal(|ui| {
                    // xfig's three buttons: approximated, angular, interpolated.
                    if ui.button("Approximated").clicked() {
                        *v = 1.0;
                    }
                    if ui.button("Angular").clicked() {
                        *v = 0.0;
                    }
                    if ui.button("Interpolated").clicked() {
                        *v = -1.0;
                    }
                });
            }
            ui.horizontal(|ui| {
                done = ui.button("Done").clicked();
                cancel = ui.button("Cancel").clicked();
            });
        });
    if done {
        app.run("object.set_geometry", json!({"id": st.id.0, "shapes": st.shapes}));
    } else if open && !cancel {
        app.dialogs.spline_point = Some(st);
    }
}

/// Cancel in xfig's edit panel restores the object as it was, also after
/// Apply (`done_*` with `CANCEL` in `e_edit.c`): undo the Applies.
pub(crate) fn cancel_edit(app: &mut App) {
    if let Some(mut st) = app.dialogs.edit.take() {
        cancel_all(app, &mut st);
    }
}

/// Undo everything the panel did: the live step, then the picture files.
fn cancel_all(app: &mut App, st: &mut EditState) {
    revert_live(app, st);
    for _ in 0..st.file_steps {
        app.run("edit.undo", json!({}));
    }
    st.file_steps = 0;
}

/// Undo the step the live panel made, if it is still the last one.
fn revert_live(app: &mut App, st: &mut EditState) {
    if st.live && app.engine.revision() == st.rev {
        app.run("edit.undo", json!({}));
    }
    st.live = false;
}

/// Apply the panel's parameters as the one step of this edit: the
/// previous step is undone first, so the history gets one entry however
/// many changes were made.
fn live_apply(app: &mut App, st: &mut EditState, params: Value) {
    revert_live(app, st);
    let before = app.engine.revision();
    st.valid = app.run("object.set_attributes", params.clone()).is_some();
    st.last = Some(params);
    // A step on the history exists only if the document changed.
    st.live = st.valid && app.engine.revision() != before;
    st.rev = app.engine.revision();
}

/// The point table (W10-19): one row per point with its coordinates (and
/// the X-spline shape factor), insert and delete per row, add at the end,
/// and a box to paste a list of points.
fn point_table(ui: &mut egui::Ui, st: &mut EditState, scale: f64) {
    let EditState {
        geo,
        shapes,
        xspline,
        min_points,
        paste,
        paste_error,
        ..
    } = st;
    let Geo::Points(pts) = geo else { return };
    ui.label("Points");
    ui.vertical(|ui| {
        #[derive(Clone, Copy)]
        enum Act {
            Insert(usize),
            Delete(usize),
        }
        let mut act = None;
        let n = pts.len();
        egui::ScrollArea::vertical()
            .id_salt("edit_points")
            .max_height(200.0)
            .auto_shrink([true, true])
            .show_rows(ui, 24.0, n, |ui, range| {
                for i in range {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{i:>3}")).monospace());
                        if let Some(p) = pts.get_mut(i) {
                            ui.label("X =");
                            coord(ui, &mut p[0], scale);
                            ui.label("Y =");
                            coord(ui, &mut p[1], scale);
                        }
                        if *xspline {
                            if let Some(v) = shapes.get_mut(i) {
                                ui.label("Shape");
                                ui.add(egui::DragValue::new(v).range(-1.0..=1.0).speed(0.05).max_decimals(2))
                                    .on_hover_text("X-spline shape factor: -1 interpolated, 0 angular, 1 approximated");
                            }
                        }
                        if ui
                            .small_button("Insert")
                            .on_hover_text("Insert a point before this one")
                            .clicked()
                        {
                            act = Some(Act::Insert(i));
                        }
                        let can_delete = n > *min_points;
                        if ui
                            .add_enabled(can_delete, egui::Button::new("Delete").small())
                            .on_hover_text("Delete this point")
                            .on_disabled_hover_text("The object needs these points")
                            .clicked()
                        {
                            act = Some(Act::Delete(i));
                        }
                    });
                }
            });
        let room = n < MAX_TABLE_POINTS;
        let mut add = false;
        ui.horizontal(|ui| {
            add = ui
                .add_enabled(room, egui::Button::new("Add point"))
                .on_hover_text("Add a point after the last one")
                .clicked();
        });
        let new_shape = 1.0;
        match act {
            Some(Act::Insert(i)) if room => {
                let at = pts.get(i).copied().unwrap_or([0.0, 0.0]);
                let p = match i.checked_sub(1).and_then(|j| pts.get(j)) {
                    Some(q) => [(q[0] + at[0]) / 2.0, (q[1] + at[1]) / 2.0],
                    None => [at[0] - 100.0, at[1] - 100.0],
                };
                pts.insert(i, p);
                shapes.insert(i.min(shapes.len()), new_shape);
            }
            Some(Act::Delete(i)) if i < pts.len() && pts.len() > *min_points => {
                pts.remove(i);
                if i < shapes.len() {
                    shapes.remove(i);
                }
            }
            _ => {}
        }
        if add {
            let last = pts.last().copied().unwrap_or([0.0, 0.0]);
            pts.push([last[0] + 100.0, last[1] + 100.0]);
            shapes.push(new_shape);
        }
        ui.collapsing("Paste points", |ui| {
            ui.label("One point per pair of numbers: x y, x, y or (x, y), in the panel's units.");
            ui.add(
                egui::TextEdit::multiline(paste)
                    .desired_rows(3)
                    .desired_width(260.0)
                    .hint_text("0 0\n600 300\n1200 0"),
            )
            .widget_info(|| egui::WidgetInfo::text_edit(true, "Pasted points", paste.as_str(), ""));
            ui.horizontal(|ui| {
                let replace = ui.button("Replace points").clicked();
                let append = ui.button("Append points").clicked();
                if replace || append {
                    match parse_points(paste, scale) {
                        Ok(v) if replace && v.len() < *min_points => {
                            *paste_error = format!("the object needs at least {} points", min_points);
                        }
                        Ok(v) if pts.len() * usize::from(append) + v.len() > MAX_TABLE_POINTS => {
                            *paste_error = "too many points".to_string();
                        }
                        Ok(v) => {
                            if replace {
                                pts.clear();
                                shapes.clear();
                            }
                            shapes.extend(std::iter::repeat_n(new_shape, v.len()));
                            pts.extend(v);
                            paste_error.clear();
                            paste.clear();
                        }
                        Err(e) => *paste_error = e,
                    }
                }
            });
            if !paste_error.is_empty() {
                ui.colored_label(ui.visuals().error_fg_color, paste_error.as_str());
            }
        });
    });
    ui.end_row();
}

/// A coordinate field: shown in the panel's units (`scale` Fig units each).
fn coord(ui: &mut egui::Ui, v: &mut f64, scale: f64) {
    let mut d = *v / scale;
    let step = if scale > 100.0 { 0.01 } else { 1.0 };
    if ui
        .add(egui::DragValue::new(&mut d).speed(step).max_decimals(4))
        .changed()
    {
        *v = d * scale;
    }
}

fn xy_row(ui: &mut egui::Ui, label: &str, p: &mut [f64; 2], scale: f64) {
    ui.label(label);
    ui.horizontal(|ui| {
        ui.label("X =");
        coord(ui, &mut p[0], scale);
        ui.label("Y =");
        coord(ui, &mut p[1], scale);
    });
    ui.end_row();
}

fn arrow_rows(ui: &mut egui::Ui, id: &str, label: &str, a: &mut ArrowRows) {
    ui.label(label);
    ui.horizontal(|ui| {
        ui.checkbox(&mut a.on, "on");
        if let Some(n) = arrow_menu(ui, id, a.choice) {
            a.choice = n;
        }
        ui.label("Thick");
        ui.add(
            egui::DragValue::new(&mut a.thick)
                .range(0.0..=10000.0)
                .speed(0.1)
                .max_decimals(2),
        );
        ui.label("Width");
        ui.add(
            egui::DragValue::new(&mut a.width)
                .range(0.0..=10000.0)
                .speed(0.1)
                .max_decimals(2),
        );
        ui.label("Length");
        ui.add(
            egui::DragValue::new(&mut a.length)
                .range(0.0..=10000.0)
                .speed(0.1)
                .max_decimals(2),
        );
    });
    ui.end_row();
}

/// xfig's arrow type menu: type 0, then types 1 to 14 hollow and filled.
pub(crate) fn arrow_choice_label(n: i32) -> String {
    let (kind, style) = rustfig_engine::arrow_choice(n);
    if kind == 0 {
        "type 0".to_string()
    } else {
        format!("type {kind} {}", if style == 0 { "hollow" } else { "filled" })
    }
}

fn edit_dialog(app: &mut App, ctx: &egui::Context) {
    let Some(mut st) = app.dialogs.edit.take() else { return };
    let mut open = true;
    let mut close = false;
    let mut cancel = false;
    let doc_unit = app.engine.doc().unit_name();
    let doc_scale = {
        let u = app.engine.doc().to_units(1.0);
        if u.is_finite() && u > 0.0 {
            1.0 / u
        } else {
            1200.0
        }
    };
    let scale = if st.fig_units { 1.0 } else { doc_scale };
    if st.is_picture {
        // W10-20: a file dropped on the window becomes the picture's file.
        let dropped = ctx.input(|i| i.raw.dropped_files.first().and_then(|f| f.path.clone()));
        if let Some(path) = dropped.and_then(|p| p.to_str().map(str::to_string)) {
            st.picture_file = path;
            picture_set_file(app, &mut st);
        }
    }
    egui::Window::new(&st.title)
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            egui::Grid::new("edit_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    if st.is_arc {
                        ui.label("Arc type");
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut st.pie, false, "Open");
                            ui.radio_value(&mut st.pie, true, "Pie wedge");
                        });
                        ui.end_row();
                    }
                    if st.is_text {
                        ui.label("Size");
                        ui.add(egui::DragValue::new(&mut st.size).range(1.0..=500.0).suffix(" pt"));
                        ui.end_row();
                        ui.label("Pen colour");
                        if let Some(c) = color_combo(ui, "edit_tcolor", app.engine.doc(), st.pen) {
                            st.pen = c;
                        }
                        ui.end_row();
                        ui.label("Angle");
                        ui.add(egui::DragValue::new(&mut st.angle).range(-360.0..=360.0).suffix("°"));
                        ui.end_row();
                        ui.label("Justification");
                        ui.horizontal(|ui| {
                            for (j, n) in [
                                (Justify::Left, "Left justified"),
                                (Justify::Center, "Centered"),
                                (Justify::Right, "Right justified"),
                            ] {
                                ui.radio_value(&mut st.justify, j, n);
                            }
                        });
                        ui.end_row();
                        ui.label("Flags");
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut st.hidden, "Hidden");
                            ui.checkbox(&mut st.rigid, "Rigid");
                            ui.checkbox(&mut st.special, "TeX");
                        });
                        ui.end_row();
                        ui.label("Font");
                        if let Some((f, ps)) = font_combo(ui, "edit_font", st.font, st.postscript) {
                            st.font = f;
                            st.postscript = ps;
                        }
                        ui.end_row();
                        ui.label("Text");
                        ui.add(
                            egui::TextEdit::multiline(&mut st.text)
                                .desired_rows(3)
                                .desired_width(240.0),
                        );
                        ui.end_row();
                    }
                    if st.has_style && !st.is_picture {
                        ui.label("Width");
                        ui.add(egui::DragValue::new(&mut st.thickness).range(0..=1000));
                        ui.end_row();
                        ui.label("Line style");
                        ui.horizontal(|ui| {
                            if let Some(k) = line_menu(ui, "edit_line", st.line) {
                                st.line = k;
                            }
                            if st.line != LineKind::Solid {
                                // xfig's edit panel names the one value by the style
                                // (`e_edit.c`); its meaning follows fig2dev's `set_style`.
                                let dotted = st.line == LineKind::Dotted;
                                ui.label(if dotted { "Dot gap" } else { "Dash length" })
                                    .on_hover_text(LINE_STYLE_VALUE_HELP);
                                ui.add(egui::DragValue::new(&mut st.style_val).range(0.0..=1000.0).speed(0.1))
                                    .on_hover_text(LINE_STYLE_VALUE_HELP);
                            }
                        });
                        ui.end_row();
                        ui.label("Pen colour");
                        if let Some(c) = color_combo(ui, "edit_pen", app.engine.doc(), st.pen) {
                            st.pen = c;
                        }
                        ui.end_row();
                        ui.label("Fill colour");
                        if let Some(c) = color_combo(ui, "edit_fill", app.engine.doc(), st.fill) {
                            st.fill = c;
                        }
                        ui.end_row();
                        ui.label("Fill style");
                        // xfig offers no tints for black, white and default.
                        st.area = limit_fill(st.area, st.fill);
                        if let Some(f) = fill_combo_for(ui, "edit_area", st.area, plain_fill_color(st.fill)) {
                            st.area = f;
                        }
                        ui.end_row();
                        // xfig: cap style for arcs, open splines and polylines;
                        // join style for polylines, polygons and boxes.
                        let cap_ok = st.is_arc || st.kind == "polyline" || (st.kind == "spline" && st.has_arrows);
                        let join_ok = matches!(st.kind.as_str(), "polyline" | "polygon" | "box");
                        if cap_ok || join_ok {
                            ui.label(match (cap_ok, join_ok) {
                                (true, true) => "Cap / join style",
                                (true, false) => "Cap style",
                                _ => "Join style",
                            });
                            ui.horizontal(|ui| {
                                if cap_ok {
                                    if let Some(c) = cap_menu(ui, "edit_cap", st.cap) {
                                        st.cap = c;
                                    }
                                }
                                if join_ok {
                                    if let Some(j) = join_menu(ui, "edit_join", st.join) {
                                        st.join = j;
                                    }
                                }
                            });
                            ui.end_row();
                        }
                    }
                    if st.has_arrows && !(st.is_arc && st.pie) {
                        arrow_rows(ui, "edit_farrow", "Forward arrow", &mut st.fwd);
                        arrow_rows(ui, "edit_barrow", "Backward arrow", &mut st.back);
                    }
                    if st.is_arcbox {
                        ui.label("Corner radius");
                        ui.add(egui::DragValue::new(&mut st.radius).range(0..=1000));
                        ui.end_row();
                    }
                    if st.is_picture {
                        picture_rows(ui, ctx, app, &mut st, scale);
                    }
                    if st.is_compound {
                        // xfig: the smallest and largest depth inside, and a
                        // button that gives every object the smallest.
                        ui.label("Depths");
                        ui.horizontal(|ui| {
                            ui.label(format!("Minimum: {}", st.min_depth));
                            ui.label(format!("Maximum: {}", st.max_depth));
                            if ui.button("Collapse Depths").clicked() {
                                st.collapse = true;
                                st.depth = st.min_depth;
                                st.max_depth = st.min_depth;
                            }
                        });
                        ui.end_row();
                        ui.label("Number of objects");
                        ui.label(st.members.to_string());
                        ui.end_row();
                        if !st.texts.is_empty() {
                            ui.label("Text objects in this compound");
                            ui.end_row();
                        }
                        for (i, t) in st.texts.iter_mut().enumerate() {
                            ui.label("Text");
                            ui.add(egui::TextEdit::singleline(t).desired_width(240.0))
                                .widget_info(|| {
                                    egui::WidgetInfo::text_edit(
                                        true,
                                        format!("Compound text {}", i + 1),
                                        t.as_str(),
                                        "",
                                    )
                                });
                            ui.end_row();
                        }
                    } else {
                        ui.label("Depth");
                        ui.add(egui::DragValue::new(&mut st.depth).range(0..=999));
                        ui.end_row();
                    }
                    if !matches!(st.geo, Geo::None) {
                        ui.label("Units");
                        egui::ComboBox::from_id_salt("edit_units")
                            .selected_text(if st.fig_units { "Fig units" } else { doc_unit })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut st.fig_units, false, doc_unit);
                                ui.selectable_value(&mut st.fig_units, true, "Fig units");
                            });
                        ui.end_row();
                    }
                    if matches!(st.geo, Geo::Points(_)) {
                        point_table(ui, &mut st, scale);
                    }
                    match &mut st.geo {
                        Geo::None => {}
                        Geo::Corners(c) => {
                            let [a, b] = c;
                            xy_row(ui, "First corner", a, scale);
                            xy_row(ui, "Opposite corner", b, scale);
                        }
                        Geo::Points(_) => {}
                        Geo::Arc(a) => {
                            for (i, p) in a.iter_mut().enumerate() {
                                xy_row(ui, ["First point", "Second point", "Third point"][i], p, scale);
                            }
                        }
                        Geo::Ellipse {
                            center,
                            radii,
                            circle,
                            angle,
                        } => {
                            if !*circle {
                                ui.label("Angle (degrees)");
                                ui.add(egui::DragValue::new(angle).range(-360.0..=360.0).speed(1.0));
                                ui.end_row();
                            }
                            xy_row(ui, "Center", center, scale);
                            if *circle {
                                ui.label("Radius");
                                coord(ui, &mut radii[0], scale);
                                ui.end_row();
                            } else {
                                xy_row(ui, "Radii", radii, scale);
                            }
                        }
                        Geo::Origin(o) => xy_row(ui, "Origin", o, scale),
                        Geo::Compound { nw, se } => {
                            // xfig: Return in a corner repositions (the size
                            // stays), in the dimensions it rescales about the
                            // top-left corner; changes apply as they are made.
                            let (nw0, se0) = (*nw, *se);
                            let mut size = [se0[0] - nw0[0], se0[1] - nw0[1]];
                            let size0 = size;
                            xy_row(ui, "Top left corner", nw, scale);
                            xy_row(ui, "Bottom right corner", se, scale);
                            ui.label("Dimensions");
                            ui.horizontal(|ui| {
                                ui.label("Width =");
                                coord(ui, &mut size[0], scale);
                                ui.label("Height =");
                                coord(ui, &mut size[1], scale);
                            });
                            ui.end_row();
                            if *nw != nw0 {
                                *se = [se0[0] + nw[0] - nw0[0], se0[1] + nw[1] - nw0[1]];
                            } else if *se != se0 {
                                *nw = [nw0[0] + se[0] - se0[0], nw0[1] + se[1] - se0[1]];
                            } else if size != size0 {
                                *se = [nw[0] + size[0], nw[1] + size[1]];
                            }
                        }
                    }
                    ui.label("Comments");
                    ui.add(
                        egui::TextEdit::multiline(&mut st.comment)
                            .desired_rows(2)
                            .desired_width(240.0),
                    );
                    ui.end_row();
                });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Done").clicked() {
                    close = true;
                }
                // The panel is live: Apply has nothing left to apply, it is
                // here as in xfig.
                ui.button("Apply").on_hover_text(
                    "Changes are applied as you make them; Done keeps them as one undo step, Cancel undoes them.",
                );
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
    // xfig's edit popup (`e_edit.c`): Return applies, Ctrl+Return is Done.
    // A single-line field gives up the focus on Return; a multi-line one
    // keeps it, and Return there is a new line.
    let (enter, command) = ctx.input(|i| {
        i.events
            .iter()
            .find_map(|e| match e {
                egui::Event::Key {
                    key: egui::Key::Enter,
                    pressed: true,
                    modifiers,
                    ..
                } => Some((true, modifiers.command)),
                _ => None,
            })
            .unwrap_or((false, false))
    });
    if enter && ctx.memory(|m| m.focused()).is_none() && !app.engine.is_typing() {
        close |= command;
    }
    if !open {
        cancel = true;
    }
    if cancel {
        // The window was taken out of the state: restore the object.
        cancel_all(app, &mut st);
        return;
    }
    // A typed picture file name is loaded on Done (Reread loads it at once).
    if close && st.is_picture && st.picture_file != st.picture_saved_file {
        picture_set_file(app, &mut st);
    }
    // Live panel (W10-18): a change is applied at once, replacing the
    // step the previous change made, so Done is one undo step.
    let params = Dialogs::edit_params(&st);
    if st.last.as_ref() != Some(&params) {
        live_apply(app, &mut st, params);
    }
    if close && !st.valid {
        close = false;
    }
    if !close && app.engine.doc().get(st.id).is_some() {
        app.dialogs.edit = Some(st);
    }
}

fn export_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.export.open {
        app.dialogs.export.export_on_save_synced = false;
        return;
    }
    if !app.dialogs.export.export_on_save_synced {
        app.dialogs.export.export_on_save = app.engine.doc().export_on_save.is_some();
        app.dialogs.export.export_on_save_synced = true;
    }
    let mut open = true;
    let mut go = false;
    let some_hidden = !app.engine.hidden_depths.is_empty();
    let st = &mut app.dialogs.export;
    egui::Window::new("Export")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            egui::Grid::new("export_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Language");
                    let current = language(st.format).map_or(st.format, |l| l.description);
                    let combo = egui::ComboBox::from_id_salt("export_language")
                        .selected_text(current)
                        .width(280.0)
                        .show_ui(ui, |ui| {
                            for l in LANGUAGES {
                                let available = rustfig_engine::fig2dev::language_available(l);
                                let item = ui
                                    .add_enabled(available, egui::Button::selectable(st.format == l.id, l.description));
                                let item = match l.task() {
                                    Some(t) if !available => item
                                        .on_disabled_hover_text(format!("Being ported from fig2dev (plan task {t})")),
                                    _ => item,
                                };
                                if item.clicked() {
                                    st.format = l.id;
                                }
                            }
                        });
                    combo
                        .response
                        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, current));
                    ui.end_row();
                    ui.label("Magnification");
                    ui.add(
                        egui::DragValue::new(&mut st.magnification)
                            .range(1.0..=10000.0)
                            .suffix(" %"),
                    );
                    ui.end_row();
                    ui.label("Border");
                    ui.add(
                        egui::DragValue::new(&mut st.margin)
                            .range(0.0..=12000.0)
                            .suffix(" fig units"),
                    );
                    ui.end_row();
                    let bitmap = language(st.format).is_some_and(|l| l.kind == Kind::Bitmap);
                    if bitmap {
                        ui.label("Resolution");
                        ui.add(egui::DragValue::new(&mut st.dpi).range(10.0..=2400.0).suffix(" dpi"));
                        ui.end_row();
                        // xfig's smoothing menu (fig2dev's -S).
                        let choices: [(Option<u32>, &str); 3] = [
                            (Some(1), "No smoothing"),
                            (Some(2), "Some smoothing"),
                            (Some(4), "More smoothing"),
                        ];
                        let current = choices
                            .iter()
                            .find(|(v, _)| *v == st.smoothing)
                            .map_or("No smoothing", |(_, t)| *t);
                        ui.label("Smoothing");
                        let combo = egui::ComboBox::from_id_salt("export_smoothing")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for (v, t) in choices {
                                    ui.selectable_value(&mut st.smoothing, v, t);
                                }
                            });
                        combo
                            .response
                            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, current));
                        ui.end_row();
                    }
                    if st.format == "jpeg" {
                        ui.label("Image quality");
                        ui.add(egui::DragValue::new(&mut st.jpeg_quality).range(1..=100).suffix(" %"));
                        ui.end_row();
                    }
                    ui.label("Background");
                    ui.checkbox(&mut st.white_background, "white (otherwise transparent)");
                    ui.end_row();
                    ui.label("Depths");
                    ui.vertical(|ui| {
                        ui.add_enabled_ui(some_hidden, |ui| {
                            ui.checkbox(&mut st.active_layers_only, "active layers only")
                                .on_disabled_hover_text("Every depth is on");
                            ui.add_enabled_ui(st.active_layers_only, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("bounds:");
                                    ui.radio_value(&mut st.whole_figure_bounds, true, "whole figure");
                                    ui.radio_value(&mut st.whole_figure_bounds, false, "active layers");
                                });
                            });
                        });
                    });
                    ui.end_row();
                    ui.label("On save");
                    ui.checkbox(&mut st.export_on_save, "Also export on every save")
                        .on_hover_text(
                            "Whenever this figure is saved, write the chosen export file again with these options",
                        );
                    ui.end_row();
                });
            ui.separator();
            if ui.button("Export…").clicked() {
                go = true;
            }
        });
    app.dialogs.export.open = open;
    // xfig's Return in the export panel exports (`ExportFile`).
    if open
        && ctx.input(|i| i.key_pressed(egui::Key::Enter))
        && ctx.memory(|m| m.focused()).is_none()
        && !app.engine.is_typing()
    {
        go = true;
    }
    if go {
        let st = app.dialogs.export.clone();
        let stem = app
            .engine
            .path()
            .and_then(|p| std::path::Path::new(p).file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("figure")
            .to_string();
        let ext = language(st.format).map_or(st.format, |l| l.extension);
        if let Some(path) = app.file_dialogs.save_file(&format!("{stem}.{ext}"), ext, &[ext]) {
            let p = export_params(app, &st, &path);
            if app.run("file.export", p.clone()).is_some() {
                if st.export_on_save {
                    app.run("file.export_on_save", p);
                } else if app.engine.doc().export_on_save.is_some() {
                    app.run("file.export_on_save", json!({ "off": true }));
                }
                app.dialogs.export.open = false;
            }
        }
    }
}

/// `file.export` parameters for the export panel's settings.
fn export_params(app: &App, st: &ExportState, path: &str) -> serde_json::Value {
    let mut p = json!({
        "path": path,
        "format": st.format,
        "magnification": st.magnification,
        "margin": st.margin,
        "dpi": st.dpi,
        "active_layers_only": st.active_layers_only && !app.engine.hidden_depths.is_empty(),
        "whole_figure_bounds": st.whole_figure_bounds,
    });
    if st.white_background {
        p["background"] = json!("#ffffff");
    }
    let mut options = serde_json::Map::new();
    if language(st.format).is_some_and(|l| l.kind == Kind::Bitmap) {
        if let Some(s) = st.smoothing {
            options.insert("smoothing".into(), json!(s));
        }
    }
    if st.format == "jpeg" {
        options.insert("quality".into(), json!(st.jpeg_quality));
    }
    if !options.is_empty() {
        p["options"] = Value::Object(options);
    }
    p
}

/// xfig's ExportFile (Shift-Meta-X): export with the panel's current
/// settings to the figure's name with the language's suffix, without the
/// panel; a figure not saved yet opens the panel instead.
pub fn export_now(app: &mut App) {
    let st = app.dialogs.export.clone();
    let ext = language(st.format).map_or(st.format, |l| l.extension);
    let Some(fig) = app.engine.path().map(str::to_string) else {
        app.dialogs.export.open = true;
        return;
    };
    let target = std::path::Path::new(&fig).with_extension(ext);
    let p = export_params(app, &st, &target.to_string_lossy());
    app.run("file.export", p);
}

fn search_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.search.open {
        return;
    }
    let mut open = true;
    // Some(false) = search, Some(true) = replace, and the update button.
    let mut action: Option<bool> = None;
    let mut update = false;
    let st = &mut app.dialogs.search;
    let found = !st.found.is_empty();
    egui::Window::new("Search and Replace")
        .open(&mut open)
        .collapsible(false)
        .default_size([460.0, 320.0])
        .show(ctx, |ui| {
            egui::Grid::new("search_grid").num_columns(2).show(ui, |ui| {
                // xfig: Return in the search field searches, in the
                // replace field replaces (`w_srchrepl.c`).
                let enter =
                    |r: &egui::Response, ui: &egui::Ui| r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.label("Search for");
                let r = ui.text_edit_singleline(&mut st.find);
                if enter(&r, ui) {
                    action = Some(false);
                }
                ui.end_row();
                ui.add_enabled(found, egui::Label::new("Replace with"));
                let r = ui.add_enabled(found, egui::TextEdit::singleline(&mut st.replace));
                if enter(&r, ui) && found {
                    action = Some(true);
                }
                ui.end_row();
            });
            ui.checkbox(&mut st.case_sensitive, "Case sensitive");
            ui.horizontal(|ui| {
                if ui.button("Search").clicked() {
                    action = Some(false);
                }
                if ui.add_enabled(found, egui::Button::new("Replace")).clicked() {
                    action = Some(true);
                }
                if ui
                    .add_enabled(found, egui::Button::new("Update settings"))
                    .on_hover_text("Give the found texts the attribute panel's font, size, colour and flags (xfig: UPDATE settings)")
                    .clicked()
                {
                    update = true;
                }
            });
            ui.label(if st.message.is_empty() {
                "Enter search string and press \"Search\""
            } else {
                st.message.as_str()
            });
            egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                for line in &st.results {
                    ui.monospace(line);
                }
            });
        });
    app.dialogs.search.open = open;
    if update {
        let ids = app.dialogs.search.found.clone();
        if app.run("object.update", json!({ "ids": ids })).is_some() {
            let n = ids.len();
            app.dialogs.search.message = format!("{n} object{} updated", if n == 1 { "" } else { "s" });
            app.dialogs.search.found.clear();
        }
    }
    if let Some(replace) = action {
        let st = app.dialogs.search.clone();
        if replace && st.replace.is_empty() && !st.empty_warned {
            // xfig asks for a second click before removing the found string.
            app.dialogs.search.empty_warned = true;
            app.dialogs.search.message = "Click \"Replace\" again if you want remove \"Search for\" string".to_string();
            return;
        }
        // Replace works on what the last search found: search first.
        let mut p = json!({"find": st.find, "case_sensitive": st.case_sensitive});
        if replace {
            p["replace"] = json!(st.replace);
        }
        if let Some(r) = app.run("edit.search_replace", p) {
            let m = r.get("matches").and_then(Value::as_u64).unwrap_or(0);
            let n = r.get("replaced").and_then(Value::as_u64).unwrap_or(0);
            let d = &mut app.dialogs.search;
            d.empty_warned = false;
            if replace {
                d.message = format!("{n} object{} replaced", if n == 1 { "" } else { "s" });
                d.results.clear();
                d.found.clear();
            } else {
                d.results = r
                    .get("texts")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .map(|t| {
                                let x = t.get("x").and_then(Value::as_f64).unwrap_or(0.0);
                                let y = t.get("y").and_then(Value::as_f64).unwrap_or(0.0);
                                let text = t.get("text").and_then(Value::as_str).unwrap_or("");
                                let (u, per) = if app.engine.doc().settings.units == rustfig_model::Units::Metric {
                                    ("cm", 450.0)
                                } else {
                                    ("in", 1200.0)
                                };
                                format!("[x={:4.1}{u} y={:4.1}{u}] {text}", x / per, y / per)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                d.found = r
                    .get("ids")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(Value::as_u64).collect())
                    .unwrap_or_default();
                d.message = if m == 0 {
                    "No match".to_string()
                } else {
                    format!(
                        "{m} line{} match{}",
                        if m == 1 { "" } else { "s" },
                        if m == 1 { "es" } else { "" }
                    )
                };
            }
        }
    }
}

fn settings_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.settings_open {
        return;
    }
    let mut open = true;
    let s = app.engine.doc().settings.clone();
    let mut cmds: Vec<(&str, Value)> = Vec::new();
    egui::Window::new("Document Settings")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            egui::Grid::new("settings_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Units");
                    ui.horizontal(|ui| {
                        if ui.radio(s.units == rustfig_model::Units::Inches, "Inches").clicked() {
                            cmds.push(("doc.set_units", json!({"units": "inches"})));
                        }
                        if ui.radio(s.units == rustfig_model::Units::Metric, "Metric").clicked() {
                            cmds.push(("doc.set_units", json!({"units": "metric"})));
                        }
                    });
                    ui.end_row();
                    ui.label("Paper");
                    egui::ComboBox::from_id_salt("paper")
                        .selected_text(s.paper.name())
                        .show_ui(ui, |ui| {
                            for p in PaperSize::ALL {
                                if ui.selectable_label(p == s.paper, p.name()).clicked() {
                                    cmds.push(("doc.set_paper", json!({"paper": p.name()})));
                                }
                            }
                        });
                    ui.end_row();
                    ui.label("Orientation");
                    if ui.button(format!("{:?} (toggle)", s.orientation)).clicked() {
                        cmds.push(("doc.toggle_orientation", json!({})));
                    }
                    ui.end_row();
                    ui.label("Magnification");
                    let mut m = s.magnification;
                    if ui
                        .add(egui::DragValue::new(&mut m).range(1.0..=10000.0).suffix(" %"))
                        .changed()
                    {
                        cmds.push(("doc.set_magnification", json!({"magnification": m})));
                    }
                    ui.end_row();
                });
            ui.label(format!("{} user colours", app.engine.doc().colors.len()));
        });
    for (c, p) in cmds {
        app.run(c, p);
    }
    app.dialogs.settings_open = open;
}

fn commands_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.commands_open {
        return;
    }
    let mut open = true;
    let filter = &mut app.dialogs.command_filter;
    egui::Window::new("Commands")
        .open(&mut open)
        .default_size([520.0, 420.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter");
                ui.text_edit_singleline(filter);
            });
            ui.separator();
            let f = filter.to_lowercase();
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("cmd_grid").striped(true).num_columns(3).show(ui, |ui| {
                    for c in rustfig_engine::cmd::all() {
                        if !f.is_empty() && !c.id.contains(&f) && !c.label.to_lowercase().contains(&f) {
                            continue;
                        }
                        ui.monospace(c.id);
                        ui.label(c.label);
                        ui.label(menus::display_shortcut(c.shortcut));
                        ui.end_row();
                    }
                    for c in menus::UI_COMMANDS {
                        if !f.is_empty() && !c.id.contains(&f) && !c.label.to_lowercase().contains(&f) {
                            continue;
                        }
                        ui.monospace(c.id);
                        ui.label(c.label);
                        ui.label(menus::display_shortcut(c.shortcut));
                        ui.end_row();
                    }
                });
            });
        });
    app.dialogs.commands_open = open;
}

fn messages_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.messages_open {
        return;
    }
    let mut open = true;
    let tk = crate::theme::Tokens::for_dark(ctx.style().visuals.dark_mode);
    let mut dismiss = false;
    egui::Window::new("Messages")
        .open(&mut open)
        .default_size([460.0, 260.0])
        .show(ctx, |ui| {
            // xfig's message window has Dismiss and Clear (`w_msgpanel.c`).
            ui.horizontal(|ui| {
                if ui.button("Dismiss").clicked() {
                    dismiss = true;
                }
                if ui.button("Clear").clicked() {
                    app.messages.clear();
                }
            });
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                if app.messages.is_empty() {
                    ui.label("No messages yet.");
                }
                for m in &app.messages {
                    let c = if m.error { tk.error } else { ui.visuals().text_color() };
                    ui.label(egui::RichText::new(&m.text).color(c));
                }
            });
        });
    // xfig's Return dismisses the message panel (`w_msgpanel.c`).
    let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter)) && ctx.memory(|m| m.focused()).is_none();
    app.dialogs.messages_open = open && !dismiss && !(enter && !app.engine.is_typing());
}

fn about_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.about_open {
        return;
    }
    let mut open = true;
    egui::Window::new("About RustFig")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.heading("RustFig");
            ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
            ui.label("A Rust-native reimplementation of xfig, close to the original in look and behaviour.");
            ui.label("Reads and writes Fig 3.2 files; exports PDF, SVG, EPS, PNG and the LaTeX box (more of fig2dev's languages are being ported).");
            ui.label("Licensed MIT OR Apache-2.0; ported parts also under the xfig licence.");
            ui.separator();
            // xfig's About lists the copyright holders (`w_help.c`); ported code keeps them.
            for line in [
                "Based on xfig 3.2.9a:",
                "  Parts Copyright \u{a9} 2016-2024 by Thomas Loimer",
                "  Parts Copyright \u{a9} 1989-2015 by Brian V. Smith",
                "  Parts Copyright \u{a9} 1991 by Paul King",
                "  Copyright \u{a9} 1985-1988 by Supoj Sutanthavibul",
                "  See the xfig licence file and NOTICE for other copyrights",
            ] {
                ui.label(line);
            }
        });
    app.dialogs.about_open = open;
}

fn unsaved_dialog(app: &mut App, ctx: &egui::Context) {
    let Some(action) = app.pending.clone() else { return };
    let mut choice = None;
    egui::Modal::new(egui::Id::new("unsaved")).show(ctx, |ui| {
        ui.heading("Save changes?");
        ui.label("The drawing has unsaved changes.");
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                choice = Some(0);
            }
            if ui.button("Discard").clicked() {
                choice = Some(1);
            }
            if ui.button("Cancel").clicked() {
                choice = Some(2);
            }
        });
    });
    match choice {
        Some(0) => {
            app.pending = None;
            app.run("app.save", json!({}));
            if app.dialogs.save_scope.is_some() {
                // The Save All / Save Part query is up: go on after it.
                app.after_save = Some(action);
            } else if !app.engine.is_modified() {
                app.do_pending(action);
            }
        }
        Some(1) => {
            app.pending = None;
            app.do_pending(action);
        }
        Some(2) => app.pending = None,
        _ => {}
    }
}

// ---- picture panel (W10-16, W10-20) -------------------------------------

/// Load the typed (or dropped, or browsed) file into the picture: undoable
/// by Cancel like the live changes.
fn picture_set_file(app: &mut App, st: &mut EditState) {
    // The live step is taken back first, so the file step lies below it;
    // the next frame applies the panel's values again on top.
    revert_live(app, st);
    st.last = None;
    let p = json!({"id": st.id.0, "path": st.picture_file});
    if app.run("picture.set_file", p).is_some() {
        st.file_steps += 1;
        let pic = match app.engine.doc().get(st.id).map(|o| &o.kind) {
            Some(ObjectKind::Polyline(pl)) => pl.picture.clone(),
            _ => None,
        };
        if let Some(pic) = pic {
            st.refresh_picture(&pic);
        }
    }
}

/// The thumbnail's texture: the image sampled down to at most 128 pixels.
fn picture_thumb(ctx: &egui::Context, st: &mut EditState) -> Option<egui::TextureHandle> {
    if st.picture_thumb.is_none() {
        let img = st.picture_image.as_ref()?;
        let (w, h) = (img.width as usize, img.height as usize);
        if w == 0 || h == 0 || img.rgba.len() < w.checked_mul(h)?.checked_mul(4)? {
            return None;
        }
        let k = (w.max(h) / 128).max(1);
        let (tw, th) = (w / k, h / k);
        let mut px = Vec::with_capacity(tw * th);
        for y in 0..th {
            for x in 0..tw {
                let i = ((y * k) * w + x * k) * 4;
                let c = img.rgba.get(i..i + 4)?;
                px.push(egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]));
            }
        }
        let ci = egui::ColorImage {
            size: [tw, th],
            source_size: egui::vec2(tw as f32, th as f32),
            pixels: px,
        };
        st.picture_thumb = Some(Thumb(ctx.load_texture(
            "picture_thumb",
            ci,
            egui::TextureOptions::LINEAR,
        )));
    }
    st.picture_thumb.as_ref().map(|t| t.0.clone())
}

/// The picture panel's rows (xfig's `e_edit.c` picture fields).
fn picture_rows(ui: &mut egui::Ui, ctx: &egui::Context, app: &mut App, st: &mut EditState, scale: f64) {
    use rustfig_engine::cmd::picture::{resize_corner, rotation_of, with_rotation, Resize};
    let info = st.picture_info.clone();
    ui.label("Picture filename");
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut st.picture_file).desired_width(220.0))
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Picture filename"));
        if ui.button("Reread").clicked() {
            picture_set_file(app, st);
        }
        let missing = info.as_ref().is_some_and(|i| !i.exists);
        if ui.button(if missing { "Relink" } else { "Browse" }).clicked() {
            if let Some(f) = app.file_dialogs.pick_file("Pictures", PICTURE_EXTS) {
                st.picture_file = f;
                picture_set_file(app, st);
            }
        }
    });
    ui.end_row();
    if let Some(i) = &info {
        if !i.exists {
            ui.label("");
            ui.colored_label(egui::Color32::RED, "File not found: use Relink, or drop a file here");
            ui.end_row();
        }
        ui.label("Type");
        ui.label(&i.kind);
        ui.end_row();
        ui.label("Size (pixels)");
        ui.label(format!("{} x {}", i.width, i.height));
        ui.end_row();
        ui.label("Colours");
        ui.label(if i.colours > 256 {
            "more than 256".to_string()
        } else {
            i.colours.to_string()
        });
        ui.end_row();
        ui.label("Transparent colour");
        ui.label(if i.transparent { "yes" } else { "none" });
        ui.end_row();
    }
    if let Some(tex) = picture_thumb(ctx, st) {
        ui.label("Preview");
        let s = tex.size_vec2();
        ui.image((tex.id(), s))
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Picture thumbnail"));
        ui.end_row();
    }
    let (w, h) = info.as_ref().map(|i| (i.width, i.height)).unwrap_or((0, 0));
    if let Geo::Corners([a, b]) = &mut st.geo {
        let (pa, pb) = (
            rustfig_geom::Point::new(a[0], a[1]),
            rustfig_geom::Point::new(b[0], b[1]),
        );
        let mut rot = rotation_of(pa, pb);
        ui.label("Width");
        let mut wd = (b[0] - a[0]).abs();
        let wd0 = wd;
        ui.horizontal(|ui| {
            coord(ui, &mut wd, scale);
            ui.label("Length");
            let mut ln = (b[1] - a[1]).abs();
            let before = ln;
            coord(ui, &mut ln, scale);
            if ln != before && ln > 0.0 {
                b[1] = a[1] + if b[1] < a[1] { -ln } else { ln };
            }
        });
        if wd != wd0 && wd > 0.0 {
            b[0] = a[0] + if b[0] < a[0] { -wd } else { wd };
        }
        ui.end_row();
        ui.label("Rotation");
        let rot0 = rot;
        egui::ComboBox::from_id_salt("edit_pic_rotation")
            .selected_text(format!("{rot}"))
            .show_ui(ui, |ui| {
                for r in [0, 90, 180, 270] {
                    ui.selectable_value(&mut rot, r, format!("{r}"));
                }
            });
        if rot != rot0 {
            if let Some(q) = with_rotation(pa, rustfig_geom::Point::new(b[0], b[1]), rot) {
                *b = [q.x, q.y];
            }
        }
        ui.end_row();
        let orig = if w > 0 { f64::from(h) / f64::from(w) } else { 0.0 };
        let cur = if (b[0] - a[0]).abs() > 0.0 {
            (b[1] - a[1]).abs() / (b[0] - a[0]).abs()
        } else {
            0.0
        };
        ui.label("h/w ratio");
        ui.label(format!("{orig:.1} (original), {cur:.1} (now)"));
        ui.end_row();
        // Two grid rows: a wrapping row inside a grid cell spills into the
        // next row's label.
        let mut mode = None;
        ui.label("Size");
        ui.horizontal(|ui| {
            if ui.button("Shrink to orig").clicked() {
                mode = Some(Resize::Shrink);
            }
            if ui.button("Expand to orig").clicked() {
                mode = Some(Resize::Expand);
            }
            if ui.button("Use original size").clicked() {
                mode = Some(Resize::Original);
            }
        });
        ui.end_row();
        ui.label("");
        ui.horizontal(|ui| {
            if ui.button("Scale by %").clicked() {
                mode = Some(Resize::Percent(st.picture_percent));
            }
            ui.add(
                egui::DragValue::new(&mut st.picture_percent)
                    .range(0.001..=100000.0)
                    .max_decimals(3),
            );
        });
        ui.end_row();
        if let Some(m) = mode {
            let (pa, pb) = (
                rustfig_geom::Point::new(a[0], a[1]),
                rustfig_geom::Point::new(b[0], b[1]),
            );
            if let Some(q) = resize_corner(m, pa, pb, st.flipped, w, h) {
                *b = [q.x, q.y];
            }
        }
    }
    ui.label("Orientation");
    ui.checkbox(&mut st.flipped, "Flipped about diagonal");
    ui.end_row();
    if info.as_ref().is_some_and(|i| i.kind == "XBM") {
        ui.label("XBM pen colour");
        if let Some(c) = color_combo(ui, "edit_xbm_pen", app.engine.doc(), st.pen) {
            st.pen = c;
        }
        ui.end_row();
    }
    ui.label("");
    if ui.button("Edit Image").clicked() {
        app.run("picture.edit_image", json!({"id": st.id.0}));
    }
    ui.end_row();
}

#[cfg(test)]
mod tests {
    #[test]
    fn picture_picker_offers_xfigs_picture_types() {
        for ext in [
            "eps", "ps", "pdf", "gif", "pcx", "ppm", "tif", "xbm", "jpg", "png", "xpm", "gz",
        ] {
            assert!(super::PICTURE_EXTS.contains(&ext), "{ext}");
        }
    }
}
