//! WYSIWYG LaTeX text (plan F8): special texts typeset by the installed TeX.
//!
//! The canvas asks [`LatexPreview::typesetter`] for a [`Typesetter`]; it
//! answers `None` for a text until a background job has run
//! `pdflatex`/`lualatex` on it and kept the PDF as a vector picture (the rasteriser renders it at the size shown, so it is sharp at every zoom), so the raw
//! source shows meanwhile (and when TeX is missing, off, or failed). Results
//! are cached per text, font, size and colour. Nothing here touches the
//! drawing or the exports. Native only: on wasm no job ever starts.

use rustfig_render::{Typeset, TypesetRequest, Typesetter};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

/// A TeX run is killed after this many seconds.
pub const MAX_TEX_SECONDS: u64 = 20;
/// Longest text sent to TeX, in bytes.
pub const MAX_TEXT_BYTES: usize = 4000;
/// Longest extra preamble, in bytes.
pub const MAX_PREAMBLE_BYTES: usize = 8000;
/// Typeset pictures kept (oldest dropped first).
pub const MAX_CACHE: usize = 512;
/// Jobs waiting or running at once; further texts wait for the next frame.
pub const MAX_PENDING: usize = 64;
/// Largest PDF read back, in bytes.
pub const MAX_PDF_BYTES: u64 = 8 << 20;

/// The preferences that matter here.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewSettings {
    pub enabled: bool,
    /// Program name or path; empty looks for `pdflatex`, then `lualatex`.
    pub program: String,
    /// Added to the preamble (packages the figure's texts need).
    pub preamble: String,
}

