//! F14: `rustfig-cli convert` writes what File > Export writes, with the
//! same settings file: every language with Export panel rows that change
//! the output, through the panel's request and through convert.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_engine::{export_panel, prefs::Prefs, Engine};
use serde_json::json;
use std::path::Path;

/// Saved panel values that differ from the defaults, per language.
const SETTINGS: &str = r#"
[rustfig.export.pdf]
border = 12.0
background = 2
grid_minor = "1/4"
grid_major = "1"
[rustfig.export.eps]
border = 5.0
preview = 1
[rustfig.export.ps]
border = 3.0
x_offset = 1.0
y_offset = 2.0
y_unit = 1
[rustfig.export.svg]
embed = true
[rustfig.export.png]
dpi = 96.0
border = 7.0
smoothing = 2
background = 3
[rustfig.export.jpeg]
dpi = 60.0
quality = 40
[rustfig.export.gif]
dpi = 50.0
transparent = -3
[rustfig.export.tiff]
dpi = 40.0
[rustfig.export.ppm]
dpi = 30.0
grid_minor = "1/2"
[rustfig.export.pcx]
dpi = 30.0
[rustfig.export.xbm]
dpi = 30.0
[rustfig.export.xpm]
dpi = 30.0
[rustfig.export.ibmgl]
hpgl_pcl = true
hpgl_font = true
[rustfig.export.asy]
asy_splines = "hobby"
"#;

/// The export as the panel makes it, with the stored settings.
fn panel_export(input: &Path, out: &Path, lang: &str) {
    let mut e = Engine::new();
    e.execute("file.open", &json!({ "path": input })).unwrap();
    e.prefs = Prefs::parse(SETTINGS).unwrap().0;
    e.prefs.apply_latex(&e);
    let s = export_panel::settings_for(&e, lang);
    let mag = e.doc().settings.magnification;
    let p = export_panel::request(&e, lang, &s, &out.to_string_lossy(), mag, false, false);
    e.execute("file.export", &p).unwrap();
}

/// Lines that carry the time of the export.
fn masked(bytes: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    if !text.contains("CreationDate") && !text.contains("ModDate") {
        return bytes.to_vec();
    }
    let mut out = Vec::new();
    for line in bytes.split(|b| *b == b'\n') {
        let l = String::from_utf8_lossy(line);
        if !(l.contains("CreationDate") || l.contains("ModDate") || l.contains("/ID [")) {
            out.extend_from_slice(line);
        }
        out.push(b'\n');
    }
    out
}

#[test]
fn convert_writes_what_the_export_panel_writes() {
    std::env::set_var("FIG2DEV_BUILTIN_FONTS", "1");
    let dir = std::env::temp_dir().join(format!("rustfig-convert-is-export-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("settings")).unwrap();
    std::fs::write(dir.join("settings/settings.toml"), SETTINGS).unwrap();
    // convert reads the settings file from here, as the app does.
    std::env::set_var("RUSTFIG_SETTINGS_DIR", dir.join("settings"));
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/engine/tests/fixtures");
    for fixture in ["text.fig", "fills.fig", "splines.fig"] {
        let input = fixtures.join(fixture);
        for (lang, ext) in [
            ("pdf", "pdf"),
            ("eps", "eps"),
            ("ps", "ps"),
            ("svg", "svg"),
            ("png", "png"),
            ("jpeg", "jpg"),
            ("gif", "gif"),
            ("tiff", "tif"),
            ("ppm", "ppm"),
            ("pcx", "pcx"),
            ("xbm", "xbm"),
            ("xpm", "xpm"),
            ("ibmgl", "hpl"),
            ("asy", "asy"),
        ] {
            let a = dir.join(format!("panel-{lang}.{ext}"));
            let b = dir.join(format!("convert-{lang}.{ext}"));
            panel_export(&input, &a, lang);
            rustfig_cli::convert(&[
                input.to_string_lossy().into_owned(),
                b.to_string_lossy().into_owned(),
                "--format".into(),
                lang.into(),
            ])
            .unwrap();
            let (a, b) = (std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
            assert!(
                masked(&a) == masked(&b),
                "{fixture} {lang}: convert differs from the Export panel"
            );
        }
    }
    // A flag changes the setting for this run: not the panel's file.
    let input = fixtures.join("fills.fig");
    let saved = dir.join("saved.png");
    let flagged = dir.join("flagged.png");
    let arg = |p: &Path| p.to_string_lossy().into_owned();
    rustfig_cli::convert(&[arg(&input), arg(&saved)]).unwrap();
    rustfig_cli::convert(&[arg(&input), arg(&flagged), "--dpi".into(), "200".into()]).unwrap();
    assert!(std::fs::read(&flagged).unwrap().len() > std::fs::read(&saved).unwrap().len());
    assert_eq!(
        std::fs::read_to_string(dir.join("settings/settings.toml")).unwrap(),
        SETTINGS
    );
    // Unknown flags and bad values are errors, not silently ignored.
    assert!(rustfig_cli::convert(&[arg(&input), arg(&saved), "--dpl".into(), "3".into()]).is_err());
    assert!(rustfig_cli::convert(&[arg(&input), arg(&saved), "--preview".into(), "fancy".into()]).is_err());
    std::fs::remove_dir_all(&dir).ok();
}
