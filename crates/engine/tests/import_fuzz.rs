#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Opening or merging arbitrary data must never panic.

use proptest::prelude::*;
use rustfig_engine::Engine;
use serde_json::json;

/// A PDF, an SVG and a PNG export with the figure embedded.
fn embedded_bases() -> &'static Vec<Vec<u8>> {
    static BASES: std::sync::OnceLock<Vec<Vec<u8>>> = std::sync::OnceLock::new();
    BASES.get_or_init(|| {
        let mut e = Engine::new();
        e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
            .unwrap();
        ["pdf", "svg", "png"]
            .iter()
            .map(|f| {
                let dir = std::env::temp_dir().join(format!("rustfig-e16-fuzz-{}", std::process::id()));
                std::fs::create_dir_all(&dir).unwrap();
                let path = dir.join(format!("f.{f}"));
                e.execute("file.export", &json!({"path": path.to_str().unwrap(), "embed": true}))
                    .unwrap();
                std::fs::read(path).unwrap()
            })
            .collect()
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn open_and_merge_random(data in "(#FIG 3\\.2\n)?[ -~\n]{0,600}") {
        let mut e = Engine::new();
        let _ = e.execute("file.open_data", &json!({"data": data}));
        let _ = e.execute("file.merge_data", &json!({"data": data}));
        let _ = e.execute("file.export_data", &json!({"format": "svg"}));
    }

    /// Fig 1.3 to 2.1 go through fig2dev's reader (G1).
    #[test]
    fn open_old_formats_random(
        head in "(#FIG 2\\.1\n80 2\n|#FIG 2\\.0\n80 2\n|#FIG 1\\.4\n1200 2\n|80 2 9 9\n)",
        body in "([0-9 \\-]{0,40}\n|9999 9999\n|4 0 -1 0 0 0 12 0\\.0 4 9 30 10 150 [ -~]{0,10}\\x01\n){0,30}",
    ) {
        let data = format!("{head}{body}");
        let mut e = Engine::new();
        let before = e.doc().clone();
        match e.execute("file.open_data", &json!({"data": data})) {
            Ok(_) => {
                let _ = e.execute("file.save_data", &json!({"format": "fig"}));
                let _ = e.execute("file.export_data", &json!({"format": "eps"}));
                let _ = e.execute("file.export_data", &json!({"format": "latex"}));
            }
            Err(_) => prop_assert_eq!(e.doc(), &before),
        }
        let _ = e.execute("file.merge_data", &json!({"data": data}));
    }

    /// PDF and EPS pictures (B15): hostile bytes after the signatures never
    /// panic, on the canvas or in an export.
    #[test]
    fn hostile_pdf_and_eps_pictures(
        head in "(%PDF-1\\.[0-7]\n|%!PS-Adobe-3\\.0 EPSF-3\\.0\n%%BoundingBox: [0-9 ]{0,12}\n)",
        body in proptest::collection::vec(any::<u8>(), 0..400),
    ) {
        let dir = std::env::temp_dir().join(format!("rustfig-b15-fuzz-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(&body);
        let file = dir.join(if bytes.starts_with(b"%PDF") { "h.pdf" } else { "h.eps" });
        std::fs::write(&file, &bytes).unwrap();
        let _ = rustfig_engine::import::load_picture(&file);
        let fig = format!(
            "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
             2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5\n\t0 {}\n\t 0 0 2400 0 2400 1200 0 1200 0 0\n",
            file.display()
        );
        let mut e = Engine::new();
        if e.execute("file.open_data", &json!({"data": fig})).is_ok() {
            for f in ["pdf", "eps", "png", "svg"] {
                let _ = e.execute("file.export_data", &json!({"format": f}));
            }
        }
    }

    /// G5: hostile PDF content streams and damaged PDFs are errors or small
    /// figures, never panics, and the figure they make saves.
    #[test]
    fn pdf_import_hostile(
        ops in "([-0-9.e ]{0,12} (m|l|c|re|h|f|S|B|q|Q|cm|w|d|g|rg|RG|n|W|Do|BT|ET|Tj|Tf)\n){0,40}",
        cut in 0usize..400,
        flip in 0usize..400,
        byte in any::<u8>(),
    ) {
        let content = format!("1 0 0 1 0 0 cm {ops}");
        let mut pdf = format!(
            "%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R >>\nendobj\n\
             4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n",
            content.len()
        )
        .into_bytes();
        let at = flip % pdf.len();
        if flip % 3 == 0 {
            pdf[at] = byte;
        }
        pdf.truncate(pdf.len().saturating_sub(cut % 40));
        let mut e = Engine::new();
        let before = e.doc().clone();
        match rustfig_engine::import::pdf::import(&pdf, 1, "fuzz") {
            Ok(_) => {
                let _ = e.execute("file.open_data", &json!({"base64": rustfig_engine::base64(&pdf), "vectors": true}));
                let _ = e.execute("file.save_data", &json!({"format": "fig"}));
                let _ = e.execute("file.merge_data", &json!({"base64": rustfig_engine::base64(&pdf), "vectors": true}));
            }
            Err(_) => {
                let r = e.execute("file.open_data", &json!({"base64": rustfig_engine::base64(&pdf), "vectors": true}));
                if r.is_err() {
                    prop_assert_eq!(e.doc(), &before);
                }
            }
        }
    }

    /// E16: damaged or hostile embedded figures (PDF, SVG, PNG) never panic.
    #[test]
    fn embedded_figure_damaged(
        which in 0usize..3,
        cut in 0usize..100_000,
        pos in 0usize..100_000,
        byte in any::<u8>(),
        junk in "[ -~]{0,200}",
    ) {
        let bases = embedded_bases();
        let base = &bases[which];
        let mut data = base.clone();
        let at = pos % data.len();
        data[at] = byte;
        let _ = rustfig_format::read_any(&data);
        data.truncate(cut % (data.len() + 1));
        let _ = rustfig_format::read_any(&data);
        data.extend_from_slice(junk.as_bytes());
        let _ = rustfig_format::read_any(&data);
        let marked = format!("%PDF-1.4 /Subtype /application#2Fx-xfig /Length {cut} stream\n{junk}");
        let _ = rustfig_format::read_any(marked.as_bytes());
        let svg = format!("<svg><rustfig:fig encoding=\"base64\">{junk}</rustfig:fig>");
        let _ = rustfig_format::read_any(svg.as_bytes());
    }

    #[test]
    fn raw_object_json(s in "[ -~]{0,300}") {
        let mut e = Engine::new();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            let _ = e.execute("object.add", &json!({"object": v}));
        }
        let _ = rustfig_engine::import::decode_picture(s.as_bytes());
    }
}

#[test]
fn exports_of_every_format() {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1200, "height": 600, "area_fill": 50}),
    )
    .unwrap();
    e.execute("shape.text", &json!({"x": 100, "y": 300, "text": "Grüße (x)"}))
        .unwrap();
    for f in ["svg", "eps", "pdf", "png"] {
        let r = e.execute("file.export_data", &json!({"format": f})).unwrap();
        assert!(r.get("data").or(r.get("base64")).is_some(), "{f}");
    }
}
