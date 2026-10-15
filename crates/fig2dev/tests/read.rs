#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's reader: the cases of fig2dev's `tests/read.at` that concern
//! the reader (messages and statuses), every Fig version, the sample
//! figures under `old/` (skipped without them), a fuzz loop, and (when
//! fig2dev is installed and `RUSTFIG_FIG2DEV_DIFF=1`) a comparison of the
//! messages with fig2dev's own.

use std::path::{Path, PathBuf};

use rustfig_fig2dev::object::*;
use rustfig_fig2dev::read::readfp_fig;
use rustfig_fig2dev::Ctx;

const TOP: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n";

fn read_bytes(src: &[u8]) -> (Result<Compound, i32>, Vec<String>, Ctx) {
    let mut ctx = Ctx::default();
    let r = readfp_fig(&mut ctx, src);
    let m = ctx.messages.clone();
    (r, m, ctx)
}

fn read(src: &str) -> (Result<Compound, i32>, Vec<String>, Ctx) {
    read_bytes(src.as_bytes())
}

/// The file reads (status 0) with exactly these messages.
fn ok_with(src: &str, msgs: &[&str]) -> Compound {
    let (r, m, _) = read(src);
    assert_eq!(m, msgs, "messages for {src:?}");
    r.unwrap_or_else(|e| panic!("status {e} for {src:?}"))
}

/// The file fails (fig2dev exits 1) with exactly these messages.
fn fails_with(src: &str, msgs: &[&str]) {
    let (r, m, _) = read(src);
    assert_eq!(m, msgs, "messages for {src:?}");
    assert!(r.is_err(), "{src:?} should fail");
}

fn top(body: &str) -> String {
    format!("{TOP}{body}")
}

#[test]
fn coincident_points() {
    let c = ok_with(
        &top("2 2 0 1 -1 -1 50 -1 -1 0.000 0 0 -1 0 0 8\n\t0 10200 1200 10200 1200 10200 1200 9225\n\t1200 9225 0 9225 0 10200 0 10200\n"),
        &[],
    );
    assert_eq!(c.lines[0].points.len(), 5);
    assert_eq!(c.lines[0].type_, T_BOX);
}

#[test]
fn open_boxes_and_polygons_are_closed() {
    let c = ok_with(
        &top("2 2 0 1 -1 -1 50 -1 -1 0.000 0 0 -1 0 0 4\n\t 0 10200 1200 10200 1200 9225 0 9225\n"),
        &["An open rectangle at line 11 - close it."],
    );
    assert_eq!(c.lines[0].points.len(), 5);
    assert_eq!(c.lines[0].points[0], c.lines[0].points[4]);
    ok_with(
        &top("2 4 0 1 -1 -1 50 -1 -1 0.000 0 0 15 0 0 4\n\t 0 10200 1200 10200 1200 9225 0 9225\n"),
        &["An open rectangle with rounded corners at line 11 - close it."],
    );
    ok_with(
        &top("2 3 0 1 -1 -1 50 -1 -1 0.000 0 0 15 0 0 5\n\t 0 10200 600 9825 1200 10200 1200 9225 0 9225\n"),
        &["An open polygon at line 11 - close it."],
    );
}

