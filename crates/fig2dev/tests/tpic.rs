#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `tpic` driver against output of the real fig2dev 3.2.9a.

mod pic_common;
mod tex_common;

use tex_common::{compare_with_fig2dev, run, HAND};

const A: &str = r#"2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 1 1 3
	1 1 1.00 60.00 120.00
	0 0 1.00 60.00 120.00
	 600 600 1800 900 2400 2400
"#;
const D: &str = r#"2 4 1 2 0 7 50 -1 10 4.000 0 0 7 0 0 5
	 600 600 1800 600 1800 1200 600 1200 600 600
1 3 0 1 0 7 50 -1 -1 0.000 1 0.0000 2400 1200 300 300 2400 1200 2700 1200
4 1 0 50 -1 16 12 0.0000 4 135 450 1200 2400 a_b $x$\001
4 0 0 50 -1 2 10 0.0000 2 135 450 1200 3000 \\LaTeX\001
"#;

#[test]
fn objects() {
    let (out, _, exit) = run("tpic", A, &[]).unwrap();
    assert_eq!(
        out,
        r#".PS
.ps 8
line <-> from 0.500,10.000 to 1.500,9.750 to 2.000,8.500
.PE
"#
    );
    assert_eq!(exit, 0);
    let (out, msgs, _) = run("tpic", D, &[]).unwrap();
    assert_eq!(
        out,
        r#".PS
.ps 8
ellipse at 2.000,9.500 wid 0.500 ht 0.500
.ps 16
dashwid = 0.003i
box height 0.500 width 1.000 dashed radius 0.087 shaded 0.450 with .nw at 0.500,10.000
.baseline 12
"\twlsf a_b $x$" at 1.000,8.542 
.baseline 10
"\tenbf \LaTeX" at 1.000,8.035 ljust
.PE
"#
    );
    assert!(msgs.is_empty());
}

#[test]
fn options() {
    let (out, _, _) = run("tpic", D, &[('f', Some("times"))]).unwrap();
    assert!(
        out.contains(r#""\font\figbf=bf at 10pt \figbf \LaTeX" at 1.000,8.035 ljust"#),
        "{out}"
    );
    assert_eq!(
        run("tpic", D, &[('x', None)]).unwrap_err(),
        "Argument -x unknown to tpic driver."
    );
}

#[test]
fn hostile_values_dont_panic() {
    let body = "2 4 7 1 0 7 50 -1 30 0.000 0 0 2147483647 0 0 5
	 -2147483648 0 2147483647 0 2147483647 9 0 9 0 0
4 9 0 50 -1 -9 -12 0.0000 4 135 450 1200 2400 x\\001
4 0 0 50 -1 99 1e300 0.0000 0 135 450 1200 2400 x\\001
";
    for opts in [&[][..], &[('m', Some("0"))][..]] {
        let _ = run("tpic", body, opts);
    }
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test tpic -- --ignored --nocapture`.
#[test]
#[ignore]
fn same_as_fig2dev() {
    let mut extra = pic_common::FIGS.to_vec();
    extra.extend(HAND);
    let opts: &[&[&str]] = &[&[], &["-f", "times", "-m", "2"], &["-f", "tt", "-s", "14"]];
    if let Some((same, total)) = compare_with_fig2dev("tpic", &extra, opts) {
        assert!(same * 10 >= total * 9, "{same} of {total} identical");
    }
}
