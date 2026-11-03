//! The four figures generated from mathematics (port of the Python generator
//! `generate.py`): the Clifford attractor, the Apollonian gasket, the Penrose
//! rhomb tiling and the Gosper curve. Each is written as Fig 3.2 text,
//! formatted exactly as the Python script formatted it.

use super::pyish::{hsv_to_rgb, round_i, Complex};
use std::collections::HashSet;
use std::f64::consts::PI;
use std::fmt::Write as _;

/// Page: Letter landscape, 11 x 8.5 in = 13200 x 10200 Fig units; drawings centerd.
const W: f64 = 13200.0;
const H: f64 = 10200.0;

type Pt = (f64, f64);

/// A figure under construction: header settings, user colors, body text.
struct Fig {
    mag: f64,
    paper: &'static str,
    comment: String,
    /// `#rrggbb` -> user color number (32, 33, ...), in order of first use.
    colors: Vec<(String, i32)>,
    body: String,
}

/// Options of a polyline (Python keyword arguments of `Fig.poly`).
struct Poly {
    fill: i32,
    depth: i32,
    thick: i32,
    closed: bool,
    cap: i32,
    join: i32,
}

impl Poly {
    const DEFAULT: Poly = Poly {
        fill: -1,
        depth: 50,
        thick: 1,
        closed: false,
        cap: 0,
        join: 0,
    };
}

impl Fig {
    fn new(comment: &str, mag: f64, paper: &'static str) -> Fig {
        Fig {
            mag,
            paper,
            comment: comment.to_string(),
            colors: Vec::new(),
            body: String::new(),
        }
    }

    /// User color number for (r, g, b) floats in 0..1.
    fn color(&mut self, rgb: (f64, f64, f64)) -> i32 {
        let ch = |c: f64| round_i(c * 255.0).clamp(0, 255);
        let (r, g, b) = (ch(rgb.0), ch(rgb.1), ch(rgb.2));
        let hx = format!("#{r:02x}{g:02x}{b:02x}");
        if let Some((_, n)) = self.colors.iter().find(|(h, _)| *h == hx) {
            return *n;
        }
        if self.colors.len() >= 512 {
            // nearest existing color (the first one on a tie, as Python's min)
            let dist = |h: &str| -> i64 {
                [1usize, 3, 5]
                    .iter()
                    .map(|&i| {
                        let a = h
                            .get(i..i + 2)
                            .and_then(|s| i64::from_str_radix(s, 16).ok())
                            .unwrap_or(0);
                        let b = hx
                            .get(i..i + 2)
                            .and_then(|s| i64::from_str_radix(s, 16).ok())
                            .unwrap_or(0);
                        (a - b) * (a - b)
                    })
                    .sum()
            };
            let mut best: Option<(i64, i32)> = None;
            for (h, n) in &self.colors {
                let d = dist(h);
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, *n));
                }
            }
            return best.map_or(32, |(_, n)| n);
        }
        let n = 32 + self.colors.len() as i32;
        self.colors.push((hx, n));
        n
    }

    fn poly(&mut self, pts: &[Pt], pen: i32, o: &Poly) {
        let mut pts = pts.to_vec();
        let sub = if o.closed { 3 } else { 1 };
        if o.closed && pts.first() != pts.last() {
            if let Some(&first) = pts.first() {
                pts.push(first);
            }
        }
        let area = if o.fill >= 0 { 20 } else { -1 };
        let f = if o.fill >= 0 { o.fill } else { 7 };
        let _ = writeln!(
            self.body,
            "2 {sub} 0 {} {pen} {f} {} -1 {area} 0.000 {} {} -1 0 0 {}",
            o.thick,
            o.depth,
            o.join,
            o.cap,
            pts.len()
        );
        for chunk in pts.chunks(8) {
            let line: Vec<String> = chunk
                .iter()
                .map(|&(x, y)| format!("{} {}", round_i(x), round_i(y)))
                .collect();
            let _ = writeln!(self.body, "\t {}", line.join(" "));
        }
    }

    fn circle(&mut self, cx: f64, cy: f64, r: f64, pen: i32, fill: i32, depth: i32) {
        let area = if fill >= 0 { 20 } else { -1 };
        let f = if fill >= 0 { fill } else { 7 };
        let (cx, cy, r) = (round_i(cx), round_i(cy), round_i(r).max(1));
        let _ = writeln!(
            self.body,
            "1 3 0 1 {pen} {f} {depth} -1 {area} 0.000 1 0.0000 {cx} {cy} {r} {r} {cx} {cy} {} {cy}",
            cx + r
        );
    }

    fn render(&self) -> String {
        let mut out = format!(
            "#FIG 3.2  Produced by RustFig's example generator\nLandscape\nCenter\nInches\n{}\n{:.2}\nSingle\n-2\n1200 2\n",
            self.paper, self.mag
        );
        out.push_str("# ");
        out.push_str(&self.comment);
        out.push('\n');
        for (hx, n) in &self.colors {
            let _ = writeln!(out, "0 {n} {hx}");
        }
        out.push_str(&self.body);
        out
    }
}