#[test]
fn malformed_boxes_are_converted() {
    let c = ok_with(
        &top("2 2 0 1 -1 -1 50 -1 -1 0.000 0 0 -1 0 0 4\n\t 0 10200 1200 9225 0 9225 0 10200\n"),
        &["A rectangle with 3 corners at line 11 - convert to a polygon."],
    );
    assert_eq!(c.lines[0].type_, T_POLYGON);
    ok_with(
        &top("2 2 0 1 -1 -1 50 -1 -1 0.000 0 0 -1 0 0 3\n\t 0 10200 1200 9225 0 9225\n"),
        &[
            "An open rectangle at line 11 - close it.",
            "A rectangle with 3 corners at line 11 - convert to a polygon.",
        ],
    );
    let c = ok_with(
        &top("2 2 0 1 -1 -1 50 -1 -1 0.0 0 0 -1 0 0 1\n\t 0 600\n"),
        &["A rectangle with 1 points at line 11 - convert to a polyline."],
    );
    assert_eq!((c.lines[0].type_, c.lines[0].fill_style), (T_POLYLINE, UNFILLED));
    let c = ok_with(
        &top("2 2 0 1 -1 -1 50 -1 -1 0.0 1 0 -1 0 0 2\n\t 0 0 0 600\n"),
        &[
            "An open rectangle at line 11 - close it.",
            "A rectangle with 3 points at line 11 - convert to a polyline.",
        ],
    );
    assert_eq!(c.lines[0].points, vec![Point::new(0, 600), Point::new(0, 0)]);
    ok_with(
        &top("2 2 0 1 -1 -1 50 -1 -1 0.0 1 0 -1 0 0 3\n\t 0 0 0 600 600 0\n"),
        &[
            "An open rectangle at line 11 - close it.",
            "A rectangle with 3 corners at line 11 - convert to a polygon.",
        ],
    );
    ok_with(
        &top("2 3 0 1 -1 -1 50 -1 -1 0.0 0 0 -1 0 0 1\n\t0 0\n"),
        &["A polygon with 1 points at line 11 - convert to a polyline."],
    );
}

#[test]
fn malformed_arc_boxes_fail() {
    fails_with(
        &top("2 4 0 1 -1 -1 50 -1 -1 0.000 0 0 60 0 0 4\n\t 0 10200 1200 9225 0 9225 0 10200\n"),
        &["A malformed rectangle with rounded corners at line 11 - it has 3 corners."],
    );
    fails_with(
        &top("2 4 0 1 -1 -1 50 -1 -1 0.000 0 0 60 0 0 5\n\t 0 10200 1210 10200 1200 9225 0 9225 0 10200\n"),
        &["A distorted or inclined rectangle with rounded corners at line 11."],
    );
    fails_with(
        &top("2 4 0 1 -1 -1 50 -1 -1 0.000 0 0 60 0 0 5\n\t 0 10200 1200 10210 1200 9225 0 9225 0 10200\n"),
        &["A distorted or inclined rectangle with rounded corners at line 11."],
    );
}

#[test]
fn arrows_on_single_points_are_removed() {
    let c = ok_with(
        &top("2 1 0 1 -1 -1 50 -1 -1 0. 0 0 0 1 1 1\n\t0 0 2. 120. 240.\n\t0 0 2. 120. 240.\n\t0 0\n"),
        &[
            "A single point with a forward arrow - remove the arrow.",
            "A single point with a backward arrow - remove the arrow.",
        ],
    );
    assert!(c.lines[0].for_arrow.is_none() && c.lines[0].back_arrow.is_none());
    ok_with(
        &top("2 3 0 1 -1 -1 50 -1 -1 0. 0 0 0 0 1 1\n\t0 0 2. 120. 240.\n\t0 0\n"),
        &[
            "A polygon with 1 points at line 12 - convert to a polyline.",
            "A single point with a backward arrow - remove the arrow.",
        ],
    );
}

#[test]
fn invalid_arrows_and_objects() {
    let c = ok_with(
        &top("2 1 0 1 -1 -1 50 -1 -1 0. 0 0 0 1 0 2\n\t10000000000000 0 1 60 120\n0 0 600 0\n"),
        &["Invalid arrow at line 11."],
    );
    assert!(c.lines[0].for_arrow.is_none());
    fails_with(
        &top("4 0 0 1 0 -99999 12 0 4 135 360 0 0 A\\001\n"),
        &["Invalid text object at line 10."],
    );
    let c = ok_with(
        &top("1 2 0 1 0 7 50 -1 -1 0.0 1 0.0 0 0 -1200 -600 -1200 600 1200 -600\n"),
        &[],
    );
    assert_eq!(c.ellipses[0].radiuses, Point::new(1200, 600));
    fails_with(
        &top("5 1 0 15 0 7 50 0 -1 0.0 1 0 0 0 0.0 1e99 0 0 600 600 1200 1200\n"),
        &["Invalid arc object at line 10."],
    );
    fails_with(
        &top("3 0 0 0 0 0 0 0 123456789 0 0-0-0 00\n"),
        &["Invalid spline object at line 10."],
    );
    fails_with(
        &top("4 0 0 50 0 -1 12 9e26 0 150 405 0 0 Very slanted text\\001\n"),
        &["Invalid text object at line 10."],
    );
    fails_with(
        &top("2 3 0 0 0 7 50 -1 63 0.000 0 0 -1 0 0 4\n\t 0 0 1200 0 600 800 0 0\n"),
        &["Invalid line object at line 10."],
    );
}

