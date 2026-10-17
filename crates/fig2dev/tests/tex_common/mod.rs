//! Shared by the latex, pictex and textyl tests: running a driver on a
//! Fig text, and comparing with the real fig2dev 3.2.9a when installed.
//!
//! `gendev::convert` orders objects of one depth with the macOS C
//! library's `qsort`, as the reference fig2dev does.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use rustfig_fig2dev::{convert, drivers, read::readfp_fig, Ctx};

pub const HDR: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n";

/// Output (without the creation date), messages and exit status of
/// `lang` on `fig`, or the error that ends the run. `qsorted` is kept
/// for the callers: `convert` itself orders the objects as fig2dev.
pub fn run_with(
    lang: &str,
    fig: &[u8],
    from: Option<&str>,
    opts: &[(char, Option<&str>)],
    qsorted: bool,
) -> Result<(String, Vec<String>, i32), String> {
    let mut ctx = Ctx {
        lang: lang.into(),
        from: from.map(String::from),
        ..Ctx::default()
    };
    let mut dev = drivers::driver(lang).unwrap();
    for &(c, a) in opts {
        if !ctx.general_option(c, a)? {
            dev.option(&mut ctx, c, a)?;
        }
    }
    ctx.font_size *= ctx.fontmag;
    let Ok(objects) = readfp_fig(&mut ctx, fig) else {
        return Ok((String::new(), ctx.messages, 1));
    };
    let _ = qsorted;
    let status = convert(&mut ctx, dev.as_mut(), &objects)?;
    let out = String::from_utf8_lossy(&status.output);
    let out: String = out
        .split_inclusive('\n')
        .filter(|l| !l.starts_with("%%CreationDate"))
        .collect();
    Ok((out, ctx.messages, status.exit))
}

/// [`run_with`] on a Fig 3.2 body (after the header).
pub fn run(lang: &str, body: &str, opts: &[(char, Option<&str>)]) -> Result<(String, Vec<String>, i32), String> {
    run_with(lang, format!("{HDR}{body}").as_bytes(), None, opts, false)
}

pub const FIG2DEV: &str = "/opt/homebrew/bin/fig2dev";

fn old() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../old")
}

/// fig2dev's test figures and every 25th figure of xfig's libraries
/// (every one with `RUSTFIG_TEX_ALL=1`).
pub fn sample_figures() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(old().join("fig2dev-3.2.9a/fig2dev/tests/data"))
        .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    v.retain(|p| p.extension().is_some_and(|e| e == "fig"));
    let mut lib = Vec::new();
    collect(&old().join("xfig-3.2.9a/Libraries"), &mut lib);
    lib.sort();
    v.sort();
    let step = if std::env::var_os("RUSTFIG_TEX_ALL").is_some() {
        1
    } else {
        25
    };
    v.extend(lib.into_iter().step_by(step));
    v
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|e| e == "fig") {
            out.push(p);
        }
    }
}

