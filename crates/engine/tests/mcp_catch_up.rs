#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Q10: MCP catches up with the UI: colors as `#rrggbb` and X11 names,
//! arrow types by name, several lines of text, a cropped render.

use rustfig_engine::Engine;
use rustfig_model::{ColorRef, ObjectId, ObjectKind, Polyline};
use serde_json::{json, Value};

fn polyline(e: &Engine, r: &Value) -> Polyline {
    match &e.doc().get(ObjectId(r["id"].as_u64().unwrap())).unwrap().kind {
        ObjectKind::Polyline(p) => p.clone(),
        _ => panic!("not a polyline"),
    }
}

fn rect(e: &mut Engine, extra: Value) -> Result<Value, rustfig_engine::EngineError> {
    let mut p = json!({"x": 0, "y": 0, "width": 600, "height": 600});
    p.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    e.execute("shape.rectangle", &p)
}

#[test]
fn hex_and_names_make_or_reuse_user_colors_in_one_undo_step() {
    let mut e = Engine::new();
    let r = rect(
        &mut e,
        json!({"pen_color": "#336699", "fill_color": "salmon", "area_fill": 20}),
    )
    .unwrap();
    let pl = polyline(&e, &r);
    assert_eq!(e.doc().colors.len(), 2);
    assert_eq!(e.doc().rgb(pl.style.pen_color).to_hex(), "#336699");
    assert_eq!(
        e.doc().rgb(pl.style.fill_color),
        rustfig_model::x_color("salmon").unwrap()
    );
    // The same value again reuses the user color; a standard color is its number.
    let r = rect(&mut e, json!({"pen_color": "#369", "fill_color": "#ff0000"})).unwrap();
    let again = polyline(&e, &r);
    assert_eq!(again.style.pen_color, pl.style.pen_color);
    assert_eq!(again.style.fill_color, ColorRef(4));
    assert_eq!(e.doc().colors.len(), 2);
    // Object and shape.text setters too, all in the command's one undo step.
    let t = e
        .execute(
            "shape.text",
            &json!({"x": 0, "y": 0, "text": "a", "pen_color": "#010203"}),
        )
        .unwrap();
    assert_eq!(e.doc().colors.len(), 3);
    e.execute(
        "object.set_attributes",
        &json!({"ids": [t["id"]], "pen_color": "#040506"}),
    )
    .unwrap();
    assert_eq!(e.doc().colors.len(), 4);
    assert!(e.undo());
    assert_eq!(e.doc().colors.len(), 3);
}

#[test]
fn bad_color_texts_and_a_full_palette_are_clear_errors() {
    let mut e = Engine::new();
    let err = rect(&mut e, json!({"pen_color": "nocolor"})).unwrap_err();
    assert!(err.message.contains("X11 color name"), "{}", err.message);
    assert!(rect(&mut e, json!({"pen_color": "#12"})).is_err());
    assert!(rect(&mut e, json!({"pen_color": "x".repeat(100)})).is_err());
    for i in 0..512 {
        e.execute("color.new", &json!({"rgb": format!("#{:06x}", 0x100000 + i)}))
            .unwrap();
    }
    let n = e.doc().objects.len();
    let err = rect(&mut e, json!({"pen_color": "#abcdef"})).unwrap_err();
    assert!(err.message.contains("512 user colors"), "{}", err.message);
    assert_eq!(e.doc().objects.len(), n);
    assert_eq!(e.doc().colors.len(), 512);
    // A value that exists still works.
    assert!(rect(&mut e, json!({"pen_color": "#100005"})).is_ok());
}

