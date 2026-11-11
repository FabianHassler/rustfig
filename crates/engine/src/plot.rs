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
    text_flags, Arrow, ColorRef, Compound, Ellipse, EllipseKind, Justify, LineKind, Object, ObjectKind, PolyKind,
    Polyline, Style, Text, MAX_DEPTH,
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
/// The comment lines that keep a curve's data points: prefix, the curve's
/// index, then x y pairs.
pub const POINTS_COMMENT: &str = "rustfig-plot-points:";
/// Most data points one plot keeps in the figure (more: a data file).
pub const MAX_KEPT_POINTS: usize = 20_000;
/// Most curves in one plot.
pub const MAX_CURVES: usize = 50;
/// Most markers in one plot.
const MAX_MARKERS: usize = 20_000;
/// Most bars, stems or error bars of one curve.
const MAX_BARS: usize = 5_000;
/// Most cells per axis of an implicit curve.
pub const MAX_IMPLICIT_GRID: usize = 300;
/// Most start points of field lines.
pub const MAX_STARTS: usize = 60;
/// Most steps of one field line direction.
pub const MAX_FIELD_STEPS: usize = 5_000;
/// Most steps of all the field lines of one curve.
const MAX_FIELD_WORK: usize = 10_000;
/// The comment lines that keep a curve's error values: prefix, the curve's
/// index, then one value per point.
pub const YERR_COMMENT: &str = "rustfig-plot-yerr:";

// ---- expressions ---------------------------------------------------------

use crate::special_fns as sf;

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
    nodes: usize,
}

impl Formula {
    /// The value for the variables (in the order given to [`parse`]); may be
    /// NaN or infinite.
    pub fn eval(&self, vars: &[f64]) -> f64 {
        eval(&self.root, vars)
    }

    /// How many nodes the expression has (its cost to evaluate).
    pub fn nodes(&self) -> usize {
        self.nodes
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
    ("j0", sf::j0),
    ("j1", sf::j1),
    ("y0", sf::y0),
    ("y1", sf::y1),
    ("i0", sf::i0),
    ("i1", sf::i1),
    ("k0", sf::k0),
    ("k1", sf::k1),
    ("gamma", sf::gamma),
    ("lgamma", sf::lgamma),
    ("erf", sf::erf),
    ("erfc", sf::erfc),
    ("heaviside", sf::heaviside),
];

const FUNCS2: &[(&str, F2)] = &[
    ("atan2", f64::atan2),
    ("min", f64::min),
    ("max", f64::max),
    ("pow", f64::powf),
    ("jn", sf::bessel_jn),
    ("yn", sf::bessel_yn),
    ("in", sf::bessel_in),
    ("kn", sf::bessel_kn),
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
    Ok(Formula { root, nodes: p.nodes })
}

/// Parse an equation `lhs = rhs` as the formula lhs - rhs (without an `=`:
/// the formula itself, which is then compared with 0).
pub fn parse_equation(src: &str, vars: &[&str]) -> Result<Formula, String> {
    let Some((l, r)) = src.split_once('=') else {
        return parse(src, vars);
    };
    if r.contains('=') {
        return Err("an equation has one \"=\"".into());
    }
    let (a, b) = (parse(l, vars)?, parse(r, vars)?);
    let nodes = a.nodes + b.nodes + 1;
    if nodes > 2 * MAX_NODES {
        return Err("the expression is too long".into());
    }
    Ok(Formula {
        root: Expr::Bin('-', Box::new(a.root), Box::new(b.root)),
        nodes,
    })
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
    /// r(t) with the angle t (`expr`, variable `t` or `theta`).
    Polar,
    /// f(x, y) = 0 (`expr`: a formula, or an equation `lhs = rhs`), by
    /// marching squares over the window.
    Implicit,
    /// The derivative of another curve (`of`, a curve before this one).
    Derivative,
    /// The integral of another curve, from its first point (`of`).
    Integral,
    /// Field lines of the vector field (`xt`, `yt`) in x and y from the
    /// start points `starts`.
    Field,
}

/// Where the ticks stand relative to the axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TickDir {
    In,
    Out,
    Both,
}

/// Where the axes stand (Q1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxesKind {
    /// Ticks and numbers on the frame's edges (the box is `frame`).
    #[default]
    Frame,
    /// Two axes crossing at (0, 0), or at the range's edge when 0 is
    /// outside it, with arrows and the axis labels at the tips; ticks and
    /// numbers on one side of each axis.
    Origin,
    /// No box, axes, ticks or numbers: the curves (and the grid) alone.
    None,
}

/// The `axes` value of a plot: a kind, or the old plots' on/off switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AxesValue {
    Switch(bool),
    Kind(AxesKind),
}

/// How tick numbers are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumFormat {
    Auto,
    /// A fixed number of decimals (`decimals`).
    Fixed,
    Scientific,
    /// Multiples of pi: 0, pi/2, pi, 3pi/2.
    Pi,
}

/// The ticks of one axis. The default is the automatic ticks of plots made
/// before this setting existed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TickSpec {
    /// Manual ticks: `first`, `step` or `count` apply (else automatic).
    pub manual: bool,
    /// Position of the first major tick (default: the first multiple of the step in range).
    pub first: Option<f64>,
    /// Distance of major ticks (log axes: decades).
    pub step: Option<f64>,
    /// Number of intervals, instead of a step.
    pub count: Option<usize>,
    /// Minor ticks between two majors, 0-9; `None` is automatic (none on a
    /// linear axis, 2-9 on a log axis with a decade step). On a log axis
    /// n gives the minors at 2..n+1 times the decade.
    pub minor: Option<u8>,
    /// Length of a major tick in Fig units (minor ticks are half).
    pub length: f64,
    pub dir: TickDir,
    pub format: NumFormat,
    pub decimals: u8,
    /// Tick marks on this axis.
    pub ticks: bool,
    /// Numbers at the major ticks.
    pub numbers: bool,
    /// Scale factor (Q2): the numbers are divided by 10^scale and a
    /// "x10^scale" stands at the axis's end (linear axes only).
    pub scale: Option<i32>,
    /// A decimal comma instead of the point.
    pub comma: bool,
    /// Own labels `[[value, "text"]]` (also categories): ticks stand at
    /// these values (inside the range) with these texts, instead of the
    /// automatic or manual ones.
    pub labels: Vec<(f64, String)>,
    /// Angle of the numbers in degrees, counterclockwise (-180 to 180).
    pub rotate: f64,
}

impl Default for TickSpec {
    fn default() -> Self {
        TickSpec {
            manual: false,
            first: None,
            step: None,
            count: None,
            minor: None,
            length: 105.0,
            dir: TickDir::In,
            format: NumFormat::Auto,
            decimals: 2,
            ticks: true,
            numbers: true,
            scale: None,
            comma: false,
            labels: Vec::new(),
            rotate: 0.0,
        }
    }
}

impl TickSpec {
    fn check(&self, axis: &str) -> Result<(), String> {
        if let Some(v) = self.first {
            if !v.is_finite() || v.abs() > BIG {
                return Err(format!("{axis} ticks: the first tick must be a finite number"));
            }
        }
        if let Some(v) = self.step {
            if !v.is_finite() || v <= 0.0 || v > BIG {
                return Err(format!("{axis} ticks: the step must be a positive number"));
            }
        }
        if let Some(n) = self.count {
            if !(1..=MAX_TICKS as usize).contains(&n) {
                return Err(format!("{axis} ticks: the count must be between 1 and {MAX_TICKS}"));
            }
        }
        if self.minor.is_some_and(|m| m > 9) {
            return Err(format!("{axis} ticks: minor ticks per major are 0 to 9"));
        }
        if !self.length.is_finite() || !(0.0..=3000.0).contains(&self.length) {
            return Err(format!("{axis} ticks: the length must be between 0 and 3000"));
        }
        if self.decimals > 9 {
            return Err(format!("{axis} ticks: at most 9 decimals"));
        }
        if self.scale.is_some_and(|e| !(-30..=30).contains(&e)) {
            return Err(format!("{axis} ticks: the scale is a power of ten from -30 to 30"));
        }
        if !self.rotate.is_finite() || !(-180.0..=180.0).contains(&self.rotate) {
            return Err(format!("{axis} ticks: the rotation is between -180 and 180 degrees"));
        }
        if self.labels.len() > MAX_TICKS as usize {
            return Err(format!("{axis} ticks: at most {MAX_TICKS} own labels"));
        }
        for (v, t) in &self.labels {
            if !v.is_finite() || v.abs() > BIG {
                return Err(format!(
                    "{axis} ticks: the positions of own labels must be finite numbers"
                ));
            }
            if t.chars().count() > MAX_TEXT {
                return Err(format!("{axis} ticks: an own label is too long"));
            }
        }
        Ok(())
    }
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
    /// The box around the plot.
    pub frame: bool,
    /// Where the axes stand (`frame`, `origin`, `none`; `None` is `frame`).
    /// Old plots' all-in-one switch: `false` turns the frame, ticks and
    /// numbers off when read (`from_value`), then it is cleared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub axes: Option<AxesValue>,
    /// Arrowheads at the tips of origin axes.
    pub axis_arrows: bool,
    pub grid: bool,
    /// Grid lines at the minor ticks too (half as dense a dotted line).
    pub grid_minor: bool,
    /// Line style of the major grid lines (`dotted`, `dashed`, `solid`...;
    /// `None`: dotted).
    pub grid_style: Option<String>,
    pub log_x: bool,
    pub log_y: bool,
    pub xlabel: String,
    pub ylabel: String,
    /// Axis labels are LaTeX (special text).
    pub latex: bool,
    /// The tick numbers are LaTeX too: special texts in `$...$`.
    pub latex_numbers: bool,
    pub xticks: TickSpec,
    pub yticks: TickSpec,
    /// The plot's frame in Fig units (min x, min y, max x, max y); written
    /// by the command, used by "Edit plot" when the frame is missing.
    pub rect: Option<[f64; 4]>,
    /// Several curves, each with its own source and style; empty: the one
    /// curve the fields above describe.
    pub curves: Vec<CurveSpec>,
    /// Depth of every part (`None`: the indicator's).
    pub depth: Option<i32>,
    /// The curves' style where a curve sets none (`None`: the indicator's).
    pub pen_color: Option<i32>,
    pub thickness: Option<i32>,
    pub line: Option<String>,
    pub style_val: Option<f64>,
}

/// The mark drawn at each point of a curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Marker {
    #[default]
    None,
    Circle,
    Square,
    Triangle,
    Diamond,
    Cross,
    Plus,
}

/// How a curve's points are drawn (Q4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveStyle {
    /// A line through the points.
    #[default]
    Line,
    /// A bar from the baseline to each point; the bars of several bar
    /// curves stand side by side (grouped).
    Bar,
    /// A vertical line from the baseline to each point (a Dirac plot).
    Stem,
}

/// A filled area behind a curve (Q3): between the curve and a horizontal
/// baseline, or between the curve and another curve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CurveFill {
    /// The number (from 1) of the curve it is filled up to; `None`: the
    /// baseline `y`.
    pub curve: Option<usize>,
    /// The baseline's y value (kept inside the window).
    pub y: f64,
    /// Fill color (`None`: the curve's pen color).
    pub color: Option<i32>,
    /// Xfig's area fill: 0 black ... 20 the full color ... 40 white.
    pub density: i32,
}

impl Default for CurveFill {
    fn default() -> Self {
        CurveFill {
            curve: None,
            y: 0.0,
            color: None,
            density: 20,
        }
    }
}

