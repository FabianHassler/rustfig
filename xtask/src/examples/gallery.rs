//! The science gallery (port of the Python generator `generate.py` in
//! `examples/gallery/`): 16 drawings from physics and mathematics, written as
//! Fig 3.2 files. Original work, MIT OR Apache-2.0.
//!
//! All text is LaTeX special text (font flags: special bit, LaTeX font),
//! typeset by the pdftex export + pdflatex. The generator places labels using
//! their typeset sizes, which `pdflatex` measured once; the table is in
//! `metrics.rs`, so no LaTeX is needed to regenerate the files. A label that
//! is not in the table falls back to an estimate from its character count.
//!
//! This file holds the drawing toolkit (`Fig`, text-to-LaTeX conversion,
//! helpers); the drawings themselves are in `drawings.rs`.

mod drawings;

use super::metrics::METRICS;
use super::pyish::{round_i, trunc_i};
use std::fmt::Write as _;

pub use drawings::DRAWINGS;

pub type Pt = (f64, f64);

const HEADER: &str =
    "#FIG 3.2  Produced by RustFig's gallery generator\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n";
/// Fig units per point.
const PT: f64 = 1200.0 / 72.0;

// ---- runs: (font, string, level), level -1 sub, +1 super ----

#[derive(Clone, Debug)]
pub struct Run {
    font: &'static str,
    s: String,
    lv: i8,
}

#[allow(non_snake_case)]
pub fn R(s: &str) -> Run {
    Run {
        font: "r",
        s: s.to_string(),
        lv: 0,
    }
}
#[allow(non_snake_case)]
pub fn I(s: &str) -> Run {
    Run {
        font: "i",
        s: s.to_string(),
        lv: 0,
    }
}
#[allow(non_snake_case)]
pub fn H(s: &str) -> Run {
    Run {
        font: "h",
        s: s.to_string(),
        lv: 0,
    }
}
/// Bold Helvetica.
#[allow(non_snake_case)]
pub fn B(s: &str) -> Run {
    Run {
        font: "hb",
        s: s.to_string(),
        lv: 0,
    }
}
/// Symbol font (Greek letters).
#[allow(non_snake_case)]
pub fn G(s: &str) -> Run {
    Run {
        font: "g",
        s: s.to_string(),
        lv: 0,
    }
}
pub fn sub(run: Run) -> Run {
    Run { lv: -1, ..run }
}
pub fn sup(run: Run) -> Run {
    Run { lv: 1, ..run }
}
/// The Symbol minus sign.
pub fn minus() -> Run {
    G("-")
}

/// 10^k as runs.
pub fn pow10(k: i32) -> Vec<Run> {
    let mut v = vec![R("10")];
    if k < 0 {
        v.push(sup(minus()));
    }
    v.push(sup(R(&k.abs().to_string())));
    v
}

/// Text of a label: LaTeX source, or runs converted to LaTeX.
pub enum Txt {
    Src(String),
    Runs(Vec<Run>),
}

impl From<&str> for Txt {
    fn from(s: &str) -> Txt {
        Txt::Src(s.to_string())
    }
}
impl From<String> for Txt {
    fn from(s: String) -> Txt {
        Txt::Src(s)
    }
}
impl From<Vec<Run>> for Txt {
    fn from(v: Vec<Run>) -> Txt {
        Txt::Runs(v)
    }
}

impl Txt {
    fn is_empty(&self) -> bool {
        match self {
            Txt::Src(s) => s.is_empty(),
            Txt::Runs(v) => v.is_empty(),
        }
    }
}

/// Octal escapes for non-ASCII, doubled backslashes (Fig text syntax).
fn esc(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch == '\\' {
            out.push_str("\\\\");
        } else if (ch as u32) > 126 {
            let _ = write!(out, "\\{:03o}", ch as u32);
        } else {
            out.push(ch);
        }
    }
    out
}

