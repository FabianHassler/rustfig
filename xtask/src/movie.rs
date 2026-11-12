//! `cargo xtask movie` (plan R8): pictures of RustFig's window for the manual, made by driving the
//! release app through the control channel.
//!
//! Writes to `docs/images/window/`:
//! - `window-{light,dark}.png`: the whole window with a numbered callout on each area of
//!   [`AREAS`], each placed from the rect `ui.state` reports for the area's label (points; scaled
//!   by the screenshot's pixels per point), so the callouts follow the layout;
//! - `mouse-indicator-polyline{,-shift}-light.png`, `mouse-indicator-move-light.png`: the mouse
//!   indicator close up, as the manual's Mouse topic shows it.
//!
//! It also films the movie of plan R9 ([`movie_film`](crate::movie_film)).
//!
//! Needs ImageMagick (`magick`), and ffmpeg for the movie; skips with a message when magick is
//! missing. Environment: `MOVIE_PART` (`pictures` or `film`: only that part), `MOVIE_PORT`
//! (default 8032; the film uses the next port), `MOVIE_OUT` (default `docs/images/window`),
//! `MOVIE_FILM_OUT` (default `docs/images/movie`), `MOVIE_UPDATE` (take the drawn figure as the
//! fixture).

use crate::xfig_compare::{have, json_str, numbers_after, sleep_ms, Rf, WIN_W};
use std::path::{Path, PathBuf};
use std::process::Command;

/// An area of the window: its name (as docs/manual/window.md calls it, lower case), the
/// `ui.state` widget label that locates it (a prefix; the "Window area: ..." labels are set by
/// `inject::mark_area` in crates/ui-egui, the units box is the ruler corner's own button) and
/// where the callout sits. The index + 1 is the callout number. Keep the labels in step with
/// `window_areas_have_rects` in crates/ui-egui/tests/ui_smoke.rs.
pub(crate) struct Area {
    pub name: &'static str,
    pub label: &'static str,
    /// Callout position on the area's outline: `tl` top left, `tr`, `bl`, `br` (inside the corner), `l`, `r` (just outside the edge, centered, so no text is covered).
    pub at: &'static str,
}

pub(crate) const AREAS: &[Area] = &[
    Area {
        name: "menus",
        label: "Window area: Menus",
        at: "r",
    },
    Area {
        name: "file name",
        label: "Window area: File name",
        at: "r",
    },
    Area {
        name: "message panel",
        label: "Window area: Message panel",
        at: "tr",
    },
    Area {
        name: "mouse indicator",
        label: "Window area: Mouse indicator",
        at: "tr",
    },
    Area {
        name: "units box",
        label: "Units: ",
        at: "l",
    },
    Area {
        name: "mode panel",
        label: "Window area: Mode panel",
        at: "tr",
    },
    Area {
        name: "canvas",
        label: "Window area: Canvas and rulers",
        at: "br",
    },
    Area {
        name: "indicator panel",
        label: "Window area: Indicator panel",
        at: "tr",
    },
    Area {
        name: "depth manager",
        label: "Window area: Depth manager",
        at: "tr",
    },
];

/// The close-ups: file key, mode button label, also with Shift held.
const CLOSE_UPS: &[(&str, &str, bool)] = &[("polyline", "Polyline", true), ("move", "Move", false)];

const RED: &str = "#d8232a";

/// The rect (points) of the first widget whose label starts with `label` in a `ui.state` reply.
pub(crate) fn rect_of(state: &str, label: &str) -> Option<[f64; 4]> {
    let i = state.find(&format!("\"label\":{}", json_str(label).trim_end_matches('"')))?;
    let v = numbers_after(state.get(i..)?, "\"rect\":[", 4)?;
    let r = [*v.first()?, *v.get(1)?, *v.get(2)?, *v.get(3)?];
    (r.iter().all(|x| x.is_finite()) && r[2] > r[0] && r[3] > r[1]).then_some(r)
}

fn run_magick(args: &[String]) -> Result<(), String> {
    let o = Command::new("magick")
        .args(args)
        .env(
            "PATH",
            format!("/opt/homebrew/bin:{}", std::env::var("PATH").unwrap_or_default()),
        )
        .output()
        .map_err(|e| format!("magick: {e}"))?;
    if o.status.success() {
        Ok(())
    } else {
        Err(format!("magick {args:?}: {}", String::from_utf8_lossy(&o.stderr)))
    }
}

fn s(x: impl ToString) -> String {
    x.to_string()
}

/// Pixel size of a PNG.
pub(crate) fn png_size(p: &Path) -> Result<(f64, f64), String> {
    let o = Command::new("magick")
        .args(["identify", "-format", "%w %h"])
        .arg(p)
        .env(
            "PATH",
            format!("/opt/homebrew/bin:{}", std::env::var("PATH").unwrap_or_default()),
        )
        .output()
        .map_err(|e| format!("magick identify: {e}"))?;
    let t = String::from_utf8_lossy(&o.stdout).to_string();
    let mut it = t.split_whitespace().filter_map(|w| w.parse::<f64>().ok());
    match (it.next(), it.next()) {
        (Some(w), Some(h)) => Ok((w, h)),
        _ => Err(format!("cannot read the size of {}", p.display())),
    }
}

