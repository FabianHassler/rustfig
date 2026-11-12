//! Frame renderer for the R9 screen movie: screenshots with an eased zoom, a drawn
//! pointer, click rings, mouse-indicator highlights, key badges and a tour spotlight,
//! piped as raw frames into one ffmpeg that writes the MP4 and the WebM.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tiny_skia::{
    Color, FillRule, FilterQuality, Paint, Path as SkPath, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform,
};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Btn {
    Left,
    Middle,
    Right,
}

impl Btn {
    fn index(self) -> usize {
        match self {
            Btn::Left => 0,
            Btn::Middle => 1,
            Btn::Right => 2,
        }
    }
}

/// One stretch of the movie. Times are seconds; positions and rects are window points
/// (screenshot pixels = points * Movie::ppp).
pub struct Seg {
    pub dur: f64,
    /// Screenshots shown during the segment: (fraction 0..1 of the segment from which it is shown, png path). Sorted, the first at 0.0; never empty.
    pub shots: Vec<(f64, PathBuf)>,
    /// Pointer at the start and end; moved with ease-in-out over the segment. None: no pointer drawn.
    pub from: Option<(f64, f64)>,
    pub to: Option<(f64, f64)>,
    /// Button held during the whole segment: a filled dot under the pointer tip, and that button's box in the mouse indicator highlighted.
    pub held: Option<Btn>,
    /// A click happens at the END of this segment with this button: an expanding, fading ring at `to` over the next 0.5 s, and the button's box highlighted for 0.35 s from the click.
    pub click: Option<Btn>,
    /// Keys shown as a keycap badge near the pointer (or bottom center when no pointer) while the segment runs.
    pub keys: Option<String>,
    /// The view at the END of the segment: a crop rect of the window in points (None = whole window). Eased from the previous segment's end view.
    pub view: Option<[f64; 4]>,
    /// Tour spotlight at the END of the segment: everything outside dimmed, a red rounded outline round it; eased from the previous segment's spot.
    pub spot: Option<[f64; 4]>,
}

pub struct Movie {
    pub segs: Vec<Seg>,
    /// Screenshot pixels per point (e.g. 2.0 on Retina).
    pub ppp: f64,
    /// Window size in points.
    pub win: (f64, f64),
    /// The mouse indicator's three button boxes (left, middle, right) in points.
    pub buttons: [[f64; 4]; 3],
    /// Captions: (start s, end s, text). Written as WebVTT, not burned in.
    pub cues: Vec<(f64, f64, String)>,
    /// Output width in pixels (even); height follows the window aspect, rounded to even.
    pub width: u32,
    pub fps: u32,
}

const DIM_ALPHA: f32 = 0.55;
const RING_SECS: f64 = 0.5;
const HIGHLIGHT_SECS: f64 = 0.35;
const MAX_CACHED_SHOTS: usize = 3;

// ---------------------------------------------------------------- pure helpers

/// Ease-in-out on 0..1 (clamped).
pub(crate) fn ease(t: f64) -> f64 {
    let t = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 };
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn lerp_rect(a: [f64; 4], b: [f64; 4], t: f64) -> [f64; 4] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
        lerp(a[3], b[3], t),
    ]
}

/// Grow `r` (x0, y0, x1, y1) around its center to `aspect` (w/h), shrink it to fit the
/// window `win` and shift it inside.
fn fix_aspect(r: [f64; 4], aspect: f64, win: (f64, f64)) -> [f64; 4] {
    let whole = [0.0, 0.0, win.0, win.1];
    if !(aspect.is_finite() && aspect > 0.0) || r.iter().any(|v| !v.is_finite()) {
        return whole;
    }
    let (mut w, mut h) = ((r[2] - r[0]).abs().max(1e-6), (r[3] - r[1]).abs().max(1e-6));
    if w / h < aspect {
        w = h * aspect;
    } else {
        h = w / aspect;
    }
    if w > win.0 {
        w = win.0;
        h = w / aspect;
    }
    if h > win.1 {
        h = win.1;
        w = h * aspect;
    }
    let cx = (r[0] + r[2]) / 2.0;
    let cy = (r[1] + r[3]) / 2.0;
    let x0 = (cx - w / 2.0).clamp(0.0, (win.0 - w).max(0.0));
    let y0 = (cy - h / 2.0).clamp(0.0, (win.1 - h).max(0.0));
    [x0, y0, x0 + w, y0 + h]
}

