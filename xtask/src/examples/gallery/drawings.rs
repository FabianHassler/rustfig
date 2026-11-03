//! The sixteen gallery drawings, in the order of the Python generator.

#![allow(non_snake_case)]

use super::*;
use crate::examples::pyish::{fmt_g, round_i, PyRandom};
use std::f64::consts::PI;

type Draw = fn() -> Result<Fig, String>;

/// (name, drawing function) in the gallery's order.
pub const DRAWINGS: &[(&str, Draw)] = &[
    ("pythagoras", pythagoras),
    ("unit_circle", unit_circle),
    ("riemann", riemann),
    ("normal", normal),
    ("fourier", fourier),
    ("complex_plane", complex_plane),
    ("inclined_plane", inclined_plane),
    ("pendulum", pendulum),
    ("snell", snell),
    ("double_slit", double_slit),
    ("carnot", carnot),
    ("rlc", rlc),
    ("feynman", feynman),
    ("minkowski", minkowski),
    ("loglog", loglog),
    ("damped", damped),
];

fn radians(a: f64) -> f64 {
    a.to_radians()
}

// ---- d1 ----

fn pythagoras() -> Result<Fig, String> {
    let mut f = Fig::new("Pythagorean theorem, Euclid's figure: squares on the sides of a 3-4-5 right triangle; the altitude from C splits the square on the hypotenuse into two rectangles equal in area to a^2 and b^2.");
    let c = pal(&mut f);
    title(
        &mut f,
        "Pythagorean theorem",
        "Euclid's figure: the squares on the legs equal the square on the hypotenuse",
    );
    let u = 640.0;
    let fr = Fr::new(3500.0, 6500.0, u);
    let (A, B, C) = ((4.0, 0.0), (0.0, 3.0), (0.0, 0.0));
    let (Ap, Bp) = ((A.0 + 3.0, A.1 + 4.0), (B.0 + 3.0, B.1 + 4.0));
    let D = (1.44, 1.92);
    let E = (7.4 * 0.6, 7.4 * 0.8);
    f.group(|f| {
        // square on leg b (below), with label
        f.poly(
            &fr.ps(&[(0.0, 0.0), (4.0, 0.0), (4.0, -4.0), (0.0, -4.0)]),
            St::new().pen(c.blue).fill(c.lblue).closed().depth(D_FILL).thick(2),
        );
        f.tp(fr.p(2, -2.15), vec![I("b"), sup(R("2"))], T::s(18).j(1));
    });
    f.group(|f| {
        // square on leg a (left)
        f.poly(
            &fr.ps(&[(0.0, 0.0), (0.0, 3.0), (-3.0, 3.0), (-3.0, 0.0)]),
            St::new().pen(c.red).fill(c.lred).closed().depth(D_FILL).thick(2),
        );
        f.tp(fr.p(-1.5, 1.35), vec![I("a"), sup(R("2"))], T::s(18).j(1));
    });
    f.group(|f| {
        // square on the hypotenuse, split by the altitude
        f.poly(
            &fr.ps(&[A, D, E, Ap]),
            St::new().pen(c.blue).fill(c.lblue).closed().depth(D_FILL).thick(2),
        );
        f.poly(
            &fr.ps(&[D, B, Bp, E]),
            St::new().pen(c.red).fill(c.lred).closed().depth(D_FILL).thick(2),
        );
        f.tp(fr.p(4.2, 2.75), vec![I("b"), sup(R("2"))], T::s(16).j(1));
        f.tp(fr.p(2.2, 4.3), vec![I("a"), sup(R("2"))], T::s(16).j(1));
        f.tp(fr.p(5.3, 5.8), vec![I("c"), sup(R("2"))], T::s(18));
    });
    f.group(|f| {
        // the triangle, hatched
        let k = f.color("#c4c4c4");
        f.poly(
            &fr.ps(&[C, A, B]),
            St::new()
                .pen(k)
                .fill(c.lgray)
                .pat(44)
                .closed()
                .depth(D_FILL - 20)
                .thick(1),
        );
        f.poly(&fr.ps(&[C, A, B]), St::new().pen(0).closed().depth(D_AXIS).thick(3));
        // altitude (dashed)
        f.poly(
            &fr.ps(&[C, D]),
            St::new().pen(c.green).thick(2).st(1, 6.0).depth(D_AXIS),
        );
        f.poly(
            &fr.ps(&[D, E]),
            St::new().pen(c.green).thick(2).st(1, 6.0).depth(D_AXIS),
        );
        // right-angle mark
        f.poly(
            &fr.ps(&[(0.4, 0.0), (0.4, 0.4), (0.0, 0.4)]),
            St::new().pen(0).thick(1).depth(D_MARK),
        );
    });
    f.group(|f| {
        // labels
        f.tp(fr.p(-0.4, 1.6), vec![I("a")], T::s(14).j(1).d(D_LABEL));
        f.tp(fr.p(2.0, -0.5), vec![I("b")], T::s(14).j(1).d(D_LABEL));
        f.tp(fr.p(2.25, 1.85), vec![I("c")], T::s(14).j(1).d(D_LABEL));
        f.tp(fr.p(4.25, 0.15), vec![R("A")], T::s(13));
        f.tp(fr.p(-0.45, 3.3), vec![R("B")], T::s(13));
        f.tp(fr.p(-0.5, -0.4), vec![R("C")], T::s(13));
        f.tp(fr.p(1.65, 2.9), vec![I("h")], T::s(14).j(1).c(c.green).d(D_LABEL));
    });
    // statement panel
    let x0 = 8600.0;
    f.group(|f| {
        f.text(
            x0,
            3000,
            vec![
                I("a"),
                sup(R("2")),
                R(" + "),
                I("b"),
                sup(R("2")),
                R(" = "),
                I("c"),
                sup(R("2")),
            ],
            T::s(16).big(),
        );
        f.text(x0, 3900, "Areas for $a = 3$, $b = 4$, $c = 5$:", T::s(12));
        for (k, (sym, val, col)) in [("a", 9, c.lred), ("b", 16, c.lblue), ("c", 25, c.lgray)]
            .into_iter()
            .enumerate()
        {
            let y = 4500.0 + k as f64 * 520.0;
            f.box_(x0, y - 230.0, x0 + 330.0, y + 40.0, St::new().pen(0).thick(1).fill(col));
            f.text(
                x0 + 500.0,
                y,
                vec![I(sym), sup(R("2")), R(&format!(" = {val}"))],
                T::s(14),
            );
        }
        f.text(x0, 6150, vec![R("9 + 16 = 25")], T::s(14));
        f.text(x0, 6900, "The altitude $h$ from the right angle", T::s(11));
        f.text(x0, 7200, vec![H("cuts the big square in two rectangles:")], T::s(11));
        f.text(x0, 7500, "blue $= b^2$, red $= a^2$.", T::s(11));
    });
    Ok(f)
}

// ---- d2 ----

fn unit_circle() -> Result<Fig, String> {
    let th = radians(40.0);
    let mut f = Fig::new("Unit circle: the angle theta and the segments sin(theta), cos(theta) and tan(theta); sin^2 + cos^2 = 1 by Pythagoras.");
    let c = pal(&mut f);
    title(
        &mut f,
        "Unit circle and the trigonometric functions",
        "$\\sin\\theta$, $\\cos\\theta$ and $\\tan\\theta$ as lengths of segments, here for $\\theta = 40^\\circ$",
    );
    let RR = 3000.0;
    let fr = Fr::new(4800.0, 5500.0, RR);
    let (cs, sn, tn) = (th.cos(), th.sin(), th.tan());
    let (O, P, Cx, T_, X1) = (fr.p(0, 0), fr.p(cs, sn), fr.p(cs, 0), fr.p(1, tn), fr.p(1, 0));
    f.group(|f| {
        // the circle, filled lightly
        f.circle(O, RR, St::new().pen(0).thick(2).fill(c.lgray).pat(20).depth(D_FILL));
        f.circle(O, RR, St::new().pen(0).thick(2).depth(D_CURVE));
    });
    f.group(|f| {
        // axes
        axis(f, fr.p(-1.4, 0), fr.p(1.4, 0), St::new());
        axis(f, fr.p(0, -1.25), fr.p(0, 1.3), St::new());
        f.tp(fr.p(1.4, -0.07), vec![I("x")], T::s(13));
        f.tp(fr.p(0.07, 1.3), vec![I("y")], T::s(13));
        for (v, lab) in [(1.0, "1"), (-1.0, "$-1$")] {
            seg(f, fr.p(v, -0.025), fr.p(v, 0.025), St::new().thick(1));
            f.text(fr.p(v, 0).0 + 110.0 * v, fr.p(v, 0).1 + 400.0, lab, T::s(11).j(1));
            seg(f, fr.p(-0.025, v), fr.p(0.025, v), St::new().thick(1));
            f.text(
                fr.p(0, v).0 - 130.0,
                fr.p(0, v).1 + (if v > 0.0 { -50.0 } else { 290.0 }),
                lab,
                T::s(11).j(2),
            );
        }
    });
    f.group(|f| {
        // radius and projections
        vec(f, O, (P.0 - O.0, P.1 - O.1), 0, St::v().thick(3).fwd(A_TRI));
        // radius extended to the tangent
        seg(f, O, T_, St::new().pen(c.gray).thick(1).st(1, 5.0).depth(D_CURVE));
        seg(f, O, Cx, St::new().pen(c.blue).thick(4).depth(D_MARK));
        seg(f, Cx, P, St::new().pen(c.red).thick(4).depth(D_MARK));
        seg(f, X1, T_, St::new().pen(c.green).thick(4).depth(D_MARK));
        seg(
            f,
            (X1.0, X1.1 - 700.0 * 0.0),
            (X1.0, T_.1),
            St::new().pen(c.green).thick(1).depth(D_CURVE),
        );
        f.circle(P, 60.0, St::new().pen(0).thick(1).fill(0).depth(D_MARK - 5));
        f.poly(
            &[(Cx.0 - 160.0, Cx.1), (Cx.0 - 160.0, Cx.1 - 160.0), (Cx.0, Cx.1 - 160.0)],
            St::new().thick(1).depth(D_MARK),
        );
    });
    f.group(|f| {
        // the angle
        f.arc(
            O,
            620.0,
            0.0,
            40.0,
            St::new().pen(0).thick(2).fwd(A_SMALL).depth(D_MARK),
        );
        f.text(O.0 + 700.0, O.1 - 130.0, vec![G("q")], T::s(15));
    });
    f.group(|f| {
        // labels
        f.text(
            Cx.0 / 2.0 + O.0 / 2.0 - 200.0,
            O.1 + 420.0,
            vec![R("cos "), G("q")],
            T::s(13).c(c.blue),
        );
        f.text(
            Cx.0 - 130.0,
            (Cx.1 + P.1) / 2.0 + 60.0,
            vec![R("sin "), G("q")],
            T::s(13).j(2).c(c.red),
        );
        f.text(
            X1.0 + 160.0,
            (X1.1 + T_.1) / 2.0 + 60.0,
            vec![R("tan "), G("q")],
            T::s(13).c(c.green),
        );
        f.text(
            P.0 - 300.0,
            P.1 - 200.0,
            vec![R("P = (cos "), G("q"), R(", sin "), G("q"), R(")")],
            T::s(12).j(2),
        );
        f.text(
            (O.0 + P.0) / 2.0 - 330.0,
            (O.1 + P.1) / 2.0 - 140.0,
            vec![R("1")],
            T::s(12),
        );
    });
    f.group(|f| {
        let x0 = 9600;
        f.text(
            x0,
            3300,
            vec![
                R("sin"),
                sup(R("2")),
                G("q"),
                R(" + cos"),
                sup(R("2")),
                G("q"),
                R(" = 1"),
            ],
            T::s(18),
        );
        f.text(
            x0,
            4100,
            vec![R("tan "), G("q"), R(" = "), R("sin "), G("q"), R(" / cos "), G("q")],
            T::s(16),
        );
        f.text(x0, 5100, r"For $\theta = 40^\circ$:", T::s(12));
        f.text(x0, 5500, format!(r"$\cos\theta = {cs:.3}$"), T::s(12).c(c.blue));
        f.text(x0, 5850, format!(r"$\sin\theta = {sn:.3}$"), T::s(12).c(c.red));
        f.text(x0, 6200, format!(r"$\tan\theta = {tn:.3}$"), T::s(12).c(c.green));
    });
    Ok(f)
}

