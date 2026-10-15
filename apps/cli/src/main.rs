//! rustfig-cli: convert, inspect and benchmark figures; run the MCP server.

use rustfig_engine::Engine;
use rustfig_geom::Point;
use rustfig_model::{Document, Object, ObjectKind, PolyKind, Polyline, Style};
use rustfig_render::{raster, ExportFormat, ExportOptions, Scene, SceneOptions};
use serde_json::{json, Value};
use std::process::ExitCode;
use std::time::Instant;

const USAGE: &str = "usage:
  rustfig-cli convert IN OUT [--format LANG] [--magnification PCT] [--dpi N] [--margin FIGUNITS] [--background #rrggbb]
                             [--option KEY=VALUE]...
        OUT: .fig/.rustfig save; otherwise an export language (fig2dev's -L names),
        from --format or the file's extension; --option passes one of fig2dev's
        options by name or letter (border=10, g=lightblue, N=true, Z=4w)
  rustfig-cli formats [LANG]        list the export languages, or one language's options
  rustfig-cli info FILE
  rustfig-cli perf [--paths N]      check performance budgets on a synthetic document
  rustfig-cli bench FILE            time render, save and open for one file
  rustfig-cli mcp                   run the MCP server on stdio";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("convert") => convert(&args[1..]),
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
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
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

fn convert(args: &[String]) -> Result<(), String> {
    let (Some(input), Some(output)) = (args.first(), args.get(1)) else {
        return Err(USAGE.into());
    };
    let mut e = open(input)?;
    let lower = output.to_ascii_lowercase();
    if lower.ends_with(".fig") || lower.ends_with(".rustfig") || lower.ends_with(".xfig") {
        e.execute("file.save", &json!({ "path": output }))
            .map_err(|e| e.message)?;
        return Ok(());
    }
    let mut p = json!({ "path": output });
    for (flag, key) in [
        ("--magnification", "magnification"),
        ("--dpi", "dpi"),
        ("--margin", "margin"),
    ] {
        if let Some(v) = opt(args, flag) {
            let n: f64 = v.parse().map_err(|_| format!("{flag} needs a number"))?;
            p[key] = json!(n);
        }
    }
    if let Some(bg) = opt(args, "--background") {
        p["background"] = json!(bg);
    }
    if let Some(f) = opt(args, "--format") {
        p["format"] = json!(f);
    }
    let mut options = serde_json::Map::new();
    for (i, a) in args.iter().enumerate() {
        if a != "--option" {
            continue;
        }
        let kv = args.get(i + 1).ok_or("--option needs KEY=VALUE")?;
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
    if !options.is_empty() {
        p["options"] = serde_json::Value::Object(options);
    }
    e.execute("file.export", &p).map_err(|e| e.message)?;
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
/// of language `lang`: only options RustFig honours and that do something
/// for `lang` (fig2dev accepts some and ignores them, like the page
/// options for EPS).
fn option_shown(lang: &str, o: &Value) -> bool {
    if o["available"].as_bool() != Some(true) {
        return false;
    }
    let key = o["key"].as_str().unwrap_or_default();
    let letter = o["letter"].as_str().unwrap_or_default();
    rustfig_render::options::options_for(lang)
        .find(|s| s.key == key && s.letter.to_string() == letter)
        .is_some_and(|s| s.effective_for(lang))
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
        Scene::build(
            doc,
            &SceneOptions {
                screen: true,
                ..Default::default()
            },
        )
    });
    report.add("scene build", ms, 400.0 * scale_budget);
    let bb = doc.bbox();
    let (w, h) = (1280u32, 860u32);
    let mut view = rustfig_engine::View {
        size: (f64::from(w), f64::from(h)),
        ..Default::default()
    };
    view.fit(&bb);
    let render = |view: &rustfig_engine::View| -> Result<f64, String> {
        let mut pm = raster::Pixmap::new(w, h).ok_or("cannot allocate pixmap")?;
        let ((), ms) = time(|| {
            raster::rasterize(
                &scene,
                &mut pm,
                &raster::RasterOptions {
                    transform: view.transform(),
                    min_line_px: 1.0,
                    pattern: rustfig_render::pattern::PatternFrame::screen(),
                },
            )
        });
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
    let (svg, ms) = time(|| rustfig_render::export(doc, ExportFormat::Svg, &ExportOptions::default()));
    svg?;
    report.add("export SVG", ms, 1500.0 * scale_budget);
    Ok(())
}

fn perf(args: &[String]) -> Result<(), String> {
    let n = opt(args, "--paths")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50_000usize)
        .clamp(1, 2_000_000);
    println!("synthetic document: {n} paths");
    let doc = synthetic(n);
    let mut report = Report { rows: Vec::new() };
    measure_doc(&doc, &mut report, (n as f64 / 50_000.0).max(0.02))?;
    if report.print() {
        Ok(())
    } else {
        Err("some budgets were exceeded".into())
    }
}

fn bench(args: &[String]) -> Result<(), String> {
    let path = args.first().ok_or(USAGE)?;
    let (e, ms) = time(|| open(path));
    let e = e?;
    println!("open: {ms:.1} ms, {} objects", e.doc().leaf_count());
    let mut report = Report { rows: Vec::new() };
    measure_doc(e.doc(), &mut report, 1.0)?;
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
        for k in ["center", "page_mode", "paper_size", "grid", "quality"] {
            assert!(!eps.contains(&k.to_string()), "eps lists {k}: {eps:?}");
        }
        let png = shown("png");
        assert!(png.contains(&"smoothing".to_string()), "{png:?}");
        assert!(!png.contains(&"quality".to_string()), "-q is JPEG only: {png:?}");
        assert!(shown("jpeg").contains(&"quality".to_string()));
        assert!(!shown("svg").contains(&"paper_size".to_string()), "planned");
    }
}
