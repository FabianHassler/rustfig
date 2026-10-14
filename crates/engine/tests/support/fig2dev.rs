//! Running fig2dev, xfig's converter, and ghostscript as the reference for
//! the comparison tests (A8, A10). Tests only: RustFig itself never runs
//! fig2dev or ghostscript (decided 2026-10-14; fig2dev's drivers are being
//! ported instead). Both are optional; [`find`] and [`ghostscript`] return
//! `None` when they aren't installed and the tests skip. A8 uses fig2dev's
//! SVG, so it needs no ghostscript; A10 rasterises EPS and PDF with it.

#![allow(dead_code)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// fig2dev 3.2.9a's output languages (`fig2dev -h`).
pub const LANGUAGES: &[&str] = &[
    "box", "cgm", "dxf", "eepic", "eepicemu", "emf", "epic", "eps", "gbx", "ge", "gif", "ibmgl", "jpeg", "latex",
    "map", "mf", "mp", "pcx", "pdf", "pdftex", "pdftex_t", "pic", "pict2e", "pictex", "png", "ppm", "ps", "pstex",
    "pstex_t", "pstricks", "ptk", "shape", "sld", "svg", "textyl", "tiff", "tikz", "tk", "tpic", "xbm", "xpm",
];

/// How long fig2dev may run before it is killed.
pub const TIMEOUT: Duration = Duration::from_secs(60);

/// Largest output read back from fig2dev.
pub const MAX_OUTPUT: u64 = 512 * 1024 * 1024;

/// Where fig2dev was found, and its version line.
#[derive(Clone, Debug)]
pub struct Fig2dev {
    pub path: PathBuf,
    pub version: String,
}

/// Locates fig2dev once per process: `RUSTFIG_FIG2DEV`, then `PATH`, then the
/// usual install directories (GUI apps on macOS don't inherit the shell's
/// `PATH`).
pub fn find() -> Option<&'static Fig2dev> {
    static FOUND: OnceLock<Option<Fig2dev>> = OnceLock::new();
    FOUND.get_or_init(locate).as_ref()
}

fn locate() -> Option<Fig2dev> {
    let exe = if cfg!(windows) { "fig2dev.exe" } else { "fig2dev" };
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(p) = std::env::var_os("RUSTFIG_FIG2DEV") {
        candidates.push(PathBuf::from(p));
    }
    if let Some(path) = std::env::var_os("PATH") {
        candidates.extend(std::env::split_paths(&path).map(|d| d.join(exe)));
    }
    for d in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/opt/local/bin"] {
        candidates.push(Path::new(d).join(exe));
    }
    candidates.into_iter().filter(|p| p.is_file()).find_map(|path| {
        let out = Command::new(&path).arg("-V").stdin(Stdio::null()).output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let version = text.lines().next().unwrap_or_default().trim().to_string();
        version.contains("fig2dev").then_some(Fig2dev { path, version })
    })
}

/// Runs `fig2dev -L lang [args] in.fig out` on the Fig file `fig` and
/// returns the output. `cwd` is the figure's directory, so relative picture
/// paths resolve.
pub fn convert(fig: &[u8], lang: &str, args: &[String], cwd: Option<&Path>) -> Result<Vec<u8>, String> {
    if !LANGUAGES.contains(&lang) {
        return Err(format!("fig2dev has no output language {lang:?}"));
    }
    let tool = find().ok_or("fig2dev is not installed (set RUSTFIG_FIG2DEV to its path)")?;
    let dir = TempDir::new()?;
    let input = dir.0.join("in.fig");
    let output = dir.0.join("out");
    std::fs::write(&input, fig).map_err(|e| format!("cannot write a temporary file: {e}"))?;
    let mut args_all: Vec<OsString> = vec!["-L".into(), lang.into()];
    args_all.extend(args.iter().map(OsString::from));
    args_all.push(input.into_os_string());
    args_all.push(output.clone().into_os_string());
    run(&tool.path, &args_all, cwd, "fig2dev")?;
    let len = std::fs::metadata(&output)
        .map_err(|_| "fig2dev wrote no output".to_string())?
        .len();
    if len > MAX_OUTPUT {
        return Err(format!("fig2dev's output is too large ({len} bytes)"));
    }
    std::fs::read(&output).map_err(|e| format!("cannot read fig2dev's output: {e}"))
}