fn riemann() -> Result<Fig, String> {
    let func = |x: f64| 1.2 + 2.2 * (PI * x / 4.0).sin();
    let mut f = Fig::new("Riemann sum: left rectangles (n = 8) under f(x) = 1.2 + 2.2 sin(pi x / 4) on [0, 4]; the curve is an X-spline, the sum is compared with the integral.");
    let c = pal(&mut f);
    title(
        &mut f,
        "Riemann sum",
        "Left-endpoint sum with 8 rectangles approximating a definite integral",
    );
    let fr = Fr::new2(1900.0, 7700.0, 1650.0, 1250.0);
    let (n, a, b) = (8usize, 0.0f64, 4.0f64);
    let dx = (b - a) / n as f64;
    let S = (0..n).fold(0.0, |s, k| s + func(a + k as f64 * dx)) * dx;
    let I_ex = 1.2 * 4.0 + 2.2 * (4.0 / PI) * 2.0;
    f.group(|f| {
        for k in 0..n {
            let x0 = a + k as f64 * dx;
            let (p0, p1) = (fr.p(x0, 0), fr.p(x0 + dx, func(x0)));
            f.box_(
                p0.0,
                p0.1,
                p1.0,
                p1.1,
                St::new().pen(c.blue).thick(1).fill(c.lblue).depth(D_FILL),
            );
        }
    });
    f.group(|f| {
        f.spline(
            &fr.ps(&sample(func, a, b, 24)),
            St::new().pen(c.red).thick(3).depth(D_CURVE),
        );
        f.tp(fr.p(4.12, 1.45), "$f(x)$", T::s(12).c(c.red));
    });
    f.group(|f| {
        // x axis with ticks
        axis(f, fr.p(-0.25, 0), fr.p(4.5, 0), St::new());
        let labels: Vec<Txt> = (0..5).map(|k| vec![H(&k.to_string())].into()).collect();
        xaxis_ticks(f, &fr, &[0.0, 1.0, 2.0, 3.0, 4.0], 0.0, labels);
        f.tp(fr.p(4.5, -0.28), vec![I("x")], T::s(13));
    });
    f.group(|f| {
        axis(f, fr.p(0, -0.1), fr.p(0, 3.9), St::new());
        yaxis_ticks(
            f,
            &fr,
            0.0,
            &[1.0, 2.0, 3.0],
            vec![vec![H("1")].into(), vec![H("2")].into(), vec![H("3")].into()],
        );
        f.tp(fr.p(-0.12, 3.95), vec![I("y")], T::s(13));
    });
    f.group(|f| {
        // delta x marker
        let (p, q) = (fr.p(0.5, -0.8), fr.p(1.0, -0.8));
        f.poly(&[p, q], St::new().thick(1).fwd(A_SMALL).back(A_SMALL).depth(D_AXIS));
        f.text((p.0 + q.0) / 2.0, p.1 + 280.0, vec![G("D"), I("x")], T::s(12).j(1));
        seg(f, fr.p(0.5, -0.45), fr.p(0.5, -0.9), St::new().thick(1).st(2, 3.0));
        seg(f, fr.p(1.0, -0.45), fr.p(1.0, -0.9), St::new().thick(1).st(2, 3.0));
    });
    f.group(|f| {
        let x0 = 9400.0;
        f.text(x0, 3300, r"$S_8 = \sum_i f(x_i)\,\Delta x$", T::s(12));
        f.text(x0, 4000, format!(r"$= {S:.3}$"), T::s(12));
        f.text(x0, 5000, format!(r"$\int_0^4 f(x)\,dx = {I_ex:.3}$"), T::s(12));
        f.text(x0, 6000, vec![H("Doubling n halves the error")], T::s(11));
        f.text(x0, 6300, vec![H("of the left-endpoint rule.")], T::s(11));
        f.box_(
            x0,
            6850.0,
            x0 + 330.0,
            7120.0,
            St::new().pen(c.blue).thick(1).fill(c.lblue),
        );
        f.text(x0 + 500.0, 7090, r"rectangle $f(x_i)$", T::s(11));
    });
    Ok(f)
}

fn normal() -> Result<Fig, String> {
    let mut f = Fig::new("Normal distribution with the 68.27 / 95.45 / 99.73 % regions of +-1, 2, 3 sigma shaded as polygons under an X-spline density curve.");
    let c = pal(&mut f);
    title(
        &mut f,
        "Normal distribution",
        "Probability within one, two and three standard deviations of the mean",
    );
    let pdf = |x: f64| (-x * x / 2.0).exp() / (2.0 * PI).sqrt();
    let fr = Fr::new2(6200.0, 5900.0, 1380.0, 9200.0);
    let shades = [f.color("#7fa8d6"), f.color("#a9c6e6"), f.color("#d2e1f2")];
    f.group(|f| {
        for (k, col) in [(3i32, shades[2]), (2, shades[1]), (1, shades[0])] {
            for sgn in [-1, 1] {
                let (lo, hi) = if sgn > 0 { (k - 1, k) } else { (-k, -(k - 1)) };
                let (lo, hi) = (lo as f64, hi as f64);
                let pts = sample(pdf, lo, hi, 24);
                let mut poly = vec![(lo, 0.0)];
                poly.extend(pts);
                poly.push((hi, 0.0));
                f.poly(
                    &fr.ps(&poly),
                    St::new()
                        .pen(col)
                        .thick(0)
                        .fill(col)
                        .closed()
                        .depth(D_FILL + 10 * (3 - k)),
                );
            }
        }
    });
    f.group(|f| {
        f.spline(
            &fr.ps(&sample(pdf, -4.0, 4.0, 40)),
            St::new().pen(c.blue).thick(3).depth(D_CURVE),
        );
    });
    f.group(|f| {
        // axes
        axis(f, fr.p(-4.2, 0), fr.p(4.5, 0), St::new());
        let labs = [
            r"$-3\sigma$",
            r"$-2\sigma$",
            r"$-1\sigma$",
            r"$\mu$",
            r"$1\sigma$",
            r"$2\sigma$",
            r"$3\sigma$",
        ];
        for (idx, lab) in labs.iter().enumerate() {
            let v = idx as f64 - 3.0;
            let (x, y) = fr.p(v, 0);
            seg(f, (x, y), (x, y + 90.0), St::new().thick(1));
            f.text(x, y + 330.0, *lab, T::s(12).j(1));
        }
        f.tp(fr.p(4.5, -0.012), vec![I("x")], T::s(13));
        axis(f, fr.p(-4.2, 0), fr.p(-4.2, 0.46), St::new());
        for v in [0.1, 0.2, 0.3, 0.4] {
            let (x, y) = fr.p(-4.2, v);
            seg(f, (x, y), (x - 90.0, y), St::new().thick(1));
            f.text(x - 150.0, y + 60.0, vec![H(&format!("{v:.1}"))], T::s(11).j(2));
        }
        f.tp(fr.p(-4.15, 0.47), vec![I("f"), R("("), I("x"), R(")")], T::s(13));
    });
    f.group(|f| {
        // sigma ticks to the density, dotted
        for v in [-3.0, -2.0, -1.0, 1.0, 2.0, 3.0] {
            seg(
                f,
                fr.p(v, 0),
                fr.p(v, pdf(v)),
                St::new().thick(1).st(2, 3.0).depth(D_AXIS),
            );
        }
        seg(
            f,
            fr.p(0, 0),
            fr.p(0, pdf(0.0)),
            St::new().thick(1).st(1, 5.0).depth(D_AXIS).pen(c.gray),
        );
    });
    let base = fr.p(0, 0).1;
    for (k, pct, row) in [
        (1.0, "68.27 %", 1250.0),
        (2.0, "95.45 %", 1700.0),
        (3.0, "99.73 %", 2150.0),
    ] {
        f.group(|f| {
            let y = base + row;
            let (a, b) = (fr.p(-k, 0).0, fr.p(k, 0).0);
            f.poly(
                &[(a, y), (b, y)],
                St::new().thick(2).fwd(A_SMALL).back(A_SMALL).pen(c.blue),
            );
            seg(f, (a, base + 120.0), (a, y), St::new().thick(1).st(2, 3.0).pen(c.gray));
            seg(f, (b, base + 120.0), (b, y), St::new().thick(1).st(2, 3.0).pen(c.gray));
            f.text((a + b) / 2.0, y - 80.0, vec![H(pct)], T::s(12).j(1));
        });
    }
    f.group(|f| {
        let x0 = 8300;
        f.text(
            x0,
            2500,
            r"$f(x) = \dfrac{1}{\sigma\sqrt{2\pi}}\,\exp\left[-\dfrac{(x-\mu)^2}{2\sigma^2}\right]$",
            T::s(12),
        );
        f.text(x0, 3300, r"mean $\mu$, standard deviation $\sigma$", T::s(11));
    });
    Ok(f)
}

// ---- d3 ----

fn fourier() -> Result<Fig, String> {
    let mut f = Fig::new("Fourier series of a square wave: partial sums S_N(x) = 4/pi * sum_{k odd <= N} sin(kx)/k for N = 1, 3, 5, 15 (X-splines) against the square wave; Gibbs overshoot marked.");
    let c = pal(&mut f);
    title(
        &mut f,
        "Fourier series of a square wave",
        "Partial sums with 1, 3, 5 and 15 terms; the overshoot at the jump does not vanish (Gibbs)",
    );
    let fr = Fr::new2(5000.0, 5500.0, 1250.0, 2300.0);
    let S = |n: i32, x: f64| -> f64 {
        let mut sum = 0.0;
        let mut k = 1;
        while k <= n {
            sum += (k as f64 * x).sin() / k as f64;
            k += 2;
        }
        4.0 / PI * sum
    };
    let col_of = |n: i32| match n {
        1 => c.blue,
        3 => c.green,
        5 => c.orange,
        _ => c.red,
    };
    f.group(|f| {
        f.poly(
            &fr.ps(&[(-PI, -1.0), (0.0, -1.0), (0.0, 1.0), (PI, 1.0)]),
            St::new().pen(0).thick(2).depth(D_CURVE + 5).join(0),
        );
    });
    for n_terms in [1, 3, 5, 15] {
        f.group(|f| {
            let n = if n_terms == 15 { 360 } else { 120 };
            let pts = sample(|x| S(n_terms, x), -PI, PI, n);
            f.spline(
                &fr.ps(&pts),
                St::new()
                    .pen(col_of(n_terms))
                    .thick(if n_terms != 15 { 2 } else { 1 })
                    .depth(D_CURVE + (10 - n_terms).div_euclid(2)),
            );
        });
    }
    f.group(|f| {
        axis(f, fr.p(-3.5, 0), fr.p(3.75, 0), St::new());
        let ticks: [(f64, Vec<Run>); 4] = [
            (-PI, vec![minus(), G("p")]),
            (-PI / 2.0, vec![minus(), G("p"), R("/2")]),
            (PI / 2.0, vec![G("p"), R("/2")]),
            (PI, vec![G("p")]),
        ];
        for (v, lab) in ticks {
            let (x, y) = fr.p(v, 0);
            seg(f, (x, y - 70.0), (x, y + 70.0), St::new().thick(1));
            let j = if v < -3.0 {
                2
            } else if v > 3.0 {
                0
            } else {
                1
            };
            f.text(
                x + (if j == 2 {
                    -70.0
                } else if j == 0 {
                    70.0
                } else {
                    0.0
                }),
                y + 380.0,
                lab,
                T::s(12).j(j),
            );
        }
        f.tp(fr.p(3.75, -0.1), vec![I("x")], T::s(13));
    });
    f.group(|f| {
        axis(f, fr.p(-3.5, -1.55), fr.p(-3.5, 1.6), St::new());
        yaxis_ticks(
            f,
            &fr,
            -3.5,
            &[-1.0, 0.0, 1.0],
            vec!["$-1$".into(), "0".into(), "1".into()],
        );
        f.tp(fr.p(-3.4, 1.62), vec![I("f"), R("("), I("x"), R(")")], T::s(13));
    });
    f.group(|f| {
        // Gibbs annotation
        let pk = (PI / 16.0, S(15, PI / 16.0));
        let a = fr.p(1.1, 1.43);
        let b = fr.p(pk.0 + 0.05, pk.1 + 0.03);
        f.poly(
            &[(a.0 - 60.0, a.1 + 40.0), b],
            St::new().pen(0).thick(1).fwd(A_SMALL).depth(D_MARK),
        );
        f.text(
            a.0,
            a.1,
            format!(r"Gibbs overshoot, $S_{{15}} = {:.2}$", pk.1),
            T::s(11),
        );
    });
    f.group(|f| {
        // legend
        let (x0, y0) = (9900.0, 2900.0);
        let items: [(i32, i32, &str); 5] = [
            (0, 0, "square wave"),
            (1, c.blue, r"$N = 1$"),
            (3, c.green, r"$N = 3$"),
            (5, c.orange, r"$N = 5$"),
            (15, c.red, r"$N = 15$"),
        ];
        for (k, (n, col, lab)) in items.into_iter().enumerate() {
            let y = y0 + k as f64 * 430.0;
            f.poly(
                &[(x0, y), (x0 + 700.0, y)],
                St::new().pen(col).thick(if n != 15 { 2 } else { 1 }),
            );
            f.text(x0 + 900.0, y + 60.0, lab, T::s(12));
        }
        f.text(x0, y0 - 450.0, vec![B("Legend")], T::s(11));
        f.text(
            x0,
            6700,
            r"$S_N(x) = \frac{4}{\pi}\sum_{k\,\mathrm{odd}}^{N}\frac{\sin kx}{k}$",
            T::s(12),
        );
        f.text(x0, 7200, r"$k$ odd, $k \le N$", T::s(11));
    });
    Ok(f)
}

