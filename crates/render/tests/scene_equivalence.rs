//! fig2dev's scene builder (`rustfig_fig2dev::scene::build`, from the
//! objects fig2dev's reader returns) draws every object as RustFig's own
//! builder does (`build_scene`, from the document the same objects
//! become): the same primitives, up to the order of objects at one depth
//! (fig2dev's `compound_dump` order against xfig's).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rustfig_fig2dev::scene::build::TextOptions;
use rustfig_fig2dev::Ctx;
use rustfig_render::{build_scene, Scene, SceneOptions};
use std::path::Path;

fn sorted(scene: &Scene) -> Vec<String> {
    let mut v: Vec<String> = scene.prims.iter().map(|p| format!("{p:?}")).collect();
    v.sort();
    v
}

fn check(name: &str, src: &[u8]) {
    let mut ctx = Ctx::default();
    let objects = rustfig_fig2dev::read::readfp_fig(&mut ctx, src).unwrap();
    let doc = rustfig_fig2dev::to_model::to_document(&ctx, &objects);
    let ours = build_scene(&doc, &SceneOptions::default());
    let (theirs, _) = rustfig_fig2dev::scene::build(&mut ctx, &objects, TextOptions::default());
    assert_eq!(ours.prims.len(), theirs.prims.len(), "{name}");
    let (a, b) = (sorted(&ours), sorted(&theirs));
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x, y, "{name}");
    }
}

#[test]
fn fig2devs_builder_draws_as_rustfigs() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../engine/tests/fixtures");
    let mut n = 0;
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "fig") {
            check(&p.display().to_string(), &std::fs::read(&p).unwrap());
            n += 1;
        }
    }
    assert!(n >= 10, "{n} fixtures");
    // Old formats: Fig 2.1 at 80 ppi with arrows and an old spline, and a
    // Fig 1.3 spline.
    check(
        "fig 2.1",
        b"#FIG 2.1\n80 2\n\
        2 2 0 1 -1 0 0 0 0.000 7 0 0\n\t 10 10 90 10 90 50 10 50 10 10 9999 9999\n\
        2 1 1 2 4 0 0 0 4.000 -1 1 0\n\t0 0 1.000 4.000 8.000\n\t 10 70 50 70 9999 9999\n\
        3 0 0 1 -1 0 0 0 0.000 0 0\n\t 10 100 50 120 90 100 9999 9999\n\
        4 0 -1 0 0 0 12 0.00000 4 9 30 10 150 Hello\x01\n",
    );
    check("fig 1.3", b"1200 2 9 9\n3 10 0 1 1 0 0 0 0 0 0\n100 100 200 0 9999 0\n");
}
