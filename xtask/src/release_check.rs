//! `cargo xtask release-check [--steps 1,12]` (plan P10): the checks run before every tag, one
//! section per step in `target/release-check/index.html`. A step not written yet is listed as such
//! and fails the check, so a release cannot pass by leaving steps out.
//!
//! Done so far: step 1 (`cargo xtask ci` with `RUSTFIG_REQUIRE_WASM=1`) and step 12 (the Rust
//! fig2dev against C fig2dev 3.2.9a). Step 12 runs both programs on the corpus sample
//! (`plan/corpus-sample.txt`) and fig2dev's own test figures for every language both write byte
//! for byte (not PDF and the bitmaps, which C makes through ghostscript, nor PostScript, which
//! the port writes from its scene: C-bug list entries 69 and 70),
//! and compares the outputs with timestamps, creator lines and the output folder masked. A
//! difference passes only when the local list `c-bugs-diffs.txt` in the fig2dev clone (next to
//! `c-bugs.md`, P12) names it: lines `LANG FIGURE #N [#M…] note`, where LANG and FIGURE may be `*`
//! and FIGURE may end in `*` (a prefix). Entries that match nothing are reported as stale.
//!
//! Environment: `FIG2DEV_C` (default the Homebrew C fig2dev 3.2.9a), `FIG2DEV_DIR` (default
//! `../fig2dev`), `RELEASE_CHECK_FILTER=substring` limits step 12 to figures whose path has it.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The steps of P10, by number; `None`: not written yet.
const STEPS: &[(u32, &str)] = &[
    (1, "cargo xtask ci with RUSTFIG_REQUIRE_WASM=1"),
    (2, "corpus sample (EPS, PDF, PNG) against the previous tag"),
    (3, "fig2dev's own suite and byte-identical counts"),
    (4, "perf budgets and bench against the previous tag"),
    (5, "release build driven over the control channel"),
    (6, "help tests and manual size"),
    (7, "spelling of new words"),
    (8, "MCP"),
    (9, "documentation"),
    (10, "website"),
    (11, "xfig reads RustFig's files"),
    (12, "Rust fig2dev = C fig2dev, differences in the C-bug list"),
];

const C_FIG2DEV: &str = "/opt/homebrew/Cellar/fig2dev/3.2.9a/bin/fig2dev";

/// Languages both programs write and that are compared byte for byte. Not compared: C's PDF and
/// bitmaps come from ghostscript, RustFig writes PDF itself and rasterizes with tiny-skia (entry
/// 69); PostScript (ps, eps, pstex) is written by the scene writer, the same drawing in other
/// PostScript code (entry 70). Step 2 compares those by rendering.
const LANGUAGES: &[(&str, &str)] = &[
    ("box", "box"),
    ("cgm", "cgm"),
    ("dxf", "dxf"),
    ("eepic", "eepic"),
    ("eepicemu", "eepicemu"),
    ("emf", "emf"),
    ("epic", "epic"),
    ("gbx", "gbx"),
    ("ge", "ge"),
    ("ibmgl", "hpl"),
    ("latex", "latex"),
    ("map", "map"),
    ("mf", "mf"),
    ("mp", "mp"),
    ("pic", "pic"),
    ("pict2e", "pict2e"),
    ("pictex", "pictex"),
    ("pstex_t", "pstex_t"),
    ("pdftex_t", "pdftex_t"),
    ("pstricks", "pstricks"),
    ("ptk", "ptk"),
    ("shape", "shape"),
    // sld is not byte for byte: C draws through ghostscript. It is compared drawn (R2b); the
    // encoder alone is byte-identical to netpbm's ppmtoacad on the same pixels.
    ("svg", "svg"),
    ("textyl", "textyl"),
    ("tikz", "tikz"),
    ("tk", "tk"),
    ("tpic", "tpic"),
];

/// Longest a single conversion may take.
const TIMEOUT: Duration = Duration::from_secs(60);

struct Section {
    step: u32,
    title: &'static str,
    passed: bool,
    html: String,
}

