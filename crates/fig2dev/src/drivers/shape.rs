//! fig2dev's `shape` driver: a LaTeX `\shapepar` shape (by Christian
//! Gollwitzer), the outline a paragraph is set into.
//!
//! Only objects whose first comment starts with `+` (a filled shape) or
//! `-` (a hole) count; a word after the sign names a group, and groups are
//! filled in the alphabetical order of their names. Comments starting with
//! `width`, `height` or `center` on a polyline set the reference width,
//! height or centre. A comment that starts with anything else (but a
//! blank) is an error. Polylines, polygons, boxes, rounded boxes, arcs and
//! ellipses become polygons; the polygons of each group are cut by
//! horizontal scan lines through every vertex and crossing, and the runs
//! inside become the `t`, `b`, `e`, `s` and `j` blocks of `\shapepar`.
//!
//! The C driver sorts with the C library's `qsort`, which is not stable;
//! [`qsort`] reproduces the BSD `qsort` of macOS, so that ties are broken
//! as there.
// Ported from fig2dev dev/genshape.c (xfig licence, see LICENSE-xfig).

use std::f64::consts::PI;

use crate::fprintf;
use crate::object::{
    Arc, Compound, Ellipse, Line, Spline, Text, T_ARC_BOX, T_BOX, T_PIC_BOX, T_PIE_WEDGE_ARC, T_POLYGON, T_POLYLINE,
};
use crate::{round, Ctx, Driver, Out};

/// Fig units per output unit.
const FSCALE: f64 = 10.0;
/// Closer polyline points are dropped.
const MIN_DIST: i32 = 5;
/// A circle becomes a 46-gon.
const ARC_STEP: f64 = PI / 23.0;

/// A polygon edge (`lineseg`).
#[derive(Clone, Copy, Debug, Default)]
struct LineSeg {
    x: i32,
    y: i32,
    intersect: bool,
    intersect_hit: bool,
    shapenr: usize,
    x2: i32,
    y2: i32,
}

/// A polygon (`shape`): its edges are `points[linestart..=lineend]`.
#[derive(Clone, Debug, Default)]
struct ShapeRec {
    ispositiv: bool,
    inside: bool,
    linestart: i64,
    lineend: i64,
    depth: i32,
    groupname: String,
}

impl ShapeRec {
    /// The indices of its edges.
    fn lines(&self) -> std::ops::Range<usize> {
        let start = usize::try_from(self.linestart).unwrap_or(0);
        let end = usize::try_from(self.lineend.saturating_add(1)).unwrap_or(0);
        start..end.max(start)
    }
}

/// Where a scan line meets an edge (`intersect_point`).
#[derive(Clone, Copy, Debug, Default)]
struct IntersectPoint {
    x: f32,
    y: f32,
    x2: f32,
    y2: f32,
    shapenr: usize,
    is_endpoint: bool,
}

/// Where two edges of different polygons cross (`fullintersect_point`).
#[derive(Clone, Copy, Debug)]
struct FullIntersect {
    y: f32,
    x: f32,
    nr1: usize,
    nr2: usize,
}

/// A `\shapepar` block (`textblock`): text from `x` of length `len`, or a
/// special (`b`, `e`, `s`, `j`) at `x`.
#[derive(Clone, Copy, Debug)]
struct Block {
    x: f32,
    len: f32,
    special: u8,
}

/// A scan line: one block list, or two (above and below) when it contains
/// horizontal edges.
#[derive(Clone, Debug, Default)]
struct Scanline {
    y: f32,
    contains_horizontal: bool,
    blocks: [Vec<Block>; 2],
}

/// The state of fig2dev's genshape.c.
struct Shape {
    macroname: Option<String>,
    scaleset: bool,
    centerset: bool,
    was_horizontal: bool,
    centerpos: f64,
    scaledim: f64,
    lastx: i32,
    lasty: i32,
    startx: i32,
    starty: i32,
    line_start: bool,
    points: Vec<LineSeg>,
    shapes: Vec<ShapeRec>,
    /// The shape being added (`shapes[num_shapes]`).
    current: ShapeRec,
}

/// The driver for `lang` (`shape`).
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    (lang == "shape").then(|| {
        Box::new(Shape {
            macroname: None,
            scaleset: false,
            centerset: false,
            was_horizontal: false,
            centerpos: 0.0,
            scaledim: 10.0,
            lastx: 0,
            lasty: 0,
            startx: 0,
            starty: 0,
            line_start: true,
            points: Vec::new(),
            shapes: Vec::new(),
            current: ShapeRec::default(),
        }) as Box<dyn Driver>
    })
}

/// `die`: fig2dev's message, and exit.
fn die(msg: &str) -> String {
    format!("fig2dev(shape): {msg}")
}

fn widthref(c: &str) -> bool {
    c.starts_with("width")
}

fn heightref(c: &str) -> bool {
    c.starts_with("height")
}

fn centerref(c: &str) -> bool {
    c.starts_with("center")
}

/// `get_comment`: the shape comment at the start of the first comment
/// (an optional sign and a word), empty for none; an error for a comment
/// that is neither a shape nor a reference.
fn get_comment(ctx: &mut Ctx, comments: &[String]) -> Result<String, String> {
    let Some(comment) = comments.first() else {
        return Ok(String::new());
    };
    let mut end = 0;
    if comment.starts_with(['+', '-']) {
        end = 1;
    }
    end += comment
        .get(end..)
        .unwrap_or("")
        .bytes()
        .take_while(u8::is_ascii_alphanumeric)
        .count();
    let comm = comment.get(..end).unwrap_or("").to_string();
    ctx.warn(format!("*comm: {comm}, comment->comment: {comment}"));
    if !comm.is_empty() && !comm.starts_with(['+', '-']) && !widthref(&comm) && !heightref(&comm) && !centerref(&comm) {
        ctx.warn(format!("Comment \"{comment}\": {comm}"));
        return Err(die(
            "Illegal shape specification, must start with '+' or '-' or be 'center', 'width' or height'\nUse blank to start real comment. See documentation for explanation",
        ));
    }
    Ok(comm)
}

