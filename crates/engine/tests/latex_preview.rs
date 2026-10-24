#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! F8: special texts typeset by TeX on the canvas, with a fake TeX program.
#![cfg(unix)]

use rustfig_engine::latex_preview::latex_document;
use rustfig_engine::render::{build_scene, Prim, SceneOptions};
use rustfig_engine::Engine;
use serde_json::json;
use std::path::{Path, PathBuf};

const DRAWING: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
    4 0 0 50 -1 0 12 0.0000 4 135 450 300 600 plain\\001\n\
    4 1 0 50 -1 0 12 0.0000 2 135 450 3000 900 $\\\\alpha^2$\\001\n";

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rustfig-f8-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A PDF made by the engine itself (a rectangle on a letter page).
fn some_pdf() -> Vec<u8> {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 600, "y": 600, "width": 3000, "height": 1500}),
    )
    .unwrap();
    let r = e.execute("print.pdf", &json!({})).unwrap();
    rustfig_engine::base64_decode(r["base64"].as_str().unwrap())
}

/// A script that acts as TeX: copies `pdf` to doc.pdf, writes the sizes and
/// counts its runs in `count`; or fails with a log.
fn fake_tex(dir: &Path, pdf: &Path, ok: bool) -> PathBuf {
    let count = dir.join("count");
    let script = dir.join(if ok { "faketex" } else { "badtex" });
    let body = if ok {
        format!(
            "#!/bin/sh\necho run >> '{}'\ncp '{}' doc.pdf\necho '10.0pt 20.0pt 2.0pt' > doc.dim\n",
            count.display(),
            pdf.display()
        )
    } else {
        "#!/bin/sh\necho '! Undefined control sequence.' > doc.log\nexit 1\n".to_string()
    };
    std::fs::write(&script, body).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

fn engine_with(program: &Path) -> Engine {
    let mut e = Engine::new();
    e.execute("file.open_data", &json!({"data": DRAWING})).unwrap();
    e.execute("view.set_latex_program", &json!({"program": program.to_str().unwrap()}))
        .unwrap();
    e
}

fn scene_of(e: &Engine) -> rustfig_engine::render::Scene {
    build_scene(
        e.doc(),
        &SceneOptions {
            screen: true,
            typeset: e.latex.typesetter(),
            ..SceneOptions::default()
        },
    )
}

/// (texts, special texts, pictures)
fn count(s: &rustfig_engine::render::Scene) -> (usize, usize, usize) {
    let (mut texts, mut special, mut images) = (0, 0, 0);
    for p in &s.prims {
        match p {
            Prim::Text(t) => {
                texts += 1;
                special += usize::from(t.special);
            }
            Prim::PdfPicture { .. } => images += 1,
            _ => {}
        }
    }
    (texts, special, images)
}

fn wait(e: &Engine) {
    for _ in 0..500 {
        if e.latex.pending() == 0 {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("TeX job did not finish");
}

#[test]
fn a_special_text_becomes_a_picture_once_typeset_and_is_cached() {
    let dir = scratch("ok");
    let pdf = dir.join("in.pdf");
    std::fs::write(&pdf, some_pdf()).unwrap();
    let tex = fake_tex(&dir, &pdf, true);
    let e = engine_with(&tex);
    assert!(e.latex.active());
    // First: the source shows while TeX runs.
    assert_eq!(count(&scene_of(&e)), (2, 1, 0));
    wait(&e);
    // Then: the special text is a picture, the plain text stays text.
    let s = scene_of(&e);
    let msgs = e.latex.take_messages();
    assert!(msgs.is_empty(), "{msgs:?}");
    assert_eq!(count(&s), (1, 0, 1));
    let Some(Prim::PdfPicture { corners, pdf, .. }) = s.prims.iter().find(|p| matches!(p, Prim::PdfPicture { .. }))
    else {
        panic!("no picture");
    };
    assert!(pdf.size().0 > 1.0 && pdf.size().1 > 1.0);
    // Centred on x = 3000, the baseline (y = 900) inside the picture.
    let cx = (corners[0].x + corners[1].x) / 2.0;
    assert!((cx - 3000.0).abs() < 1.0, "{cx}");
    assert!(corners[3].y > 900.0 && corners[0].y < 900.0, "{corners:?}");
    // Cached: more scenes do not run TeX again.
    let _ = scene_of(&e);
    let _ = scene_of(&e);
    wait(&e);
    assert_eq!(std::fs::read_to_string(dir.join("count")).unwrap().lines().count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bounds_and_picking_follow_the_typeset_size() {
    let dir = scratch("bounds");
    let pdf = dir.join("in.pdf");
    std::fs::write(&pdf, some_pdf()).unwrap();
    let tex = fake_tex(&dir, &pdf, true);
    let mut e = engine_with(&tex);
    let id = e
        .doc()
        .objects
        .iter()
        .find(|o| {
            e.typeset_box(o).is_none()
                && matches!(&o.kind, rustfig_engine::model::ObjectKind::Text(t) if t.is_special())
        })
        .unwrap()
        .id;
    // Far right of the stored box (about 450 wide, centred on 3000).
    let far = rustfig_engine::geom::Point::new(3000.0 + 4000.0, 900.0);
    assert_eq!(e.hit_test(far, 10.0), None);
    let _ = scene_of(&e);
    wait(&e);
    let o = e.doc().get(id).unwrap().clone();
    let (corners, asc) = e.typeset_box(&o).expect("typeset box");
    // The page is 792 x 612 pt, 2 pt of it below the baseline.
    let w = (corners[1].x - corners[0].x).abs();
    assert!((w - 792.0 * 1200.0 / 72.0).abs() < 1.0, "{w}");
    assert!(asc > 1000.0);
    assert_eq!(e.hit_test(far, 10.0), Some(id));
    // The stored size and fig2dev's bounds are unchanged.
    let bounds = e.object_fig_bounds(id).unwrap();
    assert!(bounds.width() < 1000.0, "{bounds:?}");
    // Zoom to fit and region selection use the typeset box.
    let fit = e.fit_bounds();
    assert!(fit.width() >= w - 1.0, "{fit:?}");
    assert!(e.doc_bounds().width() < fit.width());
    let small = rustfig_engine::geom::Rect::from_points([
        rustfig_engine::geom::Point::new(2000.0, 0.0),
        rustfig_engine::geom::Point::new(4000.0, 2000.0),
    ]);
    assert!(!e.objects_within(&small).contains(&id));
    // Off: back to the stored box.
    e.execute("view.toggle_latex_preview", &json!({"on": false})).unwrap();
    assert_eq!(e.hit_test(far, 10.0), None);
    assert_eq!(e.fit_bounds(), e.doc_bounds());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_tex_error_keeps_the_source_and_tells_once() {
    let dir = scratch("bad");
    let pdf = dir.join("in.pdf");
    std::fs::write(&pdf, some_pdf()).unwrap();
    let tex = fake_tex(&dir, &pdf, false);
    let mut e = engine_with(&tex);
    let _ = scene_of(&e);
    wait(&e);
    assert_eq!(count(&scene_of(&e)), (2, 1, 0));
    e.poll_latex();
    assert!(e.status().contains("Undefined control sequence"), "{}", e.status());
    e.set_status("other");
    e.poll_latex();
    e.poll_latex();
    assert_eq!(e.status(), "other");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn off_or_without_tex_nothing_is_typeset() {
    let dir = scratch("off");
    let pdf = dir.join("in.pdf");
    std::fs::write(&pdf, some_pdf()).unwrap();
    let tex = fake_tex(&dir, &pdf, true);
    let mut e = engine_with(&tex);
    e.execute("view.toggle_latex_preview", &json!({"on": false})).unwrap();
    assert!(e.latex.typesetter().is_none());
    assert_eq!(count(&scene_of(&e)), (2, 1, 0));
    e.execute("view.toggle_latex_preview", &json!({"on": true})).unwrap();
    e.execute("view.set_latex_program", &json!({"program": "/nonexistent/tex"}))
        .unwrap();
    assert!(e.latex.typesetter().is_none());
    assert!(e.execute("view.set_latex_program", &json!({"program": 3})).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn files_and_exports_do_not_depend_on_the_preview() {
    let dir = scratch("same");
    let pdf = dir.join("in.pdf");
    std::fs::write(&pdf, some_pdf()).unwrap();
    let tex = fake_tex(&dir, &pdf, true);
    let mut e = engine_with(&tex);
    let export = |e: &mut Engine| {
        ["svg", "eps"]
            .iter()
            .map(|f| {
                let v = e.execute("file.export_data", &json!({"format": f})).unwrap();
                // Without the time stamp of the run.
                v["data"]
                    .as_str()
                    .unwrap()
                    .lines()
                    .filter(|l| !l.contains("CreationDate"))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
    };
    let on = export(&mut e);
    let _ = scene_of(&e);
    wait(&e);
    assert_eq!(export(&mut e), on);
    e.execute("view.toggle_latex_preview", &json!({"on": false})).unwrap();
    assert_eq!(export(&mut e), on);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_document_has_the_export_preamble_and_the_colour() {
    let d = latex_document("\\usepackage{bm}", "$x$", 4, false, 12.0, [255, 0, 0]);
    for want in [
        "amsmath,amssymb,color,graphicx",
        "\\usepackage{bm}",
        "\\sffamily",
        "{1.000,0.000,0.000}",
        "$x$",
        "\\fontsize{12.00}{14.40}",
    ] {
        assert!(d.contains(want), "{want} in {d}");
    }
}

/// With a real TeX installed: `cargo test -p rustfig-engine --test latex_preview -- --ignored`.
/// Writes the typeset picture to `$F8_PNG` when set.
#[test]
#[ignore = "needs pdflatex or lualatex"]
fn real_tex_typesets_an_integral() {
    let mut e = Engine::new();
    let data = DRAWING.replace("$\\\\alpha^2$", "$\\\\int_0^1 x^2\\\\,dx$");
    e.execute("file.open_data", &json!({"data": data})).unwrap();
    assert!(e.latex.active(), "no TeX found");
    let _ = scene_of(&e);
    for _ in 0..1500 {
        if e.latex.pending() == 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let msgs = e.latex.take_messages();
    assert!(msgs.is_empty(), "{msgs:?}");
    let s = scene_of(&e);
    let Some(Prim::PdfPicture { pdf, .. }) = s.prims.iter().find(|p| matches!(p, Prim::PdfPicture { .. })) else {
        panic!("not typeset");
    };
    let (w, h) = pdf.size();
    assert!((20.0..80.0).contains(&w) && (5.0..30.0).contains(&h), "{w} x {h} pt");
    // Sharp at any size: rendered at the size asked, with ink in it.
    let img = pdf.render(400, 150).unwrap();
    let inked = img.rgba.chunks(4).filter(|p| p[3] > 128).count();
    assert!(inked > 200, "{inked} inked pixels");
    if let Ok(path) = std::env::var("F8_PNG") {
        let img = image::RgbaImage::from_raw(img.width, img.height, img.rgba.clone()).unwrap();
        img.save(path).unwrap();
    }
}

/// E12: a plot with LaTeX labels and numbers makes special texts, which the
/// canvas hands to the typesetter like any other special text.
#[test]
fn plot_labels_and_numbers_are_special_texts_the_typesetter_is_asked_for() {
    use rustfig_model::{ObjectKind, Text};
    let dir = scratch("plot");
    let pdf = dir.join("in.pdf");
    std::fs::write(&pdf, some_pdf()).unwrap();
    let tex = fake_tex(&dir, &pdf, true);
    let mut e = Engine::new();
    e.execute("view.set_latex_program", &json!({"program": tex.to_str().unwrap()}))
        .unwrap();
    let r = e
        .execute(
            "plot.add",
            &json!({"expr": "sin(x)", "xlabel": "$x$", "ylabel": "$\\sin x$", "latex": true, "latex_numbers": true}),
        )
        .unwrap();
    let o = e.doc().get(rustfig_model::ObjectId(r["id"].as_u64().unwrap())).unwrap();
    let ObjectKind::Compound(c) = &o.kind else { panic!() };
    let all: Vec<Text> = c
        .objects
        .iter()
        .filter_map(|m| match &m.kind {
            ObjectKind::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert!(all.len() > 8);
    for t in &all {
        assert!(t.is_special() && !t.is_postscript() && t.font == 0, "{t:?}");
        assert!(t.text.starts_with('$') && t.text.ends_with('$'), "{t:?}");
    }
    assert!(all.iter().any(|t| t.text == "$-1$" || t.text == "$0$"));
    // The canvas shows the sources, then one picture per text.
    let n = all.len();
    assert_eq!(count(&scene_of(&e)), (n, n, 0));
    wait(&e);
    assert_eq!(count(&scene_of(&e)), (0, 0, n));
    assert!(e.latex.take_messages().is_empty());
    // Labels only: the numbers stay plain texts.
    let mut e2 = Engine::new();
    let r2 = e2
        .execute("plot.add", &json!({"expr": "x", "xlabel": "x", "latex": true}))
        .unwrap();
    let o = e2
        .doc()
        .get(rustfig_model::ObjectId(r2["id"].as_u64().unwrap()))
        .unwrap();
    let ObjectKind::Compound(c) = &o.kind else { panic!() };
    let special = c
        .objects
        .iter()
        .filter(|m| matches!(&m.kind, ObjectKind::Text(t) if t.is_special()))
        .count();
    assert_eq!(special, 1);
    // Numbers as LaTeX: powers of ten.
    assert_eq!(rustfig_engine::plot::latex_number("1e-5"), "$10^{-5}$");
    assert_eq!(rustfig_engine::plot::latex_number("2e6"), "$2\\times10^{6}$");
    let _ = std::fs::remove_dir_all(&dir);
}
