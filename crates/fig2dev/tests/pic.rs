#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `pic` driver against output of the real fig2dev 3.2.9a.

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
    let (out, _, exit) = run("pic", A, &[]).unwrap();
    assert_eq!(
        out,
        r#".PS
.ps 11
line <-> from 0.500,10.000 to 1.500,9.750 to 2.000,8.500
.PE
"#
    );
    assert_eq!(exit, 0);
    let (out, msgs, _) = run("pic", D, &[]).unwrap();
    assert_eq!(
        out,
        r#".PS
.ps 11
circle at 2.000,9.500 rad 0.250
dashwid = 0.050
box dashed fill 0.500 with .sw at (0.500,9.500) width 1.000 height 0.500
"\s12\fHa_b $x$\fP" at 1.000,8.533 
"\s10\fB\LaTeX\fP" at 1.000,8.028 ljust
.PE
"#
    );
    assert_eq!(msgs, vec!["Arc box not implemented; substituting box."]);
}

#[test]
fn options() {
    let (out, msgs, _) = run("pic", D, &[('p', Some("allps"))]).unwrap();
    assert_eq!(
        out,
        r#".PS
.ps 11
circle at 2.000,9.500 rad 0.250 thickness 1
dashwid = 0.050
box dashed fill 0.500 with .sw at (0.500,9.500) width 1.000 height 0.500 rad 0.087 thickness 2
.ps
.ps 12
.ft
.ft H
"a_b $x$" at 1.000,8.533 
.ft 
.ps 
.ps
.ps 10
.ft
.ft B
"\LaTeX" at 1.000,8.028 ljust
.ft 
.ps 
.PE
"#
    );
    assert!(msgs.is_empty());
    let (_, msgs, _) = run("pic", D, &[('f', Some("Q"))]).unwrap();
    assert_eq!(msgs[0], "warning: non-standard font name Q");
    assert_eq!(run("pic", D, &[('p', Some("xx"))]).unwrap_err(), "Invalid option: xx");
    assert_eq!(
        run("pic", D, &[('x', None)]).unwrap_err(),
        "Argument -x unknown to pic driver."
    );
}

#[test]
fn hostile_values_dont_panic() {
    let body = "2 4 7 1 0 7 50 -1 30 0.000 0 0 2147483647 0 0 5
	 -2147483648 0 2147483647 0 2147483647 9 0 9 0 0
4 9 0 50 -1 -9 -12 0.0000 4 135 450 1200 2400 x\\001
4 0 0 50 -1 99 1e300 0.0000 6 135 450 1200 2400 x\\001
";
    for opts in [&[][..], &[('m', Some("0")), ('p', Some("allps"))][..]] {
        let _ = run("pic", body, opts);
    }
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test pic -- --ignored --nocapture`.
#[test]
#[ignore]
fn same_as_fig2dev() {
    let mut extra = pic_common::FIGS.to_vec();
    extra.extend(HAND);
    let opts: &[&[&str]] = &[
        &[],
        &["-p", "all", "-f", "B"],
        &["-p", "allps", "-m", "0.5"],
        &["-p", "psfont", "-s", "9"],
    ];
    if let Some((same, total)) = compare_with_fig2dev("pic", &extra, opts) {
        assert!(same * 10 >= total * 9, "{same} of {total} identical");
    }
}