fn greek(ch: char) -> Option<&'static str> {
    Some(match ch {
        'q' => r"\theta",
        'a' => r"\alpha",
        'b' => r"\beta",
        'g' => r"\gamma",
        'l' => r"\lambda",
        'm' => r"\mu",
        'p' => r"\pi",
        's' => r"\sigma",
        'w' => r"\omega",
        'D' => r"\Delta",
        'W' => r"\Omega",
        'f' => r"\varphi",
        'S' => r"\Sigma",
        'h' => r"\eta",
        't' => r"\tau",
        'e' => r"\epsilon",
        'n' => r"\nu",
        '-' => "-",
        '=' => "=",
        '+' => "+",
        ' ' => " ",
        '\u{b0}' => r"{}^\circ",
        '\u{b1}' => r"\pm",
        '\u{a3}' => r"\le",
        '\u{f2}' => r"\int",
        '\u{ae}' => r"\to",
        '\u{de}' => r"\Rightarrow",
        '\u{d7}' => r"\times",
        '\u{d6}' => r"\surd",
        _ => return None,
    })
}

fn is_func(t: &str) -> bool {
    matches!(t, "sin" | "cos" | "tan" | "exp")
}

/// Splits like Python's `re.split(r"(\s+|[A-Za-z]{2,})", s)` without the empty pieces.
fn split_tokens(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut toks = Vec::new();
    let mut cur = String::new();
    let mut k = 0;
    while k < chars.len() {
        let c = chars[k];
        let run_end = |pred: &dyn Fn(char) -> bool| {
            let mut j = k;
            while j < chars.len() && pred(chars[j]) {
                j += 1;
            }
            j
        };
        if c.is_whitespace() {
            let j = run_end(&|ch| ch.is_whitespace());
            if !cur.is_empty() {
                toks.push(std::mem::take(&mut cur));
            }
            toks.push(chars[k..j].iter().collect());
            k = j;
        } else if c.is_ascii_alphabetic() {
            let j = run_end(&|ch| ch.is_ascii_alphabetic());
            if j - k >= 2 {
                if !cur.is_empty() {
                    toks.push(std::mem::take(&mut cur));
                }
                toks.push(chars[k..j].iter().collect());
                k = j;
            } else {
                cur.push(c);
                k += 1;
            }
        } else {
            cur.push(c);
            k += 1;
        }
    }
    if !cur.is_empty() {
        toks.push(cur);
    }
    toks
}

fn first_alpha(t: &str) -> bool {
    t.chars().next().is_some_and(|c| c.is_alphabetic())
}

fn math_piece(font: &str, s: &str) -> String {
    if font == "g" {
        let mut out = String::new();
        for ch in s.chars() {
            match greek(ch) {
                Some(m) => out.push_str(m),
                None => out.push(ch),
            }
            if ch.is_alphabetic() {
                out.push(' ');
            }
        }
        return out;
    }
    if font == "i" {
        return s.replace(' ', "");
    }
    // roman: words become \text, known functions \sin etc.
    let toks = split_tokens(s);
    let mut out = String::new();
    for (k, t) in toks.iter().enumerate() {
        let prev = if k > 0 { toks.get(k - 1) } else { None };
        let next = toks.get(k + 1);
        if t.chars().all(char::is_whitespace) {
            let word = |x: Option<&String>| x.is_some_and(|x| first_alpha(x) && x.chars().count() > 1 && !is_func(x));
            let nb = word(prev) || word(next);
            let after_punct = prev.is_some_and(|p| p.ends_with([',', ';', ':']));
            out.push_str(if nb || after_punct { "\\ " } else { "" });
        } else if is_func(t) {
            out.push('\\');
            out.push_str(t);
            out.push(' ');
        } else if t.chars().all(char::is_alphabetic) && t.chars().count() > 1 {
            let _ = write!(out, "\\text{{{t}}}");
        } else if t.chars().all(char::is_alphabetic) {
            let _ = write!(out, "\\mathrm{{{t}}}");
        } else {
            out.push_str(t);
        }
    }
    out
}

fn tesc(s: &str) -> String {
    s.replace('%', r"\%")
        .replace('&', r"\&")
        .replace('#', r"\#")
        .replace('_', r"\_")
}

