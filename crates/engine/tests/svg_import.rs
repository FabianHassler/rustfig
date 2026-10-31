#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! G7: an SVG opened or merged as editable Fig objects.

mod support;

use rustfig_engine::model::{ObjectKind, PolyKind};
use rustfig_engine::Engine;
use serde_json::json;
use std::path::{Path, PathBuf};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustfig-g7-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn svg_of(fig: &Path, dir: &Path, name: &str) -> PathBuf {
    std::env::set_var("FIG2DEV_BUILTIN_FONTS", "1");
    let mut e = Engine::new();
    e.execute("file.open", &json!({"path": fig.to_str().unwrap()})).unwrap();
    let out = dir.join(name);
    e.execute("file.export", &json!({"path": out.to_str().unwrap(), "embed": false}))
        .unwrap();
    out
}

fn png(e: &mut Engine) -> Vec<u8> {
    let r = e.execute("file.export_data", &json!({"format": "png"})).unwrap();
    rustfig_engine::base64_decode(r["files"][0]["base64"].as_str().or(r["base64"].as_str()).unwrap())
}

fn open_svg(svg: &str) -> (Engine, serde_json::Value) {
    let mut e = Engine::new();
    let r = e
        .execute("file.open_data", &json!({"data": svg, "name": "t.svg"}))
        .unwrap();
    (e, r)
}

fn svg(body: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"192\" height=\"96\" viewBox=\"0 0 192 96\">{body}</svg>"
    )
}

#[test]
fn fixtures_round_trip_through_svg() {
    let dir = temp_dir("rt");
    for name in [
        "boxes.fig",
        "ellipses.fig",
        "lines.fig",
        "text.fig",
        "colours.fig",
        "fills.fig",
        "arrows.fig",
    ] {
        let path = svg_of(&fixture(name), &dir, &format!("{name}.svg"));
        let mut orig = Engine::new();
        orig.execute("file.open", &json!({"path": fixture(name).to_str().unwrap()}))
            .unwrap();
        let mut imp = Engine::new();
        let r = imp
            .execute("file.open", &json!({"path": path.to_str().unwrap()}))
            .unwrap();
        assert_eq!(r["imported"], json!(true), "{name}");
        assert!(r["objects"].as_u64().unwrap() > 0, "{name}");
        let c = support::compare::compare(&png(&mut orig), &png(&mut imp)).unwrap();
        eprintln!("{name}: {:?}", c.score);
        assert!(c.score.mean < 0.03 && c.score.strong < 0.03, "{name}: {:?}", c.score);
    }
}

