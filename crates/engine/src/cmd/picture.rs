//! Picture edit panel commands (plan W10-16, W10-20): size helpers, rotation,
//! reread / relink, file information, "Edit Image".

// Ported from xfig src/e_edit.c (xfig licence, see LICENSE-xfig).

use super::{num, opt_num, string, CmdResult, Command};
use crate::{import, Engine, EngineError};
use rustfig_geom::Point;
use rustfig_model::{ObjectId, ObjectKind, Picture};
use serde_json::{json, Value};
use std::sync::Arc as Shared;

/// Fig units per pixel of a picture at its original size (80 pixels per inch).
pub const UNITS_PER_PIXEL: f64 = 15.0;

/// What the panel shows about a picture's file.
#[derive(Debug, Clone, PartialEq)]
pub struct PictureInfo {
    /// The file is there.
    pub exists: bool,
    /// "PNG", "JPEG", "GIF", "EPS", "PDF", or the file's extension.
    pub kind: String,
    pub width: u32,
    pub height: u32,
    /// Distinct colours (counting stops at 257).
    pub colours: usize,
    /// Some pixel is fully transparent.
    pub transparent: bool,
}

/// Type of a picture file: its first bytes, else its extension.
pub fn picture_type(path: &str) -> String {
    use std::io::Read;
    let mut head = [0u8; 8];
    let mut n = std::fs::File::open(path)
        .and_then(|mut f| f.read(&mut head))
        .unwrap_or(0);
    // A picture kept in memory (the browser, C2).
    if n == 0 {
        if let Some(b) = import::stored_picture(path) {
            n = b.len().min(8);
            head[..n].copy_from_slice(b.get(..n).unwrap_or_default());
        }
    }
    let h = head.get(..n).unwrap_or(&[]);
    if h.starts_with(b"\x89PNG") {
        "PNG".into()
    } else if h.starts_with(&[0xFF, 0xD8]) {
        "JPEG".into()
    } else if h.starts_with(b"GIF8") {
        "GIF".into()
    } else if h.starts_with(b"%PDF") {
        "PDF".into()
    } else if h.starts_with(b"%!PS") || h.starts_with(&[0xC5, 0xD0, 0xD3, 0xC6]) {
        "EPS".into()
    } else {
        std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_uppercase)
            .unwrap_or_default()
    }
}

/// Information about a picture (read-only).
pub fn info(pic: &Picture) -> PictureInfo {
    let exists = std::path::Path::new(&pic.file).is_file() || import::stored_picture(&pic.file).is_some();
    let (mut width, mut height, mut colours, mut transparent) = (0, 0, 0, false);
    if let Some(img) = &pic.image {
        width = img.width;
        height = img.height;
        let mut set = std::collections::HashSet::new();
        for px in img.rgba.chunks(4) {
            let &[r, g, b, a] = px else { continue };
            if a == 0 {
                transparent = true;
            } else if set.len() <= 256 {
                set.insert((r, g, b));
            }
        }
        colours = set.len();
    }
    PictureInfo {
        exists,
        kind: picture_type(&pic.file),
        width,
        height,
        colours,
        transparent,
    }
}

/// xfig's rotation of the picture box from the corner order: 0, 90, 180, 270.
pub fn rotation_of(p1: Point, p2: Point) -> i32 {
    let (dx, dy) = (p2.x - p1.x, p2.y - p1.y);
    if dx < 0.0 && dy < 0.0 {
        180
    } else if dx < 0.0 {
        270
    } else if dy < 0.0 {
        90
    } else {
        0
    }
}

/// The opposite corner for rotation `deg`, keeping the corner `p1` and the
/// box's width and height.
pub fn with_rotation(p1: Point, p2: Point, deg: i32) -> Option<Point> {
    let (w, h) = ((p2.x - p1.x).abs(), (p2.y - p1.y).abs());
    let (sx, sy) = match deg {
        0 => (1.0, 1.0),
        90 => (1.0, -1.0),
        180 => (-1.0, -1.0),
        270 => (-1.0, 1.0),
        _ => return None,
    };
    Some(Point::new(p1.x + sx * w, p1.y + sy * h))
}