fn complex_plane() -> Result<Fig, String> {
    let mut f = Fig::new("Euler's formula on the complex plane: e^{i theta} = cos theta + i sin theta, a point on the unit circle. The bottom formula is LaTeX 'special' text (flag bit 1) meant for the LaTeX export (pstex/pict2e); RustFig typesets it with View > Typeset LaTeX text.");
    let c = pal(&mut f);
    title(
        &mut f,
        "Euler's formula in the complex plane",
        "$e^{i\\theta}$ runs around the unit circle; its projections are $\\cos\\theta$ and $\\sin\\theta$",
    );
    let th = radians(50.0);
    let RR = 2450.0;
    let fr = Fr::new(5000.0, 5500.0, RR);
    let (cs, sn) = (th.cos(), th.sin());
    let (O, P) = (fr.p(0, 0), fr.p(cs, sn));
    f.group(|f| {
        f.circle(
            O,
            RR,
            St::new().pen(c.gray).thick(1).fill(c.lgray).pat(20).depth(D_FILL),
        );
        f.circle(O, RR, St::new().pen(c.gray).thick(1).depth(D_CURVE));
    });
    f.group(|f| {
        axis(f, fr.p(-1.4, 0), fr.p(1.4, 0), St::new());
        axis(f, fr.p(0, -1.35), fr.p(0, 1.4), St::new());
        f.tp(fr.p(1.4, -0.09), vec![R("Re")], T::s(13));
        f.tp(fr.p(0.07, 1.4), vec![R("Im")], T::s(13));
        let marks: [(f64, Vec<Run>, bool); 4] = [
            (1.0, vec![R("1")], true),
            (-1.0, vec![minus(), R("1")], true),
            (1.0, vec![I("i")], false),
            (-1.0, vec![minus(), I("i")], false),
        ];
        for (v, lab, on_x) in marks {
            if on_x {
                let (x, y) = fr.p(v, 0);
                f.circle((x, y), 55.0, St::new().pen(0).thick(1).fill(0).depth(D_MARK));
                f.text(x + (if v > 0.0 { 60.0 } else { 0.0 }), y + 400.0, lab, T::s(13).j(1));
            } else {
                let (x, y) = fr.p(0, v);
                f.circle((x, y), 55.0, St::new().pen(0).thick(1).fill(0).depth(D_MARK));
                f.text(x - 150.0, y + (if v > 0.0 { -60.0 } else { 320.0 }), lab, T::s(13).j(2));
            }
        }
    });
    f.group(|f| {
        vec(f, O, (P.0 - O.0, P.1 - O.1), c.red, St::v().thick(3));
        seg(
            f,
            P,
            fr.p(cs, 0),
            St::new().pen(c.red).thick(1).st(1, 5.0).depth(D_MARK),
        );
        seg(
            f,
            P,
            fr.p(0, sn),
            St::new().pen(c.blue).thick(1).st(1, 5.0).depth(D_MARK),
        );
        seg(f, O, fr.p(cs, 0), St::new().pen(c.red).thick(3).depth(D_MARK + 3));
        seg(f, O, fr.p(0, sn), St::new().pen(c.blue).thick(3).depth(D_MARK + 3));
        f.circle(P, 70.0, St::new().pen(0).thick(1).fill(c.red).depth(D_MARK - 5));
        f.arc(
            O,
            620.0,
            0.0,
            50.0,
            St::new().pen(0).thick(2).fwd(A_SMALL).depth(D_MARK),
        );
        f.text(O.0 + 780.0, O.1 - 190.0, vec![G("q")], T::s(15));
    });
    f.group(|f| {
        f.text(
            fr.p(cs, 0).0 / 2.0 + O.0 / 2.0,
            O.1 + 400.0,
            vec![R("cos "), G("q")],
            T::s(13).j(1).c(c.red),
        );
        f.text(
            O.0 - 150.0,
            fr.p(0, sn).1 + 80.0,
            vec![I("i"), R(" sin "), G("q")],
            T::s(13).j(2).c(c.blue),
        );
        f.text(
            P.0 + 170.0,
            P.1 - 120.0,
            vec![I("e"), sup(I("i")), sup(G("q"))],
            T::s(16),
        );
        f.text(O.0 - 150.0, O.1 - 1300.0, "$|z| = 1$", T::s(12).j(2));
    });
    f.group(|f| {
        f.text(9400, 3500, vec![H("Euler's identity:")], T::s(11));
        f.text(9400, 4100, "$e^{i\\pi} + 1 = 0$", T::s(16));
    });
    f.text(
        5000,
        9000,
        "$e^{i\\theta} = \\cos\\theta + i\\sin\\theta$",
        T::s(14).j(1).big(),
    );
    Ok(f)
}

fn inclined_plane() -> Result<Fig, String> {
    let al = radians(30.0);
    let (ca, sa) = (al.cos(), al.sin());
    let mut f = Fig::new("Block on an inclined plane, free-body diagram: weight mg, normal force N, static friction f and the components of mg; incline angle alpha; hatched wedge and ground.");
    let c = pal(&mut f);
    title(
        &mut f,
        "Block on an inclined plane",
        "Free-body diagram of a block at rest on a $30^\\circ$ incline",
    );
    let fr = Fr::new(1400.0, 7700.0, 1.0);
    let (A, B, C) = (fr.p(0, 0), fr.p(6200, 0), fr.p(6200.0, 6200.0 * al.tan()));
    f.group(|f| {
        // ground strip and a hatched layer under the slope; the wedge interior stays white
        f.box_(
            A.0 - 700.0,
            A.1,
            B.0 + 700.0,
            A.1 + 380.0,
            St::new().pen(c.hp).thick(1).fill(c.lgray).pat(45).depth(D_FILL),
        );
        let h_ = 6200.0 * al.tan();
        let strip = [
            fr.p(0, 0),
            fr.p(6200.0, h_),
            fr.p(6200.0, h_ - 520.0),
            fr.p(520.0 / al.tan(), 0),
        ];
        f.poly(
            &strip,
            St::new()
                .pen(c.hp)
                .thick(1)
                .fill(c.lgray)
                .pat(44)
                .closed()
                .depth(D_FILL),
        );
        seg(f, A, C, St::new().thick(3).depth(D_AXIS));
        seg(
            f,
            (A.0 - 700.0, A.1),
            (B.0 + 700.0, A.1),
            St::new().thick(3).depth(D_AXIS),
        );
        seg(f, B, C, St::new().thick(3).depth(D_AXIS));
    });
    // block
    let (t, n) = ((ca, sa), (-sa, ca)); // up the slope, surface normal (math coords)
    let (s, d) = (1200.0, 3300.0);
    let mp = |du: f64, dn: f64| fr.p(d * t.0 + du * t.0 + dn * n.0, d * t.1 + du * t.1 + dn * n.1);
    let ctr_m = (d * t.0 + (s / 2.0) * n.0, d * t.1 + (s / 2.0) * n.1);
    let Cb = fr.p(ctr_m.0, ctr_m.1);
    f.group(|f| {
        f.poly(
            &[mp(-s / 2.0, 0.0), mp(s / 2.0, 0.0), mp(s / 2.0, s), mp(-s / 2.0, s)],
            St::new().pen(0).thick(2).fill(c.lorange).closed().depth(D_MARK + 10),
        );
        f.circle(Cb, 55.0, St::new().pen(0).thick(1).fill(0).depth(D_MARK - 10));
    });
    let m_ = 1800.0;
    // force vector in screen coords for math direction (vx, vy) of length L
    let F = |vx: f64, vy: f64, L: f64| (L * vx, -L * vy);
    let forces: [(Pt, f64, i32, Vec<Run>, Pt); 3] = [
        ((0.0, -1.0), m_, c.red, vec![I("m"), I("g")], (150.0, -300.0)),
        (n, m_ * ca, c.blue, vec![I("N")], (-380.0, -20.0)),
        (t, m_ * sa, c.green, vec![I("f")], (140.0, -100.0)),
    ];
    f.group(|f| {
        // components of the weight (dashed)
        let v_par = F(-t.0, -t.1, m_ * sa);
        let v_perp = F(-n.0, -n.1, m_ * ca);
        let tip = (Cb.0, Cb.1 + m_);
        vec(
            f,
            Cb,
            v_par,
            c.red,
            St::v().thick(2).fwd(A_SMALL).st(1, 5.0).depth(D_AXIS + 5),
        );
        vec(
            f,
            Cb,
            v_perp,
            c.red,
            St::v().thick(2).fwd(A_SMALL).st(1, 5.0).depth(D_AXIS + 5),
        );
        seg(
            f,
            (Cb.0 + v_par.0, Cb.1 + v_par.1),
            tip,
            St::new().pen(c.gray).thick(1).st(2, 3.0).depth(D_AXIS + 5),
        );
        seg(
            f,
            (Cb.0 + v_perp.0, Cb.1 + v_perp.1),
            tip,
            St::new().pen(c.gray).thick(1).st(2, 3.0).depth(D_AXIS + 5),
        );
        f.text(
            Cb.0 + v_par.0 - 120.0,
            Cb.1 + v_par.1 - 150.0,
            vec![I("m"), I("g"), R(" sin "), G("a")],
            T::s(12).j(2).c(c.red),
        );
        f.text(
            Cb.0 + v_perp.0 + 250.0,
            Cb.1 + v_perp.1 + 380.0,
            vec![I("m"), I("g"), R(" cos "), G("a")],
            T::s(12).c(c.red),
        );
    });
    f.group(|f| {
        for (v, L, col, lab, off) in forces {
            let vv = F(v.0, v.1, L);
            vec(f, Cb, vv, col, St::v().thick(4).fwd(A_TRI).depth(D_AXIS));
            f.text(Cb.0 + vv.0 + off.0, Cb.1 + vv.1 + off.1, lab, T::s(15).c(col));
        }
    });
    f.group(|f| {
        // angle alpha at the foot
        f.arc(
            A,
            1500.0,
            0.0,
            30.0,
            St::new().pen(0).thick(2).fwd(A_SMALL).depth(D_MARK),
        );
        f.text(A.0 + 1750.0, A.1 - 330.0, vec![G("a")], T::s(15));
        // alpha between mg and -N
        seg(
            f,
            Cb,
            (Cb.0, Cb.1 - 1250.0),
            St::new().pen(c.gray).thick(1).st(1, 5.0).depth(D_CURVE),
        ); // vertical reference
        f.arc(
            Cb,
            800.0,
            90.0,
            120.0,
            St::new().pen(c.blue).thick(2).fwd(A_SMALL).depth(D_MARK),
        ); // angle between N and the vertical = alpha
        f.text(
            Cb.0 + 1050.0 * radians(105.0).cos() - 60.0,
            Cb.1 - 1050.0 * radians(105.0).sin() - 40.0,
            vec![G("a")],
            T::s(12).c(c.blue),
        );
    });
    f.group(|f| {
        let x0 = 9000;
        f.text(x0, 3300, "At rest:", T::s(12));
        f.text(x0, 3950, vec![I("N"), R(" = "), I("mg"), R(" cos "), G("a")], T::s(16));
        f.text(x0, 4600, vec![I("f"), R(" = "), I("mg"), R(" sin "), G("a")], T::s(16));
        f.text(x0, 5400, r"$f \le \mu_s N$", T::s(12));
        f.text(x0, 6050, r"$\Rightarrow \tan\alpha \le \mu_s$", T::s(12));
        f.text(x0, 6900, r"$\mu_s$: coefficient of static friction", T::s(11));
    });
    Ok(f)
}

