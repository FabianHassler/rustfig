#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// The angles are the rounded values of the Fig files fig2dev read.
#![allow(clippy::approx_constant)]
//! Bounds of hand-built figures, compared with fig2dev 3.2.9a: the
//! expected values are the `viewBox` of `fig2dev -L svg` (x, y, width,
//! height → xmin, ymin, xmax, ymax) for the Fig files quoted with each
//! case. The objects are built as fig2dev's reader builds them: line
//! thicknesses times round(THICK_SCALE) = 15, arrow thicknesses times
//! THICK_SCALE, Fig 3.2 splines converted to lines.

use rustfig_fig2dev::bound::{arc_bound, compound_bound, ellipse_bound, line_bound, text_bound, Bounds};
use rustfig_fig2dev::gendev::depth_option;
use rustfig_fig2dev::object::{
    Arc, Arrow, Compound, Control, DPoint, Ellipse, Line, Point, Spline, Text, T_BOX, T_CLOSED_XSPLINE, T_OPEN_XSPLINE,
    T_PIE_WEDGE_ARC, T_POLYLINE,
};
use rustfig_fig2dev::trans_spline::create_line_with_spline;
use rustfig_fig2dev::Ctx;

fn ctx() -> Ctx {
    Ctx {
        ppi: 1200.0,
        thick_scale: 15.0,
        ..Ctx::default()
    }
}

/// An arrow line of a Fig file: `type style thickness wid ht`.
fn arrow(type_: i32, style: i32, thickness: f64, wid: f64, ht: f64) -> Option<Arrow> {
    Some(Arrow {
        type_,
        style,
        thickness: thickness * 15.0,
        wid,
        ht,
    })
}

fn pts(p: &[(i32, i32)]) -> Vec<Point> {
    p.iter().map(|&(x, y)| Point::new(x, y)).collect()
}

fn line(type_: i32, thickness: i32, p: &[(i32, i32)]) -> Line {
    Line {
        type_,
        thickness: thickness * 15,
        depth: 50,
        points: pts(p),
        ..Line::default()
    }
}

fn arc(type_: i32, thickness: i32, direction: i32, center: (f64, f64), p: [(i32, i32); 3]) -> Arc {
    Arc {
        type_,
        thickness: thickness * 15,
        depth: 50,
        direction,
        center: DPoint {
            x: center.0,
            y: center.1,
        },
        point: p.map(|(x, y)| Point::new(x, y)),
        ..Arc::default()
    }
}

fn ellipse(thickness: i32, angle: f64, center: (i32, i32), radii: (i32, i32)) -> Ellipse {
    Ellipse {
        thickness: thickness * 15,
        depth: 50,
        angle,
        center: Point::new(center.0, center.1),
        radiuses: Point::new(radii.0, radii.1),
        ..Ellipse::default()
    }
}

fn text(type_: i32, angle: f64, height: f64, length: f64, base: (i32, i32), s: &str) -> Text {
    Text {
        type_,
        font: 0,
        size: 12.0,
        depth: 50,
        angle,
        flags: 4,
        height,
        length,
        base_x: base.0,
        base_y: base.1,
        cstring: s.into(),
        raw: s.as_bytes().to_vec(),
        ..Text::default()
    }
}

/// A Fig 3.2 spline, converted to a line as fig2dev's reader does.
fn xspline(type_: i32, thickness: i32, p: &[(i32, i32)], s: &[f64], arrows: (Option<Arrow>, Option<Arrow>)) -> Line {
    let sp = Spline {
        type_,
        thickness: thickness * 15,
        depth: 50,
        for_arrow: arrows.0,
        back_arrow: arrows.1,
        points: pts(p),
        controls: s
            .iter()
            .map(|&s| Control {
                s,
                ..Control::default()
            })
            .collect(),
        ..Spline::default()
    };
    create_line_with_spline(&mut ctx(), &sp).unwrap()
}

fn fig_bounds(c: Compound) -> Bounds {
    compound_bound(&ctx(), &c, true)
}

fn with_lines(lines: Vec<Line>) -> Compound {
    Compound {
        lines,
        ..Compound::default()
    }
}

