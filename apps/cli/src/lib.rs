//! rustfig-cli: convert, inspect and benchmark figures; run the MCP server.
//! A library, so that the `rustfig` program runs it too when it is started
//! under the name `rustfig-cli` (one program on disk instead of two).

use rustfig_engine::Engine;
use rustfig_geom::Point;
use rustfig_model::{Document, Object, ObjectKind, PolyKind, Polyline, Style};
use rustfig_render::{raster, SceneOptions};
use serde_json::{json, Value};
use std::process::ExitCode;
use std::time::Instant;

const USAGE: &str = "usage:
  rustfig-cli convert IN OUT [--format LANG] [--magnification PCT] [--tex PROGRAM] [Export panel flags] [--option KEY=VALUE]...
        OUT: .fig/.rustfig save; otherwise an export language (fig2dev's -L names),
        from --format or the file's extension. The output depends only on the
        file and the flags, never on your settings: the Export panel's defaults
        (as File > Export with Reset to xfig defaults), the figure's own LaTeX
        preamble, TeX from the PATH (--tex names a program). Flags set a panel row:
          --border N (points; pixels for bitmaps)   --background N|none|#rrggbb
          --grid MINOR[:MAJOR] (1/4, 10, None)      --preview none|ascii|color|mono
          --page-mode | --no-page-mode              --offset X,Y (each in, cm or fig)
          --overlap | --no-overlap                  --dpi N   --smoothing none|some|more
          --quality N   --transparent N|none|background|#rrggbb
          --embed | --no-embed   --latex | --no-latex (typeset LaTeX texts with TeX)
          --hpgl-font, --hpgl-pcl (and --no-...)    --asy-splines guide|hobby|exact
        formats LANG lists the flags a language takes. --option passes one of
        fig2dev's options by name or letter (g=lightblue, N=true, Z=4w);
        --margin FIGUNITS adds a margin in Fig units
  rustfig-cli formats [LANG]        list the export languages, or one language's options
  rustfig-cli info FILE
  rustfig-cli thumbnail IN OUT.png [--size N]   fit the figure into N x N pixels (default 256; the file manager thumbnailer)
  rustfig-cli perf [--paths N]      check performance budgets on a synthetic document
  rustfig-cli bench FILE            time render, save and open for one file
  rustfig-cli mcp                   run the MCP server on stdio
  rustfig-cli fig2dev ARGS...       fig2dev's own command line (fig2dev -h for its options)";

/// The command line: `rustfig-cli` (this crate's program) and `rustfig` started under that name.
pub fn run() -> ExitCode {
    ExitCode::from(status())
}

/// [`run`] as an exit status (0 for success), for `rustfig`, which ends with `process::exit`.
pub fn status() -> u8 {
    // fig2dev's command line runs exactly as the program itself (stdin,
    // stdout, stderr, exit status), before anything here looks at the arguments.
    if rustfig_engine::fig2dev::process_args().first().map(String::as_str) == Some("fig2dev") {
        let mut args = rustfig_engine::fig2dev::process_args();
        args.drain(..1.min(args.len()));
        return rustfig_engine::fig2dev::run_cli(args);
    }
    let args: Vec<String> = rustfig_engine::fig2dev::process_args();
    let result = match args.first().map(String::as_str) {
        Some("convert") => convert(&args[1..]),
        Some("thumbnail") => rustfig_engine::thumbnail::run(&args[1..]),
        Some("info") => info(&args[1..]),
        Some("formats") => formats(args.get(1).map(String::as_str)),
        Some("perf") => perf(&args[1..]),
        Some("bench") => bench(&args[1..]),
        Some("mcp") => rustfig_mcp::Server::new().serve_stdio().map_err(|e| e.to_string()),
        Some("--version") => {
            println!("rustfig-cli {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn opt(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn open(path: &str) -> Result<Engine, String> {
    let mut e = Engine::new();
    e.execute("file.open", &json!({ "path": path }))
        .map_err(|e| e.message)?;
    Ok(e)
}

type JsonMap = serde_json::Map<String, Value>;

/// The Export panel's rows as convert's flags: (flag, takes a value).
const PANEL_FLAGS: &[(&str, bool)] = &[
    ("--border", true),
    ("--background", true),
    ("--grid", true),
    ("--preview", true),
    ("--page-mode", false),
    ("--no-page-mode", false),
    ("--offset", true),
    ("--overlap", false),
    ("--no-overlap", false),
    ("--dpi", true),
    ("--smoothing", true),
    ("--quality", true),
    ("--transparent", true),
    ("--embed", false),
    ("--no-embed", false),
    ("--latex", false),
    ("--no-latex", false),
    ("--hpgl-font", false),
    ("--no-hpgl-font", false),
    ("--hpgl-pcl", false),
    ("--no-hpgl-pcl", false),
    ("--asy-splines", true),
];

/// Other flags of convert: (flag, takes a value).
const CONVERT_FLAGS: &[(&str, bool)] = &[
    ("--format", true),
    ("--magnification", true),
    ("--margin", true),
    ("--option", true),
    ("--tex", true),
    // Before F21 it ignored the saved settings, which convert now never reads.
    ("--defaults", false),
];

fn number(flag: &str, v: &str) -> Result<f64, String> {
    v.trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("{flag} needs a number, not {v:?}"))
}

fn color_number(flag: &str, v: &str) -> Result<i64, String> {
    v.trim()
        .parse::<i64>()
        .ok()
        .filter(|n| (0..=2047).contains(n))
        .ok_or_else(|| format!("{flag} needs a color number (0-31 or a user color), none or #rrggbb, not {v:?}"))
}

/// An offset with its unit (`1.5`, `2cm`, `600fig`): the panel's value and
/// unit (0 in, 1 cm, 2 Fig units).
fn offset(flag: &str, v: &str) -> Result<(f64, u8), String> {
    let v = v.trim();
    for (suffix, unit) in [("in", 0), ("cm", 1), ("fig", 2)] {
        if let Some(n) = v.strip_suffix(suffix) {
            return Ok((number(flag, n)?, unit));
        }
    }
    Ok((number(flag, v)?, 0))
}

/// Convert's flags as the Export panel's settings (`export_panel::merge`
/// checks them as for `file.export`'s `panel`), plus the values that go
/// straight into `file.export` (a background or transparent color given as
/// #rrggbb).
fn panel_overrides(args: &[String]) -> Result<(JsonMap, JsonMap), String> {
    let mut s = serde_json::Map::new();
    let mut direct = serde_json::Map::new();
    let mut i = 0;
    while i < args.len() {
        let flag = args.get(i).map(String::as_str).unwrap_or_default();
        let Some(&(_, takes)) = PANEL_FLAGS.iter().chain(CONVERT_FLAGS).find(|(f, _)| *f == flag) else {
            return Err(format!("unknown flag {flag:?}\n{USAGE}"));
        };
        let v = if takes {
            i += 1;
            args.get(i).ok_or_else(|| format!("{flag} needs a value"))?.as_str()
        } else {
            ""
        };
        i += 1;
        let mut set = |k: &str, val: Value| {
            s.insert(k.to_string(), val);
        };
        match flag {
            "--border" => set("border", json!(number(flag, v)?)),
            "--background" if v.eq_ignore_ascii_case("none") => set("background", json!(-1)),
            "--background" if v.starts_with('#') => {
                direct.insert("background".into(), json!(v));
            }
            "--background" => set("background", json!(color_number(flag, v)?)),
            "--grid" => {
                let (minor, major) = v.split_once(':').unwrap_or((v, "None"));
                let none = |g: &str| {
                    if g.trim().is_empty() {
                        "None".to_string()
                    } else {
                        g.trim().to_string()
                    }
                };
                set("grid_minor", json!(none(minor)));
                set("grid_major", json!(none(major)));
            }
            "--preview" => {
                let n = ["none", "ascii", "color", "mono"]
                    .iter()
                    .position(|p| p.eq_ignore_ascii_case(v))
                    .ok_or_else(|| format!("--preview needs none, ascii, color or mono, not {v:?}"))?;
                set("preview", json!(n));
            }
            "--page-mode" | "--no-page-mode" => set("pdf_page_mode", json!(flag == "--page-mode")),
            "--offset" => {
                let (x, y) = v
                    .split_once(',')
                    .ok_or("--offset needs X,Y (each with in, cm or fig)")?;
                let ((x, xu), (y, yu)) = (offset(flag, x)?, offset(flag, y)?);
                set("x_offset", json!(x));
                set("x_unit", json!(xu));
                set("y_offset", json!(y));
                set("y_unit", json!(yu));
            }
            "--overlap" | "--no-overlap" => set("overlap", json!(flag == "--overlap")),
            "--dpi" => set("dpi", json!(number(flag, v)?)),
            "--smoothing" => {
                let n = match v.to_ascii_lowercase().as_str() {
                    "none" | "1" => 1,
                    "some" | "2" => 2,
                    "more" | "4" => 4,
                    _ => return Err(format!("--smoothing needs none, some or more (1, 2, 4), not {v:?}")),
                };
                set("smoothing", json!(n));
            }
            "--quality" => set("quality", json!(number(flag, v)?.round().clamp(1.0, 100.0) as u32)),
            "--transparent" if v.eq_ignore_ascii_case("none") => {
                set("transparent", json!(rustfig_engine::export_panel::TRANSP_NONE))
            }
            "--transparent" if v.eq_ignore_ascii_case("background") => {
                set("transparent", json!(rustfig_engine::export_panel::TRANSP_BACKGROUND))
            }
            "--transparent" if v.starts_with('#') => {
                direct.insert("options".into(), json!({ "t": v }));
            }
            "--transparent" => set("transparent", json!(color_number(flag, v)?)),
            "--embed" | "--no-embed" => set("embed", json!(flag == "--embed")),
            "--latex" | "--no-latex" => set("typeset_latex", json!(flag == "--latex")),
            "--hpgl-font" | "--no-hpgl-font" => set("hpgl_font", json!(flag == "--hpgl-font")),
            "--hpgl-pcl" | "--no-hpgl-pcl" => set("hpgl_pcl", json!(flag == "--hpgl-pcl")),
            "--asy-splines" => {
                if !rustfig_engine::export_panel::ASY_SPLINES.contains(&v) {
                    return Err(format!("--asy-splines needs guide, hobby or exact, not {v:?}"));
                }
                set("asy_splines", json!(v));
            }
            _ => {}
        }
    }
    Ok((s, direct))
}

/// The panel flags language `id` takes (its Export panel rows).
fn panel_flags_for(id: &str) -> Vec<&'static str> {
    let r = rustfig_engine::export_panel::rows(id);
    let mut v = Vec::new();
    let mut add = |on: bool, flags: &[&'static str]| {
        if on {
            v.extend_from_slice(flags);
        }
    };
    add(r.border, &["--border"]);
    add(r.background, &["--background"]);
    add(r.grid, &["--grid"]);
    add(r.preview, &["--preview"]);
    add(r.pdf_page_mode, &["--page-mode"]);
    add(id == "ps" || r.pdf_page_mode, &["--offset"]);
    add(r.multiple, &["--overlap"]);
    add(r.bitmap, &["--dpi", "--smoothing"]);
    add(r.quality, &["--quality"]);
    add(r.transparent, &["--transparent"]);
    add(r.embed, &["--embed"]);
    add(r.typeset_latex, &["--latex"]);
    add(r.hpgl, &["--hpgl-font", "--hpgl-pcl"]);
    add(r.asy, &["--asy-splines"]);
    v
}

/// `rustfig-cli convert ARGS…` (public for the tests).
pub fn convert(args: &[String]) -> Result<(), String> {
    let (Some(input), Some(output)) = (args.first(), args.get(1)) else {
        return Err(USAGE.into());
    };
    let flags = args.get(2..).unwrap_or_default();
    let (panel, direct) = panel_overrides(flags)?;
    let mut e = open(input)?;
    if let Some(tex) = opt(flags, "--tex") {
        e.execute("view.set_latex_program", &json!({ "program": tex }))
            .map_err(|e| e.message)?;
        if e.latex.program().is_none() {
            return Err(format!("--tex: {tex}: no such TeX program"));
        }
    }
    let lower = output.to_ascii_lowercase();
    if lower.ends_with(".fig") || lower.ends_with(".rustfig") || lower.ends_with(".xfig") {
        e.execute("file.save", &json!({ "path": output }))
            .map_err(|e| e.message)?;
        return Ok(());
    }
    // What the Export panel sends: the figure's magnification, both layer
    // choices off (no hidden depths here) and the panel's values for the
    // language: the defaults with the flags laid over them.
    let mut p = json!({
        "path": output,
        "magnification": e.doc().settings.magnification,
        "active_layers_only": false,
        "whole_figure_bounds": false,
        "panel": Value::Object(panel),
    });
    if let Some(m) = opt(flags, "--magnification") {
        p["magnification"] = json!(number("--magnification", &m)?);
    }
    if let Some(m) = opt(flags, "--margin") {
        p["margin"] = json!(number("--margin", &m)?);
    }
    if let Some(f) = opt(flags, "--format") {
        p["format"] = json!(f);
    }
    let mut options = serde_json::Map::new();
    for (i, a) in flags.iter().enumerate() {
        if a != "--option" {
            continue;
        }
        let kv = flags.get(i + 1).ok_or("--option needs KEY=VALUE")?;
        let (k, v) = kv.split_once('=').unwrap_or((kv.as_str(), "true"));
        // Numbers stay text: the engine reads them, and depth lists like
        // -5 or titles like 2024 must stay as written.
        let value = match v {
            "true" => json!(true),
            "false" => json!(false),
            _ => json!(v),
        };
        options.insert(k.to_string(), value);
    }
    for (k, v) in direct {
        match (k.as_str(), v) {
            ("options", Value::Object(o)) => {
                for (ok, ov) in o {
                    options.entry(ok).or_insert(ov);
                }
            }
            (_, v) => p[k] = v,
        }
    }
    if !options.is_empty() {
        p["options"] = Value::Object(options);
    }
    let r = e.execute("file.export", &p).map_err(|e| e.message)?;
    for w in r["warnings"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        eprintln!("rustfig-cli: {w}");
    }
    Ok(())
}

fn formats(lang: Option<&str>) -> Result<(), String> {
    let mut e = Engine::new();
    let v = e.execute("file.export_formats", &json!({})).map_err(|e| e.message)?;
    let all = v["formats"].as_array().cloned().unwrap_or_default();
    if let Some(lang) = lang {
        let f = all
            .iter()
            .find(|f| f["id"].as_str().is_some_and(|id| id.eq_ignore_ascii_case(lang)))
            .ok_or_else(|| format!("unknown language {lang:?}"))?;
        println!(
            "{} options (--option KEY=VALUE; KEY may be the fig2dev letter):",
            f["id"].as_str().unwrap_or_default()
        );
        let id = f["id"].as_str().unwrap_or_default();
        let flags = panel_flags_for(id);
        if !flags.is_empty() {
            println!("Export panel flags: {}", flags.join(" "));
        }
        for o in f["options"].as_array().into_iter().flatten() {
            if !option_shown(id, o) {
                continue;
            }
            println!(
                "  -{} {:18} {:7} {}",
                o["letter"].as_str().unwrap_or_default(),
                o["key"].as_str().unwrap_or_default(),
                o["arg"].as_str().unwrap_or_default(),
                o["help"].as_str().unwrap_or_default(),
            );
        }
        return Ok(());
    }
    for f in &all {
        let status = match (f["status"].as_str(), f["task"].as_str()) {
            (Some("available"), _) => "available".to_string(),
            (Some(s), Some(t)) => format!("{s} ({t})"),
            (s, _) => s.unwrap_or_default().to_string(),
        };
        println!(
            "{:10} .{:9} {:34} {}",
            f["id"].as_str().unwrap_or_default(),
            f["extension"].as_str().unwrap_or_default(),
            f["description"].as_str().unwrap_or_default(),
            status
        );
    }
    Ok(())
}

/// Whether `formats LANG` lists option `o` (from `file.export_formats`)
/// of language `lang`: only options RustFig honors and that do something
/// for `lang` (fig2dev accepts some and ignores them, like the page
/// options for EPS).
fn option_shown(lang: &str, o: &Value) -> bool {
    if o["available"].as_bool() != Some(true) {
        return false;
    }
    let key = o["key"].as_str().unwrap_or_default();
    let letter = o["letter"].as_str().unwrap_or_default();
    match rustfig_engine::fig2dev::options::options_for(lang).find(|s| s.key == key && s.letter.to_string() == letter) {
        Some(s) => s.effective_for(lang),
        // An option of a ported fig2dev driver, by letter.
        None => key == letter,
    }
}

fn info(args: &[String]) -> Result<(), String> {
    let path = args.first().ok_or(USAGE)?;
    let mut e = open(path)?;
    let v = e.execute("doc.info", &json!({})).map_err(|e| e.message)?;
    println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
    Ok(())
}

/// A synthetic document of `n` paths (boxes, polylines, splines, circles, texts).
fn synthetic(n: usize) -> Document {
    let mut doc = Document::new();
    let cols = (n as f64).sqrt().ceil().max(1.0) as usize;
    for i in 0..n {
        let x = (i % cols) as f64 * 300.0;
        let y = (i / cols) as f64 * 300.0;
        let mut s = Style::default();
        s.depth = (i % 100) as i32;
        s.pen_color = rustfig_model::ColorRef((i % 32) as i32);
        if i % 3 == 0 {
            s.area_fill = 20;
            s.fill_color = rustfig_model::ColorRef(((i + 5) % 32) as i32);
        }
        let kind = match i % 5 {
            0 => ObjectKind::Polyline(Polyline::rect(s, Point::new(x, y), Point::new(x + 200.0, y + 150.0))),
            1 => ObjectKind::Polyline(Polyline::new(
                PolyKind::Polyline,
                s,
                vec![
                    Point::new(x, y),
                    Point::new(x + 100.0, y + 200.0),
                    Point::new(x + 200.0, y),
                ],
            )),
            2 => ObjectKind::Spline(rustfig_model::Spline::new(
                rustfig_model::SplineKind::X,
                false,
                s,
                vec![
                    Point::new(x, y),
                    Point::new(x + 80.0, y + 200.0),
                    Point::new(x + 160.0, y),
                    Point::new(x + 240.0, y + 200.0),
                ],
            )),
            3 => ObjectKind::Ellipse(rustfig_model::Ellipse {
                kind: rustfig_model::EllipseKind::CircleRadius,
                style: s,
                direction: 1,
                angle: 0.0,
                center: Point::new(x + 100.0, y + 100.0),
                radii: Point::new(90.0, 90.0),
                start: Point::new(x, y),
                end: Point::new(x, y),
            }),
            _ => ObjectKind::Polyline(Polyline::new(
                PolyKind::Polygon,
                s,
                vec![
                    Point::new(x, y),
                    Point::new(x + 200.0, y + 50.0),
                    Point::new(x + 100.0, y + 200.0),
                ],
            )),
        };
        doc.objects.push(std::sync::Arc::new(Object::new(kind)));
    }
    doc.assign_ids();
    doc
}

fn time<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let t = Instant::now();
    let r = f();
    (r, t.elapsed().as_secs_f64() * 1000.0)
}

fn load_average() -> Option<f64> {
    let s = std::fs::read_to_string("/proc/loadavg").ok()?;
    s.split_whitespace().next()?.parse().ok()
}

struct Report {
    rows: Vec<(String, f64, f64)>,
}

impl Report {
    fn add(&mut self, name: &str, ms: f64, budget: f64) {
        self.rows.push((name.to_string(), ms, budget));
    }
    fn print(&self) -> bool {
        let mut ok = true;
        println!("{:<28} {:>10} {:>10}  status", "measurement", "ms", "budget");
        for (n, ms, b) in &self.rows {
            let pass = ms <= b;
            ok &= pass;
            println!("{n:<28} {ms:>10.1} {b:>10.0}  {}", if pass { "ok" } else { "OVER" });
        }
        if let Some(l) = load_average() {
            if l > 2.0 {
                println!("warning: load average {l:.1}; timings are noisy");
            }
        }
        ok
    }
}

fn measure_doc(doc: &Document, report: &mut Report, scale_budget: f64) -> Result<(), String> {
    let (scene, ms) = time(|| {
        rustfig_render::build_scene(
            doc,
            &SceneOptions {
                screen: true,
                ..Default::default()
            },
        )
    });
    report.add("scene build", ms, 400.0 * scale_budget);
    let (bb, ms) = time(|| rustfig_format::bridge::document_bounds(doc));
    report.add("bounds (fig2dev)", ms, 200.0 * scale_budget);
    let (w, h) = (1280u32, 860u32);
    let mut view = rustfig_engine::View {
        size: (f64::from(w), f64::from(h)),
        ..Default::default()
    };
    view.fit(&bb);
    let render = |view: &rustfig_engine::View| -> Result<f64, String> {
        let opts = raster::RasterOptions {
            transform: view.transform(),
            min_line_px: 1.0,
            adjust_px: 0.0,
            pattern_k: 1,
            pattern: rustfig_render::pattern::PatternFrame::screen(),
        };
        let (pm, ms) = time(|| rustfig_render::rasterize_banded(&scene, w, h, &opts));
        pm.ok_or("cannot allocate pixmap")?;
        Ok(ms)
    };
    report.add("render (fit)", render(&view)?, 1500.0 * scale_budget);
    view.pan_pixels(200.0, 100.0);
    report.add("render (pan)", render(&view)?, 1500.0 * scale_budget);
    let c = rustfig_geom::Point::new(640.0, 430.0);
    view.zoom_about(view.zoom * 4.0, c);
    report.add("render (zoom x4)", render(&view)?, 800.0 * scale_budget);

    let mut e = Engine::new();
    e.replace_document(doc.clone(), None);
    let pts: Vec<Point> = (0..1000)
        .map(|i| {
            Point::new(
                bb.min.x + (i as f64 * 37.0) % bb.width().max(1.0),
                bb.min.y + (i as f64 * 53.0) % bb.height().max(1.0),
            )
        })
        .collect();
    let (_, ms) = time(|| pts.iter().filter_map(|p| e.hit_test(*p, 30.0)).count());
    report.add("hit test (per click)", ms / 1000.0, 20.0 * scale_budget);
    // A1, A9: one-object edits and their undo, on the document plus 5,000
    // texts (re-measuring every text made each edit cost milliseconds).
    for i in 0..5000 {
        let (x, y) = (
            bb.min.x + (i % 100) as f64 * 400.0,
            bb.max.y + 600.0 + (i / 100) as f64 * 300.0,
        );
        e.execute("shape.text", &json!({"x": x, "y": y, "text": format!("label {i}")}))
            .map_err(|e| e.message)?;
    }
    let ids: Vec<u64> = e.doc().objects.iter().step_by(97).take(50).map(|o| o.id.0).collect();
    let (_, ms) = time(|| {
        for id in &ids {
            let _ = e.execute("transform.move", &json!({"ids": [id], "dx": 15, "dy": 0}));
        }
    });
    report.add("edit (one object)", ms / ids.len().max(1) as f64, 10.0 * scale_budget);
    let (_, ms) = time(|| {
        for _ in &ids {
            e.undo();
        }
    });
    report.add("undo (one step)", ms / ids.len().max(1) as f64, 5.0 * scale_budget);
    let (text, ms) = time(|| rustfig_format::write_fig(doc));
    report.add("save .fig", ms, 800.0 * scale_budget);
    let (parsed, ms) = time(|| rustfig_format::read_fig(text.as_bytes()));
    parsed.map_err(|e| e.to_string())?;
    report.add("open .fig", ms, 1200.0 * scale_budget);
    // As `file.export` exports: Fig text through fig2dev's reader and driver.
    let (svg, ms) = time(|| rustfig_engine::fig2dev::export(doc, "svg", &[], None));
    svg?;
    report.add("export SVG", ms, 1500.0 * scale_budget);
    Ok(())
}

/// The leaves of an object (compounds opened).
fn leaves<'a>(o: &'a Object, out: &mut Vec<&'a Object>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    match &o.kind {
        ObjectKind::Compound(c) => c.objects.iter().for_each(|x| leaves(x, out, cap)),
        _ => out.push(o),
    }
}

