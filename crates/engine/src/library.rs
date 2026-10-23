//! Figure libraries (plan D6): folders of `.fig` files the user places in a
//! drawing, as xfig's library mode does.
//!
//! A library is a folder with `.fig` files; folders inside it are libraries
//! of their own, named `Outer / Inner`. RustFig bundles xfig's libraries
//! (`libraries/` in the source tree, next to the program when installed, or
//! the folders in `RUSTFIG_LIBRARIES`) and reads the user's own from the
//! `library_dir` preference. The web build has no file system, so it lists
//! none.
//
// Ported from xfig src/w_library.c and src/e_placelib.c (xfig licence, see LICENSE-xfig).

use crate::tools::{Button, Key, ToolEvent, ToolKind, ToolState};
use crate::{Engine, EngineError, UiRequest};
use rustfig_geom::{Affine, Point, Rect};
#[cfg(not(target_arch = "wasm32"))]
use rustfig_model::{Compound, ObjectKind};
use rustfig_model::{Document, Object, ObjectId};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Most libraries listed.
pub const MAX_LIBRARIES: usize = 5_000;
/// Most objects listed in one library.
pub const MAX_OBJECTS: usize = 20_000;
/// Deepest folder nesting scanned.
#[cfg(not(target_arch = "wasm32"))]
const MAX_DEPTH: usize = 8;
/// Largest library figure read.
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// Most characters of an object name.
const MAX_NAME: usize = 120;
/// The name xfig gives the figures lying in the top folder itself.
pub const TOP_NAME: &str = "(Toplevel Directory)";

/// One library: a folder with figures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    /// `Electronic / Schematic / Classic`; unique in the list.
    pub name: String,
    pub path: PathBuf,
    /// From the user's folder, not bundled with RustFig.
    pub user: bool,
    /// How many figures it holds.
    pub objects: usize,
}

/// A change made to a library object in hand before it is placed (xfig's
/// keys r, l, h, v, s, S while placing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibOp {
    RotateCw,
    RotateCcw,
    FlipH,
    FlipV,
    Smaller,
    Larger,
}

impl LibOp {
    pub fn from_name(s: &str) -> Option<LibOp> {
        Some(match s {
            "rotate_cw" => LibOp::RotateCw,
            "rotate_ccw" => LibOp::RotateCcw,
            "flip_h" => LibOp::FlipH,
            "flip_v" => LibOp::FlipV,
            "smaller" => LibOp::Smaller,
            "larger" => LibOp::Larger,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            LibOp::RotateCw => "rotate_cw",
            LibOp::RotateCcw => "rotate_ccw",
            LibOp::FlipH => "flip_h",
            LibOp::FlipV => "flip_v",
            LibOp::Smaller => "smaller",
            LibOp::Larger => "larger",
        }
    }

    /// xfig's key for it (`transform_lib_obj`).
    pub fn from_key(c: char) -> Option<LibOp> {
        Some(match c {
            'r' => LibOp::RotateCw,
            'l' => LibOp::RotateCcw,
            'h' => LibOp::FlipH,
            'v' => LibOp::FlipV,
            's' => LibOp::Smaller,
            'S' => LibOp::Larger,
            _ => return None,
        })
    }

    fn affine(self) -> Affine {
        match self {
            LibOp::RotateCw => Affine::rotate(-std::f64::consts::FRAC_PI_2),
            LibOp::RotateCcw => Affine::rotate(std::f64::consts::FRAC_PI_2),
            LibOp::FlipH => Affine::scale(-1.0, 1.0),
            LibOp::FlipV => Affine::scale(1.0, -1.0),
            LibOp::Smaller => Affine::scale(0.9, 0.9),
            LibOp::Larger => Affine::scale(1.1, 1.1),
        }
    }
}

/// A library object read from its file, ready to place.
#[derive(Debug, Clone, PartialEq)]
pub struct LibObject {
    /// One compound holding the figure, where the file has it.
    pub object: Object,
    /// Its top-left corner in the file.
    pub nw: Point,
}

/// The folders bundled libraries are looked for in, most specific first:
/// `RUSTFIG_LIBRARIES` (folders separated as `PATH` is), `libraries` next to
/// the program, `../share/rustfig/libraries`, and the source tree's own.
#[cfg(not(target_arch = "wasm32"))]
pub fn bundled_roots() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    if let Some(p) = std::env::var_os("RUSTFIG_LIBRARIES") {
        v.extend(std::env::split_paths(&p).filter(|p| !p.as_os_str().is_empty()));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        v.push(dir.join("libraries"));
        v.push(dir.join("../share/rustfig/libraries"));
        // `cargo run`: the executable is in target/<profile>/.
        v.push(dir.join("../../libraries"));
    }
    // The source tree this build came from (development builds and tests).
    v.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries"));
    let mut out: Vec<PathBuf> = Vec::new();
    for p in v {
        if p.is_dir() && !out.iter().any(|o| same_dir(o, &p)) {
            out.push(p);
        }
    }
    // The first folder that exists is the bundle; the others would only repeat it
    // unless `RUSTFIG_LIBRARIES` named several on purpose.
    if std::env::var_os("RUSTFIG_LIBRARIES").is_none() {
        out.truncate(1);
    }
    out
}