/// Draw each area's outline and its numbered badge on `src` (pixels = points * `ppp`), shrink to
/// 256 colors, write `dst`.
fn callouts(src: &Path, dst: &Path, rects: &[(usize, [f64; 4])], ppp: f64, work: &Path) -> Result<(), String> {
    let r = (11.0 * ppp).round();
    let font = badge_font()?;
    let mut args = vec![s(src.display())];
    // Outlines first, badges (separate small images, so the digits are centered) on top.
    args.extend(["-fill", "none", "-stroke", RED].map(s));
    args.extend(["-strokewidth".into(), s((1.5 * ppp).max(2.0))]);
    for (_, a) in rects {
        let (x0, y0, x1, y1) = (a[0] * ppp, a[1] * ppp, a[2] * ppp - 1.0, a[3] * ppp - 1.0);
        args.extend(["-draw".into(), format!("rectangle {x0},{y0} {x1},{y1}")]);
    }
    let size = (2.0 * r + 4.0) as i64;
    for (n, (i, a)) in rects.iter().enumerate() {
        let badge = work.join(format!("badge{n}.png"));
        run_magick(&[
            s("-size"),
            format!("{size}x{size}"),
            s("xc:none"),
            s("-fill"),
            s(RED),
            s("-stroke"),
            s("white"),
            s("-strokewidth"),
            s((1.5 * ppp).max(1.5)),
            s("-draw"),
            format!("circle {c},{c} {c},{e}", c = size / 2, e = 2),
            s("-stroke"),
            s("none"),
            s("-fill"),
            s("white"),
            s("-font"),
            font.clone(),
            s("-pointsize"),
            s((r * 1.25).round()),
            s("-gravity"),
            s("center"),
            s("-annotate"),
            s("+0+0"),
            s(i + 1),
            s(badge.display()),
        ])?;
        let at = AREAS.get(*i).map_or("tl", |a| a.at);
        let (x0, y0, x1, y1) = (a[0] * ppp, a[1] * ppp, a[2] * ppp, a[3] * ppp);
        let h = size as f64 / 2.0;
        let (cx, cy) = match at {
            "tr" => (x1 - h - 2.0, y0 + h + 2.0),
            "bl" => (x0 + h + 2.0, y1 - h - 2.0),
            "br" => (x1 - h - 2.0, y1 - h - 2.0),
            "l" => (x0 - h - 3.0, (y0 + y1) / 2.0),
            "r" => (x1 + h + 3.0, (y0 + y1) / 2.0),
            _ => (x0 + h + 2.0, y0 + h + 2.0),
        };
        args.extend([
            s(badge.display()),
            s("-geometry"),
            format!("+{}+{}", (cx - h).round().max(0.0), (cy - h).round().max(0.0)),
            s("-composite"),
        ]);
    }
    args.extend(["-resize".into(), "1600x>".into()]);
    args.extend(["-colors".into(), "256".into(), "-strip".into()]);
    args.push(s(dst.display()));
    run_magick(&args)
}

/// A bold sans font file for the numbers (ImageMagick often has no font configured; nothing of it
/// is committed). `MOVIE_FONT` overrides.
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
    .ok_or_else(|| "no bold font file found for the callout numbers; set MOVIE_FONT".to_string())
}

fn shot(rf: &mut Rf, path: &Path) -> Result<(), String> {
    rf.shot(path)?;
    sleep_ms(200);
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("no screenshot at {}", path.display()))
    }
}

/// Wait for the frames after a mode change, theme change or modifier change.
fn settle() {
    sleep_ms(600);
}

