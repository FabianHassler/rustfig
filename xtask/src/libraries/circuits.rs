//! The PhysicsCircuits library (plan D13): lumped circuit elements the way
//! physics papers draw them (superconducting circuits, rf and lab
//! electronics), and three examples. Every part is drawn in a 1800 x 1800
//! frame (1.5 inch) around the centre (900, 900), horizontal first.
//!
//! * Two-terminal parts: the pins are (300, 900) and (1500, 900), exactly
//!   one inch apart, on the 1/4-inch grid. The `_v` version is the part turned
//!   by 90 degrees: pins (900, 300) and (900, 1500).
//! * Grounds: one pin; `_v` has it at the top (900, 300) with the symbol
//!   below, `_h` has it at the left (300, 900).
//! * Transformers: four pins at (600, 300), (600, 1500), (1200, 300),
//!   (1200, 1500) (`_v`); `_h` is turned.
//!
//! The set fills the gaps of xfig's Electronic library: it has no Josephson
//! junction, SQUID, transmission line, tunable elements or protective earth,
//! and its parts do not share one length.

use super::*;
use rustfig_geom::Affine;

type Pt = (f64, f64);
type Entry = (String, String, Vec<Object>, Vec<Pt>);

/// Frame centre and the two pins of a horizontal two-terminal part.
const C: f64 = 900.0;
const PIN_A: Pt = (300.0, 900.0);
const PIN_B: Pt = (1500.0, 900.0);

/// Names of the two-terminal parts (each has `_h` and `_v`).
pub const TWO_TERMINAL: [&str; 17] = [
    "resistor_iec",
    "resistor_zigzag",
    "capacitor",
    "inductor",
    "jj_box",
    "jj_cross",
    "squid",
    "voltage_dc",
    "voltage_ac",
    "current_dc",
    "current_ac",
    "battery",
    "transmission_line",
    "resistor_var",
    "capacitor_var",
    "inductor_var",
    "jj_var",
];

fn pt(q: Pt) -> Point {
    p(q.0, q.1)
}

fn ln(th: i32, v: &[Pt]) -> Object {
    polyline(solid(th, 50), v.iter().map(|&q| pt(q)).collect(), None)
}

fn lead(a: Pt, b: Pt) -> Object {
    ln(2, &[a, b])
}

fn rect(th: i32, a: Pt, b: Pt) -> Object {
    Object::new(ObjectKind::Polyline(Polyline::rect(solid(th, 50), pt(a), pt(b))))
}

fn ring(c: Pt, r: f64, th: i32) -> Object {
    circle(pt(c), r, solid(th, 50))
}

fn ellipse(c: Pt, rx: f64, ry: f64) -> Object {
    Object::new(ObjectKind::Ellipse(Ellipse {
        kind: EllipseKind::EllipseRadii,
        style: solid(2, 50),
        direction: 1,
        angle: 0.0,
        center: pt(c),
        radii: p(rx, ry),
        start: pt(c),
        end: p(c.0 + rx, c.1 + ry),
    }))
}

fn filled_dot(c: Pt, r: f64) -> Object {
    let mut s = solid(1, 40);
    s.fill_color = ColorRef::BLACK;
    s.area_fill = 20;
    circle(pt(c), r, s)
}

fn arrow(a: Pt, b: Pt) -> Object {
    polyline(solid(1, 48), vec![pt(a), pt(b)], Some(arrow_head(120.0)))
}

/// A thin line with an arrow head at each end.
fn double_arrow(a: Pt, b: Pt) -> Object {
    let mut l = Polyline::new(PolyKind::Polyline, solid(1, 48), vec![pt(a), pt(b)]);
    l.forward_arrow = Some(arrow_head(120.0));
    l.backward_arrow = Some(arrow_head(120.0));
    Object::new(ObjectKind::Polyline(l))
}

/// The tunable-element arrow, across the part.
fn tune() -> Object {
    arrow((690.0, 1110.0), (1110.0, 690.0))
}

