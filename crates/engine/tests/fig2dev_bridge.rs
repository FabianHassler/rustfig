//! fig2dev's reader and drivers in the engine: old Fig files open (G1,
//! S3), the ported drivers export (B9), pstex/pdftex write their two
//! parts (B2), EPS pictures are embedded in EPS exports.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::Engine;
use serde_json::{json, Value};
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fig2dev-bridge-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A Fig 2.1 file (80 ppi): a box, a dashed red line with an arrow, an
/// approximated spline and a PostScript text.
const FIG21: &str = "#FIG 2.1\n80 2\n\
    2 2 0 1 -1 0 0 0 0.000 7 0 0\n\t 10 10 90 10 90 50 10 50 10 10 9999 9999\n\
    2 1 1 2 4 0 0 0 4.000 -1 1 0\n\t0 0 1.000 4.000 8.000\n\t 10 70 50 70 9999 9999\n\
    3 0 0 1 -1 0 0 0 0.000 0 0\n\t 10 100 50 120 90 100 9999 9999\n\
    4 0 -1 0 0 0 12 0.00000 4 9 30 10 150 Hello\x01\n";

/// A Fig 1.3 file: a header of numbers, an open spline.
const FIG13: &str = "80 2 9 9\n3 10 0 1 1 0 0 0 0 0 0\n10 10 40 0 9999 0\n";

fn types(e: &Engine) -> Vec<&'static str> {
    e.doc().objects.iter().map(|o| o.type_name()).collect()
}