fn is_text_font(f: &str) -> bool {
    matches!(f, "h" | "o" | "hb")
}

/// Convert runs to LaTeX: Helvetica runs become plain text, the rest math.
fn tex(runs: &[Run]) -> String {
    let mut parts: Vec<(bool, String)> = Vec::new(); // (is math, text)
    let mut k = 0;
    while k < runs.len() {
        let run = &runs[k];
        if is_text_font(run.font) {
            let mut t = tesc(&run.s);
            if run.font == "hb" {
                t = format!("\\textbf{{{t}}}");
            }
            if run.lv != 0 {
                t = format!("${}{{{t}}}$", if run.lv > 0 { "^" } else { "_" });
            }
            parts.push((false, t));
            k += 1;
            continue;
        }
        // math: gather this and following non-text runs, grouping equal sub/sup levels
        let mut m = String::new();
        while k < runs.len() && !is_text_font(runs[k].font) {
            let lv = runs[k].lv;
            if lv == 0 {
                m.push_str(&math_piece(runs[k].font, &runs[k].s));
                k += 1;
            } else {
                let mut grp = String::new();
                while k < runs.len() && runs[k].lv == lv && !is_text_font(runs[k].font) {
                    grp.push_str(&math_piece(runs[k].font, &runs[k].s));
                    k += 1;
                }
                let _ = write!(m, "{}{{{grp}}}", if lv > 0 { "^" } else { "_" });
            }
        }
        parts.push((true, m));
    }
    let mut out = String::new();
    for (is_math, v) in parts {
        if is_math {
            out.push('$');
            out.push_str(&v);
            out.push('$');
        } else {
            out.push_str(&v);
        }
    }
    out.replace("$$", "")
}

/// Estimated width of a label that was not measured.
fn tex_width(src: &str, size: i32) -> f64 {
    // replace each \command by one character, drop $ _ ^ { }
    let mut n = 0usize;
    let chars: Vec<char> = src.chars().collect();
    let mut k = 0;
    while k < chars.len() {
        let c = chars[k];
        if c == '\\' && chars.get(k + 1).is_some_and(|c| c.is_ascii_alphabetic()) {
            k += 1;
            while chars.get(k).is_some_and(|c| c.is_ascii_alphabetic()) {
                k += 1;
            }
            n += 1;
            continue;
        }
        if !matches!(c, '$' | '_' | '^' | '{' | '}') {
            n += 1;
        }
        k += 1;
    }
    n as f64 * 0.5 * size as f64 * PT
}

fn metrics(src: &str, size: i32) -> Option<(f64, f64, f64)> {
    METRICS
        .iter()
        .find(|m| m.0 == src && m.1 == size)
        .map(|m| (m.2, m.3, m.4))
}

// ---- arrows and styles ----

/// An arrow head: type, style, thickness, width, height.
#[derive(Clone, Copy)]
pub struct Arrow(pub i32, pub i32, pub f64, pub f64, pub f64);

impl Arrow {
    fn line(&self) -> String {
        format!("\t{} {} {:.2} {:.2} {:.2}\n", self.0, self.1, self.2, self.3, self.4)
    }
}

/// Style options of a drawn object (Python keyword arguments).
#[derive(Clone, Copy)]
pub struct St {
    pen: i32,
    thick: i32,
    style: i32,
    sv: f64,
    fill: Option<i32>,
    pat: i32,
    depth: i32,
    closed: bool,
    fwd: Option<Arrow>,
    back: Option<Arrow>,
    cap: i32,
    join: i32,
    s: f64,
    ends_angular: bool,
}

