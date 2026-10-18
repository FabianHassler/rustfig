//! The languages RustFig's own writers produce from the scene: EPS, PostScript
//! pages (`ps`, and PDF in page mode), PDF, SVG, the LaTeX box,
//! the graphics halves of `pstex` and `pdftex`, PNG and fig2dev's bitmap
//! languages (`genbitmaps.c`, without ghostscript).
//!
//! Options are taken as fig2dev's drivers take them (`genps_option`,
//! `genbitmaps_option`, `gensvg_option`, `genbox_option`), with the table
//! in [`crate::options`]. The objects arrive from [`crate::convert`]
//! deepest first and become scene primitives ([`crate::scene::build`]);
//! the trailer writes the scene on the page fig2dev's bounds give.
//
// Ported from fig2dev dev/genps.c (`genps_option`, `-f`, `PSFONTMAG`),
// dev/genbitmaps.c (`genbitmaps_option`, size and border), dev/genpstex.c
// (special texts left to `pstex_t`) and fig2dev.c (`-b`, `-g`, `-N`) (xfig
// licence, see LICENSE-xfig).

use crate::gendev::strtol;
use crate::languages::{language, Language};
use crate::object::{Arc, Compound, Ellipse, Line, Spline, Text};
use crate::options::{option_spec, parse_options, Arg, DriverOptions, OptionValue, Support};
use crate::scene::build::{file_scale, transparent_color, Builder, TextOptions};
use crate::scene::grid::{grid_prims, GridSpec};
use crate::scene::output::write_with_notes;
use crate::scene::page::paper;
use crate::scene::preview::{PreviewKind, UserBox};
use crate::scene::{ExportFormat, Page, PageInput, PageLayout, Rgba};
use crate::{Ctx, Driver, Out};
use figbase::color::x_color;
use figbase::geom::{Point, Rect};

/// The languages this driver writes, with their format.
const LANGUAGES: &[(&str, ExportFormat)] = &[
    ("box", ExportFormat::LatexBox),
    ("eps", ExportFormat::Eps),
    ("gif", ExportFormat::Gif),
    ("jpeg", ExportFormat::Jpeg),
    ("pcx", ExportFormat::Pcx),
    ("pdf", ExportFormat::Pdf),
    // The graphics halves of the combined LaTeX languages (genpstex.c):
    // EPS and PDF without special texts.
    ("pdftex", ExportFormat::Pdf),
    ("png", ExportFormat::Png),
    ("ppm", ExportFormat::Ppm),
    // PostScript pages (genps with the page layout).
    ("ps", ExportFormat::Eps),
    ("pstex", ExportFormat::Eps),
    ("svg", ExportFormat::Svg),
    ("tiff", ExportFormat::Tiff),
    ("xbm", ExportFormat::Xbm),
    ("xpm", ExportFormat::Xpm),
];

/// genbitmaps' languages: unknown options are warnings there.
const BITMAP_LANGUAGES: &[&str] = &["gif", "jpeg", "pcx", "png", "ppm", "sld", "tiff", "xbm", "xpm"];

/// The driver for `lang`, if RustFig's writers produce it.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    let &(lang, format) = LANGUAGES.iter().find(|(l, _)| *l == lang)?;
    Some(Box::new(SceneDriver {
        lang,
        format,
        items: Vec::new(),
        options: None,
        builder: None,
        comments: Vec::new(),
        // genps: `ps` makes pages; `eps` and `pdf` start as EPS (`-P` or
        // `-z` make a PDF's pages).
        epsflag: lang != "ps",
        xoff: 0,
        yoff: 0,
        pdf_minor: 5,
        grid: None,
    }))
}

/// Whether `lang` is one of this driver's languages.
pub fn writes(lang: &str) -> bool {
    LANGUAGES.iter().any(|(l, _)| *l == lang)
}

/// The name the driver uses in its messages (`Err_badarg`): genps speaks
/// for ps, eps and pdf.
pub fn driver_name(lang: &str) -> &str {
    match lang {
        "eps" | "pdf" | "pstex" | "pdftex" => "ps",
        other => other,
    }
}

