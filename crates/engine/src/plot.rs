//! Plot a curve (plan E12; an addition, xfig has no such mode): a small
//! expression parser and the geometry of a plot made of Fig objects.
//!
//! A plot is one compound: the frame, grid, ticks and numbers, axis labels
//! and the curve as polylines. The plot's parameters ([`PlotSpec`]) are kept
//! as a comment line of the compound ([`PLOT_COMMENT`]), so Fig 3.2 files
//! keep them and "Edit plot..." can redraw it.
//!
//! Everything here is pure (no files, no engine): the input is untrusted
//! (typed expressions, data files, JSON), so lengths, depths, sample counts
//! and points are capped and nothing panics.

use rustfig_geom::{Point, Rect};
use rustfig_model::{
    text_flags, ColorRef, Compound, Justify, LineKind, Object, ObjectKind, PolyKind, Polyline, Style, Text, MAX_DEPTH,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Longest expression, in characters.
pub const MAX_EXPR_LEN: usize = 500;
/// Deepest nesting of parentheses, signs and powers.
pub const MAX_NEST: usize = 64;
/// Most nodes of one expression.
pub const MAX_NODES: usize = 2_000;
/// Most samples the user may ask for.
pub const MAX_SAMPLES: usize = 20_000;
/// Most points of the curve, adaptive refinement included.
pub const MAX_POINTS: usize = 100_000;
/// Largest data file read, in bytes.
pub const MAX_DATA_BYTES: u64 = 16 * 1024 * 1024;
/// Most data rows used.
pub const MAX_DATA_ROWS: usize = 200_000;
/// Most ticks on one axis.
const MAX_TICKS: i64 = 200;
/// Longest label or file name kept.
const MAX_TEXT: usize = 300;
/// Values beyond this count as infinite (no overflow in later arithmetic).
const BIG: f64 = 1e150;
/// The comment line that marks a compound as a plot: prefix, then JSON.
pub const PLOT_COMMENT: &str = "rustfig-plot:";
/// Longest comment line Fig files keep (xfig reads 1024 characters a line).
const MAX_COMMENT: usize = 1000;

// ---- expressions ---------------------------------------------------------

type F1 = fn(f64) -> f64;
type F2 = fn(f64, f64) -> f64;

#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Var(usize),
    Neg(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
    Call1(F1, Box<Expr>),
    Call2(F2, Box<Expr>, Box<Expr>),
}

/// A parsed expression in one or more variables.
#[derive(Debug, Clone)]
pub struct Formula {
    root: Expr,
}

impl Formula {
    /// The value for the variables (in the order given to [`parse`]); may be
    /// NaN or infinite.
    pub fn eval(&self, vars: &[f64]) -> f64 {
        eval(&self.root, vars)
    }
}

fn eval(e: &Expr, vars: &[f64]) -> f64 {
    match e {
        Expr::Num(v) => *v,
        Expr::Var(i) => vars.get(*i).copied().unwrap_or(f64::NAN),
        Expr::Neg(a) => -eval(a, vars),
        Expr::Bin(op, a, b) => {
            let (x, y) = (eval(a, vars), eval(b, vars));
            match op {
                '+' => x + y,
                '-' => x - y,
                '*' => x * y,
                '/' => x / y,
                _ => x.powf(y),
            }
        }
        Expr::Call1(f, a) => f(eval(a, vars)),
        Expr::Call2(f, a, b) => f(eval(a, vars), eval(b, vars)),
    }
}

fn sign(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        x
    } else {
        x.signum()
    }
}

const FUNCS1: &[(&str, F1)] = &[
    ("sin", f64::sin),
    ("cos", f64::cos),
    ("tan", f64::tan),
    ("asin", f64::asin),
    ("acos", f64::acos),
    ("atan", f64::atan),
    ("sinh", f64::sinh),
    ("cosh", f64::cosh),
    ("tanh", f64::tanh),
    ("exp", f64::exp),
    ("ln", f64::ln),
    ("log", f64::ln),
    ("log10", f64::log10),
    ("log2", f64::log2),
    ("sqrt", f64::sqrt),
    ("abs", f64::abs),
    ("floor", f64::floor),
    ("ceil", f64::ceil),
    ("sign", sign),
];

const FUNCS2: &[(&str, F2)] = &[
    ("atan2", f64::atan2),
    ("min", f64::min),
    ("max", f64::max),
    ("pow", f64::powf),
];

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Name(String),
    Op(char),
    Pow,
    LParen,
    RParen,
    Comma,
}

fn tokenize(src: &str) -> Result<Vec<(Tok, usize)>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        let at = i + 1;
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while chars.get(i).is_some_and(|c| c.is_ascii_digit()) {
                i += 1;
            }
            if chars.get(i) == Some(&'.') {
                i += 1;
                while chars.get(i).is_some_and(|c| c.is_ascii_digit()) {
                    i += 1;
                }
            }
            // An exponent only when digits follow ("2e" is 2 times e).
            if matches!(chars.get(i), Some('e' | 'E')) {
                let mut j = i + 1;
                if matches!(chars.get(j), Some('+' | '-')) {
                    j += 1;
                }
                if chars.get(j).is_some_and(|c| c.is_ascii_digit()) {
                    while chars.get(j).is_some_and(|c| c.is_ascii_digit()) {
                        j += 1;
                    }
                    i = j;
                }
            }
            let text: String = chars.get(start..i).unwrap_or_default().iter().collect();
            let v: f64 = text.parse().map_err(|_| format!("bad number {text:?} at {at}"))?;
            if !v.is_finite() {
                return Err(format!("number {text:?} at {at} is too large"));
            }
            out.push((Tok::Num(v), at));
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while chars.get(i).is_some_and(|c| c.is_alphanumeric() || *c == '_') {
                i += 1;
            }
            out.push((Tok::Name(chars.get(start..i).unwrap_or_default().iter().collect()), at));
        } else {
            i += 1;
            let t = match c {
                '+' => Tok::Op('+'),
                '-' | '\u{2212}' => Tok::Op('-'),
                '*' | '\u{d7}' | '\u{b7}' => {
                    if c == '*' && chars.get(i) == Some(&'*') {
                        i += 1;
                        Tok::Pow
                    } else {
                        Tok::Op('*')
                    }
                }
                '/' | '\u{f7}' => Tok::Op('/'),
                '^' => Tok::Pow,
                '(' => Tok::LParen,
                ')' => Tok::RParen,
                ',' => Tok::Comma,
                _ => return Err(format!("unexpected {c:?} at {at}")),
            };
            out.push((t, at));
        }
    }
    Ok(out)
}