fn wave(x0: f64, x1: f64, y: f64, amp: f64) -> Object {
    let n = 16;
    let v: Vec<Pt> = (0..=n)
        .map(|i| {
            let t = f64::from(i) / f64::from(n);
            (x0 + (x1 - x0) * t, y - amp * (2.0 * PI * t).sin())
        })
        .collect();
    ln(2, &v)
}

/// Leads from the pins to `a` and from `b` to the other pin.
fn leads(a: f64, b: f64) -> Vec<Object> {
    vec![lead(PIN_A, (a, C)), lead((b, C), PIN_B)]
}

fn cross(c: Pt, h: f64) -> Vec<Object> {
    vec![
        ln(3, &[(c.0 - h, c.1 - h), (c.0 + h, c.1 + h)]),
        ln(3, &[(c.0 - h, c.1 + h), (c.0 + h, c.1 - h)]),
    ]
}

fn zigzag() -> Object {
    let mut v = vec![PIN_A, (540.0, C)];
    for (i, y) in [780.0, 1020.0, 780.0, 1020.0, 780.0, 1020.0].iter().enumerate() {
        v.push((600.0 + 120.0 * i as f64, *y));
    }
    v.push((1260.0, C));
    v.push(PIN_B);
    ln(2, &v)
}

/// A coil of `n` humps between `x0` and `x1` on the line y = 900, bulging up.
fn coil(x0: f64, x1: f64, n: usize) -> Vec<Pt> {
    let w = (x1 - x0) / n as f64;
    let mut v = Vec::new();
    for h in 0..n {
        for i in 0..=10 {
            if h > 0 && i == 0 {
                continue;
            }
            let t = PI * f64::from(i) / 10.0;
            v.push((x0 + w * h as f64 + w / 2.0 * (1.0 - t.cos()), C - w / 2.0 * t.sin()));
        }
    }
    v
}

fn inductor() -> Vec<Object> {
    let mut v = vec![PIN_A];
    v.extend(coil(540.0, 1260.0, 4));
    v.push(PIN_B);
    vec![ln(2, &v)]
}

fn capacitor() -> Vec<Object> {
    let mut o = leads(840.0, 960.0);
    o.push(ln(3, &[(840.0, 720.0), (840.0, 1080.0)]));
    o.push(ln(3, &[(960.0, 720.0), (960.0, 1080.0)]));
    o
}

fn jj_box() -> Vec<Object> {
    let mut o = leads(780.0, 1020.0);
    o.push(rect(2, (780.0, 780.0), (1020.0, 1020.0)));
    o.extend(cross((900.0, 900.0), 120.0));
    o
}

fn jj_cross() -> Vec<Object> {
    let mut o = leads(780.0, 1020.0);
    o.extend(cross((900.0, 900.0), 120.0));
    o
}

fn squid() -> Vec<Object> {
    let mut o = vec![
        lead(PIN_A, (600.0, C)),
        lead((1200.0, C), PIN_B),
        ln(2, &[(600.0, C), (600.0, 660.0), (840.0, 660.0)]),
        ln(2, &[(960.0, 660.0), (1200.0, 660.0), (1200.0, C)]),
        ln(2, &[(600.0, C), (600.0, 1140.0), (840.0, 1140.0)]),
        ln(2, &[(960.0, 1140.0), (1200.0, 1140.0), (1200.0, C)]),
    ];
    o.extend(cross((900.0, 660.0), 60.0));
    o.extend(cross((900.0, 1140.0), 60.0));
    o
}

const SR: f64 = 270.0;

fn source_circle() -> Vec<Object> {
    let mut o = leads(900.0 - SR, 900.0 + SR);
    o.push(ring((C, C), SR, 2));
    o
}

fn voltage_dc() -> Vec<Object> {
    let mut o = source_circle();
    o.push(ln(2, &[(750.0, C), (810.0, C)]));
    o.push(ln(2, &[(780.0, 870.0), (780.0, 930.0)]));
    o.push(ln(2, &[(990.0, C), (1050.0, C)]));
    o
}

