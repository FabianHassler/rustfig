//! `cargo xtask libraries`: writes the libraries RustFig makes itself (plan
//! D13) as `.fig` files under `libraries/`, from code, so the parts stay
//! consistent and can be changed in one place. Today: `libraries/Feynman/`.
//! The output is deterministic (the same bytes on every run); a test checks
//! that the committed files equal what the generator writes.
//!
//! Conventions: Fig units (1200 per inch), the 1/4-inch grid (300 units; the
//! arrows' mid points on the 1/8-inch grid, 150), every part one compound
//! with a description as its comment (the library search reads it), xfig's
//! standard attributes (thickness 2, black, depth 50; vertices and blobs in
//! front at depth 40).

use rustfig_format::{write, FileFormat};
use rustfig_geom::Point;
use rustfig_model::*;
use std::f64::consts::PI;
use std::path::Path;

/// The grid parts snap to: 1/8 inch (line ends are on the 1/4-inch grid).
#[cfg_attr(not(test), allow(dead_code))] // used by the tests
pub const GRID: f64 = 150.0;

/// One library figure: file name (without `.fig`), the compound, and the
/// points where lines join (all on the grid).
#[cfg_attr(not(test), allow(dead_code))] // `ends` is checked by the tests
pub struct Part {
    pub name: String,
    pub object: Object,
    pub ends: Vec<Point>,
}

fn p(x: f64, y: f64) -> Point {
    Point::new(x.round(), y.round())
}

fn style(line: LineKind, thickness: i32, depth: i32) -> Style {
    Style {
        line,
        thickness,
        depth,
        style_val: match line {
            LineKind::Dashed => 6.0,
            LineKind::Dotted => 4.0,
            _ => 0.0,
        },
        ..Style::default()
    }
}

fn arrow_head(size: f64) -> Arrow {
    Arrow {
        kind: 1,
        style: 1,
        thickness: 1.0,
        width: size * 0.6,
        height: size,
    }
}

fn compound(objects: Vec<Object>, comments: &[&str]) -> Object {
    let mut o = Object::new(ObjectKind::Compound(Compound { objects }));
    o.comments = comments.iter().map(|s| s.to_string()).collect();
    o
}

fn polyline(s: Style, pts: Vec<Point>, fwd: Option<Arrow>) -> Object {
    let mut l = Polyline::new(PolyKind::Polyline, s, pts);
    l.forward_arrow = fwd;
    Object::new(ObjectKind::Polyline(l))
}

fn spline(s: Style, pts: Vec<Point>) -> Object {
    let mut sp = Spline::new(SplineKind::Interpolated, false, s, pts);
    let n = sp.shapes.len();
    for (i, sh) in sp.shapes.iter_mut().enumerate() {
        // xfig's interpolated splines: -1 inside, 0 at the open ends.
        *sh = if i == 0 || i + 1 == n { 0.0 } else { -1.0 };
    }
    Object::new(ObjectKind::Spline(sp))
}

fn arc_obj(s: Style, a: Point, m: Point, b: Point, fwd: Option<Arrow>) -> Option<Object> {
    let mut arc = Arc::through(s, a, m, b, false)?;
    arc.forward_arrow = fwd;
    Some(Object::new(ObjectKind::Arc(arc)))
}

/// The curve a line runs along: a straight segment, or the half circle on
/// the chord `a`-`b`, bulging to the left of the direction of travel.
#[derive(Clone, Copy)]
enum Route {
    Line(Point, Point),
    Arc(Point, Point),
}

impl Route {
    fn ends(self) -> (Point, Point) {
        match self {
            Route::Line(a, b) | Route::Arc(a, b) => (a, b),
        }
    }
    fn length(self) -> f64 {
        let (a, b) = self.ends();
        let c = (b.x - a.x).hypot(b.y - a.y);
        match self {
            Route::Line(..) => c,
            Route::Arc(..) => c * PI / 2.0,
        }
    }
    /// Position and unit normal at `s` in [0, 1] (arc length fraction).
    fn at(self, s: f64) -> (Point, (f64, f64)) {
        let (a, b) = self.ends();
        let c = (b.x - a.x).hypot(b.y - a.y).max(1e-9);
        let (ux, uy) = ((b.x - a.x) / c, (b.y - a.y) / c);
        let n = (uy, -ux);
        match self {
            Route::Line(..) => (Point::new(a.x + (b.x - a.x) * s, a.y + (b.y - a.y) * s), n),
            Route::Arc(..) => {
                let (cx, cy, r) = ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0, c / 2.0);
                let th = PI * s;
                let (c0, s0) = (th.cos(), th.sin());
                let x = cx - r * c0 * ux + r * s0 * n.0;
                let y = cy - r * c0 * uy + r * s0 * n.1;
                (Point::new(x, y), (-c0 * ux + s0 * n.0, -c0 * uy + s0 * n.1))
            }
        }
    }
    fn offset(self, s: f64, d: f64) -> Point {
        let (q, n) = self.at(s);
        p(q.x + n.0 * d, q.y + n.1 * d)
    }
    fn at_rounded(self, s: f64) -> Point {
        let q = self.at(s).0;
        p(q.x, q.y)
    }
    /// Number of wave periods for a wavelength of about 1/4 inch.
    fn periods(self) -> usize {
        ((self.length() / 300.0).round() as usize).max(1)
    }
}

