//! F1 (2b): "Install TeX Gyre fonts…". The TeX Gyre fonts are metric clones
//! of xfig's PostScript fonts (GUST Font License). Nothing is bundled: this
//! downloads CTAN's archive, checks its pinned SHA-256, unpacks only the OTF
//! files and the licence into RustFig's user data folder (next to the
//! settings; `RUSTFIG_SETTINGS_DIR` moves it) and registers that folder with
//! figbase's font lookup. It runs on a thread (like [`crate::export_bg`]);
//! a failure or a cancel leaves no partial files: everything is unpacked in a
//! temporary folder that is renamed into place at the end.
//!
//! Native only. The download goes through the system's `curl` (macOS, Windows
//! 10+ and Linux ship it), which keeps a TLS stack out of the build.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

use crate::App;

/// CTAN's TeX Gyre archive (`fonts/tex-gyre`, about 21 MB; the mirror
/// redirect serves the same file).
pub const URL: &str = "https://mirrors.ctan.org/fonts/tex-gyre.zip";
/// Its SHA-256 (CTAN file dated 2018-06, TeX Gyre 2.005 to 2.501).
/// The install refuses to run when this is not 64 hex digits.
pub const SHA256: &str = "1773c470f9e388e087b68e3426e115af2cd236845a7e05ceb25b2a503409a7a3";

/// Largest single file unpacked, and largest total (the OTFs are about 4 MB).
const MAX_FILE: u64 = 8 << 20;
const MAX_TOTAL: u64 = 64 << 20;
const MAX_DOWNLOAD: u64 = 64 << 20;

/// The folder the fonts are installed to.
pub fn fonts_dir() -> Option<PathBuf> {
    let cfg = rustfig_engine::prefs::config_path()?;
    Some(cfg.parent()?.join("fonts").join("tex-gyre"))
}

/// Tell the font lookup about the installed fonts (at startup and after an
/// install). Does nothing when they are not installed.
pub fn register_installed() {
    if let Some(d) = fonts_dir().filter(|d| d.is_dir()) {
        rustfig_text::add_font_dirs(&[d]);
    }
}

/// Whether the fonts are installed.
pub fn installed() -> bool {
    fonts_dir().is_some_and(|d| d.is_dir())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 of a file, as lower-case hex.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut f = fs::File::open(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f
            .read(&mut buf)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        h.update(buf.get(..n).unwrap_or_default());
    }
    Ok(hex(&h.finalize()))
}

fn temp_sibling(dest: &Path, tag: &str) -> PathBuf {
    let name = dest.file_name().and_then(|n| n.to_str()).unwrap_or("fonts");
    dest.with_file_name(format!(".{name}.{tag}-{}", std::process::id()))
}

/// Verify `zip_path` against `expected` (SHA-256 hex) and unpack its OTF
/// files and the GUST licence flat into `dest`. Returns the number of fonts.
/// On any error (or `cancel`) `dest` is left as it was.
pub fn install_from_zip(zip_path: &Path, expected: &str, dest: &Path, cancel: &AtomicBool) -> Result<usize, String> {
    let expected = expected.trim().to_ascii_lowercase();
    if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("the TeX Gyre checksum is not set in this build; not installing".into());
    }
    let got = sha256_file(zip_path)?;
    if got != expected {
        return Err(format!(
            "the downloaded archive failed its checksum test (got {got}); nothing was installed"
        ));
    }
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }
    let tmp = temp_sibling(dest, "tmp");
    let _ = fs::remove_dir_all(&tmp); // a leftover of a crashed run
    let result = unpack(zip_path, &tmp, cancel).and_then(|n| {
        if n == 0 {
            return Err("the archive holds no TeX Gyre fonts".to_string());
        }
        if dest.exists() {
            fs::remove_dir_all(dest).map_err(|e| format!("cannot replace {}: {e}", dest.display()))?;
        }
        fs::rename(&tmp, dest).map_err(|e| format!("cannot move the fonts to {}: {e}", dest.display()))?;
        Ok(n)
    });
    if result.is_err() {
        let _ = fs::remove_dir_all(&tmp); // best effort: nothing else to do on failure
    }
    result
}

