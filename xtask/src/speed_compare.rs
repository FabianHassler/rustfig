//! `cargo xtask speed-compare`: time the real xfig 3.2.9a and RustFig on the same figures (open, zoom,
//! pan, move, save, undo) and write the tables into `docs/speed-compare.md`. Not part of `ci`.
//!
//! xfig runs headless on its own `Xvfb` display (default `:78`, started with `-fbdir` so the
//! frame buffer is a file we can read cheaply); the harness speaks the X protocol itself for input
//! (XTEST fake events over the display's unix socket, so no `xdotool` start-up latency is in the
//! timings) and decides that xfig has "finished drawing" when the canvas pixels have stopped
//! changing and xfig's CPU use has dropped to zero. RustFig is driven through its JSON control port
//! (`--background`, own settings and recovery folders) and is "done" when `ui.state` reports the
//! canvas up to date (`canvas_pending: false`, no queued input) for a quiet period. Both quiet
//! periods are subtracted: the value is the time of the last change (xfig) or of the first idle
//! poll (RustFig), counted from the moment the action was sent.
//!
//! Usage: `cargo run --release -p xtask -- speed-compare [--rustfig BIN] [--label TEXT]
//! [--runs N] [--timeout SECONDS] [--filter a,b] [--examples DIR] [--out FILE] [--no-xfig]
//! [--no-rustfig] [--shots DIR] [--display :78] [--port 7990] [--xfig-geometry 1322x818]`.
//! Run it as a release build: a debug build is too slow to hash frame buffers while polling.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::FileExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::xfig_compare::{have, json_str, numbers_after, sh, sleep_ms, XF_CX, XF_CY};

/// Centre of an editing-mode button in the 818-pixel-high window (xfig squeezes the mode panel:
/// the first editing row is at y 351, rows 32.8 px apart, two columns at x 19 and 56).
fn xfig_mode_xy(_draw: bool, idx: usize) -> (i32, i32) {
    (
        if idx.is_multiple_of(2) { 19 } else { 56 },
        (351.0 + (idx / 2) as f64 * 32.8).round() as i32,
    )
}

/// Xvfb screen size (the harness reads its frame buffer file, so the size must match).
const SCREEN_W: usize = 1400;
const SCREEN_H: usize = 1000;
/// The pixels of xfig's canvas that are watched (inside the 1128x686 canvas of the default
/// geometry), and its message panel (the text changes when a figure is saved).
const CANVAS: Rect = Rect {
    x: 80,
    y: 80,
    w: 1100,
    h: 640,
};
const MSG: Rect = Rect {
    x: 2,
    y: 26,
    w: 1040,
    h: 20,
};
/// A move is 30 steps over this many pixels.
const MOVE_DX: f64 = 240.0;
const MOVE_DY: f64 = 120.0;
const MOVE_STEPS: usize = 30;
/// Pan distance in pixels (about half the canvas width).
const PAN_PX: i32 = 500;
/// Above this many objects a figure is glued into one compound before the move test.
const GLUE_ABOVE: usize = 300;

#[derive(Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

struct Opts {
    rustfig: PathBuf,
    label: String,
    runs: usize,
    timeout: f64,
    filter: Vec<String>,
    examples: PathBuf,
    out: PathBuf,
    work: PathBuf,
    xfig: bool,
    rf: bool,
    shots: Option<PathBuf>,
    display: String,
    port: u16,
    xfig_bin: String,
    xfig_geom: String,
}

/// One measurement: milliseconds, a timeout (seconds waited) or a reason it is missing.
#[derive(Clone, Debug)]
enum Val {
    Ms(f64),
    Timeout(f64),
    Na(String),
}

/// (key, label) of every measured task, in table order.
const TASKS: &[(&str, &str)] = &[
    (
        "open",
        "Open (xfig: main() to figure drawn; RustFig: process start to drawn)",
    ),
    ("open_read", "Open, part 1: reading the file (xfig only)"),
    ("open_draw", "Open, part 2: first full canvas redraw (xfig only)"),
    ("zoom_in", "Zoom in (one step)"),
    ("zoom_out", "Zoom out (one step)"),
    ("pan", "Pan half a screen"),
    ("move_step", "Move: one step of the drag (median of 30)"),
    ("move_step_max", "Move: slowest step"),
    ("move_place", "Move: place (final redraw)"),
    ("save", "Save"),
    ("undo", "Undo the move"),
];

type Trial = BTreeMap<&'static str, Val>;

/// Keys under which the external (pixel-watching) value of xfig is kept next to the internal one.
const EXT_KEYS: &[(&str, &str)] = &[
    ("open", "x_open"),
    ("zoom_in", "x_zoom_in"),
    ("zoom_out", "x_zoom_out"),
    ("pan", "x_pan"),
    ("move_step", "x_move_step"),
    ("move_place", "x_move_place"),
    ("save", "x_save"),
    ("undo", "x_undo"),
];

/// A cluster of spans: start, end, the spans.
type Cluster<'a> = (f64, f64, Vec<(&'a str, f64, f64)>);

/// Spans printed by the instrumented xfig (`XT name start duration`, milliseconds).
fn parse_xt(log: &str) -> BTreeMap<&'static str, f64> {
    let mut ev: Vec<(&str, f64, f64)> = Vec::new();
    for l in log.lines() {
        let f: Vec<&str> = l.split_whitespace().collect();
        if let ["XT", name, a, b] = f.as_slice() {
            if let (Ok(a), Ok(b)) = (a.parse::<f64>(), b.parse::<f64>()) {
                ev.push((name, a, b));
            }
        }
    }
    ev.sort_by(|x, y| x.1.total_cmp(&y.1));
    // Clusters of spans closer than 250 ms: one user action (a zoom is a "zoom" span with the
    // grid set-up and the canvas redraws inside, or after it).
    let mut clusters: Vec<Cluster> = Vec::new();
    for e in &ev {
        if e.0 == "main_start" || e.0 == "move_step" {
            continue;
        }
        match clusters.last_mut() {
            Some(c) if e.1 - c.1 <= 250.0 => {
                c.1 = c.1.max(e.1 + e.2);
                c.2.push(*e);
            }
            _ => clusters.push((e.1, e.1 + e.2, vec![*e])),
        }
    }
    let mut out = BTreeMap::new();
    let main_start = ev.iter().find(|e| e.0 == "main_start").map(|e| e.1);
    if let Some(c) = clusters.iter().find(|c| c.2.iter().any(|e| e.0 == "read_fig")) {
        if let Some(m) = main_start {
            out.insert("open", c.1 - m);
        }
        if let Some(r) = c.2.iter().find(|e| e.0 == "read_fig") {
            out.insert("open_read", r.2);
            out.insert("open_draw", c.1 - (r.1 + r.2));
        }
    }
    // Everything below happens after the figure was drawn.
    let open_end = clusters
        .iter()
        .find(|c| c.2.iter().any(|e| e.0 == "read_fig"))
        .map_or(0.0, |c| c.1);
    clusters.retain(|c| c.0 > open_end);
    let first = |c: &&Cluster| c.2.first().map(|e| e.0.to_string()).unwrap_or_default();
    let zooms: Vec<_> = clusters.iter().filter(|c| first(c) == "zoom").collect();
    if let Some(c) = zooms.first() {
        out.insert("zoom_in", c.1 - c.0);
    }
    if let Some(c) = zooms.get(1) {
        out.insert("zoom_out", c.1 - c.0);
    }
    if let Some(c) = clusters.iter().find(|c| first(c) == "setup_grid") {
        out.insert("pan", c.1 - c.0);
    }
    if let Some(c) = clusters.iter().find(|c| first(c) == "place") {
        out.insert("move_place", c.1 - c.0);
    }
    if let Some(c) = clusters.iter().find(|c| first(c) == "undo") {
        out.insert("undo", c.1 - c.0);
    }
    if let Some(c) = clusters.iter().find(|c| first(c) == "save") {
        out.insert("save", c.1 - c.0);
    }
    let mut steps: Vec<f64> = ev.iter().filter(|e| e.0 == "move_step").map(|e| e.2).collect();
    if steps.len() >= MOVE_STEPS / 2 {
        steps.sort_by(f64::total_cmp);
        out.insert("move_step", steps[steps.len() / 2]);
        out.insert("move_step_max", steps[steps.len() - 1]);
    }
    out
}

// ---------------------------------------------------------------- the figures

