//! fig2dev's reader of Fig 1.3 files (`read1_3.c`): a header line, then
//! objects as free-format numbers, lines and splines ending at `9999`.
//
// Ported from fig2dev read1_3.c (xfig licence, see LICENSE-xfig).

use super::{common_invalid, ellipse_invalid, text_invalid, Reader, R};
use crate::object::*;

/// Fig 1.3 object subtypes.
const DRAW_ELLIPSE_BY_RAD: i32 = 1;
const DRAW_ELLIPSE_BY_DIA: i32 = 2;
const DRAW_CIRCLE_BY_RAD: i32 = 3;
const DRAW_POLYLINE: i32 = 6;
const DRAW_POLYGON: i32 = 8;
const DRAW_CLOSEDSPLINE: i32 = 11;

/// The arrowhead of Fig 1.3 (`forward_arrow`, `backward_arrow`).
fn arrow_1_3(thick_scale: f32, w: i32, h: i32) -> Arrow {
    Arrow {
        type_: 0,
        style: 0,
        thickness: f64::from(thick_scale),
        wid: f64::from(w),
        ht: f64::from(h),
    }
}

impl Reader<'_, '_> {
    fn invalid_1_3(&mut self, what: &str) {
        self.msg(format!("Invalid {what} object."));
    }

    /// `read_1_3_objects`.
    pub(super) fn read_1_3_objects(&mut self) -> R<Compound> {
        let mut obj = Compound::default();
        // fig2dev's flags keep their start values; THICK_SCALE too (0).
        self.ctx.v2_flag = false;
        self.ctx.v21_flag = false;
        self.ctx.v30_flag = false;
        self.ctx.v32_flag = false;
        self.ctx.thick_scale = 0.0;

        let header = (|| {
            let a = self.inp.int()?;
            let b = self.inp.int()?;
            let c = self.inp.int()?;
            let d = self.inp.int()?;
            Some((a, b, c, d))
        })();
        let Some((pixperinch, coord_sys, _canvaswid, _canvasht)) = header else {
            self.msg("Incorrect format in the first line in input file");
            return Err(-1);
        };
        self.inp.skip_ws();
        if pixperinch <= 0 {
            self.msg(format!("Invalid resolution information ({pixperinch})."));
            return Err(-1);
        }
        self.ctx.ppi = f64::from(pixperinch);
        obj.nwcorner = Point::new(pixperinch, coord_sys);

        while let Some(object) = self.inp.int() {
            match object {
                OBJ_COMPOUND => {
                    let Some(c) = self.read_compound_1_3()? else {
                        return Err(-3);
                    };
                    obj.compounds.push(c);
                }
                _ => {
                    if !self.read_object_1_3(object, &mut obj)? {
                        self.msg(format!("Incorrect object code {object}"));
                        return Err(-1);
                    }
                }
            }
        }
        if self.inp.eof {
            Ok(obj)
        } else {
            Err(-3)
        }
    }

