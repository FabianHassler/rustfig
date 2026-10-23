#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The model <-> fig2dev object bridge (plan D1): `to_model(from_model(doc))`
//! keeps everything the Fig 3.2 format carries, and `from_model(doc)` equals
//! what fig2dev's reader returns for the text `write_fig` writes.

use fig2dev::object::Compound as FigCompound;
use fig2dev::Ctx;
use proptest::prelude::*;
use rustfig_format::bridge::{driver_objects, from_model, to_document};
use rustfig_format::{read_fig, write_fig};
use rustfig_geom::Point;
use rustfig_model::*;
use std::path::PathBuf;

fn pt(x: i32, y: i32) -> Point {
    Point::new(f64::from(x), f64::from(y))
}

/// Ids zeroed, objects in xfig's list order: what the bridge preserves.
fn norm(doc: &Document) -> Document {
    fn n(o: &Object) -> Object {
        let mut o = o.clone();
        o.id = ObjectId(0);
        // The file has no arrowheads on closed shapes.
        match &mut o.kind {
            ObjectKind::Polyline(p) if p.kind != PolyKind::Polyline => {
                (p.forward_arrow, p.backward_arrow) = (None, None)
            }
            ObjectKind::Spline(s) if s.closed => (s.forward_arrow, s.backward_arrow) = (None, None),
            _ => {}
        }
        // The file has the cap of lines, splines and arcs, the join of lines.
        let (cap, join) = (CapStyle::Butt, JoinStyle::Miter);
        match &mut o.kind {
            ObjectKind::Ellipse(e) => (e.style.cap, e.style.join) = (cap, join),
            ObjectKind::Spline(s) => s.style.join = join,
            ObjectKind::Arc(a) => {
                a.style.join = join;
                // The file has the centre to three decimals.
                a.center = Point::new(
                    (a.center.x * 1000.0).round() / 1000.0,
                    (a.center.y * 1000.0).round() / 1000.0,
                );
            }
            _ => {}
        }
        if let ObjectKind::Compound(c) = &mut o.kind {
            let mut v: Vec<Object> = c.objects.iter().map(n).collect();
            v.sort_by_key(Object::xfig_type_rank);
            c.objects = v;
        }
        o
    }
    let mut d = doc.clone();
    let mut v: Vec<Object> = doc.objects.iter().map(|o| n(o)).collect();
    v.sort_by_key(Object::xfig_type_rank);
    d.objects = v.into_iter().map(std::sync::Arc::new).collect();
    d.colors.sort_by_key(|c| c.number);
    d.next_id = 0;
    d
}

fn round_trip(doc: &Document) -> Document {
    let fig = from_model(doc);
    to_document(&fig.ctx, &fig.objects)
}

/// The reader's objects for the text of `doc`, and the bridge's.
fn read_and_bridge(doc: &Document) -> (Ctx, FigCompound, Ctx, FigCompound) {
    let mut rctx = Ctx::default();
    let text = write_fig(doc);
    let robjs = fig2dev::read::readfp_fig(&mut rctx, text.as_bytes())
        .unwrap_or_else(|_| panic!("{}", rctx.messages.join("\n")));
    let mut fig = from_model(doc);
    let bobjs = driver_objects(&mut fig);
    (rctx, robjs, fig.ctx, bobjs)
}

/// Fields of the context the drivers look at.
fn ctx_key(c: &Ctx) -> String {
    format!(
        "{:?}{:?}",
        (
            c.ppi,
            c.thick_scale,
            c.metric,
            c.landscape,
            c.center,
            &c.papersize,
            c.mag,
            c.fontmag,
            c.multi_page,
            &c.gif_transparent,
            &c.user_colors,
            &c.user_col_indx,
        ),
        (
            c.std_color_used,
            c.pattern_used,
            c.pats_used,
            c.arrow_used.to_vec(),
            c.arrows_used,
            (c.v2_flag, c.v21_flag, c.v30_flag, c.v32_flag),
        )
    )
}