const THICK: i32 = 2;
const AMP: f64 = 75.0;

/// Fermion line with the arrow in the middle: two pieces, one ending in the
/// arrow head. `reverse` points the arrow back (anti-fermion).
fn fermion(path: Route, reverse: bool) -> Vec<Object> {
    let s = style(LineKind::Solid, THICK, 50);
    let (a, b) = path.ends();
    let head = Some(arrow_head(150.0));
    match path {
        Route::Line(..) => {
            let m = p((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            if reverse {
                vec![polyline(s, vec![a, m], None), polyline(s, vec![b, m], head)]
            } else {
                vec![polyline(s, vec![a, m], head), polyline(s, vec![m, b], None)]
            }
        }
        Route::Arc(..) => {
            let (q1, top, q3) = (path.at_rounded(0.25), path.at_rounded(0.5), path.at_rounded(0.75));
            let halves = if reverse {
                [arc_obj(s, b, q3, top, head), arc_obj(s, a, q1, top, None)]
            } else {
                [arc_obj(s, a, q1, top, head), arc_obj(s, top, q3, b, None)]
            };
            halves.into_iter().flatten().collect()
        }
    }
}

/// Dashed (scalar), dotted (ghost) or thin solid line.
fn plain(path: Route, line: LineKind, thickness: i32) -> Vec<Object> {
    let s = style(line, thickness, 50);
    let (a, b) = path.ends();
    match path {
        Route::Line(..) => vec![polyline(s, vec![a, b], None)],
        Route::Arc(..) => arc_obj(s, a, path.at_rounded(0.5), b, None).into_iter().collect(),
    }
}

/// Sine wave (photon, W/Z boson, graviton) along the path: points at the
/// quarter periods of an interpolated spline, so the curve is smooth and the
/// wiggles are evenly spaced.
fn wave(path: Route, thickness: i32) -> Object {
    let n = path.periods();
    let steps = 8 * n;
    let pts: Vec<Point> = (0..=steps)
        .map(|i| {
            let s = i as f64 / steps as f64;
            path.offset(s, AMP * (2.0 * PI * n as f64 * s).sin())
        })
        .collect();
    spline(style(LineKind::Solid, thickness, 50), pts)
}

/// Curly gluon line: a prolate cycloid (loops) along the path, 16 points per
/// loop, starting and ending on the path.
fn gluon(path: Route) -> Object {
    let n = path.periods();
    let l = path.length();
    let a = l / n as f64 / (2.0 * PI);
    let b = 2.0 * a;
    let steps = 16 * n;
    let pts: Vec<Point> = (0..=steps)
        .map(|i| {
            let tau = 2.0 * PI * n as f64 * i as f64 / steps as f64;
            let u = a * tau + b * (1.0 - tau.cos());
            let v = -b * tau.sin();
            path.offset((u / l).clamp(0.0, 1.0), v)
        })
        .collect();
    spline(style(LineKind::Solid, THICK, 50), pts)
}

fn dot(c: Point, r: f64) -> Object {
    let mut s = style(LineKind::Solid, 1, 40);
    s.fill_color = ColorRef::BLACK;
    s.area_fill = 20;
    circle(c, r, s)
}

fn circle(c: Point, r: f64, s: Style) -> Object {
    Object::new(ObjectKind::Ellipse(Ellipse {
        kind: EllipseKind::CircleRadius,
        style: s,
        direction: 1,
        angle: 0.0,
        center: c,
        radii: p(r, r),
        start: c,
        end: p(c.x + r, c.y),
    }))
}

fn momentum(a: Point, b: Point) -> Object {
    polyline(
        style(LineKind::Solid, 1, 50),
        vec![a, b],
        Some(Arrow {
            kind: 1,
            style: 1,
            thickness: 1.0,
            width: 75.0,
            height: 120.0,
        }),
    )
}

/// A LaTeX label (special text) for the examples.
fn label(x: f64, y: f64, tex: &str) -> Object {
    Object::new(ObjectKind::Text(Text {
        justify: Justify::Center,
        color: ColorRef::BLACK,
        depth: 30,
        pen_style: -1,
        font: 0,
        size: 12.0,
        angle: 0.0,
        flags: text_flags::SPECIAL,
        height: 135.0,
        length: 120.0 * tex.chars().count() as f64,
        origin: p(x, y),
        text: tex.to_string(),
        local_font: None,
    }))
}

fn part(name: &str, objects: Vec<Object>, comments: &[&str], ends: Vec<Point>) -> Part {
    Part {
        name: name.to_string(),
        object: compound(objects, comments),
        ends,
    }
}

/// The line parts: horizontal (1 inch, four periods), at 45 degrees, and
/// the half-circle arcs for loops.
fn line_parts(out: &mut Vec<Part>) {
    let h = Route::Line(p(300.0, 900.0), p(1500.0, 900.0));
    let d = Route::Line(p(300.0, 1200.0), p(1200.0, 300.0));
    for (suffix, path, size) in [
        ("h", h, "horizontal, 1 inch"),
        ("45", d, "at 45 degrees, up to the right"),
    ] {
        let ends = vec![path.ends().0, path.ends().1];
        let mk = |name: &str, what: &str, how: &str, objs: Vec<Object>| {
            let c1 = format!("Feynman diagram: {what}, {size}");
            let c2 =
                format!("{how} Ends on the 1/4-inch grid; join parts end to end. Keywords: feynman particle physics");
            part(&format!("{name}_{suffix}"), objs, &[&c1, &c2], ends.clone())
        };
        out.push(mk(
            "fermion",
            "fermion line (electron, quark, muon)",
            "Arrow along the line.",
            fermion(path, false),
        ));
        out.push(mk(
            "anti_fermion",
            "anti-fermion line (positron, antiquark)",
            "Arrow against the line.",
            fermion(path, true),
        ));
        out.push(mk(
            "scalar",
            "scalar line (Higgs, squark), dashed",
            "Dashed.",
            plain(path, LineKind::Dashed, THICK),
        ));
        out.push(mk(
            "ghost",
            "ghost line, dotted",
            "Dotted.",
            plain(path, LineKind::Dotted, THICK),
        ));
        out.push(mk(
            "photon",
            "photon line (gamma), wavy",
            "Sine wave of 1/4-inch wavelength.",
            vec![wave(path, THICK)],
        ));
        out.push(mk(
            "boson_wz",
            "W or Z boson line, heavy wavy",
            "Thicker sine wave.",
            vec![wave(path, 3)],
        ));
        out.push(mk(
            "gluon",
            "gluon line (g), curly",
            "Loops 1/4 inch apart.",
            vec![gluon(path)],
        ));
        let mut g = vec![wave(path, THICK)];
        g.extend(plain(path, LineKind::Solid, 1));
        out.push(mk(
            "graviton",
            "graviton line, wavy with a straight line through it",
            "Double line.",
            g,
        ));
    }
    let arc = Route::Arc(p(300.0, 900.0), p(1500.0, 900.0));
    let ends = vec![arc.ends().0, arc.ends().1];
    let mk = |name: &str, what: &str, objs: Vec<Object>| {
        let c1 = format!("Feynman diagram: {what}, half circle on a chord of 1 inch (loops, self-energy)");
        part(
            name,
            objs,
            &[
                &c1,
                "Ends on the 1/4-inch grid. Keywords: feynman loop particle physics",
            ],
            ends.clone(),
        )
    };
    out.push(mk("fermion_arc", "fermion line", fermion(arc, false)));
    out.push(mk("anti_fermion_arc", "anti-fermion line", fermion(arc, true)));
    out.push(mk("scalar_arc", "scalar line", plain(arc, LineKind::Dashed, THICK)));
    out.push(mk("photon_arc", "photon line", vec![wave(arc, THICK)]));
    out.push(mk("gluon_arc", "gluon line", vec![gluon(arc)]));
}

fn other_parts(out: &mut Vec<Part>) {
    let c = p(300.0, 300.0);
    out.push(part(
        "vertex_dot",
        vec![dot(c, 45.0)],
        &[
            "Feynman diagram: vertex dot (interaction point, counterterm)",
            "Place on the point where lines meet. Keywords: feynman vertex",
        ],
        vec![c],
    ));
    let c = p(600.0, 600.0);
    let mut s = style(LineKind::Solid, THICK, 40);
    s.fill_color = ColorRef::WHITE;
    s.area_fill = 45;
    out.push(part(
        "blob_hatched",
        vec![circle(c, 300.0, s)],
        &[
            "Feynman diagram: hatched blob (effective vertex, form factor, hadronic part)",
            "Circle of 1/2 inch diameter, hatched at 45 degrees. Keywords: feynman blob",
        ],
        vec![c],
    ));
    let mut s = style(LineKind::Solid, THICK, 40);
    s.fill_color = ColorRef::WHITE;
    s.area_fill = 14;
    out.push(part(
        "blob_grey",
        vec![circle(c, 300.0, s)],
        &[
            "Feynman diagram: grey blob (effective vertex, form factor, hadronic part)",
            "Circle of 1/2 inch diameter, grey fill. Keywords: feynman blob",
        ],
        vec![c],
    ));
    for (name, dir, a, b) in [
        ("momentum_right", "to the right", p(300.0, 300.0), p(900.0, 300.0)),
        ("momentum_left", "to the left", p(900.0, 300.0), p(300.0, 300.0)),
        ("momentum_up", "up", p(300.0, 900.0), p(300.0, 300.0)),
        ("momentum_down", "down", p(300.0, 300.0), p(300.0, 900.0)),
    ] {
        let c1 = format!("Feynman diagram: momentum arrow {dir}");
        out.push(part(
            name,
            vec![momentum(a, b)],
            &[
                &c1,
                "Short arrow of 1/2 inch to place beside a line; add the label k or p as LaTeX text. Keywords: feynman momentum",
            ],
            vec![a, b],
        ));
    }
}

/// The complete example diagrams, built from the same generators as the parts.
fn examples(out: &mut Vec<Part>) {
    let line = Route::Line;
    let mut ex = |name: &str, what: &str, objs: Vec<Object>| {
        let c1 = format!("Feynman diagram example: {what}");
        out.push(part(
            name,
            objs,
            &[
                &c1,
                "Made from the library's parts; edit the labels (LaTeX text). Keywords: feynman example",
            ],
            Vec::new(),
        ));
    };

    // e+ e- -> mu+ mu-, s channel.
    let (v1, v2) = (p(1200.0, 1500.0), p(2400.0, 1500.0));
    let mut o = Vec::new();
    o.extend(fermion(line(p(300.0, 2400.0), v1), false));
    o.extend(fermion(line(p(300.0, 600.0), v1), true));
    o.push(wave(line(v1, v2), THICK));
    o.extend(fermion(line(v2, p(3300.0, 2400.0)), false));
    o.extend(fermion(line(v2, p(3300.0, 600.0)), true));
    o.push(dot(v1, 45.0));
    o.push(dot(v2, 45.0));
    o.push(label(300.0, 2700.0, "$e^-$"));
    o.push(label(300.0, 450.0, "$e^+$"));
    o.push(label(3300.0, 2700.0, "$\\mu^-$"));
    o.push(label(3300.0, 450.0, "$\\mu^+$"));
    o.push(label(1800.0, 1200.0, "$\\gamma^*$"));
    ex(
        "example_ee_to_mumu",
        "e+ e- annihilation into mu+ mu- (s channel, one virtual photon)",
        o,
    );

    // Compton scattering, s channel.
    let (v1, v2) = (p(1200.0, 1800.0), p(2400.0, 1800.0));
    let mut o = Vec::new();
    o.extend(fermion(line(p(300.0, 1800.0), v1), false));
    o.extend(fermion(line(v1, v2), false));
    o.extend(fermion(line(v2, p(3300.0, 1800.0)), false));
    o.push(wave(line(p(300.0, 900.0), v1), THICK));
    o.push(wave(line(v2, p(3300.0, 900.0)), THICK));
    o.push(dot(v1, 45.0));
    o.push(dot(v2, 45.0));
    o.push(label(300.0, 2100.0, "$e^-$"));
    o.push(label(3300.0, 2100.0, "$e^-$"));
    o.push(label(300.0, 750.0, "$\\gamma$"));
    o.push(label(3300.0, 750.0, "$\\gamma$"));
    o.push(label(1800.0, 2100.0, "$p+k$"));
    ex(
        "example_compton",
        "Compton scattering e- gamma to e- gamma (s channel)",
        o,
    );

    // One-loop fermion self-energy.
    let (v1, v2) = (p(1200.0, 1500.0), p(2400.0, 1500.0));
    let mut o = Vec::new();
    o.extend(fermion(line(p(300.0, 1500.0), v1), false));
    o.extend(fermion(line(v1, v2), false));
    o.push(wave(Route::Arc(v1, v2), THICK));
    o.extend(fermion(line(v2, p(3300.0, 1500.0)), false));
    o.push(dot(v1, 45.0));
    o.push(dot(v2, 45.0));
    o.push(label(300.0, 1800.0, "$p$"));
    o.push(label(3300.0, 1800.0, "$p$"));
    o.push(label(1800.0, 1800.0, "$p-k$"));
    o.push(label(1800.0, 600.0, "$k$"));
    ex(
        "example_self_energy",
        "one-loop fermion self-energy (photon loop on a fermion line)",
        o,
    );

    // Gluon exchange between two quarks (t channel).
    let (v1, v2) = (p(1200.0, 900.0), p(1200.0, 2100.0));
    let mut o = Vec::new();
    for v in [v1, v2] {
        o.extend(fermion(line(p(300.0, v.y), v), false));
        o.extend(fermion(line(v, p(2100.0, v.y)), false));
        o.push(dot(v, 45.0));
    }
    o.push(gluon(line(v1, v2)));
    o.push(label(300.0, 750.0, "$q$"));
    o.push(label(2100.0, 750.0, "$q$"));
    o.push(label(300.0, 2400.0, "$q$"));
    o.push(label(2100.0, 2400.0, "$q$"));
    o.push(label(1500.0, 1500.0, "$g$"));
    ex(
        "example_gluon_exchange",
        "quark-quark scattering by one gluon exchange (t channel)",
        o,
    );
}

/// Every figure of the Feynman library, in file-name order.
pub fn feynman() -> Vec<Part> {
    let mut v = Vec::new();
    line_parts(&mut v);
    other_parts(&mut v);
    examples(&mut v);
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

/// The `.fig` text of a part.
pub fn render(part: &Part) -> Result<String, String> {
    let mut doc = Document::new();
    doc.settings.units = Units::Inches;
    doc.objects.push(std::sync::Arc::new(part.object.clone()));
    doc.assign_ids();
    write(&doc, FileFormat::Fig).map_err(|e| format!("{}: {e:?}", part.name))
}

/// Write `libraries/Feynman/*.fig`.
pub fn run(root: &Path) -> Result<(), String> {
    let dir = root.join("libraries").join("Feynman");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let parts = feynman();
    for part in &parts {
        let path = dir.join(format!("{}.fig", part.name));
        std::fs::write(&path, render(part)?).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    println!("libraries: wrote {} figures to {}", parts.len(), dir.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generator_is_deterministic() {
        let a: Vec<String> = feynman().iter().map(|p| render(p).unwrap()).collect();
        let b: Vec<String> = feynman().iter().map(|p| render(p).unwrap()).collect();
        assert_eq!(a, b);
        assert!(a.len() > 30);
    }

    #[test]
    fn names_are_unique_and_plain() {
        let parts = feynman();
        let mut names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), parts.len());
        for n in names {
            assert!(n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'), "{n}");
        }
    }

    #[test]
    fn every_figure_reads_back_as_one_compound_with_a_comment() {
        for part in feynman() {
            let text = render(&part).unwrap();
            let doc = rustfig_format::read_any(text.as_bytes()).unwrap();
            assert_eq!(doc.objects.len(), 1, "{}", part.name);
            let o = &doc.objects[0];
            assert!(matches!(o.kind, ObjectKind::Compound(_)), "{}", part.name);
            assert!(
                o.comments.first().is_some_and(|c| c.starts_with("Feynman diagram")),
                "{}",
                part.name
            );
        }
    }

    #[test]
    fn part_ends_lie_on_the_grid() {
        for part in feynman() {
            for e in &part.ends {
                assert!(
                    (e.x / GRID).fract() == 0.0 && (e.y / GRID).fract() == 0.0,
                    "{} {e:?}",
                    part.name
                );
            }
        }
    }

    #[test]
    fn committed_files_match_the_generator() {
        let dir = crate::root().join("libraries").join("Feynman");
        for part in feynman() {
            let path = dir.join(format!("{}.fig", part.name));
            let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
            assert_eq!(
                on_disk.replace("\r\n", "\n"),
                render(&part).unwrap(),
                "{} is stale: run cargo xtask libraries",
                path.display()
            );
        }
    }
}