fn voltage_ac() -> Vec<Object> {
    let mut o = source_circle();
    o.push(wave(780.0, 1020.0, C, 60.0));
    o
}

fn current_dc() -> Vec<Object> {
    let mut o = source_circle();
    o.push(arrow((720.0, C), (1080.0, C)));
    o
}

fn current_ac() -> Vec<Object> {
    let mut o = source_circle();
    o.push(wave(780.0, 1020.0, 810.0, 45.0));
    o.push(arrow((750.0, 1000.0), (1050.0, 1000.0)));
    o
}

fn battery() -> Vec<Object> {
    let mut o = leads(840.0, 960.0);
    o.push(ln(3, &[(840.0, 690.0), (840.0, 1110.0)]));
    o.push(ln(3, &[(960.0, 780.0), (960.0, 1020.0)]));
    o.push(ln(2, &[(715.0, 800.0), (775.0, 800.0)]));
    o.push(ln(2, &[(745.0, 770.0), (745.0, 830.0)]));
    o
}

fn transmission_line() -> Vec<Object> {
    let mut o = leads(570.0, 1230.0);
    o.push(ellipse((C, C), 330.0, 150.0));
    o.push(ln(2, &[(600.0, C), (1200.0, C)]));
    o
}

fn two_terminal(name: &str) -> Vec<Object> {
    match name {
        "resistor_iec" => {
            let mut o = leads(600.0, 1200.0);
            o.push(rect(2, (600.0, 780.0), (1200.0, 1020.0)));
            o
        }
        "resistor_zigzag" => vec![zigzag()],
        "capacitor" => capacitor(),
        "inductor" => inductor(),
        "jj_box" => jj_box(),
        "jj_cross" => jj_cross(),
        "squid" => squid(),
        "voltage_dc" => voltage_dc(),
        "voltage_ac" => voltage_ac(),
        "current_dc" => current_dc(),
        "current_ac" => current_ac(),
        "battery" => battery(),
        "transmission_line" => transmission_line(),
        "resistor_var" => {
            let mut o = two_terminal("resistor_iec");
            o.push(tune());
            o
        }
        "capacitor_var" => {
            let mut o = capacitor();
            o.push(tune());
            o
        }
        "inductor_var" => {
            let mut o = inductor();
            o.push(tune());
            o
        }
        "jj_var" => {
            let mut o = jj_cross();
            o.push(tune());
            o
        }
        _ => Vec::new(),
    }
}

fn what(name: &str) -> &'static str {
    match name {
        "resistor_iec" => "resistor, rectangle (IEC)",
        "resistor_zigzag" => "resistor, zigzag",
        "capacitor" => "capacitor",
        "inductor" => "inductor (coil)",
        "jj_box" => "Josephson junction, cross in a box",
        "jj_cross" => "Josephson junction, plain cross",
        "squid" => "SQUID loop with two Josephson junctions",
        "voltage_dc" => "DC voltage source (+ on the first pin)",
        "voltage_ac" => "AC voltage source",
        "current_dc" => "DC current source",
        "current_ac" => "AC current source",
        "battery" => "battery (long plate is +)",
        "transmission_line" => "transmission-line segment",
        "resistor_var" => "variable resistor (arrow)",
        "capacitor_var" => "variable capacitor (arrow)",
        "inductor_var" => "variable inductor (arrow)",
        "jj_var" => "tunable Josephson junction (arrow)",
        _ => "",
    }
}

/// Stem from the pin (900, 300) down to `y`.
fn stem(y: f64) -> Object {
    lead((C, 300.0), (C, y))
}

fn bars(y0: f64, dy: f64, halves: &[f64]) -> Vec<Object> {
    halves
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let y = y0 + dy * i as f64;
            ln(3, &[(C - h, y), (C + h, y)])
        })
        .collect()
}