#[test]
fn debian_bugs_in_format_1_3() {
    fails_with(
        "\n22-222\n\n4\n0+4 71111111iNf7-3  3 3 0\u{ed}tyn\n",
        &["Invalid text object."],
    );
    fails_with(
        "\n 1  1\n\n1\n\n11 4-51\n\n11 0 5\n1\n91\n1\nc!!!!\n",
        &["Invalid text object."],
    );
    let big = format!(
        "7{}\n\n\n0\n1\n16 3\n\n16 6\n\n0\n\n16 6\n\n0\n1\n1 6\n1=6\n",
        "1".repeat(1100)
    );
    assert!(read(&big).0.is_err());
    let big = format!("\n222{}22213222\n22222\n2\n", "3".repeat(1500));
    assert!(read(&big).0.is_err());
    fails_with("2 4 7 0 4 0 -116 0 4 9 3585 6974 3 2\n", &["Invalid text object."]);
}

#[test]
fn too_many_comment_lines_are_ignored() {
    let src = format!(
        "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.\nSingle\n-2\n{}1200 2\n2 1 0 1 -1 -1 50 -1 -1 0.0 0 0 -1 0 0 2\n        0 0 100 100\n",
        "#\n".repeat(102)
    );
    let c = ok_with(&src, &[]);
    assert_eq!(c.comments.len(), 100);
}

#[test]
fn incomplete_objects() {
    fails_with(
        "#FIG 2\n1200 2\n3 0 0 0 0 0 0 0 0. 0 1\n0\n",
        &["Incomplete spline object at line 4."],
    );
    fails_with("#FIG 2\n0", &["Incomplete resolution information at line 2."]);
    fails_with(
        "#FIG 2\n1200 2\n2 1 1 1 -1 50 0 0 0. 0 0\n0\n",
        &["Incomplete line object at line 4."],
    );
    fails_with(
        &top("2 5 0 1 0 -1 50 0 0 0. 0 0 0 0 0 5\n0 img\n0\n"),
        &["Incomplete line object at line 12."],
    );
    fails_with(
        &top("3 2 0 0 0 0 50 0 0 0. 0 0 0 0\n0 0 0\n"),
        &["Incomplete spline object at line 11."],
    );
    fails_with(
        "1200 2 9 9\n3 10 0 1 1 0 0 0 0 0 0\n9999 0\n",
        &["spline with a single point"],
    );
    fails_with(
        "#FIG 2\n1200 2\n3 1 2 0 1 0 0 7 0 0 0\n600 0 9999 0\n",
        &["Incomplete spline object at line 4."],
    );
    fails_with(
        &top("# some comment\n3 4 0 1 0 0 0 0 -1 0.0 0 0 0 4\n  1200 600 1200 600 1200 600 1200 600\n 0 1 1 0\n"),
        &["Invalid spline with only co-incident points at or before line 13."],
    );
}

#[test]
fn splines_of_fig_3_2_become_lines() {
    let c = ok_with(
        &top("3 1 0 1 0 0 50 0 0 0. 0 0 0 3\n0 0 600 0 0 600\n1.0 1.0 1.0\n"),
        &[],
    );
    assert!(c.splines.is_empty());
    assert_eq!(c.lines.len(), 1);
    assert_eq!(c.lines[0].type_, T_POLYGON);
    assert!(c.lines[0].points.len() > 3);
}

