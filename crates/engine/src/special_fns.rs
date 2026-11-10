//! Special functions for plot expressions: Bessel functions J, Y, I, K,
//! gamma, log-gamma, error functions, Heaviside.
//!
//! Own implementations in plain `f64` (no libm): power series where the terms
//! do not cancel, Miller's downward recurrence (Numerical Recipes, section 6.5;
//! Abramowitz & Stegun 9.1.89 for Y), Hankel asymptotic expansions (A&S 9.2.5)
//! for large arguments, Temme's/Steed's continued fraction for K (Numerical
//! Recipes, section 6.7), the Lanczos approximation for log-gamma (Numerical
//! Recipes, 3rd ed.) and a series and Lentz-free continued fraction for erf.
//! Every loop has a fixed cap, so no argument (NaN, infinite, huge) can make a
//! function loop for long or panic.

use std::f64::consts::PI;

const EULER: f64 = 0.577_215_664_901_532_9;
const MAX_ORDER: i64 = 1000;

/// The order of `jn(n, x)` and the like: `n` rounded; None when NaN or beyond
/// the supported range.
fn order(n: f64) -> Option<i64> {
    if !n.is_finite() {
        return None;
    }
    let r = n.round();
    if r.abs() > MAX_ORDER as f64 {
        return None;
    }
    Some(r as i64)
}

fn odd(n: i64) -> bool {
    n & 1 == 1
}

/// Heaviside step: 0 below zero, 1 above, 0.5 at zero, NaN stays NaN.
pub fn heaviside(x: f64) -> f64 {
    if x.is_nan() {
        x
    } else if x < 0.0 {
        0.0
    } else if x > 0.0 {
        1.0
    } else {
        0.5
    }
}

// ---- Bessel J and Y ------------------------------------------------------

/// Hankel's asymptotic series P and Q for order 0 or 1 (`mu` = 4 nu^2), large x.
fn hankel_pq(mu: f64, x: f64) -> (f64, f64) {
    let (mut p, mut q) = (1.0, 0.0);
    let mut term = 1.0f64;
    for k in 1..80u32 {
        let kf = f64::from(k);
        let next = term * (mu - (2.0 * kf - 1.0) * (2.0 * kf - 1.0)) / (8.0 * kf * x);
        if next.abs() > term.abs() && k > 2 {
            break; // the series starts to diverge
        }
        term = next;
        match k % 4 {
            1 => q += term,
            2 => p -= term,
            3 => q -= term,
            _ => p += term,
        }
        if term.abs() < 1e-17 {
            break;
        }
    }
    (p, q)
}

/// J_nu(x) and Y_nu(x) for nu 0 or 1, x > 30 (Hankel expansion).
fn hankel_jy(nu: u8, x: f64) -> (f64, f64) {
    let (mu, phi) = if nu == 0 { (0.0, 0.25 * PI) } else { (4.0, 0.75 * PI) };
    let (p, q) = hankel_pq(mu, x);
    let (sx, cx) = x.sin_cos();
    let (sp, cp) = phi.sin_cos();
    let cos_chi = cx * cp + sx * sp;
    let sin_chi = sx * cp - cx * sp;
    let f = (2.0 / (PI * x)).sqrt();
    (f * (p * cos_chi - q * sin_chi), f * (p * sin_chi + q * cos_chi))
}

