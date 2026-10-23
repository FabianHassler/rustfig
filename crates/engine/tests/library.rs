#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Library mode (plan D6): xfig's figure libraries, `w_library.c` and
//! `e_placelib.c`. The bundled libraries are `libraries/` in the source tree.

use rustfig_engine::geom::Point;
use rustfig_engine::{Button, Engine, Key, ToolEvent, UiRequest};
use serde_json::{json, Value};

/// An engine whose user library folder is `dir` (never the real home).
fn engine_with_user_dir(dir: &std::path::Path) -> Engine {
    let mut e = Engine::new();
    e.prefs
        .set("library_dir", toml::Value::String(dir.to_string_lossy().into_owned()))
        .unwrap();
    e
}

fn scratch(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rustfig-library-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn press(e: &mut Engine, x: f64, y: f64, button: Button, shift: bool) {
    e.tool_event(ToolEvent::Press {
        pos: Point::new(x, y),
        button,
        shift,
    })
    .unwrap();
}

#[test]
fn the_bundled_libraries_are_listed() {
    let user = scratch("list");
    let mut e = engine_with_user_dir(&user);
    let r = e.execute("library.list", &json!({})).unwrap();
    let libs = r["libraries"].as_array().unwrap();
    let names: Vec<&str> = libs.iter().map(|l| l["name"].as_str().unwrap()).collect();
    for want in [
        "Computers",
        "Optics",
        "Electronic / Schematic / Classic",
        "Flags / Africa",
    ] {
        assert!(names.contains(&want), "{want} missing in {names:?}");
    }
    // Charts is GPL and left out.
    assert!(!names.contains(&"Charts"));
    assert!(libs.iter().all(|l| l["objects"].as_u64().unwrap() > 0));
    let o = e
        .execute("library.objects", &json!({"library": "Computers", "comments": true}))
        .unwrap();
    let objs = o["objects"].as_array().unwrap();
    assert!(objs.iter().any(|o| o["name"] == "floppy_lg"), "{objs:?}");
    let floppy = objs.iter().find(|o| o["name"] == "floppy_lg").unwrap();
    assert!(floppy["comments"][0].as_str().unwrap().contains("floppy"));
    assert!(e.execute("library.objects", &json!({"library": "Nope"})).is_err());
    let _ = std::fs::remove_dir_all(&user);
}

#[test]
fn placing_an_object_is_one_undoable_compound() {
    let user = scratch("place");
    let mut e = engine_with_user_dir(&user);
    let r = e
        .execute(
            "library.place",
            &json!({"library": "Computers", "name": "floppy_lg", "x": 1200, "y": 2400}),
        )
        .unwrap();
    assert_eq!(e.doc().objects.len(), 1);
    let id = r["id"].as_u64().unwrap();
    let o = e.doc().get(rustfig_engine::model::ObjectId(id)).unwrap();
    assert!(matches!(o.kind, rustfig_engine::model::ObjectKind::Compound(_)));
    let bb = o.geometry_bbox();
    assert!(
        (bb.min.x - 1200.0).abs() < 1.0 && (bb.min.y - 2400.0).abs() < 1.0,
        "{bb:?}"
    );
    assert!(o.comments.iter().any(|c| c.contains("floppy")));
    assert!(e.undo());
    assert!(e.doc().objects.is_empty());
    assert!(e.redo());
    assert_eq!(e.doc().objects.len(), 1);
    // At its original position: the file's own coordinates.
    e.execute(
        "library.place",
        &json!({"library": "Computers", "name": "floppy_lg", "orig": true}),
    )
    .unwrap();
    assert_eq!(e.doc().objects.len(), 2);
    let _ = std::fs::remove_dir_all(&user);
}

#[test]
fn ops_turn_and_flip_the_object() {
    let user = scratch("ops");
    let mut e = engine_with_user_dir(&user);
    let size = |e: &mut Engine, ops: Value| {
        let r = e.execute(
            "library.place",
            &json!({"library": "Arrows", "name": "3darrow1", "x": 0, "y": 0, "ops": ops}),
        );
        let r = match r {
            Ok(r) => r,
            Err(_) => {
                let names = e.execute("library.objects", &json!({"library": "Arrows"})).unwrap();
                panic!("{names}");
            }
        };
        let b = e
            .doc()
            .get(rustfig_engine::model::ObjectId(r["id"].as_u64().unwrap()))
            .unwrap()
            .geometry_bbox();
        (b.max.x - b.min.x, b.max.y - b.min.y)
    };
    let (w, h) = size(&mut e, json!([]));
    let (w2, h2) = size(&mut e, json!(["rotate_cw"]));
    assert!((w - h2).abs() < 2.0 && (h - w2).abs() < 2.0, "{w}x{h} -> {w2}x{h2}");
    let (w3, h3) = size(&mut e, json!(["larger"]));
    assert!(w3 > w * 1.05 && h3 > h * 1.05);
    assert!(e
        .execute(
            "library.place",
            &json!({"library": "Arrows", "name": "3darrow1", "x": 0, "y": 0, "ops": ["spin"]})
        )
        .is_err());
    let _ = std::fs::remove_dir_all(&user);
}

#[test]
fn the_object_follows_the_pointer_and_the_buttons_do_what_xfig_does() {
    let user = scratch("hand");
    let mut e = engine_with_user_dir(&user);
    let r = e
        .execute("library.hand", &json!({"library": "Computers", "name": "floppy_lg"}))
        .unwrap();
    assert_eq!(r["attached"], true);
    assert!(e.library_in_hand().is_some());
    assert!(e.doc().objects.is_empty());
    // xfig's hints: place object / new object / cancel library (and Shift).
    let h = e.mouse_hints();
    assert_eq!(
        (h.left, h.middle, h.right),
        ("place object", "new object", "cancel library")
    );
    let h = e.mouse_hints_with(true);
    assert_eq!(
        (h.left, h.middle, h.right),
        ("place and edit", "change draw mode", "place at orig posn")
    );
    // The preview follows the pointer.
    e.tool_event(ToolEvent::Move {
        pos: Point::new(3000.0, 3000.0),
    })
    .unwrap();
    let ov = e.overlay();
    assert_eq!(ov.preview.len(), 1);
    let bb = ov.preview[0].geometry_bbox();
    assert!((bb.min.x - 3000.0).abs() < 200.0, "{bb:?}");
    // Left places and stays in the mode.
    press(&mut e, 3000.0, 3000.0, Button::Left, false);
    press(&mut e, 6000.0, 3000.0, Button::Left, false);
    assert_eq!(e.doc().objects.len(), 2);
    assert!(e.library_in_hand().is_some());
    // r turns the object in hand.
    e.tool_event(ToolEvent::Text("r".into())).unwrap();
    assert_eq!(e.library_in_hand().unwrap().2.len(), 1);
    // Shift+middle shows a box instead of the object.
    press(&mut e, 0.0, 0.0, Button::Middle, true);
    assert!(e.overlay().preview.is_empty());
    assert!(!e.overlay().guides.is_empty());
    // Shift+right places at the original position.
    press(&mut e, 0.0, 0.0, Button::Right, true);
    assert_eq!(e.doc().objects.len(), 3);
    // Shift+left places and asks for the edit panel.
    let _ = e.take_requests();
    press(&mut e, 9000.0, 0.0, Button::Left, true);
    assert_eq!(e.doc().objects.len(), 4);
    assert!(e
        .take_requests()
        .iter()
        .any(|r| matches!(r, UiRequest::EditObject { .. })));
    // Middle: back to the library panel.
    press(&mut e, 0.0, 0.0, Button::Middle, false);
    assert!(e.library_in_hand().is_none());
    assert!(e.take_requests().contains(&UiRequest::OpenLibrary));
    // Right cancels.
    e.execute("library.hand", &json!({"library": "Computers", "name": "floppy_lg"}))
        .unwrap();
    press(&mut e, 0.0, 0.0, Button::Right, false);
    assert!(e.library_in_hand().is_none());
    assert_eq!(e.doc().objects.len(), 4);
    // Escape cancels too.
    e.execute("library.hand", &json!({"library": "Computers", "name": "floppy_lg"}))
        .unwrap();
    e.tool_event(ToolEvent::Key(Key::Escape)).unwrap();
    assert!(e.library_in_hand().is_none());
    // Each placement is its own undo step.
    assert!(e.undo());
    assert_eq!(e.doc().objects.len(), 3);
    let _ = std::fs::remove_dir_all(&user);
}

#[test]
fn a_figure_copied_to_the_user_library_is_listed_and_placeable() {
    let user = scratch("save");
    let mut e = engine_with_user_dir(&user);
    e.execute(
        "shape.rectangle",
        &json!({"x": 600, "y": 600, "width": 1200, "height": 600}),
    )
    .unwrap();
    let r = e
        .execute(
            "library.save",
            &json!({"name": "my box", "library": "Mine/Boxes", "comments": "A box."}),
        )
        .unwrap();
    assert!(std::path::Path::new(r["path"].as_str().unwrap()).is_file());
    let list = e.execute("library.list", &json!({})).unwrap();
    let mine = list["libraries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["name"] == "Mine / Boxes")
        .unwrap();
    assert_eq!(mine["user"], true);
    let objs = e
        .execute("library.objects", &json!({"library": "Mine / Boxes", "comments": true}))
        .unwrap();
    assert_eq!(objs["objects"][0]["name"], "my box");
    assert_eq!(objs["objects"][0]["comments"][0], "A box.");
    let mut e2 = engine_with_user_dir(&user);
    e2.execute(
        "library.place",
        &json!({"library": "Mine / Boxes", "name": "my box", "x": 0, "y": 0}),
    )
    .unwrap();
    assert_eq!(e2.doc().objects.len(), 1);
    // Names that would leave the folder are refused.
    for bad in ["../x", "a/b", ".hidden", "", "x\u{0}y"] {
        assert!(e.execute("library.save", &json!({"name": bad})).is_err(), "{bad:?}");
    }
    assert!(e
        .execute("library.save", &json!({"name": "ok", "library": "../.."}))
        .is_err());
    let _ = std::fs::remove_dir_all(&user);
}

#[test]
fn hostile_library_params_fail_cleanly() {
    let user = scratch("hostile");
    let hostile = [
        json!({}),
        json!({"library": 5, "name": []}),
        json!({"library": "Computers", "name": "../../x"}),
        json!({"library": "Computers", "name": "floppy_lg", "x": 1e308}),
        json!({"library": "Computers", "name": "floppy_lg", "x": 5}),
        json!({"library": "Computers", "name": "floppy_lg", "x": "a", "y": null}),
        json!({"library": "Computers", "name": "floppy_lg", "orig": "yes"}),
        json!({"library": "Computers", "name": "floppy_lg", "x": 0, "y": 0, "ops": "r"}),
        json!({"library": "Computers", "name": "floppy_lg", "x": 0, "y": 0, "ops": [1, null]}),
        json!({"library": "Computers", "name": "floppy_lg", "x": 0, "y": 0, "ops": vec!["smaller"; 300]}),
        json!({"library": "../Computers", "name": "floppy_lg"}),
        json!({"library": "Computers", "comments": 3}),
        json!({"op": "x"}),
    ];
    for id in [
        "library.list",
        "library.objects",
        "library.place",
        "library.hand",
        "library.transform",
        "library.panel",
        "library.save",
    ] {
        for p in &hostile {
            let mut e = engine_with_user_dir(&user);
            e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 100}))
                .unwrap();
            let before = e.doc().clone();
            let r = e.execute(id, p);
            if r.is_err() {
                assert_eq!(e.doc(), &before, "{id} {p}");
            }
            let _ = e.overlay();
        }
    }
    let _ = std::fs::remove_dir_all(&user);
}