/// Unfilled objects keep the file's fill colour number in the reader and
/// get a mapped one from the bridge; equal them for the comparison.
fn unfill(c: &mut FigCompound) {
    for a in &mut c.arcs {
        if a.fill_style < 0 {
            a.fill_color = 0;
        }
    }
    for e in &mut c.ellipses {
        if e.fill_style < 0 {
            e.fill_color = 0;
        }
    }
    for l in &mut c.lines {
        if l.fill_style < 0 {
            l.fill_color = 0;
        }
    }
    for s in &mut c.splines {
        if s.fill_style < 0 {
            s.fill_color = 0;
        }
    }
    // fig2dev's reader (`%d %n`) skips the blanks at the start of a text;
    // the bridge keeps them.
    for t in &mut c.texts {
        t.cstring = t.cstring.trim_start_matches(' ').to_string();
        t.raw = t.cstring.as_bytes().to_vec();
    }
    c.compounds.iter_mut().for_each(unfill);
}

/// `assert_eq!` that prints only the first differing lines.
fn eq_dbg<T: std::fmt::Debug + PartialEq>(a: &T, b: &T, what: &str) {
    if a == b {
        return;
    }
    let (x, y) = (format!("{a:#?}"), format!("{b:#?}"));
    let (xl, yl): (Vec<&str>, Vec<&str>) = (x.lines().collect(), y.lines().collect());
    let i = xl
        .iter()
        .zip(&yl)
        .position(|(p, q)| p != q)
        .unwrap_or(xl.len().min(yl.len()));
    let from = i.saturating_sub(6);
    panic!(
        "{what}: differ at line {i}\n--- left\n{}\n--- right\n{}",
        xl[from..(i + 3).min(xl.len())].join("\n"),
        yl[from..(i + 3).min(yl.len())].join("\n")
    );
}

fn same(a: &Document, b: &Document, what: &str) {
    assert_eq!(
        (&a.settings, &a.comments, &a.colors),
        (&b.settings, &b.comments, &b.colors),
        "{what}: header"
    );
    assert_eq!(a.objects.len(), b.objects.len(), "{what}: object count");
    for (x, y) in a.objects.iter().zip(&b.objects) {
        eq_dbg(x, y, what);
    }
}

fn check(doc: &Document, what: &str) {
    assert_eq!(
        write_fig(&round_trip(doc)),
        write_fig(doc),
        "{what}: Fig text after the round trip"
    );
    same(
        &norm(&round_trip(doc)),
        &norm(doc),
        &format!("{what}: model after the round trip"),
    );
    let (rctx, mut robjs, bctx, mut bobjs) = read_and_bridge(doc);
    check_bounds(doc, &rctx, &robjs, what);
    unfill(&mut robjs);
    unfill(&mut bobjs);
    eq_dbg(&bobjs, &robjs, &format!("{what}: objects as the reader returns them"));
    assert_eq!(ctx_key(&bctx), ctx_key(&rctx), "{what}: context");
}

/// The document's bounds (plan D2) are `compound_bound` of fig2dev's own
/// reading of the Fig text, whole and per top-level object.
fn check_bounds(doc: &Document, rctx: &Ctx, robjs: &FigCompound, what: &str) {
    let b = fig2dev::bound::compound_bound(rctx, robjs, true);
    let want = if b.0 > b.2 || b.1 > b.3 {
        rustfig_geom::Rect::EMPTY
    } else {
        rustfig_geom::Rect::new(pt(b.0, b.1), pt(b.2, b.3))
    };
    let got = rustfig_format::bridge::document_bounds(doc);
    assert_eq!(got, want, "{what}: document bounds");
}

fn fixtures() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../engine/tests/fixtures");
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "fig"))
        .collect();
    v.sort();
    v
}

#[test]
fn fixtures_round_trip() {
    let files = fixtures();
    assert!(files.len() >= 10);
    for p in files {
        let doc = read_fig(&std::fs::read(&p).unwrap()).unwrap();
        check(&doc, &p.display().to_string());
    }
}

/// The corpus sample (`plan/corpus-sample.txt`, local), where present.
#[test]
fn corpus_sample_round_trips() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Ok(list) = std::fs::read_to_string(root.join("plan/corpus-sample.txt")) else {
        return;
    };
    let mut n = 0;
    for line in list
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let Ok(bytes) = std::fs::read(root.join(line)) else {
            continue;
        };
        let Ok(doc) = read_fig(&bytes) else { continue };
        // Pictures with unusual names and odd texts are covered by the
        // reader comparison only where the writer keeps them.
        check(&doc, line);
        n += 1;
    }
    eprintln!("corpus sample: {n} figures");
}

// ---- generated documents ----

fn color() -> impl Strategy<Value = ColorRef> {
    prop_oneof![Just(-1), 0..32, Just(32), Just(33), Just(40)].prop_map(ColorRef)
}