/// The black or dark page rectangle behind a figure.
fn page(fig: &mut Fig, fill: i32) {
    let pts = [(0.0, 0.0), (W, 0.0), (W, H), (0.0, H)];
    fig.poly(
        &pts,
        0,
        &Poly {
            fill,
            depth: 999,
            closed: true,
            ..Poly::DEFAULT
        },
    );
}

fn min_max(v: impl Iterator<Item = f64>) -> (f64, f64) {
    v.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| (lo.min(x), hi.max(x)))
}

fn clifford() -> String {
    const N: usize = 250_000;
    let mut fig = Fig::new(
        "Clifford attractor x' = sin(a y) + c cos(a x), y' = sin(b x) + d cos(b y), a=-1.7 b=1.3 c=-0.1 d=-1.21",
        100.0,
        "Letter",
    );
    let (a, b, c, d) = (-1.7f64, 1.3f64, -0.1f64, -1.21f64);
    let (mut x, mut y) = (0.1f64, 0.1f64);
    let mut pts: Vec<Pt> = Vec::with_capacity(N);
    for i in 0..N + 100 {
        let nx = (a * y).sin() + c * (a * x).cos();
        let ny = (b * x).sin() + d * (b * y).cos();
        x = nx;
        y = ny;
        if i >= 100 {
            pts.push((x, y));
        }
    }
    let (x0, x1) = min_max(pts.iter().map(|p| p.0));
    let (y0, y1) = min_max(pts.iter().map(|p| p.1));
    let s = ((W - 1200.0) / (x1 - x0)).min((H - 1200.0) / (y1 - y0));
    let ox = (W - s * (x1 - x0)) / 2.0;
    let oy = (H - s * (y1 - y0)) / 2.0;
    // Black page, dots colored by the direction of the last step (24 hues).
    page(&mut fig, 0);
    let hues: Vec<i32> = (0..24)
        .map(|k| fig.color(hsv_to_rgb(k as f64 / 24.0, 0.75, 1.0)))
        .collect();
    let (mut px, mut py) = pts.first().copied().unwrap_or((0.0, 0.0));
    for &(x, y) in &pts {
        let ang = (y - py).atan2(x - px);
        let k = (((ang + PI) / (2.0 * PI) * 24.0) as i64).rem_euclid(24) as usize;
        let xx = ox + (x - x0) * s;
        let yy = oy + (y - y0) * s;
        let pen = hues.get(k).copied().unwrap_or(0);
        fig.poly(
            &[(xx, yy)],
            pen,
            &Poly {
                cap: 1,
                ..Poly::DEFAULT
            },
        );
        px = x;
        py = y;
    }
    fig.render()
}

/// One circle of the gasket: curvature, center.
type Kz = (f64, Complex);

/// Descartes' circle theorem in complex form: the curvature and the two
/// candidate centers of the circles tangent to three given ones.
fn solve(a: Kz, b: Kz, c: Kz) -> (f64, [Complex; 2]) {
    let ((ka, za), (kb, zb), (kc, zc)) = (a, b, c);
    let k4 = ka + kb + kc + 2.0 * (ka * kb + kb * kc + kc * ka).abs().sqrt();
    let t1 = Complex::real(ka * kb).mul(za).mul(zb);
    let t2 = Complex::real(kb * kc).mul(zb).mul(zc);
    let t3 = Complex::real(kc * ka).mul(zc).mul(za);
    let root = Complex::real(2.0).mul(t1.add(t2).add(t3).sqrt_pow());
    let s = Complex::real(ka)
        .mul(za)
        .add(Complex::real(kb).mul(zb))
        .add(Complex::real(kc).mul(zc));
    let k = Complex::real(k4);
    (k4, [s.add(root).div(k), s.sub(root).div(k)])
}

/// Is the circle (center z, curvature k) tangent to the circle (za, ka)?
fn tangent(z: Complex, k: f64, za: Complex, ka: f64) -> bool {
    let d = z.sub(za).abs();
    let (ra, r) = (1.0 / ka, 1.0 / k);
    (d - (ra + r).abs()).abs() < 1e-3 * r.abs() + 0.5 || (d - (ra.abs() - r).abs()).abs() < 1e-3 * r.abs() + 0.5
}

