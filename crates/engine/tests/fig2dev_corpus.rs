//! A10: RustFig's EPS, PDF and PNG compared with fig2dev's on xfig's sample
//! figures.
//!
//! Opt-in and slow (minutes): `cargo test --release -p rustfig-engine --test
//! fig2dev_corpus -- --ignored --nocapture`. Needs fig2dev and ghostscript
//! (test-only; RustFig never runs them) and skips without them.
//!
//! Corpus: the A8 fixtures, fig2dev's `tests/data`, xfig's `Libraries` (from
//! `old/` until D6 copies them into `libraries/`). `RUSTFIG_CORPUS_FILTER`
//! keeps paths containing a substring; `RUSTFIG_CORPUS_LIMIT=N` takes an even
//! sample of N figures.
//!
//! For each figure: fig2dev's EPS and RustFig's EPS, both rasterised by
//! ghostscript (so only the writers differ); RustFig's own PNG against
//! fig2dev's EPS; fig2dev's PDF against RustFig's PDF; the EPS bounding boxes.
//! The report `target/fig2dev-reference/corpus/index.html` lists every
//! figure, worst first, with images of the worst. Summary scores are checked
//! against `fixtures/corpus-scores.json` when the corpus has the same size;
//! `UPDATE_REFERENCE=1` rewrites them.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use rustfig_engine::model::Rgb;
use rustfig_engine::render::{export, ExportFormat, ExportOptions};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use support::{compare, fig2dev};

/// Rasterising resolution.
const DPI: u32 = 150;
/// Figures shown with images in the report.
const SHOWN: usize = 150;
/// How much a summary score may worsen before the run fails.
const MARGIN: f64 = 0.005;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn out_dir() -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("target"));
    target.join("fig2dev-reference/corpus")
}

/// Every `.fig` below `dir`, sorted.
fn figures_in(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| Some(e.ok()?.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            figures_in(&p, out);
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("fig")) {
            out.push(p);
        }
    }
}

fn corpus() -> Vec<PathBuf> {
    let r = root();
    let mut figs = Vec::new();
    for dir in [
        "crates/engine/tests/fixtures",
        "old/fig2dev-3.2.9a/fig2dev/tests/data",
        "libraries",
        "old/xfig-3.2.9a/Libraries",
    ] {
        // Prefer the bundled copy of the libraries once D6 lands.
        if dir.starts_with("old/xfig") && r.join("libraries").is_dir() {
            continue;
        }
        figures_in(&r.join(dir), &mut figs);
    }
    if let Ok(f) = std::env::var("RUSTFIG_CORPUS_FILTER") {
        figs.retain(|p| p.to_string_lossy().contains(&f));
    }
    if let Some(n) = std::env::var("RUSTFIG_CORPUS_LIMIT")
        .ok()
        .and_then(|n| n.parse::<usize>().ok())
    {
        if n > 0 && n < figs.len() {
            let step = figs.len() as f64 / n as f64;
            figs = (0..n)
                .filter_map(|i| figs.get((i as f64 * step) as usize).cloned())
                .collect();
        }
    }
    figs
}

#[derive(Default, Clone)]
struct Outcome {
    name: String,
    /// RustFig's EPS against fig2dev's EPS (both through ghostscript).
    eps: Option<compare::Score>,
    /// RustFig's PNG renderer against fig2dev's EPS.
    png: Option<compare::Score>,
    /// RustFig's PDF against fig2dev's PDF.
    pdf: Option<compare::Score>,
    /// Largest difference of the bounding boxes' width or height, in points.
    bbox: Option<f64>,
    errors: Vec<String>,
}

/// Width and height of an EPS `%%BoundingBox`.
fn bbox_size(eps: &[u8]) -> Option<(f64, f64)> {
    let text = String::from_utf8_lossy(eps.get(..eps.len().min(64 * 1024))?);
    let line = text
        .lines()
        .find(|l| l.starts_with("%%BoundingBox:") && !l.contains("atend"))?;
    let v: Vec<f64> = line.split_whitespace().skip(1).filter_map(|n| n.parse().ok()).collect();
    match v.as_slice() {
        [a, b, c, d] => Some((c - a, d - b)),
        _ => None,
    }
}