pub fn run(root: &Path, cargo: Command) -> Result<(), String> {
    let steps = selected_steps()?;
    let out = root.join("target/release-check");
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut sections = Vec::new();
    for &(n, title) in STEPS {
        if !steps.contains(&n) {
            continue;
        }
        println!("release-check: step {n}: {title}");
        let (passed, html) = match n {
            1 => step_ci(root),
            12 => step_fig2dev(root, &out, cargo_like(&cargo))?,
            _ => (false, "<p>Not written yet (plan P10).</p>".to_string()),
        };
        println!("release-check: step {n}: {}", if passed { "passed" } else { "FAILED" });
        sections.push(Section {
            step: n,
            title,
            passed,
            html,
        });
    }
    let index = out.join("index.html");
    std::fs::write(&index, report(&sections)).map_err(|e| format!("{}: {e}", index.display()))?;
    println!("release-check: report in {}", index.display());
    let failed: Vec<String> = sections
        .iter()
        .filter(|s| !s.passed)
        .map(|s| s.step.to_string())
        .collect();
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!("release-check: failed steps {}", failed.join(", ")))
    }
}

/// `--steps 1,12` (default: all).
fn selected_steps() -> Result<Vec<u32>, String> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let Some(i) = args.iter().position(|a| a == "--steps") else {
        return Ok(STEPS.iter().map(|s| s.0).collect());
    };
    let list = args.get(i + 1).ok_or("--steps needs a list such as 1,12")?;
    list.split(',')
        .map(|s| {
            s.trim()
                .parse::<u32>()
                .ok()
                .filter(|n| STEPS.iter().any(|s| s.0 == *n))
                .ok_or_else(|| format!("--steps: no step {s:?}"))
        })
        .collect()
}

/// A fresh `cargo` command with the same program and environment as `cargo`.
fn cargo_like(cargo: &Command) -> Command {
    let mut c = Command::new(cargo.get_program());
    for (k, v) in cargo.get_envs() {
        match v {
            Some(v) => c.env(k, v),
            None => c.env_remove(k),
        };
    }
    c
}

fn step_ci(root: &Path) -> (bool, String) {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("cargo"));
    let status = Command::new(exe)
        .arg("ci")
        .env("RUSTFIG_REQUIRE_WASM", "1")
        .current_dir(root)
        .status();
    match status {
        Ok(s) if s.success() => (true, "<p>All gates passed.</p>".into()),
        Ok(s) => (false, format!("<p>cargo xtask ci failed ({s}).</p>")),
        Err(e) => (
            false,
            format!("<p>cargo xtask ci did not start: {}</p>", esc(&e.to_string())),
        ),
    }
}

// ---- step 12 ----

/// One accepted difference from `c-bugs-diffs.txt`.
struct Accept {
    lang: String,
    figure: String,
    bugs: String,
    line: usize,
    used: AtomicUsize,
}

impl Accept {
    fn matches(&self, lang: &str, figure: &str) -> bool {
        let lang_ok = self.lang == "*" || self.lang == lang;
        let fig_ok = match self.figure.strip_suffix('*') {
            Some(prefix) => figure.starts_with(prefix),
            None => self.figure == figure,
        };
        lang_ok && fig_ok
    }
}

fn read_accepts(path: &Path) -> Result<Vec<Accept>, String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Ok(Vec::new());
    };
    let mut v = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with(';') {
            continue;
        }
        let mut parts = line.splitn(3, char::is_whitespace);
        let (Some(lang), Some(figure), Some(rest)) = (parts.next(), parts.next(), parts.next()) else {
            return Err(format!("{}:{}: want LANG FIGURE #N note", path.display(), i + 1));
        };
        if !rest.trim_start().starts_with('#') {
            return Err(format!(
                "{}:{}: the C-bug number (#N) is missing",
                path.display(),
                i + 1
            ));
        }
        v.push(Accept {
            lang: lang.to_string(),
            figure: figure.to_string(),
            bugs: rest.trim().to_string(),
            line: i + 1,
            used: AtomicUsize::new(0),
        });
    }
    Ok(v)
}

/// The figures of step 12: the corpus sample and fig2dev's test figures, repo-relative.
fn figures(root: &Path) -> Result<Vec<String>, String> {
    let sample = root.join("plan/corpus-sample.txt");
    let text = std::fs::read_to_string(&sample).map_err(|e| format!("{}: {e}", sample.display()))?;
    let mut v: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect();
    let data = root.join("old/fig2dev-3.2.9a/fig2dev/tests/data");
    if let Ok(dir) = std::fs::read_dir(&data) {
        let mut more: Vec<String> = dir
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "fig"))
            .filter_map(|p| p.strip_prefix(root).ok().map(|p| p.to_string_lossy().into_owned()))
            .collect();
        more.sort();
        v.extend(more);
    }
    v.sort();
    v.dedup();
    if let Ok(f) = std::env::var("RELEASE_CHECK_FILTER") {
        v.retain(|p| p.contains(&f));
    }
    Ok(v)
}

