//! Project automation: `cargo xtask ci | layers | assets | licences | wasm | web | source-audit`.

mod audit;
mod package;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Workspace crates and their layers. `figbase` and `fig2dev` live in
/// https://github.com/FabianHassler/fig2dev (git dependencies) and sit below every
/// RustFig layer, so they are not listed.
const LAYERS: &[(&str, &str, i32)] = &[
    ("rustfig-geom", "crates/geom", 0),
    ("rustfig-model", "crates/model", 1),
    ("rustfig-text", "crates/text", 1),
    ("rustfig-format", "crates/format", 2),
    ("rustfig-render", "crates/render", 3),
    ("rustfig-engine", "crates/engine", 4),
    ("rustfig-mcp", "crates/mcp", 5),
    ("rustfig-os", "crates/os", 5),
    ("rustfig-ui-egui", "crates/ui-egui", 6),
    ("rustfig", "apps/rustfig", 7),
    ("rustfig-cli", "apps/cli", 7),
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
    "ogg", "xpm", "xbm", "ps", "eps",
];
const FONT_EXTS: &[&str] = &["ttf", "otf", "woff", "woff2", "ttc", "pfb", "afm"];
// `.claude` holds agent worktrees: copies of the repository, not part of it.
const SKIP_DIRS: &[&str] = &[
    "target",
    ".git",
    ".cargo-home",
    ".claude",
    "old",
    "plan",
    "dist",
    "node_modules",
];

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

fn layer_name(layer: i32) -> String {
    if layer < 0 {
        "base".to_string()
    } else {
        format!("L{layer}")
    }
}