#[cfg(target_arch = "wasm32")]
pub fn bundled_roots() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(not(target_arch = "wasm32"))]
fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// The user's library folder from the `library_dir` preference (`~` is the
/// home folder), if set and not empty.
pub fn user_root(pref: &str) -> Option<PathBuf> {
    let s = pref.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(rest) = s.strip_prefix('~') {
        let home = home_dir()?;
        let rest = rest.trim_start_matches(['/', '\\']);
        return Some(if rest.is_empty() { home } else { home.join(rest) });
    }
    Some(PathBuf::from(s))
}

#[cfg(not(target_arch = "wasm32"))]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

#[cfg(target_arch = "wasm32")]
fn home_dir() -> Option<PathBuf> {
    None
}

/// Every library under the folders (`true`: the user's).
pub fn scan(roots: &[(PathBuf, bool)]) -> Vec<Library> {
    let mut out: Vec<Library> = Vec::new();
    for (root, user) in roots {
        let mut found = Vec::new();
        scan_dir(root, "", *user, true, 0, &mut found);
        found.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then(a.name.cmp(&b.name))
        });
        for mut l in found {
            if out.iter().any(|o| o.name == l.name) {
                l.name = format!("{} ({})", l.name, if *user { "user" } else { "bundled" });
            }
            if out.len() >= MAX_LIBRARIES {
                return out;
            }
            out.push(l);
        }
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn scan_dir(dir: &Path, name: &str, user: bool, top: bool, depth: usize, out: &mut Vec<Library>) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut figs = 0;
    let mut subs = Vec::new();
    for e in rd.flatten() {
        let fname = e.file_name().to_string_lossy().into_owned();
        let path = e.path();
        if fname.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            subs.push((fname, path));
        } else if is_fig_name(&fname) {
            figs += 1;
        }
    }
    if figs > 0 {
        let lname = if top { TOP_NAME.to_string() } else { name.to_string() };
        out.push(Library {
            name: lname,
            path: dir.to_path_buf(),
            user,
            objects: figs,
        });
    }
    for (fname, path) in subs {
        let sub = if top { fname } else { format!("{name} / {fname}") };
        scan_dir(&path, &sub, user, false, depth + 1, out);
    }
}

#[cfg(target_arch = "wasm32")]
fn scan_dir(_dir: &Path, _name: &str, _user: bool, _top: bool, _depth: usize, _out: &mut Vec<Library>) {}

#[cfg(not(target_arch = "wasm32"))]
fn is_fig_name(n: &str) -> bool {
    n.len() > 4 && n.to_ascii_lowercase().ends_with(".fig")
}

/// The names of the figures in a library folder, sorted (without `.fig`).
#[cfg(not(target_arch = "wasm32"))]
pub fn object_names(dir: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<String> = rd
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            (is_fig_name(&n) && !n.starts_with('.')).then(|| n[..n.len() - 4].to_string())
        })
        .collect();
    v.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()).then(a.cmp(b)));
    v.truncate(MAX_OBJECTS);
    v
}

#[cfg(target_arch = "wasm32")]
pub fn object_names(_dir: &Path) -> Vec<String> {
    Vec::new()
}

/// Whether `name` can be a figure's file name without the extension: no
/// folders, nothing hidden.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= MAX_NAME
        && !name.starts_with('.')
        && !name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
}

/// The comment lines at the top of a library figure (what xfig shows under
/// "Object comments"), read without parsing the whole file.
#[cfg(not(target_arch = "wasm32"))]
pub fn object_comments(dir: &Path, name: &str) -> Vec<String> {
    use std::io::{BufRead, BufReader};
    if !valid_name(name) {
        return Vec::new();
    }
    let Ok(f) = std::fs::File::open(dir.join(format!("{name}.fig"))) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    // The `#FIG` line (and `#encoding`), then seven header lines (orientation,
    // justification, units, paper size, magnification, pages, transparent
    // colour); the figure's comment lines follow.
    let lines: Vec<String> = BufReader::new(f).lines().map_while(Result::ok).take(200).collect();
    let first = lines.iter().take_while(|l| l.starts_with('#')).count();
    for l in lines.into_iter().skip(first + 7) {
        let Some(c) = l.strip_prefix('#') else { break };
        out.push(c.strip_prefix(' ').unwrap_or(c).to_string());
        if out.len() >= 100 {
            break;
        }
    }
    out
}

