//! Canvas view: zoom and pan.

use rustfig_geom::{Affine, Point, Rect};

/// Screen pixels per Fig unit at zoom 1 (80 pixels per inch, like xfig).
pub const BASE_PX_PER_FIG: f64 = 80.0 / 1200.0;
pub const MIN_ZOOM: f64 = 0.01;
pub const MAX_ZOOM: f64 = 500.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Zoom factor (1 = 80 px per inch).
    pub zoom: f64,
    /// Fig coordinates shown at the top-left of the canvas.
    pub origin: Point,
    /// Canvas size in pixels (set by the UI).
    pub size: (f64, f64),
    /// xfig's "Integer zoom" (`integral_zoom`): zoom values above 1 stay whole
    /// numbers when set from the zoom pop-up or by Fit.
    pub integer_zoom: bool,
}

impl Default for View {
    fn default() -> Self {
        View {
            zoom: 1.0,
            origin: Point::new(-600.0, -600.0),
            size: (1000.0, 700.0),
            integer_zoom: false,
        }
    }
}

impl View {
    pub fn px_per_fig(&self) -> f64 {
        self.zoom * BASE_PX_PER_FIG
    }

    /// Fig → canvas pixels.
    pub fn transform(&self) -> Affine {
        let s = self.px_per_fig();
        Affine::translate(-self.origin.x, -self.origin.y).then(&Affine::scale(s, s))
    }

    pub fn to_screen(&self, p: Point) -> Point {
        self.transform().apply(p)
    }

    pub fn to_fig(&self, p: Point) -> Point {
        let s = self.px_per_fig();
        Point::new(self.origin.x + p.x / s, self.origin.y + p.y / s)
    }

    /// Visible region in Fig units.
    pub fn visible(&self) -> Rect {
        Rect::new(
            self.to_fig(Point::ZERO),
            self.to_fig(Point::new(self.size.0, self.size.1)),
        )
    }

    /// Zoom to `zoom`, keeping the screen point `anchor` fixed.
    pub fn zoom_about(&mut self, zoom: f64, anchor: Point) {
        if !zoom.is_finite() || !anchor.is_finite() {
            return;
        }
        let fixed = self.to_fig(anchor);
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        let s = self.px_per_fig();
        self.origin = Point::new(fixed.x - anchor.x / s, fixed.y - anchor.y / s);
    }

    /// One Zoom In (`up`) or Zoom Out step as xfig's `inc_zoom`/`dec_zoom`
    /// followed by `show_zoom`'s rounding (Ported from xfig w_indpanel.c):
    /// 1, 1.41, 2, 2.83, 4 ... with the same corrections of rounding errors.
    pub fn zoom_step(zoom: f64, up: bool) -> f64 {
        const STEP: f32 = std::f32::consts::SQRT_2; // xfig's ZOOM_STEP 1.41421356f
        let z = zoom as f32;
        if !z.is_finite() {
            return zoom;
        }
        let n = if up {
            if z >= MAX_ZOOM as f32 {
                z
            } else if z == 1.41 {
                2.0
            } else if z == 45.3 {
                64.0
            } else if z == 0.35 {
                0.5
            } else if z == 0.06 {
                0.09
            } else if z == 0.01 {
                0.02
            } else {
                z * STEP
            }
        } else if z <= MIN_ZOOM as f32 {
            z
        } else if z == 11.3 {
            8.0
        } else {
            z / STEP
        };
        let n = n.clamp(MIN_ZOOM as f32, MAX_ZOOM as f32);
        let r = if n < 10.0 {
            (n * 100.0 + 0.5).trunc() / 100.0
        } else if n < 100.0 {
            (n * 10.0 + 0.5).trunc() / 10.0
        } else {
            (n + 0.5).trunc()
        };
        f64::from(r)
    }