struct FigSrc {
    name: String,
    path: PathBuf,
    gz: bool,
    /// Why this figure is in the set.
    kind: &'static str,
}

struct FigInfo {
    objects: usize,
    max_x: i64,
    max_y: i64,
    /// Vertices of visible polylines (pen not white, thickness above zero): points to click on.
    cands: Vec<(i64, i64)>,
    /// Any polyline vertex and ellipse side: used when no visible polyline vertex exists.
    weak: Vec<(i64, i64)>,
}

fn fig_sources(o: &Opts) -> Vec<FigSrc> {
    let big = [
        ("pines", "pines/pines.fig.gz"),
        ("snowy", "snowy/snowy.fig.gz"),
        ("wenig", "wenig/wenig.fig.gz"),
        ("gosper-curve", "gosper-curve/gosper-curve.fig.gz"),
        ("penrose-tiling", "penrose-tiling/penrose-tiling.fig.gz"),
        ("apollonian-gasket", "apollonian-gasket/apollonian-gasket.fig.gz"),
        ("blankmap", "blankmap/blankmap.fig.gz"),
        ("ne-coastline-10m", "ne-coast/ne-coastline-10m.fig.gz"),
        ("ne-countries-10m", "ne-countries/ne-countries-10m.fig.gz"),
        ("greece", "greece/greece.fig.gz"),
        ("clifford-attractor", "clifford-attractor/clifford-attractor.fig.gz"),
        ("cube8", "cube8/cube8.fig.gz"),
    ];
    let mut v: Vec<FigSrc> = Vec::new();
    for f in ["carnot", "feynman", "fourier"] {
        v.push(FigSrc {
            name: format!("{f} (small)"),
            path: o.examples.join(format!("gallery/{f}.fig")),
            gz: false,
            kind: "small",
        });
    }
    for (n, p) in big {
        v.push(FigSrc {
            name: n.to_string(),
            path: o.examples.join(p),
            gz: true,
            kind: "large",
        });
    }
    v.retain(|f| o.filter.is_empty() || o.filter.iter().any(|s| f.name.contains(s.as_str())));
    v
}

/// Plain copy of the figure (and the pictures next to it) in `dir`; returns the file.
fn stage_fig(src: &FigSrc, dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let stem = src.name.split(' ').next().unwrap_or("fig");
    let dst = dir.join(format!("{stem}.fig"));
    if src.gz {
        let out = Command::new("gzip")
            .args(["-dc"])
            .arg(&src.path)
            .output()
            .map_err(|e| format!("gzip: {e}"))?;
        if !out.status.success() {
            return Err(format!("gzip -dc {} failed", src.path.display()));
        }
        std::fs::write(&dst, out.stdout).map_err(|e| e.to_string())?;
    } else {
        std::fs::copy(&src.path, &dst).map_err(|e| format!("{}: {e}", src.path.display()))?;
    }
    if let Some(parent) = src.path.parent() {
        if let Ok(rd) = std::fs::read_dir(parent) {
            for e in rd.flatten() {
                let p = e.path();
                let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
                // Pictures a figure may reference (greece-1.png); not the preview PNG of the figure itself.
                if matches!(ext, "png" | "jpg")
                    && p.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.contains("-1"))
                {
                    if let Some(n) = p.file_name() {
                        let _ = std::fs::copy(&p, dir.join(n));
                    }
                }
            }
        }
    }
    Ok(dst)
}

fn parse_fig(text: &str) -> FigInfo {
    let mut info = FigInfo {
        objects: 0,
        max_x: 0,
        max_y: 0,
        cands: Vec::new(),
        weak: Vec::new(),
    };
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    let grow = |x: i64, y: i64, info: &mut FigInfo| {
        info.max_x = info.max_x.max(x);
        info.max_y = info.max_y.max(y);
    };
    while i < lines.len() {
        let l = lines[i];
        i += 1;
        if l.starts_with('\t') || l.starts_with(' ') {
            let t: Vec<i64> = l.split_whitespace().filter_map(|w| w.parse().ok()).collect();
            if t.len() == l.split_whitespace().count() && t.len().is_multiple_of(2) {
                for p in t.chunks(2) {
                    if let [x, y] = p {
                        grow(*x, *y, &mut info);
                    }
                }
            }
            continue;
        }
        let f: Vec<&str> = l.split_whitespace().collect();
        let code: i32 = f.first().and_then(|c| c.parse().ok()).unwrap_or(0);
        if !(1..=5).contains(&code) {
            continue;
        }
        info.objects += 1;
        if code == 2 {
            let n = |k: usize| f.get(k).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
            let (sub, thick, pen) = (n(1), n(3), n(4));
            let arrows = (n(13) != 0) as usize + (n(14) != 0) as usize + (sub == 5) as usize;
            let want = usize::try_from(n(15)).unwrap_or(0);
            let mut got = 0;
            let mut j = i + arrows;
            while got < want && j < lines.len() {
                let t: Vec<i64> = lines[j].split_whitespace().filter_map(|w| w.parse().ok()).collect();
                for p in t.chunks(2) {
                    if let [x, y] = p {
                        got += 1;
                        if thick > 0 && pen != 7 {
                            info.cands.push((*x, *y));
                        } else if info.weak.len() < 100_000 {
                            info.weak.push((*x, *y));
                        }
                    }
                }
                j += 1;
            }
        } else if code == 4 {
            let n = |k: usize| f.get(k).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
            grow(n(11), n(12), &mut info);
        } else if code == 1 {
            let n = |k: usize| f.get(k).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
            grow(n(12) + n(14), n(13) + n(15), &mut info);
            if info.weak.len() < 100_000 {
                // The right-hand side of the outline.
                info.weak.push((n(12) + n(14), n(13)));
            }
        }
    }
    info
}

/// xfig's zoom after `n` steps out from 1 (sqrt 2 each).
fn zoom_after(n: u32) -> f64 {
    2f64.powf(-f64::from(n) / 2.0)
}

/// Zoom-out steps from 1 so that the whole figure fits a canvas of `cw` x `ch` pixels.
fn steps_to_fit(info: &FigInfo, cw: f64, ch: f64) -> u32 {
    for n in 0..12 {
        let z = zoom_after(n);
        if info.max_x as f64 * z / 15.0 <= cw - 12.0 && info.max_y as f64 * z / 15.0 <= ch - 12.0 {
            return n;
        }
    }
    12
}

/// The vertex nearest the canvas centre at zoom `z` (origin 0), in Fig units.
fn pick_point(info: &FigInfo, z: f64, cw: f64, ch: f64) -> Option<(i64, i64)> {
    let (cx, cy) = (cw / 2.0 * 15.0 / z, ch / 2.0 * 15.0 / z);
    let list = if info.cands.is_empty() { &info.weak } else { &info.cands };
    list.iter()
        .filter(|(x, y)| (*x as f64) * z / 15.0 < cw - 4.0 && (*y as f64) * z / 15.0 < ch - 4.0)
        .min_by(|a, b| {
            let d = |p: &&(i64, i64)| (p.0 as f64 - cx).powi(2) + (p.1 as f64 - cy).powi(2);
            d(a).total_cmp(&d(b))
        })
        .copied()
}

// ---------------------------------------------------------------- X11 (XTEST) client

struct X11 {
    s: UnixStream,
    xtest: u8,
    root: u32,
    kmin: u8,
    kmax: u8,
}