/// Runs `program` with `args`, killing it after [`TIMEOUT`]; its stderr
/// becomes the error message.
pub fn run(program: &Path, args: &[OsString], cwd: Option<&Path>, name: &str) -> Result<(), String> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    if let Some(cwd) = cwd.filter(|d| d.is_dir()) {
        cmd.current_dir(cwd);
    }
    let mut child = cmd.spawn().map_err(|e| format!("cannot run {name}: {e}"))?;
    // Drain stderr on a thread so a chatty program can't block on a full pipe.
    let reader = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            use std::io::Read;
            let mut text = String::new();
            // Diagnostics only; a read failure leaves the message empty.
            let _ = pipe.by_ref().take(64 * 1024).read_to_string(&mut text);
            text
        })
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() > TIMEOUT => {
                // Killing can fail only if it already exited; either way we stop waiting.
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{name} took longer than {} s and was stopped",
                    TIMEOUT.as_secs()
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => return Err(format!("{name} failed: {e}")),
        }
    };
    let stderr = reader.and_then(|r| r.join().ok()).unwrap_or_default();
    if status.success() {
        Ok(())
    } else {
        let msg = stderr.trim();
        Err(if msg.is_empty() {
            format!("{name} failed ({status})")
        } else {
            format!("{name}: {msg}")
        })
    }
}

/// Ghostscript, found once per process: `RUSTFIG_GS`, then `PATH`, then the
/// usual install directories.
pub fn ghostscript() -> Option<&'static Fig2dev> {
    static FOUND: OnceLock<Option<Fig2dev>> = OnceLock::new();
    FOUND
        .get_or_init(|| {
            let names: &[&str] = if cfg!(windows) {
                &["gswin64c.exe", "gswin32c.exe"]
            } else {
                &["gs"]
            };
            let mut candidates: Vec<PathBuf> = Vec::new();
            if let Some(p) = std::env::var_os("RUSTFIG_GS") {
                candidates.push(PathBuf::from(p));
            }
            for name in names {
                if let Some(path) = std::env::var_os("PATH") {
                    candidates.extend(std::env::split_paths(&path).map(|d| d.join(name)));
                }
                for d in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/opt/local/bin"] {
                    candidates.push(Path::new(d).join(name));
                }
            }
            candidates.into_iter().filter(|p| p.is_file()).find_map(|path| {
                let out = Command::new(&path)
                    .arg("--version")
                    .stdin(Stdio::null())
                    .output()
                    .ok()?;
                let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
                (out.status.success() && !version.is_empty()).then(|| Fig2dev {
                    path,
                    version: format!("ghostscript {version}"),
                })
            })
        })
        .as_ref()
}

/// Rasterises an EPS (cropped to its bounding box) or a PDF page to PNG at
/// `dpi`, anti-aliased, with ghostscript.
pub fn rasterise(doc: &[u8], pdf: bool, dpi: u32) -> Result<Vec<u8>, String> {
    let gs = ghostscript().ok_or("ghostscript is not installed (set RUSTFIG_GS to its path)")?;
    let dir = TempDir::new()?;
    let input = dir.0.join(if pdf { "in.pdf" } else { "in.eps" });
    let output = dir.0.join("out.png");
    std::fs::write(&input, doc).map_err(|e| format!("cannot write a temporary file: {e}"))?;
    let mut args: Vec<OsString> = [
        "-q",
        "-dSAFER",
        "-dBATCH",
        "-dNOPAUSE",
        "-sDEVICE=png16m",
        "-dTextAlphaBits=4",
        "-dGraphicsAlphaBits=4",
        "-dFirstPage=1",
        "-dLastPage=1",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    args.push(format!("-r{dpi}").into());
    if !pdf {
        args.push("-dEPSCrop".into());
    }
    let mut out = OsString::from("-sOutputFile=");
    out.push(&output);
    args.push(out);
    args.push(input.into_os_string());
    run(&gs.path, &args, None, "ghostscript")?;
    std::fs::read(&output).map_err(|e| format!("ghostscript wrote no image: {e}"))
}

/// A private temporary directory, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<TempDir, String> {
        static COUNT: AtomicU64 = AtomicU64::new(0);
        let n = COUNT.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("rustfig-fig2dev-{}-{n}-{nanos}", std::process::id()));
        std::fs::create_dir(&dir).map_err(|e| format!("cannot create a temporary directory: {e}"))?;
        Ok(TempDir(dir))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Best effort: a leftover temporary directory is harmless.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_languages() {
        let err = convert(b"", "sh -c rm", &[], None).unwrap_err();
        assert!(err.contains("no output language"));
    }

    #[test]
    fn converts_when_installed() {
        if find().is_none() {
            eprintln!("fig2dev not installed; skipping");
            return;
        }
        let out = convert(include_bytes!("../../../format/tests/sample.fig"), "svg", &[], None).unwrap();
        assert!(String::from_utf8_lossy(&out).contains("<svg"));
    }
}