#[test]
fn invalid_colour_numbers_become_default() {
    let c = ok_with(
        "#FIG 2\n1200 2\n2 1 1 1 -502350 50 0 0 0. 0 0\n0 0 100 100 9999 0\n",
        &["Invalid color number -502350 at line 3, using default color."],
    );
    assert_eq!(c.lines[0].pen_color, DEFAULT);
    let c = ok_with(
        &top("2 1 0 1 40 -1 50 -1 -1 0.0 0 0 -1 0 0 2\n 0 0 100 100\n"),
        &["Cannot locate user color 40, using default color at line 10."],
    );
    assert_eq!(c.lines[0].pen_color, DEFAULT);
}

#[test]
fn colour_definitions() {
    let (r, m, ctx) = read(&top(
        "0 32 #ff0000\n0 7 #000000\n0 33 #1g0000\n0 34 #-10000\n2 1 0 1 32 7 50 -1 -1 0.0 0 0 -1 0 0 2\n 0 0 1 1\n0 35 #000000\n",
    ));
    assert_eq!(r, Err(-1));
    assert_eq!(
        m,
        vec![
            "User color number at line 11 out of range (7), should be between 32 and 543.",
            "Invalid color definition at line 12: 0 33 #1g0000.",
            "Invalid color definition at line 13: 0 34 #-10000, color values must be between 0 through 255.",
            "Color definitions must come before other objects (line 16).",
        ]
    );
    assert_eq!(ctx.user_colors.len(), 1);
    // The transparent colour may be a user colour.
    let src = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n40\n1200 2\n0 40 #0a0b0c\n";
    let (r, _, ctx) = read(src);
    r.unwrap();
    assert_eq!(ctx.gif_transparent, "# a b c");
}

#[test]
fn magnification_and_text() {
    let src = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n42000.0\nSingle\n-2\n1200 2\n2 1 0 1 0 7 50 -1 -1 0.0 0 0 -1 1 0 2\n\t5 0 1.0 60.0 120.0\n\t 0 0 600 0\n";
    let (r, m, ctx) = read(src);
    assert!(m.is_empty());
    assert_eq!(r.unwrap().lines[0].for_arrow.unwrap().thickness, 15.0);
    assert_eq!(ctx.mag, 420.0);
    assert!(ctx.arrows_used && ctx.arrow_used[10]);
    let c = ok_with("#FIG 2\n1200 2\n4 2 0 0 1 0 0 0 6 110 376 639 5 Text\u{1}\u{1}\n", &[]);
    let t = &c.texts[0];
    assert_eq!((t.cstring.as_str(), t.size), ("Text", 11.0));
    assert!(t.special_text());
}

#[test]
fn paper_size_and_header_errors() {
    fails_with(
        "#FIG 3.2\nLandscape\nCenter\nInches\nPapersize_name_too_long\n",
        &["Invalid paper size specification at line 5: Papersize_name_too_long"],
    );
    fails_with(
        "#FIG 3.2\nLandscape\n",
        &["File is truncated at metric/inches or centering specification."],
    );
    fails_with("#FIG 3.2\n", &["Could not read input file."]);
    fails_with("#FIG 3.2", &["ASCII NUL ('\\0') character within the first line."]);
    fails_with(
        "#FIG 4.0\n",
        &[
            "Fig file format (4.0",
            ") newer than this version of fig2dev (3.2.9a), exiting",
        ],
    );
    fails_with(
        "#FIG 1.0\n",
        &["Cannot determine fig file format from string '1.0", "'."],
    );
    fails_with("#FIX 3.2\n", &["Incorrect format string in first line of input file."]);
    fails_with("#FIG 2\n0.5 2\n", &["Invalid resolution information (0.5) at line 2."]);
    assert_eq!(read("").0, Err(-2));
    let c = ok_with(&top("4 0 0 50 -1 -1 12 0.0 0 150 405 0 0 Text\\001\n"), &[]);
    assert_eq!(c.texts[0].font, -1);
}