#[test]
fn rectangle() {
    // 2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5
    //   1200 1200 2400 1200 2400 1800 1200 1800 1200 1200
    let b = line(
        T_BOX,
        1,
        &[(1200, 1200), (2400, 1200), (2400, 1800), (1200, 1800), (1200, 1200)],
    );
    assert_eq!(fig_bounds(with_lines(vec![b])), (1188, 1188, 2412, 1812));
}

#[test]
fn polylines_with_arrows() {
    // 2 1 0 2 0 7 50 -1 -1 0.000 0 0 -1 1 1 2
    //   1 1 1.00 60.00 120.00 / 2 0 2.00 90.00 150.00 / 1200 1200 2400 1500
    let mut l = line(T_POLYLINE, 2, &[(1200, 1200), (2400, 1500)]);
    l.for_arrow = arrow(1, 1, 1.0, 60.0, 120.0);
    l.back_arrow = arrow(2, 0, 2.0, 90.0, 150.0);
    assert_eq!(fig_bounds(with_lines(vec![l])), (1178, 1178, 2422, 1522));

    // half-circle arrowhead: 6 1 1.00 90.00 90.00
    let mut l = line(T_POLYLINE, 1, &[(1200, 1200), (2400, 2400), (3000, 1200)]);
    l.for_arrow = arrow(6, 1, 1.0, 90.0, 90.0);
    assert_eq!(fig_bounds(with_lines(vec![l])), (1188, 1168, 3052, 2412));

    // half-wide spearhead at the start of a thick line: 12 1 1.00 60 120
    let mut l = line(T_POLYLINE, 3, &[(1200, 1200), (2400, 1800)]);
    l.back_arrow = arrow(12, 1, 1.0, 60.0, 120.0);
    assert_eq!(fig_bounds(with_lines(vec![l])), (1167, 1167, 2433, 1833));
}

fn arcs_only(arcs: Vec<Arc>) -> Bounds {
    fig_bounds(Compound {
        arcs,
        ..Compound::default()
    })
}

#[test]
fn arcs() {
    // 5 1 0 1 0 7 50 -1 -1 0.000 0 0 0 0 1500 1500 1200 1500 1500 1200 1800 1500
    let a1 = arc(1, 1, 0, (1500.0, 1500.0), [(1200, 1500), (1500, 1200), (1800, 1500)]);
    assert_eq!(arcs_only(vec![a1.clone()]), (1191, 1191, 1809, 1508));
    // 5 1 0 1 0 7 50 -1 -1 0.000 0 1 1 0 4500 1500 4200 1500 4500 1800 4800 1500
    //   1 1 1.00 60.00 120.00
    let mut a2 = arc(1, 1, 1, (4500.0, 1500.0), [(4200, 1500), (4500, 1800), (4800, 1500)]);
    a2.for_arrow = arrow(1, 1, 1.0, 60.0, 120.0);
    assert_eq!(arcs_only(vec![a2.clone()]), (4191, 1492, 4809, 1809));
    assert_eq!(arcs_only(vec![a1, a2]), (1191, 1191, 4809, 1809));
    // three quarters, counter-clockwise from the first quadrant
    let a3 = arc(1, 1, 1, (0.0, 0.0), [(1000, -100), (707, -707), (-100, -1000)]);
    assert_eq!(arcs_only(vec![a3]), (-108, -1013, 1008, -92));
    // clockwise, with a back arrow: 3 1 2.00 120.00 240.00
    let mut a4 = arc(1, 2, 0, (3000.0, 3000.0), [(3000, 2000), (2000, 3000), (3000, 4000)]);
    a4.back_arrow = arrow(3, 1, 2.0, 120.0, 240.0);
    assert_eq!(arcs_only(vec![a4]), (1985, 1953, 4016, 4016));
    // pie wedge with a circle arrowhead at its start: 5 0 1.00 60.00 120.00
    let mut pie = arc(
        T_PIE_WEDGE_ARC,
        3,
        1,
        (1500.0, 1500.0),
        [(1800, 1500), (1500, 1200), (1200, 1500)],
    );
    pie.back_arrow = arrow(5, 0, 1.0, 60.0, 120.0);
    assert_eq!(arcs_only(vec![pie]), (1176, 1176, 1869, 1523));
}