/// How the size buttons change the box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resize {
    /// "Shrink to orig": keep the proportions, shrink the longer side.
    Shrink,
    /// "Expand to orig": keep the proportions, grow the shorter side.
    Expand,
    /// "Use original size".
    Original,
    /// "Scale by %" (of the original size).
    Percent(f64),
}

/// New opposite corner after a size button (xfig `shrink_pic`, `expand_pic`,
/// `origsize_pic`, `scale_percent_pic`). `width`/`height` are the picture's
/// pixels. `None` when the box has no area or the picture no size.
pub fn resize_corner(mode: Resize, p1: Point, p2: Point, flipped: bool, width: u32, height: u32) -> Option<Point> {
    if width == 0 || height == 0 {
        return None;
    }
    let (dx, dy) = (p2.x - p1.x, p2.y - p1.y);
    if dx == 0.0 || dy == 0.0 || !dx.is_finite() || !dy.is_finite() {
        return None;
    }
    let orig_ratio = f64::from(height) / f64::from(width);
    let (sx, sy) = (dx.signum(), dy.signum());
    let rot = rotation_of(p1, p2);
    let upright = (rot == 0 || rot == 180) != flipped;
    let size = |pct: f64| {
        Point::new(
            p1.x + sx * (f64::from(width) * UNITS_PER_PIXEL * pct / 100.0),
            p1.y + sy * (f64::from(height) * UNITS_PER_PIXEL * pct / 100.0),
        )
    };
    let grow = match mode {
        Resize::Original => return Some(size(100.0)),
        Resize::Percent(pct) => {
            let pct = if pct.is_finite() && pct > 0.0 {
                pct.min(100_000.0)
            } else {
                0.001
            };
            return Some(size(pct));
        }
        Resize::Expand => true,
        Resize::Shrink => false,
    };
    let (ax, ay) = (dx.abs(), dy.abs());
    // The box's ratio in the picture's own direction.
    let ratio = if upright { ay / ax } else { ax / ay };
    // Expand changes the side that makes the box larger, shrink the one
    // that makes it smaller.
    let change_first = if grow { ratio < orig_ratio } else { ratio > orig_ratio };
    let mut q = p2;
    if upright {
        if change_first {
            q.y = p1.y + sy * (ax * orig_ratio).trunc();
        } else {
            q.x = p1.x + sx * (ay / orig_ratio).trunc();
        }
    } else if change_first {
        q.x = p1.x + sx * (ay * orig_ratio).trunc();
    } else {
        q.y = p1.y + sy * (ax / orig_ratio).trunc();
    }
    Some(q)
}

pub(super) fn commands() -> Vec<Command> {
    let cmd = |id, label, params, mutates, run| Command {
        id,
        label,
        menu: "",
        shortcut: "",
        params,
        mutates,
        enabled: super::always,
        run,
    };
    vec![
        cmd(
            "picture.resize",
            "Resize picture",
            "{\"id\", \"mode\": shrink|expand|original|scale, \"percent\"?: number (scale)} — the picture panel's size buttons; moves the opposite corner",
            true,
            resize,
        ),
        cmd(
            "picture.rotate",
            "Rotate picture box",
            "{\"id\", \"rotation\": 0|90|180|270} — the picture panel's rotation; changes the corner order",
            true,
            rotate,
        ),
        cmd(
            "picture.set_file",
            "Reread / relink picture",
            "{\"id\", \"path\"?: string (default: the picture's file; Reread)} — loads the file again or points the picture at another one (Relink)",
            true,
            set_file,
        ),
        cmd(
            "picture.info",
            "Picture information",
            "{\"id\"} -> {file, exists, type, width, height, colours, transparent, rotation, flipped}",
            false,
            info_cmd,
        ),
        cmd(
            "picture.store",
            "Keep a picture in memory",
            "{\"name\": string, \"base64\": string} — keeps the picture's bytes under its file name for the session; pictures with that file name (Picture mode, Reread, Relink, a merged or opened figure) then load from memory when the file is not on disk. The browser has no files, so its Picture mode uses this",
            false,
            store,
        ),
        cmd(
            "picture.edit_image",
            "Edit picture's image",
            "{\"id\"} — opens the picture's file in the system's default application (native only)",
            false,
            edit_image,
        ),
    ]
}