struct Parser<'a> {
    toks: Vec<(Tok, usize)>,
    pos: usize,
    nest: usize,
    nodes: usize,
    vars: &'a [&'a str],
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(t, _)| t)
    }

    fn here(&self) -> usize {
        self.toks.get(self.pos).or(self.toks.last()).map_or(1, |(_, p)| *p)
    }

    fn node(&mut self, e: Expr) -> Result<Expr, String> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err("the expression is too long".into());
        }
        Ok(e)
    }

    fn enter(&mut self) -> Result<(), String> {
        self.nest += 1;
        if self.nest > MAX_NEST {
            return Err("the expression is nested too deeply".into());
        }
        Ok(())
    }

    fn expr(&mut self) -> Result<Expr, String> {
        let mut a = self.term()?;
        while let Some(Tok::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.pos += 1;
            let b = self.term()?;
            a = self.node(Expr::Bin(op, Box::new(a), Box::new(b)))?;
        }
        Ok(a)
    }

    fn term(&mut self) -> Result<Expr, String> {
        let mut a = self.unary()?;
        while let Some(Tok::Op(op @ ('*' | '/'))) = self.peek().cloned() {
            self.pos += 1;
            let b = self.unary()?;
            a = self.node(Expr::Bin(op, Box::new(a), Box::new(b)))?;
        }
        Ok(a)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Tok::Op('-')) => {
                self.pos += 1;
                self.enter()?;
                let a = self.unary()?;
                self.nest -= 1;
                self.node(Expr::Neg(Box::new(a)))
            }
            Some(Tok::Op('+')) => {
                self.pos += 1;
                self.enter()?;
                let a = self.unary()?;
                self.nest -= 1;
                Ok(a)
            }
            _ => self.power(),
        }
    }

    /// `a ^ b` binds tighter than a sign on the left (`-x^2` is `-(x^2)`)
    /// and is right associative; a sign is allowed in the exponent.
    fn power(&mut self) -> Result<Expr, String> {
        let a = self.primary()?;
        if self.peek() == Some(&Tok::Pow) {
            self.pos += 1;
            self.enter()?;
            let b = self.unary()?;
            self.nest -= 1;
            return self.node(Expr::Bin('^', Box::new(a), Box::new(b)));
        }
        Ok(a)
    }

    fn expect(&mut self, t: Tok, what: &str) -> Result<(), String> {
        if self.peek() == Some(&t) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("expected {what} at {}", self.here()))
        }
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let at = self.here();
        let Some(tok) = self.peek().cloned() else {
            return Err("the expression ends too early".into());
        };
        self.pos += 1;
        match tok {
            Tok::Num(v) => self.node(Expr::Num(v)),
            Tok::LParen => {
                self.enter()?;
                let e = self.expr()?;
                self.nest -= 1;
                self.expect(Tok::RParen, "\")\"")?;
                Ok(e)
            }
            Tok::Name(name) => {
                if self.peek() == Some(&Tok::LParen) {
                    self.pos += 1;
                    self.enter()?;
                    let a = self.expr()?;
                    let r = if let Some((_, f)) = FUNCS1.iter().find(|(n, _)| *n == name) {
                        Expr::Call1(*f, Box::new(a))
                    } else if let Some((_, f)) = FUNCS2.iter().find(|(n, _)| *n == name) {
                        self.expect(Tok::Comma, "\",\" (two arguments)")?;
                        let b = self.expr()?;
                        Expr::Call2(*f, Box::new(a), Box::new(b))
                    } else {
                        return Err(format!("unknown function {name:?} at {at}"));
                    };
                    self.nest -= 1;
                    self.expect(Tok::RParen, "\")\"")?;
                    return self.node(r);
                }
                if let Some(i) = self.vars.iter().position(|v| *v == name) {
                    return self.node(Expr::Var(i));
                }
                match name.as_str() {
                    "pi" => self.node(Expr::Num(std::f64::consts::PI)),
                    "e" => self.node(Expr::Num(std::f64::consts::E)),
                    _ => Err(format!("unknown name {name:?} at {at}")),
                }
            }
            Tok::Comma => Err(format!("unexpected \",\" at {at}")),
            Tok::RParen => Err(format!("unexpected \")\" at {at}")),
            Tok::Op(c) => Err(format!("unexpected {c:?} at {at}")),
            Tok::Pow => Err(format!("unexpected \"^\" at {at}")),
        }
    }
}

/// Parse `src` with the variables `vars` (their values are given to
/// [`Formula::eval`] in this order). Operators + − × ÷ ^ (also `*`, `/`,
/// `**`), parentheses, `pi`, `e`, and the functions sin cos tan asin acos
/// atan sinh cosh tanh exp ln log (natural) log10 log2 sqrt abs floor ceil
/// sign, and atan2 min max pow with two arguments.
pub fn parse(src: &str, vars: &[&str]) -> Result<Formula, String> {
    if src.chars().count() > MAX_EXPR_LEN {
        return Err(format!("the expression is longer than {MAX_EXPR_LEN} characters"));
    }
    let toks = tokenize(src)?;
    if toks.is_empty() {
        return Err("the expression is empty".into());
    }
    let mut p = Parser {
        toks,
        pos: 0,
        nest: 0,
        nodes: 0,
        vars,
    };
    let root = p.expr()?;
    if p.pos < p.toks.len() {
        return Err(format!("unexpected input at {}", p.here()));
    }
    Ok(Formula { root })
}

// ---- the plot's parameters -----------------------------------------------

/// What is plotted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlotMode {
    /// y = f(x)
    Function,
    /// x(t), y(t)
    Parametric,
    /// Columns of a CSV or whitespace separated file.
    Data,
}