/// P6: group everything into one compound, then drag it in Move mode as the
/// canvas does (press on a member, pointer moves with `overlay()` asked for
/// each, release): ms per move step and per grouping.
fn measure_group_drag(doc: &Document, report: &mut Report, scale_budget: f64) -> Result<(), String> {
    let mut e = Engine::new();
    e.replace_document(doc.clone(), None);
    if doc.objects.len() < 2 {
        return Ok(());
    }
    // Commands on a large selection must stay linear (`Vec::contains` or
    // `Document::get` per selected id made these take seconds).
    let (r, ms) = time(|| e.execute("edit.select_all", &json!({})));
    r.map_err(|e| e.message)?;
    report.add("select all", ms, 200.0 * scale_budget);
    let (r, ms) = time(|| e.execute("transform.move", &json!({"dx": 60, "dy": 0, "reappend": true})));
    r.map_err(|e| e.message)?;
    report.add("move all (selected)", ms, 500.0 * scale_budget);
    let (r, ms) = time(|| e.execute("object.group", &json!({})));
    r.map_err(|e| e.message)?;
    report.add("group all", ms, 1000.0 * scale_budget);
    let Some(group) = e.doc().objects.first().cloned() else {
        return Ok(());
    };
    let mut ls = Vec::new();
    leaves(&group, &mut ls, 300);
    let tol = 40.0;
    let press = ls
        .iter()
        .map(|o| {
            let b = o.geometry_bbox();
            Point::new(b.min.x, (b.min.y + b.max.y) / 2.0)
        })
        .find(|p| e.hit_test(*p, tol).is_some());
    let Some(p0) = press else {
        println!("drag: no pickable point found, skipped");
        return Ok(());
    };
    let before = e.doc().objects.first().map(|o| o.geometry_bbox());
    e.execute("tool.move", &json!({})).map_err(|e| e.message)?;
    use rustfig_engine::tools::{Button, ToolEvent};
    let ev = |e: &mut Engine, ev| e.tool_event(ev).map_err(|e| e.message);
    ev(
        &mut e,
        ToolEvent::Press {
            pos: p0,
            button: Button::Left,
            shift: false,
        },
    )?;
    let steps = 40usize;
    let (r, ms) = time(|| {
        for i in 1..=steps {
            let pos = Point::new(p0.x + 40.0 * i as f64, p0.y + 20.0 * i as f64);
            ev(&mut e, ToolEvent::Move { pos })?;
            let _ = e.overlay();
        }
        Ok::<(), String>(())
    });
    r?;
    report.add("drag (per move step)", ms / steps as f64, 20.0 * scale_budget);
    let end = Point::new(p0.x + 40.0 * steps as f64, p0.y + 20.0 * steps as f64);
    let (r, ms) = time(|| {
        ev(
            &mut e,
            ToolEvent::Release {
                pos: end,
                button: Button::Left,
            },
        )
    });
    r?;
    report.add("drag release (commit)", ms, 100.0 * scale_budget);
    let after = e.doc().objects.first().map(|o| o.geometry_bbox());
    match (before, after) {
        (Some(a), Some(b)) if (b.min.x - a.min.x - 40.0 * steps as f64).abs() < 300.0 => {}
        (a, b) => return Err(format!("the drag did not move the group: {a:?} -> {b:?}")),
    }
    Ok(())
}

