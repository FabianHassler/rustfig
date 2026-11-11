//! Shape creation commands. Every shape takes optional attribute overrides
//! (see [`STYLE_PARAMS`]); missing attributes come from the indicator.

use super::{num, num_or, opt_bool, opt_num, opt_str, points, string, CmdResult, Command};
use crate::{import, Engine, EngineError};
use rustfig_geom::{Point, Rect};
use rustfig_model::{
    Arc, CapStyle, ColorRef, Ellipse, EllipseKind, JoinStyle, Justify, LineKind, Object, ObjectKind, Picture, PolyKind,
    Polyline, Spline, SplineKind, Style, Text,
};
use serde_json::{json, Value};
use std::sync::Arc as Shared;

/// Attribute overrides accepted by every shape command.
pub const STYLE_PARAMS: &str = "\"depth\"?, \"thickness\"?, \"pen_color\"?, \"fill_color\"?, \"area_fill\"?, \"line\"?: solid|dashed|dotted|dash-dotted|..., \"style_val\"?, \"cap\"?: butt|round|projecting, \"join\"?: miter|round|bevel; open shapes (polyline, open spline, arc) also \"forward_arrow\"?: bool, \"backward_arrow\"?: bool, \"arrow_kind\"?: 0-14, \"arrow_style\"?: hollow|filled (or 0|1), \"arrow_width\"?, \"arrow_height\"?, \"arrow_thickness\"? (Fig units, both arrows), \"forward_arrow_spec\"?, \"backward_arrow_spec\"?: {kind, style, thickness, width, height} (turns that arrow on)";

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
            "{\"x\", \"y\", \"text\" (a \\n starts a new text one line below, as Return does in Text mode; {\"ids\"} lists them), \"size\"?: points (or \"font_size\"), \"font\"?: number, \"postscript\"?: bool, \"special\"?: bool (LaTeX text; unless postscript or font is given it leaves the PostScript font for the LaTeX default font), \"justify\"?: left|center|right, \"angle\"?: degrees, \"pen_color\"? (or \"color\"), \"depth\"?}",
            text
        ),
        shape_cmd!(
            "shape.picture",
            "Add picture",
            "{\"path\": string, \"x\", \"y\", \"width\"?, \"height\"?, \"keep_aspect\"?: bool}  — width/height default to the image size at 80 dpi; keep_aspect fits the picture into width x height without distorting it, at the corner (x, y)",
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

pub(crate) fn style_from(e: &mut Engine, p: &Value) -> Result<Style, EngineError> {
    let mut s = e.indicator.style();
    if let Some(v) = opt_num(p, "depth")? {
        s.depth = (v.round() as i32).clamp(0, rustfig_model::MAX_DEPTH);
    }
    if let Some(v) = opt_num(p, "thickness")? {
        s.thickness = (v.round() as i32).clamp(0, 10_000);
    }
    if let Some(v) = color_param(e, p, "pen_color")? {
        s.pen_color = ColorRef(v);
    }
    if let Some(v) = color_param(e, p, "fill_color")? {
        s.fill_color = ColorRef(v);
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
    if let Some(v) = cap_param(p)? {
        s.cap = v;
    }
    if let Some(v) = join_param(p)? {
        s.join = v;
    }
    Ok(s)
}

/// A color parameter: a color number (clamped to the color numbers), or
/// `#rrggbb`, `#rgb` or an X11 name as the color dialog takes them. A text
/// color is the standard or user color with that value, else a new user
/// color (part of the command's one undo step); a number must be a defined
/// user color or one already used in the figure (a file may refer to one it
/// does not define), otherwise fig2dev would only find out on export.
pub(super) fn color_param(e: &mut Engine, p: &Value, key: &str) -> Result<Option<i32>, EngineError> {
    if let Some(Value::String(text)) = super::param(p, key) {
        if text.chars().count() > 64 {
            return Err(EngineError::bad_params(format!("{key}: color text too long")));
        }
        let rgb = rustfig_model::x_color(text.trim()).ok_or_else(|| {
            EngineError::bad_params(format!(
                "{key} {text:?} is neither a color number, #rrggbb, #rgb nor an X11 color name"
            ))
        })?;
        let known = e.doc().colors.iter().any(|u| u.rgb == rgb);
        let c = e.doc_mut().color_for_rgb(rgb).ok_or_else(|| {
            EngineError::bad_params(format!(
                "{key} {text:?}: all 512 user colors are in use (reuse one, or free some with color.remove_unused)"
            ))
        })?;
        if !known {
            e.prefs.add_recent_color(&rgb.to_hex());
            e.prefs_dirty = true;
        }
        return Ok(Some(c.0));
    }
    let Some(v) = opt_num(p, key)? else { return Ok(None) };
    let c = (v.round() as i32).clamp(-1, rustfig_model::LAST_USER_COLOR);
    let doc = e.doc();
    if c >= rustfig_model::FIRST_USER_COLOR
        && !doc.colors.iter().any(|u| u.number == c)
        && !doc.used_colors().contains(&c)
    {
        return Err(EngineError::bad_params(format!(
            "{key} {c}: no user color {c} in this figure (define it with color.add)"
        )));
    }
    Ok(Some(c))
}

pub(super) fn cap_param(p: &Value) -> Result<Option<CapStyle>, EngineError> {
    Ok(match opt_str(p, "cap")? {
        Some("butt") => Some(CapStyle::Butt),
        Some("round") => Some(CapStyle::Round),
        Some("projecting") => Some(CapStyle::Projecting),
        Some(o) => {
            return Err(EngineError::bad_params(format!(
                "unknown cap {o:?} (butt, round, projecting)"
            )))
        }
        None => None,
    })
}

pub(super) fn join_param(p: &Value) -> Result<Option<JoinStyle>, EngineError> {
    Ok(match opt_str(p, "join")? {
        Some("miter") => Some(JoinStyle::Miter),
        Some("round") => Some(JoinStyle::Round),
        Some("bevel") => Some(JoinStyle::Bevel),
        Some(o) => {
            return Err(EngineError::bad_params(format!(
                "unknown join {o:?} (miter, round, bevel)"
            )))
        }
        None => None,
    })
}

/// An arrow style: `hollow`/`filled` or Fig's 0/1.
pub(super) fn arrow_style_value(v: &Value, key: &str) -> Result<i32, EngineError> {
    match v {
        Value::String(n) if n == "hollow" => Ok(0),
        Value::String(n) if n == "filled" => Ok(1),
        Value::Number(n) => match n.as_f64().map(f64::round) {
            Some(x) if x == 0.0 || x == 1.0 => Ok(x as i32),
            _ => Err(EngineError::bad_params(format!(
                "{key} must be hollow or filled (0 or 1)"
            ))),
        },
        _ => Err(EngineError::bad_params(format!(
            "{key} must be hollow or filled (0 or 1)"
        ))),
    }
}

/// The arrowhead names, by Fig's arrow types 0 to 14 (xfig's `arrow_shapes`):
/// the name, the type and the style the name implies (a name without one
/// leaves the style to `arrow_style` and the indicator).
pub const ARROW_NAMES: [(&str, i32, Option<i32>); 18] = [
    ("stick", 0, None),
    ("triangle", 1, None),
    ("concave", 2, None),
    ("convex", 3, None),
    ("diamond", 4, None),
    ("circle", 5, Some(0)),
    ("dot", 5, Some(1)),
    ("half_circle", 6, None),
    ("square", 7, None),
    ("reverse_triangle", 8, None),
    ("half_filled_concave", 9, None),
    ("half_triangle", 10, None),
    ("half_concave", 11, None),
    ("half_convex", 12, None),
    ("wye", 13, Some(0)),
    ("bar", 13, Some(1)),
    ("fork", 14, Some(0)),
    ("backward_fork", 14, Some(1)),
];

/// An arrow type: a number 0-14, or a name from [`ARROW_NAMES`]; returns the
/// type and the style the name implies.
pub(super) fn arrow_kind_value(v: &Value, key: &str) -> Result<(i32, Option<i32>), EngineError> {
    match v {
        Value::Number(n) => match n.as_f64() {
            Some(x) if x.is_finite() => Ok(((x.round() as i32).clamp(0, 14), None)),
            _ => Err(EngineError::bad_params(format!(
                "{key} must be a number 0-14 or a name"
            ))),
        },
        Value::String(name) => ARROW_NAMES
            .iter()
            .find(|(n, _, _)| n.eq_ignore_ascii_case(name.trim()))
            .map(|&(_, k, st)| (k, st))
            .ok_or_else(|| {
                let names: Vec<&str> = ARROW_NAMES.iter().map(|a| a.0).collect();
                EngineError::bad_params(format!(
                    "{key} {name:?}: unknown arrow type (0-14, or {})",
                    names.join(", ")
                ))
            }),
        _ => Err(EngineError::bad_params(format!(
            "{key} must be a number 0-14 or a name"
        ))),
    }
}

/// The per-arrow changes `arrow_kind`, `arrow_style`, `arrow_width`,
/// `arrow_height` and `arrow_thickness` (Fig units) take, for both arrows.
#[derive(Default)]
pub(super) struct ArrowChanges {
    kind: Option<i32>,
    style: Option<i32>,
    width: Option<f64>,
    height: Option<f64>,
    thickness: Option<f64>,
}

impl ArrowChanges {
    pub(super) fn from_params(p: &Value) -> Result<Self, EngineError> {
        let named = super::param(p, "arrow_kind")
            .filter(|v| !v.is_null())
            .map(|v| arrow_kind_value(v, "arrow_kind"))
            .transpose()?;
        let explicit = super::param(p, "arrow_style")
            .map(|v| arrow_style_value(v, "arrow_style"))
            .transpose()?;
        Ok(Self {
            kind: named.map(|n| n.0),
            style: explicit.or(named.and_then(|n| n.1)),
            width: opt_num(p, "arrow_width")?.map(|v| v.clamp(1.0, 1.0e5)),
            height: opt_num(p, "arrow_height")?.map(|v| v.clamp(1.0, 1.0e5)),
            thickness: opt_num(p, "arrow_thickness")?.map(|v| v.clamp(0.0, 1.0e4)),
        })
    }

    pub(super) fn apply(&self, a: &mut rustfig_model::Arrow) {
        if let Some(v) = self.kind {
            a.kind = v;
        }
        if let Some(v) = self.style {
            a.style = v;
        }
        if let Some(v) = self.width {
            a.width = v;
        }
        if let Some(v) = self.height {
            a.height = v;
        }
        if let Some(v) = self.thickness {
            a.thickness = v;
        }
    }
}

/// `{"kind", "style", "thickness", "width", "height"}` (width and height in
/// Fig units) for one arrow, as the edit panel's arrow rows give it; `style`
/// also takes `hollow`/`filled`.
pub(super) fn arrow_spec(p: &Value, key: &str) -> Result<Option<rustfig_model::Arrow>, EngineError> {
    let Some(v) = super::param(p, key) else { return Ok(None) };
    if !v.is_object() {
        return Err(EngineError::bad_params(format!(
            "{key} must be an object {{kind, style, thickness, width, height}}"
        )));
    }
    let f = |k: &str, lo: f64, hi: f64| -> Result<f64, EngineError> {
        v.get(k)
            .and_then(Value::as_f64)
            .filter(|x| x.is_finite())
            .map(|x| x.clamp(lo, hi))
            .ok_or_else(|| EngineError::bad_params(format!("{key}.{k} must be a number")))
    };
    let kind = match v.get("kind") {
        Some(k) => arrow_kind_value(k, &format!("{key}.kind"))?,
        None => {
            return Err(EngineError::bad_params(format!(
                "{key}.kind must be a number 0-14 or a name"
            )))
        }
    };
    let style = match v.get("style") {
        Some(s) => arrow_style_value(s, &format!("{key}.style"))?,
        None if kind.1.is_some() => kind.1.unwrap_or(0),
        None => {
            return Err(EngineError::bad_params(format!(
                "{key}.style must be hollow or filled (0 or 1)"
            )))
        }
    };
    Ok(Some(rustfig_model::Arrow {
        kind: kind.0,
        style,
        thickness: f("thickness", 0.0, 1.0e4)?,
        width: f("width", 0.0, 1.0e5)?,
        height: f("height", 0.0, 1.0e5)?,
    }))
}

pub(super) fn arrows_from(
    e: &Engine,
    p: &Value,
) -> Result<(Option<rustfig_model::Arrow>, Option<rustfig_model::Arrow>), EngineError> {
    let (mut f, mut b) = e.indicator.arrows();
    let template = e.indicator.arrows();
    let a = e.indicator.arrow_value();
    let fwd_on = opt_bool(p, "forward_arrow")?;
    if let Some(on) = fwd_on {
        f = if on { template.0.or(Some(a)) } else { None };
    }
    let back_on = opt_bool(p, "backward_arrow")?;
    if let Some(on) = back_on {
        b = if on { template.1.or(Some(a)) } else { None };
    }
    let changes = ArrowChanges::from_params(p)?;
    for arrow in [&mut f, &mut b].into_iter().flatten() {
        changes.apply(arrow);
    }
    // A spec turns its arrow on, unless that arrow is switched off.
    if let Some(spec) = arrow_spec(p, "forward_arrow_spec")? {
        if fwd_on != Some(false) {
            f = Some(spec);
        }
    }
    if let Some(spec) = arrow_spec(p, "backward_arrow_spec")? {
        if back_on != Some(false) {
            b = Some(spec);
        }
    }
    Ok((f, b))
}

pub(super) fn added(e: &mut Engine, o: Object) -> CmdResult {
    let id = e.add_object(o);
    e.set_selection(vec![id]);
    Ok(json!({"id": id.0}))
}

fn rectangle(e: &mut Engine, p: &Value) -> CmdResult {
    let (x, y, w, h) = (num(p, "x")?, num(p, "y")?, num(p, "width")?, num(p, "height")?);
    // xfig's `create_boxobject`: a box without area is refused (the Fig
    // reader drops it, so it would vanish on save). Fig stores whole units.
    if x.round() == (x + w).round() || y.round() == (y + h).round() {
        return Err(EngineError::bad_params("Box must have area"));
    }
    let style = style_from(e, p)?;
    let mut pl = Polyline::rect(style, Point::new(x, y), Point::new(x + w, y + h));
    if let Some(r) = opt_num(p, "radius")? {
        pl.kind = PolyKind::ArcBox;
        pl.radius = (r.round() as i32).clamp(0, 100_000);
    }
    added(e, Object::new(ObjectKind::Polyline(pl)))
}

/// The vertices of xfig's regular polygon (`create_regpoly`, `d_regpoly.c`):
/// the point clicked last first, then one vertex per step of `2 pi / sides`
/// against the angle of that point seen from the center (y down), each
/// rounded to whole Fig units like xfig's integers.
pub(crate) fn xfig_regular_polygon(center: Point, first: Point, sides: usize) -> Vec<Point> {
    let n = sides.clamp(3, 1000);
    let d = first - center;
    let mag = d.x.hypot(d.y);
    let init = d.y.atan2(d.x).rem_euclid(std::f64::consts::TAU);
    let mut pts = vec![first];
    for i in 1..n {
        let angle = (init - std::f64::consts::TAU * i as f64 / n as f64).rem_euclid(std::f64::consts::TAU);
        pts.push(Point::new(
            center.x + (mag * angle.cos()).round(),
            center.y + (mag * angle.sin()).round(),
        ));
    }
    pts
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
    // xfig deletes an ellipse without a radius when it reads the file.
    if rx.abs() < 1.0 || ry.abs() < 1.0 {
        return Err(EngineError::bad_params(
            "an ellipse needs two radii of at least 1 Fig unit",
        ));
    }
    let angle = num_or(p, "angle", 0.0)?.to_radians();
    let style = style_from(e, p)?;
    added(e, make_ellipse(style, c, rx, ry, angle, EllipseKind::EllipseRadii))
}

fn circle(e: &mut Engine, p: &Value) -> CmdResult {
    let c = Point::new(num(p, "cx")?, num(p, "cy")?);
    let r = num(p, "r")?;
    if r.abs() < 1.0 {
        return Err(EngineError::bad_params(
            "a circle needs a radius of at least 1 Fig unit",
        ));
    }
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
    if let Some(Value::Array(a)) = super::param(p, "shapes") {
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
    // A polygon of zero size has no shape (and the Fig reader drops it).
    if !r.is_finite() || r.abs() < 1.0 {
        return Err(EngineError::bad_params("the radius must be at least 1"));
    }
    let sides = num_or(p, "sides", e.indicator.sides as f64)?.round().clamp(3.0, 1000.0) as usize;
    let a = num_or(p, "angle", -90.0)?.to_radians();
    let first = Point::new(c.x + r * a.cos(), c.y + r * a.sin());
    let mut pl = Polyline::new(
        PolyKind::Polygon,
        style_from(e, p)?,
        xfig_regular_polygon(c, first, sides),
    );
    pl.radius = 0;
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
        local_font: ind.local_font.clone(),
    }
}

fn text(e: &mut Engine, p: &Value) -> CmdResult {
    let origin = Point::new(num(p, "x")?, num(p, "y")?);
    let s = string(p, "text")?;
    if s.len() > rustfig_format::MAX_TEXT_BYTES {
        return Err(EngineError::bad_params("text is too long"));
    }
    let mut t = make_text(e, origin, s);
    // (`font_size`: the name `object.set_attributes` also takes.)
    if let Some(v) = opt_num(p, "size")?.or(opt_num(p, "font_size")?) {
        t.size = v.clamp(1.0, 2000.0);
    }
    if let Some(v) = opt_num(p, "font")? {
        t.font = (v.round() as i32).clamp(-1, 34);
        t.local_font = None;
    }
    if let Some(ps) = opt_bool(p, "postscript")? {
        if ps {
            t.flags |= rustfig_model::text_flags::POSTSCRIPT;
        } else {
            t.flags &= !rustfig_model::text_flags::POSTSCRIPT;
        }
    }
    if let Some(special) = opt_bool(p, "special")? {
        latex_text(&mut t, special, p)?;
    }
    if let Some(j) = opt_str(p, "justify")? {
        t.justify = parse_justify(j)?;
    }
    if let Some(a) = opt_num(p, "angle")? {
        t.angle = a.to_radians();
    }
    // `pen_color` as for the other shapes and `object.set_attributes`;
    // `color` is the older name.
    if let Some(c) = color_param(e, p, "pen_color")?.or(color_param(e, p, "color")?) {
        t.color = ColorRef(c);
    }
    if let Some(d) = opt_num(p, "depth")? {
        t.depth = (d.round() as i32).clamp(0, rustfig_model::MAX_DEPTH);
    }
    if !t.text.contains('\n') {
        return added(e, Object::new(ObjectKind::Text(t)));
    }
    // One text per line, as Return does in Text mode: separate objects (not
    // glued), each line one step (`text_step` of the line height) below the
    // one before, along the text's angle. An empty line only leaves a gap.
    let lines: Vec<String> = t.text.lines().map(str::to_string).collect();
    if lines.len() > MAX_TEXT_LINES {
        return Err(EngineError::bad_params(format!(
            "a text of at most {MAX_TEXT_LINES} lines"
        )));
    }
    let step = if e.indicator.text_step.is_finite() {
        e.indicator.text_step.clamp(0.0, 100.0)
    } else {
        1.0
    };
    let down = rustfig_geom::Affine::rotate(t.angle).apply_vec(Point::new(
        0.0,
        t.size * rustfig_geom::FIG_PER_POINT * rustfig_text::LINE_SPACING * step,
    ));
    let mut ids = Vec::new();
    for (i, line) in lines.into_iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        let mut l = t.clone();
        l.text = line;
        l.origin = origin + down * i as f64;
        ids.push(e.add_object(Object::new(ObjectKind::Text(l))));
    }
    if ids.is_empty() {
        return Err(EngineError::bad_params("the text has no characters"));
    }
    e.set_selection(ids.clone());
    Ok(json!({"id": ids[0].0, "ids": super::ids_value(&ids)}))
}

/// The most lines `shape.text` writes as separate texts.
const MAX_TEXT_LINES: usize = 1000;

/// Sets or clears `t`'s special (LaTeX) flag. Turned on without an explicit
/// `postscript` it also turns off the PostScript font, and without an
/// explicit `font` picks the LaTeX default font: with a PostScript font,
/// fig2dev's LaTeX drivers set the text in it (Times for font 0).
pub(crate) fn latex_text(t: &mut rustfig_model::Text, special: bool, p: &Value) -> Result<(), EngineError> {
    use rustfig_model::text_flags::{POSTSCRIPT, SPECIAL};
    if !special {
        t.flags &= !SPECIAL;
        return Ok(());
    }
    t.flags |= SPECIAL;
    if opt_bool(p, "postscript")?.is_none() && t.flags & POSTSCRIPT != 0 {
        t.flags &= !POSTSCRIPT;
        if opt_num(p, "font")?.is_none() {
            t.font = 0;
            t.local_font = None;
        }
    }
    Ok(())
}

fn picture(e: &mut Engine, p: &Value) -> CmdResult {
    let path = string(p, "path")?;
    let img = import::load_picture(std::path::Path::new(&path)).map_err(EngineError::io)?;
    let (x, y) = (num(p, "x")?, num(p, "y")?);
    // Default size: 80 pixels per inch, like xfig's screen resolution.
    let w = opt_num(p, "width")?.unwrap_or(f64::from(img.width) * 15.0);
    let h = opt_num(p, "height")?.unwrap_or(f64::from(img.height) * 15.0);
    let (w, h) = if opt_bool(p, "keep_aspect")?.unwrap_or(false) && img.width > 0 && img.height > 0 {
        // Fit inside the box, keeping the picture's proportions.
        let k = (w.abs() / f64::from(img.width)).min(h.abs() / f64::from(img.height));
        let sign = |v: f64| if v < 0.0 { -1.0 } else { 1.0 };
        (f64::from(img.width) * k * sign(w), f64::from(img.height) * k * sign(h))
    } else {
        (w, h)
    };
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
