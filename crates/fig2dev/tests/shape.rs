#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `shape` driver against output of the real fig2dev 3.2.9a.

use rustfig_fig2dev::{convert, drivers, read::readfp_fig, Ctx};

const HDR: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n";

/// Output (without the creation date), messages and exit status, or the
/// error that ends the run.
fn run(body: &str, opts: &[(char, Option<&str>)]) -> Result<(String, Vec<String>, i32), String> {
    let mut ctx = Ctx {
        lang: "shape".into(),
        ..Ctx::default()
    };
    let mut dev = drivers::driver("shape").unwrap();
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
fn single_box() {
    let (out, msgs, exit) = run(
        r#"1200 2
# +
2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5
	 0 0 1200 0 1200 600 0 600 0 0
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"\def\newshapepar#1{\shapepar{\newshapeshape}#1\par}
\def\newshapeshape{%
{25.4167}%
{0.416667}b{0.416667}\\%
{0.416667}t{0.416667}{50}\\%
{25.4167}t{0.416667}{50}\\%
{25.4167}e{50.4167}%
}
"#
    );
    let expected: Vec<&str> = vec![r#"*comm: +, comment->comment: +"#];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn shape_with_hole() {
    let (out, msgs, exit) = run(
        r#"1200 2
# +
2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5
	 0 0 1200 0 1200 1200 0 1200 0 0
# -
1 3 0 1 0 7 40 -1 -1 0.000 1 0.0000 600 600 300 300 600 600 900 600
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"\def\newshapepar#1{\shapepar{\newshapeshape}#1\par}
\def\newshapeshape{%
{25.4167}%
{0.416667}b{0.416667}\\%
{0.416667}t{0.416667}{50}\\%
{12.9167}t{0.416667}{26.6667}st{27.0833}{23.3333}\\%
{12.9167}t{0.416667}{23.3333}t{27.0833}{23.3333}\\%
{13.3333}t{0.416667}{21.6667}t{28.75}{21.6667}\\%
{13.75}t{0.416667}{20}t{30.4167}{20}\\%
{14.5833}t{0.416667}{18.3333}t{32.0833}{18.3333}\\%
{15.8333}t{0.416667}{17.0833}t{33.3333}{17.0833}\\%
{17.0833}t{0.416667}{15.8333}t{34.5833}{15.8333}\\%
{18.3333}t{0.416667}{14.5833}t{35.8333}{14.5833}\\%
{19.5833}t{0.416667}{13.75}t{36.6667}{13.75}\\%
{21.25}t{0.416667}{13.3333}t{37.0833}{13.3333}\\%
{22.9167}t{0.416667}{12.9167}t{37.5}{12.9167}\\%
{24.5833}t{0.416667}{12.5}t{37.9167}{12.5}\\%
{26.25}t{0.416667}{12.5}t{37.9167}{12.5}\\%
{27.9167}t{0.416667}{12.9167}t{37.5}{12.9167}\\%
{29.5833}t{0.416667}{13.3333}t{37.0833}{13.3333}\\%
{31.25}t{0.416667}{13.75}t{36.6667}{13.75}\\%
{32.5}t{0.416667}{14.5833}t{35.8333}{14.5833}\\%
{33.75}t{0.416667}{15.8333}t{34.5833}{15.8333}\\%
{35}t{0.416667}{17.0833}t{33.3333}{17.0833}\\%
{36.25}t{0.416667}{18.3333}t{32.0833}{18.3333}\\%
{37.0833}t{0.416667}{20}t{30.4167}{20}\\%
{37.5}t{0.416667}{21.6667}t{28.75}{21.6667}\\%
{37.9167}t{0.416667}{23.3333}t{27.0833}{23.3333}\\%
{37.9167}t{0.416667}{23.3333}jt{23.75}{26.6667}\\%
{50.4167}t{0.416667}{50}\\%
{50.4167}e{50.4167}%
}
"#
    );
    let expected: Vec<&str> = vec![r#"*comm: +, comment->comment: +"#, r#"*comm: -, comment->comment: -"#];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn groups() {
    let (out, msgs, exit) = run(
        r#"1200 2
# +a
2 3 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 4
	 0 0 1200 600 0 1200 0 0
# +b
2 3 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 4
	 600 0 1800 600 600 1200 600 0
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"\def\newshapepar#1{\shapepar{\newshapeshape}#1\par}
\def\newshapeshape{%
{25.2778}%
{0.277778}b{0.277778}\\%
{16.9444}t{0.277778}{33.3333}\\%
{33.6111}e{0.277778}\\%
{0.277778}b{16.9444}\\%
{16.9444}t{16.9444}{33.3333}\\%
{33.6111}e{16.9444}%
}
"#
    );
    let expected: Vec<&str> = vec![
        r#"*comm: +a, comment->comment: +a"#,
        r#"*comm: +b, comment->comment: +b"#,
    ];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn figure_comment_names_macros() {
    let (out, msgs, exit) = run(
        r#"# face
1200 2
# +
5 1 0 1 0 7 50 -1 -1 0.000 0 1 0 0 600.000 600.000 300 600 600 300 900 600
# +
5 2 0 1 0 7 50 -1 -1 0.000 0 0 0 0 600.000 1200.000 300 1200 600 900 900 1200
# +
2 4 0 1 0 7 50 -1 -1 0.000 0 0 7 0 0 5
	 0 1500 1200 1500 1200 2400 0 2400 0 1500
# +
1 1 0 1 0 7 50 -1 -1 0.000 1 0.3000 600 3000 500 250 600 3000 1100 3000
# width
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2
	 0 3600 2400 3600
# center
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2
	 0 3700 1300 3700
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"\def\facepar#1{\shapepar{\faceshape}#1\par}
\def\faceshape{%
{2.75}%
{1.29167}b{1.29167}\\%
{1.29167}t{1.29167}{2.5}\\%
{1.45833}t{1.29167}{2.5}\\%
{1.625}t{1.33333}{2.41667}\\%
{1.79167}t{1.375}{2.33333}\\%
{1.95833}t{1.45833}{2.16667}\\%
{2.08333}t{1.58333}{1.91667}\\%
{2.20833}t{1.70833}{1.66667}\\%
{2.33333}t{1.83333}{1.41667}\\%
{2.41667}t{1.95833}{1.16667}\\%
{2.45833}t{2.125}{0.833333}\\%
{2.5}t{2.29167}{0.5}\\%
{2.54167}t{2.45833}{0.166667}\\%
{2.58333}t{2.29167}{0.5}\\%
{2.625}t{2.125}{0.833333}\\%
{2.66667}t{1.95833}{1.16667}\\%
{2.75}t{1.83333}{1.41667}\\%
{2.875}t{1.70833}{1.66667}\\%
{3}t{1.58333}{1.91667}\\%
{3.125}t{1.45833}{2.16667}\\%
{3.29167}t{1.375}{2.33333}\\%
{3.45833}t{1.33333}{2.41667}\\%
{3.625}t{1.29167}{2.5}\\%
{3.79167}t{1.29167}{2.5}\\%
{3.79167}e{3.79167}\\%
{5.04167}b{0.375}\\%
{5.04167}t{0.375}{4.33333}\\%
{5.08333}t{0.333333}{4.41667}\\%
{5.08333}t{0.25}{4.58333}\\%
{5.125}t{0.208333}{4.66667}\\%
{5.16667}t{0.166667}{4.75}\\%
{5.20833}t{0.125}{4.83333}\\%
{5.25}t{0.0833333}{4.91667}\\%
{5.29167}t{0.0833333}{4.91667}\\%
{5.33333}t{0.0416667}{5}\\%
{5.41667}t{0.0416667}{5}\\%
{8.375}t{0.0416667}{5}\\%
{8.41667}t{0.0416667}{5}\\%
{8.5}t{0.0416667}{5}\\%
{8.54167}t{0.0833333}{4.91667}\\%
{8.58333}t{0.0833333}{4.91667}\\%
{8.625}t{0.125}{4.83333}\\%
{8.66667}t{0.166667}{4.75}\\%
{8.70833}t{0.208333}{4.66667}\\%
{8.75}t{0.25}{4.58333}\\%
{8.75}t{0.333333}{4.41667}\\%
{8.79167}t{0.375}{4.33333}\\%
{8.79167}e{4.70833}\\%
{10.125}b{3.04167}\\%
{10.125}t{3.04167}{0.5}\\%
{10.1667}t{2.79167}{1}\\%
{10.2083}t{2.5}{1.5}\\%
{10.2917}t{2.25}{1.91667}\\%
{10.375}t{1.95833}{2.375}\\%
{10.5}t{1.70833}{2.70833}\\%
{10.625}t{1.45833}{3.04167}\\%
{10.75}t{1.25}{3.29167}\\%
{10.875}t{1.09375}{3.44792}\\%
{10.9167}t{1.04167}{3.48958}\\%
{11.0417}t{0.875}{3.625}\\%
{11.2083}t{0.75}{3.70833}\\%
{11.375}t{0.625}{3.70833}\\%
{11.5417}t{0.583333}{3.625}\\%
{11.6667}t{0.552083}{3.48958}\\%
{11.7083}t{0.541667}{3.44792}\\%
{11.8333}t{0.541667}{3.29167}\\%
{11.9583}t{0.583333}{3.04167}\\%
{12.0833}t{0.666667}{2.70833}\\%
{12.2083}t{0.75}{2.375}\\%
{12.2917}t{0.916667}{1.91667}\\%
{12.375}t{1.08333}{1.5}\\%
{12.4167}t{1.29167}{1}\\%
{12.4583}t{1.54167}{0.5}\\%
{12.4583}e{2.04167}%
}
"#
    );
    let expected: Vec<&str> = vec![
        r#"*comm: +, comment->comment: +"#,
        r#"*comm: +, comment->comment: +"#,
        r#"*comm: +, comment->comment: +"#,
        r#"*comm: +, comment->comment: +"#,
        r#"*comm: width, comment->comment: width"#,
        r#"*comm: center, comment->comment: center"#,
    ];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn macro_name_option() {
    let (out, msgs, exit) = run(
        r#"# face
1200 2
# +
5 1 0 1 0 7 50 -1 -1 0.000 0 1 0 0 600.000 600.000 300 600 600 300 900 600
# +
5 2 0 1 0 7 50 -1 -1 0.000 0 0 0 0 600.000 1200.000 300 1200 600 900 900 1200
# +
2 4 0 1 0 7 50 -1 -1 0.000 0 0 7 0 0 5
	 0 1500 1200 1500 1200 2400 0 2400 0 1500
# +
1 1 0 1 0 7 50 -1 -1 0.000 1 0.3000 600 3000 500 250 600 3000 1100 3000
# width
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2
	 0 3600 2400 3600
# center
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2
	 0 3700 1300 3700
"#,
        &[('n', Some(r#"face"#))],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"\def\facepar#1{\shapepar{\faceshape}#1\par}
\def\faceshape{%
{2.75}%
{1.29167}b{1.29167}\\%
{1.29167}t{1.29167}{2.5}\\%
{1.45833}t{1.29167}{2.5}\\%
{1.625}t{1.33333}{2.41667}\\%
{1.79167}t{1.375}{2.33333}\\%
{1.95833}t{1.45833}{2.16667}\\%
{2.08333}t{1.58333}{1.91667}\\%
{2.20833}t{1.70833}{1.66667}\\%
{2.33333}t{1.83333}{1.41667}\\%
{2.41667}t{1.95833}{1.16667}\\%
{2.45833}t{2.125}{0.833333}\\%
{2.5}t{2.29167}{0.5}\\%
{2.54167}t{2.45833}{0.166667}\\%
{2.58333}t{2.29167}{0.5}\\%
{2.625}t{2.125}{0.833333}\\%
{2.66667}t{1.95833}{1.16667}\\%
{2.75}t{1.83333}{1.41667}\\%
{2.875}t{1.70833}{1.66667}\\%
{3}t{1.58333}{1.91667}\\%
{3.125}t{1.45833}{2.16667}\\%
{3.29167}t{1.375}{2.33333}\\%
{3.45833}t{1.33333}{2.41667}\\%
{3.625}t{1.29167}{2.5}\\%
{3.79167}t{1.29167}{2.5}\\%
{3.79167}e{3.79167}\\%
{5.04167}b{0.375}\\%
{5.04167}t{0.375}{4.33333}\\%
{5.08333}t{0.333333}{4.41667}\\%
{5.08333}t{0.25}{4.58333}\\%
{5.125}t{0.208333}{4.66667}\\%
{5.16667}t{0.166667}{4.75}\\%
{5.20833}t{0.125}{4.83333}\\%
{5.25}t{0.0833333}{4.91667}\\%
{5.29167}t{0.0833333}{4.91667}\\%
{5.33333}t{0.0416667}{5}\\%
{5.41667}t{0.0416667}{5}\\%
{8.375}t{0.0416667}{5}\\%
{8.41667}t{0.0416667}{5}\\%
{8.5}t{0.0416667}{5}\\%
{8.54167}t{0.0833333}{4.91667}\\%
{8.58333}t{0.0833333}{4.91667}\\%
{8.625}t{0.125}{4.83333}\\%
{8.66667}t{0.166667}{4.75}\\%
{8.70833}t{0.208333}{4.66667}\\%
{8.75}t{0.25}{4.58333}\\%
{8.75}t{0.333333}{4.41667}\\%
{8.79167}t{0.375}{4.33333}\\%
{8.79167}e{4.70833}\\%
{10.125}b{3.04167}\\%
{10.125}t{3.04167}{0.5}\\%
{10.1667}t{2.79167}{1}\\%
{10.2083}t{2.5}{1.5}\\%
{10.2917}t{2.25}{1.91667}\\%
{10.375}t{1.95833}{2.375}\\%
{10.5}t{1.70833}{2.70833}\\%
{10.625}t{1.45833}{3.04167}\\%
{10.75}t{1.25}{3.29167}\\%
{10.875}t{1.09375}{3.44792}\\%
{10.9167}t{1.04167}{3.48958}\\%
{11.0417}t{0.875}{3.625}\\%
{11.2083}t{0.75}{3.70833}\\%
{11.375}t{0.625}{3.70833}\\%
{11.5417}t{0.583333}{3.625}\\%
{11.6667}t{0.552083}{3.48958}\\%
{11.7083}t{0.541667}{3.44792}\\%
{11.8333}t{0.541667}{3.29167}\\%
{11.9583}t{0.583333}{3.04167}\\%
{12.0833}t{0.666667}{2.70833}\\%
{12.2083}t{0.75}{2.375}\\%
{12.2917}t{0.916667}{1.91667}\\%
{12.375}t{1.08333}{1.5}\\%
{12.4167}t{1.29167}{1}\\%
{12.4583}t{1.54167}{0.5}\\%
{12.4583}e{2.04167}%
}
"#
    );
    let expected: Vec<&str> = vec![
        r#"*comm: +, comment->comment: +"#,
        r#"*comm: +, comment->comment: +"#,
        r#"*comm: +, comment->comment: +"#,
        r#"*comm: +, comment->comment: +"#,
        r#"*comm: width, comment->comment: width"#,
        r#"*comm: center, comment->comment: center"#,
    ];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn star_with_hole() {
    let (out, msgs, exit) = run(
        r#"1200 2
# +
2 3 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 11
	 600 0 750 450 1200 450 840 720 990 1170 600 900 210 1170 360 720 0 450 450 450 600 0
# -
2 2 0 1 0 7 40 -1 -1 0.000 0 0 -1 0 0 5
	 500 500 700 500 700 700 500 700 500 500
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"\def\newshapepar#1{\shapepar{\newshapeshape}#1\par}
\def\newshapeshape{%
{28.5937}%
{0.46875}b{28.5937}\\%
{21.5625}t{21.5625}{14.0625}\\%
{21.5625}t{0.46875}{56.25}\\%
{23.9062}t{3.59375}{29.6875}st{33.2812}{20.3125}\\%
{23.9062}t{3.59375}{20.3125}t{33.2812}{20.3125}\\%
{33.2812}t{16.0937}{7.8125}t{33.2812}{7.8125}\\%
{33.2812}t{16.0937}{7.8125}jt{23.9062}{17.1875}\\%
{34.2187}t{17.3437}{22.5}\\%
{42.6562}t{14.5312}{14.0625}st{28.5937}{14.0625}\\%
{55.3125}e{10.3125}e{46.875}%
}
"#
    );
    let expected: Vec<&str> = vec![r#"*comm: +, comment->comment: +"#, r#"*comm: -, comment->comment: -"#];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn overlapping_shapes() {
    let (out, msgs, exit) = run(
        r#"1200 2
# +
2 3 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 6
	 0 0 1200 0 1200 600 600 300 0 600 0 0
# +
2 3 0 1 0 7 45 -1 -1 0.000 0 0 -1 0 0 4
	 300 200 900 200 600 900 300 200
# -
2 2 0 1 0 7 40 -1 -1 0.000 0 0 -1 0 0 5
	 100 100 300 100 300 300 100 300 100 100
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"\def\newshapepar#1{\shapepar{\newshapeshape}#1\par}
\def\newshapeshape{%
{25.4167}%
{0.416667}b{0.416667}\\%
{0.416667}t{0.416667}{50}\\%
{4.58333}t{0.416667}{12.5}st{12.9167}{37.5}\\%
{4.58333}t{0.416667}{4.16667}t{12.9167}{37.5}\\%
{8.75}t{0.416667}{4.16667}t{12.9167}{37.5}\\%
{12.9167}t{0.416667}{4.16667}t{12.9167}{37.5}\\%
{12.9167}t{0.416667}{4.16667}jt{4.58333}{45.8333}\\%
{17.3284}t{0.416667}{16.1765}st{16.5931}{17.6471}st{34.2402}{16.1765}\\%
{25.4167}e{0.416667}t{20.0595}{10.7143}e{50.4167}\\%
{37.9167}e{25.4167}%
}
"#
    );
    let expected: Vec<&str> = vec![
        r#"*comm: +, comment->comment: +"#,
        r#"*comm: +, comment->comment: +"#,
        r#"*comm: -, comment->comment: -"#,
    ];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn reference_height_and_text() {
    let (out, msgs, exit) = run(
        r#"1200 2
# +
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 4
	 0 0 1200 300 600 1200 0 0
# height
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2
	 2000 0 2000 2400
# width0
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2
	 2000 0 2000 2400
# heightx
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2
	 2000 0 2000 0
# +
4 0 0 50 -1 0 12 0.0000 4 135 405 300 300 Text\001
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"\def\newshapepar#1{\shapepar{\newshapeshape}#1\par}
\def\newshapeshape{%
{2.54167}%
{0.0416667}b{0.0416667}\\%
{1.29167}t{0.666667}{4.375}\\%
{5.04167}e{2.54167}%
}
"#
    );
    let expected: Vec<&str> = vec![
        r#"*comm: +, comment->comment: +"#,
        r#"*comm: height, comment->comment: height"#,
        r#"*comm: width0, comment->comment: width0"#,
        r#"fig2dev(shape) Warning: Width of reference object=0 - ignored "#,
        r#"*comm: heightx, comment->comment: heightx"#,
        r#"fig2dev(shape) Warning: Height of reference object=0 - ignored "#,
        r#"*comm: +, comment->comment: +"#,
        r#"fig2dev(shape): TEXT cannot be used as shape"#,
    ];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn long_group_and_blank_comment() {
    let (out, msgs, exit) = run(
        r#"1200 2
#  real comment
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2
	 0 0 1200 600
# +verylonggroupidentifier123 x
2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5
	 0 0 1200 0 1200 600 0 600 0 0
"#,
        &[],
    )
    .unwrap();
    assert_eq!(
        out,
        r#"\def\newshapepar#1{\shapepar{\newshapeshape}#1\par}
\def\newshapeshape{%
{25.4167}%
{0.416667}b{0.416667}\\%
{0.416667}t{0.416667}{50}\\%
{25.4167}t{0.416667}{50}\\%
{25.4167}e{50.4167}%
}
"#
    );
    let expected: Vec<&str> = vec![
        r#"*comm: , comment->comment:  real comment"#,
        r#"*comm: +verylonggroupidentifier123, comment->comment: +verylonggroupidentifier123 x"#,
    ];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

/// The messages and the error of a run that fails.
fn fails(body: &str, opts: &[(char, Option<&str>)]) -> (Vec<String>, String) {
    let mut ctx = Ctx {
        lang: "shape".into(),
        ..Ctx::default()
    };
    let mut dev = drivers::driver("shape").unwrap();
    for &(c, a) in opts {
        if !ctx.general_option(c, a).unwrap() {
            if let Err(e) = dev.option(&mut ctx, c, a) {
                return (ctx.messages, e);
            }
        }
    }
    let objects = readfp_fig(&mut ctx, format!("{HDR}{body}").as_bytes()).unwrap();
    let err = convert(&mut ctx, dev.as_mut(), &objects).unwrap_err();
    (ctx.messages, err)
}

#[test]
fn comment_that_is_no_shape_fails() {
    let (msgs, err) = fails(
        "1200 2\n# hello\n2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n\t 0 0 1200 600\n",
        &[],
    );
    assert_eq!(
        msgs,
        ["*comm: hello, comment->comment: hello", "Comment \"hello\": hello"]
    );
    assert_eq!(
        err,
        "fig2dev(shape): Illegal shape specification, must start with '+' or '-' or be 'center', 'width' or height'\nUse blank to start real comment. See documentation for explanation"
    );
}

#[test]
fn no_shape_fails() {
    let (msgs, err) = fails(
        "1200 2\n2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n\t 0 0 1200 600\n",
        &[],
    );
    assert!(msgs.is_empty());
    assert_eq!(
        err,
        "fig2dev(shape): No shape found - have you forgotten to set comments '+'?"
    );
}

#[test]
fn reference_on_an_ellipse_fails() {
    let body = "1200 2\n# width\n1 1 0 1 0 7 50 -1 -1 0.000 1 0.3000 600 3000 500 250 600 3000 1100 3000\n";
    let (msgs, err) = fails(body, &[]);
    assert_eq!(msgs, ["*comm: width, comment->comment: width"]);
    assert_eq!(err, "fig2dev(shape): Ill-formed shape comment - start with '+' or '-'");
}

#[test]
fn unknown_option_fails() {
    let (_, err) = fails("1200 2\n", &[('x', None)]);
    assert_eq!(err, "Argument -x unknown to shape driver.");
}

/// Opt-in: compare with the real fig2dev on random figures of boxes,
/// polygons, circles, ellipses, rounded boxes and arcs marked as shapes
/// and holes in up to three groups, each object at its own depth (fig2dev's
/// `qsort` orders objects sharing a depth differently from `convert`).
#[test]
#[ignore]
fn compare_with_real_fig2dev() {
    let dir = std::env::temp_dir().join(format!("rustfig-shape-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut seed: u64 = 1;
    let mut rnd = |n: u64| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) % n
    };
    let (mut same, mut checked) = (0, 0);
    let mut failed = Vec::new();
    for i in 0..300 {
        let mut body = String::from("1200 2\n");
        let n = 1 + rnd(5);
        for k in 0..n {
            let sign = if rnd(3) == 0 { '-' } else { '+' };
            let group = ["", "", "a", "b"][rnd(4) as usize];
            let depth = 30 + 5 * k;
            body.push_str(&format!("# {sign}{group}\n"));
            let p = |r: &mut dyn FnMut(u64) -> u64| r(25) * 100;
            match rnd(5) {
                0 => {
                    let (x0, y0, x1, y1) = (p(&mut rnd), p(&mut rnd), p(&mut rnd), p(&mut rnd));
                    body.push_str(&format!("2 2 0 1 0 7 {depth} -1 -1 0.000 0 0 -1 0 0 5\n\t {x0} {y0} {x1} {y0} {x1} {y1} {x0} {y1} {x0} {y0}\n"));
                }
                1 => {
                    let m = 3 + rnd(4);
                    let pts: Vec<(u64, u64)> = (0..m).map(|_| (p(&mut rnd), p(&mut rnd))).collect();
                    let mut s: Vec<String> = pts.iter().map(|(a, b)| format!("{a} {b}")).collect();
                    s.push(s[0].clone());
                    body.push_str(&format!(
                        "2 3 0 1 0 7 {depth} -1 -1 0.000 0 0 -1 0 0 {}\n\t {}\n",
                        m + 1,
                        s.join(" ")
                    ));
                }
                2 => {
                    let (cx, cy, r) = (p(&mut rnd), p(&mut rnd), 100 + rnd(700));
                    body.push_str(&format!(
                        "1 3 0 1 0 7 {depth} -1 -1 0.000 1 0.0000 {cx} {cy} {r} {r} {cx} {cy} {} {cy}\n",
                        cx + r
                    ));
                }
                3 => {
                    let (cx, cy, rx, ry) = (p(&mut rnd), p(&mut rnd), 100 + rnd(700), 100 + rnd(700));
                    body.push_str(&format!(
                        "1 1 0 1 0 7 {depth} -1 -1 0.000 1 0.3 {cx} {cy} {rx} {ry} {cx} {cy} {} {cy}\n",
                        cx + rx
                    ));
                }
                _ => {
                    let (x0, y0) = (p(&mut rnd), p(&mut rnd));
                    let (x1, y1, r) = (x0 + 200 + rnd(1000), y0 + 200 + rnd(1000), 2 + rnd(18));
                    body.push_str(&format!("2 4 0 1 0 7 {depth} -1 -1 0.000 0 0 {r} 0 0 5\n\t {x0} {y0} {x1} {y0} {x1} {y1} {x0} {y1} {x0} {y0}\n"));
                }
            }
        }
        let path = dir.join(format!("r{i}.fig"));
        std::fs::write(&path, format!("{HDR}{body}")).unwrap();
        let Ok(real) = std::process::Command::new("fig2dev")
            .args(["-L", "shape"])
            .arg(&path)
            .output()
        else {
            eprintln!("fig2dev is not installed; skipping");
            return;
        };
        checked += 1;
        let ours = run(&body, &[]);
        let ok = match ours {
            Ok((out, msgs, _)) => {
                real.status.success()
                    && out.as_bytes() == real.stdout.as_slice()
                    && msgs.join("\n") == String::from_utf8_lossy(&real.stderr).trim_end()
            }
            Err(_) => !real.status.success(),
        };
        if ok {
            same += 1;
        } else {
            failed.push(path.display().to_string());
        }
    }
    eprintln!("shape: {same} of {checked} identical; differ: {failed:#?}");
    if failed.is_empty() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    assert!(failed.is_empty());
}