impl X11 {
    fn connect(display: &str) -> Result<X11, String> {
        let num = display.trim_start_matches(':').split('.').next().unwrap_or("0");
        let mut s = UnixStream::connect(format!("/tmp/.X11-unix/X{num}")).map_err(|e| format!("X socket: {e}"))?;
        s.write_all(&[b'l', 0, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0])
            .map_err(|e| e.to_string())?;
        let mut h = [0u8; 8];
        s.read_exact(&mut h).map_err(|e| e.to_string())?;
        if h[0] != 1 {
            return Err("X server refused the connection (needs no authorisation)".into());
        }
        let mut d = vec![0u8; usize::from(u16::from_le_bytes([h[6], h[7]])) * 4];
        s.read_exact(&mut d).map_err(|e| e.to_string())?;
        let vlen = usize::from(u16::from_le_bytes([d[16], d[17]]));
        let nfmt = usize::from(d[21]);
        let off = 32 + vlen.div_ceil(4) * 4 + nfmt * 8;
        let root = u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]]);
        let (kmin, kmax) = (d[26], d[27]);
        // QueryExtension XTEST
        let name = b"XTEST";
        let mut q = vec![98u8, 0, 4, 0, name.len() as u8, 0, 0, 0];
        q.extend_from_slice(name);
        q.extend_from_slice(&[0, 0, 0]);
        s.write_all(&q).map_err(|e| e.to_string())?;
        let mut r = [0u8; 32];
        s.read_exact(&mut r).map_err(|e| e.to_string())?;
        if r[0] != 1 || r[8] == 0 {
            return Err("X server has no XTEST extension".into());
        }
        Ok(X11 {
            s,
            xtest: r[9],
            root,
            kmin,
            kmax,
        })
    }

    fn fake(&mut self, kind: u8, detail: u8, x: i32, y: i32) -> Result<(), String> {
        let mut m = [0u8; 36];
        m[0] = self.xtest;
        m[1] = 2;
        m[2] = 9;
        m[4] = kind;
        m[5] = detail;
        m[12..16].copy_from_slice(&self.root.to_le_bytes());
        m[24..26].copy_from_slice(&(x as i16).to_le_bytes());
        m[26..28].copy_from_slice(&(y as i16).to_le_bytes());
        self.s.write_all(&m).map_err(|e| e.to_string())
    }
    fn motion(&mut self, x: i32, y: i32) -> Result<(), String> {
        self.fake(6, 0, x, y)
    }
    fn button(&mut self, b: u8, down: bool) -> Result<(), String> {
        self.fake(if down { 4 } else { 5 }, b, 0, 0)
    }
    fn key(&mut self, code: u8, down: bool) -> Result<(), String> {
        self.fake(if down { 2 } else { 3 }, code, 0, 0)
    }
    fn click(&mut self, b: u8) -> Result<(), String> {
        self.button(b, true)?;
        sleep_ms(40);
        self.button(b, false)
    }
    /// Keycode whose first keysym is `sym`.
    fn keycode(&mut self, sym: u32) -> Result<u8, String> {
        let count = self.kmax - self.kmin + 1;
        let q = [101u8, 0, 2, 0, self.kmin, count, 0, 0];
        self.s.write_all(&q).map_err(|e| e.to_string())?;
        let mut h = [0u8; 32];
        self.s.read_exact(&mut h).map_err(|e| e.to_string())?;
        let per = usize::from(h[1]);
        let words = u32::from_le_bytes([h[4], h[5], h[6], h[7]]) as usize;
        let mut d = vec![0u8; words * 4];
        self.s.read_exact(&mut d).map_err(|e| e.to_string())?;
        for k in 0..usize::from(count) {
            let o = k * per * 4;
            if let Some(b) = d.get(o..o + 4) {
                if u32::from_le_bytes([b[0], b[1], b[2], b[3]]) == sym {
                    return Ok(self.kmin + k as u8);
                }
            }
        }
        Err(format!("no key for keysym {sym:#x}"))
    }
    /// Move the pointer in small steps (xfig reacts to motion events, not to jumps).
    fn glide(&mut self, from: (i32, i32), to: (i32, i32), steps: i32) -> Result<(), String> {
        for k in 1..=steps {
            let p = |a: i32, b: i32| a + (b - a) * k / steps;
            self.motion(p(from.0, to.0), p(from.1, to.1))?;
            sleep_ms(4);
        }
        Ok(())
    }
}

struct Keys {
    z: u8,
    shift: u8,
    alt: u8,
    s: u8,
    u: u8,
    ctrl: u8,
}

// ---------------------------------------------------------------- frame buffer watching

/// Xvfb's frame buffer file (XWD: a header, then 32-bit pixels).
struct Fb {
    f: File,
    off: u64,
    buf: RefCell<Vec<u8>>,
}

impl Fb {
    fn open(path: &Path) -> Result<Fb, String> {
        let f = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let len = f.metadata().map_err(|e| e.to_string())?.len();
        let pix = (SCREEN_W * SCREEN_H * 4) as u64;
        if len < pix {
            return Err("frame buffer file is smaller than the screen".into());
        }
        Ok(Fb {
            f,
            off: len - pix,
            buf: RefCell::new(Vec::new()),
        })
    }

    fn hash(&self, r: Rect) -> u64 {
        let stride = SCREEN_W * 4;
        let start = self.off + (r.y * stride) as u64;
        let mut buf = self.buf.borrow_mut();
        buf.resize(r.h * stride, 0);
        if self.f.read_exact_at(&mut buf, start).is_err() {
            return 0;
        }
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for row in buf.chunks_exact(stride) {
            if let Some(px) = row.get(r.x * 4..(r.x + r.w) * 4) {
                for w in px.as_chunks::<8>().0 {
                    let v = u64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]]);
                    h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3).rotate_left(17);
                }
            }
        }
        h
    }
}

/// Accumulated CPU time of a process in milliseconds (`ps`, 10 ms resolution).
fn cpu_ms(pid: u32) -> Option<f64> {
    let o = Command::new("ps")
        .args(["-o", "cputime=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let t = String::from_utf8_lossy(&o.stdout);
    let t = t.trim();
    if t.is_empty() {
        return None;
    }
    let mut secs = 0.0;
    for part in t.split(':') {
        secs = secs * 60.0 + part.parse::<f64>().ok()?;
    }
    Some(secs * 1000.0)
}

enum Settled {
    /// Milliseconds from the reference time to the last change of the pixels.
    Done(f64),
    NoChange,
    Timeout,
}

struct Watch<'a> {
    fb: &'a Fb,
    rect: Rect,
    pid: u32,
    /// The pixels before the action, so that its first change counts.
    base: u64,
    quiet: Duration,
    /// How long to wait for a first change before reporting none.
    first_change: Duration,
    limit: Duration,
}

fn settle(w: &Watch, t_ref: Instant) -> Settled {
    let mut last_hash = w.base;
    let mut last_change: Option<Instant> = None;
    let mut cpu_last = cpu_ms(w.pid);
    let mut cpu_busy = t_ref;
    let mut cpu_at = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(2));
        let now = Instant::now();
        let h = w.fb.hash(w.rect);
        if h != last_hash {
            last_hash = h;
            last_change = Some(now);
        }
        if now.duration_since(cpu_at) >= Duration::from_millis(100) {
            let c = cpu_ms(w.pid);
            if c != cpu_last {
                cpu_busy = now;
            }
            cpu_last = c;
            cpu_at = now;
        }
        let idle_since = last_change.unwrap_or(t_ref).max(cpu_busy);
        if now.duration_since(idle_since) >= w.quiet {
            if let Some(lc) = last_change {
                return Settled::Done(lc.duration_since(t_ref).as_secs_f64() * 1000.0);
            }
            if now.duration_since(t_ref) >= w.first_change {
                return Settled::NoChange;
            }
        }
        if now.duration_since(t_ref) >= w.limit {
            return Settled::Timeout;
        }
    }
}

fn val_of(s: Settled, limit: Duration, what: &str) -> Val {
    match s {
        Settled::Done(ms) => Val::Ms(ms),
        Settled::NoChange => Val::Na(format!("{what}: no visible change")),
        Settled::Timeout => Val::Timeout(limit.as_secs_f64()),
    }
}

// ---------------------------------------------------------------- xfig

struct XEnv {
    x: X11,
    fb: Fb,
    k: Keys,
}

fn kill(c: &mut Child) {
    let _ = c.kill();
    let _ = c.wait();
}

fn xfig_canvas_size(o: &Opts) -> (f64, f64) {
    let (w, h) = o.xfig_geom.split_once('x').unwrap_or(("1322", "818"));
    let (w, h): (f64, f64) = (w.parse().unwrap_or(1322.0), h.parse().unwrap_or(818.0));
    (w - 194.0, h - 132.0)
}

fn trial_xfig(o: &Opts, env: &mut XEnv, src: &FigSrc, info: &FigInfo, run: usize) -> Result<Trial, String> {
    let r = trial_xfig_inner(o, env, src, info, run);
    let _ = std::fs::remove_dir_all(
        o.work
            .join("xfig")
            .join(format!("{}-{run}", src.name.split(' ').next().unwrap_or("f")))
            .join("fig"),
    );
    r
}