/// J_0..J_m (and J_{m+1}) at 0 < x <= 30 or any x with order up to `nmax`, by
/// Miller's downward recurrence normalized with J0 + 2 sum J_2k = 1.
fn miller(x: f64, nmax: usize) -> Vec<f64> {
    let big = nmax.max(x.ceil() as usize).max(2);
    let m = 2 * ((big + (400.0 * big as f64).sqrt() as usize) / 2);
    let tox = 2.0 / x;
    // Stored from index m+1 down to 0, reversed at the end.
    let mut v: Vec<f64> = Vec::with_capacity(m + 2);
    v.push(0.0);
    v.push(1.0);
    for j in (1..=m).rev() {
        let n = v.len();
        let (a, b) = match (v.get(n - 1), v.get(n.wrapping_sub(2))) {
            (Some(a), Some(b)) => (*a, *b),
            _ => break,
        };
        let next = j as f64 * tox * a - b;
        v.push(next);
        if next.abs() > 1e250 {
            for e in v.iter_mut() {
                *e *= 1e-250;
            }
        }
    }
    v.reverse();
    // v[i] = J_i up to a common factor; normalize.
    let mut sum = v.first().copied().unwrap_or(0.0);
    let mut k = 2;
    while let Some(e) = v.get(k) {
        if k > m {
            break;
        }
        sum += 2.0 * e;
        k += 2;
    }
    if sum.is_finite() && sum != 0.0 {
        for e in v.iter_mut() {
            *e /= sum;
        }
    }
    v
}

/// J_n(x) for 0 <= x <= 5 (power series, all orders).
fn j_series(n: usize, x: f64) -> f64 {
    let h = 0.5 * x;
    let mut t = 1.0f64;
    for k in 1..=n {
        t *= h / k as f64;
    }
    let mut s = t;
    let q = -h * h;
    for k in 1..80usize {
        t *= q / (k as f64 * (n + k) as f64);
        s += t;
        if t.abs() <= 1e-18 * s.abs() {
            break;
        }
    }
    s
}

/// J_n(x) for x > 0 and n >= 0.
fn j_pos(n: usize, x: f64) -> f64 {
    if x <= 5.0 {
        j_series(n, x)
    } else if x > 30.0 && n as f64 <= x {
        let (j0, _) = hankel_jy(0, x);
        if n == 0 {
            return j0;
        }
        let (j1, _) = hankel_jy(1, x);
        let (mut a, mut b) = (j0, j1);
        for k in 1..n {
            let c = 2.0 * k as f64 / x * b - a;
            a = b;
            b = c;
        }
        b
    } else {
        miller(x, n).get(n).copied().unwrap_or(f64::NAN)
    }
}

/// Bessel function of the first kind J_n(x).
pub fn bessel_jn(n: f64, x: f64) -> f64 {
    let Some(n) = order(n) else { return f64::NAN };
    if x.is_nan() {
        return f64::NAN;
    }
    let neg = odd(n) && (n < 0) != (x < 0.0);
    let ax = x.abs();
    let v = if ax == 0.0 {
        if n == 0 {
            1.0
        } else {
            0.0
        }
    } else if ax.is_infinite() {
        0.0
    } else {
        j_pos(n.unsigned_abs() as usize, ax)
    };
    if neg {
        -v
    } else {
        v
    }
}

