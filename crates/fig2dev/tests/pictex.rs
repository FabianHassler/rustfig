#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `pictex` driver against output of the real fig2dev 3.2.9a.

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

const HEAD: &str = r"%Title: stdin
%%Created by: fig2dev Version 3.2.9a
\font\thinlinefont=cmr5
\mbox{\beginpicture
\setcoordinatesystem units <1.00000cm,1.00000cm>
\unitlength=1.00000cm
\linethickness=1pt
\setplotsymbol ({\makebox(0,0)[l]{\tencirc\symbol{'160}}})
\setshadesymbol ({\thinlinefont .})
\setlinear
";

#[test]
fn polyline_with_arrows() {
    let (out, msgs, exit) = run("pictex", POLYLINE, &[]).unwrap();
    let body = r"%
% Fig POLYLINE object
%
\linethickness= 0.500pt
\setplotsymbol ({\thinlinefont .})
{\color[rgb]{0,0,0}%
% arrow head
%
\plot  1.532 25.400  1.270 25.400  1.501 25.277 /
%
\plot  1.270 25.400  3.810 24.765 /
\plot  3.810 24.765  5.080 21.590 /
%
% arrow head
%
\plot  4.927 21.802  5.080 21.590  5.045 21.849 /
%
}%
\linethickness=0pt
\putrectangle corners at  1.245 25.425 and  5.105 21.565
\endpicture}
";
    assert_eq!(out, format!("{HEAD}{body}"));
    assert!(msgs.is_empty());
    assert_eq!(exit, 0);
}

#[test]
fn dashed_box() {
    let (out, _, _) = run("pictex", DASHED_BOX, &[]).unwrap();
    assert!(out.contains(
        r"\linethickness=1pt
\setplotsymbol ({\makebox(0,0)[l]{\tencirc\symbol{'160}}})
\setdashes < 0.1905cm>
{\color[rgb]{1,0,0}\color[rgb]{1,0,0}\putrectangle corners at  1.270 25.400 and  3.810 24.130
}%
\linethickness=0pt
\putrectangle corners at  1.223 25.447 and  3.857 24.083
"
    ));
}

#[test]
fn arc_and_filled_circle() {
    let (out, msgs, _) = run("pictex", ARC_AND_DOT, &[]).unwrap();
    assert!(out.contains(
        r"\plot  3.747 24.384  3.810 24.130  3.873 24.384 /
%
\circulararc 180.000 degrees from  1.270 24.130 center at  2.540 24.130
}%
"
    ));
    assert!(out.contains(r"{\color[rgb]{0,0,0}\put{\makebox(0,0)[l]{\circle*{ 0.635}}} at  5.080 24.130"));
    assert_eq!(
        msgs,
        vec!["Too big black filled circle substituted by a diameter of 15pt"]
    );
}

#[test]
fn texts_and_options() {
    let (out, msgs, _) = run("pictex", TEXTS, &[('l', Some("2pt")), ('p', Some("xx")), ('r', None)]).unwrap();
    assert!(out.contains("\\linethickness=2pt\n\\setplotsymbol ({xx})\n"));
    assert!(out.contains(
        r"\fontsize{12}{14.4}\normalfont\rmfamily {\color[rgb]{1,0,0}a\_b\ \$x\$}%
} [B] at  2.540 24.130
"
    ));
    assert!(out.contains(
        r"\fontsize{10}{12}\normalfont\bfseries {\color[rgb]{0,0,0}\LaTeX}%
} [lB] at  2.540 21.590
"
    ));
    assert!(msgs.is_empty());
    let (_, msgs, _) = run("pictex", TEXTS, &[]).unwrap();
    assert_eq!(msgs, vec!["Rotated Text only for left justified text"]);
    assert_eq!(
        run("pictex", TEXTS, &[('n', Some("name"))]).unwrap_err(),
        "Argument -n unknown to pictex driver."
    );
}

#[test]
fn hostile_values_dont_panic() {
    let body = "5 1 0 1 0 7 50 -1 -1 0.000 0 0 1 1 0.000 0.000 0 0 0 0 0 0
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 1
	 2147483647 -2147483648
";
    let _ = run("pictex", body, &[('m', Some("0"))]);
    let _ = run("pictex", body, &[]);
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test pictex -- --ignored --nocapture`.
#[test]
#[ignore]
fn same_as_fig2dev() {
    let mut extra = vec![POLYLINE, DASHED_BOX, ARC_AND_DOT, TEXTS];
    extra.extend(HAND);
    let opts: &[&[&str]] = &[
        &[],
        &["-l", "2pt", "-p", "xx", "-r", "-f", "tt"],
        &["-m", "0.5", "-s", "14"],
    ];
    if let Some((same, total)) = compare_with_fig2dev("pictex", &extra, opts) {
        assert!(same * 10 >= total * 9, "{same} of {total} identical");
    }
}
