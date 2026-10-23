//! Printing (plan B5): xfig's Print panel, modernised.
//!
//! xfig pipes fig2dev's PostScript to `lpr` (`print_to_printer`, `print_to_batchfile`
//! and `do_print` of `u_print.c` and `w_print.c`). RustFig makes a PDF of the
//! same page, with the same fig2dev options, and hands it to the operating
//! system's print system: the viewer's print dialog (macOS Preview, the
//! desktop's PDF viewer, Windows) for printer, copies and page ranges, or
//! straight to `lp` (macOS, Linux) when a printer is named. Magnification,
//! paper size, orientation, justification and single/multiple pages are the
//! figure's own settings (the .fig file, as in xfig); everything else is kept
//! in the settings file ([`PrintSettings`]).
//!
//! What reaches the printer goes through [`Spooler`], so tests and scripts
//! can look at a job instead of sending it.
//!
// Ported from xfig src/u_print.c, src/w_print.c (xfig licence, see LICENSE-xfig).

use crate::export_panel::{self, PanelSettings, COLOR_NONE};
use crate::{Engine, EngineError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

/// Most figures in the batch, and most bytes all of them may hold.
pub const MAX_BATCH_FIGURES: usize = 200;
pub const MAX_BATCH_BYTES: usize = 512 << 20;
/// Longest printer name, job parameter string, and number of parameters.
const MAX_NAME: usize = 256;
const MAX_PARAMS: usize = 1024;
const MAX_ARGS: usize = 32;
/// Most pixels along the longer side of a page preview.
pub const MAX_PREVIEW_EDGE: u32 = 2048;

/// The values of the Print panel that aren't the figure's own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrintSettings {
    /// Multiple pages overlap ("Overlap" / "No Overlap").
    pub overlap: bool,
    /// Background colour number, `COLOR_NONE` for none.
    pub background: i32,
    /// Minor and major grid spacing as the menus give them ("None", "1/4", "10" …).
    pub grid_minor: String,
    pub grid_major: String,
    /// "Print all layers" (true) or "Print only active" (false).
    pub all_layers: bool,
    /// With only the active layers: size the page by them too (fig2dev's `-K`).
    pub bound_active_layers: bool,
    /// The printer's name; empty is the system's default printer.
    pub printer: String,
    /// "Print Job Params": extra options for `lp`, `%f` is the file name.
    pub params: String,
    /// Hand the PDF to the system's print dialog (true, the default) or send it
    /// to the printer at once with `lp` (macOS, Linux) or the Print verb (Windows).
    pub system_dialog: bool,
}

impl Default for PrintSettings {
    fn default() -> Self {
        PrintSettings {
            overlap: false,
            background: COLOR_NONE,
            grid_minor: "None".into(),
            grid_major: "None".into(),
            all_layers: true,
            bound_active_layers: false,
            printer: String::new(),
            params: String::new(),
            system_dialog: true,
        }
    }
}

impl PrintSettings {
    /// Clamp what a file or a command gave to what the panel can show.
    pub fn sanitized(mut self) -> PrintSettings {
        let grids = PanelSettings {
            grid_minor: std::mem::take(&mut self.grid_minor),
            grid_major: std::mem::take(&mut self.grid_major),
            background: self.background,
            ..PanelSettings::xfig()
        }
        .sanitized();
        self.grid_minor = grids.grid_minor;
        self.grid_major = grids.grid_major;
        self.background = grids.background;
        if check_printer(&self.printer).is_err() {
            self.printer.clear();
        }
        if check_params(&self.params).is_err() {
            self.params.clear();
        }
        self
    }
}

/// `base` with the fields present in `v` replaced; unknown keys and wrong
/// types are errors.
pub fn merge(base: PrintSettings, v: &Value) -> Result<PrintSettings, EngineError> {
    let Value::Object(m) = v else {
        return Err(EngineError::bad_params("settings must be an object"));
    };
    let mut cur = serde_json::to_value(&base).map_err(|err| EngineError::bad_params(err.to_string()))?;
    if let Some(cm) = cur.as_object_mut() {
        for (k, val) in m {
            if !cm.contains_key(k) {
                return Err(EngineError::bad_params(format!("unknown print setting {k:?}")));
            }
            cm.insert(k.clone(), val.clone());
        }
    }
    let s: PrintSettings =
        serde_json::from_value(cur).map_err(|err| EngineError::bad_params(format!("print settings: {err}")))?;
    check_printer(&s.printer)?;
    check_params(&s.params)?;
    Ok(s.sanitized())
}