impl St {
    pub const fn new() -> St {
        St {
            pen: 0,
            thick: 2,
            style: 0,
            sv: 0.0,
            fill: None,
            pat: 20,
            depth: 50,
            closed: false,
            fwd: None,
            back: None,
            cap: 0,
            join: 0,
            s: -1.0,
            ends_angular: true,
        }
    }
    /// The defaults of a force vector: thick 3, filled triangle head.
    pub const fn v() -> St {
        St::new().thick(3).fwd(A_TRI).depth(D_AXIS)
    }
    pub const fn pen(mut self, v: i32) -> St {
        self.pen = v;
        self
    }
    pub const fn thick(mut self, v: i32) -> St {
        self.thick = v;
        self
    }
    /// Line style and style value (dash length).
    pub const fn st(mut self, style: i32, sv: f64) -> St {
        self.style = style;
        self.sv = sv;
        self
    }
    pub const fn fill(mut self, v: i32) -> St {
        self.fill = Some(v);
        self
    }
    pub const fn pat(mut self, v: i32) -> St {
        self.pat = v;
        self
    }
    pub const fn depth(mut self, v: i32) -> St {
        self.depth = v;
        self
    }
    pub const fn closed(mut self) -> St {
        self.closed = true;
        self
    }
    pub const fn fwd(mut self, a: Arrow) -> St {
        self.fwd = Some(a);
        self
    }
    pub const fn back(mut self, a: Arrow) -> St {
        self.back = Some(a);
        self
    }
    pub const fn cap(mut self, v: i32) -> St {
        self.cap = v;
        self
    }
    pub const fn join(mut self, v: i32) -> St {
        self.join = v;
        self
    }
    /// X-spline shape factor of the inner points.
    pub const fn s(mut self, v: f64) -> St {
        self.s = v;
        self
    }
    pub const fn smooth_ends(mut self) -> St {
        self.ends_angular = false;
        self
    }
}

/// Text options.
#[derive(Clone, Copy)]
pub struct T {
    just: usize,
    size: i32,
    ang: f64,
    col: i32,
    depth: i32,
    halo: bool,
    big: bool,
}

impl T {
    pub const fn s(size: i32) -> T {
        T {
            just: 0,
            size,
            ang: 0.0,
            col: 0,
            depth: D_LABEL,
            halo: false,
            big: false,
        }
    }
    /// 0 left, 1 centred, 2 right.
    pub const fn j(mut self, v: usize) -> T {
        self.just = v;
        self
    }
    pub const fn c(mut self, v: i32) -> T {
        self.col = v;
        self
    }
    pub const fn d(mut self, v: i32) -> T {
        self.depth = v;
        self
    }
    pub const fn a(mut self, deg: f64) -> T {
        self.ang = deg;
        self
    }
    pub const fn halo(mut self) -> T {
        self.halo = true;
        self
    }
    pub const fn big(mut self) -> T {
        self.big = true;
        self
    }
}

pub const A_TRI: Arrow = Arrow(1, 1, 1.0, 110.0, 200.0); // closed filled triangle
pub const A_SMALL: Arrow = Arrow(1, 1, 1.0, 75.0, 140.0);
pub const A_STICK: Arrow = Arrow(0, 0, 1.0, 110.0, 200.0); // stick (open) arrowhead
pub const A_IND: Arrow = Arrow(2, 1, 1.0, 120.0, 230.0); // indented butt

// depths: fills 200..100, grid 90, curves 60, axes/lines 50, markers 40, labels 30, title 20
pub const D_FILL: i32 = 150;
pub const D_GRID: i32 = 90;
pub const D_CURVE: i32 = 60;
pub const D_AXIS: i32 = 50;
pub const D_MARK: i32 = 40;
pub const D_LABEL: i32 = 30;

/// A figure under construction.
pub struct Fig {
    colors: Vec<(String, i32)>,
    /// Output buffers, innermost group last.
    stack: Vec<String>,
    /// Bounding boxes of the open groups (x0, y0, x1, y1).
    bbs: Vec<[f64; 4]>,
    gbb: [f64; 4],
    comment: String,
}

impl Fig {
    pub fn new(comment: &str) -> Fig {
        Fig {
            colors: Vec::new(),
            stack: vec![String::new()],
            bbs: Vec::new(),
            gbb: [1e9, 1e9, -1e9, -1e9],
            comment: comment.to_string(),
        }
    }

