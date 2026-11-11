#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Shape parameters found missing by agents drawing over MCP: cap, join and
//! arrows at creation, arrow styles by name, a dimension line's own label,
//! undefined user colors refused, and the LaTeX default font.

use rustfig_engine::Engine;
use rustfig_model::{CapStyle, JoinStyle, ObjectId, ObjectKind, Polyline};
use serde_json::{json, Value};

fn polyline(e: &Engine, r: &Value) -> Polyline {
    match &e.doc().get(ObjectId(r["id"].as_u64().unwrap())).unwrap().kind {
        ObjectKind::Polyline(p) => p.clone(),
        _ => panic!("not a polyline"),
    }
}

/// The texts of the figure (the dimension line is its only object).
fn texts(e: &Engine) -> Vec<String> {
    let mut texts = Vec::new();
    e.doc().for_each_leaf(&mut |m| {
        if let ObjectKind::Text(t) = &m.kind {
            texts.push(t.text.clone());
        }
    });
    texts
}

fn no_warnings(r: &Value) {
    assert!(
        r.get("warnings").is_none_or(|w| w.as_array().unwrap().is_empty()),
        "{r}"
    );
}

#[test]
fn polyline_takes_cap_join_and_arrows() {
    let mut e = Engine::new();
    let r = e
        .execute(
            "shape.polyline",
            &json!({"points": [[0, 0], [500, 0], [500, 500]], "cap": "round", "join": "bevel",
                    "forward_arrow_spec": {"kind": 2, "style": "filled", "thickness": 15, "width": 100, "height": 200}}),
        )
        .unwrap();
    no_warnings(&r);
    let pl = polyline(&e, &r);
    assert_eq!(pl.style.cap, CapStyle::Round);
    assert_eq!(pl.style.join, JoinStyle::Bevel);
    let a = pl.forward_arrow.expect("a spec turns the arrow on");
    assert_eq!((a.kind, a.style, a.width, a.height), (2, 1, 100.0, 200.0));
    assert!(pl.backward_arrow.is_none());

    // The single-value arrow params change both arrows.
    let r = e
        .execute(
            "shape.polyline",
            &json!({"points": [[0, 0], [500, 0]], "forward_arrow": true, "backward_arrow": true,
                    "arrow_style": "hollow", "arrow_width": 80, "arrow_kind": 1}),
        )
        .unwrap();
    no_warnings(&r);
    let pl = polyline(&e, &r);
    for a in [pl.forward_arrow.unwrap(), pl.backward_arrow.unwrap()] {
        assert_eq!((a.kind, a.style, a.width), (1, 0, 80.0));
    }

    // Switched off, a spec does not turn it on.
    let r = e
        .execute(
            "shape.polyline",
            &json!({"points": [[0, 0], [500, 0]], "forward_arrow": false,
                    "forward_arrow_spec": {"kind": 2, "style": 1, "thickness": 15, "width": 100, "height": 200}}),
        )
        .unwrap();
    assert!(polyline(&e, &r).forward_arrow.is_none());
}

#[test]
fn closed_shapes_take_cap_and_join_but_not_arrows() {
    let mut e = Engine::new();
    let r = e
        .execute(
            "shape.rectangle",
            &json!({"x": 0, "y": 0, "width": 100, "height": 100, "join": "round"}),
        )
        .unwrap();
    no_warnings(&r);
    assert_eq!(polyline(&e, &r).style.join, JoinStyle::Round);
    let r = e
        .execute(
            "shape.polygon",
            &json!({"points": [[0, 0], [100, 0], [0, 100]], "arrow_kind": 1}),
        )
        .unwrap();
    assert!(r["warnings"].to_string().contains("arrow_kind"), "{r}");
}

#[test]
fn bad_cap_and_arrow_style_say_what_is_allowed() {
    let mut e = Engine::new();
    let err = e
        .execute("shape.polyline", &json!({"points": [[0, 0], [1, 0]], "cap": "square"}))
        .unwrap_err();
    assert!(err.to_string().contains("projecting"), "{err}");
    let id = e
        .execute("shape.polyline", &json!({"points": [[0, 0], [100, 0]]}))
        .unwrap()["id"]
        .clone();
    let err = e
        .execute("object.set_attributes", &json!({"ids": [id], "arrow_style": "solid"}))
        .unwrap_err();
    assert!(err.to_string().contains("hollow or filled"), "{err}");
    e.execute(
        "object.set_attributes",
        &json!({"ids": [id], "forward_arrow": true, "arrow_style": "filled"}),
    )
    .unwrap();
    let r = json!({"id": id});
    assert_eq!(polyline(&e, &r).forward_arrow.unwrap().style, 1);
}

#[test]
fn undefined_user_colors_are_refused() {
    let mut e = Engine::new();
    let err = e
        .execute("shape.text", &json!({"x": 0, "y": 0, "text": "a", "color": 44}))
        .unwrap_err();
    assert!(err.to_string().contains("color.add"), "{err}");
    assert!(e
        .execute("shape.circle", &json!({"cx": 0, "cy": 0, "r": 100, "fill_color": 40}))
        .is_err());
    assert!(e.doc().objects.is_empty());

    let c = e.execute("color.add", &json!({"rgb": "#d9d9d9"})).unwrap()["color"]
        .as_i64()
        .unwrap();
    let r = e
        .execute(
            "shape.circle",
            &json!({"cx": 0, "cy": 0, "r": 100, "fill_color": c, "area_fill": 20}),
        )
        .unwrap();
    assert!(r["id"].is_u64());
    let err = e
        .execute("object.set_attributes", &json!({"ids": [r["id"]], "pen_color": c + 1}))
        .unwrap_err();
    assert!(err.to_string().contains("no user color"), "{err}");
}

#[test]
fn dimension_line_text_is_its_label() {
    let mut e = Engine::new();
    let r = e
        .execute(
            "shape.dimension_line",
            &json!({"x1": 1000, "y1": 1000, "x2": 2000, "y2": 1000, "text": "60"}),
        )
        .unwrap();
    assert_eq!(texts(&e), ["60"]);
    // It stays after the dimension line is recomputed.
    e.execute("object.recompute_dimension", &json!({"ids": [r["id"]]}))
        .unwrap();
    assert_eq!(texts(&e), ["60"]);
}

#[test]
fn latex_default_font_is_the_documents() {
    // Font 0 ("Default") follows \familydefault, as fig2dev's texfontfamily.
    let doc = rustfig_engine::latex_preview::latex_document("", "x", 0, false, 12.0, [0, 0, 0]);
    assert!(doc.contains("\\normalfont"), "{doc}");
    let doc = rustfig_engine::latex_preview::latex_document("", "x", 1, false, 12.0, [0, 0, 0]);
    assert!(doc.contains("\\rmfamily") && !doc.contains("\\normalfont"), "{doc}");
}