/// Y_0(x) and Y_1(x) for x > 0.
fn y01(x: f64) -> (f64, f64) {
    let two_pi = 2.0 / PI;
    if x <= 5.0 {
        // Power series with the logarithm (A&S 9.1.13 and 9.1.11).
        let h2 = 0.25 * x * x;
        let lg = (0.5 * x).ln() + EULER;
        let j0 = j_series(0, x);
        let j1 = j_series(1, x);
        // Y0
        let mut t = 1.0f64; // (x^2/4)^k / (k!)^2 times (-1)^k
        let mut hk = 0.0f64;
        let mut s0 = 0.0f64;
        // Y1: sum (-1)^k [psi(k+1)+psi(k+2)] (x/2)^(2k+1) / (k!(k+1)!)
        let mut u = 0.5 * x; // (x/2)^(2k+1)/(k!(k+1)!) times (-1)^k
        let mut s1 = u * (-2.0 * EULER + 1.0); // k = 0: psi(1)+psi(2) = -2g + 1
        let mut hk1 = 1.0f64; // H_{k+1}
        for k in 1..60usize {
            let kf = k as f64;
            hk += 1.0 / kf;
            t *= -h2 / (kf * kf);
            s0 -= hk * t; // (-1)^{k+1} H_k (..)
            u *= -h2 / (kf * (kf + 1.0));
            hk1 += 1.0 / (kf + 1.0);
            s1 += u * (-2.0 * EULER + hk + hk1);
            if t.abs() < 1e-19 && u.abs() < 1e-19 {
                break;
            }
        }
        let y0 = two_pi * (lg * j0 + s0);
        let y1 = -two_pi / x + two_pi * (0.5 * x).ln() * j1 - s1 / PI;
        (y0, y1)
    } else if x <= 30.0 {
        // Neumann series from Miller's J_k (A&S 9.1.89 and its derivative).
        let j = miller(x, 2);
        let lg = (0.5 * x).ln() + EULER;
        let get = |i: usize| j.get(i).copied().unwrap_or(0.0);
        let (mut a0, mut a1) = (0.0f64, 0.0f64);
        let mut k = 1usize;
        let mut sg = -1.0f64;
        while 2 * k + 1 < j.len() && 2 * k + 1 < 200 {
            let kf = k as f64;
            a0 += sg * get(2 * k) / kf;
            a1 += sg * (get(2 * k - 1) - get(2 * k + 1)) / kf;
            sg = -sg;
            k += 1;
        }
        (
            two_pi * (lg * get(0) - 2.0 * a0),
            two_pi * (lg * get(1) - get(0) / x + a1),
        )
    } else {
        let (_, y0) = hankel_jy(0, x);
        let (_, y1) = hankel_jy(1, x);
        (y0, y1)
    }
}

/// Bessel function of the second kind Y_n(x); NaN for x < 0, -inf at 0.
pub fn bessel_yn(n: f64, x: f64) -> f64 {
    let Some(n) = order(n) else { return f64::NAN };
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    let neg = n < 0 && odd(n);
    let v = if x == 0.0 {
        f64::NEG_INFINITY
    } else if x.is_infinite() {
        0.0
    } else {
        let (y0, y1) = y01(x);
        let m = n.unsigned_abs();
        if m == 0 {
            y0
        } else {
            let (mut a, mut b) = (y0, y1);
            for k in 1..m {
                let c = 2.0 * k as f64 / x * b - a;
                a = b;
                b = c;
                if !b.is_finite() {
                    b = f64::NEG_INFINITY;
                    break;
                }
            }
            b
        }
    };
    if neg {
        -v
    } else {
        v
    }
}

// ---- modified Bessel I and K ---------------------------------------------

/// I_n(x) for x >= 0 and n >= 0 (series with positive terms).
fn i_pos(n: usize, x: f64) -> f64 {
    if x > 720.0 {
        return f64::INFINITY;
    }
    let h = 0.5 * x;
    let mut t = 1.0f64;
    for k in 1..=n {
        t *= h / k as f64;
    }
    let mut s = t;
    let h2 = h * h;
    for k in 1..6000usize {
        t *= h2 / (k as f64 * (n + k) as f64);
        s += t;
        if t <= 1e-18 * s {
            break;
        }
    }
    s
}

/// Modified Bessel function of the first kind I_n(x).
pub fn bessel_in(n: f64, x: f64) -> f64 {
    let Some(n) = order(n) else { return f64::NAN };
    if x.is_nan() {
        return f64::NAN;
    }
    let neg = odd(n) && x < 0.0;
    let v = i_pos(n.unsigned_abs() as usize, x.abs());
    if neg {
        -v
    } else {
        v
    }
}