    // ---- colours ----
    pub fn color(&mut self, hx: &str) -> i32 {
        if let Some((_, n)) = self.colors.iter().find(|(h, _)| h == hx) {
            return *n;
        }
        let n = 32 + self.colors.len() as i32;
        self.colors.push((hx.to_string(), n));
        n
    }

    // ---- bookkeeping ----
    fn u(&mut self, x: f64, y: f64) {
        let gb = &mut self.gbb;
        gb[0] = gb[0].min(x);
        gb[1] = gb[1].min(y);
        gb[2] = gb[2].max(x);
        gb[3] = gb[3].max(y);
        for bb in &mut self.bbs {
            bb[0] = bb[0].min(x);
            bb[1] = bb[1].min(y);
            bb[2] = bb[2].max(x);
            bb[3] = bb[3].max(y);
        }
    }

    fn emit(&mut self, s: &str) {
        if let Some(top) = self.stack.last_mut() {
            top.push_str(s);
        }
    }

    /// Objects drawn in `body` become one compound.
    pub fn group(&mut self, body: impl FnOnce(&mut Fig)) {
        self.stack.push(String::new());
        self.bbs.push([1e9, 1e9, -1e9, -1e9]);
        body(self);
        let bb = self.bbs.pop().unwrap_or([1e9, 1e9, -1e9, -1e9]);
        let text = self.stack.pop().unwrap_or_default();
        if bb[0] > bb[2] {
            return;
        }
        for p in &mut self.bbs {
            // propagate to parents
            p[0] = p[0].min(bb[0]);
            p[1] = p[1].min(bb[1]);
            p[2] = p[2].max(bb[2]);
            p[3] = p[3].max(bb[3]);
        }
        self.emit(&format!(
            "6 {} {} {} {}\n",
            trunc_i(bb[0] - 20.0),
            trunc_i(bb[1] - 20.0),
            trunc_i(bb[2] + 20.0),
            trunc_i(bb[3] + 20.0)
        ));
        self.emit(&text);
        self.emit("-6\n");
    }

    // ---- objects ----
    fn emit_arrows(&mut self, s: &St) {
        if let Some(a) = &s.fwd {
            self.emit(&a.line());
        }
        if let Some(a) = &s.back {
            self.emit(&a.line());
        }
    }

    fn point_rows(&mut self, pts: &[(i64, i64)]) {
        for chunk in pts.chunks(8) {
            let row: Vec<String> = chunk.iter().map(|(x, y)| format!("{x} {y}")).collect();
            self.emit(&format!("\t {}\n", row.join(" ")));
        }
    }

    fn poly_sub(&mut self, pts: &[Pt], s: St, sub: Option<i32>) {
        let mut pts: Vec<(i64, i64)> = pts.iter().map(|&(x, y)| (round_i(x), round_i(y))).collect();
        if s.closed && pts.first() != pts.last() {
            if let Some(&first) = pts.first() {
                pts.push(first);
            }
        }
        let st = sub.unwrap_or(if s.closed { 3 } else { 1 });
        let area = if s.fill.is_some() { s.pat } else { -1 };
        let fc = s.fill.unwrap_or(7);
        self.emit(&format!(
            "2 {st} {} {} {} {fc} {} -1 {area} {:.3} {} {} -1 {} {} {}\n",
            s.style,
            s.thick,
            s.pen,
            s.depth,
            s.sv,
            s.join,
            s.cap,
            i32::from(s.fwd.is_some()),
            i32::from(s.back.is_some()),
            pts.len()
        ));
        self.emit_arrows(&s);
        self.point_rows(&pts);
        for &(x, y) in &pts {
            self.u(x as f64, y as f64);
        }
    }

    pub fn poly(&mut self, pts: &[Pt], s: St) {
        self.poly_sub(pts, s, None);
    }