#[test]
fn every_bundled_figure_reads() {
    // The sweep over the whole bundle: every figure loads into a compound.
    let user = scratch("all");
    let mut e = engine_with_user_dir(&user);
    let libs = e.execute("library.list", &json!({})).unwrap();
    let mut n = 0;
    let mut failed = Vec::new();
    for l in libs["libraries"].as_array().unwrap() {
        let name = l["name"].as_str().unwrap();
        let lib = e.find_library(name).unwrap();
        let objs = e.execute("library.objects", &json!({"library": name})).unwrap();
        for o in objs["objects"].as_array().unwrap() {
            let oname = o["name"].as_str().unwrap();
            n += 1;
            if e.library_object_in(&lib, oname).is_err() {
                failed.push(format!("{name}/{oname}"));
            }
        }
    }
    assert!(n > 2400, "only {n} figures");
    // xfig itself skips an empty library file; a handful may be empty.
    assert!(failed.len() < 10, "{failed:?}");
    let _ = std::fs::remove_dir_all(&user);
}

#[test]
fn search_finds_figures_in_all_libraries_by_name_and_comment() {
    let user = scratch("search");
    let sub = user.join("Mine");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(
        sub.join("gizmo.fig"),
        "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n# a Zorgblat for tests\n1200 2\n2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t0 0 600 0 600 600 0 600 0 0\n",
    )
    .unwrap();
    let mut e = engine_with_user_dir(&user);
    let search = |e: &mut Engine, p: Value| e.execute("library.search", &p).unwrap();
    // By name, across libraries, case-insensitive, every word.
    let r = search(&mut e, json!({"query": "FLOPPY lg"}));
    let hits = r["hits"].as_array().unwrap();
    assert!(
        hits.iter()
            .any(|h| h["library"] == "Computers" && h["name"] == "floppy_lg"),
        "{hits:?}"
    );
    // By comment, in the user's library.
    let r = search(&mut e, json!({"query": "zorgblat"}));
    assert_eq!(r["hits"][0]["name"], "gizmo");
    assert_eq!(r["hits"][0]["library"], "Mine");
    assert_eq!(r["hits"][0]["user"], true);
    assert_eq!(r["hits"][0]["comment"], true);
    // Without comments it finds nothing; a name match is not a comment match.
    let r = search(&mut e, json!({"query": "zorgblat", "comments": false}));
    assert!(r["hits"].as_array().unwrap().is_empty());
    let r = search(&mut e, json!({"query": "gizmo"}));
    assert_eq!(r["hits"][0]["comment"], false);
    // Limits, truncation, empty and hostile input.
    let r = search(&mut e, json!({"query": "a", "limit": 3}));
    assert_eq!(r["hits"].as_array().unwrap().len(), 3);
    assert_eq!(r["truncated"], true);
    assert!(search(&mut e, json!({"query": "  "}))["hits"]
        .as_array()
        .unwrap()
        .is_empty());
    let r = search(&mut e, json!({"query": "e", "limit": 1e6}));
    assert!(r["hits"].as_array().unwrap().len() <= 1000);
    assert!(e.execute("library.search", &json!({})).is_err());
    assert!(e
        .execute("library.search", &json!({"query": "a", "limit": -1}))
        .is_err());
    assert!(e.execute("library.search", &json!({"query": 5})).is_err());
    // A hit is placeable.
    e.execute(
        "library.place",
        &json!({"library": "Mine", "name": "gizmo", "x": 0, "y": 0}),
    )
    .unwrap();
    let _ = std::fs::remove_dir_all(&user);
}

