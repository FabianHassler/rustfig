//! xfig's mode panel icons redrawn as vectors (plan W9 b): each is a small
//! Fig drawing traced from xfig's bitmap (`w_icons.c`), rendered by
//! RustFig's rasteriser at the screen's density. Geometry (lines, curves,
//! ellipses, arcs) is smooth at any scale; tiny lettering and pictures that
//! are pixel art by nature are copied from the bitmap as one square per
//! pixel, so they stay crisp at whole-number scales and match xfig at 1×.
//
// Traced from xfig src/w_icons.c (xfig licence, see LICENSE-xfig).

use crate::xfig_icons::{self as xi, Xbm};
use rustfig_geom::Point;
use rustfig_model::{
    Arc, ColorRef, Ellipse, EllipseKind, LineKind, Object, ObjectKind, PolyKind, Polyline, Spline, SplineKind, Style,
};

/// Fig units per icon pixel.
pub const U: f64 = 15.0;

/// Pixel centre of pixel column `x`, row `y`, in Fig units.
fn c(x: f64, y: f64) -> Point {
    Point::new((x + 0.5) * U, (y + 0.5) * U)
}

fn pen(thickness: i32) -> Style {
    Style {
        line: LineKind::Solid,
        thickness,
        pen_color: ColorRef::BLACK,
        ..Style::default()
    }
}

/// One-pixel ink: thickness 2 is drawn 1/80 inch wide, one icon pixel.
const PX: i32 = 2;

/// A drawing in icon pixel coordinates.
struct Ic {
    bitmap: &'static Xbm,
    objs: Vec<Object>,
}

impl Ic {
    fn new(bitmap: &'static Xbm) -> Ic {
        Ic {
            bitmap,
            objs: Vec::new(),
        }
    }

    fn push(&mut self, k: ObjectKind) -> &mut Ic {
        self.objs.push(Object::new(k));
        self
    }

    fn poly(&mut self, kind: PolyKind, st: Style, pts: &[(f64, f64)]) -> &mut Ic {
        let pts = pts.iter().map(|&(x, y)| c(x, y)).collect();
        self.push(ObjectKind::Polyline(Polyline::new(kind, st, pts)))
    }

    /// An open polyline through pixel centres.
    fn line(&mut self, pts: &[(f64, f64)]) -> &mut Ic {
        self.poly(PolyKind::Polyline, pen(PX), pts)
    }

    /// A two-pixel-wide polyline (thickness 3).
    fn wide(&mut self, pts: &[(f64, f64)]) -> &mut Ic {
        self.poly(PolyKind::Polyline, pen(3), pts)
    }

    /// A dotted line through pixel centres: one pixel on, one off.
    fn dotted(&mut self, pts: &[(f64, f64)]) -> &mut Ic {
        let st = Style {
            line: LineKind::Dotted,
            style_val: 1.0,
            ..pen(PX)
        };
        self.poly(PolyKind::Polyline, st, pts)
    }

    fn polygon(&mut self, pts: &[(f64, f64)]) -> &mut Ic {
        self.poly(PolyKind::Polygon, pen(PX), pts)
    }