fn unpack(zip_path: &Path, tmp: &Path, cancel: &AtomicBool) -> Result<usize, String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("cannot open the archive: {e}"))?;
    let mut ar = zip::ZipArchive::new(file).map_err(|e| format!("not a zip archive: {e}"))?;
    fs::create_dir_all(tmp).map_err(|e| format!("cannot create {}: {e}", tmp.display()))?;
    let mut fonts = 0usize;
    let mut total = 0u64;
    for i in 0..ar.len() {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        let mut entry = ar.by_index(i).map_err(|e| format!("bad archive entry: {e}"))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        let Some(base) = name.rsplit('/').next().filter(|b| !b.is_empty()) else {
            continue;
        };
        let lower = base.to_ascii_lowercase();
        let in_opentype = name.split('/').any(|c| c == "opentype");
        let is_font = in_opentype && lower.ends_with(".otf") && lower.starts_with("texgyre");
        let is_licence = lower == "gust-font-license.txt";
        if !(is_font || is_licence) || base.contains("..") {
            continue;
        }
        if entry.size() > MAX_FILE {
            return Err(format!("{base} is unexpectedly large"));
        }
        total += entry.size();
        if total > MAX_TOTAL {
            return Err("the fonts are unexpectedly large".into());
        }
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry
            .by_ref()
            .take(MAX_FILE + 1)
            .read_to_end(&mut data)
            .map_err(|e| format!("cannot unpack {base}: {e}"))?;
        if data.len() as u64 > MAX_FILE {
            return Err(format!("{base} is unexpectedly large"));
        }
        let mut out = fs::File::create(tmp.join(base)).map_err(|e| format!("cannot write {base}: {e}"))?;
        out.write_all(&data).map_err(|e| format!("cannot write {base}: {e}"))?;
        if is_font {
            fonts += 1;
        }
    }
    Ok(fonts)
}

/// Download `url` to `to` with curl; polls `cancel` and kills curl on it.
fn download(url: &str, to: &Path, cancel: &AtomicBool) -> Result<(), String> {
    let mut child = Command::new("curl")
        .args(["--fail", "--silent", "--show-error", "--location", "--max-time", "900"])
        .args(["--max-filesize", &MAX_DOWNLOAD.to_string(), "--output"])
        .arg(to)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run curl (needed for the download): {e}"))?;
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill(); // we are cancelling: the exit status no longer matters
            let _ = child.wait();
            return Err("cancelled".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                }
                let mut msg = String::new();
                if let Some(mut e) = child.stderr.take() {
                    let _ = e.read_to_string(&mut msg); // only for the message
                }
                let msg = msg.trim();
                return Err(format!(
                    "the download failed: {}",
                    if msg.is_empty() { "curl error" } else { msg }
                ));
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(e) => return Err(format!("the download failed: {e}")),
        }
    }
}

/// Download, verify and install; the whole job for the thread.
pub fn install(url: &str, expected: &str, dest: &Path, cancel: &AtomicBool) -> Result<usize, String> {
    if expected.trim().len() != 64 {
        return Err("the TeX Gyre checksum is not set in this build; not installing".into());
    }
    let parent = dest.parent().ok_or("no folder to install into")?;
    fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    let part = temp_sibling(dest, "download");
    let r = download(url, &part, cancel).and_then(|()| install_from_zip(&part, expected, dest, cancel));
    let _ = fs::remove_file(&part); // the archive is not kept
    r
}

/// An install on its thread.
pub struct Running {
    rx: Receiver<Result<usize, String>>,
    cancel: Arc<AtomicBool>,
    started: std::time::Instant,
}

/// Start the install (to the user data folder) on a thread.
pub fn start(app: &mut App) -> Result<(), String> {
    if app.tex_gyre.is_some() {
        return Err("The TeX Gyre fonts are already being installed".into());
    }
    let dest = fonts_dir().ok_or("no user data folder to install into")?;
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    let (tx, rx) = channel();
    std::thread::Builder::new()
        .name("tex-gyre".into())
        .spawn(move || {
            let r = match rustfig_engine::guard::catch(|| install(URL, SHA256, &dest, &flag)) {
                Ok(r) => r,
                Err(m) => Err(format!("internal error (please report): {m}")),
            };
            let _ = tx.send(r); // the UI may be gone
        })
        .map_err(|e| format!("cannot start the install: {e}"))?;
    app.tex_gyre = Some(Running {
        rx,
        cancel,
        started: std::time::Instant::now(),
    });
    Ok(())
}