    pub fn box_(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, s: St) {
        self.poly_sub(
            &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)],
            St { closed: true, ..s },
            Some(2),
        );
    }

    /// X-spline through pts. s = -1: interpolating (the curve passes the
    /// points); 1: approximating.
    pub fn spline(&mut self, pts: &[Pt], s: St) {
        let pts: Vec<(i64, i64)> = pts.iter().map(|&(x, y)| (round_i(x), round_i(y))).collect();
        let n = pts.len();
        let mut fs = vec![s.s; n];
        if !s.closed && s.ends_angular {
            if let Some(f) = fs.first_mut() {
                *f = 0.0;
            }
            if let Some(f) = fs.last_mut() {
                *f = 0.0;
            }
        }
        let area = if s.fill.is_some() { s.pat } else { -1 };
        let fc = s.fill.unwrap_or(7);
        self.emit(&format!(
            "3 {} {} {} {} {fc} {} -1 {area} {:.3} {} {} {} {n}\n",
            if s.closed { 5 } else { 4 },
            s.style,
            s.thick,
            s.pen,
            s.depth,
            s.sv,
            s.cap,
            i32::from(s.fwd.is_some()),
            i32::from(s.back.is_some()),
        ));
        self.emit_arrows(&s);
        self.point_rows(&pts);
        for chunk in fs.chunks(8) {
            let row: Vec<String> = chunk.iter().map(|f| format!("{f:.3}")).collect();
            self.emit(&format!("\t {}\n", row.join(" ")));
        }
        for &(x, y) in &pts {
            self.u(x as f64, y as f64);
        }
    }

    pub fn circle(&mut self, c: Pt, r: f64, s: St) {
        self.ellipse(c, r, r, s, 3);
    }

    fn ellipse(&mut self, c: Pt, rx: f64, ry: f64, s: St, sub: i32) {
        let area = if s.fill.is_some() { s.pat } else { -1 };
        let fc = s.fill.unwrap_or(7);
        let (cx, cy, rx, ry) = (round_i(c.0), round_i(c.1), round_i(rx).max(1), round_i(ry).max(1));
        self.emit(&format!(
            "1 {sub} {} {} {} {fc} {} -1 {area} {:.3} 1 {:.4} {cx} {cy} {rx} {ry} {cx} {cy} {} {cy}\n",
            s.style,
            s.thick,
            s.pen,
            s.depth,
            s.sv,
            0.0,
            cx + rx
        ));
        self.u((cx - rx) as f64, (cy - ry) as f64);
        self.u((cx + rx) as f64, (cy + ry) as f64);
    }

    /// Circular arc about c (Fig coords) from math angle a0 to a1 degrees (y up
    /// on screen). a1 > a0 runs counterclockwise on screen.
    pub fn arc(&mut self, c: Pt, r: f64, a0: f64, a1: f64, s: St) {
        let (cx, cy) = c;
        let pt = |a: f64| (cx + r * a.to_radians().cos(), cy - r * a.to_radians().sin());
        let (p1, p2, p3) = (pt(a0), pt((a0 + a1) / 2.0), pt(a1));
        let direction = i32::from(a1 > a0);
        let area = if s.fill.is_some() { s.pat } else { -1 };
        let fc = s.fill.unwrap_or(7);
        self.emit(&format!(
            "5 1 {} {} {} {fc} {} -1 {area} {:.3} {} {direction} {} {} {:.3} {:.3} {} {} {} {} {} {}\n",
            s.style,
            s.thick,
            s.pen,
            s.depth,
            s.sv,
            s.cap,
            i32::from(s.fwd.is_some()),
            i32::from(s.back.is_some()),
            cx,
            cy,
            round_i(p1.0),
            round_i(p1.1),
            round_i(p2.0),
            round_i(p2.1),
            round_i(p3.0),
            round_i(p3.1)
        ));
        self.emit_arrows(&s);
        for a in [a0, (a0 + a1) / 2.0, a1] {
            let (x, y) = pt(a);
            self.u(x, y);
        }
    }

    /// LaTeX special text (font flags: special bit, LaTeX default font) at
    /// baseline point (x, y). Text is LaTeX source or runs converted to LaTeX.
    pub fn text(&mut self, x: impl Into<f64>, y: impl Into<f64>, txt: impl Into<Txt>, o: T) -> f64 {
        let (x, y) = (x.into(), y.into());
        let src = match txt.into() {
            Txt::Src(s) => s,
            Txt::Runs(v) => tex(&v),
        };
        let size = if o.big {
            o.size
        } else if o.size <= 10 {
            10
        } else {
            o.size.min(12)
        };
        let met = metrics(&src, size);
        let w = met.map_or_else(|| tex_width(&src, size), |m| m.0);
        let a = o.ang.to_radians();
        let (ux, uy) = (a.cos(), -a.sin());
        let (nx, ny) = (a.sin(), a.cos());
        let start = [0.0, -w / 2.0, -w].get(o.just).copied().unwrap_or(0.0);
        let h = met.map_or(size as f64 * PT * 0.8, |m| m.1 + m.2);
        if o.halo {
            // white box behind the text, sized from the measured typeset label (use sparingly)
            let (x0, x1) = (x + start - 40.0, x + start + w + 40.0);
            let (ht, dp) = met.map_or((size as f64 * PT * 0.7, size as f64 * PT * 0.25), |m| (m.1, m.2));
            self.box_(
                x0,
                y - ht - 40.0,
                x1,
                y + dp + 40.0,
                St::new().pen(7).thick(0).fill(7).depth(o.depth + 2),
            );
        }
        self.emit(&format!(
            "4 {} {} {} -1 0 {:.1} {:.4} 2 {} {} {} {} {}\\001\n",
            o.just,
            o.col,
            o.depth,
            size as f64,
            a,
            round_i(h),
            round_i(w),
            round_i(x),
            round_i(y),
            esc(&src)
        ));
        for k in [0.0, 1.0] {
            let (px, py) = (x + ux * (start + w * k), y + uy * (start + w * k));
            self.u(px, py);
            self.u(px - nx * h, py - ny * h);
        }
        w
    }

    /// `text` at a point.
    pub fn tp(&mut self, p: Pt, txt: impl Into<Txt>, o: T) -> f64 {
        self.text(p.0, p.1, txt, o)
    }

    /// The file's text: header, comment, colours, objects, and the invisible
    /// white frame that fixes the cropped page (padding for text that LaTeX
    /// sets wider than estimated).
    pub fn render(&self) -> String {
        let mut out = String::from(HEADER);
        out.push_str("# ");
        out.push_str(&self.comment);
        out.push('\n');
        for (hx, n) in &self.colors {
            let _ = writeln!(out, "0 {n} {hx}");
        }
        out.push_str(self.stack.first().map_or("", String::as_str));
        let gb = &self.gbb;
        let (x0, y0) = (round_i(gb[0]) - 150, round_i(gb[1]) - 150);
        let (x1, y1) = (round_i(gb[2]) + 250, round_i(gb[3]) + 200);
        let _ = write!(
            out,
            "2 2 0 1 7 7 999 -1 -1 0.000 0 0 -1 0 0 5\n\t {x0} {y0} {x1} {y0} {x1} {y1} {x0} {y1} {x0} {y0}\n"
        );
        out
    }
}