#[derive(Debug, PartialEq)]
enum Outcome {
    Same,
    /// Both failed (or C crashed and RustFig refused): nothing to compare.
    BothFailed,
    /// C crashed or failed where RustFig wrote a file.
    COnly(String),
    /// RustFig failed where C wrote a file.
    RustFailed(String),
    /// The same lines in another order: objects of one depth in xfig's order, where C sorts
    /// them with the platform's `qsort` (C-bug list entry 63).
    Reordered,
    /// The first differing line (masked) of C and of RustFig.
    Differ(usize, String, String),
}

struct Run {
    ok: bool,
    output: Vec<u8>,
    message: String,
}

fn convert(program: &Path, pre: &[&str], lang: &str, fig: &Path, out: &Path) -> Run {
    let _ = std::fs::remove_file(out);
    let cwd = fig.parent().unwrap_or(Path::new("."));
    let child = Command::new(program)
        .args(pre)
        .args(["-L", lang])
        .arg(fig.file_name().unwrap_or_default())
        .arg(out)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            return Run {
                ok: false,
                output: Vec::new(),
                message: e.to_string(),
            }
        }
    };
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() > TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(_) => break None,
        }
    };
    let mut stderr = String::new();
    if let Some(mut e) = child.stderr.take() {
        use std::io::Read;
        let mut buf = Vec::new();
        let _ = e.read_to_end(&mut buf);
        stderr = String::from_utf8_lossy(&buf)
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
    }
    let output = std::fs::read(out).unwrap_or_default();
    match status {
        Some(s) if s.success() => Run {
            ok: true,
            output,
            message: stderr,
        },
        Some(s) => Run {
            ok: false,
            output,
            message: format!("{s}: {stderr}"),
        },
        None => Run {
            ok: false,
            output,
            message: "timed out".into(),
        },
    }
}

/// Lines that carry the time, the user, the program's version or the output's folder.
fn masked(bytes: &[u8], dir: &str) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter(|l| {
            !(l.contains("CreationDate")
                || l.contains("Creation date")
                || l.contains("%%For:")
                || l.contains("%%Creator")
                || l.contains("Creator:")
                || l.contains("fig2dev Version")
                || l.contains("fig2dev version")
                || l.contains("Produced by fig2dev"))
        })
        .map(|l| l.replace(dir, "OUT"))
        .collect()
}

fn compare(c: &Run, r: &Run, c_dir: &str, r_dir: &str) -> Outcome {
    match (c.ok, r.ok) {
        (false, false) => Outcome::BothFailed,
        (false, true) => Outcome::COnly(c.message.clone()),
        (true, false) => Outcome::RustFailed(r.message.clone()),
        (true, true) => {
            let (a, b) = (masked(&c.output, c_dir), masked(&r.output, r_dir));
            if a == b {
                return Outcome::Same;
            }
            let (mut sa, mut sb) = (a.clone(), b.clone());
            sa.sort_unstable();
            sb.sort_unstable();
            if sa == sb {
                return Outcome::Reordered;
            }
            let i = a
                .iter()
                .zip(&b)
                .position(|(x, y)| x != y)
                .unwrap_or(a.len().min(b.len()));
            let line = |v: &[String]| {
                let s = v.get(i).cloned().unwrap_or_else(|| "(end)".into());
                s.chars().take(160).collect::<String>()
            };
            Outcome::Differ(i + 1, line(&a), line(&b))
        }
    }
}