fn style() -> impl Strategy<Value = Style> {
    (
        (0..6, 0..12, color(), color(), 0..1000),
        (-1..2, -1..63, 0..4000, 0..3, 0..3),
    )
        .prop_map(|((line, th, pen, fill, depth), (ps, fillstyle, sv, cap, join))| Style {
            line: LineKind::from_fig(line),
            thickness: th,
            pen_color: pen,
            fill_color: fill,
            depth,
            pen_style: ps,
            area_fill: fillstyle,
            style_val: f64::from(sv) / 1000.0,
            cap: CapStyle::from_fig(cap),
            join: JoinStyle::from_fig(join),
        })
}

fn arrow() -> impl Strategy<Value = Option<Arrow>> {
    prop::option::of(
        (0..4, 0..2, 0..400, 100..20000, 100..20000).prop_map(|(k, s, t, w, h)| Arrow {
            kind: k,
            style: s,
            thickness: f64::from(t) / 100.0,
            width: f64::from(w) / 100.0,
            height: f64::from(h) / 100.0,
        }),
    )
}

fn coords() -> impl Strategy<Value = (i32, i32)> {
    (-5000..20000, -5000..20000)
}

/// Strictly increasing x, so that no two neighbours coincide.
fn points(min: usize, max: usize) -> impl Strategy<Value = Vec<Point>> {
    prop::collection::vec((1..500, -3000..3000), min..max).prop_map(|v| {
        let mut x = 0;
        v.into_iter()
            .map(|(dx, y)| {
                x += dx;
                pt(x, y)
            })
            .collect()
    })
}

fn comments() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[a-z0-9]{1,8}( [a-z0-9]{1,5})?", 0..3)
}

fn object(depth: u32) -> BoxedStrategy<Object> {
    let ellipse = (
        style(),
        1..5,
        0..2,
        -31416..31416,
        coords(),
        (1..4000, 1..4000),
        coords(),
        coords(),
    )
        .prop_map(|(st, k, dir, ang, c, r, s, e)| {
            ObjectKind::Ellipse(Ellipse {
                kind: EllipseKind::from_fig(k),
                style: st,
                direction: dir,
                angle: f64::from(ang) / 10000.0,
                center: pt(c.0, c.1),
                radii: pt(r.0, r.1),
                start: pt(s.0, s.1),
                end: pt(e.0, e.1),
            })
        });
    let polyline = (
        style(),
        0..5,
        points(4, 8),
        arrow(),
        arrow(),
        0..50,
        any::<bool>(),
        "[a-z]{1,6}/[a-z]{1,6}\\.png",
    )
        .prop_map(|(st, k, pts, fa, ba, radius, flipped, file)| {
            let kind = [
                PolyKind::Polyline,
                PolyKind::Box,
                PolyKind::Polygon,
                PolyKind::ArcBox,
                PolyKind::Picture,
            ][k as usize];
            let pts = if kind.is_rect() {
                let (a, b) = (pts[0], pts[pts.len() - 1]);
                let (lo, hi) = (
                    Point::new(a.x.min(b.x), a.y.min(b.y)),
                    Point::new(a.x.max(b.x) + 1.0, a.y.max(b.y) + 1.0),
                );
                vec![lo, Point::new(hi.x, lo.y), hi, Point::new(lo.x, hi.y)]
            } else {
                pts
            };
            let open = kind == PolyKind::Polyline;
            let mut p = Polyline::new(kind, st, pts);
            p.radius = if kind == PolyKind::ArcBox { radius } else { 0 };
            if open {
                p.forward_arrow = fa;
                p.backward_arrow = ba;
            }
            if kind == PolyKind::Picture {
                p.picture = Some(Picture {
                    flipped,
                    file,
                    image: None,
                });
            }
            ObjectKind::Polyline(p)
        });
    let spline = (
        style(),
        0..3,
        any::<bool>(),
        points(3, 7),
        arrow(),
        arrow(),
        prop::collection::vec(-1000..1000, 7),
    )
        .prop_map(|(st, k, closed, pts, fa, ba, sh)| {
            let kind = [SplineKind::Approximated, SplineKind::Interpolated, SplineKind::X][k as usize];
            let shapes = sh.iter().take(pts.len()).map(|&v| f64::from(v) / 1000.0).collect();
            let mut s = Spline::new(kind, closed, st, pts);
            s.shapes = shapes;
            if !closed {
                s.forward_arrow = fa;
                s.backward_arrow = ba;
            }
            ObjectKind::Spline(s)
        });
    let text = (
        0..3,
        color(),
        0..1000,
        (
            -1..2,
            0..35,
            6..40,
            -31416..31416,
            prop_oneof![Just(0), Just(4), Just(6), Just(2)],
        ),
        (0..500, 0..5000),
        coords(),
        "([a-zA-Z0-9\\\\é\n][a-zA-Z0-9 \\\\é\n]{0,11})?",
    )
        .prop_map(|(j, col, depth, (pen, font, size, ang, flags), (h, l), o, txt)| {
            ObjectKind::Text(Text {
                justify: Justify::from_fig(j),
                color: col,
                depth,
                pen_style: pen,
                font,
                size: f64::from(size),
                angle: f64::from(ang) / 10000.0,
                flags,
                height: f64::from(h),
                length: f64::from(l),
                origin: pt(o.0, o.1),
                text: txt,
            })
        });
    let arc = (style(), points(3, 4), any::<bool>(), arrow(), arrow()).prop_filter_map(
        "collinear",
        |(st, p, pie, fa, ba)| {
            let mut a = Arc::through(st, p[0], pt(p[1].x as i32, p[1].y as i32 + 2000), p[2], pie)?;
            a.forward_arrow = fa;
            a.backward_arrow = ba;
            Some(ObjectKind::Arc(a))
        },
    );
    let leaf = prop_oneof![ellipse, polyline, spline, text, arc];
    let with_comments = (leaf, comments()).prop_map(|(kind, c)| {
        let mut o = Object::new(kind);
        o.comments = c;
        o
    });
    if depth == 0 {
        with_comments.boxed()
    } else {
        prop_oneof![
            4 => with_comments,
            1 => (prop::collection::vec(object(depth - 1), 1..4), comments()).prop_map(|(objs, c)| {
                let mut o = Object::new(ObjectKind::Compound(Compound { objects: objs }));
                o.comments = c;
                o
            }),
        ]
        .boxed()
    }
}