fn pendulum() -> Result<Fig, String> {
    let th = radians(28.0);
    let mut f = Fig::new("Simple pendulum at a turning point: weight mg, tension T and the components of mg along and across the string; angle theta; hatched ceiling.");
    let c = pal(&mut f);
    title(
        &mut f,
        "Simple pendulum",
        "Forces on the bob at the turning point of its swing",
    );
    let P0 = (4300.0, 2300.0);
    let L = 5200.0;
    let bob = (P0.0 + L * th.sin(), P0.1 + L * th.cos());
    let bob2 = (P0.0 - L * th.sin(), P0.1 + L * th.cos());
    f.group(|f| {
        // ceiling
        f.box_(
            P0.0 - 1700.0,
            P0.1 - 450.0,
            P0.0 + 1700.0,
            P0.1,
            St::new().pen(c.hp).thick(1).fill(c.lgray).pat(45).depth(D_FILL),
        );
        f.box_(
            P0.0 - 1700.0,
            P0.1 - 450.0,
            P0.0 + 1700.0,
            P0.1,
            St::new().pen(0).thick(1).depth(D_AXIS),
        );
        seg(f, (P0.0 - 1700.0, P0.1), (P0.0 + 1700.0, P0.1), St::new().thick(3));
    });
    f.group(|f| {
        // reference line and swing arc
        seg(
            f,
            P0,
            (P0.0, P0.1 + L + 500.0),
            St::new().pen(c.gray).thick(1).st(1, 6.0).depth(D_CURVE),
        );
        f.arc(
            P0,
            L,
            -90.0 - 28.0 - 12.0,
            -90.0 + 28.0 + 12.0,
            St::new().pen(c.gray).thick(1).st(2, 3.0).depth(D_CURVE),
        );
        f.circle(bob2, 330.0, St::new().pen(c.gray).thick(1).st(1, 4.0).depth(D_CURVE));
        seg(f, P0, bob2, St::new().pen(c.gray).thick(1).st(1, 4.0).depth(D_CURVE));
    });
    f.group(|f| {
        seg(f, P0, bob, St::new().thick(3).depth(D_AXIS + 10));
        f.circle(bob, 330.0, St::new().pen(0).thick(3).fill(c.lblue).depth(D_AXIS + 5));
        f.circle(P0, 70.0, St::new().pen(0).thick(1).fill(0).depth(D_MARK));
        f.circle(bob, 55.0, St::new().pen(0).thick(1).fill(0).depth(D_MARK - 10));
    });
    let r = (th.sin(), th.cos()); // screen unit vector pivot -> bob (y down)
    let tg = (-th.cos(), th.sin()); // screen vector tangent, toward lower theta (down-left): x<0, y>0 (down)
    let mg = 2100.0;
    let tip = (bob.0, bob.1 + mg);
    f.group(|f| {
        vec(f, bob, (0.0, mg), c.red, St::v().thick(4).depth(D_AXIS));
        // tension
        vec(
            f,
            bob,
            (-r.0 * mg * th.cos(), -r.1 * mg * th.cos()),
            c.blue,
            St::v().thick(4).depth(D_AXIS),
        );
        let a = (bob.0 + r.0 * mg * th.cos(), bob.1 + r.1 * mg * th.cos());
        let b = (bob.0 + tg.0 * mg * th.sin(), bob.1 + tg.1 * mg * th.sin());
        vec(
            f,
            bob,
            (a.0 - bob.0, a.1 - bob.1),
            c.red,
            St::v().thick(2).fwd(A_SMALL).st(1, 5.0).depth(D_AXIS + 5),
        );
        vec(
            f,
            bob,
            (b.0 - bob.0, b.1 - bob.1),
            c.red,
            St::v().thick(2).fwd(A_SMALL).st(1, 5.0).depth(D_AXIS + 5),
        );
        seg(f, a, tip, St::new().pen(c.gray).thick(1).st(2, 3.0).depth(D_AXIS + 5));
        seg(f, b, tip, St::new().pen(c.gray).thick(1).st(2, 3.0).depth(D_AXIS + 5));
        f.text(tip.0 + 120.0, tip.1 + 100.0, vec![I("m"), I("g")], T::s(15).c(c.red));
        f.text(
            bob.0 - r.0 * mg * th.cos() - 450.0,
            bob.1 - r.1 * mg * th.cos() - 50.0,
            vec![I("T")],
            T::s(15).c(c.blue),
        );
        f.text(
            a.0 + 150.0,
            a.1 + 80.0,
            vec![I("m"), I("g"), R(" cos "), G("q")],
            T::s(12).c(c.red),
        );
        f.text(
            b.0 - 100.0,
            b.1 + 380.0,
            vec![I("m"), I("g"), R(" sin "), G("q")],
            T::s(12).j(2).c(c.red),
        );
    });
    f.group(|f| {
        f.arc(
            P0,
            1500.0,
            -90.0,
            -90.0 + 28.0,
            St::new().pen(0).thick(2).fwd(A_SMALL).depth(D_MARK),
        );
        f.text(P0.0 + 480.0, P0.1 + 1750.0, vec![G("q")], T::s(15));
        f.arc(
            bob,
            800.0,
            -90.0,
            -90.0 + 28.0,
            St::new().pen(c.red).thick(2).fwd(A_SMALL).depth(D_MARK),
        );
        f.text(bob.0 + 330.0, bob.1 + 1000.0, vec![G("q")], T::s(13).c(c.red));
        f.text(
            (P0.0 + bob.0) / 2.0 + 150.0,
            (P0.1 + bob.1) / 2.0 - 150.0,
            vec![I("L")],
            T::s(15),
        );
    });
    f.group(|f| {
        let x0 = 9300;
        f.text(x0, 3600, r"At the turning point ($v = 0$):", T::s(12));
        f.text(x0, 4300, vec![I("T"), R(" = "), I("mg"), R(" cos "), G("q")], T::s(16));
        f.text(x0, 5100, r"$mL\,\dfrac{d^2\theta}{dt^2} = -mg\sin\theta$", T::s(12));
        f.text(x0, 6000, r"Small angles: $\omega = \sqrt{g/L}$", T::s(12));
    });
    Ok(f)
}

// ---- d4 ----

fn snell() -> Result<Fig, String> {
    let (n1, n2, t1d) = (1.0f64, 1.5f64, 50.0f64);
    let t1 = radians(t1d);
    let t2 = (n1 * t1.sin() / n2).asin();
    let t2d = t2.to_degrees();
    let mut f = Fig::new(&format!("Snell's law: a ray from air (n1 = 1.0) into glass (n2 = 1.5) bends toward the normal; theta1 = 50 deg gives theta2 = {t2d:.1} deg from n1 sin(theta1) = n2 sin(theta2); the partly reflected ray leaves at theta1."));
    let c = pal(&mut f);
    title(
        &mut f,
        "Refraction: Snell's law",
        "A ray entering the denser medium bends toward the normal",
    );
    let O = (4800.0, 5200.0);
    let Lr = 3200.0;
    let (X0, X1, Y0, Y2) = (1300.0, 8300.0, 2100.0, 8500.0);
    f.group(|f| {
        // media
        f.box_(
            X0,
            Y0,
            X1,
            O.1,
            St::new().pen(c.lgray).thick(0).fill(c.lgray).depth(D_FILL + 20),
        );
        f.box_(
            X0,
            O.1,
            X1,
            Y2,
            St::new().pen(c.lblue).thick(0).fill(c.lblue).depth(D_FILL),
        );
        f.box_(X0, Y0, X1, Y2, St::new().pen(0).thick(2).depth(D_AXIS + 20));
        seg(f, (X0, O.1), (X1, O.1), St::new().thick(3).depth(D_AXIS));
        f.text(X1 + 200.0, Y0 + 300.0, format!(r"$n_1 = {n1:.2}$ (air)"), T::s(12));
        f.text(X1 + 200.0, Y2 - 100.0, format!(r"$n_2 = {n2:.2}$ (glass)"), T::s(12));
    });
    let P = |a: f64, L: f64| (O.0 + L * radians(a).cos(), O.1 - L * radians(a).sin());
    f.group(|f| {
        // normal
        seg(
            f,
            (O.0, O.1 - 3000.0),
            (O.0, O.1 + 3000.0),
            St::new().pen(0).thick(1).st(1, 6.0).depth(D_AXIS),
        );
        f.text(O.0 + 100.0, O.1 - 2800.0, vec![H("normal")], T::s(11));
    });
    f.group(|f| {
        // rays
        let a_inc = 90.0 + t1d;
        let S = P(a_inc, Lr);
        seg(f, S, O, St::new().pen(c.red).thick(3).depth(D_CURVE));
        mid_arrow(
            f,
            S,
            O,
            0.55,
            0.05,
            St::new().pen(c.red).thick(3).fwd(A_TRI).depth(D_CURVE),
        );
        let R_ = P(90.0 - t1d, Lr);
        seg(f, O, R_, St::new().pen(c.orange).thick(2).depth(D_CURVE));
        mid_arrow(
            f,
            O,
            R_,
            0.55,
            0.05,
            St::new().pen(c.orange).thick(2).fwd(A_TRI).depth(D_CURVE),
        );
        let T_ = P(-90.0 + t2d, Lr);
        seg(f, O, T_, St::new().pen(c.red).thick(3).depth(D_CURVE));
        mid_arrow(
            f,
            O,
            T_,
            0.55,
            0.05,
            St::new().pen(c.red).thick(3).fwd(A_TRI).depth(D_CURVE),
        );
        f.text(S.0 - 100.0, S.1 - 100.0, vec![H("incident")], T::s(12).j(2).c(c.red));
        f.text(R_.0 + 100.0, R_.1 - 100.0, vec![H("reflected")], T::s(12).c(c.orange));
        f.text(T_.0 + 150.0, T_.1 + 100.0, vec![H("refracted")], T::s(12).c(c.red));
    });
    f.group(|f| {
        // angles
        f.arc(
            O,
            1250.0,
            90.0,
            90.0 + t1d,
            St::new().pen(0).thick(2).fwd(A_SMALL).depth(D_MARK),
        );
        f.text(
            P(90.0 + t1d / 2.0, 1650.0).0,
            P(90.0 + t1d / 2.0, 1650.0).1 + 80.0,
            vec![G("q"), sub(R("1"))],
            T::s(14).j(1),
        );
        f.arc(
            O,
            1250.0,
            90.0,
            90.0 - t1d,
            St::new().pen(c.orange).thick(2).fwd(A_SMALL).depth(D_MARK),
        );
        f.text(
            P(90.0 - t1d / 2.0, 1650.0).0,
            P(90.0 - t1d / 2.0, 1650.0).1 + 80.0,
            vec![G("q"), sub(R("1"))],
            T::s(14).j(1).c(c.orange),
        );
        f.arc(
            O,
            1250.0,
            -90.0,
            -90.0 + t2d,
            St::new().pen(0).thick(2).fwd(A_SMALL).depth(D_MARK),
        );
        f.text(
            P(-90.0 + t2d / 2.0, 1700.0).0 + 30.0,
            P(-90.0 + t2d / 2.0, 1700.0).1 + 120.0,
            vec![G("q"), sub(R("2"))],
            T::s(14).j(1),
        );
    });
    f.group(|f| {
        let x0 = 9100;
        f.text(
            x0,
            3300,
            vec![
                I("n"),
                sub(R("1")),
                R(" sin "),
                G("q"),
                sub(R("1")),
                R(" = "),
                I("n"),
                sub(R("2")),
                R(" sin "),
                G("q"),
                sub(R("2")),
            ],
            T::s(16),
        );
        f.text(
            x0,
            4100,
            format!(
                r"$\sin\theta_2 = \sin 50^\circ / {:.2} = {:.3}$",
                n2 / n1,
                t1.sin() * n1 / n2
            ),
            T::s(12),
        );
        f.text(
            x0,
            4700,
            format!(r"$\theta_2 = {t2d:.1}^\circ < \theta_1 = 50^\circ$"),
            T::s(12),
        );
        f.text(x0, 5500, r"$n_2 > n_1$: the ray bends", T::s(11));
        f.text(x0, 5800, vec![H("toward the normal.")], T::s(11));
    });
    Ok(f)
}