// ---- library list files (xfig's library_dir naming a file) and Stop ----

fn fig_in(dir: &std::path::Path, name: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let fig = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/Arrows/3darrow1.fig"),
    )
    .unwrap();
    std::fs::write(dir.join(name), fig).unwrap();
}

#[test]
fn parse_list_follows_xfigs_format() {
    use rustfig_engine::library::parse_list;
    let base = std::path::Path::new("/base");
    let t = "# comment\n/abs/one\n\n  /abs/two  My Two  \nrel/three/\n#/abs/skipped\n/abs/four\tTab Name\n";
    let v = parse_list(t, base);
    let got: Vec<(String, String)> = v
        .iter()
        .map(|e| (e.name.clone(), e.path.to_string_lossy().into_owned()))
        .collect();
    assert_eq!(
        got,
        vec![
            ("one".into(), "/abs/one".into()),
            ("My Two".into(), "/abs/two".into()),
            ("three".into(), "/base/rel/three/".into()),
            ("Tab Name".into(), "/abs/four".into()),
        ]
    );
}

#[test]
fn parse_list_survives_hostile_input() {
    use rustfig_engine::library::{parse_list, MAX_LIST_ENTRIES};
    let base = std::path::Path::new("/b");
    assert!(parse_list("", base).is_empty());
    assert!(parse_list("\u{0}\u{1}\n\r\n   \n/", base).is_empty());
    assert!(parse_list(&"x".repeat(100_000), base).is_empty(), "overlong line");
    assert!(parse_list("/a ..\n/a .\n/a (Toplevel Directory)\n/a \u{7}bell\n..\n", base).is_empty());
    let many = "/a/x\n".repeat(MAX_LIST_ENTRIES * 3);
    assert_eq!(parse_list(&many, base).len(), MAX_LIST_ENTRIES);
    let long_name = format!("/a {}", "n".repeat(500));
    assert!(parse_list(&long_name, base).is_empty());
    // multi-byte characters at every position must not split a char.
    let _ = parse_list("/a/é ü ö\n/é\u{3000}名前\n~\n~/x", base);
}

