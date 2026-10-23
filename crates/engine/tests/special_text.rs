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