/// The language whose options a language takes: pstex takes geneps's
/// options, pdftex genpdf's (genpstex.c).
pub fn options_lang(lang: &str) -> &str {
    match lang {
        "pstex" => "eps",
        "pdftex" => "pdf",
        other => other,
    }
}

/// An option's value as given.
#[derive(Debug, Clone)]
enum Value {
    Bool(bool),
    Text(String),
}

/// One of the scene languages.
#[derive(Debug)]
pub struct SceneDriver {
    lang: &'static str,
    format: ExportFormat,
    /// The options given, by key, in order.
    items: Vec<(String, Value)>,
    /// The options read when the figure starts.
    options: Option<DriverOptions>,
    builder: Option<Builder>,
    /// The whole-figure comments (the LaTeX box writes them).
    comments: Vec<String>,
    /// genps' `epsflag`: the page options are ignored while it is set.
    epsflag: bool,
    xoff: i32,
    yoff: i32,
    /// The PDF version 1.n (`-Y`).
    pdf_minor: u8,
    /// `-G`: the major and minor spacings in file units (`grid` call).
    grid: Option<(f32, f32)>,
}

impl SceneDriver {
    /// Whether the PostScript languages (not the bitmaps) are written.
    fn is_ps_family(&self) -> bool {
        matches!(self.lang, "ps" | "eps" | "pdf" | "pstex" | "pdftex")
    }

    /// `gen_ps_eps_option` and `genpdf_option`'s page options; true if
    /// `c` is one of them.
    fn page_option(&mut self, ctx: &mut Ctx, c: char, arg: Option<&str>) -> bool {
        if !self.is_ps_family() {
            return false;
        }
        let pdf = matches!(self.lang, "pdf" | "pdftex");
        let int = || arg.map_or(0, |a| strtol(a).0);
        match c {
            // -P and -z make PDF pages instead of an EPS figure.
            'P' | 'z' if pdf => {
                self.epsflag = false;
                if c == 'z' {
                    ctx.papersize = arg.unwrap_or("").to_string();
                    ctx.paperspec = true;
                }
            }
            'P' => {}
            'Y' => {
                if pdf {
                    let v = int();
                    self.pdf_minor = if (1..=7).contains(&v) { v as u8 } else { 5 };
                }
            }
            'c' | 'e' | 'l' | 'p' | 'M' | 'o' | 'O' | 'x' | 'y' | 'z' => {
                if !self.epsflag {
                    match c {
                        'c' | 'e' => {
                            ctx.center = c == 'c';
                            ctx.centerspec = true;
                        }
                        'l' | 'p' => {
                            ctx.landscape = c == 'l';
                            ctx.orientspec = true;
                        }
                        'M' | 'o' => {
                            ctx.multi_page = c == 'M';
                            ctx.multispec = true;
                        }
                        'O' => ctx.overlap = true,
                        'x' => self.xoff = int(),
                        'y' => self.yoff = int(),
                        _ => {
                            ctx.papersize = arg.unwrap_or("").to_string();
                            ctx.paperspec = true;
                        }
                    }
                }
            }
            _ => return false,
        }
        true
    }

    fn language(&self) -> Result<&'static Language, String> {
        language(match options_lang(self.lang) {
            "ps" => "eps",
            l => l,
        })
        .ok_or_else(|| format!("fig2dev: no writer for {}", self.lang))
    }

    /// The options given so far, with fig2dev's defaults first: font sizes
    /// in 1/80 inch unless `-F` says points, and no smoothing for the
    /// bitmaps unless `-S` (ghostscript's default is hard edges).
    fn parse(&self) -> Result<DriverOptions, String> {
        let olang = options_lang(self.lang);
        let mut all: Vec<(&str, OptionValue<'_>)> = Vec::new();
        if option_spec(olang, "correct_font_size").is_some_and(|s| s.effective_for(olang)) {
            all.push(("correct_font_size", OptionValue::Bool(false)));
        }
        if option_spec(olang, "smoothing").is_some_and(|s| s.effective_for(olang)) {
            all.push(("smoothing", OptionValue::Text("0")));
        }
        all.extend(self.items.iter().map(|(k, v)| {
            (
                k.as_str(),
                match v {
                    Value::Bool(b) => OptionValue::Bool(*b),
                    Value::Text(t) => OptionValue::Text(t.as_str()),
                },
            )
        }));
        parse_options(self.language()?, &all).map_err(|e| format!("fig2dev: {e}"))
    }

    fn builder(&mut self) -> Result<&mut Builder, String> {
        self.builder
            .as_mut()
            .ok_or_else(|| "fig2dev: objects before the header (internal error)".to_string())
    }
}

