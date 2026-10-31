#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! G5: a PDF opened or merged as editable Fig objects.

mod support;

use rustfig_engine::model::{ObjectKind, PolyKind};
use rustfig_engine::Engine;
use serde_json::json;
use std::path::{Path, PathBuf};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustfig-g5-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn pdf_of(fig: &Path, dir: &Path, name: &str) -> PathBuf {
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

fn kinds(e: &Engine) -> (usize, usize, usize, usize, usize) {
    let (mut lines, mut boxes, mut ell, mut text, mut pic) = (0, 0, 0, 0, 0);
    for o in &e.doc().objects {
        match &o.kind {
            ObjectKind::Polyline(p) if p.kind == PolyKind::Picture => pic += 1,
            ObjectKind::Polyline(p) if p.kind == PolyKind::Box => boxes += 1,
            ObjectKind::Polyline(_) => lines += 1,
            ObjectKind::Ellipse(_) => ell += 1,
            ObjectKind::Text(_) => text += 1,
            _ => {}
        }
    }
    (lines, boxes, ell, text, pic)
}

#[test]
fn fixtures_round_trip_through_pdf() {
    let dir = temp_dir("rt");
    for name in [
        "boxes.fig",
        "ellipses.fig",
        "lines.fig",
        "text.fig",
        "colours.fig",
        "fills.fig",
    ] {
        let pdf = pdf_of(&fixture(name), &dir, &format!("{name}.pdf"));
        let mut orig = Engine::new();
        orig.execute("file.open", &json!({"path": fixture(name).to_str().unwrap()}))
            .unwrap();
        let mut imp = Engine::new();
        let r = imp
            .execute("file.open", &json!({"path": pdf.to_str().unwrap()}))
            .unwrap();
        assert!(r["objects"].as_u64().unwrap() > 0, "{name}");
        let c = support::compare::compare(&png(&mut orig), &png(&mut imp)).unwrap();
        eprintln!("{name}: {:?} kinds {:?}", c.score, kinds(&imp));
        assert!(c.score.mean < 0.03 && c.score.strong < 0.03, "{name}: {:?}", c.score);
    }
}

/// A one-page PDF around a content stream (and an optional 2x2 RGB image).
fn tiny_pdf(content: &str, with_image: bool) -> Vec<u8> {
    let mut objs: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >> /F2 << /Type /Font /Subtype /Type1 /BaseFont /ABCDEF+CMTT10 /FontDescriptor << /Type /FontDescriptor /FontName /ABCDEF+CMTT10 /Flags 33 >> >> /F3 << /Type /Font /Subtype /Type1 /BaseFont /CMBX12 >> >> /XObject << /Im 5 0 R >> >> >>".to_vec(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content).into_bytes(),
    ];
    if with_image {
        let mut o = b"<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 12 >>\nstream\n".to_vec();
        o.extend_from_slice(&[255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0]);
        o.extend_from_slice(b"\nendstream");
        objs.push(o);
    }
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offs = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offs.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let x = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offs {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",
            objs.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn open_bytes(bytes: &[u8], dir: &Path, name: &str) -> (Engine, serde_json::Value) {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    let mut e = Engine::new();
    let r = e
        .execute("file.open", &json!({"path": path.to_str().unwrap()}))
        .unwrap();
    (e, r)
}

#[test]
fn shapes_text_and_images_map_to_fig_objects() {
    let dir = temp_dir("map");
    let content = "1 0 0 rg 10 10 50 30 re f 0 0 1 RG 3 w [6 3] 0 d 70 20 m 120 60 l 150 20 l S \
        0 g BT /F1 12 Tf 20 80 Td (Hello) Tj ET q 40 0 0 40 150 50 cm /Im Do Q";
    let (e, r) = open_bytes(&tiny_pdf(content, true), &dir, "map.pdf");
    assert_eq!(r["imported"], json!(true));
    let (lines, boxes, _, text, pic) = kinds(&e);
    assert_eq!((boxes, text, pic), (1, 1, 1), "kinds {:?}", kinds(&e));
    assert_eq!(lines, 1);
    for o in &e.doc().objects {
        match &o.kind {
            ObjectKind::Text(t) => {
                assert_eq!(t.text, "Hello");
                assert_eq!(t.font, 18, "Helvetica-Bold");
                assert!((t.size - 12.0).abs() < 0.2);
            }
            ObjectKind::Polyline(p) if p.kind == PolyKind::Box => {
                assert_eq!(
                    e.doc().rgb(p.style.fill_color),
                    rustfig_engine::model::Rgb::new(255, 0, 0)
                );
                assert_eq!(p.style.fill_color.0, 4, "red is the standard colour 4");
                assert_eq!(p.style.area_fill, 20);
            }
            ObjectKind::Polyline(p) if p.kind == PolyKind::Polyline => {
                assert_eq!(p.style.line, rustfig_engine::model::LineKind::Dashed);
                assert_eq!(p.style.pen_color.0, 1, "blue is 1");
                assert_eq!(p.points.len(), 3);
            }
            _ => {}
        }
    }
    // The picture file sits next to the PDF and is written with the figure.
    let pic = e
        .doc()
        .objects
        .iter()
        .find_map(|o| match &o.kind {
            ObjectKind::Polyline(p) => p.picture.as_ref().map(|p| p.file.clone()),
            _ => None,
        })
        .unwrap();
    assert!(dir.join(&pic).is_file(), "{pic}");
    // Page origin top-left: the box is at the bottom of the 100 pt page.
    let b = e.doc_bounds();
    assert!(b.max.y > 1200.0 * 80.0 / 72.0 * 0.9 && b.min.y >= 0.0, "{b:?}");
    // The figure saves and opens again.
    let out = dir.join("saved.fig");
    let mut e = e;
    e.execute("file.save", &json!({"path": out.to_str().unwrap()})).unwrap();
    let mut e2 = Engine::new();
    e2.execute("file.open", &json!({"path": out.to_str().unwrap()}))
        .unwrap();
    assert_eq!(e2.doc().objects.len(), e.doc().objects.len());
}

#[test]
fn merge_adds_a_compound_and_pages_are_counted() {
    let dir = temp_dir("merge");
    let path = dir.join("m.pdf");
    std::fs::write(&path, tiny_pdf("0 0 10 10 re f", false)).unwrap();
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100}))
        .unwrap();
    e.execute("file.merge", &json!({"path": path.to_str().unwrap()}))
        .unwrap();
    assert_eq!(e.doc().objects.len(), 2);
    assert!(matches!(e.doc().objects[1].kind, ObjectKind::Compound(_)));
    let r = e
        .execute("file.pdf_pages", &json!({"path": path.to_str().unwrap()}))
        .unwrap();
    assert_eq!(r["pages"], json!(1));
    assert!(e
        .execute("file.open", &json!({"path": path.to_str().unwrap(), "page": 2}))
        .is_err());
    assert!(e
        .execute("file.open", &json!({"path": path.to_str().unwrap(), "page": 0}))
        .is_err());
}

