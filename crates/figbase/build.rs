//! Embeds the optional craft-fonts collection.
//!
//! When `CRAFT_FONTS_DIR` is set to an absolute path of a checkout of
//! <https://github.com/storytold/craft-fonts>, every `.ttf`/`.otf` file under
//! it is embedded into `figbase::text::CRAFT_FONTS`. Without it the list is
//! empty and figbase falls back to the fonts bundled with its dependencies.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const MAX_FONTS: usize = 400;

fn collect(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 8 || out.len() >= MAX_FONTS {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
        {
            continue;
        }
        if p.is_dir() {
            collect(&p, depth + 1, out);
        } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            let ext = ext.to_ascii_lowercase();
            if (ext == "ttf" || ext == "otf") && out.len() < MAX_FONTS {
                out.push(p);
            }
        }
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=CRAFT_FONTS_DIR");
    let mut files = Vec::new();
    if let Ok(dir) = std::env::var("CRAFT_FONTS_DIR") {
        let dir = PathBuf::from(dir);
        if dir.is_absolute() && dir.is_dir() {
            println!("cargo:rerun-if-changed={}", dir.display());
            collect(&dir, 0, &mut files);
        } else {
            println!("cargo:warning=CRAFT_FONTS_DIR must be an absolute path to a directory; ignoring it");
        }
    }
    let mut code = String::from(
        "/// Fonts embedded from craft-fonts: (file stem, bytes).\npub static CRAFT_FONTS: &[(&str, &[u8])] = &[\n",
    );
    for f in &files {
        let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or("font");
        let _ = writeln!(code, "    ({:?}, include_bytes!({:?})),", stem, f.display().to_string());
    }
    code.push_str("];\n");
    let Some(out) = std::env::var_os("OUT_DIR") else {
        println!("cargo:warning=OUT_DIR not set");
        return;
    };
    if let Err(e) = std::fs::write(Path::new(&out).join("craft_fonts.rs"), code) {
        println!("cargo:warning=cannot write craft_fonts.rs: {e}");
    }
}