/// The settings kept across sessions.
pub fn settings_of(e: &Engine) -> PrintSettings {
    e.prefs.rustfig.print.clone().sanitized()
}

/// Keep `s`; the settings file is written when the preferences are next saved.
pub fn store(e: &mut Engine, s: PrintSettings) {
    let s = s.sanitized();
    if e.prefs.rustfig.print != s {
        e.prefs.rustfig.print = s;
        e.prefs_dirty = true;
    }
}

/// A printer name the system's print command can take: it can't look like an
/// option and has no control characters.
pub fn check_printer(name: &str) -> Result<(), EngineError> {
    if name.len() > MAX_NAME || name.starts_with('-') || name.chars().any(char::is_control) {
        return Err(EngineError::bad_params("the printer name is not valid"));
    }
    Ok(())
}

/// Job parameters: control characters and absurd lengths are refused.
pub fn check_params(params: &str) -> Result<(), EngineError> {
    if params.len() > MAX_PARAMS || params.chars().any(char::is_control) {
        return Err(EngineError::bad_params("the print job parameters are not valid"));
    }
    if split_args(params).len() > MAX_ARGS {
        return Err(EngineError::bad_params("too many print job parameters"));
    }
    Ok(())
}

/// Break `cmdline` at spaces; a space quoted by a backslash stays in its
/// word (xfig's `cmdlinetoargarray`).
pub fn split_args(cmdline: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut started = false;
    let mut chars = cmdline.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&' ') => {
                cur.push(' ');
                chars.next();
                started = true;
            }
            ' ' => {
                if started {
                    out.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            c => {
                cur.push(c);
                started = true;
            }
        }
    }
    if started {
        out.push(cur);
    }
    out
}

/// What the printer is sent.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintJob {
    /// The PDF.
    pub pdf: Vec<u8>,
    /// A name for the job and the temporary file.
    pub name: String,
    pub printer: String,
    /// `lp`'s options, already split (`%f` replaced).
    pub args: Vec<String>,
    pub system_dialog: bool,
}

/// Where print jobs go. The system's one is [`SystemSpooler`]; tests use
/// [`RecordingSpooler`]; an engine without a front end has [`NoSpooler`].
pub trait Spooler: Send {
    /// Hand `job` over; a short message for the user.
    fn submit(&mut self, job: &PrintJob) -> Result<String, String>;
}

/// Refuses every job: the engine's default, so scripts and tests never print.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoSpooler;

impl Spooler for NoSpooler {
    fn submit(&mut self, _: &PrintJob) -> Result<String, String> {
        Err("no print system is attached".into())
    }
}

/// Keeps the jobs it is given instead of printing them.
#[derive(Debug, Default, Clone)]
pub struct RecordingSpooler {
    pub jobs: Arc<Mutex<Vec<PrintJob>>>,
    /// When set, every job fails with this message.
    pub fail: Option<String>,
}

impl Spooler for RecordingSpooler {
    fn submit(&mut self, job: &PrintJob) -> Result<String, String> {
        if let Some(f) = &self.fail {
            return Err(f.clone());
        }
        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.push(job.clone());
        }
        Ok("recorded".into())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use system::SystemSpooler;

#[cfg(not(target_arch = "wasm32"))]
mod system {
    use super::{PrintJob, Spooler};
    use std::path::PathBuf;
    use std::process::Command;

    /// The operating system's print system: the PDF goes to a temporary
    /// file, then to the viewer's print dialog (`open -a Preview` on macOS,
    /// `xdg-open` on Linux, the shell on Windows) or, without a dialog, to
    /// `lp` (macOS, Linux) or the Print verb (Windows).
    #[derive(Debug, Default, Clone, Copy)]
    pub struct SystemSpooler;

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join("rustfig-print")
    }