fn layers() -> Result<(), String> {
    let mut errors = Vec::new();
    for (name, dir, layer) in LAYERS {
        let manifest =
            std::fs::read_to_string(root().join(dir).join("Cargo.toml")).map_err(|e| format!("{dir}: {e}"))?;
        for dep in dependencies(&manifest) {
            if let Some((_, _, dl)) = LAYERS.iter().find(|(n, _, _)| *n == dep) {
                if dl >= layer && dep != *name {
                    errors.push(format!(
                        "{name} ({}) depends on {dep} ({})",
                        layer_name(*layer),
                        layer_name(*dl)
                    ));
                }
            }
            if *layer < 6 && UI_ONLY.contains(&dep.as_str()) {
                errors.push(format!("{name} ({}) depends on UI crate {dep}", layer_name(*layer)));
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
            // `wt-*`: git worktrees of parallel agents (excluded from git).
            if !SKIP_DIRS.contains(&name.as_str()) && !name.starts_with("target") && !name.starts_with("wt-") {
                walk(&p, out);
            }
        } else {
            out.push(p);
        }
    }
}

/// The one font file the repository ships (decided 2026-10-22): the key-symbol subset
/// (SIL OFL 1.1, `assets/fonts/OFL.txt`). Matched by path and a size cap, so any other
/// font, or a bigger one under that name, still fails.
const ALLOWED_FONT: &str = "assets/fonts/rustfig-key-symbols.ttf";
const ALLOWED_FONT_MAX_BYTES: u64 = 16 * 1024;

fn is_allowed_font(rel: &str, path: &Path) -> bool {
    allowed_font(rel, std::fs::metadata(path).map(|m| m.len()).unwrap_or(u64::MAX))
}

fn allowed_font(rel: &str, len: u64) -> bool {
    rel == ALLOWED_FONT && len <= ALLOWED_FONT_MAX_BYTES
}

#[cfg(test)]
mod font_tests {
    use super::allowed_font;

    #[test]
    fn only_the_key_symbol_font_is_allowed() {
        assert!(allowed_font("assets/fonts/rustfig-key-symbols.ttf", 2000));
        assert!(!allowed_font("assets/fonts/rustfig-key-symbols.ttf", 600_000));
        assert!(!allowed_font("assets/fonts/other.ttf", 2000));
        assert!(!allowed_font("crates/x/rustfig-key-symbols.ttf", 2000));
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
        if FONT_EXTS.contains(&ext.as_str()) && !is_allowed_font(&rel, &f) {
            errors.push(format!(
                "{rel}: font files do not belong in this repository (only the key-symbol subset, listed in ASSETS.md)"
            ));
            continue;
        }
        // xfig's figure libraries (D6): a `.fig` file under `libraries/` is an asset
        // covered by the row of its library folder, `libraries/<Name>/`.
        if ext == "fig" {
            if let Some(top) = rel
                .strip_prefix("libraries/")
                .and_then(|r| r.split_once('/'))
                .map(|(t, _)| t)
            {
                count += 1;
                let key = format!("`libraries/{top}/`");
                let listed = table
                    .lines()
                    .any(|l| l.trim_start().starts_with('|') && l.contains(&key));
                if !listed {
                    errors.push(format!("{rel}: no row for libraries/{top}/ in ASSETS.md"));
                }
            }
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
    // CI runs the wasm check once (on Linux); the other desktops opt out explicitly.
    if std::env::var_os("RUSTFIG_SKIP_WASM").is_some_and(|v| !v.is_empty() && v != "0") {
        eprintln!("wasm check skipped (RUSTFIG_SKIP_WASM is set)");
        return Ok(());
    }
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

/// `trunk` on `PATH`, or in cargo's bin folder (where `cargo install` puts it).
fn find_trunk() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "trunk.exe" } else { "trunk" };
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    if let Some(h) = std::env::var_os("CARGO_HOME") {
        dirs.push(Path::new(&h).join("bin"));
    }
    if let Some(h) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        dirs.push(Path::new(&h).join(".cargo/bin"));
    }
    dirs.into_iter().map(|d| d.join(exe)).find(|p| p.is_file())
}

/// JSON string with the escapes a file name can need.
fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// The index of the bundled figure libraries for the web build (C2): every
/// folder under `root` with `.fig` files is a library named `Outer / Inner`
/// (as the engine's scan names them), with its figures' names. The browser
/// fetches `libraries.json` and then the figures it draws, lazily.
fn libraries_index(root: &Path) -> Result<String, String> {
    fn walk(dir: &Path, rel: &[String], out: &mut Vec<String>) -> Result<(), String> {
        let mut figs: Vec<String> = Vec::new();
        let mut subs: Vec<(String, PathBuf)> = Vec::new();
        for e in std::fs::read_dir(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .flatten()
        {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if e.path().is_dir() {
                subs.push((name, e.path()));
            } else if name.len() > 4 && name.to_ascii_lowercase().ends_with(".fig") {
                figs.push(name[..name.len() - 4].to_string());
            }
        }
        // (figures directly in `libraries/` itself are not part of the bundle)
        if !figs.is_empty() && !rel.is_empty() {
            figs.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()).then(a.cmp(b)));
            let list: Vec<String> = figs.iter().map(|f| json_str(f)).collect();
            out.push(format!(
                "{{\"name\":{},\"dir\":{},\"figures\":[{}]}}",
                json_str(&rel.join(" / ")),
                json_str(&rel.join("/")),
                list.join(",")
            ));
        }
        subs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()).then(a.0.cmp(&b.0)));
        for (name, path) in subs {
            let mut r = rel.to_vec();
            r.push(name);
            walk(&path, &r, out)?;
        }
        Ok(())
    }
    let mut libs = Vec::new();
    walk(root, &[], &mut libs)?;
    Ok(format!("{{\"libraries\":[{}]}}\n", libs.join(",")))
}