/// Collect a finished install and show the progress window.
pub fn tick(app: &mut App, ctx: &egui::Context) {
    let Some(r) = &app.tex_gyre else { return };
    match r.rx.try_recv() {
        Ok(result) => {
            app.tex_gyre = None;
            match result {
                Ok(n) => {
                    register_installed();
                    app.canvas.invalidate();
                    app.info(format!("Installed {n} TeX Gyre fonts"));
                }
                Err(m) => app.error(format!("TeX Gyre fonts: {m}")),
            }
        }
        Err(TryRecvError::Disconnected) => {
            app.tex_gyre = None;
            app.error("The TeX Gyre install stopped unexpectedly");
        }
        Err(TryRecvError::Empty) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            let secs = r.started.elapsed().as_secs();
            let mut cancel_clicked = false;
            egui::Window::new("Installing TeX Gyre fonts")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(format!("Downloading from CTAN ({secs} s)"));
                    });
                    if ui.button("Cancel").clicked() {
                        cancel_clicked = true;
                    }
                });
            if cancel_clicked {
                if let Some(r) = app.tex_gyre.take() {
                    r.cancel.store(true, Ordering::Relaxed);
                }
                app.info("Install cancelled");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_zip(path: &Path) {
        let f = fs::File::create(path).unwrap();
        let mut z = zip::ZipWriter::new(f);
        let o = zip::write::SimpleFileOptions::default();
        for (name, data) in [
            ("tex-gyre/opentype/texgyretermes-regular.otf", &b"OTTO-termes"[..]),
            ("tex-gyre/opentype/texgyreheros-bold.otf", b"OTTO-heros"),
            ("tex-gyre/doc/GUST-FONT-LICENSE.txt", b"licence"),
            ("tex-gyre/doc/README-TeX-Gyre-Termes.txt", b"readme"),
            ("tex-gyre/tex/other.otf", b"not opentype dir"),
        ] {
            z.start_file(name, o).unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap();
    }

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rustfig-tg-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn installs_only_otf_and_licence_after_checksum() {
        let d = scratch("ok");
        let z = d.join("a.zip");
        make_zip(&z);
        let sum = sha256_file(&z).unwrap();
        let dest = d.join("fonts").join("tex-gyre");
        let n = install_from_zip(&z, &sum, &dest, &AtomicBool::new(false)).unwrap();
        assert_eq!(n, 2);
        let mut names: Vec<_> = fs::read_dir(&dest)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "GUST-FONT-LICENSE.txt",
                "texgyreheros-bold.otf",
                "texgyretermes-regular.otf"
            ]
        );
        // no temporary folder is left next to it
        let siblings = fs::read_dir(d.join("fonts")).unwrap().count();
        assert_eq!(siblings, 1);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn bad_checksum_is_refused_and_leaves_nothing() {
        let d = scratch("bad");
        let z = d.join("a.zip");
        make_zip(&z);
        let dest = d.join("fonts").join("tex-gyre");
        let bad = "0".repeat(64);
        let e = install_from_zip(&z, &bad, &dest, &AtomicBool::new(false)).unwrap_err();
        assert!(e.contains("checksum"), "{e}");
        assert!(!dest.exists());
        assert!(!d.join("fonts").exists() || fs::read_dir(d.join("fonts")).unwrap().count() == 0);
        let e = install_from_zip(&z, "TODO", &dest, &AtomicBool::new(false)).unwrap_err();
        assert!(e.contains("not set"), "{e}");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn failed_install_keeps_the_previous_one() {
        let d = scratch("keep");
        let z = d.join("a.zip");
        make_zip(&z);
        let dest = d.join("fonts").join("tex-gyre");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("old.otf"), b"old").unwrap();
        let _ = install_from_zip(&z, &"1".repeat(64), &dest, &AtomicBool::new(false)).unwrap_err();
        assert!(dest.join("old.otf").exists());
        // cancelled after a good checksum
        let sum = sha256_file(&z).unwrap();
        let _ = install_from_zip(&z, &sum, &dest, &AtomicBool::new(true)).unwrap_err();
        assert!(dest.join("old.otf").exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn archive_without_fonts_is_an_error() {
        let d = scratch("empty");
        let z = d.join("a.zip");
        let f = fs::File::create(&z).unwrap();
        let mut w = zip::ZipWriter::new(f);
        w.start_file("readme.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(b"x").unwrap();
        w.finish().unwrap();
        let sum = sha256_file(&z).unwrap();
        let dest = d.join("tg");
        assert!(install_from_zip(&z, &sum, &dest, &AtomicBool::new(false)).is_err());
        assert!(!dest.exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn pinned_checksum_is_well_formed() {
        assert_eq!(SHA256.len(), 64);
        assert!(SHA256.bytes().all(|b| b.is_ascii_hexdigit()));
    }
}