/// Compares `lang` with fig2dev on the sample figures and `extra` Fig
/// texts, with each option set; returns (identical, total) and prints the
/// differing files.
pub fn compare_with_fig2dev(lang: &str, extra: &[&str], option_sets: &[&[&str]]) -> Option<(usize, usize)> {
    if !Path::new(FIG2DEV).exists() {
        eprintln!("{FIG2DEV} not installed; skipped");
        return None;
    }
    let dir = std::env::temp_dir().join(format!("rustfig-{lang}-cmp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut files = sample_figures();
    for (i, body) in extra.iter().enumerate() {
        let p = dir.join(format!("extra{i}.fig"));
        let text = if body.starts_with("#FIG") {
            body.to_string()
        } else {
            format!("{HDR}{body}")
        };
        std::fs::write(&p, text).unwrap();
        files.push(p);
    }
    let (mut same, mut total) = (0, 0);
    for opts in option_sets {
        for f in &files {
            let path = f.to_string_lossy().to_string();
            let c = Command::new(FIG2DEV)
                .arg("-L")
                .arg(lang)
                .args(*opts)
                .arg(&path)
                .output()
                .unwrap();
            let expected: Vec<u8> = c
                .stdout
                .split_inclusive(|&b| b == b'\n')
                .filter(|l| !l.starts_with(b"%%CreationDate"))
                .flatten()
                .copied()
                .collect();
            let parsed = parse_opts(opts);
            let pairs: Vec<(char, Option<&str>)> = parsed.iter().map(|(c, a)| (*c, a.as_deref())).collect();
            let fig = std::fs::read(f).unwrap();
            total += 1;
            match run_with(lang, &fig, Some(&path), &pairs, true) {
                Ok((out, _, _)) if out.as_bytes() == expected.as_slice() => same += 1,
                Ok((out, _, _)) => {
                    let exp = String::from_utf8_lossy(&expected);
                    let line = out.lines().zip(exp.lines()).position(|(a, b)| a != b);
                    let stem = f.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    let _ = std::fs::write(dir.join(format!("{stem}.fig2dev")), &expected);
                    let _ = std::fs::write(dir.join(format!("{stem}.rustfig")), out.as_bytes());
                    eprintln!(
                        "DIFF {lang} {opts:?} {path} (first differing line {line:?}; outputs in {})",
                        dir.display()
                    );
                }
                Err(e) => eprintln!("ERR {lang} {opts:?} {path}: {e}"),
            }
        }
    }
    eprintln!("{lang}: {same} of {total} identical");
    Some((same, total))
}

/// `-x arg` / `-xarg` / `-x` (flags without argument) into pairs.
fn parse_opts(opts: &[&str]) -> Vec<(char, Option<String>)> {
    const WITH_ARG: &str = "BbCDdEfGgiLlmnpqRSstXxYyZz";
    let mut v = Vec::new();
    let mut it = opts.iter();
    while let Some(o) = it.next() {
        let mut ch = o.chars().skip(1);
        let Some(c) = ch.next() else { continue };
        let rest: String = ch.collect();
        if WITH_ARG.contains(c) {
            let a = if rest.is_empty() {
                it.next().map(|s| s.to_string())
            } else {
                Some(rest)
            };
            v.push((c, a));
        } else {
            v.push((c, None));
        }
    }
    v
}

/// Hand-written figures exercising each object type, arrows, fills,
/// dashes, text flags, fonts and colours (`SPLINES` is Fig 3.1, whose
/// splines stay splines).
pub const LINES: &str = r##"#FIG 3.2
Landscape
Center
Inches
Letter
100.00
Single
-2
1200 2
0 32 #ff8000
0 33 #123456
# whole figure comment
2 1 0 1 0 7 60 -1 -1 0.000 0 0 -1 1 1 3
	1 1 1.00 60.00 120.00
	2 1 2.00 90.00 150.00
	 600 600 1800 900 2400 2400
2 1 1 2 4 7 59 -1 -1 4.000 0 0 -1 1 0 2
	1 1 1.00 60.00 120.00
	 600 3000 3000 3000
2 1 1 2 4 7 58 -1 -1 4.000 0 0 -1 0 1 2
	1 1 1.00 60.00 120.00
	 600 3300 1500 4200
2 1 2 1 32 7 57 -1 -1 3.000 0 0 -1 1 1 2
	0 0 1.00 60.00 120.00
	0 0 1.00 60.00 120.00
	 3600 600 3600 2400
2 1 2 3 33 7 56 -1 -1 3.000 0 0 -1 0 0 3
	 3600 3000 4800 3600 6000 3000
# a comment on a short line
2 1 0 1 0 7 55 -1 -1 0.000 0 0 -1 0 0 2
	 600 4500 700 4560
2 1 0 1 0 7 54 -1 -1 0.000 0 0 -1 1 0 2
	1 1 1.00 60.00 120.00
	 600 4800 680 4860
2 1 1 1 0 7 53 -1 -1 3.000 0 0 -1 0 0 2
	 600 5100 1300 5300
2 1 3 1 0 7 52 -1 -1 3.000 0 0 -1 0 0 2
	 600 5400 1300 5400
2 1 0 1 0 7 51 -1 -1 0.000 0 0 -1 0 0 1
	 900 5700
2 3 0 2 1 2 50 -1 20 0.000 0 0 -1 0 0 4
	 4200 4200 5400 4200 4800 5100 4200 4200
"##;
pub const BOXES: &str = r##"#FIG 3.2
Landscape
Center
Inches
Letter
100.00
Single
-2
1200 2
0 32 #ff8000
0 33 #123456
2 2 0 1 0 7 60 -1 -1 0.000 0 0 -1 0 0 5
	 600 600 1800 600 1800 1200 600 1200 600 600
2 2 1 2 1 7 59 -1 20 5.000 0 0 -1 0 0 5
	 2400 600 3600 600 3600 1200 2400 1200 2400 600
2 2 2 1 2 7 58 -1 17 4.000 0 0 -1 0 0 5
	 600 1800 1800 1800 1800 2400 600 2400 600 1800
2 2 0 4 3 7 57 -1 12 0.000 0 0 -1 0 0 5
	 2400 1800 3600 1800 3600 2400 2400 2400 2400 1800
2 2 0 1 4 7 56 -1 7 0.000 0 0 -1 0 0 5
	 600 3000 1800 3000 1800 3600 600 3600 600 3000
2 2 0 1 5 7 55 -1 0 0.000 0 0 -1 0 0 5
	 2400 3000 3600 3000 3600 3600 2400 3600 2400 3000
2 4 0 1 6 7 54 -1 -1 0.000 0 0 7 0 0 5
	 600 4200 1800 4200 1800 4800 600 4800 600 4200
2 4 1 3 0 7 53 -1 15 4.000 0 0 7 0 0 5
	 2400 4200 3600 4200 3600 4800 2400 4800 2400 4200
2 4 2 1 0 7 52 -1 -1 4.000 0 0 7 0 0 5
	 600 5400 1800 5400 1800 6000 600 6000 600 5400
2 2 3 1 0 7 51 -1 45 4.000 0 0 7 0 0 5
	 2400 5400 3600 5400 3600 6000 2400 6000 2400 5400
"##;
pub const ELLARC: &str = r##"#FIG 3.2
Landscape
Center
Inches
Letter
100.00
Single
-2
1200 2
0 32 #ff8000
0 33 #123456
1 3 0 1 0 7 60 -1 -1 0.000 1 0.0000 1200 1200 300 300 1200 1200 1500 1200
1 3 0 2 4 7 59 -1 20 0.000 1 0.0000 2400 1200 150 150 2400 1200 2550 1200
1 3 1 1 0 7 58 -1 10 4.000 1 0.0000 3600 1200 900 900 3600 1200 4500 1200
1 1 2 3 1 7 57 -1 -1 3.000 1 0.0000 1200 3000 900 450 1200 3000 2100 3000
1 2 0 1 32 7 56 -1 20 0.000 1 0.0000 3600 3000 300 600 3600 3000 3900 3000
1 4 3 1 0 7 55 -1 -1 3.000 1 0.0000 5400 3000 200 200 5400 3000 5600 3000
5 1 0 1 0 7 54 -1 -1 0.000 0 0 1 1 1200.000 4800.000 600 4800 1200 4200 1800 4800
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
5 1 1 2 4 7 53 -1 -1 4.000 0 1 1 0 3600.000 4800.000 3000 4800 3600 5400 4200 4800
	1 1 1.00 60.00 120.00
5 2 0 1 0 7 52 -1 20 0.000 0 1 0 1 5400.000 4800.000 5000 4500 5400 4300 5900 4700
	1 1 1.00 60.00 120.00
5 1 2 1 0 7 51 -1 -1 3.000 0 0 0 0 1200.000 6600.000 1700 6400 1300 6100 800 6500
5 1 0 1 0 7 50 -1 -1 0.000 0 1 1 1 3600.000 6600.000 4100 6800 3700 7100 3200 6700
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
"##;
pub const TEXT: &str = r##"#FIG 3.2
Landscape
Center
Inches
Letter
100.00
Single
-2
1200 2
0 32 #ff8000
0 33 #123456
4 0 0 60 -1 0 12 0.0000 4 135 450 600 600 Hello world\001
4 1 4 59 -1 0 12 0.5236 4 135 450 2400 600 Centred # $ % & _ { } ~ ^ \\ < > |\001
4 2 1 58 -1 2 14 0.0000 2 135 450 4200 600 $x^2$ special\001
4 0 32 57 -1 16 10 1.5708 6 135 450 600 1800 Helvetica rotated\001
4 0 33 56 -1 32 10 0.0000 4 135 450 2400 1800 abg\001
4 0 0 55 -1 3 24 0.0000 0 135 450 4200 1800 italic big\001
4 1 0 54 -1 5 8 0.7854 1 135 450 600 3000 tt rigid\001
4 0 0 53 -1 -1 50 0.0000 4 135 450 2400 3000 huge default\001
4 2 0 52 -1 18 12 0.3000 12 135 450 4200 3000 hidden\001
4 0 0 51 -1 4 12 0.0000 0 135 450 600 4200 Grüße ±\001
"##;
pub const SPLINES: &str = r##"#FIG 3.1
Landscape
Center
Inches
1200 2
3 0 0 1 0 7 60 0 -1 0.000 0 1 1 4
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
	 600 600 1800 1500 3000 600 4200 1500
3 1 1 2 4 7 59 0 -1 4.000 0 0 0 4
	 600 2400 1800 3300 3000 2400 1800 1800
3 2 0 1 0 7 58 0 -1 0.000 0 1 1 3
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
	 600 4200 1800 5100 3000 4200
	 600 4200 600 4200 1500 5100 2100 5100 3000 4200 3000 4200
3 3 2 1 1 7 57 0 10 3.000 0 0 0 3
	 3600 4200 4800 5100 6000 4200
	 3400 4100 3800 4300 4500 5100 5100 5100 6000 4000 6000 4400
"##;
pub const HAND: [&str; 5] = [LINES, BOXES, ELLARC, TEXT, SPLINES];