    /// Fit `r` into the canvas with a small border.
    pub fn fit(&mut self, r: &Rect) {
        if r.is_empty() {
            return;
        }
        let (w, h) = (self.size.0.max(10.0), self.size.1.max(10.0));
        let rw = r.width().max(100.0);
        let rh = r.height().max(100.0);
        let zoom = ((w * 0.9) / (rw * BASE_PX_PER_FIG)).min((h * 0.9) / (rh * BASE_PX_PER_FIG));
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        let s = self.px_per_fig();
        let c = r.center();
        self.origin = Point::new(c.x - w / 2.0 / s, c.y - h / 2.0 / s);
    }

    /// With Integer zoom on, make the zoom a whole number about the canvas
    /// centre: rounded above 1.99 after the pop-up sets it, truncated above 1
    /// after Fit (xfig's `show_zoom` and `zoom_to_fit`).
    pub fn apply_integer_zoom(&mut self, truncate: bool) {
        if !self.integer_zoom || !self.zoom.is_finite() {
            return;
        }
        let z = self.zoom;
        let n = if truncate {
            if z > 1.0 {
                z.trunc()
            } else {
                z
            }
        } else if z > 1.99 {
            z.round()
        } else {
            z
        };
        if n != z {
            let c = Point::new(self.size.0 / 2.0, self.size.1 / 2.0);
            self.zoom_about(n, c);
        }
    }

