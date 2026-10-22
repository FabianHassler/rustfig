//! W10-4, W10-11: the Export panel's options as fig2dev's options, kept per
//! language; W10-8, W10-9: new files and `.bak`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::export_panel::{params, rows, PanelSettings};
use rustfig_engine::Engine;
use serde_json::{json, Value};

fn drawing() -> Engine {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 2400, "height": 1200}),
    )
    .unwrap();
    e
}

fn text(e: &mut Engine, p: Value) -> String {
    let v = e.execute("file.export_data", &p).unwrap();
    v["data"].as_str().unwrap_or_default().to_string()
}

fn bbox(eps: &str) -> Vec<i64> {
    let l = eps.lines().find(|l| l.starts_with("%%BoundingBox:")).unwrap();
    l.split_whitespace().skip(1).map(|n| n.parse().unwrap()).collect()
}

fn set(e: &mut Engine, format: &str, s: Value) {
    e.execute("export.settings", &json!({"format": format, "settings": s}))
        .unwrap();
}

#[test]
fn settings_are_kept_per_language_and_reset() {
    let mut e = Engine::new();
    let v = e.execute("export.settings", &json!({"format": "eps"})).unwrap();
    assert_eq!(v["settings"]["border"], 0.0);
    assert_eq!(v["rows"]["preview"], true);
    set(&mut e, "eps", json!({"border": 10, "preview": 1}));
    set(&mut e, "png", json!({"border": 3, "dpi": 200}));
    assert_eq!(
        e.execute("export.settings", &json!({"format": "eps"})).unwrap()["settings"]["border"],
        10.0
    );
    assert_eq!(
        e.execute("export.settings", &json!({"format": "png"})).unwrap()["settings"]["dpi"],
        200.0
    );
    // Through the settings file, not the drawing.
    assert!(e.prefs_dirty);
    let toml = e.prefs.to_toml();
    assert!(toml.contains("[rustfig.export.eps]"), "{toml}");
    let (back, warnings) = rustfig_engine::prefs::Prefs::parse(&toml).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(back.rustfig.export, e.prefs.rustfig.export);
    let fig = rustfig_format::write(e.doc(), rustfig_format::FileFormat::Fig).unwrap();
    assert!(!fig.contains("border"));
    // Reset to xfig's defaults.
    let v = e
        .execute("export.settings", &json!({"format": "png", "reset": true}))
        .unwrap();
    assert_eq!(v["settings"]["border"], 0.0);
    assert_eq!(v["settings"]["smoothing"], 1);
    // Bad values are errors and change nothing.
    assert!(e
        .execute("export.settings", &json!({"format": "eps", "settings": {"nope": 1}}))
        .is_err());
    assert!(e
        .execute(
            "export.settings",
            &json!({"format": "eps", "settings": {"border": "x"}})
        )
        .is_err());
    assert!(e.execute("export.settings", &json!({"format": "nope"})).is_err());
    assert_eq!(
        e.execute("export.settings", &json!({"format": "eps"})).unwrap()["settings"]["border"],
        10.0
    );
}

#[test]
fn rows_follow_xfigs_manage_optional() {
    let mut e = Engine::new();
    for (id, row, shown) in [
        ("gif", "transparent", true),
        ("png", "transparent", false),
        ("jpeg", "quality", true),
        ("eps", "preview", true),
        ("pdf", "preview", false),
        ("pdf", "pdf_page_mode", true),
        ("ps", "multiple", true),
        ("eps", "multiple", false),
        ("svg", "border", false),
        ("tk", "background", true),
        ("tk", "grid", false),
        ("png", "bitmap", true),
    ] {
        let v = e.execute("export.settings", &json!({"format": id})).unwrap();
        assert_eq!(v["rows"][row], shown, "{id} {row}");
        assert_eq!(rows(id).border, v["rows"]["border"] == true);
    }
}

