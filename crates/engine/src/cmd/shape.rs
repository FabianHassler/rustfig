//! Shape creation commands. Every shape takes optional attribute overrides
//! (see [`STYLE_PARAMS`]); missing attributes come from the indicator.

use super::{num, num_or, opt_bool, opt_num, opt_str, points, string, CmdResult, Command};
use crate::{import, Engine, EngineError};
use rustfig_geom::{regular_polygon, Point, Rect};
use rustfig_model::{
    Arc, ColorRef, Ellipse, EllipseKind, Justify, LineKind, Object, ObjectKind, Picture, PolyKind, Polyline, Spline,
    SplineKind, Style, Text,
};
use serde_json::{json, Value};
use std::sync::Arc as Shared;

/// Attribute overrides accepted by every shape command.
pub const STYLE_PARAMS: &str = "\"depth\"?, \"thickness\"?, \"pen_color\"?, \"fill_color\"?, \"area_fill\"?, \"line\"?: solid|dashed|dotted|dash-dotted|..., \"style_val\"?, \"forward_arrow\"?: bool, \"backward_arrow\"?: bool";

macro_rules! shape_cmd {
    ($id:expr, $label:expr, $params:expr, $run:expr) => {
        Command {
            id: $id,
            label: $label,
            menu: "",
            shortcut: "",
            params: $params,
            mutates: true,
            enabled: super::always,
            run: $run,
        }
    };
}

pub(super) fn commands() -> Vec<Command> {
    vec![
        shape_cmd!(
            "shape.rectangle",
            "Add rectangle",
            "{\"x\", \"y\", \"width\", \"height\", \"radius\"?: 1/80 inch (rounded corners)} + style",
            rectangle
        ),
        shape_cmd!("shape.ellipse", "Add ellipse", "{\"cx\", \"cy\", \"rx\", \"ry\", \"angle\"?: degrees} + style", ellipse),
        shape_cmd!("shape.circle", "Add circle", "{\"cx\", \"cy\", \"r\"} + style", circle),
        shape_cmd!("shape.polyline", "Add polyline", "{\"points\": [[x,y],...]} + style", polyline),
        shape_cmd!("shape.polygon", "Add polygon", "{\"points\": [[x,y],...]} + style", polygon),
        shape_cmd!(
            "shape.spline",
            "Add spline",
            "{\"points\": [[x,y],...], \"closed\"?: bool, \"kind\"?: approximated|interpolated|x, \"shapes\"?: [s]} + style",
            spline
        ),
        shape_cmd!(
            "shape.arc",
            "Add arc",
            "{\"points\": [p1, p2, p3], \"pie\"?: bool}  — circular arc through three points + style",
            arc
        ),
        shape_cmd!(
            "shape.regular_polygon",
            "Add regular polygon",
            "{\"cx\", \"cy\", \"r\", \"sides\"?: number, \"angle\"?: degrees} + style",
            regpoly
        ),
        shape_cmd!(
            "shape.text",
            "Add text",
            "{\"x\", \"y\", \"text\", \"size\"?: points, \"font\"?: number, \"postscript\"?: bool, \"justify\"?: left|center|right, \"angle\"?: degrees, \"color\"?, \"depth\"?}",
            text
        ),
        shape_cmd!(
            "shape.picture",
            "Add picture",
            "{\"path\": string, \"x\", \"y\", \"width\"?, \"height\"?}  — width/height default to the image size at 80 dpi",
            picture
        ),
        shape_cmd!(
            "object.add",
            "Add object (raw)",
            "{\"object\": object JSON as returned by object.get}",
            add_raw
        ),
    ]
}