    /// Pan by a screen-pixel delta.
    pub fn pan_pixels(&mut self, dx: f64, dy: f64) {
        if !(dx.is_finite() && dy.is_finite()) {
            return;
        }
        let s = self.px_per_fig();
        self.origin = Point::new(self.origin.x - dx / s, self.origin.y - dy / s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_zoom() {
        let mut v = View::default();
        let p = Point::new(1234.0, -55.0);
        assert!(v.to_fig(v.to_screen(p)).dist(p) < 1e-6);
        let anchor = Point::new(300.0, 200.0);
        let before = v.to_fig(anchor);
        v.zoom_about(4.0, anchor);
        assert!(v.to_fig(anchor).dist(before) < 1e-6);
        v.zoom_about(f64::NAN, anchor);
        assert_eq!(v.zoom, 4.0);
    }

    #[test]
    fn zoom_steps_follow_xfig() {
        let mut z = 1.0;
        let mut up = vec![];
        for _ in 0..7 {
            z = View::zoom_step(z, true);
            up.push(z);
        }
        for (got, want) in up.iter().zip([1.41, 2.0, 2.83, 4.0, 5.66, 8.0, 11.3]) {
            assert!((got - want).abs() < 0.011, "{got} vs {want}");
        }
        assert_eq!(View::zoom_step(1.41, true), 2.0);
        assert_eq!(View::zoom_step(11.3, false), 8.0);
        assert!((View::zoom_step(1.0, false) - 0.71).abs() < 1e-6);
        assert_eq!(View::zoom_step(MAX_ZOOM, true), MAX_ZOOM);
    }
}

/// How rulers and the units box show lengths (xfig's Unit/Scale panel,
/// `w_rulers.c`): inches as fractions or decimals, and an optional figure
/// unit with a scale factor ("1 cm = 2.00 m").
#[derive(Debug, Clone, PartialEq)]
pub struct UnitScale {
    /// Imperial rulers in tenths rather than sixteenths of an inch.
    pub decimal_inches: bool,
    /// Figure units per ruler unit.
    pub factor: f64,
    /// Name of the figure unit; empty means the ruler unit.
    pub unit: String,
}

impl Default for UnitScale {
    fn default() -> Self {
        UnitScale {
            decimal_inches: false,
            factor: 1.0,
            unit: String::new(),
        }
    }
}

/// Longest figure unit name accepted.
pub const MAX_UNIT_NAME: usize = 16;

// ---- length strings (w_msgpanel.c `make_dimension_string`) -----------------

/// A length in Fig units as xfig's `make_dimension_string` writes it: scaled
/// by the unit/scale setting, in the figure unit name (or the ruler's),
/// with `precision` decimals. Inch drawings show whole numbers and fractions
/// of 1/64 inch ("1-1/2 in", "2 ft 3-1/2 in") unless the unit/scale setting
/// asks for decimal inches.
pub fn length_string(
    length: f64,
    units: rustfig_model::Units,
    scale: &UnitScale,
    ruler: &str,
    precision: u32,
) -> String {
    let (per_unit, inches) = match units {
        rustfig_model::Units::Inches => (1200.0, true),
        rustfig_model::Units::Metric => (450.0, false),
    };
    let factor = if scale.factor.is_finite() && scale.factor > 0.0 {
        scale.factor
    } else {
        1.0
    };
    let unit = if scale.unit.is_empty() {
        ruler
    } else {
        scale.unit.as_str()
    };
    let ulen = length / per_unit * factor;
    if inches && !scale.decimal_inches && ulen.is_finite() && ulen.abs() < 1e12 {
        inch_fraction(ulen, unit, precision as usize)
    } else {
        format!("{:.*} {unit}", precision as usize, ulen)
    }
}

/// Inches as a whole number and a fraction of 1/64, with feet first when
/// the unit is feet (the fractional branch of `make_dimension_string`).
fn inch_fraction(mut ulen: f64, unit: &str, prec: usize) -> String {
    let mut ilen = ulen as i64;
    if ilen as f64 == ulen {
        return format!("{ilen} {unit}");
    }
    let mut tmp = String::new();
    let mut units = unit;
    let mut ifeet = 0_i64;
    if matches!(unit, "f" | "ft" | "feet") {
        ifeet = ulen as i64;
        if ifeet.abs() >= 1 {
            tmp = format!("{ifeet} {unit} ");
        }
        ulen = (ulen - ifeet as f64) * 12.0;
        ilen = ulen as i64;
        units = "in";
    }
    let mut ifract = ((ulen - ilen as f64) * 64.0) as i64;
    if ifract == 0 {
        if ifeet < 0 {
            ilen = ilen.abs();
        }
        return format!("{tmp}{ilen} {units}");
    }
    if ilen as f64 + ifract as f64 / 64.0 == ulen {
        let mut iexp = 64_i64;
        while iexp >= 2 && ifract % 2 == 0 {
            ifract /= 2;
            iexp /= 2;
        }
        format!("{tmp}{}-{}/{iexp} {units}", ilen.abs(), ifract.abs())
    } else {
        format!("{tmp}{ulen:.prec$} {units}")
    }
}

#[cfg(test)]
mod length_tests {
    use super::*;
    use rustfig_model::Units;

    fn inch(len_inch: f64, unit: &str) -> String {
        let sc = UnitScale {
            unit: unit.into(),
            ..UnitScale::default()
        };
        length_string(len_inch * 1200.0, Units::Inches, &sc, "in", 1)
    }

    #[test]
    fn fractions_of_an_inch() {
        assert_eq!(inch(2.0, ""), "2 in");
        assert_eq!(inch(1.5, ""), "1-1/2 in");
        assert_eq!(inch(3.0 / 64.0, ""), "0-3/64 in");
        assert_eq!(inch(1.75, ""), "1-3/4 in");
    }

    #[test]
    fn feet_and_fractions() {
        // As in xfig, a fraction that does not reproduce the value exactly
        // (2 ft 3-1/2 in) falls back to decimals; 2 ft 6-3/4 in is exact.
        assert_eq!(inch(2.5625, "ft"), "2 ft 6-3/4 in");
        assert_eq!(inch(2.25, "ft"), "2 ft 3 in");
        assert_eq!(inch(2.0, "ft"), "2 ft");
    }

    #[test]
    fn decimal_fallbacks() {
        assert_eq!(inch(1.1, ""), "1.1 in");
        let sc = UnitScale {
            decimal_inches: true,
            ..UnitScale::default()
        };
        assert_eq!(length_string(1800.0, Units::Inches, &sc, "in", 1), "1.5 in");
        assert_eq!(
            length_string(450.0, Units::Metric, &UnitScale::default(), "cm", 1),
            "1.0 cm"
        );
    }
}