fn double_slit() -> Result<Fig, String> {
    let (lam, d, a, D) = (300.0f64, 1500.0f64, 750.0f64, 6000.0f64);
    let (Ym, bx, HH) = (5400.0f64, 3000.0f64, 3300.0f64);
    let xs = bx + D;
    let mut f = Fig::new("Young's double-slit experiment: plane wave, two slits (d = 5 lambda, a = d/2), circular wavefronts from each slit, and the screen with the intensity I(y) = cos^2(pi d sin(t)/lambda) * sinc^2(pi a sin(t)/lambda) beside it.");
    let c = pal(&mut f);
    title(
        &mut f,
        "Young's double-slit experiment",
        "Interference of two coherent wavefronts: $\\cos^2$ fringes under a single-slit $\\mathrm{sinc}^2$ envelope",
    );
    let inten = |y: f64| -> (f64, f64) {
        let s = y / y.hypot(D);
        let al = PI * d * s / lam;
        let be = PI * a * s / lam;
        let sinc = if be.abs() < 1e-9 { 1.0 } else { be.sin() / be };
        (al.cos().powf(2.0) * sinc.powf(2.0), sinc.powf(2.0))
    };
    let small = Arrow(1, 1, 1.0, 50.0, 90.0);
    f.group(|f| {
        // incoming plane wave
        for k in 0..6 {
            let x = 700.0 + k as f64 * lam;
            seg(
                f,
                (x, Ym - 3000.0),
                (x, Ym + 3000.0),
                St::new().pen(c.blue).thick(1).depth(D_GRID),
            );
        }
        axis(f, (400.0, Ym), (1900.0, Ym), St::new().pen(c.blue).thick(2).fwd(A_TRI));
        f.text(400, Ym + 3350.0, vec![H("plane wave")], T::s(11).c(c.blue));
        let (p, q) = ((700.0, Ym - 3350.0), (700.0 + lam, Ym - 3350.0));
        f.poly(&[p, q], St::new().thick(1).fwd(small).back(small));
        f.text(700.0 + lam / 2.0, Ym - 3500.0, vec![G("l")], T::s(13).j(1));
    });
    f.group(|f| {
        // barrier with two slits
        let (top, bot) = (Ym - HH, Ym + HH);
        let (s1, s2) = (Ym - d / 2.0, Ym + d / 2.0);
        for (ya, yb) in [(top, s1 - a / 2.0), (s1 + a / 2.0, s2 - a / 2.0), (s2 + a / 2.0, bot)] {
            f.box_(
                bx - 90.0,
                ya,
                bx + 90.0,
                yb,
                St::new().pen(0).thick(2).fill(c.hatch).pat(45).depth(D_AXIS),
            );
        }
        f.text(bx, top - 220.0, vec![H("double slit")], T::s(11).j(1));
    });
    f.group(|f| {
        // wavefronts
        for (sgn, col) in [(-1.0f64, c.blue), (1.0, c.red)] {
            let cy = Ym + sgn * d / 2.0;
            let off = -sgn * d / 2.0; // slit center height relative to Ym, math (up positive)
            let mut k = 1.0;
            while k * lam <= D - 50.0 {
                let r = k * lam;
                let up = ((HH - off) / r).min(1.0).asin().to_degrees();
                let dn = ((HH + off) / r).min(1.0).asin().to_degrees();
                let (up, dn) = (up.min(75.0), dn.min(75.0));
                f.arc((bx + 90.0, cy), r, -dn, up, St::new().pen(col).thick(1).depth(D_GRID));
                k += 1.0;
            }
        }
    });
    f.group(|f| {
        // screen with fringes
        let n = 66.0;
        for k in 0..66 {
            let k = k as f64;
            let y0 = -HH + 2.0 * HH * k / n;
            let y1 = -HH + 2.0 * HH * (k + 1.0) / n;
            let I0 = inten((y0 + y1) / 2.0).0;
            f.box_(
                xs,
                Ym - y1,
                xs + 240.0,
                Ym - y0,
                St::new()
                    .pen(c.orange)
                    .thick(0)
                    .fill(c.orange)
                    .pat(40 - round_i(20.0 * I0) as i32)
                    .depth(D_FILL),
            );
        }
        seg(f, (xs, Ym - HH), (xs, Ym + HH), St::new().thick(4));
        f.text(xs, Ym + HH + 400.0, vec![H("screen")], T::s(11).j(1));
    });
    f.group(|f| {
        // intensity plot
        let x0 = xs + 500.0;
        let sc = 2300.0;
        let pts: Vec<Pt> = (0..201)
            .map(|k| -HH + 2.0 * HH * k as f64 / 200.0)
            .map(|y| (x0 + sc * inten(y).0, Ym - y))
            .collect();
        let env: Vec<Pt> = (0..101)
            .map(|k| -HH + 2.0 * HH * k as f64 / 100.0)
            .map(|y| (x0 + sc * inten(y).1, Ym - y))
            .collect();
        seg(f, (x0, Ym - HH), (x0, Ym + HH), St::new().thick(2));
        seg(
            f,
            (x0, Ym + HH),
            (x0 + sc + 200.0, Ym + HH),
            St::new().thick(1).depth(D_AXIS),
        );
        f.spline(&env, St::new().pen(c.gray).thick(1).st(1, 5.0).depth(D_CURVE + 5));
        f.spline(&pts, St::new().pen(c.red).thick(3).depth(D_CURVE));
        f.text(
            x0 + sc / 2.0,
            Ym - HH - 220.0,
            vec![I("I"), R("("), I("y"), R(")")],
            T::s(13).j(1),
        );
        let y1 = D * (lam / d).asin().tan();
        for (m, yy) in [(0, 0.0), (1, y1), (-1, -y1)] {
            let lab = if m == 0 {
                r"$m = 0$"
            } else if m > 0 {
                r"$m = +1$"
            } else {
                r"$m = -1$"
            };
            f.text(x0 + sc * inten(yy).0 + 130.0, Ym - yy + 60.0, lab, T::s(11));
        }
    });
    f.group(|f| {
        // geometry: d, a, theta
        let (p, q) = ((bx - 500.0, Ym - d / 2.0), (bx - 500.0, Ym + d / 2.0));
        f.poly(&[p, q], St::new().thick(1).fwd(small).back(small));
        seg(
            f,
            (bx - 650.0, Ym - d / 2.0),
            (bx - 90.0, Ym - d / 2.0),
            St::new().thick(1).st(2, 3.0),
        );
        seg(
            f,
            (bx - 650.0, Ym + d / 2.0),
            (bx - 90.0, Ym + d / 2.0),
            St::new().thick(1).st(2, 3.0),
        );
        f.text(bx - 440.0, Ym + 60.0, vec![I("d")], T::s(13));
        seg(
            f,
            (bx + 90.0, Ym),
            (xs, Ym),
            St::new().pen(c.gray).thick(1).st(2, 3.0).depth(D_AXIS),
        );
        let y1 = D * (lam / d).asin().tan();
        seg(
            f,
            (bx + 90.0, Ym),
            (xs, Ym - y1),
            St::new().pen(0).thick(1).st(1, 5.0).depth(D_AXIS),
        );
        f.arc(
            (bx + 90.0, Ym),
            2600.0,
            0.0,
            (lam / d).asin().to_degrees(),
            St::new().pen(0).thick(1).fwd(A_SMALL).depth(D_MARK),
        );
        f.text(bx + 90.0 + 2750.0, Ym - 120.0, vec![G("q")], T::s(14).halo());
    });
    f.group(|f| {
        f.text(
            bx - 700.0,
            9450,
            r"$d\sin\theta = m\lambda$ (maxima), $d = 5\lambda$, $a = d/2$: orders $\pm 2$ are missing (single-slit zeros)",
            T::s(11),
        );
    });
    Ok(f)
}

// ---- d5 ----