/// `between`, `between_exclude`.
fn between(b1: f32, b2: f32, test: f32) -> bool {
    (b1 <= test && test <= b2) || (b2 <= test && test <= b1)
}

fn between_exclude(b1: f32, b2: f32, test: f32) -> bool {
    (b1 < test && test < b2) || (b2 < test && test < b1)
}

/// C's `%g`.
fn fmt_g(x: f64) -> String {
    if x.is_nan() {
        return if x.is_sign_negative() { "-nan" } else { "nan" }.into();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf" } else { "inf" }.into();
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0" } else { "0" }.into();
    }
    let e = format!("{x:.5e}");
    let (mant, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let trim = |s: &str| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if !(-4..6).contains(&exp) {
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{}e{sign}{:02}", trim(mant), exp.abs())
    } else {
        let prec = usize::try_from(5 - exp).unwrap_or(0);
        trim(&format!("{x:.prec$}"))
    }
}

/// The C library's `qsort` as macOS has it (FreeBSD's Bentley–McIlroy
/// quicksort with a recursion limit falling back to heapsort, and an
/// insertion sort tried after a partition that moved nothing), so that
/// elements comparing equal end up where they do there. `cmp` returns
/// C's negative, zero or positive.
pub(crate) fn qsort<T: Clone>(v: &mut [T], cmp: &mut impl FnMut(&T, &T) -> i32) {
    let n = v.len();
    let fls = usize::BITS - n.leading_zeros();
    let depth = 2 * (i64::from(fls) - 1);
    qsort_part(v, depth, cmp);
}

fn med3<T>(v: &[T], a: usize, b: usize, c: usize, cmp: &mut impl FnMut(&T, &T) -> i32) -> usize {
    let (Some(x), Some(y), Some(z)) = (v.get(a), v.get(b), v.get(c)) else {
        return a;
    };
    if cmp(x, y) < 0 {
        if cmp(y, z) < 0 {
            b
        } else if cmp(x, z) < 0 {
            c
        } else {
            a
        }
    } else if cmp(y, z) > 0 {
        b
    } else if cmp(x, z) < 0 {
        a
    } else {
        c
    }
}

/// `cmp(v[i], v[j])`, 0 for indices out of range.
fn cmp_at<T>(v: &[T], i: usize, j: usize, cmp: &mut impl FnMut(&T, &T) -> i32) -> i32 {
    match (v.get(i), v.get(j)) {
        (Some(a), Some(b)) => cmp(a, b),
        _ => 0,
    }
}

/// Insertion sort; gives up (`false`) after `limit` swaps, unless 0.
fn isort<T>(v: &mut [T], limit: usize, cmp: &mut impl FnMut(&T, &T) -> i32) -> bool {
    let mut swaps = 0;
    for m in 1..v.len() {
        let mut l = m;
        while l > 0 && cmp_at(v, l - 1, l, cmp) > 0 {
            v.swap(l, l - 1);
            swaps += 1;
            if limit != 0 && swaps > limit {
                return false;
            }
            l -= 1;
        }
    }
    true
}

/// `vecswap`: swap `n` elements from `a` with `n` from `b`.
fn vecswap<T>(v: &mut [T], a: usize, b: usize, n: usize) {
    for i in 0..n {
        if a + i < v.len() && b + i < v.len() {
            v.swap(a + i, b + i);
        }
    }
}

/// BSD `heapsort`.
fn heapsort<T: Clone>(v: &mut [T], cmp: &mut impl FnMut(&T, &T) -> i32) {
    let mut nmemb = v.len();
    if nmemb <= 1 {
        return;
    }
    // One-based indices, as in the C code.
    let c = |v: &[T], i: usize, j: usize, cmp: &mut dyn FnMut(&T, &T) -> i32| match (v.get(i - 1), v.get(j - 1)) {
        (Some(a), Some(b)) => cmp(a, b),
        _ => 0,
    };
    let mut l = nmemb / 2 + 1;
    while l > 1 {
        l -= 1;
        let mut i = l;
        while i * 2 <= nmemb {
            let mut j = i * 2;
            if j < nmemb && c(v, j, j + 1, cmp) < 0 {
                j += 1;
            }
            if c(v, j, i, cmp) <= 0 {
                break;
            }
            v.swap(i - 1, j - 1);
            i = j;
        }
    }
    while nmemb > 1 {
        let (Some(k), Some(top)) = (v.get(nmemb - 1).cloned(), v.first().cloned()) else {
            return;
        };
        if let Some(slot) = v.get_mut(nmemb - 1) {
            *slot = top;
        }
        nmemb -= 1;
        let mut i = 1;
        while i * 2 <= nmemb {
            let mut j = i * 2;
            if j < nmemb && c(v, j, j + 1, cmp) < 0 {
                j += 1;
            }
            if let Some(child) = v.get(j - 1).cloned() {
                if let Some(slot) = v.get_mut(i - 1) {
                    *slot = child;
                }
            }
            i = j;
        }
        loop {
            let child = i;
            i = child / 2;
            let place = child == 1 || v.get(i.wrapping_sub(1)).is_none_or(|p| cmp(&k, p) < 0);
            if place {
                if let Some(slot) = v.get_mut(child - 1) {
                    *slot = k;
                }
                break;
            }
            if let Some(parent) = v.get(i - 1).cloned() {
                if let Some(slot) = v.get_mut(child - 1) {
                    *slot = parent;
                }
            }
        }
    }
}

fn qsort_part<T: Clone>(mut v: &mut [T], mut depth: i64, cmp: &mut impl FnMut(&T, &T) -> i32) {
    loop {
        let n = v.len();
        if depth <= 0 {
            heapsort(v, cmp);
            return;
        }
        depth -= 1;
        if n < 8 {
            isort(v, 0, cmp);
            return;
        }
        let mut pm = n / 2;
        let mut pl = 0;
        let mut pn = n - 1;
        if n > 40 {
            let d = n / 8;
            pl = med3(v, pl, pl + d, pl + 2 * d, cmp);
            pm = med3(v, pm - d, pm, pm + d, cmp);
            pn = med3(v, pn - 2 * d, pn - d, pn, cmp);
        }
        pm = med3(v, pl, pm, pn, cmp);
        v.swap(0, pm);
        let (mut pa, mut pb) = (1, 1);
        let (mut pc, mut pd) = (n - 1, n - 1);
        let mut swapped = false;
        loop {
            while pb <= pc {
                let r = cmp_at(v, pb, 0, cmp);
                if r > 0 {
                    break;
                }
                if r == 0 {
                    swapped = true;
                    v.swap(pa, pb);
                    pa += 1;
                }
                pb += 1;
            }
            while pb <= pc {
                let r = cmp_at(v, pc, 0, cmp);
                if r < 0 {
                    break;
                }
                if r == 0 {
                    swapped = true;
                    v.swap(pc, pd);
                    pd -= 1;
                }
                pc -= 1;
            }
            if pb > pc {
                break;
            }
            v.swap(pb, pc);
            swapped = true;
            pb += 1;
            pc -= 1;
        }
        let r = pa.min(pb - pa);
        vecswap(v, 0, pb - r, r);
        let r = (pd - pc).min(n - pd - 1);
        vecswap(v, pb, n - r, r);
        if !swapped && isort(v, 1 + n / 4, cmp) {
            return;
        }
        let d1 = pb - pa;
        let d2 = pd - pc;
        let whole = v;
        if d1 <= d2 {
            let (left, rest) = whole.split_at_mut(d1);
            if d1 > 1 {
                qsort_part(left, depth, cmp);
            }
            if d2 <= 1 {
                return;
            }
            let len = rest.len();
            let Some(r) = rest.get_mut(len.saturating_sub(d2)..) else {
                return;
            };
            v = r;
        } else {
            let (left, rest) = whole.split_at_mut(n - d2);
            if d2 > 1 {
                qsort_part(rest, depth, cmp);
            }
            if d1 <= 1 {
                return;
            }
            let Some(l) = left.get_mut(..d1) else {
                return;
            };
            v = l;
        }
    }
}

/// The text block engine (`start_blocks`, `begin_block`, `end_block`,
/// `special_block`).
#[derive(Default)]
struct Blocks {
    list: Vec<Block>,
    inblock: bool,
    xval: f32,
}

impl Blocks {
    fn begin(&mut self, atx: f32) {
        self.inblock = true;
        self.xval = atx;
    }

    fn end(&mut self, atx: f32) -> Result<(), String> {
        if !self.inblock {
            return Err(die("End block outside of a block ???"));
        }
        self.inblock = false;
        self.list.push(Block {
            x: self.xval,
            len: atx - self.xval,
            special: 0,
        });
        Ok(())
    }

    fn special(&mut self, atx: f32, special: u8) -> Result<(), String> {
        if self.inblock {
            self.end(atx)?;
            self.list.push(Block {
                x: atx,
                len: 0.0,
                special,
            });
            self.begin(atx);
        } else {
            self.list.push(Block {
                x: atx,
                len: 0.0,
                special,
            });
        }
        Ok(())
    }
}

/// `intersect`: where the scan line at `y_val` meets edge `l`, unless a
/// crossing already handled it.
fn intersect(y_val: f32, l: &LineSeg, ips: &mut Vec<IntersectPoint>) {
    if l.intersect_hit {
        return;
    }
    let (x, y, x2, y2) = (l.x as f32, l.y as f32, l.x2 as f32, l.y2 as f32);
    if y == y_val {
        ips.push(IntersectPoint {
            x,
            y,
            x2,
            y2,
            shapenr: l.shapenr,
            is_endpoint: !l.intersect,
        });
        if y2 != y_val {
            return;
        }
        // Horizontal: it hits the second end too.
    }
    if y2 == y_val {
        ips.push(IntersectPoint {
            x: x2,
            y: y2,
            x2: x,
            y2: y,
            shapenr: l.shapenr,
            is_endpoint: !l.intersect,
        });
        return;
    }
    if between(y, y2, y_val) {
        let xi = x2 + ((l.x.wrapping_sub(l.x2)) as f32 * (y_val - y2)) / (l.y.wrapping_sub(l.y2)) as f32;
        ips.push(IntersectPoint {
            x: xi,
            y: y_val,
            x2,
            y2,
            shapenr: l.shapenr,
            is_endpoint: false,
        });
    }
}

/// `full_intersect`: where edges `nr1` and `nr2` cross, if inside both.
fn full_intersect(lines: &[LineSeg], nr1: usize, nr2: usize, fips: &mut Vec<FullIntersect>) {
    let (Some(l1), Some(l2)) = (lines.get(nr1), lines.get(nr2)) else {
        return;
    };
    let dx = l1.x.wrapping_sub(l1.x2);
    let dy = l1.y.wrapping_sub(l1.y2);
    let dxs = l2.x.wrapping_sub(l2.x2);
    let dys = l2.y.wrapping_sub(l2.y2);
    // Crossings with horizontal edges are not wanted.
    if dy == 0 || dys == 0 {
        return;
    }
    let d = i64::from(dx)
        .wrapping_mul(i64::from(dys))
        .wrapping_sub(i64::from(dxs).wrapping_mul(i64::from(dy)));
    if d == 0 {
        return;
    }
    let f = i64::from(l2.x.wrapping_sub(l1.x))
        .wrapping_mul(i64::from(dys))
        .wrapping_sub(i64::from(l2.y.wrapping_sub(l1.y)).wrapping_mul(i64::from(dxs)));
    let xs = l1.x as f32 + (i64::from(dx).wrapping_mul(f) as f32) / d as f32;
    let ys = l1.y as f32 + (i64::from(dy).wrapping_mul(f) as f32) / d as f32;
    let (x1, y1, x2, y2) = (l1.x as f32, l1.y as f32, l1.x2 as f32, l1.y2 as f32);
    let (sx1, sy1, sx2, sy2) = (l2.x as f32, l2.y as f32, l2.x2 as f32, l2.y2 as f32);
    if (between_exclude(sx1, sx2, xs) || between_exclude(sy1, sy2, ys))
        && (between_exclude(x1, x2, xs) || between_exclude(y1, y2, ys))
    {
        fips.push(FullIntersect { y: ys, x: xs, nr1, nr2 });
    }
}

/// `intersect_comp_above` (`below` reverses the slope order).
fn intersect_comp(a: &IntersectPoint, b: &IntersectPoint, above: bool) -> i32 {
    if a.x < b.x {
        return -1;
    }
    if a.x > b.x {
        return 1;
    }
    // For real crossings, a falling line comes before a rising one.
    if a.y != a.y2 && b.y != b.y2 {
        let sa = (a.x2 - a.x) / (a.y2 - a.y);
        let sb = (b.x2 - b.x) / (b.y2 - b.y);
        let s = if above { 1 } else { -1 };
        if sa > sb {
            return -s;
        }
        if sa < sb {
            return s;
        }
    }
    if a.shapenr < b.shapenr {
        return -1;
    }
    if a.shapenr > b.shapenr {
        return 1;
    }
    0
}

/// `total_inside`: whether the shallowest polygon we are in is a shape
/// (not a hole).
fn total_inside(shapes: &[ShapeRec]) -> bool {
    let mut take: Option<&ShapeRec> = None;
    for s in shapes.iter().filter(|s| s.inside) {
        if take.is_none_or(|t| s.depth < t.depth) {
            take = Some(s);
        }
    }
    take.is_some_and(|s| s.ispositiv)
}

/// The borders where the scan line crosses into and out of the figure,
/// counting the intersections with an edge on the given side.
fn borders(
    ips: &[IntersectPoint],
    shapes: &mut [ShapeRec],
    group: std::ops::RangeInclusive<usize>,
    y_val: f32,
    above: bool,
) -> Result<Vec<f32>, String> {
    for s in shapes.get_mut(group).into_iter().flatten() {
        s.inside = false;
    }
    let mut out = Vec::new();
    let mut inside = false;
    for ip in ips {
        let hit = if above { ip.y2 < y_val } else { ip.y2 > y_val };
        if !ip.is_endpoint || hit {
            if let Some(s) = shapes.get_mut(ip.shapenr) {
                s.inside = !s.inside;
            }
            let now = total_inside(shapes);
            if inside != now {
                out.push(ip.x);
                inside = now;
            }
        }
    }
    if inside {
        return Err(die(if above {
            "Outside all polygons (above) inside the shape ???"
        } else {
            "Outside all polygons (below) inside the shape ???"
        }));
    }
    Ok(out)
}

/// The blocks of one scan line from the borders above and below it.
fn scan_blocks(above: &[f32], below: &[f32]) -> Result<Scanline, String> {
    let a = |i: usize| above.get(i).copied().unwrap_or(0.0);
    let b = |i: usize| below.get(i).copied().unwrap_or(0.0);
    let (na, nb) = (above.len(), below.len());

    // Identical but for zero-length blocks and gaps: an ordinary line.
    let mut blocks = Blocks::default();
    let mut equal = true;
    let (mut ai, mut bi) = (0, 0);
    while equal && ai < na && bi < nb {
        if a(ai) == b(bi) {
            if ai % 2 == 0 {
                blocks.begin(a(ai));
            } else {
                blocks.end(a(ai))?;
            }
            ai += 1;
            bi += 1;
            continue;
        }
        if a(ai) < b(bi) {
            if ai + 1 < na && a(ai) == a(ai + 1) {
                blocks.special(a(ai), if ai % 2 == 0 { b'e' } else { b'j' })?;
                ai += 2;
                continue;
            }
            equal = false;
            break;
        }
        if a(ai) > b(bi) {
            if bi + 1 < nb && b(bi) == b(bi + 1) {
                blocks.special(b(bi), if bi % 2 == 0 { b'b' } else { b's' })?;
                bi += 2;
                continue;
            }
            equal = false;
            break;
        }
    }
    while equal && ai < na {
        if ai + 1 < na {
            if a(ai) == a(ai + 1) {
                blocks.special(a(ai), if ai % 2 == 0 { b'e' } else { b'j' })?;
                ai += 2;
                continue;
            }
            equal = false;
            break;
        }
        // fig2dev loops forever on a single border left; borders come
        // in pairs, so this is not reached.
        break;
    }
    while equal && bi < nb {
        if bi + 1 < nb {
            if b(bi) == b(bi + 1) {
                blocks.special(b(bi), if bi % 2 == 0 { b'b' } else { b's' })?;
                bi += 2;
                continue;
            }
            equal = false;
            break;
        }
        break;
    }
    if equal {
        return Ok(Scanline {
            y: 0.0,
            contains_horizontal: false,
            blocks: [blocks.list, Vec::new()],
        });
    }

    // Horizontal edges: separate lists above and below the line. When one
    // side has no borders, fig2dev reads an unset value here; it can't
    // change the result.
    let start = |x: f32, y: f32| if x < y { x } else { y };
    let first_a = if na == 0 { b(0) } else { a(0) };
    let first_b = if nb == 0 { a(0) } else { b(0) };

    let mut up = Blocks::default();
    let mut bi = 1;
    let mut last_ax = start(first_a, first_b) - 1.0;
    let mut ax = 0.0f32;
    for ai in 0..na {
        ax = a(ai);
        if ax == last_ax {
            up.special(ax, if ai % 2 == 1 { b'e' } else { b'j' })?;
            last_ax = ax;
            continue;
        }
        if ai % 2 == 1 {
            up.begin(last_ax);
        }
        while bi < nb && b(bi) < ax {
            let last_bx = b(bi - 1);
            let bx = b(bi);
            if between(last_ax, ax, last_bx) && between(last_ax, ax, bx) && last_bx != bx {
                if bi % 2 == 1 && ai % 2 == 0 {
                    up.special(last_bx, b'b')?;
                }
                if bi % 2 == 0 && ai % 2 == 1 {
                    up.special(bx, b's')?;
                }
            }
            bi += 1;
        }
        if ai % 2 == 1 {
            up.end(ax)?;
        }
        last_ax = ax;
    }
    while bi < nb {
        let last_bx = b(bi - 1);
        let bx = b(bi);
        if last_bx != bx && (last_bx > ax || na == 0) && bi % 2 == 1 {
            up.special(last_bx, b'b')?;
        }
        bi += 1;
    }

    let mut down = Blocks::default();
    let mut ai = 1;
    let mut last_bx = start(first_a, first_b) - 1.0;
    let mut bx = 0.0f32;
    for bi in 0..nb {
        bx = b(bi);
        if bx == last_bx {
            down.special(bx, if bi % 2 == 1 { b'b' } else { b's' })?;
            last_bx = bx;
            continue;
        }
        if bi % 2 == 1 {
            down.begin(last_bx);
        }
        while ai < na && a(ai) < bx {
            let last_ax = a(ai - 1);
            let ax = a(ai);
            if between(last_bx, bx, last_ax) && between(last_bx, bx, ax) && last_ax != ax {
                if ai % 2 == 1 && bi % 2 == 0 {
                    down.special(ax, b'e')?;
                }
                if ai % 2 == 0 && bi % 2 == 1 {
                    down.special(last_ax, b'j')?;
                }
            }
            ai += 1;
        }
        if bi % 2 == 1 {
            down.end(bx)?;
        }
        last_bx = bx;
    }
    while ai < na {
        let last_ax = a(ai - 1);
        let ax = a(ai);
        if last_ax != ax && (last_ax > bx || nb == 0) && ai % 2 == 1 {
            down.special(ax, b'e')?;
        }
        ai += 1;
    }
    Ok(Scanline {
        y: 0.0,
        contains_horizontal: true,
        blocks: [up.list, down.list],
    })
}

impl Shape {
    fn xzoom(ctx: &Ctx, x: f64) -> i32 {
        round(ctx.mag * (x - f64::from(ctx.llx)) / FSCALE)
    }

    fn yzoom(ctx: &Ctx, y: f64) -> i32 {
        round(ctx.mag * (y - f64::from(ctx.lly)) / FSCALE)
    }

    /// `add_object`: start a shape for comment `groupid`.
    fn add_object(&mut self, groupid: &str, lstart: usize, depth: i32) -> Result<(), String> {
        let ispositiv = match groupid.as_bytes().first() {
            None => return Err(die("Programming Error: no groupname specified")),
            Some(b'+') => true,
            Some(b'-') => false,
            Some(_) => return Err(die("Ill-formed shape comment - start with '+' or '-'")),
        };
        self.current = ShapeRec {
            ispositiv,
            inside: false,
            linestart: lstart as i64,
            lineend: 0,
            depth,
            groupname: groupid.get(1..).unwrap_or("").to_string(),
        };
        Ok(())
    }

    fn finish_object(&mut self, lend: i64) {
        let mut s = std::mem::take(&mut self.current);
        s.lineend = lend;
        self.shapes.push(s);
    }

    /// `add_point`: an edge, joined to the last one if both are horizontal.
    fn add_point(&mut self, x: i32, y: i32, x2: i32, y2: i32, intersect: bool) {
        let shapenr = self.shapes.len();
        if self.was_horizontal && y == y2 {
            if let Some(last) = self.points.last_mut() {
                if last.x2 == x && last.y2 == y && last.shapenr == shapenr {
                    last.x2 = x2;
                    last.y2 = y2;
                    return;
                }
            }
        }
        self.points.push(LineSeg {
            x,
            y,
            intersect,
            intersect_hit: false,
            shapenr,
            x2,
            y2,
        });
        self.was_horizontal = y == y2;
    }

    fn start_line(&mut self, groupid: &str, depth: i32) -> Result<(), String> {
        self.add_object(groupid, self.points.len(), depth)?;
        self.line_start = true;
        Ok(())
    }

    fn line_to(&mut self, x: i32, y: i32) {
        if self.line_start {
            self.startx = x;
            self.lastx = x;
            self.starty = y;
            self.lasty = y;
            self.line_start = false;
        } else {
            self.add_point(self.lastx, self.lasty, x, y, false);
            self.lastx = x;
            self.lasty = y;
        }
    }

    fn finish_line(&mut self) {
        if self.lastx != self.startx || self.lasty != self.starty {
            self.add_point(self.lastx, self.lasty, self.startx, self.starty, false);
        }
        self.finish_object(self.points.len() as i64 - 1);
        self.line_start = true;
    }

    /// `print_shape`.
    fn print_shape(&self, out: &mut Out, scanlines: &[Scanline]) {
        let name = self.macroname.as_deref().unwrap_or("newshape");
        fprintf!(
            out,
            "\\def\\{name}par#1{{\\shapepar{{\\{name}shape}}#1\\par}}\n\\def\\{name}shape{{%\n"
        );
        // The extent, from the first block lists (as fig2dev computes it:
        // the first block counts by its start only).
        let mut ext: Option<(f32, f32)> = None;
        for b in scanlines.iter().flat_map(|s| s.blocks[0].iter()) {
            ext = Some(match ext {
                None => (b.x, b.x),
                Some((mut xmin, mut xmax)) => {
                    if xmin > b.x {
                        xmin = b.x;
                    }
                    if xmax < b.x + b.len {
                        xmax = b.x + b.len;
                    }
                    (xmin, xmax)
                }
            });
        }
        // Without blocks fig2dev reads them unset; its output then shows
        // "nan", as NaN gives.
        let (xmin, xmax) = ext.unwrap_or((f32::NAN, f32::NAN));
        let mut lastscale: f32 = 0.1;
        if xmax != xmin {
            lastscale = (50.0 / f64::from(xmax - xmin)) as f32;
        }
        if self.scaleset {
            lastscale = (10.0 / self.scaledim) as f32;
        }
        let centerpos = if self.centerset {
            self.centerpos
        } else {
            f64::from((xmax + xmin) / 2.0)
        };
        fprintf!(out, "{{{}}}%\n", fmt_g(centerpos * f64::from(lastscale)));
        let mut firstline = true;
        for s in scanlines {
            let n = if s.contains_horizontal { 2 } else { 1 };
            for list in s.blocks.iter().take(n) {
                if list.is_empty() {
                    continue;
                }
                if firstline {
                    firstline = false;
                } else {
                    out.puts("\\\\%\n");
                }
                fprintf!(out, "{{{}}}", fmt_g(f64::from(s.y * lastscale)));
                for b in list {
                    match b.special {
                        b's' | b'j' => fprintf!(out, "{}", char::from(b.special)),
                        b'e' | b'b' => fprintf!(
                            out,
                            "{}{{{}}}",
                            char::from(b.special),
                            fmt_g(f64::from(b.x * lastscale))
                        ),
                        _ => fprintf!(
                            out,
                            "t{{{}}}{{{}}}",
                            fmt_g(f64::from(b.x * lastscale)),
                            fmt_g(f64::from(b.len * lastscale))
                        ),
                    }
                }
            }
        }
        out.puts("%\n}\n");
    }
}

impl Driver for Shape {
    fn option(&mut self, _ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        match opt {
            'n' => self.macroname = Some(arg.unwrap_or("").to_string()),
            'G' | 'L' => {}
            _ => return Err(format!("Argument -{opt} unknown to shape driver.")),
        }
        Ok(())
    }

    /// The figure's first comment names the macros, unless `-n` did.
    fn start(&mut self, _ctx: &mut Ctx, _out: &mut Out, objects: &Compound) -> Result<(), String> {
        if let Some(c) = objects.comments.first() {
            if self.macroname.is_none() {
                self.macroname = Some(c.clone());
            }
        }
        Ok(())
    }

    fn arc(&mut self, ctx: &mut Ctx, _out: &mut Out, a: &Arc) -> Result<(), String> {
        let comm = get_comment(ctx, &a.comments)?;
        if comm.is_empty() {
            return Ok(());
        }
        let cx = Self::xzoom(ctx, a.center.x);
        let cy = Self::yzoom(ctx, a.center.y);
        let sx = Self::xzoom(ctx, a.point[0].x.into());
        let sy = Self::yzoom(ctx, a.point[0].y.into());
        let ex = Self::xzoom(ctx, a.point[2].x.into());
        let ey = Self::yzoom(ctx, a.point[2].y.into());
        let angle = |y: i32, x: i32| f64::from(y.wrapping_sub(cy)).atan2(f64::from(x.wrapping_sub(cx)));
        let (sa, mut ea) = if a.direction == 0 {
            (angle(sy, sx), angle(ey, ex))
        } else {
            (angle(ey, ex), angle(sy, sx))
        };
        if ea < sa {
            ea += 2.0 * PI;
        }
        let (dx, dy) = (sx.wrapping_sub(cx), sy.wrapping_sub(cy));
        let r = f64::from(dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy))).sqrt();
        self.start_line(&comm, a.depth)?;
        if a.type_ == T_PIE_WEDGE_ARC {
            self.line_to(cx, cy);
        }
        let (cxf, cyf) = (f64::from(cx), f64::from(cy));
        let mut alpha = sa;
        while alpha < ea - ARC_STEP / 4.0 {
            self.line_to(round(cxf + r * alpha.cos()), round(cyf + r * alpha.sin()));
            alpha += ARC_STEP;
        }
        self.line_to(round(cxf + r * ea.cos()), round(cyf + r * ea.sin()));
        self.finish_line();
        Ok(())
    }

    fn ellipse(&mut self, ctx: &mut Ctx, _out: &mut Out, e: &Ellipse) -> Result<(), String> {
        let comm = get_comment(ctx, &e.comments)?;
        if comm.is_empty() {
            return Ok(());
        }
        let x0 = f64::from(Self::xzoom(ctx, e.center.x.into()));
        let y0 = f64::from(Self::yzoom(ctx, e.center.y.into()));
        let rx = ctx.mag * f64::from(e.radiuses.x) / FSCALE;
        let ry = ctx.mag * f64::from(e.radiuses.y) / FSCALE;
        let angle = -e.angle;
        self.start_line(&comm, e.depth)?;
        let mut theta = ARC_STEP / 2.0;
        while theta < 2.0 * PI + ARC_STEP / 2.0 {
            self.line_to(
                round(x0 + angle.cos() * rx * theta.cos() - angle.sin() * ry * theta.sin()),
                round(y0 + angle.sin() * rx * theta.cos() + angle.cos() * ry * theta.sin()),
            );
            theta += ARC_STEP;
        }
        self.finish_line();
        Ok(())
    }

    fn line(&mut self, ctx: &mut Ctx, _out: &mut Out, l: &Line) -> Result<(), String> {
        let comm = get_comment(ctx, &l.comments)?;
        if comm.is_empty() {
            return Ok(());
        }
        let Some(&first) = l.points.first() else {
            return Ok(());
        };
        let xs = || l.points.iter().map(|p| Self::xzoom(ctx, p.x.into()));
        if widthref(&comm) {
            // The reference width; reference objects are no shapes.
            let (xmin, xmax) = (xs().min().unwrap_or(0), xs().max().unwrap_or(0));
            if xmin != xmax {
                self.scaleset = true;
                self.scaledim = f64::from(xmax.wrapping_sub(xmin));
            } else {
                ctx.warn("fig2dev(shape) Warning: Width of reference object=0 - ignored ");
            }
            return Ok(());
        }
        if heightref(&comm) {
            let ys = || l.points.iter().map(|p| Self::yzoom(ctx, p.y.into()));
            let (ymin, ymax) = (ys().min().unwrap_or(0), ys().max().unwrap_or(0));
            if ymin != ymax {
                self.scaleset = true;
                self.scaledim = f64::from(ymax.wrapping_sub(ymin));
            } else {
                ctx.warn("fig2dev(shape) Warning: Height of reference object=0 - ignored ");
            }
            return Ok(());
        }
        if centerref(&comm) {
            let (xmin, xmax) = (xs().min().unwrap_or(0), xs().max().unwrap_or(0));
            self.centerset = true;
            // Integer division, as in fig2dev.
            self.centerpos = f64::from(xmin.wrapping_add(xmax) / 2);
            return Ok(());
        }
        match l.type_ {
            T_BOX | T_PIC_BOX => {
                self.start_line(&comm, l.depth)?;
                for p in &l.points {
                    self.line_to(Self::xzoom(ctx, p.x.into()), Self::yzoom(ctx, p.y.into()));
                }
                self.finish_line();
            }
            T_ARC_BOX => {
                self.start_line(&comm, l.depth)?;
                let (mut xmin, mut xmax, mut ymin, mut ymax) = (first.x, first.x, first.y, first.y);
                for p in l.points.iter().skip(1) {
                    xmin = xmin.min(p.x);
                    xmax = xmax.max(p.x);
                    ymin = ymin.min(p.y);
                    ymax = ymax.max(p.y);
                }
                // ZZOOM: the radius is divided as an integer first.
                let mut r = round(ctx.mag * f64::from(l.radius / FSCALE as i32));
                let xmin = Self::xzoom(ctx, xmin.into());
                let xmax = Self::xzoom(ctx, xmax.into());
                let ymin = Self::yzoom(ctx, ymin.into());
                let ymax = Self::yzoom(ctx, ymax.into());
                // The corner radius is at most half a side.
                r = r.min(xmax.wrapping_sub(xmin) / 2);
                r = r.min(ymax.wrapping_sub(ymin) / 2);
                let rf = f64::from(r);
                let mut theta = 0.0f64;
                let mut corner = |s: &mut Shape, x: i32, y: i32, fin: f64| {
                    while theta < fin {
                        s.line_to(
                            round(f64::from(x) + rf * theta.cos()),
                            round(f64::from(y) + rf * theta.sin()),
                        );
                        theta += ARC_STEP;
                    }
                };
                let (xr, xl) = (xmax.wrapping_sub(r), xmin.wrapping_add(r));
                let (yb, yt) = (ymax.wrapping_sub(r), ymin.wrapping_add(r));
                corner(self, xr, yb, PI / 2.0);
                corner(self, xl, yb, PI);
                corner(self, xl, yt, 3.0 * PI / 2.0);
                corner(self, xr, yt, 2.0 * PI);
                self.finish_line();
            }
            T_POLYLINE | T_POLYGON => {
                self.start_line(&comm, l.depth)?;
                // A polygon's last point repeats its first.
                let n = if l.type_ == T_POLYLINE {
                    l.points.len()
                } else {
                    l.points.len().saturating_sub(1)
                };
                let (mut last_x, mut last_y) = (0i32, 0i32);
                for (i, p) in l.points.iter().take(n).enumerate() {
                    let x = Self::xzoom(ctx, p.x.into());
                    let y = Self::yzoom(ctx, p.y.into());
                    if i == 0
                        || MIN_DIST < last_x.wrapping_sub(x).wrapping_abs()
                        || MIN_DIST < last_y.wrapping_sub(y).wrapping_abs()
                    {
                        self.line_to(x, y);
                        last_x = x;
                        last_y = y;
                    }
                }
                self.finish_line();
            }
            _ => {}
        }
        Ok(())
    }

    fn spline(&mut self, _ctx: &mut Ctx, _out: &mut Out, _s: &Spline) -> Result<(), String> {
        Ok(())
    }

    fn text(&mut self, ctx: &mut Ctx, _out: &mut Out, t: &Text) -> Result<(), String> {
        if !get_comment(ctx, &t.comments)?.is_empty() {
            ctx.warn("fig2dev(shape): TEXT cannot be used as shape");
        }
        Ok(())
    }

    /// Everything happens here: the scan lines of each group, then the
    /// macros.
    fn end(&mut self, _ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        if self.points.is_empty() || self.shapes.is_empty() {
            return Err(die("No shape found - have you forgotten to set comments '+'?"));
        }
        // Sort the shapes by group name and renumber their edges.
        qsort(
            &mut self.shapes,
            &mut |a: &ShapeRec, b: &ShapeRec| match a.groupname.cmp(&b.groupname) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            },
        );
        for (nr, s) in self.shapes.iter().enumerate() {
            for p in self.points.get_mut(s.lines()).into_iter().flatten() {
                p.shapenr = nr;
            }
        }
        // The groups: runs of shapes with the same name.
        let mut groups: Vec<(usize, usize)> = Vec::new();
        let mut gstart = 0;
        for (i, s) in self.shapes.iter().enumerate() {
            let start_name = self.shapes.get(gstart).map(|g| g.groupname.as_str());
            if start_name != Some(s.groupname.as_str()) {
                groups.push((gstart, i - 1));
                gstart = i;
            }
        }
        // fig2dev adds the last group even without shapes.
        groups.push((gstart, self.shapes.len().wrapping_sub(1)));

        let mut scanlines: Vec<Scanline> = Vec::new();
        let mut fips: Vec<FullIntersect> = Vec::new();
        for &(gs, ge) in &groups {
            let in_group = |nr: usize| gs <= nr && nr <= ge;
            // The crossings between polygons of the group (a polygon may
            // not cross itself).
            fips.clear();
            for s1 in gs..=ge {
                for s2 in s1 + 1..=ge {
                    let (Some(a), Some(b)) = (self.shapes.get(s1), self.shapes.get(s2)) else {
                        continue;
                    };
                    for nr1 in a.lines() {
                        for nr2 in b.lines() {
                            full_intersect(&self.points, nr1, nr2, &mut fips);
                        }
                    }
                }
            }
            // The distinct y values of vertices and crossings.
            let mut ys: Vec<f32> = self
                .points
                .iter()
                .filter(|p| in_group(p.shapenr))
                .map(|p| p.y as f32)
                .chain(fips.iter().map(|f| f.y))
                .collect();
            qsort(&mut ys, &mut |a: &f32, b: &f32| {
                if a < b {
                    -1
                } else if a > b {
                    1
                } else {
                    0
                }
            });
            ys.dedup_by(|a, b| a == b);

            for &y_val in &ys {
                let mut ips: Vec<IntersectPoint> = Vec::new();
                // Crossings first, then the edges they don't cover.
                for f in &fips {
                    for nr in [f.nr1, f.nr2] {
                        if let Some(p) = self.points.get_mut(nr) {
                            p.intersect_hit = false;
                        }
                    }
                }
                for f in &fips {
                    if f.y != y_val {
                        continue;
                    }
                    for nr in [f.nr1, f.nr2] {
                        if let Some(p) = self.points.get_mut(nr) {
                            ips.push(IntersectPoint {
                                x: f.x,
                                y: f.y,
                                x2: p.x2 as f32,
                                y2: p.y2 as f32,
                                shapenr: p.shapenr,
                                is_endpoint: false,
                            });
                            p.intersect_hit = true;
                        }
                    }
                }
                for p in self.points.iter().filter(|p| in_group(p.shapenr)) {
                    intersect(y_val, p, &mut ips);
                }

                qsort(&mut ips, &mut |a: &IntersectPoint, b: &IntersectPoint| {
                    intersect_comp(a, b, true)
                });
                let above = borders(&ips, &mut self.shapes, gs..=ge, y_val, true)?;
                qsort(&mut ips, &mut |a: &IntersectPoint, b: &IntersectPoint| {
                    intersect_comp(a, b, false)
                });
                let below = borders(&ips, &mut self.shapes, gs..=ge, y_val, false)?;

                let mut line = scan_blocks(&above, &below)?;
                line.y = y_val;
                scanlines.push(line);
            }
        }
        self.print_shape(out, &scanlines);
        Ok(0)
    }

    fn text_include(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ties end up where macOS's `qsort` puts them (recorded there).
    #[test]
    fn qsort_as_macos() {
        let keys = [3, 3, 1, 2, 0, 4, 3, 2, 3, 5];
        let mut v: Vec<(i32, usize)> = keys.iter().copied().zip(0..).collect();
        qsort(&mut v, &mut |a: &(i32, usize), b: &(i32, usize)| a.0 - b.0);
        let ids: Vec<usize> = v.iter().map(|e| e.1).collect();
        assert_eq!(ids, [4, 2, 3, 7, 8, 1, 0, 6, 5, 9]);
        // Large and presorted input goes through the heapsort fallback.
        let mut w: Vec<i32> = (0..500).rev().map(|i| i / 2).collect();
        qsort(&mut w, &mut |a: &i32, b: &i32| a - b);
        assert!(w.windows(2).all(|p| p[0] <= p[1]));
    }

    #[test]
    fn percent_g() {
        assert_eq!(fmt_g(28.59375), "28.5938");
        assert_eq!(fmt_g(0.416_666_7), "0.416667");
        assert_eq!(fmt_g(50.0), "50");
        assert_eq!(fmt_g(f64::NAN), "nan");
        assert_eq!(fmt_g(1.5e-5), "1.5e-05");
    }
}