#[test]
fn fig_2_1_and_1_3_files_open() {
    let dir = temp_dir("old");
    let p21 = dir.join("old21.fig");
    std::fs::write(&p21, FIG21).unwrap();
    let mut e = Engine::new();
    let r = e.execute("file.open", &json!({"path": p21})).unwrap();
    assert_eq!(r["objects"], 4);
    assert_eq!(types(&e), ["box", "polyline", "spline", "text"]);
    let x = e.doc_bounds().max.x;
    assert!((1350.0..1370.0).contains(&x), "80 ppi scaled to 1200: {x}");
    // Saved as Fig 3.2, it reads back with RustFig's own reader.
    let saved = dir.join("saved.fig");
    e.execute("file.save", &json!({"path": saved})).unwrap();
    let text = std::fs::read_to_string(&saved).unwrap();
    assert!(text.starts_with("#FIG 3.2"), "{text}");
    let mut e2 = Engine::new();
    e2.execute("file.open", &json!({"path": saved})).unwrap();
    assert_eq!(types(&e2), types(&e));

    let p13 = dir.join("old13.fig");
    std::fs::write(&p13, FIG13).unwrap();
    let r = e.execute("file.open", &json!({"path": p13})).unwrap();
    assert_eq!(r["objects"], 1);
    assert_eq!(types(&e), ["spline"]);

    // And from data, with fig2dev's repair messages as warnings.
    let open_box = "#FIG 2.1\n80 2\n2 2 0 1 -1 0 0 0 0.000 7 0 0\n\t 10 10 90 10 90 50 10 50 9999 9999\n";
    let r = e.execute("file.open_data", &json!({"data": open_box})).unwrap();
    assert_eq!(r["objects"], 1);
    let w = r["warnings"].as_array().unwrap();
    assert!(w.iter().any(|m| m.as_str().unwrap().contains("open rectangle")), "{r}");
    e.execute("file.merge_data", &json!({"data": FIG21})).unwrap();
    assert_eq!(e.doc().objects.len(), 5);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn bad_old_files_are_errors_and_change_nothing() {
    let mut e = Engine::new();
    e.execute("file.open_data", &json!({"data": FIG21})).unwrap();
    for bad in [
        "#FIG 2.1\n",
        "#FIG 2.1\n80 2\n2 9 9\n",
        "12 x\n",
        "#FIG 1.4\n80 2\n7 1 2 3\n",
    ] {
        let before = e.doc().clone();
        assert!(e.execute("file.open_data", &json!({"data": bad})).is_err(), "{bad:?}");
        assert!(e.execute("file.merge_data", &json!({"data": bad})).is_err(), "{bad:?}");
        assert_eq!(*e.doc(), before);
    }
}

/// A box, a PostScript text and a special (LaTeX) text.
const DRAWING: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
    2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 2400 0 2400 1200 0 1200 0 0\n\
    4 0 0 50 -1 0 12 0.0000 4 135 450 300 600 plain\\001\n\
    4 0 0 50 -1 0 12 0.0000 2 135 450 300 900 $\\alpha^2$\\001\n";

fn drawing() -> Engine {
    let mut e = Engine::new();
    e.execute("file.open_data", &json!({"data": DRAWING})).unwrap();
    e
}

fn special_count(e: &Engine) -> usize {
    let mut n = 0;
    e.doc().for_each_leaf(&mut |o| {
        if let rustfig_model::ObjectKind::Text(t) = &o.kind {
            n += usize::from(t.is_special());
        }
    });
    n
}

#[test]
fn ported_drivers_export_and_are_listed() {
    let mut e = drawing();
    let v = e.execute("file.export_formats", &json!({})).unwrap();
    let latex = v["formats"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "latex")
        .unwrap()
        .clone();
    assert_eq!(latex["available"], true, "{latex}");
    assert_eq!(latex["status"], "available");
    let letters: Vec<&str> = latex["options"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| o["available"] == true)
        .map(|o| o["letter"].as_str().unwrap())
        .collect();
    assert!(letters.contains(&"v") && letters.contains(&"m"), "{letters:?}");

    let r = e.execute("file.export_data", &json!({"format": "latex"})).unwrap();
    let text = r["data"].as_str().unwrap();
    assert!(
        text.contains("\\begin{picture}") && text.contains("\\end{picture}"),
        "{text}"
    );
    // Options by letter reach the driver; general ones by key too.
    let verbose = e
        .execute("file.export_data", &json!({"format": "latex", "options": {"v": true}}))
        .unwrap();
    assert_ne!(verbose["data"], r["data"]);
    let big = e
        .execute(
            "file.export_data",
            &json!({"format": "latex", "options": {"magnification": 2}}),
        )
        .unwrap();
    assert_ne!(big["data"], r["data"]);
    let pct = e
        .execute("file.export_data", &json!({"format": "latex", "magnification": 200}))
        .unwrap();
    assert_eq!(pct["data"], big["data"]);
    // Unknown letters and keys are refused.
    for bad in [json!({"q": true}), json!({"nonsense": 1}), json!({"v": [1]})] {
        let err = e
            .execute("file.export_data", &json!({"format": "latex", "options": bad}))
            .unwrap_err();
        assert!(err.message.contains("option"), "{}", err.message);
    }

    // Every language the crate writes exports through file.export_data.
    for f in v["formats"].as_array().unwrap() {
        let id = f["id"].as_str().unwrap();
        if f["available"] == true {
            // A driver may refuse a figure (shape wants comments), as fig2dev does.
            let r = e.execute("file.export_data", &json!({"format": id}));
            assert!(
                r.as_ref().map_or_else(|err| err.code == "format", |_| true),
                "{id}: {r:?}"
            );
        }
    }
}

#[test]
fn pstex_writes_graphics_without_special_text_and_the_latex_part() {
    let mut e = drawing();
    assert_eq!(special_count(&e), 1);
    let dir = temp_dir("pstex");
    let path = dir.join("fig.pstex").to_string_lossy().into_owned();
    let r = e
        .execute("file.export", &json!({"path": path, "format": "pstex"}))
        .unwrap();
    let paths: Vec<&str> = r["paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert_eq!(paths, [path.clone(), format!("{path}_t")]);
    let eps = std::fs::read_to_string(&path).unwrap();
    assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0"), "{eps}");
    assert!(eps.contains("(plain)") && !eps.contains("alpha"), "{eps}");
    let tex = std::fs::read_to_string(format!("{path}_t")).unwrap();
    assert!(tex.contains("\\includegraphics{fig.pstex}"), "{tex}");
    assert!(tex.contains("$\\alpha^2$") && !tex.contains("plain"), "{tex}");

    // The EPS and the overlay have the same size.
    let bb: Vec<f64> = eps
        .lines()
        .find(|l| l.starts_with("%%BoundingBox:"))
        .unwrap()
        .split_whitespace()
        .skip(1)
        .map(|n| n.parse().unwrap())
        .collect();
    assert!(bb.len() == 4 && bb[2] > 0.0, "{eps}");
    assert!(
        tex.lines().filter(|l| l.starts_with("\\begin{picture}(")).count() == 2,
        "{tex}"
    );

    // pdftex: NAME.pdf and NAME.pdf_t.
    let pdf = dir.join("fig.pdf").to_string_lossy().into_owned();
    e.execute("file.export", &json!({"path": pdf, "format": "pdftex"}))
        .unwrap();
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
    let tex = std::fs::read_to_string(format!("{pdf}_t")).unwrap();
    assert!(tex.contains("\\includegraphics{fig.pdf}"), "{tex}");

    // pspdftex: NAME_pspdftex.eps, .pdf and the LaTeX part at PATH.
    let both = dir.join("fig.pspdftex").to_string_lossy().into_owned();
    let r = e
        .execute(
            "file.export",
            &json!({"path": both, "format": "pspdftex", "options": {"border": 5}}),
        )
        .unwrap();
    assert_eq!(r["paths"].as_array().unwrap().len(), 3, "{r}");
    assert!(dir.join("fig_pspdftex.eps").is_file() && dir.join("fig_pspdftex.pdf").is_file());
    let tex = std::fs::read_to_string(&both).unwrap();
    assert!(tex.contains("\\includegraphics{fig_pspdftex}"), "{tex}");

    // export_data returns every part.
    let r = e.execute("file.export_data", &json!({"format": "pstex"})).unwrap();
    let files = r["files"].as_array().unwrap();
    let ext: Vec<&str> = files.iter().map(|f| f["extension"].as_str().unwrap()).collect();
    assert_eq!(ext, ["pstex", "pstex_t"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn eps_pictures_are_embedded_in_eps() {
    let dir = temp_dir("embed");
    std::fs::write(
        dir.join("inner.eps"),
        "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 10 20 110 70\nnewpath 10 20 moveto 110 70 lineto stroke\n%%EOF\n",
    )
    .unwrap();
    let fig = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
        2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5\n\t0 inner.eps\n\t 0 0 2400 0 2400 1200 0 1200 0 0\n";
    let path = dir.join("withpic.fig");
    std::fs::write(&path, fig).unwrap();
    let mut e = Engine::new();
    let r = e.execute("file.open", &json!({"path": path})).unwrap();
    let w: Vec<Value> = r["warnings"].as_array().unwrap().clone();
    assert!(w.iter().any(|m| m.as_str().unwrap().contains("EPS picture")), "{r}");
    let eps = e.execute("file.export_data", &json!({"format": "eps"})).unwrap();
    let eps = eps["data"].as_str().unwrap();
    assert!(eps.contains("% Begin Imported EPS File: inner.eps"), "{eps}");
    assert!(eps.contains("110 70 lineto stroke"), "{eps}");
    assert!(eps.contains("/showpage {} def"), "{eps}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn compressed_and_xbm_pictures_load() {
    let dir = temp_dir("pics");
    std::fs::write(
        dir.join("dot.xbm"),
        "#define dot_width 8\n#define dot_height 2\nstatic char dot_bits[] = {\n 0x01, 0x80 };\n",
    )
    .unwrap();
    let fig = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
        2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5\n\t0 dot.xbm\n\t 0 0 2400 0 2400 1200 0 1200 0 0\n";
    let path = dir.join("xbm.fig");
    std::fs::write(&path, fig).unwrap();
    let mut e = Engine::new();
    let r = e.execute("file.open", &json!({"path": path})).unwrap();
    assert_eq!(r["warnings"].as_array().unwrap().len(), 0, "{r}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// D2: the frame the engine fits and reports is the frame fig2dev's EPS
/// driver exports (points: 72/1200 of a Fig unit, rounded outward).
#[test]
fn doc_bounds_are_the_eps_frame() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut n = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().is_none_or(|x| x != "fig") {
            continue;
        }
        let mut e = Engine::new();
        e.execute("file.open", &json!({"path": p})).unwrap();
        let r = e
            .execute("file.export_data", &json!({"format": "eps", "options": {}}))
            .unwrap();
        let eps = r["data"].as_str().unwrap();
        let line = eps.lines().find(|l| l.starts_with("%%BoundingBox:")).unwrap();
        let v: Vec<f64> = line.split_whitespace().skip(1).map(|n| n.parse().unwrap()).collect();
        let b = e.doc_bounds();
        // Metric drawings have 450 units per cm, 1143 per inch.
        let k = if e.doc().settings.units == rustfig_model::Units::Metric {
            72.0 / 1143.0
        } else {
            0.06
        };
        let (w, h) = ((b.max.x - b.min.x) * k, (b.max.y - b.min.y) * k);
        // fig2dev rounds the corners outward; the extent differs by under 2 points.
        assert!(
            (v[2] - v[0] - w).abs() < 2.0,
            "{}: width {} vs {w}",
            p.display(),
            v[2] - v[0]
        );
        assert!(
            (v[3] - v[1] - h).abs() < 2.0,
            "{}: height {} vs {h}",
            p.display(),
            v[3] - v[1]
        );
        n += 1;
    }
    assert!(n >= 10);
}

/// D2: arrowheads count, and the selection box is fig2dev's.
#[test]
fn arrowheads_count_in_the_bounds() {
    let mut e = Engine::new();
    e.execute("shape.polyline", &json!({"points": [[0, 0], [1200, 0]]}))
        .unwrap();
    let plain = e.doc_bounds();
    e.execute("edit.select_all", &json!({})).unwrap();
    e.execute("object.set_attributes", &json!({"forward_arrow": true}))
        .unwrap();
    let arrowed = e.doc_bounds();
    assert!(
        arrowed.max.x > plain.max.x || arrowed.max.y > plain.max.y,
        "{plain:?} {arrowed:?}"
    );
    assert_eq!(e.selection_bbox(), arrowed);
}