#[test]
fn a_pdf_with_an_embedded_figure_still_opens_that_figure() {
    let dir = temp_dir("embed");
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    let path = dir.join("emb.pdf");
    e.execute("file.export", &json!({"path": path.to_str().unwrap(), "embed": true}))
        .unwrap();
    let mut e2 = Engine::new();
    let r = e2
        .execute("file.open", &json!({"path": path.to_str().unwrap()}))
        .unwrap();
    assert_eq!(r["embedded"], json!(true));
    let mut e3 = Engine::new();
    let r = e3
        .execute("file.open", &json!({"path": path.to_str().unwrap(), "vectors": true}))
        .unwrap();
    assert_eq!(r["imported"], json!(true));
}

#[test]
fn hostile_pdfs_are_errors_not_panics() {
    let dir = temp_dir("bad");
    let good = tiny_pdf("0 0 10 10 re f", false);
    let mut cases: Vec<Vec<u8>> = vec![
        b"%PDF-1.4\n".to_vec(),
        b"%PDF-1.4 garbage garbage".to_vec(),
        good[..good.len() / 2].to_vec(),
        tiny_pdf("1e400 0 0 1e400 0 0 cm 0 0 1 1 re f 99999999999 0 m 5 5 l S", false),
        tiny_pdf("q q q q q q Q", false),
        tiny_pdf("BT /F1 1e30 Tf (x) Tj ET /Im Do", true),
    ];
    for cut in [10, 40, 120, 300] {
        let mut b = good.clone();
        for (i, x) in b.iter_mut().enumerate().skip(cut).step_by(37) {
            *x = x.wrapping_add(i as u8);
        }
        cases.push(b);
    }
    for (i, c) in cases.iter().enumerate() {
        let path = dir.join(format!("h{i}.pdf"));
        std::fs::write(&path, c).unwrap();
        let mut e = Engine::new();
        let _ = e.execute("file.open", &json!({"path": path.to_str().unwrap(), "vectors": true}));
        let _ = e.execute("file.merge", &json!({"path": path.to_str().unwrap(), "vectors": true}));
    }
    let mut e = Engine::new();
    assert!(e
        .execute("file.open_data", &json!({"data": "%PDF-1.4 nonsense", "vectors": true}))
        .is_err());
}

fn texts(e: &Engine) -> Vec<rustfig_engine::model::Text> {
    e.doc()
        .objects
        .iter()
        .filter_map(|o| match &o.kind {
            ObjectKind::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn tex_style_text_gets_spaces_ligatures_fonts_and_depths() {
    let dir = temp_dir("tex");
    // Words set with a kern instead of a space glyph, a fi ligature, a far
    // away piece on the same line, other fonts, then a white box on top.
    let content = "BT /F1 10 Tf 20 80 Td [(Hello) -400 (World) -300 (\\256nal) -9000 (Far)] TJ ET \
        BT /F2 10 Tf 20 60 Td (code) Tj ET BT /F3 10 Tf 20 40 Td (Bold) Tj ET \
        1 g 0 0 200 20 re f";
    let (e, _) = open_bytes(&tiny_pdf(content, false), &dir, "tex.pdf");
    let t = texts(&e);
    let strings: Vec<&str> = t.iter().map(|t| t.text.as_str()).collect();
    assert!(strings.contains(&"Hello World final"), "{strings:?}");
    assert!(strings.contains(&"Far"), "{strings:?}");
    assert!(strings.contains(&"code"));
    let font = |s: &str| t.iter().find(|t| t.text == s).unwrap().font;
    assert_eq!(font("Hello World final"), 18, "Helvetica-Bold");
    assert_eq!(font("code"), 12, "CMTT10: Courier");
    assert_eq!(font("Bold"), 2, "CMBX12: Times Bold");
    // The white box was painted last: it is above every text (smaller depth).
    let box_depth = e
        .doc()
        .objects
        .iter()
        .find_map(|o| match &o.kind {
            ObjectKind::Polyline(p) if p.kind == PolyKind::Box => Some(p.style.depth),
            _ => None,
        })
        .unwrap();
    assert!(
        t.iter().all(|t| box_depth < t.depth),
        "box {box_depth} texts {:?}",
        t.iter().map(|t| t.depth).collect::<Vec<_>>()
    );
}