fn carnot() -> Result<Fig, String> {
    let g = 1.4f64;
    // states: 1 (V1,TH) -> 2 (V2,TH) isothermal expansion; 2->3 adiabatic expansion; 3->4 isothermal compression; 4->1 adiabatic
    let (V1, V2) = (1.0f64, 2.2f64);
    let (TH, TC) = (1.0f64, 0.59f64); // T_H / T_C = 1.7, efficiency 1 - T_C/T_H = 0.41
                                      // adiabats: T V^(gamma-1) = const
    let V3 = V2 * (TH / TC).powf(1.0 / (g - 1.0));
    let V4 = V1 * (TH / TC).powf(1.0 / (g - 1.0));
    let p = |T: f64, V: f64| T / V; // p V = n R T with n R = 1 (arbitrary units)
    let mut f = Fig::new(&format!("Carnot cycle in the p-V plane: isotherms pV = const (T_H, T_C) and adiabats pV^gamma = const with gamma = 1.4, computed from T V^(gamma-1) = const; V2/V1 = {:.2}, V3/V4 = {:.2} (equal, as required); the enclosed area is the work. T_H/T_C = 1.7: larger ratios give a thinner loop on linear axes.", V2 / V1, V3 / V4));
    let c = pal(&mut f);
    title(
        &mut f,
        "Carnot cycle",
        "Two isotherms and two adiabats ($\\gamma = 1.4$); the enclosed area is the net work $W$",
    );
    let (Vmax, pmax) = (8.6, 1.1);
    let fr = Fr::new2(2100.0, 8500.0, 950.0, 6000.0);
    let n = 60usize;
    let nf = n as f64;
    let iso_h: Vec<Pt> = (0..=n)
        .map(|i| (V1 + (V2 - V1) * i as f64 / nf, p(TH, V1 + (V2 - V1) * i as f64 / nf)))
        .collect();
    let iso_c: Vec<Pt> = (0..=n)
        .map(|i| (V4 + (V3 - V4) * i as f64 / nf, p(TC, V4 + (V3 - V4) * i as f64 / nf)))
        .collect();
    let (p2, p3, p4, p1) = (p(TH, V2), p(TC, V3), p(TC, V4), p(TH, V1));
    let adi23: Vec<Pt> = (0..=n)
        .map(|i| {
            let v = V2 + (V3 - V2) * i as f64 / nf;
            (v, p2 * (V2 / v).powf(g))
        })
        .collect();
    let adi41: Vec<Pt> = (0..=n)
        .map(|i| {
            let v = V1 + (V4 - V1) * i as f64 / nf;
            (v, p1 * (V1 / v).powf(g))
        })
        .collect();
    let end_ok = |curve: &[Pt], want: f64| curve.last().is_some_and(|e| (e.1 - want).abs() < 1e-6);
    if !(end_ok(&adi23, p3) && end_ok(&adi41, p4)) {
        return Err("carnot: the adiabats do not end on the isotherms".into());
    }
    f.group(|f| {
        // enclosed area
        let mut lp = iso_h.clone();
        lp.extend(adi23.iter().skip(1));
        lp.extend(iso_c.iter().rev().skip(1));
        lp.extend(adi41.iter().rev().skip(1));
        f.poly(
            &fr.ps(&lp),
            St::new()
                .pen(c.lblue)
                .thick(0)
                .fill(c.lblue)
                .pat(20)
                .closed()
                .depth(D_FILL),
        );
    });
    f.group(|f| {
        f.spline(&fr.ps(&iso_h), St::new().pen(c.red).thick(3).depth(D_CURVE));
        f.spline(&fr.ps(&iso_c), St::new().pen(c.blue).thick(3).depth(D_CURVE));
        f.spline(&fr.ps(&adi23), St::new().pen(c.green).thick(3).depth(D_CURVE));
        f.spline(&fr.ps(&adi41), St::new().pen(c.green).thick(3).depth(D_CURVE));
        // thin dashed continuations of the isotherms
        let ext_h: Vec<Pt> = (0..31)
            .map(|i| {
                (
                    V2 + (3.6 - V2) * i as f64 / 30.0,
                    p(TH, V2 + (3.6 - V2) * i as f64 / 30.0),
                )
            })
            .collect();
        f.spline(
            &fr.ps(&ext_h),
            St::new().pen(c.red).thick(1).st(1, 5.0).depth(D_CURVE + 5),
        );
        let ext_c: Vec<Pt> = (0..21)
            .map(|i| {
                (
                    V3 + (Vmax - 0.3 - V3) * i as f64 / 20.0,
                    p(TC, V3 + (Vmax - 0.3 - V3) * i as f64 / 20.0),
                )
            })
            .collect();
        f.spline(
            &fr.ps(&ext_c),
            St::new().pen(c.blue).thick(1).st(1, 5.0).depth(D_CURVE + 5),
        );
    });
    f.group(|f| {
        // direction arrows on the curves
        let mut head = |curve: &[Pt], i: usize, col: i32| {
            if let (Some(&a), Some(&b)) = (curve.get(i - 1), curve.get(i + 1)) {
                let (a, b) = (fr.p(a.0, a.1), fr.p(b.0, b.1));
                f.poly(&[a, b], St::new().pen(col).thick(3).fwd(A_TRI).depth(D_CURVE - 5));
            }
        };
        let rev = |v: &[Pt]| -> Vec<Pt> { v.iter().rev().copied().collect() };
        head(&iso_h, 30, c.red);
        head(&adi23, 30, c.green);
        head(&rev(&iso_c), 30, c.blue);
        head(&rev(&adi41), 30, c.green);
    });
    let ax0 = fr.p(0, 0);
    f.group(|f| {
        // axes, states, guides
        axis(f, fr.p(0, 0), fr.p(Vmax, 0), St::new());
        axis(f, fr.p(0, 0), fr.p(0, pmax), St::new());
        f.tp(fr.p(Vmax, -0.05), "$V$", T::s(12));
        f.tp(fr.p(0.12, pmax + 0.02), "$p$", T::s(12));
        let states: [(&str, f64, f64, Pt); 4] = [
            ("1", V1, p1, (-260.0, -120.0)),
            ("2", V2, p2, (150.0, -120.0)),
            ("3", V3, p3, (150.0, -150.0)),
            ("4", V4, p4, (-80.0, -170.0)),
        ];
        for (nm, V, pp, off) in states {
            let (x, y) = fr.p(V, pp);
            f.circle((x, y), 65.0, St::new().pen(0).thick(1).fill(0).depth(D_MARK));
            f.text(x + off.0, y + off.1, format!("\\textbf{{{nm}}}"), T::s(12));
            seg(
                f,
                (x, y),
                (x, ax0.1),
                St::new().pen(c.gray).thick(1).st(2, 3.0).depth(D_GRID),
            );
            seg(
                f,
                (x, y),
                (ax0.0, y),
                St::new().pen(c.gray).thick(1).st(2, 3.0).depth(D_GRID),
            );
            f.text(x, ax0.1 + 420.0, format!("$V_{nm}$"), T::s(12).j(1));
            f.text(ax0.0 - 150.0, y + 60.0, format!("$p_{nm}$"), T::s(12).j(2));
        }
    });
    f.group(|f| {
        // curve names, well clear of the lines
        f.tp(fr.p(1.6, 1.07), "isotherm $T_H$", T::s(12).j(1).c(c.red));
        f.tp(fr.p(5.6, 0.22), "isotherm $T_C$", T::s(12).c(c.blue));
    });
    f.group(|f| {
        // heat flows: arrows touch the curve, labels sit beyond the arrow tails
        let (mut x, y) = fr.p(1.6, p(TH, 1.6));
        x += 300.0;
        vec(f, (x, y - 1050.0), (0.0, 850.0), c.red, St::v().thick(5).fwd(A_IND)); // points down onto the curve
        f.text(x, y - 1250.0, "$Q_H$ in", T::s(12).j(1).c(c.red));
        let (x, y) = fr.p(4.9, p(TC, 4.9));
        vec(f, (x, y + 150.0), (0.0, 500.0), c.blue, St::v().thick(5).fwd(A_IND)); // leaves the curve downward
        f.text(x + 300.0, y + 480.0, "$Q_C$ out", T::s(12).c(c.blue));
    });
    f.group(|f| {
        // work label with a leader into the loop
        let (lx, ly) = fr.p(4.6, 0.66);
        f.text(lx, ly, r"$W = \oint p\,dV$", T::s(12).j(1));
        let tip = fr.p(3.0, 0.255);
        f.poly(
            &[(lx - 350.0, ly + 150.0), tip],
            St::new().pen(0).thick(1).fwd(A_SMALL).depth(D_MARK),
        );
    });
    f.group(|f| {
        // legend and numbers
        let x0 = 9900.0;
        f.text(x0, 2600, "Steps:", T::s(12));
        let steps = [
            (r"$1 \to 2$ isothermal expansion", c.red),
            (r"$2 \to 3$ adiabatic expansion", c.green),
            (r"$3 \to 4$ isothermal compression", c.blue),
            (r"$4 \to 1$ adiabatic compression", c.green),
        ];
        for (k, (txt, col)) in steps.into_iter().enumerate() {
            let y = 3150.0 + k as f64 * 450.0;
            seg(f, (x0, y - 60.0), (x0 + 400.0, y - 60.0), St::new().pen(col).thick(3));
            f.text(x0 + 550.0, y, txt, T::s(12));
        }
        f.text(
            x0,
            5400,
            r"adiabats: $pV^\gamma = \mathrm{const}$, $\gamma = 1.4$",
            T::s(12),
        );
        f.text(x0, 5850, r"isotherms: $pV = \mathrm{const}$", T::s(12));
        f.text(x0, 6700, format!(r"$T_H/T_C = {:.1}$", TH / TC), T::s(12));
        f.text(
            x0,
            7200,
            format!(r"$\eta = W/Q_H = 1 - T_C/T_H = {:.2}$", 1.0 - TC / TH),
            T::s(12),
        );
        f.text(x0, 7700, format!(r"$V_3/V_4 = V_2/V_1 = {:.2}$", V2 / V1), T::s(11));
    });
    Ok(f)
}

fn rlc() -> Result<Fig, String> {
    let mut f = Fig::new("Series RLC circuit: AC source V(t), resistor (zigzag polyline), inductor (X-spline coils), capacitor (two plates), the current I(t) as an arrow and the voltage drops; resonance at omega_0 = 1/sqrt(LC).");
    let c = pal(&mut f);
    title(
        &mut f,
        "Series RLC circuit",
        "AC source driving a resistor, an inductor and a capacitor in series",
    );
    let (xL, xR, yT, yB) = (2500.0f64, 8000.0f64, 3300.0f64, 7300.0f64);
    let ymid = ((yT + yB) / 2.0).floor();
    let (RX0, RX1) = (3500.0f64, 5000.0f64); // resistor ends (the wires end exactly here)
    let (LX0, LX1) = (5500.0f64, 6900.0f64); // inductor ends
    let (SRC_R, GAP) = (450.0f64, 80.0f64); // source circle radius, half the capacitor gap
    let (srcT, srcB) = ((xL, ymid - SRC_R), (xL, ymid + SRC_R));
    let (capT, capB) = ((xR, ymid - GAP), (xR, ymid + GAP));
    f.group(|f| {
        // wires: every end is the exact end point of its component
        f.poly(&[srcT, (xL, yT), (RX0, yT)], St::new().thick(3).join(1).cap(1)); // source -> resistor
        f.poly(&[(RX1, yT), (LX0, yT)], St::new().thick(3).cap(1)); // resistor -> inductor
        f.poly(&[(LX1, yT), (xR, yT), capT], St::new().thick(3).join(1).cap(1)); // inductor -> upper plate
        f.poly(&[capB, (xR, yB), (xL, yB), srcB], St::new().thick(3).join(1).cap(1));
        // lower plate -> source
    });
    f.group(|f| {
        // AC source
        f.circle((xL, ymid), SRC_R, St::new().pen(0).thick(3).fill(c.lgray).depth(D_MARK));
        let pts: Vec<Pt> = (0..41)
            .map(|k| {
                let k = k as f64;
                (
                    xL - 300.0 + 600.0 * k / 40.0,
                    ymid - 170.0 * (2.0 * PI * k / 40.0).sin(),
                )
            })
            .collect();
        f.spline(&pts, St::new().pen(c.red).thick(2).depth(D_MARK - 5));
        f.text(
            xL - 650.0,
            ymid + 80.0,
            vec![I("V"), R("("), I("t"), R(")")],
            T::s(14).j(2),
        );
    });
    f.group(|f| {
        // resistor: zigzag from (RX0, yT) to (RX1, yT)
        let n = 6;
        let mut zz = vec![(RX0, yT)];
        for k in 0..n {
            let xm = RX0 + (RX1 - RX0) * (k as f64 + 0.5) / n as f64;
            zz.push((xm, yT + (if k % 2 == 0 { -1.0 } else { 1.0 }) * 280.0));
        }
        zz.push((RX1, yT));
        f.poly(&zz, St::new().pen(c.green).thick(3).join(0).cap(1).depth(D_MARK));
        f.text((RX0 + RX1) / 2.0, yT - 420.0, vec![I("R")], T::s(15).j(1).c(c.green));
    });
    f.group(|f| {
        // inductor: four half-circle coils from (LX0, yT) to (LX1, yT)
        let k = 4;
        let w = (LX1 - LX0) / k as f64;
        for q in 0..k {
            f.arc(
                (LX0 + q as f64 * w + w / 2.0, yT),
                w / 2.0,
                0.0,
                180.0,
                St::new().pen(c.blue).thick(3).cap(1).depth(D_MARK),
            );
        }
        f.text((LX0 + LX1) / 2.0, yT - 480.0, vec![I("L")], T::s(15).j(1).c(c.blue));
    });
    f.group(|f| {
        // capacitor: two plates, the leads end exactly on them
        seg(
            f,
            (xR - 420.0, ymid - GAP),
            (xR + 420.0, ymid - GAP),
            St::new().pen(c.orange).thick(5).cap(1).depth(D_MARK),
        );
        seg(
            f,
            (xR - 420.0, ymid + GAP),
            (xR + 420.0, ymid + GAP),
            St::new().pen(c.orange).thick(5).cap(1).depth(D_MARK),
        );
        f.text(xR + 560.0, ymid + 70.0, vec![I("C")], T::s(15).c(c.orange));
        f.text(xR - 520.0, ymid - 160.0, "$+$", T::s(12).j(2));
        f.text(xR - 520.0, ymid + 300.0, "$-$", T::s(12).j(2));
    });
    f.group(|f| {
        // current: the arrow head sits on the wire, pointing to the resistor
        vec(
            f,
            (2900.0, yT),
            (450.0, 0.0),
            0,
            St::v().thick(3).fwd(A_TRI).depth(D_MARK),
        );
        f.text(3050, yT - 220.0, vec![I("I"), R("("), I("t"), R(")")], T::s(14));
    });
    f.group(|f| {
        // voltage drops: double-headed arrows along the elements, above their name labels
        let ya = yT - 1000.0;
        for (xa, xb, nm, col) in [(RX0, RX1, "V_R", c.green), (LX0, LX1, "V_L", c.blue)] {
            f.poly(
                &[(xa, ya), (xb, ya)],
                St::new().pen(col).thick(1).fwd(A_SMALL).back(A_SMALL).depth(D_AXIS),
            );
            for x in [xa, xb] {
                // dotted guides from the element ends up to the arrow
                seg(
                    f,
                    (x, if nm == "V_R" { yT - 480.0 } else { yT - 120.0 }),
                    (x, ya - 60.0),
                    St::new().pen(c.gray).thick(1).st(2, 3.0).depth(D_GRID),
                );
            }
            f.text((xa + xb) / 2.0, ya - 140.0, format!("${nm}$"), T::s(12).j(1).c(col));
        }
    });
    f.group(|f| {
        let x0 = 9700;
        f.text(x0, 3400, vec![H("Kirchhoff's loop rule:")], T::s(12));
        f.text(
            x0,
            3950,
            r"$L\,\dfrac{dI}{dt} + RI + \dfrac{Q}{C} = V_0\cos\omega t$",
            T::s(12),
        );
        f.text(x0, 4900, r"$\omega_0 = 1/\sqrt{LC}$", T::s(12));
        f.text(x0, 5400, vec![H("resonance frequency")], T::s(11));
        f.text(x0, 6200, r"$|Z| = \sqrt{R^2 + (\omega L - 1/\omega C)^2}$", T::s(12));
    });
    Ok(f)
}