#[test]
fn ascii_nul_is_rejected() {
    let mut src = top("4 0 0 50 -1 -1 12 0.0 0 150 405 0 0 A ").into_bytes();
    src.extend_from_slice(b"\0 B\\001\n");
    let (r, m, _) = read_bytes(&src);
    assert_eq!(r, Err(-1));
    assert_eq!(m, vec!["ASCII NUL ('\\0') in line 10."]);
    let data = old().join("fig2dev-3.2.9a/fig2dev/tests/data/text_w_ascii0.fig");
    if let Ok(src) = std::fs::read(data) {
        let (r, m, _) = read_bytes(&src);
        assert_eq!(r, Err(-1));
        assert_eq!(m, vec!["ASCII NUL ('\\0') in line 11."]);
    }
}

#[test]
fn files_ending_without_newline() {
    let c = ok_with(&top("4 0 0 50 0 -1 12 0 0 150 405 0 0 No end-of-line here -->"), &[]);
    assert_eq!(c.texts[0].cstring, "No end-of-line here -->\n");
    let c = ok_with(
        &top("4 0 0 50 0 -1 12 0 0 150 405 0 0 Start string\nNo end-of-line after one backslash --> \\"),
        &[],
    );
    assert_eq!(
        c.texts[0].cstring,
        "Start string\nNo end-of-line after one backslash --> \\"
    );
    let long = top(&format!(
        "4 0 0 50 0 -1 12 0 0 150 405 0 0 500chars {}",
        "H".repeat(500)
    ));
    ok_with(&long, &[]);
}

#[test]
fn huge_spline_controls_are_rejected() {
    fails_with(
        "#FIG 3.1\nLandscape\nCenter\nMetric\n1200 2\n3 2 0 1 0 7 50 -1 -1 0.0 0 0 0 2\n\t 0 0 1200 0\n\t 600 600 600 9e999\n\t 600 600 600 600\n",
        &["Spline control points out of range at line 8."],
    );
}

#[test]
fn picture_path_from_the_fig_file() {
    let data = old().join("fig2dev-3.2.9a/fig2dev/tests/data/boxwimg.fig");
    let Ok(src) = std::fs::read(&data) else {
        return;
    };
    let mut ctx = Ctx {
        from: Some(data.to_string_lossy().into_owned()),
        ..Ctx::default()
    };
    let c = readfp_fig(&mut ctx, &src).unwrap();
    let pic = c.lines.iter().find_map(|l| l.pic.as_ref()).unwrap();
    assert_eq!(Path::new(&pic.file), data.with_file_name("line.eps"));
}

