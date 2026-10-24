//! The bundled libraries in the browser (C2): there is no file system, so the
//! page serves `libraries/` as static files next to `libraries.json`, an index
//! (`cargo xtask web` writes it) of the library names and their figure names.
//! This module keeps the parsed index and the figures fetched so far; the UI
//! does the fetching (`web_sys`) and puts the bytes in with [`put`]. The code
//! is plain Rust and runs natively too, so it is tested there.

use crate::library::{Library, MAX_LIBRARIES, MAX_OBJECTS};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The folder name the bundle is served under (relative to the page).
pub const ROOT: &str = "libraries";
/// The index's file name, next to the page.
pub const INDEX_FILE: &str = "libraries.json";
/// Fetched figures kept, in bytes; the cache is emptied when it grows past.
const MAX_CACHE_BYTES: usize = 64 << 20;

#[derive(Default)]
struct State {
    index: Option<Vec<Library>>,
    names: HashMap<PathBuf, Vec<String>>,
    figures: HashMap<String, Arc<Vec<u8>>>,
    bytes: usize,
}

fn state() -> &'static Mutex<State> {
    static S: std::sync::OnceLock<Mutex<State>> = std::sync::OnceLock::new();
    S.get_or_init(Default::default)
}

fn key(dir: &Path, name: &str) -> String {
    format!("{}/{name}", dir.to_string_lossy())
}

/// Parse the index: `{"libraries": [{"name": "Electronic / Schematic",
/// "dir": "Electronic/Schematic", "figures": ["and", "or"]}, …]}`. Names with
/// a `..` or a leading `/` in `dir` are refused (they would leave `libraries/`).
pub fn parse_index(text: &str) -> Result<Vec<(Library, Vec<String>)>, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("library index: {e}"))?;
    let list = v
        .get("libraries")
        .and_then(Value::as_array)
        .ok_or("library index: no \"libraries\" list")?;
    let mut out = Vec::new();
    for l in list.iter().take(MAX_LIBRARIES) {
        let name = l.get("name").and_then(Value::as_str).unwrap_or_default();
        let dir = l.get("dir").and_then(Value::as_str).unwrap_or_default();
        if name.is_empty()
            || dir.is_empty()
            || dir.starts_with('/')
            || dir.split('/').any(|s| s == ".." || s.is_empty())
        {
            continue;
        }
        let figures: Vec<String> = l
            .get("figures")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .filter(|n| crate::library::valid_name(n))
                    .take(MAX_OBJECTS)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        out.push((
            Library {
                name: name.to_string(),
                path: Path::new(ROOT).join(dir),
                user: false,
                objects: figures.len(),
            },
            figures,
        ));
    }
    Ok(out)
}

/// Install the index text the page served.
pub fn set_index(text: &str) -> Result<usize, String> {
    let parsed = parse_index(text)?;
    let n = parsed.len();
    let mut s = state().lock().map_err(|_| "library state is poisoned".to_string())?;
    s.names = parsed.iter().map(|(l, f)| (l.path.clone(), f.clone())).collect();
    s.index = Some(parsed.into_iter().map(|(l, _)| l).collect());
    Ok(n)
}

/// Whether the index is in.
pub fn index_loaded() -> bool {
    state().lock().map(|s| s.index.is_some()).unwrap_or(false)
}

/// The bundled libraries (none before the index is in).
pub fn libraries() -> Vec<Library> {
    state().lock().ok().and_then(|s| s.index.clone()).unwrap_or_default()
}

/// The figure names of the library in `dir`.
pub fn figure_names(dir: &Path) -> Vec<String> {
    state()
        .lock()
        .ok()
        .and_then(|s| s.names.get(dir).cloned())
        .unwrap_or_default()
}

/// A fetched figure's bytes.
pub fn cached(dir: &Path, name: &str) -> Option<Arc<Vec<u8>>> {
    state().lock().ok()?.figures.get(&key(dir, name)).cloned()
}

/// Keep a fetched figure.
pub fn put(dir: &Path, name: &str, bytes: Vec<u8>) {
    if let Ok(mut s) = state().lock() {
        if s.bytes + bytes.len() > MAX_CACHE_BYTES {
            s.figures.clear();
            s.bytes = 0;
        }
        s.bytes += bytes.len();
        s.figures.insert(key(dir, name), Arc::new(bytes));
    }
}

/// Where the page serves a figure, relative to the page, each folder or file
/// name percent-encoded (`libraries/Electronic/a%20b.fig`).
pub fn figure_url(dir: &Path, name: &str) -> String {
    fn enc(seg: &str) -> String {
        let mut o = String::new();
        for b in seg.bytes() {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                o.push(char::from(b));
            } else {
                o.push_str(&format!("%{b:02X}"));
            }
        }
        o
    }
    let mut parts: Vec<String> = dir
        .to_string_lossy()
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .map(enc)
        .collect();
    parts.push(format!("{}.fig", enc(name)));
    parts.join("/")
}

/// The index of a library folder (what `cargo xtask web` writes), for tests and
/// tools: every folder holding `.fig` files, named `Outer / Inner`.
#[cfg(not(target_arch = "wasm32"))]
pub fn generate_index(root: &Path) -> String {
    let libs = crate::library::scan(&[crate::library::LibRoot {
        path: root.to_path_buf(),
        user: false,
        name: None,
    }]);
    let list: Vec<Value> = libs
        .iter()
        .filter_map(|l| {
            let rel = l.path.strip_prefix(root).ok()?;
            let dir: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            Some(serde_json::json!({
                "name": l.name,
                "dir": dir.join("/"),
                "figures": crate::library::object_names(&l.path),
            }))
        })
        .collect();
    serde_json::json!({ "libraries": list }).to_string()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn generated_index_parses_and_lists_the_bundle() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries");
        let text = generate_index(&root);
        let parsed = parse_index(&text).unwrap();
        assert!(parsed.len() > 20, "{}", parsed.len());
        let arrows = parsed.iter().find(|(l, _)| l.name == "Arrows").unwrap();
        assert!(arrows.1.iter().any(|n| n == "3darrow1"));
        assert_eq!(arrows.0.objects, arrows.1.len());
    }

    #[test]
    fn index_refuses_paths_out_of_the_folder_and_bad_names() {
        let t = r#"{"libraries":[
            {"name":"A","dir":"../etc","figures":["x"]},
            {"name":"B","dir":"/abs","figures":["x"]},
            {"name":"C","dir":"C/sub","figures":["ok","../bad",".hidden","a/b"]}]}"#;
        let p = parse_index(t).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].1, vec!["ok".to_string()]);
        assert!(parse_index("{}").is_err());
        assert!(parse_index("nonsense").is_err());
    }

    #[test]
    fn urls_are_percent_encoded_and_cache_round_trips() {
        let dir = Path::new("libraries").join("A b").join("c");
        assert_eq!(figure_url(&dir, "x y"), "libraries/A%20b/c/x%20y.fig");
        assert!(cached(&dir, "zz").is_none());
        put(&dir, "zz", b"abc".to_vec());
        assert_eq!(cached(&dir, "zz").unwrap().as_slice(), b"abc");
    }

    #[test]
    fn comments_come_from_bytes() {
        let fig = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n# first\n# second\n1200 2\n2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n 0 0 100 100\n";
        assert_eq!(comments_of_bytes_for_test(fig), vec!["first", "second"]);
    }

    fn comments_of_bytes_for_test(s: &str) -> Vec<String> {
        crate::library::comments_of_bytes(s.as_bytes())
    }
}