#[test]
fn eps_options_reach_fig2dev() {
    let mut e = drawing();
    let plain = text(&mut e, json!({"format": "eps"}));
    let b0 = bbox(&plain);
    set(&mut e, "eps", json!({"border": 10}));
    let bordered = text(&mut e, json!({"format": "eps", "panel": true}));
    let b1 = bbox(&bordered);
    assert_eq!(b1[2] - b1[0], b0[2] - b0[0] + 20);
    assert_eq!(b1[3] - b1[1], b0[3] - b0[1] + 20);
    // Explicit parameters win over the panel.
    let own = text(&mut e, json!({"format": "eps", "panel": true, "options": {"b": 0}}));
    assert_eq!(bbox(&own), b0);
    // Preview and background colour.
    set(&mut e, "eps", json!({"border": 0, "preview": 1, "background": 4}));
    let epsi = text(&mut e, json!({"format": "eps", "panel": true}));
    assert!(epsi.contains("%%BeginPreview"), "no preview");
    assert!(epsi.contains("0 0 1"), "no blue background");
    set(
        &mut e,
        "eps",
        json!({"preview": 0, "background": -1, "grid_minor": "1/4", "grid_major": "1"}),
    );
    let grid = text(&mut e, json!({"format": "eps", "panel": true}));
    assert!(grid.len() > plain.len() + 200, "the grid adds lines");
}

#[test]
fn pdf_page_mode_and_page_settings() {
    let mut e = drawing();
    let mb = |s: &str| {
        let i = s.find("/MediaBox").unwrap();
        s[i..i + 60].to_string()
    };
    let fig = e.execute("file.export_data", &json!({"format": "pdf"})).unwrap();
    let small = String::from_utf8_lossy(&decode(fig["base64"].as_str().unwrap())).to_string();
    set(&mut e, "pdf", json!({"pdf_page_mode": true}));
    e.execute("doc.set_paper", &json!({"paper": "A4"})).unwrap();
    e.execute("doc.set_page", &json!({"orientation": "portrait"})).unwrap();
    let page = e
        .execute("file.export_data", &json!({"format": "pdf", "panel": true}))
        .unwrap();
    let page = String::from_utf8_lossy(&decode(page["base64"].as_str().unwrap())).to_string();
    assert_ne!(mb(&small), mb(&page));
    assert!(mb(&page).contains("595"), "{}", mb(&page));
    // The page settings are the figure's, in the .fig file.
    let fig = rustfig_format::write(e.doc(), rustfig_format::FileFormat::Fig).unwrap();
    assert!(fig.contains("Portrait") && fig.contains("A4"), "{}", &fig[..80]);
    assert!(e.execute("doc.set_page", &json!({"orientation": "sideways"})).is_err());
    e.execute("doc.set_page", &json!({"justify": "flush_left", "pages": "multiple"}))
        .unwrap();
    assert!(!e.doc().settings.center && e.doc().settings.multiple_pages);
}