// ---- d6 ----

fn feynman() -> Result<Fig, String> {
    let mut f = Fig::new("Feynman diagram for e- e+ -> gamma* -> mu- mu+ (time runs to the right): fermion lines with arrows (the positron's arrow points against time), a virtual photon drawn as a sine-wave X-spline, vertices as dots.");
    let c = pal(&mut f);
    title(
        &mut f,
        "Electron-positron annihilation into muons",
        "Lowest-order QED process, time flows to the right",
    );
    let (V1, V2) = ((3900.0f64, 5400.0f64), (6700.0f64, 5400.0f64));
    let ds = 2100.0;
    f.group(|f| {
        // incoming electron and positron
        let a = (V1.0 - ds, V1.1 + ds);
        seg(f, a, V1, St::new().pen(c.blue).thick(3));
        mid_arrow(f, a, V1, 0.5, 0.04, St::new().pen(c.blue).thick(3).fwd(A_IND));
        let b = (V1.0 - ds, V1.1 - ds);
        seg(f, V1, b, St::new().pen(c.red).thick(3));
        mid_arrow(f, V1, b, 0.5, 0.04, St::new().pen(c.red).thick(3).fwd(A_IND));
        f.text(a.0 - 150.0, a.1 + 100.0, r"$e^-$", T::s(14).j(2).big().c(c.blue));
        f.text(b.0 - 150.0, b.1 + 100.0, r"$e^+$", T::s(14).j(2).big().c(c.red));
    });
    f.group(|f| {
        // outgoing muons
        let a = (V2.0 + ds, V2.1 - ds);
        seg(f, V2, a, St::new().pen(c.blue).thick(3));
        mid_arrow(f, V2, a, 0.5, 0.04, St::new().pen(c.blue).thick(3).fwd(A_IND));
        let b = (V2.0 + ds, V2.1 + ds);
        seg(f, b, V2, St::new().pen(c.red).thick(3));
        mid_arrow(f, b, V2, 0.5, 0.04, St::new().pen(c.red).thick(3).fwd(A_IND));
        f.text(a.0 + 150.0, a.1 - 40.0, r"$\mu^-$", T::s(14).big().c(c.blue));
        f.text(b.0 + 150.0, b.1 + 160.0, r"$\mu^+$", T::s(14).big().c(c.red));
    });
    f.group(|f| {
        // photon: sine wave spline
        let n = 6.0; // periods
        let nn = 12 * 6;
        let mut pts: Vec<Pt> = Vec::new();
        for k in 0..=nn {
            let t = k as f64 / nn as f64;
            let amp = 280.0 * 1.0f64.min(t * 12.0).min((1.0 - t) * 12.0);
            pts.push((V1.0 + (V2.0 - V1.0) * t, V1.1 - amp * (2.0 * PI * n * t).sin()));
        }
        f.spline(
            &pts,
            St::new().pen(c.green).thick(3).s(1.0).smooth_ends().depth(D_AXIS + 10),
        );
        f.text(
            (V1.0 + V2.0) / 2.0,
            V1.1 - 600.0,
            r"$\gamma^*$",
            T::s(14).j(1).big().c(c.green),
        );
        f.text(
            (V1.0 + V2.0) / 2.0,
            V1.1 + 800.0,
            vec![H("virtual photon")],
            T::s(11).j(1).c(c.green),
        );
    });
    f.group(|f| {
        // vertices
        for v in [V1, V2] {
            f.circle(v, 100.0, St::new().pen(0).thick(1).fill(0).depth(D_MARK));
        }
    });
    f.group(|f| {
        // time arrow and axis
        let y = 8600.0;
        vec(f, (2000.0, y), (6000.0, 0.0), 0, St::v().thick(2).fwd(A_STICK));
        f.text(8200, y + 80.0, vec![I("t")], T::s(14));
        f.text(2000, y + 450.0, vec![H("time")], T::s(11));
        // dashed time slices
        for x in [V1.0, V2.0] {
            seg(
                f,
                (x, 2300.0),
                (x, 8400.0),
                St::new().pen(c.gray).thick(1).st(2, 3.0).depth(D_GRID),
            );
        }
        f.text(V1.0, 2200, r"$e^-e^+$ annihilate", T::s(10).j(1).c(c.gray));
        f.text(V2.0, 2200, vec![H("pair created")], T::s(10).j(1).c(c.gray));
    });
    f.group(|f| {
        let x0 = 9800;
        f.text(x0, 3300, vec![H("Process")], T::s(12));
        f.text(x0, 3900, r"$e^-\,e^+ \to \gamma^* \to \mu^-\,\mu^+$", T::s(12));
        f.text(x0, 4700, vec![H("Arrow: flow of lepton number;")], T::s(11));
        f.text(x0, 5000, vec![H("the antiparticle's arrow points")], T::s(11));
        f.text(x0, 5300, vec![H("backwards in time.")], T::s(11));
        f.text(x0, 6000, vec![H("Cross section (QED):")], T::s(11));
        f.text(x0, 6500, r"$\sigma = 4\pi\alpha^2/3s$", T::s(12));
    });
    Ok(f)
}

fn minkowski() -> Result<Fig, String> {
    let b = 0.5f64;
    let gm = 1.0 / (1.0 - b * b).sqrt();
    let mut f = Fig::new("Minkowski diagram: light cone (45 degree dashed lines), boosted frame S' for v = 0.5c: the ct' axis is the line x = beta*ct (tilted by atan(0.5) = 26.57 deg from ct), the x' axis the line ct = beta*x; calibration hyperbolas c^2t^2 - x^2 = +-1; one event with its coordinates in both frames.");
    let c = pal(&mut f);
    title(
        &mut f,
        "Minkowski diagram",
        "Light cone and the axes of a frame moving at $v = 0.5\\,c$; the same event has coordinates in both frames",
    );
    let fr = Fr::new(5200.0, 5800.0, 1050.0);
    let L = 3.6;
    f.group(|f| {
        // light-cone regions
        f.poly(
            &fr.ps(&[(0.0, 0.0), (L, L), (-L, L)]),
            St::new()
                .pen(c.lblue)
                .thick(0)
                .fill(c.lblue)
                .pat(20)
                .closed()
                .depth(D_FILL + 10),
        );
        f.poly(
            &fr.ps(&[(0.0, 0.0), (L, -L), (-L, -L)]),
            St::new()
                .pen(c.lorange)
                .thick(0)
                .fill(c.lorange)
                .pat(20)
                .closed()
                .depth(D_FILL + 10),
        );
        f.tp(fr.p(-0.9, 3.0), vec![H("future")], T::s(13).j(1).c(c.blue));
        f.tp(fr.p(-0.9, -3.2), vec![H("past")], T::s(13).j(1).c(c.orange));
        f.tp(fr.p(-3.0, 0.4), vec![H("elsewhere")], T::s(12).j(1).c(c.gray));
        f.tp(fr.p(3.0, 0.4), vec![H("elsewhere")], T::s(12).j(1).c(c.gray));
    });
    f.group(|f| {
        // light lines
        seg(
            f,
            fr.p(-L, -L),
            fr.p(L, L),
            St::new().pen(c.orange).thick(2).st(1, 6.0).depth(D_CURVE),
        );
        seg(
            f,
            fr.p(-L, L),
            fr.p(L, -L),
            St::new().pen(c.orange).thick(2).st(1, 6.0).depth(D_CURVE),
        );
        f.tp(fr.p(L - 0.02, L - 0.12), vec![H("light")], T::s(11).j(2).c(c.orange));
    });
    f.group(|f| {
        // S axes
        axis(f, fr.p(-3.9, 0), fr.p(3.9, 0), St::new());
        axis(f, fr.p(0, -3.9), fr.p(0, 3.9), St::new());
        f.tp(fr.p(3.9, -0.28), vec![I("x")], T::s(14));
        f.tp(fr.p(0.12, 3.9), vec![I("ct")], T::s(14));
    });
    let k = 3.0;
    f.group(|f| {
        // S' axes
        axis(
            f,
            fr.p(-b * k, -k),
            fr.p(b * k * 1.15, k * 1.15),
            St::new().pen(c.red).fwd(A_TRI),
        );
        axis(
            f,
            fr.p(-k, -b * k),
            fr.p(k * 1.15, b * k * 1.15),
            St::new().pen(c.red).fwd(A_TRI),
        );
        f.tp(fr.p(b * k * 1.15 + 0.1, k * 1.15 + 0.05), "$ct'$", T::s(14).c(c.red));
        f.tp(fr.p(k * 1.15 + 0.1, b * k * 1.15 - 0.2), "$x'$", T::s(14).c(c.red));
    });
    f.group(|f| {
        // hyperbolas c^2t^2 - x^2 = +-1 (X-splines)
        let us: Vec<f64> = (0..25).map(|i| -1.6 + 3.2 * i as f64 / 24.0).collect();
        let up: Vec<Pt> = us.iter().map(|u| (u.sinh(), u.cosh())).collect();
        let rt: Vec<Pt> = us.iter().map(|u| (u.cosh(), u.sinh())).collect();
        f.spline(&fr.ps(&up), St::new().pen(c.green).thick(1).depth(D_CURVE + 5));
        f.spline(&fr.ps(&rt), St::new().pen(c.green).thick(1).depth(D_CURVE + 5));
        // unit marks on the primed axes: points gamma*(b,1) and gamma*(1,b)
        for pt in [(gm * b, gm), (gm, gm * b)] {
            let (x, y) = fr.p(pt.0, pt.1);
            f.circle((x, y), 55.0, St::new().pen(c.red).thick(1).fill(c.red).depth(D_MARK));
        }
        f.tp(fr.p(gm * b + 0.12, gm - 0.05), vec![H("1")], T::s(11).c(c.red));
        f.tp(fr.p(gm + 0.05, gm * b - 0.28), vec![H("1")], T::s(11).c(c.red));
        f.tp(fr.p(-1.7, 3.2), r"$c^2t^2 - x^2 = 1$", T::s(11).c(c.green).j(1));
        f.tp(fr.p(2.6, -0.9), r"$x^2 - c^2t^2 = 1$", T::s(11).c(c.green).j(1));
    });
    let P = (1.5f64, 2.0f64);
    let xp = gm * (P.0 - b * P.1);
    let tp = gm * (P.1 - b * P.0);
    f.group(|f| {
        // an event P with parallels to both axis systems
        // foot on S' axes (parallels to the other primed axis)
        let fx = (xp * gm, xp * gm * b); // along x' axis
        let ft = (tp * gm * b, tp * gm); // along ct' axis
        seg(
            f,
            fr.p(P.0, P.1),
            fr.p(fx.0, fx.1),
            St::new().pen(c.red).thick(1).st(2, 3.0).depth(D_MARK),
        );
        seg(
            f,
            fr.p(P.0, P.1),
            fr.p(ft.0, ft.1),
            St::new().pen(c.red).thick(1).st(2, 3.0).depth(D_MARK),
        );
        seg(
            f,
            fr.p(P.0, P.1),
            fr.p(P.0, 0),
            St::new().pen(0).thick(1).st(2, 3.0).depth(D_MARK),
        );
        seg(
            f,
            fr.p(P.0, P.1),
            fr.p(0, P.1),
            St::new().pen(0).thick(1).st(2, 3.0).depth(D_MARK),
        );
        let (x, y) = fr.p(P.0, P.1);
        f.circle((x, y), 80.0, St::new().pen(0).thick(1).fill(0).depth(D_MARK - 5));
        f.text(x + 140.0, y - 100.0, r"$P$", T::s(14));
    });
    f.group(|f| {
        let x0 = 9700;
        f.text(x0, 3000, r"Frame $S'$ moves at $v = 0.5\,c$", T::s(12));
        f.text(x0, 3600, format!(r"$\beta = v/c = 0.5,\ \gamma = {gm:.3}$"), T::s(12));
        f.text(
            x0,
            4300,
            format!(
                r"$ct'$ axis: $x = \beta\,ct$ (tilt ${:.2}^\circ$)",
                b.atan().to_degrees()
            ),
            T::s(11),
        );
        f.text(x0, 4700, r"$x'$ axis: $ct = \beta x$", T::s(11));
        f.text(
            x0,
            5300,
            format!(r"Event $P = (x, ct) = ({:.1}, {:.1})$", P.0, P.1),
            T::s(11),
        );
        f.text(
            x0,
            5700,
            format!(r"in $S'$: $x' = {xp:.2}$, $ct' = {tp:.2}$"),
            T::s(11).c(c.red),
        );
        f.text(x0, 6300, r"$(ct')^2 - x'^2 = (ct)^2 - x^2$", T::s(12));
        f.text(
            x0,
            6700,
            format!(r"invariant interval: ${:.2}$", P.1 * P.1 - P.0 * P.0),
            T::s(11),
        );
    });
    Ok(f)
}