/// The parameters of a plot, as the dialog and `plot.add` give them and as
/// the compound's comment keeps them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlotSpec {
    pub mode: PlotMode,
    /// y = f(x) (variable `x`).
    pub expr: String,
    /// x(t) and y(t) (variable `t`).
    pub xt: String,
    pub yt: String,
    /// Data file and its 1-based columns (x column 0: the row number).
    pub file: String,
    pub xcol: usize,
    pub ycol: usize,
    /// Ranges; `None` fits the data (function: x defaults to -10..10).
    pub xmin: Option<f64>,
    pub xmax: Option<f64>,
    pub ymin: Option<f64>,
    pub ymax: Option<f64>,
    /// Parameter range of a parametric curve (default 0..2π).
    pub tmin: Option<f64>,
    pub tmax: Option<f64>,
    /// Samples before adaptive refinement.
    pub samples: usize,
    /// Frame with ticks and numbers.
    pub axes: bool,
    pub grid: bool,
    pub log_x: bool,
    pub log_y: bool,
    pub xlabel: String,
    pub ylabel: String,
    /// Axis labels are LaTeX (special text).
    pub latex: bool,
    /// The plot's frame in Fig units (min x, min y, max x, max y); written
    /// by the command, used by "Edit plot" when the frame is missing.
    pub rect: Option<[f64; 4]>,
}

impl Default for PlotSpec {
    fn default() -> Self {
        PlotSpec {
            mode: PlotMode::Function,
            expr: "sin(x)".into(),
            xt: "cos(t)".into(),
            yt: "sin(t)".into(),
            file: String::new(),
            xcol: 1,
            ycol: 2,
            xmin: Some(0.0),
            xmax: Some(std::f64::consts::TAU),
            ymin: None,
            ymax: None,
            tmin: None,
            tmax: None,
            samples: 400,
            axes: true,
            grid: false,
            log_x: false,
            log_y: false,
            xlabel: String::new(),
            ylabel: String::new(),
            latex: false,
            rect: None,
        }
    }
}

impl PlotSpec {
    /// A spec from JSON params (unknown keys are ignored), checked.
    pub fn from_value(v: &Value) -> Result<PlotSpec, String> {
        let spec: PlotSpec = serde_json::from_value(v.clone()).map_err(|e| format!("bad plot parameters: {e}"))?;
        spec.check()?;
        Ok(spec)
    }

    fn check(&self) -> Result<(), String> {
        let nums = [self.xmin, self.xmax, self.ymin, self.ymax, self.tmin, self.tmax];
        if nums.iter().flatten().any(|v| !v.is_finite() || v.abs() > BIG) {
            return Err("ranges must be finite numbers".into());
        }
        if !(2..=MAX_SAMPLES).contains(&self.samples) {
            return Err(format!("samples must be between 2 and {MAX_SAMPLES}"));
        }
        for s in [&self.expr, &self.xt, &self.yt, &self.file, &self.xlabel, &self.ylabel] {
            if s.chars().count() > MAX_TEXT.max(MAX_EXPR_LEN) {
                return Err("a text of the plot is too long".into());
            }
        }
        if self.xcol > 1_000 || self.ycol == 0 || self.ycol > 1_000 {
            return Err("columns are numbered from 1 (x column 0 means the row number)".into());
        }
        if let Some(r) = self.rect {
            if r.iter().any(|v| !v.is_finite() || v.abs() > 1e9) {
                return Err("rect must be finite".into());
            }
        }
        Ok(())
    }

    /// The comment line that keeps this spec (only values that differ from
    /// the defaults), or an error when it would be too long for a Fig file.
    pub fn comment(&self) -> Result<String, String> {
        let mut v = serde_json::to_value(self).map_err(|e| e.to_string())?;
        let d = serde_json::to_value(PlotSpec::default()).map_err(|e| e.to_string())?;
        // Which expression fields matter depends on the mode.
        let mode = self.mode;
        if let (Some(m), Some(dm)) = (v.as_object_mut(), d.as_object()) {
            m.retain(|k, x| k == "mode" || k == "rect" || dm.get(k) != Some(x));
            let drop: &[&str] = match mode {
                PlotMode::Function => &["xt", "yt", "file", "xcol", "ycol", "tmin", "tmax"],
                PlotMode::Parametric => &["expr", "file", "xcol", "ycol"],
                PlotMode::Data => &["expr", "xt", "yt", "tmin", "tmax", "samples"],
            };
            for k in drop {
                m.remove(*k);
            }
        }
        let line = format!("{PLOT_COMMENT} {v}");
        if line.chars().count() > MAX_COMMENT {
            return Err("the plot's description is too long to keep in the figure".into());
        }
        Ok(line)
    }

    /// The spec a comment line keeps, if it is a plot comment.
    pub fn from_comment(line: &str) -> Option<PlotSpec> {
        let json = line.trim().strip_prefix(PLOT_COMMENT)?;
        let v: Value = serde_json::from_str(json.trim()).ok()?;
        PlotSpec::from_value(&v).ok()
    }

    /// The spec of a plot compound (its comment), if it is one.
    pub fn of_object(o: &Object) -> Option<PlotSpec> {
        if !matches!(o.kind, ObjectKind::Compound(_)) {
            return None;
        }
        o.comments.iter().find_map(|c| PlotSpec::from_comment(c))
    }
}

// ---- data ----------------------------------------------------------------

/// The numbers of a data file: lines of values separated by commas,
/// semicolons, tabs or spaces; `#`/`%` lines and lines that are not numbers
/// (a header) are skipped. Returns the (x, y) of the chosen columns.
pub fn parse_table(text: &str, xcol: usize, ycol: usize) -> Result<Vec<(f64, f64)>, String> {
    let need = xcol.max(ycol);
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('%') {
            continue;
        }
        let cells: Vec<&str> = line.split([',', ';', '\t', ' ']).filter(|c| !c.is_empty()).collect();
        if cells.len() < need {
            continue;
        }
        let num = |col: usize| -> Option<f64> {
            let v: f64 = cells.get(col.checked_sub(1)?)?.parse().ok()?;
            (v.is_finite() && v.abs() <= BIG).then_some(v)
        };
        let y = num(ycol);
        let x = if xcol == 0 {
            Some(out.len() as f64 + 1.0)
        } else {
            num(xcol)
        };
        if let (Some(x), Some(y)) = (x, y) {
            out.push((x, y));
            if out.len() >= MAX_DATA_ROWS {
                break;
            }
        }
    }
    if out.is_empty() {
        return Err(format!(
            "no rows with numbers in columns {xcol} and {ycol} (columns are numbered from 1)"
        ));
    }
    Ok(out)
}

