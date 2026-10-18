//! fig2dev's `map` driver: an HTML client-side image map. An object whose
//! comment reads `HREF="url" ALT="text"` becomes a link area (boxes as
//! rectangles, circles as circles, polylines, polygons, ellipses and arcs
//! as polygons), shallower objects first; a figure comment is the link of
//! the whole figure. Coordinates are pixels at 80 per inch, plus the
//! border margin `-b`. Splines and texts can't be link areas.
//
// Ported from fig2dev dev/genmap.c (xfig licence, see LICENSE-xfig).

use crate::gendev::{Ctx, Driver};
use crate::object::{
    Arc, Compound, Ellipse, Line, Spline, Text, T_ARC_BOX, T_BOX, T_PIC_BOX, T_PIE_WEDGE_ARC, T_POLYGON, T_POLYLINE,
};
use crate::util::textconvert::{byte_view, from_byte_view};
use crate::{fprintf, round, Out};
use std::f64::consts::PI;
// Writing to a String can't fail, so the results of write! are ignored.
use std::fmt::Write as _;

/// The driver for `lang`.
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    let _ = lang;
    Some(Box::new(Map::default()))
}

/// Fig units per pixel.
const FSCALE: f64 = 15.0;
/// Closer points of a polyline are left out.
const MIN_DIST: i32 = 5;
const ARC_STEP: f64 = PI / 9.0;
/// Polygons of arcs and ellipses are made slightly larger.
const ARC_EXPAND: f64 = 1.02;
/// fig2dev's buffer for one area; longer polylines are ignored.
const BUF_SIZE: usize = 2000;

/// A link (`hlink`).
#[derive(Debug)]
struct Link {
    url: String,
    alt: String,
    area: String,
}

#[derive(Debug, Default)]
struct Map {
    border_margin: i32,
    /// In the order added; written last first.
    links: Vec<Link>,
    mag: f64,
    llx: i32,
    lly: i32,
}

/// `isspace` in the C locale.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r')
}

/// `isgraph` in the C locale.
fn is_graph(c: char) -> bool {
    c.is_ascii_graphic()
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| s.get(prefix.len()..))
        .flatten()
}

/// A link found in an object's comments (`is_link`): the URL, the ALT
/// text, and the comment (from its first non-space), which goes into the
/// area tag as it is.
fn is_link(ctx: &mut Ctx, comments: &[crate::object::Comment]) -> Option<(String, String, String)> {
    for comment in comments {
        // One character per byte: written back unchanged by `end`.
        let comment = byte_view(&comment.raw);
        let cp1 = comment.trim_start_matches(is_space);
        let Some(rest) = strip_prefix_ci(cp1, "HREF=") else {
            continue;
        };
        let rest = rest.trim_start_matches(is_space);
        let url_end = rest.find(|c| !is_graph(c)).unwrap_or(rest.len());
        let url = rest.get(..url_end).unwrap_or_default().to_string();
        let rest = rest.get(url_end..).unwrap_or_default().trim_start_matches(is_space);
        let mut alt = String::new();
        let mut have_alt = false;
        if !rest.is_empty() {
            if let Some(a) = strip_prefix_ci(rest, "ALT=") {
                have_alt = true;
                let a = a.trim_start_matches(is_space);
                alt = if let Some(q) = a.strip_prefix('"') {
                    q.split('"').next().unwrap_or_default().to_string()
                } else {
                    a.chars().take_while(|&c| is_graph(c)).collect()
                };
            } else {
                ctx.warn(format!("fig2dev(map): unknown attribute: {rest}"));
            }
        }
        if !have_alt {
            ctx.warn(format!("fig2dev(map): ALT is required in HTML 3.2: {cp1}"));
        }
        return Some((url, alt, cp1.to_string()));
    }
    None
}

impl Map {
    fn xzoom(&self, x: f64) -> i32 {
        round(self.mag * (x - f64::from(self.llx)) / FSCALE + f64::from(self.border_margin))
    }
    fn yzoom(&self, y: f64) -> i32 {
        round(self.mag * (y - f64::from(self.lly)) / FSCALE + f64::from(self.border_margin))
    }
    fn xz(&self, x: i32) -> i32 {
        self.xzoom(f64::from(x))
    }
    fn yz(&self, y: i32) -> i32 {
        self.yzoom(f64::from(y))
    }

    fn add_link(&mut self, link: (String, String, String), area: String) {
        let (url, alt, _) = link;
        self.links.push(Link { url, alt, area });
    }
}