    /// A box through the centres of its corner pixels.
    fn rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) -> &mut Ic {
        self.poly(PolyKind::Box, pen(PX), &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
    }

    fn round_rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, radius: i32) -> &mut Ic {
        let pts = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| c(x, y)).to_vec();
        let mut b = Polyline::new(PolyKind::ArcBox, pen(PX), pts);
        b.radius = radius;
        self.push(ObjectKind::Polyline(b))
    }

    /// A filled square of `size` pixels centred on pixel (x, y): xfig's
    /// point markers.
    fn marker(&mut self, x: f64, y: f64, size: f64) -> &mut Ic {
        let h = size / 2.0;
        let st = Style {
            area_fill: 20,
            fill_color: ColorRef::BLACK,
            ..pen(0)
        };
        let (x, y) = (x + 0.5, y + 0.5);
        let pts = [(x - h, y - h), (x + h, y - h), (x + h, y + h), (x - h, y + h)]
            .map(|(x, y)| Point::new(x * U, y * U))
            .to_vec();
        self.push(ObjectKind::Polyline(Polyline::new(PolyKind::Box, st, pts)))
    }

    fn ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) -> &mut Ic {
        self.ellipse_styled(pen(PX), cx, cy, rx, ry)
    }

    /// A two-pixel-wide ellipse.
    fn wide_ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) -> &mut Ic {
        self.ellipse_styled(pen(3), cx, cy, rx, ry)
    }

    fn dotted_ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) -> &mut Ic {
        let st = Style {
            line: LineKind::Dotted,
            style_val: 1.0,
            ..pen(PX)
        };
        self.ellipse_styled(st, cx, cy, rx, ry)
    }

    fn dotted_rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) -> &mut Ic {
        let st = Style {
            line: LineKind::Dotted,
            style_val: 1.0,
            ..pen(PX)
        };
        self.poly(PolyKind::Box, st, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
    }

    /// A filled ellipse (black, or white to punch a hole into black).
    fn fill_ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, white: bool) -> &mut Ic {
        let st = Style {
            area_fill: 20,
            fill_color: if white { ColorRef::WHITE } else { ColorRef::BLACK },
            ..pen(0)
        };
        self.ellipse_styled(st, cx, cy, rx, ry)
    }

    /// A filled rectangle over pixels x0..=x1, y0..=y1 (black or white).
    fn fill_rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, white: bool) -> &mut Ic {
        let st = Style {
            area_fill: 20,
            fill_color: if white { ColorRef::WHITE } else { ColorRef::BLACK },
            ..pen(0)
        };
        let pts = [(x0, y0), (x1 + 1.0, y0), (x1 + 1.0, y1 + 1.0), (x0, y1 + 1.0)]
            .map(|(x, y)| Point::new(x * U, y * U))
            .to_vec();
        self.push(ObjectKind::Polyline(Polyline::new(PolyKind::Box, st, pts)))
    }

    fn ellipse_styled(&mut self, style: Style, cx: f64, cy: f64, rx: f64, ry: f64) -> &mut Ic {
        self.push(ObjectKind::Ellipse(Ellipse {
            kind: EllipseKind::EllipseRadii,
            style,
            direction: 1,
            angle: 0.0,
            center: c(cx, cy),
            radii: Point::new(rx * U, ry * U),
            start: c(cx, cy),
            end: c(cx + rx, cy),
        }))
    }

    fn arc(&mut self, a: (f64, f64), b: (f64, f64), e: (f64, f64)) -> &mut Ic {
        if let Some(arc) = Arc::through(pen(PX), c(a.0, a.1), c(b.0, b.1), c(e.0, e.1), false) {
            self.push(ObjectKind::Arc(arc));
        }
        self
    }

    /// A smooth curve through the points (an interpolated X-spline).
    fn curve(&mut self, closed: bool, pts: &[(f64, f64)]) -> &mut Ic {
        let pts = pts.iter().map(|&(x, y)| c(x, y)).collect();
        self.push(ObjectKind::Spline(Spline::new(
            SplineKind::Interpolated,
            closed,
            pen(PX),
            pts,
        )))
    }

    /// The bitmap's pixels in columns x0..=x1, rows y0..=y1, one filled
    /// square each: lettering and pixel art.
    fn pixels(&mut self, x0: usize, y0: usize, x1: usize, y1: usize) -> &mut Ic {
        let st = Style {
            area_fill: 20,
            fill_color: ColorRef::BLACK,
            ..pen(0)
        };
        let b = self.bitmap;
        for y in y0..=y1.min(b.height.saturating_sub(1)) {
            let mut x = x0;
            while x <= x1.min(b.width.saturating_sub(1)) {
                if !b.get(x, y) {
                    x += 1;
                    continue;
                }
                // One rectangle per run of set pixels in the row.
                let start = x;
                while x <= x1 && x < b.width && b.get(x, y) {
                    x += 1;
                }
                let (fx0, fx1, fy0, fy1) = (start as f64, x as f64, y as f64, y as f64 + 1.0);
                let pts = [(fx0, fy0), (fx1, fy0), (fx1, fy1), (fx0, fy1)]
                    .map(|(x, y)| Point::new(x * U, y * U))
                    .to_vec();
                self.objs
                    .push(Object::new(ObjectKind::Polyline(Polyline::new(PolyKind::Box, st, pts))));
            }
        }
        self
    }

    fn done(&mut self) -> Vec<Object> {
        std::mem::take(&mut self.objs)
    }
}

/// xfig's bitmaps traced so far (mode icons and attribute pictures); the
/// others keep their bitmap.
pub const TRACED: &[&Xbm] = &[
    &xi::ELLRAD,
    &xi::ELLDIA,
    &xi::CIRRAD,
    &xi::CIRDIA,
    &xi::C_SPL,
    &xi::SPL,
    &xi::C_INTSPL,
    &xi::INTSPL,
    &xi::POLYGON,
    &xi::LINE,
    &xi::BOX,
    &xi::ARC_BOX,
    &xi::REGPOLY,
    &xi::ARC,
    &xi::PICOBJ,
    &xi::TEXT,
    &xi::GLUE,
    &xi::BREAK,
    &xi::SCALE,
    &xi::ALIGN,
    &xi::MOVE,
    &xi::COPY,
    &xi::ADDPT,
    &xi::EDIT,
    &xi::FLIP_Y,
    &xi::CONVERT,
    &xi::AUTOARROW,
    &xi::TANGENT,
    &xi::LENMEAS,
    &xi::C_XSPL,
    &xi::XSPL,
    &xi::DELETE,
    &xi::UPDATE,
    &xi::MOVEPT,
    &xi::FLIP_X,
    &xi::ROTCW,
    &xi::ROTCCW,
    &xi::OPEN_COMP,
    &xi::JOIN_SPLIT,
    &xi::ANGLEMEAS,
    &xi::DELETEPT,
    &xi::CHOP,
    &xi::AREAMEAS,
    &xi::LIBRARY,
    &xi::NONE,
    &xi::ANY,
    &xi::FINE_GRID,
    &xi::GRID1,
    &xi::GRID2,
    &xi::GRID3,
    &xi::GRID4,
    &xi::VALIGNT,
    &xi::VALIGNC,
    &xi::VALIGNB,
    &xi::HALIGNL,
    &xi::HALIGNC,
    &xi::HALIGNR,
];

/// Mode icons RustFig draws itself: no xfig bitmap to trace, so a blank
/// bitmap carries the name and size.
pub const ORIGINAL: &[&Xbm] = &[&xi::TRIM, &xi::PLOT];

/// The traced bitmap called `name`.
pub fn bitmap(name: &str) -> Option<&'static Xbm> {
    TRACED.iter().chain(ORIGINAL).find(|b| b.name == name).copied()
}

