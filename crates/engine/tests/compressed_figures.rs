#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! G10: compressed Fig files open, merge and save.

use proptest::prelude::*;
use rustfig_engine::Engine;
use rustfig_format::compress::{compress, Compression};
use serde_json::json;

const SUFFIXES: [(&str, Compression); 5] = [
    ("gz", Compression::Gzip),
    ("Z", Compression::Compress),
    ("z", Compression::Compress),
    ("bz2", Compression::Bzip2),
    ("xz", Compression::Xz),
];

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rustfig-g10-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn figure_text() -> String {
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    e.execute(
        "shape.rectangle",
        &json!({"x": 900, "y": 0, "width": 600, "height": 300}),
    )
    .unwrap();
    e.execute("file.save_data", &json!({})).unwrap()["data"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn open_and_merge_each_codec_and_save_keeps_it() {
    let text = figure_text();
    let dir = scratch("open");
    for (ext, kind) in SUFFIXES {
        let path = dir.join(format!("a.fig.{ext}"));
        std::fs::write(&path, compress(text.as_bytes(), kind).unwrap()).unwrap();
        let p = path.to_str().unwrap();
        let mut e = Engine::new();
        let r = e.execute("file.open", &json!({"path": p})).unwrap();
        assert_eq!(r["objects"], 2, "{ext}");
        assert_eq!(e.path(), Some(p));
        e.execute("file.merge", &json!({"path": p})).unwrap();
        assert_eq!(e.doc().objects.len(), 3, "{ext}: merge adds one compound");
        // Save keeps the compression of the name.
        e.execute("file.save", &json!({})).unwrap();
        let back = std::fs::read(&path).unwrap();
        assert_eq!(Compression::detect(&back), Some(kind), "{ext}");
        let mut e2 = Engine::new();
        e2.execute("file.open", &json!({"path": p})).unwrap();
        assert_eq!(e2.doc().objects.len(), 3, "{ext}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_as_gz_writes_gzip_with_plain_fig_inside() {
    let dir = scratch("save");
    let path = dir.join("x.fig.gz");
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    e.execute("file.save", &json!({"path": path.to_str().unwrap()}))
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[..2], &[0x1f, 0x8b]);
    let inner = rustfig_format::compress::decompress(&bytes, Compression::Gzip, 1 << 24).unwrap();
    assert!(inner.starts_with(b"#FIG 3.2"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_name_opens_its_compressed_twin() {
    let text = figure_text();
    let dir = scratch("twin");
    std::fs::write(
        dir.join("t.fig.bz2"),
        compress(text.as_bytes(), Compression::Bzip2).unwrap(),
    )
    .unwrap();
    let mut e = Engine::new();
    e.execute("file.open", &json!({"path": dir.join("t.fig").to_str().unwrap()}))
        .unwrap();
    assert_eq!(e.doc().objects.len(), 2);
    assert!(e.path().unwrap().ends_with("t.fig.bz2"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bomb_and_damage_are_errors_and_leave_the_document() {
    let dir = scratch("bomb");
    let bomb = compress(&vec![b' '; 300 << 20], Compression::Gzip).unwrap();
    let path = dir.join("b.fig.gz");
    std::fs::write(&path, bomb).unwrap();
    let mut e = Engine::new();
    e.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 600, "height": 300}))
        .unwrap();
    assert!(e
        .execute("file.open", &json!({"path": path.to_str().unwrap()}))
        .is_err());
    assert_eq!(e.doc().objects.len(), 1);
    let bad = dir.join("c.fig.xz");
    std::fs::write(&bad, [0xfd, b'7', b'z', b'X', b'Z', 0, 1, 2, 3]).unwrap();
    assert!(e.execute("file.open", &json!({"path": bad.to_str().unwrap()})).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn open_data_reads_compressed_bytes_and_save_data_writes_them() {
    let text = figure_text();
    let mut e = Engine::new();
    let packed = compress(text.as_bytes(), Compression::Xz).unwrap();
    let r = e
        .execute(
            "file.open_data",
            &json!({"base64": rustfig_engine::base64(&packed), "name": "w.fig.xz"}),
        )
        .unwrap();
    assert_eq!(r["objects"], 2);
    let r = e.execute("file.save_data", &json!({"name": "w.fig.xz"})).unwrap();
    let back = rustfig_engine::base64_decode(r["base64"].as_str().unwrap());
    assert_eq!(Compression::detect(&back), Some(Compression::Xz));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn random_bytes_behind_each_magic(which in 0usize..4, tail in proptest::collection::vec(any::<u8>(), 0..400)) {
        let head: &[u8] = [&[0x1f, 0x8b, 8][..], &[0x1f, 0x9d, 0x90], b"BZh9", &[0xfd, b'7', b'z', b'X', b'Z', 0]][which];
        let mut data = head.to_vec();
        data.extend(tail);
        let mut e = Engine::new();
        let _ = e.execute("file.open_data", &json!({"base64": rustfig_engine::base64(&data), "name": "f.fig.gz"}));
        let _ = e.execute("file.merge_data", &json!({"base64": rustfig_engine::base64(&data)}));
    }
}
