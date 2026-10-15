#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `mp` driver against output of the real fig2dev 3.2.9a.

mod mp_common;
mod tex_common;

const CARGO_TOML: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");

const BODY: &str = concat!(
    "# HREF=\"http://a\" ALT=\"A\"\n",
    "2 1 1 2 4 7 50 -1 -1 4.000 0 0 -1 1 0 3\n",
    "\t1 1 1.00 60.00 120.00\n",
    "\t 600 600 1800 600 1800 1200\n",
    "1 3 0 1 0 7 40 -1 20 0.000 1 0.0000 1200 2400 300 300 1200 2400 1500 2400\n",
    "4 1 0 30 -1 0 12 0.0000 4 135 450 2400 2400 a_b\\001\n",
);

const EXPECTED: &str = concat!(
    "%\n",
    "% fig2dev (version 3.2.9a) -L (m)mp version 0.05 --- Preamble\n",
    "%\n",
    "\n",
    "%\n",
    "% mp output driver options:\n",
    "%  -L mp\n",
    "%\n",
    "\n",
    "% +MP-ADDITIONAL-HEADER\n",
    "verbatimtex\n",
    "%&latex\n",
    "\\documentclass{article}\n",
    "\\begin{document}\n",
    "etex\n",
    "% -MP-ADDITIONAL-HEADER\n",
    "\n",
    "\n",
    "%SetFigFont macros for latex\n",
    "verbatimtex\n",
    "%\n",
    "\\begingroup\\makeatletter\\ifx\\SetFigFont\\undefined%\n",
    "\\gdef\\SetFigFont#1#2#3#4#5{%\n",
    "  \\reset@font\\fontsize{#1}{#2pt}%\n",
    "  \\fontfamily{#3}\\fontseries{#4}\\fontshape{#5}%\n",
    "  \\selectfont}%\n",
    "\\fi\\endgroup%\n",
    "\\ifx\\SetFigFontSize\\undefined%\n",
    "\\gdef\\SetFigFontSize#1#2{%\n",
    "\t\\fontsize{#1}{#2pt}%\n",
    "\t\\selectfont}%\n",
    "\\fi%\n",
    "etex\n",
    "\n",
    "% Now draw the figure\n",
    "beginfig(0)\n",
    "% Some reasonable defaults\n",
    "  labeloffset:=0;\n",
    "  truecorners:=1;\n",
    "  bboxmargin:=0;\n",
    "%  HREF=\"http://a\" ALT=\"A\" \n",
    "% Begin polyline object\n",
    "  linecap:=0;\n",
    "  linejoin:=0;\n",
    "  pickup pencircle scaled 1.80;\n",
    "  path p;\n",
    "  p = (36.00, 756.00)\n",
    "    --(108.00, 756.00)\n",
    "    --(108.00, 720.00);\n",
    "\tdraw p withcolor (1.00,0.00,0.00) dashed evenly scaled 1.21;\n",
    "% Draw arrowhead type 1\n",
    "  linecap:=0;\n",
    "  linejoin:=0;\n",
    "  pickup pencircle scaled 0.90;\n",
    "  path arr;\n",
    "  arr = (106.20, 728.16)\n",
    "      --(108.00, 720.96)\n",
    "      --(109.80, 728.16)\n",
    "      --(106.20, 728.16) -- cycle;\n",
    "  fill arr withcolor (1.00,0.00,0.00);\n",
    "  draw arr withcolor (1.00,0.00,0.00);\n",
    "% End polyline object\n",
    "% Begin ellipse object\n",
    "  pickup pencircle scaled 0.90;\n",
    "  path c;\n",
    "  c = fullcircle scaled 36.00 yscaled 1.00\n",
    "\t    rotated 0.00 shifted (72.00,648.00);\n",
    " fill c withcolor ((1.00,1.00,1.00) + 0.00white);\n",
    "\tdraw c withcolor (0.00,0.00,0.00);\n",
    "% End ellipse object\n",
    "% Begin text object\n",
    "  picture q;\n",
    "  q=thelabel.urt(\"a_b\" infont \"ptmr\" scaled (10pt/fontsize defaultfont),(0,0));\n",
    "  picture p;\n",
    "  p = q rotated 0.00;\n",
    "  label.urt(p,((144.00,648.00))+xpart (lrcorner q - llcorner q)*(-cosd 0.00,-sind 0.00)/2+llcorner p) withcolor (0.00,0.00,0.00);\n",
    "% End text object\n",
    "endfig;\n",
    "end\n",
);

