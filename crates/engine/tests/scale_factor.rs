#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! S3: xfig's `-scale_factor` scales files as they are read (`f_read.c`,
//! `read_scale_compound`): open and merge scale coordinates and arrowheads
//! about the origin; font sizes and line widths stay.

use rustfig_engine::model::ObjectKind;
use rustfig_engine::Engine;
use serde_json::json;

const FIG: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
2 1 0 2 0 7 50 -1 -1 0.000 0 0 -1 1 0 2\n\t1 1 1.00 60.00 120.00\n\t 1200 1200 2400 1200\n\
4 0 0 50 -1 0 12 0.0000 4 135 405 600 600 Text\\001\n";

fn write_fig(name: &str) -> String {
    let p = std::env::temp_dir().join(format!("rustfig-scale-{}-{name}.fig", std::process::id()));
    std::fs::write(&p, FIG).unwrap();
    p.to_string_lossy().into_owned()
}

fn check(e: &Engine, k: f64) {
    let mut seen = 0;
    let mut all: Vec<&rustfig_engine::model::Object> = e.doc().objects.iter().map(|o| &**o).collect();
    while let Some(o) = all.pop() {
        match &o.kind {
            ObjectKind::Compound(c) => all.extend(c.objects.iter()),
            ObjectKind::Polyline(p) => {
                assert_eq!(p.points[0].x, 1200.0 * k);
                assert_eq!(p.points[1].x, 2400.0 * k);
                let a = p.forward_arrow.unwrap();
                assert_eq!((a.width, a.height), (60.0 * k, 120.0 * k));
                assert_eq!(p.style.thickness, 2);
                seen += 1;
            }
            ObjectKind::Text(t) => {
                assert_eq!((t.origin.x, t.origin.y), (600.0 * k, 600.0 * k));
                assert_eq!(t.size, 12.0);
                seen += 1;
            }
            _ => {}
        }
    }
    assert_eq!(seen, 2);
}

#[test]
fn open_and_merge_scale_as_they_read() {
    let path = write_fig("open");
    let mut e = Engine::new();
    e.read_scale = 2.0;
    e.execute("file.open", &json!({ "path": path })).unwrap();
    check(&e, 2.0);
    let mut m = Engine::new();
    m.read_scale = 0.5;
    m.execute("file.merge", &json!({ "path": path })).unwrap();
    check(&m, 0.5);
    // The default reads the file unchanged.
    let mut d = Engine::new();
    d.execute("file.open", &json!({ "path": path })).unwrap();
    check(&d, 1.0);
    let _ = std::fs::remove_file(path);
}

#[test]
fn the_resource_sets_the_factor_and_bad_values_mean_one() {
    let mut e = Engine::new();
    for (v, want) in [(3.0, 3.0), (0.0, 1.0), (-2.0, 1.0), (1e9, 1.0)] {
        let mut p = e.prefs.clone();
        p.xfig.insert("scale_factor".into(), toml::Value::Float(v));
        p.apply(&mut e);
        assert_eq!(e.read_scale, want, "{v}");
    }
}
