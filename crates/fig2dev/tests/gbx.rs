#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `gbx` driver against output of the real fig2dev 3.2.9a.

use rustfig_fig2dev::{convert, drivers, read::readfp_fig, Ctx};

const HDR: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n";

/// Output (without the creation date), messages and exit status, or the
/// error that ends the run.
fn run(body: &str, opts: &[(char, Option<&str>)]) -> Result<(String, Vec<String>, i32), String> {
    let mut ctx = Ctx {
        lang: "gbx".into(),
        ..Ctx::default()
    };
    let mut dev = drivers::driver("gbx").unwrap();
    for &(c, a) in opts {
        if !ctx.general_option(c, a)? {
            dev.option(&mut ctx, c, a)?;
        }
    }
    let objects = readfp_fig(&mut ctx, format!("{HDR}{body}").as_bytes()).unwrap();
    let status = convert(&mut ctx, dev.as_mut(), &objects)?;
    let out = String::from_utf8(status.output).unwrap();
    let out: String = out
        .split_inclusive('\n')
        .filter(|l| !l.starts_with("G04 Creation date"))
        .collect();
    Ok((out, ctx.messages, status.exit))
}

#[test]
fn butt_cap_polyline() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 1 0 2 0 7 50 -1 -1 0.000 0 0 -1 0 0 2
	 0 0 600 100
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11R,0.635000X0.635000*%
G54D11*
X31750Y4233D02*
X1238250Y207433D01*
M02*
"#
    );
    let expected: Vec<&str> = vec![];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn round_join_three_points() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 1 0 2 0 7 50 -1 -1 0.000 1 0 -1 0 0 3
	 0 0 600 0 600 600
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11R,0.635000X0.635000*%
G54D11*
X31750Y0D02*
X1238250Y0D01*
%ADD12C,0.635000%*
D02*
G54D12*
X1270000Y0D03*
G54D11*
X1270000Y31750D02*
X1270000Y1238250D01*
M02*
"#
    );
    let expected: Vec<&str> = vec![];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn round_join_square_cap_four_points() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 1 0 2 0 7 50 -1 -1 0.000 1 2 -1 0 0 4
	 0 0 600 0 600 600 1200 900
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11R,0.635000X0.635000*%
G54D11*
X0Y0D02*
X1238250Y0D01*
%ADD12C,0.635000%*
G54D12*
X1238250Y0D02*
X1270000Y0D01*
X1270000Y31750*
X1270000Y31750D02*
X1270000Y31750D01*
X1270000Y1270000*
G54D11*
X1297516Y1284816D02*
X2540000Y1905000D01*
M02*
"#
    );
    let expected: Vec<&str> = vec![];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn round_join_six_points_with_arrow() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 1 0 3 -1 7 50 -1 -1 0.000 1 0 -1 1 0 6
	1 1 0 1.00 60.00 120.00
	 0 0 600 0 600 600 1200 900 1500 300 2000 2000
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11R,0.952500X0.952500*%
G54D11*
X46566Y0D02*
X1223433Y0D01*
%ADD12C,0.952500%*
G54D12*
X1223433Y0D02*
X1270000Y0D01*
X1270000Y46566*
X1270000Y46566D02*
X1270000Y46566D01*
X1270000Y1270000*
X2540000Y1905000*
X3175000Y635000*
G54D11*
X3187700Y679450D02*
X4220633Y4188883D01*
M02*
"#
    );
    let expected: Vec<&str> = vec![r#"Warning: Arrows on lines are not yet implemented."#];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn round_caps_and_joins() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 1 0 2 0 7 50 -1 -1 0.000 1 1 -1 0 0 4
	 0 0 600 0 600 600 1200 900
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11C,0.635000%*
G54D11*
X0Y0D02*
X1270000Y0D01*
X1270000Y1270000*
X2540000Y1905000*
M02*
"#
    );
    let expected: Vec<&str> = vec![];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn box_with_round_joins() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 2 0 2 0 7 50 -1 -1 0.000 1 0 -1 0 0 5
	 0 0 600 0 600 600 0 600 0 0
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11R,0.635000X0.635000*%
G54D11*
X0Y0D02*
X1270000Y0D01*
%ADD12C,0.635000%*
G54D12*
X1270000Y0D02*
X1270000Y1270000D01*
X0Y1270000*
X0Y1270000D02*
G54D11*
X0Y1270000D02*
X0Y0D01*
M02*
"#
    );
    let expected: Vec<&str> = vec![];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn filled_polygon_with_edge() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 3 0 2 0 7 50 -1 20 0.000 0 0 -1 0 0 4
	 0 0 600 0 600 600 0 0
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11R,0.635000X0.635000*%
G54D11*
X0Y0D02*
X1270000Y0D01*
X1270000Y1270000*
X0Y0*
X0Y0D02*
G36*
X1270000Y0D01*
X1270000Y1270000*
X0Y0*
G37*
D02M02*
"#
    );
    let expected: Vec<&str> = vec![
        r#"Warning: You have specified a filled polygon with a finite edge thickness. Check if this is what you want"#,
        r#"Warning: Join style of square does not translate well to intermediate points in Gerber format."#,
    ];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn filled_polyline_with_edge() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 1 0 2 0 7 50 -1 20 0.000 1 0 -1 0 0 5
	 0 0 600 0 600 600 0 900 100 1000
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11R,0.635000X0.635000*%
G54D11*
X31750Y0D02*
X1238250Y0D01*
%ADD12C,0.635000%*
G54D12*
X1238250Y0D02*
X1270000Y0D01*
X1270000Y31750*
X1270000Y31750D02*
X1270000Y31750D01*
X1270000Y1270000*
X0Y1905000*
G54D11*
X23283Y1928283D02*
X188383Y2093383D01*
X31750Y0D02*
G36*
X1238250Y0D01*
X1270000Y0*
X1270000Y31750*
X1270000Y1270000*
X0Y1905000*
X23283Y1928283*
X188383Y2093383*
G37*
D02M02*
"#
    );
    let expected: Vec<&str> = vec![
        r#"Warning: You have specified a filled polygon with a finite edge thickness. Check if this is what you want"#,
        r#"Warning: A filled polygon is not closed. This can appear different in the Gerber file."#,
    ];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn polygon_with_four_points() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 3 0 1 0 7 50 0 -1 0 1 0 0 0 0 4
	0 0 0 600 600 600 0 0
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11R,0.317500X0.317500*%
G54D11*
X0Y0D02*
X0Y1270000D01*
%ADD12C,0.317500%*
G54D12*
X0Y1270000D02*
X1270000Y1270000D01*
X0Y0*
M02*
"#
    );
    let expected: Vec<&str> = vec![];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn single_point_flashes() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 1 0 2 0 7 50 -1 -1 0.000 1 0 -1 0 0 1
	 300 300
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11C,0.635000%*
D02*
G54D11*
X635000Y635000D03*
M02*
"#
    );
    let expected: Vec<&str> = vec![];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn colours_and_zero_width() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 1 0 2 4 7 50 -1 -1 0.000 1 0 -1 0 0 2
	 0 0 600 100