/// Maps math coordinates (y up) to Fig coordinates (y down).
#[derive(Clone, Copy)]
pub struct Fr {
    ox: f64,
    oy: f64,
    sx: f64,
    sy: f64,
}

impl Fr {
    pub fn new(ox: f64, oy: f64, sx: f64) -> Fr {
        Fr { ox, oy, sx, sy: sx }
    }
    pub fn new2(ox: f64, oy: f64, sx: f64, sy: f64) -> Fr {
        Fr { ox, oy, sx, sy }
    }
    pub fn p(&self, x: impl Into<f64>, y: impl Into<f64>) -> Pt {
        (self.ox + self.sx * x.into(), self.oy - self.sy * y.into())
    }
    pub fn ps(&self, pts: &[Pt]) -> Vec<Pt> {
        pts.iter().map(|&(x, y)| self.p(x, y)).collect()
    }
}

/// The gallery palette: ink black, four accents, a grey, and light tints of
/// them for fills.
#[derive(Clone, Copy)]
pub struct Pal {
    pub blue: i32,
    pub red: i32,
    pub green: i32,
    pub orange: i32,
    pub grey: i32,
    pub lblue: i32,
    pub lred: i32,
    pub lorange: i32,
    pub lgrey: i32,
    pub hatch: i32,
    pub hp: i32,
}