// ---- sampling ------------------------------------------------------------

/// A point of the plot's own coordinates (log axes already transformed).
type P = (f64, f64);

/// `a < b`, false for NaN too (a range must have min below max).
fn ordered(a: f64, b: f64) -> bool {
    a < b
}

fn finite(v: f64) -> Option<f64> {
    (v.is_finite() && v.abs() <= BIG).then_some(v)
}

/// The axis transform: logarithms of positive values on a log axis.
fn tr(v: f64, log: bool) -> Option<f64> {
    if log {
        if v > 0.0 {
            finite(v.log10())
        } else {
            None
        }
    } else {
        finite(v)
    }
}

struct Refiner<'a> {
    f: &'a dyn Fn(f64) -> Option<P>,
    /// Normalising spans of the two axes (curvature is judged on screen).
    sx: f64,
    sy: f64,
    budget: usize,
}

impl Refiner<'_> {
    /// Points between `u0` and `u1` where the curve bends or breaks off.
    fn refine(&mut self, u0: f64, p0: Option<P>, u1: f64, p1: Option<P>, depth: u32, out: &mut Vec<(f64, Option<P>)>) {
        if depth == 0 || self.budget == 0 || (p0.is_none() && p1.is_none()) {
            return;
        }
        let um = 0.5 * (u0 + u1);
        if um <= u0 || um >= u1 {
            return;
        }
        self.budget -= 1;
        let pm = (self.f)(um);
        let split = match (p0, p1, pm) {
            (Some(a), Some(b), Some(m)) => {
                let dx = (m.0 - 0.5 * (a.0 + b.0)) / self.sx;
                let dy = (m.1 - 0.5 * (a.1 + b.1)) / self.sy;
                dx.abs().max(dy.abs()) > 0.001
            }
            // An end that is missing: look for where the curve stops.
            _ => true,
        };
        if split {
            self.refine(u0, p0, um, pm, depth - 1, out);
            out.push((um, pm));
            self.refine(um, pm, u1, p1, depth - 1, out);
        }
    }
}