#[test]
fn ellipses() {
    // 1 1 0 1 0 7 50 -1 -1 0.000 1 0.5236 3000 3000 900 450 ...
    assert_eq!(
        ellipse_bound(&ellipse(1, 0.5236, (3000, 3000), (900, 450))),
        (2180, 2397, 3820, 3603)
    );
    // 1 3 0 2 0 7 50 -1 -1 0.000 1 0.0000 6000 3000 301 301 ...
    assert_eq!(
        ellipse_bound(&ellipse(2, 0.0, (6000, 3000), (301, 301))),
        (5684, 2684, 6316, 3316)
    );
    // 1 1 0 1 0 7 50 -1 -1 0.000 1 1.2000 3000 3000 2000 300 ...
    assert_eq!(
        ellipse_bound(&ellipse(1, 1.2, (3000, 3000), (2000, 300))),
        (2215, 1125, 3785, 4875)
    );
}

#[test]
fn texts() {
    let c = ctx();
    // 4 0 0 50 -1 0 12 0.0000 4 135 1020 1200 1200 Hello Fig\001
    let t1 = text(0, 0.0, 135.0, 1020.0, (1200, 1200), "Hello Fig");
    assert_eq!(text_bound(&c, &t1, true), (1185, 1077, 2261, 1255));
    // 4 1 0 50 -1 0 12 0.5236 4 180 900 3000 3000 jumpy\001
    let t2 = text(1, 0.5236, 180.0, 900.0, (3000, 3000), "jumpy");
    assert_eq!(text_bound(&c, &t2, true), (2513, 2629, 3441, 3292));
    // 4 2 0 50 -1 0 12 1.5708 4 135 600 6000 2000 RIGHT\001
    let t3 = text(2, 1.5708, 135.0, 600.0, (6000, 2000), "RIGHT");
    assert_eq!(text_bound(&c, &t3, true), (5850, 1985, 6015, 2630));
    // 4 0 0 50 -1 0 12 0.0000 4 135 600 1200 1200 HELLO\001
    let t4 = text(0, 0.0, 135.0, 600.0, (1200, 1200), "HELLO");
    assert_eq!(text_bound(&c, &t4, true), (1185, 1050, 1830, 1215));
    let all = Compound {
        texts: vec![t1, t2, t3],
        ..Compound::default()
    };
    assert_eq!(fig_bounds(all), (1185, 1077, 6015, 3292));
}

#[test]
fn xsplines_as_lines() {
    // 3 0 0 1 0 7 50 -1 -1 0.000 0 0 0 4
    //   1200 1200 2400 600 3600 1800 4800 1200 / 0 1 1 0
    let s1 = xspline(
        T_OPEN_XSPLINE & !4,
        1,
        &[(1200, 1200), (2400, 600), (3600, 1800), (4800, 1200)],
        &[0.0, 1.0, 1.0, 0.0],
        (None, None),
    );
    assert_eq!(fig_bounds(with_lines(vec![s1.clone()])), (1188, 866, 4812, 1534));
    // 3 5 0 1 0 7 50 -1 -1 0.000 0 0 0 4
    //   1200 3000 2400 2400 3600 3600 2400 4200 / -1 -1 -1 -1
    let s2 = xspline(
        T_CLOSED_XSPLINE,
        1,
        &[(1200, 3000), (2400, 2400), (3600, 3600), (2400, 4200)],
        &[-1.0; 4],
        (None, None),
    );
    assert_eq!(fig_bounds(with_lines(vec![s2.clone()])), (1188, 2359, 3612, 4241));
    // 3 4 0 2 0 7 50 -1 -1 0.000 0 1 1 3, arrows 1 1 1.00 60.00 120.00
    //   6000 1200 7200 2400 8400 1200 / 0 1 0
    let a = arrow(1, 1, 1.0, 60.0, 120.0);
    let s3 = xspline(
        T_OPEN_XSPLINE,
        2,
        &[(6000, 1200), (7200, 2400), (8400, 1200)],
        &[0.0, 1.0, 0.0],
        (a, a),
    );
    assert_eq!(fig_bounds(with_lines(vec![s3.clone()])), (5978, 1178, 8422, 2022));
    assert_eq!(fig_bounds(with_lines(vec![s1, s2, s3])), (1188, 866, 8422, 4241));
}