fn perf(args: &[String]) -> Result<(), String> {
    // Timings must not depend on the installed fonts (F1).
    rustfig_engine::text::set_builtin_fonts(true);
    let n = opt(args, "--paths")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50_000usize)
        .clamp(1, 2_000_000);
    println!("synthetic document: {n} paths");
    let doc = synthetic(n);
    let mut report = Report { rows: Vec::new() };
    let scale = (n as f64 / 50_000.0).max(0.02);
    measure_doc(&doc, &mut report, scale)?;
    measure_group_drag(&doc, &mut report, scale)?;
    if report.print() {
        Ok(())
    } else {
        Err("some budgets were exceeded".into())
    }
}

fn bench(args: &[String]) -> Result<(), String> {
    rustfig_engine::text::set_builtin_fonts(true); // see perf()
    let path = args.first().ok_or(USAGE)?;
    let (e, ms) = time(|| open(path));
    let e = e?;
    println!("open: {ms:.1} ms, {} objects", e.doc().leaf_count());
    let mut report = Report { rows: Vec::new() };
    measure_doc(e.doc(), &mut report, 1.0)?;
    measure_group_drag(e.doc(), &mut report, 1.0)?;
    report.print();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keys `formats LANG` lists.
    fn shown(lang: &str) -> Vec<String> {
        let mut e = Engine::new();
        let v = e.execute("file.export_formats", &json!({})).unwrap();
        let f = v["formats"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["id"] == lang)
            .unwrap()
            .clone();
        f["options"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| option_shown(lang, o))
            .map(|o| o["key"].as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn formats_lists_only_effective_options() {
        let eps = shown("eps");
        assert!(eps.contains(&"border".to_string()), "{eps:?}");
        assert!(
            eps.contains(&"grid".to_string()) && !shown("svg").contains(&"grid".to_string()),
            "{eps:?}"
        );
        for k in ["center", "page_mode", "paper_size", "quality"] {
            assert!(!eps.contains(&k.to_string()), "eps lists {k}: {eps:?}");
        }
        let png = shown("png");
        assert!(png.contains(&"smoothing".to_string()), "{png:?}");
        assert!(!png.contains(&"quality".to_string()), "-q is JPEG only: {png:?}");
        assert!(shown("jpeg").contains(&"quality".to_string()));
        assert!(shown("svg").contains(&"paper_size".to_string()), "svg -z is ported");
    }
}