// ---- d7 ----

fn loglog() -> Result<Fig, String> {
    let mut f = Fig::new("Log-log plot in publication style: decade ticks with minor ticks, power-law data y = A x^-1.5 with scatter, error bars and a fitted line; legend.");
    let c = pal(&mut f);
    title(
        &mut f,
        "Power law on log-log axes",
        "Data with error bars and a least-squares line of slope $-1.5$",
    );
    let (x0, y0, w, h) = (2600.0f64, 8300.0f64, 7000.0f64, 5800.0f64);
    // x in [1e0, 1e3], y in [1e-3, 1e2]
    let lx = |x: f64| x0 + w * x.log10() / 3.0;
    let ly = |y: f64| y0 - h * (y.log10() + 3.0) / 5.0;
    let mut rnd = PyRandom::new(7);
    let (A, k) = (40.0f64, -1.5f64);
    // 10 ** d for an integer d, as Python computes it
    let ten = |d: i32| if d >= 0 { 10f64.powi(d) } else { 10f64.powf(d as f64) };
    f.group(|f| {
        // grid at decades
        for d in 0..4 {
            seg(
                f,
                (lx(ten(d)), y0),
                (lx(ten(d)), y0 - h),
                St::new().pen(c.lgray).thick(1).depth(D_GRID),
            );
        }
        for d in -3..3 {
            seg(
                f,
                (x0, ly(ten(d))),
                (x0 + w, ly(ten(d))),
                St::new().pen(c.lgray).thick(1).depth(D_GRID),
            );
        }
    });
    f.group(|f| {
        // frame
        f.box_(x0, y0 - h, x0 + w, y0, St::new().pen(0).thick(2).depth(D_AXIS));
    });
    f.group(|f| {
        // x ticks
        for d in 0..4 {
            for m in 1..10 {
                let v = m as f64 * ten(d);
                if v > 1000.0 {
                    break;
                }
                let L = if m == 1 { 150.0 } else { 80.0 };
                seg(f, (lx(v), y0), (lx(v), y0 - L), St::new().thick(1));
                seg(f, (lx(v), y0 - h), (lx(v), y0 - h + L), St::new().thick(1));
            }
            f.text(lx(ten(d)), y0 + 400.0, pow10(d), T::s(12).j(1));
        }
        f.text(x0 + w / 2.0, y0 + 850.0, r"$x$ (frequency, Hz)", T::s(12).j(1));
    });
    f.group(|f| {
        // y ticks
        for d in -3..3 {
            for m in 1..10 {
                let v = m as f64 * ten(d);
                if v > 100.0 {
                    break;
                }
                let L = if m == 1 { 150.0 } else { 80.0 };
                seg(f, (x0, ly(v)), (x0 + L, ly(v)), St::new().thick(1));
                seg(f, (x0 + w, ly(v)), (x0 + w - L, ly(v)), St::new().thick(1));
            }
            f.text(x0 - 150.0, ly(ten(d)) + 80.0, pow10(d), T::s(12).j(2));
        }
        f.text(
            x0 - 1000.0,
            y0 - h / 2.0 + 1500.0,
            r"$y$ (power spectral density, a.u.)",
            T::s(12).j(1).a(90.0),
        );
    });
    f.group(|f| {
        // fit line
        seg(
            f,
            (lx(1.3), ly(A * 1.3f64.powf(k))),
            (lx(700.0), ly(A * 700.0f64.powf(k))),
            St::new().pen(c.red).thick(2).depth(D_CURVE),
        );
    });
    let mut pts: Vec<(f64, f64, f64)> = Vec::new();
    for i in 0..14 {
        let x = 10f64.powf(0.12 + 2.7 * i as f64 / 13.0);
        let y = A * x.powf(k) * rnd.gauss(0.0, 0.12).exp();
        let e = if i < 13 {
            y * (0.10 + 0.25 * i as f64 / 13.0)
        } else {
            y * 0.4
        };
        pts.push((x, y, e));
    }
    f.group(|f| {
        for &(x, y, e) in &pts {
            let X = lx(x);
            let lo = (y - e).max(1.2e-3);
            seg(
                f,
                (X, ly(y + e)),
                (X, ly(lo)),
                St::new().pen(c.blue).thick(1).depth(D_MARK + 5),
            );
            seg(
                f,
                (X - 70.0, ly(y + e)),
                (X + 70.0, ly(y + e)),
                St::new().pen(c.blue).thick(1).depth(D_MARK + 5),
            );
            seg(
                f,
                (X - 70.0, ly(lo)),
                (X + 70.0, ly(lo)),
                St::new().pen(c.blue).thick(1).depth(D_MARK + 5),
            );
            f.circle(
                (X, ly(y)),
                70.0,
                St::new().pen(c.blue).thick(1).fill(c.lblue).depth(D_MARK),
            );
        }
    });
    f.group(|f| {
        // legend
        let (lx0, ly0) = (x0 + w - 2500.0, y0 - h + 350.0);
        f.box_(
            lx0,
            ly0 - 150.0,
            lx0 + 2300.0,
            ly0 + 820.0,
            St::new().pen(0).thick(1).fill(7).depth(D_MARK - 5),
        );
        f.circle(
            (lx0 + 280.0, ly0 + 100.0),
            70.0,
            St::new().pen(c.blue).thick(1).fill(c.lblue).depth(D_MARK - 10),
        );
        seg(
            f,
            (lx0 + 280.0, ly0 - 30.0),
            (lx0 + 280.0, ly0 + 230.0),
            St::new().pen(c.blue).thick(1).depth(D_MARK - 10),
        );
        f.text(lx0 + 600.0, ly0 + 160.0, r"data $\pm 1\sigma$", T::s(11).d(D_MARK - 20));
        seg(
            f,
            (lx0 + 100.0, ly0 + 520.0),
            (lx0 + 460.0, ly0 + 520.0),
            St::new().pen(c.red).thick(2).depth(D_MARK - 10),
        );
        f.text(lx0 + 600.0, ly0 + 580.0, r"$y = A\,x^{-1.5}$", T::s(11).d(D_MARK - 20));
    });
    Ok(f)
}

fn damped() -> Result<Fig, String> {
    let mut f = Fig::new("Damped harmonic oscillator x(t) = x0 exp(-t/tau) cos(omega t) with the dashed exponential envelope +-x0 exp(-t/tau); axes labeled with units, the period T and 1/e time tau marked.");
    let c = pal(&mut f);
    title(
        &mut f,
        "Damped harmonic oscillator",
        "Underdamped motion $x(t)$ inside its exponential envelope",
    );
    let (tau, om) = (3.0f64, 2.0 * PI / 2.0); // period 2 s
    let fr = Fr::new2(2300.0, 5300.0, 780.0, 2200.0);
    let xf = |t: f64| (-t / tau).exp() * (om * t).cos();
    let env = |t: f64| (-t / tau).exp();
    let Tmax = 11.0f64;
    f.group(|f| {
        for v in [-1.0, -0.5, 0.5, 1.0] {
            seg(
                f,
                fr.p(0, v),
                fr.p(Tmax, v),
                St::new().pen(c.lgray).thick(1).depth(D_GRID),
            );
        }
        for t in [2, 4, 6, 8, 10] {
            seg(
                f,
                fr.p(t, -1.1),
                fr.p(t, 1.1),
                St::new().pen(c.lgray).thick(1).depth(D_GRID),
            );
        }
    });
    f.group(|f| {
        f.spline(
            &fr.ps(&sample(env, 0.0, Tmax, 40)),
            St::new().pen(c.red).thick(2).st(1, 6.0).depth(D_CURVE + 5),
        );
        f.spline(
            &fr.ps(&sample(|t| -env(t), 0.0, Tmax, 40)),
            St::new().pen(c.red).thick(2).st(1, 6.0).depth(D_CURVE + 5),
        );
        f.tp(fr.p(0.55, 1.02), r"$x_0\,e^{-t/\tau}$", T::s(12).c(c.red));
    });
    f.group(|f| {
        f.spline(
            &fr.ps(&sample(xf, 0.0, Tmax, 330)),
            St::new().pen(c.blue).thick(3).depth(D_CURVE),
        );
        f.tp(
            fr.p(7.6, 0.75),
            r"$x(t) = x_0\,e^{-t/\tau}\cos\omega t$",
            T::s(12).c(c.blue),
        );
    });
    f.group(|f| {
        axis(f, fr.p(0, 0), fr.p(Tmax + 0.4, 0), St::new());
        axis(f, fr.p(0, -1.2), fr.p(0, 1.25), St::new());
        for t in [0, 2, 4, 6, 8, 10] {
            let (x, y) = fr.p(t, 0);
            seg(f, (x, y - 70.0), (x, y + 70.0), St::new().thick(1));
            f.text(x, y + 420.0, vec![H(&t.to_string())], T::s(11).j(1));
        }
        f.tp(fr.p(Tmax + 0.4, -0.2), r"$t$ (s)", T::s(13).j(1));
        for v in [-1.0f64, -0.5, 0.5, 1.0] {
            let (x, y) = fr.p(0, v);
            seg(f, (x - 70.0, y), (x + 70.0, y), St::new().thick(1));
            let lab = if v < 0.0 { format!("$-{}$", fmt_g(-v)) } else { fmt_g(v) };
            f.text(x - 130.0, y + 60.0, lab, T::s(11).j(2));
        }
        f.text(
            fr.p(0, 0).0 - 750.0,
            fr.p(0, 0).1 + 700.0,
            r"$x$ (cm)",
            T::s(13).j(1).a(90.0),
        );
    });
    f.group(|f| {
        // period
        let (t0, t1) = (2.0f64, 4.0f64);
        let (a, b) = (fr.p(t0, -1.15), fr.p(t1, -1.15));
        f.poly(&[a, b], St::new().thick(1).fwd(A_SMALL).back(A_SMALL));
        f.text((a.0 + b.0) / 2.0, a.1 + 330.0, r"$T = 2\,\mathrm{s}$", T::s(12).j(1));
        for t in [t0, t1] {
            seg(
                f,
                fr.p(t, xf(t)),
                fr.p(t, -1.2),
                St::new().pen(c.gray).thick(1).st(2, 3.0).depth(D_GRID),
            );
        }
    });
    f.group(|f| {
        // tau marker
        let (x, y) = fr.p(tau, env(tau));
        f.circle((x, y), 65.0, St::new().pen(c.red).thick(1).fill(c.red).depth(D_MARK));
        seg(
            f,
            (x, y),
            (x, fr.p(0, 0).1),
            St::new().pen(c.red).thick(1).st(2, 3.0).depth(D_GRID),
        );
        seg(
            f,
            (x, y),
            (fr.p(0, 0).0, y),
            St::new().pen(c.red).thick(1).st(2, 3.0).depth(D_GRID),
        );
        f.text(fr.p(0, 0).0 - 130.0, y + 60.0, vec![R("1/e")], T::s(10).j(2).c(c.red));
        f.text(x + 140.0, y - 200.0, r"$\tau = 3\,\mathrm{s}$", T::s(12).c(c.red));
    });
    f.group(|f| {
        let x0 = 10800;
        f.text(x0, 3300, vec![H("Equation of motion")], T::s(12));
        f.text(x0, 3900, r"$m\ddot x + b\dot x + kx = 0$", T::s(12));
        f.text(
            x0,
            4700,
            r"$\tau = 2m/b,\ \omega = \sqrt{\omega_0^2 - 1/\tau^2}$",
            T::s(12),
        );
    });
    Ok(f)
}
