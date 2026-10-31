//! `cargo xtask ctan [REF]`: the CTAN upload `target/ctan/rustfig.zip` of
//! git revision `REF` (default `HEAD`): one top directory `rustfig/` with
//! the source as `git archive` gives it, without the files CTAN refuses
//! (names starting with a dot: `.github/`, `.gitignore`, `.gitattributes`,
//! `.cargo/`), the CTAN README (`packaging/ctan/README.md`; the
//! repository's README becomes `README-repository.md`) and the manual as
//! `doc/rustfig-manual.pdf`, made from `docs/manual/*.md` in the order of
//! `index.md` with pandoc and LuaLaTeX (both needed).

use crate::package::{sh, workspace_version};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Top-level entries CTAN refuses (dot files), left out of the archive.
const LEFT_OUT: &[&str] = &[".github", ".gitignore", ".gitattributes", ".cargo"];

/// The manual's fonts: Charter and Menlo, Apple Symbols for ⌘ ⇧ ⌥ and the
/// arrows (macOS); `RUSTFIG_CTAN_FONTS="main,sans,mono,fallback"` overrides.
const FONTS: [&str; 4] = ["Charter", "Helvetica Neue", "Menlo", "Apple Symbols"];

pub fn run(root: &Path, rev: &str) -> Result<(), String> {
    let manifest = fs::read_to_string(root.join("Cargo.toml")).map_err(|e| format!("Cargo.toml: {e}"))?;
    let version = workspace_version(&manifest);
    let out = root.join("target/ctan");
    let top = out.join("rustfig");
    if out.exists() {
        fs::remove_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    fs::create_dir_all(&top).map_err(|e| format!("{}: {e}", top.display()))?;

    // The source of `rev`, without the dot files.
    let tar = out.join("source.tar");
    let mut archive = Command::new("git");
    archive
        .current_dir(root)
        .args(["archive", "--format=tar", "-o"])
        .arg(&tar)
        .arg(rev)
        .arg("--")
        .arg(".");
    for p in LEFT_OUT {
        archive.arg(format!(":(exclude){p}"));
    }
    sh(&mut archive)?;
    sh(Command::new("tar").arg("-xf").arg(&tar).arg("-C").arg(&top))?;
    fs::remove_file(&tar).map_err(|e| format!("{}: {e}", tar.display()))?;

    // CTAN's README on top.
    let readme = top.join("README.md");
    fs::rename(&readme, top.join("README-repository.md")).map_err(|e| format!("README.md: {e}"))?;
    fs::copy(top.join("packaging/ctan/README.md"), &readme).map_err(|e| format!("packaging/ctan/README.md: {e}"))?;

    manual_pdf(&top, &out, &version)?;

    let dots = dot_files(&top);
    if !dots.is_empty() {
        return Err(format!("CTAN refuses names starting with a dot: {}", dots.join(", ")));
    }
    sh(Command::new("zip")
        .current_dir(&out)
        .args(["-q", "-r", "-X", "rustfig.zip", "rustfig"]))?;
    println!("ctan: {} (RustFig {version}, {rev})", out.join("rustfig.zip").display());
    Ok(())
}

/// `doc/rustfig-manual.pdf` from the manual pages, through pandoc's LaTeX
/// and two LuaLaTeX runs (the table of contents) in `work`.
fn manual_pdf(top: &Path, work: &Path, version: &str) -> Result<(), String> {
    let pages = top.join("docs/manual");
    let index = fs::read_to_string(pages.join("index.md")).map_err(|e| format!("docs/manual/index.md: {e}"))?;
    let mut files = vec!["index.md".to_string()];
    for part in index.split("](").skip(1) {
        if let Some(name) = part
            .split(')')
            .next()
            .filter(|n| n.ends_with(".md") && !n.contains('/'))
        {
            if !files.iter().any(|f| f == name) && pages.join(name).is_file() {
                files.push(name.to_string());
            }
        }
    }
    let fonts: Vec<String> = std::env::var("RUSTFIG_CTAN_FONTS")
        .ok()
        .map(|v| v.split(',').map(str::to_string).collect::<Vec<_>>())
        .filter(|v| v.len() == 4)
        .unwrap_or_else(|| FONTS.iter().map(|s| s.to_string()).collect());
    let tex = work.join("rustfig-manual.tex");
    let mut pandoc = Command::new("pandoc");
    pandoc
        .current_dir(&pages)
        .args(&files)
        .args(["--file-scope", "-s", "--toc", "-o"])
        .arg(&tex)
        .args([
            "-V",
            "documentclass=article",
            "-V",
            "geometry:margin=2.2cm",
            "-V",
            "colorlinks",
        ])
        .arg("-V")
        .arg(format!("mainfont={}", fonts[0]))
        .arg("-V")
        .arg(format!("sansfont={}", fonts[1]))
        .arg("-V")
        .arg(format!("monofont={}", fonts[2]))
        .arg("-V")
        .arg(format!("mainfontfallback={}:mode=harf", fonts[3]))
        .arg("-V")
        .arg(format!("monofontfallback={}:mode=harf", fonts[3]))
        .args(["-M", "title=RustFig manual"])
        .arg("-M")
        .arg(format!("subtitle=Version {version}"));
    sh(&mut pandoc).map_err(|e| format!("{e} (pandoc is needed for the manual PDF)"))?;
    for _ in 0..2 {
        sh(Command::new("lualatex")
            .current_dir(work)
            .args(["-interaction=nonstopmode", "-halt-on-error"])
            .arg(&tex))
        .map_err(|e| format!("{e} (see {})", work.join("rustfig-manual.log").display()))?;
    }
    let doc = top.join("doc");
    fs::create_dir_all(&doc).map_err(|e| format!("{}: {e}", doc.display()))?;
    fs::copy(work.join("rustfig-manual.pdf"), doc.join("rustfig-manual.pdf"))
        .map_err(|e| format!("manual PDF: {e}"))?;
    for ext in ["tex", "aux", "log", "toc", "out", "pdf"] {
        let _ = fs::remove_file(work.join(format!("rustfig-manual.{ext}"))); // leftovers only
    }
    Ok(())
}

/// Paths under `dir` whose name starts with a dot.
fn dot_files(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if e.file_name().to_string_lossy().starts_with('.') {
                found.push(p.strip_prefix(dir).unwrap_or(&p).display().to_string());
            } else if p.is_dir() {
                stack.push(p);
            }
        }
    }
    found
}