#[test]
fn every_version() {
    // 1.3
    let c = ok_with(
        "80 2 600 400\n2 6 0 1 0.0 1 0 8 4 10 10 50 10 50 50 9999 9999\n1 3 0 1 0.0 1 40 40 20 20 0 0 0 0\n5 1 0 1 0.0 1 0 0 0 0 20.0 20.0 30 20 20 30 10 20\n4 0 12 0 10 30 5 5 hello world\n6 0 0 100 100\n3 11 0 1 0.0 0 0 0 0 1 1 9 9 5 5 9999 9999\n-6\n",
        &[],
    );
    assert_eq!(c.nwcorner, Point::new(80, 2));
    assert_eq!(c.lines[0].points.len(), 3);
    assert!(c.lines[0].for_arrow.is_some());
    assert_eq!(c.ellipses[0].type_, T_CIRCLE_BY_RAD);
    assert_eq!(c.arcs[0].type_, T_OPEN_ARC);
    assert_eq!(c.texts[0].cstring, "hello world");
    assert_eq!(c.compounds[0].splines[0].type_, T_CLOSED_APPROX);
    // 1.4
    let c = ok_with(
        "#FIG 1.4\n80 2\n2 1 0 1 -1 0 0 2 0.0 0 0\n10 10 50 10 50 50 9999 9999\n",
        &[],
    );
    assert_eq!(c.lines[0].fill_style, 15); // 20 − (2 − 1)·5
                                           // 2.0 and 2.1
    let (r, m, ctx) = read("#FIG 2.0\n80 2\n4 0 0 12 0 -1 0 0.0 0 10 30 5 5 hi\u{1}\n");
    assert!(m.is_empty(), "{m:?}");
    assert!(ctx.v2_flag && !ctx.v21_flag && !ctx.v30_flag);
    assert!(r.unwrap().texts[0].special_text());
    let (r, _, ctx) = read("#FIG 2.1\n80 2\n2 4 0 1 -1 0 0 0 0.0 7 0 0\n0 0 50 0 50 50 0 50 0 0 9999 9999\n");
    let c = r.unwrap();
    assert!(ctx.v21_flag && !ctx.v30_flag);
    assert_eq!(c.lines[0].radius, 7);
    // 3.0 with a comment and a polyline
    let (r, _, ctx) = read(
        "#FIG 3.0\nPortrait\nCenter\nMetric\n1200 2\n# c1\n2 1 0 1 -1 7 0 0 -1 0.000 0 0 -1 0 0 2\n\t 0 0 600 600\n",
    );
    let c = r.unwrap();
    assert!(ctx.v30_flag && !ctx.v32_flag && ctx.metric);
    assert_eq!(ctx.papersize, "A4");
    assert_eq!(c.lines[0].comments, vec!["c1".to_string()]);
    // 3.2 with the encoding line
    let (r, _, ctx) = read(
        "#FIG 3.2\n#encoding: UTF-8\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n4 0 0 50 -1 0 12 0.0 4 150 405 0 0 \u{e4}\\001\n",
    );
    assert_eq!(ctx.input_encoding.as_deref(), Some("UTF-8"));
    assert_eq!(r.unwrap().texts[0].cstring, "\u{e4}");
}

#[test]
fn latin1_files_decode_as_latin1() {
    let mut src = top("4 0 0 50 -1 0 12 0.0 4 150 405 0 0 ").into_bytes();
    src.extend_from_slice(b"\xe4\xf6\\001\n");
    let (r, _, _) = read_bytes(&src);
    assert_eq!(r.unwrap().texts[0].cstring, "\u{e4}\u{f6}");
}

/// A deterministic xorshift generator (no external crates).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

#[test]
fn fuzzed_input_never_panics() {
    let seeds: Vec<Vec<u8>> = vec![
        top("2 2 0 1 -1 -1 50 -1 -1 0.000 0 0 -1 1 1 5\n 1 1 2. 60 120\n 1 1 2. 60 120\n 0 0 1200 0 1200 600 0 600 0 0\n").into_bytes(),
        top("0 32 #ff8000\n6 0 0 100 100\n5 1 0 1 32 7 50 -1 20 0.0 0 1 1 0 300.0 300.0 0 300 300 0 600 300\n 1 1 1.0 60.0 120.0\n-6\n").into_bytes(),
        top("3 5 0 1 0 0 50 0 0 0. 0 1 0 4\n 1 1 2. 60 120\n0 0 600 0 600 600 0 600\n0 1 -1 1\n").into_bytes(),
        top("4 1 0 50 -1 0 12 0.3 4 150 405 0 0 a\\\\b\\344\\001\n1 1 0 1 0 7 50 -1 -1 0.0 1 0.0 0 0 60 60 0 0 60 0\n").into_bytes(),
        b"#FIG 3.1\nLandscape\nCenter\nInches\n1200 2\n3 3 0 1 0 7 50 -1 -1 0.0 0 0 0 3\n 0 0 1200 0 0 600\n 1 2 3 4\n 1 2 3 4\n 1 2 3 4\n".to_vec(),
        b"#FIG 2.1\n80 2\n2 5 0 1 -1 0 0 0 0.0 7 0 0\n0 0 50 0 50 50 0 50 0 0 9999 9999\n".to_vec(),
        b"80 2 600 400\n2 6 0 1 0.0 1 0 8 4 10 10 50 10 50 50 9999 9999\n6 0 0 1 1\n4 0 12 0 10 30 5 5 hi\n-6\n".to_vec(),
    ];
    let alphabet: &[u8] = b"0123456789 -+.\n\t#\\\x01eEinf9999-6\r\0\xff";
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for round in 0..6000 {
        let mut s = seeds[round % seeds.len()].clone();
        for _ in 0..1 + rng.below(8) {
            let pos = rng.below(s.len() + 1);
            match rng.below(4) {
                0 if pos < s.len() => {
                    s.remove(pos);
                }
                1 => s.insert(pos, alphabet[rng.below(alphabet.len())]),
                2 if pos < s.len() => s[pos] = alphabet[rng.below(alphabet.len())],
                _ => {
                    let n = rng.below(20);
                    let end = (pos + n).min(s.len());
                    let chunk: Vec<u8> = s[pos..end].to_vec();
                    s.splice(pos..pos, chunk);
                }
            }
        }
        let mut ctx = Ctx::default();
        let _ = readfp_fig(&mut ctx, &s);
        // Random bytes too.
        let n = rng.below(200);
        let junk: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        let _ = readfp_fig(&mut Ctx::default(), &junk);
    }
}