pub(crate) fn style_from(e: &Engine, p: &Value) -> Result<Style, EngineError> {
    let mut s = e.indicator.style();
    if let Some(v) = opt_num(p, "depth")? {
        s.depth = (v.round() as i32).clamp(0, rustfig_model::MAX_DEPTH);
    }
    if let Some(v) = opt_num(p, "thickness")? {
        s.thickness = (v.round() as i32).clamp(0, 10_000);
    }
    if let Some(v) = opt_num(p, "pen_color")? {
        s.pen_color = ColorRef((v.round() as i32).clamp(-1, rustfig_model::LAST_USER_COLOR));
    }
    if let Some(v) = opt_num(p, "fill_color")? {
        s.fill_color = ColorRef((v.round() as i32).clamp(-1, rustfig_model::LAST_USER_COLOR));
    }
    if let Some(v) = opt_num(p, "area_fill")? {
        s.area_fill = (v.round() as i32).clamp(-1, 62);
    }
    if let Some(name) = opt_str(p, "line")? {
        s.line =
            LineKind::from_name(name).ok_or_else(|| EngineError::bad_params(format!("unknown line style {name:?}")))?;
        if s.line != LineKind::Solid && s.style_val <= 0.0 {
            s.style_val = e.indicator.style_val;
        }
    }
    if let Some(v) = opt_num(p, "style_val")? {
        s.style_val = v.clamp(0.0, 1000.0);
    }
    Ok(s)
}

fn arrows_from(
    e: &Engine,
    p: &Value,
) -> Result<(Option<rustfig_model::Arrow>, Option<rustfig_model::Arrow>), EngineError> {
    let (mut f, mut b) = e.indicator.arrows();
    let template = e.indicator.arrows();
    let a = rustfig_model::Arrow {
        thickness: f64::from(e.indicator.thickness.max(1)),
        ..e.indicator.arrow
    };
    if let Some(on) = opt_bool(p, "forward_arrow")? {
        f = if on { template.0.or(Some(a)) } else { None };
    }
    if let Some(on) = opt_bool(p, "backward_arrow")? {
        b = if on { template.1.or(Some(a)) } else { None };
    }
    Ok((f, b))
}

fn added(e: &mut Engine, o: Object) -> CmdResult {
    let id = e.add_object(o);
    e.set_selection(vec![id]);
    Ok(json!({"id": id.0}))
}

fn rectangle(e: &mut Engine, p: &Value) -> CmdResult {
    let (x, y, w, h) = (num(p, "x")?, num(p, "y")?, num(p, "width")?, num(p, "height")?);
    let style = style_from(e, p)?;
    let mut pl = Polyline::rect(style, Point::new(x, y), Point::new(x + w, y + h));
    if let Some(r) = opt_num(p, "radius")? {
        pl.kind = PolyKind::ArcBox;
        pl.radius = (r.round() as i32).clamp(0, 100_000);
    }
    added(e, Object::new(ObjectKind::Polyline(pl)))
}

pub(crate) fn make_ellipse(style: Style, center: Point, rx: f64, ry: f64, angle: f64, kind: EllipseKind) -> Object {
    Object::new(ObjectKind::Ellipse(Ellipse {
        kind,
        style,
        direction: 1,
        angle,
        center,
        radii: Point::new(rx.abs(), ry.abs()),
        start: center,
        end: Point::new(center.x + rx.abs(), center.y + ry.abs()),
    }))
}

fn ellipse(e: &mut Engine, p: &Value) -> CmdResult {
    let c = Point::new(num(p, "cx")?, num(p, "cy")?);
    let (rx, ry) = (num(p, "rx")?, num(p, "ry")?);
    let angle = num_or(p, "angle", 0.0)?.to_radians();
    let style = style_from(e, p)?;
    added(e, make_ellipse(style, c, rx, ry, angle, EllipseKind::EllipseRadii))
}

fn circle(e: &mut Engine, p: &Value) -> CmdResult {
    let c = Point::new(num(p, "cx")?, num(p, "cy")?);
    let r = num(p, "r")?;
    let style = style_from(e, p)?;
    added(e, make_ellipse(style, c, r, r, 0.0, EllipseKind::CircleRadius))
}

fn need(pts: &[Point], n: usize, what: &str) -> Result<(), EngineError> {
    if pts.len() < n {
        return Err(EngineError::bad_params(format!("a {what} needs at least {n} points")));
    }
    Ok(())
}