    /// Write the job's PDF; files older than a day are removed first.
    fn write_pdf(job: &PrintJob) -> Result<PathBuf, String> {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        if let Ok(rd) = std::fs::read_dir(&dir) {
            let day = std::time::Duration::from_secs(24 * 3600);
            for f in rd.flatten() {
                let old = f
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.elapsed().ok())
                    .is_some_and(|age| age > day);
                if old {
                    // A leftover of an earlier print; failing to remove it is harmless.
                    let _ = std::fs::remove_file(f.path());
                }
            }
        }
        let name: String = job
            .name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                    c
                } else {
                    '_'
                }
            })
            .take(80)
            .collect();
        let name = name.trim_start_matches('.');
        let name = if name.is_empty() { "figure" } else { name };
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let path = dir.join(format!("{stamp}-{}-{name}.pdf", std::process::id()));
        std::fs::write(&path, &job.pdf).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path)
    }

    fn run(mut c: Command) -> Result<String, String> {
        let program = c.get_program().to_string_lossy().into_owned();
        let out = c.output().map_err(|e| format!("cannot run {program}: {e}"))?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if out.status.success() {
            Ok(text)
        } else {
            let err = String::from_utf8_lossy(&out.stderr);
            let err = err.trim();
            Err(format!(
                "{program} failed: {}",
                if err.is_empty() { &text } else { err }
            ))
        }
    }

    impl Spooler for SystemSpooler {
        fn submit(&mut self, job: &PrintJob) -> Result<String, String> {
            let path = write_pdf(job)?;
            let file = path.to_string_lossy().into_owned();
            if job.system_dialog {
                #[cfg(target_os = "macos")]
                {
                    let mut c = Command::new("open");
                    c.args(["-a", "Preview", &file]);
                    run(c)?;
                    return Ok("Opened in Preview: choose File > Print there".to_string());
                }
                #[cfg(target_os = "windows")]
                {
                    let mut c = Command::new("cmd");
                    c.args(["/C", "start", "", &file]);
                    run(c)?;
                    return Ok("Opened in the PDF viewer: print it from there".to_string());
                }
                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                {
                    let mut c = Command::new("xdg-open");
                    c.arg(&file);
                    run(c)?;
                    return Ok("Opened in the PDF viewer: print it from there".to_string());
                }
            }
            #[cfg(target_os = "windows")]
            {
                let mut script = format!("Start-Process -FilePath '{}' ", file.replace('\'', "''"));
                if job.printer.is_empty() {
                    script.push_str("-Verb Print");
                } else {
                    script.push_str(&format!(
                        "-Verb PrintTo -ArgumentList '\"{}\"'",
                        job.printer.replace('\'', "''")
                    ));
                }
                let mut c = Command::new("powershell");
                c.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
                run(c)?;
                Ok("Sent to the printer".into())
            }
            #[cfg(not(target_os = "windows"))]
            {
                let mut c = Command::new("lp");
                if !job.printer.is_empty() {
                    c.args(["-d", &job.printer]);
                }
                c.args(&job.args);
                c.arg("--");
                c.arg(&file);
                let id = run(c)?;
                Ok(if id.is_empty() {
                    "Sent to the printer".into()
                } else {
                    id
                })
            }
        }
    }
}

/// The fig2dev PDF of the figure as the Print panel has it: a full page of
/// the figure's paper size (fig2dev's `-P`), the figure's magnification,
/// orientation, justification and pages, and the panel's background, grid and
/// layers (`addargs_postscript` and `start_argumentlist` in `u_print.c`).
/// The PDF and fig2dev's messages.
pub fn make_pdf(e: &Engine, s: &PrintSettings) -> Result<(Vec<u8>, Vec<String>), EngineError> {
    let doc = e.whole_doc();
    if doc.objects.is_empty() {
        return Err(EngineError::bad_params("The figure is empty"));
    }
    let panel = PanelSettings {
        pdf_page_mode: true,
        overlap: s.overlap,
        background: s.background,
        grid_minor: s.grid_minor.clone(),
        grid_major: s.grid_major.clone(),
        ..PanelSettings::xfig()
    };
    let mut p = json!({
        "format": "pdf",
        "magnification": doc.settings.magnification,
        "active_layers_only": !s.all_layers && !e.hidden_depths.is_empty(),
        "whole_figure_bounds": !s.bound_active_layers,
    });
    for (k, v) in export_panel::params(e, "pdf", &panel) {
        p[k] = v;
    }
    crate::cmd::file::export_bytes_msgs(e, &doc, "pdf", &p)
}

/// One page of a PDF as pixels on white paper.
#[derive(Debug, Clone, PartialEq)]
pub struct PageImage {
    pub width: u32,
    pub height: u32,
    /// Opaque RGBA.
    pub rgba: Vec<u8>,
}

/// The number of pages of `pdf` and the first one's size in points.
pub fn pdf_info(pdf: &[u8]) -> Option<(usize, f64, f64)> {
    guarded(|| {
        let doc = hayro::hayro_syntax::Pdf::new(Arc::new(pdf.to_vec())).ok()?;
        let pages = doc.pages();
        let (w, h) = pages.first()?.render_dimensions();
        Some((pages.len(), f64::from(w), f64::from(h)))
    })
}

