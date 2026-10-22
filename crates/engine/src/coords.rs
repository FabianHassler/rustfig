//! Typed coordinates for the next point: xfig's keyboard point entry.
//
// Ported from xfig src/w_keyboard.c (`handle_keyboard_input`, its grammar
// and conversions; xfig licence, see LICENSE-xfig).

use rustfig_geom::Point;
use rustfig_model::Units;

/// A cursor over the typed text.
struct Cur<'a> {
    s: &'a [u8],
    i: usize,
}

impl Cur<'_> {
    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn digits(&mut self) -> Option<&str> {
        let start = self.i;
        while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
            self.i += 1;
        }
        (self.i > start).then(|| std::str::from_utf8(self.s.get(start..self.i)?).ok())?
    }

    /// A magnitude: `[+-]` then `int`, `int.frac`, `.frac` or `int-n/d`.
    fn mag(&mut self) -> Option<f64> {
        let mut sign = 1.0;
        match self.peek() {
            Some(b'-') => {
                sign = -1.0;
                self.i += 1;
            }
            Some(b'+') => self.i += 1,
            _ => {}
        }
        let int = self.digits().map(str::to_owned);
        let mut v = match &int {
            Some(d) => d.parse::<f64>().ok()?,
            None => 0.0,
        };
        match self.peek() {
            Some(b'.') => {
                self.i += 1;
                let f = self.digits()?.to_owned();
                v = format!("{}.{f}", int.as_deref().unwrap_or("0")).parse().ok()?;
            }
            // "1-1/2": a whole number and a fraction (xfig's INT_VALUE).
            Some(b'-') if int.is_some() => {
                let save = self.i;
                self.i += 1;
                let n = self.digits().map(str::to_owned);
                self.ws();
                if let (Some(n), Some(b'/')) = (n, self.peek()) {
                    self.i += 1;
                    self.ws();
                    let d = self.digits()?.to_owned();
                    let d = d.parse::<f64>().ok()?;
                    if d == 0.0 {
                        return None;
                    }
                    v += n.parse::<f64>().ok()? / d;
                } else {
                    self.i = save;
                }
            }
            _ => {
                int.as_ref()?;
            }
        }
        Some(sign * v)
    }

    fn rel_abs(&mut self) -> Option<bool> {
        let r = match self.peek() {
            Some(b'r' | b'R') => Some(true),
            Some(b'a' | b'A') => Some(false),
            _ => None,
        };
        if r.is_some() {
            self.i += 1;
            self.ws();
        }
        r
    }
}

/// The point typed as text. `origin` is the current point (the last point of
/// a line, or where the operation started); relative and polar coordinates
/// need one. Rectangular: `x, y` with an optional `r` (relative) or `a`
/// (absolute) before each number; polar: `length<angle` with the angle in
/// degrees, or radians (`r`), or multiples of pi (`p`). Lengths are in the
/// ruler units (inches or cm), fractions as `1-1/2`.
pub fn parse_coordinate(text: &str, origin: Option<Point>, units: Units) -> Result<Point, String> {
    let ppu = match units {
        Units::Inches => 1200.0,
        Units::Metric => 450.0,
    };
    let mut c = Cur {
        s: text.trim().as_bytes(),
        i: 0,
    };
    let bad = || format!("cannot read {text:?} as a coordinate (x,y or length<angle)");

    // Polar: magnitude, '<', angle.
    let start = c.i;
    if let Some(mag) = c.mag() {
        c.ws();
        if c.peek() == Some(b'<') {
            c.i += 1;
            c.ws();
            let angle = c.mag().ok_or_else(bad)?;
            let k = match c.peek() {
                Some(b'r' | b'R') => {
                    c.i += 1;
                    1.0
                }
                Some(b'p' | b'P') => {
                    c.i += 1;
                    std::f64::consts::PI
                }
                Some(b'd' | b'D') => {
                    c.i += 1;
                    std::f64::consts::PI / 180.0
                }
                _ => std::f64::consts::PI / 180.0,
            };
            c.ws();
            if c.i != c.s.len() {
                return Err(bad());
            }
            let o = origin.ok_or("Polar coordinates must be relative to a current point.")?;
            let a = angle * k;
            return finite(Point::new(
                o.x + (ppu * mag * a.cos()).round(),
                o.y + (ppu * mag * a.sin()).round(),
            ));
        }
    }
    c.i = start;

    // Rectangular: [r|a] x [,] [r|a] y.
    c.ws();
    let xr = c.rel_abs();
    let x = c.mag().ok_or_else(bad)?;
    c.ws();
    if c.peek() == Some(b',') {
        c.i += 1;
    }
    c.ws();
    let yr = c.rel_abs();
    let y = c.mag().ok_or_else(bad)?;
    c.ws();
    if c.i != c.s.len() {
        return Err(bad());
    }
    let x_rel = xr.unwrap_or(false);
    // With only x marked, y follows it.
    let y_rel = yr.unwrap_or(xr.unwrap_or(false));
    if (x_rel || y_rel) && origin.is_none() {
        return Err("Relative coordinates require a current point.".into());
    }
    let o = origin.unwrap_or(Point::ZERO);
    finite(Point::new(
        (ppu * x).round() + if x_rel { o.x } else { 0.0 },
        (ppu * y).round() + if y_rel { o.y } else { 0.0 },
    ))
}

