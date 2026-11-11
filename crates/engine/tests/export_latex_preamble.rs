#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! F22: the figure's LaTeX preamble reaches fig2dev's output that C
//! fig2dev does not have (complete documents with `-P`, `asy`), as when
//! fig2dev reads the saved file; elsewhere it stays comment lines.

use rustfig_engine::Engine;
use serde_json::{json, Value};

const FIG: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
4 0 0 50 -1 0 12 0.0000 2 135 300 1200 1200 $x\\\\in\\\\R$\\001\n";

fn export(e: &mut Engine, format: &str, options: Value) -> String {
    let r = e
        .execute("file.export_data", &json!({"format": format, "options": options}))
        .unwrap();
    r["data"].as_str().unwrap().to_string()
}

#[test]
fn the_preamble_goes_into_complete_documents_and_asy() {
    let mut e = Engine::new();
    e.execute("file.open_data", &json!({"data": FIG})).unwrap();
    e.execute(
        "doc.set_latex_preamble",
        &json!({"preamble": "\\usepackage{amssymb}\n\\newcommand{\\R}{\\mathbb{R}}"}),
    )
    .unwrap();
    for lang in ["tikz", "pict2e", "pstricks"] {
        let out = export(&mut e, lang, json!({"P": true}));
        let doc = out.find("\\begin{document}").unwrap();
        assert!(
            out[..doc].contains("\n\\newcommand{\\R}{\\mathbb{R}}\n"),
            "{lang}: {out}"
        );
        // Without -P the lines are only the figure's comments.
        let out = export(&mut e, lang, json!({}));
        assert!(!out.contains("\n\\newcommand"), "{lang}: {out}");
        assert!(out.contains("RustFigLaTeXPreamble \\newcommand"), "{lang}: {out}");
    }
    let out = export(&mut e, "asy", json!({}));
    assert!(
        out.contains("texpreamble('\\\\newcommand{\\\\R}{\\\\mathbb{R}}');"),
        "{out}"
    );

    // As fig2dev on the saved file.
    let saved = e.execute("file.save_data", &json!({})).unwrap()["data"]
        .as_str()
        .unwrap()
        .to_string();
    let mut f = Engine::new();
    f.execute("file.open_data", &json!({"data": saved})).unwrap();
    assert_eq!(
        export(&mut f, "tikz", json!({"P": true})),
        export(&mut e, "tikz", json!({"P": true}))
    );
}