2 1 0 0 0 7 40 -1 -1 0.000 1 0 -1 0 0 2
	 0 0 600 100
5 1 0 2 0 7 30 -1 -1 0.000 0 1 0 0 600.000 600.000 300 600 600 300 900 600
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11R,0.635000X0.635000*%
G54D11*
X635000Y1270000D02*
G75*
G02*
X1905000Y1270000I635000J0D01*
G74*
G01*
D02M02*
"#
    );
    let expected: Vec<&str> = vec![
        r#"Warning: By default only objects with line colour of "default" or  "black" are output."#,
        r#"Warning: Unfilled line of zero width, ignoring."#,
    ];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn arcs() {
    let (out, msgs, exit) = run(
        r#"1200 2
5 1 0 2 0 7 50 -1 -1 0.000 1 0 0 0 600.000 600.000 300 600 600 300 900 600
5 2 0 2 0 7 40 -1 20 0.000 0 1 0 0 600.000 600.000 300 600 600 300 900 600
5 1 0 0 0 7 30 -1 20 0.000 0 1 0 0 600.000 600.000 300 600 600 300 900 600
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11C,0.635000%*
G54D11*
X635000Y1270000D02*
G75*
G03*
X1905000Y1270000I635000J0D01*
G74*
G01*
D02X635000Y1270000D02*
G75*
G36*
G02*
X1905000Y1270000I635000J0D01*
G37*
G74*
G01*
D02%ADD12R,0.635000X0.635000*%
G54D12*
X635000Y1270000D02*
G75*
G02*
X1905000Y1270000I635000J0D01*
G74*
G01*
D02X635000Y1270000D02*
X1905000Y1270000D01*
X635000Y1270000D02*
G75*
G36*
G02*
X1905000Y1270000I635000J0D01*
G37*
G74*
G01*
D02M02*
"#
    );
    let expected: Vec<&str> =
        vec![r#"Warning: Polygon fill on an arc with finite line thickness.  Check this is what you want."#];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn circles_ellipses_texts() {
    let (out, msgs, exit) = run(
        r#"1200 2
1 3 0 1 0 7 50 -1 -1 0.000 1 0.0000 1200 1200 300 300 1200 1200 1500 1200
1 3 0 2 -1 7 40 -1 20 0.000 1 0.0000 2400 1200 300 300 2400 1200 2700 1200
1 1 0 2 -1 7 30 -1 20 0.000 1 0.0000 2400 1200 300 200 2400 1200 2700 1200
4 0 0 20 -1 0 12 0.0000 4 135 405 300 300 Text\001
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11R,0.317500X0.317500*%
G54D11*
X2988733Y2988733D02*
G75*
G03*
X2988733Y2988733I-448733J-448733D01*
G74*
G01*
D02X5528733Y2988733D02*
G75*
G36*
G03*
X5528733Y2988733I-448733J-448733D01*
G37*
G74*
G01*
D02%ADD12R,0.635000X0.635000*%
G54D12*
X5528733Y2988733D02*
G75*
G03*
X5528733Y2988733I-448733J-448733D01*
G74*
G01*
D02X5528733Y2988733D02*
X5528733Y2988733D01*
M02*
"#
    );
    let expected: Vec<&str> = vec![
        r#"Warning: Polygon fill on an arc with finite line thickness.  Check this is what you want."#,
        r#"Warning: Ellipse output is currently unsupported."#,
        r#"Warning: Text output is currently unsupported."#,
    ];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn figure_comments() {
    let (out, msgs, exit) = run(
        r#"# a figure comment
1200 2
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2
	 0 0 600 100
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
G04 a figure comment*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11R,0.317500X0.317500*%
G54D11*
X14816Y2116D02*
X1255183Y209550D01*
M02*
"#
    );
    let expected: Vec<&str> = vec![];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn options() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 1 0 2 0 7 50 -1 -1 0.000 1 0 -1 0 0 3
	 0 0 600 0 600 600
"#,
        &[
            ('d', Some(r#"in"#)),
            ('p', Some(r#"pos"#)),
            ('g', Some(r#"1x-1+0.5+0"#)),
            ('f', Some(r#"2.4"#)),
            ('v', None),
        ],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOIN*%
%OFA0.500000B0.000000*%
%FSLAX24Y24*%
%SFA1.000000B-1.000000%
%IPPOS*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
G04 ## START:POLYGON:OPEN*
%ADD11R,0.025000X0.025000*%
G54D11*
X125Y0D02*
X4875Y0D01*
%ADD12C,0.025000%*
D02*
G54D12*
X5000Y0D03*
G54D11*
X5000Y125D02*
X5000Y4875D01*
G04 ## END:POLYGON:OPEN*
M02*
"#
    );
    let expected: Vec<&str> = vec![];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn bad_option_values() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 1 0 2 0 7 50 -1 -1 0.000 0 0 -1 0 0 2
	 0 0 600 100
"#,
        &[
            ('d', Some(r#"ft"#)),
            ('p', Some(r#"foo"#)),
            ('g', Some(r#"x"#)),
            ('f', Some(r#"-1.7"#)),
            ('i', Some(r#"foo"#)),
        ],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX42949672957Y42949672957*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11R,0.635000X0.635000*%
G54D11*
X3175000Y423333D02*
X123825000Y20743333D01*
M02*
"#
    );
    let expected: Vec<&str> = vec![
        r#"Dimensions should be given in 'mm' for millimeters or 'in' for inches, respectively"#,
        r#"Polarity option should be [+,pos] for positive images (marks add material making it opaque) or [-,neg] for negative images (marks remove material, making it transparent)"#,
        r#"Error in scaling format, expect <double>x<double>+<double>+<double> for X and Y scale and x, y offset, respectively."#,
        r#"Warning: Having more than 4 digits before the decimal place is unusual"#,
        r#"Warning: Having more than 5 digits after the decimal place is unusual"#,
        r#"Warning: Option -i obsolete, use -v (verbose) instead."#,
        r#"Error: Debug comments option should be 'on' or 'off'"#,
    ];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn obsolete_i_option() {
    let (out, msgs, exit) = run(
        r#"1200 2
5 2 0 2 0 7 40 -1 20 0.000 0 1 0 0 600.000 600.000 300 600 600 300 900 600
"#,
        &[('i', Some(r#"on"#))],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
G04 ## START:ARC:FILLED*
X635000Y1270000D02*
G75*
G36*
G02*
X1905000Y1270000I635000J0D01*
G37*
G74*
G01*
D02G04 ## START:ARC:OUTLINE:CURVE*
G04 ## START:ARC:OPEN*
%ADD11R,0.635000X0.635000*%
G54D11*
X635000Y1270000D02*
G75*
G02*
X1905000Y1270000I635000J0D01*
G74*
G01*
D02G04 ## END:ARC:OPEN*
G04 ## START:ARC:OUTLINE:LINE*
X635000Y1270000D02*
X1905000Y1270000D01*
G04 ## END:ARC:OUTLINE:LINE*
G04 ## END:ARC:OUTLINE:CURVE*
G04 ## END:ARC:OPEN*
M02*
"#
    );
    let expected: Vec<&str> = vec![
        r#"Warning: Option -i obsolete, use -v (verbose) instead."#,
        r#"Warning: Polygon fill on an arc with finite line thickness.  Check this is what you want."#,
    ];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn magnification() {
    let (out, msgs, exit) = run(
        r#"1200 2
2 1 0 2 0 7 50 -1 -1 0.000 0 2 -1 0 0 2
	 0 0 600 100
"#,
        &[('m', Some(r#"2"#))],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"G04 Gerber RS-274x file*
G04 Creator: fig2dev*
G04 Version: 3.2.9a*
G04 Driver version: 0.1.1*
G04 Author: Edward Grace <edward.grace@gmail.com>*
%MOMM*%
%OFA0.000000B0.000000*%
%FSLAX35Y35*%
%SFA1.000000B1.000000%
%IPNEG*%
%LPD*%
G04 The following is an aperture definition of width pi/10. It should never be used.*
%ADD10R,0.314159X0.314159*%
%ADD11R,1.270000X1.270000*%
G54D11*
X0Y0D02*
X2540000Y423333D01*
M02*
"#
    );
    let expected: Vec<&str> = vec![];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn unknown_option_fails() {
    let err = run("1200 2\n", &[('x', None)]).unwrap_err();
    assert_eq!(err, "Argument -x unknown to gbx driver.");
}

#[test]
fn language_and_grid_are_ignored() {
    let body = "1200 2\n2 1 0 2 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n\t 0 0 600 100\n";
    let plain = run(body, &[]).unwrap();
    assert_eq!(run(body, &[('L', Some("gbx")), ('G', Some("1cm"))]).unwrap(), plain);
}

/// Every object of `c` at its own depth: fig2dev's unstable `qsort` of the
/// depths can't reorder anything.
fn depths_unique(c: &rustfig_fig2dev::object::Compound, seen: &mut Vec<i32>) -> bool {
    let depths = c
        .arcs
        .iter()
        .map(|o| o.depth)
        .chain(c.ellipses.iter().map(|o| o.depth))
        .chain(c.lines.iter().map(|o| o.depth))
        .chain(c.splines.iter().map(|o| o.depth))
        .chain(c.texts.iter().map(|o| o.depth));
    for d in depths {
        if seen.contains(&d) {
            return false;
        }
        seen.push(d);
    }
    c.compounds.iter().all(|k| depths_unique(k, seen))
}

/// The real fig2dev's output for `path` (without the creation date) and
/// its stderr, or `None` when fig2dev isn't installed.
pub fn real_fig2dev(lang: &str, path: &std::path::Path) -> Option<(String, String)> {
    let out = std::process::Command::new("fig2dev")
        .args(["-L", lang])
        .arg(path)
        .env("SOURCE_DATE_EPOCH", "0")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let text = text
        .split_inclusive('\n')
        .filter(|l| !l.starts_with("G04 Creation date"))
        .collect();
    Some((text, String::from_utf8_lossy(&out.stderr).into_owned()))
}

/// Opt-in: compare with the real fig2dev over fig2dev's test figures and
/// every 10th library figure of xfig. Figures whose objects share depths
/// are only reported: fig2dev's `qsort` orders them differently from the
/// stable sort in `convert`.
#[test]
#[ignore]
fn compare_with_real_fig2dev() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../old");
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(root.join("fig2dev-3.2.9a/fig2dev/tests/data"))
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    let mut lib = Vec::new();
    let mut stack = vec![root.join("xfig-3.2.9a/Libraries")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                lib.push(p);
            }
        }
    }
    lib.sort();
    files.extend(lib.into_iter().step_by(10));
    files.retain(|p| p.extension().is_some_and(|e| e == "fig"));
    if files.is_empty() || real_fig2dev("gbx", &files[0]).is_none() {
        eprintln!("fig2dev or the sample figures are missing; skipping");
        return;
    }
    let (mut same, mut reordered, mut failed) = (0, 0, Vec::new());
    for f in &files {
        let (want, _) = real_fig2dev("gbx", f).unwrap();
        let mut ctx = Ctx {
            lang: "gbx".into(),
            from: Some(f.display().to_string()),
            ..Ctx::default()
        };
        let Ok(objects) = readfp_fig(&mut ctx, &std::fs::read(f).unwrap()) else {
            continue;
        };
        let mut dev = drivers::driver("gbx").unwrap();
        let got = match convert(&mut ctx, dev.as_mut(), &objects) {
            Ok(s) => String::from_utf8(s.output).unwrap(),
            Err(_) => String::new(),
        };
        let got: String = got
            .split_inclusive('\n')
            .filter(|l| !l.starts_with("G04 Creation date"))
            .collect();
        if got == want {
            same += 1;
        } else if !depths_unique(&objects, &mut Vec::new()) {
            reordered += 1;
        } else {
            failed.push(f.display().to_string());
        }
    }
    eprintln!(
        "gbx: {same} identical, {reordered} differ with objects sharing depths, {} differ: {failed:#?}",
        failed.len()
    );
    assert!(failed.is_empty());
}
