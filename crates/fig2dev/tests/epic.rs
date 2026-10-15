#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `epic`, `eepic` and `eepicemu` drivers against output of the
//! real fig2dev 3.2.9a.

mod pic_common;
mod tex_common;

use tex_common::{compare_with_fig2dev, run, HAND};

const A: &str = r#"2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 1 1 3
	1 1 1.00 60.00 120.00
	0 0 1.00 60.00 120.00
	 600 600 1800 900 2400 2400
"#;
const B: &str = r#"2 3 0 1 0 7 50 -1 5 0.000 0 0 -1 0 0 4
	 600 600 1800 600 1200 1500 600 600
1 1 1 1 0 7 50 -1 20 4.000 1 0.0000 3000 1200 600 300 3000 1200 3600 1200
5 1 0 1 0 7 50 -1 -1 0.000 0 1 1 0 1200.000 3000.000 600 3000 1200 2400 1800 3000
	1 1 1.00 60.00 120.00
"#;
const C: &str = r#"2 1 3 1 0 7 50 -1 -1 3.000 0 0 -1 0 0 2
	 600 600 1800 900
2 1 0 1 0 7 40 -1 -1 0.000 0 0 -1 0 0 2
	 600 1200 1800 1200
"#;
const D: &str = r#"2 4 1 2 0 7 50 -1 10 4.000 0 0 7 0 0 5
	 600 600 1800 600 1800 1200 600 1200 600 600
1 3 0 1 0 7 50 -1 -1 0.000 1 0.0000 2400 1200 300 300 2400 1200 2700 1200
4 1 0 50 -1 16 12 0.0000 4 135 450 1200 2400 a_b $x$\001
4 0 0 50 -1 2 10 0.0000 2 135 450 1200 3000 \\LaTeX\001
"#;

#[test]
fn polyline_with_arrows() {
    let (out, msgs, exit) = run("eepic", A, &[]).unwrap();
    assert_eq!(
        out,
        r#"\setlength{\unitlength}{0.00083333in}
{\renewcommand{\dashlinestretch}{30}
\begin{picture}(1824,1839)(0,-10)
\path(135.693,1812.000)(12.000,1812.000)(121.141,1753.791)
\path(12,1812)(1212,1512)(1812,12)
\blacken\path(1739.579,112.275)(1812.000,12.000)(1795.287,134.559)(1739.579,112.275)
\end{picture}
}
"#
    );
    assert!(msgs.is_empty());
    assert_eq!(exit, 0);
    // epic draws with \drawline: the driver name reaches the driver
    // (fig2dev's test "pass driver name to driver, ticket #139").
    let (out, _, _) = run("epic", A, &[]).unwrap();
    assert_eq!(
        out,
        r#"\setlength{\unitlength}{0.00083333in}
{\renewcommand{\dashlinestretch}{30}
\begin{picture}(1824,1839)(0,-10)
\drawline(135.693,1812.000)(12.000,1812.000)(121.141,1753.791)
\drawline(12,1812)(1212,1512)(1812,12)
\blacken\drawline(1739.579,112.275)(1812.000,12.000)(1795.287,134.559)(1739.579,112.275)
\end{picture}
}
"#
    );
}

#[test]
fn fills_ellipse_and_arc() {
    let (out, _, _) = run("eepic", B, &[]).unwrap();
    assert_eq!(
        out,
        r#"\setlength{\unitlength}{0.00083333in}
{\renewcommand{\dashlinestretch}{30}
\begin{picture}(3020,3036)(0,-10)
\put(612.000,609.000){\arc{1200.000}{6.2832}{9.4248}}
\blacken\path(1230.047,486.630)(1212.000,609.000)(1170.338,492.534)(1230.047,486.630)
\put(2412,2409){\whiten\ellipse{1200}{600}}
\put(2412,2409){\ellipse{1200}{600}}
\texture{aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa 
	aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa 
	aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa 
	aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa }
\shade\path(12,3009)(1212,3009)(612,2109)(12,3009)
\path(12,3009)(1212,3009)(612,2109)(12,3009)
\end{picture}
}
"#
    );
    let (out, _, _) = run("epic", B, &[]).unwrap();
    assert_eq!(
        out,
        r#"\setlength{\unitlength}{0.00083333in}
{\renewcommand{\dashlinestretch}{30}
\begin{picture}(3020,3036)(0,-10)
\drawline(12.000,609.000)(16.731,533.800)(30.850,459.786)
	(54.134,388.125)(86.216,319.948)(126.590,256.329)
	(174.619,198.272)(229.546,146.692)(290.504,102.403)
	(356.532,66.104)(426.590,38.366)(499.571,19.628)
	(574.326,10.184)(649.674,10.184)(724.429,19.628)
	(797.410,38.366)(867.468,66.104)(933.496,102.403)
	(994.454,146.692)(1049.381,198.272)(1097.410,256.329)
	(1137.784,319.948)(1169.866,388.125)(1193.150,459.786)
	(1207.269,533.800)(1212.000,609.000)
\blacken\drawline(1230.047,486.630)(1212.000,609.000)(1170.338,492.534)(1230.047,486.630)
\put(2412,2409){\oval(1200,600)}
\texture{aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa 
	aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa 
	aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa 
	aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa aaffffff ffaaaaaa }
\drawline(12,3009)(1212,3009)(612,2109)(12,3009)
\drawline(12,3009)(1212,3009)(612,2109)(12,3009)
\end{picture}
}
"#
    );
}

