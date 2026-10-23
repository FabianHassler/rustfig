#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! E16: exports that carry the figure (PDF, SVG, PNG) and open again.

use rustfig_engine::Engine;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustfig-e16-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn drawing() -> Engine {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 0, "y": 0, "width": 1200, "height": 600}),
    )
    .unwrap();
    e.execute("shape.ellipse", &json!({"cx": 3000, "cy": 300, "rx": 400, "ry": 200}))
        .unwrap();
    e
}

fn fig_text(e: &Engine) -> String {
    rustfig_format::write(e.doc(), rustfig_format::FileFormat::Fig).unwrap()
}

fn export(e: &mut Engine, dir: &std::path::Path, name: &str, embed: bool) -> PathBuf {
    let path = dir.join(name);
    e.execute("file.export", &json!({"path": path.to_str().unwrap(), "embed": embed}))
        .unwrap();
    path
}

fn reopen(path: &std::path::Path) -> Result<Engine, String> {
    let mut e2 = Engine::new();
    e2.execute("file.open", &json!({"path": path.to_str().unwrap()}))
        .map_err(|err| err.to_string())?;
    Ok(e2)
}

#[test]
fn round_trip_every_format() {
    let dir = temp_dir("rt");
    let mut e = drawing();
    let want = fig_text(&e);
    for name in ["fig.pdf", "fig.svg", "fig.png"] {
        let path = export(&mut e, &dir, name, true);
        let e2 = reopen(&path).unwrap_or_else(|err| panic!("{name}: {err}"));
        assert_eq!(fig_text(&e2), want, "{name}");
    }
}

#[test]
fn other_programs_still_read_the_files() {
    let dir = temp_dir("valid");
    let mut e = drawing();

    let pdf = std::fs::read(export(&mut e, &dir, "a.pdf", true)).unwrap();
    let plain = std::fs::read(export(&mut e, &dir, "b.pdf", false)).unwrap();
    assert!(pdf.starts_with(&plain), "the update only appends");
    let doc = hayro_syntax::Pdf::new(Arc::new(pdf.clone())).expect("PDF parses");
    assert_eq!(doc.pages().len(), 1);
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/EmbeddedFiles") && text.contains("/AF [") && text.contains("figure.fig"));

    let svg = std::fs::read(export(&mut e, &dir, "a.svg", true)).unwrap();
    resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default()).expect("SVG parses");

    let png = std::fs::read(export(&mut e, &dir, "a.png", true)).unwrap();
    let img = image::load_from_memory_with_format(&png, image::ImageFormat::Png).expect("PNG decodes");
    assert!(img.width() > 0);
}

#[test]
fn off_by_default_and_files_without_payload_say_so() {
    let dir = temp_dir("none");
    let mut e = drawing();
    for name in ["x.pdf", "x.svg", "x.png"] {
        let path = dir.join(name);
        e.execute("file.export", &json!({"path": path.to_str().unwrap()}))
            .unwrap();
        let err = reopen(&path).err().unwrap();
        assert!(err.contains("no embedded figure"), "{name}: {err}");
    }
}

#[test]
fn embed_in_other_languages_is_an_error() {
    let dir = temp_dir("lang");
    let mut e = drawing();
    let path = dir.join("x.eps");
    let r = e.execute("file.export", &json!({"path": path.to_str().unwrap(), "embed": true}));
    assert!(r.is_err());
    assert!(!path.exists());
}

#[test]
fn panel_setting_is_kept_per_language_and_applies_on_save() {
    let dir = temp_dir("panel");
    let mut e = drawing();
    let svg = dir.join("d.svg");
    let fig = dir.join("d.fig");
    e.execute(
        "file.export_on_save",
        &json!({"path": svg.to_str().unwrap(), "format": "svg", "embed": true}),
    )
    .unwrap();
    e.execute("file.save", &json!({"path": fig.to_str().unwrap()})).unwrap();
    let e2 = reopen(&svg).unwrap();
    assert_eq!(fig_text(&e2), fig_text(&e));

    let s = rustfig_engine::export_panel::PanelSettings {
        embed: true,
        ..Default::default()
    };
    rustfig_engine::export_panel::store(&mut e, "pdf", s);
    assert!(rustfig_engine::export_panel::settings_for(&e, "pdf").embed);
    assert!(!rustfig_engine::export_panel::settings_for(&e, "svg").embed);
    let st = rustfig_engine::export_panel::settings_for(&e, "pdf");
    assert_eq!(rustfig_engine::export_panel::params(&e, "pdf", &st)["embed"], true);
    assert!(rustfig_engine::export_panel::params(&e, "eps", &st)
        .get("embed")
        .is_none());
}