/// Corners and picture of picture object `id`.
fn target(e: &Engine, p: &Value) -> Result<(ObjectId, Point, Point, Picture), EngineError> {
    let id = ObjectId(num(p, "id")?.round().max(0.0) as u64);
    let o = e
        .doc()
        .get(id)
        .ok_or_else(|| EngineError::not_found(format!("no top-level object with id {}", id.0)))?;
    if let ObjectKind::Polyline(pl) = &o.kind {
        if let (Some(pic), Some(a), Some(b)) = (&pl.picture, pl.points.first(), pl.points.get(2)) {
            return Ok((id, *a, *b, pic.clone()));
        }
    }
    Err(EngineError::bad_params("object is not a picture"))
}

fn set_corners(e: &mut Engine, id: ObjectId, a: Point, b: Point) {
    if let Some(o) = e.doc_mut().get_mut(id) {
        if let ObjectKind::Polyline(pl) = &mut o.kind {
            pl.points = vec![a, Point::new(b.x, a.y), b, Point::new(a.x, b.y)];
        }
    }
}

fn resize(e: &mut Engine, p: &Value) -> CmdResult {
    let (id, a, b, pic) = target(e, p)?;
    let mode = match string(p, "mode")?.as_str() {
        "shrink" => Resize::Shrink,
        "expand" => Resize::Expand,
        "original" => Resize::Original,
        "scale" => Resize::Percent(opt_num(p, "percent")?.unwrap_or(100.0)),
        _ => {
            return Err(EngineError::bad_params(
                "mode must be shrink, expand, original or scale",
            ))
        }
    };
    let (w, h) = pic.image.as_ref().map(|i| (i.width, i.height)).unwrap_or((0, 0));
    let q = resize_corner(mode, a, b, pic.flipped, w, h)
        .ok_or_else(|| EngineError::bad_params("the picture has no size or the box no area"))?;
    if q.x.round() == a.x.round() || q.y.round() == a.y.round() || !q.is_finite() {
        return Err(EngineError::bad_params("the box would have no area"));
    }
    set_corners(e, id, a, q);
    Ok(json!({"id": id.0, "corner": [q.x, q.y]}))
}

fn rotate(e: &mut Engine, p: &Value) -> CmdResult {
    let (id, a, b, _) = target(e, p)?;
    let deg = num(p, "rotation")?.round() as i32;
    let q = with_rotation(a, b, deg).ok_or_else(|| EngineError::bad_params("rotation must be 0, 90, 180 or 270"))?;
    set_corners(e, id, a, q);
    Ok(json!({"id": id.0, "rotation": deg}))
}

fn store(_e: &mut Engine, p: &Value) -> CmdResult {
    let name = string(p, "name")?;
    let bytes = crate::base64_decode(&string(p, "base64")?);
    // Only what the image crate or readpics can read is worth keeping.
    if bytes.is_empty() {
        return Err(EngineError::bad_params("no picture data"));
    }
    import::store_picture(&name, bytes).map_err(EngineError::bad_params)?;
    Ok(json!({"name": name}))
}

fn set_file(e: &mut Engine, p: &Value) -> CmdResult {
    let (id, _, _, pic) = target(e, p)?;
    let path = match p.get("path") {
        None | Some(Value::Null) => pic.file.clone(),
        Some(_) => string(p, "path")?,
    };
    if path.is_empty() {
        return Err(EngineError::bad_params("the picture has no file name"));
    }
    let img = import::load_picture(std::path::Path::new(&path)).map_err(EngineError::io)?;
    if let Some(o) = e.doc_mut().get_mut(id) {
        if let ObjectKind::Polyline(pl) = &mut o.kind {
            if let Some(pic) = &mut pl.picture {
                pic.file = path.clone();
                pic.image = Some(Shared::new(img));
            }
        }
    }
    Ok(json!({"id": id.0, "file": path}))
}