fn decode(b64: &str) -> Vec<u8> {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0);
    for c in b64.bytes() {
        let Some(v) = T.iter().position(|t| *t == c) else {
            continue;
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

#[test]
fn parameters_per_language() {
    let e = drawing();
    let mut s = PanelSettings::xfig();
    s.border = 4.0;
    s.background = 7;
    s.grid_minor = "1/4".into();
    s.preview = 2;
    s.x_offset = 1.0;
    s.transparent = 4;
    s.quality = 50;
    s.smoothing = Some(2);
    let opts = |id: &str, s: &PanelSettings| params(&e, id, s).get("options").cloned().unwrap_or(json!({}));
    assert_eq!(opts("eps", &s)["b"], 4.0);
    assert_eq!(opts("eps", &s)["G"], "1/4in");
    assert_eq!(opts("eps", &s)["C"], "x");
    assert!(opts("eps", &s).get("x").is_none(), "offsets are for pages");
    assert_eq!(opts("ps", &s)["x"], 72);
    assert!(opts("pdf", &s).get("P").is_none());
    s.pdf_page_mode = true;
    assert_eq!(opts("pdf", &s)["P"], true);
    assert_eq!(opts("pdf", &s)["x"], 72);
    assert_eq!(opts("gif", &s)["t"], "#ff0000");
    assert!(opts("png", &s).get("t").is_none());
    assert_eq!(
        opts("jpeg", &s)["q"].as_i64().or(opts("jpeg", &s)["quality"].as_i64()),
        Some(50)
    );
    assert_eq!(opts("png", &s)["smoothing"], 2);
    assert!(opts("svg", &s).as_object().unwrap().is_empty());
    assert_eq!(params(&e, "tk", &s)["background"], "#ffffff");
    assert!(params(&e, "tk", &s).get("options").is_none());
    assert!(params(&e, "png", &s).contains_key("dpi"));
}

#[test]
fn every_language_exports_with_panel_settings() {
    let mut e = drawing();
    e.execute("doc.set_page", &json!({"pages": "multiple"})).unwrap();
    for l in rustfig_engine::fig2dev::languages::LANGUAGES {
        if !rustfig_engine::fig2dev::language_available(l) {
            continue;
        }
        e.execute(
            "export.settings",
            &json!({"format": l.id, "settings": {
                "border": 2, "background": 7, "grid_minor": "1/4", "grid_major": "1", "preview": 1,
                "pdf_page_mode": true, "overlap": true, "x_offset": 1, "y_offset": 1,
                "transparent": -3, "quality": 60, "smoothing": 2, "dpi": 50
            }}),
        )
        .unwrap();
        let r = e.execute("file.export_data", &json!({"format": l.id, "panel": true}));
        // The shape language needs special comments; the rest must work.
        let shape = r
            .as_ref()
            .err()
            .is_some_and(|err| err.message.contains("No shape found"));
        assert!(r.is_ok() || shape, "{}: {:?}", l.id, r.err());
    }
}

#[test]
fn gif_transparent_color() {
    let mut e = drawing();
    set(&mut e, "gif", json!({"transparent": 4, "dpi": 40}));
    let r = e
        .execute("file.export_data", &json!({"format": "gif", "panel": true}))
        .unwrap();
    let gif = decode(r["base64"].as_str().unwrap());
    assert_eq!(&gif[..3], b"GIF");
    // A graphic control extension marks the transparent index.
    assert!(gif.windows(2).any(|w| w == [0x21, 0xf9]), "no transparency");
}

#[test]
fn fit_to_page_and_figure_size() {
    let mut e = drawing();
    let s = e.execute("export.figure_size", &json!({"magnification": 50})).unwrap();
    assert!((s["width"].as_f64().unwrap() - 1.0).abs() < 0.02, "{s}");
    assert_eq!(s["unit"], "in");
    e.execute("doc.set_page", &json!({"orientation": "portrait"})).unwrap();
    let f = e.execute("export.fit_page", &json!({})).unwrap();
    // Letter portrait: 7.5 in wide for a 2 in wide figure.
    assert!((f["magnification"].as_f64().unwrap() - 375.0).abs() < 5.0, "{f}");
    assert!((e.doc().settings.magnification - 375.0).abs() < 5.0);
    assert!(Engine::new().execute("export.fit_page", &json!({})).is_err());
}

#[test]
fn bak_on_save_and_new_file_on_open() {
    let dir = std::env::temp_dir().join(format!("rustfig-bak-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.fig");
    let path = path.to_str().unwrap().to_string();
    let _ = std::fs::remove_file(format!("{path}.bak"));
    let _ = std::fs::remove_file(&path);

    // A name that doesn't exist: an empty figure of that name.
    let mut e = Engine::new();
    let r = e.execute("file.open", &json!({"path": path})).unwrap();
    assert_eq!(r["new_file"], true);
    assert_eq!(e.path(), Some(path.as_str()));
    assert!(e.doc().objects.is_empty() && !e.is_modified());
    assert!(!std::path::Path::new(&path).exists());

    // First save: no file yet, no .bak.
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 600}))
        .unwrap();
    e.execute("file.save", &json!({})).unwrap();
    assert!(std::path::Path::new(&path).exists());
    assert!(!std::path::Path::new(&format!("{path}.bak")).exists());
    let first = std::fs::read(&path).unwrap();

    // Second save: the old one is kept.
    e.execute(
        "shape.rectangle",
        &json!({"x": 900, "y": 0, "width": 600, "height": 600}),
    )
    .unwrap();
    e.execute("file.save", &json!({})).unwrap();
    assert_eq!(std::fs::read(format!("{path}.bak")).unwrap(), first);
    assert_ne!(std::fs::read(&path).unwrap(), first);

    // Switched off (write_bak false): the .bak stays as it was.
    let _ = std::fs::remove_file(format!("{path}.bak"));
    e.execute("prefs.set", &json!({"resource": "write_bak", "value": false}))
        .unwrap();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 900, "width": 600, "height": 600}),
    )
    .unwrap();
    e.execute("file.save", &json!({})).unwrap();
    assert!(!std::path::Path::new(&format!("{path}.bak")).exists());

    // A bad name is an error, not a new file.
    assert!(e.execute("file.open", &json!({"path": "a\u{0}b"})).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