/// One curve of a plot with several ([`PlotSpec::curves`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CurveSpec {
    /// `None`: data when it has points, data or a file, parametric with
    /// `xt`/`yt`, else a function.
    pub mode: Option<PlotMode>,
    pub expr: String,
    pub xt: String,
    pub yt: String,
    pub tmin: Option<f64>,
    pub tmax: Option<f64>,
    pub file: String,
    pub xcol: usize,
    pub ycol: usize,
    /// The text of a data file given inline; drawing turns it into `points`.
    #[serde(skip_serializing)]
    pub data: String,
    /// Data points, kept in the figure ([`POINTS_COMMENT`] lines).
    pub points: Vec<[f64; 2]>,
    /// Samples of a function (`None`: the plot's); markers stand at them.
    pub samples: Option<usize>,
    pub pen_color: Option<i32>,
    pub thickness: Option<i32>,
    /// A line style name, or `"none"` for markers alone.
    pub line: Option<String>,
    pub style_val: Option<f64>,
    pub marker: Marker,
    /// Width of a marker in Fig units.
    pub marker_size: f64,
    /// Filled with the curve's color (else open).
    pub marker_fill: bool,
    /// An area filled behind the curve: to a baseline or to another curve.
    pub fill: Option<CurveFill>,
    /// Line, bars or stems.
    pub style: CurveStyle,
    /// Width of a bar as a fraction (0.01 to 1) of the distance between the
    /// bars' x values; grouped bars share it.
    pub bar_width: f64,
    /// Where bars and stems start (y value).
    pub baseline: f64,
    /// Error values (y plus and minus) of the data points, one each or one
    /// for all; kept in the figure like `points`.
    pub yerr: Vec<f64>,
    /// The column of a data file or text with the error values (0: none).
    pub yerr_col: usize,
    /// Width of the end caps of error bars in Fig units.
    pub err_cap: f64,
    /// Derivative and integral: the number (from 1) of the curve it is of.
    pub of: Option<usize>,
    /// Field lines start here.
    pub starts: Vec<[f64; 2]>,
    /// Steps of a field line in each direction (from its start point).
    pub field_steps: usize,
    /// Length of one step in data units (`None`: 1/100 of the window's
    /// diagonal).
    pub field_step: Option<f64>,
}

impl Default for CurveSpec {
    fn default() -> Self {
        CurveSpec {
            mode: None,
            expr: String::new(),
            xt: String::new(),
            yt: String::new(),
            tmin: None,
            tmax: None,
            file: String::new(),
            xcol: 1,
            ycol: 2,
            data: String::new(),
            points: Vec::new(),
            samples: None,
            pen_color: None,
            thickness: None,
            line: None,
            style_val: None,
            marker: Marker::None,
            marker_size: 75.0,
            marker_fill: true,
            fill: None,
            style: CurveStyle::Line,
            bar_width: 0.8,
            baseline: 0.0,
            yerr: Vec::new(),
            yerr_col: 0,
            err_cap: 60.0,
            of: None,
            starts: Vec::new(),
            field_steps: 400,
            field_step: None,
        }
    }
}

impl CurveSpec {
    pub fn mode(&self) -> PlotMode {
        match self.mode {
            Some(m) => m,
            None if !self.points.is_empty() || !self.data.is_empty() || !self.file.is_empty() => PlotMode::Data,
            None if !self.xt.is_empty() || !self.yt.is_empty() => PlotMode::Parametric,
            None => PlotMode::Function,
        }
    }

    /// Whether the curve draws its line (`line: "none"`: markers only).
    pub fn has_line(&self) -> bool {
        self.line.as_deref() != Some("none")
    }

    fn check(&self, n: usize) -> Result<(), String> {
        let at = |m: &str| format!("curve {n}: {m}");
        if let Some(name) = self.line.as_deref().filter(|l| *l != "none") {
            if LineKind::from_name(name).is_none() {
                return Err(at(&format!("unknown line style {name:?}")));
            }
        }
        if self
            .pen_color
            .is_some_and(|c| !(-1..=rustfig_model::LAST_USER_COLOR).contains(&c))
        {
            return Err(at("pen_color is not a color number"));
        }
        if self.thickness.is_some_and(|t| !(0..=10_000).contains(&t)) {
            return Err(at("thickness must be between 0 and 10000"));
        }
        if self
            .style_val
            .is_some_and(|v| !v.is_finite() || !(0.0..=1000.0).contains(&v))
        {
            return Err(at("style_val must be between 0 and 1000"));
        }
        if !self.marker_size.is_finite() || !(1.0..=10_000.0).contains(&self.marker_size) {
            return Err(at("marker_size must be between 1 and 10000"));
        }
        if self.samples.is_some_and(|n| !(2..=MAX_SAMPLES).contains(&n)) {
            return Err(at(&format!("samples must be between 2 and {MAX_SAMPLES}")));
        }
        let nums = [self.tmin, self.tmax];
        if nums.iter().flatten().any(|v| !v.is_finite() || v.abs() > BIG) {
            return Err(at("ranges must be finite numbers"));
        }
        for s in [&self.expr, &self.xt, &self.yt, &self.file] {
            if s.chars().count() > MAX_TEXT.max(MAX_EXPR_LEN) {
                return Err(at("a text of the curve is too long"));
            }
        }
        if self.xcol > 1_000 || self.ycol == 0 || self.ycol > 1_000 {
            return Err(at("columns are numbered from 1 (x column 0 means the row number)"));
        }
        if self.points.len() > MAX_KEPT_POINTS {
            return Err(at(&format!("at most {MAX_KEPT_POINTS} points (use a data file)")));
        }
        if self.points.iter().flatten().any(|v| !v.is_finite() || v.abs() > BIG) {
            return Err(at("points must be finite numbers"));
        }
        if !self.bar_width.is_finite() || !(0.01..=1.0).contains(&self.bar_width) {
            return Err(at("bar_width is a fraction between 0.01 and 1"));
        }
        if !self.baseline.is_finite() || self.baseline.abs() > BIG {
            return Err(at("the baseline must be a finite number"));
        }
        if !self.err_cap.is_finite() || !(0.0..=10_000.0).contains(&self.err_cap) {
            return Err(at("err_cap must be between 0 and 10000"));
        }
        if !(1..=MAX_FIELD_STEPS).contains(&self.field_steps) {
            return Err(at(&format!("field_steps must be between 1 and {MAX_FIELD_STEPS}")));
        }
        if self.field_step.is_some_and(|h| !h.is_finite() || h <= 0.0 || h > BIG) {
            return Err(at("field_step must be a positive number"));
        }
        if self.starts.len() > MAX_STARTS {
            return Err(at(&format!("at most {MAX_STARTS} start points")));
        }
        if self.starts.iter().flatten().any(|v| !v.is_finite() || v.abs() > BIG) {
            return Err(at("start points must be finite numbers"));
        }
        if self.of == Some(0) {
            return Err(at("\"of\" is the number of a curve, counted from 1"));
        }
        if self.yerr_col > 1_000 {
            return Err(at("yerr_col is a column number from 1 (0: none)"));
        }
        if self.yerr.len() > MAX_KEPT_POINTS || self.yerr.iter().any(|v| !v.is_finite() || v.abs() > BIG) {
            return Err(at(&format!("yerr must be at most {MAX_KEPT_POINTS} finite numbers")));
        }
        if let Some(f) = &self.fill {
            if !f.y.is_finite() || f.y.abs() > BIG {
                return Err(at("the fill's y must be a finite number"));
            }
            if !(0..=40).contains(&f.density) {
                return Err(at("the fill density is 0 (black) to 40 (white), 20 the full color"));
            }
            if f.color
                .is_some_and(|c| !(-1..=rustfig_model::LAST_USER_COLOR).contains(&c))
            {
                return Err(at("the fill color is not a color number"));
            }
        }
        Ok(())
    }
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
            frame: true,
            axes: None,
            axis_arrows: true,
            grid: false,
            grid_minor: false,
            grid_style: None,
            log_x: false,
            log_y: false,
            xlabel: String::new(),
            ylabel: String::new(),
            latex: false,
            latex_numbers: false,
            xticks: TickSpec::default(),
            yticks: TickSpec::default(),
            rect: None,
            curves: Vec::new(),
            depth: None,
            pen_color: None,
            thickness: None,
            line: None,
            style_val: None,
        }
    }
}

impl PlotSpec {
    /// A spec from JSON params (unknown keys are ignored), checked.
    pub fn from_value(v: &Value) -> Result<PlotSpec, String> {
        let spec: PlotSpec = serde_json::from_value(v.clone()).map_err(|e| format!("bad plot parameters: {e}"))?;
        spec.check()?;
        Ok(spec.migrate())
    }

    /// A function plot with an empty expression: only the frame, ticks and
    /// labels.
    pub fn has_no_curve(&self) -> bool {
        self.curves.is_empty() && self.mode == PlotMode::Function && self.expr.trim().is_empty()
    }

    /// The curves to draw: the list, or the one curve of the fields.
    pub fn curve_list(&self) -> Vec<CurveSpec> {
        if !self.curves.is_empty() {
            return self.curves.clone();
        }
        if self.has_no_curve() {
            return Vec::new();
        }
        vec![CurveSpec {
            mode: Some(self.mode),
            expr: self.expr.clone(),
            xt: self.xt.clone(),
            yt: self.yt.clone(),
            tmin: self.tmin,
            tmax: self.tmax,
            file: self.file.clone(),
            xcol: self.xcol,
            ycol: self.ycol,
            samples: Some(self.samples),
            ..CurveSpec::default()
        }]
    }

    /// Where the axes stand.
    pub fn axes_kind(&self) -> AxesKind {
        match self.axes {
            Some(AxesValue::Kind(k)) => k,
            _ => AxesKind::Frame,
        }
    }

    /// Old comments' `axes: false` means no frame, ticks or numbers.
    fn migrate(mut self) -> PlotSpec {
        match self.axes.take() {
            Some(AxesValue::Switch(false)) => {
                self.frame = false;
                for t in [&mut self.xticks, &mut self.yticks] {
                    t.ticks = false;
                    t.numbers = false;
                }
            }
            Some(AxesValue::Kind(k)) if k != AxesKind::Frame => self.axes = Some(AxesValue::Kind(k)),
            _ => {}
        }
        self
    }

