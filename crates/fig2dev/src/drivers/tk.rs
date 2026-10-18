//! fig2dev's `tk` and `ptk` drivers: the figure as Tcl/Tk canvas commands
//! (`-L tk`) or as a Perl/Tk subroutine (`-L ptk`).
//!
//! The two C files are near-duplicates; one `Tk` serves both with a flag.
//! Everything is a canvas item at a position in inches. Fill patterns are
//! X bitmap files the script writes next to itself; shades and tints are
//! calls to helper procedures. Splines (only in pre-3.2 files) are
//! flattened to polylines. Text that C writes with `niceLine` is wrapped
//! after 80 columns outside double quotes.
//
// Ported from fig2dev dev/gentk.c, dev/genptk.c and dev/tkpattern.c (xfig licence, see LICENSE-xfig).

use std::f64::consts::PI;

use crate::drivers::latex::Cf;
use crate::drivers::pict2e::round_mul;
use crate::gendev::{print_comments, Ctx, PAPERDEF};
use crate::object::{
    Arc, Arrow, Compound, Ellipse, Line, Point, Spline, Text, BLACK_COLOR, BLACK_FILL, DASH_2_DOTS_LINE,
    DASH_3_DOTS_LINE, DASH_DOT_LINE, DASH_LINE, DEFAULT, DOTTED_LINE, NUMPATTERNS, NUMSHADES, NUMTINTS, T_ARC_BOX,
    T_BOX, T_CENTER_JUSTIFIED, T_LEFT_JUSTIFIED, T_OPEN_ARC, T_PIC_BOX, T_PIE_WEDGE_ARC, T_POLYGON, T_POLYLINE,
    T_RIGHT_JUSTIFIED, UNFILLED, WHITE_COLOR, WHITE_FILL,
};
use crate::util::readpics;
use crate::util::textconvert::{byte_view, from_byte_view};
use crate::util::tkpattern::TKPATTERN;
use crate::{fprintf, round, Driver, Out};

/// `NONE`: no colour (never a real colour number).
const NONE: i32 = 0xffffff + 1;
/// `THRESHOLD` of the spline flattening, inches.
const THRESHOLD: f64 = 0.05;
/// A cap on the spline flattening's recursion (C recurses until the
/// pieces are short; NaN coordinates would never stop it).
const MAX_DEPTH: u32 = 48;

/// The standard colours as `0xrrggbb` (`rgbColors[]`).
const RGB_COLORS: [u32; 32] = [
    0x000000, 0x0000ff, 0x00ff00, 0x00ffff, 0xff0000, 0xff00ff, 0xffff00, 0xffffff, 0x00008f, 0x0000b0, 0x0000d1,
    0x87cfff, 0x008f00, 0x00b000, 0x00d100, 0x008f8f, 0x00b0b0, 0x00d1d1, 0x8f0000, 0xb00000, 0xd10000, 0x8f008f,
    0xb000b0, 0xd100d1, 0x803000, 0xa14000, 0xb46100, 0xff8080, 0xffa1a1, 0xffbfbf, 0xffe0e0, 0xffd600,
];

/// `fontNames[]`: prefix and suffix of the X font name of each PostScript font.
const FONT_NAMES: [(&str, &str); 36] = [
    ("-adobe-times-medium-r-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-times-medium-r-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-times-medium-i-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-times-bold-r-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-times-bold-i-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-urw-itc avant garde-medium-r-normal-sans-", "-0-0-0-p-0-iso8859-1"),
    ("-urw-itc avant garde-medium-o-normal-sans-", "-0-0-0-p-0-iso8859-1"),
    ("-urw-itc avant garde-demi-r-normal-sans-", "-0-0-0-p-0-iso8859-1"),
    ("-urw-itc avant garde-demi-o-normal-sans-", "-0-0-0-p-0-iso8859-1"),
    ("-urw-itc bookman-light-r-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-urw-itc bookman-light-i-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-urw-itc bookman-demi-r-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-urw-itc bookman-demi-i-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-courier-medium-r-normal--", "-0-0-0-m-0-iso8859-1"),
    ("-adobe-courier-medium-o-normal--", "-0-0-0-m-0-iso8859-1"),
    ("-adobe-courier-bold-r-normal--", "-0-0-0-m-0-iso8859-1"),
    ("-adobe-courier-bold-o-normal--", "-0-0-0-m-0-iso8859-1"),
    ("-adobe-helvetica-medium-r-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-helvetica-medium-o-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-helvetica-bold-r-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-helvetica-bold-o-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-helvetica-medium-r-narrow--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-helvetica-medium-o-narrow--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-helvetica-bold-r-narrow--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-helvetica-bold-o-narrow--", "-0-0-0-p-0-iso8859-1"),
    (
        "-adobe-new century schoolbook-medium-r-normal--",
        "-0-0-0-p-0-iso8859-1",
    ),
    (
        "-adobe-new century schoolbook-medium-i-normal--",
        "-0-0-0-p-0-iso8859-1",
    ),
    ("-adobe-new century schoolbook-bold-r-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-new century schoolbook-bold-i-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-palatino-medium-r-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-palatino-medium-i-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-palatino-bold-r-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-adobe-palatino-medium-i-normal--", "-0-0-0-p-0-iso8859-1"),
    ("--symbol-medium-r-normal--", "-0-0-0-p-0-*-*"),
    ("-*-itc zapf chancery-medium-i-normal--", "-0-0-0-p-0-iso8859-1"),
    ("-*-itc zapf dingbats-medium-r-normal--", "-0-0-0-p-0-*-*"),
];

/// What `drawFilledShape` draws (the `tkShape` callback and its argument).
#[derive(Clone, Copy)]
enum Shape<'a> {
    /// An arc; the thickness decides between `-style arc` and `chord`
    /// (C passes it by changing the arc's own thickness).
    Arc(&'a Arc, i32),
    Ellipse(&'a Ellipse),
    Polygon(&'a [Point]),
}

/// Which arrow format: a line, an interpolated or a control spline.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Line,
    Itp,
    Ctl,
}

/// The `tk` (Tcl) and `ptk` (Perl) drivers' state (the C statics).
#[derive(Debug)]
pub struct Tk {
    ptk: bool,
    full_page: bool,
    wrap: bool,
    png_required: bool,
    jpeg_required: bool,
    tiff_required: bool,
    /// `niceLine`'s statics.
    in_quote: bool,
    pos: usize,
    ppi: f64,
}

/// The driver for `lang` (`tk` or `ptk`).
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    let ptk = match lang {
        "tk" => false,
        "ptk" => true,
        _ => return None,
    };
    Some(Box::new(Tk {
        ptk,
        full_page: false,
        wrap: false,
        png_required: false,
        jpeg_required: false,
        tiff_required: false,
        in_quote: false,
        pos: 0,
        ppi: 1200.0,
    }))
}

/// `%f`.
fn f6(x: f64) -> String {
    format!("{:.6}", Cf(x))
}

/// `%.Nf`.
fn fp(x: f64, n: usize) -> String {
    format!("{:.*}", n, Cf(x))
}