fn apollonian() -> String {
    let min_r = 8.0;
    // Drawn at twice the size and exported at 50 % (the file's magnification), so
    // thickness-1 outlines come out at 1/320 inch, finer than Fig's 1/160.
    let mut fig = Fig::new(
        "Apollonian gasket from Descartes' circle theorem (complex form), drawn at 2x, exported at 50 %",
        50.0,
        "Tabloid",
    );
    let rr = 9600.0;
    let (cx0, cy0) = (W, H);
    // Outer circle (curvature -1/R) and two inner circles of radius R/2.
    let (k1, z1) = (-1.0 / rr, Complex::ZERO);
    let (k2, z2) = (2.0 / rr, Complex::new(-rr / 2.0, 0.0));
    let (k3, z3) = (2.0 / rr, Complex::new(rr / 2.0, 0.0));
    let mut circles: Vec<(Complex, f64, i32)> = vec![(z1, rr, 0), (z2, rr / 2.0, 1), (z3, rr / 2.0, 1)];
    // The two circles tangent to all three starting ones.
    let (k4, cands) = solve((k1, z1), (k2, z2), (k3, z3));
    for z4 in cands {
        circles.push((z4, 1.0 / k4, 2));
    }
    // Iterative recursion on triples (a, b, new): fill the three gaps around each new circle.
    let mut work: Vec<(Kz, Kz, Kz, i32)> = Vec::new();
    for z4 in cands {
        work.push(((k1, z1), (k2, z2), (k4, z4), 3));
        work.push(((k1, z1), (k3, z3), (k4, z4), 3));
        work.push(((k2, z2), (k3, z3), (k4, z4), 3));
    }
    while let Some((a, b, c, gen)) = work.pop() {
        let (k4, cands) = solve(a, b, c);
        let r = 1.0 / k4;
        if r < min_r {
            continue;
        }
        let mut z4 = None;
        for cand in cands {
            if tangent(cand, k4, a.1, a.0) && tangent(cand, k4, b.1, b.0) && tangent(cand, k4, c.1, c.0) {
                // skip the circle we came from
                if circles.last().iter().all(|(z, _, _)| cand.sub(*z).abs() > 1e-6 * rr) {
                    z4 = Some(cand);
                    break;
                }
            }
        }
        let Some(z4) = z4 else { continue };
        circles.push((z4, r, gen));
        work.push((a, b, (k4, z4), gen + 1));
        work.push((a, c, (k4, z4), gen + 1));
        work.push((b, c, (k4, z4), gen + 1));
    }
    // Deduplicate (the same gap can be reached twice).
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for &(z, r, g) in &circles {
        if seen.insert((round_i(z.re), round_i(z.im), round_i(r))) {
            out.push((z, r, g));
        }
    }
    let pen = fig.color((0.12, 0.12, 0.18));
    out.sort_by(|p, q| (-p.1).partial_cmp(&(-q.1)).unwrap_or(std::cmp::Ordering::Equal));
    for (z, r, g) in out {
        let h = (0.58 + 0.07 * g as f64) % 1.0;
        let col = fig.color(hsv_to_rgb(h, 0.55, 0.95));
        let fill = if g == 0 { fig.color((0.97, 0.96, 0.92)) } else { col };
        let rgb = hsv_to_rgb(h, 0.55, 0.95);
        let edge = if g != 0 {
            fig.color((0.55 * rgb.0, 0.55 * rgb.1, 0.55 * rgb.2))
        } else {
            pen
        };
        fig.circle(cx0 + z.re, cy0 + z.im, r, edge, fill, (900 - g * 10).max(1));
    }
    fig.render()
}

