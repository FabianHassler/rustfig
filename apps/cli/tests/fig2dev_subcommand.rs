//! `rustfig-cli fig2dev ARGS` runs fig2dev's command line unchanged (DD10).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

fn run(args: &[&str], via_cli: bool) -> (Vec<u8>, Vec<u8>, i32) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_rustfig-cli"));
    if via_cli {
        c.arg("fig2dev");
    }
    let o = c.args(args).env("SOURCE_DATE_EPOCH", EPOCH).output().unwrap();
    (o.stdout, o.stderr, o.status.code().unwrap_or(-1))
}

/// fig2dev stamps the run time into its output (EPS `%%CreationDate`, the
/// PDF Info dictionary); a fixed `SOURCE_DATE_EPOCH` makes every run equal.
const EPOCH: &str = "1767225600";

#[test]
fn subcommand_matches_in_process_fig2dev() {
    let fx = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/engine/tests/fixtures");
    let boxes = fx.join("boxes.fig").to_string_lossy().into_owned();
    let cases: Vec<Vec<&str>> = vec![
        vec!["-L", "eps", &boxes],
        vec!["-L", "svg", &boxes],
        vec!["-h"],
        vec!["-V"],
        vec!["-L", "eps", "/nonexistent/missing.fig"],
        vec!["--bogus"],
    ];
    std::env::set_var("SOURCE_DATE_EPOCH", EPOCH);
    for args in cases {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = fig2dev::cli::run(
            args.iter().map(|s| s.to_string()).collect(),
            &mut std::io::empty(),
            &mut out,
            &mut err,
        );
        assert_eq!(run(&args, true), (out, err, i32::from(code)), "{args:?}");
    }
}