fn old() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../old")
}

fn fig_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            fig_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "fig") {
            out.push(p);
        }
    }
}

#[test]
fn sample_figures_read() {
    let mut files = Vec::new();
    fig_files(&old(), &mut files);
    if files.is_empty() {
        eprintln!("old/ not found: skipped");
        return;
    }
    let mut bad = Vec::new();
    for f in &files {
        if f.ends_with("text_w_ascii0.fig") {
            continue; // rejected by fig2dev too
        }
        let src = std::fs::read(f).unwrap();
        let mut ctx = Ctx {
            from: Some(f.to_string_lossy().into_owned()),
            ..Ctx::default()
        };
        if let Err(e) = readfp_fig(&mut ctx, &src) {
            bad.push(format!("{}: {e} {:?}", f.display(), ctx.messages));
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} failed:\n{}",
        bad.len(),
        files.len(),
        bad.join("\n")
    );
}

/// With `RUSTFIG_FIG2DEV_DIFF=1` and fig2dev installed: the reader's
/// messages and status agree with fig2dev's (`-L box`) on the sample
/// figures and on mutated inputs.
#[test]
fn messages_match_fig2dev() {
    if std::env::var_os("RUSTFIG_FIG2DEV_DIFF").is_none() {
        return;
    }
    use std::io::Write;
    use std::process::{Command, Stdio};
    let run = |src: &[u8]| -> Option<(bool, Vec<String>)> {
        let mut child = Command::new("fig2dev")
            .args(["-L", "box"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .ok()?;
        child.stdin.take()?.write_all(src).ok()?;
        let out = child.wait_with_output().ok()?;
        let err = String::from_utf8_lossy(&out.stderr).into_owned();
        Some((out.status.success(), err.lines().map(str::to_string).collect()))
    };
    let mut rng = Rng(42);
    let seed = top("2 2 0 1 -1 -1 50 -1 -1 0.000 0 0 -1 1 0 5\n 1 1 2. 60 120\n 0 0 1200 0 1200 600 0 600 0 0\n3 5 0 1 0 0 50 0 0 0. 0 0 0 4\n0 0 600 0 600 600 0 600\n0 1 -1 1\n");
    let mut mismatches = Vec::new();
    for _ in 0..500 {
        let mut s = seed.clone().into_bytes();
        let pos = rng.below(s.len());
        s[pos] = b"0123456789 -\n"[rng.below(13)];
        let Some((ok, theirs)) = run(&s) else {
            return;
        };
        let mut ctx = Ctx::default();
        let r = readfp_fig(&mut ctx, &s);
        let ours = ctx.messages.clone();
        let prefix = theirs.get(..ours.len().min(theirs.len())).unwrap_or_default();
        if prefix != ours.as_slice() || (r.is_err() && ok) {
            mismatches.push(format!(
                "{:?}\n ours {r:?} {ours:?}\n fig2dev {ok} {theirs:?}",
                String::from_utf8_lossy(&s)
            ));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
