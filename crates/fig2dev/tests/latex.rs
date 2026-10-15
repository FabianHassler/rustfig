#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `latex` driver against output of the real fig2dev 3.2.9a.

mod tex_common;

use tex_common::{compare_with_fig2dev, run, HAND};

pub const POLYLINE: &str = "2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 1 1 3
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
	 600 600 1800 900 2400 2400
";
pub const DASHED_BOX: &str = "2 2 1 2 4 7 50 -1 -1 4.000 0 0 -1 0 0 5
	 600 600 1800 600 1800 1200 600 1200 600 600
";
pub const ARC_AND_DOT: &str = "5 1 0 1 1 7 50 -1 -1 0.000 0 1 1 0 1200.000 1200.000 600 1200 1200 600 1800 1200
	1 1 1.00 60.00 120.00
1 3 0 1 0 7 40 -1 20 0.000 1 0.0000 2400 1200 150 150 2400 1200 2550 1200
";
pub const TEXTS: &str = "4 1 4 50 -1 0 12 0.5236 4 135 450 1200 1200 a_b $x$\\001
4 0 0 40 -1 2 10 0.0000 2 135 450 1200 2400 \\\\LaTeX\\001
";

#[test]
fn polyline_with_arrows() {
    let (out, msgs, exit) = run("latex", POLYLINE, &[]).unwrap();
    assert_eq!(
        out,
        r"\setlength{\unitlength}{3947sp}%
\begin{picture}(1824,1824)(589,-1573)
\thinlines
{\color[rgb]{0,0,0}\put(1801,-61){\vector(-4, 1){1200}}
\put(1801,-61){\vector( 1,-3){510}}
}%
\end{picture}%
"
    );
    assert_eq!(msgs, vec!["Not a LaTeX slope (600, -1500), deviation 120.0 pixels"]);
    assert_eq!(exit, 0);
}

#[test]
fn dashed_box_and_options() {
    let (out, _, _) = run("latex", DASHED_BOX, &[]).unwrap();
    assert!(out.contains("\\thicklines\n{\\color[rgb]{1,0,0}\\put(601,-361){\\dashbox{60}(1200,600){}}\n}%\n"));
    let (out, _, _) = run(
        "latex",
        DASHED_BOX,
        &[('b', Some("10")), ('d', Some("2")), ('l', Some("1")), ('v', None)],
    )
    .unwrap();
    assert_eq!(
        out,
        r"\setlength{\unitlength}{3947sp}%
\begin{picture}(1576,976)(413,-549)
%
% Fig POLYLINE object
%
\thicklines
{\color[rgb]{1,0,0}\put(601,-361){\dashbox{120}(1200,600){}}
}%
\end{picture}%
"
    );
    let (_, msgs, _) = run("latex", DASHED_BOX, &[('a', None), ('f', Some("zz"))]).unwrap();
    assert_eq!(
        msgs,
        vec![
            "warning: latex option -a obsolete",
            "warning: non-standard font name zz ignored"
        ]
    );
    assert_eq!(
        run("latex", DASHED_BOX, &[('x', None)]).unwrap_err(),
        "Argument -x unknown to latex driver."
    );
}

#[test]
fn arc_and_filled_circle() {
    let (out, _, _) = run("latex", ARC_AND_DOT, &[]).unwrap();
    assert_eq!(
        out,
        r"\setlength{\unitlength}{3947sp}%
\begin{picture}(1967,1217)(592,-970)
\thinlines
{\color[rgb]{0,0,1}\put(1201,-361){\oval(1200,1200)[bl]}
\put(1201,-361){\oval(1200,1200)[br]}
\put(1801,-361){\vector(-1, 0){0}}
}%
{\color[rgb]{0,0,0}\put(2401,-361){\circle*{300}}
}%
\end{picture}%
"
    );
}

#[test]
fn texts() {
    let (out, _, _) = run("latex", TEXTS, &[]).unwrap();
    assert_eq!(
        out,
        r"\setlength{\unitlength}{3947sp}%
\begin{picture}(105,1324)(1131,-1576)
\put(1201,-361){\rotatebox{30.0}{\makebox(0,0)[b]{\smash{\fontsize{12}{14.4}\usefont{T1}{ptm}{m}{n}{\color[rgb]{1,0,0}a\_b\ \$x\$}%
}}}}
\put(1201,-1561){\makebox(0,0)[lb]{\smash{\fontsize{10}{12}\normalfont\bfseries {\color[rgb]{0,0,0}\LaTeX}%
}}}
\end{picture}%
"
    );
    let (out, _, _) = run("latex", TEXTS, &[('F', None), ('f', Some("bf"))]).unwrap();
    assert!(out.contains("\\fontsize{12}{14.4}\\selectfont {\\color[rgb]{1,0,0}a"));
    assert!(out.contains("\\fontsize{10}{12}\\selectfont {\\color[rgb]{0,0,0}\\LaTeX}"));
}

#[test]
fn hostile_values_dont_panic() {
    // Zero dash length and magnification, a zero-length arc, a huge box.
    let body = "2 1 1 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2
	 0 0 2147483647 -2147483648
2 2 2 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5
	 -2147483648 0 2147483647 0 2147483647 9 0 9 0 0
5 1 0 1 0 7 50 -1 -1 0.000 0 0 1 1 0.000 0.000 0 0 0 0 0 0
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
";
    for opts in [&[][..], &[('m', Some("0")), ('d', Some("0"))][..]] {
        let _ = run("latex", body, opts);
    }
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test latex -- --ignored --nocapture`.
#[test]
#[ignore]
fn same_as_fig2dev() {
    let mut extra = vec![POLYLINE, DASHED_BOX, ARC_AND_DOT, TEXTS];
    extra.extend(HAND);
    let opts: &[&[&str]] = &[&[], &["-b", "20", "-d", "2", "-l", "1", "-v"], &["-F", "-m", "0.5"]];
    if let Some((same, total)) = compare_with_fig2dev("latex", &extra, opts) {
        assert!(same * 10 >= total * 9, "{same} of {total} identical");
    }
}