impl Driver for SceneDriver {
    /// `genps_option`, `genbitmaps_option`, `gensvg_option`: the option is
    /// checked now, so that a bad value stops fig2dev before it reads the
    /// figure.
    fn option(&mut self, ctx: &mut Ctx, c: char, arg: Option<&str>) -> Result<(), String> {
        // -L and -G reach every driver; they are handled globally.
        if c == 'L' || c == 'G' {
            return Ok(());
        }
        let lang = options_lang(self.lang);
        let Some(spec) = option_spec(lang, &c.to_string()).filter(|s| s.letter == c) else {
            let msg = format!("Argument -{c} unknown to {} driver.", driver_name(self.lang));
            if BITMAP_LANGUAGES.contains(&lang) {
                // genbitmaps_option warns and goes on.
                ctx.warn(msg);
                return Ok(());
            }
            return Err(msg);
        };
        if self.page_option(ctx, c, arg) {
            return Ok(());
        }
        // genbitmaps_option's `-t`: a specification too long for
        // `gif_transparent[8]` is an error (the argument still has its
        // leading space here, as in fig2dev's, so `#rrggbb` is too long);
        // the message shows the (empty) buffer.
        let too_long_transparent = || {
            if c == 't' && arg.is_some_and(|a| a.len() >= 8) {
                Err(
                    "fig2dev: invalid color specification \n         specify gif transparent color in the form #rrggbb"
                        .to_string(),
                )
            } else {
                Ok(())
            }
        };
        if !spec.effective_for(lang) {
            // fig2dev accepts it here and does nothing; genbitmaps says so
            // for -q and -t.
            match c {
                'q' => ctx.warn("-q option only allowed for jpeg quality; ignored"),
                't' => {
                    ctx.warn("-t option only allowed for GIF transparent color; ignored");
                    too_long_transparent()?;
                }
                _ => {}
            }
            return Ok(());
        }
        too_long_transparent()?;
        // `sscanf(optarg, "%d", ...)` for -b, -q and -S: the leading
        // integer; without one the setting stays as it was. -S must be
        // 0, 1, 2 or 4.
        let int_arg;
        let arg = if (matches!(c, 'b' | 'q' | 'S') && BITMAP_LANGUAGES.contains(&lang))
            || (c == 'b' && self.is_ps_family())
        {
            let a = arg.unwrap_or("");
            let (v, used) = strtol(a);
            if used == 0 {
                return Ok(());
            }
            if c == 'S' && ![0, 1, 2, 4].contains(&v) {
                return Err(format!("fig2dev: bad value for -S option: {a}, should be 1, 2 or 4"));
            }
            int_arg = v.to_string();
            Some(int_arg.as_str())
        } else {
            arg
        };
        if let Support::Planned(task) = spec.support {
            ctx.warn(format!(
                "fig2dev: option -{c} is not implemented in RustFig yet (plan task {task}); ignored"
            ));
            return Ok(());
        }
        if c == 'g' {
            // A colour lookup_X_color doesn't know is warned about and
            // ignored.
            let name = arg.unwrap_or("");
            if x_color(name).is_none() {
                ctx.warn(format!("Can't parse color '{name}', ignoring background option"));
                return Ok(());
            }
        }
        let value = match (spec.arg, arg) {
            (Arg::Flag, _) => Value::Bool(true),
            (_, Some(a)) => Value::Text(a.to_string()),
            (_, None) => Value::Bool(true),
        };
        self.items.push((spec.key.to_string(), value));
        self.parse().map(|_| ())
    }