/// `text` (a Fig 3.x file) with no two objects at one depth: depths renumbered by (depth, place
/// in the file), so C's `qsort` and xfig's order agree. `None`: not Fig 3, no ties, or more
/// objects than depths.
fn untied(text: &str) -> Option<String> {
    if !text.starts_with("#FIG 3") {
        return None;
    }
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    // The header ends with the resolution line (the 8th line not a comment in Fig 3.2; Fig
    // 3.0/3.1: the 4th). Objects: a line starting with 1 to 5 and a blank.
    let header = if text.starts_with("#FIG 3.2") { 8 } else { 4 };
    let mut seen = 0;
    let mut start = lines.len();
    for (i, l) in lines.iter().enumerate() {
        if l.starts_with('#') {
            continue;
        }
        seen += 1;
        if seen == header {
            start = i + 1;
            break;
        }
    }
    // (line, field index of the depth, depth)
    let mut objs: Vec<(usize, usize, i64)> = Vec::new();
    for (i, l) in lines.iter().enumerate().skip(start) {
        let field = match l.as_bytes() {
            [b'1'..=b'3' | b'5', b' ', ..] => 6,
            [b'4', b' ', ..] => 3,
            _ => continue,
        };
        let depth = l.split_whitespace().nth(field)?.parse::<i64>().ok()?;
        objs.push((i, field, depth));
    }
    let mut depths: Vec<i64> = objs.iter().map(|o| o.2).collect();
    depths.sort_unstable();
    if depths.windows(2).all(|w| w[0] != w[1]) || objs.len() > 1000 {
        return None;
    }
    let mut order: Vec<usize> = (0..objs.len()).collect();
    order.sort_by_key(|&k| (objs[k].2, k));
    let mut out: Vec<String> = lines.iter().map(|l| (*l).to_string()).collect();
    for (rank, &k) in order.iter().enumerate() {
        let (i, field, _) = objs[k];
        let l = &lines[i];
        let end = if l.ends_with('\n') { "\n" } else { "" };
        let mut f: Vec<String> = l.split_whitespace().map(str::to_string).collect();
        // Text strings keep their blanks: only the fields before the string are rewritten.
        if field == 3 {
            let mut parts = l.trim_end_matches('\n').splitn(14, ' ');
            let mut head: Vec<String> = (0..13).filter_map(|_| parts.next().map(str::to_string)).collect();
            let rest = parts.next()?;
            *head.get_mut(3)? = rank.to_string();
            out[i] = format!("{} {rest}{end}", head.join(" "));
            continue;
        }
        *f.get_mut(field)? = rank.to_string();
        out[i] = format!("{}{end}", f.join(" "));
    }
    Some(out.concat())
}

fn rustfig_cli(root: &Path, mut cargo: Command) -> Result<PathBuf, String> {
    println!("release-check: building rustfig-cli (release)");
    let s = cargo
        .args(["build", "--release", "-p", "rustfig-cli"])
        .current_dir(root)
        .status()
        .map_err(|e| e.to_string())?;
    if !s.success() {
        return Err("cargo build --release -p rustfig-cli failed".into());
    }
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    Ok(target.join("release/rustfig-cli"))
}

