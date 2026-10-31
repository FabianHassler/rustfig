#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! G6: "Import as objects" turns a PDF picture into the objects of its page.

use rustfig_engine::model::ObjectKind;
use rustfig_engine::Engine;
use serde_json::json;
use std::path::PathBuf;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustfig-g6-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

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

fn engine_with_picture(dir: &std::path::Path, bytes: &[u8], name: &str) -> (Engine, u64) {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    let mut e = Engine::new();
    let r = e
        .execute(
            "shape.picture",
            &json!({"path": path.to_str().unwrap(), "x": 600, "y": 300, "width": 2000, "height": 1000}),
        )
        .unwrap();
    (e, r["id"].as_u64().unwrap())
}

#[test]
fn pdf_picture_becomes_a_compound_filling_its_box_and_undo_restores_it() {
    let dir = temp_dir("box");
    // A filled rectangle over the whole 200 x 100 point page, and a small one.
    let (mut e, id) = engine_with_picture(
        &dir,
        &tiny_pdf("0 0 200 100 re f 1 0 0 rg 20 20 30 30 re f", false),
        "a.pdf",
    );
    e.execute("edit.select_all", &json!({})).unwrap();
    assert!(rustfig_engine::cmd::find("picture.import_objects")
        .map(|c| (c.enabled)(&e))
        .unwrap());
    let before = e.doc().objects.len();
    let r = e.execute("picture.import_objects", &json!({})).unwrap();
    let new_id = rustfig_engine::model::ObjectId(r["id"].as_u64().unwrap());
    assert_eq!(e.doc().objects.len(), before);
    let o = e.doc().get(new_id).unwrap();
    assert!(matches!(o.kind, ObjectKind::Compound(_)));
    assert!(e.doc().get(rustfig_engine::model::ObjectId(id)).is_none());
    let b = e.object_fig_bounds(new_id).unwrap();
    for (got, want) in [(b.min.x, 600.0), (b.min.y, 300.0), (b.max.x, 2600.0), (b.max.y, 1300.0)] {
        assert!((got - want).abs() < 6.0, "{b:?}");
    }
    e.execute("edit.undo", &json!({})).unwrap();
    let ObjectKind::Polyline(pl) = &e.doc().get(rustfig_engine::model::ObjectId(id)).unwrap().kind else {
        panic!("picture is back")
    };
    assert!(pl.picture.is_some());
}

#[test]
fn flipped_picture_transposes_the_page() {
    let dir = temp_dir("flip");
    let (mut e, id) = engine_with_picture(&dir, &tiny_pdf("0 0 200 100 re f", false), "f.pdf");
    e.execute("picture.rotate", &json!({"id": id, "rotation": 0})).unwrap();
    e.execute("object.set_attributes", &json!({"ids": [id], "flipped": true}))
        .unwrap();
    let r = e.execute("picture.import_objects", &json!({"id": id})).unwrap();
    let nid = rustfig_engine::model::ObjectId(r["id"].as_u64().unwrap());
    let b = e.object_fig_bounds(nid).unwrap();
    assert!((b.min.x - 600.0).abs() < 6.0 && (b.max.x - 2600.0).abs() < 6.0, "{b:?}");
}

#[test]
fn not_a_pdf_picture_is_refused_and_disabled() {
    let dir = temp_dir("no");
    // A PostScript file named .eps: not importable.
    let (mut e, id) = engine_with_picture(&dir, b"%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 10 10\n", "x.eps");
    e.execute("edit.select_all", &json!({})).unwrap();
    assert!(!rustfig_engine::cmd::find("picture.import_objects")
        .map(|c| (c.enabled)(&e))
        .unwrap());
    let rev = e.doc().objects.len();
    assert!(e.execute("picture.import_objects", &json!({"id": id})).is_err());
    assert_eq!(e.doc().objects.len(), rev);
    // A damaged PDF is an error and changes nothing.
    let (mut e2, id2) = engine_with_picture(&dir, &tiny_pdf("0 0 9 9 re f", false), "bad.pdf");
    std::fs::write(dir.join("bad.pdf"), b"%PDF-1.4 garbage").unwrap();
    assert!(e2.execute("picture.import_objects", &json!({"id": id2})).is_err());
    assert_eq!(e2.doc().objects.len(), 1);
}
