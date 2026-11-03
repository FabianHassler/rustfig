//! The few Python behaviors the figure generators rely on, so that the Rust
//! port writes the very same bytes as the scripts it replaces: `round()` rounds
//! half to even, `%d` of a float truncates, complex arithmetic follows
//! CPython's formulas (including the sign of zero), `colorsys.hsv_to_rgb`, and
//! `random.Random(seed).gauss`.

/// Python's `round(x)` for a float: half to even.
pub fn round_i(x: f64) -> i64 {
    x.round_ties_even() as i64
}

/// Python's `"%d" % x` for a float: truncation towards zero.
pub fn trunc_i(x: f64) -> i64 {
    x.trunc() as i64
}

/// Python's `"%g" % v` for the plain values the figures print (a handful of
/// digits, magnitude between 1e-4 and 1e6): six significant digits, trailing
/// zeros removed.
pub fn fmt_g(v: f64) -> String {
    if v == v.trunc() {
        return format!("{}", v as i64);
    }
    let digits = 5 - v.abs().log10().floor() as i32;
    let s = format!("{:.*}", digits.max(0) as usize, v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// `colorsys.hsv_to_rgb`.
pub fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (f64, f64, f64) {
    if s == 0.0 {
        return (v, v, v);
    }
    let i = (h * 6.0).trunc();
    let f = (h * 6.0) - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match (i as i64).rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

/// A Python `complex`, with CPython's formulas for the operations the
/// generators use (a float operand is first widened to `x + 0j`, as Python does).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const ZERO: Complex = Complex { re: 0.0, im: 0.0 };

    pub fn new(re: f64, im: f64) -> Complex {
        Complex { re, im }
    }

    pub fn real(re: f64) -> Complex {
        Complex { re, im: 0.0 }
    }

    pub fn add(self, o: Complex) -> Complex {
        Complex::new(self.re + o.re, self.im + o.im)
    }

    pub fn sub(self, o: Complex) -> Complex {
        Complex::new(self.re - o.re, self.im - o.im)
    }

    pub fn mul(self, o: Complex) -> Complex {
        Complex::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }

    /// `self / o` (CPython's `_Py_c_quot`); a zero divisor gives zero (Python
    /// raises, which the generators never reach).
    pub fn div(self, o: Complex) -> Complex {
        let (abs_re, abs_im) = (o.re.abs(), o.im.abs());
        if abs_re >= abs_im {
            if abs_re == 0.0 {
                return Complex::ZERO;
            }
            let ratio = o.im / o.re;
            let denom = o.re + o.im * ratio;
            Complex::new((self.re + self.im * ratio) / denom, (self.im - self.re * ratio) / denom)
        } else if abs_im >= abs_re {
            let ratio = o.re / o.im;
            let denom = o.re * ratio + o.im;
            Complex::new((self.re * ratio + self.im) / denom, (self.im * ratio - self.re) / denom)
        } else {
            Complex::new(f64::NAN, f64::NAN)
        }
    }

    /// `abs(z)`.
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }

    /// `z ** 0.5` (CPython's `_Py_c_pow` for a real, non-integer exponent).
    pub fn sqrt_pow(self) -> Complex {
        if self.re == 0.0 && self.im == 0.0 {
            return Complex::ZERO;
        }
        let len = self.abs().powf(0.5);
        let phase = self.im.atan2(self.re) * 0.5;
        Complex::new(len * phase.cos(), len * phase.sin())
    }
}

/// `random.Random(seed)` (MT19937, seeded the way CPython seeds from an int).
pub struct PyRandom {
    mt: [u32; 624],
    idx: usize,
    gauss_next: Option<f64>,
}

impl PyRandom {
    pub fn new(seed: u32) -> PyRandom {
        let mut r = PyRandom {
            mt: [0; 624],
            idx: 624,
            gauss_next: None,
        };
        r.init_genrand(19_650_218);
        // init_by_array with the one-word key [seed]
        let key = [seed];
        let (mut i, mut j) = (1usize, 0usize);
        for _ in 0..624usize.max(key.len()) {
            let prev = r.mt[i - 1];
            r.mt[i] = (r.mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key.get(j).copied().unwrap_or(0))
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= 624 {
                r.mt[0] = r.mt[623];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..623 {
            let prev = r.mt[i - 1];
            r.mt[i] = (r.mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1_566_083_941)).wrapping_sub(i as u32);
            i += 1;
            if i >= 624 {
                r.mt[0] = r.mt[623];
                i = 1;
            }
        }
        r.mt[0] = 0x8000_0000;
        r
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..624 {
            let prev = self.mt[i - 1];
            self.mt[i] = 1_812_433_253u32
                .wrapping_mul(prev ^ (prev >> 30))
                .wrapping_add(i as u32);
        }
    }

    fn next_u32(&mut self) -> u32 {
        if self.idx >= 624 {
            for k in 0..624 {
                let y = (self.mt[k] & 0x8000_0000) | (self.mt[(k + 1) % 624] & 0x7fff_ffff);
                let mut v = self.mt[(k + 397) % 624] ^ (y >> 1);
                if y & 1 != 0 {
                    v ^= 0x9908_b0df;
                }
                self.mt[k] = v;
            }
            self.idx = 0;
        }
        let mut y = self.mt.get(self.idx).copied().unwrap_or(0);
        self.idx += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// `random.random()`.
    pub fn random(&mut self) -> f64 {
        let a = (self.next_u32() >> 5) as f64;
        let b = (self.next_u32() >> 6) as f64;
        (a * 67_108_864.0 + b) / 9_007_199_254_740_992.0
    }

    /// `random.gauss(mu, sigma)`.
    pub fn gauss(&mut self, mu: f64, sigma: f64) -> f64 {
        let z = match self.gauss_next.take() {
            Some(z) => z,
            None => {
                let x2pi = self.random() * std::f64::consts::TAU;
                let g2rad = (-2.0 * (1.0 - self.random()).ln()).sqrt();
                self.gauss_next = Some(x2pi.sin() * g2rad);
                x2pi.cos() * g2rad
            }
        };
        mu + z * sigma
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_is_half_to_even() {
        assert_eq!(round_i(0.5), 0);
        assert_eq!(round_i(1.5), 2);
        assert_eq!(round_i(2.5), 2);
        assert_eq!(round_i(-0.5), 0);
        assert_eq!(trunc_i(-2.9), -2);
    }

    #[test]
    fn g_format() {
        assert_eq!(fmt_g(1.0), "1");
        assert_eq!(fmt_g(0.5), "0.5");
    }

    #[test]
    fn random_matches_python() {
        // random.Random(7).random() -> 0.32383276483316237
        let mut r = PyRandom::new(7);
        assert_eq!(r.random(), 0.32383276483316237);
    }
}
