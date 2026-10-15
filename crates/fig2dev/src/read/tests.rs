use super::*;

const TOP: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n";

fn read(src: &str) -> (Result<Compound, i32>, Vec<String>, Ctx) {
    let mut ctx = Ctx::default();
    let r = readfp_fig(&mut ctx, src.as_bytes());
    let m = ctx.messages.clone();
    (r, m, ctx)
}

#[test]
fn header_sets_globals() {
    let src = "#FIG 3.2\nPortrait\nFlush left\nMetric\nA4 \n50.00\nMultiple\n3\n# fig comment\n1200 2\n";
    let (r, m, ctx) = read(src);
    let c = r.unwrap();
    assert!(m.is_empty(), "{m:?}");
    assert!(!ctx.landscape && !ctx.center && ctx.metric && ctx.multi_page);
    assert_eq!(ctx.papersize, "A4");
    assert_eq!((ctx.mag, ctx.fontmag), (0.5, 0.5));
    assert_eq!(ctx.gif_transparent, "#00ffff");
    assert_eq!((ctx.ppi, ctx.thick_scale), (1200.0, 15.0));
    assert!(ctx.v2_flag && ctx.v21_flag && ctx.v30_flag && ctx.v32_flag);
    assert_eq!(c.comments, vec!["fig comment".to_string()]);
}

#[test]
fn options_override_the_file() {
    let mut ctx = Ctx {
        orientspec: true,
        magspec: true,
        paperspec: true,
        mag: 2.0,
        papersize: "B5".into(),
        ..Ctx::default()
    };
    readfp_fig(&mut ctx, TOP.as_bytes()).unwrap();
    assert!(!ctx.landscape);
    assert_eq!((ctx.mag, ctx.papersize.as_str()), (2.0, "B5"));
}

#[test]
fn user_colours_are_renumbered() {
    let src = format!("{TOP}0 32 #ff8000\n0 40 #102030\n2 1 0 1 40 32 50 -1 20 0.0 0 0 -1 0 0 2\n 0 0 600 600\n");
    let (r, m, ctx) = read(&src);
    let c = r.unwrap();
    assert!(m.is_empty(), "{m:?}");
    let l = &c.lines[0];
    assert_eq!((l.pen_color, l.fill_color), (33, 32));
    assert_eq!(ctx.user_col_indx, vec![32, 40]);
    assert_eq!(ctx.color_rgb(33), Some((0x10, 0x20, 0x30)));
}

#[test]
fn texts_decode_escapes_and_encodings() {
    let src = format!("{TOP}4 0 0 50 -1 0 12 0.0 4 150 405 0 0 a\\\\b\\344\\001x\\001\n");
    let (r, m, _) = read(&src);
    let c = r.unwrap_or_else(|e| panic!("{e} {m:?}"));
    assert_eq!(c.texts[0].cstring, "a\\bä");
    // UTF-8 in a UTF-8 file.
    let src = format!("{TOP}4 0 0 50 -1 0 12 0.0 4 150 405 0 0 Grüße\\001\n");
    assert_eq!(read(&src).0.unwrap().texts[0].cstring, "Grüße");
    // An escaped end mark does not end the string.
    let src = format!("{TOP}4 0 0 50 -1 0 12 0.0 4 150 405 0 0 a\\\\001b\\001\n");
    assert_eq!(read(&src).0.unwrap().texts[0].cstring, "a\\001b");
    // Latin-2 by -E.
    let mut ctx = Ctx {
        input_encoding: Some("ISO-8859-2".into()),
        ..Ctx::default()
    };
    let mut src = format!("{TOP}4 0 0 50 -1 0 12 0.0 4 150 405 0 0 ").into_bytes();
    src.extend_from_slice(b"\xb1\\001\n");
    assert_eq!(readfp_fig(&mut ctx, &src).unwrap().texts[0].cstring, "ą");
}

#[test]
fn multi_line_text_keeps_newlines() {
    let src = format!("{TOP}4 0 0 50 -1 0 12 0.0 4 150 405 0 0 one\ntwo\\001\n");
    let c = read(&src).0.unwrap();
    assert_eq!(c.texts[0].cstring, "one\ntwo");
}

#[test]
fn picture_names_follow_the_fig_file() {
    let src = format!("{TOP}2 5 0 1 0 -1 50 -1 -1 0.0 0 0 -1 0 0 5\n 1 line.eps\n 0 0 600 0 600 300 0 300 0 0\n");
    let mut ctx = Ctx {
        from: Some("dir/sub/a.fig".into()),
        ..Ctx::default()
    };
    let c = readfp_fig(&mut ctx, src.as_bytes()).unwrap();
    let pic = c.lines[0].pic.as_ref().unwrap();
    assert_eq!((pic.file.as_str(), pic.flipped), ("dir/sub/line.eps", 1));
    let c = read(&src).0.unwrap();
    assert_eq!(c.lines[0].pic.as_ref().unwrap().file, "line.eps");
}

#[test]
fn arcs_are_sanitized() {
    let src = format!("{TOP}5 1 0 1 0 7 50 -1 -1 0.0 0 0 0 0 0.0 0.0 600 0 0 600 300 0\n");
    let c = read(&src).0.unwrap();
    assert_eq!(c.arcs[0].point[2], Point::new(600, 0));
    let src = format!("{TOP}5 1 0 1 0 7 50 -1 -1 0.0 0 0 0 0 0.0 0.0 0 0 0 600 300 0\n");
    let (r, m, _) = read(&src);
    assert_eq!(r, Err(-1));
    assert_eq!(
        m,
        vec!["Invalid arc object at line 10: arc radius smaller than one Fig unit."]
    );
}

#[test]
fn compounds_nest_and_cap() {
    let src = format!("{TOP}6 0 0 100 100\n6 0 0 100 100\n1 3 0 1 0 7 50 -1 -1 0.0 1 0.0 0 0 60 60 0 0 60 0\n-6\n-6\n");
    let c = read(&src).0.unwrap();
    assert_eq!(c.compounds[0].compounds[0].ellipses.len(), 1);
    let deep = format!("{TOP}{}", "6 0 0 1 1\n".repeat(MAX_NESTING + 1));
    let (r, m, _) = read(&deep);
    assert_eq!(r, Err(-1));
    assert!(m[0].starts_with("Compound objects nested deeper"));
}

#[test]
fn old_splines_keep_controls() {
    let src = "#FIG 3.1\nLandscape\nCenter\nInches\n1200 2\n3 2 0 1 0 7 50 -1 -1 0.0 0 0 0 2\n 0 0 1200 0\n 1 2 3 4\n 5 6 7 8\n";
    let (r, m, ctx) = read(src);
    let c = r.unwrap();
    assert!(m.is_empty(), "{m:?}");
    assert_eq!(ctx.papersize, "Letter");
    assert_eq!(c.splines[0].controls.len(), 2);
    assert_eq!(c.splines[0].controls[1].rx, 7.0);
}
