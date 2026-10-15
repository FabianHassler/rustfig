//! Project automation: `cargo xtask ci | layers | assets | wasm`.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Workspace crates and their layers.
const LAYERS: &[(&str, &str, u32)] = &[
    ("rustfig-geom", "crates/geom", 0),
    ("rustfig-model", "crates/model", 1),
    ("rustfig-text", "crates/text", 1),
    ("rustfig-format", "crates/format", 2),
    ("rustfig-render", "crates/render", 3),
    ("rustfig-engine", "crates/engine", 4),
    ("rustfig-mcp", "crates/mcp", 5),
    ("rustfig-ui-egui", "crates/ui-egui", 6),
    ("rustfig", "apps/rustfig", 7),
    ("rustfig-cli", "apps/cli", 7),
    ("fig2dev", "apps/fig2dev", 7),
];

/// Dependencies allowed only from layer 6 up.
const UI_ONLY: &[&str] = &[
    "egui",
    "eframe",
    "egui-winit",
    "egui_glow",
    "egui-wgpu",
    "winit",
    "rfd",
    "epaint",
];

/// Asset file extensions that need an ASSETS.md row.
const ASSET_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "svg", "ico", "icns", "bmp", "webp", "ttf", "otf", "woff", "woff2", "wav", "mp3",
    "ogg",
];
const FONT_EXTS: &[&str] = &["ttf", "otf", "woff", "woff2", "ttc", "pfb", "afm"];
const SKIP_DIRS: &[&str] = &["target", ".git", ".cargo-home", "old", "plan", "dist", "node_modules"];

/// The workspace root, found at run time: the nearest directory at or above
/// the current one whose Cargo.toml declares `[workspace]`. A path fixed at
/// compile time would go stale when the checkout moves and the xtask binary
/// isn't rebuilt.
fn root() -> PathBuf {
    let is_root = |d: &Path| {
        std::fs::read_to_string(d.join("Cargo.toml")).is_ok_and(|m| m.lines().any(|l| l.trim() == "[workspace]"))
    };
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(d) = cwd.ancestors().find(|d| is_root(d)) {
            return d.to_path_buf();
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

fn run(cmd: &mut Command) -> Result<(), String> {
    eprintln!("$ {cmd:?}");
    let status = cmd.current_dir(root()).status().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed: {cmd:?}"))
    }
}

/// Cargo of the running toolchain, using that toolchain's rustc too
/// (unless RUSTC is set), so `cargo +toolchain xtask …` stays on it.
fn cargo() -> Command {
    let mut c = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    if std::env::var_os("RUSTC").is_none() {
        if let Some(r) = sibling_rustc() {
            c.env("RUSTC", r);
        }
    }
    c
}

/// Names of the dependencies declared in a Cargo.toml (all dependency tables).
fn dependencies(manifest: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_deps = l.contains("dependencies");
            continue;
        }
        if !in_deps || l.is_empty() || l.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = l.split_once('=') {
            let name = name.trim().split('.').next().unwrap_or("").trim().trim_matches('"');
            if !name.is_empty() {
                deps.push(name.to_string());
            }
        }
    }
    deps
}

fn layers() -> Result<(), String> {
    let mut errors = Vec::new();
    for (name, dir, layer) in LAYERS {
        let manifest =
            std::fs::read_to_string(root().join(dir).join("Cargo.toml")).map_err(|e| format!("{dir}: {e}"))?;
        for dep in dependencies(&manifest) {
            if let Some((_, _, dl)) = LAYERS.iter().find(|(n, _, _)| *n == dep) {
                if dl >= layer && dep != *name {
                    errors.push(format!("{name} (L{layer}) depends on {dep} (L{dl})"));
                }
            }
            if *layer < 6 && UI_ONLY.contains(&dep.as_str()) {
                errors.push(format!("{name} (L{layer}) depends on UI crate {dep}"));
            }
        }
    }
    if errors.is_empty() {
        println!("layers: ok");
        Ok(())
    } else {
        Err(format!("layer violations:\n  {}", errors.join("\n  ")))
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if !SKIP_DIRS.contains(&name.as_str()) && !name.starts_with("target") {
                walk(&p, out);
            }
        } else {
            out.push(p);
        }
    }
}

fn assets() -> Result<(), String> {
    let root = root();
    let table = std::fs::read_to_string(root.join("ASSETS.md")).map_err(|e| format!("ASSETS.md: {e}"))?;
    let mut files = Vec::new();
    walk(&root, &mut files);
    let mut errors = Vec::new();
    let mut count = 0;
    for f in files {
        let ext = f
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let rel = f.strip_prefix(&root).unwrap_or(&f).to_string_lossy().replace('\\', "/");
        if FONT_EXTS.contains(&ext.as_str()) {
            errors.push(format!(
                "{rel}: font files belong in storytold/craft-fonts, not this repository"
            ));
            continue;
        }
        if ASSET_EXTS.contains(&ext.as_str()) {
            count += 1;
            let listed = table
                .lines()
                .any(|l| l.trim_start().starts_with('|') && l.contains(&format!("`{rel}`")));
            if !listed {
                errors.push(format!("{rel}: no row in ASSETS.md"));
            }
        }
    }
    if errors.is_empty() {
        println!("assets: ok ({count} attributed)");
        Ok(())
    } else {
        Err(format!("asset problems:\n  {}", errors.join("\n  ")))
    }
}

