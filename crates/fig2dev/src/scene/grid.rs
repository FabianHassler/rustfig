//! The `-G` grid of the PostScript, PDF and bitmap languages, as a list of
//! scene primitives drawn before the objects.
//
// Ported from fig2dev dev/genps.c (`genps_grid`, `draw_gridline`, with
// C's int truncation and float steps) (xfig licence, see LICENSE-xfig).

use super::{Prim, Rgba, Stroke, SubPath};
use figbase::geom::Point;
use figbase::style::{CapStyle, JoinStyle};

/// Lines per direction beyond which a grid is refused: a spacing far below
/// the figure's size would write millions of lines (or never advance).
pub const MAX_GRID_LINES: f64 = 20_000.0;

/// Where and how the grid is drawn. Lengths in file units (the figure's
/// `ppi` per inch), as in `genps_grid`.
#[derive(Debug, Clone, Copy)]
pub struct GridSpec {
    pub major: f32,
    pub minor: f32,
    /// The grid's extent.
    pub lx: f32,
    pub ly: f32,
    pub ux: f32,
    pub uy: f32,
    /// `THICK_SCALE`: the thin line is `thick_scale` (drawn half as wide),
    /// the thick one 2.5 times that (drawn `thick_scale` less).
    pub thick_scale: f32,
    /// Fig units (1200 per inch) per file unit.
    pub k: f64,
    pub metric: bool,
    /// Thinnest line drawn, in Fig units: one pixel for the unsmoothed
    /// bitmaps (ghostscript draws no thinner line there), else 0.
    pub min_width: f64,
}

/// `genps_grid`'s lines, as paths; `Ok(None)` without a grid.
pub fn grid_prims(g: &GridSpec) -> Result<Option<Vec<Prim>>, String> {
    let (major, minor) = (g.major, g.minor);
    if (minor == 0.0 && major == 0.0) || !(minor >= 0.0 && major >= 0.0) {
        return Ok(None);
    }
    let m = if minor == 0.0 { major } else { minor };
    let finite = [g.lx, g.ly, g.ux, g.uy, m, major, g.thick_scale]
        .iter()
        .all(|v| v.is_finite());
    if !finite || g.k <= 0.0 || !g.k.is_finite() {
        return Ok(None);
    }
    let n = (f64::from(g.ux) - f64::from(g.lx)).max(f64::from(g.uy) - f64::from(g.ly)) / f64::from(m);
    if n > MAX_GRID_LINES {
        return Err("grid spacing is too small for the figure; grid ignored".into());
    }
    let thin = f64::from(g.thick_scale);
    let thick = f64::from(g.thick_scale) * 2.5;
    // set_linewidth: `slw` of half the width, or the width less THICK_SCALE.
    let slw = |w: f64| {
        if w <= f64::from(g.thick_scale) {
            0.5 * w
        } else {
            w - f64::from(g.thick_scale)
        }
    };
    // In metric mode the grid is drawn under `450 472 div dup scale`.
    let f = if g.metric { 450.0 / 472.0 } else { 1.0 };
    let stroke = |w: f64| Stroke {
        width: (slw(w) * f * g.k).max(g.min_width.max(0.0)),
        color: Rgba {
            r: 128,
            g: 128,
            b: 128,
            a: 255,
        },
        dash: Vec::new(),
        cap: CapStyle::Butt,
        join: JoinStyle::Miter,
    };
    // draw_gridline prints "%.1f".
    let r1 = |v: f32| f64::from(format!("{v:.1}").parse::<f32>().unwrap_or(v));
    let line = |x1: f32, y1: f32, x2: f32, y2: f32| SubPath {
        pts: vec![
            Point::new(r1(x1) * f * g.k, r1(y1) * f * g.k),
            Point::new(r1(x2) * f * g.k, r1(y2) * f * g.k),
        ],
        closed: false,
        curve: None,
    };
    let (mut thin_lines, mut thick_lines) = (Vec::new(), Vec::new());
    // The C loops walk in float steps; the number of lines is capped.
    for vertical in [true, false] {
        let (start, end) = if vertical { (g.lx, g.ux) } else { (g.ly, g.uy) };
        let mut x = start;
        let mut count = 0.0;
        while x <= end && count <= MAX_GRID_LINES {
            count += 1.0;
            let mut is_thick = false;
            if major > 0.0 {
                // `itick` and `ntick` are ints in C.
                let itick = ((x / major) as i32) as f32 * major;
                let itick = itick as i32;
                if itick as f32 == x {
                    is_thick = true;
                } else {
                    let ntick = ((((x + minor) / major) as i32) as f32 * major) as i32;
                    if (ntick as f32) < x + minor {
                        let t = ntick as f32;
                        let s = if vertical {
                            line(t, g.ly, t, g.uy)
                        } else {
                            line(g.lx, t, g.ux, t)
                        };
                        thick_lines.push(s);
                    }
                }
            }
            let s = if vertical {
                line(x, g.ly, x, g.uy)
            } else {
                line(g.lx, x, g.ux, x)
            };
            if is_thick {
                thick_lines.push(s);
            } else {
                thin_lines.push(s);
            }
            let next = x + m;
            if next <= x {
                break;
            }
            x = next;
        }
    }
    let mut prims = Vec::new();
    for (lines, w) in [(thin_lines, thin), (thick_lines, thick)] {
        if !lines.is_empty() {
            prims.push(Prim::Path {
                subpaths: lines,
                fill: None,
                stroke: Some(stroke(w)),
            });
        }
    }
    Ok(Some(prims))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(major: f32, minor: f32) -> GridSpec {
        GridSpec {
            major,
            minor,
            lx: 0.0,
            ly: 0.0,
            ux: 2400.0,
            uy: 1200.0,
            thick_scale: 15.0,
            k: 1.0,
            metric: false,
            min_width: 0.0,
        }
    }

    fn count(p: &[Prim]) -> usize {
        p.iter()
            .map(|p| match p {
                Prim::Path { subpaths, .. } => subpaths.len(),
                _ => 0,
            })
            .sum()
    }

    #[test]
    fn none_without_spacing() {
        assert!(grid_prims(&spec(0.0, 0.0)).unwrap().is_none());
    }

    #[test]
    fn lines_at_each_tick() {
        let p = grid_prims(&spec(1200.0, 300.0)).unwrap().unwrap();
        // 9 vertical (0..=2400) and 5 horizontal, plus the major line C
        // redraws at every minor tick below the next major (6 times)
        assert_eq!(count(&p), 20);
    }

    #[test]
    fn tiny_spacing_is_refused() {
        assert!(grid_prims(&spec(0.0, 0.001)).is_err());
        assert!(grid_prims(&spec(f32::NAN, 1.0)).unwrap().is_none());
    }
}
