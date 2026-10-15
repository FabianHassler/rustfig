//! B10: fig2dev's own test suite (`fig2dev/tests/*.at`, autotest) run
//! against RustFig's fig2dev: every `AT_CHECK` runs through `sh` with this
//! fig2dev first on `PATH`, and its exit status, stdout and stderr are
//! compared as autotest compares them. Cases that need a language RustFig
//! hasn't ported are counted apart. Needs fig2dev's sources in `old/`;
//! opt-in: `cargo test -p fig2dev --test fig2dev_suite -- --ignored
//! --nocapture` (`RUSTFIG_SUITE_FILTER=read` narrows it). Writes a report
//! to `target/fig2dev-suite.txt`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../old/fig2dev-3.2.9a/fig2dev/tests")
}

/// The arguments of the m4 macro call starting at `s[open]` == '(': split
/// at top-level commas, with m4's [ ] quotes and nested parentheses.
fn macro_args(s: &str, open: usize) -> Option<(Vec<String>, usize)> {
    let b = s.as_bytes();
    let (mut quote, mut paren, mut i) = (0i32, 0i32, open + 1);
    let mut args = Vec::new();
    let mut cur = String::new();
    while i < b.len() {
        let c = b[i] as char;
        match c {
            '[' => {
                quote += 1;
                cur.push(c);
            }
            ']' => {
                quote -= 1;
                cur.push(c);
            }
            '(' if quote == 0 => {
                paren += 1;
                cur.push(c);
            }
            ')' if quote == 0 && paren == 0 => {
                args.push(cur);
                return Some((args, i + 1));
            }
            ')' if quote == 0 => {
                paren -= 1;
                cur.push(c);
            }
            ',' if quote == 0 && paren == 0 => {
                args.push(std::mem::take(&mut cur));
                // m4 skips whitespace after the comma.
                while b.get(i + 1).is_some_and(|c| c.is_ascii_whitespace()) {
                    i += 1;
                }
            }
            _ => {
                let ch = s[i..].chars().next()?;
                cur.push(ch);
                i += ch.len_utf8() - 1;
            }
        }
        i += 1;
    }
    None
}

