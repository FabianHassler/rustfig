//! A8: compares RustFig's rendering of each fixture with fig2dev's.
//!
//! fig2dev (xfig's converter) is the reference. It is optional: without it
//! the test prints a note and passes. Its SVG output is rasterised with
//! resvg, so ghostscript isn't needed. Each fixture's score is checked
//! against `fixtures/scores.json`; a run with `UPDATE_REFERENCE=1` rewrites
//! the baselines. The images go to `target/fig2dev-reference/<fixture>/`
//! (`fig2dev.png`, `rustfig.png`, `diff.png`: red = only fig2dev, blue =
//! only RustFig).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use rustfig_engine::model::Rgb;
use rustfig_engine::render::{export, ExportFormat, ExportOptions};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use support::{compare, fig2dev};

/// How much worse than its baseline a score may get.
const MARGIN: f64 = 0.02;
/// Longest side of the rasterised images, in pixels.
const SIDE: f64 = 1000.0;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn out_dir() -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    target.join("fig2dev-reference")
}

/// fig2dev's SVG, rasterised on white.
fn reference_png(fig: &[u8], dir: &Path) -> Vec<u8> {
    let svg = fig2dev::convert(fig, "svg", &[], Some(dir)).unwrap();
    let mut opt = resvg::usvg::Options::default();
    let db = opt.fontdb_mut();
    db.load_system_fonts();
    // fig2dev names PostScript families without a generic fallback; where
    // they are missing (Linux CI has URW's clones, fonts-urw-base35), fall
    // back to the first clone installed.
    let has = |db: &resvg::usvg::fontdb::Database, f: &str| db.faces().any(|x| x.families.iter().any(|(n, _)| n == f));
    let pick =
        |db: &resvg::usvg::fontdb::Database, names: &[&str]| names.iter().find(|n| has(db, n)).map(|n| n.to_string());
    if let Some(f) = pick(
        db,
        &[
            "Times",
            "Nimbus Roman",
            "Nimbus Roman No9 L",
            "Liberation Serif",
            "DejaVu Serif",
        ],
    ) {
        db.set_serif_family(f);
    }
    if let Some(f) = pick(
        db,
        &[
            "Helvetica",
            "Nimbus Sans",
            "Nimbus Sans L",
            "Liberation Sans",
            "DejaVu Sans",
        ],
    ) {
        db.set_sans_serif_family(f);
    }
    if let Some(f) = pick(
        db,
        &[
            "Courier",
            "Nimbus Mono PS",
            "Nimbus Mono L",
            "Liberation Mono",
            "DejaVu Sans Mono",
        ],
    ) {
        db.set_monospace_family(f);
    }
    let svg = String::from_utf8(svg)
        .unwrap()
        .replace("font-family=\"Times\"", "font-family=\"Times,serif\"")
        .replace("font-family=\"Times,Symbol\"", "font-family=\"Times,Symbol,serif\"")
        .replace("font-family=\"Helvetica\"", "font-family=\"Helvetica,sans-serif\"")
        .replace(
            "font-family=\"AvantGarde\"",
            "font-family=\"AvantGarde,URW Gothic,sans-serif\"",
        )
        .replace("font-family=\"Courier\"", "font-family=\"Courier,monospace\"");
    let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &opt).unwrap();
    let size = tree.size();
    let s = SIDE / f64::from(size.width().max(size.height()));
    let (w, h) = (
        (f64::from(size.width()) * s).ceil() as u32,
        (f64::from(size.height()) * s).ceil() as u32,
    );
    let mut pix = resvg::tiny_skia::Pixmap::new(w.max(1), h.max(1)).unwrap();
    pix.fill(resvg::tiny_skia::Color::WHITE);
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(s as f32, s as f32),
        &mut pix.as_mut(),
    );
    pix.encode_png().unwrap()
}

/// RustFig's PNG export at a similar size, on white.
fn rustfig_png(fig: &[u8]) -> Vec<u8> {
    let doc = rustfig_engine::format::read_fig(fig).unwrap();
    let opts = ExportOptions {
        dpi: 150.0,
        background: Some(Rgb::from_hex("#ffffff").unwrap()),
        ..Default::default()
    };
    export(&doc, ExportFormat::Png, &opts).unwrap()
}

#[test]
fn fixtures_match_fig2dev() {
    let Some(tool) = fig2dev::find() else {
        eprintln!("fig2dev not installed; skipping the reference comparison");
        return;
    };
    let update = std::env::var_os("UPDATE_REFERENCE").is_some();
    let scores_path = fixtures().join("scores.json");
    let baseline: Map<String, Value> = std::fs::read_to_string(&scores_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut names: Vec<String> = std::fs::read_dir(fixtures())
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter_map(|n| n.strip_suffix(".fig").map(str::to_string))
        .collect();
    names.sort();
    assert!(!names.is_empty());

    let mut scores = Map::new();
    let mut failures = Vec::new();
    eprintln!("{}; images in {}", tool.version, out_dir().display());
    for name in &names {
        let fig = std::fs::read(fixtures().join(format!("{name}.fig"))).unwrap();
        let c = compare::compare(&reference_png(&fig, &fixtures()), &rustfig_png(&fig)).unwrap();
        let dir = out_dir().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("fig2dev.png"), compare::png(&c.reference).unwrap()).unwrap();
        std::fs::write(dir.join("rustfig.png"), compare::png(&c.candidate).unwrap()).unwrap();
        std::fs::write(dir.join("diff.png"), compare::png(&c.diff).unwrap()).unwrap();
        let (mean, strong) = (round(c.score.mean), round(c.score.strong));
        eprintln!("  {name:10} mean {mean:.4}  strong {strong:.4}");
        scores.insert(name.clone(), json!({"mean": mean, "strong": strong}));
        match baseline.get(name) {
            Some(b) => {
                let bm = b["mean"].as_f64().unwrap_or(0.0);
                let bs = b["strong"].as_f64().unwrap_or(0.0);
                if mean > bm + MARGIN || strong > bs + MARGIN {
                    failures.push(format!(
                        "{name}: mean {mean:.4} (baseline {bm:.4}), strong {strong:.4} (baseline {bs:.4})"
                    ));
                }
            }
            None if !update => failures.push(format!("{name}: no baseline (run with UPDATE_REFERENCE=1)")),
            None => {}
        }
    }
    if update {
        let text = serde_json::to_string_pretty(&Value::Object(scores)).unwrap();
        std::fs::write(&scores_path, text + "\n").unwrap();
        eprintln!("baselines written to {}", scores_path.display());
        return;
    }
    assert!(
        failures.is_empty(),
        "renderings got worse than their baselines:\n{}",
        failures.join("\n")
    );
}

fn round(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}