impl Default for PreviewSettings {
    fn default() -> Self {
        PreviewSettings {
            enabled: true,
            program: String::new(),
            preamble: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    text: String,
    font: i32,
    postscript: bool,
    /// Size in hundredths of a point.
    size: i64,
    rgb: [u8; 3],
}

#[derive(Debug, Clone)]
enum Entry {
    Pending,
    Ready(Typeset),
    Failed,
}

#[derive(Debug, Default)]
struct Cache {
    map: HashMap<Key, Entry>,
    order: VecDeque<Key>,
    pending: usize,
    /// Errors not yet shown in the message panel.
    messages: Vec<String>,
}

#[derive(Debug)]
struct Inner {
    settings: Mutex<PreviewSettings>,
    /// The TeX program found for the settings.
    program: Mutex<Option<PathBuf>>,
    cache: Mutex<Cache>,
    /// Counts finished jobs (the canvas redraws when it changes).
    generation: AtomicU64,
    /// One TeX at a time.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    gate: Mutex<()>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    // A poisoned lock only means a job thread died; the data is still usable.
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The shared state of the preview: settings, cache, background jobs.
#[derive(Debug, Clone)]
pub struct LatexPreview {
    inner: Arc<Inner>,
}

impl Default for LatexPreview {
    fn default() -> Self {
        LatexPreview::new()
    }
}

impl LatexPreview {
    pub fn new() -> LatexPreview {
        let p = LatexPreview {
            inner: Arc::new(Inner {
                settings: Mutex::new(PreviewSettings::default()),
                program: Mutex::new(None),
                cache: Mutex::new(Cache::default()),
                generation: AtomicU64::new(0),
                gate: Mutex::new(()),
            }),
        };
        *lock(&p.inner.program) = find_tex("");
        p
    }

    /// Change the settings; a change drops the cache.
    pub fn configure(&self, mut s: PreviewSettings) {
        s.preamble.truncate(floor_boundary(&s.preamble, MAX_PREAMBLE_BYTES));
        let mut cur = lock(&self.inner.settings);
        if *cur == s {
            return;
        }
        let program_changed = cur.program != s.program;
        *cur = s.clone();
        drop(cur);
        if program_changed || lock(&self.inner.program).is_none() {
            *lock(&self.inner.program) = find_tex(&s.program);
        }
        {
            let mut c = lock(&self.inner.cache);
            // Running jobs find their entry gone and drop their result.
            c.map.clear();
            c.order.clear();
            c.messages.clear();
            c.pending = 0;
        }
        self.inner.generation.fetch_add(1, Ordering::Relaxed);
    }

    pub fn settings(&self) -> PreviewSettings {
        lock(&self.inner.settings).clone()
    }

    /// The TeX program in use, when there is one.
    pub fn program(&self) -> Option<PathBuf> {
        lock(&self.inner.program).clone()
    }

    /// Whether texts get typeset: on, native, and a TeX program found.
    pub fn active(&self) -> bool {
        cfg!(not(target_arch = "wasm32")) && lock(&self.inner.settings).enabled && self.program().is_some()
    }

    /// Changes whenever a job finishes or the settings change.
    pub fn generation(&self) -> u64 {
        self.inner.generation.load(Ordering::Relaxed)
    }

    /// Jobs waiting or running.
    pub fn pending(&self) -> usize {
        lock(&self.inner.cache).pending
    }

    /// Errors of finished jobs, once each, for the message panel.
    pub fn take_messages(&self) -> Vec<String> {
        std::mem::take(&mut lock(&self.inner.cache).messages)
    }

    /// The typeset picture for sizes (bounds, picking); never starts TeX.
    pub fn peek(&self, req: &TypesetRequest<'_>) -> Option<Typeset> {
        if !self.acceptable(req) {
            return None;
        }
        match lock(&self.inner.cache).map.get(&Self::base_key(req)) {
            Some(Entry::Ready(t)) => Some(t.clone()),
            _ => None,
        }
    }

    fn base_key(req: &TypesetRequest<'_>) -> Key {
        Key {
            text: req.text.to_string(),
            font: req.font,
            postscript: req.postscript,
            size: (req.size * 100.0).round() as i64,
            rgb: [req.color.r, req.color.g, req.color.b],
        }
    }

    fn acceptable(&self, req: &TypesetRequest<'_>) -> bool {
        self.active()
            && !req.text.trim().is_empty()
            && req.text.len() <= MAX_TEXT_BYTES
            && req.size.is_finite()
            && req.size > 0.0
            && req.size <= 1000.0
    }

    /// The source for the render crate, `None` when nothing would be typeset.
    pub fn typesetter(&self) -> Option<Arc<dyn Typesetter>> {
        self.active()
            .then(|| Arc::new(Handle(self.clone())) as Arc<dyn Typesetter>)
    }

    fn lookup(&self, req: &TypesetRequest<'_>) -> Option<Typeset> {
        if !self.acceptable(req) {
            return None;
        }
        let key = Self::base_key(req);
        {
            let mut c = lock(&self.inner.cache);
            match c.map.get(&key) {
                Some(Entry::Ready(t)) => return Some(t.clone()),
                Some(_) => return None,
                None => {}
            }
            if c.pending >= MAX_PENDING {
                return None;
            }
            c.map.insert(key.clone(), Entry::Pending);
            c.pending += 1;
        }
        self.spawn(key);
        None
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn spawn(&self, key: Key) {
        let me = self.clone();
        let spawned = std::thread::Builder::new()
            .name("latex-preview".into())
            .spawn(move || me.job(key));
        if spawned.is_err() {
            // Cannot start a thread: forget the pending entries so a later
            // frame tries again.
            let mut c = lock(&self.inner.cache);
            c.map.retain(|_, e| !matches!(e, Entry::Pending));
            c.pending = 0;
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn spawn(&self, key: Key) {
        self.finish(key, Err("TeX does not run in the browser".to_string()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn job(&self, key: Key) {
        let _one_at_a_time = lock(&self.inner.gate);
        let settings = self.settings();
        let program = self.program();
        // Dropped from the cache by a settings change while waiting?
        if !matches!(lock(&self.inner.cache).map.get(&key), Some(Entry::Pending)) {
            return;
        }
        static JOBS: AtomicU64 = AtomicU64::new(0);
        let n = JOBS.fetch_add(1, Ordering::Relaxed);
        let result = match program {
            Some(p) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_tex(&p, &settings.preamble, &key, n)
            }))
            .unwrap_or_else(|_| Err("internal error while typesetting".to_string())),
            None => Err("no TeX program found".to_string()),
        };
        self.finish(key, result);
    }

    fn finish(&self, key: Key, result: Result<Typeset, String>) {
        let mut c = lock(&self.inner.cache);
        if !matches!(c.map.get(&key), Some(Entry::Pending)) {
            return;
        }
        c.pending = c.pending.saturating_sub(1);
        let entry = match result {
            Ok(t) => Entry::Ready(t),
            Err(e) => {
                let first = key.text.lines().next().unwrap_or_default();
                c.messages.push(format!("LaTeX text \"{}\": {e}", shorten(first, 30)));
                Entry::Failed
            }
        };
        c.map.insert(key.clone(), entry);
        c.order.push_back(key);
        while c.order.len() > MAX_CACHE {
            if let Some(old) = c.order.pop_front() {
                c.map.remove(&old);
            }
        }
        drop(c);
        self.inner.generation.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Debug)]
struct Handle(LatexPreview);

impl Typesetter for Handle {
    fn typeset(&self, req: &TypesetRequest<'_>) -> Option<Typeset> {
        self.0.lookup(req)
    }
}

fn floor_boundary(s: &str, max: usize) -> usize {
    let mut n = max.min(s.len());
    while n > 0 && !s.is_char_boundary(n) {
        n -= 1;
    }
    n
}

fn shorten(s: &str, n: usize) -> String {
    let end = floor_boundary(s, n);
    if end < s.len() {
        format!("{}...", s.get(..end).unwrap_or_default())
    } else {
        s.to_string()
    }
}

/// `program` as a path, or `pdflatex` / `lualatex` on the `PATH` and in the
/// usual TeX folders.
pub fn find_tex(program: &str) -> Option<PathBuf> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    let names: Vec<&str> = if program.trim().is_empty() {
        vec!["pdflatex", "lualatex"]
    } else {
        vec![program.trim()]
    };
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    for extra in ["/Library/TeX/texbin", "/usr/local/bin", "/opt/homebrew/bin", "/usr/bin"] {
        dirs.push(PathBuf::from(extra));
    }
    for name in names {
        let direct = PathBuf::from(name);
        if direct.components().count() > 1 {
            if direct.is_file() {
                return Some(direct);
            }
            continue;
        }
        for d in &dirs {
            for ext in ["", ".exe"] {
                let p = d.join(format!("{name}{ext}"));
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// The LaTeX file for one text: the preamble the `pdftex` export's
/// documents need (`amsmath`, `amssymb`, `color`, `graphicx`), the user's
/// additions, and the text in a box whose size and depth go to `doc.dim`.
pub fn latex_document(preamble: &str, key_text: &str, font: i32, postscript: bool, size: f64, rgb: [u8; 3]) -> String {
    let family = if postscript {
        // Times, Helvetica, Courier families of the PostScript set.
        match font {
            12..=15 => "\\sffamily",
            16..=19 => "\\ttfamily",
            _ => "\\rmfamily",
        }
    } else {
        match font {
            2 => "\\rmfamily\\bfseries",
            3 => "\\rmfamily\\itshape",
            4 => "\\sffamily",
            5 => "\\ttfamily",
            _ => "\\rmfamily",
        }
    };
    let c = |v: u8| format!("{:.3}", f64::from(v) / 255.0);
    format!(
        "\\documentclass{{article}}\n\\usepackage{{amsmath,amssymb,color,graphicx}}\n{preamble}\n\
         \\pagestyle{{empty}}\n\\newwrite\\rfdim\n\
         \\begin{{document}}\n\\immediate\\openout\\rfdim=doc.dim\n\
         \\setbox0\\hbox{{\\fontsize{{{size:.2}}}{{{lead:.2}}}\\selectfont {family}\\color[rgb]{{{r},{g},{b}}}{text}}}\n\
         \\immediate\\write\\rfdim{{\\the\\wd0,\\the\\ht0,\\the\\dp0}}\n\\immediate\\closeout\\rfdim\n\
         \\newdimen\\rfh \\rfh=\\ht0 \\advance\\rfh by \\dp0\n\
         \\ifdefined\\pagewidth\\pagewidth=\\wd0 \\pageheight=\\rfh\\else\\pdfpagewidth=\\wd0 \\pdfpageheight=\\rfh\\fi\n\
         \\hoffset=-1in \\voffset=-1in\n\\shipout\\hbox{{\\box0}}\n\\end{{document}}\n",
        lead = size * 1.2,
        r = c(rgb[0]),
        g = c(rgb[1]),
        b = c(rgb[2]),
        text = key_text,
    )
}

/// Parse `doc.dim`: width, height and depth in TeX points ("12.0pt 5.0pt 1.0pt").
#[cfg(not(target_arch = "wasm32"))]
fn parse_dim(s: &str) -> Option<(f64, f64, f64)> {
    let mut v = s
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| t.trim_end_matches("pt").parse::<f64>().ok());
    let (w, h, d) = (v.next()??, v.next()??, v.next()??);
    // TeX points (1/72.27 in) to PostScript points.
    let k = 72.0 / 72.27;
    [w, h, d]
        .iter()
        .all(|x| x.is_finite() && *x >= 0.0)
        .then_some((w * k, h * k, d * k))
}

#[cfg(not(target_arch = "wasm32"))]
fn run_tex(program: &std::path::Path, preamble: &str, key: &Key, n: u64) -> Result<Typeset, String> {
    use std::process::{Command, Stdio};
    let dir = std::env::temp_dir().join(format!("rustfig-tex-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot make a work folder: {e}"))?;
    let result = (|| {
        let size = key.size as f64 / 100.0;
        let tex = latex_document(preamble, &key.text, key.font, key.postscript, size, key.rgb);
        std::fs::write(dir.join("doc.tex"), tex).map_err(|e| format!("cannot write the TeX file: {e}"))?;
        let mut child = Command::new(program)
            .args([
                "-interaction=nonstopmode",
                "-halt-on-error",
                "-no-shell-escape",
                "doc.tex",
            ])
            .current_dir(&dir)
            .env("openin_any", "p")
            .env("openout_any", "p")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", program.display()))?;
        let start = std::time::Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) if start.elapsed().as_secs() >= MAX_TEX_SECONDS => {
                    let _ = child.kill(); // already failing; the wait below reaps it
                    let _ = child.wait(); // same
                    return Err(format!("TeX took longer than {MAX_TEX_SECONDS} s"));
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
                Err(e) => return Err(format!("TeX did not finish: {e}")),
            }
        };
        let pdf_path = dir.join("doc.pdf");
        let pdf = if status.success() || pdf_path.is_file() {
            read_limited(&pdf_path)
        } else {
            None
        };
        let Some(pdf) = pdf.filter(|_| status.success()) else {
            return Err(tex_error(&dir.join("doc.log")));
        };
        let dim = std::fs::read_to_string(dir.join("doc.dim")).unwrap_or_default();
        let (_w, _h, d) = parse_dim(&dim).ok_or_else(|| format!("TeX gave no size ({:?})", shorten(dim.trim(), 40)))?;
        render_pdf(pdf, d)
    })();
    // Best effort: a stray temp folder is harmless.
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[cfg(not(target_arch = "wasm32"))]
fn read_limited(p: &std::path::Path) -> Option<Vec<u8>> {
    use std::io::Read;
    let f = std::fs::File::open(p).ok()?;
    let mut buf = Vec::new();
    f.take(MAX_PDF_BYTES + 1).read_to_end(&mut buf).ok()?;
    (buf.len() as u64 <= MAX_PDF_BYTES).then_some(buf)
}

/// The first "! ..." line of TeX's log, or a general message.
#[cfg(not(target_arch = "wasm32"))]
fn tex_error(log: &std::path::Path) -> String {
    let text = read_limited(log).map(|b| String::from_utf8_lossy(&b).into_owned());
    text.as_deref()
        .and_then(|t| t.lines().find(|l| l.starts_with('!')))
        .map(|l| shorten(l.trim_start_matches('!').trim(), 80))
        .unwrap_or_else(|| "TeX failed".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn render_pdf(pdf: Vec<u8>, d: f64) -> Result<Typeset, String> {
    // The page is TeX's box (height plus depth); TeX's depth places the baseline.
    let pic = fig2dev::util::pdfpic::PdfPicture::open(pdf).ok_or("the TeX output cannot be shown")?;
    let (pw, ph) = pic.size();
    Ok(Typeset {
        pdf: Arc::new(pic),
        width_pt: pw,
        height_pt: ph,
        depth_pt: d.min(ph),
    })
}