#[test]
fn as_fig2dev() {
    let (out, msgs) = mp_common::run("mp", BODY, &[]);
    assert_eq!(out, EXPECTED);
    assert!(msgs.is_empty(), "{msgs:?}");
}

const BODY_TEX: &str = concat!(
    "2 1 0 1 0 7 100 -1 -1 0.000 0 0 -1 0 0 2\n",
    "\t 0 0 1200 1200\n",
    "4 0 0 50 -1 2 12 0.0000 2 135 450 600 600 $x$ \\\\(y\\\\)\\001\n",
);

const EXPECTED_TEX: &str = concat!(
    "%\n",
    "% fig2dev (version 3.2.9a) -L (m)mp version 0.05 --- Preamble\n",
    "%\n",
    "\n",
    "%\n",
    "% mp output driver options:\n",
    "%  -L mp -M -o -p 2\n",
    "%\n",
    "\n",
    "prologues:=2;\n",
    "\n",
    "% +MP-ADDITIONAL-HEADER\n",
    "% -MP-ADDITIONAL-HEADER\n",
    "\n",
    "path allbounds;\n",
    "allbounds = (-0.72,792.72)--(72.72,792.72)--(72.72,719.28)--(-0.72,719.28)--cycle;\n",
    "% Some reasonable defaults\n",
    "  labeloffset:=0;\n",
    "  truecorners:=0;\n",
    "  bboxmargin:=0;\n",
    "% Now draw objects of depth: 100\n",
    "beginfig(0)\n",
    "% Begin polyline object\n",
    "  linecap:=0;\n",
    "  linejoin:=0;\n",
    "  pickup pencircle scaled 0.90;\n",
    "  path p;\n",
    "  p = (0.00, 792.00)\n",
    "    --(72.00, 720.00);\n",
    "\tdraw p withcolor (0.00,0.00,0.00);\n",
    "% End polyline object\n",
    "setbounds currentpicture to allbounds;\n",
    "endfig;\n",
    "% Now draw objects of depth: 50\n",
    "beginfig(1)\n",
    "% Begin text object\n",
    "  picture q;\n",
    "  verbatimtex\n",
    "\t \\font\\mpsetfnt=cmbx10 at 10.80pt\n",
    "\t \\font\\tenrm=cmr10 at 10.80pt\\font\\sevenrm=cmr7 at 7.56pt\n",
    "\t \\font\\fiverm=cmr5 at 5.40pt\\font\\teni=cmmi10 at 10.80pt\n",
    "\t \\font\\seveni=cmmi7 at 7.56pt\\font\\fivei=cmmi5 at 5.40pt\n",
    "\t \\font\\tensy=cmsy10 at 10.80pt\\font\\sevensy=cmsy7 at 7.56pt\n",
    "\t \\font\\fivesy=cmsy5 at 5.40pt\n",
    "\t \\def\\figtodev@genmp@mathfonts{%\n",
    "\t   \\textfont0\\tenrm\\scriptfont0\\sevenrm\\scriptscriptfont0\\fiverm\n",
    "\t   \\textfont1\\teni\\scriptfont1\\seveni\\scriptscriptfont1\\fivei\n",
    "\t   \\textfont2\\tensy\\scriptfont2\\sevensy\\scriptscriptfont2\\fivesy}\n",
    "  etex;\n",
    "  q = btex \\mpsetfnt $\\figtodev@genmp@mathfonts x$ \\(\\figtodev@genmp@mathfonts y\\) etex;\n",
    "  picture p;\n",
    "  p = q rotated 0.00;\n",
    "  label.urt(p,((36.00,756.00))+llcorner p) withcolor (0.00,0.00,0.00);\n",
    "% End text object\n",
    "setbounds currentpicture to allbounds;\n",
    "endfig;\n",
    "end\n",
);

/// Layers (`-M`), TeX mode (`-o`) with a special text, `prologues` (`-p`).
#[test]
fn layers_and_tex_mode() {
    let (out, _) = mp_common::run("mp", BODY_TEX, &[('M', None), ('o', None), ('p', Some("2"))]);
    assert_eq!(out, EXPECTED_TEX);
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test mp -- --ignored --nocapture`.
#[test]
#[ignore]
fn same_as_fig2dev() {
    if let Some((same, total)) = mp_common::compare(
        "mp",
        &mp_common::HAND,
        &[
            &[],
            &[('M', None)],
            &[('o', None)],
            &[('p', Some("1")), ('i', Some("inc.tex"))],
            &[('o', None), ('i', Some("x.tex")), ('d', Some(CARGO_TOML))],
            &[('m', Some("2"))],
        ],
    ) {
        assert!(same * 10 >= total * 9, "{same} of {total} identical");
    }
}