fn finite(p: Point) -> Result<Point, String> {
    if p.is_finite() && p.x.abs() <= 1.0e8 && p.y.abs() <= 1.0e8 {
        Ok(p)
    } else {
        Err("coordinate out of range".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const O: Option<Point> = Some(Point { x: 1000.0, y: 2000.0 });

    #[test]
    fn rectangular() {
        let p = parse_coordinate("1, 2", None, Units::Inches).unwrap();
        assert_eq!((p.x, p.y), (1200.0, 2400.0));
        let p = parse_coordinate("1.5 .5", None, Units::Inches).unwrap();
        assert_eq!((p.x, p.y), (1800.0, 600.0));
        let p = parse_coordinate("2,3", None, Units::Metric).unwrap();
        assert_eq!((p.x, p.y), (900.0, 1350.0));
    }

    #[test]
    fn fractions_and_relative() {
        let p = parse_coordinate("1-1/2, 0", None, Units::Inches).unwrap();
        assert_eq!(p.x, 1800.0);
        let p = parse_coordinate("r1, r-1", O, Units::Inches).unwrap();
        assert_eq!((p.x, p.y), (2200.0, 800.0));
        // x marked relative, y follows.
        let p = parse_coordinate("r1 1", O, Units::Inches).unwrap();
        assert_eq!((p.x, p.y), (2200.0, 3200.0));
        // Mixed.
        let p = parse_coordinate("a1, r1", O, Units::Inches).unwrap();
        assert_eq!((p.x, p.y), (1200.0, 3200.0));
        assert!(parse_coordinate("r1,1", None, Units::Inches).is_err());
    }

    #[test]
    fn polar() {
        let p = parse_coordinate("1<0", O, Units::Inches).unwrap();
        assert_eq!((p.x, p.y), (2200.0, 2000.0));
        let p = parse_coordinate("1 < 90", O, Units::Inches).unwrap();
        assert_eq!((p.x, p.y), (1000.0, 3200.0));
        let p = parse_coordinate("2<0.5p", O, Units::Inches).unwrap();
        assert_eq!((p.x, p.y), (1000.0, 4400.0));
        assert!(parse_coordinate("1<0", None, Units::Inches).is_err());
    }

    #[test]
    fn hostile_text() {
        for t in [
            "",
            "<",
            "1,",
            ",",
            "1<",
            "1e400,1",
            "a",
            "1/0,1-1/0",
            "99999999999999999999,1",
            "\u{0}",
        ] {
            let _ = parse_coordinate(t, O, Units::Inches);
        }
    }
}