    fn check(&self) -> Result<(), String> {
        self.xticks.check("x")?;
        self.yticks.check("y")?;
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
        if self.curves.len() > MAX_CURVES {
            return Err(format!("at most {MAX_CURVES} curves"));
        }
        for (i, c) in self.curves.iter().enumerate() {
            c.check(i + 1)?;
            if matches!(c.mode, Some(PlotMode::Derivative | PlotMode::Integral)) && !c.of.is_some_and(|k| k <= i) {
                return Err(format!(
                    "curve {}: a derivative or integral is of an earlier curve (\"of\": its number, from 1)",
                    i + 1
                ));
            }
            if let Some(k) = c.fill.as_ref().and_then(|f| f.curve) {
                if k == 0 || k > self.curves.len() || k == i + 1 {
                    return Err(format!(
                        "curve {}: the fill goes up to another curve, numbered from 1 to {}",
                        i + 1,
                        self.curves.len()
                    ));
                }
            }
        }
        if self.curves.iter().map(|c| c.yerr.len()).sum::<usize>() > MAX_KEPT_POINTS {
            return Err(format!(
                "at most {MAX_KEPT_POINTS} error values in one plot (use a data file)"
            ));
        }
        if self.curves.iter().map(|c| c.points.len()).sum::<usize>() > MAX_KEPT_POINTS {
            return Err(format!("at most {MAX_KEPT_POINTS} points in one plot (use data files)"));
        }
        // The plot's own style, as for a curve.
        CurveSpec {
            pen_color: self.pen_color,
            thickness: self.thickness,
            line: self.line.clone(),
            style_val: self.style_val,
            ..CurveSpec::default()
        }
        .check(0)
        .map_err(|e| e.replacen("curve 0", "plot", 1))?;
        if self.depth.is_some_and(|d| !(0..=MAX_DEPTH).contains(&d)) {
            return Err(format!("depth must be between 0 and {MAX_DEPTH}"));
        }
        if let Some(name) = &self.grid_style {
            if LineKind::from_name(name).is_none() {
                return Err(format!("unknown grid line style {name:?}"));
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
                PlotMode::Function | PlotMode::Implicit => &["xt", "yt", "file", "xcol", "ycol", "tmin", "tmax"],
                PlotMode::Polar => &["xt", "yt", "file", "xcol", "ycol"],
                PlotMode::Parametric => &["expr", "file", "xcol", "ycol"],
                PlotMode::Data => &["expr", "xt", "yt", "tmin", "tmax", "samples"],
                PlotMode::Derivative | PlotMode::Integral | PlotMode::Field => &["file", "xcol", "ycol"],
            };
            for k in drop {
                m.remove(*k);
            }
            // Ticks: what differs from the defaults of a tick setting.
            let dt = serde_json::to_value(TickSpec::default()).map_err(|e| e.to_string())?;
            for k in ["xticks", "yticks"] {
                if let (Some(Value::Object(t)), Some(dt)) = (m.get_mut(k), dt.as_object()) {
                    t.retain(|k, x| dt.get(k) != Some(x));
                }
            }
            // Curves: what differs from a curve's defaults; the points go
            // on lines of their own.
            let dc = serde_json::to_value(CurveSpec::default()).map_err(|e| e.to_string())?;
            if let (Some(Value::Array(list)), Some(dc)) = (m.get_mut("curves"), dc.as_object()) {
                for c in list.iter_mut().filter_map(Value::as_object_mut) {
                    c.retain(|k, x| k != "points" && k != "yerr" && dc.get(k) != Some(x));
                }
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

    /// The comment lines that keep this spec: [`Self::comment`], then the
    /// curves' points ([`POINTS_COMMENT`]).
    pub fn comments(&self) -> Result<Vec<String>, String> {
        let mut out = vec![self.comment()?];
        for (i, c) in self.curves.iter().enumerate() {
            let mut line = String::new();
            for p in &c.points {
                let pair = format!(" {} {}", short(p[0]), short(p[1]));
                if line.is_empty() || line.len() + pair.len() > MAX_COMMENT {
                    if !line.is_empty() {
                        out.push(std::mem::take(&mut line));
                    }
                    line = format!("{POINTS_COMMENT} {i}");
                }
                line.push_str(&pair);
            }
            if !line.is_empty() {
                out.push(line);
            }
            let mut line = String::new();
            for e in &c.yerr {
                let item = format!(" {}", short(*e));
                if line.is_empty() || line.len() + item.len() > MAX_COMMENT {
                    if !line.is_empty() {
                        out.push(std::mem::take(&mut line));
                    }
                    line = format!("{YERR_COMMENT} {i}");
                }
                line.push_str(&item);
            }
            if !line.is_empty() {
                out.push(line);
            }
        }
        Ok(out)
    }

    /// The spec of a plot compound (its comments), if it is one.
    pub fn of_object(o: &Object) -> Option<PlotSpec> {
        if !matches!(o.kind, ObjectKind::Compound(_)) {
            return None;
        }
        let mut spec = o.comments.iter().find_map(|c| PlotSpec::from_comment(c))?;
        let mut kept = 0;
        for line in &o.comments {
            let Some(rest) = line.trim().strip_prefix(POINTS_COMMENT) else {
                continue;
            };
            let mut words = rest.split_whitespace();
            let Some(curve) = words
                .next()
                .and_then(|w| w.parse::<usize>().ok())
                .and_then(|i| spec.curves.get_mut(i))
            else {
                continue;
            };
            let nums: Vec<f64> = words.map_while(|w| w.parse::<f64>().ok()).collect();
            for &[x, y] in nums.as_chunks::<2>().0 {
                if kept < MAX_KEPT_POINTS && x.is_finite() && y.is_finite() && x.abs() <= BIG && y.abs() <= BIG {
                    curve.points.push([x, y]);
                    kept += 1;
                }
            }
        }
        let mut kept_err = 0;
        for line in &o.comments {
            let Some(rest) = line.trim().strip_prefix(YERR_COMMENT) else {
                continue;
            };
            let mut words = rest.split_whitespace();
            let Some(curve) = words
                .next()
                .and_then(|w| w.parse::<usize>().ok())
                .and_then(|i| spec.curves.get_mut(i))
            else {
                continue;
            };
            for v in words.map_while(|w| w.parse::<f64>().ok()) {
                if kept_err < MAX_KEPT_POINTS && v.is_finite() && v.abs() <= BIG {
                    curve.yerr.push(v);
                    kept_err += 1;
                }
            }
        }
        Some(spec)
    }
}

// ---- data ----------------------------------------------------------------

/// The numbers of a data file: lines of values separated by commas,
/// semicolons, tabs or spaces; `#`/`%` lines and lines that are not numbers
/// (a header) are skipped. Returns the (x, y) of the chosen columns.
pub fn parse_table(text: &str, xcol: usize, ycol: usize) -> Result<Vec<(f64, f64)>, String> {
    Ok(parse_table_err(text, xcol, ycol, 0)?
        .into_iter()
        .map(|(x, y, _)| (x, y))
        .collect())
}

/// [`parse_table`] with a third column of error values (`ecol` 0: none, the
/// error is NaN); rows where that column is no number are skipped.
pub fn parse_table_err(text: &str, xcol: usize, ycol: usize, ecol: usize) -> Result<Vec<(f64, f64, f64)>, String> {
    let need = xcol.max(ycol).max(ecol);
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
        let e = if ecol == 0 { Some(f64::NAN) } else { num(ecol) };
        if let (Some(x), Some(y), Some(e)) = (x, y, e) {
            out.push((x, y, e));
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
    /// Normalizing spans of the two axes (curvature is judged on screen).
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

/// One curve's raw samples: (parameter, point) with the parameter ordering
/// them, and the function behind them (none for data).
struct Raw {
    pts: Vec<(f64, Option<P>)>,
    curve: Option<Box<dyn Fn(f64) -> Option<P>>>,
    /// Error values of data points, one per point (NaN: none); empty without.
    errs: Vec<f64>,
}

/// The points of a bar, stem or error bar curve: the point and its error.
type Series = Vec<(P, Option<f64>)>;

/// What [`sample`] gives: the curves as pieces of points in the plot's
/// coordinates (transformed by the axes' logarithms), the points each
/// curve's markers stand at, the ranges, which fit all the curves, the
/// points of curves drawn as bars, stems or with error bars, and the
/// distance between bars.
struct Sampled {
    pieces: Vec<Vec<Vec<P>>>,
    marks: Vec<Vec<P>>,
    ranges: Ranges,
    series: Vec<Series>,
    bar_spacing: f64,
}

/// The smallest distance between the x values of the bar curves (1 when
/// there is no distance).
fn bar_spacing(series: &[Series], curves: &[CurveSpec]) -> f64 {
    let mut xs: Vec<f64> = series
        .iter()
        .zip(curves)
        .filter(|(_, c)| c.style == CurveStyle::Bar)
        .flat_map(|(s, _)| s.iter().map(|(p, _)| p.0))
        .collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let scale = xs.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1e-300);
    let d = xs
        .windows(2)
        .map(|w| w[1] - w[0])
        .filter(|d| *d > 1e-12 * scale)
        .fold(f64::INFINITY, f64::min);
    if d.is_finite() && d > 0.0 {
        d
    } else {
        1.0
    }
}

fn sample(spec: &PlotSpec, curves: &[CurveSpec]) -> Result<Sampled, String> {
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
    // A function's x range when none is given.
    let (fx0, fx1) = (
        gx0.unwrap_or(if lx { -1.0 } else { -10.0 }),
        gx1.unwrap_or(if lx { 2.0 } else { 10.0 }),
    );

    // The window of implicit curves and field lines when no y range is given.
    let (wy0, wy1) = (
        gy0.unwrap_or(if ly { -1.0 } else { -10.0 }),
        gy1.unwrap_or(if ly { 2.0 } else { 10.0 }),
    );

    if curves.is_empty() {
        // The frame, ticks and labels without a curve.
        let (Some(y0), Some(y1)) = (gy0, gy1) else {
            return Err("a plot without a curve (empty \"expr\") needs \"ymin\" and \"ymax\"".into());
        };
        if !ordered(fx0, fx1) || !ordered(y0, y1) {
            return Err("a range must have min below max".into());
        }
        return Ok(Sampled {
            pieces: Vec::new(),
            marks: Vec::new(),
            ranges: Ranges {
                x0: fx0,
                x1: fx1,
                y0,
                y1,
            },
            series: Vec::new(),
            bar_spacing: 1.0,
        });
    }

    let mut raws: Vec<Raw> = Vec::with_capacity(curves.len());
    for (i, c) in curves.iter().enumerate() {
        let which = |m: String| {
            if curves.len() > 1 {
                format!("curve {}: {m}", i + 1)
            } else {
                m
            }
        };
        let win = [fx0, fx1, wy0, wy1];
        let made = raw(c, spec, (fx0, fx1), (lx, ly), win, &raws).map_err(which)?;
        raws.push(made);
    }

    // The points of bars, stems and error bars.
    let mut series: Vec<Series> = Vec::with_capacity(raws.len());
    for (i, (r, c)) in raws.iter().zip(curves).enumerate() {
        let wanted = c.style != CurveStyle::Line || !r.errs.is_empty();
        let s: Series = if wanted {
            r.pts
                .iter()
                .enumerate()
                .filter_map(|(k, (_, p))| p.map(|p| (p, r.errs.get(k).copied().filter(|e| e.is_finite()))))
                .collect()
        } else {
            Vec::new()
        };
        if s.len() > MAX_BARS {
            return Err(format!(
                "curve {}: more than {MAX_BARS} bars, stems or error bars (use fewer points or samples)",
                i + 1
            ));
        }
        series.push(s);
    }
    let spacing = bar_spacing(&series, curves);

    // Ranges.
    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for r in &raws {
        xs.extend(r.pts.iter().filter_map(|(_, p)| p.map(|p| p.0)));
        ys.extend(r.pts.iter().filter_map(|(_, p)| p.map(|p| p.1)));
    }
    for (s, c) in series.iter().zip(curves) {
        if c.style == CurveStyle::Bar {
            // Room for the half bar at both ends.
            if let (Some(a), Some(b)) = (
                s.iter().map(|(p, _)| p.0).reduce(f64::min),
                s.iter().map(|(p, _)| p.0).reduce(f64::max),
            ) {
                xs.extend([a - 0.5 * spacing, b + 0.5 * spacing]);
            }
        }
        if c.style != CurveStyle::Line && !s.is_empty() {
            ys.extend(tr(c.baseline, ly));
        }
        for &(p, e) in s {
            if let Some(e) = e {
                let (lo, hi) = err_ends(p.1, e, ly);
                ys.extend(lo);
                ys.extend(hi);
            }
        }
    }
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

    let budget = MAX_POINTS / raws.len().max(1);
    let mut all_pieces = Vec::with_capacity(raws.len());
    let mut marks = Vec::with_capacity(raws.len());
    for (r, c) in raws.into_iter().zip(curves) {
        marks.push(if c.marker == Marker::None {
            Vec::new()
        } else {
            r.pts
                .iter()
                .filter_map(|(_, p)| *p)
                .filter(|p| p.0 >= x0 && p.0 <= x1 && p.1 >= y0 && p.1 <= y1)
                .collect()
        });
        all_pieces.push(pieces_of(r, &ranges, budget));
    }
    Ok(Sampled {
        pieces: all_pieces,
        marks,
        ranges,
        series,
        bar_spacing: spacing,
    })
}

/// The ends of an error bar of half length `e` at `y` (plot coordinates);
/// on a logarithmic axis the lower end may not exist (below zero).
fn err_ends(y: f64, e: f64, log: bool) -> (Option<f64>, Option<f64>) {
    let e = e.abs();
    if log {
        let d = 10f64.powf(y);
        (tr(d - e, true), tr(d + e, true))
    } else {
        (finite(y - e), finite(y + e))
    }
}

/// The raw samples of one curve; `fx` is a function's x range (already on
/// the axis's scale), `log` the axes' logarithms.
fn raw(
    c: &CurveSpec,
    spec: &PlotSpec,
    fx: (f64, f64),
    log: (bool, bool),
    win: [f64; 4],
    prev: &[Raw],
) -> Result<Raw, String> {
    let (lx, ly) = log;
    let mut pts: Vec<(f64, Option<P>)> = Vec::new();
    let mut errs: Vec<f64> = Vec::new();
    let mut curve: Option<Box<dyn Fn(f64) -> Option<P>>> = None;
    let (mut u0, mut u1) = (0.0, 1.0);
    if c.mode() != PlotMode::Data && (c.yerr_col > 0 || !c.yerr.is_empty()) {
        return Err("error bars (yerr) belong to data curves".into());
    }
    match c.mode() {
        PlotMode::Data => {
            let table: Vec<(f64, f64, f64)> = if c.points.is_empty() {
                if c.data.trim().is_empty() {
                    return Err("the plot has no data (give a file)".into());
                }
                parse_table_err(&c.data, c.xcol, c.ycol, c.yerr_col)?
            } else {
                // One error value for all points, or one each.
                c.points
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let e = if c.yerr.len() == 1 {
                            c.yerr.first()
                        } else {
                            c.yerr.get(i)
                        };
                        (p[0], p[1], e.copied().unwrap_or(f64::NAN))
                    })
                    .collect()
            };
            let has_err = c.yerr_col > 0 || !c.yerr.is_empty();
            for (i, (x, y, e)) in table.into_iter().enumerate() {
                pts.push((i as f64, tr(x, lx).zip(tr(y, ly))));
                if has_err {
                    errs.push(e);
                }
            }
        }
        PlotMode::Function => {
            if c.expr.trim().is_empty() {
                return Err("the curve has no expression".into());
            }
            let f = parse(&c.expr, &["x"])?;
            let (a, b) = fx;
            if !ordered(a, b) {
                return Err("the x range must have min below max".into());
            }
            (u0, u1) = (a, b);
            curve = Some(Box::new(move |u| {
                let x = if lx { 10f64.powf(u) } else { u };
                tr(f.eval(&[x]), ly).map(|y| (u, y))
            }));
        }
        PlotMode::Polar => {
            if c.expr.trim().is_empty() {
                return Err("the curve has no expression r(t)".into());
            }
            let f = parse(&c.expr, &["t", "theta"])?;
            let (a, b) = (c.tmin.unwrap_or(0.0), c.tmax.unwrap_or(std::f64::consts::TAU));
            if !ordered(a, b) {
                return Err("the t range must have min below max".into());
            }
            (u0, u1) = (a, b);
            curve = Some(Box::new(move |t| {
                let r = f.eval(&[t, t]);
                let x = tr(r * t.cos(), lx)?;
                let y = tr(r * t.sin(), ly)?;
                Some((x, y))
            }));
        }
        PlotMode::Implicit => {
            if c.expr.trim().is_empty() {
                return Err("the curve has no equation f(x, y) = 0".into());
            }
            let f = parse_equation(&c.expr, &["x", "y"])?;
            if !ordered(win[0], win[1]) || !ordered(win[2], win[3]) {
                return Err("a range must have min below max".into());
            }
            let n = c.samples.unwrap_or(spec.samples);
            pts = implicit_pts(&f, win, n, log);
        }
        PlotMode::Field => {
            if lx || ly {
                return Err("field lines need linear axes".into());
            }
            let (fp, fq) = (parse(&c.xt, &["x", "y"])?, parse(&c.yt, &["x", "y"])?);
            if c.starts.is_empty() {
                return Err("field lines need start points (\"starts\": [[x, y]...])".into());
            }
            if !ordered(win[0], win[1]) || !ordered(win[2], win[3]) {
                return Err("a range must have min below max".into());
            }
            pts = field_pts(&fp, &fq, c, win);
        }
        PlotMode::Derivative | PlotMode::Integral => {
            let src =
                c.of.and_then(|k| k.checked_sub(1))
                    .and_then(|k| prev.get(k))
                    .ok_or("a derivative or integral needs \"of\": an earlier curve's number")?;
            pts = derived_pts(src, c.mode() == PlotMode::Integral, log);
        }
        PlotMode::Parametric => {
            let fx = parse(&c.xt, &["t"])?;
            let fy = parse(&c.yt, &["t"])?;
            let (a, b) = (c.tmin.unwrap_or(0.0), c.tmax.unwrap_or(std::f64::consts::TAU));
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
        let n = c.samples.unwrap_or(spec.samples).clamp(2, MAX_SAMPLES);
        for i in 0..=n {
            let u = if i == n {
                u1
            } else {
                u0 + (u1 - u0) * i as f64 / n as f64
            };
            pts.push((u, f(u)));
        }
    }
    Ok(Raw { pts, curve, errs })
}

/// Marching squares: the curve f(x, y) = 0 over the window `win` (in plot
/// coordinates) on an n by n grid, as polylines separated by `None`.
fn implicit_pts(f: &Formula, win: [f64; 4], n: usize, log: (bool, bool)) -> Vec<(f64, Option<P>)> {
    use std::collections::HashMap;
    let [x0, x1, y0, y1] = win;
    let n = n.clamp(2, MAX_IMPLICIT_GRID);
    let (dx, dy) = ((x1 - x0) / n as f64, (y1 - y0) / n as f64);
    let w = n + 1;
    let mut v = vec![f64::NAN; w * w];
    for j in 0..=n {
        let y = y0 + dy * j as f64;
        let yd = if log.1 { 10f64.powf(y) } else { y };
        for i in 0..=n {
            let x = x0 + dx * i as f64;
            let xd = if log.0 { 10f64.powf(x) } else { x };
            let val = f.eval(&[xd, yd]);
            if let Some(slot) = v.get_mut(j * w + i) {
                if val.is_finite() && val.abs() <= BIG {
                    *slot = val;
                }
            }
        }
    }
    let at = |i: usize, j: usize| v.get(j * w + i).copied().unwrap_or(f64::NAN);
    // Edge keys: horizontal edge (i, j)-(i+1, j) is 2(jw+i), vertical
    // (i, j)-(i, j+1) is 2(jw+i)+1.
    let h = |i: usize, j: usize| 2 * (j * w + i);
    let vt = |i: usize, j: usize| 2 * (j * w + i) + 1;
    let mut segs: Vec<(usize, usize)> = Vec::new();
    for j in 0..n {
        for i in 0..n {
            let (a, b, c, d) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
            if [a, b, c, d].iter().any(|q| q.is_nan()) {
                continue;
            }
            let case = usize::from(a > 0.0)
                | usize::from(b > 0.0) << 1
                | usize::from(c > 0.0) << 2
                | usize::from(d > 0.0) << 3;
            let (bo, r, t, l) = (h(i, j), vt(i + 1, j), h(i, j + 1), vt(i, j));
            let center = 0.25 * (a + b + c + d) > 0.0;
            let mut push = |p: usize, q: usize| segs.push((p, q));
            match case {
                1 | 14 => push(l, bo),
                2 | 13 => push(bo, r),
                3 | 12 => push(l, r),
                4 | 11 => push(r, t),
                6 | 9 => push(bo, t),
                7 | 8 => push(t, l),
                5 => {
                    if center {
                        push(bo, r);
                        push(t, l);
                    } else {
                        push(l, bo);
                        push(r, t);
                    }
                }
                10 => {
                    if center {
                        push(l, bo);
                        push(r, t);
                    } else {
                        push(bo, r);
                        push(t, l);
                    }
                }
                _ => {}
            }
            if segs.len() > MAX_POINTS {
                break;
            }
        }
    }
    let point = |key: usize| -> P {
        let (cell, vert) = (key / 2, key % 2 == 1);
        let (i, j) = (cell % w, cell / w);
        let v1 = at(i, j);
        let v2 = if vert { at(i, j + 1) } else { at(i + 1, j) };
        let t = if v1 == v2 {
            0.5
        } else {
            (v1 / (v1 - v2)).clamp(0.0, 1.0)
        };
        if vert {
            (x0 + dx * i as f64, y0 + dy * (j as f64 + t))
        } else {
            (x0 + dx * (i as f64 + t), y0 + dy * j as f64)
        }
    };
    let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
    for (k, (p, q)) in segs.iter().enumerate() {
        adj.entry(*p).or_default().push(k);
        adj.entry(*q).or_default().push(k);
    }
    let mut used = vec![false; segs.len()];
    let mut out: Vec<(f64, Option<P>)> = Vec::new();
    let next = |end: usize, used: &mut Vec<bool>| -> Option<usize> {
        let k = adj
            .get(&end)?
            .iter()
            .copied()
            .find(|k| !used.get(*k).copied().unwrap_or(true))?;
        *used.get_mut(k)? = true;
        let (p, q) = segs.get(k).copied()?;
        Some(if p == end { q } else { p })
    };
    for s0 in 0..segs.len() {
        if used.get(s0).copied().unwrap_or(true) {
            continue;
        }
        if let Some(u) = used.get_mut(s0) {
            *u = true;
        }
        let Some(&(p, q)) = segs.get(s0) else { continue };
        let mut path = std::collections::VecDeque::from([p, q]);
        while let Some(k) = path.back().copied().and_then(|e| next(e, &mut used)) {
            path.push_back(k);
        }
        while let Some(k) = path.front().copied().and_then(|e| next(e, &mut used)) {
            path.push_front(k);
        }
        for key in &path {
            out.push((out.len() as f64, Some(point(*key))));
        }
        out.push((out.len() as f64, None));
        if out.len() > MAX_POINTS {
            break;
        }
    }
    out
}

/// Field lines of (P, Q) from the start points, by RK4 on the direction
/// field (unit steps along the line), forward and backward, until a line
/// leaves the window, the field vanishes or the steps are used up.
fn field_pts(fp: &Formula, fq: &Formula, c: &CurveSpec, win: [f64; 4]) -> Vec<(f64, Option<P>)> {
    let [x0, x1, y0, y1] = win;
    let h = c.field_step.unwrap_or_else(|| ((x1 - x0).hypot(y1 - y0)) / 100.0);
    let dir = |p: P| -> Option<P> {
        let (a, b) = (fp.eval(&[p.0, p.1]), fq.eval(&[p.0, p.1]));
        let m = a.hypot(b);
        (m.is_finite() && m > 1e-12 && a.is_finite() && b.is_finite()).then(|| (a / m, b / m))
    };
    let inside = |p: P| p.0 >= x0 && p.0 <= x1 && p.1 >= y0 && p.1 <= y1;
    let rk4 = |p: P, s: f64| -> Option<P> {
        let h = s * h;
        let k1 = dir(p)?;
        let k2 = dir((p.0 + 0.5 * h * k1.0, p.1 + 0.5 * h * k1.1))?;
        let k3 = dir((p.0 + 0.5 * h * k2.0, p.1 + 0.5 * h * k2.1))?;
        let k4 = dir((p.0 + h * k3.0, p.1 + h * k3.1))?;
        let q = (
            p.0 + h / 6.0 * (k1.0 + 2.0 * k2.0 + 2.0 * k3.0 + k4.0),
            p.1 + h / 6.0 * (k1.1 + 2.0 * k2.1 + 2.0 * k3.1 + k4.1),
        );
        finite(q.0).zip(finite(q.1))
    };
    let mut work = MAX_FIELD_WORK;
    let mut out: Vec<(f64, Option<P>)> = Vec::new();
    for s in &c.starts {
        let start = (s[0], s[1]);
        let mut sides: Vec<Vec<P>> = Vec::new();
        for sign in [1.0, -1.0] {
            let mut line = Vec::new();
            let mut p = start;
            for _ in 0..c.field_steps {
                if work == 0 || !inside(p) {
                    break;
                }
                work -= 1;
                let Some(q) = rk4(p, sign) else { break };
                line.push(q);
                p = q;
            }
            sides.push(line);
        }
        let mut path: Vec<P> = sides.pop().unwrap_or_default().into_iter().rev().collect();
        path.push(start);
        path.extend(sides.pop().unwrap_or_default());
        for p in path {
            out.push((out.len() as f64, Some(p)));
        }
        out.push((out.len() as f64, None));
        if work == 0 || out.len() > MAX_POINTS {
            break;
        }
    }
    out
}

/// The derivative (central differences) or the running integral (trapezoids,
/// from 0 at the first point) of a curve's points, in data coordinates.
fn derived_pts(src: &Raw, integral: bool, log: (bool, bool)) -> Vec<(f64, Option<P>)> {
    let back = |v: f64, l: bool| if l { 10f64.powf(v) } else { v };
    let mut out: Vec<(f64, Option<P>)> = Vec::new();
    let mut run: Vec<(f64, f64)> = Vec::new();
    let mut total = 0.0;
    let flush = |run: &mut Vec<(f64, f64)>, out: &mut Vec<(f64, Option<P>)>, total: &mut f64| {
        let n = run.len();
        for k in 0..n {
            let Some(&(x, y)) = run.get(k) else { continue };
            let val = if integral {
                if let Some(&(xp, yp)) = k.checked_sub(1).and_then(|k| run.get(k)) {
                    *total += 0.5 * (y + yp) * (x - xp);
                }
                *total
            } else {
                let a = run.get(k.saturating_sub(1)).copied().unwrap_or((x, y));
                let b = run.get((k + 1).min(n.saturating_sub(1))).copied().unwrap_or((x, y));
                let dx = b.0 - a.0;
                if dx == 0.0 {
                    f64::NAN
                } else {
                    (b.1 - a.1) / dx
                }
            };
            out.push((out.len() as f64, tr(x, log.0).zip(tr(val, log.1))));
        }
        if n > 0 {
            out.push((out.len() as f64, None));
        }
        run.clear();
    };
    for (_, p) in &src.pts {
        match p {
            Some(p) => run.push((back(p.0, log.0), back(p.1, log.1))),
            None => flush(&mut run, &mut out, &mut total),
        }
    }
    flush(&mut run, &mut out, &mut total);
    out
}

/// One curve's pieces: adaptive refinement of a sampled curve (at most
/// `budget` points more), cut where it is undefined or crosses a pole.
fn pieces_of(raw: Raw, r: &Ranges, budget: usize) -> Vec<Vec<P>> {
    let Raw { pts, curve, .. } = raw;
    let (y0, y1) = (r.y0, r.y1);
    let mut all: Vec<(f64, Option<P>)> = Vec::with_capacity(pts.len());
    if let Some(f) = &curve {
        let mut rf = Refiner {
            f: &**f,
            sx: r.x1 - r.x0,
            sy: r.y1 - r.y0,
            budget: budget.saturating_sub(pts.len()) / 2,
        };
        for w in pts.windows(2) {
            if let [(ua, pa), (ub, pb)] = *w {
                all.push((ua, pa));
                rf.refine(ua, pa, ub, pb, 10, &mut all);
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
    pieces
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

/// How far a tick of length `len` reaches outside and inside the frame.
fn tick_reach(dir: TickDir, len: f64) -> (f64, f64) {
    match dir {
        TickDir::In => (0.0, len),
        TickDir::Out => (len, 0.0),
        TickDir::Both => (len, len),
    }
}

/// A tick number as LaTeX math: `-2.5` is `$-2.5$`, `1e-5` is `$10^{-5}$`
/// and `2e6` is `$2\\times10^{6}$`.
pub fn latex_number(l: &str) -> String {
    match l.split_once('e') {
        Some(("1", exp)) => format!("$10^{{{exp}}}$"),
        Some(("-1", exp)) => format!("$-10^{{{exp}}}$"),
        Some((m, exp)) => format!("${m}\\times10^{{{exp}}}$"),
        None if l.contains('\u{3c0}') => format!("${}$", l.replace('\u{3c0}', "\\pi{}")),
        None => format!("${l}$"),
    }
}

/// A tick: the position in plot coordinates, its number (major ticks only).
struct Tick {
    at: f64,
    major: bool,
    label: Option<String>,
    /// The label is the user's own text, not a number.
    own: bool,
}

fn pi_label(v: f64) -> String {
    let r = v / std::f64::consts::PI;
    for d in 1..=12i64 {
        let n = (r * d as f64).round();
        if (r * d as f64 - n).abs() < 1e-9 && n.abs() < 1e6 {
            let n = n as i64;
            if n == 0 {
                return "0".into();
            }
            let g = {
                let (mut a, mut b) = (n.abs(), d);
                while b != 0 {
                    (a, b) = (b, a % b);
                }
                a.max(1)
            };
            let (n, d) = (n / g, d / g);
            let coef = match n {
                1 => String::new(),
                -1 => "-".into(),
                n => n.to_string(),
            };
            return if d == 1 {
                format!("{coef}\u{3c0}")
            } else {
                format!("{coef}\u{3c0}/{d}")
            };
        }
    }
    format!("{}\u{3c0}", format_tick(r))
}

/// A tick number as written: scaled (a linear axis), formatted, with the
/// decimal comma when asked.
fn format_with(spec: &TickSpec, v: f64) -> String {
    let s = format_plain(spec, v);
    if spec.comma {
        s.replace('.', ",")
    } else {
        s
    }
}

fn format_plain(spec: &TickSpec, v: f64) -> String {
    match spec.format {
        NumFormat::Auto => format_tick(v),
        NumFormat::Fixed => {
            let s = format!("{:.*}", spec.decimals as usize, v);
            if s.trim_start_matches('-').chars().all(|c| c == '0' || c == '.') {
                s.trim_start_matches('-').to_string()
            } else {
                s
            }
        }
        NumFormat::Scientific => {
            if v == 0.0 || !v.is_finite() {
                "0".into()
            } else {
                format!("{v:e}")
            }
        }
        NumFormat::Pi => pi_label(v),
    }
}

fn ticks(lo: f64, hi: f64, log: bool, spec: &TickSpec) -> Result<Vec<Tick>, String> {
    let mut out = Vec::new();
    // Scaled numbers: 1000 shows as 1 with a scale of 3 (not on a log axis).
    let div = if log { 1.0 } else { 10f64.powi(spec.scale.unwrap_or(0)) };
    let label = |v: f64| spec.numbers.then(|| format_with(spec, v / div));
    if !(lo.is_finite() && hi.is_finite()) {
        return Ok(out);
    }
    if !spec.labels.is_empty() {
        // Own labels: the ticks are exactly these.
        for (v, text) in &spec.labels {
            let Some(at) = tr(*v, log) else { continue };
            let eps = 1e-9 * (hi - lo).abs();
            if at >= lo - eps && at <= hi + eps {
                out.push(Tick {
                    at,
                    major: true,
                    label: spec.numbers.then(|| text.clone()),
                    own: true,
                });
            }
        }
        return Ok(out);
    }
    if log {
        let (a, b) = (lo.floor() as i64, hi.ceil() as i64);
        let step = match (spec.manual, spec.step) {
            (true, Some(s)) => (s.round() as i64).max(1),
            _ => ((b - a) / 10 + 1).max(1),
        };
        if (b - a) / step > MAX_TICKS {
            return Err("too many ticks on the logarithmic axis: use a larger step".into());
        }
        let minors = match spec.minor {
            Some(n) => i64::from(n.min(8)),
            None if step == 1 => 8,
            None => 0,
        };
        let mut e = a;
        while e <= b {
            let v = e as f64;
            if v >= lo - 1e-9 && v <= hi + 1e-9 {
                out.push(Tick {
                    at: v,
                    major: true,
                    label: label(10f64.powf(v)),
                    own: false,
                });
            }
            for m in 2..(2 + minors) {
                let w = v + (m as f64).log10();
                if w >= lo && w <= hi {
                    out.push(Tick {
                        at: w,
                        major: false,
                        label: None,
                        own: false,
                    });
                }
            }
            e += step;
        }
        return Ok(out);
    }
    let span = hi - lo;
    let auto_step = if spec.format == NumFormat::Pi {
        if span <= 4.0 * std::f64::consts::PI {
            std::f64::consts::FRAC_PI_2
        } else {
            std::f64::consts::PI
        }
    } else {
        nice_step(span, 6.0)
    };
    let step = match (spec.manual, spec.step, spec.count) {
        (true, Some(s), _) => s,
        (true, None, Some(n)) => span / n as f64,
        _ => auto_step,
    };
    if !(step.is_finite() && step > 0.0) {
        return Err("the tick step must be positive".into());
    }
    let first = match (spec.manual, spec.first) {
        (true, Some(f)) => f,
        _ => (lo / step - 1e-9).ceil() * step,
    };
    let minor = i64::from(spec.minor.unwrap_or(0));
    // Majors k with first + k*step in range (also one before, for minors).
    let k0 = ((lo - first) / step).floor() - 1.0;
    let k1 = ((hi - first) / step).ceil() + 1.0;
    if !(k0.is_finite() && k1.is_finite()) || k1 - k0 > (MAX_TICKS + 2) as f64 {
        return Err(format!("too many ticks (more than {MAX_TICKS}): use a larger step"));
    }
    let eps = 1e-9 * span.abs().max(1e-300);
    // A manual first tick starts the series.
    let lo = if spec.manual && spec.first.is_some() {
        lo.max(first)
    } else {
        lo
    };
    let mut k = k0 as i64;
    while k <= k1 as i64 {
        let v = first + k as f64 * step;
        if v >= lo - eps && v <= hi + eps {
            out.push(Tick {
                at: v,
                major: true,
                label: label(if v.abs() < eps { 0.0 } else { v }),
                own: false,
            });
        }
        for j in 1..=minor {
            let w = v + j as f64 * step / (minor + 1) as f64;
            if w >= lo - eps && w <= hi + eps {
                out.push(Tick {
                    at: w,
                    major: false,
                    label: None,
                    own: false,
                });
            }
        }
        k += 1;
    }
    Ok(out)
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
    pub markers: usize,
    /// Bars, stems and error bars drawn.
    pub bars: usize,
    /// What the caller should know (a curve outside the ranges).
    pub warnings: Vec<String>,
}

/// A number for a points line: plain, or with an exponent when that is shorter.
fn short(v: f64) -> String {
    let plain = format!("{v}");
    let exp = format!("{v:e}");
    if exp.len() < plain.len() {
        exp
    } else {
        plain
    }
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
    // The curves; inline data of a listed curve is kept as its points.
    let mut keep = spec.clone();
    let mut curves = spec.curve_list();
    if spec.curves.is_empty() {
        if let (Some(d), Some(c)) = (data, curves.first_mut()) {
            c.data = d.to_string();
        }
    }
    for (k, c) in keep.curves.iter_mut().zip(curves.iter_mut()) {
        if c.mode() == PlotMode::Data && c.points.is_empty() && c.file.is_empty() && !c.data.trim().is_empty() {
            let table = parse_table_err(&c.data, c.xcol, c.ycol, c.yerr_col)?;
            if table.len() > MAX_KEPT_POINTS {
                return Err(format!(
                    "at most {MAX_KEPT_POINTS} points kept in a plot (use a data file)"
                ));
            }
            if c.yerr_col > 0 {
                // The error column is kept as a list beside the points.
                c.yerr = table.iter().map(|t| t.2).collect();
                c.yerr_col = 0;
                k.yerr = c.yerr.clone();
                k.yerr_col = 0;
            }
            c.points = table.into_iter().map(|(x, y, _)| [x, y]).collect();
            c.data.clear();
            k.points = c.points.clone();
        }
        k.data.clear();
    }
    keep.check()?;
    let Sampled {
        pieces,
        marks,
        ranges: r,
        series,
        bar_spacing: spacing,
    } = sample(spec, &curves)?;
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
    let depth = spec.depth.unwrap_or(look.curve.depth);
    // The curves' style: the indicator's, then the plot's, then the curve's.
    let base = restyle(
        Style { depth, ..look.curve },
        spec.pen_color,
        spec.thickness,
        spec.line.as_deref(),
        spec.style_val,
    );
    let mut members: Vec<Object> = Vec::new();

    let kind = spec.axes_kind();
    let origin = kind == AxesKind::Origin;
    // The box only goes with ticks on the frame's edges.
    if spec.frame && kind == AxesKind::Frame {
        let a = Point::new(fig(rect.min.x), fig(rect.min.y));
        let b = Point::new(fig(rect.max.x), fig(rect.max.y));
        let mut frame = thin(LineKind::Solid, depth);
        frame.thickness = look.curve.thickness.clamp(1, 3);
        push_part(
            &mut members,
            "frame",
            Object::new(ObjectKind::Polyline(Polyline::rect(frame, a, b))),
        );
    }

    // Where the axes cross: (0, 0), or the range's nearest edge (the lower
    // one on a logarithmic axis).
    let cross = |lo: f64, hi: f64, log: bool| {
        if log || lo > 0.0 {
            lo
        } else if hi < 0.0 {
            hi
        } else {
            0.0
        }
    };
    let (cross_x, cross_y) = (cross(r.x0, r.x1, spec.log_x), cross(r.y0, r.y1, spec.log_y));
    // Where the x axis (its ticks and numbers) and the y axis stand.
    let ay = if origin { fig(my(cross_y)) } else { fig(rect.max.y) };
    let ax = if origin { fig(mx(cross_x)) } else { fig(rect.min.x) };
    if origin {
        let mut st = thin(LineKind::Solid, depth);
        st.thickness = look.curve.thickness.clamp(1, 3);
        let arrow = spec.axis_arrows.then(|| Arrow {
            thickness: f64::from(st.thickness),
            ..Arrow::default()
        });
        let mut xa = Polyline::new(
            PolyKind::Polyline,
            st,
            vec![Point::new(fig(rect.min.x), ay), Point::new(fig(rect.max.x), ay)],
        );
        xa.forward_arrow = arrow;
        let mut ya = Polyline::new(
            PolyKind::Polyline,
            st,
            vec![Point::new(ax, fig(rect.max.y)), Point::new(ax, fig(rect.min.y))],
        );
        ya.forward_arrow = arrow;
        push_part(&mut members, "axis", Object::new(ObjectKind::Polyline(xa)));
        push_part(&mut members, "axis", Object::new(ObjectKind::Polyline(ya)));
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

    // A tick number as typed on the canvas, or as math for LaTeX.
    let tick_text = |l: &str, own: bool| {
        if own || !spec.latex_numbers {
            l.to_string()
        } else {
            // A decimal comma stands in braces, so TeX does not space it.
            latex_number(l).replace(',', "{,}")
        }
    };
    // No axes: no ticks and numbers either.
    let (mut xt, mut yt) = (spec.xticks.clone(), spec.yticks.clone());
    if kind == AxesKind::None {
        for t in [&mut xt, &mut yt] {
            t.ticks = false;
            t.numbers = false;
        }
    }
    let mut left_width = 0.0f64;
    let mut x_chars = 0usize;
    {
        let tick_h = tick_size * unit * 0.75;
        let gap = 0.5 * tick_h;
        // Ticks start on the axis whether or not the box is drawn.
        let xout = if xt.ticks { tick_reach(xt.dir, xt.length).0 } else { 0.0 };
        let yout = if yt.ticks { tick_reach(yt.dir, yt.length).0 } else { 0.0 };
        if spec.grid {
            // Major lines in the grid style, minor ones dotted at twice the gap.
            let major_kind = spec
                .grid_style
                .as_deref()
                .and_then(LineKind::from_name)
                .unwrap_or(LineKind::Dotted);
            let mut major = thin(major_kind, depth + 1);
            major.style_val = match major_kind {
                LineKind::Solid => 0.0,
                LineKind::Dotted => look.grid_gap,
                _ => 4.0,
            };
            let mut minor = thin(LineKind::Dotted, depth + 1);
            minor.style_val = 2.0 * look.grid_gap;
            let gridticks = |lo: f64, hi: f64, log: bool, t: &TickSpec| {
                let mut g = t.clone();
                g.numbers = false;
                if spec.grid_minor && g.minor.is_none() && !log {
                    g.minor = Some(4);
                }
                ticks(lo, hi, log, &g)
            };
            for t in gridticks(r.x0, r.x1, spec.log_x, &spec.xticks)? {
                if t.major || spec.grid_minor {
                    let x = fig(mx(t.at));
                    let st = if t.major { major } else { minor };
                    push_part(
                        &mut members,
                        "grid",
                        line(st, vec![Point::new(x, fig(rect.min.y)), Point::new(x, fig(rect.max.y))]),
                    );
                }
            }
            for t in gridticks(r.y0, r.y1, spec.log_y, &spec.yticks)? {
                if t.major || spec.grid_minor {
                    let y = fig(my(t.at));
                    let st = if t.major { major } else { minor };
                    push_part(
                        &mut members,
                        "grid",
                        line(st, vec![Point::new(fig(rect.min.x), y), Point::new(fig(rect.max.x), y)]),
                    );
                }
            }
        }
        for t in ticks(r.x0, r.x1, spec.log_x, &xt)? {
            let x = fig(mx(t.at));
            let len = if t.major { xt.length } else { 0.5 * xt.length };
            let (o, i) = tick_reach(xt.dir, len);
            if xt.ticks {
                push_part(
                    &mut members,
                    "tick",
                    line(
                        thin(LineKind::Solid, depth),
                        vec![Point::new(x, fig(ay + o)), Point::new(x, fig(ay - i))],
                    ),
                );
            }
            if let Some(l) = &t.label {
                let a = xt.rotate.to_radians();
                let (justify, at) = if xt.rotate == 0.0 {
                    (Justify::Center, Point::new(x, fig(ay + xout + gap + tick_h)))
                } else {
                    // The text ends (or starts, turned clockwise) under the tick.
                    let j = if xt.rotate > 0.0 { Justify::Right } else { Justify::Left };
                    (
                        j,
                        Point::new(
                            fig(x + 0.35 * tick_h * a.sin()),
                            fig(ay + xout + gap + 0.35 * tick_h * a.cos() + 0.3 * tick_h),
                        ),
                    )
                };
                x_chars = x_chars.max(l.chars().count());
                push_part(
                    &mut members,
                    "number",
                    text(&tick_text(l, t.own), justify, at, a, tick_size, spec.latex_numbers),
                );
            }
        }
        for t in ticks(r.y0, r.y1, spec.log_y, &yt)? {
            let y = fig(my(t.at));
            let len = if t.major { yt.length } else { 0.5 * yt.length };
            let (o, i) = tick_reach(yt.dir, len);
            if yt.ticks {
                push_part(
                    &mut members,
                    "tick",
                    line(
                        thin(LineKind::Solid, depth),
                        vec![Point::new(fig(ax - o), y), Point::new(fig(ax + i), y)],
                    ),
                );
            }
            if let Some(l) = &t.label {
                // The number at the crossing is the x axis's.
                if origin && (t.at - cross_y).abs() < 1e-9 * (r.y1 - r.y0).abs() {
                    continue;
                }
                let a = yt.rotate.to_radians();
                let w = l.chars().count() as f64 * tick_size * unit * 0.55;
                left_width = left_width.max(w * a.cos().abs() + tick_h * a.sin().abs());
                push_part(
                    &mut members,
                    "number",
                    text(
                        &tick_text(l, t.own),
                        Justify::Right,
                        Point::new(fig(ax - yout - gap), fig(y + 0.35 * tick_h * a.cos())),
                        a,
                        tick_size,
                        spec.latex_numbers,
                    ),
                );
            }
        }
        let label_size = size;
        let label_h = label_size * unit * 0.75;
        // The scale factor stands at the end of each axis that has one.
        let scale_text = |e: i32| {
            if spec.latex_numbers {
                format!("$\\times10^{{{e}}}$")
            } else {
                format!("\u{d7}10^{e}")
            }
        };
        if let Some(e) = xt.scale.filter(|_| xt.numbers && !spec.log_x) {
            let half = 0.5 * x_chars as f64 * tick_size * unit * 0.55;
            push_part(
                &mut members,
                "number",
                text(
                    &scale_text(e),
                    Justify::Left,
                    Point::new(fig(rect.max.x + half + gap), fig(ay + xout + gap + tick_h)),
                    0.0,
                    tick_size,
                    spec.latex_numbers,
                ),
            );
        }
        if let Some(e) = yt.scale.filter(|_| yt.numbers && !spec.log_y) {
            let (justify, at) = if origin {
                (Justify::Center, Point::new(ax, fig(rect.min.y - gap - 1.3 * label_h)))
            } else {
                (Justify::Right, Point::new(fig(ax - yout - gap), fig(rect.min.y - gap)))
            };
            push_part(
                &mut members,
                "number",
                text(&scale_text(e), justify, at, 0.0, tick_size, spec.latex_numbers),
            );
        }
        if origin {
            // The labels stand at the arrow tips.
            if !spec.xlabel.trim().is_empty() {
                push_part(
                    &mut members,
                    "label",
                    text(
                        &spec.xlabel,
                        Justify::Left,
                        Point::new(fig(rect.max.x + gap), fig(ay + 0.35 * label_h)),
                        0.0,
                        label_size,
                        spec.latex,
                    ),
                );
            }
            if !spec.ylabel.trim().is_empty() {
                push_part(
                    &mut members,
                    "label",
                    text(
                        &spec.ylabel,
                        Justify::Center,
                        Point::new(ax, fig(rect.min.y - gap)),
                        0.0,
                        label_size,
                        spec.latex,
                    ),
                );
            }
        } else {
            if !spec.xlabel.trim().is_empty() {
                let below = if xt.numbers { gap + tick_h } else { xout };
                let y = rect.max.y + below + gap + label_h * 1.1;
                push_part(
                    &mut members,
                    "label",
                    text(
                        &spec.xlabel,
                        Justify::Center,
                        Point::new(fig(0.5 * (rect.min.x + rect.max.x)), fig(y)),
                        0.0,
                        label_size,
                        spec.latex,
                    ),
                );
            }
            if !spec.ylabel.trim().is_empty() {
                // Reads upwards; the baseline lies right of the glyphs.
                let x = rect.min.x - yout - gap - left_width - gap;
                push_part(
                    &mut members,
                    "label",
                    text(
                        &spec.ylabel,
                        Justify::Center,
                        Point::new(fig(x), fig(0.5 * (rect.min.y + rect.max.y))),
                        std::f64::consts::FRAC_PI_2,
                        label_size,
                        spec.latex,
                    ),
                );
            }
        }
    }

    let mut count = 0;
    let mut total = 0;
    let mut markers = 0;
    let mut bars = 0;
    let mut warnings = Vec::new();
    let several = curves.len() > 1;
    // Every curve cut at the window: what leaves it ends on its edge.
    let clipped: Vec<Vec<Vec<P>>> = pieces.iter().map(|p| clip_pieces(p, &r)).collect();
    // Areas first, behind the grid and the curves.
    for (i, c) in curves.iter().enumerate() {
        let Some(f) = &c.fill else { continue };
        let own = clipped.get(i).map(Vec::as_slice).unwrap_or_default();
        let polys: Vec<Vec<P>> = match f.curve {
            Some(k) => {
                // Up to the other curve: this one forward, that one back.
                let other = k
                    .checked_sub(1)
                    .and_then(|k| clipped.get(k))
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let mut poly: Vec<P> = own.iter().flatten().copied().collect();
                poly.extend(other.iter().flatten().rev().copied());
                vec![poly]
            }
            None => {
                let base_y = tr(f.y, spec.log_y).unwrap_or(r.y0).clamp(r.y0, r.y1);
                own.iter()
                    .filter_map(|run| {
                        let (first, last) = (run.first()?, run.last()?);
                        let mut poly = run.clone();
                        poly.push((last.0, base_y));
                        poly.push((first.0, base_y));
                        Some(poly)
                    })
                    .collect()
            }
        };
        let fill_color = ColorRef(f.color.unwrap_or(c.pen_color.unwrap_or(base.pen_color.0)));
        let mut drawn = 0;
        for poly in polys {
            let mut pts: Vec<Point> = Vec::with_capacity(poly.len());
            for &(x, y) in &poly {
                let p = Point::new(fig(mx(x)), fig(my(y)));
                if pts.last() != Some(&p) {
                    pts.push(p);
                }
            }
            while pts.len() > 1 && pts.first() == pts.last() {
                pts.pop();
            }
            if pts.len() < 3 {
                continue;
            }
            let st = Style {
                line: LineKind::Solid,
                thickness: 0,
                pen_color: fill_color,
                fill_color,
                depth: (depth + 2).clamp(0, MAX_DEPTH),
                pen_style: -1,
                area_fill: f.density,
                style_val: 0.0,
                cap: look.curve.cap,
                join: look.curve.join,
            };
            drawn += 1;
            push_part(
                &mut members,
                "fill",
                Object::new(ObjectKind::Polyline(Polyline::new(PolyKind::Polygon, st, pts))),
            );
        }
        if drawn == 0 {
            let m = "nothing to fill inside the ranges";
            warnings.push(if several {
                format!("curve {}: {m}", i + 1)
            } else {
                m.to_string()
            });
        }
    }
    let bar_count = curves.iter().filter(|c| c.style == CurveStyle::Bar).count();
    for (i, (((_, marks), c), runs)) in pieces.iter().zip(&marks).zip(&curves).zip(&clipped).enumerate() {
        let style = restyle(base, c.pen_color, c.thickness, c.line.as_deref(), c.style_val);
        let mut drawn = 0;
        let ser = series.get(i).map(Vec::as_slice).unwrap_or_default();
        // Grouped bars: this curve's bar among the bar curves of the plot.
        let bar_pos = curves.iter().take(i).filter(|c| c.style == CurveStyle::Bar).count();
        let slot = spacing * c.bar_width / bar_count.max(1) as f64;
        let shift = if c.style == CurveStyle::Bar {
            (bar_pos as f64 - 0.5 * (bar_count as f64 - 1.0)) * slot
        } else {
            0.0
        };
        let base_y = tr(c.baseline, spec.log_y).unwrap_or(r.y0).clamp(r.y0, r.y1);
        match c.style {
            CurveStyle::Line => {}
            CurveStyle::Bar => {
                let fill_color = ColorRef(c.fill.as_ref().and_then(|f| f.color).unwrap_or(style.pen_color.0));
                let bar = Style {
                    thickness: if c.has_line() { style.thickness } else { 0 },
                    fill_color,
                    area_fill: c.fill.as_ref().map_or(20, |f| f.density),
                    ..style
                };
                for &((x, y), _) in ser {
                    let (xa, xb) = ((x + shift - 0.5 * slot).max(r.x0), (x + shift + 0.5 * slot).min(r.x1));
                    let (ya, yb) = (base_y, y.clamp(r.y0, r.y1));
                    let a = Point::new(fig(mx(xa)), fig(my(ya.max(yb))));
                    let b = Point::new(fig(mx(xb)), fig(my(ya.min(yb))));
                    if xb > xa && a.x < b.x && a.y < b.y {
                        bars += 1;
                        drawn += 1;
                        push_part(
                            &mut members,
                            "bar",
                            Object::new(ObjectKind::Polyline(Polyline::rect(bar, a, b))),
                        );
                    }
                }
            }
            CurveStyle::Stem => {
                for &((x, y), _) in ser {
                    if let Some((p, q)) = clip((x, base_y), (x, y), &r) {
                        let (p, q) = (
                            Point::new(fig(mx(p.0)), fig(my(p.1))),
                            Point::new(fig(mx(q.0)), fig(my(q.1))),
                        );
                        if p != q {
                            bars += 1;
                            drawn += 1;
                            push_part(&mut members, "stem", line(style, vec![p, q]));
                        }
                    }
                }
            }
        }
        // Error bars: a vertical line with a cap at each end that is inside.
        let ebar = Style {
            line: LineKind::Solid,
            style_val: 0.0,
            ..style
        };
        for &(p, e) in ser {
            let Some(e) = e else { continue };
            let x = p.0 + shift;
            let (lo, hi) = err_ends(p.1, e, spec.log_y);
            let (Some(hi), lo) = (hi, lo.unwrap_or(r.y0)) else {
                continue;
            };
            if x < r.x0 || x > r.x1 {
                continue;
            }
            let Some((a, b)) = clip((x, lo), (x, hi), &r) else {
                continue;
            };
            let at = |p: P| Point::new(fig(mx(p.0)), fig(my(p.1)));
            let half = 0.5 * c.err_cap;
            if at(a) != at(b) {
                bars += 1;
                drawn += 1;
                push_part(&mut members, "errorbar", line(ebar, vec![at(a), at(b)]));
            }
            if half > 0.0 {
                for (y, inside) in [(lo, lo >= r.y0 && lo <= r.y1), (hi, hi >= r.y0 && hi <= r.y1)] {
                    if inside {
                        let m = at((x, y));
                        push_part(
                            &mut members,
                            "errorbar",
                            line(
                                ebar,
                                vec![Point::new(fig(m.x - half), m.y), Point::new(fig(m.x + half), m.y)],
                            ),
                        );
                    }
                }
            }
        }
        if c.has_line() && c.style == CurveStyle::Line {
            for piece in runs {
                let mut pts: Vec<Point> = Vec::with_capacity(piece.len());
                for &(x, y) in piece {
                    let p = Point::new(fig(mx(x)), fig(my(y)));
                    if pts.last() != Some(&p) {
                        pts.push(p);
                    }
                }
                if let [only] = pts[..] {
                    // A single dot: a zero-length line draws it.
                    pts.push(only);
                }
                if pts.len() >= 2 {
                    total += pts.len();
                    count += 1;
                    drawn += 1;
                    push_part(&mut members, "curve", line(style, pts));
                }
            }
        }
        for &(x, y) in marks {
            if markers >= MAX_MARKERS {
                warnings.push(format!("only the first {MAX_MARKERS} markers are drawn"));
                break;
            }
            let at = Point::new(fig(mx(x)), fig(my(y)));
            for o in marker(c, style, at) {
                push_part(&mut members, "marker", o);
            }
            markers += 1;
            drawn += 1;
        }
        // A curve outside the ranges still gives the frame (wider ranges
        // later show it); the result says so.
        if drawn == 0 {
            let m = "nothing of the curve lies inside the ranges";
            warnings.push(if several {
                format!("curve {}: {m}", i + 1)
            } else {
                m.to_string()
            });
        }
    }
    warnings.dedup();

    let mut object = Object::new(ObjectKind::Compound(Compound { objects: members }));
    keep.rect = Some([rect.min.x, rect.min.y, rect.max.x, rect.max.y]);
    object.comments = keep.comments()?;
    Ok(Plot {
        object,
        curves: count,
        points: total,
        markers,
        bars,
        warnings,
    })
}

/// `s` with the style values that are given.
fn restyle(
    mut s: Style,
    pen: Option<i32>,
    thickness: Option<i32>,
    line: Option<&str>,
    style_val: Option<f64>,
) -> Style {
    if let Some(c) = pen {
        s.pen_color = ColorRef(c.clamp(-1, rustfig_model::LAST_USER_COLOR));
    }
    if let Some(t) = thickness {
        s.thickness = t.clamp(0, 10_000);
    }
    if let Some(l) = line.and_then(LineKind::from_name) {
        s.line = l;
        if l != LineKind::Solid && s.style_val <= 0.0 {
            // xfig's default dash length.
            s.style_val = 4.0;
        }
    }
    if let Some(v) = style_val {
        s.style_val = v.clamp(0.0, 1000.0);
    }
    s
}

/// The objects of one marker of curve `c` at `at`: solid lines of width 1
/// in the curve's color, closed shapes filled with it unless open.
fn marker(c: &CurveSpec, curve: Style, at: Point) -> Vec<Object> {
    let r = (c.marker_size / 2.0).max(0.5);
    let style = Style {
        line: LineKind::Solid,
        thickness: 1,
        style_val: 0.0,
        fill_color: curve.pen_color,
        area_fill: if c.marker_fill { 20 } else { -1 },
        ..curve
    };
    let pt = |dx: f64, dy: f64| Point::new(fig(at.x + dx), fig(at.y + dy));
    let shape = |pts: Vec<Point>| Object::new(ObjectKind::Polyline(Polyline::new(PolyKind::Polygon, style, pts)));
    let stroke = |a: Point, b: Point| {
        let s = Style { area_fill: -1, ..style };
        Object::new(ObjectKind::Polyline(Polyline::new(PolyKind::Polyline, s, vec![a, b])))
    };
    match c.marker {
        Marker::None => Vec::new(),
        Marker::Circle => vec![Object::new(ObjectKind::Ellipse(Ellipse {
            kind: EllipseKind::CircleRadius,
            style,
            direction: 1,
            angle: 0.0,
            center: at,
            radii: Point::new(fig(r).max(1.0), fig(r).max(1.0)),
            start: at,
            end: Point::new(at.x + fig(r).max(1.0), at.y),
        }))],
        Marker::Square => vec![shape(vec![pt(-r, -r), pt(r, -r), pt(r, r), pt(-r, r), pt(-r, -r)])],
        Marker::Diamond => vec![shape(vec![
            pt(0.0, -r),
            pt(r, 0.0),
            pt(0.0, r),
            pt(-r, 0.0),
            pt(0.0, -r),
        ])],
        Marker::Triangle => {
            let h = r * 3f64.sqrt() / 2.0;
            vec![shape(vec![pt(0.0, -r), pt(h, r / 2.0), pt(-h, r / 2.0), pt(0.0, -r)])]
        }
        Marker::Plus => vec![stroke(pt(-r, 0.0), pt(r, 0.0)), stroke(pt(0.0, -r), pt(0.0, r))],
        Marker::Cross => {
            let d = r / std::f64::consts::SQRT_2;
            vec![stroke(pt(-d, -d), pt(d, d)), stroke(pt(-d, d), pt(d, -d))]
        }
    }
}

/// The comment line that marks a generated part of a plot (frame, grid,
/// tick, number, label, curve); "Redraw" replaces exactly the marked parts.
pub const PART_COMMENT: &str = "rustfig-plot-part:";

fn push_part(members: &mut Vec<Object>, kind: &str, mut o: Object) {
    o.comments = vec![format!("{PART_COMMENT} {kind}")];
    members.push(o);
}

/// Whether the object is a generated part of a plot.
pub fn is_part(o: &Object) -> bool {
    part_kind(o).is_some()
}

fn part_kind(o: &Object) -> Option<&str> {
    o.comments
        .iter()
        .find_map(|c| c.trim_start().strip_prefix(PART_COMMENT))
        .map(str::trim)
}

/// Remove the part markers of a member of a compound that is being broken:
/// the pieces become an ordinary drawing.
pub fn strip_part_marker(o: &mut Object) {
    o.comments.retain(|c| !c.trim_start().starts_with(PART_COMMENT));
}

/// The frame of an existing plot: its first member when that is a box (the
/// frame, wherever the user moved or scaled it), else the stored rectangle.
pub fn frame_of(o: &Object, spec: &PlotSpec) -> Option<Rect> {
    if let ObjectKind::Compound(c) = &o.kind {
        let frame = c.objects.iter().find(|m| part_kind(m) == Some("frame"));
        if let Some(Object {
            kind: ObjectKind::Polyline(p),
            ..
        }) = frame
        {
            if let (PolyKind::Box, Some(first)) = (p.kind, p.points.first()) {
                let mut r = Rect::new(*first, *first);
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

    /// [`sample`] of the spec's one curve, with `data` as its data text.
    fn sample1(spec: &PlotSpec, data: Option<&str>) -> Result<(Vec<Vec<P>>, Ranges), String> {
        let mut curves = spec.curve_list();
        if let (Some(d), Some(c)) = (data, curves.first_mut()) {
            c.data = d.to_string();
        }
        let Sampled {
            mut pieces, ranges: r, ..
        } = sample(spec, &curves)?;
        Ok((
            if pieces.is_empty() {
                Vec::new()
            } else {
                pieces.remove(0)
            },
            r,
        ))
    }

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
        let (pieces, r) = sample1(&spec("sin(x)", 0.0, std::f64::consts::TAU), None).unwrap();
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
        let (pieces, r) = sample1(&s, None).unwrap();
        assert!(pieces.len() >= 2, "{} pieces", pieces.len());
        // No piece crosses x = 0.
        for p in &pieces {
            assert!(p.iter().all(|q| q.0 < 0.0) || p.iter().all(|q| q.0 > 0.0));
        }
        let clipped = clip_pieces(&pieces, &r);
        assert!(clipped.len() >= 2);
        // Auto range keeps the bulk, not 1e5.
        let (_, r) = sample1(&spec("1/x", -2.0, 2.0), None).unwrap();
        assert!(r.y1 < 100.0 && r.y0 > -100.0);
        // tan too.
        let (pieces, _) = sample1(&spec("tan(x)", -4.0, 4.0), None).unwrap();
        assert!(pieces.len() >= 3);
    }

    #[test]
    fn log_axes() {
        let mut s = spec("x^2", 1.0, 100.0);
        s.log_x = true;
        s.log_y = true;
        let (pieces, r) = sample1(&s, None).unwrap();
        assert_eq!(pieces.len(), 1);
        assert!((r.x0 - 0.0).abs() < 1e-9 && (r.x1 - 2.0).abs() < 1e-9);
        assert!(spec_err(PlotSpec {
            log_x: true,
            ..spec("x", -1.0, 1.0)
        }));
        let t = ticks(0.0, 2.0, true, &TickSpec::default()).unwrap();
        assert_eq!(t.iter().filter(|t| t.label.is_some()).count(), 3);
    }

    fn spec_err(s: PlotSpec) -> bool {
        sample1(&s, None).is_err()
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
        let (pieces, r) = sample1(&s, Some("0 0\n1 1\n2 4\n3 9\n")).unwrap();
        assert_eq!(pieces[0].len(), 4);
        assert!(r.x0 <= 0.0 && r.x1 >= 3.0 && r.y1 >= 9.0);
        assert!(sample1(&s, None).is_err());
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

    fn labels(t: &[Tick]) -> Vec<String> {
        t.iter().filter_map(|t| t.label.clone()).collect()
    }

    #[test]
    fn manual_ticks() {
        let mut ts = TickSpec {
            manual: true,
            first: Some(1.0),
            step: Some(2.5),
            minor: Some(4),
            ..TickSpec::default()
        };
        let t = ticks(0.0, 10.0, false, &ts).unwrap();
        let majors: Vec<f64> = t.iter().filter(|t| t.major).map(|t| t.at).collect();
        assert_eq!(majors, vec![1.0, 3.5, 6.0, 8.5]);
        assert!(t.iter().filter(|t| !t.major).count() >= 14);
        // A count of intervals.
        ts = TickSpec {
            manual: true,
            count: Some(4),
            ..TickSpec::default()
        };
        let t = ticks(0.0, 8.0, false, &ts).unwrap();
        assert_eq!(labels(&t), ["0", "2", "4", "6", "8"]);
        // No numbers, fixed decimals.
        ts.numbers = false;
        assert!(labels(&ticks(0.0, 8.0, false, &ts).unwrap()).is_empty());
        ts.numbers = true;
        ts.format = NumFormat::Fixed;
        ts.decimals = 1;
        assert_eq!(labels(&ticks(0.0, 8.0, false, &ts).unwrap())[1], "2.0");
        // Log: decades only.
        let ts = TickSpec {
            minor: Some(0),
            ..TickSpec::default()
        };
        assert_eq!(ticks(0.0, 3.0, true, &ts).unwrap().len(), 4);
    }

    #[test]
    fn pi_ticks() {
        let ts = TickSpec {
            manual: true,
            first: Some(std::f64::consts::FRAC_PI_2),
            step: Some(std::f64::consts::FRAC_PI_2),
            format: NumFormat::Pi,
            ..TickSpec::default()
        };
        let t = ticks(0.0, 2.0 * std::f64::consts::PI, false, &ts).unwrap();
        assert_eq!(labels(&t), ["\u{3c0}/2", "\u{3c0}", "3\u{3c0}/2", "2\u{3c0}"]);
        let auto = TickSpec {
            format: NumFormat::Pi,
            ..TickSpec::default()
        };
        let t = ticks(0.0, std::f64::consts::TAU, false, &auto).unwrap();
        assert_eq!(labels(&t)[0], "0");
        assert_eq!(labels(&t)[2], "\u{3c0}");
        assert_eq!(latex_number("3\u{3c0}/2"), "$3\\pi{}/2$");
    }

    #[test]
    fn hostile_ticks_are_errors() {
        let d = TickSpec::default;
        for ts in [
            TickSpec { step: Some(0.0), ..d() },
            TickSpec {
                step: Some(-1.0),
                ..d()
            },
            TickSpec {
                step: Some(f64::NAN),
                ..d()
            },
            TickSpec {
                first: Some(f64::INFINITY),
                ..d()
            },
            TickSpec { count: Some(0), ..d() },
            TickSpec {
                count: Some(1_000_000),
                ..d()
            },
            TickSpec { minor: Some(10), ..d() },
            TickSpec { length: -5.0, ..d() },
            TickSpec { decimals: 50, ..d() },
        ] {
            assert!(ts.check("x").is_err(), "{ts:?}");
        }
        // Valid but too many ticks for the range.
        let ts = TickSpec {
            manual: true,
            step: Some(1e-9),
            ..d()
        };
        assert!(ticks(0.0, 10.0, false, &ts).is_err());
    }

    #[test]
    fn nice_numbers() {
        assert_eq!(format_tick(0.5), "0.5");
        assert_eq!(format_tick(-0.0), "0");
        assert_eq!(format_tick(1e-5), "1e-5");
        assert_eq!(format_tick(100.0), "100");
        assert_eq!(nice_range(0.0, 0.93, false), (0.0, 1.0));
        let t = ticks(0.0, 6.5, false, &TickSpec::default()).unwrap();
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

    fn kinds(s: &PlotSpec) -> std::collections::BTreeMap<String, usize> {
        let look = PlotLook {
            curve: Style::default(),
            text_color: ColorRef::BLACK,
            font: 0,
            font_size: 12.0,
            postscript: true,
            grid_gap: 3.0,
        };
        let r = Rect::new(Point::new(0.0, 0.0), Point::new(4800.0, 3000.0));
        let p = build(s, r, None, &look).unwrap();
        let ObjectKind::Compound(c) = &p.object.kind else {
            panic!()
        };
        let mut m = std::collections::BTreeMap::new();
        for o in &c.objects {
            *m.entry(part_kind(o).unwrap_or("").to_string()).or_insert(0) += 1;
        }
        m
    }

    #[test]
    fn switches_are_independent() {
        let base = spec("sin(x)", 0.0, 6.0);
        let all = kinds(&base);
        assert!(all.contains_key("frame") && all.contains_key("tick") && all.contains_key("number"));

        let mut s = base.clone();
        s.frame = false;
        let m = kinds(&s);
        assert!(!m.contains_key("frame") && m.contains_key("tick") && m.contains_key("number"));

        let mut s = base.clone();
        s.xticks.ticks = false;
        s.yticks.ticks = false;
        let m = kinds(&s);
        assert!(m.contains_key("frame") && !m.contains_key("tick") && m.contains_key("number"));

        let mut s = base.clone();
        s.xticks.ticks = false;
        let m = kinds(&s);
        assert!(m["tick"] < all["tick"] && m["tick"] > 0);

        let mut s = base.clone();
        s.xticks.numbers = false;
        s.yticks.numbers = false;
        let m = kinds(&s);
        assert!(m.contains_key("tick") && !m.contains_key("number"));

        let mut s = base.clone();
        s.yticks.numbers = false;
        assert!(kinds(&s)["number"] < all["number"]);

        // Grid and labels do not depend on the others.
        let mut s = base;
        s.frame = false;
        s.xticks.ticks = false;
        s.yticks.ticks = false;
        s.xticks.numbers = false;
        s.yticks.numbers = false;
        s.grid = true;
        s.xlabel = "x".into();
        let m = kinds(&s);
        assert!(m.contains_key("grid") && m.contains_key("label") && !m.contains_key("frame"));
    }

    #[test]
    fn old_axes_switch_is_read() {
        let off = PlotSpec::from_comment(&format!("{PLOT_COMMENT} {{\"axes\":false}}")).unwrap();
        assert!(!off.frame && !off.xticks.ticks && !off.yticks.numbers && off.axes.is_none());
        let on = PlotSpec::from_comment(&format!("{PLOT_COMMENT} {{\"axes\":true}}")).unwrap();
        assert!(on.frame && on.xticks.ticks && on.yticks.numbers);
        let none = PlotSpec::from_comment(&format!("{PLOT_COMMENT} {{}}")).unwrap();
        assert!(none.frame && none.xticks.ticks && none.xticks.numbers);
        // Written with the new fields only.
        let mut s = PlotSpec::default();
        s.frame = false;
        let c = s.comment().unwrap();
        assert!(c.contains("\"frame\":false") && !c.contains("axes"));
        assert!(!PlotSpec::from_comment(&c).unwrap().frame);
    }
}