fn document() -> impl Strategy<Value = Document> {
    (
        prop::collection::vec(object(2), 0..8),
        (any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>()),
        (
            0..PaperSize::ALL.len(),
            1..40000,
            prop_oneof![Just(-2), Just(0), Just(7), Just(32), Just(40)],
        ),
        comments(),
    )
        .prop_map(|(objs, (land, center, metric, multi), (paper, mag, trans), cm)| {
            let mut d = Document::new();
            d.settings = DocSettings {
                orientation: if land {
                    Orientation::Landscape
                } else {
                    Orientation::Portrait
                },
                center,
                units: if metric { Units::Metric } else { Units::Inches },
                paper: PaperSize::ALL[paper],
                magnification: f64::from(mag) / 100.0,
                multiple_pages: multi,
                transparent_color: trans,
            };
            d.colors = vec![
                UserColor {
                    number: 40,
                    rgb: Rgb::new(1, 2, 3),
                },
                UserColor {
                    number: 32,
                    rgb: Rgb::new(255, 128, 0),
                },
                UserColor {
                    number: 33,
                    rgb: Rgb::new(0, 16, 255),
                },
            ];
            d.comments = cm;
            d.objects = objs.into_iter().map(std::sync::Arc::new).collect();
            d.assign_ids();
            d
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn generated_documents_round_trip(doc in document()) {
        check(&doc, "generated");
    }
}

// ---- hostile documents ----

fn hostile_style() -> Style {
    Style {
        thickness: i32::MAX,
        pen_color: ColorRef(i32::MIN),
        fill_color: ColorRef(99999),
        depth: -5,
        area_fill: 9999,
        style_val: f64::NAN,
        ..Style::default()
    }
}

#[test]
fn hostile_documents_do_not_panic() {
    let bad = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e300, -1e300];
    let mut d = Document::new();
    d.settings.magnification = f64::NAN;
    d.colors = vec![
        UserColor {
            number: -5,
            rgb: Rgb::new(1, 1, 1),
        },
        UserColor {
            number: 32,
            rgb: Rgb::new(1, 1, 1),
        },
        UserColor {
            number: 32,
            rgb: Rgb::new(2, 2, 2),
        },
        UserColor {
            number: 100000,
            rgb: Rgb::new(1, 1, 1),
        },
    ];
    let mut objs = Vec::new();
    for &v in &bad {
        let p = Point::new(v, -v);
        objs.push(Object::new(ObjectKind::Ellipse(Ellipse {
            kind: EllipseKind::from_fig(9),
            style: hostile_style(),
            direction: 7,
            angle: v,
            center: p,
            radii: p,
            start: p,
            end: p,
        })));
        let mut pl = Polyline::new(PolyKind::Picture, hostile_style(), vec![p]);
        pl.radius = i32::MAX;
        pl.forward_arrow = Some(Arrow {
            kind: -4,
            style: 9,
            thickness: v,
            width: v,
            height: v,
        });
        objs.push(Object::new(ObjectKind::Polyline(pl)));
        objs.push(Object::new(ObjectKind::Polyline(Polyline::new(
            PolyKind::Polygon,
            Style::default(),
            vec![],
        ))));
        let mut sp = Spline::new(SplineKind::X, true, hostile_style(), vec![p, p, p]);
        sp.shapes = vec![v];
        objs.push(Object::new(ObjectKind::Spline(sp)));
        objs.push(Object::new(ObjectKind::Spline(Spline::new(
            SplineKind::X,
            false,
            Style::default(),
            vec![],
        ))));
        objs.push(Object::new(ObjectKind::Text(Text {
            justify: Justify::from_fig(77),
            color: ColorRef(i32::MAX),
            depth: i32::MIN,
            pen_style: 0,
            font: i32::MAX,
            size: v,
            angle: v,
            flags: -1,
            height: v,
            length: v,
            origin: p,
            text: "a\u{0}b\u{1}c\n".into(),
        })));
        objs.push(Object::new(ObjectKind::Arc(Arc {
            pie: false,
            style: hostile_style(),
            ccw: true,
            forward_arrow: None,
            backward_arrow: None,
            center: p,
            points: [p, p, p],
        })));
    }
    // Deeply nested compounds.
    let mut inner = Object::new(ObjectKind::Text(Text {
        justify: Justify::from_fig(0),
        color: ColorRef(0),
        depth: 0,
        pen_style: 0,
        font: 0,
        size: 12.0,
        angle: 0.0,
        flags: 0,
        height: 0.0,
        length: 0.0,
        origin: Point::new(0.0, 0.0),
        text: String::new(),
    }));
    for _ in 0..(rustfig_format::MAX_NESTING + 40) {
        inner = Object::new(ObjectKind::Compound(Compound { objects: vec![inner] }));
    }
    objs.push(inner);
    d.objects = objs.into_iter().map(std::sync::Arc::new).collect();
    let mut fig = from_model(&d);
    let _ = driver_objects(&mut fig);
    let _ = to_document(&fig.ctx, &fig.objects);
    let _ = from_model(&Document::new());
}

#[test]
fn arcs_keep_centre_and_pie_arrows() {
    let arrow = Arrow {
        kind: 1,
        style: 1,
        thickness: 1.0,
        width: 60.0,
        height: 120.0,
    };
    let mut doc = Document::default();
    // Centre deliberately off the circle through the rounded points.
    for (pie, c) in [(true, pt(1000, 1000)), (false, pt(2000, 2000))] {
        let mut a = Arc::through(Style::default(), pt(0, 100), pt(100, 0), pt(200, 100), pie).unwrap();
        a.center = Point::new(c.x + 0.375, c.y - 0.25);
        a.forward_arrow = Some(arrow);
        a.backward_arrow = Some(arrow);
        doc.objects.push(std::sync::Arc::new(Object {
            id: Default::default(),
            comments: vec![],
            kind: ObjectKind::Arc(a),
        }));
    }
    let check_arcs = |d: &Document, what: &str| {
        let mut n = 0;
        for o in &d.objects {
            let ObjectKind::Arc(a) = &o.kind else { continue };
            n += 1;
            assert!(
                a.forward_arrow.is_some() && a.backward_arrow.is_some(),
                "{what}: arrows"
            );
            assert!(
                (a.center.x.fract().abs() - 0.375).abs() < 1e-9,
                "{what}: centre {:?}",
                a.center
            );
        }
        assert_eq!(n, 2, "{what}");
    };
    check_arcs(&read_fig(write_fig(&doc).as_bytes()).unwrap(), "fig text");
    check_arcs(&round_trip(&doc), "bridge");
}