/// Page `index` of `pdf` with its longer side `max_edge` pixels (at most
/// [`MAX_PREVIEW_EDGE`]), composed on white.
pub fn render_page(pdf: &[u8], index: usize, max_edge: u32) -> Result<PageImage, String> {
    let max_edge = max_edge.clamp(16, MAX_PREVIEW_EDGE);
    guarded(|| {
        let doc = hayro::hayro_syntax::Pdf::new(Arc::new(pdf.to_vec())).ok()?;
        let pages = doc.pages();
        let page = pages.get(index)?;
        let (pw, ph) = page.render_dimensions();
        if !(pw.is_finite() && ph.is_finite() && pw >= 1.0 && ph >= 1.0) {
            return None;
        }
        let k = max_edge as f32 / pw.max(ph);
        let settings = hayro::PixmapSettings {
            x_scale: k,
            y_scale: k,
            ..hayro::PixmapSettings::default()
        };
        let cache = hayro::RenderCache::new();
        let pm = hayro::render(
            page,
            &cache,
            &hayro::hayro_interpret::InterpreterSettings::default(),
            &hayro::RenderSettings::default(),
            &settings,
        );
        let (w, h) = (u32::from(pm.width()), u32::from(pm.height()));
        let mut out = pm.take_rgba8(hayro::vello_cpu::peniko::ImageAlphaType::Alpha);
        if w == 0 || h == 0 || out.len() != (w as usize) * (h as usize) * 4 {
            return None;
        }
        // Straight alpha over white paper.
        for px in out.as_chunks_mut::<4>().0 {
            let a = u32::from(px[3]);
            for c in px.iter_mut().take(3) {
                *c = ((u32::from(*c) * a + 255 * (255 - a) + 127) / 255) as u8;
            }
            px[3] = 255;
        }
        Some(PageImage {
            width: w,
            height: h,
            rgba: out,
        })
    })
    .ok_or_else(|| "the page cannot be shown".to_string())
}

/// `f`, with a panic in the PDF reader or renderer turned into `None`.
fn guarded<T>(f: impl FnOnce() -> Option<T>) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok().flatten()
}

/// One figure waiting in the batch.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchItem {
    pub name: String,
    pub pdf: Vec<u8>,
}

/// A name for jobs and files: the figure's file name without folder and
/// extension.
pub fn job_name(e: &Engine) -> String {
    e.path()
        .and_then(|p| std::path::Path::new(p).file_stem())
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("figure")
        .to_string()
}

/// "A4 paper size in PORTRAIT mode", xfig's message.
pub fn mode_text(e: &Engine) -> String {
    let s = &e.doc().settings;
    format!(
        "{} paper size in {} mode",
        s.paper.name(),
        if s.orientation == rustfig_model::Orientation::Landscape {
            "LANDSCAPE"
        } else {
            "PORTRAIT"
        }
    )
}

/// The job for `pdf`: `%f` in the parameters becomes the figure's file name.
pub fn job_for(e: &Engine, s: &PrintSettings, name: String, pdf: Vec<u8>) -> PrintJob {
    let file = e.path().unwrap_or("");
    let args = split_args(&s.params)
        .into_iter()
        .map(|a| a.replace("%f", file))
        .collect();
    PrintJob {
        pdf,
        name,
        printer: s.printer.clone(),
        args,
        system_dialog: s.system_dialog,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_split_like_xfig() {
        assert_eq!(
            split_args("  -o  sides=two-sided  -n 2 "),
            ["-o", "sides=two-sided", "-n", "2"]
        );
        assert_eq!(split_args(r"-t my\ title x"), ["-t", "my title", "x"]);
        assert!(split_args("   ").is_empty());
    }

    #[test]
    fn names_are_checked() {
        assert!(check_printer("hp1700").is_ok());
        assert!(check_printer("-d").is_err());
        assert!(check_printer("a\nb").is_err());
        assert!(check_params("-n 2").is_ok());
        assert!(check_params("a\0b").is_err());
        assert!(check_params(&"x ".repeat(100)).is_err());
    }

    #[test]
    fn merge_rejects_unknown_keys_and_bad_values() {
        let d = PrintSettings::default();
        assert!(merge(d.clone(), &json!({"nope": 1})).is_err());
        assert!(merge(d.clone(), &json!({"overlap": "yes"})).is_err());
        assert!(merge(d.clone(), &json!({"printer": "-x"})).is_err());
        let m = merge(d, &json!({"overlap": true, "grid_minor": "1/4"})).unwrap();
        assert!(m.overlap && m.grid_minor == "1/4");
    }
}
