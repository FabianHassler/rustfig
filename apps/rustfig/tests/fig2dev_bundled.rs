//! D10 / DD10: fig2dev bundled into RustFig. The `rustfig` binary run as
//! `rustfig fig2dev ARGS` and under the name `fig2dev` (a symlink or copy),
//! and `rustfig-cli fig2dev ARGS`, behave exactly as fig2dev's command line
//! (`fig2dev::cli::run`, which the standalone `fig2dev` binary is a thin
//! `main` around): same stdout bytes, stderr and exit status.
//!
//! Set `FIG2DEV_BIN` to a standalone `fig2dev` binary to compare against it
//! as well.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(not(target_arch = "wasm32"))]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, PartialEq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: i32,
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/engine/tests/fixtures")
}

/// fig2dev stamps the run time into its output (EPS `%%CreationDate`, the
/// PDF Info dictionary); a fixed `SOURCE_DATE_EPOCH` makes every run equal.
const EPOCH: &str = "1767225600";

/// The reference: fig2dev::cli::run in this process.
fn in_process(args: &[&str], stdin: &[u8]) -> Run {
    std::env::set_var("SOURCE_DATE_EPOCH", EPOCH);
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = fig2dev::cli::run(
        args.iter().map(|s| s.to_string()).collect(),
        &mut &stdin[..],
        &mut out,
        &mut err,
    );
    Run {
        stdout: out,
        stderr: err,
        code: i32::from(code),
    }
}

fn spawn(mut cmd: Command, stdin: &[u8]) -> Run {
    let mut child = cmd
        .env("SOURCE_DATE_EPOCH", EPOCH)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut si = child.stdin.take().unwrap();
    let data = stdin.to_vec();
    let w = std::thread::spawn(move || {
        let _ = si.write_all(&data); // the child may exit without reading
    });
    let o = child.wait_with_output().unwrap();
    w.join().unwrap();
    Run {
        stdout: o.stdout,
        stderr: o.stderr,
        code: o.status.code().unwrap_or(-1),
    }
}

/// A copy of the rustfig binary named `fig2dev` (a copy works on every platform).
fn fig2dev_named(dir: &Path) -> PathBuf {
    let name = if cfg!(windows) { "fig2dev.exe" } else { "fig2dev" };
    let dst = dir.join(name);
    std::fs::copy(env!("CARGO_BIN_EXE_rustfig"), &dst).unwrap();
    dst
}

fn cases() -> Vec<(Vec<String>, Vec<u8>)> {
    let fx = fixtures();
    let mut v: Vec<(Vec<String>, Vec<u8>)> = Vec::new();
    for fig in ["boxes.fig", "text.fig", "splines.fig", "arrows.fig", "metric.fig"] {
        let p = fx.join(fig).to_string_lossy().into_owned();
        for lang in ["eps", "svg", "pdf", "pic", "tk"] {
            v.push((vec!["-L".into(), lang.into(), p.clone()], vec![]));
        }
        v.push((
            vec![
                "-L".into(),
                "eps".into(),
                "-m".into(),
                "0.5".into(),
                "-c".into(),
                p.clone(),
            ],
            vec![],
        ));
        v.push((
            vec!["-L".into(), "svg".into(), "-g".into(), "lightblue".into(), p.clone()],
            vec![],
        ));
        v.push((vec!["-L".into(), "png".into(), "-S".into(), "2".into(), p], vec![]));
    }
    // stdin input
    let data = std::fs::read(fx.join("lines.fig")).unwrap();
    v.push((vec!["-L".into(), "eps".into()], data.clone()));
    v.push((vec!["-L".into(), "svg".into(), "-".into()], data));
    // help, version, errors
    v.push((vec!["-h".into()], vec![]));
    v.push((vec!["-V".into()], vec![]));
    v.push((vec![], vec![]));
    v.push((vec!["--bogus".into()], vec![]));
    v.push((vec!["-L".into(), "nosuchlang".into(), "x.fig".into()], vec![]));
    v.push((
        vec!["-L".into(), "eps".into(), "/nonexistent/missing.fig".into()],
        vec![],
    ));
    v
}

#[test]
fn bundled_entry_points_match_fig2dev() {
    let dir = std::env::temp_dir().join(format!("rustfig-d10-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let named = fig2dev_named(&dir);
    let standalone = std::env::var_os("FIG2DEV_BIN").map(PathBuf::from);
    let cases = cases();
    assert!(cases.len() > 30);
    let mut nonzero = 0;
    for (args, stdin) in &cases {
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let want = in_process(&refs, stdin);
        if want.code != 0 {
            nonzero += 1;
        }
        let mut sub = Command::new(env!("CARGO_BIN_EXE_rustfig"));
        sub.arg("fig2dev").args(args);
        assert_eq!(spawn(sub, stdin), want, "rustfig fig2dev {args:?}");

        let mut by_name = Command::new(&named);
        by_name.args(args);
        assert_eq!(spawn(by_name, stdin), want, "as fig2dev {args:?}");

        if let Some(bin) = &standalone {
            let mut c = Command::new(bin);
            c.args(args);
            assert_eq!(spawn(c, stdin), want, "standalone {args:?}");
        }
    }
    assert!(nonzero >= 5, "the error cases must fail");
    // -h prints something, a missing file names it.
    let help = in_process(&["-h"], b"");
    assert!(!help.stdout.is_empty() || !help.stderr.is_empty());
    let missing = in_process(&["-L", "eps", "/nonexistent/missing.fig"], b"");
    assert_ne!(missing.code, 0);
    assert!(String::from_utf8_lossy(&missing.stderr).contains("missing.fig"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The GUI's own options are untouched.
#[test]
fn rustfig_own_options_unchanged() {
    let mut c = Command::new(env!("CARGO_BIN_EXE_rustfig"));
    c.arg("-V");
    let r = spawn(c, b"");
    assert_eq!(r.code, 0);
    assert!(String::from_utf8_lossy(&r.stdout).starts_with("rustfig "));
}