fn hover_canvas(rf: &mut Rf) -> Result<(), String> {
    // Over the drawing, away from every button: the indicator then shows the mode's own hints.
    let (x, y) = (rf.canvas.0 + 60.0, rf.canvas.1 + 60.0);
    rf.call("ui.pointer", &format!(r#"{{"action":"move","x":{x},"y":{y}}}"#))?;
    settle();
    Ok(())
}

fn area_rects(state: &str) -> Result<Vec<(usize, [f64; 4])>, String> {
    let mut v = Vec::new();
    for (i, a) in AREAS.iter().enumerate() {
        let r =
            rect_of(state, a.label).ok_or_else(|| format!("ui.state has no rect for {:?} ({})", a.label, a.name))?;
        v.push((i, r));
    }
    Ok(v)
}

pub fn run(root: &Path) -> Result<(), String> {
    if !have("magick", &["/opt/homebrew/bin", "/usr/local/bin"]) {
        println!("movie: skipped, ImageMagick (magick) is missing");
        return Ok(());
    }
    let port: u16 = std::env::var("MOVIE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8032);
    let out = std::env::var_os("MOVIE_OUT").map_or_else(|| root.join("docs/images/window"), PathBuf::from);
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let mut b = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    b.args(["build", "--release", "-p", "rustfig"]).current_dir(root);
    eprintln!("$ {b:?}");
    if !b.status().map_err(|e| e.to_string())?.success() {
        return Err("building rustfig failed".into());
    }
    let part = std::env::var("MOVIE_PART").unwrap_or_default();
    if part != "pictures" {
        let work = target.join("movie-film");
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
        let film_out = std::env::var_os("MOVIE_FILM_OUT").map_or_else(|| root.join("docs/images/movie"), PathBuf::from);
        crate::movie_film::film(root, &target.join("release/rustfig"), port + 1, &work, &film_out)?;
        let _ = std::fs::remove_dir_all(&work);
        if part == "film" {
            return Ok(());
        }
    }
    let work = target.join("movie");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;

    let mut rf = Rf::start(
        &target.join("release/rustfig"),
        port,
        &work,
        &root.join("docs/images/rustfig-window.fig"),
    )?;
    // The whole drawing in view, no zoom the viewer has to guess.
    rf.call("engine.execute", r#"{"command":"view.zoom_fit","params":{}}"#)?;
    rf.layout()?;

    // Close-ups of the mouse indicator, light theme; the Polyline one doubles as the window picture.
    for (key, label, with_shift) in CLOSE_UPS {
        rf.click_label(label)?;
        hover_canvas(&mut rf)?;
        for shift in [false, true].into_iter().filter(|s| !s || *with_shift) {
            if shift {
                rf.call("ui.modifiers", r#"{"modifiers":["shift"]}"#)?;
                settle();
            }
            let full = work.join(format!("full-{key}{}.png", if shift { "-shift" } else { "" }));
            shot(&mut rf, &full)?;
            let state = rf.call("ui.state", "{}")?;
            let r = rect_of(&state, "Window area: Mouse indicator")
                .ok_or("ui.state has no rect for the mouse indicator")?;
            let (pw, _) = png_size(&full)?;
            let ppp = pw / f64::from(WIN_W);
            // A margin round the frame, but not down into the coordinates row below it.
            let (pad, below) = (4.0, 1.0);
            let (x, y) = (((r[0] - pad) * ppp).max(0.0), ((r[1] - pad) * ppp).max(0.0));
            let (w, h) = ((r[2] - r[0] + 2.0 * pad) * ppp, (r[3] - r[1] + pad + below) * ppp);
            let dst = out.join(format!(
                "mouse-indicator-{key}{}-light.png",
                if shift { "-shift" } else { "" }
            ));
            run_magick(&[
                s(full.display()),
                s("-crop"),
                format!("{}x{}+{}+{}", w.round(), h.round(), x.round(), y.round()),
                s("+repage"),
                s("-colors"),
                s("256"),
                s("-strip"),
                s(dst.display()),
            ])?;
            if *key == "polyline" && !shift {
                let rects = area_rects(&state)?;
                callouts(&full, &out.join("window-light.png"), &rects, ppp, &work)?;
            }
        }
        rf.call("ui.modifiers", r#"{"modifiers":[]}"#)?;
        settle();
    }

    // Dark window, in Polyline mode like the light one.
    rf.click_label("Polyline")?;
    rf.call("ui.run", r#"{"command":"app.theme","params":{"theme":"dark"}}"#)?;
    hover_canvas(&mut rf)?;
    let full = work.join("full-dark.png");
    shot(&mut rf, &full)?;
    let state = rf.call("ui.state", "{}")?;
    let (pw, _) = png_size(&full)?;
    callouts(
        &full,
        &out.join("window-dark.png"),
        &area_rects(&state)?,
        pw / f64::from(WIN_W),
        &work,
    )?;
    if !rf.log.is_empty() {
        eprintln!("control channel errors:\n{}", rf.log.join("\n"));
    }
    drop(rf);
    let _ = std::fs::remove_dir_all(&work);
    println!("movie: pictures written to {}", out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// docs/manual/window.md names every area the pictures mark (the app side is
    /// `window_areas_have_rects` in crates/ui-egui/tests/ui_smoke.rs).
    #[test]
    fn manual_names_every_area() {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/manual/window.md");
        let text = std::fs::read_to_string(&p).unwrap_or_default().to_lowercase();
        for a in AREAS {
            assert!(
                text.contains(a.name),
                "docs/manual/window.md does not mention the {}",
                a.name
            );
        }
    }

    #[test]
    fn rect_is_found_by_label_prefix() {
        let st = r#"{"widgets":[{"disabled":false,"label":"Units: 1in = 1.00in","rect":[1165.0,71.0,1260.0,95.0],"role":"Button"}]}"#;
        assert_eq!(rect_of(st, "Units: "), Some([1165.0, 71.0, 1260.0, 95.0]));
        assert_eq!(rect_of(st, "Menus"), None);
    }
}
