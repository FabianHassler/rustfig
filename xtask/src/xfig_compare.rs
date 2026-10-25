//! `cargo xtask xfig-compare` (plan W22): drive the real xfig (headless, under Xvfb) and RustFig
//! through the same mouse and key steps, screenshot both, compare the saved `.fig` files and write
//! `target/xfig-compare/index.html` (worst match first). Not part of `ci`; skips with a message
//! when xfig, Xvfb, xdotool or the image tools are missing.
//!
//! xfig is run with its default options (`/opt/homebrew/bin/xfig`, `HOME` pointing at a scratch
//! directory so the user's recent-file list and settings do not leak in). Positions of the mode
//! and indicator buttons come from xfig's `w_modepanel.c` / `w_indpanel.c` (button order) and from
//! a calibration screenshot of the 1400x1000 window; RustFig is driven through its control channel.
//!
//! Environment: `XFIG_COMPARE_FILTER=substring` runs the scenarios whose id contains it;
//! `XFIG_COMPARE_DISPLAY=:77`; `XFIG_COMPARE_PORT=8031`; `XFIG_BIN` (default `/opt/homebrew/bin/xfig`).

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Window size in points / pixels, the same for both apps.
const WIN_W: u32 = 1400;
const WIN_H: u32 = 1000;
/// xfig's canvas origin in the window (pixels) at zoom 1: Fig (0,0).
const XF_CX: f64 = 75.0;
const XF_CY: f64 = 74.0;
/// Fig units per pixel at zoom 1 (1200 per inch, 80 pixels per inch).
const FU_PER_PX: f64 = 15.0;
/// Canvas crop compared (pixels), and where xfig's menu entries sit (File menu, Save).
const CROP_W: u32 = 1000;
const CROP_H: u32 = 680;

const FIXTURE: &str =
    "#FIG 3.2  Produced by xfig version 3.2.9a\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 1200 1200 3600 1200 3600 2400 1200 2400 1200 1200\n\
1 3 0 1 0 7 50 -1 -1 0.000 1 0.0000 6000 1800 600 600 6000 1800 6600 1800\n\
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 3\n\t 1200 3600 2400 4800 3600 3600\n\
4 0 0 50 -1 0 12 0.0000 4 135 480 6000 4200 Hello\\001\n";

#[derive(Clone, Copy, PartialEq)]
enum Btn {
    L,
    M,
}

impl Btn {
    fn xdo(self) -> &'static str {
        match self {
            Btn::L => "1",
            Btn::M => "2",
        }
    }
    fn name(self) -> &'static str {
        match self {
            Btn::L => "left",
            Btn::M => "middle",
        }
    }
}

