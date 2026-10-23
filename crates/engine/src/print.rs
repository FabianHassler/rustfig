//! Printing (plan B5, B5b): xfig's Print panel, modernised.
//!
//! xfig pipes fig2dev's PostScript to `lpr` (`print_to_printer`, `print_to_batchfile`
//! and `do_print` of `u_print.c` and `w_print.c`). RustFig makes a PDF of the
//! same page, with the same fig2dev options, and shows the operating system's
//! own print dialog for it (`rustfig-os`: macOS print panel, Windows print
//! dialog, the desktop portal's on Linux, the browser's viewer on the web):
//! printer, copies and page ranges are chosen there, not in RustFig.
//! Magnification, paper size, orientation, justification and single/multiple
//! pages are the figure's own settings (the .fig file, as in xfig); everything
//! else is kept in the settings file ([`PrintSettings`]).
//!
//! What reaches the printer goes through [`Spooler`], so tests and scripts
//! can look at a job instead of sending it; the app installs the one that
//! calls the system dialog.
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

/// What the printer is sent.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintJob {
    /// The PDF.
    pub pdf: Vec<u8>,
    /// A name for the job and the temporary file.
    pub name: String,
}

/// Where print jobs go. The system's one is [`SystemSpooler`]; tests use
/// [`RecordingSpooler`]; an engine without a front end has [`NoSpooler`].
pub trait Spooler: Send {
    /// Hand `job` over: `Some(message)` when it went to the print system,
    /// `None` when the user cancelled the dialog.
    fn submit(&mut self, job: &PrintJob) -> Result<Option<String>, String>;
}

/// Refuses every job: the engine's default, so scripts and tests never print.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoSpooler;

impl Spooler for NoSpooler {
    fn submit(&mut self, _: &PrintJob) -> Result<Option<String>, String> {
        Err("no print system is attached".into())
    }
}

/// Keeps the jobs it is given instead of printing them.
#[derive(Debug, Default, Clone)]
pub struct RecordingSpooler {
    pub jobs: Arc<Mutex<Vec<PrintJob>>>,
    /// When set, every job fails with this message.
    pub fail: Option<String>,
    /// When set, the "user" cancels the dialog: the job is not recorded.
    pub cancel: bool,
}

impl Spooler for RecordingSpooler {
    fn submit(&mut self, job: &PrintJob) -> Result<Option<String>, String> {
        if let Some(f) = &self.fail {
            return Err(f.clone());
        }
        if self.cancel {
            return Ok(None);
        }
        if let Ok(mut jobs) = self.jobs.lock() {
            jobs.push(job.clone());
        }
        Ok(Some("recorded".into()))
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

/// The PDFs one after the other as one multi-page PDF (the batch as one job).
/// A single PDF is returned as it is.
pub fn merge_pdfs(pdfs: &[&[u8]]) -> Result<Vec<u8>, String> {
    use hayro_write::pdf_writer::{Pdf as Writer, Ref};
    use hayro_write::{extract, ChunkSettings, ExtractionQuery};
    use std::cell::Cell;
    use std::rc::Rc;
    if let [one] = pdfs {
        return Ok(one.to_vec());
    }
    if pdfs.is_empty() {
        return Err("nothing to merge".into());
    }
    let counter = Rc::new(Cell::new(1i32));
    let bump = |c: &Cell<i32>| {
        let v = c.get();
        c.set(v.saturating_add(1));
        Ref::new(v)
    };
    let catalog = bump(&counter);
    let root = bump(&counter);
    let mut groups: Vec<(Ref, Vec<Ref>)> = Vec::new();
    let mut chunks = Vec::new();
    for (i, bytes) in pdfs.iter().enumerate() {
        let doc = hayro_syntax::Pdf::new(Arc::new(bytes.to_vec()))
            .map_err(|_| format!("figure {} is not a readable PDF", i + 1))?;
        let queries: Vec<_> = (0..doc.pages().len()).map(ExtractionQuery::new_page).collect();
        let c = counter.clone();
        let res = guarded(|| {
            extract(
                &doc,
                Box::new(move || {
                    let v = c.get();
                    c.set(v.saturating_add(1));
                    Ref::new(v)
                }),
                ChunkSettings::default(),
                &queries,
            )
            .ok()
        })
        .ok_or_else(|| format!("figure {} could not be copied", i + 1))?;
        let pages: Vec<Ref> = res.root_refs.iter().filter_map(|r| r.as_ref().ok().copied()).collect();
        groups.push((res.page_tree_parent_ref, pages));
        chunks.push(res.chunk);
    }
    let mut out = Writer::new();
    out.catalog(catalog).pages(root);
    let total: usize = groups.iter().map(|g| g.1.len()).sum();
    out.pages(root)
        .kids(groups.iter().map(|g| g.0))
        .count(i32::try_from(total).unwrap_or(i32::MAX));
    for (node, pages) in &groups {
        let mut t = out.pages(*node);
        t.parent(root);
        t.kids(pages.iter().copied())
            .count(i32::try_from(pages.len()).unwrap_or(i32::MAX));
    }
    for c in &chunks {
        out.extend(c);
    }
    Ok(out.finish())
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

/// The job for `pdf`.
pub fn job_for(name: String, pdf: Vec<u8>) -> PrintJob {
    PrintJob { pdf, name }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_rejects_unknown_keys_and_bad_values() {
        let d = PrintSettings::default();
        assert!(merge(d.clone(), &json!({"nope": 1})).is_err());
        assert!(merge(d.clone(), &json!({"overlap": "yes"})).is_err());
        assert!(merge(d.clone(), &json!({"printer": "hp"})).is_err());
        let m = merge(d, &json!({"overlap": true, "grid_minor": "1/4"})).unwrap();
        assert!(m.overlap && m.grid_minor == "1/4");
    }
}
