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
//! Environment: `XFIG_COMPARE_FILTER=substring[,substring...]` runs the scenarios whose id contains one;
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

/// Round 2 fixture: a compound (box + circle), two lines to join, a line to split, a polygon,
/// an approximated spline, an arc, two texts, a picture and a compound of two boxes.
const FIXTURE2: &str =
    "#FIG 3.2  Produced by xfig version 3.2.9a\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
6 1200 1200 5400 2400\n\
2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 1200 1200 3600 1200 3600 2400 1200 2400 1200 1200\n\
1 3 0 1 0 7 50 -1 -1 0.000 1 0.0000 4800 1800 600 600 4800 1800 5400 1800\n\
-6\n\
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n\t 1200 3600 2400 3600\n\
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n\t 3000 3600 4200 3600\n\
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 4\n\t 1200 4800 2400 4800 3600 5400 4800 4800\n\
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n\t 1800 4500 1800 5100\n\
2 3 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 6000 1200 7500 1200 7500 2400 6000 2400 6000 1200\n\
3 0 0 1 0 7 50 -1 -1 0.000 0 0 0 4\n\t 6000 3600 7200 4500 8400 3600 9600 4500\n\t 0.000 1.000 1.000 0.000\n\
5 1 0 1 0 7 50 -1 -1 0.000 0 0 0 0 11400.000 3000.000 10200 3000 11400 1800 12600 3000\n\
4 0 0 50 -1 0 12 0.0000 4 135 480 1200 7200 Hello\\001\n\
4 0 0 50 -1 0 12 0.0000 4 135 480 3600 7200 World\\001\n\
2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5\n\t0 pic.png\n\t 9000 6000 10500 6000 10500 7200 9000 7200 9000 6000\n\
6 6000 6000 8400 7500\n\
2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 6000 6000 7200 6000 7200 7200 6000 7200 6000 6000\n\
2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 7800 6600 8400 6600 8400 7500 7800 7500 7800 6600\n\
-6\n";

#[derive(Clone, Copy, PartialEq)]
enum Btn {
    L,
    M,
    R,
}

