#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `textyl` driver against output of the real fig2dev 3.2.9a.

mod tex_common;

use tex_common::{compare_with_fig2dev, run, HAND};

const POLYLINE: &str = "2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 1 1 3
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
	 600 600 1800 900 2400 2400
";
const DASHED_BOX: &str = "2 2 1 2 4 7 50 -1 -1 4.000 0 0 -1 0 0 5
	 600 600 1800 600 1800 1200 600 1200 600 600
";
const ARC_AND_DOT: &str = "5 1 0 1 1 7 50 -1 -1 0.000 0 1 1 0 1200.000 1200.000 600 1200 1200 600 1800 1200
	1 1 1.00 60.00 120.00
1 3 0 1 0 7 40 -1 20 0.000 1 0.0000 2400 1200 150 150 2400 1200 2550 1200
";
const TEXTS: &str = "4 1 4 50 -1 0 12 0.5236 4 135 450 1200 1200 a_b $x$\\001
4 0 0 40 -1 2 10 0.0000 2 135 450 1200 2400 \\\\LaTeX\\001
";

#[test]
fn polyline_with_arrows() {
    let (out, msgs, exit) = run("textyl", POLYLINE, &[]).unwrap();
    assert_eq!(
        out,
        r"\begintyl{9519936.307200sp}
%
% Fig POLYLINE object
%
%
% arrow head
%
\special{tyl line S 2 2798912 6922049; 2368143 7151792}
\special{tyl line S 2 2368143 7151792; 2856348 7151792}
\special{tyl line S 2 L 0 2368143 7151792; 7104430 5967721}
\special{tyl line S 2 L 0 7104430 5967721; 9472573 47362}
%
% arrow head
%
\special{tyl line S 2 9406610 531091; 9472573 47362}
\special{tyl line S 2 9472573 47362; 9186734 443140}
\endtyl
"
    );
    assert!(msgs.is_empty());
    assert_eq!(exit, 0);
}

#[test]
fn dashed_box_and_options() {
    let (out, _, _) = run("textyl", DASHED_BOX, &[]).unwrap();
    assert!(out.contains("\\special{tyl line S 2 L 2 7104430 86831; 2368143 86831}\n"));
    let (out, _, _) = run("textyl", DASHED_BOX, &[('l', Some("5"))]).unwrap();
    assert!(out.contains("\\special{tyl line S 5 L 2 2368143 2454975; 7104430 2454975}\n"));
    assert_eq!(
        run("textyl", DASHED_BOX, &[('l', Some("13"))]).unwrap_err(),
        "Line thickness must be between 1 and 12."
    );
    assert_eq!(
        run("textyl", DASHED_BOX, &[('x', None)]).unwrap_err(),
        "Argument -x unknown to textyl driver."
    );
}

#[test]
fn arc_and_circle() {
    let (out, _, _) = run("textyl", ARC_AND_DOT, &[]).unwrap();
    assert_eq!(
        out,
        r"\begintyl{7139952.230400sp}
%
% arrow head
%
\special{tyl line S 2 7222836 2877294; 7104430 2403665}
\special{tyl line S 2 7104430 2403665; 6986022 2877294}
\special{tyl arc S 2 L 0 2368143 @ 4736286,2403665 180 0}
%
% Fig ELLIPSE
%
\special{tyl arc S 2 L 0 592035 @ 9472573,2403665 0 360}
\endtyl
"
    );
}

#[test]
fn texts() {
    let (out, msgs, _) = run("textyl", TEXTS, &[]).unwrap();
    assert!(out.contains("\\special{tyl label S 1 4736286 4795490 \"a_b $x$\"}\n"));
    assert!(out.contains("\\special{tyl label S 1 4736286 59203 \"\\LaTeX\"}\n"));
    assert_eq!(msgs, vec!["Warning: Text incorrectly positioned"]);
}

#[test]
fn hostile_values_dont_panic() {
    let body = "5 1 0 1 0 7 50 -1 -1 0.000 0 0 1 1 0.000 0.000 0 0 0 0 0 0
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
1 1 0 1 0 7 50 -1 -1 0.000 1 0.0000 0 0 2147483647 0 0 0 0 0
";
    let _ = run("textyl", body, &[]);
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test textyl -- --ignored --nocapture`.
#[test]
#[ignore]
fn same_as_fig2dev() {
    let mut extra = vec![POLYLINE, DASHED_BOX, ARC_AND_DOT, TEXTS];
    extra.extend(HAND);
    let opts: &[&[&str]] = &[&[], &["-l", "7", "-a", "-f", "it"], &["-m", "2"]];
    if let Some((same, total)) = compare_with_fig2dev("textyl", &extra, opts) {
        assert!(same * 10 >= total * 9, "{same} of {total} identical");
    }
}