/// Segment index and fraction (0..1) at time `t`; past the end: the last segment at 1.
fn locate(durs: &[f64], t: f64) -> Option<(usize, f64)> {
    let mut start = 0.0;
    for (i, &d) in durs.iter().enumerate() {
        if t < start + d {
            return Some((
                i,
                if d > 0.0 {
                    ((t - start) / d).clamp(0.0, 1.0)
                } else {
                    1.0
                },
            ));
        }
        start += d;
    }
    if durs.is_empty() {
        None
    } else {
        Some((durs.len() - 1, 1.0))
    }
}

fn vtt_time(t: f64) -> String {
    let ms = if t.is_finite() && t > 0.0 {
        (t * 1000.0).round() as u64
    } else {
        0
    };
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

/// The WebVTT text of the cues.
pub fn vtt(cues: &[(f64, f64, String)]) -> String {
    let mut o = String::from("WEBVTT\n\n");
    for (i, (a, b, text)) in cues.iter().enumerate() {
        o.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            vtt_time(*a),
            vtt_time(*b),
            text
        ));
    }
    o
}

// ------------------------------------------------------------------- the plan

struct Plan {
    durs: Vec<f64>,
    starts: Vec<f64>,
    total: f64,
    /// Aspect-fixed view at the end of each segment.
    views: Vec<[f64; 4]>,
    spots: Vec<Option<[f64; 4]>>,
    out_w: u32,
    out_h: u32,
}

fn make_plan(m: &Movie) -> Result<Plan, String> {
    if m.segs.is_empty() {
        return Err("movie has no segments".into());
    }
    if !(m.win.0 > 0.0 && m.win.1 > 0.0 && m.ppp > 0.0) {
        return Err("bad window size or ppp".into());
    }
    if m.fps == 0 || m.width < 2 {
        return Err("bad fps or width".into());
    }
    let out_w = m.width & !1;
    let out_h = ((f64::from(out_w) * m.win.1 / m.win.0 / 2.0).round() as u32).max(1) * 2;
    let aspect = f64::from(out_w) / f64::from(out_h);
    let whole = [0.0, 0.0, m.win.0, m.win.1];
    let mut starts = Vec::new();
    let mut durs = Vec::new();
    let mut t = 0.0;
    for s in &m.segs {
        if s.shots.is_empty() {
            return Err("segment without screenshots".into());
        }
        let d = if s.dur.is_finite() { s.dur.max(0.0) } else { 0.0 };
        starts.push(t);
        durs.push(d);
        t += d;
    }
    let views = m
        .segs
        .iter()
        .map(|s| fix_aspect(s.view.unwrap_or(whole), aspect, m.win))
        .collect();
    let spots = m.segs.iter().map(|s| s.spot).collect();
    Ok(Plan {
        durs,
        starts,
        total: t,
        views,
        spots,
        out_w,
        out_h,
    })
}

#[derive(Clone, PartialEq, Debug)]
struct FrameState {
    shot: (usize, usize),
    view: [f64; 4],
    pointer: Option<(f64, f64)>,
    held: Option<Btn>,
    /// Ring centers (points) with progress 0..1.
    rings: Vec<(f64, f64, f64)>,
    highlight: [bool; 3],
    spot: Option<([f64; 4], f64)>,
    keys: Option<String>,
}

