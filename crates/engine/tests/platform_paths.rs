#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! P2: file names and folders the systems allow: the command line, open,
//! save, recovery and settings with non-ASCII names.

use rustfig_engine::cmdline::parse;
use rustfig_engine::recovery::Recovery;
use rustfig_engine::Engine;
use serde_json::json;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rustfig-p2-{name}-ünï-日本-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn command_line_takes_any_file_name() {
    for name in [
        "Zeichnung ä.fig",
        "図面.fig",
        "a b  c.fig",
        "emoji😀.fig",
        "with'quote.fig",
        "C:\\Users\\Zoë\\figure.fig",
        "name.with.dots.fig",
        "fig", // a file called like a word
    ] {
        let p = parse([name]).unwrap();
        assert_eq!(p.file.as_deref(), Some(name), "{name}");
    }
    // Options and a file together, the file anywhere.
    let p = parse(["-landscape", "Größe.fig", "--control", "7979"]).unwrap();
    assert_eq!(p.file.as_deref(), Some("Größe.fig"));
    assert_eq!(p.control, Some(7979));
    // A second file name is an error (as in xfig), never a panic.
    assert!(parse(["a.fig", "b.fig"]).is_err());
}

#[test]
fn open_save_and_reopen_a_non_ascii_path() {
    let dir = scratch("open").join("Bilder & Pläne");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Übersicht – Entwurf (1).fig").to_string_lossy().into_owned();
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 900, "height": 450}))
        .unwrap();
    e.execute("file.save", &json!({"path": path})).unwrap();
    assert_eq!(e.path(), Some(path.as_str()));
    // Saving again keeps the previous version as the .bak next to it.
    e.execute(
        "shape.rectangle",
        &json!({"x": 1000, "y": 0, "width": 900, "height": 450}),
    )
    .unwrap();
    e.execute("file.save", &json!({})).unwrap();
    let mut f = Engine::new();
    let r = f.execute("file.open", &json!({"path": path})).unwrap();
    assert_eq!(r["objects"], 2);
    assert_eq!(f.path(), Some(path.as_str()));
    assert!(
        f.prefs.rustfig.recent_files.iter().any(|r| r == &path),
        "recent files keep the name"
    );
    // No temporary file is left next to it.
    let names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().all(|n| !n.ends_with(".rustfig-tmp")), "{names:?}");
    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}

#[test]
fn recovery_remembers_a_non_ascii_original() {
    let dir = scratch("recovery");
    let original = "/home/Zoë/Zeichnungen/図面 ä.fig";
    let a = Recovery::new(dir.clone());
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 900, "height": 450}))
        .unwrap();
    a.save(e.doc(), Some(original)).unwrap();
    let id = a.id().to_string();
    // The instance stops without cleaning up (its lock goes with the handle).
    drop(a);
    let orphans = rustfig_engine::recovery::orphans(&dir, None);
    let o = orphans.iter().find(|o| o.id == id).expect("the copy is offered");
    assert_eq!(o.original.as_deref(), Some(original));
    let _ = std::fs::remove_dir_all(&dir);
}
