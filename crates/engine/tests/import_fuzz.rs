#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Opening or merging arbitrary data must never panic.

use proptest::prelude::*;
use rustfig_engine::Engine;
use serde_json::json;

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