/// Whether `name` has a vector drawing.
pub fn traced(name: &str) -> bool {
    bitmap(name).is_some()
}

/// Size in pixels of the traced bitmap `name` (36×32 for unknown names).
pub fn size(name: &str) -> (usize, usize) {
    bitmap(name).map_or((36, 32), |b| (b.width, b.height))
}

/// The figure eight of the closed spline icons (traced).
const EIGHT: &[(f64, f64)] = &[
    (18.0, 13.0),
    (22.5, 11.0),
    (24.0, 7.5),
    (21.0, 4.5),
    (16.0, 4.0),
    (10.5, 5.0),
    (8.0, 8.0),
    (10.0, 11.5),
    (18.0, 13.0),
    (25.5, 14.0),
    (29.0, 18.5),
    (27.5, 23.5),
    (21.0, 26.5),
    (14.0, 27.0),
    (8.0, 25.5),
    (5.0, 21.0),
    (6.5, 17.0),
    (11.0, 14.5),
];

/// The S of the open spline icons (traced).
const ESS: &[(f64, f64)] = &[
    (23.0, 6.0),
    (16.0, 4.0),
    (9.0, 5.5),
    (7.0, 9.0),
    (10.0, 12.0),
    (17.0, 13.0),
    (25.0, 14.0),
    (28.5, 18.0),
    (27.0, 22.5),
    (22.0, 25.0),
    (16.0, 26.0),
    (10.5, 24.5),
    (7.0, 20.0),
];

/// The sample objects inside the Glue and Break icons: an S, an ellipse,
/// a T, a box and an arc.
fn glue_contents(ic: &mut Ic) -> &mut Ic {
    ic.curve(
        false,
        &[
            (15.0, 9.0),
            (14.0, 7.5),
            (11.5, 7.0),
            (9.0, 7.8),
            (8.0, 9.8),
            (9.2, 11.2),
            (11.5, 12.0),
            (14.0, 13.2),
            (15.0, 14.8),
            (14.0, 16.4),
            (11.5, 17.0),
            (9.0, 16.4),
            (8.2, 15.0),
        ],
    )
    .ellipse(22.0, 9.0, 4.0, 3.0)
    .line(&[(17.0, 18.0), (23.0, 18.0)])
    .line(&[(20.0, 18.0), (20.0, 25.0)])
    .rect(8.0, 21.0, 15.0, 25.0)
    .arc((24.0, 15.0), (27.0, 20.0), (24.0, 25.0))
}

/// The rotate icons: a curved band ending in an arrowhead; `mirror` for
/// counter-clockwise (x mirrored about the icon's centre).
fn rotate(ic: &mut Ic, mirror: bool) -> &mut Ic {
    let m = |(x, y): (f64, f64)| if mirror { (36.0 - x, y) } else { (x, y) };
    ic.arc(m((19.0, 5.0)), m((11.0, 12.0)), m((24.0, 15.5)))
        .arc(m((19.0, 8.5)), m((14.0, 12.0)), m((21.0, 15.0)))
        .line(&[m((21.0, 12.0)), m((25.0, 12.0)), m((25.0, 14.0))])
        .polygon(&[m((19.0, 2.0)), m((23.0, 6.5)), m((19.0, 11.0))])
}

/// A grid picture: vertical lines at `xs` from `y.0` to `y.1`, horizontal
/// lines at `ys` from `x.0` to `x.1`.
fn grid(b: &'static Xbm, xs: &[f64], ys: &[f64], x: (f64, f64), y: (f64, f64)) -> Vec<Object> {
    let mut ic = Ic::new(b);
    for &gx in xs {
        ic.line(&[(gx, y.0), (gx, y.1)]);
    }
    for &gy in ys {
        ic.line(&[(x.0, gy), (x.1, gy)]);
    }
    ic.done()
}