enum Step {
    /// Select a mode by its key (see `MODES`).
    Mode(&'static str),
    /// Move the pointer to a Fig position, then click.
    Click(f64, f64, Btn),
    /// Move the pointer only.
    Move(f64, f64),
    /// Type text (xdotool type / ui.key text).
    Type(&'static str),
    /// Open a menu and keep it open: (xfig x, label).
    Menu(i32, &'static str),
    /// Click an indicator-panel button: (xfig x, y 962, RustFig label prefix).
    Indicator(i32, &'static str),
    Wait(u64),
    /// Screenshot of both apps under this name.
    Shot(&'static str),
    /// Close what is open (menu release / Escape).
    Close,
}

struct Scenario {
    id: String,
    group: &'static str,
    steps: Vec<Step>,
    save: bool,
    note: &'static str,
}

/// (key, draw panel?, index in the panel, RustFig label). Order follows xfig's `mode_switches[]`.
const MODES: &[(&str, bool, usize, &str)] = &[
    ("ellipse_radius", true, 0, "Circle (radius)"),
    ("ellipse_diameter", true, 1, "Circle (diameter)"),
    ("closed_spline", true, 2, "Closed approximated spline"),
    ("spline", true, 3, "Approximated spline"),
    ("closed_interp_spline", true, 4, "Closed interpolated spline"),
    ("interp_spline", true, 5, "Interpolated spline"),
    ("polygon", true, 6, "Polygon"),
    ("polyline", true, 7, "Polyline"),
    ("box", true, 8, "Box"),
    ("arcbox", true, 9, "Arc-box (rounded)"),
    ("regpoly", true, 10, "Regular polygon"),
    ("arc", true, 11, "Arc"),
    ("picture", true, 12, "Picture"),
    ("text", true, 13, "Text"),
    ("library", true, 14, "Library"),
    ("glue", false, 0, "Glue into compound"),
    ("break", false, 1, "Break compound"),
    ("open_compound", false, 2, "Open compound"),
    ("join_split", false, 3, "Join/split lines and splines"),
    ("chop", false, 4, "Chop"),
    ("scale", false, 5, "Scale"),
    ("align", false, 6, "Align"),
    ("move_point", false, 7, "Move point"),
    ("move", false, 8, "Move"),
    ("add_point", false, 9, "Add point"),
    ("copy", false, 10, "Copy"),
    ("delete_point", false, 11, "Delete point"),
    ("delete", false, 12, "Delete"),
    ("update", false, 13, "Update attributes"),
    ("edit", false, 14, "Edit attributes"),
    ("flip_ud", false, 15, "Flip up/down"),
    ("flip_lr", false, 16, "Flip left/right"),
    ("rotate_cw", false, 17, "Rotate clockwise"),
    ("rotate_ccw", false, 18, "Rotate counter-clockwise"),
    ("convert", false, 19, "Convert line ↔ spline"),
    ("autoarrow", false, 20, "Add/remove arrows"),
    ("tangent", false, 21, "Add tangent/normal to curve"),
    ("angle_measure", false, 22, "Measure angle"),
    ("length_measure", false, 23, "Measure length"),
    ("area_measure", false, 24, "Measure area"),
];

/// xfig's mode button centre (window pixels).
fn xfig_mode_xy(draw: bool, idx: usize) -> (i32, i32) {
    let (col, row) = (idx % 2, idx / 2);
    let x = if draw && idx == 14 {
        37
    } else if col == 0 {
        19
    } else {
        56
    };
    let y0 = if draw { 167.0 } else { 533.0 };
    (x, (y0 + row as f64 * 32.8).round() as i32)
}

fn scenarios() -> Vec<Scenario> {
    use Btn::{L, M};
    let mut v: Vec<Scenario> = Vec::new();
    let mut add = |id: &str, group: &'static str, steps: Vec<Step>, save: bool, note: &'static str| {
        v.push(Scenario {
            id: id.into(),
            group,
            steps,
            save,
            note,
        });
    };

    // Every mode selected: mode panel, indicator panel, message and mouse-button hints.
    for (key, ..) in MODES {
        add(
            &format!("mode-{key}"),
            "modes",
            vec![
                Step::Mode(key),
                Step::Move(9000.0, 9000.0),
                Step::Wait(300),
                Step::Shot("mode"),
            ],
            false,
            "pointer on the canvas, so the mouse-button hints are the mode's",
        );
    }

    // Draw one object per drawing mode, default attributes, in the empty right half.
    let (x0, y0, x1, y1) = (9000.0, 2000.0, 13000.0, 5000.0);
    let two = |m: &'static str| {
        vec![
            Step::Mode(m),
            Step::Click(x0, y0, L),
            Step::Click(x1, y1, L),
            Step::Shot("drawn"),
        ]
    };
    add("draw-box", "draw", two("box"), true, "");
    add("draw-arcbox", "draw", two("arcbox"), true, "");
    add("draw-circle-radius", "draw", two("ellipse_radius"), true, "");
    add("draw-circle-diameter", "draw", two("ellipse_diameter"), true, "");
    add("draw-regpoly", "draw", two("regpoly"), true, "");
    let poly = |m: &'static str| {
        vec![
            Step::Mode(m),
            Step::Click(9000.0, 2000.0, L),
            Step::Click(13000.0, 2500.0, L),
            Step::Click(11000.0, 5500.0, L),
            Step::Click(9000.0, 6000.0, M),
            Step::Shot("drawn"),
        ]
    };
    add(
        "draw-polyline",
        "draw",
        poly("polyline"),
        true,
        "points by left click, middle click ends",
    );
    add("draw-polygon", "draw", poly("polygon"), true, "");
    add("draw-spline", "draw", poly("spline"), true, "");
    add("draw-closed-spline", "draw", poly("closed_spline"), true, "");
    add("draw-interp-spline", "draw", poly("interp_spline"), true, "");
    add(
        "draw-closed-interp-spline",
        "draw",
        poly("closed_interp_spline"),
        true,
        "",
    );
    add(
        "draw-arc",
        "draw",
        vec![
            Step::Mode("arc"),
            Step::Click(9000.0, 4000.0, L),
            Step::Click(11000.0, 2000.0, L),
            Step::Click(13000.0, 4000.0, L),
            Step::Shot("drawn"),
        ],
        true,
        "three points",
    );
    add(
        "draw-text",
        "draw",
        vec![
            Step::Mode("text"),
            Step::Click(9000.0, 3000.0, L),
            Step::Type("Hello xfig"),
            Step::Wait(200),
            Step::Shot("typing"),
            Step::Mode("move"),
            Step::Shot("done"),
        ],
        true,
        "text typed, ended by choosing another mode",
    );
    add(
        "draw-library",
        "draw",
        vec![Step::Mode("library"), Step::Wait(800), Step::Shot("library")],
        false,
        "library dialog",
    );
    add(
        "draw-picture",
        "draw",
        vec![
            Step::Mode("picture"),
            Step::Click(9000.0, 3000.0, L),
            Step::Wait(800),
            Step::Shot("dialog"),
        ],
        false,
        "picture dialog",
    );

    // Editing modes on the fixture: box (1200,1200)-(3600,2400), circle at (6000,1800) r 600,
    // polyline (1200,3600) (2400,4800) (3600,3600), text "Hello" at (6000,4200).
    let on_box = (1200.0, 1800.0);
    let on_line = (1800.0, 4200.0);
    let on_circle = (6600.0, 1800.0);
    add(
        "edit-move",
        "edit",
        vec![
            Step::Mode("move"),
            Step::Click(on_box.0, on_box.1, L),
            Step::Click(on_box.0 + 3000.0, on_box.1 + 6000.0, L),
            Step::Shot("moved"),
        ],
        true,
        "",
    );
    add(
        "edit-copy",
        "edit",
        vec![
            Step::Mode("copy"),
            Step::Click(on_box.0, on_box.1, L),
            Step::Click(on_box.0 + 3000.0, on_box.1 + 6000.0, L),
            Step::Shot("copied"),
        ],
        true,
        "",
    );
    add(
        "edit-delete",
        "edit",
        vec![
            Step::Mode("delete"),
            Step::Click(on_box.0, on_box.1, L),
            Step::Shot("deleted"),
        ],
        true,
        "",
    );
    add(
        "edit-flip-ud",
        "edit",
        vec![
            Step::Mode("flip_ud"),
            Step::Click(on_line.0, on_line.1, L),
            Step::Shot("flipped"),
        ],
        true,
        "",
    );
    add(
        "edit-flip-lr",
        "edit",
        vec![
            Step::Mode("flip_lr"),
            Step::Click(on_line.0, on_line.1, L),
            Step::Shot("flipped"),
        ],
        true,
        "",
    );
    add(
        "edit-rotate-cw",
        "edit",
        vec![
            Step::Mode("rotate_cw"),
            Step::Click(on_line.0, on_line.1, L),
            Step::Shot("rotated"),
        ],
        true,
        "",
    );
    add(
        "edit-rotate-ccw",
        "edit",
        vec![
            Step::Mode("rotate_ccw"),
            Step::Click(on_line.0, on_line.1, L),
            Step::Shot("rotated"),
        ],
        true,
        "",
    );
    add(
        "edit-scale",
        "edit",
        vec![
            Step::Mode("scale"),
            Step::Click(3600.0, 2400.0, L),
            Step::Click(4800.0, 3000.0, L),
            Step::Shot("scaled"),
        ],
        true,
        "pick the corner, click the new corner",
    );
    add(
        "edit-move-point",
        "edit",
        vec![
            Step::Mode("move_point"),
            Step::Click(2400.0, 4800.0, L),
            Step::Click(2400.0, 6000.0, L),
            Step::Shot("moved"),
        ],
        true,
        "",
    );
    add(
        "edit-add-point",
        "edit",
        vec![
            Step::Mode("add_point"),
            Step::Click(1800.0, 4200.0, L),
            Step::Click(1500.0, 5400.0, L),
            Step::Shot("added"),
        ],
        true,
        "",
    );
    add(
        "edit-delete-point",
        "edit",
        vec![
            Step::Mode("delete_point"),
            Step::Click(2400.0, 4800.0, L),
            Step::Shot("deleted"),
        ],
        true,
        "",
    );
    add(
        "edit-update",
        "edit",
        vec![
            Step::Mode("update"),
            Step::Click(on_circle.0, on_circle.1, L),
            Step::Shot("updated"),
        ],
        true,
        "updates with the default attributes",
    );
    add(
        "edit-popup-box",
        "edit",
        vec![
            Step::Mode("edit"),
            Step::Click(on_box.0, on_box.1, L),
            Step::Wait(800),
            Step::Shot("popup"),
        ],
        false,
        "edit panel of a box",
    );
    add(
        "edit-popup-circle",
        "edit",
        vec![
            Step::Mode("edit"),
            Step::Click(on_circle.0, on_circle.1, L),
            Step::Wait(800),
            Step::Shot("popup"),
        ],
        false,
        "edit panel of a circle",
    );
    add(
        "edit-popup-polyline",
        "edit",
        vec![
            Step::Mode("edit"),
            Step::Click(on_line.0, on_line.1, L),
            Step::Wait(800),
            Step::Shot("popup"),
        ],
        false,
        "edit panel of a polyline",
    );
    add(
        "edit-popup-text",
        "edit",
        vec![
            Step::Mode("edit"),
            Step::Click(6100.0, 4150.0, L),
            Step::Wait(800),
            Step::Shot("popup"),
        ],
        false,
        "edit panel of a text",
    );
    add(
        "edit-convert",
        "edit",
        vec![
            Step::Mode("convert"),
            Step::Click(on_line.0, on_line.1, L),
            Step::Shot("converted"),
        ],
        true,
        "polyline to spline",
    );
    add(
        "edit-autoarrow",
        "edit",
        vec![
            Step::Mode("autoarrow"),
            Step::Click(3600.0, 3600.0, L),
            Step::Shot("arrow"),
        ],
        true,
        "",
    );

    // Menus (xfig: press and hold the menu button).
    for (name, x) in [("File", 30), ("Edit", 91), ("View", 152), ("Snap", 213), ("Help", 274)] {
        add(
            &format!("menu-{}", name.to_lowercase()),
            "menus",
            vec![Step::Menu(x, name), Step::Wait(700), Step::Shot("menu"), Step::Close],
            false,
            "",
        );
    }

    // Indicator pop-ups (box mode: line width, style, fill, colours, depth, join, cap, grid, point position).
    for (id, x, label) in [
        ("line-width", 532, "Line Width"),
        ("line-style", 597, "Line Style"),
        ("fill-style", 466, "Fill Style"),
        ("pen-colour", 295, "PenColor"),
        ("fill-colour", 387, "FillColor"),
        ("depth", 220, "Depth"),
        ("join-style", 663, "Join Style"),
        ("cap-style", 729, "Cap Style"),
        ("grid-mode", 92, "Grid Mode"),
        ("point-position", 158, "Point Posn"),
        ("zoom", 29, "Zoom"),
    ] {
        add(
            &format!("ind-{id}"),
            "indicator",
            vec![
                Step::Mode("box"),
                Step::Indicator(x, label),
                Step::Wait(700),
                Step::Shot("popup"),
            ],
            false,
            "box mode",
        );
    }
    // Polyline mode: arrow buttons sit further right (xfig layout after Angle Geom/Fill/Line columns).
    for (id, x, label) in [
        ("arrow-mode", 860, "Arrow Mode"),
        ("arrow-type", 928, "Arrow Type"),
        ("arrow-size", 1000, "Arrow Size"),
    ] {
        add(
            &format!("ind-{id}"),
            "indicator",
            vec![
                Step::Mode("polyline"),
                Step::Indicator(x, label),
                Step::Wait(700),
                Step::Shot("popup"),
            ],
            false,
            "polyline mode",
        );
    }
    // Text mode.
    for (id, x, label) in [
        ("text-just", 375, "Text Just"),
        ("text-angle", 445, "Text/Ellipse"),
        ("text-flags", 530, "Text Flags"),
        ("text-size", 612, "Text Size"),
        ("text-font", 800, "Text Font"),
    ] {
        add(
            &format!("ind-{id}"),
            "indicator",
            vec![
                Step::Mode("text"),
                Step::Indicator(x, label),
                Step::Wait(700),
                Step::Shot("popup"),
            ],
            false,
            "text mode",
        );
    }
    v
}

// ---------------------------------------------------------------- tools

fn have(tool: &str, extra_dirs: &[&str]) -> bool {
    let path = std::env::var("PATH").unwrap_or_default();
    path.split(':')
        .chain(extra_dirs.iter().copied())
        .any(|d| Path::new(d).join(tool).is_file())
}

fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

fn sh(display: &str, script: &str) -> Result<(), String> {
    let st = Command::new("sh")
        .arg("-c")
        .arg(script)
        .env("DISPLAY", display)
        .env(
            "PATH",
            format!(
                "/opt/homebrew/bin:/opt/X11/bin:{}",
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("sh: {e}"))?;
    if st.success() {
        Ok(())
    } else {
        Err(format!("failed: {script}"))
    }
}

fn xdo(display: &str, args: &[&str]) {
    // A failing xdotool call shows up as a missing effect in the screenshot; the run goes on.
    let _ = Command::new("xdotool")
        .args(args)
        .env("DISPLAY", display)
        .env(
            "PATH",
            format!("/opt/homebrew/bin:{}", std::env::var("PATH").unwrap_or_default()),
        )
        .stderr(Stdio::null())
        .status();
}

fn xfig_shot(display: &str, out: &Path) -> Result<(), String> {
    sh(
        display,
        &format!(
            "xwd -root -display {display} | xwdtopnm | pnmtopng > '{}'",
            out.display()
        ),
    )
}

fn magick(args: &[&str]) -> Result<String, String> {
    let o = Command::new("magick")
        .args(args)
        .env(
            "PATH",
            format!("/opt/homebrew/bin:{}", std::env::var("PATH").unwrap_or_default()),
        )
        .output()
        .map_err(|e| format!("magick: {e}"))?;
    // `compare` reports on stderr and exits 1 when the images differ.
    Ok(String::from_utf8_lossy(&o.stderr).to_string() + &String::from_utf8_lossy(&o.stdout))
}

// ---------------------------------------------------------------- RustFig control channel

struct Rf {
    child: Child,
    stream: TcpStream,
    reader: BufReader<TcpStream>,
    id: u64,
    /// Canvas top-left (points) and Fig position of that corner.
    canvas: (f64, f64),
    origin: (f64, f64),
    log: Vec<String>,
}

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn numbers_after(s: &str, key: &str, n: usize) -> Option<Vec<f64>> {
    let i = s.find(key)? + key.len();
    let rest = s.get(i..)?;
    let end = rest.find(']')?;
    let v: Vec<f64> = rest
        .get(..end)?
        .split(',')
        .filter_map(|t| t.trim().parse().ok())
        .collect();
    (v.len() >= n).then_some(v)
}

impl Rf {
    fn start(bin: &Path, port: u16, dir: &Path, file: &Path) -> Result<Rf, String> {
        let child = Command::new(bin)
            .args([
                "--control",
                &port.to_string(),
                "--background",
                "-geometry",
                &format!("{WIN_W}x{WIN_H}+0+0"),
            ])
            .arg(file)
            .env("RUSTFIG_SETTINGS_DIR", dir.join("settings"))
            .env("RUSTFIG_RECOVERY_DIR", dir.join("recovery"))
            .env("FIG2DEV_BUILTIN_FONTS", "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("start rustfig: {e}"))?;
        let t = Instant::now();
        let stream = loop {
            if let Ok(s) = TcpStream::connect(("127.0.0.1", port)) {
                break s;
            }
            if t.elapsed() > Duration::from_secs(30) {
                return Err("RustFig control port did not open".into());
            }
            sleep_ms(200);
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
        let reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
        let mut rf = Rf {
            child,
            stream,
            reader,
            id: 0,
            canvas: (0.0, 0.0),
            origin: (0.0, 0.0),
            log: Vec::new(),
        };
        rf.call("app.ping", "{}")?;
        rf.call("ui.run", r#"{"command":"app.theme","params":{"theme":"light"}}"#)?;
        rf.call("engine.execute", r#"{"command":"view.zoom_set","params":{"zoom":1}}"#)?;
        rf.layout()?;
        // Fig (0,0) at the canvas corner, as in xfig.
        let s = 80.0 / 1200.0;
        let (ox, oy) = rf.origin;
        rf.call(
            "engine.execute",
            &format!(
                r#"{{"command":"view.pan","params":{{"dx":{},"dy":{}}}}}"#,
                ox * s,
                oy * s
            ),
        )?;
        rf.layout()?;
        Ok(rf)
    }

    fn call(&mut self, method: &str, params: &str) -> Result<String, String> {
        self.id += 1;
        let line = format!("{{\"id\":{},\"method\":\"{method}\",\"params\":{params}}}\n", self.id);
        self.stream.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        let mut reply = String::new();
        self.reader
            .read_line(&mut reply)
            .map_err(|e| format!("{method}: {e}"))?;
        if reply.contains("\"error\"") {
            self.log.push(format!(
                "{method} {params}: {}",
                reply.trim().chars().take(160).collect::<String>()
            ));
        }
        Ok(reply)
    }

    fn layout(&mut self) -> Result<(), String> {
        let s = self.call("ui.state", "{}")?;
        if let Some(o) = numbers_after(&s, "\"origin\":[", 2) {
            self.origin = (o[0], o[1]);
        }
        if let Some(i) = s.find("\"label\":\"Canvas:") {
            if let Some(r) = s.get(i..).and_then(|t| numbers_after(t, "\"rect\":[", 2)) {
                self.canvas = (r[0], r[1]);
            }
        }
        Ok(())
    }

    fn pt(&self, x: f64, y: f64) -> (f64, f64) {
        let s = 80.0 / 1200.0;
        (
            self.canvas.0 + (x - self.origin.0) * s,
            self.canvas.1 + (y - self.origin.1) * s,
        )
    }

    fn pointer(&mut self, action: &str, x: f64, y: f64, b: Btn) -> Result<(), String> {
        let (px, py) = self.pt(x, y);
        self.call(
            "ui.pointer",
            &format!(r#"{{"action":"{action}","x":{px},"y":{py},"button":"{}"}}"#, b.name()),
        )?;
        Ok(())
    }

    fn click_label(&mut self, label: &str) -> Result<(), String> {
        let r = self.call("ui.click_label", &format!(r#"{{"label":{}}}"#, json_str(label)))?;
        if r.contains("\"error\"") {
            self.log.push(format!("no widget labelled {label:?}"));
        }
        Ok(())
    }

    fn shot(&mut self, path: &Path) -> Result<(), String> {
        self.call(
            "ui.screenshot",
            &format!(r#"{{"path":{}}}"#, json_str(&path.display().to_string())),
        )?;
        Ok(())
    }
}

impl Drop for Rf {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ---------------------------------------------------------------- .fig comparison

/// The object lines of a .fig file (header, comments and blank lines dropped, whitespace normalised).
fn fig_objects(text: &str) -> Vec<String> {
    let mut lines = text.lines().map(str::trim_end);
    // header: up to the "<resolution> <coord system>" line
    for l in lines.by_ref() {
        let t: Vec<&str> = l.split_whitespace().collect();
        if t.len() == 2 && t.iter().all(|x| x.parse::<i64>().is_ok()) {
            break;
        }
    }
    lines
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(normalise)
        .collect()
}

/// Numbers printed alike ("12.00" = "12"); a text's height and length (fields 9 and 10) depend on
/// the fonts of the machine and are left out.
fn normalise(line: &str) -> String {
    let t: Vec<&str> = line.split_whitespace().collect();
    let is_text = t.first() == Some(&"4");
    let mut out: Vec<String> = Vec::new();
    for (i, w) in t.iter().enumerate() {
        if is_text && (i == 9 || i == 10) {
            out.push("*".into());
        } else if is_text && i > 12 {
            out.push((*w).to_string());
        } else if t.first() == Some(&"2") && t.get(1) != Some(&"4") && i == 12 && *w == "-1" {
            // radius: xfig writes -1 for everything but arc-boxes (RustFig 0 or its default 7)
            out.push("0".into());
        } else if let Ok(n) = w.parse::<f64>() {
            out.push(format!("{n}"));
        } else {
            out.push((*w).to_string());
        }
    }
    out.join(" ")
}

fn kind(line: &str) -> &'static str {
    match line.split_whitespace().next() {
        Some("0") => "colour",
        Some("1") => "ellipse",
        Some("2") => "polyline",
        Some("3") => "spline",
        Some("4") => "text",
        Some("5") => "arc",
        Some("6") => "compound",
        Some("-6") => "end compound",
        _ => "",
    }
}

/// Differences between two object lists as readable lines (empty when equal).
fn fig_diff(a: &[String], b: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i), b.get(i));
        if x != y {
            let k = x.or(y).map_or("", |l| kind(l));
            out.push(format!(
                "[{i}] {k}\n  xfig:    {}\n  RustFig: {}",
                x.map_or("(none)", String::as_str),
                y.map_or("(none)", String::as_str)
            ));
        }
    }
    out
}

// ---------------------------------------------------------------- running one scenario

struct Outcome {
    id: String,
    group: &'static str,
    note: String,
    shots: Vec<String>,
    full_rmse: f64,
    canvas_rmse: f64,
    diff: Vec<String>,
    fig_note: String,
    errors: Vec<String>,
}

struct Env {
    display: String,
    port: u16,
    xfig: String,
    rustfig: PathBuf,
    out: PathBuf,
}

fn wait_for_xfig(display: &str) -> bool {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(20) {
        let o = Command::new("xdotool")
            .args(["search", "--onlyvisible", "--name", "xfig"])
            .env("DISPLAY", display)
            .env(
                "PATH",
                format!("/opt/homebrew/bin:{}", std::env::var("PATH").unwrap_or_default()),
            )
            .output();
        if o.is_ok_and(|o| !o.stdout.is_empty()) {
            sleep_ms(1500);
            return true;
        }
        sleep_ms(300);
    }
    false
}

fn rmse(a: &Path, b: &Path, diff: &Path) -> f64 {
    let r = magick(&[
        "compare",
        "-metric",
        "RMSE",
        &a.display().to_string(),
        &b.display().to_string(),
        &diff.display().to_string(),
    ])
    .unwrap_or_default();
    // "1234.5 (0.0188)"
    r.split('(')
        .nth(1)
        .and_then(|t| t.split(')').next())
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(1.0)
}

fn run_scenario(env: &Env, sc: &Scenario) -> Result<Outcome, String> {
    let dir = env.out.join("work").join(&sc.id);
    let img = env.out.join("img").join(&sc.id);
    for d in [
        &dir,
        &img,
        &dir.join("home"),
        &dir.join("settings"),
        &dir.join("recovery"),
    ] {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    let (xf_fig, rf_fig) = (dir.join("xfig.fig"), dir.join("rustfig.fig"));
    std::fs::write(&xf_fig, FIXTURE).map_err(|e| e.to_string())?;
    std::fs::write(&rf_fig, FIXTURE).map_err(|e| e.to_string())?;

    let d = env.display.as_str();
    let mut xfig = Command::new(&env.xfig)
        .args(["-geometry", &format!("{WIN_W}x{WIN_H}+0+0")])
        .arg(&xf_fig)
        .env("DISPLAY", d)
        .env("HOME", dir.join("home"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("start xfig: {e}"))?;
    let started = (|| -> Result<Rf, String> {
        if !wait_for_xfig(d) {
            return Err("xfig window did not appear".into());
        }
        Rf::start(&env.rustfig, env.port, &dir, &rf_fig)
    })();
    let mut rf = match started {
        Ok(r) => r,
        Err(e) => {
            let _ = xfig.kill();
            let _ = xfig.wait();
            return Err(e);
        }
    };
    let result = drive(env, sc, &mut rf, &dir, &img);
    // saving (both apps) before shutting down
    let _ = xfig.kill();
    let _ = xfig.wait();
    drop(rf);
    sleep_ms(500);
    result
}

/// Move both pointers to a Fig position (xfig gets a short approach so its motion handlers run).
/// Both apps snap to 1/16 inch (75 Fig units) by default; clicking on the grid avoids
/// rounding differences between the apps' pixel origins.
fn snap(v: f64) -> f64 {
    (v / 75.0).round() * 75.0
}

fn pointer_to(d: &str, rf: &mut Rf, x: f64, y: f64, px: i32, py: i32) -> Result<(), String> {
    xdo(d, &["mousemove", &(px - 3).to_string(), &(py - 3).to_string()]);
    sleep_ms(120);
    xdo(d, &["mousemove", &px.to_string(), &py.to_string()]);
    sleep_ms(150);
    rf.pointer("move", x, y, Btn::L)
}

fn drive(env: &Env, sc: &Scenario, rf: &mut Rf, dir: &Path, img: &Path) -> Result<Outcome, String> {
    let d = env.display.as_str();
    let fx = |x: f64| (XF_CX + x / FU_PER_PX).round() as i32;
    let fy = |y: f64| (XF_CY + y / FU_PER_PX).round() as i32;
    let mut shots: Vec<String> = Vec::new();
    let (mut full_worst, mut canvas_worst) = (0.0f64, 0.0f64);
    xdo(d, &["mousemove", "700", "990"]);
    sleep_ms(300);
    for step in &sc.steps {
        match step {
            Step::Mode(key) => {
                let Some((_, draw, idx, label)) = MODES.iter().find(|m| m.0 == *key) else {
                    return Err(format!("unknown mode {key}"));
                };
                let (x, y) = xfig_mode_xy(*draw, *idx);
                xdo(d, &["mousemove", &x.to_string(), &y.to_string()]);
                sleep_ms(150);
                xdo(d, &["click", "1"]);
                sleep_ms(300);
                rf.click_label(label)?;
            }
            Step::Click(x, y, b) => {
                let (x, y) = (&snap(*x), &snap(*y));
                pointer_to(d, rf, *x, *y, fx(*x), fy(*y))?;
                xdo(d, &["click", b.xdo()]);
                rf.pointer("click", *x, *y, *b)?;
                sleep_ms(250);
            }
            Step::Move(x, y) => {
                let (x, y) = (&snap(*x), &snap(*y));
                pointer_to(d, rf, *x, *y, fx(*x), fy(*y))?;
                sleep_ms(250);
            }
            Step::Type(t) => {
                xdo(d, &["type", "--delay", "90", t]);
                rf.call("ui.key", &format!(r#"{{"key":"A","text":{}}}"#, json_str(t)))?;
                sleep_ms(300);
            }
            Step::Menu(x, label) => {
                xdo(d, &["mousemove", &x.to_string(), "12"]);
                sleep_ms(150);
                xdo(d, &["mousedown", "1"]);
                sleep_ms(300);
                rf.click_label(label)?;
            }
            Step::Indicator(x, label) => {
                xdo(d, &["mousemove", &x.to_string(), "962"]);
                sleep_ms(150);
                xdo(d, &["click", "1"]);
                sleep_ms(300);
                rf.click_label(label)?;
            }
            Step::Wait(ms) => sleep_ms(*ms),
            Step::Close => {
                xdo(d, &["mousemove", "700", "500"]);
                xdo(d, &["mouseup", "1"]);
                xdo(d, &["key", "Escape"]);
                rf.call("ui.key", r#"{"key":"Escape"}"#)?;
            }
            Step::Shot(name) => {
                sleep_ms(500);
                let (xr, rr) = (
                    img.join(format!("{name}-xfig.png")),
                    img.join(format!("{name}-rustfig.png")),
                );
                let (xw, rw) = (img.join("xfig-raw.png"), img.join("rustfig-raw.png"));
                xfig_shot(d, &xw)?;
                rf.shot(&rw)?;
                rf.layout()?;
                let geom = format!("{WIN_W}x{WIN_H}");
                magick(&[
                    &xw.display().to_string(),
                    "-crop",
                    &format!("{geom}+0+0"),
                    "+repage",
                    &xr.display().to_string(),
                ])?;
                // RustFig's window is clipped to the screen height and drawn at 2x: scale to 1400 wide,
                // pad to xfig's window size.
                magick(&[
                    &rw.display().to_string(),
                    "-resize",
                    &format!("{WIN_W}x"),
                    "-background",
                    "#808080",
                    "-gravity",
                    "NorthWest",
                    "-extent",
                    &geom,
                    &rr.display().to_string(),
                ])?;
                let df = img.join(format!("{name}-diff.png"));
                full_worst = full_worst.max(rmse(&xr, &rr, &df));
                // canvas crops: xfig at (75,75); RustFig at its canvas corner (points = pixels here)
                let crop = format!("{CROP_W}x{CROP_H}");
                let (cx, cy) = (rf.canvas.0.round() as i32, rf.canvas.1.round() as i32);
                let (xc, rc, dc) = (
                    img.join(format!("{name}-canvas-xfig.png")),
                    img.join(format!("{name}-canvas-rustfig.png")),
                    img.join(format!("{name}-canvas-diff.png")),
                );
                magick(&[
                    &xr.display().to_string(),
                    "-crop",
                    &format!("{crop}+{}+{}", XF_CX as i32, XF_CY as i32),
                    "+repage",
                    &xc.display().to_string(),
                ])?;
                magick(&[
                    &rr.display().to_string(),
                    "-crop",
                    &format!("{crop}+{cx}+{cy}"),
                    "+repage",
                    &rc.display().to_string(),
                ])?;
                canvas_worst = canvas_worst.max(rmse(&xc, &rc, &dc));
                shots.push(name.to_string());
            }
        }
    }
    let mut diff = Vec::new();
    let mut fig_note = String::new();
    if sc.save {
        // xfig: File menu, Save (hold the button, release on the entry); RustFig: file.save.
        xdo(d, &["mousemove", "30", "12"]);
        sleep_ms(150);
        xdo(d, &["mousedown", "1"]);
        sleep_ms(300);
        xdo(d, &["mousemove", "25", "70"]);
        sleep_ms(150);
        xdo(d, &["mousemove", "20", "83"]);
        sleep_ms(300);
        xdo(d, &["mouseup", "1"]);
        sleep_ms(1200);
        rf.call(
            "engine.execute",
            &format!(
                r#"{{"command":"file.save","params":{{"path":{}}}}}"#,
                json_str(&dir.join("rustfig.fig").display().to_string())
            ),
        )?;
        let a = std::fs::read_to_string(dir.join("xfig.fig")).map_err(|e| e.to_string())?;
        let b = std::fs::read_to_string(dir.join("rustfig.fig")).map_err(|e| e.to_string())?;
        diff = fig_diff(&fig_objects(&a), &fig_objects(&b));
        fig_note = if a.trim() == FIXTURE.trim() {
            "xfig file unchanged (save may not have run)".into()
        } else {
            String::new()
        };
    }
    Ok(Outcome {
        id: sc.id.clone(),
        group: sc.group,
        note: sc.note.into(),
        shots,
        full_rmse: full_worst,
        canvas_rmse: canvas_worst,
        diff,
        fig_note,
        errors: std::mem::take(&mut rf.log),
    })
}

// ---------------------------------------------------------------- report

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn report(out: &Path, mut rows: Vec<Outcome>, failed: &[(String, String)]) -> Result<(), String> {
    // worst first: canvas weighs double (that is where drawing results show)
    rows.sort_by(|a, b| {
        let (sa, sb) = (a.canvas_rmse * 2.0 + a.full_rmse, b.canvas_rmse * 2.0 + b.full_rmse);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut h = String::from("<!doctype html><meta charset=utf-8><title>xfig vs RustFig</title><style>body{font:13px sans-serif;margin:16px}table{border-collapse:collapse}td,th{border:1px solid #bbb;padding:4px;vertical-align:top}img{max-width:420px;display:block;margin:2px 0}pre{font-size:11px;margin:0;white-space:pre-wrap}.bad{background:#fdd}</style><h1>xfig (real) vs RustFig</h1><p>Scores are RMSE (0 = identical, 1 = opposite) of the whole window and of the canvas crop, worst first. Images: xfig | RustFig | difference.</p><table><tr><th>scenario</th><th>full</th><th>canvas</th><th>images</th><th>.fig object diff (xfig vs RustFig)</th><th>notes</th></tr>");
    for r in &rows {
        h.push_str(&format!(
            "<tr><td><b>{}</b><br>{}</td><td>{:.4}</td><td>{:.4}</td><td>",
            esc(&r.id),
            r.group,
            r.full_rmse,
            r.canvas_rmse
        ));
        for s in &r.shots {
            for (suffix, label) in [("", "window"), ("-canvas", "canvas")] {
                h.push_str(&format!("<div>{s} {label}:</div><div style=\"display:flex;gap:4px\">"));
                for k in ["xfig", "rustfig", "diff"] {
                    let f = format!("img/{}/{s}{suffix}-{k}.png", r.id);
                    h.push_str(&format!("<a href=\"{f}\"><img loading=lazy src=\"{f}\"></a>"));
                }
                h.push_str("</div>");
            }
        }
        h.push_str("</td><td>");
        if r.diff.is_empty() {
            h.push_str(if r.fig_note.is_empty() {
                "equal or not saved"
            } else {
                &r.fig_note
            });
        } else {
            h.push_str(&format!("<pre class=bad>{}</pre>", esc(&r.diff.join("\n"))));
        }
        h.push_str("</td><td>");
        h.push_str(&esc(&r.note));
        for e in &r.errors {
            h.push_str(&format!("<br><i>{}</i>", esc(e)));
        }
        h.push_str("</td></tr>");
    }
    h.push_str("</table>");
    if !failed.is_empty() {
        h.push_str("<h2>Scenarios that did not run</h2><ul>");
        for (id, e) in failed {
            h.push_str(&format!("<li>{}: {}</li>", esc(id), esc(e)));
        }
        h.push_str("</ul>");
    }
    std::fs::write(out.join("index.html"), h).map_err(|e| e.to_string())
}

pub fn run(root: &Path, cargo: Command) -> Result<(), String> {
    let display = std::env::var("XFIG_COMPARE_DISPLAY").unwrap_or_else(|_| ":77".into());
    let port: u16 = std::env::var("XFIG_COMPARE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8031);
    let xfig = std::env::var("XFIG_BIN").unwrap_or_else(|_| "/opt/homebrew/bin/xfig".into());
    let extra = ["/opt/homebrew/bin", "/opt/X11/bin", "/usr/bin"];
    let mut missing = Vec::new();
    if !Path::new(&xfig).is_file() {
        missing.push(xfig.clone());
    }
    for t in ["Xvfb", "xdotool", "xwd", "xwdtopnm", "pnmtopng", "magick"] {
        if !have(t, &extra) {
            missing.push(t.to_string());
        }
    }
    if !missing.is_empty() {
        println!("xfig-compare: skipped, missing {}", missing.join(", "));
        return Ok(());
    }
    drop(cargo);
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let mut b = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    b.args(["build", "--release", "-p", "rustfig"]).current_dir(root);
    eprintln!("$ {b:?}");
    if !b.status().map_err(|e| e.to_string())?.success() {
        return Err("building rustfig failed".into());
    }
    let out = root.join("target/xfig-compare");
    let _ = std::fs::remove_dir_all(out.join("img"));
    let _ = std::fs::remove_dir_all(out.join("work"));
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let env = Env {
        display: display.clone(),
        port,
        xfig,
        rustfig: target.join("release/rustfig"),
        out: out.clone(),
    };

    let mut xvfb = Command::new("Xvfb")
        .args([display.as_str(), "-screen", "0", "1600x1200x24"])
        .env(
            "PATH",
            format!("/opt/X11/bin:{}", std::env::var("PATH").unwrap_or_default()),
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Xvfb: {e}"))?;
    sleep_ms(2000);
    let filter = std::env::var("XFIG_COMPARE_FILTER").unwrap_or_default();
    let mut rows = Vec::new();
    let mut failed = Vec::new();
    let all = scenarios();
    let total = all.iter().filter(|s| s.id.contains(&filter)).count();
    for (n, sc) in all.iter().filter(|s| s.id.contains(&filter)).enumerate() {
        eprintln!("[{}/{total}] {}", n + 1, sc.id);
        match run_scenario(&env, sc) {
            Ok(o) => rows.push(o),
            Err(e) => {
                eprintln!("  failed: {e}");
                failed.push((sc.id.clone(), e));
            }
        }
    }
    let _ = xvfb.kill();
    let _ = xvfb.wait();
    let count = rows.len();
    report(&out, rows, &failed)?;
    println!(
        "xfig-compare: {count} scenarios, {} failed; report: {}",
        failed.len(),
        out.join("index.html").display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_buttons_follow_xfigs_grid() {
        assert_eq!(xfig_mode_xy(true, 0), (19, 167));
        assert_eq!(xfig_mode_xy(true, 1), (56, 167));
        assert_eq!(xfig_mode_xy(true, 14), (37, 397));
        assert_eq!(xfig_mode_xy(false, 24), (19, 927));
        assert_eq!(MODES.len(), 40);
    }

    #[test]
    fn scenario_ids_are_unique_and_modes_exist() {
        let all = scenarios();
        let mut ids: Vec<&str> = all.iter().map(|s| s.id.as_str()).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
        for s in &all {
            for st in &s.steps {
                if let Step::Mode(m) = st {
                    assert!(MODES.iter().any(|x| x.0 == *m), "{m}");
                }
            }
        }
    }

    #[test]
    fn fig_objects_skip_header_and_comments() {
        let o = fig_objects(FIXTURE);
        assert_eq!(o.len(), 6);
        assert!(fig_diff(&o, &o).is_empty());
        let mut p = o.clone();
        p[1] = "x".into();
        assert_eq!(fig_diff(&o, &p).len(), 1);
    }

    #[test]
    fn reads_numbers_from_a_reply() {
        let s = r#"{"origin":[1.5,-2],"x":1}"#;
        assert_eq!(numbers_after(s, "\"origin\":[", 2), Some(vec![1.5, -2.0]));
    }
}