/// The images of one figure: fig2dev's EPS raster, RustFig's EPS raster.
struct Rasters {
    reference: Vec<u8>,
    eps: Vec<u8>,
}

fn rustfig(doc: &rustfig_engine::model::Document, f: ExportFormat) -> Result<Vec<u8>, String> {
    let opts = ExportOptions {
        dpi: f64::from(DPI),
        background: Some(Rgb::from_hex("#ffffff").ok_or("colour")?),
        ..Default::default()
    };
    export(doc, f, &opts)
}

fn run_one(path: &Path) -> (Outcome, Option<Rasters>) {
    let name = path
        .strip_prefix(root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace("old/xfig-3.2.9a/", "")
        .replace("old/fig2dev-3.2.9a/", "");
    let mut o = Outcome {
        name,
        ..Default::default()
    };
    let fig = match std::fs::read(path) {
        Ok(f) => f,
        Err(e) => {
            o.errors.push(format!("read: {e}"));
            return (o, None);
        }
    };
    let dir = path.parent();
    let doc = match rustfig_engine::format::read_fig(&fig) {
        Ok(d) => d,
        Err(e) => {
            o.errors.push(format!("RustFig cannot read it: {e}"));
            return (o, None);
        }
    };
    let mut rasters = None;
    match fig2dev::convert(&fig, "eps", &xfig_args(), dir) {
        Err(e) => o.errors.push(e),
        Ok(ref_eps) => {
            let ref_png = fig2dev::rasterise(&ref_eps, false, DPI);
            let our_eps = rustfig(&doc, ExportFormat::Eps);
            match (&ref_png, &our_eps) {
                (Ok(rp), Ok(oe)) => {
                    if let (Some((w1, h1)), Some((w2, h2))) = (bbox_size(&ref_eps), bbox_size(oe)) {
                        o.bbox = Some((w1 - w2).abs().max((h1 - h2).abs()));
                    }
                    match fig2dev::rasterise(oe, false, DPI) {
                        Ok(op) => {
                            match compare::compare(rp, &op) {
                                Ok(c) => o.eps = Some(c.score),
                                Err(e) => o.errors.push(format!("compare eps: {e}")),
                            }
                            rasters = Some(Rasters {
                                reference: rp.clone(),
                                eps: op,
                            });
                        }
                        Err(e) => o.errors.push(format!("RustFig's EPS: {e}")),
                    }
                    match rustfig(&doc, ExportFormat::Png).and_then(|png| compare::compare(rp, &png)) {
                        Ok(c) => o.png = Some(c.score),
                        Err(e) => o.errors.push(format!("RustFig's PNG: {e}")),
                    }
                }
                (Err(e), _) => o.errors.push(format!("fig2dev's EPS: {e}")),
                (_, Err(e)) => o.errors.push(format!("RustFig's EPS: {e}")),
            }
        }
    }
    let pdf = fig2dev::convert(&fig, "pdf", &xfig_args(), dir)
        .and_then(|p| fig2dev::rasterise(&p, true, DPI))
        .and_then(|rp| {
            let ours = rustfig(&doc, ExportFormat::Pdf)?;
            let op = fig2dev::rasterise(&ours, true, DPI)?;
            compare::compare(&rp, &op)
        });
    match pdf {
        Ok(c) => o.pdf = Some(c.score),
        Err(e) => o.errors.push(format!("pdf: {e}")),
    }
    (o, rasters)
}

fn quantile(mut v: Vec<f64>, q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    let i = ((v.len() - 1) as f64 * q).round() as usize;
    v[i.min(v.len() - 1)]
}

fn summary(results: &[Outcome]) -> Value {
    let stat = |get: &dyn Fn(&Outcome) -> Option<f64>| {
        let v: Vec<f64> = results.iter().filter_map(get).collect();
        json!({
            "count": v.len(),
            "median": round(quantile(v.clone(), 0.5)),
            "p95": round(quantile(v, 0.95)),
        })
    };
    json!({
        "figures": results.len(),
        "with_errors": results.iter().filter(|o| !o.errors.is_empty()).count(),
        "eps_mean": stat(&|o| o.eps.map(|s| s.mean)),
        "eps_strong": stat(&|o| o.eps.map(|s| s.strong)),
        "png_mean": stat(&|o| o.png.map(|s| s.mean)),
        "pdf_mean": stat(&|o| o.pdf.map(|s| s.mean)),
        "bbox_points": stat(&|o| o.bbox),
    })
}

/// The options xfig passes to fig2dev: `-F`, true font sizes (`u_print.c`).
fn xfig_args() -> Vec<String> {
    vec!["-F".to_string()]
}

fn round(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn cell(s: Option<compare::Score>) -> String {
    s.map_or("—".into(), |s| format!("{:.4}", s.mean))
}

fn report(results: &[Outcome], images: &[(usize, String)], sum: &Value, versions: &str) -> String {
    let mut h = String::from(
        "<!doctype html><meta charset=utf-8><title>fig2dev comparison</title><style>\
         body{font:14px system-ui,sans-serif;margin:16px;background:#fff;color:#111}\
         table{border-collapse:collapse}td,th{border:1px solid #ccc;padding:2px 6px;vertical-align:top}\
         td.n{text-align:right;font-variant-numeric:tabular-nums}img{max-width:300px;max-height:220px;border:1px solid #ddd}\
         .err{color:#a00;font-size:12px}</style>",
    );
    h.push_str("<h1>RustFig compared with fig2dev</h1>");
    h.push_str(&format!("<p>{}</p>", esc(versions)));
    h.push_str(
        "<p>Scores: mean grey difference after trimming, scaling and blurring (0 = identical). \
        EPS: both EPS files rasterised by ghostscript. PNG: RustFig's renderer against fig2dev's EPS. \
        PDF: both PDFs through ghostscript. BBox: largest difference of the bounding box's width or height in points. \
        Diff images: red only in fig2dev, blue only in RustFig.</p>",
    );
    h.push_str(&format!(
        "<pre>{}</pre>",
        esc(&serde_json::to_string_pretty(sum).unwrap_or_default())
    ));
    h.push_str("<table><tr><th>#</th><th>Figure</th><th>EPS</th><th>strong</th><th>PNG</th><th>PDF</th><th>BBox pt</th><th>Images / errors</th></tr>");
    for (rank, o) in results.iter().enumerate() {
        h.push_str(&format!(
            "<tr><td class=n>{}</td><td>{}</td><td class=n>{}</td><td class=n>{}</td><td class=n>{}</td><td class=n>{}</td><td class=n>{}</td><td>",
            rank + 1,
            esc(&o.name),
            cell(o.eps),
            o.eps.map_or("—".into(), |s| format!("{:.4}", s.strong)),
            cell(o.png),
            cell(o.pdf),
            o.bbox.map_or("—".into(), |b| format!("{b:.1}")),
        ));
        if let Some((_, dir)) = images.iter().find(|(i, _)| *i == rank) {
            for k in ["fig2dev", "rustfig", "diff"] {
                h.push_str(&format!("<img src=\"{dir}/{k}.png\" alt=\"{k}\" loading=lazy> "));
            }
        }
        for e in &o.errors {
            h.push_str(&format!("<div class=err>{}</div>", esc(e)));
        }
        h.push_str("</td></tr>");
    }
    h.push_str("</table>");
    h
}

#[test]
#[ignore = "slow: compares the whole sample corpus with fig2dev (A10)"]
fn corpus_matches_fig2dev() {
    let (Some(f2d), Some(gs)) = (fig2dev::find(), fig2dev::ghostscript()) else {
        eprintln!("fig2dev or ghostscript not installed; skipping the corpus comparison");
        return;
    };
    let figs = corpus();
    assert!(!figs.is_empty(), "no figures found");
    let versions = format!("{}; {}; {} figures", f2d.version, gs.version, figs.len());
    eprintln!("{versions}");

    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let results: Mutex<Vec<(usize, Outcome)>> = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(path) = figs.get(i) else { break };
                let (o, _) = run_one(path);
                results.lock().unwrap().push((i, o));
                let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                if d % 100 == 0 {
                    eprintln!("  {d}/{}", figs.len());
                }
            });
        }
    });
    let mut indexed = results.into_inner().unwrap();
    // Worst first: by the EPS score, figures with errors on top.
    indexed.sort_by(|(_, a), (_, b)| {
        let key = |o: &Outcome| (o.eps.is_none(), o.eps.map_or(0.0, |s| s.mean));
        let (ea, sa) = key(a);
        let (eb, sb) = key(b);
        eb.cmp(&ea).then(sb.total_cmp(&sa)).then(a.name.cmp(&b.name))
    });

    // Second pass: images of the worst figures.
    let out = out_dir();
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    let mut images = Vec::new();
    for (rank, (i, o)) in indexed.iter().enumerate().take(SHOWN) {
        if o.eps.is_none() {
            continue;
        }
        let Some(path) = figs.get(*i) else { continue };
        if let (_, Some(r)) = run_one(path) {
            if let Ok(c) = compare::compare(&r.reference, &r.eps) {
                let dir = format!("img{rank:04}");
                std::fs::create_dir_all(out.join(&dir)).unwrap();
                let thumb = |img: &image::GrayImage| {
                    let (w, h) = img.dimensions();
                    let s = 600.0 / f64::from(w.max(h).max(1));
                    image::imageops::resize(
                        img,
                        ((f64::from(w) * s) as u32).max(1),
                        ((f64::from(h) * s) as u32).max(1),
                        image::imageops::FilterType::Triangle,
                    )
                };
                std::fs::write(
                    out.join(&dir).join("fig2dev.png"),
                    compare::png(&thumb(&c.reference)).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    out.join(&dir).join("rustfig.png"),
                    compare::png(&thumb(&c.candidate)).unwrap(),
                )
                .unwrap();
                let diff = {
                    let (w, h) = c.diff.dimensions();
                    let s = 600.0 / f64::from(w.max(h).max(1));
                    image::imageops::resize(
                        &c.diff,
                        ((f64::from(w) * s) as u32).max(1),
                        ((f64::from(h) * s) as u32).max(1),
                        image::imageops::FilterType::Nearest,
                    )
                };
                std::fs::write(out.join(&dir).join("diff.png"), compare::png(&diff).unwrap()).unwrap();
                images.push((rank, dir));
            }
        }
    }
    let results: Vec<Outcome> = indexed.into_iter().map(|(_, o)| o).collect();
    let sum = summary(&results);
    std::fs::write(out.join("index.html"), report(&results, &images, &sum, &versions)).unwrap();
    eprintln!("{}", serde_json::to_string_pretty(&sum).unwrap());
    eprintln!("report: {}", out.join("index.html").display());

    let base_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/corpus-scores.json");
    if std::env::var_os("UPDATE_REFERENCE").is_some() {
        std::fs::write(&base_path, serde_json::to_string_pretty(&sum).unwrap() + "\n").unwrap();
        eprintln!("baseline written to {}", base_path.display());
        return;
    }
    let Some(base) = std::fs::read_to_string(&base_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
    else {
        eprintln!("no baseline yet (run with UPDATE_REFERENCE=1)");
        return;
    };
    if base["figures"] != sum["figures"] {
        eprintln!("corpus size differs from the baseline's; not checking for regressions");
        return;
    }
    let mut worse = Vec::new();
    for key in ["eps_mean", "eps_strong", "png_mean", "pdf_mean"] {
        for q in ["median", "p95"] {
            let (now, then) = (
                sum[key][q].as_f64().unwrap_or(0.0),
                base[key][q].as_f64().unwrap_or(0.0),
            );
            if now > then + MARGIN {
                worse.push(format!("{key} {q}: {now:.4} (baseline {then:.4})"));
            }
        }
    }
    if sum["with_errors"].as_u64() > base["with_errors"].as_u64() {
        worse.push(format!(
            "figures with errors: {} (baseline {})",
            sum["with_errors"], base["with_errors"]
        ));
    }
    assert!(
        worse.is_empty(),
        "the comparison with fig2dev got worse:\n{}",
        worse.join("\n")
    );
}