#[test]
fn ellipses_boxes_and_texts_keep_their_kinds() {
    let dir = temp_dir("kinds");
    let path = svg_of(&fixture("ellipses.fig"), &dir, "e.svg");
    let mut e = Engine::new();
    e.execute("file.open", &json!({"path": path.to_str().unwrap()}))
        .unwrap();
    let ell = e
        .doc()
        .objects
        .iter()
        .filter(|o| matches!(o.kind, ObjectKind::Ellipse(_)))
        .count();
    let mut orig = Engine::new();
    orig.execute("file.open", &json!({"path": fixture("ellipses.fig").to_str().unwrap()}))
        .unwrap();
    let want = orig
        .doc()
        .objects
        .iter()
        .filter(|o| matches!(o.kind, ObjectKind::Ellipse(_)))
        .count();
    assert_eq!(ell, want);
    // A rotated ellipse keeps its angle (to a degree).
    assert!(e
        .doc()
        .objects
        .iter()
        .any(|o| matches!(&o.kind, ObjectKind::Ellipse(x) if x.angle.abs() > 0.1)));

    let path = svg_of(&fixture("boxes.fig"), &dir, "b.svg");
    let mut e = Engine::new();
    e.execute("file.open", &json!({"path": path.to_str().unwrap()}))
        .unwrap();
    assert!(
        e.doc()
            .objects
            .iter()
            .any(|o| matches!(&o.kind, ObjectKind::Polyline(p) if p.kind == PolyKind::ArcBox)),
        "the rounded box stays an arc-box"
    );

    let path = svg_of(&fixture("text.fig"), &dir, "t.svg");
    let mut e = Engine::new();
    e.execute("file.open", &json!({"path": path.to_str().unwrap()}))
        .unwrap();
    let t: Vec<_> = e
        .doc()
        .objects
        .iter()
        .filter_map(|o| match &o.kind {
            ObjectKind::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert!(t
        .iter()
        .any(|t| t.text == "Justified 1" && t.justify == rustfig_engine::model::Justify::Center));
    assert!(t
        .iter()
        .any(|t| t.text == "Justified 2" && t.justify == rustfig_engine::model::Justify::Right));
    assert!(
        t.iter().any(|t| t.text.starts_with("Font 2 ") && t.font == 2),
        "Times Bold"
    );
    assert!(
        t.iter().any(|t| t.text.starts_with("Font 1 ") && t.font == 1),
        "Times Italic"
    );
    assert!(
        t.iter().any(|t| t.text.starts_with("Font 12 ") && t.font == 12),
        "Courier"
    );
    assert!(
        t.iter().any(|t| t.text.starts_with("Font 16 ") && t.font == 16),
        "Helvetica"
    );
    assert!(t.iter().any(|t| t.angle > 0.4 && t.angle < 0.6), "30 degrees");
}

#[test]
fn rect_round_rect_circle_ellipse_and_path() {
    let (e, r) = open_svg(&svg(
        "<rect x=\"10\" y=\"10\" width=\"40\" height=\"20\" fill=\"#ff0000\"/>\
         <rect x=\"60\" y=\"10\" width=\"40\" height=\"20\" rx=\"6\" fill=\"none\" stroke=\"#0000ff\" stroke-width=\"2\"/>\
         <circle cx=\"30\" cy=\"60\" r=\"15\" fill=\"#00ff00\"/>\
         <ellipse cx=\"90\" cy=\"60\" rx=\"20\" ry=\"10\" fill=\"none\" stroke=\"black\" transform=\"rotate(30 90 60)\"/>\
         <path d=\"M120 10 C 140 10 150 40 170 40 L 170 80 Z\" fill=\"#ffff00\" stroke=\"black\" stroke-dasharray=\"6 3\"/>\
         <path d=\"M120 90 L 160 90 L 160 70\" fill=\"none\" stroke=\"black\"/>",
    ));
    assert_eq!(r["objects"], json!(6), "{r}");
    let o = &e.doc().objects;
    // 1 px = 12.5 Fig units.
    let ObjectKind::Polyline(b) = &o[0].kind else {
        panic!("box")
    };
    assert_eq!(b.kind, PolyKind::Box);
    assert_eq!(
        e.doc().rgb(b.style.fill_color),
        rustfig_engine::model::Rgb::new(255, 0, 0)
    );
    assert_eq!(b.style.fill_color.0, 4, "standard red");
    assert_eq!(b.style.area_fill, 20);
    assert_eq!(b.style.thickness, 0, "no stroke");
    assert_eq!((b.points[0].x, b.points[0].y), (125.0, 125.0));
    assert_eq!((b.points[2].x, b.points[2].y), (625.0, 375.0));
    let ObjectKind::Polyline(rb) = &o[1].kind else {
        panic!("arc-box")
    };
    assert_eq!(rb.kind, PolyKind::ArcBox);
    assert_eq!(rb.radius, 5, "6 px = 75 units = 5/80 inch");
    assert_eq!(rb.style.pen_color.0, 1, "blue");
    assert_eq!(rb.style.area_fill, -1);
    assert!(rb.style.thickness >= 2);
    let ObjectKind::Ellipse(c) = &o[2].kind else {
        panic!("circle")
    };
    assert!(c.kind.is_circle());
    assert_eq!((c.center.x, c.center.y, c.radii.x), (375.0, 750.0, 188.0));
    let ObjectKind::Ellipse(el) = &o[3].kind else {
        panic!("ellipse")
    };
    assert!(!el.kind.is_circle());
    assert_eq!((el.radii.x, el.radii.y), (250.0, 125.0));
    assert!(
        (el.angle - (-30f64).to_radians().rem_euclid(std::f64::consts::PI)).abs() < 0.01,
        "{}",
        el.angle
    );
    let ObjectKind::Polyline(p) = &o[4].kind else {
        panic!("path")
    };
    assert_eq!(p.kind, PolyKind::Polygon);
    assert!(p.points.len() > 4, "the curve is flattened");
    assert_eq!(p.style.line, rustfig_engine::model::LineKind::Dashed);
    let ObjectKind::Polyline(l) = &o[5].kind else {
        panic!("line")
    };
    assert_eq!(l.kind, PolyKind::Polyline);
    assert_eq!(l.points.len(), 3);
}

#[test]
fn transforms_text_and_units() {
    let (e, _) = open_svg(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"2in\" height=\"1in\" viewBox=\"0 0 100 50\">\
         <g transform=\"translate(10 5) scale(2)\"><rect width=\"10\" height=\"5\"/></g>\
         <text x=\"50\" y=\"30\" font-family=\"Helvetica, Arial\" font-size=\"10\" font-weight=\"bold\" text-anchor=\"middle\" fill=\"#800000\">Hi there</text>\
         </svg>",
    );
    let mut text = None;
    for o in &e.doc().objects {
        match &o.kind {
            ObjectKind::Polyline(p) => {
                // 100 user units = 2 in = 2400 Fig units: 24 per unit.
                assert_eq!((p.points[0].x, p.points[0].y), (240.0, 120.0));
                assert_eq!((p.points[2].x, p.points[2].y), (720.0, 360.0));
            }
            ObjectKind::Text(t) => text = Some(t.clone()),
            _ => {}
        }
    }
    let t = text.unwrap();
    assert_eq!(t.text, "Hi there");
    assert_eq!(t.font, 18, "Helvetica Bold");
    assert_eq!(t.justify, rustfig_engine::model::Justify::Center);
    // 10 user units = 0.2 in = 14.4 pt.
    assert!((t.size - 14.4).abs() < 0.2, "{}", t.size);
    assert_eq!((t.origin.x, t.origin.y), (1200.0, 720.0));
    assert_eq!(e.doc().rgb(t.color), rustfig_engine::model::Rgb::new(128, 0, 0));
}

#[test]
fn gradients_and_effects_warn_once() {
    let (e, r) = open_svg(&svg(
        "<defs><linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"#ff0000\"/><stop offset=\"1\" stop-color=\"#0000ff\"/></linearGradient>\
         <clipPath id=\"c\"><rect width=\"50\" height=\"50\"/></clipPath></defs>\
         <rect width=\"40\" height=\"40\" fill=\"url(#g)\"/>\
         <rect x=\"50\" width=\"40\" height=\"40\" fill=\"url(#g)\" opacity=\"0.5\"/>\
         <g clip-path=\"url(#c)\"><rect x=\"100\" width=\"40\" height=\"40\"/></g>",
    ));
    let w: Vec<String> = r["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w.as_str().unwrap().to_string())
        .collect();
    assert_eq!(w.iter().filter(|w| w.contains("Gradients")).count(), 1, "{w:?}");
    assert_eq!(w.iter().filter(|w| w.contains("Clipping")).count(), 1, "{w:?}");
    assert!(w.iter().any(|w| w.contains("Transparency")), "{w:?}");
    // The average of red and blue.
    let ObjectKind::Polyline(p) = &e.doc().objects[0].kind else {
        panic!()
    };
    assert_eq!(
        e.doc().rgb(p.style.fill_color),
        rustfig_engine::model::Rgb::new(128, 0, 128)
    );
}

#[test]
fn images_become_pictures_next_to_the_file() {
    let dir = temp_dir("img");
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(4, 2, image::Rgba([255, 0, 0, 255])))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    std::fs::write(dir.join("pic.png"), &png).unwrap();
    let b64 = rustfig_engine::base64(&png);
    let doc = svg(&format!(
        "<image x=\"10\" y=\"10\" width=\"40\" height=\"20\" xlink:href=\"data:image/png;base64,{b64}\"/>\
         <image x=\"60\" y=\"10\" width=\"40\" height=\"20\" xlink:href=\"pic.png\"/>\
         <image x=\"110\" y=\"10\" width=\"40\" height=\"20\" xlink:href=\"../../etc/hosts\"/>"
    ));
    let path = dir.join("i.svg");
    std::fs::write(&path, doc).unwrap();
    let mut e = Engine::new();
    let r = e
        .execute("file.open", &json!({"path": path.to_str().unwrap()}))
        .unwrap();
    let pics: Vec<String> = e
        .doc()
        .objects
        .iter()
        .filter_map(|o| match &o.kind {
            ObjectKind::Polyline(p) => p.picture.as_ref().map(|p| p.file.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(pics.len(), 2, "{r}");
    for p in &pics {
        assert!(dir.join(p).is_file(), "{p}");
    }
}

#[test]
fn svgz_open_merge_and_the_embedded_figure() {
    use std::io::Write;
    let dir = temp_dir("z");
    let text = svg("<rect width=\"10\" height=\"10\"/>");
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    enc.write_all(text.as_bytes()).unwrap();
    let path = dir.join("z.svgz");
    std::fs::write(&path, enc.finish().unwrap()).unwrap();
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100}))
        .unwrap();
    e.execute("file.merge", &json!({"path": path.to_str().unwrap()}))
        .unwrap();
    assert_eq!(e.doc().objects.len(), 2);
    assert!(matches!(e.doc().objects[1].kind, ObjectKind::Compound(_)));
    let mut e = Engine::new();
    let r = e
        .execute("file.open", &json!({"path": path.to_str().unwrap()}))
        .unwrap();
    assert_eq!(r["imported"], json!(true));
    assert_eq!(r["path"].as_str().unwrap(), dir.join("z.fig").to_str().unwrap());

    // An export with the figure inside opens that figure, unless `vectors`.
    let mut src = Engine::new();
    src.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    let emb = dir.join("emb.svg");
    src.execute("file.export", &json!({"path": emb.to_str().unwrap(), "embed": true}))
        .unwrap();
    let mut e2 = Engine::new();
    let r = e2
        .execute("file.open", &json!({"path": emb.to_str().unwrap()}))
        .unwrap();
    assert_eq!(r["embedded"], json!(true));
    let mut e3 = Engine::new();
    let r = e3
        .execute("file.open", &json!({"path": emb.to_str().unwrap(), "vectors": true}))
        .unwrap();
    assert_eq!(r["imported"], json!(true));
}

#[test]
fn hostile_svgs_are_errors_not_panics() {
    let bombs = [
        "<svg".to_string(),
        "<svg xmlns=\"http://www.w3.org/2000/svg\"/>".to_string(),
        svg("<rect width=\"1e400\" height=\"1e38\"/><path d=\"M 1e300 0 L 5 5 Z\" stroke=\"red\"/>"),
        svg("<circle r=\"1e30\" cx=\"1e30\" cy=\"0\" stroke=\"red\" stroke-width=\"1e30\"/>"),
        svg(&"<g>".repeat(5000)),
        svg("<use href=\"#a\" id=\"a\"/>"),
        svg("<text font-size=\"1e30\">x</text><text font-size=\"0\">y</text>"),
        svg("<image width=\"10\" height=\"10\" href=\"/etc/passwd\"/>"),
        "<!DOCTYPE svg [<!ENTITY a \"aaaaaaaaaa\"><!ENTITY b \"&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;\"><!ENTITY c \"&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;\">]><svg xmlns=\"http://www.w3.org/2000/svg\" width=\"9\" height=\"9\"><text>&c;&c;&c;&c;&c;&c;&c;</text></svg>".to_string(),
    ];
    for (i, b) in bombs.iter().enumerate() {
        let mut e = Engine::new();
        let _ = e.execute("file.open_data", &json!({"data": b, "vectors": true}));
        let _ = e.execute("file.merge_data", &json!({"data": b, "vectors": true}));
        let _ = i;
    }
    let mut e = Engine::new();
    assert!(e
        .execute("file.open_data", &json!({"data": "<svg", "vectors": true}))
        .is_err());
}