fn state_at(m: &Movie, p: &Plan, t: f64) -> Option<FrameState> {
    let (i, f) = locate(&p.durs, t)?;
    let seg = m.segs.get(i)?;
    let whole = [0.0, 0.0, m.win.0, m.win.1];
    let e = ease(f);
    let prev_view = if i == 0 { whole } else { *p.views.get(i - 1)? };
    let view = lerp_rect(prev_view, *p.views.get(i)?, e);
    let prev_spot = if i == 0 { None } else { *p.spots.get(i - 1)? };
    let spot = match (prev_spot, *p.spots.get(i)?) {
        (Some(a), Some(b)) => Some((lerp_rect(a, b, e), 1.0)),
        (None, Some(b)) => Some((b, e)),
        (Some(a), None) => Some((a, 1.0 - e)),
        (None, None) => None,
    }
    .filter(|(_, a)| *a > 0.0);
    let pointer = match (seg.from.or(seg.to), seg.to.or(seg.from)) {
        (Some(a), Some(b)) => Some((lerp(a.0, b.0, e), lerp(a.1, b.1, e))),
        _ => None,
    };
    let shot = seg.shots.iter().rposition(|(from, _)| *from <= f + 1e-9).unwrap_or(0);
    let mut rings = Vec::new();
    let mut highlight = [false; 3];
    if let Some(b) = seg.held {
        highlight[b.index()] = true;
    }
    for (j, s) in m.segs.iter().enumerate() {
        let (Some(b), Some(&st), Some(&d)) = (s.click, p.starts.get(j), p.durs.get(j)) else {
            continue;
        };
        let dt = t - (st + d);
        if dt < -1e-9 {
            continue;
        }
        if dt < HIGHLIGHT_SECS {
            highlight[b.index()] = true;
        }
        if dt < RING_SECS {
            if let Some(c) = s.to.or(s.from) {
                rings.push((c.0, c.1, dt.max(0.0) / RING_SECS));
            }
        }
    }
    Some(FrameState {
        shot: (i, shot),
        view,
        pointer,
        held: seg.held,
        rings,
        highlight,
        spot,
        keys: seg.keys.clone(),
    })
}

// -------------------------------------------------------------------- drawing

fn paint_rgba(r: u8, g: u8, b: u8, a: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(Color::from_rgba8(r, g, b, (a.clamp(0.0, 1.0) * 255.0).round() as u8));
    p.anti_alias = true;
    p
}

fn rrect_path(r: [f32; 4], rad: f32) -> Option<SkPath> {
    let (x0, y0, x1, y1) = (r[0], r[1], r[2], r[3]);
    let rad = rad.min((x1 - x0) / 2.0).min((y1 - y0) / 2.0).max(0.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x0 + rad, y0);
    pb.line_to(x1 - rad, y0);
    pb.quad_to(x1, y0, x1, y0 + rad);
    pb.line_to(x1, y1 - rad);
    pb.quad_to(x1, y1, x1 - rad, y1);
    pb.line_to(x0 + rad, y1);
    pb.quad_to(x0, y1, x0, y1 - rad);
    pb.line_to(x0, y0 + rad);
    pb.quad_to(x0, y0, x0 + rad, y0);
    pb.close();
    pb.finish()
}

/// Points -> output pixels for one view.
struct Map {
    x0: f64,
    y0: f64,
    s: f64,
}

impl Map {
    fn pt(&self, x: f64, y: f64) -> (f32, f32) {
        (((x - self.x0) * self.s) as f32, ((y - self.y0) * self.s) as f32)
    }
    fn rect(&self, r: [f64; 4]) -> [f32; 4] {
        let a = self.pt(r[0], r[1]);
        let b = self.pt(r[2], r[3]);
        [a.0, a.1, b.0, b.1]
    }
}

struct Renderer<'a> {
    m: &'a Movie,
    p: &'a Plan,
    cache: Vec<(PathBuf, Pixmap)>,
    badges: HashMap<String, Pixmap>,
}

