//! `cargo xtask movie` (plan R9): a movie of drawing a block on an inclined plane, made by driving
//! the release app through the control channel. A short tour lights up the window's areas (the
//! rects of R8's [`AREAS`]), then the figure is drawn with the mouse as a user would: the slope as a
//! polygon, the angle measured and annotated, a box rotated onto the slope, force arrows, LaTeX
//! labels, a copy and its undo, Save and File › Export to PDF; the movie ends on the PDF.
//!
//! Every step takes a screenshot; [`movie_render`](crate::movie_render) draws the pointer, clicks,
//! the pressed mouse button, key badges, the tour spotlight and the zooms over them and has ffmpeg
//! write `docs/images/movie/inclined-plane.{mp4,webm,vtt}` and `inclined-plane-poster.jpg`. The
//! saved figure must match `examples/inclined-plane/inclined-plane.fig` (`MOVIE_UPDATE=1` writes
//! it, with the preview picture, when the drawing changes on purpose).

use crate::movie::{rect_of, AREAS};
use crate::movie_render::{self, ease, Btn, Movie, Seg};
use crate::xfig_compare::{fig_objects, json_str, numbers_after, sleep_ms, Rf};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The figure, in Fig units (1200 per inch, 1/16 inch grid). The slope rises 37 in 64 (30.03
/// degrees, labeled 30°); the block's corner `P` is the grid point nearest to it (1.2 units off).
const A: (f64, f64) = (1800.0, 4500.0);
const B: (f64, f64) = (6600.0, 4500.0);
const C: (f64, f64) = (6600.0, 1725.0);
/// The block's lower left corner on the slope, its opposite corner, and the middle of its top edge.
const P: (f64, f64) = (5175.0, 2550.0);
const P2: (f64, f64) = (6225.0, 1800.0);
const TOP: (f64, f64) = (5700.0, 1800.0);
/// The block's center after the rotation, the tips of the weight and of the normal force.
const MID: (f64, f64) = (5475.0, 1950.0);
const WEIGHT: (f64, f64) = (5475.0, 3300.0);
const NORMAL: (f64, f64) = (4650.0, 525.0);
/// The labels' anchors.
const FG_AT: (f64, f64) = (5600.0, 3300.0);
const N_AT: (f64, f64) = (4200.0, 600.0);
/// A point on the rotated block's lower edge (to pick it), and where the copy goes (down the slope).
const PICK: (f64, f64) = (5630.0, 2287.0);
const PLACE: (f64, f64) = (5030.0, 2634.0);

/// Each tour stop's caption, in the order of [`AREAS`].
const TOUR: [&str; 9] = [
    "The menus: File, Edit, View, Tools, Snap and Help",
    "The name of the figure",
    "The message panel says what to do next",
    "The mouse indicator: what the left, middle and right buttons do right now",
    "The units of the rulers",
    "The mode panel: drawing modes at the top, editing modes below",
    "The canvas, with its rulers",
    "The indicator panel: the attributes of new objects",
    "Depths and snapping",
];

struct Director {
    rf: Rf,
    work: PathBuf,
    segs: Vec<Seg>,
    cues: Vec<(f64, f64, String)>,
    t: f64,
    /// The caption shown now and since when.
    caption: Option<(f64, String)>,
    ptr: Option<(f64, f64)>,
    view: Option<[f64; 4]>,
    spot: Option<[f64; 4]>,
    /// The latest screenshot.
    cur: PathBuf,
    n: usize,
    /// Canvas corner (points), Fig position at that corner, points per Fig unit.
    canvas: (f64, f64),
    origin: (f64, f64),
    scale: f64,
    /// Modifiers held (`ui.modifiers` and each pointer step), JSON names.
    mods: Vec<&'static str>,
}

impl Director {
    fn state(&mut self) -> Result<String, String> {
        self.rf.call("ui.state", "{}")
    }

    /// Read where the canvas is and its zoom.
    fn locate(&mut self) -> Result<(), String> {
        let st = self.state()?;
        let o = numbers_after(&st, "\"origin\":[", 2).ok_or("ui.state has no origin")?;
        self.origin = (*o.first().ok_or("origin")?, *o.get(1).ok_or("origin")?);
        let r = rect_of(&st, "Canvas:").ok_or("ui.state has no canvas")?;
        self.canvas = (r[0], r[1]);
        let z = st
            .find("\"zoom\":")
            .and_then(|i| st.get(i + 7..))
            .and_then(|t| t.split([',', '}']).next())
            .and_then(|t| t.trim().parse::<f64>().ok())
            .filter(|z| z.is_finite() && *z > 0.0)
            .ok_or("ui.state has no zoom")?;
        self.scale = z * 80.0 / 1200.0;
        Ok(())
    }

    /// Window points of a Fig position.
    fn pt(&self, p: (f64, f64)) -> (f64, f64) {
        (
            self.canvas.0 + (p.0 - self.origin.0) * self.scale,
            self.canvas.1 + (p.1 - self.origin.1) * self.scale,
        )
    }

    /// Window rect (points) of a Fig rect.
    fn fig_rect(&self, a: (f64, f64), b: (f64, f64)) -> [f64; 4] {
        let (p, q) = (self.pt(a), self.pt(b));
        [p.0.min(q.0), p.1.min(q.1), p.0.max(q.0), p.1.max(q.1)]
    }

    fn shot(&mut self) -> Result<PathBuf, String> {
        self.n += 1;
        let p = self.work.join(format!("f{:04}.png", self.n));
        // Nothing on screen may name the home folder of whoever films.
        if let Some(home) = std::env::var_os("HOME").and_then(|h| h.into_string().ok()) {
            if home.len() > 1 && self.state()?.contains(&json_str(&home).trim_matches('"').to_string()) {
                return Err(format!("the window shows the home folder {home} (frame {})", self.n));
            }
        }
        self.rf.shot(&p)?;
        for _ in 0..50 {
            if p.is_file() {
                self.cur.clone_from(&p);
                return Ok(p);
            }
            sleep_ms(100);
        }
        Err(format!("no screenshot at {}", p.display()))
    }

    fn push(&mut self, dur: f64, shots: Vec<(f64, PathBuf)>, to: Option<(f64, f64)>) -> &mut Seg {
        let seg = Seg {
            dur,
            shots,
            from: self.ptr,
            to,
            held: None,
            click: None,
            keys: None,
            view: self.view,
            spot: self.spot,
        };
        self.ptr = to;
        self.t += dur;
        self.segs.push(seg);
        let last = self.segs.len() - 1;
        &mut self.segs[last]
    }

    /// Show the current state for `dur` seconds.
    fn hold(&mut self, dur: f64) {
        let shots = vec![(0.0, self.cur.clone())];
        let to = self.ptr;
        self.push(dur, shots, to);
    }

    /// Start a caption (the one before ends here).
    fn say(&mut self, text: &str) {
        self.end_caption();
        self.caption = Some((self.t, text.to_string()));
    }

    fn end_caption(&mut self) {
        if let Some((t0, text)) = self.caption.take() {
            if self.t > t0 + 0.2 {
                self.cues.push((t0, self.t, text));
            }
        }
    }

    fn pointer_json(&self, action: &str, at: (f64, f64), b: Option<Btn>) -> String {
        let button = match b {
            Some(Btn::Left) => r#","button":"left""#,
            Some(Btn::Middle) => r#","button":"middle""#,
            Some(Btn::Right) => r#","button":"right""#,
            None => "",
        };
        let mods = if self.mods.is_empty() {
            String::new()
        } else {
            let m: Vec<String> = self.mods.iter().map(|m| json_str(m)).collect();
            format!(r#","modifiers":[{}]"#, m.join(","))
        };
        format!(r#"{{"action":"{action}","x":{},"y":{}{button}{mods}}}"#, at.0, at.1)
    }

    /// Move the pointer to `to` (window points), with screenshots on the way so what follows the
    /// pointer (rubber bands, hover) moves too.
    fn move_pt(&mut self, to: (f64, f64), steps: usize) -> Result<(), String> {
        let from = self.ptr.unwrap_or(to);
        let dist = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
        if dist < 0.5 {
            return Ok(());
        }
        let dur = (0.35 + dist / 900.0).min(1.1);
        let mut shots = vec![(0.0, self.cur.clone())];
        let steps = steps.max(1);
        for i in 1..=steps {
            let f = i as f64 / (steps + 1) as f64;
            let e = ease(f);
            let at = (from.0 + (to.0 - from.0) * e, from.1 + (to.1 - from.1) * e);
            let j = self.pointer_json("move", at, None);
            self.rf.call("ui.pointer", &j)?;
            shots.push((f, self.shot()?));
        }
        let j = self.pointer_json("move", to, None);
        self.rf.call("ui.pointer", &j)?;
        self.ptr = Some(from);
        self.push(dur, shots, Some(to));
        // The frame at the end of the move.
        self.shot()?;
        Ok(())
    }

    /// Move over the canvas to a Fig position.
    fn to(&mut self, p: (f64, f64)) -> Result<(), String> {
        let q = self.pt(p);
        self.move_pt(q, 6)
    }

    /// Click with `b` where the pointer is: a short press, then the result.
    fn click(&mut self, b: Btn) -> Result<(), String> {
        let at = self.ptr.ok_or("click without a pointer")?;
        let before = self.cur.clone();
        let j = self.pointer_json("click", at, Some(b));
        self.rf.call("ui.pointer", &j)?;
        sleep_ms(250);
        let keys = self.keys_shown();
        let seg = self.push(0.18, vec![(0.0, before)], Some(at));
        seg.held = Some(b);
        seg.click = Some(b);
        seg.keys = keys;
        self.shot()?;
        self.hold(0.45);
        Ok(())
    }

    fn keys_shown(&self) -> Option<String> {
        self.mods.contains(&"shift").then(|| "Shift".to_string())
    }

    /// Move to a Fig position and click there.
    fn click_at(&mut self, p: (f64, f64), b: Btn) -> Result<(), String> {
        self.to(p)?;
        self.click(b)
    }

    /// The rect of the widget labeled `label` (exact, else the first starting with it).
    fn widget(&mut self, label: &str) -> Result<[f64; 4], String> {
        let st = self.state()?;
        let exact = format!("\"label\":{}", json_str(label));
        let r = st
            .find(&exact)
            .and_then(|i| st.get(i..))
            .and_then(|t| numbers_after(t, "\"rect\":[", 4))
            .and_then(|v| Some([*v.first()?, *v.get(1)?, *v.get(2)?, *v.get(3)?]))
            .or_else(|| rect_of(&st, label));
        r.ok_or_else(|| format!("no widget labeled {label:?}"))
    }

    /// Move to a widget and click it with the left button.
    fn press(&mut self, label: &str) -> Result<(), String> {
        let r = self.widget(label)?;
        self.move_pt(((r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0), 2)?;
        self.click(Btn::Left)?;
        sleep_ms(200);
        self.shot()?;
        Ok(())
    }

    /// A key (egui name) with modifiers, shown as `badge`.
    fn key(&mut self, key: &str, mods: &[&str], badge: &str) -> Result<(), String> {
        let m: Vec<String> = mods.iter().map(|m| json_str(m)).collect();
        self.rf.call(
            "ui.key",
            &format!(r#"{{"key":{},"modifiers":[{}]}}"#, json_str(key), m.join(",")),
        )?;
        sleep_ms(400);
        let before = self.cur.clone();
        let to = self.ptr;
        let seg = self.push(0.5, vec![(0.0, before)], to);
        seg.keys = Some(badge.to_string());
        self.shot()?;
        let to = self.ptr;
        let cur = self.cur.clone();
        let seg = self.push(0.6, vec![(0.0, cur)], to);
        seg.keys = Some(badge.to_string());
        Ok(())
    }

    /// Type `text` one character at a time.
    fn type_text(&mut self, text: &str) -> Result<(), String> {
        for ch in text.chars() {
            self.rf.call(
                "ui.key",
                &format!(r#"{{"key":"A","text":{}}}"#, json_str(&ch.to_string())),
            )?;
            sleep_ms(60);
            self.shot()?;
            self.hold(0.11);
        }
        Ok(())
    }

    fn hold_mods(&mut self, mods: &[&'static str]) -> Result<(), String> {
        self.mods = mods.to_vec();
        let m: Vec<String> = mods.iter().map(|m| json_str(m)).collect();
        self.rf
            .call("ui.modifiers", &format!(r#"{{"modifiers":[{}]}}"#, m.join(",")))?;
        sleep_ms(400);
        self.shot()?;
        Ok(())
    }

    /// Zoom the view to `r` (window points; None: the whole window) over `dur` seconds.
    fn zoom(&mut self, r: Option<[f64; 4]>, dur: f64) {
        self.view = r;
        self.hold(dur);
    }

    /// Wait until the canvas has drawn everything (TeX runs in the background).
    fn settle(&mut self) -> Result<(), String> {
        for _ in 0..100 {
            let st = self.state()?;
            if st.contains("\"canvas_pending\":false") && st.contains("\"input_pending\":false") {
                break;
            }
            sleep_ms(100);
        }
        sleep_ms(300);
        self.shot()?;
        Ok(())
    }
}

/// The result: the exported PDF, rendered at the window's pixel size on a gray backdrop.
fn pdf_frame(pdf: &Path, w: f64, h: f64, out: &Path) -> Result<(), String> {
    let o = Command::new("magick")
        .args(["-density", "300"])
        .arg(pdf)
        .args([
            "-background",
            "white",
            "-flatten",
            "-trim",
            "-bordercolor",
            "white",
            "-border",
            "60",
        ])
        .args(["-resize", &format!("{}x{}", (w * 0.8).round(), (h * 0.8).round())])
        .args(["-background", "#d9dce1", "-gravity", "center", "-extent"])
        .arg(format!("{}x{}", w.round(), h.round()))
        .arg(out)
        .env(
            "PATH",
            format!("/opt/homebrew/bin:{}", std::env::var("PATH").unwrap_or_default()),
        )
        .output()
        .map_err(|e| format!("magick: {e}"))?;
    if o.status.success() {
        Ok(())
    } else {
        Err(format!("rendering the PDF: {}", String::from_utf8_lossy(&o.stderr)))
    }
}

/// The preview of the example: the PDF at 150 dpi.
fn preview(pdf: &Path, out: &Path) -> Result<(), String> {
    let o = Command::new("magick")
        .args(["-density", "150"])
        .arg(pdf)
        .args(["-background", "white", "-flatten", "-colors", "256", "-strip"])
        .arg(out)
        .env(
            "PATH",
            format!("/opt/homebrew/bin:{}", std::env::var("PATH").unwrap_or_default()),
        )
        .output()
        .map_err(|e| format!("magick: {e}"))?;
    if o.status.success() {
        Ok(())
    } else {
        Err(format!("preview: {}", String::from_utf8_lossy(&o.stderr)))
    }
}

const EMPTY_FIG: &str =
    "#FIG 3.2  Produced by RustFig\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n";

/// Film it: `bin` is the release app, `out` the movie folder.
pub fn film(root: &Path, bin: &Path, port: u16, work: &Path, out: &Path) -> Result<(), String> {
    // The figure's folder shows in the window (messages, the export panel): a neutral one, never
    // the home folder of whoever films.
    let dir = std::env::var_os("MOVIE_FIG_DIR").map_or_else(
        || {
            if cfg!(unix) {
                PathBuf::from("/tmp/rustfig-movie")
            } else {
                std::env::temp_dir().join("rustfig-movie")
            }
        },
        PathBuf::from,
    );
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let fig = dir.join("inclined-plane.fig");
    std::fs::write(&fig, EMPTY_FIG).map_err(|e| e.to_string())?;
    let pdf = dir.join("inclined-plane.pdf");
    let rf = Rf::start(bin, port, work, &fig)?;
    let mut d = Director {
        rf,
        work: work.to_path_buf(),
        segs: Vec::new(),
        cues: Vec::new(),
        t: 0.0,
        caption: None,
        ptr: None,
        view: None,
        spot: None,
        cur: PathBuf::new(),
        n: 0,
        canvas: (0.0, 0.0),
        origin: (0.0, 0.0),
        scale: 1.0,
        mods: Vec::new(),
    };
    // Zoom 2 with Fig (0,0) at the canvas corner: the drawing fills the canvas.
    d.rf.call("engine.execute", r#"{"command":"view.zoom_set","params":{"zoom":2}}"#)?;
    sleep_ms(300);
    d.locate()?;
    let (ox, oy) = (d.origin.0 * d.scale, d.origin.1 * d.scale);
    d.rf.call(
        "engine.execute",
        &format!(r#"{{"command":"view.pan","params":{{"dx":{ox},"dy":{oy}}}}}"#),
    )?;
    sleep_ms(300);
    d.locate()?;
    // The pointer rests on the canvas at the start.
    let rest = d.pt((7800.0, 2600.0));
    let j = d.pointer_json("move", rest, None);
    d.rf.call("ui.pointer", &j)?;
    sleep_ms(400);
    d.ptr = Some(rest);
    d.shot()?;

    let st = d.state()?;
    let mut buttons = [[0.0; 4]; 3];
    for (b, name) in buttons.iter_mut().zip(["left", "middle", "right"]) {
        *b = rect_of(&st, &format!("Mouse button: {name}")).ok_or("ui.state has no mouse button boxes")?;
    }

    // ---- the tour
    d.say("Drawing a block on an inclined plane with RustFig");
    d.hold(2.0);
    for (a, text) in AREAS.iter().zip(TOUR) {
        let r = rect_of(&st, a.label).ok_or_else(|| format!("ui.state has no rect for {:?}", a.label))?;
        d.say(text);
        d.spot = Some(r);
        d.hold(0.5);
        d.hold(1.1);
    }
    d.spot = None;
    d.end_caption();
    d.hold(0.6);

    // ---- the slope
    d.say("Polygon mode: the left button places the corners of the slope");
    d.press("Polygon")?;
    d.click_at(A, Btn::Left)?;
    d.click_at(B, Btn::Left)?;
    d.say("The middle button places the last corner and closes the polygon");
    d.click_at(C, Btn::Middle)?;
    d.hold(1.2);

    // ---- the angle
    d.say("Go to mode: type part of a mode's name and press Return");
    d.press("Go to mode")?;
    d.type_text("angle")?;
    d.key("Enter", &[], "Return")?;
    d.say("Measure angle, middle button at the corner: it measures the angle and keeps it as the rotation angle");
    d.click_at(A, Btn::Middle)?;
    d.hold(1.6);
    let near_a = d.fig_rect((A.0 - 300.0, A.1 - 1500.0), (A.0 + 2700.0, A.1 + 300.0));
    d.zoom(Some(near_a), 0.9);
    d.say("Shift+middle draws the angle as an arc with its label");
    d.hold_mods(&["shift"])?;
    d.hold(0.8);
    d.click_at(A, Btn::Middle)?;
    d.hold_mods(&[])?;
    d.hold(1.4);
    d.zoom(None, 0.9);

    // ---- the block
    d.say("Box mode: two left clicks at opposite corners");
    d.press("Box")?;
    d.click_at(P, Btn::Left)?;
    d.click_at(P2, Btn::Left)?;
    d.say("Rotate counter-clockwise: the right button sets the center");
    d.press("Rotate counter-clockwise")?;
    d.click_at(P, Btn::Right)?;
    d.say("The left button turns the box by the measured angle, onto the slope");
    d.click_at(TOP, Btn::Left)?;
    d.hold(1.2);

    // ---- the forces
    d.say("Polyline mode, with a forward arrow set in the indicator panel");
    d.press("Polyline")?;
    d.press("Arrow Mode")?;
    d.press("forward")?;
    d.press("Arrow Type")?;
    d.press("solid type 2")?;
    d.say("The left button starts a line, the middle button ends it: the weight and the normal force");
    d.click_at(MID, Btn::Left)?;
    d.click_at(WEIGHT, Btn::Middle)?;
    d.click_at(MID, Btn::Left)?;
    d.click_at(NORMAL, Btn::Middle)?;
    d.hold(1.0);

    // ---- the labels
    d.say("Text mode with the TeX flag: the text is LaTeX, typeset on the canvas");
    d.press("Text")?;
    d.press("Text Flags")?;
    d.press("TeX")?;
    d.key("Escape", &[], "Esc")?;
    let labels = d.fig_rect((3700.0, 150.0), (7000.0, 3700.0));
    d.zoom(Some(labels), 0.9);
    d.click_at(FG_AT, Btn::Left)?;
    d.type_text("$\\vec F_g$")?;
    d.say("The middle button ends the text");
    d.click(Btn::Middle)?;
    d.say("Text mode with the TeX flag: the text is LaTeX, typeset on the canvas");
    d.click_at(N_AT, Btn::Left)?;
    d.type_text("$\\vec N$")?;
    d.click(Btn::Middle)?;
    d.settle()?;
    d.hold(1.6);
    d.zoom(None, 0.9);

    // ---- copy and undo
    d.say("Copy mode: pick the block with the left button, place the copy with another click");
    d.press("Copy")?;
    d.click_at(PICK, Btn::Left)?;
    d.to(((PICK.0 + PLACE.0) / 2.0, (PICK.1 + PLACE.1) / 2.0))?;
    d.click_at(PLACE, Btn::Left)?;
    d.hold(0.8);
    d.say("Undo takes it back");
    d.key("Z", &["command"], "⌘ Z")?;
    d.hold(0.8);
    d.say("Save");
    d.key("S", &["command"], "⌘ S")?;

    // ---- export
    d.say("File › Export…: the PDF language is preselected, with the LaTeX typeset");
    d.press("File")?;
    d.press("Export...")?;
    d.hold(2.2);
    d.rf.call(
        "ui.file_answers",
        &format!(r#"{{"answers":[{}]}}"#, json_str(&pdf.display().to_string())),
    )?;
    d.say("Export… asks where to save, then writes the PDF");
    d.press("Export…")?;
    for _ in 0..100 {
        if pdf.is_file() {
            break;
        }
        sleep_ms(100);
    }
    d.settle()?;
    d.hold(2.0);

    // ---- the result
    let (pw, ph) = crate::movie::png_size(&d.cur)?;
    let result = work.join("result.png");
    pdf_frame(&pdf, pw, ph, &result)?;
    d.say("The PDF");
    let shots = vec![(0.0, result)];
    d.push(4.0, shots, None);
    d.end_caption();
    if !d.rf.log.is_empty() {
        eprintln!("control channel errors:\n{}", d.rf.log.join("\n"));
    }

    // ---- the drawing against the fixture
    let saved = std::fs::read_to_string(&fig).map_err(|e| format!("{}: {e}", fig.display()))?;
    let ex = root.join("examples/inclined-plane");
    let fixture = ex.join("inclined-plane.fig");
    if std::env::var_os("MOVIE_UPDATE").is_some() || !fixture.is_file() {
        std::fs::create_dir_all(&ex).map_err(|e| e.to_string())?;
        std::fs::write(&fixture, &saved).map_err(|e| e.to_string())?;
        preview(&pdf, &ex.join("inclined-plane.png"))?;
        println!("movie: wrote {}", fixture.display());
    } else {
        let want = std::fs::read_to_string(&fixture).map_err(|e| e.to_string())?;
        if fig_objects(&want) != fig_objects(&saved) {
            return Err(format!(
                "the drawn figure differs from {} (saved: {}); MOVIE_UPDATE=1 takes it",
                fixture.display(),
                fig.display()
            ));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);

    let st = d.state()?;
    let ppp = rect_of(&st, "Window area: Indicator panel")
        .map(|r| pw / r[2])
        .filter(|p| p.is_finite() && *p > 0.0)
        .unwrap_or(2.0);
    let movie = Movie {
        segs: d.segs,
        ppp,
        win: (pw / ppp, ph / ppp),
        buttons,
        cues: d.cues,
        width: 1920,
        fps: 30,
    };
    drop(d.rf);
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    movie_render::render(&movie, out, "inclined-plane", work)?;
    println!("movie: inclined-plane written to {}", out.display());
    Ok(())
}