fn trial_xfig_inner(o: &Opts, env: &mut XEnv, src: &FigSrc, info: &FigInfo, run: usize) -> Result<Trial, String> {
    let dir = o
        .work
        .join("xfig")
        .join(format!("{}-{run}", src.name.split(' ').next().unwrap_or("f")));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("home")).map_err(|e| e.to_string())?;
    let fig = stage_fig(src, &dir.join("fig"))?;
    let lim = Duration::from_secs_f64(o.timeout);
    let mut t = Trial::new();
    sleep_ms(700);
    let base = env.fb.hash(CANVAS);
    let t0 = Instant::now();
    let mut child = Command::new(&o.xfig_bin)
        .args(["-geometry", &format!("{}+0+0", o.xfig_geom)])
        .arg(&fig)
        .current_dir(fig.parent().unwrap_or(&dir))
        .env("DISPLAY", &o.display)
        .env("HOME", dir.join("home"))
        .env("XFIG_TIMING", "1")
        .stdout(Stdio::null())
        .stderr(File::create(dir.join("xt.log")).map_err(|e| e.to_string())?)
        .spawn()
        .map_err(|e| format!("start xfig: {e}"))?;
    let pid = child.id();
    let w = Watch {
        fb: &env.fb,
        rect: CANVAS,
        pid,
        base,
        quiet: Duration::from_millis(500),
        first_change: lim,
        limit: lim,
    };
    let open = settle(&w, t0);
    let ok = matches!(open, Settled::Done(_));
    t.insert("open", val_of(open, lim, "open"));
    let result = if ok {
        xfig_tasks(o, env, info, pid, &mut t, run)
    } else {
        Ok(())
    };
    if !ok {
        for (k, _) in TASKS.iter().skip(1) {
            t.insert(k, Val::Na("open did not finish".into()));
        }
    }
    kill(&mut child);
    // The instrumented xfig's own spans replace the pixel-watching values (kept as `x_*`).
    let log = std::fs::read_to_string(dir.join("xt.log")).unwrap_or_default();
    let internal = parse_xt(&log);
    for (k, xk) in EXT_KEYS {
        if let Some(v) = t.get(k).cloned() {
            t.insert(xk, v);
        }
    }
    for (k, v) in internal {
        t.insert(k, Val::Ms(v));
    }
    if let Err(e) = &result {
        eprintln!("   xfig: {e}");
        for (k, _) in TASKS {
            t.entry(k).or_insert_with(|| Val::Na(e.clone()));
        }
    }
    Ok(t)
}

/// Time `act` (after `pre`) until the canvas (or `rect`) has been still for `quiet`.
#[allow(clippy::too_many_arguments)]
fn xmeasure(
    o: &Opts,
    env: &mut XEnv,
    pid: u32,
    rect: Rect,
    quiet_ms: u64,
    what: &str,
    pre: impl FnOnce(&mut X11) -> Result<(), String>,
    act: impl FnOnce(&mut X11) -> Result<(), String>,
) -> Result<Val, String> {
    let lim = Duration::from_secs_f64(o.timeout);
    pre(&mut env.x)?;
    sleep_ms(150);
    let base = env.fb.hash(rect);
    let t = Instant::now();
    act(&mut env.x)?;
    let w = Watch {
        fb: &env.fb,
        rect,
        pid,
        base,
        quiet: Duration::from_millis(quiet_ms),
        first_change: Duration::from_millis(1500),
        limit: lim,
    };
    Ok(val_of(settle(&w, t), lim, what))
}

fn key_chord(x: &mut X11, mods: &[u8], key: u8) -> Result<(), String> {
    for m in mods {
        x.key(*m, true)?;
    }
    x.key(key, true)?;
    sleep_ms(20);
    x.key(key, false)?;
    for m in mods.iter().rev() {
        x.key(*m, false)?;
    }
    Ok(())
}

fn shot_xfig(o: &Opts, name: &str) {
    if let Some(d) = &o.shots {
        let _ = std::fs::create_dir_all(d);
        let _ = sh(
            &o.display,
            &format!("xwd -root | xwdtopnm | pnmtopng > '{}'", d.join(name).display()),
        );
    }
}

fn xfig_tasks(o: &Opts, env: &mut XEnv, info: &FigInfo, pid: u32, t: &mut Trial, run: usize) -> Result<(), String> {
    let (cw, ch) = xfig_canvas_size(o);
    let centre = (76 + (cw / 2.0) as i32, 76 + (ch / 2.0) as i32);
    let corner = (80, 80);
    let (z, shift, alt) = (env.k.z, env.k.shift, env.k.alt);
    let (ks, ku) = (env.k.s, env.k.u);
    env.x.motion(centre.0, centre.1)?;
    sleep_ms(200);
    let v = xmeasure(
        o,
        env,
        pid,
        CANVAS,
        400,
        "zoom in",
        |_| Ok(()),
        |x| key_chord(x, &[shift], z),
    )?;
    t.insert("zoom_in", v);
    let v = xmeasure(
        o,
        env,
        pid,
        CANVAS,
        400,
        "zoom out",
        |_| Ok(()),
        |x| key_chord(x, &[], z),
    )?;
    t.insert("zoom_out", v);
    // Pan: middle button dragged along the top ruler; the canvas redraws when the button is released.
    let v = xmeasure(
        o,
        env,
        pid,
        CANVAS,
        400,
        "pan",
        |x| {
            x.glide(centre, (800, 62), 4)?;
            x.button(2, true)?;
            x.glide((800, 62), (800 - PAN_PX, 62), 25)
        },
        |x| x.button(2, false),
    )?;
    t.insert("pan", v);
    // Back to the origin: Ctrl + middle button on the canvas ("pan to origin").
    env.x.motion(centre.0, centre.1)?;
    sleep_ms(100);
    let ctrl = env.k.ctrl;
    env.x.key(ctrl, true)?;
    env.x.click(2)?;
    env.x.key(ctrl, false)?;
    sleep_ms(700);

    // Move test.
    let n = steps_to_fit(info, cw, ch);
    let zoom = zoom_after(n);
    let heavy = info.objects > GLUE_ABOVE;
    env.x.motion(corner.0, corner.1)?;
    sleep_ms(150);
    for _ in 0..n {
        let r = xmeasure(
            o,
            env,
            pid,
            CANVAS,
            400,
            "zoom to fit",
            |_| Ok(()),
            |x| key_chord(x, &[], z),
        )?;
        if !matches!(r, Val::Ms(_)) {
            for k in ["move_step", "move_step_max", "move_place", "save", "undo"] {
                t.insert(k, Val::Na("zooming out for the move did not finish".into()));
            }
            return Ok(());
        }
    }
    if heavy {
        let (gx, gy) = xfig_mode_xy(false, 0);
        let (mx, my) = xfig_mode_xy(false, 8);
        env.x.glide(corner, (gx, gy), 6)?;
        env.x.click(1)?;
        sleep_ms(200);
        let (x1, y1) = (corner.0, corner.1);
        let (x2, y2) = (76 + cw as i32 - 6, 76 + ch as i32 - 6);
        env.x.glide((gx, gy), (x1, y1), 6)?;
        env.x.button(2, true)?;
        env.x.glide((x1, y1), (x2, y2), 12)?;
        let lim = Duration::from_secs_f64(o.timeout);
        env.x.button(2, false)?;
        let base = env.fb.hash(CANVAS);
        let _ = base;
        // Tagging redraws every glued object in xfig's highlight colour: wait for it.
        let w = Watch {
            fb: &env.fb,
            rect: CANVAS,
            pid,
            base: 0,
            quiet: Duration::from_millis(1000),
            first_change: Duration::from_millis(1500),
            limit: lim,
        };
        if matches!(settle(&w, Instant::now()), Settled::Timeout) {
            return Err("xfig: tagging the figure took longer than the timeout".into());
        }
        env.x.glide((x2, y2), (centre.0, centre.1), 6)?;
        env.x.click(3)?;
        let w = Watch {
            fb: &env.fb,
            rect: CANVAS,
            pid,
            base: env.fb.hash(CANVAS),
            quiet: Duration::from_millis(1000),
            first_change: Duration::from_millis(1500),
            limit: lim,
        };
        if matches!(settle(&w, Instant::now()), Settled::Timeout) {
            return Err("xfig: gluing the figure took longer than the timeout".into());
        }
        env.x.glide(centre, (mx, my), 6)?;
        env.x.click(1)?;
    } else {
        let (mx, my) = xfig_mode_xy(false, 8);
        env.x.glide(corner, (mx, my), 6)?;
        env.x.click(1)?;
    }
    sleep_ms(300);
    let Some((px, py)) = pick_point(info, zoom, cw, ch) else {
        return Err("no polyline vertex to click on".into());
    };
    let start = (XF_CX + px as f64 * zoom / 15.0, XF_CY + py as f64 * zoom / 15.0);
    let start = (start.0.round() as i32, start.1.round() as i32);
    let (mx, my) = xfig_mode_xy(false, 8);
    env.x.glide((mx, my), start, 8)?;
    sleep_ms(200);
    eprintln!("   pick vertex {px},{py} at pixel {start:?}, zoom {zoom}, {n} zoom-outs");
    env.x.click(1)?;
    sleep_ms(300);
    shot_xfig(o, &format!("xfig-{run}-after-pick.png"));
    // 30 steps with the object held (xfig draws its outline in XOR mode).
    let mut steps: Vec<f64> = Vec::new();
    let mut lost = 0;
    for k in 1..=MOVE_STEPS {
        let to = (
            start.0 + (MOVE_DX * k as f64 / MOVE_STEPS as f64).round() as i32,
            start.1 + (MOVE_DY * k as f64 / MOVE_STEPS as f64).round() as i32,
        );
        let base = env.fb.hash(CANVAS);
        let t1 = Instant::now();
        env.x.motion(to.0, to.1)?;
        let w = Watch {
            fb: &env.fb,
            rect: CANVAS,
            pid,
            base,
            quiet: Duration::from_millis(150),
            first_change: Duration::from_millis(1200),
            limit: Duration::from_secs_f64(o.timeout),
        };
        match settle(&w, t1) {
            Settled::Done(ms) => steps.push(ms),
            Settled::NoChange => lost += 1,
            Settled::Timeout => {
                t.insert("move_step", Val::Timeout(o.timeout));
                t.insert("move_step_max", Val::Timeout(o.timeout));
                for k in ["move_place", "save", "undo"] {
                    t.insert(k, Val::Na("a move step timed out".into()));
                }
                return Ok(());
            }
        }
        if k == MOVE_STEPS / 2 {
            shot_xfig(o, &format!("xfig-{run}-mid-move.png"));
        }
    }
    if steps.len() < MOVE_STEPS / 2 {
        t.insert(
            "move_step",
            Val::Na(format!(
                "the display did not change in {lost} of 30 steps (object not picked?)"
            )),
        );
        t.insert("move_step_max", Val::Na("-".into()));
        for k in ["move_place", "save", "undo"] {
            t.insert(k, Val::Na("move did not start".into()));
        }
        return Ok(());
    }
    steps.sort_by(f64::total_cmp);
    t.insert("move_step", Val::Ms(steps[steps.len() / 2]));
    t.insert("move_step_max", Val::Ms(steps[steps.len() - 1]));
    let v = xmeasure(o, env, pid, CANVAS, 500, "place", |_| Ok(()), |x| x.click(1))?;
    t.insert("move_place", v);
    shot_xfig(o, &format!("xfig-{run}-placed.png"));
    // Save (Alt-S) is seen in the message panel; undo (Alt-U) on the canvas.
    env.x.motion(centre.0, centre.1)?;
    let v = xmeasure(o, env, pid, MSG, 500, "save", |_| Ok(()), |x| key_chord(x, &[alt], ks))?;
    t.insert("save", v);
    let v = xmeasure(
        o,
        env,
        pid,
        CANVAS,
        500,
        "undo",
        |_| Ok(()),
        |x| key_chord(x, &[alt], ku),
    )?;
    t.insert("undo", v);
    Ok(())
}