#[test]
fn dash_dot_ends_the_output() {
    let (out, msgs, exit) = run("eepic", C, &[]).unwrap();
    assert_eq!(
        out,
        r#"\setlength{\unitlength}{0.00083333in}
{\renewcommand{\dashlinestretch}{30}
\begin{picture}(1224,639)(0,-10)
"#
    );
    assert_eq!(
        msgs,
        vec!["Only solid, dashed, and dotted line styles supported by epic(eepic)"]
    );
    assert_eq!(exit, 1);
}

#[test]
fn options() {
    let (out, msgs, _) = run("eepic", D, &[('v', None), ('P', None), ('W', None)]).unwrap();
    assert_eq!(
        out,
        r#"\documentclass{minimal}
\usepackage{epic,eepic}\pagestyle{empty}
\begin{document}\begin{center}
%
% Language in use is EEpic
%
\setlength{\unitlength}{0.00083333in}
{\renewcommand{\dashlinestretch}{30}
\begin{picture}(2130,2452)(0,-10)
\allinethickness{1.000pt}%
\put(1822,1815){\ellipse{600}{600}}
\allinethickness{2.000pt}%
%
% A box
%
\texture{44555555 55aaaaaa aa555555 55aaaaaa aa555555 55aaaaaa aa555555 55aaaaaa 
	aa555555 55aaaaaa aa555555 55aaaaaa aa555555 55aaaaaa aa555555 55aaaaaa 
	aa555555 55aaaaaa aa555555 55aaaaaa aa555555 55aaaaaa aa555555 55aaaaaa 
	aa555555 55aaaaaa aa555555 55aaaaaa aa555555 55aaaaaa aa555555 55aaaaaa }
\dashline{60.000}(22,2415)(1222,2415)(1222,1815)
	(22,1815)(22,2415)
\put(622,615){\makebox(0,0)[b]{\smash{\fontsize{12}{14.4}\usefont{T1}{phv}{m}{n}a\_b\ \$x\$}}}
\put(622,15){\makebox(0,0)[lb]{\smash{\fontsize{10}{12}\normalfont\bfseries \LaTeX}}}
\end{picture}
}
\end{center}
\end{document}
"#
    );
    assert_eq!(msgs, vec!["Arc box not implemented; substituting box."]);
    let (_, msgs, _) = run("eepic", D, &[('a', None), ('f', Some("zz"))]).unwrap();
    assert_eq!(
        &msgs[..2],
        [
            "warning: genepic option -a obsolete",
            "warning: non-standard font name zz ignored"
        ]
    );
    assert_eq!(
        run("eepic", D, &[('S', Some("13"))]).unwrap_err(),
        "Scale must be between 8 and 12 inclusively"
    );
    assert_eq!(
        run("epic", D, &[('x', None)]).unwrap_err(),
        "Argument -x unknown to epic driver."
    );
}

#[test]
fn hostile_values_dont_panic() {
    // Huge coordinates, a zero-radius arc, zero magnification and arrows.
    let body = "2 1 1 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2
	 0 0 2147483647 -2147483648
2 4 0 1 0 7 50 -1 -1 0.000 0 0 2147483647 0 0 5
	 -2147483648 0 2147483647 0 2147483647 9 0 9 0 0
5 1 0 1 0 7 50 -1 -1 0.000 0 0 1 1 0.000 0.000 0 0 0 0 0 0
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
1 3 0 1 0 7 50 -1 -1 0.000 1 0.0000 0 0 2147483647 2147483647 0 0 1 1
";
    for lang in ["epic", "eepic", "eepicemu"] {
        for opts in [&[][..], &[('m', Some("0")), ('d', Some("0")), ('W', None)][..]] {
            let _ = run(lang, body, opts);
        }
    }
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test epic -- --ignored --nocapture`
/// (every library figure with `RUSTFIG_TEX_ALL=1`).
#[test]
#[ignore]
fn same_as_fig2dev() {
    let mut extra = pic_common::FIGS.to_vec();
    extra.extend(HAND);
    let opts: &[&[&str]] = &[
        &[],
        &["-P", "-v", "-R", "1"],
        &["-W", "-l", "2", "-t", "0", "-d", "2", "-F"],
        &["-S", "10", "-m", "0.5", "-f", "bf", "-a"],
    ];
    for lang in ["eepic", "epic", "eepicemu"] {
        if let Some((same, total)) = compare_with_fig2dev(lang, &extra, opts) {
            assert!(same * 10 >= total * 9, "{lang}: {same} of {total} identical");
        }
    }
}