    fn start(&mut self, ctx: &mut Ctx, _out: &mut Out, objects: &Compound) -> Result<(), String> {
        if self.is_ps_family() {
            // genps_start looks the paper up for every PostScript language.
            paper(&ctx.papersize)?;
        }
        let o = self.parse()?;
        // genps_start: one kind of preview.
        if o.ascii_preview && o.tiff_preview.is_some() {
            return Err("Only one type of preview allowed: -A or -T/-C".into());
        }
        // Without -F, fig2dev reads font sizes as 1/80 inch (`PSFONTMAG`);
        // with it, metric figures keep their text's point size.
        let unit_fix = if ctx.metric { 80.0 / 76.2 } else { 1.0 };
        let text = TextOptions {
            scale: if o.correct_font_size {
                1.0 / unit_fix
            } else {
                72.0 / 80.0
            },
            font: o.font,
            skip_special: matches!(self.lang, "pstex" | "pdftex"),
            eps_preview: self.format.is_bitmap(),
            genps: !matches!(self.format, ExportFormat::Svg),
        };
        self.builder = Some(Builder::new(ctx, text));
        self.options = Some(o);
        self.comments = objects.comments.iter().map(|c| c.text.clone()).collect();
        Ok(())
    }

    /// `genps_grid` (the bitmaps, PostScript, PDF and pstex; not SVG or
    /// the box): remembered, drawn first when the scene is written.
    fn grid(&mut self, _ctx: &mut Ctx, _out: &mut Out, major: f32, minor: f32) -> Result<(), String> {
        if !matches!(self.format, ExportFormat::Svg | ExportFormat::LatexBox) {
            self.grid = Some((major, minor));
        }
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, _out: &mut Out, a: &Arc) -> Result<(), String> {
        self.builder()?.arc(ctx, a);
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, _out: &mut Out, e: &Ellipse) -> Result<(), String> {
        self.builder()?.ellipse(ctx, e);
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, _out: &mut Out, l: &Line) -> Result<(), String> {
        self.builder()?.line(ctx, l);
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, _out: &mut Out, s: &Spline) -> Result<(), String> {
        self.builder()?.spline(ctx, s);
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, _out: &mut Out, t: &Text) -> Result<(), String> {
        self.builder()?.text(ctx, t);
        Ok(())
    }