/// The vector drawing of xfig's mode icon `name` (`Xbm::name`), if it has
/// been traced; others keep their bitmap.
pub fn drawing(name: &str) -> Option<Vec<Object>> {
    Some(match name {
        // Ellipse by radii: the ellipse, its centre cross and dotted radii.
        "ellrad" => Ic::new(&xi::ELLRAD)
            .ellipse(18.0, 15.0, 13.0, 9.0)
            .line(&[(16.0, 15.0), (20.0, 15.0)])
            .dotted(&[(22.0, 15.0), (30.0, 15.0)])
            .dotted(&[(18.0, 7.0), (18.0, 23.0)])
            .done(),
        "elldia" => Ic::new(&xi::ELLDIA)
            .ellipse(18.0, 15.0, 13.0, 9.0)
            .line(&[(16.0, 15.0), (20.0, 15.0)])
            .dotted(&[(6.0, 15.0), (14.0, 15.0)])
            .dotted(&[(22.0, 15.0), (30.0, 15.0)])
            .dotted(&[(18.0, 7.0), (18.0, 23.0)])
            .done(),
        "cirrad" => Ic::new(&xi::CIRRAD)
            .ellipse(18.0, 15.5, 13.0, 12.5)
            .line(&[(16.0, 15.0), (20.0, 15.0)])
            .line(&[(18.0, 13.0), (18.0, 17.0)])
            .dotted(&[(22.0, 15.0), (30.0, 15.0)])
            .done(),
        "cirdia" => Ic::new(&xi::CIRDIA)
            .ellipse(18.0, 15.5, 13.0, 12.5)
            .line(&[(16.0, 15.0), (20.0, 15.0)])
            .line(&[(18.0, 13.0), (18.0, 17.0)])
            .dotted(&[(6.0, 15.0), (14.0, 15.0)])
            .dotted(&[(22.0, 15.0), (30.0, 15.0)])
            .done(),
        // Splines: a figure eight (closed) or an S (open); the interpolated
        // ones show their control points.
        "c_spl" => Ic::new(&xi::C_SPL).curve(true, EIGHT).done(),
        "spl" => Ic::new(&xi::SPL).curve(false, ESS).done(),
        "c_intspl" => Ic::new(&xi::C_INTSPL)
            .curve(true, EIGHT)
            .marker(16.0, 4.0, 3.0)
            .marker(23.0, 7.0, 3.0)
            .marker(8.0, 8.0, 3.0)
            .marker(19.0, 13.0, 3.0)
            .marker(29.0, 19.0, 3.0)
            .marker(6.0, 21.0, 3.0)
            .marker(15.0, 27.0, 3.0)
            .done(),
        "intspl" => Ic::new(&xi::INTSPL)
            .curve(false, ESS)
            .marker(17.0, 4.0, 3.0)
            .marker(24.0, 6.0, 3.0)
            .marker(8.0, 9.0, 3.0)
            .marker(18.0, 13.0, 3.0)
            .marker(29.0, 19.0, 3.0)
            .marker(6.0, 21.0, 3.0)
            .marker(15.0, 26.0, 3.0)
            .done(),
        "polygon" => Ic::new(&xi::POLYGON)
            .polygon(&[
                (8.0, 4.0),
                (22.0, 4.0),
                (27.0, 9.0),
                (27.0, 13.0),
                (22.0, 18.0),
                (16.0, 18.0),
                (16.0, 27.0),
                (8.0, 27.0),
            ])
            .done(),
        "line" => Ic::new(&xi::LINE)
            .line(&[
                (5.0, 21.0),
                (13.0, 5.0),
                (18.0, 16.0),
                (11.0, 23.0),
                (18.5, 27.0),
                (24.0, 18.0),
                (26.0, 5.0),
                (30.0, 16.0),
            ])
            .done(),
        "box" => Ic::new(&xi::BOX).rect(7.0, 8.0, 28.0, 23.0).done(),
        "arc_box" => Ic::new(&xi::ARC_BOX).round_rect(6.0, 7.0, 29.0, 24.0, 4).done(),
        // A hexagon with its centre and the radius drawn to a corner.
        "regpoly" => Ic::new(&xi::REGPOLY)
            .polygon(&[
                (11.0, 3.0),
                (23.0, 3.0),
                (29.0, 14.0),
                (23.0, 25.0),
                (11.0, 25.0),
                (5.0, 14.0),
            ])
            .marker(17.0, 14.0, 3.0)
            .pixels(10, 10, 14, 18)
            .pixels(26, 21, 33, 29)
            .done(),
        // A three-point arc with its numbered points.
        "arc" => Ic::new(&xi::ARC)
            .arc((12.0, 4.0), (23.0, 15.0), (12.0, 26.0))
            .marker(12.0, 4.0, 3.0)
            .marker(23.0, 15.0, 3.0)
            .marker(12.0, 26.0, 3.0)
            .pixels(4, 2, 9, 10)
            .pixels(26, 11, 33, 19)
            .pixels(4, 20, 9, 28)
            .done(),
        // Glue: the sample objects in a compound's frame with its corners.
        "glue" => {
            let mut ic = Ic::new(&xi::GLUE);
            ic.rect(4.0, 4.0, 30.0, 28.0)
                .marker(4.0, 4.0, 3.0)
                .marker(30.0, 4.0, 3.0)
                .marker(4.0, 28.0, 3.0)
                .marker(30.0, 28.0, 3.0);
            glue_contents(&mut ic).done()
        }
        // Break: the same objects in a dotted frame.
        "break" => {
            let mut ic = Ic::new(&xi::BREAK);
            ic.dotted_rect(4.0, 4.0, 30.0, 28.0);
            glue_contents(&mut ic).done()
        }
        "scale" => Ic::new(&xi::SCALE)
            .polygon(&[(6.0, 6.0), (8.5, 11.0), (4.0, 11.0)])
            .polygon(&[(26.0, 3.0), (32.0, 16.0), (20.0, 16.0)])
            .line(&[(12.0, 8.0), (21.0, 8.0)])
            .line(&[(14.0, 6.0), (12.0, 8.0), (14.0, 10.0)])
            .line(&[(19.0, 6.0), (21.0, 8.0), (19.0, 10.0)])
            .pixels(3, 19, 33, 29)
            .done(),
        "align" => Ic::new(&xi::ALIGN)
            .polygon(&[(8.0, 8.0), (11.0, 15.0), (5.0, 15.0)])
            .rect(14.0, 9.0, 17.0, 15.0)
            .ellipse(26.0, 9.0, 6.0, 6.0)
            .dotted(&[(2.0, 16.0), (32.0, 16.0)])
            .pixels(3, 17, 33, 29)
            .done(),
        // Move: the dotted original, an arrow, the moved object.
        "move" => Ic::new(&xi::MOVE)
            .dotted_ellipse(7.5, 10.0, 3.5, 7.0)
            .line(&[(14.0, 11.0), (21.0, 11.0)])
            .line(&[(19.0, 9.0), (21.0, 11.0), (19.0, 13.0)])
            .wide_ellipse(27.0, 10.5, 3.5, 7.0)
            .pixels(5, 20, 31, 29)
            .done(),
        // Copy: the object, an arrow, its copy.
        "copy" => Ic::new(&xi::COPY)
            .wide_ellipse(8.5, 9.5, 3.5, 7.0)
            .line(&[(14.0, 10.0), (21.0, 10.0)])
            .line(&[(19.0, 8.0), (21.0, 10.0), (19.0, 12.0)])
            .wide_ellipse(27.5, 9.5, 3.5, 7.0)
            .pixels(5, 18, 30, 29)
            .done(),
        // Add point: a new point pushed into a line.
        "addpt" => Ic::new(&xi::ADDPT)
            .wide(&[(3.5, 21.5), (14.5, 10.5), (21.5, 10.5), (32.5, 21.5)])
            .poly(
                PolyKind::Box,
                pen(3),
                &[(15.5, 5.5), (19.5, 5.5), (19.5, 8.5), (15.5, 8.5)],
            )
            .line(&[(17.0, 15.0), (17.0, 22.0)])
            .line(&[(15.0, 16.0), (17.0, 14.0), (19.0, 16.0)])
            .dotted(&[(3.0, 25.0), (31.0, 25.0)])
            .done(),
        // Edit: an ellipse, a dotted pointer, the edit panel.
        "edit" => Ic::new(&xi::EDIT)
            .wide_ellipse(8.5, 8.0, 4.0, 3.5)
            .dotted(&[(13.0, 13.0), (21.0, 17.0)])
            .dotted(&[(13.0, 4.0), (22.0, 4.0)])
            .rect(23.0, 4.0, 31.0, 17.0)
            .pixels(24, 5, 30, 16)
            .pixels(7, 19, 30, 29)
            .done(),
        // Flip up/down: a triangle, its mirror image, the axis, a turning arrow.
        "flip_y" => Ic::new(&xi::FLIP_Y)
            .polygon(&[(14.0, 4.0), (18.0, 13.0), (10.0, 13.0)])
            .polygon(&[(10.0, 17.0), (18.0, 17.0), (14.0, 26.0)])
            .dotted(&[(4.0, 15.0), (26.0, 15.0)])
            .arc((26.0, 8.0), (31.0, 15.0), (26.0, 22.0))
            .pixels(24, 6, 29, 10)
            .pixels(24, 20, 29, 24)
            .done(),
        // Convert: a polyline and its spline, a box and its rounded form.
        "convert" => Ic::new(&xi::CONVERT)
            .curve(
                false,
                &[
                    (7.0, 3.0),
                    (4.0, 4.0),
                    (3.0, 6.0),
                    (4.0, 8.0),
                    (6.5, 9.5),
                    (8.0, 11.5),
                    (7.5, 13.5),
                    (4.0, 15.0),
                ],
            )
            .line(&[(31.0, 3.0), (26.0, 8.0), (30.0, 12.0), (27.0, 15.0)])
            .line(&[(10.0, 10.0), (24.0, 10.0)])
            .line(&[(12.0, 8.0), (10.0, 10.0), (12.0, 12.0)])
            .line(&[(22.0, 8.0), (24.0, 10.0), (22.0, 12.0)])
            .rect(3.0, 18.0, 14.0, 27.0)
            .round_rect(21.0, 18.0, 32.0, 27.0, 4)
            .line(&[(9.0, 22.0), (26.0, 22.0)])
            .line(&[(11.0, 20.0), (9.0, 22.0), (11.0, 24.0)])
            .line(&[(24.0, 20.0), (26.0, 22.0), (24.0, 24.0)])
            .done(),
        // Arrows: a line given an arrowhead.
        "autoarrow" => Ic::new(&xi::AUTOARROW)
            .line(&[(10.0, 4.0), (10.0, 27.0)])
            .polygon(&[(10.0, 6.0), (14.5, 18.0), (5.5, 18.0)])
            .pixels(17, 4, 33, 28)
            .done(),
        // Tangent: a curve, its tangent and normal at a point.
        "tangent" => Ic::new(&xi::TANGENT)
            .arc((13.0, 4.0), (22.5, 15.0), (13.0, 26.0))
            .line(&[(24.0, 3.0), (24.0, 28.0)])
            .line(&[(3.0, 15.0), (29.0, 15.0)])
            .marker(23.0, 15.0, 3.0)
            .pixels(4, 9, 8, 13)
            .pixels(25, 3, 31, 7)
            .done(),
        // Length: a curve over a ruler.
        "lenmeas" => Ic::new(&xi::LENMEAS)
            .pixels(2, 3, 33, 16)
            .line(&[(3.0, 26.0), (33.0, 26.0)])
            .line(&[(3.0, 23.0), (3.0, 26.0)])
            .line(&[(8.0, 23.0), (8.0, 26.0)])
            .line(&[(13.0, 23.0), (13.0, 26.0)])
            .line(&[(18.0, 23.0), (18.0, 26.0)])
            .line(&[(23.0, 23.0), (23.0, 26.0)])
            .line(&[(28.0, 23.0), (28.0, 26.0)])
            .line(&[(33.0, 23.0), (33.0, 26.0)])
            .done(),
        // X-splines: their control polygons, closed and open.
        "c_xspl" => Ic::new(&xi::C_XSPL)
            .polygon(&[
                (3.0, 3.0),
                (11.0, 3.0),
                (16.0, 12.0),
                (18.0, 12.0),
                (25.0, 3.0),
                (33.0, 3.0),
                (27.0, 12.0),
                (27.0, 17.0),
                (33.0, 27.0),
                (24.0, 27.0),
                (19.0, 18.0),
                (16.0, 18.0),
                (11.0, 27.0),
                (3.0, 27.0),
                (8.0, 17.0),
                (8.0, 12.0),
            ])
            .done(),
        "xspl" => Ic::new(&xi::XSPL)
            .line(&[(23.0, 5.0), (11.0, 5.0), (11.0, 13.0), (19.0, 13.0)])
            .curve(
                false,
                &[
                    (19.0, 13.0),
                    (22.0, 15.0),
                    (24.0, 19.0),
                    (24.0, 22.0),
                    (21.5, 25.0),
                    (17.0, 26.0),
                    (13.0, 25.0),
                    (10.0, 23.0),
                ],
            )
            .done(),
        // Delete: a skull and crossbones.
        "delete" => Ic::new(&xi::DELETE)
            .fill_ellipse(16.5, 6.5, 4.6, 3.6, false)
            .fill_rect(14.0, 6.0, 15.0, 7.0, true)
            .fill_rect(18.0, 6.0, 19.0, 7.0, true)
            .fill_rect(15.0, 9.0, 18.0, 13.0, false)
            .curve(
                false,
                &[
                    (10.0, 11.0),
                    (11.0, 13.0),
                    (13.5, 14.2),
                    (16.5, 15.0),
                    (19.5, 14.2),
                    (22.0, 13.0),
                    (23.5, 12.0),
                ],
            )
            .curve(
                false,
                &[
                    (10.0, 18.0),
                    (11.5, 17.0),
                    (14.0, 16.3),
                    (16.5, 16.0),
                    (19.5, 16.3),
                    (22.5, 17.0),
                    (23.0, 18.0),
                ],
            )
            .pixels(2, 20, 34, 29)
            .done(),
        // Update: an object, its attributes copied into the indicator panel.
        "update" => Ic::new(&xi::UPDATE)
            .pixels(4, 3, 13, 11)
            .rect(16.0, 3.0, 32.0, 11.0)
            .pixels(17, 4, 31, 10)
            .line(&[(8.0, 14.0), (8.0, 17.0), (24.0, 17.0), (24.0, 14.0)])
            .line(&[(6.0, 15.0), (8.0, 13.0), (10.0, 15.0)])
            .line(&[(22.0, 15.0), (24.0, 13.0), (26.0, 15.0)])
            .pixels(2, 19, 34, 29)
            .done(),
        // Move point: the dotted original, the point and its two lines moved.
        "movept" => Ic::new(&xi::MOVEPT)
            .pixels(4, 3, 16, 29)
            .line(&[(13.0, 6.0), (21.0, 6.0)])
            .line(&[(19.0, 4.0), (21.0, 6.0), (19.0, 8.0)])
            .poly(
                PolyKind::Box,
                pen(3),
                &[(25.5, 3.5), (29.5, 3.5), (29.5, 7.5), (25.5, 7.5)],
            )
            .wide(&[(8.0, 28.0), (26.0, 10.0)])
            .wide(&[(27.5, 9.0), (27.5, 28.5)])
            .done(),
        // Flip left/right: the mirrored figure stays as pixels; its axis is a line.
        "flip_x" => Ic::new(&xi::FLIP_X)
            .pixels(3, 2, 16, 29)
            .pixels(18, 2, 33, 29)
            .dotted(&[(17.0, 2.0), (17.0, 29.0)])
            .done(),
        "rotCW" => {
            let mut ic = Ic::new(&xi::ROTCW);
            rotate(&mut ic, false).pixels(2, 20, 34, 29).done()
        }
        "rotCCW" => {
            let mut ic = Ic::new(&xi::ROTCCW);
            rotate(&mut ic, true).pixels(2, 20, 34, 29).done()
        }
        // Attribute pictures: no grid / any position (a framed word) …
        "none" => Ic::new(&xi::NONE)
            .rect(3.0, 4.0, 28.0, 27.0)
            .pixels(5, 10, 26, 21)
            .done(),
        "any" => Ic::new(&xi::ANY)
            .rect(3.0, 4.0, 28.0, 27.0)
            .pixels(5, 10, 26, 21)
            .done(),
        // … grids of xfig's spacings …
        "fine_grid" => grid(
            &xi::FINE_GRID,
            &[3.0, 6.0, 9.0, 12.0, 15.0, 18.0, 21.0, 24.0, 27.0],
            &[4.0, 7.0, 10.0, 13.0, 16.0, 19.0, 22.0, 25.0, 28.0],
            (0.0, 31.0),
            (0.0, 31.0),
        ),
        "grid1" => grid(
            &xi::GRID1,
            &[5.0, 9.0, 13.0, 17.0, 21.0, 25.0],
            &[5.0, 9.0, 13.0, 17.0, 21.0, 25.0],
            (1.0, 29.0),
            (1.0, 29.0),
        ),
        "grid2" => grid(
            &xi::GRID2,
            &[3.0, 8.0, 13.0, 18.0, 23.0, 28.0],
            &[4.0, 9.0, 14.0, 19.0, 24.0, 29.0],
            (0.0, 31.0),
            (0.0, 31.0),
        ),
        "grid3" => grid(
            &xi::GRID3,
            &[5.0, 15.0, 25.0],
            &[5.0, 15.0, 25.0],
            (0.0, 31.0),
            (0.0, 31.0),
        ),
        "grid4" => grid(&xi::GRID4, &[6.0, 26.0], &[6.0, 26.0], (0.0, 31.0), (0.0, 31.0)),
        // … and alignment: two boxes and the arrow pointing at the edge or centre.
        "valignt" => Ic::new(&xi::VALIGNT)
            .rect(3.0, 3.0, 12.0, 12.0)
            .rect(24.0, 3.0, 28.0, 20.0)
            .line(&[(15.0, 3.0), (21.0, 3.0)])
            .line(&[(15.0, 4.0), (18.0, 10.0), (21.0, 4.0)])
            .done(),
        "valignc" => Ic::new(&xi::VALIGNC)
            .rect(3.0, 11.0, 12.0, 20.0)
            .rect(24.0, 7.0, 28.0, 24.0)
            .line(&[(21.0, 12.0), (15.0, 15.0), (21.0, 18.0), (21.0, 12.0)])
            .done(),
        "valignb" => Ic::new(&xi::VALIGNB)
            .rect(3.0, 19.0, 12.0, 28.0)
            .rect(24.0, 11.0, 28.0, 28.0)
            .line(&[(15.0, 28.0), (21.0, 28.0)])
            .line(&[(15.0, 27.0), (18.0, 21.0), (21.0, 27.0)])
            .done(),
        "halignl" => Ic::new(&xi::HALIGNL)
            .rect(3.0, 3.0, 14.0, 12.0)
            .rect(3.0, 24.0, 23.0, 28.0)
            .line(&[(3.0, 15.0), (3.0, 21.0)])
            .line(&[(4.0, 15.0), (10.0, 18.0), (4.0, 21.0)])
            .done(),
        "halignc" => Ic::new(&xi::HALIGNC)
            .rect(10.0, 3.0, 21.0, 12.0)
            .rect(5.0, 24.0, 26.0, 28.0)
            .polygon(&[(16.0, 15.0), (19.0, 21.0), (13.0, 21.0)])
            .done(),
        "halignr" => Ic::new(&xi::HALIGNR)
            .rect(17.0, 3.0, 28.0, 12.0)
            .rect(8.0, 24.0, 28.0, 28.0)
            .line(&[(28.0, 15.0), (28.0, 21.0)])
            .line(&[(27.0, 15.0), (21.0, 18.0), (27.0, 21.0)])
            .done(),
        // Open compound: the compound's bracketed frame with its contents.
        "open_comp" => Ic::new(&xi::OPEN_COMP)
            .line(&[(3.0, 6.0), (19.0, 6.0)])
            .line(&[(3.0, 18.0), (19.0, 18.0)])
            .line(&[(4.0, 6.0), (4.0, 18.0)])
            .line(&[(18.0, 6.0), (18.0, 18.0)])
            .pixels(3, 5, 19, 5)
            .pixels(3, 19, 19, 19)
            .pixels(5, 7, 17, 17)
            .pixels(20, 19, 33, 29)
            .done(),
        // Join or split: two lines and two curves meeting at a point.
        "join_split" => Ic::new(&xi::JOIN_SPLIT)
            .line(&[(3.0, 3.0), (9.0, 12.0), (11.0, 10.0)])
            .fill_rect(11.0, 6.0, 14.0, 9.0, false)
            .curve(
                false,
                &[(15.0, 4.0), (18.0, 5.0), (21.0, 7.0), (23.5, 9.0), (26.5, 11.0)],
            )
            .line(&[(32.0, 3.0), (28.0, 10.0), (26.5, 11.0)])
            .line(&[(3.0, 19.0), (8.0, 27.0)])
            .curve(
                false,
                &[
                    (15.0, 20.0),
                    (17.0, 21.0),
                    (19.5, 22.5),
                    (21.5, 24.0),
                    (23.5, 26.0),
                    (25.0, 27.0),
                ],
            )
            .line(&[(32.0, 20.0), (28.0, 27.0), (26.5, 28.0)])
            .pixels(16, 10, 20, 18)
            .done(),
        // Angle: three numbered points, their rays and the arc between.
        "anglemeas" => Ic::new(&xi::ANGLEMEAS)
            .line(&[(11.0, 5.5), (18.0, 10.5)])
            .line(&[(21.0, 13.0), (24.0, 14.0)])
            .line(&[(19.0, 19.0), (10.0, 24.5)])
            .ellipse(18.5, 15.0, 3.5, 4.0)
            .marker(9.0, 5.0, 3.0)
            .fill_rect(24.0, 14.0, 27.0, 16.0, false)
            .marker(9.0, 25.0, 3.0)
            .pixels(3, 3, 7, 10)
            .pixels(28, 11, 33, 19)
            .pixels(2, 19, 7, 27)
            .done(),
        // Pixel drawings in xfig, kept as crisp pixels: the pencil of delete
        // point, the blade of chop, the hatching of area, the books of the
        // library.
        "deletept" => Ic::new(&xi::DELETEPT).pixels(0, 0, 35, 31).done(),
        "chop" => Ic::new(&xi::CHOP).pixels(0, 0, 35, 31).done(),
        // Trim, split and extend (RustFig's own): a solid line that ends at
        // a cutting edge (the short cross-stroke) and goes on dotted, as
        // the extension does.
        "trim" => Ic::new(&xi::TRIM)
            .wide(&[(3.0, 16.0), (17.0, 16.0)])
            .wide(&[(19.0, 8.0), (19.0, 24.0)])
            .marker(23.0, 15.5, 2.0)
            .marker(27.0, 15.5, 2.0)
            .marker(31.0, 15.5, 2.0)
            .done(),
        // Plot a curve (RustFig's own): the two axes and a sine wave.
        "plot" => Ic::new(&xi::PLOT)
            .wide(&[(5.0, 4.0), (5.0, 27.0), (32.0, 27.0)])
            .wide(&[
                (7.0, 24.0),
                (10.0, 14.0),
                (13.0, 8.0),
                (16.0, 10.0),
                (19.0, 18.0),
                (23.0, 24.0),
                (26.0, 22.0),
                (30.0, 12.0),
            ])
            .done(),
        "areameas" => Ic::new(&xi::AREAMEAS).pixels(0, 0, 35, 31).done(),
        "library" => Ic::new(&xi::LIBRARY).pixels(0, 0, 72, 31).done(),
        // Pixel art in xfig: the camera and the serif T stay as pixels.
        "picobj" => Ic::new(&xi::PICOBJ).pixels(0, 0, 35, 31).done(),
        "text" => Ic::new(&xi::TEXT).pixels(0, 0, 35, 31).done(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_icons::render_objects;

    /// Every traced icon covers xfig's bitmap: nearly every bitmap pixel has
    /// ink within one pixel, and nearly all ink lies within one pixel of the
    /// bitmap.
    pub(crate) fn overlap(bitmap: &Xbm) -> (f64, f64) {
        let objs = drawing(bitmap.name).unwrap();
        let img = render_objects(objs, bitmap.width, bitmap.height, 1.0).unwrap();
        assert_eq!(img.size, [bitmap.width, bitmap.height]);
        let ink = |x: usize, y: usize| img.pixels.get(y * bitmap.width + x).is_some_and(|p| p.a() > 90);
        let near = |f: &dyn Fn(usize, usize) -> bool, x: usize, y: usize| {
            (y.saturating_sub(1)..=(y + 1).min(bitmap.height - 1))
                .any(|yy| (x.saturating_sub(1)..=(x + 1).min(bitmap.width - 1)).any(|xx| f(xx, yy)))
        };
        let (mut b_total, mut b_hit, mut i_total, mut i_hit) = (0, 0, 0, 0);
        for y in 0..bitmap.height {
            for x in 0..bitmap.width {
                if bitmap.get(x, y) {
                    b_total += 1;
                    if near(&ink, x, y) {
                        b_hit += 1;
                    }
                }
                if ink(x, y) {
                    i_total += 1;
                    if near(&|x, y| bitmap.get(x, y), x, y) {
                        i_hit += 1;
                    }
                }
            }
        }
        (
            f64::from(b_hit) / f64::from(b_total.max(1)),
            f64::from(i_hit) / f64::from(i_total.max(1)),
        )
    }

    #[test]
    fn traced_bitmaps_have_drawings() {
        for b in TRACED {
            assert!(drawing(b.name).is_some_and(|d| !d.is_empty()), "{}", b.name);
            assert_eq!(size(b.name), (b.width, b.height));
        }
    }

    #[test]
    fn traced_icons_match_xfigs_bitmaps() {
        let mut report = Vec::new();
        for b in TRACED {
            let (covered, inside) = overlap(b);
            report.push(format!("{:12} covered {covered:.2} inside {inside:.2}", b.name));
            assert!(covered >= 0.9 && inside >= 0.9, "{}", report.join("\n"));
        }
        eprintln!("{}", report.join("\n"));
    }

    /// A review sheet: per traced icon, xfig's bitmap, the vector at 1× and
    /// at 3×. `RUSTFIG_ICON_SHEET=path.png cargo test -p rustfig-ui-egui
    /// --lib icon_sheet -- --ignored`.
    #[test]
    #[ignore]
    fn icon_sheet() {
        let path = std::env::var("RUSTFIG_ICON_SHEET").unwrap_or_else(|_| "icon_sheet.png".into());
        let (cw, ch) = (36 * 5 + 16, 32 * 3 + 8);
        let rows = TRACED.len();
        let (w, h) = (cw * 2, ch * rows.div_ceil(2));
        let mut pm = rustfig_render::raster::Pixmap::new(w as u32, h as u32).unwrap();
        pm.data_mut().fill(255);
        let mut put = |x: usize, y: usize, v: u8| {
            if x < w && y < h {
                let o = (y * w + x) * 4;
                let d = pm.data_mut();
                d[o] = 255 - v;
                d[o + 1] = 255 - v;
                d[o + 2] = 255 - v;
            }
        };
        for (i, b) in TRACED.iter().enumerate() {
            let (ox, oy) = ((i % 2) * cw, (i / 2) * ch);
            for y in 0..b.height {
                for x in 0..b.width {
                    if b.get(x, y) {
                        put(ox + x, oy + y, 255);
                    }
                }
            }
            let one = render_objects(drawing(b.name).unwrap(), b.width, b.height, 1.0).unwrap();
            for y in 0..one.size[1] {
                for x in 0..one.size[0] {
                    put(ox + 40 + x, oy + y, one.pixels[y * one.size[0] + x].a());
                }
            }
            let three = render_objects(drawing(b.name).unwrap(), b.width, b.height, 3.0).unwrap();
            for y in 0..three.size[1] {
                for x in 0..three.size[0] {
                    put(ox + 80 + x, oy + y, three.pixels[y * three.size[0] + x].a());
                }
            }
        }
        pm.save_png(&path).unwrap();
    }
}