impl Renderer<'_> {
    fn shot(&mut self, path: &Path) -> Result<&Pixmap, String> {
        if let Some(pos) = self.cache.iter().position(|(p, _)| p == path) {
            // Most recently used last.
            let e = self.cache.remove(pos);
            self.cache.push(e);
        } else {
            let pm = Pixmap::load_png(path).map_err(|e| format!("{}: {e}", path.display()))?;
            if self.cache.len() >= MAX_CACHED_SHOTS {
                self.cache.remove(0);
            }
            self.cache.push((path.to_path_buf(), pm));
        }
        self.cache
            .last()
            .map(|(_, pm)| pm)
            .ok_or_else(|| "shot cache".to_string())
    }

    fn draw(&mut self, st: &FrameState, out: &mut Pixmap) -> Result<(), String> {
        let m = self.m;
        let ow = f64::from(self.p.out_w);
        out.fill(Color::WHITE);
        let path = m
            .segs
            .get(st.shot.0)
            .and_then(|s| s.shots.get(st.shot.1))
            .map(|(_, p)| p.clone())
            .ok_or("frame state out of range")?;
        let vw = (st.view[2] - st.view[0]).max(1e-6);
        let map = Map {
            x0: st.view[0],
            y0: st.view[1],
            s: ow / vw,
        };
        let zoom = (m.win.0 / vw).max(1.0);
        let sc = (map.s / m.ppp) as f32;
        let pp = PixmapPaint {
            quality: FilterQuality::Bicubic,
            ..PixmapPaint::default()
        };
        let tr = Transform::from_row(sc, 0.0, 0.0, sc, (-map.x0 * map.s) as f32, (-map.y0 * map.s) as f32);
        let pm = self.shot(&path)?;
        out.draw_pixmap(0, 0, pm.as_ref(), &pp, tr, None);

        let id = Transform::identity();
        let (fw, fh) = (self.p.out_w as f32, self.p.out_h as f32);
        // Spotlight.
        if let Some((r, a)) = st.spot {
            let a = a as f32;
            let [x0, y0, x1, y1] = map.rect(r);
            if let Some(hole) = rrect_path([x0, y0, x1, y1], 6.0) {
                let mut pb = PathBuilder::new();
                if let Some(full) = Rect::from_xywh(0.0, 0.0, fw, fh) {
                    pb.push_rect(full);
                }
                pb.push_path(&hole);
                if let Some(path) = pb.finish() {
                    out.fill_path(&path, &paint_rgba(0, 0, 0, DIM_ALPHA * a), FillRule::EvenOdd, id, None);
                }
                let stroke = Stroke {
                    width: 3.0,
                    line_join: tiny_skia::LineJoin::Round,
                    ..Stroke::default()
                };
                out.stroke_path(&hole, &paint_rgba(0xd8, 0x23, 0x2a, a), &stroke, id, None);
            }
        }
        // Mouse indicator highlights.
        for (b, on) in st.highlight.iter().enumerate() {
            if !on {
                continue;
            }
            let Some(r) = m.buttons.get(b) else { continue };
            if let Some(path) = rrect_path(map.rect(*r), 3.0) {
                out.fill_path(&path, &paint_rgba(0xff, 0xb0, 0x20, 0.5), FillRule::Winding, id, None);
                let stroke = Stroke {
                    width: 2.0,
                    ..Stroke::default()
                };
                out.stroke_path(&path, &paint_rgba(0xe0, 0x60, 0x00, 0.95), &stroke, id, None);
            }
        }
        // Click rings.
        let pscale = zoom.sqrt() as f32;
        for &(x, y, p) in &st.rings {
            let (cx, cy) = map.pt(x, y);
            let p = p as f32;
            if let Some(c) = PathBuilder::from_circle(cx, cy, (6.0 + 30.0 * p) * pscale) {
                let stroke = Stroke {
                    width: 3.0 * (1.0 - 0.5 * p),
                    ..Stroke::default()
                };
                out.stroke_path(&c, &paint_rgba(0xd8, 0x23, 0x2a, 1.0 - p), &stroke, id, None);
            }
        }
        // Held dot.
        if let (Some((x, y)), Some(b)) = (st.pointer, st.held) {
            let (cx, cy) = map.pt(x, y);
            let col = match b {
                Btn::Left => (0xd8, 0x23, 0x2a),
                Btn::Middle => (0x20, 0x90, 0x40),
                Btn::Right => (0x20, 0x60, 0xd0),
            };
            if let Some(c) = PathBuilder::from_circle(cx, cy, 8.0 * pscale) {
                out.fill_path(&c, &paint_rgba(col.0, col.1, col.2, 0.6), FillRule::Winding, id, None);
            }
        }
        // Key badge.
        if let Some(k) = &st.keys {
            if let Some(b) = self.badges.get(k) {
                let (bw, bh) = (b.width() as f32, b.height() as f32);
                let (x, y) = match st.pointer {
                    Some((px, py)) => {
                        let (cx, cy) = map.pt(px, py);
                        (cx + 24.0, cy + 26.0)
                    }
                    None => ((fw - bw) / 2.0, fh - bh - 40.0),
                };
                let x = x.clamp(4.0, (fw - bw - 4.0).max(4.0));
                let y = y.clamp(4.0, (fh - bh - 4.0).max(4.0));
                out.draw_pixmap(
                    x.round() as i32,
                    y.round() as i32,
                    b.as_ref(),
                    &PixmapPaint::default(),
                    id,
                    None,
                );
            }
        }
        // Pointer arrow on top.
        if let Some((x, y)) = st.pointer {
            let (cx, cy) = map.pt(x, y);
            let k = 22.0 / 19.0 * pscale;
            let pts = [
                (0.0, 0.0),
                (0.0, 16.0),
                (4.0, 12.5),
                (7.0, 19.0),
                (9.5, 18.0),
                (6.5, 11.5),
                (11.0, 11.5),
            ];
            let mut pb = PathBuilder::new();
            for (n, (px, py)) in pts.iter().enumerate() {
                let (qx, qy) = (cx + px * k, cy + py * k);
                if n == 0 {
                    pb.move_to(qx, qy);
                } else {
                    pb.line_to(qx, qy);
                }
            }
            pb.close();
            if let Some(path) = pb.finish() {
                let stroke = Stroke {
                    width: 1.8,
                    line_join: tiny_skia::LineJoin::Round,
                    ..Stroke::default()
                };
                out.stroke_path(&path, &paint_rgba(255, 255, 255, 1.0), &stroke, id, None);
                out.fill_path(&path, &paint_rgba(0, 0, 0, 1.0), FillRule::Winding, id, None);
            }
        }
        Ok(())
    }
}

