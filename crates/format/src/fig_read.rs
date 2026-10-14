//! Reader for the Fig format, versions 3.0–3.2.
//
// Reading follows xfig src/f_read.c and fig2dev read.c, for example the
// optional centring line (xfig licence, see LICENSE-xfig).

use crate::FormatError;
use rustfig_geom::Point;
use rustfig_model::{
    Arc, Arrow, CapStyle, ColorRef, Compound, DocSettings, Document, Ellipse, EllipseKind, JoinStyle, Justify,
    LineKind, Object, ObjectKind, Orientation, PaperSize, Picture, PolyKind, Polyline, Rgb, Spline, SplineKind, Style,
    Text, Units, UserColor, FIRST_USER_COLOR, LAST_USER_COLOR,
};

/// Largest point count accepted for one object.
pub const MAX_POINTS: usize = 5_000_000;
/// Deepest compound nesting accepted.
pub const MAX_NESTING: usize = 200;
/// Largest text string accepted, in bytes.
pub const MAX_TEXT_BYTES: usize = 1 << 20;

struct Reader<'a> {
    lines: Vec<&'a str>,
    line: usize,
    col: usize,
    /// Minor version (0, 1 or 2 for 3.x).
    minor: u32,
    utf8: bool,
    scale: f64,
}

type R<T> = Result<T, FormatError>;

impl<'a> Reader<'a> {
    fn err<T>(&self, msg: impl Into<String>) -> R<T> {
        Err(FormatError::at(self.line + 1, msg))
    }