fn info_cmd(e: &mut Engine, p: &Value) -> CmdResult {
    let (id, a, b, pic) = target(e, p)?;
    let i = info(&pic);
    Ok(json!({
        "id": id.0, "file": pic.file, "exists": i.exists, "type": i.kind,
        "width": i.width, "height": i.height, "colours": i.colours,
        "transparent": i.transparent, "rotation": rotation_of(a, b), "flipped": pic.flipped,
    }))
}

fn edit_image(e: &mut Engine, p: &Value) -> CmdResult {
    let (id, _, _, pic) = target(e, p)?;
    if !std::path::Path::new(&pic.file).is_file() {
        return Err(EngineError::not_found(format!("{}: file not found", pic.file)));
    }
    let editor = e.prefs.str("image_editor").unwrap_or_default();
    open_external(&pic.file, &editor)?;
    Ok(json!({"id": id.0, "file": pic.file}))
}

/// The words of an editor command line with `%f` replaced by the file (the
/// file is added as the last word when there is no `%f`), as xfig's Image
/// editor setting; `None` for xfig's default (the system's own program).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn editor_command(editor: &str, file: &str) -> Option<Vec<String>> {
    let words: Vec<&str> = editor.split_whitespace().collect();
    // The system's opener under any system's name means "the default".
    if words.is_empty()
        || matches!(editor.trim(), "xdg-open" | "open" | "start")
        || editor.trim() == crate::prefs::SYSTEM_OPEN
    {
        return None;
    }
    let mut out: Vec<String> = words.iter().map(|w| w.replace("%f", file)).collect();
    if !editor.contains("%f") {
        out.push(file.to_string());
    }
    Some(out)
}

#[cfg(not(target_arch = "wasm32"))]
fn open_external(file: &str, editor: &str) -> Result<(), EngineError> {
    use std::process::Command as Process;
    if let Some(words) = editor_command(editor, file) {
        let Some((prog, args)) = words.split_first() else {
            return Err(EngineError::io("the image editor command is empty"));
        };
        return Process::new(prog)
            .args(args)
            .spawn()
            .map(|_| ())
            .map_err(|err| EngineError::io(format!("can't start the image editor {prog}: {err}")));
    }
    let mut c = if cfg!(target_os = "macos") {
        let mut c = Process::new("open");
        c.arg(file);
        c
    } else if cfg!(target_os = "windows") {
        let mut c = Process::new("cmd");
        c.args(["/C", "start", ""]).arg(file);
        c
    } else {
        let mut c = Process::new("xdg-open");
        c.arg(file);
        c
    };
    c.spawn()
        .map(|_| ())
        .map_err(|err| EngineError::io(format!("can't start an editor for {file}: {err}")))
}

#[cfg(target_arch = "wasm32")]
fn open_external(_file: &str, _editor: &str) -> Result<(), EngineError> {
    Err(EngineError::io("Edit Image is not available in the web app"))
}

#[cfg(test)]
mod editor_tests {
    use super::editor_command;

    #[test]
    fn image_editor_command_takes_the_file_for_percent_f() {
        assert_eq!(editor_command("xdg-open", "a.png"), None);
        assert_eq!(editor_command("  ", "a.png"), None);
        assert_eq!(
            editor_command("gimp %f", "a.png"),
            Some(vec!["gimp".to_string(), "a.png".to_string()])
        );
        // Without %f the file is the last word.
        assert_eq!(
            editor_command("xv -quit", "a.png"),
            Some(vec!["xv".to_string(), "-quit".to_string(), "a.png".to_string()])
        );
    }
}
