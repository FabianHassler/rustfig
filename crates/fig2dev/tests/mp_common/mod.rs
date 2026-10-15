//! Shared by the mp, mf, ge and map tests: comparing with the real
//! fig2dev 3.2.9a over its test figures, every 25th library figure and
//! extra Fig texts, passing `-L lang` to the driver as fig2dev does.
#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

use crate::tex_common::{run_with, sample_figures, FIG2DEV, HDR};

/// Runs `lang` with `-L lang` and `opts` on a Fig 3.2 body.
pub fn run(lang: &str, body: &str, opts: &[(char, Option<&str>)]) -> (String, Vec<String>) {
    let mut all = vec![('L', Some(lang))];
    all.extend_from_slice(opts);
    let (out, msgs, _) = run_with(lang, format!("{HDR}{body}").as_bytes(), None, &all, false).unwrap();
    (out, msgs)
}

/// (identical, total) over the sample figures and `extra`, for each
/// option set (pairs of letter and argument); prints the differing files.
pub fn compare(lang: &str, extra: &[&str], option_sets: &[&[(char, Option<&str>)]]) -> Option<(usize, usize)> {
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
        let mut args = vec!["-L".to_string(), lang.to_string()];
        for (c, a) in *opts {
            args.push(format!("-{c}"));
            if let Some(a) = a {
                args.push(a.to_string());
            }
        }
        let mut pairs = vec![('L', Some(lang))];
        pairs.extend_from_slice(opts);
        for f in &files {
            let path = f.to_string_lossy().to_string();
            let c = Command::new(FIG2DEV).args(&args).arg(&path).output().unwrap();
            let fig = std::fs::read(f).unwrap();
            total += 1;
            match run_with(lang, &fig, Some(&path), &pairs, true) {
                Ok((out, _, _)) if out.as_bytes() == c.stdout.as_slice() => same += 1,
                Ok((out, _, _)) => {
                    let stem = f.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    let _ = std::fs::write(dir.join(format!("{stem}.fig2dev")), &c.stdout);
                    let _ = std::fs::write(dir.join(format!("{stem}.rustfig")), out.as_bytes());
                    eprintln!("DIFF {lang} {opts:?} {path} (outputs in {})", dir.display());
                }
                Err(e) => eprintln!("ERR {lang} {opts:?} {path}: {e}"),
            }
        }
    }
    eprintln!("{lang}: {same} of {total} identical");
    Some((same, total))
}