/// `lookup_X_color`, with 16-bit channels as C returns them.
fn lookup_x_color(name: &str) -> Option<[u16; 3]> {
    if let Some(hex) = name.strip_prefix('#') {
        let digits = match hex.len() {
            3 | 6 | 9 | 12 => hex.len() / 3,
            _ => return None,
        };
        let mut v = [0u16; 3];
        for (i, slot) in v.iter_mut().enumerate() {
            let s = hex.get(i * digits..(i + 1) * digits)?;
            if !s.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let n = u32::from_str_radix(s, 16).ok()?;
            *slot = match digits {
                1 => ((n << 4) + n) << 8,
                2 => n << 8,
                3 => n << 4,
                _ => n,
            } as u16;
        }
        return Some(v);
    }
    let c = figbase::color::x_color(name)?;
    Some([u16::from(c.r) << 8, u16::from(c.g) << 8, u16::from(c.b) << 8])
}

impl Tk {
    fn x(&self, v: i32) -> f64 {
        f64::from(v) / self.ppi
    }

    fn canvas(&self) -> &'static str {
        if self.ptk {
            "$c"
        } else {
            "$xfigCanvas"
        }
    }

    fn patname(&self) -> &'static str {
        if self.ptk {
            "$xfigp"
        } else {
            "xfigp"
        }
    }

    /// `niceLine`: writes `s`, breaking at a blank after column 80 outside
    /// double quotes.
    fn nice_line(&mut self, out: &mut Out, s: &str) {
        for &c in s.as_bytes() {
            if c == b'"' {
                self.in_quote = !self.in_quote;
                out.bytes(&[c]);
                self.pos += 1;
            } else if !self.in_quote && self.pos > 80 && c == b' ' {
                if self.ptk {
                    out.puts("\n    ");
                    self.pos = 4;
                } else {
                    out.puts(" \\\n  ");
                    self.pos = 2;
                }
            } else {
                out.bytes(&[c]);
                if c == b'\n' {
                    self.pos = 0;
                } else {
                    self.pos += 1;
                }
            }
        }
    }

    /// `rgbColorVal`.
    fn rgb_color_val(&self, ctx: &Ctx, idx: i32) -> u32 {
        if idx == DEFAULT {
            return RGB_COLORS[0];
        }
        if idx < 32 {
            return usize::try_from(idx)
                .ok()
                .and_then(|i| RGB_COLORS.get(i))
                .copied()
                .unwrap_or(0);
        }
        // C reads past the table for a colour the file doesn't define.
        let Some(u) = usize::try_from(idx - 32).ok().and_then(|i| ctx.user_colors.get(i)) else {
            return 0;
        };
        (((u.r & 0xff) as u32) << 16) | (((u.g & 0xff) as u32) << 8) | (u.b & 0xff) as u32
    }

    /// `define_pattern`.
    fn define_pattern(&self, out: &mut Out, n: usize) {
        let Some(p) = TKPATTERN.get(n) else {
            return;
        };
        let pat = self.patname();
        fprintf!(out, "# {}\n", p.name);
        if self.ptk {
            fprintf!(out, "my {pat}{n} = \"$xbmPath/{}.xbm\";\n", p.name);
            fprintf!(out, "unless (-e {pat}{n}) {{\n    open(my $fh, '>', {pat}{n}) or die ");
            fprintf!(out, "\"Cannot open file '{pat}{n}' for writing: $!\\n\";\n");
            out.puts("    print $fh <<'EOF';\n");
            fprintf!(out, "#define {}_width {}\n", p.name, p.width);
            fprintf!(out, "#define {}_height {}\n", p.name, p.height);
            fprintf!(out, "static unsigned char {}_bits[] = {{\n", p.name);
            fprintf!(out, "\t0x{:02x}", p.bits.first().copied().unwrap_or(0));
        } else {
            fprintf!(out, "set {pat}{n} [file join $xbmPath {}.xbm]\n", p.name);
            fprintf!(out, "if {{ [file exist ${pat}{n}] == 0 }} {{\n");
            fprintf!(out, "    set fd [open ${pat}{n} w]\n    puts $fd {{");
            fprintf!(
                out,
                "#define {}_width {}\n#define {}_height {}\n",
                p.name,
                p.width,
                p.name,
                p.height
            );
            fprintf!(
                out,
                "static unsigned char {}_bits[] = {{\n\t0x{:02x}",
                p.name,
                p.bits.first().copied().unwrap_or(0)
            );
        }
        for i in 1..p.width.div_ceil(8) * p.height {
            if i % 12 == 0 {
                out.puts(",\n\t");
            } else {
                out.puts(", ");
            }
            fprintf!(out, "0x{:02x}", p.bits.get(i).copied().unwrap_or(0));
        }
        if self.ptk {
            out.puts("};\nEOF\n    close $fh;\n};\n");
        } else {
            out.puts("};}\nclose $fd\n}\n");
        }
    }

    /// `tk_setstyle` / `ptk_setstyle`.
    fn setstyle(&mut self, out: &mut Out, style: i32, v: f64) {
        let v = v * (96. / 80.);
        let r = round;
        let one = 1;
        // For v <= 0 C passes an uninitialised buffer to niceLine; nothing
        // is written here.
        if v <= 0.0 || v.is_nan() {
            return;
        }
        let nums: Vec<i32> = match style {
            DASH_LINE => vec![r(v), r(v)],
            DOTTED_LINE => vec![one, r(v)],
            DASH_DOT_LINE => vec![r(v), round_mul(v, 0.5), one, round_mul(v, 0.5)],
            DASH_2_DOTS_LINE => vec![
                r(v),
                round_mul(v, 0.45),
                one,
                round_mul(v, 0.333),
                one,
                round_mul(v, 0.45),
            ],
            DASH_3_DOTS_LINE => vec![
                r(v),
                round_mul(v, 0.4),
                one,
                round_mul(v, 0.3),
                one,
                round_mul(v, 0.3),
                one,
                round_mul(v, 0.4),
            ],
            _ => return,
        };
        let l: Vec<String> = nums.iter().map(i32::to_string).collect();
        let s = if self.ptk {
            format!(", -dash => [{}]", l.join(","))
        } else {
            format!(" -dash {{{}}}", l.join(" "))
        };
        self.nice_line(out, &s);
    }

    /// The `-arrow` option of a line or spline.
    fn arrow(&mut self, ctx: &mut Ctx, out: &mut Out, f: Option<&Arrow>, b: Option<&Arrow>, kind: Kind) {
        let (a, dir) = match (f, b) {
            (Some(f), None) => (f, "last"),
            (None, Some(b)) => (b, "first"),
            (Some(_), Some(b)) => (b, "both"),
            (None, None) => return,
        };
        let ht = a.ht / self.ppi;
        let wid2 = a.wid / self.ppi / 2.;
        let (ht_s, wid_s) = (f6(ht), f6(wid2));
        let s = match a.type_ {
            0 => {
                ctx.warn("Warning: stick arrows do not work well in Tk.");
                if self.ptk {
                    let lead = if kind == Kind::Ctl { " " } else { ", " };
                    let (sp, tail) = if kind == Kind::Ctl { ("=>", ",") } else { ("=> ", "") };
                    format!("{lead}-arrow => '{dir}', -arrowshape {sp}[0, '{ht_s}i', '{wid_s}i']{tail}")
                } else {
                    format!(" -arrow {dir} -arrowshape {{0 {ht_s}i {wid_s}i}}")
                }
            }
            t @ 1..=3 => {
                let p0 = match t {
                    1 => ht_s.clone(),
                    2 => f6(0.8 * ht),
                    _ => f6(1.2 * ht),
                };
                let (p1, p2) = (ht_s.clone(), wid_s.clone());
                if self.ptk {
                    let (lead, sp) = match kind {
                        Kind::Line => (", ", "=> "),
                        Kind::Itp => (" ", "=> "),
                        Kind::Ctl => (" ", "=>"),
                    };
                    let d = if kind == Kind::Ctl && t == 3 {
                        dir.to_string()
                    } else {
                        format!("'{dir}'")
                    };
                    format!("{lead}-arrow => {d}, -arrowshape {sp}['{p0}i', '{p1}i', '{p2}i']")
                } else {
                    format!(" -arrow {dir} -arrowshape {{{p0}i {p1}i {p2}i}}")
                }
            }
            _ => {
                ctx.warn(if self.ptk && kind == Kind::Line {
                    "ptkLine: unknown arrow type."
                } else {
                    "tkLine: unknown arrow type."
                });
                return;
            }
        };
        self.nice_line(out, &s);
    }

    /// The `-capstyle` option.
    fn capstyle(&mut self, ctx: &mut Ctx, out: &mut Out, cap: i32, kind: Kind) {
        let name = match cap {
            0 => return,
            1 => "round",
            2 => "projecting",
            _ => {
                ctx.warn(if self.ptk && kind == Kind::Line {
                    "ptkLine: unknown cap style."
                } else {
                    "tkLine: unknown cap style."
                });
                return;
            }
        };
        let s = if !self.ptk {
            format!(" -capstyle {name}")
        } else if kind == Kind::Ctl {
            format!(" -capstyle => '{name}',")
        } else {
            format!(", -capstyle => '{name}'")
        };
        self.nice_line(out, &s);
    }

    /// `tkArc` / `ptkArc`.
    fn tk_arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc, thickness: i32) {
        let cx = a.center.x / self.ppi;
        let cy = a.center.y / self.ppi;
        let sx = self.x(a.point[0].x) - cx;
        let sy = cy - self.x(a.point[0].y);
        let ex = self.x(a.point[2].x) - cx;
        let ey = cy - self.x(a.point[2].y);
        let radius = sy.mul_add(sy, sx * sx).sqrt();
        let mut angle1 = sy.atan2(sx) * 180.0 / PI;
        if angle1 < 0. {
            angle1 += 360.;
        }
        let mut angle2 = ey.atan2(ex) * 180.0 / PI;
        if angle2 < 0. {
            angle2 += 360.;
        }
        let (start, mut extent) = if a.direction == 1 {
            (angle1, angle2 - angle1)
        } else {
            (angle2, angle1 - angle2)
        };
        if extent < 0. {
            extent += 360.;
        }
        let c = self.canvas();
        let coords = format!(
            "{}i {}i {}i {}i",
            fp(cx - radius, 3),
            fp(cy - radius, 3),
            fp(cx + radius, 3),
            fp(cy + radius, 3)
        );
        if self.ptk {
            self.nice_line(out, &format!("{c}->createArc("));
            self.nice_line(out, &format!("qw/{coords}"));
            self.nice_line(out, &format!(" -start {} -extent {}/", f6(start), f6(extent)));
        } else {
            self.nice_line(out, &format!("{c} create arc"));
            self.nice_line(out, &format!(" {coords}"));
            self.nice_line(out, &format!(" -start {} -extent {}", f6(start), f6(extent)));
        }
        let style = match a.type_ {
            T_OPEN_ARC => {
                if thickness > 0 {
                    "arc"
                } else {
                    "chord"
                }
            }
            T_PIE_WEDGE_ARC => "pieslice",
            _ => {
                ctx.warn(if self.ptk {
                    "ptkArc: unknown arc type."
                } else {
                    "tkArc: unknown arc type."
                });
                return;
            }
        };
        let s = if self.ptk {
            format!(", -style => '{style}'")
        } else {
            format!(" -style {style}")
        };
        self.nice_line(out, &s);
    }

    /// `tkEllipse` / `ptkEllipse`.
    fn tk_ellipse(&mut self, out: &mut Out, e: &Ellipse) {
        let c = self.canvas();
        let v = [
            self.x(e.center.x.wrapping_sub(e.radiuses.x)),
            self.x(e.center.y.wrapping_sub(e.radiuses.y)),
            self.x(e.center.x.wrapping_add(e.radiuses.x)),
            self.x(e.center.y.wrapping_add(e.radiuses.y)),
        ];
        let s = if self.ptk {
            format!(
                "{c}->createOval(qw/{}i {}i {}i {}i/",
                f6(v[0]),
                f6(v[1]),
                f6(v[2]),
                f6(v[3])
            )
        } else {
            format!(
                "{c} create oval {}i {}i {}i {}i",
                f6(v[0]),
                f6(v[1]),
                f6(v[2]),
                f6(v[3])
            )
        };
        self.nice_line(out, &s);
    }

    /// `tkPolygon` / `ptkPolygon`.
    fn tk_polygon(&mut self, out: &mut Out, pts: &[Point]) {
        let Some((p, rest)) = pts.split_first() else {
            return;
        };
        let c = self.canvas();
        let first = if self.ptk {
            format!("{c}->createPolygon('{}i','{}i'", f6(self.x(p.x)), f6(self.x(p.y)))
        } else {
            format!("{c} create polygon {}i {}i", f6(self.x(p.x)), f6(self.x(p.y)))
        };
        self.nice_line(out, &first);
        // all but the last point, then the last one unless it closes the shape
        let Some((last, middle)) = rest.split_last() else {
            return;
        };
        let pt = |t: &Tk, q: &Point| {
            if t.ptk {
                format!(", '{}i','{}i'", f6(t.x(q.x)), f6(t.x(q.y)))
            } else {
                format!(" {}i {}i", f6(t.x(q.x)), f6(t.x(q.y)))
            }
        };
        for q in middle {
            let s = pt(self, q);
            self.nice_line(out, &s);
        }
        if last.x != p.x || last.y != p.y {
            let s = pt(self, last);
            self.nice_line(out, &s);
        }
    }

    fn shape(&mut self, ctx: &mut Ctx, out: &mut Out, sh: Shape<'_>) {
        match sh {
            Shape::Arc(a, t) => self.tk_arc(ctx, out, a, t),
            Shape::Ellipse(e) => self.tk_ellipse(out, e),
            Shape::Polygon(p) => self.tk_polygon(out, p),
        }
    }

    /// `drawFilledShape`.
    #[allow(clippy::too_many_arguments)]
    fn draw_filled_shape(
        &mut self,
        ctx: &mut Ctx,
        out: &mut Out,
        sh: Shape<'_>,
        thickness: i32,
        pen_color: i32,
        mut fill_color: i32,
        mut fill_style: i32,
        style: i32,
        style_val: f64,
    ) {
        let ptk = self.ptk;
        let pat = self.patname();
        if ptk {
            self.shape(ctx, out, sh);
        }
        if fill_style >= NUMSHADES + NUMTINTS {
            let stip = fill_style - NUMSHADES - NUMTINTS;
            if !ptk {
                self.shape(ctx, out, sh);
            }
            let s = if ptk {
                format!(
                    ", -fill => '#{:06x}', -outline => undef, -width => 0);\n",
                    self.rgb_color_val(ctx, fill_color)
                )
            } else {
                format!(
                    " -fill #{:06x} -outline {{}} -width 0\n",
                    self.rgb_color_val(ctx, fill_color)
                )
            };
            self.nice_line(out, &s);
            self.shape(ctx, out, sh);
            let pc = self.rgb_color_val(ctx, pen_color);
            let s = if ptk {
                format!(", -fill => '#{pc:06x}', -stipple => \"\\@{pat}{stip}\"")
            } else {
                format!(" -fill #{pc:06x} -stipple @${pat}{stip}")
            };
            self.nice_line(out, &s);
        } else if fill_style != UNFILLED {
            if !ptk {
                self.shape(ctx, out, sh);
            }
            let s = if fill_style > WHITE_FILL && fill_style < BLACK_FILL {
                if fill_color == BLACK_COLOR || fill_color == DEFAULT {
                    fill_color = WHITE_COLOR;
                    fill_style = BLACK_FILL - fill_style;
                }
                let c = self.rgb_color_val(ctx, fill_color);
                if ptk {
                    format!(", -fill => &$shade('#{c:06x}', {})", fill_style * 5)
                } else {
                    format!(" -fill [::tk::Darken #{c:06x} {}]", fill_style * 5)
                }
            } else if fill_style > BLACK_FILL && fill_style < NUMSHADES + NUMTINTS - 1 {
                let c = self.rgb_color_val(ctx, fill_color);
                let t = (NUMSHADES + NUMTINTS - 1 - fill_style) * 5;
                if ptk {
                    format!(", -fill => &$tint('#{c:06x}', {t})")
                } else {
                    format!(" -fill [tint #{c:06x} {t}]")
                }
            } else {
                if fill_style == WHITE_FILL {
                    fill_color = if fill_color == BLACK_COLOR {
                        WHITE_COLOR
                    } else {
                        BLACK_COLOR
                    };
                } else if fill_style == NUMSHADES + NUMTINTS - 1 {
                    fill_color = WHITE_COLOR;
                }
                let c = self.rgb_color_val(ctx, fill_color);
                if ptk {
                    format!(", -fill => '#{c:06x}'")
                } else {
                    format!(" -fill #{c:06x}")
                }
            };
            self.nice_line(out, &s);
        } else if ptk {
            self.nice_line(out, ", -fill => ''");
        } else {
            self.shape(ctx, out, sh);
            out.puts(" -fill {}");
        }

        if thickness > 0 {
            let pc = self.rgb_color_val(ctx, pen_color);
            let s = if ptk {
                format!(", -outline => '#{pc:06x}'")
            } else {
                format!(" -outline #{pc:06x}")
            };
            self.nice_line(out, &s);
            if thickness != 15 {
                let w = thickness / 15;
                let s = if ptk {
                    format!(", -width => {w}")
                } else {
                    format!(" -width {w}")
                };
                self.nice_line(out, &s);
            }
            self.setstyle(out, style, style_val);
        } else if ptk {
            self.nice_line(out, ", -outline => undef, -width => 0");
        } else {
            self.nice_line(out, " -outline {} -width 0");
        }
        if ptk {
            self.nice_line(out, ");\n");
        } else {
            out.puts("\n");
        }
    }

    /// `tkLine` / `ptkLine`.
    fn tk_line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line, pen_color: i32, thickness: i32) {
        let Some((p, rest)) = l.points.split_first() else {
            return;
        };
        let c = self.canvas();
        let (px, py) = (f6(self.x(p.x)), f6(self.x(p.y)));
        if rest.is_empty() {
            let s = if self.ptk {
                format!("{c}->createLine(qw/{px}i {py}i {px}i {py}i/")
            } else {
                format!("{c} create line {px}i {py}i {px}i {py}i")
            };
            self.nice_line(out, &s);
        } else if self.ptk {
            self.nice_line(out, &format!("{c}->createLine('{px}i','{py}i'"));
            for q in rest {
                let s = format!(", '{}i','{}i'", f6(self.x(q.x)), f6(self.x(q.y)));
                self.nice_line(out, &s);
            }
        } else {
            self.nice_line(out, &format!("{c} create line"));
            self.nice_line(out, &format!(" {px}i {py}i"));
            for q in rest {
                let s = format!(" {}i {}i", f6(self.x(q.x)), f6(self.x(q.y)));
                self.nice_line(out, &s);
            }
        }
        self.arrow(ctx, out, l.for_arrow.as_ref(), l.back_arrow.as_ref(), Kind::Line);
        self.setstyle(out, l.style, l.style_val);
        let join = match l.join_style {
            0 => None,
            1 => Some("round"),
            2 => Some("bevel"),
            _ => {
                ctx.warn(if self.ptk {
                    "ptkLine: unknown join style."
                } else {
                    "tkLine: unknown join style."
                });
                None
            }
        };
        if let Some(j) = join {
            let s = if self.ptk {
                format!(", -joinstyle => '{j}'")
            } else {
                format!(" -joinstyle {j}")
            };
            self.nice_line(out, &s);
        }
        self.capstyle(ctx, out, l.cap_style, Kind::Line);
        if thickness != 1 {
            let s = if self.ptk {
                format!(", -width => {thickness}")
            } else {
                format!(" -width {thickness}")
            };
            self.nice_line(out, &s);
        }
        if pen_color != BLACK_COLOR && pen_color != DEFAULT {
            let c = self.rgb_color_val(ctx, pen_color);
            let s = if self.ptk {
                format!(", -fill => '#{c:06x}'")
            } else {
                format!(" -fill #{c:06x}")
            };
            self.nice_line(out, &s);
        }
        if self.ptk {
            self.nice_line(out, ");\n");
        } else {
            out.puts("\n");
        }
    }

    /// `drawBitmap`.
    fn draw_bitmap(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) {
        let Some(p) = l.pic.as_ref() else {
            return;
        };
        let (Some(p0), Some(p2)) = (l.points.first(), l.points.get(2)) else {
            return;
        };
        let dx = f64::from(p2.x.wrapping_sub(p0.x));
        let dy = f64::from(p2.y.wrapping_sub(p0.y));
        let ptk = self.ptk;
        if !(dx >= 0. && dy >= 0.) {
            ctx.warn(if ptk {
                "drawBitmap: rotated bitmaps not supported by Tk."
            } else {
                "Rotated images not supported by Tk."
            });
        }
        let stream = match readpics::open_stream_pic(ctx, p) {
            Ok(s) => s,
            Err(_) => {
                ctx.warn(if ptk {
                    format!("drawBitmap: cannot open bitmap file {}", p.file)
                } else {
                    format!("Cannot open image file {}", p.file)
                });
                return;
            }
        };
        let need = if ptk { 6 } else { 7 };
        let Some(buf) = stream.data.get(..need) else {
            ctx.warn(if ptk {
                format!("drawBitmap: bitmap file {} too short", p.file)
            } else {
                format!("Image file {} too short", p.file)
            });
            return;
        };
        // C's strncmp stops at a NUL of the pattern: "MM\0*" means "MM" and a
        // zero byte, "II*\0" means "II*" and a zero byte.
        let mut isphoto = false;
        if ptk {
            if buf.starts_with(b"GIF")
                || (buf[0] == b'P' && (buf[1] == b'5' || buf[1] == b'6'))
                || buf.starts_with(b"BM")
                || buf.starts_with(b"/* XPM */")
            {
                isphoto = true;
            } else if buf[1..].starts_with(b"PNG") {
                isphoto = true;
                if !self.png_required {
                    out.puts("require Tk::PNG;\n");
                    self.png_required = true;
                }
            } else if buf.starts_with(&[0xff, 0xd8, 0xff, 0xe0]) {
                isphoto = true;
                if !self.jpeg_required {
                    out.puts("require Tk::JPEG;\n");
                    self.jpeg_required = true;
                }
            } else if buf.starts_with(b"MM\0") || buf.starts_with(b"II*\0") {
                isphoto = true;
                if !self.tiff_required {
                    out.puts("require Tk::TIFF;\n");
                    self.tiff_required = true;
                }
            }
        } else if buf.starts_with(b"GIF") || buf.starts_with(b"P3\n") || buf.starts_with(b"P6\n") {
            isphoto = true;
        }
        let (x, y) = (f6(self.x(p0.x)), f6(self.x(p0.y)));
        // The name as a byte view; the section's output is converted back.
        let pfile = byte_view(p.name_bytes());
        let start = out.buf.len();
        let c = self.canvas();
        if isphoto {
            let pname = pfile.split('.').next().unwrap_or("");
            if ptk {
                fprintf!(out, "$img{{\"{pname}\"}} = $top->Photo(-file => \"{}\");\n", pfile);
                fprintf!(
                    out,
                    "{c}->createImage(qw/{x}i {y}i -anchor nw -image/, $img{{\"{pname}\"}});"
                );
            } else {
                fprintf!(out, "image create photo {pname} -file {}\n\n", pfile);
                fprintf!(out, "{c} create image {x}i {y}i -anchor nw -image {pname}\n");
            }
        } else if readpics::read_bitmap_file(&stream.data).is_some() {
            if ptk {
                self.nice_line(out, &format!("{c}->createBitmap(qw/{x}i {y}i -anchor nw"));
                self.nice_line(out, &format!(" -bitmap/, \"@{}\"", pfile));
            } else {
                self.nice_line(out, &format!("{c} create bitmap {x}i {y}i -anchor nw"));
                self.nice_line(out, &format!(" -bitmap @{}", pfile));
            }
            if l.pen_color != BLACK_COLOR && l.pen_color != DEFAULT {
                let v = self.rgb_color_val(ctx, l.pen_color);
                let s = if ptk {
                    format!(", -foreground => '#{v:06x}'")
                } else {
                    format!(" -foreground #{v:06x}")
                };
                self.nice_line(out, &s);
            }
            if l.fill_color != UNFILLED {
                let v = self.rgb_color_val(ctx, l.fill_color);
                let s = if ptk {
                    format!(", -background => '#{v:06x}'")
                } else {
                    format!(" -background #{v:06x}")
                };
                self.nice_line(out, &s);
            }
            if ptk {
                self.nice_line(out, ");\n");
            } else {
                out.puts("\n");
            }
        } else {
            ctx.warn(if ptk {
                "Only X bitmap, TIFF, JPEG, PPM and GIF picture objects are supported in Tk canvases."
            } else {
                "Only X bitmap and GIF picture objects are supported in Tk canvases."
            });
        }
        let tail = from_byte_view(&String::from_utf8_lossy(out.buf.get(start..).unwrap_or_default()));
        out.buf.truncate(start);
        out.bytes(&tail);
    }

    /// `bezierSpline`.
    fn bezier(&mut self, out: &mut Out, p: [f64; 8], depth: u32) {
        let [x0, y0, a1, b1, a2, b2, x3, y3] = p;
        if ((x0 - x3).abs() < THRESHOLD && (y0 - y3).abs() < THRESHOLD) || depth >= MAX_DEPTH {
            let s = if self.ptk {
                format!(", '{}i', '{}i'", fp(x3, 4), fp(y3, 4))
            } else {
                format!(" {} {}", fp(x3, 4), fp(y3, 4))
            };
            self.nice_line(out, &s);
        } else {
            let tx = (a1 + a2) / 2.0;
            let ty = (b1 + b2) / 2.0;
            let sx1 = (x0 + a1) / 2.0;
            let sy1 = (y0 + b1) / 2.0;
            let sx2 = (sx1 + tx) / 2.0;
            let sy2 = (sy1 + ty) / 2.0;
            let tx2 = (a2 + x3) / 2.0;
            let ty2 = (b2 + y3) / 2.0;
            let tx1 = (tx2 + tx) / 2.0;
            let ty1 = (ty2 + ty) / 2.0;
            let xmid = (sx2 + tx1) / 2.0;
            let ymid = (sy2 + ty1) / 2.0;
            self.bezier(out, [x0, y0, sx1, sy1, sx2, sy2, xmid, ymid], depth + 1);
            self.bezier(out, [xmid, ymid, tx1, ty1, tx2, ty2, x3, y3], depth + 1);
        }
    }

    /// `quadraticSpline`.
    fn quadratic(&mut self, out: &mut Out, p: [f64; 8], depth: u32) {
        let [x1, y1, a2, b2, a3, b3, x4, y4] = p;
        let xmid = (a2 + a3) / 2.0;
        let ymid = (b2 + b3) / 2.0;
        let done = depth >= MAX_DEPTH;
        if ((x1 - xmid).abs() < THRESHOLD && (y1 - ymid).abs() < THRESHOLD) || done {
            let s = if self.ptk {
                format!("'{}i', '{}i',\n", fp(xmid, 4), fp(ymid, 4))
            } else {
                format!(" {} {};\n", fp(xmid, 4), fp(ymid, 4))
            };
            self.nice_line(out, &s);
        } else {
            self.quadratic(
                out,
                [
                    x1,
                    y1,
                    (x1 + a2) / 2.0,
                    (y1 + b2) / 2.0,
                    3.0f64.mul_add(a2, a3) / 4.0,
                    3.0f64.mul_add(b2, b3) / 4.0,
                    xmid,
                    ymid,
                ],
                depth + 1,
            );
        }
        if ((xmid - x4).abs() < THRESHOLD && (ymid - y4).abs() < THRESHOLD) || done {
            let s = if self.ptk {
                format!("'{}i', '{}i', ", fp(x4, 4), fp(y4, 4))
            } else {
                format!(" {} {}", fp(x4, 4), fp(y4, 4))
            };
            self.nice_line(out, &s);
        } else {
            self.quadratic(
                out,
                [
                    xmid,
                    ymid,
                    3.0f64.mul_add(a3, a2) / 4.0,
                    3.0f64.mul_add(b3, b2) / 4.0,
                    (a3 + x4) / 2.0,
                    (b3 + y4) / 2.0,
                    x4,
                    y4,
                ],
                depth + 1,
            );
        }
    }

    /// `gentk_itpSpline`.
    fn itp_spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        let ppi = self.ppi;
        let c = self.canvas();
        let Some(p1) = s.points.first() else {
            return Err("tk: spline without points".into());
        };
        if s.controls.len() < s.points.len() {
            return Err("tk: interpolated spline without control points".into());
        }
        let (mut x2, mut y2) = (f64::from(p1.x) / ppi, f64::from(p1.y) / ppi);
        let head = if self.ptk {
            format!("{c}->createLine(qw/{}i {}i/", fp(x2, 4), fp(y2, 4))
        } else {
            format!("{c} create line {} {}", fp(x2, 4), fp(y2, 4))
        };
        self.nice_line(out, &head);
        for i in 1..s.points.len() {
            let (Some(p2), Some(cp1), Some(cp2)) = (s.points.get(i), s.controls.get(i - 1), s.controls.get(i)) else {
                break;
            };
            let (x1, y1) = (x2, y2);
            x2 = f64::from(p2.x) / ppi;
            y2 = f64::from(p2.y) / ppi;
            self.bezier(
                out,
                [x1, y1, cp1.rx / ppi, cp1.ry / ppi, cp2.lx / ppi, cp2.ly / ppi, x2, y2],
                0,
            );
        }
        self.arrow(ctx, out, s.for_arrow.as_ref(), s.back_arrow.as_ref(), Kind::Itp);
        self.capstyle(ctx, out, s.cap_style, Kind::Itp);
        if s.thickness != 1 {
            let t = if self.ptk {
                format!(", -width => '{}'", s.thickness)
            } else {
                format!(" -width {}", s.thickness)
            };
            self.nice_line(out, &t);
        }
        if s.pen_color != BLACK_COLOR && s.pen_color != DEFAULT {
            let t = if self.ptk {
                format!(", -fill => '#{:06x}'", s.pen_color as u32)
            } else {
                format!(" -fill #{:06x}", s.pen_color as u32)
            };
            self.nice_line(out, &t);
        }
        if self.ptk {
            self.nice_line(out, ");\n");
        }
        Ok(())
    }

    /// `gentk_ctlSpline`.
    fn ctl_spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        let ppi = self.ppi;
        let c = self.canvas();
        let (Some(p0), Some(p1)) = (s.points.first(), s.points.get(1)) else {
            return Err("tk: spline with fewer than two points".into());
        };
        let (mut x1, mut y1) = (f64::from(p0.x) / ppi, f64::from(p0.y) / ppi);
        let (mut x2, mut y2) = (f64::from(p1.x) / ppi, f64::from(p1.y) / ppi);
        let mut cx1 = (x1 + x2) / 2.0;
        let mut cy1 = (y1 + y2) / 2.0;
        let mut cx2 = 3.0f64.mul_add(x2, x1) / 4.0;
        let mut cy2 = 3.0f64.mul_add(y2, y1) / 4.0;
        let closed = s.closed_spline();
        let head = match (closed, self.ptk) {
            (true, true) => format!("{c}->createPolygon(qw/{} {}/,", fp(cx1, 4), fp(cy1, 4)),
            (true, false) => format!("{c} create polygon {} {}", fp(cx1, 4), fp(cy1, 4)),
            (false, true) => format!("{c}->createLine("),
            (false, false) => format!("{c} create line"),
        };
        self.nice_line(out, &head);
        for p in s.points.iter().skip(2) {
            x1 = x2;
            y1 = y2;
            x2 = f64::from(p.x) / ppi;
            y2 = f64::from(p.y) / ppi;
            let cx3 = 3.0f64.mul_add(x1, x2) / 4.0;
            let cy3 = 3.0f64.mul_add(y1, y2) / 4.0;
            let cx4 = (x1 + x2) / 2.0;
            let cy4 = (y1 + y2) / 2.0;
            self.quadratic(out, [cx1, cy1, cx2, cy2, cx3, cy3, cx4, cy4], 0);
            cx1 = cx4;
            cy1 = cy4;
            cx2 = 3.0f64.mul_add(x2, x1) / 4.0;
            cy2 = 3.0f64.mul_add(y2, y1) / 4.0;
        }
        x1 = x2;
        y1 = y2;
        x2 = f64::from(p1.x) / ppi;
        y2 = f64::from(p1.y) / ppi;
        let cx3 = 3.0f64.mul_add(x1, x2) / 4.0;
        let cy3 = 3.0f64.mul_add(y1, y2) / 4.0;
        let cx4 = (x1 + x2) / 2.0;
        let cy4 = (y1 + y2) / 2.0;
        if closed {
            self.quadratic(out, [cx1, cy1, cx2, cy2, cx3, cy3, cx4, cy4], 0);
        }
        let ptk = self.ptk;
        if closed {
            let pen = s.pen_color;
            let t = match (pen == NONE, ptk) {
                (true, true) => " -outline => undef,".to_string(),
                (true, false) => " -outline {}".to_string(),
                (false, true) => format!(" -outline => '#{:06x}',", pen as u32),
                (false, false) => format!(" -outline #{:06x}", pen as u32),
            };
            self.nice_line(out, &t);
            let fc = s.fill_color;
            let t = match (fc == NONE, ptk) {
                (true, true) => " -fill => undef,".to_string(),
                (true, false) => " -fill {}".to_string(),
                (false, true) => format!(" -fill => '#{:06x}',", fc as u32),
                (false, false) => format!(" -fill #{:06x}", fc as u32),
            };
            self.nice_line(out, &t);
            // `s->fill_style != NONE` is always true
            let pat = self.patname();
            let n = s.fill_style - NUMSHADES - NUMTINTS;
            let t = if ptk {
                format!(" -stipple => \"\\@{pat}{n}\",")
            } else {
                format!(" -stipple @${pat}{n}")
            };
            self.nice_line(out, &t);
            if s.thickness != 1 {
                let t = if ptk {
                    format!(" -width => {}", s.thickness)
                } else {
                    format!(" -width {}", s.thickness)
                };
                self.nice_line(out, &t);
            }
        } else {
            self.arrow(ctx, out, s.for_arrow.as_ref(), s.back_arrow.as_ref(), Kind::Ctl);
            self.capstyle(ctx, out, s.cap_style, Kind::Ctl);
            if s.thickness != 1 {
                let t = if ptk {
                    format!(" -width => '{}',", s.thickness)
                } else {
                    format!(" -width {}", s.thickness)
                };
                self.nice_line(out, &t);
            }
            if s.pen_color != BLACK_COLOR && s.pen_color != DEFAULT {
                let t = if ptk {
                    format!(" -fill => '#{:06x}',", s.pen_color as u32)
                } else {
                    format!(" -fill #{:06x}", s.pen_color as u32)
                };
                self.nice_line(out, &t);
            }
        }
        // genptk's closing ");\n" is formatted into a buffer and never written
        Ok(())
    }
}