fn triangle(filled: bool) -> Object {
    let mut s = solid(2, 50);
    if filled {
        s.fill_color = ColorRef::BLACK;
        s.area_fill = 20;
    }
    let pp = [(660.0, 600.0), (1140.0, 600.0), (C, 900.0), (660.0, 600.0)];
    Object::new(ObjectKind::Polyline(Polyline::new(
        PolyKind::Polygon,
        s,
        pp.iter().map(|&q| pt(q)).collect(),
    )))
}

/// Ground symbols, drawn for `_v` (pin at the top).
fn ground(name: &str) -> Vec<Object> {
    match name {
        "ground_earth" => {
            let mut o = vec![stem(600.0)];
            o.extend(bars(600.0, 90.0, &[240.0, 150.0, 60.0]));
            o
        }
        "ground_chassis" => {
            let mut o = vec![stem(600.0), ln(3, &[(660.0, 600.0), (1140.0, 600.0)])];
            for i in 0..5 {
                let x = 720.0 + 90.0 * f64::from(i);
                o.push(ln(2, &[(x, 600.0), (x - 90.0, 720.0)]));
            }
            o
        }
        "ground_protective" => {
            let mut o = vec![stem(570.0), ring((C, 780.0), 210.0, 2)];
            o.extend(bars(720.0, 60.0, &[150.0, 100.0, 50.0]));
            o
        }
        "ground_analog" => vec![stem(600.0), triangle(false)],
        "ground_digital" => vec![stem(600.0), triangle(true)],
        _ => Vec::new(),
    }
}

/// A transformer or mutual inductance, drawn for `_v`: coil axes vertical.
fn coupled(core: bool) -> Vec<Object> {
    let side = |x: f64, k: f64| {
        let mut v = vec![(x, 300.0)];
        // Turn the horizontal coil: its run (x) becomes y, its bulge x.
        v.extend(coil(540.0, 1260.0, 4).iter().map(|&(cx, cy)| (x + k * (C - cy), cx)));
        v.push((x, 1500.0));
        ln(2, &v)
    };
    let mut o = vec![side(600.0, -1.0), side(1200.0, 1.0)];
    if core {
        o.push(ln(2, &[(870.0, 540.0), (870.0, 1260.0)]));
        o.push(ln(2, &[(930.0, 540.0), (930.0, 1260.0)]));
        o.push(filled_dot((690.0, 450.0), 30.0));
        o.push(filled_dot((1110.0, 450.0), 30.0));
    } else {
        o.push(double_arrow((720.0, C), (1080.0, C)));
    }
    o
}

fn port() -> Vec<Object> {
    vec![lead(PIN_A, (1410.0, C)), ring((1500.0, C), 90.0, 2)]
}

/// Rotate frame objects by `deg` (counter-clockwise on screen) about the
/// frame centre, then move them by `d`.
fn place(objs: &[Object], d: Pt, deg: f64) -> Vec<Object> {
    let t = Affine::about(Affine::rotate(deg.to_radians()), Point::new(C, C)).then(&Affine::translate(d.0, d.1));
    objs.iter()
        .cloned()
        .map(|mut o| {
            o.transform(&t, false);
            o
        })
        .collect()
}

/// A frame point turned by `deg` about the frame centre.
fn rot_pt(q: Pt, deg: f64) -> Pt {
    let (s, c) = deg.to_radians().sin_cos();
    let (x, y) = (q.0 - C, q.1 - C);
    (C + x * c + y * s, C - x * s + y * c)
}