/// `cargo xtask web [--debug] [--serve]`: the web app (C1), built by trunk
/// from `web/index.html` into `dist/` (index.html, the wasm, its JS glue).
/// `--serve` runs trunk's dev server instead (http://127.0.0.1:8080).
fn web() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let Some(trunk) = find_trunk() else {
        return Err(
            "trunk is not installed. Install it once with:\n  cargo install --locked trunk\n\
             (the build also needs the wasm target: rustup target add wasm32-unknown-unknown; \
             trunk downloads wasm-bindgen itself on the first build)"
                .into(),
        );
    };
    if !wasm_target_installed() {
        return Err("wasm32-unknown-unknown is not installed (rustup target add wasm32-unknown-unknown)".into());
    }
    // The bundled libraries' index, which `web/index.html` copies into dist/
    // beside the `libraries/` folder (the Library mode in the browser, C2).
    std::fs::write(
        root().join("web/libraries.json"),
        libraries_index(&root().join("libraries"))?,
    )
    .map_err(|e| format!("web/libraries.json: {e}"))?;
    let (html, dist) = (root().join("web/index.html"), root().join("dist"));
    let mut cmd = Command::new(trunk);
    if args.iter().any(|a| a == "--serve") {
        cmd.arg("serve");
    } else {
        cmd.arg("build");
    }
    cmd.arg(&html).arg("--dist").arg(&dist);
    // (after the path: `--release` takes an optional value)
    if !args.iter().any(|a| a == "--debug" || a == "--serve") {
        cmd.arg("--release");
    }
    // Where the site is served from, e.g. `/rustfig/` on GitHub Pages.
    if let Some(url) = args
        .iter()
        .position(|a| a == "--public-url")
        .and_then(|i| args.get(i + 1))
    {
        cmd.arg("--public-url").arg(url);
    }
    if std::env::var_os("RUSTC").is_none() {
        if let Some(r) = sibling_rustc() {
            cmd.env("RUSTC", r);
        }
    }
    // Trunk reads the package it builds from the Cargo.toml of its working
    // directory, which must name a package (the workspace root is virtual).
    cmd.current_dir(root().join("apps/rustfig"));
    eprintln!("$ {cmd:?}");
    let status = cmd.status().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("trunk failed: {status}"));
    }
    let wasm = std::fs::read_dir(&dist)
        .map_err(|e| format!("dist/: {e}"))?
        .filter_map(Result::ok)
        .any(|e| e.path().extension().is_some_and(|x| x == "wasm"));
    if !dist.join("index.html").is_file() || !wasm {
        return Err("trunk finished, but dist/ has no index.html and .wasm file".into());
    }
    println!("web: built dist/ (serve it with `cargo xtask web --serve`, or any static server)");
    Ok(())
}

fn ci() -> Result<(), String> {
    run(cargo().args(["fmt", "--all", "--check"]))?;
    run(cargo().args(["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]))?;
    // Tests must not depend on the fonts installed on the machine (F1).
    run(cargo()
        .env("FIG2DEV_BUILTIN_FONTS", "1")
        .args(["test", "--workspace", "--no-fail-fast"]))?;
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
        "web" => web(),
        "package" => package::run(&root(), cargo()),
        "source-audit" => audit::run(&root()),
        _ => Err("usage: cargo xtask ci | layers | assets | licences | wasm | web | package | source-audit".into()),
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
    fn libraries_index_lists_folders_with_figures() {
        let t = std::env::temp_dir().join(format!("rustfig-libidx-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&t);
        std::fs::create_dir_all(t.join("Outer/In ner")).unwrap();
        std::fs::create_dir_all(t.join("Empty")).unwrap();
        std::fs::write(t.join("Outer/b.fig"), "x").unwrap();
        std::fs::write(t.join("Outer/A.fig"), "x").unwrap();
        // Windows forbids `"` in file names; elsewhere it tests the escaping.
        let (quoted, escaped) = if cfg!(windows) {
            ("q'x", "q'x")
        } else {
            ("q\"x", "q\\\"x")
        };
        std::fs::write(t.join(format!("Outer/In ner/{quoted}.fig")), "x").unwrap();
        std::fs::write(t.join("Outer/README"), "x").unwrap();
        std::fs::write(t.join("top.fig"), "x").unwrap();
        let s = libraries_index(&t).unwrap();
        let _ = std::fs::remove_dir_all(&t);
        assert_eq!(
            s,
            format!(
                "{{\"libraries\":[{{\"name\":\"Outer\",\"dir\":\"Outer\",\"figures\":[\"A\",\"b\"]}},\
                 {{\"name\":\"Outer / In ner\",\"dir\":\"Outer/In ner\",\"figures\":[\"{escaped}\"]}}]}}\n"
            )
        );
    }

    #[test]
    fn parses_dependencies() {
        let m = "[package]\nname = \"x\"\n[dependencies]\nfoo.workspace = true\nbar = \"1\"\n[target.'cfg(x)'.dependencies]\nrfd = { workspace = true }\n";
        assert_eq!(dependencies(m), vec!["foo", "bar", "rfd"]);
    }
}
