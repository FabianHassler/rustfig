#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! F2: a LaTeX special text is raw source on screen and in latex-type exports.

use rustfig_engine::Engine;
use serde_json::json;

const DRAWING: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
    4 0 0 50 -1 0 12 0.0000 4 135 450 300 600 plain\\001\n\
    4 0 0 50 -1 0 12 0.0000 2 135 450 300 900 $\\\\alpha^2$\\001\n";

fn drawing() -> Engine {
    let mut e = Engine::new();
    e.execute("file.open_data", &json!({"data": DRAWING})).unwrap();
    e
}

/// Shown as its raw source (as xfig does), the flag kept on the primitive.
#[test]
fn special_text_is_raw_source_on_screen() {
    let e = drawing();
    let scene = rustfig_render::build_scene(e.doc(), &rustfig_render::SceneOptions::default());
    let texts: Vec<_> = scene
        .prims
        .iter()
        .filter_map(|p| match p {
            rustfig_render::Prim::Text(t) => Some(t),
            _ => None,
        })
        .collect();
    let s = texts.iter().find(|t| t.special).expect("special text");
    assert_eq!(s.text, "$\\alpha^2$");
    assert!(texts.iter().filter(|t| !t.special).all(|t| t.text == "plain"));
}

/// Latex-type exports get the special text raw, not escaped.
#[test]
fn special_text_reaches_latex_outputs_raw() {
    let mut e = drawing();
    for format in ["latex", "pstex", "pdftex", "pict2e", "tikz", "epic", "eepic"] {
        let Ok(r) = e.execute("file.export_data", &json!({"format": format})) else {
            continue;
        };
        let mut all = String::new();
        if let Some(d) = r["data"].as_str() {
            all.push_str(d);
        }
        for f in r["files"].as_array().into_iter().flatten() {
            if let Some(d) = f["data"].as_str() {
                all.push_str(d);
            }
        }
        assert!(all.contains("$\\alpha^2$"), "{format}: {all}");
        assert!(!all.contains("\\textbackslash"), "{format}: {all}");
    }
}

/// `typeset_latex`: PDF, EPS, SVG and PNG get the special text as TeX
/// typeset it (a vector picture), the plain text stays text. Skipped
/// without TeX.
#[test]
fn exports_typeset_special_text_with_tex() {
    let e = drawing();
    if e.latex.export_typesetter().is_none() {
        eprintln!("no TeX installed: skipped");
        return;
    }
    let dir = std::env::temp_dir().join(format!("rf-typeset-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut e = e;
    for (ext, typeset_marker, text_marker) in [
        ("pdf", "/Form", ""),
        ("eps", "% Begin typeset text", "alpha"),
        ("svg", "class=\"rustfig-latex\"", ""),
    ] {
        let path = dir.join(format!("t.{ext}"));
        let p = path.to_str().unwrap();
        e.execute("file.export", &json!({"path": p, "typeset_latex": true}))
            .unwrap();
        let with = String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into_owned();
        e.execute("file.export", &json!({"path": p})).unwrap();
        let without = String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into_owned();
        assert!(
            with.matches(typeset_marker).count() > without.matches(typeset_marker).count(),
            "{ext}: the special text is typeset"
        );
        if !text_marker.is_empty() {
            assert!(
                without.contains(text_marker) && !with.contains(text_marker),
                "{ext}: no raw source"
            );
        }
    }
    // The bitmap differs where the text is.
    let png = dir.join("t.png");
    let p = png.to_str().unwrap();
    e.execute("file.export", &json!({"path": p, "typeset_latex": true}))
        .unwrap();
    let with = std::fs::read(&png).unwrap();
    e.execute("file.export", &json!({"path": p})).unwrap();
    assert_ne!(with, std::fs::read(&png).unwrap());
    let _ = std::fs::remove_dir_all(&dir);
}

/// LaTeX texts typeset into a PDF or SVG come back as LaTeX texts when the
/// file is opened (from the source the export keeps), not as glyph outlines.
#[test]
fn typeset_exports_import_their_latex_texts() {
    let fig = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
        2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 6000 0 6000 4000 0 4000 0 0\n\
        4 0 4 50 -1 0 14 0.0000 2 135 450 1200 1200 $\\\\alpha^2+\\\\int_0^1 f$\\001\n\
        4 1 0 50 -1 0 12 0.5236 2 135 450 3000 3000 $\\\\pi/2$\\001\n";
    let mut e = Engine::new();
    e.execute("file.open_data", &json!({"data": fig})).unwrap();
    if e.latex.export_typesetter().is_none() {
        eprintln!("no TeX installed: skipped");
        return;
    }
    let dir = std::env::temp_dir().join(format!("rf-typeset-back-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for ext in ["pdf", "svg"] {
        let path = dir.join(format!("t.{ext}"));
        let p = path.to_str().unwrap();
        e.execute("file.export", &json!({"path": p, "typeset_latex": true}))
            .unwrap();
        let mut back = Engine::new();
        back.execute("file.open", &json!({"path": p, "vectors": true})).unwrap();
        let texts: Vec<_> = back
            .doc()
            .objects
            .iter()
            .filter_map(|o| match &o.kind {
                rustfig_model::ObjectKind::Text(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        let special: Vec<_> = texts.iter().filter(|t| t.is_special()).collect();
        assert_eq!(special.len(), 2, "{ext}: both LaTeX texts: {texts:?}");
        assert_eq!(texts.len(), 2, "{ext}: no glyphs left as texts");
        let alpha = special.iter().find(|t| t.text.contains("alpha")).unwrap();
        let pi = special.iter().find(|t| t.text == "$\\pi/2$").unwrap();
        // Positions relative to the frame (the import may move the drawing).
        let frame = back.doc().objects.iter().find_map(|o| match &o.kind {
            rustfig_model::ObjectKind::Polyline(p) => p.points.first().copied(),
            _ => None,
        });
        let polys = back
            .doc()
            .objects
            .iter()
            .filter(|o| matches!(o.kind, rustfig_model::ObjectKind::Polyline(_)))
            .count();
        assert!(polys <= 2, "{ext}: the glyph outlines are gone ({polys} polylines)");
        let f = frame.unwrap();
        let rel = |t: &rustfig_model::Text| (t.origin.x - f.x, t.origin.y - f.y);
        let (ax, ay) = rel(alpha);
        assert!(
            (ax - 1200.0).abs() < 30.0 && (ay - 1200.0).abs() < 30.0,
            "{ext}: alpha at {ax},{ay}"
        );
        let (px, py) = rel(pi);
        assert!(
            (px - 3000.0).abs() < 30.0 && (py - 3000.0).abs() < 30.0,
            "{ext}: pi at {px},{py}"
        );
        assert!(
            (pi.angle - std::f64::consts::FRAC_PI_6).abs() < 0.01,
            "{ext}: angle {}",
            pi.angle
        );
        assert_eq!(pi.justify, rustfig_model::Justify::Center);
        assert!((alpha.size - 14.0).abs() < 0.2, "{ext}: size {}", alpha.size);
        assert_eq!(alpha.font, 0);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