impl Driver for Tk {
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        let a = arg.unwrap_or("");
        match opt {
            'g' if !self.ptk => match lookup_x_color(a) {
                Some([r, g, b]) => {
                    ctx.background = crate::gendev::Rgb {
                        red: r,
                        green: g,
                        blue: b,
                    };
                    ctx.bgspec = true;
                }
                None => ctx.warn(format!("Can't parse color '{a}', ignoring background option")),
            },
            'l' => {
                ctx.landscape = true;
                ctx.orientspec = true;
            }
            'p' => {
                ctx.landscape = false;
                ctx.orientspec = true;
            }
            'P' => self.full_page = true,
            'z' => {
                ctx.papersize = a.to_string();
                ctx.paperspec = true;
            }
            'w' if self.ptk => self.wrap = true,
            'G' | 'L' => {}
            _ => {
                return Err(format!(
                    "Argument -{opt} unknown to {} driver.",
                    if self.ptk { "ptk" } else { "tk" }
                ));
            }
        }
        Ok(())
    }

    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        let ptk = self.ptk;
        if !ptk {
            out.puts("# Produced by fig2dev Version 3.2.9a\n");
            self.ppi = ctx.ppi / ctx.mag * 80. / 72.0;
        } else {
            self.ppi = ctx.ppi / ctx.mag;
        }
        ctx.ppi = self.ppi;
        let ppi = self.ppi;
        if !objects.comments.is_empty() {
            out.puts("#\n");
            print_comments(out, "# ", &objects.comments, "");
            out.puts("#\n");
        }
        if !ptk {
            out.puts(
                "# The canvas name (\".c\") can be changed to anything you like.  It only\n# occurs in the following line.  The canvas size can be changed as well.\n\n",
            );
        }
        let (mut wid, mut ht) = (-1f32, -1f32);
        if !self.full_page {
            wid = ctx.urx.wrapping_sub(ctx.llx) as f32;
            ht = ctx.ury.wrapping_sub(ctx.lly) as f32;
            let bw = round(f64::from(wid) / 100.0);
            let bh = round(f64::from(ht) / 100.0);
            ctx.llx = ctx.llx.wrapping_sub(bw);
            ctx.lly = ctx.lly.wrapping_sub(bh);
            ctx.urx = ctx.urx.wrapping_add(bw);
            ctx.ury = ctx.ury.wrapping_add(bh);
            wid = (1.0 * f64::from(ctx.urx.wrapping_sub(ctx.llx)) / ppi) as f32;
            ht = (1.0 * f64::from(ctx.ury.wrapping_sub(ctx.lly)) / ppi) as f32;
        } else {
            if let Some(&(name, w, h)) = PAPERDEF.iter().find(|p| p.0.eq_ignore_ascii_case(&ctx.papersize)) {
                wid = (f64::from(w) / 80.0) as f32;
                ht = (f64::from(h) / 80.0) as f32;
                ctx.papersize = name.to_string();
            }
            if wid < 0. || ht < 0. {
                return Err(format!("Unknown paper size `{}'", ctx.papersize));
            }
            fprintf!(out, "# Page size specified: {}\n", ctx.papersize);
            if ctx.landscape {
                out.puts("# Landscape orientation\n");
                std::mem::swap(&mut wid, &mut ht);
            } else {
                out.puts("# Portrait orientation\n");
            }
        }
        let (wid, ht) = (f64::from(wid), f64::from(ht));
        if ptk {
            if self.wrap {
                out.puts("use strict;\nuse warnings;\nuse Tk;\nmy $xfigure = ");
            }
            out.puts("sub {\nmy $top = shift;\nmy %img;\n");
            fprintf!(
                out,
                "my $c = $top->Canvas(qw/-width {}i -height {}i -bg ivory/);\n",
                fp(wid, 2),
                fp(ht, 2)
            );
            out.puts("#$c->configure(qw/-xscrollincrement 1p -yscrollincrement 1p/);\n");
            if !self.full_page {
                let (ux, lx) = (f64::from(ctx.urx) / ppi, f64::from(ctx.llx) / ppi);
                let (uy, ly) = (f64::from(ctx.ury) / ppi, f64::from(ctx.lly) / ppi);
                let min = |a: f64, b: f64| if a < b { a } else { b };
                let max = |a: f64, b: f64| if a > b { a } else { b };
                fprintf!(
                    out,
                    "$c->configure(-scrollregion => ['{}i','{}i','{}i','{}i']);\n",
                    fp(min(ux, lx), 2),
                    fp(min(uy, ly), 2),
                    fp(max(ux, lx), 2),
                    fp(max(uy, ly), 2)
                );
                out.puts("# Shift canvas by lower of bounding box\n");
                fprintf!(
                    out,
                    "#$c->xview(qw/scroll {} u/);\n",
                    round_mul(f64::from(ctx.llx) / 16.45, ctx.mag)
                );
                fprintf!(
                    out,
                    "#$c->yview(qw/scroll {} u/);\n",
                    round_mul(f64::from(ctx.lly) / 16.45, ctx.mag)
                );
            }
            out.puts("$c->pack(-expand => 1, -fill => 'both');\n\n");
            out.puts(concat!(
                "# Shade and tint colors: &$shade($color, shade), &$tint($color, tint),\n",
                "# where shade and tint are between 0 and 100, respectively.\n",
                "my $shade = sub {\n    my($r, $g, $b, $c, $s);\n",
                "    ($c, $s) = @_;    $s /= 100;\n    ($r, $g, $b) = $top->rgb($c);\n",
                "    return(sprintf('#%.2x%.2x%.2x', $r*$s/256, $g*$s/256, $b*$s/256));",
                "\n};\nmy $tint = sub {\n    my($r, $g, $b, $c, $t);\n",
                "    ($c, $t) = @_;    $t /= 100;\n    ($r, $g, $b) = $top->rgb($c);\n",
                "    return(sprintf('#%.2x%.2x%.2x', 255 - (65535-$r)*$t/256,\n",
                "                   255 - (65535-$g)*$t/256, 255 - (65535-$b)*$t/256));",
                "\n};\n\n"
            ));
            if ctx.pats_used {
                out.puts(
                    "# If the fill-pattern bitmaps should not be placed into\n# the current directory, change the next line accordingly.\n",
                );
                out.puts("my $xbmPath = '.';\n\n");
                for i in 0..NUMPATTERNS as usize {
                    if ctx.pattern_used.get(i).copied().unwrap_or(false) {
                        self.define_pattern(out, i);
                    }
                }
                out.puts("\n");
            }
        } else {
            fprintf!(
                out,
                "set xfigCanvas [canvas .c -width {}i -height {}i]",
                fp(wid, 2),
                fp(ht, 2)
            );
            if ctx.bgspec {
                let b = ctx.background;
                fprintf!(
                    out,
                    " -bg #{:02x}{:02x}{:02x}",
                    b.red / 255,
                    b.green / 255,
                    b.blue / 255
                );
            }
            out.puts("\n");
            out.puts("$xfigCanvas config -xscrollincrement 1p -yscrollincrement 1p\n");
            if !self.full_page {
                out.puts("# Shift canvas by lower of bounding box\n");
                fprintf!(
                    out,
                    "$xfigCanvas xview scroll {} u\n",
                    round_mul(f64::from(ctx.llx) / 16.45, ctx.mag)
                );
                fprintf!(
                    out,
                    "$xfigCanvas yview scroll {} u\n",
                    round_mul(f64::from(ctx.lly) / 16.45, ctx.mag)
                );
            }
            out.puts("pack $xfigCanvas\n\n");
            out.puts(concat!(
                "proc tint {color percent} {\n",
                "    foreach {red green blue} [winfo rgb . $color] {\n",
                "\tset red [expr {255 - ((65535-$red)/256)*$percent/100}]\n",
                "\tset green [expr {255 - ((65535-$green)/256)*$percent/100}]\n",
                "\tset blue [expr {255 - ((65535-$blue)/256)*$percent/100}]\n",
                "\t# break\n",
                "    }\n",
                "    return [format \"#%02x%02x%02x\" $red $green $blue]\n",
                "}\n\n"
            ));
            if ctx.pats_used {
                out.puts(
                    "# If the files defining the stipple bitmaps should not be placed\n# into the current directory, change the next line appropriately.\n",
                );
                out.puts("set xbmPath .\n\n");
                for i in 0..NUMPATTERNS as usize {
                    if ctx.pattern_used.get(i).copied().unwrap_or(false) {
                        self.define_pattern(out, i);
                    }
                }
            }
        }
        out.puts("# The xfig objects begin here.\n");
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, out: &mut Out, a: &Arc) -> Result<(), String> {
        print_comments(out, "# ", &a.comments, "");
        if a.type_ == T_OPEN_ARC {
            if a.for_arrow.is_some() || a.back_arrow.is_some() {
                ctx.warn(if self.ptk {
                    "genptk_arc: arc arrows not supported by Tk."
                } else {
                    "gentk_arc: arc arrows not supported by Tk."
                });
            }
            if a.fill_style != UNFILLED {
                self.draw_filled_shape(ctx, out, Shape::Arc(a, 0), 0, 0, a.fill_color, a.fill_style, 0, 0.0);
            }
            if a.thickness > 0 {
                self.draw_filled_shape(
                    ctx,
                    out,
                    Shape::Arc(a, a.thickness),
                    a.thickness,
                    a.pen_color,
                    0,
                    UNFILLED,
                    a.style,
                    a.style_val,
                );
            }
        } else {
            self.draw_filled_shape(
                ctx,
                out,
                Shape::Arc(a, a.thickness),
                a.thickness,
                a.pen_color,
                a.fill_color,
                a.fill_style,
                a.style,
                a.style_val,
            );
        }
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, out: &mut Out, e: &Ellipse) -> Result<(), String> {
        print_comments(out, "# ", &e.comments, "");
        self.draw_filled_shape(
            ctx,
            out,
            Shape::Ellipse(e),
            e.thickness,
            e.pen_color,
            e.fill_color,
            e.fill_style,
            e.style,
            e.style_val,
        );
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, out: &mut Out, l: &Line) -> Result<(), String> {
        print_comments(out, "# ", &l.comments, "");
        let name = if self.ptk { "genptk_line" } else { "gentk_line" };
        match l.type_ {
            T_POLYLINE => {
                if l.fill_style != UNFILLED && l.points.len() >= 3 {
                    self.draw_filled_shape(
                        ctx,
                        out,
                        Shape::Polygon(&l.points),
                        0,
                        0,
                        l.fill_color,
                        l.fill_style,
                        0,
                        0.0,
                    );
                }
                self.tk_line(ctx, out, l, l.pen_color, l.thickness / 15);
            }
            T_PIC_BOX => self.draw_bitmap(ctx, out, l),
            T_ARC_BOX | T_BOX | T_POLYGON => {
                if l.type_ == T_ARC_BOX {
                    ctx.warn(format!("{name}: arc box not supported."));
                }
                self.draw_filled_shape(
                    ctx,
                    out,
                    Shape::Polygon(&l.points),
                    l.thickness,
                    l.pen_color,
                    l.fill_color,
                    l.fill_style,
                    l.style,
                    l.style_val,
                );
            }
            _ => ctx.warn(format!("{name}: Whatchew talkin' 'bout, Willis?")),
        }
        Ok(())
    }

    fn spline(&mut self, ctx: &mut Ctx, out: &mut Out, s: &Spline) -> Result<(), String> {
        print_comments(out, "# ", &s.comments, "");
        if s.int_spline() {
            self.itp_spline(ctx, out, s)
        } else {
            self.ctl_spline(ctx, out, s)
        }
    }

    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String> {
        let ptk = self.ptk;
        let name = if ptk { "genptk_text" } else { "gentk_text" };
        print_comments(out, "# ", &t.comments, "");
        if t.angle != 0. {
            ctx.warn(format!("{name}: rotated text not supported by Tk."));
        }
        let c = self.canvas();
        let (x, y) = (f6(self.x(t.base_x)), f6(self.x(t.base_y)));
        let bytes = t.raw.clone();
        let bytes = bytes.split(|&b| b == 0).next().unwrap_or(&[]);
        let quote = if ptk { b'\'' } else { b'"' };
        if ptk {
            fprintf!(out, "{c}->createText(qw/{x}i {y}i -text/, '");
        } else {
            fprintf!(out, "{c} create text {x}i {y}i -text \"");
        }
        for &b in bytes {
            if b == quote {
                out.bytes(b"\\");
            }
            out.bytes(&[b]);
        }
        out.puts(if ptk { "'," } else { "\" \\\n" });
        let anchor = match t.type_ {
            T_LEFT_JUSTIFIED | DEFAULT => Some("sw"),
            T_CENTER_JUSTIFIED => Some("s"),
            T_RIGHT_JUSTIFIED => Some("se"),
            _ => {
                ctx.warn(format!("{name}: Unknown text justification"));
                None
            }
        };
        if let Some(an) = anchor {
            if ptk {
                fprintf!(out, ", -anchor => '{an}'");
            } else {
                fprintf!(out, " -anchor {an}");
            }
        }
        let size = if ptk { t.size as i32 } else { (t.size * ctx.mag) as i32 };
        let (prefix, suffix) = if t.psfont_text() {
            usize::try_from(t.font.wrapping_add(1))
                .ok()
                .and_then(|i| FONT_NAMES.get(i))
                .copied()
                .unwrap_or(FONT_NAMES[0])
        } else {
            let fnum = match t.font {
                0 => 0,
                1 => 1,
                2 => 3,
                3 => 2,
                4 => 17,
                5 => 13,
                _ => {
                    ctx.warn(format!("{name}: unknown LaTeX font."));
                    0
                }
            };
            FONT_NAMES[fnum]
        };
        if ptk {
            fprintf!(out, ", -font => \"{prefix}{size}{suffix}\"");
        } else {
            fprintf!(out, " -font \"{prefix}{size}{suffix}\"");
        }
        if t.color != BLACK_COLOR && t.color != DEFAULT {
            let v = self.rgb_color_val(ctx, t.color);
            if ptk {
                fprintf!(out, ", -fill => '#{v:06x}'");
            } else {
                fprintf!(out, " -fill #{v:06x}");
            }
        }
        out.puts(if ptk { ");\n" } else { "\n" });
        Ok(())
    }

    fn end(&mut self, _ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        if self.ptk {
            out.puts("\n}");
            if self.wrap {
                out.puts(";\nmy $mw = MainWindow->new;\n$xfigure->($mw);\nMainLoop;\n");
            } else {
                out.puts("\n");
            }
        } else {
            out.puts("focus $xfigCanvas\n");
        }
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}