    fn cur(&self) -> Option<&'a str> {
        self.lines.get(self.line).copied()
    }

    /// Rest of the current line after the cursor.
    fn rest(&self) -> &'a str {
        self.cur().and_then(|l| l.get(self.col..)).unwrap_or("")
    }

    fn next_line(&mut self) {
        self.line += 1;
        self.col = 0;
    }

    /// Next whitespace-separated token, skipping blank and comment lines.
    fn token(&mut self) -> R<&'a str> {
        loop {
            let Some(line) = self.cur() else {
                return self.err("unexpected end of file");
            };
            let rest = line.get(self.col..).unwrap_or("");
            if self.col == 0 && rest.starts_with('#') {
                self.next_line();
                continue;
            }
            let trimmed = rest.trim_start();
            if trimmed.is_empty() {
                self.next_line();
                continue;
            }
            let start = self.col + (rest.len() - trimmed.len());
            let len = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
            self.col = start + len;
            return Ok(trimmed.get(..len).unwrap_or(""));
        }
    }

    /// Like [`token`] but returns `None` at end of file.
    fn try_token(&mut self) -> Option<&'a str> {
        let save = (self.line, self.col);
        match self.token() {
            Ok(t) => Some(t),
            Err(_) => {
                (self.line, self.col) = save;
                None
            }
        }
    }

    fn float(&mut self) -> R<f64> {
        let t = self.token()?;
        match t.parse::<f64>() {
            Ok(v) if v.is_finite() => Ok(v),
            _ => self.err(format!("expected a number, found {t:?}")),
        }
    }

    fn int(&mut self) -> R<i32> {
        let v = self.float()?;
        if v.abs() > 2.0e9 {
            return self.err(format!("number out of range: {v}"));
        }
        Ok(v.round() as i32)
    }

    fn coord(&mut self) -> R<f64> {
        Ok(self.float()? * self.scale)
    }

    fn point(&mut self) -> R<Point> {
        Ok(Point::new(self.coord()?, self.coord()?))
    }

    /// Consume comment lines at the cursor (at an object boundary).
    fn comments(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        if self.col > 0 {
            if !self.rest().trim().is_empty() {
                return out;
            }
            self.next_line();
        }
        while let Some(line) = self.cur() {
            if let Some(c) = line.strip_prefix('#') {
                out.push(c.strip_prefix(' ').unwrap_or(c).to_string());
                self.next_line();
            } else if line.trim().is_empty() {
                self.next_line();
            } else {
                break;
            }
        }
        out
    }

    fn style_common(&mut self) -> R<Style> {
        // line_style thickness pen_color fill_color depth pen_style area_fill style_val
        let line = LineKind::from_fig(self.int()?);
        let thickness = self.int()?.clamp(0, 10_000);
        let pen_color = ColorRef(self.int()?);
        let fill_color = ColorRef(self.int()?);
        let depth = self.int()?.clamp(0, rustfig_model::MAX_DEPTH);
        let pen_style = self.int()?;
        let area_fill = self.int()?.clamp(-1, 62);
        let style_val = self.float()?;
        Ok(Style {
            line,
            thickness,
            pen_color,
            fill_color,
            depth,
            pen_style,
            area_fill,
            style_val,
            cap: CapStyle::Butt,
            join: JoinStyle::Miter,
        })
    }

    fn arrow(&mut self) -> R<Arrow> {
        Ok(Arrow {
            kind: self.int()?.clamp(0, 30),
            style: self.int()?.clamp(0, 1),
            thickness: self.float()?.clamp(0.0, 10_000.0),
            width: (self.float()? * self.scale).abs().min(1.0e7),
            height: (self.float()? * self.scale).abs().min(1.0e7),
        })
    }

    fn arrows(&mut self, fwd: bool, back: bool) -> R<(Option<Arrow>, Option<Arrow>)> {
        let f = if fwd { Some(self.arrow()?) } else { None };
        let b = if back { Some(self.arrow()?) } else { None };
        Ok((f, b))
    }

    fn npoints(&mut self) -> R<usize> {
        let n = self.int()?;
        if n < 0 || n as usize > MAX_POINTS {
            return self.err(format!("bad point count {n}"));
        }
        Ok(n as usize)
    }

    fn points(&mut self, n: usize) -> R<Vec<Point>> {
        let mut v = Vec::with_capacity(n.min(4096));
        for _ in 0..n {
            v.push(self.point()?);
        }
        Ok(v)
    }

    fn ellipse(&mut self) -> R<ObjectKind> {
        let kind = EllipseKind::from_fig(self.int()?);
        let style = self.style_common()?;
        let direction = self.int()?;
        let angle = self.float()?;
        let center = self.point()?;
        let radii = self.point()?;
        let start = self.point()?;
        let end = self.point()?;
        Ok(ObjectKind::Ellipse(Ellipse {
            kind,
            style,
            direction,
            angle,
            center,
            radii: Point::new(radii.x.abs(), radii.y.abs()),
            start,
            end,
        }))
    }

    fn polyline(&mut self) -> R<ObjectKind> {
        let kind = PolyKind::from_fig(self.int()?);
        let mut style = self.style_common()?;
        style.join = JoinStyle::from_fig(self.int()?);
        style.cap = CapStyle::from_fig(self.int()?);
        let radius = self.int()?.clamp(0, 100_000);
        let fwd = self.int()? != 0;
        let back = self.int()? != 0;
        let n = self.npoints()?;
        let (forward_arrow, backward_arrow) = self.arrows(fwd, back)?;
        let picture = if kind == PolyKind::Picture {
            let flipped = self.int()? != 0;
            let file = self.rest().trim().to_string();
            self.next_line();
            Some(Picture {
                flipped,
                file,
                image: None,
            })
        } else {
            None
        };
        let mut points = self.points(n)?;
        if kind.is_closed() && points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        if kind.is_rect() && !points.is_empty() {
            let r = rustfig_geom::Rect::from_points(points.iter().copied());
            points = r.corners().to_vec();
        }
        Ok(ObjectKind::Polyline(Polyline {
            kind,
            style,
            radius,
            forward_arrow,
            backward_arrow,
            points,
            picture,
        }))
    }

    fn spline(&mut self) -> R<ObjectKind> {
        let sub = self.int()?;
        let mut style = self.style_common()?;
        style.cap = CapStyle::from_fig(self.int()?);
        let fwd = self.int()? != 0;
        let back = self.int()? != 0;
        let n = self.npoints()?;
        let (forward_arrow, backward_arrow) = self.arrows(fwd, back)?;
        let mut points = self.points(n)?;
        let closed = sub % 2 == 1;
        let kind = match sub {
            0 | 1 => SplineKind::Approximated,
            2 | 3 => SplineKind::Interpolated,
            _ => SplineKind::X,
        };
        let shapes = if self.minor >= 2 {
            let mut s = Vec::with_capacity(n.min(4096));
            for _ in 0..n {
                s.push(self.float()?.clamp(-1.0, 1.0));
            }
            s
        } else {
            if kind == SplineKind::Interpolated {
                // Pre-3.2 Bezier control points: two per knot, not needed.
                for _ in 0..n {
                    for _ in 0..4 {
                        self.float()?;
                    }
                }
            }
            let mut s = vec![kind.default_shape(); n];
            if !closed {
                if let Some(f) = s.first_mut() {
                    *f = 0.0;
                }
                if let Some(l) = s.last_mut() {
                    *l = 0.0;
                }
            }
            s
        };
        if closed && points.len() > 2 && points.first() == points.last() && self.minor < 2 {
            points.pop();
        }
        let mut shapes = shapes;
        shapes.truncate(points.len());
        Ok(ObjectKind::Spline(Spline {
            kind,
            closed,
            style,
            forward_arrow,
            backward_arrow,
            points,
            shapes,
        }))
    }

    fn arc(&mut self) -> R<ObjectKind> {
        let sub = self.int()?;
        let mut style = self.style_common()?;
        style.cap = CapStyle::from_fig(self.int()?);
        let direction = self.int()?;
        let fwd = self.int()? != 0;
        let back = self.int()? != 0;
        let center = self.point()?;
        let p1 = self.point()?;
        let p2 = self.point()?;
        let p3 = self.point()?;
        let (forward_arrow, backward_arrow) = self.arrows(fwd, back)?;
        let mut arc = Arc {
            pie: sub == 2,
            style,
            ccw: direction == 1,
            forward_arrow,
            backward_arrow,
            center,
            points: [p1, p2, p3],
        };
        arc.refit();
        Ok(ObjectKind::Arc(arc))
    }

    /// The text string: after one blank following the last number, up to `\001`.
    fn text_string(&mut self) -> R<String> {
        let mut bytes: Vec<u8> = Vec::new();
        // Skip exactly one blank.
        let mut rest = self.rest();
        if let Some(r) = rest.strip_prefix([' ', '\t']) {
            rest = r;
        }
        loop {
            let mut chars = rest.char_indices().peekable();
            while let Some((i, c)) = chars.next() {
                if bytes.len() > MAX_TEXT_BYTES {
                    return self.err("text string too long");
                }
                if c == '\u{1}' {
                    self.col = self.cur().map(|l| l.len()).unwrap_or(0);
                    return Ok(String::from_utf8_lossy(&bytes).into_owned());
                }
                if c != '\\' {
                    let mut buf = [0u8; 4];
                    bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                    continue;
                }
                let tail = rest.get(i + 1..).unwrap_or("");
                if tail.starts_with('\\') {
                    bytes.push(b'\\');
                    chars.next();
                    continue;
                }
                let oct: Vec<u8> = tail.bytes().take(3).collect();
                if oct.len() == 3 && oct.iter().all(|b| (b'0'..=b'7').contains(b)) {
                    let v = oct.iter().fold(0u32, |a, b| a * 8 + u32::from(b - b'0'));
                    for _ in 0..3 {
                        chars.next();
                    }
                    if v == 1 {
                        self.col = self.cur().map(|l| l.len()).unwrap_or(0);
                        return Ok(String::from_utf8_lossy(&bytes).into_owned());
                    }
                    let v = (v & 0xff) as u8;
                    if self.utf8 {
                        bytes.push(v);
                    } else {
                        let mut buf = [0u8; 4];
                        bytes.extend_from_slice(char::from(v).encode_utf8(&mut buf).as_bytes());
                    }
                    continue;
                }
                bytes.push(b'\\');
            }
            // String continues on the next line.
            self.next_line();
            match self.cur() {
                Some(l) => {
                    bytes.push(b'\n');
                    rest = l;
                }
                None => return self.err("unterminated text string"),
            }
        }
    }

    fn text(&mut self) -> R<ObjectKind> {
        let justify = Justify::from_fig(self.int()?);
        let color = ColorRef(self.int()?);
        let depth = self.int()?.clamp(0, rustfig_model::MAX_DEPTH);
        let pen_style = self.int()?;
        let font = self.int()?;
        let size = self.float()?.clamp(0.1, 5000.0);
        let angle = self.float()?;
        let flags = self.int()?;
        let height = self.float()? * self.scale;
        let length = self.float()? * self.scale;
        let origin = self.point()?;
        let text = self.text_string()?;
        Ok(ObjectKind::Text(Text {
            justify,
            color,
            depth,
            pen_style,
            font,
            size,
            angle,
            flags,
            height,
            length,
            origin,
            text,
        }))
    }

    fn objects(&mut self, doc: &mut Document, nesting: usize) -> R<Vec<Object>> {
        if nesting > MAX_NESTING {
            return self.err("compounds nested too deeply");
        }
        let mut out = Vec::new();
        loop {
            let comments = self.comments();
            let Some(code) = self.try_token() else {
                if nesting > 0 {
                    return self.err("missing end of compound (-6)");
                }
                return Ok(out);
            };
            let kind = match code {
                "0" => {
                    let num = self.int()?;
                    let hex = self.token()?;
                    let Some(rgb) = Rgb::from_hex(hex) else {
                        return self.err(format!("bad colour {hex:?}"));
                    };
                    if (FIRST_USER_COLOR..=LAST_USER_COLOR).contains(&num) {
                        doc.colors.retain(|c| c.number != num);
                        doc.colors.push(UserColor { number: num, rgb });
                    }
                    continue;
                }
                "1" => self.ellipse()?,
                "2" => self.polyline()?,
                "3" => self.spline()?,
                "4" => self.text()?,
                "5" => self.arc()?,
                "6" => {
                    for _ in 0..4 {
                        self.float()?;
                    }
                    let children = self.objects(doc, nesting + 1)?;
                    ObjectKind::Compound(Compound { objects: children })
                }
                "-6" => {
                    if nesting == 0 {
                        // Stray end marker: ignore it.
                        continue;
                    }
                    return Ok(out);
                }
                other => return self.err(format!("unknown object code {other:?}")),
            };
            let mut o = Object::new(kind);
            o.comments = comments;
            out.push(o);
        }
    }
}