    /// An object other than a compound; `Ok(false)` for an unknown code.
    fn read_object_1_3(&mut self, object: i32, com: &mut Compound) -> R<bool> {
        match object {
            OBJ_POLYLINE => {
                let l = self.read_line_1_3()?;
                com.lines.push(l);
            }
            OBJ_SPLINE => {
                let s = self.read_spline_1_3()?;
                com.splines.push(s);
            }
            OBJ_ELLIPSE => {
                let e = self.read_ellipse_1_3()?;
                com.ellipses.push(e);
            }
            OBJ_ARC => {
                let a = self.read_arc_1_3()?;
                com.arcs.push(a);
            }
            OBJ_TEXT => {
                let t = self.read_text_1_3()?;
                com.texts.push(t);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// `read_compoundobject`: `Ok(None)` where fig2dev's returns NULL,
    /// which the top level reports as an input error.
    fn read_compound_1_3(&mut self) -> R<Option<Compound>> {
        let mut com = Compound::default();
        let corners = (|| Some((self.inp.int()?, self.inp.int()?, self.inp.int()?, self.inp.int()?)))();
        let Some((a, b, c, d)) = corners else {
            self.msg("Incorrect compound object format");
            return Ok(None);
        };
        self.inp.skip_ws();
        com.nwcorner = Point::new(a, b);
        com.secorner = Point::new(c, d);
        if self.nesting >= super::MAX_NESTING {
            self.msg(format!(
                "Compound objects nested deeper than {} levels.",
                super::MAX_NESTING
            ));
            return Ok(None);
        }
        self.nesting += 1;
        let r = self.read_compound_body_1_3(&mut com);
        self.nesting -= 1;
        Ok(r?.then_some(com))
    }

    fn read_compound_body_1_3(&mut self, com: &mut Compound) -> R<bool> {
        while let Some(object) = self.inp.int() {
            match object {
                OBJ_END_COMPOUND => return Ok(true),
                OBJ_COMPOUND => match self.read_compound_1_3() {
                    Ok(Some(c)) => com.compounds.push(c),
                    _ => return Ok(false),
                },
                // Unknown codes are skipped.
                _ => {
                    if self.read_object_1_3(object, com).is_err() {
                        return Ok(false);
                    }
                }
            }
        }
        if self.inp.eof {
            Ok(true)
        } else {
            self.msg("Fig 1.3 format error.");
            Ok(false)
        }
    }

    /// `read_arcobject`.
    fn read_arc_1_3(&mut self) -> R<Arc> {
        let f = self.inp.fields("dddfdddddffdddddd");
        self.inp.skip_ws();
        let mut a = Arc {
            type_: T_OPEN_ARC,
            pen_color: BLACK_COLOR,
            fill_color: BLACK_COLOR,
            fill_style: UNFILLED,
            ..Arc::default()
        };
        if f.n() != 17 {
            self.msg("incomplete arc data");
            return Err(-1);
        }
        (a.style, a.thickness, a.style_val, a.direction) = (f.i(1), f.i(2), f.f(3), f.i(4));
        let (fa, ba, h, w) = (f.i(5), f.i(6), f.i(7), f.i(8));
        a.center = DPoint { x: f.f(9), y: f.f(10) };
        for (k, p) in a.point.iter_mut().enumerate() {
            *p = Point::new(f.i(11 + 2 * k), f.i(12 + 2 * k));
        }
        let coord = |v: f64| v.is_finite() && v >= f64::from(i32::MIN) && v <= f64::from(i32::MAX);
        if common_invalid(a.style, a.thickness, a.depth, a.fill_style, a.style_val)
            || !coord(a.center.x)
            || !coord(a.center.y)
            || (a.direction != 0 && a.direction != 1)
        {
            self.invalid_1_3("arc");
            return Err(-1);
        }
        let ts = self.ctx.thick_scale;
        if fa != 0 {
            a.for_arrow = Some(arrow_1_3(ts, w, h));
        }
        if ba != 0 {
            a.back_arrow = Some(arrow_1_3(ts, w, h));
        }
        Ok(a)
    }

    /// `read_ellipseobject`.
    fn read_ellipse_1_3(&mut self) -> R<Ellipse> {
        let f = self.inp.fields("dddfddddddddd");
        self.inp.skip_ws();
        if f.n() != 13 {
            self.msg("incomplete ellipse data");
            return Err(-1);
        }
        let mut e = Ellipse {
            pen_color: BLACK_COLOR,
            fill_color: BLACK_COLOR,
            fill_style: UNFILLED,
            ..Ellipse::default()
        };
        e.type_ = match f.i(0) {
            DRAW_ELLIPSE_BY_RAD => T_ELLIPSE_BY_RAD,
            DRAW_ELLIPSE_BY_DIA => T_ELLIPSE_BY_DIA,
            DRAW_CIRCLE_BY_RAD => T_CIRCLE_BY_RAD,
            _ => T_CIRCLE_BY_DIA,
        };
        (e.style, e.thickness, e.style_val, e.direction) = (f.i(1), f.i(2), f.f(3), f.i(4));
        e.center = Point::new(f.i(5), f.i(6));
        e.radiuses = Point::new(f.i(7), f.i(8));
        e.start = Point::new(f.i(9), f.i(10));
        e.end = Point::new(f.i(11), f.i(12));
        if ellipse_invalid(&e) {
            self.invalid_1_3("ellipse");
            return Err(-1);
        }
        Ok(e)
    }

    /// The points after the first, up to `9999` (`what` names the object
    /// in the message).
    fn points_1_3(&mut self, points: &mut Vec<Point>, what: &str) -> R<()> {
        loop {
            let xy = (|| Some((self.inp.int()?, self.inp.int()?)))();
            let Some((x, y)) = xy else {
                self.msg(format!("incomplete {what} object"));
                return Err(-1);
            };
            if x == 9999 {
                return Ok(());
            }
            points.push(Point::new(x, y));
        }
    }

    /// `read_lineobject`.
    fn read_line_1_3(&mut self) -> R<Line> {
        let f = self.inp.fields("dddfdddddd");
        if f.n() != 10 {
            self.msg("incomplete line data");
            return Err(-1);
        }
        let mut l = Line {
            pen_color: BLACK_COLOR,
            fill_color: BLACK_COLOR,
            fill_style: UNFILLED,
            ..Line::default()
        };
        l.type_ = match f.i(0) {
            DRAW_POLYLINE => T_POLYLINE,
            DRAW_POLYGON => T_POLYGON,
            _ => T_BOX,
        };
        (l.style, l.thickness, l.style_val) = (f.i(1), f.i(2), f.f(3));
        let (fa, ba, h, w) = (f.i(4), f.i(5), f.i(6), f.i(7));
        l.points.push(Point::new(f.i(8), f.i(9)));
        if common_invalid(l.style, l.thickness, l.depth, l.fill_style, l.style_val) {
            self.invalid_1_3("line");
            return Err(-1);
        }
        let ts = self.ctx.thick_scale;
        if fa != 0 {
            l.for_arrow = Some(arrow_1_3(ts, w, h));
        }
        if ba != 0 {
            l.back_arrow = Some(arrow_1_3(ts, w, h));
        }
        self.points_1_3(&mut l.points, "line")?;
        Ok(l)
    }

    /// `read_splineobject`.
    fn read_spline_1_3(&mut self) -> R<Spline> {
        let f = self.inp.fields("dddfdddddd");
        if f.n() != 10 {
            self.msg("incomplete spline data");
            return Err(-1);
        }
        let mut s = Spline {
            pen_color: BLACK_COLOR,
            fill_color: BLACK_COLOR,
            fill_style: UNFILLED,
            ..Spline::default()
        };
        s.type_ = if f.i(0) == DRAW_CLOSEDSPLINE {
            T_CLOSED_APPROX
        } else {
            T_OPEN_APPROX
        };
        (s.style, s.thickness, s.style_val) = (f.i(1), f.i(2), f.f(3));
        let (fa, ba, h, w) = (f.i(4), f.i(5), f.i(6), f.i(7));
        s.points.push(Point::new(f.i(8), f.i(9)));
        if common_invalid(s.style, s.thickness, s.depth, s.fill_style, s.style_val) {
            self.invalid_1_3("spline");
            return Err(-1);
        }
        let ts = self.ctx.thick_scale;
        if fa != 0 {
            s.for_arrow = Some(arrow_1_3(ts, w, h));
        }
        if ba != 0 {
            s.back_arrow = Some(arrow_1_3(ts, w, h));
        }
        self.points_1_3(&mut s.points, "spline")?;
        if s.points.len() < 2 {
            self.msg("spline with a single point");
            return Err(-1);
        }
        Ok(s)
    }

    /// `read_textobject`: the string is the rest of the line, at most 127
    /// bytes.
    fn read_text_1_3(&mut self) -> R<Text> {
        let f = self.inp.fields("dfdffdd");
        let rest = if f.n() == 7 {
            self.inp.skip_ws();
            self.inp.line127()
        } else {
            None
        };
        let Some(rest) = rest else {
            self.msg("incomplete text data");
            return Err(-1);
        };
        let mut t = Text {
            type_: T_LEFT_JUSTIFIED,
            color: BLACK_COLOR,
            ..Text::default()
        };
        (t.font, t.size, t.flags, t.height, t.length) = (f.i(0), f.f(1), f.i(2), f.f(3), f.f(4));
        (t.base_x, t.base_y) = (f.i(5), f.i(6));
        if text_invalid(&t) {
            self.invalid_1_3("text");
            return Err(-1);
        }
        t.cstring = self.decode(rest);
        t.raw = rest.split(|&b| b == 0).next().unwrap_or_default().to_vec();
        if t.size == 0.0 {
            t.size = 18.0;
        }
        Ok(t)
    }
}