/// Every Rust file with code ported from xfig or fig2dev names its sources
/// and points to LICENSE-xfig, and LICENSE-xfig lists exactly those files.
fn licences() -> Result<(), String> {
    let root = root();
    let notice = std::fs::read_to_string(root.join("LICENSE-xfig")).map_err(|e| format!("LICENSE-xfig: {e}"))?;
    let listed: Vec<String> = notice
        .lines()
        .filter_map(|l| l.strip_prefix("- "))
        .filter_map(|l| l.split_once(':').map(|(p, _)| p.trim().to_string()))
        .collect();
    let mut files = Vec::new();
    walk(&root, &mut files);
    let mut errors = Vec::new();
    let mut cited = Vec::new();
    for f in files.iter().filter(|f| f.extension().is_some_and(|e| e == "rs")) {
        let rel = f.strip_prefix(&root).unwrap_or(f).to_string_lossy().replace('\\', "/");
        if rel.starts_with("xtask/") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(f) else { continue };
        let ported = text.contains("Ported from xfig") || text.contains("Ported from fig2dev");
        let cites = text.contains("LICENSE-xfig");
        if ported && !cites {
            errors.push(format!(
                "{rel}: ported from xfig or fig2dev but doesn't point to LICENSE-xfig"
            ));
        }
        if cites {
            if !listed.contains(&rel) {
                errors.push(format!("{rel}: cites LICENSE-xfig but isn't listed there"));
            }
            cited.push(rel);
        }
    }
    for l in &listed {
        if !cited.contains(l) {
            errors.push(format!(
                "LICENSE-xfig lists {l}, which doesn't exist or doesn't cite it"
            ));
        }
    }
    if errors.is_empty() {
        println!("licences: ok ({} files derived from xfig and fig2dev)", cited.len());
        Ok(())
    } else {
        Err(format!("licence problems:\n  {}", errors.join("\n  ")))
    }
}

/// The rustc of the toolchain running this cargo (its sibling binary), so a
/// `cargo +toolchain xtask` checks that toolchain, not whichever rustc comes
/// first on PATH.
fn sibling_rustc() -> Option<PathBuf> {
    std::env::var_os("CARGO")
        .map(PathBuf::from)
        .and_then(|c| {
            c.parent()
                .map(|d| d.join(if cfg!(windows) { "rustc.exe" } else { "rustc" }))
        })
        .filter(|r| r.is_file())
}

fn rustc() -> Command {
    Command::new(sibling_rustc().unwrap_or_else(|| PathBuf::from("rustc")))
}

fn wasm_target_installed() -> bool {
    let out = rustc().args(["--print", "sysroot"]).output();
    let Ok(out) = out else { return false };
    let sysroot = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Path::new(&sysroot).join("lib/rustlib/wasm32-unknown-unknown").is_dir()
}

fn wasm() -> Result<(), String> {
    if !wasm_target_installed() {
        if std::env::var_os("RUSTFIG_REQUIRE_WASM").is_some() {
            return Err("wasm32-unknown-unknown is not installed (rustup target add wasm32-unknown-unknown)".into());
        }
        eprintln!("warning: wasm32-unknown-unknown target not installed; wasm check SKIPPED");
        eprintln!("         install it (rustup target add wasm32-unknown-unknown) or set RUSTFIG_REQUIRE_WASM=1 in CI");
        return Ok(());
    }
    run(cargo().args(["check", "--target", "wasm32-unknown-unknown", "-p", "rustfig"]))
}

fn ci() -> Result<(), String> {
    run(cargo().args(["fmt", "--all", "--check"]))?;
    run(cargo().args(["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]))?;
    run(cargo().args(["test", "--workspace"]))?;
    layers()?;
    assets()?;
    licences()?;
    wasm()?;
    println!("ci: all gates passed");
    Ok(())
}

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    let r = match task.as_str() {
        "ci" => ci(),
        "layers" => layers(),
        "assets" => assets(),
        "licences" => licences(),
        "wasm" => wasm(),
        _ => Err("usage: cargo xtask ci | layers | assets | licences | wasm".into()),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dependencies() {
        let m = "[package]\nname = \"x\"\n[dependencies]\nfoo.workspace = true\nbar = \"1\"\n[target.'cfg(x)'.dependencies]\nrfd = { workspace = true }\n";
        assert_eq!(dependencies(m), vec!["foo", "bar", "rfd"]);
    }
}