fn depth_figure() -> Compound {
    let mut r1 = line(
        T_BOX,
        1,
        &[(1200, 1200), (2400, 1200), (2400, 1800), (1200, 1800), (1200, 1200)],
    );
    r1.depth = 50;
    let mut r2 = line(
        T_BOX,
        1,
        &[(6000, 6000), (7200, 6000), (7200, 7800), (6000, 7800), (6000, 6000)],
    );
    r2.depth = 40;
    let mut e = ellipse(1, 0.0, (8000, 8000), (600, 600));
    e.depth = 30;
    Compound {
        lines: vec![r1, r2],
        compounds: vec![Compound {
            nwcorner: Point::new(0, 0),
            secorner: Point::new(9000, 9000),
            ellipses: vec![e],
            ..Compound::default()
        }],
        ..Compound::default()
    }
}

#[test]
fn depths_count_only_with_k() {
    let fig = depth_figure();
    let mut c = ctx();
    assert_eq!(compound_bound(&c, &fig, true), (1188, 1188, 8608, 8607));
    // -D +50 alone doesn't change the bounds
    c.depths = Some(depth_option("+50").unwrap());
    assert_eq!(compound_bound(&c, &fig, true), (1188, 1188, 8608, 8607));
    // -K -D +50
    c.adjust_boundingbox = true;
    assert_eq!(compound_bound(&c, &fig, true), (1188, 1188, 2412, 1812));
    // -K -D -50
    c.depths = Some(depth_option("-50").unwrap());
    assert_eq!(compound_bound(&c, &fig, true), (5988, 5988, 8608, 8607));
}

#[test]
fn single_objects_and_degenerate_input() {
    let c = ctx();
    assert_eq!(line_bound(&c, &Line::default()), None);
    let a = arc(1, 1, 1, (0.0, 0.0), [(0, 0), (0, 0), (0, 0)]);
    let _ = arc_bound(&c, &a);
    let mut l = line(T_POLYLINE, 1, &[(i32::MAX, i32::MIN), (i32::MIN, i32::MAX)]);
    l.for_arrow = arrow(5, 1, 1.0, 60.0, 120.0);
    l.back_arrow = arrow(14, 0, f64::NAN, f64::INFINITY, 0.0);
    let _ = fig_bounds(with_lines(vec![l]));
}

/// The points of the converted splines match fig2dev's (its `-L tikz`
/// output of the files in `xsplines_as_lines`, y negated; tikz closes the
/// polygon with `cycle` instead of the repeated first point).
#[test]
fn xspline_points_as_fig2dev() {
    let p = |v: &[(i32, i32)]| pts(v);
    let s1 = xspline(
        0,
        1,
        &[(1200, 1200), (2400, 600), (3600, 1800), (4800, 1200)],
        &[0.0, 1.0, 1.0, 0.0],
        (None, None),
    );
    assert_eq!(s1.points.len(), 103);
    assert_eq!(
        s1.points[..6],
        p(&[
            (1200, 1200),
            (1202, 1199),
            (1206, 1197),
            (1214, 1194),
            (1227, 1188),
            (1245, 1180)
        ])[..]
    );
    assert_eq!(s1.points[100..], p(&[(4794, 1203), (4798, 1201), (4800, 1200)])[..]);

    let s2 = xspline(
        T_CLOSED_XSPLINE,
        1,
        &[(1200, 3000), (2400, 2400), (3600, 3600), (2400, 4200)],
        &[-1.0; 4],
        (None, None),
    );
    assert_eq!(s2.points.len(), 165);
    assert_eq!(s2.points[..3], p(&[(2400, 2400), (2457, 2416), (2513, 2436)])[..]);
    assert_eq!(s2.points[162..], p(&[(2281, 2378), (2340, 2387), (2400, 2400)])[..]);

    let a = arrow(1, 1, 1.0, 60.0, 120.0);
    let s3 = xspline(
        T_OPEN_XSPLINE,
        2,
        &[(6000, 1200), (7200, 2400), (8400, 1200)],
        &[0.0, 1.0, 0.0],
        (a, a),
    );
    assert_eq!(s3.points.len(), 65);
    assert_eq!(s3.points[..3], p(&[(6000, 1200), (6023, 1221), (6038, 1236)])[..]);
    assert_eq!(s3.points[62..], p(&[(8362, 1236), (8377, 1221), (8400, 1200)])[..]);
}