// ----------------------------------------------------------- external programs

fn tool_path() -> String {
    format!("/opt/homebrew/bin:{}", std::env::var("PATH").unwrap_or_default())
}

fn run_magick(args: &[String]) -> Result<String, String> {
    let o = Command::new("magick")
        .args(args)
        .env("PATH", tool_path())
        .output()
        .map_err(|e| format!("magick: {e}"))?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).into_owned())
    } else {
        Err(format!("magick {args:?}: {}", String::from_utf8_lossy(&o.stderr)))
    }
}

fn badge_font() -> Result<String, String> {
    if let Ok(f) = std::env::var("MOVIE_FONT") {
        return Ok(f);
    }
    [
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/Library/Fonts/Arial Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf",
        "C:/Windows/Fonts/arialbd.ttf",
    ]
    .iter()
    .find(|f| Path::new(f).is_file())
    .map(|f| (*f).to_string())
    .ok_or_else(|| "no bold font file found for the key badges; set MOVIE_FONT".to_string())
}

/// A keycap badge (rounded dark-gray rect, white bold text) as a pixmap.
fn make_badge(text: &str, n: usize, font: &str, work: &Path) -> Result<Pixmap, String> {
    let txt = work.join(format!("key{n}.txt"));
    std::fs::write(&txt, text).map_err(|e| format!("{}: {e}", txt.display()))?;
    let label = format!("label:@{}", txt.display());
    let args = |extra: &[&str]| -> Vec<String> {
        let mut v: Vec<String> = [
            "-font",
            font,
            "-pointsize",
            "18",
            "-background",
            "none",
            "-fill",
            "white",
        ]
        .iter()
        .map(|x| (*x).to_string())
        .collect();
        v.push(label.clone());
        v.extend(extra.iter().map(|x| (*x).to_string()));
        v
    };
    let size = run_magick(&args(&["-format", "%w %h", "info:"]))?;
    let mut it = size.split_whitespace().filter_map(|v| v.parse::<u32>().ok());
    let (tw, th) = (it.next().ok_or("badge size")?, it.next().ok_or("badge size")?);
    let (w, h) = (tw + 28, th + 16);
    let png = work.join(format!("key{n}.png"));
    let mut cmd: Vec<String> = vec![
        "-size".into(),
        format!("{w}x{h}"),
        "xc:none".into(),
        "-fill".into(),
        "#2b2b2b".into(),
        "-stroke".into(),
        "#9a9a9a".into(),
        "-strokewidth".into(),
        "1.5".into(),
        "-draw".into(),
        format!("roundrectangle 1,1 {},{} 8,8", w - 2, h - 2),
        "(".into(),
    ];
    cmd.extend(args(&[]));
    cmd.extend([")".into(), "-gravity".into(), "center".into(), "-composite".into()]);
    cmd.push(png.display().to_string());
    run_magick(&cmd)?;
    Pixmap::load_png(&png).map_err(|e| format!("{}: {e}", png.display()))
}