#[cfg(target_arch = "wasm32")]
pub fn object_comments(_dir: &Path, _name: &str) -> Vec<String> {
    Vec::new()
}

/// Read a library figure as one drawing whose only object is one compound:
/// the file's compound where it holds nothing else, else all of its objects
/// in a new one; the figure's comments become the compound's. Pictures it
/// names are loaded from the library folder. The user colours stay the
/// file's own.
#[cfg(not(target_arch = "wasm32"))]
pub fn read_figure(dir: &Path, name: &str) -> Result<Document, String> {
    if !valid_name(name) {
        return Err(format!("bad object name {name:?}"));
    }
    let path = dir.join(format!("{name}.fig"));
    let meta = std::fs::metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!("{}: too large", path.display()));
    }
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (mut doc, _) = rustfig_format::read_any_messages(&bytes).map_err(|e| format!("{name}.fig: {e}"))?;
    // A picture that fails to load draws as its empty frame, as in xfig.
    let _ = crate::import::load_all_pictures(&mut doc, Some(dir));
    absolutize_pictures(&mut doc.objects, dir);
    let comments = std::mem::take(&mut doc.comments);
    let objects: Vec<Object> = std::mem::take(&mut doc.objects)
        .into_iter()
        .map(std::sync::Arc::unwrap_or_clone)
        .collect();
    if objects.is_empty() {
        return Err(format!("{name}.fig: empty library file"));
    }
    let mut one = match <[Object; 1]>::try_from(objects) {
        Ok([o]) if matches!(o.kind, ObjectKind::Compound(_)) => o,
        Ok([o]) => Object::new(ObjectKind::Compound(Compound { objects: vec![o] })),
        Err(objects) => Object::new(ObjectKind::Compound(Compound { objects })),
    };
    if !comments.is_empty() {
        one.comments = comments;
    }
    doc.objects = vec![std::sync::Arc::new(one)];
    Ok(doc)
}

#[cfg(target_arch = "wasm32")]
pub fn read_figure(_dir: &Path, _name: &str) -> Result<Document, String> {
    Err("libraries are not available here".into())
}

/// Picture names in library figures are relative to the library folder;
/// placed in a drawing they must keep pointing at the same files (xfig's
/// `REMAP_IMAGES`).
#[cfg(not(target_arch = "wasm32"))]
fn absolutize_pictures(objs: &mut [std::sync::Arc<Object>], dir: &Path) {
    fn walk(o: &mut Object, dir: &Path) {
        match &mut o.kind {
            ObjectKind::Compound(c) => c.objects.iter_mut().for_each(|o| walk(o, dir)),
            ObjectKind::Polyline(p) => {
                if let Some(pic) = &mut p.picture {
                    if !pic.file.is_empty() && !Path::new(&pic.file).is_absolute() {
                        pic.file = dir.join(&pic.file).to_string_lossy().into_owned();
                    }
                }
            }
            _ => {}
        }
    }
    for o in objs {
        walk(std::sync::Arc::make_mut(o), dir);
    }
}

/// The compound with `ops` applied about its centre (xfig applies them about
/// the pointer, which is the same up to where the result is placed).
pub fn with_ops(obj: &Object, ops: &[LibOp]) -> Object {
    let mut o = obj.clone();
    let bb = obj.geometry_bbox();
    if bb.is_empty() {
        return o;
    }
    let c = bb.center();
    for op in ops {
        o.transform(
            &Affine::about(op.affine(), c),
            *op == LibOp::Smaller || *op == LibOp::Larger,
        );
    }
    o
}

/// Top-left corner of an object's geometry.
pub fn nw_of(o: &Object) -> Point {
    let b: Rect = o.geometry_bbox();
    if b.is_empty() {
        Point::ZERO
    } else {
        b.min
    }
}