fn polyline(e: &mut Engine, p: &Value) -> CmdResult {
    let pts = points(p, "points")?;
    need(&pts, 1, "polyline")?;
    let mut pl = Polyline::new(PolyKind::Polyline, style_from(e, p)?, pts);
    (pl.forward_arrow, pl.backward_arrow) = arrows_from(e, p)?;
    added(e, Object::new(ObjectKind::Polyline(pl)))
}

fn polygon(e: &mut Engine, p: &Value) -> CmdResult {
    let pts = points(p, "points")?;
    need(&pts, 3, "polygon")?;
    let pl = Polyline::new(PolyKind::Polygon, style_from(e, p)?, pts);
    added(e, Object::new(ObjectKind::Polyline(pl)))
}

fn spline(e: &mut Engine, p: &Value) -> CmdResult {
    let pts = points(p, "points")?;
    let closed = opt_bool(p, "closed")?.unwrap_or(false);
    need(&pts, if closed { 3 } else { 2 }, "spline")?;
    let kind = match opt_str(p, "kind")?.unwrap_or("approximated") {
        "approximated" | "approx" => SplineKind::Approximated,
        "interpolated" | "interp" => SplineKind::Interpolated,
        "x" | "xspline" => SplineKind::X,
        other => return Err(EngineError::bad_params(format!("unknown spline kind {other:?}"))),
    };
    let mut s = Spline::new(kind, closed, style_from(e, p)?, pts);
    if let Some(Value::Array(a)) = p.get("shapes") {
        if a.len() != s.points.len() {
            return Err(EngineError::bad_params("\"shapes\" needs one value per point"));
        }
        s.shapes = a
            .iter()
            .map(|v| v.as_f64().filter(|f| f.is_finite()).map(|f| f.clamp(-1.0, 1.0)))
            .collect::<Option<Vec<f64>>>()
            .ok_or_else(|| EngineError::bad_params("shapes must be numbers in [-1, 1]"))?;
    }
    if !closed {
        (s.forward_arrow, s.backward_arrow) = arrows_from(e, p)?;
    }
    added(e, Object::new(ObjectKind::Spline(s)))
}

fn arc(e: &mut Engine, p: &Value) -> CmdResult {
    let pts = points(p, "points")?;
    let [a, b, c] = pts.as_slice() else {
        return Err(EngineError::bad_params("an arc needs exactly three points"));
    };
    let pie = opt_bool(p, "pie")?.unwrap_or(false);
    let mut arc = Arc::through(style_from(e, p)?, *a, *b, *c, pie)
        .ok_or_else(|| EngineError::bad_params("the three points of an arc must not be collinear"))?;
    if !pie {
        (arc.forward_arrow, arc.backward_arrow) = arrows_from(e, p)?;
    }
    added(e, Object::new(ObjectKind::Arc(arc)))
}

fn regpoly(e: &mut Engine, p: &Value) -> CmdResult {
    let c = Point::new(num(p, "cx")?, num(p, "cy")?);
    let r = num(p, "r")?;
    let sides = num_or(p, "sides", e.indicator.sides as f64)?.round().clamp(3.0, 1000.0) as usize;
    let a = num_or(p, "angle", -90.0)?.to_radians();
    let first = Point::new(c.x + r * a.cos(), c.y + r * a.sin());
    let pl = Polyline::new(PolyKind::Polygon, style_from(e, p)?, regular_polygon(c, first, sides));
    added(e, Object::new(ObjectKind::Polyline(pl)))
}

pub(crate) fn parse_justify(s: &str) -> Result<Justify, EngineError> {
    match s {
        "left" => Ok(Justify::Left),
        "center" | "centre" => Ok(Justify::Center),
        "right" => Ok(Justify::Right),
        other => Err(EngineError::bad_params(format!("unknown justification {other:?}"))),
    }
}

pub(crate) fn make_text(e: &Engine, origin: Point, s: String) -> Text {
    let ind = &e.indicator;
    Text {
        justify: ind.justify,
        color: ColorRef(ind.pen_color),
        depth: ind.depth.clamp(0, rustfig_model::MAX_DEPTH),
        pen_style: -1,
        font: ind.font,
        size: ind.font_size.clamp(1.0, 2000.0),
        angle: ind.text_angle.to_radians(),
        flags: ind.text_flags(),
        height: 0.0,
        length: 0.0,
        origin,
        text: s,
    }
}