/// Write `<out>/<stem>.mp4` (H.264), `<out>/<stem>.webm` (VP9), `<out>/<stem>.vtt`
/// and `<out>/<stem>-poster.jpg` (the last frame).
pub fn render(m: &Movie, out: &Path, stem: &str, work: &Path) -> Result<(), String> {
    let plan = make_plan(m)?;
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    std::fs::create_dir_all(work).map_err(|e| format!("{}: {e}", work.display()))?;
    std::fs::write(out.join(format!("{stem}.vtt")), vtt(&m.cues)).map_err(|e| format!("vtt: {e}"))?;

    let mut badges = HashMap::new();
    let mut font = None;
    for s in &m.segs {
        if let Some(k) = &s.keys {
            if !badges.contains_key(k) {
                if font.is_none() {
                    font = Some(badge_font()?);
                }
                let f = font.as_deref().unwrap_or_default();
                badges.insert(k.clone(), make_badge(k, badges.len(), f, work)?);
            }
        }
    }
    let mut r = Renderer {
        m,
        p: &plan,
        cache: Vec::new(),
        badges,
    };
    let (w, h) = (plan.out_w, plan.out_h);
    let mp4 = out.join(format!("{stem}.mp4"));
    let webm = out.join(format!("{stem}.webm"));
    let log = work.join("ffmpeg.log");
    let logf = std::fs::File::create(&log).map_err(|e| format!("{}: {e}", log.display()))?;
    let mut child = Command::new("ffmpeg")
        .env("PATH", tool_path())
        .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgba", "-s"])
        .arg(format!("{w}x{h}"))
        .arg("-r")
        .arg(m.fps.to_string())
        .args([
            "-i", "-", "-map", "0:v", "-c:v", "libx264", "-preset", "slow", "-crf", "24",
        ])
        .args(["-tune", "animation", "-pix_fmt", "yuv420p", "-movflags", "+faststart"])
        .arg(&mp4)
        .args([
            "-map",
            "0:v",
            "-c:v",
            "libvpx-vp9",
            "-crf",
            "36",
            "-b:v",
            "0",
            "-row-mt",
            "1",
        ])
        .args(["-deadline", "good", "-cpu-used", "2", "-pix_fmt", "yuv420p"])
        .arg(&webm)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::from(logf))
        .spawn()
        .map_err(|e| format!("ffmpeg: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("ffmpeg stdin")?;

    let frames = ((plan.total * f64::from(m.fps)).ceil() as usize).max(1);
    let mut pix = Pixmap::new(w, h).ok_or("output size")?;
    let mut last: Option<FrameState> = None;
    let mut result = Ok(());
    for i in 0..frames {
        let t = i as f64 / f64::from(m.fps);
        let Some(st) = state_at(m, &plan, t) else { break };
        if last.as_ref() != Some(&st) {
            if let Err(e) = r.draw(&st, &mut pix) {
                result = Err(e);
                break;
            }
            last = Some(st);
        }
        if let Err(e) = stdin.write_all(pix.data()) {
            result = Err(format!("ffmpeg stdin: {e}"));
            break;
        }
    }
    drop(stdin);
    let status = child.wait().map_err(|e| format!("ffmpeg: {e}"))?;
    result?;
    if !status.success() {
        let l = std::fs::read_to_string(&log).unwrap_or_default();
        return Err(format!("ffmpeg failed: {l}"));
    }
    let last_png = work.join("last.png");
    pix.save_png(&last_png)
        .map_err(|e| format!("{}: {e}", last_png.display()))?;
    run_magick(&[
        last_png.display().to_string(),
        "-background".into(),
        "white".into(),
        "-alpha".into(),
        "remove".into(),
        "-quality".into(),
        "85".into(),
        out.join(format!("{stem}-poster.jpg")).display().to_string(),
    ])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vtt_text() {
        let v = vtt(&[(1.5, 4.0, "Hello".into()), (3725.25, 3726.0, "Two\nlines".into())]);
        assert_eq!(
            v,
            "WEBVTT\n\n1\n00:00:01.500 --> 00:00:04.000\nHello\n\n2\n01:02:05.250 --> 01:02:06.000\nTwo\nlines\n\n"
        );
    }

    #[test]
    fn easing() {
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        assert!((ease(0.5) - 0.5).abs() < 1e-12);
        assert!(ease(0.25) < 0.25 && ease(0.75) > 0.75);
        assert_eq!(ease(-3.0), 0.0);
        assert_eq!(ease(f64::NAN), 0.0);
    }

    #[test]
    fn aspect_fix() {
        let win = (1400.0, 800.0);
        let a = 1400.0 / 800.0;
        let r = fix_aspect([100.0, 100.0, 300.0, 200.0], a, win);
        assert!(((r[2] - r[0]) / (r[3] - r[1]) - a).abs() < 1e-9);
        assert!(r[0] <= 100.0 && r[2] >= 300.0 && r[1] <= 100.0 && r[3] >= 200.0);
        // Near the corner: shifted inside.
        let r = fix_aspect([0.0, 0.0, 100.0, 100.0], a, win);
        assert!(r[0] >= 0.0 && r[1] >= 0.0);
        // Too large: the whole window.
        assert_eq!(
            fix_aspect([-50.0, -50.0, 5000.0, 5000.0], a, win),
            [0.0, 0.0, 1400.0, 800.0]
        );
        assert_eq!(fix_aspect([f64::NAN, 0.0, 1.0, 1.0], a, win), [0.0, 0.0, 1400.0, 800.0]);
    }

    #[test]
    fn segment_lookup() {
        let d = [1.0, 0.0, 2.0];
        assert_eq!(locate(&d, 0.0), Some((0, 0.0)));
        assert_eq!(locate(&d, 0.5), Some((0, 0.5)));
        assert_eq!(locate(&d, 1.0), Some((2, 0.0)));
        assert_eq!(locate(&d, 2.0), Some((2, 0.5)));
        assert_eq!(locate(&d, 9.0), Some((2, 1.0)));
        assert_eq!(locate(&[], 0.0), None);
    }

    #[test]
    #[ignore = "needs ffmpeg, magick and local screenshots; renders into $TMPDIR"]
    fn render_sample() {
        let dir = PathBuf::from(
            "/private/tmp/claude-501/-Users-fabian-Documents-RustFig/d9cc0e8c-a7c5-4310-9ee2-5e6503e1ce42/scratchpad",
        );
        let (a, b) = (dir.join("s4.png"), dir.join("s5.png"));
        let seg = |dur: f64| Seg {
            dur,
            shots: vec![(0.0, a.clone())],
            from: None,
            to: None,
            held: None,
            click: None,
            keys: None,
            view: None,
            spot: None,
        };
        let tmp = PathBuf::from(std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into())).join("movie_render_test");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut s1 = seg(1.5);
        s1.from = Some((100.0, 100.0));
        s1.to = Some((700.0, 400.0));
        let mut s2 = seg(0.5);
        s2.from = Some((700.0, 400.0));
        s2.to = Some((700.0, 400.0));
        s2.held = Some(Btn::Left);
        s2.click = Some(Btn::Left);
        let mut s3 = seg(1.5);
        s3.shots = vec![(0.0, a.clone()), (0.5, b.clone())];
        s3.from = Some((700.0, 400.0));
        s3.to = Some((300.0, 300.0));
        s3.keys = Some("Shift".into());
        s3.view = Some([100.0, 150.0, 500.0, 375.0]);
        let mut s4 = seg(1.5);
        s4.from = Some((300.0, 300.0));
        s4.to = Some((1230.0, 32.0));
        s4.spot = Some([1190.0, 10.0, 1280.0, 60.0]);
        let mut s5 = seg(1.0);
        s5.from = Some((1230.0, 32.0));
        s5.to = Some((1230.0, 32.0));
        s5.shots = vec![(0.0, b.clone())];
        s5.keys = Some("⌘ Z".into());
        let m = Movie {
            segs: vec![s1, s2, s3, s4, s5],
            ppp: 2.0,
            win: (1400.0, 826.0),
            buttons: [
                [1212.0, 20.0, 1226.0, 45.0],
                [1226.0, 20.0, 1238.0, 45.0],
                [1238.0, 20.0, 1252.0, 45.0],
            ],
            cues: vec![(0.0, 2.0, "Start".into())],
            width: 1280,
            fps: 30,
        };
        let t0 = std::time::Instant::now();
        let res = render(&m, &tmp, "t", &tmp.join("work"));
        assert!(res.is_ok(), "{res:?}");
        eprintln!("render took {:?}", t0.elapsed());
        assert!(tmp.join("t.mp4").is_file() && tmp.join("t.webm").is_file());
        assert!(tmp.join("t-poster.jpg").is_file());
    }
}