/// A range from the values, for an axis with no range given: the full range
/// unless a few outliers (poles) stretch it, then the bulk.
fn fit_range(vals: &mut Vec<f64>) -> Option<(f64, f64)> {
    vals.retain(|v| v.is_finite());
    if vals.is_empty() {
        return None;
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = vals.len();
    let at = |q: f64| vals.get(((n - 1) as f64 * q).round() as usize).copied().unwrap_or(0.0);
    let (lo, hi) = (at(0.0), at(1.0));
    let (qlo, qhi) = (at(0.02), at(0.98));
    if qhi > qlo && hi - lo > 3.0 * (qhi - qlo) {
        let pad = 0.1 * (qhi - qlo);
        Some((qlo - pad, qhi + pad))
    } else {
        Some((lo, hi))
    }
}

/// A step of 1, 2 or 5 times a power of ten that gives about `target` steps.
fn nice_step(span: f64, target: f64) -> f64 {
    let raw = span / target;
    if !(raw.is_finite() && raw > 0.0) {
        return 1.0;
    }
    let p = 10f64.powf(raw.log10().floor());
    let m = raw / p;
    let n = if m < 1.5 {
        1.0
    } else if m < 3.5 {
        2.0
    } else if m < 7.5 {
        5.0
    } else {
        10.0
    };
    n * p
}

/// An automatic range widened to tick positions (decades on a log axis).
fn nice_range(lo: f64, hi: f64, log: bool) -> (f64, f64) {
    let (mut lo, mut hi) = (lo, hi);
    if hi - lo <= 1e-12 * lo.abs().max(hi.abs()).max(1e-300) {
        lo -= 1.0;
        hi += 1.0;
    }
    if log {
        let (a, b) = (lo.floor(), hi.ceil());
        return if b > a { (a, b) } else { (a, a + 1.0) };
    }
    let step = nice_step(hi - lo, 6.0);
    ((lo / step).floor() * step, (hi / step).ceil() * step)
}

struct Ranges {
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
}

/// The curve as pieces of points in the plot's coordinates (transformed by
/// the axes' logarithms), and the ranges.
fn sample(spec: &PlotSpec, data: Option<&str>) -> Result<(Vec<Vec<P>>, Ranges), String> {
    let (lx, ly) = (spec.log_x, spec.log_y);
    let given = |v: Option<f64>, log: bool, name: &str| -> Result<Option<f64>, String> {
        match v {
            None => Ok(None),
            Some(v) => tr(v, log)
                .map(Some)
                .ok_or_else(|| format!("{name} must be positive on a logarithmic axis")),
        }
    };
    let (gx0, gx1) = (given(spec.xmin, lx, "x range")?, given(spec.xmax, lx, "x range")?);
    let (gy0, gy1) = (given(spec.ymin, ly, "y range")?, given(spec.ymax, ly, "y range")?);

    // Raw samples: (parameter, point) with the parameter ordering them.
    let mut pts: Vec<(f64, Option<P>)> = Vec::new();
    let mut curve: Option<Box<dyn Fn(f64) -> Option<P>>> = None;
    let (mut u0, mut u1) = (0.0, 1.0);
    match spec.mode {
        PlotMode::Data => {
            let text = data.ok_or("the plot has no data (give a file)")?;
            for (i, (x, y)) in parse_table(text, spec.xcol, spec.ycol)?.into_iter().enumerate() {
                let p = tr(x, lx).zip(tr(y, ly));
                pts.push((i as f64, p));
            }
        }
        PlotMode::Function => {
            let f = parse(&spec.expr, &["x"])?;
            let (a, b) = (
                gx0.unwrap_or(if lx { -1.0 } else { -10.0 }),
                gx1.unwrap_or(if lx { 2.0 } else { 10.0 }),
            );
            if !ordered(a, b) {
                return Err("the x range must have min below max".into());
            }
            (u0, u1) = (a, b);
            curve = Some(Box::new(move |u| {
                let x = if lx { 10f64.powf(u) } else { u };
                tr(f.eval(&[x]), ly).map(|y| (u, y))
            }));
        }
        PlotMode::Parametric => {
            let fx = parse(&spec.xt, &["t"])?;
            let fy = parse(&spec.yt, &["t"])?;
            let (a, b) = (spec.tmin.unwrap_or(0.0), spec.tmax.unwrap_or(std::f64::consts::TAU));
            if !ordered(a, b) {
                return Err("the t range must have min below max".into());
            }
            (u0, u1) = (a, b);
            curve = Some(Box::new(move |t| {
                let x = tr(fx.eval(&[t]), lx)?;
                let y = tr(fy.eval(&[t]), ly)?;
                Some((x, y))
            }));
        }
    }
    if let Some(f) = &curve {
        let n = spec.samples;
        for i in 0..=n {
            let u = if i == n {
                u1
            } else {
                u0 + (u1 - u0) * i as f64 / n as f64
            };
            pts.push((u, f(u)));
        }
    }

    // Ranges.
    let mut xs: Vec<f64> = pts.iter().filter_map(|(_, p)| p.map(|p| p.0)).collect();
    let mut ys: Vec<f64> = pts.iter().filter_map(|(_, p)| p.map(|p| p.1)).collect();
    if xs.is_empty() {
        return Err("the curve has no finite values in this range".into());
    }
    let auto = |given: (Option<f64>, Option<f64>), vals: &mut Vec<f64>, log: bool| {
        if let (Some(a), Some(b)) = given {
            return Ok((a, b));
        }
        let (a, b) = fit_range(vals).ok_or("the curve has no finite values in this range")?;
        let (lo, hi) = nice_range(a, b, log);
        Ok::<_, String>((given.0.unwrap_or(lo), given.1.unwrap_or(hi)))
    };
    let (x0, x1) = auto((gx0, gx1), &mut xs, lx)?;
    let (y0, y1) = auto((gy0, gy1), &mut ys, ly)?;
    if !ordered(x0, x1) || !ordered(y0, y1) {
        return Err("a range must have min below max".into());
    }
    let ranges = Ranges { x0, x1, y0, y1 };

    // Adaptive refinement of a sampled curve, then the pieces.
    let mut all: Vec<(f64, Option<P>)> = Vec::with_capacity(pts.len());
    if let Some(f) = &curve {
        let mut r = Refiner {
            f: &**f,
            sx: x1 - x0,
            sy: y1 - y0,
            budget: MAX_POINTS.saturating_sub(pts.len()) / 2,
        };
        for w in pts.windows(2) {
            if let [(ua, pa), (ub, pb)] = *w {
                all.push((ua, pa));
                r.refine(ua, pa, ub, pb, 10, &mut all);
            }
        }
        if let Some(last) = pts.last() {
            all.push(*last);
        }
    } else {
        all = pts;
    }

    let mut pieces: Vec<Vec<P>> = Vec::new();
    let mut cur: Vec<P> = Vec::new();
    let mut prev: Option<P> = None;
    for (_, p) in all {
        match p {
            None => {
                pieces.push(std::mem::take(&mut cur));
                prev = None;
            }
            Some(p) => {
                if let Some(q) = prev {
                    // No line across a pole: the two ends are beyond the
                    // range on opposite sides.
                    let above = |p: P| p.1 > y1;
                    let below = |p: P| p.1 < y0;
                    if (above(p) && below(q)) || (below(p) && above(q)) {
                        pieces.push(std::mem::take(&mut cur));
                    }
                }
                cur.push(p);
                prev = Some(p);
            }
        }
    }
    pieces.push(cur);
    pieces.retain(|p| !p.is_empty());
    Ok((pieces, ranges))
}

/// The part of the segment a-b inside the rectangle (Liang-Barsky).
fn clip(a: P, b: P, r: &Ranges) -> Option<(P, P)> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [(-dx, a.0 - r.x0), (dx, r.x1 - a.0), (-dy, a.1 - r.y0), (dy, r.y1 - a.1)] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                if t > t1 {
                    return None;
                }
                t0 = t0.max(t);
            } else {
                if t < t0 {
                    return None;
                }
                t1 = t1.min(t);
            }
        }
    }
    // Ends that were not cut are kept exactly, so pieces stay joined.
    let c = if t0 > 0.0 { (a.0 + t0 * dx, a.1 + t0 * dy) } else { a };
    let d = if t1 < 1.0 { (a.0 + t1 * dx, a.1 + t1 * dy) } else { b };
    Some((c, d))
}

/// Cut the pieces to the plot's rectangle: runs of points inside it.
fn clip_pieces(pieces: &[Vec<P>], r: &Ranges) -> Vec<Vec<P>> {
    let mut out: Vec<Vec<P>> = Vec::new();
    for piece in pieces {
        let mut cur: Vec<P> = Vec::new();
        if let [only] = piece.as_slice() {
            if only.0 >= r.x0 && only.0 <= r.x1 && only.1 >= r.y0 && only.1 <= r.y1 {
                out.push(vec![*only, *only]);
            }
            continue;
        }
        for w in piece.windows(2) {
            let [a, b] = *w else { continue };
            match clip(a, b, r) {
                Some((c, d)) => {
                    // A new run unless this segment starts where the last ended.
                    if cur.last() != Some(&c) {
                        if cur.len() > 1 {
                            out.push(std::mem::take(&mut cur));
                        } else {
                            cur.clear();
                        }
                        cur.push(c);
                    }
                    cur.push(d);
                }
                None => {
                    if cur.len() > 1 {
                        out.push(std::mem::take(&mut cur));
                    } else {
                        cur.clear();
                    }
                }
            }
        }
        if cur.len() > 1 {
            out.push(cur);
        }
    }
    out
}

// ---- ticks and numbers -----------------------------------------------------