/// Hand-written figures: every object type, arrows, fills, dashes, text
/// flags, colours, link comments, depths, and Fig 3.1 splines.
pub const HAND: [&str; 5] = [
    concat!(
        "#FIG 3.2\n",
        "Landscape\n",
        "Center\n",
        "Inches\n",
        "Letter\n",
        "100.00\n",
        "Single\n",
        "-2\n",
        "1200 2\n",
        "0 32 #a0b0c0\n",
        "# HREF=\"http://a.b/c\" ALT=\"whole fig\"\n",
        "2 1 1 2 4 7 50 -1 -1 4.000 0 0 -1 1 1 3\n",
        "\t1 1 1.00 60.00 120.00\n",
        "\t2 0 2.00 90.00 150.00\n",
        "\t 600 600 1800 600 1800 1200\n",
        "# HREF=http://x.y/poly ALT=poly\n",
        "2 1 2 1 32 7 45 -1 20 3.000 1 1 -1 1 0 4\n",
        "\t3 1 1.00 60.00 120.00\n",
        "\t 600 1800 900 2100 1200 1800 1500 2400\n",
        "2 2 0 1 0 1 40 -1 30 0.000 0 0 -1 0 0 5\n",
        "\t 2400 600 3600 600 3600 1200 2400 1200 2400 600\n",
        "# HREF=\"http://box\" ALT=\"a box\"\n",
        "2 4 1 3 2 5 40 -1 10 5.000 0 0 15 0 0 5\n",
        "\t 2400 1500 3600 1500 3600 2400 2400 2400 2400 1500\n",
        "2 3 0 2 3 4 40 -1 38 0.000 2 2 -1 0 0 4\n",
        "\t 3900 600 4800 900 4200 1500 3900 600\n",
        "# HREF=http://pic\n",
        "2 5 0 1 0 -1 30 -1 -1 0.000 0 0 -1 0 0 5\n",
        "\t0 nothing.png\n",
        "\t 600 3000 1200 3000 1200 3600 600 3600 600 3000\n",
        "2 1 0 1 0 7 30 -1 -1 0.000 0 0 -1 1 1 2\n",
        "\t10 0 1.00 60.00 120.00\n",
        "\t13 1 1.00 60.00 120.00\n",
        "\t 600 4000 2000 4000\n",
        "2 1 0 3 1 7 30 -1 -1 0.000 0 0 -1 1 1 2\n",
        "\t6 1 1.00 60.00 120.00\n",
        "\t8 1 1.00 60.00 120.00\n",
        "\t 600 4400 2000 4600\n",
        "2 1 0 0 1 7 30 -1 5 0.000 0 0 -1 0 0 3\n",
        "\t 3000 4000 3500 4200 3200 4600\n",
        "2 1 0 1 0 7 30 -1 -1 0.000 0 0 -1 0 0 1\n",
        "\t 4000 4000\n",
    ),
    concat!(
        "#FIG 3.2\n",
        "Landscape\n",
        "Center\n",
        "Inches\n",
        "Letter\n",
        "100.00\n",
        "Single\n",
        "-2\n",
        "1200 2\n",
        "# HREF=\"http://arc\" ALT=\"an arc\"\n",
        "5 1 0 2 0 7 50 -1 -1 0.000 0 0 1 1 1500.000 1500.000 900 1500 1500 900 2100 1500\n",
        "\t1 1 1.00 60.00 120.00\n",
        "\t2 1 1.00 60.00 120.00\n",
        "# HREF=\"http://pie\" ALT=\"a pie\"\n",
        "5 2 1 1 4 6 40 -1 20 4.000 0 1 0 0 3600.000 1500.000 3000 1500 3600 2100 4200 1500\n",
        "5 1 2 3 32 7 40 -1 -1 3.000 1 1 1 0 1500.000 3600.000 1200 3300 1500 3200 1800 3300\n",
        "\t1 0 2.00 90.00 180.00\n",
        "# HREF=http://circle ALT=circle\n",
        "1 3 0 1 0 7 30 -1 -1 0.000 1 0.0000 1200 4800 300 300 1200 4800 1500 4800\n",
        "# HREF=http://ell\n",
        "1 1 1 2 4 2 30 -1 15 4.000 1 0.5236 3000 4800 600 300 3000 4800 3600 5100\n",
        "1 2 2 1 1 0 30 -1 40 3.000 1 0.0000 4500 4800 300 500 4200 4300 4800 5300\n",
        "1 4 0 0 1 0 30 -1 0 3.000 1 0.0000 5500 4800 300 300 5200 4500 5800 5100\n",
    ),
    concat!(
        "#FIG 3.2\n",
        "Landscape\n",
        "Center\n",
        "Inches\n",
        "Letter\n",
        "100.00\n",
        "Single\n",
        "-2\n",
        "1200 2\n",
        "0 33 #ff8000\n",
        "# HREF=http://t ALT=t\n",
        "4 0 0 50 -1 0 12 0.0000 4 135 450 600 600 Plain {text} with $&#_%~^\\\\ <>\\001\n",
        "4 1 4 50 -1 16 14 0.5236 4 135 450 2400 600 Centered ps\\001\n",
        "4 2 33 50 -1 2 10 1.5708 0 135 450 4000 600 Right latex bold\\001\n",
        "4 0 0 40 -1 -1 12 0.0000 4 135 450 600 1200 PS default font\\001\n",
        "4 0 0 40 -1 0 12 0.0000 0 135 450 600 1500 LaTeX default font\\001\n",
        "4 0 0 40 -1 5 20 0.0000 2 135 450 600 1800 $x^2$ special \\\\(y\\\\) \\\\begin{math}z\\\\end{math} \\\\$\\001\n",
        "4 1 0 40 -1 18 20 0.0000 6 135 450 600 2100 $\\\\alpha$ special ps\\001\n",
        "4 2 0 40 -1 3 30 0.2 3 135 450 3600 2400 rigid special latex $a$\\001\n",
        "4 0 0 40 -1 33 8 0.0000 12 135 450 600 2700 hidden ps\\001\n",
        "4 0 0 40 -1 4 50 0.0000 1 135 450 600 3000 big rigid\\001\n",
        "4 0 0 40 -1 -1 12 0.0000 0 135 450 600 3300 latex font -1\\001\n",
    ),
    concat!(
        "#FIG 3.1\n",
        "Landscape\n",
        "Center\n",
        "Inches\n",
        "1200 2\n",
        "3 0 0 1 0 7 50 0 -1 0.000 0 1 1 4\n",
        "\t1 1 1.00 60.00 120.00\n",
        "\t1 1 1.00 60.00 120.00\n",
        "\t 600 600 1200 300 1800 900 2400 600\n",
        "3 1 1 2 4 5 50 0 10 4.000 0 0 0 4\n",
        "\t 3000 600 3600 300 4200 900 3600 1200\n",
        "3 0 2 1 0 7 50 0 -1 3.000 0 1 1 2\n",
        "\t2 1 1.00 60.00 120.00\n",
        "\t2 1 1.00 60.00 120.00\n",
        "\t 600 1800 1800 2100\n",
        "3 2 0 1 1 7 40 0 -1 0.000 0 1 1 3\n",
        "\t1 1 1.00 60.00 120.00\n",
        "\t1 1 1.00 60.00 120.00\n",
        "\t 600 3000 1200 2700 1800 3000\n",
        "\t 600.000 3000.000 700.000 2900.000 1100.000 2700.000 1300.000 2700.000 1700.000 2900.000 1800.000 3000.000\n",
        "3 3 1 2 2 3 40 0 20 4.000 0 0 0 4\n",
        "\t 3000 3000 3600 2700 4200 3000 3600 3300\n",
        "\t 3000.000 3000.000 3000.000 2900.000 3500.000 2700.000 3700.000 2700.000 4100.000 2900.000 4200.000 3100.000 3700.000 3300.000 3500.000 3300.000\n",
    ),
    concat!(
        "#FIG 3.2\n",
        "Landscape\n",
        "Center\n",
        "Inches\n",
        "Letter\n",
        "100.00\n",
        "Single\n",
        "-2\n",
        "1200 2\n",
        "2 1 0 1 0 7 100 -1 -1 0.000 0 0 -1 0 0 2\n",
        "\t 0 0 1200 1200\n",
        "2 1 0 1 0 7 99 -1 -1 0.000 0 0 -1 0 0 2\n",
        "\t 0 1200 1200 0\n",
        "2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 2\n",
        "\t 600 0 600 1200\n",
        "1 3 0 1 0 7 10 -1 -1 0.000 1 0.0000 600 600 300 300 600 600 900 600\n",
        "4 0 0 0 -1 0 12 0.0000 4 135 450 600 600 top\\001\n",
    ),
];