/// exp(x) K_0(x) and exp(x) K_1(x) for x > 0 (scaled, so large orders and
/// arguments do not underflow in the recurrence).
fn k01(x: f64) -> (f64, f64) {
    if x <= 2.0 {
        let h2 = 0.25 * x * x;
        let lg = (0.5 * x).ln();
        let i0 = i_pos(0, x);
        let i1 = i_pos(1, x);
        // K0 = -(ln(x/2)+g) I0 + sum H_k (x^2/4)^k/(k!)^2
        let mut t = 1.0f64;
        let mut hk = 0.0f64;
        let mut s0 = 0.0f64;
        // K1 = 1/x + ln(x/2) I1 - (x/4) sum [psi(k+1)+psi(k+2)] (x^2/4)^k/(k!(k+1)!)
        let mut u = 1.0f64;
        let mut hk1 = 1.0f64;
        let mut s1 = u * (-2.0 * EULER + 1.0);
        for k in 1..60usize {
            let kf = k as f64;
            hk += 1.0 / kf;
            t *= h2 / (kf * kf);
            s0 += hk * t;
            u *= h2 / (kf * (kf + 1.0));
            hk1 += 1.0 / (kf + 1.0);
            s1 += u * (-2.0 * EULER + hk + hk1);
            if t < 1e-19 && u.abs() < 1e-19 {
                break;
            }
        }
        let e = x.exp();
        (e * (-(lg + EULER) * i0 + s0), e * (1.0 / x + lg * i1 - 0.25 * x * s1))
    } else {
        // Steed's continued fraction CF2 for nu = 0 (Numerical Recipes 6.7).
        let a1 = 0.25f64;
        let mut b = 2.0 * (1.0 + x);
        let mut d = 1.0 / b;
        let mut h = d;
        let mut delh = d;
        let (mut q1, mut q2) = (0.0f64, 1.0f64);
        let mut q = a1;
        let mut c = a1;
        let mut a = -a1;
        let mut s = 1.0 + q * delh;
        for i in 2..10_000u32 {
            a -= 2.0 * f64::from(i - 1);
            c = -a * c / f64::from(i);
            let qnew = (q1 - b * q2) / a;
            q1 = q2;
            q2 = qnew;
            q += c * qnew;
            b += 2.0;
            d = 1.0 / (b + a * d);
            delh *= b * d - 1.0;
            h += delh;
            let dels = q * delh;
            s += dels;
            if (dels / s).abs() < 1e-16 {
                break;
            }
        }
        h *= a1;
        let k0 = (PI / (2.0 * x)).sqrt() / s;
        let k1 = k0 * (x + 0.5 - h) / x;
        (k0, k1)
    }
}

/// Modified Bessel function of the second kind K_n(x); NaN for x < 0, +inf at 0.
pub fn bessel_kn(n: f64, x: f64) -> f64 {
    let Some(n) = order(n) else { return f64::NAN };
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return f64::INFINITY;
    }
    if x.is_infinite() {
        return 0.0;
    }
    let (k0, k1) = k01(x);
    let m = n.unsigned_abs();
    if m == 0 {
        return k0 * (-x).exp();
    }
    let (mut a, mut b) = (k0, k1);
    let mut shift = 0.0f64; // the values carry a factor 1e200^shift
    for k in 1..m {
        let c = a + 2.0 * k as f64 / x * b;
        a = b;
        b = c;
        if b > 1e200 {
            a *= 1e-200;
            b *= 1e-200;
            shift += 1.0;
        }
        if !b.is_finite() {
            return f64::INFINITY;
        }
    }
    b * (shift * 200.0 * std::f64::consts::LN_10 - x).exp()
}

// ---- gamma ---------------------------------------------------------------

const LANCZOS: [f64; 14] = [
    57.156_235_665_862_92,
    -59.597_960_355_475_49,
    14.136_097_974_741_747,
    -0.491_913_816_097_620_2,
    0.339_946_499_848_118_9e-4,
    0.465_236_289_270_485_8e-4,
    -0.983_744_753_048_795_6e-4,
    0.158_088_703_224_912_5e-3,
    -0.210_264_441_724_104_9e-3,
    0.217_439_618_115_212_6e-3,
    -0.164_318_106_536_763_9e-3,
    0.844_182_239_838_527_4e-4,
    -0.261_908_384_015_814_1e-4,
    0.368_991_826_595_316_2e-5,
];