fn text(e: &mut Engine, p: &Value) -> CmdResult {
    let origin = Point::new(num(p, "x")?, num(p, "y")?);
    let s = string(p, "text")?;
    if s.len() > rustfig_format::MAX_TEXT_BYTES {
        return Err(EngineError::bad_params("text is too long"));
    }
    let mut t = make_text(e, origin, s);
    if let Some(v) = opt_num(p, "size")? {
        t.size = v.clamp(1.0, 2000.0);
    }
    if let Some(v) = opt_num(p, "font")? {
        t.font = (v.round() as i32).clamp(-1, 34);
    }
    if let Some(ps) = opt_bool(p, "postscript")? {
        if ps {
            t.flags |= rustfig_model::text_flags::POSTSCRIPT;
        } else {
            t.flags &= !rustfig_model::text_flags::POSTSCRIPT;
        }
    }
    if let Some(j) = opt_str(p, "justify")? {
        t.justify = parse_justify(j)?;
    }
    if let Some(a) = opt_num(p, "angle")? {
        t.angle = a.to_radians();
    }
    if let Some(c) = opt_num(p, "color")? {
        t.color = ColorRef((c.round() as i32).clamp(-1, rustfig_model::LAST_USER_COLOR));
    }
    if let Some(d) = opt_num(p, "depth")? {
        t.depth = (d.round() as i32).clamp(0, rustfig_model::MAX_DEPTH);
    }
    added(e, Object::new(ObjectKind::Text(t)))
}

fn picture(e: &mut Engine, p: &Value) -> CmdResult {
    let path = string(p, "path")?;
    let img = import::load_picture(std::path::Path::new(&path)).map_err(EngineError::io)?;
    let (x, y) = (num(p, "x")?, num(p, "y")?);
    // Default size: 80 pixels per inch, like xfig's screen resolution.
    let w = opt_num(p, "width")?.unwrap_or(f64::from(img.width) * 15.0);
    let h = opt_num(p, "height")?.unwrap_or(f64::from(img.height) * 15.0);
    let r = Rect::new(Point::new(x, y), Point::new(x + w, y + h));
    let mut pl = Polyline::new(PolyKind::Picture, style_from(e, p)?, r.corners().to_vec());
    pl.style.thickness = 0;
    pl.picture = Some(Picture {
        flipped: false,
        file: path,
        image: Some(Shared::new(img)),
    });
    added(e, Object::new(ObjectKind::Polyline(pl)))
}

fn add_raw(e: &mut Engine, p: &Value) -> CmdResult {
    let v = p
        .get("object")
        .ok_or_else(|| EngineError::bad_params("missing \"object\""))?;
    let obj: Object =
        serde_json::from_value(v.clone()).map_err(|err| EngineError::bad_params(format!("bad object: {err}")))?;
    validate(&obj, 0)?;
    added(e, obj)
}

/// Reject objects with non-finite coordinates or absurd sizes.
pub(crate) fn validate(o: &Object, depth: usize) -> Result<(), EngineError> {
    if depth > rustfig_format::MAX_NESTING {
        return Err(EngineError::bad_params("compound nested too deeply"));
    }
    let bad = || EngineError::bad_params("object has invalid coordinates");
    let fin = |p: &Point| p.is_finite() && p.x.abs() < 1.0e9 && p.y.abs() < 1.0e9;
    match &o.kind {
        ObjectKind::Compound(c) => {
            for x in &c.objects {
                validate(x, depth + 1)?;
            }
        }
        ObjectKind::Text(t) => {
            if !fin(&t.origin) || !t.size.is_finite() || !t.angle.is_finite() {
                return Err(bad());
            }
        }
        ObjectKind::Ellipse(el) => {
            if !fin(&el.center) || !fin(&el.radii) || !el.angle.is_finite() {
                return Err(bad());
            }
        }
        _ => {
            let pts = o.points().unwrap_or(&[]);
            if pts.len() > super::MAX_PARAM_POINTS || !pts.iter().all(fin) {
                return Err(bad());
            }
        }
    }
    Ok(())
}