impl Btn {
    fn xdo(self) -> &'static str {
        match self {
            Btn::L => "1",
            Btn::M => "2",
            Btn::R => "3",
        }
    }
    fn name(self) -> &'static str {
        match self {
            Btn::L => "left",
            Btn::M => "middle",
            Btn::R => "right",
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
    /// A key: (xdotool key name, egui key name), e.g. ("Left", "ArrowLeft").
    Key(&'static str, &'static str),
    /// Click a dialog button: xfig pixel (x < 0: xfig is not touched), RustFig accessible label
    /// ("": RustFig is not touched).
    Dlg(i32, i32, &'static str),
    /// Pick a menu entry: (menu x, entry y in xfig, menu label, entry label in RustFig; "" leaves
    /// RustFig alone: its entry opens a native OS dialog).
    Item(i32, i32, &'static str, &'static str),
    /// Press at the first Fig point, drag to the second, release (button).
    Drag(f64, f64, f64, f64, Btn),
}

struct Scenario {
    id: String,
    /// 0: `FIXTURE`; 1: `FIXTURE2` (compounds, joins, picture) with `pic.png` beside it.
    fixture: u8,
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

/// Round 2 of the check (W22): dialogs behind the menu entries, compounds, align, join/split, chop,
/// tangent, measure, picture objects, text editing keys and the middle and right buttons of the
/// editing and drawing modes, all on `FIXTURE2`.
fn scenarios_r2() -> Vec<Scenario> {
    use Btn::{L, M, R};
    let mut v: Vec<Scenario> = Vec::new();
    let mut add = |id: &str, group: &'static str, steps: Vec<Step>, save: bool, note: &'static str| {
        v.push(Scenario {
            id: id.into(),
            fixture: 1,
            group,
            steps,
            save,
            note,
        });
    };

    // Dialogs behind the menu entries. xfig: File x=30, Edit 91, View 152, Help 274; entries 16 px apart.
    for (id, x, y, menu, item, wait) in [
        ("file-open", 30, 50, "File", "", 900),
        ("file-merge", 30, 66, "File", "", 900),
        ("file-saveas", 30, 98, "File", "", 900),
        ("file-export", 30, 114, "File", "Export", 900),
        ("file-print", 30, 130, "File", "Print", 900),
        ("edit-paste-empty", 91, 50, "Edit", "Paste", 400),
        ("edit-search", 91, 82, "Edit", "Search", 700),
        ("edit-spell", 91, 98, "Edit", "Spell", 700),
        ("edit-global", 91, 130, "Edit", "Global", 700),
        ("edit-units", 91, 146, "Edit", "units", 700),
        ("view-styles", 152, 34, "View", "Styles", 700),
        ("view-zoom-in", 152, 82, "View", "Zoom In", 500),
        ("view-zoom-fit", 152, 114, "View", "Zoom to Fit", 500),
        ("view-charmap", 152, 162, "View", "Character", 700),
        ("view-line-lengths", 152, 228, "View", "line lengths", 500),
        ("view-vertex-numbers", 152, 244, "View", "vertex numbers", 500),
        ("help-about", 274, 98, "Help", "About", 700),
    ] {
        add(
            &format!("dlg-{id}"),
            "dialogs",
            vec![Step::Item(x, y, menu, item), Step::Wait(wait), Step::Shot("dialog")],
            false,
            if item.is_empty() {
                "RustFig opens the operating system's file dialog (rfd): not driven"
            } else {
                ""
            },
        );
    }
    add(
        "dlg-edit-delete-all",
        "dialogs",
        vec![
            Step::Item(91, 114, "Edit", "Delete All"),
            Step::Wait(500),
            Step::Shot("after"),
        ],
        true,
        "",
    );
    add(
        "dlg-view-zoom-in-click",
        "dialogs",
        vec![
            Step::Item(152, 82, "View", "Zoom In"),
            Step::Mode("box"),
            Step::Click(9000.0, 2000.0, L),
            Step::Click(12000.0, 3000.0, L),
            Step::Shot("zoomed"),
        ],
        false,
        "zoom 2, then a box",
    );

    // Compounds.
    let comp = (1200.0, 1800.0);
    add(
        "comp-glue",
        "compound",
        vec![
            Step::Mode("glue"),
            Step::Click(1800.0, 3600.0, L),
            Step::Click(3600.0, 3600.0, L),
            Step::Shot("tagged"),
            Step::Click(1800.0, 3600.0, R),
            Step::Shot("glued"),
        ],
        true,
        "tag two lines, right button makes the compound",
    );
    add(
        "comp-glue-region",
        "compound",
        vec![
            Step::Mode("glue"),
            Step::Click(900.0, 3300.0, M),
            Step::Move(2700.0, 3600.0),
            Step::Shot("region"),
            Step::Click(4500.0, 3900.0, M),
            Step::Shot("tagged"),
            Step::Click(1800.0, 3600.0, R),
            Step::Shot("glued"),
        ],
        true,
        "tag by region: middle click at the first corner, middle click at the final corner",
    );
    add(
        "comp-glue-region-drag",
        "compound",
        vec![
            Step::Mode("glue"),
            Step::Drag(900.0, 3300.0, 4500.0, 3900.0, M),
            Step::Shot("region"),
            Step::Click(1800.0, 3600.0, R),
            Step::Shot("glued"),
        ],
        true,
        "region by press, drag and release of the middle button",
    );
    add(
        "comp-break",
        "compound",
        vec![
            Step::Mode("break"),
            Step::Click(comp.0, comp.1, L),
            Step::Shot("broken"),
        ],
        true,
        "",
    );
    add(
        "comp-break-tag",
        "compound",
        vec![
            Step::Mode("break"),
            Step::Click(comp.0, comp.1, M),
            Step::Shot("broken"),
        ],
        true,
        "middle: break and tag",
    );
    add(
        "comp-open-move-close",
        "compound",
        vec![
            Step::Mode("open_compound"),
            Step::Click(comp.0, comp.1, L),
            Step::Shot("opened"),
            Step::Mode("move"),
            Step::Click(5400.0, 1800.0, L),
            Step::Click(5400.0, 3000.0, L),
            Step::Shot("moved-inside"),
            Step::Dlg(187, 103, "Close This Compound"),
            Step::Shot("closed"),
        ],
        true,
        "open, move the circle inside, close with the pop-up button",
    );
    add(
        "comp-open-keep",
        "compound",
        vec![
            Step::Mode("open_compound"),
            Step::Click(comp.0, comp.1, M),
            Step::Shot("opened-keep"),
            Step::Dlg(336, 103, "Close All Compounds"),
        ],
        true,
        "middle: open, keep visible; closed with the pop-up before saving",
    );
    for (id, mode) in [
        ("move", "move"),
        ("copy", "copy"),
        ("delete", "delete"),
        ("flip-lr", "flip_lr"),
        ("rotate-cw", "rotate_cw"),
        ("update", "update"),
    ] {
        let steps = if id == "move" || id == "copy" {
            vec![
                Step::Mode(mode),
                Step::Click(comp.0, comp.1, L),
                Step::Click(comp.0 + 6000.0, comp.1 + 3000.0, L),
                Step::Shot("done"),
            ]
        } else {
            vec![Step::Mode(mode), Step::Click(comp.0, comp.1, L), Step::Shot("done")]
        };
        add(&format!("comp-{id}"), "compound", steps, true, "on the compound");
    }
    add(
        "comp-scale",
        "compound",
        vec![
            Step::Mode("scale"),
            Step::Click(5400.0, 2400.0, L),
            Step::Click(6600.0, 3000.0, L),
            Step::Shot("scaled"),
        ],
        true,
        "corner of the compound",
    );
    add(
        "comp-edit-panel",
        "compound",
        vec![
            Step::Mode("edit"),
            Step::Click(comp.0, comp.1, L),
            Step::Wait(800),
            Step::Shot("popup"),
        ],
        false,
        "",
    );
    // Align: compound of two boxes at (6000,6000)-(7200,7200) and (7800,6600)-(8400,7500).
    add(
        "align-compound",
        "align",
        vec![
            Step::Mode("align"),
            Step::Click(6000.0, 6600.0, L),
            Step::Shot("aligned"),
        ],
        true,
        "left button on the compound, default alignment",
    );
    add(
        "align-canvas",
        "align",
        vec![
            Step::Mode("align"),
            Step::Click(6000.0, 6600.0, M),
            Step::Shot("aligned"),
        ],
        true,
        "middle button: align to the canvas",
    );
    // Join / split.
    add(
        "join-lines",
        "joinsplit",
        vec![
            Step::Mode("join_split"),
            Step::Click(1800.0, 3600.0, L),
            Step::Shot("first"),
            Step::Click(3600.0, 3600.0, L),
            Step::Shot("joined"),
        ],
        true,
        "",
    );
    add(
        "split-line",
        "joinsplit",
        vec![
            Step::Mode("join_split"),
            Step::Click(3600.0, 5400.0, M),
            Step::Shot("split"),
        ],
        true,
        "middle on the vertex of the 4-point line",
    );
    add(
        "split-line-segment",
        "joinsplit",
        vec![
            Step::Mode("join_split"),
            Step::Click(3000.0, 5100.0, M),
            Step::Shot("split"),
        ],
        true,
        "middle on a segment",
    );
    // Chop: axe = the vertical line (left), log = the 4-point line (middle).
    add(
        "chop-axe",
        "chop",
        vec![
            Step::Mode("chop"),
            Step::Click(1800.0, 4800.0, L),
            Step::Click(3000.0, 5100.0, M),
            Step::Shot("chopped"),
        ],
        true,
        "",
    );
    add(
        "chop-clear",
        "chop",
        vec![
            Step::Mode("chop"),
            Step::Click(1800.0, 4800.0, L),
            Step::Click(1800.0, 4800.0, R),
            Step::Click(3000.0, 5100.0, M),
            Step::Shot("after"),
        ],
        true,
        "right clears the axe list, so nothing is chopped",
    );
    // Tangent / normal on the spline and on the arc.
    for (id, x, y, b) in [
        ("tangent-spline", 7200.0, 4200.0, L),
        ("normal-spline", 7200.0, 4200.0, M),
        ("tangent-arc", 11400.0, 1800.0, L),
        ("normal-arc", 11400.0, 1800.0, M),
    ] {
        add(
            id,
            "tangent",
            vec![Step::Mode("tangent"), Step::Click(x, y, b), Step::Shot("done")],
            true,
            "",
        );
    }
    // Measure.
    add(
        "measure-length",
        "measure",
        vec![
            Step::Mode("length_measure"),
            Step::Click(1800.0, 4800.0, L),
            Step::Shot("one"),
            Step::Click(1800.0, 4800.0, M),
            Step::Shot("added"),
            Step::Click(1800.0, 4800.0, R),
            Step::Shot("reset"),
        ],
        false,
        "left picks, middle adds, right resets",
    );
    add(
        "measure-length-points",
        "measure",
        vec![
            Step::Mode("length_measure"),
            Step::Click(9000.0, 9000.0, L),
            Step::Click(10500.0, 9600.0, L),
            Step::Shot("points"),
        ],
        false,
        "two free points",
    );
    add(
        "measure-area",
        "measure",
        vec![
            Step::Mode("area_measure"),
            Step::Click(7500.0, 1800.0, L),
            Step::Shot("area"),
        ],
        false,
        "the polygon",
    );
    add(
        "measure-area-points",
        "measure",
        vec![
            Step::Mode("area_measure"),
            Step::Click(9000.0, 9000.0, L),
            Step::Click(10500.0, 9000.0, L),
            Step::Click(10500.0, 10200.0, L),
            Step::Click(10500.0, 10200.0, M),
            Step::Shot("area"),
        ],
        false,
        "three points, middle closes",
    );
    add(
        "measure-angle",
        "measure",
        vec![
            Step::Mode("angle_measure"),
            Step::Click(9000.0, 9000.0, L),
            Step::Click(10500.0, 9000.0, L),
            Step::Click(10500.0, 10200.0, L),
            Step::Shot("angle"),
        ],
        false,
        "three points",
    );
    add(
        "measure-angle-object",
        "measure",
        vec![
            Step::Mode("angle_measure"),
            Step::Click(3600.0, 5400.0, R),
            Step::Shot("angle"),
        ],
        false,
        "right: select the line's angle",
    );
    // Picture objects (the picture is part of the fixture; drawing one opens a file dialog).
    let pic = (9000.0, 6600.0);
    add(
        "pic-dialog",
        "picture",
        vec![Step::Mode("picture"), Step::Move(9000.0, 3000.0), Step::Shot("mode")],
        false,
        "mode only; the click opens a file dialog in both",
    );
    for (id, mode) in [
        ("move", "move"),
        ("copy", "copy"),
        ("delete", "delete"),
        ("flip-lr", "flip_lr"),
        ("rotate-cw", "rotate_cw"),
        ("update", "update"),
    ] {
        let steps = if id == "move" || id == "copy" {
            vec![
                Step::Mode(mode),
                Step::Click(pic.0, pic.1, L),
                Step::Click(pic.0 + 1500.0, pic.1 + 1500.0, L),
                Step::Shot("done"),
            ]
        } else {
            vec![Step::Mode(mode), Step::Click(pic.0, pic.1, L), Step::Shot("done")]
        };
        add(&format!("pic-{id}"), "picture", steps, true, "picture object");
    }
    add(
        "pic-scale",
        "picture",
        vec![
            Step::Mode("scale"),
            Step::Click(10500.0, 7200.0, L),
            Step::Click(12000.0, 8100.0, L),
            Step::Shot("scaled"),
        ],
        true,
        "",
    );
    add(
        "pic-edit-panel",
        "picture",
        vec![
            Step::Mode("edit"),
            Step::Click(pic.0, pic.1, L),
            Step::Wait(800),
            Step::Shot("popup"),
        ],
        false,
        "",
    );
    // Text editing keys: click the text "Hello" (1200,7200), edit, leave by choosing a mode.
    let hello = (1275.0, 7200.0);
    add(
        "text-keys-edit",
        "text",
        vec![
            Step::Mode("text"),
            Step::Click(hello.0, hello.1, L),
            Step::Wait(300),
            Step::Shot("start"),
            Step::Key("Left", "ArrowLeft"),
            Step::Key("Left", "ArrowLeft"),
            Step::Type("X"),
            Step::Key("Home", "Home"),
            Step::Type(">"),
            Step::Key("End", "End"),
            Step::Type("<"),
            Step::Key("BackSpace", "Backspace"),
            Step::Key("BackSpace", "Backspace"),
            Step::Shot("edited"),
            Step::Mode("move"),
            Step::Shot("done"),
        ],
        true,
        "arrows, Home/End, Backspace",
    );
    add(
        "text-keys-return",
        "text",
        vec![
            Step::Mode("text"),
            Step::Click(hello.0, hello.1, L),
            Step::Key("End", "End"),
            Step::Key("Return", "Enter"),
            Step::Type("two"),
            Step::Shot("two-lines"),
            Step::Mode("move"),
        ],
        true,
        "Return inside a text",
    );
    add(
        "text-keys-ctrl",
        "text",
        vec![
            Step::Mode("text"),
            Step::Click(hello.0, hello.1, L),
            Step::Key("ctrl+a", "A"),
            Step::Type("A"),
            Step::Key("ctrl+e", "E"),
            Step::Type("E"),
            Step::Key("ctrl+k", "K"),
            Step::Shot("ctrl"),
            Step::Mode("move"),
        ],
        true,
        "emacs keys: xfig w_keyboard/d_text (ctrl-a, ctrl-e, ctrl-k)",
    );
    add(
        "text-keys-delete-word",
        "text",
        vec![
            Step::Mode("text"),
            Step::Click(hello.0, hello.1, L),
            Step::Key("End", "End"),
            Step::Key("ctrl+w", "W"),
            Step::Shot("ctrlw"),
            Step::Mode("move"),
        ],
        true,
        "ctrl-w deletes a word in xfig",
    );
    add(
        "text-middle-right",
        "text",
        vec![
            Step::Mode("text"),
            Step::Click(9000.0, 9000.0, M),
            Step::Type("mid"),
            Step::Shot("mid"),
            Step::Mode("move"),
            Step::Mode("text"),
            Step::Click(9000.0, 10000.0, R),
            Step::Type("right"),
            Step::Shot("right"),
            Step::Mode("move"),
        ],
        true,
        "middle and right button in text mode on empty canvas",
    );
    // Middle and right buttons of the editing modes (box (1200,1200)-(3600,2400) is in the compound,
    // so most use the line (1200,3600)-(2400,3600) and the polygon).
    let on_a = (1800.0, 3600.0);
    let on_poly = (7500.0, 1800.0);
    for (id, mode, steps_fn) in [
        ("scale-M", "scale", 0),
        ("move-M", "move", 1),
        ("move-R", "move", 2),
        ("copy-M", "copy", 1),
        ("copy-R", "copy", 2),
        ("delete-M", "delete", 3),
        ("delete-R", "delete", 2),
        ("update-M", "update", 2),
        ("flip-M", "flip_ud", 2),
        ("flip-R", "flip_ud", 4),
        ("rotate-M", "rotate_cw", 2),
        ("rotate-R", "rotate_cw", 4),
        ("convert-R", "convert", 5),
        ("autoarrow-M", "autoarrow", 2),
        ("move-point-M", "move_point", 6),
        ("move-point-R", "move_point", 7),
        ("add-point-M", "add_point", 2),
        ("add-point-R", "add_point", 7),
        ("delete-point-M", "delete_point", 8),
        ("edit-M", "edit", 9),
    ] {
        let (b, base) = match id.rsplit('-').next() {
            Some("M") => (M, true),
            _ => (R, true),
        };
        let _ = base;
        let steps = match steps_fn {
            // scale about the centre
            0 => vec![
                Step::Mode(mode),
                Step::Click(on_poly.0, on_poly.1, M),
                Step::Click(9000.0, 3000.0, M),
                Step::Shot("done"),
            ],
            // press the button on the object, then place with the same button (horiz/vert)
            1 => vec![
                Step::Mode(mode),
                Step::Click(on_poly.0, on_poly.1, M),
                Step::Click(on_poly.0 + 1500.0, on_poly.1 + 600.0, M),
                Step::Shot("done"),
            ],
            2 => vec![
                Step::Mode(mode),
                Step::Click(on_poly.0, on_poly.1, b),
                Step::Shot("done"),
            ],
            3 => vec![
                Step::Mode(mode),
                Step::Click(5700.0, 900.0, M),
                Step::Click(7800.0, 2700.0, M),
                Step::Shot("done"),
            ],
            4 => vec![
                Step::Mode(mode),
                Step::Click(on_poly.0, on_poly.1, R),
                Step::Click(on_poly.0, on_poly.1, L),
                Step::Shot("done"),
            ],
            5 => vec![Step::Mode(mode), Step::Click(7500.0, 1800.0, R), Step::Shot("done")],
            6 => vec![
                Step::Mode(mode),
                Step::Click(7500.0, 2400.0, M),
                Step::Click(8100.0, 3000.0, M),
                Step::Shot("done"),
            ],
            7 => vec![
                Step::Mode(mode),
                Step::Click(7500.0, 2400.0, L),
                Step::Click(8100.0, 3000.0, R),
                Step::Shot("done"),
            ],
            8 => vec![Step::Mode(mode), Step::Click(7500.0, 2400.0, M), Step::Shot("done")],
            _ => vec![
                Step::Mode(mode),
                Step::Click(on_poly.0, on_poly.1, M),
                Step::Wait(800),
                Step::Shot("done"),
            ],
        };
        add(
            &format!("btn-{id}"),
            "buttons",
            steps,
            id != "edit-M",
            "middle/right button of an editing mode on the polygon",
        );
    }
    let _ = on_a;
    // Drawing modes: right button cancels / ends, middle button ends or is unused.
    add(
        "btn-box-cancel",
        "buttons",
        vec![
            Step::Mode("box"),
            Step::Click(9000.0, 9000.0, L),
            Step::Click(10500.0, 10200.0, R),
            Step::Shot("cancelled"),
        ],
        true,
        "right cancels the box in progress",
    );
    add(
        "btn-box-middle",
        "buttons",
        vec![
            Step::Mode("box"),
            Step::Click(9000.0, 9000.0, M),
            Step::Click(10500.0, 10200.0, L),
            Step::Shot("middle"),
        ],
        true,
        "middle button in box mode",
    );
    add(
        "btn-polyline-undo",
        "buttons",
        vec![
            Step::Mode("polyline"),
            Step::Click(9000.0, 9000.0, L),
            Step::Click(10500.0, 9000.0, L),
            Step::Click(10500.0, 10200.0, L),
            Step::Click(10500.0, 10200.0, R),
            Step::Shot("undone"),
            Step::Click(12000.0, 10200.0, M),
            Step::Shot("ended"),
        ],
        true,
        "right removes the last point, middle ends",
    );
    add(
        "btn-polyline-first-middle",
        "buttons",
        vec![
            Step::Mode("polyline"),
            Step::Click(9000.0, 9000.0, M),
            Step::Click(10500.0, 9000.0, L),
            Step::Click(10500.0, 10200.0, M),
            Step::Shot("done"),
        ],
        true,
        "middle as the first point",
    );
    add(
        "btn-circle-cancel",
        "buttons",
        vec![
            Step::Mode("ellipse_radius"),
            Step::Click(9000.0, 9000.0, L),
            Step::Click(10500.0, 10200.0, R),
            Step::Shot("cancelled"),
        ],
        true,
        "",
    );
    add(
        "btn-arc-cancel",
        "buttons",
        vec![
            Step::Mode("arc"),
            Step::Click(9000.0, 9000.0, L),
            Step::Click(10500.0, 8400.0, L),
            Step::Click(12000.0, 9000.0, R),
            Step::Shot("cancelled"),
        ],
        true,
        "",
    );
    add(
        "btn-spline-undo",
        "buttons",
        vec![
            Step::Mode("spline"),
            Step::Click(9000.0, 9000.0, L),
            Step::Click(10500.0, 8400.0, L),
            Step::Click(12000.0, 9000.0, L),
            Step::Click(12000.0, 9000.0, R),
            Step::Click(13000.0, 10000.0, M),
            Step::Shot("done"),
        ],
        true,
        "",
    );
    // Hints and messages in the middle of an operation (xfig `set_mousefun` calls in e_*.c, d_*.c):
    // one shot per state, compared in the top strip (message panel and Mouse Buttons box).
    let on_poly = (7500.0, 1800.0);
    for (id, mode, first_btn) in [
        ("move", "move", L),
        ("copy", "copy", L),
        ("scale", "scale", L),
        ("move-point", "move_point", L),
        ("add-point", "add_point", L),
        ("rotate", "rotate_cw", R),
        ("flip", "flip_ud", R),
    ] {
        add(
            &format!("hint-{id}"),
            "hints",
            vec![
                Step::Mode(mode),
                Step::Move(on_poly.0 + 600.0, on_poly.1 + 3000.0),
                Step::Shot("start"),
                Step::Move(on_poly.0, on_poly.1),
                Step::Shot("over-object"),
                Step::Click(on_poly.0, on_poly.1, first_btn),
                Step::Move(on_poly.0 + 900.0, on_poly.1 + 900.0),
                Step::Shot("after-first"),
                Step::Click(on_poly.0 + 900.0, on_poly.1 + 900.0, R),
                Step::Shot("after-right"),
            ],
            false,
            "hint and message per state",
        );
    }
    add(
        "hint-move-middle",
        "hints",
        vec![
            Step::Mode("move"),
            Step::Click(on_poly.0, on_poly.1, M),
            Step::Move(on_poly.0 + 900.0, on_poly.1 + 900.0),
            Step::Shot("after-middle"),
            Step::Click(on_poly.0 + 900.0, on_poly.1 + 900.0, R),
            Step::Shot("after-right"),
        ],
        false,
        "",
    );
    add(
        "hint-glue-region",
        "hints",
        vec![
            Step::Mode("glue"),
            Step::Click(1800.0, 3600.0, L),
            Step::Shot("tagged-one"),
            Step::Click(1800.0, 3600.0, R),
            Step::Shot("compound-made"),
        ],
        false,
        "",
    );
    add(
        "hint-join",
        "hints",
        vec![
            Step::Mode("join_split"),
            Step::Click(1800.0, 3600.0, L),
            Step::Move(3000.0, 4200.0),
            Step::Shot("after-first"),
            Step::Click(3000.0, 4200.0, R),
            Step::Shot("after-right"),
        ],
        false,
        "",
    );
    add(
        "hint-chop",
        "hints",
        vec![
            Step::Mode("chop"),
            Step::Click(1800.0, 4800.0, L),
            Step::Shot("axe-selected"),
            Step::Click(1800.0, 4800.0, R),
            Step::Shot("cleared"),
        ],
        false,
        "",
    );
    add(
        "hint-angle",
        "hints",
        vec![
            Step::Mode("angle_measure"),
            Step::Click(9000.0, 9000.0, L),
            Step::Move(10000.0, 9000.0),
            Step::Shot("after-first"),
            Step::Click(10500.0, 9000.0, L),
            Step::Move(10500.0, 9900.0),
            Step::Shot("after-second"),
        ],
        false,
        "",
    );
    for (id, mode, pts) in [
        ("box", "box", 1usize),
        ("circle", "ellipse_radius", 1),
        ("polyline", "polyline", 2),
        ("polygon", "polygon", 2),
        ("spline", "spline", 2),
        ("closed-interp", "closed_interp_spline", 2),
        ("arc", "arc", 2),
        ("regpoly", "regpoly", 1),
        ("arcbox", "arcbox", 1),
    ] {
        let mut steps = vec![Step::Mode(mode), Step::Shot("start")];
        let path = [(9000.0, 9000.0), (10500.0, 8400.0), (12000.0, 9000.0)];
        for p in path.iter().take(pts) {
            steps.push(Step::Click(p.0, p.1, L));
        }
        steps.push(Step::Move(10500.0, 10200.0));
        steps.push(Step::Shot("in-progress"));
        steps.push(Step::Click(10500.0, 10200.0, R));
        steps.push(Step::Shot("after-right"));
        add(&format!("hint-draw-{id}"), "hints", steps, false, "hints while drawing");
    }
    add(
        "hint-text",
        "hints",
        vec![
            Step::Mode("text"),
            Step::Shot("start"),
            Step::Click(9000.0, 9000.0, L),
            Step::Type("ab"),
            Step::Shot("typing"),
            Step::Mode("move"),
        ],
        false,
        "",
    );
    add(
        "hint-pic-xfig",
        "hints",
        vec![
            Step::Mode("picture"),
            Step::Shot("start"),
            Step::Dlg(675, 274, ""),
            Step::Dlg(875, 407, ""),
            Step::Wait(900),
            Step::Shot("dialog"),
        ],
        false,
        "xfig's Picture dialog after the second corner (RustFig opens a native file dialog, not driven)",
    );
    add(
        "text-keys-ctrl-bf",
        "text",
        vec![
            Step::Mode("text"),
            Step::Click(hello.0, hello.1, L),
            Step::Key("ctrl+a", "A"),
            Step::Key("ctrl+f", "F"),
            Step::Key("ctrl+f", "F"),
            Step::Type("Z"),
            Step::Key("ctrl+b", "B"),
            Step::Key("ctrl+d", "D"),
            Step::Shot("ctrl"),
            Step::Mode("move"),
        ],
        true,
        "ctrl-f, ctrl-b, ctrl-d (xfig d_text.c)",
    );
    add(
        "key-zoom-out",
        "keys",
        vec![
            Step::Mode("move"),
            Step::Move(9000.0, 9000.0),
            Step::Key("z", "Z"),
            Step::Shot("zoomed-out"),
        ],
        false,
        "xfig: z zooms out by sqrt 2",
    );
    add(
        "key-shift-z",
        "keys",
        vec![
            Step::Mode("move"),
            Step::Move(9000.0, 9000.0),
            Step::Key("shift+z", "Z"),
            Step::Shot("zoomed-in"),
        ],
        false,
        "xfig: Shift-Z zooms in by sqrt 2",
    );
    v
}

fn scenarios() -> Vec<Scenario> {
    use Btn::{L, M};
    let mut v: Vec<Scenario> = Vec::new();
    let mut add = |id: &str, group: &'static str, steps: Vec<Step>, save: bool, note: &'static str| {
        v.push(Scenario {
            id: id.into(),
            fixture: 0,
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
    v.extend(scenarios_r2());
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

/// The objects of a .fig file as one string each (object line, then its point lines after " / ";
/// header, comments and blank lines dropped, whitespace and number formats normalised).
fn fig_objects(text: &str) -> Vec<String> {
    let mut lines = text.lines().map(str::trim_end);
    // header: up to the "<resolution> <coord system>" line
    for l in lines.by_ref() {
        let t: Vec<&str> = l.split_whitespace().collect();
        if t.len() == 2 && t.iter().all(|x| x.parse::<i64>().is_ok()) {
            break;
        }
    }
    let mut out: Vec<String> = Vec::new();
    for l in lines.filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        if l.starts_with(char::is_whitespace) {
            // point or picture-file line of the object before
            let nums: Vec<String> = l
                .split_whitespace()
                .map(|w| w.parse::<f64>().map_or_else(|_| w.to_string(), |n| format!("{n}")))
                .collect();
            if let Some(last) = out.last_mut() {
                last.push_str(" / ");
                last.push_str(&nums.join(" "));
            }
        } else {
            out.push(normalise(l));
        }
    }
    out
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

/// Differences between two object lists as readable lines (empty when equal): the objects only
/// one app has, or a note when both have the same objects in another order (xfig re-appends moved
/// objects, so the order matters for drawing).
fn fig_diff(a: &[String], b: &[String]) -> Vec<String> {
    if a == b {
        return Vec::new();
    }
    let short = |l: &str| -> String {
        let t: String = l.chars().take(170).collect();
        format!("{}: {t}", kind(l))
    };
    let mut only_a: Vec<&String> = Vec::new();
    let mut rest_b: Vec<&String> = b.iter().collect();
    for x in a {
        if let Some(i) = rest_b.iter().position(|y| *y == x) {
            rest_b.remove(i);
        } else {
            only_a.push(x);
        }
    }
    let mut out = Vec::new();
    if only_a.is_empty() && rest_b.is_empty() {
        let first = a.iter().zip(b).position(|(x, y)| x != y).unwrap_or(0);
        out.push(format!(
            "same objects, other order from position {first}:\n  xfig:    {}\n  RustFig: {}",
            a.get(first).map_or(String::new(), |l| short(l)),
            b.get(first).map_or(String::new(), |l| short(l))
        ));
        return out;
    }
    for x in only_a {
        out.push(format!("xfig only:    {}", short(x)));
    }
    for y in rest_b {
        out.push(format!("RustFig only: {}", short(y)));
    }
    if a.len() != b.len() {
        out.push(format!("({} objects in xfig, {} in RustFig)", a.len(), b.len()));
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
    let fixture = if sc.fixture == 1 { FIXTURE2 } else { FIXTURE };
    std::fs::write(&xf_fig, fixture).map_err(|e| e.to_string())?;
    std::fs::write(&rf_fig, fixture).map_err(|e| e.to_string())?;
    if sc.fixture == 1 {
        // a small picture beside each .fig (relative path in the figure)
        let png = env.out.join("pic.png");
        if !png.is_file() {
            magick(&["-size", "120x80", "gradient:red-blue", &png.display().to_string()])?;
        }
        for f in [&xf_fig, &rf_fig] {
            if let Some(parent) = f.parent() {
                std::fs::copy(&png, parent.join("pic.png")).map_err(|e| e.to_string())?;
            }
        }
    }

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
            Step::Key(xk, rk) => {
                xdo(d, &["key", xk]);
                let mods = if xk.starts_with("ctrl+") {
                    r#","modifiers":["ctrl"]"#
                } else if xk.starts_with("shift+") {
                    r#","modifiers":["shift"]"#
                } else {
                    ""
                };
                rf.call("ui.key", &format!(r#"{{"key":{}{mods}}}"#, json_str(rk)))?;
                sleep_ms(300);
            }
            Step::Drag(x0, y0, x1, y1, b) => {
                let (x0, y0, x1, y1) = (snap(*x0), snap(*y0), snap(*x1), snap(*y1));
                pointer_to(d, rf, x0, y0, fx(x0), fy(y0))?;
                xdo(d, &["mousedown", b.xdo()]);
                rf.pointer("press", x0, y0, *b)?;
                sleep_ms(200);
                for k in 1..=4 {
                    let t = f64::from(k) / 4.0;
                    let (mx, my) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
                    xdo(d, &["mousemove", &fx(mx).to_string(), &fy(my).to_string()]);
                    rf.pointer("move", mx, my, *b)?;
                    sleep_ms(150);
                }
                xdo(d, &["mouseup", b.xdo()]);
                rf.pointer("release", x1, y1, *b)?;
                sleep_ms(300);
            }
            Step::Dlg(x, y, label) => {
                if *x >= 0 {
                    xdo(d, &["mousemove", &x.to_string(), &y.to_string()]);
                    sleep_ms(150);
                    xdo(d, &["click", "1"]);
                }
                if !label.is_empty() {
                    rf.click_label(label)?;
                }
                sleep_ms(400);
            }
            Step::Item(x, y, menu, item) => {
                xdo(d, &["mousemove", &x.to_string(), "12"]);
                sleep_ms(150);
                xdo(d, &["mousedown", "1"]);
                sleep_ms(300);
                xdo(d, &["mousemove", &(x - 4).to_string(), &(y - 12).to_string()]);
                sleep_ms(150);
                xdo(d, &["mousemove", &(x - 5).to_string(), &y.to_string()]);
                sleep_ms(300);
                xdo(d, &["mouseup", "1"]);
                if !item.is_empty() {
                    rf.click_label(menu)?;
                    sleep_ms(400);
                    rf.click_label(item)?;
                }
                sleep_ms(700);
            }
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
        fig_note = if a.trim() == (if sc.fixture == 1 { FIXTURE2 } else { FIXTURE }).trim() {
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
    // comma-separated substrings: any match runs
    let pick = |id: &str| filter.split(',').any(|f| id.contains(f));
    let total = all.iter().filter(|s| pick(&s.id)).count();
    for (n, sc) in all.iter().filter(|s| pick(&s.id)).enumerate() {
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
        assert_eq!(o.len(), 4);
        assert!(o.first().is_some_and(|l| l.contains(" / 1200 1200 3600 1200")));
        assert!(fig_diff(&o, &o).is_empty());
        let mut p = o.clone();
        p[1] = "x".into();
        assert_eq!(fig_diff(&o, &p).len(), 2);
        let mut q = o.clone();
        q.swap(0, 1);
        assert!(fig_diff(&o, &q).iter().all(|l| !l.contains("only")));
    }

    #[test]
    fn reads_numbers_from_a_reply() {
        let s = r#"{"origin":[1.5,-2],"x":1}"#;
        assert_eq!(numbers_after(s, "\"origin\":[", 2), Some(vec![1.5, -2.0]));
    }
}