/// Remove one level of m4 quotes.
fn unquote(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '[' => {
                if depth > 0 {
                    out.push(c);
                }
                depth += 1;
            }
            ']' => {
                depth -= 1;
                if depth > 0 {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// An argument as the shell sees it: the outer quotes and one rescan's
/// quotes removed, the suite's macros expanded, m4's quadrigraphs decoded.
fn arg(s: &str) -> String {
    unquote(&expand(&unquote(s.trim())))
        .replace("@<:@", "[")
        .replace("@:>@", "]")
        .replace("@S|@", "$")
        .replace("@%:@", "#")
}

/// Top-level text between the macros: shell lines of the test group.
fn shell_text(s: &str) -> String {
    unquote(&expand(s))
        .replace("@<:@", "[")
        .replace("@:>@", "]")
        .replace("@S|@", "$")
        .replace("@%:@", "#")
}

/// Expand the macros the suite uses: FIG_FILE_TOP, NO_GS, AS_ECHO_N,
/// m4_for (without the loop variable), dnl; drop AT_KEYWORDS.
fn expand(s: &str) -> String {
    let fig_top = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2";
    let mut out = String::new();
    let mut i = 0;
    let word_start = |out: &String| !out.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_');
    while i < s.len() {
        let r = &s[i..];
        if r.starts_with("dnl") && word_start(&out) {
            i += r.find('\n').map_or(r.len(), |n| n + 1);
        } else if r.starts_with("FIG_FILE_TOP") {
            out.push_str(fig_top);
            i += "FIG_FILE_TOP".len();
        } else if r.starts_with("NO_GS") {
            out.push_str("! $GSEXE --version");
            i += "NO_GS".len();
        } else if let Some((name, a, after)) = ["AS_ECHO_N", "m4_for", "AT_KEYWORDS"]
            .iter()
            .find(|m| r.starts_with(&format!("{m}(")) && word_start(&out))
            .and_then(|m| macro_args(s, i + m.len()).map(|(a, after)| (*m, a, after)))
        {
            match name {
                "AS_ECHO_N" => {
                    out.push_str("printf %s ");
                    out.push_str(&expand(a.first().map_or("", |x| x.as_str())));
                }
                "m4_for" => {
                    let n = |k: usize| {
                        a.get(k)
                            .and_then(|x| unquote(x).trim().parse::<usize>().ok())
                            .unwrap_or(0)
                    };
                    let body = expand(&unquote(a.get(4).map_or("", |x| x.as_str())));
                    for _ in n(1)..=n(2) {
                        out.push_str(&body);
                    }
                }
                _ => {}
            }
            i = after;
        } else {
            let Some(ch) = r.chars().next() else { break };
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

#[derive(Debug)]
enum Step {
    Check {
        cmd: String,
        status: Option<String>,
        stdout: Option<String>,
        stderr: Option<String>,
    },
    Data(String, String),
    SkipIf(String),
    Shell(String),
}

struct Case {
    file: String,
    title: String,
    steps: Vec<Step>,
}

fn parse(file: &str, text: &str) -> Vec<Case> {
    let mut cases = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("AT_SETUP(") {
        let start = i + "AT_SETUP".len();
        let Some((title, after)) = macro_args(rest, start) else {
            break;
        };
        let end = rest[after..].find("AT_CLEANUP").map_or(rest.len(), |e| after + e);
        let body = &rest[after..end];
        let mut steps = Vec::new();
        let mut k = 0;
        while k < body.len() {
            let next = ["AT_CHECK(", "AT_DATA(", "AT_SKIP_IF("]
                .iter()
                .filter_map(|m| body[k..].find(m).map(|p| (k + p, *m)))
                .min_by_key(|(p, _)| *p);
            let Some((p, m)) = next else {
                steps.push(Step::Shell(shell_text(&body[k..])));
                break;
            };
            steps.push(Step::Shell(shell_text(&body[k..p])));
            let Some((a, after)) = macro_args(body, p + m.len() - 1) else {
                break;
            };
            let opt = |i: usize| a.get(i).map(|s| arg(s)).filter(|s| !s.is_empty());
            steps.push(match m {
                "AT_CHECK(" => Step::Check {
                    cmd: opt(0).unwrap_or_default(),
                    status: opt(1),
                    stdout: a.get(2).map(|s| arg(s)),
                    stderr: a.get(3).map(|s| arg(s)),
                },
                "AT_DATA(" => Step::Data(opt(0).unwrap_or_default(), a.get(1).map(|s| arg(s)).unwrap_or_default()),
                _ => Step::SkipIf(opt(0).unwrap_or_default()),
            });
            k = after;
        }
        // Commented-out cases (dnl AT_SETUP) are skipped by the caller.
        let line_start = rest[..i].rfind('\n').map_or(0, |n| n + 1);
        if !rest[line_start..i].contains("dnl") {
            cases.push(Case {
                file: file.to_string(),
                title: title.first().map(|t| arg(t)).unwrap_or_default(),
                steps,
            });
        }
        rest = &rest[end.min(rest.len())..];
        if let Some(c) = rest.find("AT_CLEANUP") {
            rest = &rest[c + "AT_CLEANUP".len()..];
        } else {
            break;
        }
    }
    cases
}

/// The languages a command asks for (`-L lang`).
fn languages(cmd: &str) -> Vec<String> {
    let mut out = Vec::new();
    let toks: Vec<&str> = cmd.split_whitespace().collect();
    for (i, t) in toks.iter().enumerate() {
        if *t == "-L" {
            if let Some(n) = toks.get(i + 1) {
                out.push(n.trim_matches(['"', '\'']).to_string());
            }
        } else if let Some(l) = t.strip_prefix("-L").filter(|l| !l.is_empty()) {
            out.push(l.to_string());
        }
    }
    out
}

const PORTED: &[&str] = &[
    "box", "eps", "gif", "jpeg", "pcx", "pdf", "png", "ppm", "ps", "svg", "tiff", "xbm", "xpm",
];

enum Outcome {
    Pass,
    Fail(String),
    Skipped(String),
    NotPorted(String),
}

fn run_case(case: &Case, bin_dir: &Path, work: &Path) -> Outcome {
    let _ = std::fs::remove_dir_all(work);
    std::fs::create_dir_all(work).unwrap();
    let path = format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap_or_default());
    let sh = |cmd: &str| {
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(work)
            .env("PATH", &path)
            .env("srcdir", tests_dir())
            .env("abs_srcdir", tests_dir())
            .env("FGREP", "grep -F")
            .env("SED", "sed")
            .env("GSEXE", "gs")
            .env("WITH_PNG_TRUE", "")
            .env("LC_ALL", "C")
            .env("INSTALLCHECK", "no")
            .output()
            .unwrap()
    };
    let mut prelude = String::new();
    for step in &case.steps {
        match step {
            Step::Shell(text) => {
                prelude.push_str(text);
                prelude.push('\n');
                continue;
            }
            Step::Data(name, content) => {
                std::fs::write(work.join(name), content).unwrap();
            }
            Step::SkipIf(cond) => {
                if sh(&format!("{prelude}{cond}")).status.success() {
                    return Outcome::Skipped(format!("AT_SKIP_IF({cond})"));
                }
            }
            Step::Check {
                cmd,
                status,
                stdout,
                stderr,
            } => {
                if let Some(l) = languages(cmd).into_iter().find(|l| !PORTED.contains(&l.as_str())) {
                    return Outcome::NotPorted(l);
                }
                let out = sh(&format!("{prelude}{cmd}"));
                let code = out.status.code().unwrap_or(-1);
                let want = status.as_deref().unwrap_or("0").trim();
                if want != "ignore" && want.parse::<i32>().ok() != Some(code) {
                    return Outcome::Fail(format!(
                        "exit {code}, expected {want}\n  stderr: {}",
                        String::from_utf8_lossy(&out.stderr).trim()
                    ));
                }
                for (got, expected, what) in [(&out.stdout, stdout, "stdout"), (&out.stderr, stderr, "stderr")] {
                    let got = String::from_utf8_lossy(got);
                    match expected.as_deref() {
                        Some("ignore") | Some("ignore-no-log") | Some("ignore-nolog") => {}
                        Some("stdout") | Some("stderr") | Some("expout") | Some("experr") => {}
                        Some(e) if got != e => {
                            return Outcome::Fail(format!("{what}: got {got:?}, expected {e:?}"));
                        }
                        None if !got.is_empty() => {
                            return Outcome::Fail(format!("{what}: got {got:?}, expected nothing"));
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Outcome::Pass
}

#[test]
#[ignore]
fn fig2dev_test_suite() {
    let dir = tests_dir();
    if !dir.exists() {
        eprintln!("fig2dev's sources are not in old/; skipping");
        return;
    }
    let bin = PathBuf::from(env!("CARGO_BIN_EXE_fig2dev"));
    let bin_dir = bin.parent().unwrap().to_path_buf();
    let filter = std::env::var("RUSTFIG_SUITE_FILTER").unwrap_or_default();
    let work = std::env::temp_dir().join(format!("rustfig-fig2dev-suite-{}", std::process::id()));
    let mut report = String::new();
    let (mut pass, mut fail, mut skipped, mut not_ported) = (0, 0, 0, 0);
    for file in ["input.at", "read.at", "output.at", "bitmaps.at"] {
        if !filter.is_empty() && !file.contains(&filter) {
            continue;
        }
        let text = std::fs::read_to_string(dir.join(file)).unwrap();
        for case in parse(file, &text) {
            let line = match run_case(&case, &bin_dir, &work) {
                Outcome::Pass => {
                    pass += 1;
                    format!("PASS       {}: {}", case.file, case.title)
                }
                Outcome::Fail(why) => {
                    fail += 1;
                    format!("FAIL       {}: {}\n  {why}", case.file, case.title)
                }
                Outcome::Skipped(why) => {
                    skipped += 1;
                    format!("SKIP       {}: {} ({why})", case.file, case.title)
                }
                Outcome::NotPorted(l) => {
                    not_ported += 1;
                    format!("NOT PORTED {}: {} (-L {l})", case.file, case.title)
                }
            };
            report.push_str(&line);
            report.push('\n');
        }
    }
    let _ = std::fs::remove_dir_all(&work);
    let summary = format!(
        "fig2dev's test suite on RustFig's fig2dev: {pass} pass, {fail} fail, {skipped} skipped, {not_ported} need a language not ported yet\n"
    );
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/fig2dev-suite.txt");
    std::fs::write(&out, format!("{summary}\n{report}")).unwrap();
    eprintln!("{report}\n{summary}report: {}", out.display());
}

#[test]
fn m4_arguments_and_quotes() {
    let s = "AT_CHECK([fig2dev -L pict2e <<EOF | grep 'p\\(([[0-9]]*)\\)'\nFIG_FILE_TOP\nEOF\n],0,ignore,[An open box - close it.\n])";
    let (a, _) = macro_args(s, "AT_CHECK".len()).unwrap();
    assert_eq!(a.len(), 4);
    let cmd = arg(&a[0]);
    assert!(cmd.contains("grep 'p\\(([0-9]*)\\)'"), "{cmd}");
    assert!(cmd.contains("#FIG 3.2\nLandscape"), "{cmd}");
    assert_eq!(arg(&a[3]), "An open box - close it.\n");
    assert_eq!(languages("fig2dev -L pict2e x | y"), vec!["pict2e"]);
    assert_eq!(languages("fig2dev -Leps -F"), vec!["eps"]);
}