fn catalog() -> Vec<Entry> {
    let mut v: Vec<Entry> = Vec::new();
    let mut add = |name: String, text: String, objs: Vec<Object>, pins: &[Pt], deg: f64| {
        let o = place(&objs, (0.0, 0.0), deg);
        let rp: Vec<Pt> = pins
            .iter()
            .map(|&q| {
                let r = rot_pt(q, deg);
                (r.0.round(), r.1.round())
            })
            .collect();
        v.push((name, text, o, rp));
    };
    for name in TWO_TERMINAL {
        for (suffix, deg, orient) in [("h", 0.0, "horizontal"), ("v", 90.0, "vertical")] {
            add(
                format!("{name}_{suffix}"),
                format!("{}, {orient}; pins 1 inch apart on the grid", what(name)),
                two_terminal(name),
                &[PIN_A, PIN_B],
                deg,
            );
        }
    }
    for (name, text) in [
        ("ground_earth", "earth ground (three bars)"),
        ("ground_chassis", "chassis or frame ground (hatched bar)"),
        ("ground_protective", "protective earth (earth bars in a circle)"),
        ("ground_analog", "analog ground (open triangle)"),
        ("ground_digital", "digital ground (filled triangle)"),
    ] {
        add(
            format!("{name}_v"),
            format!("{text}, pin on top"),
            ground(name),
            &[(C, 300.0)],
            0.0,
        );
        add(
            format!("{name}_h"),
            format!("{text}, pin on the left"),
            ground(name),
            &[(C, 300.0)],
            90.0,
        );
    }
    let pins = [(600.0, 300.0), (600.0, 1500.0), (1200.0, 300.0), (1200.0, 1500.0)];
    for (suffix, deg, orient) in [("h", 90.0, "horizontal"), ("v", 0.0, "vertical")] {
        add(
            format!("transformer_{suffix}"),
            format!("transformer with iron core, {orient}; four pins on the grid"),
            coupled(true),
            &pins,
            deg,
        );
        add(
            format!("mutual_inductance_{suffix}"),
            format!("two inductively coupled coils (mutual inductance), {orient}; four pins on the grid"),
            coupled(false),
            &pins,
            deg,
        );
    }
    for (suffix, deg, orient) in [("h", 0.0, "horizontal"), ("v", 90.0, "vertical")] {
        add(
            format!("port_{suffix}"),
            format!("port: wire ending in an open terminal circle, {orient}; the pin is the free wire end"),
            port(),
            &[PIN_A],
            deg,
        );
    }
    add(
        "terminal".into(),
        "terminal: open circle, the pin is its centre".into(),
        vec![ring((C, C), 90.0, 2)],
        &[(C, C)],
        0.0,
    );
    add(
        "junction".into(),
        "wire junction: filled dot, the pin is its centre".into(),
        vec![filled_dot((C, C), 60.0)],
        &[(C, C)],
        0.0,
    );
    v
}

fn wires(v: &[&[Pt]]) -> Vec<Object> {
    v.iter().filter_map(|w| Some(lead(*w.first()?, *w.get(1)?))).collect()
}

