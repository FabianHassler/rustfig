#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Q7: arrowheads partway along a line. The line is pieces glued into a
//! compound; it must look like one line with a head in it: the same
//! picture as the plain line away from the head, nothing at the seam, the
//! head clean over the start of the next piece.

use image::GrayImage;
use rustfig_engine::Engine;
use rustfig_geom::Point;
use rustfig_model::{Object, ObjectId, ObjectKind};
use serde_json::{json, Value};

const DPI: f64 = 150.0;
const PX: f64 = DPI / 1200.0;
const BOX: [i32; 4] = [0, 0, 6000, 3000];

fn unbase64(s: &str) -> Vec<u8> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        _ => 63,
    };
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0;
    for c in s.bytes().filter(|c| *c != b'=') {
        acc = (acc << 6) | u32::from(val(c));
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

fn render(e: &mut Engine) -> GrayImage {
    e.execute(
        "doc.set_bounding_box",
        &json!({"x1": BOX[0], "y1": BOX[1], "x2": BOX[2], "y2": BOX[3]}),
    )
    .unwrap();
    let r = e
        .execute(
            "file.export_data",
            &json!({"format": "png", "dpi": DPI, "background": "#ffffff", "options": {"smoothing": 4}}),
        )
        .unwrap();
    let bytes = unbase64(r["base64"].as_str().unwrap());
    image::load_from_memory(&bytes).unwrap().to_luma8()
}

fn px(img: &GrayImage, p: Point) -> u8 {
    img.get_pixel((p.x * PX).round() as u32, (p.y * PX).round() as u32).0[0]
}

/// How many pixels differ much between two pictures outside the discs of
/// radius `r` (Fig units) around `centers`.
fn differing(a: &GrayImage, b: &GrayImage, centers: &[Point], r: f64) -> usize {
    assert_eq!(a.dimensions(), b.dimensions());
    let mut n = 0;
    for (x, y, p) in a.enumerate_pixels() {
        let at = Point::new(f64::from(x) / PX, f64::from(y) / PX);
        if centers.iter().any(|c| c.dist(at) < r) {
            continue;
        }
        if (i32::from(p.0[0]) - i32::from(b.get_pixel(x, y).0[0])).abs() > 100 {
            n += 1;
        }
    }
    n
}

struct Look {
    cap: &'static str,
    line: &'static str,
    kind: i32,
    style: i32,
}

const HEAD_W: f64 = 180.0;
const HEAD_L: f64 = 300.0;

fn draw(cmd: &str, geometry: Value, look: &Look, mid: Option<Value>) -> Engine {
    let mut e = Engine::new();
    let mut p = geometry;
    let o = p.as_object_mut().unwrap();
    o.extend(
        json!({"thickness": 4, "cap": look.cap, "line": look.line, "style_val": 6,
               "arrow_kind": look.kind, "arrow_style": look.style,
               "arrow_width": HEAD_W, "arrow_height": HEAD_L})
        .as_object()
        .unwrap()
        .clone(),
    );
    if let Some(m) = mid {
        o.insert("mid_arrows".into(), m);
    }
    e.execute(cmd, &p).unwrap();
    e
}

fn compound_pieces(e: &Engine) -> Vec<Object> {
    match &e
        .doc()
        .get(ObjectId(1))
        .or_else(|| e.doc().objects.first().map(|o| &**o))
        .unwrap()
        .kind
    {
        ObjectKind::Compound(c) => c.objects.clone(),
        _ => panic!("not a compound"),
    }
}

/// The picture matches the plain line away from the head, for every head,
/// cap and line style, on a straight line.
#[test]
fn straight_line_looks_like_one_line() {
    let line = json!({"points": [[500, 1500], [5500, 1500]]});
    let head = Point::new(3000.0, 1500.0);
    for (kind, style) in [(0, 0), (1, 1), (1, 0), (2, 1), (4, 0), (7, 1), (8, 0), (13, 1)] {
        for cap in ["butt", "round"] {
            let look = Look {
                cap,
                line: "solid",
                kind,
                style,
            };
            let mut plain = draw("shape.polyline", line.clone(), &look, None);
            let mut mid = draw("shape.polyline", line.clone(), &look, Some(json!([0.5])));
            let a = render(&mut plain);
            let b = render(&mut mid);
            let n = differing(&a, &b, &[head], HEAD_L * 0.75 + 60.0);
            assert_eq!(
                n, 0,
                "kind {kind} style {style} {cap}: {n} pixels differ away from the head"
            );
            // The head is there: dark pixels near it that the plain line lacks (or, for a
            // hollow head, white ones on the line).
            let near = differing(&a, &b, &[], 0.0);
            assert!(near > 0, "kind {kind} style {style} {cap}: no head drawn");
        }
    }
}

/// A hollow head's fill covers the line under it; a filled and a stick head
/// leave the line dark across the head.
#[test]
fn hollow_head_is_clean_filled_and_stick_heads_are_not_cut() {
    let line = json!({"points": [[500, 1500], [5500, 1500]]});
    let centre = Point::new(3000.0, 1500.0);
    let ahead = Point::new(3000.0 + HEAD_L / 2.0 + 120.0, 1500.0);
    let behind = Point::new(3000.0 - HEAD_L / 2.0 - 120.0, 1500.0);
    for cap in ["butt", "round"] {
        let hollow = Look {
            cap,
            line: "solid",
            kind: 1,
            style: 0,
        };
        let mut e = draw("shape.polyline", line.clone(), &hollow, Some(json!([0.5])));
        let img = render(&mut e);
        assert!(
            px(&img, centre) > 200,
            "{cap}: the line shows through a hollow head ({})",
            px(&img, centre)
        );
        assert!(
            px(&img, ahead) < 100 && px(&img, behind) < 100,
            "{cap}: gap beside the hollow head"
        );
        for (kind, style) in [(1, 1), (0, 0), (13, 1)] {
            let look = Look {
                cap,
                line: "solid",
                kind,
                style,
            };
            let mut e = draw("shape.polyline", line.clone(), &look, Some(json!([0.5])));
            let img = render(&mut e);
            for at in [centre, ahead, behind] {
                assert!(px(&img, at) < 100, "kind {kind} {cap}: gap in the line at {at:?}");
            }
        }
    }
}

/// Dashed lines: the first piece is the plain line's start, the pattern goes
/// on after the head, and the head sits on the line.
#[test]
fn dashed_line_keeps_its_pattern_before_the_head() {
    let line = json!({"points": [[500, 1500], [5500, 1500]]});
    let look = Look {
        cap: "butt",
        line: "dashed",
        kind: 1,
        style: 1,
    };
    let mut plain = draw("shape.polyline", line.clone(), &look, None);
    let mut mid = draw("shape.polyline", line.clone(), &look, Some(json!([0.5])));
    let (a, b) = (render(&mut plain), render(&mut mid));
    let row = (1500.0 * PX) as u32;
    let x_head = (3000.0 - HEAD_L / 2.0) * PX;
    let dark = |img: &GrayImage, from: f64, to: f64| {
        (from as u32..to as u32)
            .filter(|x| img.get_pixel(*x, row).0[0] < 128)
            .count()
    };
    assert_eq!(dark(&a, 500.0 * PX, x_head - 2.0), dark(&b, 500.0 * PX, x_head - 2.0));
    // After the head the dashes go on: about as much ink as the plain line, and no long gap.
    let after = (3000.0 + HEAD_L / 2.0 + 20.0) * PX;
    let (da, db) = (dark(&a, after, 5500.0 * PX), dark(&b, after, 5500.0 * PX));
    assert!(
        db * 10 >= da * 8 && db * 10 <= da * 12,
        "ink after the head: {da} plain, {db} here"
    );
    let mut run = 0;
    for x in after as u32..(5500.0 * PX) as u32 {
        if b.get_pixel(x, row).0[0] > 128 {
            run += 1;
            assert!(
                run < (8.0 * 6.0 * 1.6 / 12.0 * 150.0 / 80.0 * 4.0) as i32,
                "gap too long after the head"
            );
        } else {
            run = 0;
        }
    }
}

/// The pieces: heads at the requested positions, ends keep their own
/// arrows, drawn last piece first, one undo step.
#[test]
fn pieces_are_glued_and_ordered() {
    let look = Look {
        cap: "butt",
        line: "solid",
        kind: 1,
        style: 1,
    };
    let mut e = draw(
        "shape.polyline",
        json!({"points": [[500, 1500], [5500, 1500]], "backward_arrow": true}),
        &look,
        Some(json!([0.25, 0.5, 0.75])),
    );
    let pieces = compound_pieces(&e);
    assert_eq!(pieces.len(), 4);
    let polys: Vec<_> = pieces
        .iter()
        .map(|o| match &o.kind {
            ObjectKind::Polyline(p) => p.clone(),
            _ => panic!("not a line"),
        })
        .collect();
    // Last piece first: it has no head at its end; the first piece (last in the list) has the backward arrow.
    assert!(polys[0].forward_arrow.is_none());
    assert!(polys[3].backward_arrow.is_some());
    assert!(polys[..3]
        .iter()
        .all(|p| p.backward_arrow.is_none() || p.points[0].x < 600.0));
    for (i, p) in polys[1..].iter().rev().enumerate() {
        let x = 500.0 + 5000.0 * [0.25, 0.5, 0.75][i];
        let tip = p.points.last().unwrap();
        assert!(
            (tip.x - (x + HEAD_L / 2.0)).abs() <= 1.0,
            "tip at {tip:?}, position {x}"
        );
        assert!(p.forward_arrow.is_some());
    }
    // The next piece starts before the tip, under the head.
    let first = &polys[3];
    let second = &polys[2];
    assert!(second.points[0].x < first.points.last().unwrap().x);
    assert!(e.undo());
    assert!(e.doc().objects.is_empty());
}

/// Along a spline and an arc: the pieces stay on the curve and the picture
/// matches the plain curve away from the head.
#[test]
fn curves_look_like_one_line() {
    let look = Look {
        cap: "round",
        line: "solid",
        kind: 1,
        style: 1,
    };
    let cases = [
        (
            "shape.spline",
            json!({"points": [[500, 2500], [2000, 800], [4000, 2200], [5500, 700]], "kind": "interpolated"}),
        ),
        ("shape.arc", json!({"points": [[500, 2500], [3000, 500], [5500, 2500]]})),
        (
            "shape.polyline",
            json!({"points": [[500, 2500], [2000, 800], [4000, 2200], [5500, 700]]}),
        ),
    ];
    for (cmd, geometry) in cases {
        let mut plain = draw(cmd, geometry.clone(), &look, None);
        let mut mid = draw(cmd, geometry.clone(), &look, Some(json!([0.4])));
        let pieces = compound_pieces(&mid);
        assert_eq!(pieces.len(), 2, "{cmd}");
        // Head tip: the end of the first piece (the last in the list).
        let tip = match &pieces[1].kind {
            ObjectKind::Polyline(p) => *p.points.last().unwrap(),
            ObjectKind::Arc(a) => a.points[2],
            _ => panic!(),
        };
        let (a, b) = (render(&mut plain), render(&mut mid));
        let n = differing(&a, &b, &[tip], HEAD_L * 1.2 + 80.0);
        let ink = a.pixels().filter(|p| p.0[0] < 128).count();
        assert!(n * 100 <= ink, "{cmd}: {n} pixels of {ink} differ away from the head");
    }
}

#[test]
fn bad_positions_are_clear_errors() {
    let look = Look {
        cap: "butt",
        line: "solid",
        kind: 1,
        style: 1,
    };
    let go = |mid: Value, geometry: Value, cmd: &str| {
        let mut e = Engine::new();
        let mut p = geometry;
        p.as_object_mut().unwrap().extend(
            json!({"arrow_width": HEAD_W, "arrow_height": HEAD_L, "arrow_kind": look.kind})
                .as_object()
                .unwrap()
                .clone(),
        );
        p.as_object_mut().unwrap().insert("mid_arrows".into(), mid);
        let r = e.execute(cmd, &p);
        assert!(e.doc().objects.is_empty() || r.is_ok());
        r
    };
    let line = json!({"points": [[0, 0], [3000, 0]]});
    for bad in [
        json!([]),
        json!([0.0]),
        json!([1.0]),
        json!([0.6, 0.4]),
        json!(["a"]),
        json!(0.5),
        json!([f64::NAN]),
        json!([0.5, 0.5]),
    ] {
        assert!(go(bad.clone(), line.clone(), "shape.polyline").is_err(), "{bad}");
    }
    // Too near the end for the head; heads too close to each other.
    assert!(go(json!([0.01]), line.clone(), "shape.polyline")
        .unwrap_err()
        .message
        .contains("past an end"));
    assert!(go(json!([0.5, 0.55]), line.clone(), "shape.polyline")
        .unwrap_err()
        .message
        .contains("overlap"));
    assert!(go(json!([0.5]), json!({"points": [[0, 0], [100, 0]]}), "shape.polyline").is_err());
    // A closed spline and a pie have no open line to cut.
    assert!(go(
        json!([0.5]),
        json!({"points": [[0, 0], [3000, 0], [3000, 3000]], "closed": true}),
        "shape.spline"
    )
    .is_err());
    assert!(go(
        json!([0.5]),
        json!({"points": [[0, 0], [1000, 1000], [2000, 0]], "pie": true}),
        "shape.arc"
    )
    .is_err());
}

/// Arrow mode "middle" in the indicator: a line drawn with the tool gets
/// the head.
#[test]
fn arrow_mode_middle_draws_a_head_in_the_line() {
    use rustfig_engine::tools::{Button, ToolEvent, ToolKind};
    let mut e = Engine::new();
    e.execute("indicator.set", &json!({"mid_arrow": true, "thickness": 3, "arrow": {"kind": 1, "style": 1, "thickness": 1.0, "width": 90.0, "height": 180.0}, "arrow_absolute": true})).unwrap();
    e.set_tool(ToolKind::Polyline);
    for (x, y) in [(600.0, 600.0), (3600.0, 600.0)] {
        e.tool_event(ToolEvent::Press {
            pos: Point::new(x, y),
            button: Button::Left,
            shift: false,
        })
        .unwrap();
        e.tool_event(ToolEvent::Release {
            pos: Point::new(x, y),
            button: Button::Left,
        })
        .ok();
    }
    e.tool_event(ToolEvent::Press {
        pos: Point::new(3600.0, 600.0),
        button: Button::Middle,
        shift: false,
    })
    .unwrap();
    assert_eq!(e.doc().objects.len(), 1);
    assert!(matches!(&e.doc().objects[0].kind, ObjectKind::Compound(c) if c.objects.len() == 2));
}
