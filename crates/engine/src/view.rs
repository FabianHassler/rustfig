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