/// Decode file bytes: UTF-8 if valid, Latin-1 otherwise.
pub(crate) fn decode(src: &[u8]) -> (String, bool) {
    match std::str::from_utf8(src) {
        Ok(s) => (s.to_string(), true),
        Err(_) => (src.iter().map(|&b| char::from(b)).collect(), false),
    }
}

/// Parse a Fig file (versions 3.0 to 3.2).
pub fn read_fig(src: &[u8]) -> Result<Document, FormatError> {
    let (text, valid_utf8) = decode(src);
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = text.split('\n').collect();
    let first = lines.first().copied().unwrap_or("");
    let Some(ver) = first.strip_prefix("#FIG") else {
        return Err(FormatError::at(1, "not a Fig file (missing #FIG header)"));
    };
    let ver = ver.split_whitespace().next().unwrap_or("");
    let (major, minor) = match ver.split_once('.') {
        Some((a, b)) => (a.parse::<u32>().unwrap_or(0), b.parse::<u32>().unwrap_or(0)),
        None => (ver.parse::<u32>().unwrap_or(0), 0),
    };
    if major != 3 {
        return Err(FormatError::at(
            1,
            format!("Fig version {ver} is not supported (only 3.0–3.2); convert it with fig2dev -L fig first"),
        ));
    }
    let mut r = Reader {
        lines,
        line: 1,
        col: 0,
        minor,
        utf8: false,
        scale: 1.0,
    };
    let mut doc = Document::new();
    // Encoding line and other leading comments.
    while let Some(l) = r.cur() {
        if let Some(c) = l.strip_prefix('#') {
            if c.trim().eq_ignore_ascii_case("encoding: UTF-8") {
                r.utf8 = true;
            }
            r.next_line();
        } else if l.trim().is_empty() {
            r.next_line();
        } else {
            break;
        }
    }
    if !valid_utf8 {
        r.utf8 = false;
    }
    let mut settings = DocSettings::default();
    let header_line = |r: &mut Reader| -> R<String> {
        loop {
            let Some(l) = r.cur() else {
                return r.err("unexpected end of file in header");
            };
            r.next_line();
            if !l.trim().is_empty() && !l.starts_with('#') {
                return Ok(l.trim().to_string());
            }
        }
    };
    let orient = header_line(&mut r)?;
    settings.orientation = if orient.eq_ignore_ascii_case("portrait") {
        Orientation::Portrait
    } else {
        Orientation::Landscape
    };
    // The centring line may be missing; fig2dev then reads the units line
    // here (read.c `read_objects`).
    let just = header_line(&mut r)?;
    let lower = just.to_ascii_lowercase();
    let units = if lower.starts_with("center") || lower.starts_with("flush") {
        settings.center = !lower.starts_with("flush");
        header_line(&mut r)?
    } else {
        just
    };
    settings.units = if units.to_ascii_lowercase().starts_with("metric") {
        Units::Metric
    } else {
        Units::Inches
    };
    if minor >= 2 {
        let paper = header_line(&mut r)?;
        settings.paper = PaperSize::from_name(&paper).unwrap_or_default();
        let mag = header_line(&mut r)?;
        settings.magnification = mag
            .parse::<f64>()
            .ok()
            .filter(|m| m.is_finite() && *m > 0.0)
            .unwrap_or(100.0)
            .min(100_000.0);
        let multi = header_line(&mut r)?;
        settings.multiple_pages = multi.eq_ignore_ascii_case("multiple");
        let tc = header_line(&mut r)?;
        settings.transparent_color = tc.parse::<i32>().unwrap_or(-2);
    }
    // Whole-figure comments, then "resolution coord_system".
    doc.comments = r.comments();
    let res = r.float()?;
    let _coord_system = r.int()?;
    if !(1.0..=1.0e6).contains(&res) {
        return r.err(format!("bad resolution {res}"));
    }
    r.scale = 1200.0 / res;
    // The rest of the resolution line is ignored.
    r.next_line();
    doc.settings = settings;
    doc.objects = r.objects(&mut doc, 0)?.into_iter().map(Into::into).collect();
    doc.assign_ids();
    Ok(doc)
}