    fn end(&mut self, ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        let (Some(o), Some(mut b)) = (self.options.take(), self.builder.take()) else {
            return Err("fig2dev: no header written (internal error)".into());
        };
        let mut scene = b.scene();
        // fig2dev's bounds, in Fig units of 1200 per inch.
        let k = file_scale(ctx);
        let raw = Rect::from_points([
            Point::new(f64::from(ctx.llx) * k, f64::from(ctx.lly) * k),
            Point::new(f64::from(ctx.urx) * k, f64::from(ctx.ury) * k),
        ]);
        // The magnification after -m or -Z, with the metric correction.
        let scale = if ctx.mag.is_finite() {
            ctx.mag.clamp(0.001, 1000.0 * 80.0 / 76.2)
        } else {
            1.0
        };
        // `-b`: points on the page; for the bitmaps pixels at 80 dpi
        // before the magnification, so `THICK_SCALE` (15) Fig units each.
        let border = if !o.border.is_finite() {
            0.0
        } else if self.format.is_bitmap() {
            o.border.clamp(0.0, 72_000.0) * 15.0
        } else {
            o.border.clamp(0.0, 72_000.0) * 1200.0 / 72.0 / scale
        };
        let mut bg = o.background.map(Rgba::opaque);
        if o.grayscale {
            scene.map_colors(Rgba::luminance_gray);
            bg = bg.map(Rgba::luminance_gray);
        }
        let transparent = o.transparent.or_else(|| transparent_color(ctx));
        let bounds = raw.expand(border);
        // PostScript pages, and PDF in page mode.
        let layout =
            if self.is_ps_family() && !self.epsflag && matches!(self.format, ExportFormat::Eps | ExportFormat::Pdf) {
                let pt = 72.0 / 1200.0 * scale;
                Some(PageLayout::compute(&PageInput {
                    papersize: &ctx.papersize,
                    landscape: ctx.landscape,
                    center: ctx.center,
                    multi_page: ctx.multi_page,
                    overlap: ctx.overlap,
                    xoff: self.xoff,
                    yoff: self.yoff,
                    fllx: bounds.min.x * pt,
                    flly: bounds.min.y * pt,
                    furx: bounds.max.x * pt,
                    fury: bounds.max.y * pt,
                })?)
            } else {
                None
            };
        let eps_page = layout.is_none();
        if let Some((major, minor)) = self.grid {
            // genps_grid: EPS (and the bitmaps) grid the figure's box, pages
            // the page's size, from the figure's origin.
            let k = file_scale(ctx);
            let ppi = if ctx.ppi.is_finite() && ctx.ppi > 0.0 {
                ctx.ppi
            } else {
                1200.0
            };
            let (lx, ly, ux, uy) = match &layout {
                Some(l) => {
                    let sx = scale * 72.0 / ppi;
                    (0.0, 0.0, f64::from(l.width) / sx, f64::from(l.height) / sx)
                }
                None => {
                    let m = f64::from(if minor == 0.0 { major } else { minor });
                    let fl = |v: f64| if m > 0.0 { (v / m).floor() * m } else { v };
                    (
                        fl(bounds.min.x / k),
                        fl(bounds.min.y / k),
                        bounds.max.x / k,
                        bounds.max.y / k,
                    )
                }
            };
            let spec = GridSpec {
                major,
                minor,
                lx: lx as f32,
                ly: ly as f32,
                ux: ux as f32,
                uy: uy as f32,
                thick_scale: ctx.thick_scale,
                k,
                metric: ctx.metric,
                // One device pixel at 80 dpi times the magnification.
                min_width: if self.format.is_bitmap() && o.smoothing.unwrap_or(0) <= 1 {
                    15.0 / scale
                } else {
                    0.0
                },
            };
            match grid_prims(&spec) {
                Ok(Some(p)) => {
                    scene.prims.splice(0..0, p);
                }
                Ok(None) => {}
                Err(e) => ctx.warn(format!("fig2dev: {e}")),
            }
        }
        let (bytes, notes) = write_with_notes(
            &scene,
            self.format,
            &Page {
                bounds,
                scale,
                background: bg,
                title: o.title.as_deref(),
                embedded_eps: &b.embedded_eps,
                comments: &self.comments,
                // genbitmaps: ghostscript at 80 dpi times the magnification.
                dpi: 80.0,
                smoothing: o.smoothing,
                quality: o.quality,
                transparent,
                layout,
                pdf_minor: self.pdf_minor,
                magnification: if ctx.metric { ctx.mag * 76.2 / 80.0 } else { ctx.mag },
                bbox: o
                    .bbox
                    .as_deref()
                    .filter(|_| eps_page && self.is_ps_family() && !matches!(self.lang, "pdf" | "pdftex"))
                    .map(|spec| UserBox {
                        spec,
                        absolute: o.bbox_absolute,
                    }),
                preview: if matches!(self.format, ExportFormat::Eps) && self.is_ps_family() {
                    match (o.ascii_preview, o.tiff_preview) {
                        (true, _) => Some(PreviewKind::Ascii),
                        (_, Some(false)) => Some(PreviewKind::TiffMono),
                        (_, Some(true)) => Some(PreviewKind::TiffColor),
                        _ => None,
                    }
                } else {
                    None
                },
                metric: ctx.metric,
                opaque: true,
                border_pt: o.border as i32,
            },
        )
        .map_err(|e| format!("fig2dev: {e}"))?;
        for n in notes {
            ctx.warn(n);
        }
        out.bytes(&bytes);
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}