#[test]
fn arrow_types_by_name() {
    let mut e = Engine::new();
    let go = |e: &mut Engine, p: Value| {
        let mut q = json!({"points": [[0, 0], [900, 0]]});
        q.as_object_mut().unwrap().extend(p.as_object().unwrap().clone());
        let r = e.execute("shape.polyline", &q).unwrap();
        polyline(e, &r)
    };
    let a = go(&mut e, json!({"forward_arrow": true, "arrow_kind": "dot"}))
        .forward_arrow
        .unwrap();
    assert_eq!((a.kind, a.style), (5, 1));
    let a = go(&mut e, json!({"forward_arrow": true, "arrow_kind": "circle"}))
        .forward_arrow
        .unwrap();
    assert_eq!((a.kind, a.style), (5, 0));
    let a = go(&mut e, json!({"forward_arrow": true, "arrow_kind": "bar"}))
        .forward_arrow
        .unwrap();
    assert_eq!((a.kind, a.style), (13, 1));
    // An explicit style wins over the one a name implies.
    let a = go(
        &mut e,
        json!({"forward_arrow": true, "arrow_kind": "Diamond", "arrow_style": "filled"}),
    )
    .forward_arrow
    .unwrap();
    assert_eq!((a.kind, a.style), (4, 1));
    let a = go(
        &mut e,
        json!({"backward_arrow_spec": {"kind": "stick", "style": 0, "thickness": 1, "width": 60, "height": 120}}),
    )
    .backward_arrow
    .unwrap();
    assert_eq!(a.kind, 0);
    let t = go(&mut e, json!({"forward_arrow": true}));
    assert!(t.forward_arrow.is_some());
    let err = e
        .execute(
            "shape.polyline",
            &json!({"points": [[0, 0], [9, 0]], "arrow_kind": "blob"}),
        )
        .unwrap_err();
    assert!(
        err.message.contains("dot") && err.message.contains("bar"),
        "{}",
        err.message
    );
    assert!(e
        .execute(
            "shape.polyline",
            &json!({"points": [[0, 0], [9, 0]], "arrow_kind": [1]})
        )
        .is_err());
    // object.set_attributes takes them too.
    let r = e
        .execute(
            "shape.polyline",
            &json!({"points": [[0, 0], [900, 0]], "forward_arrow": true}),
        )
        .unwrap();
    e.execute("object.set_attributes", &json!({"ids": [r["id"]], "arrow_kind": "dot"}))
        .unwrap();
    let a = polyline(&e, &r).forward_arrow.unwrap();
    assert_eq!((a.kind, a.style), (5, 1));
}

#[test]
fn text_with_newlines_is_one_text_per_line_like_return() {
    let mut e = Engine::new();
    let r = e
        .execute(
            "shape.text",
            &json!({"x": 1000, "y": 2000, "text": "one\ntwo\n\nfour", "size": 12}),
        )
        .unwrap();
    let ids = r["ids"].as_array().unwrap();
    assert_eq!(ids.len(), 3);
    let mut found = Vec::new();
    e.doc().for_each_leaf(&mut |o| {
        if let ObjectKind::Text(t) = &o.kind {
            found.push((t.text.clone(), t.origin));
        }
    });
    assert_eq!(found.len(), 3);
    let step = 12.0 * rustfig_geom::FIG_PER_POINT * rustfig_text::LINE_SPACING;
    assert_eq!(found[0].0, "one");
    assert_eq!(found[0].1.x, 1000.0);
    assert!((found[1].1.y - found[0].1.y - step).abs() < 1e-6);
    assert!((found[2].1.y - found[0].1.y - 3.0 * step).abs() < 1e-6);
    // One undo step for the whole text.
    assert!(e.undo());
    assert!(e.doc().objects.is_empty());
    // A text without a newline still answers with its id.
    let r = e.execute("shape.text", &json!({"x": 0, "y": 0, "text": "a"})).unwrap();
    assert!(r["id"].is_u64());
    assert!(e
        .execute("shape.text", &json!({"x": 0, "y": 0, "text": "\n\n"}))
        .is_err());
}

#[test]
fn crop_exports_the_content_bounds_and_leaves_the_figure_alone() {
    let mut e = Engine::new();
    e.execute(
        "shape.rectangle",
        &json!({"x": 3000, "y": 3000, "width": 1200, "height": 600}),
    )
    .unwrap();
    let size = |e: &mut Engine, crop: bool| -> (f64, f64) {
        let r = e
            .execute("file.export_data", &json!({"format": "svg", "crop": crop}))
            .unwrap();
        let svg = r["data"].as_str().unwrap().to_string();
        let i = svg.find("viewBox=\"").unwrap() + 9;
        let v: Vec<f64> = svg[i..]
            .split('"')
            .next()
            .unwrap()
            .split_whitespace()
            .map(|n| n.parse().unwrap())
            .collect();
        (v[2], v[3])
    };
    let page = size(&mut e, false);
    let cropped = size(&mut e, true);
    assert!(
        cropped.0 < page.0 / 2.0 && cropped.1 < page.1 / 2.0,
        "{page:?} {cropped:?}"
    );
    assert!(cropped.0 > cropped.1, "a 2:1 rectangle");
    assert!(e.doc().bounding_box.is_none());
}