fn step_fig2dev(root: &Path, out: &Path, cargo: Command) -> Result<(bool, String), String> {
    let c_bin = std::env::var_os("FIG2DEV_C").map_or_else(|| PathBuf::from(C_FIG2DEV), PathBuf::from);
    if !c_bin.is_file() {
        return Ok((
            false,
            format!("<p>C fig2dev not found at {}.</p>", esc(&c_bin.to_string_lossy())),
        ));
    }
    let r_bin = rustfig_cli(root, cargo)?;
    let fig2dev_dir = std::env::var_os("FIG2DEV_DIR").map_or_else(|| root.join("../fig2dev"), PathBuf::from);
    let accepts_path = fig2dev_dir.join("c-bugs-diffs.txt");
    let accepts = read_accepts(&accepts_path)?;
    let figs = figures(root)?;
    let work = out.join("fig2dev");
    let _ = std::fs::remove_dir_all(&work);
    let jobs: Vec<(usize, &str)> = figs
        .iter()
        .enumerate()
        .flat_map(|(i, _)| LANGUAGES.iter().map(move |(l, _)| (i, *l)))
        .collect();
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(String, &str, Outcome)>> = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let done = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for t in 0..threads {
            let (jobs, figs, next, results, done, work) = (&jobs, &figs, &next, &results, &done, &work);
            let (c_bin, r_bin) = (&c_bin, &r_bin);
            s.spawn(move || {
                let c_dir = work.join(format!("c{t}"));
                let r_dir = work.join(format!("r{t}"));
                let _ = std::fs::create_dir_all(&c_dir);
                let _ = std::fs::create_dir_all(&r_dir);
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(i, lang)) = jobs.get(k) else { break };
                    let Some(rel) = figs.get(i) else { break };
                    let ext = LANGUAGES.iter().find(|l| l.0 == lang).map_or("out", |l| l.1);
                    let fig = root.join(rel);
                    let c = convert(c_bin, &[], lang, &fig, &c_dir.join(format!("out.{ext}")));
                    let r = convert(r_bin, &["fig2dev"], lang, &fig, &r_dir.join(format!("out.{ext}")));
                    let mut o = compare(&c, &r, &c_dir.to_string_lossy(), &r_dir.to_string_lossy());
                    if matches!(o, Outcome::Differ(..)) {
                        // The same figure without ties of depth, next to the original (its
                        // pictures are named relative to it).
                        let text = std::fs::read(&fig)
                            .ok()
                            .map(|b| String::from_utf8_lossy(&b).into_owned());
                        if let Some(u) = text.as_deref().and_then(untied) {
                            let tmp = fig.with_file_name(format!(".release-check-{t}.fig"));
                            if std::fs::write(&tmp, u).is_ok() {
                                let c = convert(c_bin, &[], lang, &tmp, &c_dir.join(format!("out.{ext}")));
                                let r = convert(r_bin, &["fig2dev"], lang, &tmp, &r_dir.join(format!("out.{ext}")));
                                if compare(&c, &r, &c_dir.to_string_lossy(), &r_dir.to_string_lossy()) == Outcome::Same
                                {
                                    o = Outcome::Reordered;
                                }
                                let _ = std::fs::remove_file(&tmp);
                            }
                        }
                    }
                    if let Ok(mut v) = results.lock() {
                        v.push((rel.clone(), lang, o));
                    }
                    let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if n % 1000 == 0 {
                        println!("release-check: step 12: {n} of {}", jobs.len());
                    }
                }
            });
        }
    });
    let mut results = results.into_inner().map_err(|_| "a worker panicked".to_string())?;
    results.sort_by(|a, b| (a.1, &a.0).cmp(&(b.1, &b.0)));
    let _ = std::fs::remove_dir_all(&work);

    // Count per language; unlisted differences fail.
    let mut per_lang: BTreeMap<&str, [usize; 5]> = BTreeMap::new();
    let mut failing = Vec::new();
    let mut listed = Vec::new();
    let mut tsv = String::from("status\tlang\tfigure\tline\tC\tRustFig\n");
    for (fig, lang, o) in &results {
        let e = per_lang.entry(lang).or_default();
        let accepted = || {
            accepts.iter().find(|a| a.matches(lang, fig)).inspect(|a| {
                a.used.fetch_add(1, Ordering::Relaxed);
            })
        };
        let (status, line, c, r) = match o {
            Outcome::Same => {
                e[0] += 1;
                continue;
            }
            Outcome::BothFailed => {
                e[1] += 1;
                continue;
            }
            Outcome::Reordered => {
                e[4] += 1;
                continue;
            }
            Outcome::COnly(m) => ("c-failed", 0, m.clone(), String::new()),
            Outcome::RustFailed(m) => ("rust-failed", 0, String::new(), m.clone()),
            Outcome::Differ(n, a, b) => ("differ", *n, a.clone(), b.clone()),
        };
        match accepted() {
            Some(a) => {
                e[2] += 1;
                listed.push((fig.clone(), *lang, status, a.bugs.clone()));
            }
            None => {
                e[3] += 1;
                failing.push((fig.clone(), *lang, status, line, c.clone(), r.clone()));
            }
        }
        let _ = writeln!(
            tsv,
            "{status}\t{lang}\t{fig}\t{line}\t{}\t{}",
            c.replace('\t', " "),
            r.replace('\t', " ")
        );
    }
    let tsv_path = out.join("fig2dev.tsv");
    std::fs::write(&tsv_path, tsv).map_err(|e| format!("{}: {e}", tsv_path.display()))?;
    let stale: Vec<&Accept> = accepts.iter().filter(|a| a.used.load(Ordering::Relaxed) == 0).collect();

    let mut h = String::new();
    let _ = write!(
        h,
        "<p>{} figures × {} languages; C fig2dev {}; accepted differences from {} ({} entries{}).</p>\
         <table><tr><th>language</th><th>same</th><th>order of equal depths (63)</th><th>both failed</th><th>listed</th><th>unlisted</th></tr>",
        figs.len(),
        LANGUAGES.len(),
        esc(&c_bin.to_string_lossy()),
        esc(&accepts_path.to_string_lossy()),
        accepts.len(),
        if accepts_path.is_file() { "" } else { ", file missing" }
    );
    for (lang, n) in &per_lang {
        let _ = write!(
            h,
            "<tr><td>{lang}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td class={}>{}</td></tr>",
            n[0],
            n[4],
            n[1],
            n[2],
            if n[3] > 0 { "bad" } else { "ok" },
            n[3]
        );
    }
    h.push_str("</table>");
    if !failing.is_empty() {
        let _ = write!(
            h,
            "<h3>Unlisted differences ({})</h3><p>Every row in <code>fig2dev.tsv</code>.</p><table>\
             <tr><th>language</th><th>figure</th><th>kind</th><th>line</th><th>C</th><th>RustFig</th></tr>",
            failing.len()
        );
        for (fig, lang, status, line, c, r) in failing.iter().take(500) {
            let _ = write!(
                h,
                "<tr><td>{lang}</td><td>{}</td><td>{status}</td><td>{line}</td><td><code>{}</code></td><td><code>{}</code></td></tr>",
                esc(fig),
                esc(c),
                esc(r)
            );
        }
        h.push_str("</table>");
    }
    if !stale.is_empty() {
        h.push_str("<h3>Entries that matched nothing</h3><ul>");
        for a in &stale {
            let _ = write!(
                h,
                "<li>line {}: {} {} {}</li>",
                a.line,
                esc(&a.lang),
                esc(&a.figure),
                esc(&a.bugs)
            );
        }
        h.push_str("</ul>");
    }
    println!(
        "release-check: step 12: {} comparisons, {} listed differences, {} unlisted, {} stale entries",
        results.len(),
        listed.len(),
        failing.len(),
        stale.len()
    );
    Ok((failing.is_empty(), h))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn report(sections: &[Section]) -> String {
    let mut h = String::from(
        "<!doctype html><meta charset=utf-8><title>Release check</title><style>\
         body{font:14px system-ui;margin:16px;max-width:1200px}table{border-collapse:collapse}\
         td,th{border:1px solid #ccc;padding:2px 6px;text-align:left;vertical-align:top}\
         code{font-size:12px;word-break:break-all}.bad{color:#b00;font-weight:bold}.ok{color:#070}</style>\
         <h1>Release check</h1><ul>",
    );
    for s in sections {
        let _ = write!(
            h,
            "<li><a href=#s{}>Step {}: {}</a> <span class={}>{}</span></li>",
            s.step,
            s.step,
            esc(s.title),
            if s.passed { "ok" } else { "bad" },
            if s.passed { "passed" } else { "failed" }
        );
    }
    h.push_str("</ul>");
    for s in sections {
        let _ = write!(h, "<h2 id=s{}>Step {}: {}</h2>{}", s.step, s.step, esc(s.title), s.html);
    }
    h.push_str(
        "<h2>Checks by hand</h2><ul><li>Windows: install and print hand-off</li>\
         <li>Linux: Wayland and HiDPI</li><li>The web demo in browsers other than the headless one</li></ul>",
    );
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_dates_creators_and_the_folder() {
        let a = b"%!PS\n%%CreationDate: Mon\n%%For: me@host\nfoo /tmp/c0/out.eps\n";
        assert_eq!(masked(a, "/tmp/c0"), ["%!PS", "foo OUT/out.eps"]);
    }

    #[test]
    fn untied_renumbers_equal_depths_in_file_order() {
        let fig = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n\t 0 0 10 10\n\
4 0 0 50 -1 0 12 0.0000 4 135 300 1200 1200 a  b\\001\n\
1 3 0 1 0 7 40 -1 -1 0.000 1 0.0000 0 0 10 10 0 0 10 0\n";
        let u = untied(fig).unwrap();
        assert!(u.contains("\n2 1 0 1 0 7 1 -1"), "{u}");
        assert!(
            u.contains("\n4 0 0 2 -1 0 12 0.0000 4 135 300 1200 1200 a  b\\001\n"),
            "{u}"
        );
        assert!(u.contains("\n1 3 0 1 0 7 0 -1"), "{u}");
        assert!(u.contains("\t 0 0 10 10\n"), "{u}");
        assert!(untied(&fig.replace(" 40 ", " 30 ").replace("4 0 0 50", "4 0 0 45")).is_none());
    }

    #[test]
    fn accept_patterns() {
        let a = |lang: &str, figure: &str| Accept {
            lang: lang.into(),
            figure: figure.into(),
            bugs: "#1".into(),
            line: 1,
            used: AtomicUsize::new(0),
        };
        assert!(a("svg", "old/x.fig").matches("svg", "old/x.fig"));
        assert!(!a("svg", "old/x.fig").matches("tikz", "old/x.fig"));
        assert!(a("*", "old/*").matches("tikz", "old/y.fig"));
        assert!(!a("*", "old/*").matches("tikz", "crates/y.fig"));
    }
}