pub fn pal(f: &mut Fig) -> Pal {
    let blue = f.color("#1f5fa8");
    let red = f.color("#c0392b");
    let green = f.color("#2e8b57");
    let orange = f.color("#d98a1f");
    let grey = f.color("#7f7f7f");
    let lblue = f.color("#cfe0f3");
    let lred = f.color("#f3d3cf");
    let _lgreen = f.color("#d3ebdd");
    let lorange = f.color("#f6e2c2");
    let lgrey = f.color("#e4e4e4");
    let hatch = f.color("#b4b4b4");
    let hp = f.color("#a0a0a0");
    Pal {
        blue,
        red,
        green,
        orange,
        grey,
        lblue,
        lred,
        lorange,
        lgrey,
        hatch,
        hp,
    }
}

pub fn title(f: &mut Fig, name: &str, caption: &str) {
    f.text(900, 1050, format!("\\textbf{{{name}}}"), T::s(14).big().d(20));
    let grey = f.color("#6f6f6f");
    f.text(900, 1380, caption, T::s(10).c(grey).d(20));
}

pub fn axis(f: &mut Fig, a: Pt, b: Pt, s: St) {
    let s = if s.fwd.is_none() { s.fwd(A_STICK) } else { s };
    f.poly(&[a, b], s);
}

pub fn seg(f: &mut Fig, a: Pt, b: Pt, s: St) {
    f.poly(&[a, b], s);
}

/// A short arrow-carrying segment sitting on the line a-b at parameter t
/// (direction a -> b).
pub fn mid_arrow(f: &mut Fig, a: Pt, b: Pt, t: f64, span: f64, s: St) {
    let p = (a.0 + (b.0 - a.0) * (t - span), a.1 + (b.1 - a.1) * (t - span));
    let q = (a.0 + (b.0 - a.0) * (t + span), a.1 + (b.1 - a.1) * (t + span));
    f.poly(&[p, q], s);
}

pub fn xaxis_ticks(f: &mut Fig, fr: &Fr, xs: &[f64], y: f64, labels: Vec<Txt>) {
    for (&v, lab) in xs.iter().zip(labels) {
        let (x, yy) = fr.p(v, y);
        seg(f, (x, yy), (x, yy + 90.0), St::new().thick(1).depth(D_AXIS));
        if !lab.is_empty() {
            f.text(x, yy + 190.0 + 120.0, lab, T::s(10).j(1));
        }
    }
}

pub fn yaxis_ticks(f: &mut Fig, fr: &Fr, x: f64, ys: &[f64], labels: Vec<Txt>) {
    for (&v, lab) in ys.iter().zip(labels) {
        let (xx, y) = fr.p(x, v);
        seg(f, (xx, y), (xx - 90.0, y), St::new().thick(1).depth(D_AXIS));
        if !lab.is_empty() {
            f.text(xx - (90.0 + 70.0), y + 70.0, lab, T::s(11).j(2));
        }
    }
}

/// n + 1 samples of fn on [a, b].
pub fn sample(func: impl Fn(f64) -> f64, a: f64, b: f64, n: usize) -> Vec<Pt> {
    (0..=n)
        .map(|k| {
            let x = a + (b - a) * k as f64 / n as f64;
            (x, func(x))
        })
        .collect()
}

/// Arrow from o along screen vector v.
pub fn vec(f: &mut Fig, o: Pt, v: Pt, pen: i32, s: St) {
    f.poly(&[o, (o.0 + v.0, o.1 + v.1)], s.pen(pen));
}

/// The 16 drawings, as (file stem, Fig text).
pub fn all() -> Result<Vec<(String, String)>, String> {
    DRAWINGS
        .iter()
        .map(|(name, draw)| Ok((name.replace('_', "-"), draw()?.render())))
        .collect()
}