// ---------------------------------------------------------------- RustFig

struct Rf {
    child: Child,
    stream: TcpStream,
    reader: BufReader<TcpStream>,
    id: u64,
    canvas: (f64, f64),
    origin: (f64, f64),
}

impl Drop for Rf {
    fn drop(&mut self) {
        kill(&mut self.child);
    }
}

struct Idle {
    wall: Val,
    busy: f64,
}

#[derive(Default)]
struct RfState {
    pending: bool,
    input_pending: bool,
    frame_ms: f64,
    max_recent: f64,
    render_ms: f64,
}

impl Rf {
    fn call(&mut self, method: &str, params: &str) -> Result<String, String> {
        self.id += 1;
        let line = format!("{{\"id\":{},\"method\":\"{method}\",\"params\":{params}}}\n", self.id);
        self.stream.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        let mut reply = String::new();
        self.reader
            .read_line(&mut reply)
            .map_err(|e| format!("{method}: {e}"))?;
        if reply.contains("\"error\"") {
            return Err(format!(
                "{method} {params}: {}",
                reply.trim().chars().take(200).collect::<String>()
            ));
        }
        Ok(reply)
    }

    fn state(&mut self) -> Result<RfState, String> {
        let s = self.call("ui.state", "{}")?;
        let num = |key: &str| -> f64 {
            s.find(key)
                .and_then(|i| s.get(i + key.len()..))
                .and_then(|r| r.split([',', '}']).next())
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(0.0)
        };
        let flag = |key: &str, default: bool| -> bool {
            s.find(key)
                .and_then(|i| s.get(i + key.len()..))
                .map_or(default, |r| r.trim_start().starts_with("true"))
        };
        if let Some(o) = numbers_after(&s, "\"origin\":[", 2) {
            self.origin = (o[0], o[1]);
        }
        if let Some(i) = s.find("\"label\":\"Canvas:") {
            if let Some(r) = s.get(i..).and_then(|t| numbers_after(t, "\"rect\":[", 2)) {
                self.canvas = (r[0], r[1]);
            }
        }
        Ok(RfState {
            // Older builds have no `canvas_pending`: then the caller falls back to a fixed wait.
            pending: flag("\"canvas_pending\":", false),
            input_pending: flag("\"input_pending\":", false),
            frame_ms: num("\"frame_ms\":"),
            max_recent: num("\"frame_ms_max_recent\":"),
            render_ms: num("\"render_ms\":"),
        })
    }

    /// Wait until the canvas is current and no input is queued for `quiet`; the time (ms since
    /// `t_ref`) of the first idle poll of that quiet period.
    fn idle(&mut self, t_ref: Instant, quiet_ms: u64, limit: Duration, has_pending: bool) -> Result<Val, String> {
        self.idle2(t_ref, quiet_ms, limit, has_pending, false).map(|i| i.wall)
    }

    /// Like `idle`, and RustFig's own work: the sum of its frame times (`frame_ms`, read by the
    /// polls: each poll sees the frame before it) from the action to the first idle poll, plus the
    /// time of a background render if one ran. The wall time cannot be shorter than one frame
    /// interval (16.7 ms at 60 Hz); the frame times can.
    fn idle2(
        &mut self,
        t_ref: Instant,
        quiet_ms: u64,
        limit: Duration,
        has_pending: bool,
        use_max: bool,
    ) -> Result<Idle, String> {
        let mut since: Option<Instant> = None;
        let (mut busy, mut counting, mut pend_polls) = (0.0, true, 0);
        loop {
            let st = self.state()?;
            let now = Instant::now();
            if counting {
                busy += st.frame_ms;
            }
            if st.pending {
                pend_polls += 1;
            }
            if !st.pending && !st.input_pending {
                let s = *since.get_or_insert(now);
                if counting {
                    counting = false;
                    if use_max {
                        // The recent-frames window (120 frames) was flushed before the action.
                        busy = busy.max(st.max_recent);
                    }
                    if pend_polls >= 2 {
                        busy += st.render_ms;
                    }
                }
                // Builds without `canvas_pending` cannot tell: settle for a fixed 600 ms instead.
                let q = if has_pending {
                    Duration::from_millis(quiet_ms)
                } else {
                    Duration::from_millis(600)
                };
                if now.duration_since(s) >= q {
                    let wall = s.duration_since(t_ref).as_secs_f64() * 1000.0;
                    return Ok(Idle {
                        wall: Val::Ms(wall),
                        busy,
                    });
                }
            } else {
                since = None;
            }
            if now.duration_since(t_ref) >= limit {
                return Ok(Idle {
                    wall: Val::Timeout(limit.as_secs_f64()),
                    busy,
                });
            }
            std::thread::sleep(Duration::from_millis(3));
        }
    }

    fn pt(&self, x: f64, y: f64, zoom: f64) -> (f64, f64) {
        let s = zoom * 80.0 / 1200.0;
        (
            self.canvas.0 + (x - self.origin.0) * s,
            self.canvas.1 + (y - self.origin.1) * s,
        )
    }