impl Engine {
    /// The folders libraries are read from: the bundled ones, then the
    /// user's (`library_dir`).
    pub fn library_roots(&self) -> Vec<(PathBuf, bool)> {
        let mut v: Vec<(PathBuf, bool)> = bundled_roots().into_iter().map(|p| (p, false)).collect();
        let pref = self
            .prefs
            .get("library_dir")
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "~/xfiglib".to_string());
        if let Some(u) = user_root(&pref) {
            v.push((u, true));
        }
        v
    }

    /// All libraries, bundled first.
    pub fn libraries(&self) -> Vec<Library> {
        scan(&self.library_roots())
    }

    /// The library called `name`.
    pub fn find_library(&self, name: &str) -> Result<Library, EngineError> {
        self.libraries()
            .into_iter()
            .find(|l| l.name == name)
            .ok_or_else(|| EngineError::bad_params(format!("no library {name:?} (see library.list)")))
    }

    /// Read a library object for placing, its user colours mapped into the
    /// drawing (as xfig's merge does).
    pub fn library_object(&mut self, library: &str, name: &str) -> Result<LibObject, EngineError> {
        let lib = self.find_library(library)?;
        self.library_object_in(&lib, name)
    }

    /// [`Engine::library_object`] in a library already found.
    pub fn library_object_in(&mut self, lib: &Library, name: &str) -> Result<LibObject, EngineError> {
        let doc = read_figure(&lib.path, name).map_err(EngineError::bad_params)?;
        let mut objs = crate::cmd::file::mapped_objects(self, doc);
        let object = objs
            .pop()
            .ok_or_else(|| EngineError::bad_params(format!("{name}.fig holds no objects")))?;
        let nw = nw_of(&object);
        Ok(LibObject { object, nw })
    }

    /// Library mode: the object follows the pointer (xfig's `put_selected`).
    pub fn begin_library(&mut self, library: &str, name: &str, ops: Vec<LibOp>) -> Result<(), EngineError> {
        let lo = self.library_object(library, name)?;
        let hand = with_ops(&lo.object, &ops);
        let nw = nw_of(&hand);
        let v = self.view.visible();
        let cursor = if v.is_empty() {
            nw
        } else {
            Point::new(v.min.x + 600.0, v.min.y + 600.0)
        };
        // xfig turns the current mode off: the library has its own.
        self.set_tool(ToolKind::None);
        self.tool_state = ToolState::Library {
            library: library.to_string(),
            name: name.to_string(),
            orig: Box::new(lo.object),
            ops,
            hand: Box::new(hand),
            nw,
            cursor,
            draw_box: false,
        };
        self.set_status(format!("Placing library object {name}.fig"));
        Ok(())
    }

    /// The library object in hand: (library, name, ops), if any.
    pub fn library_in_hand(&self) -> Option<(&str, &str, &[LibOp])> {
        match &self.tool_state {
            ToolState::Library { library, name, ops, .. } => Some((library, name, ops)),
            _ => None,
        }
    }

    /// Apply a change to the library object in hand (xfig's keys).
    pub fn library_transform(&mut self, op: LibOp) -> Result<(), EngineError> {
        let ToolState::Library {
            orig, ops, hand, nw, ..
        } = &mut self.tool_state
        else {
            return Err(EngineError::bad_params("no library object in hand"));
        };
        ops.push(op);
        // Keep the list short: four turns are none, two flips are none.
        if ops.len() > 64 {
            ops.remove(0);
        }
        **hand = with_ops(orig, ops);
        *nw = nw_of(hand);
        Ok(())
    }

    pub(crate) fn library_event(&mut self, ev: ToolEvent) -> Result<(), EngineError> {
        let ToolState::Library { library, name, ops, .. } = &self.tool_state else {
            return Ok(());
        };
        let (library, name) = (library.clone(), name.clone());
        let op_names: Vec<&str> = ops.iter().map(|o| o.name()).collect();
        let op_names = json!(op_names);
        match ev {
            ToolEvent::Move { pos } if pos.is_finite() => {
                let p = self.snap(pos);
                if let ToolState::Library { cursor, .. } = &mut self.tool_state {
                    *cursor = p;
                }
            }
            ToolEvent::Press { pos, button, shift } if pos.is_finite() => match (button, shift) {
                (Button::Left, shift) => {
                    let p = self.snap(pos);
                    let r = self.execute(
                        "library.place",
                        &json!({"library": library, "name": name, "ops": op_names, "x": p.x, "y": p.y}),
                    )?;
                    if shift {
                        // xfig: edit the object just placed.
                        if let Some(id) = r.get("id").and_then(Value::as_u64) {
                            self.request(UiRequest::EditObject { id: ObjectId(id) });
                        }
                    }
                }
                (Button::Middle, false) => {
                    // "new object": back to the library panel.
                    self.tool_state = ToolState::Idle;
                    self.request(UiRequest::OpenLibrary);
                }
                (Button::Middle, true) => {
                    if let ToolState::Library { draw_box, .. } = &mut self.tool_state {
                        *draw_box = !*draw_box;
                    }
                }
                (Button::Right, true) => {
                    self.execute(
                        "library.place",
                        &json!({"library": library, "name": name, "ops": op_names, "orig": true}),
                    )?;
                }
                (Button::Right, false) => {
                    self.tool_state = ToolState::Idle;
                    self.set_status("Library mode cancelled.");
                }
            },
            ToolEvent::Key(Key::Escape) => {
                self.tool_state = ToolState::Idle;
                self.set_status("Library mode cancelled.");
            }
            ToolEvent::Text(t) => {
                for c in t.chars().take(16) {
                    if let Some(op) = LibOp::from_key(c) {
                        self.library_transform(op)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
