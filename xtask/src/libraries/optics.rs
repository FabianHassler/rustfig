//! The OpticsLab library (plan D13): optical-table parts the way physics
//! papers draw them, and three example setups. Every part is drawn in a
//! frame whose optical axis is the line y = 900, with the part's centre at
//! (900, 900); parts that sit in the beam have their beam ports on the
//! 1/4-inch grid at (300, 900) and (1500, 900), so they line up when placed
//! on the same height.

use super::*;
use rustfig_geom::Affine;

const AX: f64 = 900.0;
const R2: f64 = std::f64::consts::FRAC_1_SQRT_2;
/// xfig's standard red (the beams), light blue (glass) and gold.
const RED: ColorRef = ColorRef(4);
const GLASS: ColorRef = ColorRef(11);
const GOLD: ColorRef = ColorRef(31);

type Pt = (f64, f64);
type Fill = Option<(ColorRef, i32)>;
type Entry = (&'static str, &'static str, &'static str, Vec<Object>);

/// Light grey (a tint of black), dark, white and light blue fills.
const GREY: Fill = Some((ColorRef::BLACK, 36));
const DARK: Fill = Some((ColorRef::BLACK, 20));
const WHITE: Fill = Some((ColorRef::WHITE, 20));
const GLASSF: Fill = Some((GLASS, 20));

fn pt(q: Pt) -> Point {
    p(q.0, q.1)
}

fn pts(v: &[Pt]) -> Vec<Point> {
    v.iter().map(|&q| pt(q)).collect()
}

fn line(thickness: i32, v: &[Pt]) -> Object {
    polyline(solid(thickness, 50), pts(v), None)
}

fn beam_style(kind: LineKind) -> Style {
    let mut s = style(kind, 2, 45);
    s.pen_color = RED;
    if kind == LineKind::Dashed {
        s.style_val = 5.0;
    }
    s
}

/// A beam segment (red, in front of the glass).
fn beam(a: Pt, b: Pt) -> Object {
    polyline(beam_style(LineKind::Solid), pts(&[a, b]), None)
}

fn with_fill(mut s: Style, fill: Fill) -> Style {
    if let Some((c, t)) = fill {
        s.fill_color = c;
        s.area_fill = t;
    }
    s
}

/// A closed polygon (the first point is repeated, as xfig writes it).
fn poly(thickness: i32, fill: Fill, v: &[Pt]) -> Object {
    let mut pp = pts(v);
    if let Some(&f) = pp.first() {
        pp.push(f);
    }
    let s = with_fill(solid(thickness, 50), fill);
    Object::new(ObjectKind::Polyline(Polyline::new(PolyKind::Polygon, s, pp)))
}

fn rect(thickness: i32, fill: Fill, a: Pt, b: Pt) -> Object {
    let s = with_fill(solid(thickness, 50), fill);
    Object::new(ObjectKind::Polyline(Polyline::rect(s, pt(a), pt(b))))
}

fn disc(c: Pt, r: f64, thickness: i32, fill: Fill) -> Object {
    circle(pt(c), r.round(), with_fill(solid(thickness, 50), fill))
}

fn arrow_head2() -> Arrow {
    Arrow {
        kind: 1,
        style: 1,
        thickness: 1.0,
        width: 54.0,
        height: 90.0,
    }
}

/// A thin line with an arrow head at each end.
fn double_arrow(a: Pt, b: Pt) -> Object {
    let mut l = Polyline::new(PolyKind::Polyline, solid(1, 48), pts(&[a, b]));
    l.forward_arrow = Some(arrow_head2());
    l.backward_arrow = Some(arrow_head2());
    Object::new(ObjectKind::Polyline(l))
}

/// A short slanted tick on the back of a mirror: from `q` backwards along
/// the unit normal `n`, slanted against the tangent `t`.
fn tick(q: Pt, n: Pt, t: Pt) -> Object {
    line(1, &[q, (q.0 + 60.0 * n.0 - 60.0 * t.0, q.1 + 60.0 * n.1 - 60.0 * t.1)])
}

/// Hatching on the back of a straight mirror from `a` to `b`; `n` points to the back.
fn hatch(a: Pt, b: Pt, n: Pt) -> Vec<Object> {
    let len = (b.0 - a.0).hypot(b.1 - a.1).max(1.0);
    let t = ((b.0 - a.0) / len, (b.1 - a.1) / len);
    let count = (len / 60.0).floor() as usize;
    (1..=count)
        .map(|i| {
            let s = 60.0 * i as f64;
            tick((a.0 + t.0 * s, a.1 + t.1 * s), n, t)
        })
        .collect()
}

/// A thin plate of length `len` and thickness `th`, centred on `c`, tilted
/// 45 degrees up to the right.
fn plate45(c: Pt, len: f64, th: f64, fill: Fill) -> Object {
    let (u, n) = ((R2, -R2), (R2, R2));
    let (l, w) = (len / 2.0, th / 2.0);
    let corner = |a: f64, b: f64| (c.0 + u.0 * a + n.0 * b, c.1 + u.1 * a + n.1 * b);
    poly(2, fill, &[corner(-l, -w), corner(l, -w), corner(l, w), corner(-l, w)])
}

/// Lens outline: half height `h`, half thickness `half_thickness(y)` along the axis.
fn lens_outline(x: f64, h: f64, half_thickness: impl Fn(f64) -> f64) -> Vec<Pt> {
    let n = 16;
    let ys: Vec<f64> = (0..=2 * n).map(|i| -h + h * i as f64 / n as f64).collect();
    let mut v: Vec<Pt> = ys.iter().map(|&y| (x + half_thickness(y), AX + y)).collect();
    v.extend(ys.iter().rev().map(|&y| (x - half_thickness(y), AX + y)));
    v
}

/// Radius of the circle through the sag `s` over the half chord `h`.
fn sag_radius(h: f64, s: f64) -> f64 {
    (h * h + s * s) / (2.0 * s)
}

/// Biconvex lens: half thickness `t` in the middle, `rim` at the edge.
fn convex_lens(x: f64, h: f64, t: f64, rim: f64) -> Object {
    let r = sag_radius(h, t - rim);
    let outline = lens_outline(x, h, |y| (r * r - y * y).max(0.0).sqrt() - (r - t));
    poly(2, GLASSF, &outline)
}

fn concave_lens(x: f64, h: f64) -> Object {
    let r = sag_radius(h, 60.0);
    let outline = lens_outline(x, h, |y| 30.0 + r - (r * r - y * y).max(0.0).sqrt());
    poly(2, GLASSF, &outline)
}

/// A curved mirror surface with its hatching: vertex at (`vx`, AX), centre of
/// curvature `r` to the right (`dir` = 1) or left (-1).
fn curved_mirror(vx: f64, dir: f64, half: f64, r: f64, hatched: bool) -> Vec<Object> {
    let th_max = (half / r).asin();
    let n = 12;
    let theta = |i: usize| -th_max + th_max * i as f64 / n as f64;
    let at = |th: f64| (vx + dir * r - dir * r * th.cos(), AX + r * th.sin());
    let curve: Vec<Pt> = (0..=2 * n).map(|i| at(theta(i))).collect();
    let mut v = vec![line(3, &curve)];
    if hatched {
        for i in 1..2 * n {
            let th = theta(i);
            v.push(tick(at(th), (-dir * th.cos(), th.sin()), (dir * th.sin(), th.cos())));
        }
    }
    v
}

fn flat_mirror(c: Pt, half: f64, hatched: bool) -> Vec<Object> {
    let (a, b) = ((c.0, c.1 - half), (c.0, c.1 + half));
    let mut v = vec![line(3, &[a, b])];
    if hatched {
        v.extend(hatch(a, b, (1.0, 0.0)));
    }
    v
}

fn mirror45(c: Pt, hatched: bool) -> Vec<Object> {
    let (a, b) = ((c.0 - 255.0, c.1 + 255.0), (c.0 + 255.0, c.1 - 255.0));
    let mut v = vec![line(3, &[a, b])];
    if hatched {
        v.extend(hatch(a, b, (R2, R2)));
    }
    v
}

fn cube(fill: Fill) -> Object {
    rect(2, fill, (675.0, 675.0), (1125.0, 1125.0))
}

fn laser() -> Vec<Object> {
    vec![
        rect(2, GREY, (300.0, 675.0), (1050.0, 1125.0)),
        rect(2, DARK, (1050.0, 825.0), (1110.0, 975.0)),
        line(1, &[(360.0, 750.0), (360.0, 1050.0)]),
        beam((1110.0, AX), (1500.0, AX)),
    ]
}

fn coil() -> Vec<Object> {
    let mut v = vec![
        line(2, &[(300.0, AX), (630.0, AX)]),
        line(2, &[(1170.0, AX), (1500.0, AX)]),
    ];
    for r in [270.0, 225.0, 180.0] {
        v.push(disc((AX, AX), r, 2, None));
    }
    v
}

fn coupler() -> Vec<Object> {
    vec![
        convex_lens(540.0, 180.0, 36.0, 6.0),
        poly(
            2,
            GREY,
            &[(660.0, 735.0), (1020.0, 825.0), (1020.0, 975.0), (660.0, 1065.0)],
        ),
        line(2, &[(1020.0, AX), (1500.0, AX)]),
        rect(2, DARK, (1020.0, 810.0), (1065.0, 990.0)),
        beam((300.0, AX), (480.0, AX)),
    ]
}

fn photodiode() -> Vec<Object> {
    vec![
        disc((AX, AX), 225.0, 2, WHITE),
        poly(2, GREY, &[(840.0, 765.0), (840.0, 1035.0), (1020.0, AX)]),
        line(3, &[(1020.0, 765.0), (1020.0, 1035.0)]),
        line(2, &[(675.0, AX), (840.0, AX)]),
        line(2, &[(1020.0, AX), (1125.0, AX)]),
    ]
}

fn detector() -> Vec<Object> {
    vec![
        rect(2, GREY, (900.0, 675.0), (1200.0, 1125.0)),
        line(4, &[(900.0, 675.0), (900.0, 1125.0)]),
    ]
}

fn camera() -> Vec<Object> {
    vec![
        rect(2, GREY, (900.0, 675.0), (1260.0, 1125.0)),
        rect(2, DARK, (720.0, 780.0), (900.0, 1020.0)),
        line(4, &[(930.0, 720.0), (930.0, 1080.0)]),
        line(
            1,
            &[
                (1020.0, 735.0),
                (1200.0, 735.0),
                (1200.0, 1065.0),
                (1020.0, 1065.0),
                (1020.0, 735.0),
            ],
        ),
    ]
}

fn aom() -> Vec<Object> {
    let mut v = vec![
        rect(2, GLASSF, (720.0, 675.0), (1080.0, 1125.0)),
        rect(2, DARK, (810.0, 1125.0), (990.0, 1215.0)),
        line(1, &[(900.0, 1215.0), (900.0, 1305.0)]),
    ];
    for y in [765.0, 855.0, 945.0, 1035.0] {
        v.push(polyline(
            style(LineKind::Dashed, 1, 48),
            pts(&[(735.0, y), (1065.0, y)]),
            None,
        ));
    }
    v
}

fn eom() -> Vec<Object> {
    vec![
        rect(2, GLASSF, (720.0, 765.0), (1080.0, 1035.0)),
        rect(2, DARK, (720.0, 705.0), (1080.0, 765.0)),
        rect(2, DARK, (720.0, 1035.0), (1080.0, 1095.0)),
        line(1, &[(900.0, 705.0), (900.0, 540.0), (1275.0, 540.0), (1275.0, 825.0)]),
        line(
            1,
            &[(900.0, 1095.0), (900.0, 1260.0), (1275.0, 1260.0), (1275.0, 975.0)],
        ),
        disc((1275.0, AX), 75.0, 1, WHITE),
        spline(
            style(LineKind::Solid, 1, 48),
            pts(&[
                (1230.0, AX),
                (1252.0, 870.0),
                (1275.0, AX),
                (1298.0, 930.0),
                (1320.0, AX),
            ]),
        ),
    ]
}

fn isolator() -> Vec<Object> {
    vec![
        rect(2, WHITE, (600.0, 660.0), (1200.0, 1140.0)),
        poly(2, DARK, &[(720.0, 780.0), (720.0, 1020.0), (900.0, AX)]),
        line(3, &[(900.0, 780.0), (900.0, 1020.0)]),
        line(1, &[(990.0, AX), (1110.0, AX)]),
    ]
}

fn prism() -> Vec<Object> {
    vec![poly(2, GLASSF, &[(900.0, 630.0), (660.0, 1045.0), (1140.0, 1045.0)])]
}

fn grating() -> Vec<Object> {
    let mut outline: Vec<Pt> = Vec::new();
    for i in 0..8 {
        let x = 660.0 + 60.0 * i as f64;
        outline.push((x, AX));
        outline.push((x + 60.0, AX - 30.0));
        outline.push((x + 60.0, AX));
    }
    outline.push((1140.0, 1050.0));
    outline.push((660.0, 1050.0));
    vec![poly(2, GREY, &outline)]
}

fn cavity() -> Vec<Object> {
    let mut v = curved_mirror(450.0, 1.0, 300.0, 1200.0, true);
    v.extend(curved_mirror(1350.0, -1.0, 300.0, 1200.0, true));
    v.push(polyline(
        beam_style(LineKind::Dashed),
        pts(&[(450.0, AX), (1350.0, AX)]),
        None,
    ));
    v
}

fn vacuum_chamber() -> Vec<Object> {
    let r = 450.0;
    let half: f64 = 120.0;
    let gap = (half / r).asin();
    // The fill: a circle without an outline.
    let mut v = vec![circle(p(AX, AX), r, with_fill(solid(0, 70), GREY))];
    // Four arcs between the ports (at 0, 90, 180 and 270 degrees).
    for k in 0..4 {
        let start = k as f64 * PI / 2.0 + gap;
        let end = (k + 1) as f64 * PI / 2.0 - gap;
        let arc: Vec<Pt> = (0..=12)
            .map(|i| {
                let a = start + (end - start) * i as f64 / 12.0;
                (AX + r * a.cos(), AX + r * a.sin())
            })
            .collect();
        v.push(line(3, &arc));
    }
    // Ports: tube walls and a flange each; local x runs outwards, y across.
    let reach = (r * r - half * half).sqrt();
    for k in 0..4 {
        let (s, c) = (k as f64 * PI / 2.0).sin_cos();
        let map = |x: f64, y: f64| (AX + c * x - s * y, AX + s * x + c * y);
        v.push(line(2, &[map(reach, -half), map(600.0, -half)]));
        v.push(line(2, &[map(reach, half), map(600.0, half)]));
        v.push(poly(
            2,
            WHITE,
            &[
                map(555.0, -180.0),
                map(600.0, -180.0),
                map(600.0, 180.0),
                map(555.0, 180.0),
            ],
        ));
    }
    v
}

fn cryostat() -> Vec<Object> {
    let mut v = vec![
        rect(3, WHITE, (450.0, 450.0), (1350.0, 1350.0)),
        rect(2, GLASSF, (570.0, 570.0), (1230.0, 1230.0)),
        rect(2, WHITE, (780.0, 780.0), (1020.0, 1020.0)),
        // Cold finger from the top plate down to the stage.
        rect(2, GREY, (870.0, 450.0), (930.0, 780.0)),
    ];
    for x in [450.0, 570.0, 1230.0, 1350.0] {
        v.push(rect(2, GLASSF, (x - 24.0, 780.0), (x + 24.0, 1020.0)));
    }
    v.push(line(2, &[(300.0, AX), (426.0, AX)]));
    v.push(line(2, &[(1374.0, AX), (1500.0, AX)]));
    v
}

fn viewport() -> Vec<Object> {
    vec![
        rect(2, GREY, (810.0, 600.0), (990.0, 1200.0)),
        rect(2, GLASSF, (870.0, 750.0), (930.0, 1050.0)),
        line(2, &[(810.0, 750.0), (990.0, 750.0)]),
        line(2, &[(810.0, 1050.0), (990.0, 1050.0)]),
    ]
}

fn flange() -> Vec<Object> {
    let mut v = vec![
        rect(2, GREY, (720.0, 600.0), (870.0, 1200.0)),
        rect(2, GREY, (930.0, 600.0), (1080.0, 1200.0)),
        rect(1, DARK, (870.0, 660.0), (930.0, 1140.0)),
    ];
    for y in [660.0, 1140.0] {
        v.push(rect(1, DARK, (690.0, y - 30.0), (1110.0, y + 30.0)));
    }
    v
}

fn sample_holder() -> Vec<Object> {
    vec![
        rect(2, Some((GOLD, 20)), (870.0, 750.0), (930.0, 1050.0)),
        rect(2, GREY, (840.0, 1050.0), (960.0, 1110.0)),
        rect(2, GREY, (885.0, 1110.0), (915.0, 1320.0)),
        rect(2, DARK, (780.0, 1320.0), (1020.0, 1380.0)),
    ]
}

fn waveplate(tex: &str) -> Vec<Object> {
    vec![
        rect(2, Some((ColorRef(6), 30)), (840.0, 600.0), (960.0, 1200.0)),
        line(1, &[(900.0, 600.0), (900.0, 1200.0)]),
        label(AX, 1380.0, tex),
    ]
}

fn polarizer(dot: bool) -> Vec<Object> {
    let mut v = vec![rect(2, GREY, (840.0, 600.0), (960.0, 1200.0))];
    if dot {
        v.push(disc((900.0, AX), 24.0, 1, DARK));
        v.push(disc((900.0, AX), 60.0, 1, None));
    } else {
        v.push(double_arrow((900.0, 660.0), (900.0, 1140.0)));
    }
    v
}

fn iris() -> Vec<Object> {
    vec![
        rect(2, DARK, (870.0, 540.0), (930.0, 750.0)),
        rect(2, DARK, (870.0, 1050.0), (930.0, 1260.0)),
    ]
}

fn pbs() -> Vec<Object> {
    vec![
        cube(GLASSF),
        line(2, &[(705.0, 1125.0), (1125.0, 705.0)]),
        line(2, &[(675.0, 1095.0), (1095.0, 675.0)]),
        // Polarisation marks outside: in the plane (arrow), out of it (dot).
        double_arrow((1215.0, 660.0), (1215.0, 780.0)),
        disc((1215.0, 1020.0), 24.0, 1, DARK),
        disc((1215.0, 1020.0), 60.0, 1, None),
    ]
}

fn beam_band(wide: bool) -> Vec<Object> {
    let (a, b) = if wide {
        ([(300.0, 780.0), (1500.0, 780.0)], [(300.0, 1020.0), (1500.0, 1020.0)])
    } else {
        ([(300.0, 870.0), (1500.0, 540.0)], [(300.0, 930.0), (1500.0, 1260.0)])
    };
    vec![
        poly(1, Some((RED, 35)), &[a[0], a[1], b[1], b[0]]),
        beam(a[0], a[1]),
        beam(b[0], b[1]),
    ]
}

fn beam_with_arrow() -> Object {
    let mut l = Polyline::new(
        PolyKind::Polyline,
        beam_style(LineKind::Solid),
        pts(&[(300.0, AX), (1500.0, AX)]),
    );
    l.forward_arrow = Some(Arrow {
        kind: 1,
        style: 1,
        thickness: 1.0,
        width: 75.0,
        height: 120.0,
    });
    Object::new(ObjectKind::Polyline(l))
}

/// (name, what, how, objects) of every plain part.
fn catalog() -> Vec<Entry> {
    let e = |name, what, how, objs| -> Entry { (name, what, how, objs) };
    let mir = "Reflecting side to the left.";
    let turn = "Rotate it with the editing modes for other directions.";
    vec![
        e(
            "laser",
            "laser (box with output aperture and beam)",
            "Red beam leaves at the right, on the grid.",
            laser(),
        ),
        e(
            "beam",
            "laser beam, red line, 1 inch",
            "Ends on the 1/4-inch grid.",
            vec![beam((300.0, AX), (1500.0, AX))],
        ),
        e(
            "beam_arrow",
            "laser beam with a direction arrow, 1 inch",
            "Ends on the 1/4-inch grid.",
            vec![beam_with_arrow()],
        ),
        e(
            "beam_dashed",
            "laser beam, dashed red line (weak or virtual beam), 1 inch",
            "Ends on the 1/4-inch grid.",
            vec![polyline(
                beam_style(LineKind::Dashed),
                pts(&[(300.0, AX), (1500.0, AX)]),
                None,
            )],
        ),
        e(
            "beam_wide",
            "expanded, collimated beam (light red band with red edges), 1 inch",
            "Band 1/4 inch wide.",
            beam_band(true),
        ),
        e(
            "beam_expanding",
            "diverging beam (red cone), 1 inch",
            "Narrow at the left, 3/4 inch wide at the right.",
            beam_band(false),
        ),
        e(
            "lens_convex",
            "convex (biconvex) lens, focusing",
            "Light-blue lens 1.5 inches tall.",
            vec![convex_lens(AX, 450.0, 90.0, 0.0)],
        ),
        e(
            "lens_concave",
            "concave (biconcave) lens, diverging",
            "Light-blue lens 1.5 inches tall.",
            vec![concave_lens(AX, 450.0)],
        ),
        e(
            "lens_cylindrical",
            "cylindrical lens (side view, flat rim)",
            "Focuses in the drawing plane only.",
            vec![convex_lens(AX, 450.0, 90.0, 30.0)],
        ),
        e(
            "mirror",
            "flat mirror, perpendicular to the beam",
            mir,
            flat_mirror((AX, AX), 300.0, false),
        ),
        e(
            "mirror_hatched",
            "flat mirror with hatched back, perpendicular to the beam",
            mir,
            flat_mirror((AX, AX), 300.0, true),
        ),
        e(
            "mirror_45",
            "flat mirror at 45 degrees (turns a beam from the left upwards)",
            turn,
            mirror45((AX, AX), false),
        ),
        e(
            "mirror_45_hatched",
            "flat mirror at 45 degrees with hatched back (turns a beam from the left upwards)",
            turn,
            mirror45((AX, AX), true),
        ),
        e(
            "mirror_concave",
            "concave (focusing) mirror with hatched back",
            mir,
            curved_mirror(AX, -1.0, 300.0, 900.0, true),
        ),
        e(
            "dichroic_mirror",
            "dichroic mirror at 45 degrees (reflects one wavelength, transmits the other)",
            "Tinted plate; a beam from the left is turned upwards.",
            vec![plate45((AX, AX), 540.0, 45.0, Some((ColorRef(23), 25)))],
        ),
        e(
            "beam_splitter_cube",
            "beam splitter cube, 50/50",
            "Diagonal splitting surface; a beam from the left goes on and upwards.",
            vec![cube(GLASSF), line(2, &[(675.0, 1125.0), (1125.0, 675.0)])],
        ),
        e(
            "beam_splitter_plate",
            "beam splitter plate at 45 degrees",
            "Thin plate; a beam from the left goes on and upwards.",
            vec![plate45((AX, AX), 540.0, 45.0, GLASSF)],
        ),
        e(
            "polarizing_beam_splitter",
            "polarizing beam splitter cube (PBS)",
            "Double diagonal; the marks outside show the transmitted (arrow) and reflected (dot) polarisation.",
            pbs(),
        ),
        e(
            "waveplate_half",
            "half-wave plate (lambda over 2)",
            "Thin yellow plate, label as LaTeX text.",
            waveplate("$\\lambda/2$"),
        ),
        e(
            "waveplate_quarter",
            "quarter-wave plate (lambda over 4)",
            "Thin yellow plate, label as LaTeX text.",
            waveplate("$\\lambda/4$"),
        ),
        e(
            "polarizer",
            "linear polarizer, transmission axis in the plane (arrow)",
            "Grey plate with a double arrow.",
            polarizer(false),
        ),
        e(
            "polarizer_perpendicular",
            "linear polarizer, transmission axis out of the plane (dot)",
            "Grey plate with a dotted circle.",
            polarizer(true),
        ),
        e("iris", "iris (aperture)", "Two jaws around the axis.", iris()),
        e(
            "fibre_coil",
            "optical fibre, coiled, with straight ends",
            "Ends on the 1/4-inch grid.",
            coil(),
        ),
        e(
            "fibre_coupler",
            "fibre coupler: launch lens, holder and fibre",
            "Beam enters at the left, the fibre leaves at the right; flip it for an output coupler.",
            coupler(),
        ),
        e(
            "photodiode",
            "photodiode (circuit symbol in a circle)",
            "Leads on the axis.",
            photodiode(),
        ),
        e(
            "detector",
            "photodetector (grey box, sensitive face at the centre)",
            "The beam hits the face from the left.",
            detector(),
        ),
        e(
            "camera_ccd",
            "camera (CCD or CMOS) with lens barrel",
            "The beam enters on the left.",
            camera(),
        ),
        e(
            "aom",
            "acousto-optic modulator (AOM)",
            "Crystal with transducer below and sound wave fronts (dashed).",
            aom(),
        ),
        e(
            "eom",
            "electro-optic modulator (EOM, Pockels cell)",
            "Crystal between two electrodes, driven by a voltage source.",
            eom(),
        ),
        e(
            "optical_isolator",
            "optical isolator (one-way symbol)",
            "Box with a triangle and a bar; light passes from left to right only.",
            isolator(),
        ),
        e("prism", "prism, equilateral", "Light-blue triangle.", prism()),
        e(
            "grating",
            "reflection grating (blazed)",
            "The grooved surface is on the axis.",
            grating(),
        ),
        e(
            "cavity",
            "optical cavity: two curved mirrors with the intracavity beam",
            "Concave sides facing each other, 3/4 inch apart.",
            cavity(),
        ),
        e(
            "vacuum_chamber",
            "vacuum chamber: circle with four flanged ports",
            "The two ports on the axis end on the 1/4-inch grid.",
            vacuum_chamber(),
        ),
        e(
            "cryostat",
            "cryostat: nested shells with windows, cold finger and sample stage",
            "Optical access along the axis; ends on the 1/4-inch grid.",
            cryostat(),
        ),
        e(
            "viewport",
            "vacuum viewport: flange with a window",
            "Side view across the beam.",
            viewport(),
        ),
        e(
            "flange",
            "vacuum flange pair with gasket and bolts",
            "Side view across the beam.",
            flange(),
        ),
        e(
            "sample_holder",
            "sample holder: sample plate on a rod with a base",
            "The (gold) sample is on the axis.",
            sample_holder(),
        ),
    ]
}

fn beam_ends(name: &str) -> Vec<Point> {
    match name {
        "beam" | "beam_arrow" | "beam_dashed" | "beam_wide" | "beam_expanding" | "fibre_coil" | "fibre_coupler"
        | "vacuum_chamber" | "cryostat" => vec![p(300.0, AX), p(1500.0, AX)],
        "laser" => vec![p(1500.0, AX)],
        _ => Vec::new(),
    }
}

/// Put a part's objects at `c` (the part's centre moves there), mirrored
/// left-right first if `flip`, then turned by `deg` degrees counter-clockwise
/// on screen.
fn place(objs: Vec<Object>, c: Pt, deg: f64, flip: bool) -> Vec<Object> {
    let centre = Point::new(AX, AX);
    let mut t = Affine::translate(0.0, 0.0);
    if flip {
        t = Affine::about(Affine::scale(-1.0, 1.0), centre);
    }
    t = t
        .then(&Affine::about(Affine::rotate(deg.to_radians()), centre))
        .then(&Affine::translate(c.0 - AX, c.1 - AX));
    objs.into_iter()
        .map(|mut o| {
            o.transform(&t, false);
            o
        })
        .collect()
}

fn group(name: &str) -> Vec<Object> {
    catalog()
        .into_iter()
        .find(|c| c.0 == name)
        .map(|c| c.3)
        .unwrap_or_default()
}

fn examples(out: &mut Vec<Part>) {
    let mut ex = |name: &str, what: &str, objs: Vec<Object>| {
        let c1 = format!("Optics lab example: {what}");
        out.push(part(
            name,
            objs,
            &[
                &c1,
                "Made from the library's parts; edit the labels (LaTeX text). Keywords: optics lab laser example setup",
            ],
            Vec::new(),
        ));
    };
    let at = |name: &str, c: Pt, deg: f64| place(group(name), c, deg, false);

    // Michelson interferometer: laser from the left, beam splitter, end
    // mirrors to the right and up, detector below.
    let mut o = at("laser", (900.0, 1500.0), 0.0);
    o.push(beam((1500.0, 1500.0), (3900.0, 1500.0)));
    o.push(beam((2700.0, 1500.0), (2700.0, 600.0)));
    o.push(beam((2700.0, 1500.0), (2700.0, 2400.0)));
    o.extend(at("beam_splitter_cube", (2700.0, 1500.0), 0.0));
    o.extend(at("mirror_hatched", (3900.0, 1500.0), 0.0));
    o.extend(at("mirror_hatched", (2700.0, 600.0), 90.0));
    o.extend(at("detector", (2700.0, 2400.0), -90.0));
    o.push(label(2700.0, 1980.0, "BS"));
    o.push(label(3900.0, 1170.0, "$M_1$"));
    o.push(label(2550.0, 510.0, "$M_2$"));
    o.push(label(2700.0, 2850.0, "PD"));
    ex("example_michelson", "Michelson interferometer", o);

    // Mach-Zehnder interferometer.
    let mut o = at("laser", (900.0, 2100.0), 0.0);
    o.push(beam((1500.0, 2100.0), (3900.0, 2100.0)));
    o.push(beam((2100.0, 2100.0), (2100.0, 1200.0)));
    o.push(beam((2100.0, 1200.0), (4800.0, 1200.0)));
    o.push(beam((3900.0, 2100.0), (3900.0, 300.0)));
    o.extend(at("beam_splitter_cube", (2100.0, 2100.0), 0.0));
    o.extend(at("mirror_45_hatched", (3900.0, 2100.0), 0.0));
    o.extend(at("mirror_45_hatched", (2100.0, 1200.0), 0.0));
    o.extend(at("beam_splitter_cube", (3900.0, 1200.0), 0.0));
    o.extend(at("detector", (4800.0, 1200.0), 0.0));
    o.extend(at("detector", (3900.0, 300.0), 90.0));
    o.push(label(2100.0, 2580.0, "$\\mathrm{BS}_1$"));
    o.push(label(3900.0, 2580.0, "$M_1$"));
    o.push(label(1650.0, 1260.0, "$M_2$"));
    o.push(label(4350.0, 1740.0, "$\\mathrm{BS}_2$"));
    o.push(label(5400.0, 1170.0, "$D_1$"));
    o.push(label(4380.0, 270.0, "$D_2$"));
    ex("example_mach_zehnder", "Mach-Zehnder interferometer", o);

    // Fibre-coupled laser to detector.
    let mut o = at("laser", (900.0, 900.0), 0.0);
    o.extend(at("fibre_coupler", (2100.0, 900.0), 0.0));
    o.extend(at("fibre_coil", (3300.0, 900.0), 0.0));
    o.extend(place(group("fibre_coupler"), (4500.0, 900.0), 0.0, true));
    o.push(beam((5100.0, 900.0), (5700.0, 900.0)));
    o.extend(at("detector", (5700.0, 900.0), 0.0));
    o.push(label(900.0, 1380.0, "laser"));
    o.push(label(3300.0, 1380.0, "fibre"));
    o.push(label(5850.0, 1380.0, "PD"));
    ex(
        "example_fibre_path",
        "laser light through an optical fibre to a detector",
        o,
    );
}

/// Every figure of the OpticsLab library, in file-name order.
pub fn optics_lab() -> Vec<Part> {
    let mut v: Vec<Part> = catalog()
        .into_iter()
        .map(|(name, what, how, objs)| {
            let c1 = format!("Optics lab: {what}");
            let c2 = format!(
                "{how} Optical axis through the centre. Keywords: optics laser lens mirror beam fibre detector lab"
            );
            part(name, objs, &[&c1, &c2], beam_ends(name))
        })
        .collect();
    examples(&mut v);
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}