/// A number for a tick label.
pub fn format_tick(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return "0".into();
    }
    let a = v.abs();
    if !(1e-4..1e6).contains(&a) {
        return format!("{v:e}");
    }
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

/// A tick: the position in plot coordinates, its number (major ticks only).
struct Tick {
    at: f64,
    label: Option<String>,
}

fn ticks(lo: f64, hi: f64, log: bool) -> Vec<Tick> {
    let mut out = Vec::new();
    if log {
        let (a, b) = (lo.floor() as i64, hi.ceil() as i64);
        let step = ((b - a) / 10 + 1).max(1);
        let mut e = a;
        let mut n = 0;
        while e <= b && n < MAX_TICKS {
            let v = e as f64;
            if v >= lo - 1e-9 && v <= hi + 1e-9 {
                out.push(Tick {
                    at: v,
                    label: Some(format_tick(10f64.powf(v))),
                });
            }
            if step == 1 {
                for m in 2..10 {
                    let w = v + (m as f64).log10();
                    if w >= lo && w <= hi {
                        out.push(Tick { at: w, label: None });
                    }
                }
            }
            e += step;
            n += 1;
        }
    } else {
        let step = nice_step(hi - lo, 6.0);
        let (k0, k1) = ((lo / step - 1e-9).ceil(), (hi / step + 1e-9).floor());
        if !(k0.is_finite() && k1.is_finite()) || k1 - k0 > MAX_TICKS as f64 {
            return out;
        }
        let mut k = k0 as i64;
        while k <= k1 as i64 {
            let v = k as f64 * step;
            out.push(Tick {
                at: v,
                label: Some(format_tick(v)),
            });
            k += 1;
        }
    }
    out
}

// ---- the compound ------------------------------------------------------------

/// The look of the plot: the indicator's line style for the curve and its
/// text settings for numbers and labels.
#[derive(Debug, Clone)]
pub struct PlotLook {
    pub curve: Style,
    pub text_color: ColorRef,
    pub font: i32,
    pub font_size: f64,
    pub postscript: bool,
    /// Dot gap of the grid in 1/80 inch.
    pub grid_gap: f64,
}

/// A plot ready to add.
pub struct Plot {
    pub object: Object,
    pub curves: usize,
    pub points: usize,
}

fn fig(v: f64) -> f64 {
    v.round()
}

/// Build the plot compound for `spec` in the frame `rect` (Fig units).
/// `data` is the text of the data file in data mode.
pub fn build(spec: &PlotSpec, rect: Rect, data: Option<&str>, look: &PlotLook) -> Result<Plot, String> {
    spec.check()?;
    let (w, h) = (rect.max.x - rect.min.x, rect.max.y - rect.min.y);
    if !(w.is_finite() && h.is_finite() && w >= 10.0 && h >= 10.0 && w <= 1e7 && h <= 1e7) {
        return Err("the plot's frame is too small or too large".into());
    }
    let (pieces, r) = sample(spec, data)?;
    let pieces = clip_pieces(&pieces, &r);
    let mx = |x: f64| rect.min.x + (x - r.x0) / (r.x1 - r.x0) * w;
    let my = |y: f64| rect.max.y - (y - r.y0) / (r.y1 - r.y0) * h;

    let thin = |line: LineKind, depth: i32| Style {
        line,
        thickness: 1,
        pen_color: look.curve.pen_color,
        fill_color: look.curve.fill_color,
        depth: depth.clamp(0, MAX_DEPTH),
        pen_style: -1,
        area_fill: -1,
        style_val: if line == LineKind::Dotted { look.grid_gap } else { 0.0 },
        cap: look.curve.cap,
        join: look.curve.join,
    };
    let line = |style: Style, pts: Vec<Point>| {
        Object::new(ObjectKind::Polyline(Polyline::new(PolyKind::Polyline, style, pts)))
    };
    let depth = look.curve.depth;
    let mut members: Vec<Object> = Vec::new();

    if spec.axes {
        let a = Point::new(fig(rect.min.x), fig(rect.min.y));
        let b = Point::new(fig(rect.max.x), fig(rect.max.y));
        let mut frame = thin(LineKind::Solid, depth);
        frame.thickness = look.curve.thickness.clamp(1, 3);
        members.push(Object::new(ObjectKind::Polyline(Polyline::rect(frame, a, b))));
    }

    let size = look.font_size.clamp(6.0, 72.0);
    let tick_size = (size * 0.8).max(6.0);
    let unit = 1200.0 / 72.0;
    let text = |s: &str, justify: Justify, origin: Point, angle: f64, size: f64, special: bool| {
        let flags = if special {
            text_flags::SPECIAL
        } else if look.postscript {
            text_flags::POSTSCRIPT
        } else {
            0
        };
        Object::new(ObjectKind::Text(Text {
            justify,
            color: look.text_color,
            depth: depth.clamp(0, MAX_DEPTH),
            pen_style: -1,
            font: if special { 0 } else { look.font },
            size,
            angle,
            flags,
            height: 0.0,
            length: 0.0,
            origin,
            text: s.to_string(),
            local_font: None,
        }))
    };

    let mut left_width = 0.0f64;
    if spec.axes {
        let tick_len = 105.0; // about 0.09 inch
        let tick_h = tick_size * unit * 0.75;
        let gap = 0.5 * tick_h;
        for t in ticks(r.x0, r.x1, spec.log_x) {
            let x = fig(mx(t.at));
            let len = if t.label.is_some() { tick_len } else { 0.5 * tick_len };
            if spec.grid && t.label.is_some() {
                let g = thin(LineKind::Dotted, depth + 1);
                members.push(line(
                    g,
                    vec![Point::new(x, fig(rect.min.y)), Point::new(x, fig(rect.max.y))],
                ));
            }
            members.push(line(
                thin(LineKind::Solid, depth),
                vec![Point::new(x, fig(rect.max.y)), Point::new(x, fig(rect.max.y - len))],
            ));
            if let Some(l) = &t.label {
                members.push(text(
                    l,
                    Justify::Center,
                    Point::new(x, fig(rect.max.y + gap + tick_h)),
                    0.0,
                    tick_size,
                    false,
                ));
            }
        }
        for t in ticks(r.y0, r.y1, spec.log_y) {
            let y = fig(my(t.at));
            let len = if t.label.is_some() { tick_len } else { 0.5 * tick_len };
            if spec.grid && t.label.is_some() {
                let g = thin(LineKind::Dotted, depth + 1);
                members.push(line(
                    g,
                    vec![Point::new(fig(rect.min.x), y), Point::new(fig(rect.max.x), y)],
                ));
            }
            members.push(line(
                thin(LineKind::Solid, depth),
                vec![Point::new(fig(rect.min.x), y), Point::new(fig(rect.min.x + len), y)],
            ));
            if let Some(l) = &t.label {
                left_width = left_width.max(l.chars().count() as f64 * tick_size * unit * 0.55);
                members.push(text(
                    l,
                    Justify::Right,
                    Point::new(fig(rect.min.x - gap), fig(y + 0.35 * tick_h)),
                    0.0,
                    tick_size,
                    false,
                ));
            }
        }
        let label_size = size;
        let label_h = label_size * unit * 0.75;
        if !spec.xlabel.trim().is_empty() {
            let y = rect.max.y + gap + tick_h + gap + label_h * 1.1;
            members.push(text(
                &spec.xlabel,
                Justify::Center,
                Point::new(fig(0.5 * (rect.min.x + rect.max.x)), fig(y)),
                0.0,
                label_size,
                spec.latex,
            ));
        }
        if !spec.ylabel.trim().is_empty() {
            // Reads upwards; the baseline lies right of the glyphs.
            let x = rect.min.x - gap - left_width - gap;
            members.push(text(
                &spec.ylabel,
                Justify::Center,
                Point::new(fig(x), fig(0.5 * (rect.min.y + rect.max.y))),
                std::f64::consts::FRAC_PI_2,
                label_size,
                spec.latex,
            ));
        }
    }

    let mut count = 0;
    let mut total = 0;
    for piece in &pieces {
        let mut pts: Vec<Point> = Vec::with_capacity(piece.len());
        for &(x, y) in piece {
            let p = Point::new(fig(mx(x)), fig(my(y)));
            if pts.last() != Some(&p) {
                pts.push(p);
            }
        }
        if pts.len() == 1 {
            // A single dot: a zero-length line draws it.
            pts.push(pts[0]);
        }
        if pts.len() >= 2 {
            total += pts.len();
            count += 1;
            members.push(line(look.curve, pts));
        }
    }
    if count == 0 {
        return Err("nothing of the curve lies inside the ranges".into());
    }

    let mut object = Object::new(ObjectKind::Compound(Compound { objects: members }));
    let mut keep = spec.clone();
    keep.rect = Some([rect.min.x, rect.min.y, rect.max.x, rect.max.y]);
    object.comments = vec![keep.comment()?];
    Ok(Plot {
        object,
        curves: count,
        points: total,
    })
}