fn examples(out: &mut Vec<Part>) {
    let cat = catalog();
    // A part by file name, moved so that its centre is at `c` (parts with
    // one pin: so that the pin is at `c`).
    let at = |name: &str, c: Pt| -> Vec<Object> {
        let Some(e) = cat.iter().find(|e| e.0 == name) else {
            return Vec::new();
        };
        let one_pin = e.3.len() == 1;
        let q = e.3.first().copied().unwrap_or((C, C));
        let d = if one_pin {
            (c.0 - q.0, c.1 - q.1)
        } else {
            (c.0 - C, c.1 - C)
        };
        place(&e.2, d, 0.0)
    };
    let mut ex = |name: &str, text: &str, objs: Vec<Object>| {
        let c1 = format!("Physics circuits example: {text}");
        out.push(part(
            name,
            objs,
            &[
                &c1,
                "Made from the library's parts; edit the labels (LaTeX text). Keywords: circuit example physics",
            ],
            Vec::new(),
        ));
    };

    // Series RLC circuit driven by an AC source.
    let mut o = at("voltage_ac_v", (600.0, 1500.0));
    o.extend(at("resistor_iec_h", (1500.0, 900.0)));
    o.extend(at("inductor_h", (3300.0, 900.0)));
    o.extend(at("capacitor_v", (3900.0, 1500.0)));
    o.extend(at("ground_earth_v", (2250.0, 2100.0)));
    o.extend(wires(&[
        &[(600.0, 900.0), (900.0, 900.0)],
        &[(2100.0, 900.0), (2700.0, 900.0)],
        &[(600.0, 2100.0), (3900.0, 2100.0)],
    ]));
    o.extend(at("junction", (2250.0, 2100.0)));
    o.push(label(150.0, 1500.0, "$V(t)$"));
    o.push(label(1500.0, 660.0, "$R$"));
    o.push(label(3300.0, 660.0, "$L$"));
    o.push(label(4350.0, 1500.0, "$C$"));
    ex("example_rlc", "series RLC circuit driven by an AC voltage source", o);

    // Transmon: Josephson junction and shunt capacitor to ground, coupled
    // to a drive line through a coupling capacitor.
    let mut o = at("terminal", (300.0, 900.0));
    o.extend(at("capacitor_h", (1200.0, 900.0)));
    o.extend(at("jj_box_v", (2400.0, 1500.0)));
    o.extend(at("capacitor_v", (3300.0, 1500.0)));
    o.extend(at("ground_earth_v", (2850.0, 2100.0)));
    o.extend(wires(&[
        &[(390.0, 900.0), (600.0, 900.0)],
        &[(1800.0, 900.0), (3300.0, 900.0)],
        &[(2400.0, 2100.0), (3300.0, 2100.0)],
    ]));
    o.extend(at("junction", (2400.0, 900.0)));
    o.extend(at("junction", (2850.0, 2100.0)));
    o.push(label(1200.0, 660.0, "$C_c$"));
    o.push(label(1950.0, 1500.0, "$E_J$"));
    o.push(label(3750.0, 1500.0, "$C_S$"));
    o.push(label(300.0, 660.0, "drive"));
    ex(
        "example_transmon",
        "transmon qubit: Josephson junction and shunt capacitor, coupled through a capacitor",
        o,
    );

    // LC resonator coupled to a transmission line.
    let mut o = at("terminal", (300.0, 900.0));
    o.extend(at("transmission_line_h", (1500.0, 900.0)));
    o.extend(at("capacitor_v", (3000.0, 1500.0)));
    o.extend(at("inductor_v", (2400.0, 2700.0)));
    o.extend(at("capacitor_v", (3600.0, 2700.0)));
    o.extend(at("ground_earth_v", (3000.0, 3300.0)));
    o.extend(at("terminal", (4200.0, 900.0)));
    o.extend(wires(&[
        &[(390.0, 900.0), (900.0, 900.0)],
        &[(2100.0, 900.0), (4110.0, 900.0)],
        &[(2400.0, 2100.0), (3600.0, 2100.0)],
        &[(2400.0, 3300.0), (3600.0, 3300.0)],
    ]));
    o.extend(at("junction", (3000.0, 900.0)));
    o.extend(at("junction", (3000.0, 2100.0)));
    o.extend(at("junction", (3000.0, 3300.0)));
    o.push(label(1500.0, 660.0, "$Z_0$"));
    o.push(label(3450.0, 1500.0, "$C_c$"));
    o.push(label(1950.0, 2700.0, "$L_r$"));
    o.push(label(4050.0, 2700.0, "$C_r$"));
    ex(
        "example_resonator",
        "LC resonator coupled to a transmission line through a capacitor",
        o,
    );
}

/// Every figure of the PhysicsCircuits library, in file-name order.
pub fn physics_circuits() -> Vec<Part> {
    let mut v: Vec<Part> = catalog()
        .into_iter()
        .map(|(name, text, objs, pins)| {
            let c1 = format!("Physics circuits: {text}");
            let c2 = "Keywords: circuit physics electronics resistor capacitor inductor Josephson ground source";
            part(&name, objs, &[&c1, c2], pins.iter().map(|&q| p(q.0, q.1)).collect())
        })
        .collect();
    examples(&mut v);
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}