impl Driver for Map {
    fn option(&mut self, _ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        match opt {
            'b' => {
                // sscanf("%d"): unchanged when there is no number.
                let (v, n) = crate::gendev::strtol(arg.unwrap_or(""));
                if n > 0 {
                    self.border_margin = v;
                }
            }
            'G' | 'L' => {}
            _ => return Err(format!("Argument -{opt} unknown to map driver.")),
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        self.mag = ctx.mag;
        self.llx = ctx.llx;
        self.lly = ctx.lly;
        // The output's name up to its last dot (not a leading one).
        let basename = match ctx.to.as_deref() {
            Some(to) => match to.rfind('.') {
                Some(i) if i > 0 => to.get(..i).unwrap_or(to),
                _ => to,
            },
            None => "imagemap",
        }
        .to_string();
        out.puts("<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 4.0 Frameset//EN\">\n");
        out.puts("<HTML>\n");
        out.puts("<HEAD><TITLE>HTML imagemap generated by fig2dev</TITLE></HEAD>\n");
        out.puts("<BODY>\n\n");
        out.puts("<H1>HTML imagemap generated by fig2dev</H1>\n");
        out.puts("<P>This file is generated by fig2dev.</P>\n");
        out.puts("<P>You can copy the following lines into your HTML document.\n");
        out.puts("You may need to edit the name of the image file in the first line.</P>\n");
        out.puts("\n");
        fprintf!(out, "<IMG SRC=\"{}.png\" USEMAP=\"#{}\">\n", basename, basename);
        fprintf!(out, "<MAP NAME=\"{}\">\n", basename);
        if let Some(link) = is_link(ctx, &objects.comments) {
            let area = format!(
                "<AREA COORDS=\"{},{},{},{}\" {}>\n",
                self.xz(ctx.llx),
                self.yz(ctx.lly),
                self.xz(ctx.urx),
                self.yz(ctx.ury),
                link.2
            );
            self.add_link(link, area);
        }
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, _out: &mut Out, a: &Arc) -> Result<(), String> {
        let Some(link) = is_link(ctx, &a.comments) else {
            return Ok(());
        };
        let cx = self.xzoom(a.center.x);
        let cy = self.yzoom(a.center.y);
        let [p0, _, p2] = a.point;
        let (sx, sy, ex, ey) = (self.xz(p0.x), self.yz(p0.y), self.xz(p2.x), self.yz(p2.y));
        let angle = |x: i32, y: i32| f64::from(y.wrapping_sub(cy)).atan2(f64::from(x.wrapping_sub(cx)));
        let (sa, mut ea) = if a.direction == 0 {
            (angle(sx, sy), angle(ex, ey))
        } else {
            (angle(ex, ey), angle(sx, sy))
        };
        if ea < sa {
            ea += 2.0 * PI;
        }
        let dx = sx.wrapping_sub(cx);
        let dy = sy.wrapping_sub(cy);
        let r = f64::from(dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy)))
            .sqrt()
            .mul_add(ARC_EXPAND, 1.0);
        let (cxf, cyf) = (f64::from(cx), f64::from(cy));
        let mut buf = String::from("<AREA SHAPE=\"poly\" COORDS=\"");
        if a.type_ == T_PIE_WEDGE_ARC {
            let _ = write!(buf, "{cx},{cy},");
        }
        // At most a full turn (sa and ea are finite: atan2 of integers).
        let mut alpha = sa;
        while alpha < ea - ARC_STEP / 4.0 {
            if alpha != sa {
                buf.push(',');
            }
            let _ = write!(
                buf,
                "{},{}",
                round(r.mul_add(alpha.cos(), cxf)),
                round(r.mul_add(alpha.sin(), cyf))
            );
            alpha += ARC_STEP;
        }
        let _ = write!(
            buf,
            ",{},{}",
            round(r.mul_add(ea.cos(), cxf)),
            round(r.mul_add(ea.sin(), cyf))
        );
        let _ = writeln!(buf, "\" {}>", link.2);
        self.add_link(link, buf);
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, _out: &mut Out, e: &Ellipse) -> Result<(), String> {
        let Some(link) = is_link(ctx, &e.comments) else {
            return Ok(());
        };
        let x0 = self.xz(e.center.x);
        let y0 = self.yz(e.center.y);
        let mut rx = self.mag * f64::from(e.radiuses.x) / FSCALE;
        let mut ry = self.mag * f64::from(e.radiuses.y) / FSCALE;
        let angle = -e.angle;
        if e.radiuses.x == e.radiuses.y {
            let area = format!(
                "<AREA SHAPE=\"circle\" COORDS=\"{},{},{}\" {}>\n",
                x0,
                y0,
                round(rx).wrapping_add(1),
                link.2
            );
            self.add_link(link, area);
            return Ok(());
        }
        // Contracted to fused multiply-adds, as by fig2dev's build.
        rx = rx.mul_add(ARC_EXPAND, 1.0);
        ry = ry.mul_add(ARC_EXPAND, 1.0);
        let (x0f, y0f) = (f64::from(x0), f64::from(y0));
        let (ca, sa) = (angle.cos(), angle.sin());
        let mut buf = String::from("<AREA SHAPE=\"poly\" COORDS=\"");
        let mut theta: f64 = 0.0;
        while theta < 2.0 * PI {
            if theta != 0.0 {
                buf.push(',');
            }
            // x0 + cos(angle)*rx*cos(theta) - sin(angle)*ry*sin(theta),
            // contracted to fused multiply-adds as fig2dev's build does.
            let x = (-(sa * ry)).mul_add(theta.sin(), (ca * rx).mul_add(theta.cos(), x0f));
            let y = (ca * ry).mul_add(theta.sin(), (sa * rx).mul_add(theta.cos(), y0f));
            let _ = write!(buf, "{},{}", round(x), round(y));
            theta += ARC_STEP;
        }
        let _ = writeln!(buf, "\" {}>", link.2);
        self.add_link(link, buf);
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, _out: &mut Out, l: &Line) -> Result<(), String> {
        let Some(link) = is_link(ctx, &l.comments) else {
            return Ok(());
        };
        let Some(first) = l.points.first() else {
            return Ok(());
        };
        match l.type_ {
            T_BOX | T_ARC_BOX | T_PIC_BOX => {
                let (mut xmin, mut xmax, mut ymin, mut ymax) = (first.x, first.x, first.y, first.y);
                for p in &l.points {
                    xmin = xmin.min(p.x);
                    xmax = xmax.max(p.x);
                    ymin = ymin.min(p.y);
                    ymax = ymax.max(p.y);
                }
                let area = format!(
                    "<AREA COORDS=\"{},{},{},{}\" {}>\n",
                    self.xz(xmin),
                    self.yz(ymin),
                    self.xz(xmax),
                    self.yz(ymax),
                    link.2
                );
                self.add_link(link, area);
            }
            T_POLYLINE | T_POLYGON => {
                // A polygon's last point repeats its first.
                let n = if l.type_ == T_POLYLINE {
                    l.points.len()
                } else {
                    l.points.len().saturating_sub(1)
                };
                let mut buf = String::from("<AREA SHAPE=\"poly\" COORDS=\"");
                let (mut last_x, mut last_y) = (0i32, 0i32);
                for (i, p) in l.points.iter().take(n).enumerate() {
                    let x = self.xz(p.x);
                    let y = self.yz(p.y);
                    if i == 0
                        || MIN_DIST < last_x.wrapping_sub(x).wrapping_abs()
                        || MIN_DIST < last_y.wrapping_sub(y).wrapping_abs()
                    {
                        if BUF_SIZE < buf.len() + 100 {
                            ctx.warn("fig2dev(map): too complex POLYLINE... ignored");
                            return Ok(());
                        }
                        if i != 0 {
                            buf.push(',');
                        }
                        let _ = write!(buf, "{x},{y}");
                        last_x = x;
                        last_y = y;
                    }
                }
                let _ = writeln!(buf, "\" {}>", link.2);
                self.add_link(link, buf);
            }
            _ => {}
        }
        Ok(())
    }

    fn spline(&mut self, _ctx: &mut Ctx, _out: &mut Out, _s: &Spline) -> Result<(), String> {
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, _out: &mut Out, t: &Text) -> Result<(), String> {
        if is_link(ctx, &t.comments).is_some() {
            ctx.warn("fig2dev(map): TEXT can't be used as link region");
        }
        Ok(())
    }

    fn end(&mut self, _ctx: &mut Ctx, real_out: &mut Out) -> Result<i32, String> {
        // The links are byte views (see `is_link`): convert back at the end.
        let mut tmp = Out::new();
        let out = &mut tmp;
        for l in self.links.iter().rev() {
            out.puts(&l.area);
        }
        out.puts("</MAP>\n");
        out.puts("\n");
        out.puts("<h2>Alternative Text Links</h2>\n");
        out.puts("<P>Because not all people can use imagemaps,\n");
        out.puts("it is recommended to prepare an alternative way for the people\n");
        out.puts("who can't use or don't want to use them.</P>\n");
        out.puts("<P>Here are text the links extracted from the above imagemap.\n");
        out.puts("If the ALT attribute (it is required in HTML 3.2) was not specified\n");
        out.puts("in the imagemap, those URLs will be displayed\n");
        out.puts("instead of labels specified by the ALT attributes.</P>\n");
        out.puts("\n");
        out.puts("<P ALIGN=\"CENTER\">\n");
        let mut len = 0usize;
        for l in self.links.iter().rev() {
            let label = if l.alt.is_empty() {
                format!("<TT>{}</TT>", l.url)
            } else {
                l.alt.clone()
            };
            fprintf!(
                out,
                "{}<A HREF={}>{}</A>\n",
                if len == 0 { "" } else { " | " },
                l.url,
                label
            );
            len += label.chars().count() + 3;
            if 50 < len {
                out.puts("<BR>\n");
                len = 0;
            }
        }
        out.puts("</P>\n");
        out.puts("\n");
        out.puts("</BODY>\n");
        out.puts("</HTML>\n");
        let text = String::from_utf8_lossy(&tmp.buf).into_owned();
        real_out.bytes(&from_byte_view(&text));
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}