/// The frame of an existing plot: its first member when that is a box (the
/// frame, wherever the user moved or scaled it), else the stored rectangle.
pub fn frame_of(o: &Object, spec: &PlotSpec) -> Option<Rect> {
    if let ObjectKind::Compound(c) = &o.kind {
        if let Some(Object {
            kind: ObjectKind::Polyline(p),
            ..
        }) = c.objects.first()
        {
            if p.kind == PolyKind::Box && !p.points.is_empty() {
                let mut r = Rect::new(p.points[0], p.points[0]);
                for q in &p.points {
                    r = Rect::new(
                        Point::new(r.min.x.min(q.x), r.min.y.min(q.y)),
                        Point::new(r.max.x.max(q.x), r.max.y.max(q.y)),
                    );
                }
                return Some(r);
            }
        }
    }
    spec.rect
        .map(|[a, b, c, d]| Rect::new(Point::new(a, b), Point::new(c, d)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(src: &str) -> f64 {
        parse(src, &["x"]).unwrap().eval(&[2.0])
    }

    #[test]
    fn precedence_and_associativity() {
        assert_eq!(ev("1+2*3"), 7.0);
        assert_eq!(ev("(1+2)*3"), 9.0);
        assert_eq!(ev("2^3^2"), 512.0);
        assert_eq!(ev("-x^2"), -4.0);
        assert_eq!(ev("2^-1"), 0.5);
        assert_eq!(ev("10-4-3"), 3.0);
        assert_eq!(ev("8/4/2"), 1.0);
        assert_eq!(ev("2 \u{d7} 3 \u{f7} 4 \u{2212} 1"), 0.5);
        assert_eq!(ev("2**3"), 8.0);
        assert_eq!(ev("--x"), 2.0);
    }

    #[test]
    fn functions_and_constants() {
        assert!((ev("sin(pi/2)") - 1.0).abs() < 1e-12);
        assert!((ev("ln(e)") - 1.0).abs() < 1e-12);
        assert_eq!(ev("sqrt(16)+abs(-3)"), 7.0);
        assert_eq!(ev("max(1,x)+min(5,x)"), 4.0);
        assert!((ev("exp(0)*cos(0)") - 1.0).abs() < 1e-12);
        assert_eq!(ev("1.5e1"), 15.0);
        assert_eq!(ev(".5+2e0"), 2.5);
        // No implicit multiplication.
        assert!(parse("2e", &["x"]).is_err());
        assert!(ev("1/0").is_infinite());
        assert!(ev("sqrt(-1)").is_nan());
    }

    #[test]
    fn errors() {
        for bad in [
            "", "1+", "(1", "1)", "foo(1)", "y", "sin", "sin()", "1 2", "1,2", "max(1)", "$", "1..2.", "()", "*2",
        ] {
            assert!(parse(bad, &["x"]).is_err(), "{bad:?} should fail");
        }
        assert!(parse(&"(".repeat(200), &["x"]).is_err());
        assert!(parse(&"-".repeat(300), &["x"]).is_err());
        assert!(parse(&"1+".repeat(2_000), &["x"]).is_err());
        assert!(parse(&"x".repeat(MAX_EXPR_LEN + 1), &["x"]).is_err());
        assert!(parse("1e999", &["x"]).is_err());
    }

    fn spec(expr: &str, a: f64, b: f64) -> PlotSpec {
        PlotSpec {
            expr: expr.into(),
            xmin: Some(a),
            xmax: Some(b),
            ..PlotSpec::default()
        }
    }

    #[test]
    fn sin_samples() {
        let (pieces, r) = sample(&spec("sin(x)", 0.0, std::f64::consts::TAU), None).unwrap();
        assert_eq!(pieces.len(), 1);
        let p = &pieces[0];
        assert!(p.len() >= 401);
        assert!((p[0].0).abs() < 1e-12 && p[0].1.abs() < 1e-12);
        assert!((p[p.len() - 1].0 - std::f64::consts::TAU).abs() < 1e-12);
        assert!(r.y0 <= -1.0 && r.y1 >= 1.0 && r.y1 < 2.0);
    }

    #[test]
    fn pole_splits_the_curve() {
        let mut s = spec("1/x", -2.0, 2.0);
        s.ymin = Some(-5.0);
        s.ymax = Some(5.0);
        let (pieces, r) = sample(&s, None).unwrap();
        assert!(pieces.len() >= 2, "{} pieces", pieces.len());
        // No piece crosses x = 0.
        for p in &pieces {
            assert!(p.iter().all(|q| q.0 < 0.0) || p.iter().all(|q| q.0 > 0.0));
        }
        let clipped = clip_pieces(&pieces, &r);
        assert!(clipped.len() >= 2);
        // Auto range keeps the bulk, not 1e5.
        let (_, r) = sample(&spec("1/x", -2.0, 2.0), None).unwrap();
        assert!(r.y1 < 100.0 && r.y0 > -100.0);
        // tan too.
        let (pieces, _) = sample(&spec("tan(x)", -4.0, 4.0), None).unwrap();
        assert!(pieces.len() >= 3);
    }

    #[test]
    fn log_axes() {
        let mut s = spec("x^2", 1.0, 100.0);
        s.log_x = true;
        s.log_y = true;
        let (pieces, r) = sample(&s, None).unwrap();
        assert_eq!(pieces.len(), 1);
        assert!((r.x0 - 0.0).abs() < 1e-9 && (r.x1 - 2.0).abs() < 1e-9);
        assert!(spec_err(PlotSpec {
            log_x: true,
            ..spec("x", -1.0, 1.0)
        }));
        let t = ticks(0.0, 2.0, true);
        assert_eq!(t.iter().filter(|t| t.label.is_some()).count(), 3);
    }

    fn spec_err(s: PlotSpec) -> bool {
        sample(&s, None).is_err()
    }

    #[test]
    fn csv_columns() {
        let text = "x,a,b\n# comment\n1,10,100\n2,20,200\n3;30;300\n\nbad,row,here\n4 40 400\n";
        assert_eq!(
            parse_table(text, 1, 3).unwrap(),
            vec![(1.0, 100.0), (2.0, 200.0), (3.0, 300.0), (4.0, 400.0)]
        );
        assert_eq!(parse_table(text, 0, 2).unwrap()[1], (2.0, 20.0));
        assert!(parse_table(text, 1, 9).is_err());
        assert!(parse_table("", 1, 2).is_err());
        assert!(parse_table("1,nan\n2,inf\n", 1, 2).is_err());
    }

    #[test]
    fn data_mode_draws() {
        let s = PlotSpec {
            mode: PlotMode::Data,
            xmin: None,
            xmax: None,
            ..PlotSpec::default()
        };
        let (pieces, r) = sample(&s, Some("0 0\n1 1\n2 4\n3 9\n")).unwrap();
        assert_eq!(pieces[0].len(), 4);
        assert!(r.x0 <= 0.0 && r.x1 >= 3.0 && r.y1 >= 9.0);
        assert!(sample(&s, None).is_err());
    }

    #[test]
    fn comment_roundtrip() {
        let mut s = spec("sin(3*x)", -1.0, 1.0);
        s.xlabel = "time".into();
        s.grid = true;
        s.xmax = None;
        let c = s.comment().unwrap();
        assert!(c.starts_with(PLOT_COMMENT));
        let back = PlotSpec::from_comment(&c).unwrap();
        assert_eq!(back.expr, "sin(3*x)");
        assert_eq!(back.xmin, Some(-1.0));
        assert_eq!(back.xmax, None);
        assert!(back.grid);
        assert_eq!(back.xlabel, "time");
        assert!(PlotSpec::from_comment("hello").is_none());
        assert!(PlotSpec::from_comment("rustfig-plot: {").is_none());
    }

    #[test]
    fn nice_numbers() {
        assert_eq!(format_tick(0.5), "0.5");
        assert_eq!(format_tick(-0.0), "0");
        assert_eq!(format_tick(1e-5), "1e-5");
        assert_eq!(format_tick(100.0), "100");
        assert_eq!(nice_range(0.0, 0.93, false), (0.0, 1.0));
        let t = ticks(0.0, 6.5, false);
        assert!(t.len() >= 5 && t.len() <= 12);
    }

    #[test]
    fn builds_a_compound() {
        let look = PlotLook {
            curve: Style::default(),
            text_color: ColorRef::BLACK,
            font: 0,
            font_size: 12.0,
            postscript: true,
            grid_gap: 3.0,
        };
        let mut s = spec("sin(x)", 0.0, std::f64::consts::TAU);
        s.xlabel = "$x$".into();
        s.ylabel = "y".into();
        s.latex = true;
        s.grid = true;
        let r = Rect::new(Point::new(0.0, 0.0), Point::new(4800.0, 3000.0));
        let p = build(&s, r, None, &look).unwrap();
        let ObjectKind::Compound(c) = &p.object.kind else {
            panic!()
        };
        assert!(c.objects.len() > 10);
        assert!(p.points > 400);
        assert_eq!(frame_of(&p.object, &s), Some(r));
        let special = c
            .objects
            .iter()
            .filter(|o| matches!(&o.kind, ObjectKind::Text(t) if t.is_special()))
            .count();
        assert_eq!(special, 2);
        assert!(PlotSpec::of_object(&p.object).is_some());
        // A frame too small is an error.
        assert!(build(&s, Rect::new(Point::ZERO, Point::new(1.0, 1.0)), None, &look).is_err());
    }
}