/// ln Gamma(x) for x > 0 (Lanczos, g = 671/128).
fn lgamma_pos(x: f64) -> f64 {
    if x == 1.0 || x == 2.0 {
        return 0.0;
    }
    if x > 1e305 {
        return f64::INFINITY;
    }
    let mut y = x;
    let mut tmp = x + 5.242_187_5;
    tmp = (x + 0.5) * tmp.ln() - tmp;
    let mut ser = 0.999_999_999_999_997_1;
    for c in LANCZOS {
        y += 1.0;
        ser += c / y;
    }
    tmp + (2.506_628_274_631_000_5 * ser / x).ln()
}

/// sin(pi x) without losing the argument to rounding.
fn sin_pi(x: f64) -> f64 {
    let r = x - 2.0 * (x / 2.0).floor(); // [0, 2)
    (PI * r).sin()
}

/// ln |Gamma(x)|.
pub fn lgamma(x: f64) -> f64 {
    if x.is_nan() {
        x
    } else if x > 0.0 {
        lgamma_pos(x)
    } else if x == x.floor() {
        f64::INFINITY
    } else if x < -1e15 {
        f64::NAN // sin(pi x) is meaningless
    } else {
        (PI / sin_pi(x).abs()).ln() - lgamma_pos(1.0 - x)
    }
}

/// Gamma(x); NaN at 0 and the negative integers.
pub fn gamma(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x == f64::INFINITY {
        return x;
    }
    if x.is_infinite() {
        return f64::NAN;
    }
    if x == x.floor() {
        if x <= 0.0 {
            return f64::NAN;
        }
        if x <= 171.0 {
            let mut p = 1.0f64;
            let mut k = 2.0f64;
            while k < x {
                p *= k;
                k += 1.0;
            }
            return p;
        }
        return f64::INFINITY;
    }
    if x > 171.7 {
        return f64::INFINITY;
    }
    if x > 0.0 {
        return lgamma_pos(x).exp();
    }
    if x < -1e15 {
        return f64::NAN;
    }
    let s = sin_pi(x);
    let g = lgamma_pos(1.0 - x).exp(); // may be inf: result 0
    PI / (s * g)
}

// ---- error functions -----------------------------------------------------

/// erfc(x) for x >= 2 (continued fraction).
fn erfc_cf(x: f64) -> f64 {
    let mut t = x;
    for k in (1..=120u32).rev() {
        t = x + 0.5 * f64::from(k) / t;
    }
    (-x * x).exp() / (PI.sqrt() * t)
}

/// erf(x) for 0 <= x < 2 (series with positive terms).
fn erf_series(x: f64) -> f64 {
    let x2 = x * x;
    let mut term = x;
    let mut s = x;
    for n in 1..200u32 {
        term *= 2.0 * x2 / f64::from(2 * n + 1);
        s += term;
        if term < 1e-18 * s {
            break;
        }
    }
    2.0 / PI.sqrt() * (-x2).exp() * s
}

/// The error function.
pub fn erf(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    let a = x.abs();
    let v = if a < 2.0 { erf_series(a) } else { 1.0 - erfc_cf(a) };
    if x < 0.0 {
        -v
    } else {
        v
    }
}

/// The complementary error function 1 - erf(x).
pub fn erfc(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x < 0.0 {
        return 2.0 - erfc(-x);
    }
    if x < 2.0 {
        1.0 - erf_series(x)
    } else {
        erfc_cf(x)
    }
}

// ---- one-argument wrappers for the expression tables ----------------------

pub fn j0(x: f64) -> f64 {
    bessel_jn(0.0, x)
}
pub fn j1(x: f64) -> f64 {
    bessel_jn(1.0, x)
}
pub fn y0(x: f64) -> f64 {
    bessel_yn(0.0, x)
}
pub fn y1(x: f64) -> f64 {
    bessel_yn(1.0, x)
}
pub fn i0(x: f64) -> f64 {
    bessel_in(0.0, x)
}
pub fn i1(x: f64) -> f64 {
    bessel_in(1.0, x)
}
pub fn k0(x: f64) -> f64 {
    bessel_kn(0.0, x)
}
pub fn k1(x: f64) -> f64 {
    bessel_kn(1.0, x)
}