#[test]
fn a_list_file_names_several_library_folders() {
    let d = scratch("listfile");
    fig_in(&d.join("alpha"), "a1.fig");
    fig_in(&d.join("alpha/inner"), "a2.fig");
    fig_in(&d.join("beta"), "b1.fig");
    std::fs::write(
        d.join("libs.txt"),
        format!(
            "# my libraries\n{}  First Set\nbeta\n{}\n",
            d.join("alpha").display(),
            d.join("missing").display()
        ),
    )
    .unwrap();
    let mut e = engine_with_user_dir(&d.join("libs.txt"));
    let src = e.execute("library.source", &json!({})).unwrap();
    assert_eq!(src["kind"], "list");
    let entries = src["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[1]["name"], "beta");
    assert_eq!(entries[1]["exists"], true, "relative to the list file");
    assert_eq!(entries[2]["exists"], false);
    let r = e.execute("library.list", &json!({})).unwrap();
    let user: Vec<(String, u64)> = r["libraries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["user"] == true)
        .map(|l| (l["name"].as_str().unwrap().to_string(), l["objects"].as_u64().unwrap()))
        .collect();
    // In the list file's order, as xfig keeps it.
    assert_eq!(
        user,
        vec![
            ("First Set".to_string(), 1),
            ("First Set / inner".to_string(), 1),
            ("beta".to_string(), 1)
        ]
    );
    let o = e
        .execute("library.objects", &json!({"library": "First Set / inner"}))
        .unwrap();
    assert_eq!(o["objects"][0]["name"], "a2");
    // Copy to library goes to the first folder named.
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 600}))
        .unwrap();
    let s = e.execute("library.save", &json!({"name": "mine"})).unwrap();
    assert_eq!(s["library"], "First Set");
    assert!(d.join("alpha/mine.fig").exists());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn library_use_checks_the_path_and_sets_the_preference() {
    let d = scratch("use");
    fig_in(&d.join("folder/sub"), "s.fig");
    std::fs::write(d.join("empty.txt"), "# nothing\n").unwrap();
    std::fs::write(d.join("ok.txt"), format!("{} Sub\n", d.join("folder/sub").display())).unwrap();
    let mut e = engine_with_user_dir(&d.join("nowhere"));
    assert_eq!(e.execute("library.source", &json!({})).unwrap()["kind"], "missing");
    assert!(e
        .execute("library.use", &json!({"path": d.join("nowhere").to_string_lossy()}))
        .is_err());
    assert!(e
        .execute("library.use", &json!({"path": d.join("empty.txt").to_string_lossy()}))
        .is_err());
    assert!(e.execute("library.use", &json!({"path": ""})).is_err());
    assert!(e.execute("library.use", &json!({"path": 5})).is_err());
    let r = e
        .execute("library.use", &json!({"path": d.join("ok.txt").to_string_lossy()}))
        .unwrap();
    assert_eq!(r["kind"], "list");
    assert!(r["libraries"].as_u64().unwrap() >= 31);
    let r = e
        .execute("library.use", &json!({"path": d.join("folder").to_string_lossy()}))
        .unwrap();
    assert_eq!(r["kind"], "folder");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_list_file_that_is_binary_or_huge_is_harmless() {
    let d = scratch("hostile-list");
    let big = d.join("big.txt");
    std::fs::write(&big, vec![b'a'; 300 * 1024]).unwrap();
    let bin = d.join("bin.txt");
    std::fs::write(&bin, [0xffu8, 0xfe, b'\n', 0, 1, 2, b'/', b'\n']).unwrap();
    for f in [big, bin] {
        let mut e = engine_with_user_dir(&f);
        let r = e.execute("library.list", &json!({})).unwrap();
        assert!(r["libraries"].as_array().unwrap().iter().all(|l| l["user"] == false));
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_scan_can_be_stopped() {
    use rustfig_engine::library::{scan, scan_cancellable, LibRoot};
    use std::sync::atomic::AtomicBool;
    let d = scratch("stop");
    for i in 0..20 {
        fig_in(&d.join(format!("lib{i:02}")), "f.fig");
    }
    let roots = vec![LibRoot {
        path: d.clone(),
        user: true,
        name: None,
    }];
    assert_eq!(scan(&roots).len(), 20);
    let stop = AtomicBool::new(true);
    assert!(scan_cancellable(&roots, &stop).is_empty(), "stopped before it began");
    let _ = std::fs::remove_dir_all(&d);
}