fn penrose() -> String {
    let divisions = 7;
    let mut fig = Fig::new(
        "Penrose P3 rhomb tiling by Robinson-triangle subdivision, 7 deflations",
        100.0,
        "Letter",
    );
    let phi = (1.0 + 5.0f64.powf(0.5)) / 2.0;
    let phi_c = Complex::real(phi);
    // Wheel of 10 red triangles around the center.
    let mut tris: Vec<(i32, Complex, Complex, Complex)> = Vec::new();
    for i in 0..10 {
        let a1 = (2 * i - 1) as f64 * PI / 10.0;
        let a2 = (2 * i + 1) as f64 * PI / 10.0;
        let mut b = Complex::new(a1.cos(), a1.sin());
        let mut c = Complex::new(a2.cos(), a2.sin());
        if i % 2 == 0 {
            std::mem::swap(&mut b, &mut c);
        }
        tris.push((0, Complex::ZERO, b, c));
    }
    for _ in 0..divisions {
        let mut nxt = Vec::with_capacity(tris.len() * 3);
        for &(col, a, b, c) in &tris {
            if col == 0 {
                let p = a.add(b.sub(a).div(phi_c));
                nxt.push((0, c, p, b));
                nxt.push((1, p, c, a));
            } else {
                let q = b.add(a.sub(b).div(phi_c));
                let r = b.add(c.sub(b).div(phi_c));
                nxt.push((1, r, c, a));
                nxt.push((1, q, r, b));
                nxt.push((0, r, q, a));
            }
        }
        tris = nxt;
    }
    // Each Robinson triangle is half a rhomb: fill it without outline, then draw
    // its two legs, so the shared base stays invisible and rhombs appear.
    let s = 8400.0;
    let (cx, cy) = (W / 2.0, H / 2.0);
    let thick_col = fig.color((0.93, 0.66, 0.30));
    let thin_col = fig.color((0.12, 0.43, 0.50));
    let pen = fig.color((0.10, 0.10, 0.14));
    let mut legs: HashSet<((i64, i64), (i64, i64))> = HashSet::new();
    for &(col, a, b, c) in &tris {
        let p: Vec<Pt> = [a, b, c].iter().map(|z| (cx + s * z.re, cy + s * z.im)).collect();
        if !p.iter().all(|&(x, y)| (0.0..=W).contains(&x) && (0.0..=H).contains(&y)) {
            continue;
        }
        let fill = if col == 0 { thin_col } else { thick_col };
        fig.poly(
            &p,
            fill,
            &Poly {
                fill,
                closed: true,
                depth: 60,
                thick: 0,
                ..Poly::DEFAULT
            },
        );
        let Some(&p0) = p.first() else { continue };
        for &q in p.iter().skip(1) {
            let k0 = (round_i(p0.0), round_i(p0.1));
            let k1 = (round_i(q.0), round_i(q.1));
            let key = if k0 <= k1 { (k0, k1) } else { (k1, k0) };
            if legs.insert(key) {
                fig.poly(
                    &[p0, q],
                    pen,
                    &Poly {
                        cap: 1,
                        ..Poly::DEFAULT
                    },
                );
            }
        }
    }
    fig.render()
}

fn gosper() -> String {
    let order = 5;
    let mut fig = Fig::new(
        "Gosper flowsnake: L-system A -> A-B--B+A++AA+B-, B -> +A-BB--B-A++A+B, 60 degrees, order 5, one polyline",
        100.0,
        "Letter",
    );
    let mut s = String::from("A");
    for _ in 0..order {
        s = s
            .chars()
            .map(|ch| match ch {
                'A' => "A-B--B+A++AA+B-".to_string(),
                'B' => "+A-BB--B-A++A+B".to_string(),
                other => other.to_string(),
            })
            .collect();
    }
    let (mut x, mut y, mut ang) = (0.0f64, 0.0f64, 0.0f64);
    let mut pts: Vec<Pt> = vec![(x, y)];
    for ch in s.chars() {
        match ch {
            'A' | 'B' => {
                x += ang.cos();
                y += ang.sin();
                pts.push((x, y));
            }
            '+' => ang += PI / 3.0,
            '-' => ang -= PI / 3.0,
            _ => {}
        }
    }
    let (x0, x1) = min_max(pts.iter().map(|p| p.0));
    let (y0, y1) = min_max(pts.iter().map(|p| p.1));
    let sc = ((W - 1200.0) / (x1 - x0)).min((H - 1200.0) / (y1 - y0));
    let ox = (W - sc * (x1 - x0)) / 2.0;
    let oy = (H - sc * (y1 - y0)) / 2.0;
    let p: Vec<Pt> = pts
        .iter()
        .map(|&(px, py)| (ox + (px - x0) * sc, oy + (py - y0) * sc))
        .collect();
    // One continuous curve, drawn as joined pieces so its color runs through the spectrum.
    let dark = fig.color((0.07, 0.08, 0.12));
    page(&mut fig, dark);
    let piece = 120;
    let nparts = (p.len() - 1).div_ceil(piece);
    for i in 0..nparts {
        let seg = p.get(i * piece..((i + 1) * piece + 1).min(p.len())).unwrap_or(&[]);
        let h = 0.95 * i as f64 / (nparts.saturating_sub(1)).max(1) as f64;
        let col = fig.color(hsv_to_rgb(h, 0.7, 1.0));
        fig.poly(
            seg,
            col,
            &Poly {
                thick: 2,
                join: 1,
                cap: 1,
                ..Poly::DEFAULT
            },
        );
    }
    fig.render()
}

/// The four figures: (name, Fig 3.2 text).
pub fn all() -> Vec<(&'static str, String)> {
    vec![
        ("clifford-attractor", clifford()),
        ("apollonian-gasket", apollonian()),
        ("penrose-tiling", penrose()),
        ("gosper-curve", gosper()),
    ]
}