    fn pointer(&mut self, action: &str, p: (f64, f64)) -> Result<(), String> {
        self.call(
            "ui.pointer",
            &format!(r#"{{"action":"{action}","x":{},"y":{},"button":"left"}}"#, p.0, p.1),
        )
        .map(|_| ())
    }

    /// Zoom and pan so that Fig (0, 0) is at the canvas corner, as in xfig.
    fn view_origin(&mut self, zoom: f64) -> Result<(), String> {
        self.call(
            "engine.execute",
            &format!(r#"{{"command":"view.zoom_set","params":{{"zoom":{zoom}}}}}"#),
        )?;
        self.state()?;
        self.state()?;
        let (ox, oy) = self.origin;
        let s = zoom * 80.0 / 1200.0;
        self.call(
            "engine.execute",
            &format!(
                r#"{{"command":"view.pan","params":{{"dx":{},"dy":{}}}}}"#,
                ox * s,
                oy * s
            ),
        )?;
        self.state()?;
        self.state()?;
        Ok(())
    }

    fn exec(&mut self, command: &str) -> Result<String, String> {
        self.call("engine.execute", &format!(r#"{{"command":"{command}","params":{{}}}}"#))
    }
}

fn trial_rf(o: &Opts, src: &FigSrc, info: &FigInfo, run: usize) -> Result<Trial, String> {
    let r = trial_rf_inner(o, src, info, run);
    // The staged copy of a big figure is not needed any more.
    let _ = std::fs::remove_dir_all(
        o.work
            .join("rustfig")
            .join(format!("{}-{run}", src.name.split(' ').next().unwrap_or("f")))
            .join("fig"),
    );
    r
}

fn trial_rf_inner(o: &Opts, src: &FigSrc, info: &FigInfo, run: usize) -> Result<Trial, String> {
    let dir = o
        .work
        .join("rustfig")
        .join(format!("{}-{run}", src.name.split(' ').next().unwrap_or("f")));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let fig = stage_fig(src, &dir.join("fig"))?;
    let lim = Duration::from_secs_f64(o.timeout);
    let mut t = Trial::new();
    let t0 = Instant::now();
    let child = Command::new(&o.rustfig)
        .args([
            "--control",
            &o.port.to_string(),
            "--background",
            "-geometry",
            "1400x1000+0+0",
        ])
        .arg(&fig)
        .current_dir(fig.parent().unwrap_or(&dir))
        .env("RUSTFIG_SETTINGS_DIR", dir.join("settings"))
        .env("RUSTFIG_RECOVERY_DIR", dir.join("recovery"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("start rustfig: {e}"))?;
    let stream = loop {
        if let Ok(s) = TcpStream::connect(("127.0.0.1", o.port)) {
            break s;
        }
        if t0.elapsed() > lim {
            let mut c = child;
            kill(&mut c);
            return Err("RustFig control port did not open".into());
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs_f64(o.timeout + 30.0)));
    let reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut rf = Rf {
        child,
        stream,
        reader,
        id: 0,
        canvas: (0.0, 0.0),
        origin: (0.0, 0.0),
    };
    let first = rf.state()?;
    let _ = first;
    let probe = rf.call("ui.state", "{}")?;
    let has_pending = probe.contains("canvas_pending");
    let open = rf.idle(t0, 150, lim, has_pending)?;
    let ok = matches!(open, Val::Ms(_));
    t.insert("x_open", open.clone());
    t.insert("open", open);
    if !ok {
        for (k, _) in TASKS.iter().skip(1) {
            t.insert(k, Val::Na("open did not finish".into()));
        }
        return Ok(t);
    }
    rf.view_origin(1.0)?;
    rf.idle(Instant::now(), 150, lim, has_pending)?;

    let timed = |rf: &mut Rf,
                 t: &mut Trial,
                 name: &'static str,
                 act: &dyn Fn(&mut Rf) -> Result<(), String>|
     -> Result<(), String> {
        // 130 idle frames push the earlier actions out of the window of recent frame times.
        for _ in 0..130 {
            rf.state()?;
        }
        let t1 = Instant::now();
        act(rf)?;
        let i = rf.idle2(t1, 100, lim, has_pending, true)?;
        let xk = EXT_KEYS.iter().find(|e| e.0 == name).map_or(name, |e| e.1);
        match i.wall {
            Val::Ms(_) => {
                t.insert(name, Val::Ms(i.busy));
            }
            ref w => {
                t.insert(name, w.clone());
            }
        }
        t.insert(xk, i.wall);
        Ok(())
    };
    timed(&mut rf, &mut t, "zoom_in", &|r| r.exec("view.zoom_in").map(|_| ()))?;
    timed(&mut rf, &mut t, "zoom_out", &|r| r.exec("view.zoom_out").map(|_| ()))?;
    timed(&mut rf, &mut t, "pan", &|r| {
        r.call(
            "engine.execute",
            &format!(r#"{{"command":"view.pan","params":{{"dx":{PAN_PX},"dy":0}}}}"#),
        )
        .map(|_| ())
    })?;

    // Move test (an error here keeps what was measured before it).
    let mv = (|| -> Result<(), String> {
        let (cw, ch) = (rf.canvas.0, rf.canvas.1);
        let _ = (cw, ch);
        let st = rf.state()?;
        let _ = st;
        let (cvw, cvh) = {
            let s = rf.call("ui.state", "{}")?;
            let c = numbers_after(&s, "\"canvas\":[", 2).unwrap_or(vec![1128.0, 686.0]);
            (c[0], c[1])
        };
        let n = steps_to_fit(info, cvw, cvh);
        let zoom = zoom_after(n);
        rf.view_origin(zoom)?;
        rf.idle(Instant::now(), 150, lim, has_pending)?;
        if info.objects > GLUE_ABOVE {
            rf.exec("edit.select_all")?;
            rf.exec("object.group")?;
            rf.idle(Instant::now(), 150, lim, has_pending)?;
        }
        rf.exec("tool.move")?;
        let Some((px, py)) = pick_point(info, zoom, cvw, cvh) else {
            return Err("no polyline vertex to click on".into());
        };
        let start = rf.pt(px as f64, py as f64, zoom);
        rf.pointer("move", start)?;
        rf.pointer("click", start)?;
        rf.idle(Instant::now(), 150, lim, has_pending)?;
        let mut steps: Vec<f64> = Vec::new();
        let mut walls: Vec<f64> = Vec::new();
        for k in 1..=MOVE_STEPS {
            let to = (
                start.0 + MOVE_DX * k as f64 / MOVE_STEPS as f64,
                start.1 + MOVE_DY * k as f64 / MOVE_STEPS as f64,
            );
            let t1 = Instant::now();
            rf.pointer("move", to)?;
            let i = rf.idle2(t1, 40, lim, has_pending, false)?;
            match i.wall {
                Val::Ms(w) => {
                    steps.push(i.busy);
                    walls.push(w);
                }
                other => {
                    t.insert("move_step", other);
                    break;
                }
            }
            if k == MOVE_STEPS / 2 {
                if let Some(d) = &o.shots {
                    let _ = std::fs::create_dir_all(d);
                    let _ = rf.call(
                        "ui.screenshot",
                        &format!(
                            r#"{{"path":{}}}"#,
                            json_str(&d.join(format!("rustfig-{run}-mid-move.png")).display().to_string())
                        ),
                    );
                }
            }
        }
        if !steps.is_empty() {
            steps.sort_by(f64::total_cmp);
            t.insert("move_step", Val::Ms(steps[steps.len() / 2]));
            t.insert("move_step_max", Val::Ms(steps[steps.len() - 1]));
            t.insert("x_move_step", Val::Ms(median(&mut walls)));
            let end = (start.0 + MOVE_DX, start.1 + MOVE_DY);
            timed(&mut rf, &mut t, "move_place", &|r| r.pointer("click", end))?;
            let ts = Instant::now();
            rf.exec("file.save")?;
            t.insert("save", Val::Ms(ts.elapsed().as_secs_f64() * 1000.0));
            timed(&mut rf, &mut t, "undo", &|r| r.exec("edit.undo").map(|_| ()))?;
        }
        Ok(())
    })();
    if let Err(e) = mv {
        eprintln!("   rustfig: {e}");
        for (k, _) in TASKS {
            t.entry(k).or_insert_with(|| Val::Na(e.clone()));
        }
    }
    Ok(t)
}

// ---------------------------------------------------------------- statistics and report

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n == 0 {
        0.0
    } else if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Median over the runs: a timeout in any run makes it "> T s" (the app was too slow for the budget).
fn aggregate(runs: &[Trial], key: &str) -> Val {
    let mut ms = Vec::new();
    let mut timeout = None;
    let mut na = None;
    for r in runs {
        match r.get(key) {
            Some(Val::Ms(v)) => ms.push(*v),
            Some(Val::Timeout(s)) => timeout = Some(*s),
            Some(Val::Na(w)) => na = Some(w.clone()),
            None => {}
        }
    }
    if let Some(s) = timeout {
        return Val::Timeout(s);
    }
    if ms.is_empty() {
        return Val::Na(na.unwrap_or_else(|| "not measured".into()));
    }
    Val::Ms(median(&mut ms))
}

fn fmt_ms(v: f64) -> String {
    if v >= 10_000.0 {
        format!("{:.1} s", v / 1000.0)
    } else if v >= 1000.0 {
        format!("{:.2} s", v / 1000.0)
    } else if v >= 100.0 {
        format!("{v:.0} ms")
    } else {
        format!("{v:.1} ms")
    }
}

fn fmt_val(v: &Val) -> String {
    match v {
        Val::Ms(x) => fmt_ms(*x),
        Val::Timeout(s) => format!("> {s:.0} s"),
        Val::Na(_) => "n/a".into(),
    }
}

fn ratio(x: &Val, r: &Val) -> String {
    match (x, r) {
        (Val::Ms(a), Val::Ms(b)) if *b > 0.0 => {
            let q = a / b;
            if q >= 10.0 {
                format!("{q:.0}x")
            } else {
                format!("{q:.1}x")
            }
        }
        (Val::Timeout(s), Val::Ms(b)) if *b > 0.0 => format!("> {:.0}x", s * 1000.0 / b),
        _ => "-".into(),
    }
}

struct Row {
    fig: String,
    kind: &'static str,
    objects: usize,
    bytes: usize,
    glue: bool,
    xfig: Vec<Trial>,
    rf: Vec<Trial>,
}

fn shell_out(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn render(o: &Opts, rows: &[Row]) -> String {
    let mut s = String::new();
    let mach = shell_out("sysctl", &["-n", "machdep.cpu.brand_string"]);
    let chip = if mach.is_empty() {
        shell_out(
            "sh",
            &[
                "-c",
                "system_profiler SPHardwareDataType | grep -E 'Chip|Cores' | tr -s ' ' | paste -sd ';' -",
            ],
        )
    } else {
        mach
    };
    let os = shell_out("sw_vers", &["-productVersion"]);
    let xfig_v = shell_out(&o.xfig_bin, &["-v"]);
    let rf_v = shell_out(&o.rustfig.display().to_string(), &["--version"]);
    s.push_str("### Setup of this run\n\n");
    s.push_str(&format!("- Machine: {chip}, macOS {os}\n"));
    s.push_str(&format!("- xfig: `{}` ({}), default options, scratch `HOME`, on `Xvfb {}` 1400x1000x24; window `{}` (canvas 1128x686 pixels)\n", o.xfig_bin, xfig_v.lines().next().unwrap_or("3.2.9a"), o.display, o.xfig_geom));
    s.push_str(&format!("- RustFig: `{}` ({}), release build, label: {}; `-geometry 1400x1000` (clipped to the screen), canvas 1128x686 points\n", o.rustfig.display(), rf_v.lines().next().unwrap_or("no --version"), o.label));
    s.push_str(&format!("- Each value is the median of {} run(s), a new process per run; timeout {:.0} s per measurement (\"> N s\": not finished in that time; the real time is longer)\n", o.runs, o.timeout));
    s.push_str(&format!(
        "- Generated {} by `cargo xtask speed-compare`\n\n",
        shell_out("date", &["+%Y-%m-%d %H:%M"])
    ));
    s.push_str("### Results\n\n");
    s.push_str("xfig / RustFig is the speed-up of RustFig (above 1 = RustFig is faster).\n\n");
    for (key, label) in TASKS {
        if *key == "move_step_max" {
            continue;
        }
        s.push_str(&format!(
            "#### {label}\n\n| Figure | Size | xfig | RustFig | xfig / RustFig |\n|---|---|---|---|---|\n"
        ));
        for r in rows {
            let (x, f) = (aggregate(&r.xfig, key), aggregate(&r.rf, key));
            let sz = if r.bytes >= 1_000_000 {
                format!("{:.1} MB, {} obj.", r.bytes as f64 / 1e6, r.objects)
            } else {
                format!("{} kB, {} obj.", r.bytes / 1000, r.objects)
            };
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                r.fig,
                sz,
                fmt_val(&x),
                fmt_val(&f),
                ratio(&x, &f)
            ));
        }
        s.push('\n');
    }
    s.push_str("#### Cross-check: the same actions timed from outside\n\nxfig: its own hooks against watching the canvas pixels; RustFig: its frame times against the wall time until `ui.state` reports the canvas current (never below one frame, 16.7 ms).\n\n| Figure | Task | xfig, own hooks | xfig, pixels | RustFig, frame times | RustFig, wall to idle |\n|---|---|---|---|---|---|\n");
    for r in rows {
        for (k, xk) in EXT_KEYS {
            let (a, b) = (aggregate(&r.xfig, k), aggregate(&r.xfig, xk));
            let (c, d) = (aggregate(&r.rf, k), aggregate(&r.rf, xk));
            if matches!(b, Val::Ms(_) | Val::Timeout(_)) || matches!(d, Val::Ms(_) | Val::Timeout(_)) {
                s.push_str(&format!(
                    "| {} | {k} | {} | {} | {} | {} |\n",
                    r.fig,
                    fmt_val(&a),
                    fmt_val(&b),
                    fmt_val(&c),
                    fmt_val(&d)
                ));
            }
        }
    }
    s.push('\n');
    s.push_str("<details><summary>Raw numbers (every run, milliseconds)</summary>\n\n| Figure | App | Task | Runs |\n|---|---|---|---|\n");
    for r in rows {
        for (app, runs) in [("xfig", &r.xfig), ("RustFig", &r.rf)] {
            for key in TASKS.iter().map(|t| t.0).chain(EXT_KEYS.iter().map(|e| e.1)) {
                let cells: Vec<String> = runs
                    .iter()
                    .map(|t| match t.get(key) {
                        Some(Val::Ms(v)) => format!("{v:.1}"),
                        Some(Val::Timeout(sec)) => format!(">{sec:.0}s"),
                        Some(Val::Na(w)) => format!("n/a ({w})"),
                        None => "-".into(),
                    })
                    .collect();
                if !cells.is_empty() {
                    s.push_str(&format!("| {} | {app} | {key} | {} |\n", r.fig, cells.join("; ")));
                }
            }
        }
    }
    s.push_str("\n</details>\n\n");
    s.push_str(&format!(
        "Move test: {MOVE_STEPS} steps over {MOVE_DX:.0} x {MOVE_DY:.0} pixels. Figures with more than {GLUE_ABOVE} objects ({}) were glued into one compound first (xfig: Glue, middle-button region, right button; RustFig: select all, group), smaller ones move a single object.\n",
        rows.iter().filter(|r| r.glue).map(|r| r.fig.as_str()).collect::<Vec<_>>().join(", ")
    ));
    s
}

fn write_doc(o: &Opts, body: &str) -> Result<(), String> {
    const START: &str = "<!-- results:start -->";
    const END: &str = "<!-- results:end -->";
    let old = std::fs::read_to_string(&o.out).unwrap_or_default();
    let new = match (old.find(START), old.find(END)) {
        (Some(a), Some(b)) if a < b => format!("{}{START}\n\n{body}\n{}", &old[..a], &old[b..]),
        _ => format!("# Speed comparison with xfig\n\n{START}\n\n{body}\n{END}\n"),
    };
    std::fs::write(&o.out, new).map_err(|e| format!("{}: {e}", o.out.display()))
}

// ---------------------------------------------------------------- driver

fn parse_args(root: &Path) -> Result<Opts, String> {
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let mut o = Opts {
        rustfig: target.join("release/rustfig"),
        label: String::new(),
        runs: 3,
        timeout: 120.0,
        filter: Vec::new(),
        examples: root.join("examples"),
        out: root.join("docs/speed-compare.md"),
        work: root.join("target/speed-compare"),
        xfig: true,
        rf: true,
        shots: None,
        display: ":78".into(),
        port: 7990,
        xfig_bin: std::env::var("XFIG_BIN").unwrap_or_else(|_| "/opt/homebrew/bin/xfig".into()),
        xfig_geom: "1322x818".into(),
    };
    let mut a = std::env::args().skip(2);
    while let Some(k) = a.next() {
        let mut val = || a.next().ok_or_else(|| format!("{k} needs a value"));
        match k.as_str() {
            "--rustfig" => o.rustfig = PathBuf::from(val()?),
            "--label" => o.label = val()?,
            "--runs" => o.runs = val()?.parse().map_err(|_| "--runs N")?,
            "--timeout" => o.timeout = val()?.parse().map_err(|_| "--timeout SECONDS")?,
            "--filter" => o.filter = val()?.split(',').map(str::to_string).collect(),
            "--examples" => o.examples = PathBuf::from(val()?),
            "--out" => o.out = PathBuf::from(val()?),
            "--work" => o.work = PathBuf::from(val()?),
            "--shots" => o.shots = Some(PathBuf::from(val()?)),
            "--display" => o.display = val()?,
            "--port" => o.port = val()?.parse().map_err(|_| "--port N")?,
            "--xfig-geometry" => o.xfig_geom = val()?,
            "--xfig-bin" => o.xfig_bin = val()?,
            "--no-xfig" => o.xfig = false,
            "--no-rustfig" => o.rf = false,
            other => return Err(format!("unknown option {other}")),
        }
    }
    if o.label.is_empty() {
        o.label = o.rustfig.display().to_string();
    }
    Ok(o)
}

pub fn run(root: &Path) -> Result<(), String> {
    let o = parse_args(root)?;
    let extra = ["/opt/homebrew/bin", "/opt/X11/bin"];
    for t in ["Xvfb", "xwd", "gzip"] {
        if !have(t, &extra) {
            return Err(format!("speed-compare: {t} not found"));
        }
    }
    if o.xfig && !Path::new(&o.xfig_bin).is_file() {
        return Err(format!("xfig not found at {}", o.xfig_bin));
    }
    if o.rf && !o.rustfig.is_file() {
        return Err(format!(
            "{} not found (build it with cargo build --release -p rustfig, or pass --rustfig)",
            o.rustfig.display()
        ));
    }
    let figs = fig_sources(&o);
    for f in &figs {
        if !f.path.is_file() {
            return Err(format!(
                "{} not found (--examples DIR points at the figure folders)",
                f.path.display()
            ));
        }
    }
    std::fs::create_dir_all(&o.work).map_err(|e| e.to_string())?;
    let fbdir = o.work.join("fb");
    let _ = std::fs::remove_dir_all(&fbdir);
    std::fs::create_dir_all(&fbdir).map_err(|e| e.to_string())?;
    let mut xvfb: Option<Child> = None;
    let mut env: Option<XEnv> = None;
    if o.xfig {
        let sock = format!("/tmp/.X11-unix/X{}", o.display.trim_start_matches(':'));
        if Path::new(&sock).exists() {
            if UnixStream::connect(&sock).is_ok() {
                return Err(format!("display {} is in use; pass --display", o.display));
            }
            // A stale socket of an Xvfb that was killed.
            let _ = std::fs::remove_file(&sock);
        }
        xvfb = Some(
            Command::new("Xvfb")
                .args([
                    o.display.as_str(),
                    "-screen",
                    "0",
                    &format!("{SCREEN_W}x{SCREEN_H}x24"),
                    "-fbdir",
                ])
                .arg(&fbdir)
                .env(
                    "PATH",
                    format!("/opt/X11/bin:{}", std::env::var("PATH").unwrap_or_default()),
                )
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("Xvfb: {e}"))?,
        );
        sleep_ms(2500);
        let mut x = X11::connect(&o.display)?;
        let k = Keys {
            z: x.keycode(0x7a)?,
            shift: x.keycode(0xffe1)?,
            alt: x.keycode(0xffe9)?,
            s: x.keycode(0x73)?,
            u: x.keycode(0x75)?,
            ctrl: x.keycode(0xffe3)?,
        };
        env = Some(XEnv {
            x,
            fb: Fb::open(&fbdir.join("Xvfb_screen0"))?,
            k,
        });
    }
    let result = (|| -> Result<(), String> {
        let mut rows: Vec<Row> = Vec::new();
        for src in &figs {
            let dir = o.work.join("info");
            let plain = stage_fig(src, &dir)?;
            let text = std::fs::read(&plain).map_err(|e| e.to_string())?;
            let info = parse_fig(&String::from_utf8_lossy(&text));
            eprintln!("== {} ({} objects)", src.name, info.objects);
            let mut row = Row {
                fig: src.name.clone(),
                kind: src.kind,
                objects: info.objects,
                bytes: text.len(),
                glue: info.objects > GLUE_ABOVE,
                xfig: Vec::new(),
                rf: Vec::new(),
            };
            let _ = row.kind;
            if let Some(env) = env.as_mut() {
                for run in 0..o.runs {
                    match trial_xfig(&o, env, src, &info, run) {
                        Ok(t) => {
                            let slow = matches!(t.get("open"), Some(Val::Ms(v)) if *v > 60_000.0)
                                || !matches!(t.get("open"), Some(Val::Ms(_)));
                            eprintln!("   xfig run {run}: {:?}", t.get("open"));
                            row.xfig.push(t);
                            if slow {
                                break;
                            }
                        }
                        Err(e) => {
                            eprintln!("   xfig run {run} failed: {e}");
                            let mut t = Trial::new();
                            for (k, _) in TASKS {
                                t.insert(k, Val::Na(e.clone()));
                            }
                            row.xfig.push(t);
                            break;
                        }
                    }
                }
            }
            if o.rf {
                for run in 0..o.runs {
                    match trial_rf(&o, src, &info, run) {
                        Ok(t) => {
                            eprintln!("   rustfig run {run}: {:?}", t.get("open"));
                            row.rf.push(t);
                        }
                        Err(e) => {
                            eprintln!("   rustfig run {run} failed: {e}");
                            let mut t = Trial::new();
                            for (k, _) in TASKS {
                                t.insert(k, Val::Na(e.clone()));
                            }
                            row.rf.push(t);
                            break;
                        }
                    }
                }
            }
            rows.push(row);
            // Write after every figure: a long run can be interrupted.
            write_doc(&o, &render(&o, &rows))?;
        }
        Ok(())
    })();
    if let Some(mut x) = xvfb {
        kill(&mut x);
        let _ = std::fs::remove_file(format!("/tmp/.X11-unix/X{}", o.display.trim_start_matches(':')));
    }
    result?;
    println!("speed-compare: written {}", o.out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_steps_follow_sqrt_two() {
        assert!((zoom_after(2) - 0.5).abs() < 1e-9);
        assert!((zoom_after(0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn parses_polylines_with_arrows_and_pictures() {
        let t = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 1 0 3\n\t1 1 1.00 60.00 120.00\n\t 600 600 3000 600 3000 900\n\
2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5\n\t0 pic.png\n\t 0 0 100 0 100 100 0 100 0 0\n\
4 0 0 50 -1 0 12 0.0000 4 135 300 5000 7000 Hi\\001\n";
        let i = parse_fig(t);
        assert_eq!(i.objects, 3);
        assert_eq!(i.cands.len(), 8);
        assert_eq!(i.max_x, 5000);
        assert_eq!(i.max_y, 7000);
    }

    #[test]
    fn fit_steps_grow_with_the_figure() {
        let small = FigInfo {
            objects: 1,
            max_x: 5000,
            max_y: 5000,
            cands: vec![],
            weak: vec![],
        };
        let big = FigInfo {
            objects: 1,
            max_x: 60000,
            max_y: 40000,
            cands: vec![],
            weak: vec![],
        };
        assert_eq!(steps_to_fit(&small, 1128.0, 686.0), 0);
        assert!(steps_to_fit(&big, 1128.0, 686.0) >= 4);
    }

    #[test]
    fn medians_and_timeouts() {
        let mut a = Trial::new();
        a.insert("open", Val::Ms(10.0));
        let mut b = Trial::new();
        b.insert("open", Val::Ms(30.0));
        let mut c = Trial::new();
        c.insert("open", Val::Ms(20.0));
        assert!(matches!(aggregate(&[a.clone(), b.clone(), c], "open"), Val::Ms(v) if (v - 20.0).abs() < 1e-9));
        let mut d = Trial::new();
        d.insert("open", Val::Timeout(5.0));
        assert!(matches!(aggregate(&[a, b, d], "open"), Val::Timeout(_)));
    }
}
