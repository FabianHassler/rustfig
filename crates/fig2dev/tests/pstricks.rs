#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `pstricks` driver against output of the real fig2dev 3.2.9a
//! (the expected texts were written by `/opt/homebrew/bin/fig2dev -L
//! pstricks`, with the options each test gives).

mod pic_common;
mod tex_common;

use tex_common::{compare_with_fig2dev, run, run_with, HAND};

#[test]
fn dashed_polylines_dots_and_arrows() {
    let body = r##"0 32 #ff8000
0 33 #123456
2 1 1 2 0 7 50 -1 -1 4.000 0 0 -1 1 1 4
	1 1 1.00 60.00 120.00
	2 1 1.00 60.00 120.00
	 600 600 1800 900 2400 2400 3000 2450
2 3 3 1 4 7 50 -1 -1 3.000 1 1 -1 0 0 5
	 3600 600 4200 600 4200 1200 3600 1200 3600 600
2 2 4 3 0 7 50 -1 -1 3.000 2 1 -1 0 0 5
	 3600 1800 4200 1800 4200 2400 3600 2400 3600 1800
2 1 5 1 33 7 50 -1 -1 3.000 0 2 -1 0 0 2
	 600 3000 3000 3000
2 1 0 0 0 7 50 -1 -1 0.000 0 0 -1 0 0 1
	 700 3300
2 1 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 1
	 700 3500
"##;
    let (out, msgs, exit) = run("pstricks", body, &[]).unwrap();
    assert_eq!(
        out,
        r##"\begin{pspicture}(-0.0000,-0.0000)(7.7385,6.2124)
\psline[linewidth=0.0318,linestyle=dashed,dash=0.1270 0.1270,linecolor=black]{-}(0.0466,6.1637)(2.5866,5.5287)(3.8566,2.3537)(5.1266,2.2479)
\pspolygon[linewidth=0.0318,linecolor=black,fillstyle=solid,fillcolor=black](0.0466,6.1637)(0.2776,6.0405)(0.2191,6.1206)(0.3084,6.1637)
\pspolygon[linewidth=0.0318,linecolor=black,fillstyle=solid,fillcolor=black](5.1266,2.2479)(4.8787,2.3323)(4.8682,2.2057)
\pspolygon[linewidth=0.0159,linestyle=dashed,dash=0.0953 0.0476 0.0318 0.0476,linejoin=1,linecolor=red](6.3966,6.1637)(7.6666,6.1637)(7.6666,4.8937)(6.3966,4.8937)
\psframe[linewidth=0.0476,linestyle=dashed,dash=0.0953 0.0318 0.0953 0.0318 0.0953 0.0318,linejoin=2,linecolor=black](6.3966,2.3537)(7.6666,3.6237)
\newrgbcolor{usrclri}{0.0703 0.2031 0.3359}%
\psline[linewidth=0.0159,linestyle=dashed,dash=0.0953 0.0238 0.0318 0.0238 0.0318 0.0238 0.0318 0.0238,linecolor=usrclri]{C-C}(0.0466,1.0837)(5.1266,1.0837)
\psdots[dotsize=0.0000](0.2582,0.4487)
\psdots[dotsize=0.0079](0.2582,0.0254)
\end{pspicture}
"##
    );
    assert_eq!(exit, 0);
    assert_eq!(
        msgs,
        vec![
            "IMPORTANT notes from pstricks driver:",
            "  Dash-dot lines require pstricks patchlevel 15 of June 2004 or later.",
            "  Dash-dot-dot lines require \\usepackage{pstricks-add}.",
            "  Linejoins depend on PSTricks version. Use -t option to ensure compatibility."
        ]
    );
}

#[test]
fn filled_and_hatched_polygons() {
    let body = r##"0 32 #ff8000
0 33 #123456
2 3 0 1 0 7 50 -1 0 0.000 0 0 -1 0 0 5
	 600 600 1800 600 1800 1200 600 1200 600 600
2 3 0 1 4 7 50 -1 10 0.000 0 0 -1 0 0 5
	 2000 600 3200 600 3200 1200 2000 1200 2000 600
2 3 0 1 4 32 50 -1 20 0.000 0 0 -1 0 0 5
	 3400 600 4600 600 4600 1200 3400 1200 3400 600
2 3 0 1 4 33 50 -1 25 0.000 0 0 -1 0 0 5
	 600 1500 1800 1500 1800 2100 600 2100 600 1500
2 3 0 1 4 12 50 -1 35 0.000 0 0 -1 0 0 5
	 2000 1500 3200 1500 3200 2100 2000 2100 2000 1500
2 3 0 1 0 7 50 -1 15 0.000 0 0 -1 0 0 5
	 3400 1500 4600 1500 4600 2100 3400 2100 3400 1500
2 3 0 1 0 7 50 -1 41 0.000 0 0 -1 0 0 5
	 600 2400 1800 2400 1800 3000 600 3000 600 2400
2 3 0 1 4 2 50 -1 44 0.000 0 0 -1 0 0 5
	 2000 2400 3200 2400 3200 3000 2000 3000 2000 2400
2 3 0 1 0 7 50 -1 47 0.000 0 0 -1 0 0 5
	 3400 2400 4600 2400 4600 3000 3400 3000 3400 2400
2 3 0 1 0 7 50 -1 51 0.000 0 0 -1 0 0 4
	 600 3300 1800 3300 1200 3900 600 3300
"##;
    let (out, msgs, exit) = run("pstricks", body, &[]).unwrap();
    assert_eq!(
        out,
        r##"\begin{pspicture}(-0.0000,-0.0000)(8.5196,7.0379)
\pspolygon[linewidth=0.0159,linecolor=black,fillcolor=white,hatchcolor=black,fillstyle=crosshatch*,hatchangle=-0.00](0.0254,1.2954)(2.5654,1.2954)(1.2954,0.0254)
\newgray{grayx}{0.5000}%
\pspolygon[linewidth=0.0159,linecolor=red,fillstyle=solid,fillcolor=grayx](2.9887,7.0104)(5.5287,7.0104)(5.5287,5.7404)(2.9887,5.7404)
\newrgbcolor{usrclr}{0.9961 0.5000 0.0000}%
\pspolygon[linewidth=0.0159,linecolor=red,fillstyle=solid,fillcolor=usrclr](5.9521,7.0104)(8.4921,7.0104)(8.4921,5.7404)(5.9521,5.7404)
\newrgbcolor{usrclristxxv}{0.3027 0.4023 0.5020}%
\pspolygon[linewidth=0.0159,linecolor=red,fillstyle=solid,fillcolor=usrclristxxv](0.0254,5.1054)(2.5654,5.1054)(2.5654,3.8354)(0.0254,3.8354)
\newrgbcolor{greenistxxxv}{0.7500 0.8900 0.7500}%
\pspolygon[linewidth=0.0159,linecolor=red,fillstyle=solid,fillcolor=greenistxxxv](2.9887,5.1054)(5.5287,5.1054)(5.5287,3.8354)(2.9887,3.8354)
\newgray{grayxv}{0.7500}%
\pspolygon[linewidth=0.0159,linecolor=black,fillstyle=solid,fillcolor=grayxv](5.9521,5.1054)(8.4921,5.1054)(8.4921,3.8354)(5.9521,3.8354)
\pspolygon[linewidth=0.0159,linecolor=black,fillcolor=white,hatchcolor=black,fillstyle=hlines*,hatchangle=-30.00](0.0254,3.2004)(2.5654,3.2004)(2.5654,1.9304)(0.0254,1.9304)
\pspolygon[linewidth=0.0159,linecolor=red,fillcolor=green,hatchcolor=red,fillstyle=vlines*,hatchangle=-45.00](2.9887,3.2004)(5.5287,3.2004)(5.5287,1.9304)(2.9887,1.9304)
\pspolygon[linewidth=0.0159,linecolor=black,fillcolor=white,hatchcolor=black,fillstyle=crosshatch*,hatchangle=-10.00](5.9521,3.2004)(8.4921,3.2004)(8.4921,1.9304)(5.9521,1.9304)
\pspolygon[linewidth=0.0159,linecolor=black,fillstyle=solid,fillcolor=black](0.0254,7.0104)(2.5654,7.0104)(2.5654,5.7404)(0.0254,5.7404)
\end{pspicture}
"##
    );
    assert_eq!(exit, 0);
    assert_eq!(
        msgs,
        vec![
            "IMPORTANT notes from pstricks driver:",
            "  A hatch pattern not supported by pstricks has been converted to crosshatch."
        ]
    );
}

#[test]
fn arc_box_ellipses_and_arcs() {
    let body = r##"0 32 #ff8000
0 33 #123456
2 4 0 1 0 7 50 -1 -1 0.000 0 0 12 0 0 5
	 600 600 1800 600 1800 1200 600 1200 600 600
1 3 0 1 0 7 50 -1 20 0.000 1 0.0000 1200 2000 300 300 1200 2000 1500 2000
1 1 1 2 4 7 50 -1 30 4.000 1 0.5236 2400 2000 400 200 2400 2000 2800 2000
1 1 0 1 0 7 50 -1 43 0.000 1 0.7854 3600 2000 400 200 3600 2000 4000 2000
1 1 0 1 0 7 50 -1 -1 0.000 1 0.0000 4600 2000 400 200 4600 2000 5000 2000
5 1 0 1 0 7 50 -1 -1 0.000 0 1 1 1 1800 3000 1200 3000 1800 2400 2400 3000
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
5 2 2 1 0 7 50 -1 25 4.000 0 0 0 0 4200 3000 3600 3000 4200 3600 4800 3000
5 1 0 1 0 7 50 -1 -1 0.000 0 0 0 0 1800 4200 1200 4200 1800 4800 2400 4200
5 1 0 1 0 7 50 -1 -1 0.000 0 0 1 0 1800 5400 1200 5400 1800 6000 2400 5400
	2 1 1.00 60.00 120.00
"##;
    let (out, msgs, exit) = run("pstricks", body, &[]).unwrap();
    assert_eq!(
        out,
        r##"\begin{pspicture}(-0.0000,-0.0000)(9.3578,11.4745)
\psframe[cornersize=absolute,linearc=0.3810,linewidth=0.0159,linecolor=black](0.0254,10.1769)(2.5654,11.4469)
\newrgbcolor{whitestxxv}{1.0000 1.0000 1.0000}%
\pswedge[linewidth=0.0159,linestyle=dotted,dotsep=0.1270,linecolor=black,fillstyle=solid,fillcolor=whitestxxv](7.6454,6.3669){1.2700}{0.0000}{180.0000}%
\psarc[linewidth=0.0159,linecolor=black]{-}(2.5654,3.8269){1.2700}{0.0000}{180.0000}%
\psarc[linewidth=0.0159,linecolor=black]{-}(2.5654,1.2869){1.2700}{0.0000}{180.0000}%
\pspolygon[linewidth=0.0159,linecolor=black,fillstyle=solid,fillcolor=black](3.8354,1.2869)(3.8989,1.5409)(3.8354,1.4647)(3.7719,1.5409)
\pscircle[linewidth=0.0159,linecolor=black,fillstyle=solid,fillcolor=white](1.2954,8.4836){0.6350}%
\newrgbcolor{whitestxxx}{1.0000 1.0000 1.0000}%
\rput{30.0001}(3.8354,8.4836){\psellipse[linewidth=0.0318,linestyle=dashed,dash=0.1270 0.1270,linecolor=red,fillstyle=solid,fillcolor=whitestxxx](0,0)(0.8467,0.4233)}%
\rput{45.0001}(6.3754,8.4836){\psellipse[linewidth=0.0159,linecolor=black,fillcolor=white,hatchcolor=black,fillstyle=crosshatch*,hatchangle=-75.00](0,0)(0.8467,0.4233)}%
\psellipse[linewidth=0.0159,linecolor=black](8.4921,8.4836)(0.8467,0.4233)
\psarc[linewidth=0.0159,linecolor=black]{-}(2.5654,6.3669){1.2700}{180.0000}{0.0000}%
\pspolygon[linewidth=0.0159,linecolor=black,fillstyle=solid,fillcolor=black](1.2954,6.3669)(1.2319,6.1129)(1.3589,6.1129)
\pspolygon[linewidth=0.0159,linecolor=black,fillstyle=solid,fillcolor=black](3.8354,6.3669)(3.7719,6.1129)(3.8989,6.1129)
\end{pspicture}
"##
    );
    assert_eq!(exit, 0);
    assert_eq!(
        msgs,
        vec![
            "IMPORTANT notes from pstricks driver:",
            "  Hatch pattern in rotated ellipse may be imprecisely converted."
        ]
    );
}

#[test]
fn texts_and_fonts() {
    let body = r##"0 32 #ff8000
0 33 #123456
4 0 0 50 -1 0 12 0.0000 4 135 450 600 600 Hello world\001
4 1 4 50 -1 1 12 0.5236 4 135 450 2400 600 Centred # $ % & _ { } ~ ^ \\ < > |\001
4 2 0 50 -1 16 14 0.0000 4 135 450 4000 900 right\001
4 0 0 50 -1 12 10 0.0000 4 135 450 600 1500 Courier\001
4 0 0 50 -1 32 10 0.0000 4 135 450 600 1800 Symbol\001
4 0 0 50 -1 0 12 0.0000 6 135 450 600 2100 $x^2$ unbalanced {\001
4 0 33 50 -1 0 12 0.0000 2 135 450 600 2400 Multi
line\001
"##;
    let (out, msgs, exit) = run("pstricks", body, &[]).unwrap();
    assert_eq!(
        out,
        r##"%
\begingroup\makeatletter%
\ifx\SetFigFont\undefined%
  \gdef\SetFigFont#1#2#3#4#5{%
	 \reset@font\fontsize{#1}{#2pt}%
	 \fontfamily{#3}\fontseries{#4}\fontshape{#5}%
	 \selectfont}%
\fi
\endgroup%
\begin{pspicture}(-0.0000,-0.0000)(7.2623,4.3180)
\begingroup\SetFigFont{12}{14.4}{\rmdefault}{\mddefault}{\updefault}%
\rput[lb](0.0318,3.8417){Hello world}%
\endgroup%
\begingroup\SetFigFont{12}{14.4}{\rmdefault}{\mddefault}{\itdefault}%
\rput[b]{30.00}(3.8417,3.8417){Centred \# \$ \% \& \_ $\{$ $\}$ \~{} \^{} $\backslash$ $<$ $>$ |}%
\endgroup%
\begingroup\SetFigFont{14}{16.8}{\sfdefault}{\mddefault}{\updefault}%
\rput[rb](7.2284,3.2067){right}%
\endgroup%
\begingroup\SetFigFont{10}{12.0}{\ttdefault}{\mddefault}{\updefault}%
\rput[lb](0.0318,1.9367){Courier}%
\endgroup%
\begingroup\SetFigFont{10}{12.0}{\rmdefault}{\mddefault}{\updefault}%
\rput[lb](0.0318,1.3017){Symbol}%
\endgroup%
\begingroup\SetFigFont{12}{14.4}{\rmdefault}{\mddefault}{\updefault}%
\rput[lb](0.0318,0.6668){$x^2$ unbalanced {}%
\endgroup%
\begingroup\SetFigFont{12}{14.4}{\familydefault}{\mddefault}{\updefault}%
\rput[lb](0.0318,0.0318){Multi
line}%
\endgroup%
\end{pspicture}
"##
    );
    assert_eq!(exit, 0);
    assert_eq!(
        msgs,
        vec![
            "warning: unbalanced braces in special text '$x^2$ unbalanced {'",
            "IMPORTANT notes from pstricks driver:",
            "  Substituted LaTeX font for PS.  Use -v to see details."
        ]
    );
}

#[test]
fn all_arrow_types_with_pstricks_arrows() {
    let body = r##"0 32 #ff8000
0 33 #123456
2 1 0 3 0 7 50 -1 -1 0.000 0 0 -1 1 1 2
	0 0 1.00 90.00 150.00
	1 0 1.00 90.00 150.00
	 600 300 3000 300
2 1 0 3 4 7 50 -1 -1 0.000 0 0 -1 1 1 2
	2 1 1.00 90.00 150.00
	3 0 1.00 90.00 150.00
	 600 600 3000 600
2 1 0 3 0 7 50 -1 -1 0.000 0 0 -1 1 1 2
	4 1 1.00 90.00 150.00
	5 1 1.00 90.00 150.00
	 600 900 3000 900
2 1 0 3 0 7 50 -1 -1 0.000 0 0 -1 1 1 2
	6 0 1.00 90.00 150.00
	7 1 1.00 90.00 150.00
	 600 1200 3000 1200
2 1 0 3 0 7 50 -1 -1 0.000 0 0 -1 1 1 2
	8 1 1.00 90.00 150.00
	9 0 1.00 90.00 150.00
	 600 1500 3000 1500
2 1 0 3 0 7 50 -1 -1 0.000 0 0 -1 1 1 2
	10 1 1.00 90.00 150.00
	11 0 1.00 90.00 150.00
	 600 1800 3000 1800
2 1 0 3 0 7 50 -1 -1 0.000 0 0 -1 1 1 2
	12 1 1.00 90.00 150.00
	13 0 1.00 90.00 150.00
	 600 2100 3000 2100
2 1 0 3 0 7 50 -1 -1 0.000 0 0 -1 1 1 2
	14 1 1.00 90.00 150.00
	14 0 1.00 90.00 150.00
	 600 2400 3000 2400
"##;
    let (out, msgs, exit) = run("pstricks", body, &[('R', Some("1"))]).unwrap();
    assert_eq!(
        out,
        r##"\begin{pspicture}(-0.0000,-0.0000)(5.2218,4.7773)
\psline[linewidth=0.0476,tbarsize=0.1905 2,bracketlength=1.6667,linecolor=black]{[-]}(0.0698,0.1651)(5.1498,0.1651)
\psline[linewidth=0.0476,arrowsize=0.1905 2,arrowlength=1.66667,arrowinset=0.30000,linecolor=red]{<->}(0.0698,3.9751)(5.1498,3.9751)
\psline[linewidth=0.0476,arrowsize=0.1905 2,arrowlength=0.83333,arrowinset=-1.00000,linecolor=black]{**->}(0.0698,3.3401)(5.1498,3.3401)
\psline[linewidth=0.0476,dotsize=0.3175 2,linecolor=black]{<-o}(0.0698,2.7051)(5.1498,2.7051)
\psline[linewidth=0.0476,arrowsize=0.1905 2,arrowlength=1.66667,arrowinset=0.00000,linecolor=black]{<-<}(0.0698,2.0701)(5.1498,2.0701)
\psline[linewidth=0.0476,arrowsize=0.1905 2,arrowlength=1.66667,arrowinset=0.00000,linecolor=black]{<->}(0.0698,1.4351)(5.1498,1.4351)
\psline[linewidth=0.0476,arrowsize=0.1905 2,arrowlength=1.28205,arrowinset=-0.30000,linecolor=black]{>->}(0.0698,0.8001)(5.1498,0.8001)
\psline[linewidth=0.0476,arrowsize=0.1905 2,arrowlength=1.66667,arrowinset=1.00000,ArrowFill=false,linecolor=black]{<->}(0.0698,4.6101)(5.1498,4.6101)
\end{pspicture}
"##
    );
    assert_eq!(exit, 0);
    assert_eq!(
        msgs,
        vec![
            "IMPORTANT notes from pstricks driver:",
            "  Hollow arrows required \\usepackage{pstricks-add}.",
            "  One or more arrows have been approximated with a PSTricks style. Try -R 0."
        ]
    );
}

#[test]
fn pictures_in_page_mode() {
    let body = r##"0 32 #ff8000
0 33 #123456
2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5
	0 pics/a.png
	 600 600 1800 600 1800 1200 600 1200 600 600
2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5
	1 sub\dir/B file.JPG
	 3000 600 3600 600 3600 1200 3000 1200 3000 600
2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5
	0 pics/a.png
	 600 2000 1800 2000 1800 2600 600 2600 600 2000
2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5
	0 other/a.png
	 3000 2000 3600 2000 3600 2600 3000 2600 3000 2000
"##;
    let (out, msgs, exit) = run("pstricks", body, &[('n', Some("2")), ('v', None)]).unwrap();
    assert_eq!(
        out,
        r##"% PSTricks driver for Fig Version 1.2 by Gene Ressler.
% PSTricks version 1.20 is assumed in Sketch output (change with -t).
\documentclass{article}
\pagestyle{empty}
\setlength\topmargin{-.5in}
\setlength\textheight{10in}
\setlength\oddsidemargin{-.5in}
\setlength\evensidemargin\oddsidemargin
\setlength\textwidth{7.5in}
\usepackage{pstricks,graphicx}
\begin{document}
\begin{center}
% Following sets up unit length for picture dimensions. If you 
% do \psset{unit=LEN} to scale the picture, you must also
% \newlength\unit\setlength\unit{LEN} for the bitmaps to match.
\makeatletter%
\@ifundefined{unit}{\newlength\unit\setlength\unit{1.000cm}}%
\makeatother%
\begin{pspicture}(-0.0000,-0.0000)(6.3521,4.2355)
% pic box
\rput[bl](0.0000,2.9633){\includegraphics[origin=c,angle=0,width=2.5400\unit,totalheight=1.2700\unit]{a}}%
% pic box
\rput[bl](5.0800,2.9633){\includegraphics[origin=c,angle=90,width=1.2700\unit,totalheight=-1.2700\unit]{B file}}%
% pic box
\rput[bl](0.0000,0.0000){\includegraphics[origin=c,angle=0,width=2.5400\unit,totalheight=1.2700\unit]{a}}%
% pic box
\rput[bl](5.0800,0.0000){\includegraphics[origin=c,angle=0,width=1.2700\unit,totalheight=1.2700\unit]{a-001}}%
\end{pspicture}
\end{center}
\end{document}
"##
    );
    assert_eq!(exit, 0);
    assert_eq!(
        msgs,
        vec![
            "IMPORTANT notes from pstricks driver:",
            "  Picture requires \\usepackage{graphicx}",
            "  Spaces and special chars in picture file path are likely to cause problems.",
            "  Backslashes were converted to slashes in picture file path.",
            "  Non-EPS picture paths were converted to the eps image directory.",
            "  Setting unit requires \\psset{unit=X}\\newlength\\unit\\setlength\\unit{X}."
        ]
    );
}

#[test]
fn old_splines() {
    let fig = r##"#FIG 3.1
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
3 0 0 1 0 7 58 0 -1 0.000 0 1 1 2
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
	 600 6000 1800 6600
"##;
    let (out, msgs, exit) = run_with(
        "pstricks",
        fig.as_bytes(),
        None,
        &[('R', Some("2")), ('t', Some("1.1")), ('v', None)],
        false,
    )
    .unwrap();
    assert_eq!(
        out,
        r##"% PSTricks driver for Fig Version 1.2 by Gene Ressler.
% PSTricks version 1.20 is assumed in Sketch output (change with -t).
\begin{pspicture}(-0.0000,-0.0000)(11.4660,12.7360)
\psbezier[linewidth=0.0159,linecolor=black]{<->}(0.0169,12.7169)(1.7103,11.4469)(2.9803,11.1294)(3.8269,11.7644)
\psbezier[linewidth=0.0159,linecolor=black]{<->}(3.8269,23.5289)(4.6736,16.3209)(5.9436,12.0819)(7.6369,10.8119)
\psbezier[linewidth=0.0318,linestyle=dashed,dash=0.1270 0.1270,linecolor=red](3.8269,9.5419)(2.9803,9.9653)(2.1336,9.9653)(1.2869,9.5419)
\psbezier[linewidth=0.0318,linestyle=dashed,dash=0.1270 0.1270,linecolor=red](1.2869,9.5419)(0.4403,9.1186)(0.4403,8.5894)(1.2869,7.9544)
\psbezier[linewidth=0.0318,linestyle=dashed,dash=0.1270 0.1270,linecolor=red](1.2869,7.9544)(2.1336,7.3194)(2.9803,7.3194)(3.8269,7.9544)
\psbezier[linewidth=0.0318,linestyle=dashed,dash=0.1270 0.1270,linecolor=red](3.8269,7.9544)(4.6736,8.5894)(4.6736,9.1186)(3.8269,9.5419)
\pscurve[linewidth=0.0159,linecolor=black]{<->}(0.0169,5.0969)(2.5569,3.1919)(5.0969,5.0969)
\psline[linewidth=0.0159,linecolor=black]{<->}(0.0169,1.2869)(2.5569,0.0169)
% declare gray 10
\newgray{grayx}{0.5000}%
\psecurve[linewidth=0.0159,linestyle=dotted,dotsep=0.0953,linecolor=blue,fillstyle=solid,fillcolor=grayx](6.3669,5.0969)(8.9069,3.1919)(11.4469,5.0969)(6.3669,5.0969)
	(8.9069,3.1919)
\end{pspicture}
"##
    );
    assert_eq!(exit, 0);
    assert_eq!(
        msgs,
        vec![
            "IMPORTANT notes from pstricks driver:",
            "  Driver never tested with old splines. Send examples to ressler@usma.edu."
        ]
    );
}

#[test]
fn grid_and_margins() {
    let body = "2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 600 600 1800 600 1800 1200 600 1200 600 600\n";
    let (out, msgs, _) = run("pstricks", body, &[('G', Some("1:1")), ('x', Some("1")), ('P', None)]).unwrap();
    assert!(out.contains("\\begin{pspicture}(-1.0000,-0.0000)"), "{out}");
    assert!(out.contains("\\psgrid["), "{out}");
    assert!(msgs.iter().any(|m| m.contains("Grid sizes are ignored")), "{msgs:?}");
}

#[test]
fn bad_options() {
    let mut ctx = rustfig_fig2dev::Ctx::default();
    let mut dev = rustfig_fig2dev::drivers::driver("pstricks").unwrap();
    assert!(dev.option(&mut ctx, 'Q', None).is_err());
    assert!(dev.option(&mut ctx, 'S', Some("13")).is_err());
    assert!(dev.option(&mut ctx, 'S', None).is_err());
    assert!(dev.option(&mut ctx, 't', Some("x")).is_ok());
    assert!(ctx
        .messages
        .last()
        .unwrap()
        .contains("bad PSTricks version 'x' was ignored"));
    assert!(dev.option(&mut ctx, 'l', Some("3")).is_ok());
    assert!(ctx.messages.last().unwrap().contains("bad line weight 3"));
}

#[test]
fn hostile_values_dont_panic() {
    // Huge coordinates, radii and thicknesses, one-point and degenerate
    // lines, odd arrows, arcs with equal points, bad colours.
    let bodies = [
        "2 1 1 1 0 7 50 -1 -1 0.001 0 0 -1 1 1 2
	1 1 1.00 60.00 120.00
	1 1 1.00 60.00 120.00
	 0 0 2147483647 -2147483648
2 2 3 1 0 7 50 -1 -1 0.010 0 0 -1 0 0 5
	 -2147483648 0 2147483647 0 2147483647 9 0 9 0 0
2 4 1 1 0 7 50 -1 -1 4.0 0 0 0 0 0 5
	 0 0 0 0 0 0 0 0 0 0
1 1 1 1 0 7 50 -1 -1 4.0 1 1e300 0 0 5 5 0 0 0 0
",
        "5 1 2 4 7 2 0 -1 0 0 0 0 1 1 79E6 12003420 79196029 3765 5984 3270 6254 51752685
	1 1 2 120 240
	1 1 2 120 240
5 1 0 1 0 7 50 -1 -1 0.0 0 0 1 0 0.0 0.0 600 0 0 600 -600 0
	1 1 1.0 60.0 0.0
5 1 0 1 0 7 50 -1 -1 0.0 0 0 1 1 0.0 0.0 0 0 0 0 0 0
	14 1 1.0 60.0 0.0
	14 1 1.0 60.0 0.0
",
        "2 1 0 1 0 7 50 -1 -1 0.0 0 0 -1 1 1 1
	14 1 1.0 -60.0 -10.0
	14 0 1.0 60.0 0.0
	 -2147483648
2 3 0 1 0 7 50 -1 -1 0.0 0 0 -1 0 0 3
	 0 0 0 0 0 0
2 3 0 1 99 99 50 -1 25 0.0 0 0 -1 0 0 4
	 0 0 100 0 100 100 0 0
4 1 0 50 -1 0 12 1e300 4 135 600 -2147483648 2147483647 x\\001
",
    ];
    for body in bodies {
        for opts in [
            &[][..],
            &[('P', None), ('v', None)][..],
            &[('R', Some("1")), ('x', Some("100"))][..],
            &[('m', Some("0")), ('n', Some("1"))][..],
            &[('G', Some("1:1")), ('z', Some("2"))][..],
        ] {
            let _ = run("pstricks", body, opts);
        }
    }
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test pstricks -- --ignored --nocapture`.
#[test]
#[ignore]
fn same_as_fig2dev() {
    let extra: Vec<&str> = HAND.to_vec();
    let opts: &[&[&str]] = &[
        &[],
        &["-R", "1", "-v", "-n", "2"],
        &["-R", "2", "-P", "-x", "1", "-y", "1"],
        &["-z", "1", "-l", "1", "-S", "10", "-f", "bf"],
        &["-G", "0.5:1", "-t", "1.1", "-z", "2", "-n", "1"],
    ];
    if let Some((same, total)) = compare_with_fig2dev("pstricks", &extra, opts) {
        assert!(same * 20 >= total * 19, "{same} of {total} identical");
    }
}

fn pic_fig(path: &str) -> String {
    format!("2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5\n\t0 {path}\n\t 600 600 1800 600 1800 1200 600 1200 600 600\n")
}

#[test]
fn p_option_converts_png_and_jpeg_to_eps() {
    let dir = std::env::temp_dir().join(format!("pst-p-{}", std::process::id()));
    let out_dir = dir.join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    let img = image::RgbaImage::from_fn(5, 3, |x, y| {
        image::Rgba([(x * 50) as u8, (y * 80) as u8, 200, if x == 4 { 0 } else { 255 }])
    });
    let png = dir.join("a.png");
    img.save(&png).unwrap();
    let jpg = dir.join("b.jpg");
    image::DynamicImage::ImageRgba8(img).to_rgb8().save(&jpg).unwrap();
    let body = format!("{}{}", pic_fig(png.to_str().unwrap()), pic_fig(jpg.to_str().unwrap()));
    let (out, msgs, _) = run("pstricks", &body, &[('p', Some(out_dir.to_str().unwrap()))]).unwrap();
    assert!(out.contains("out/a}") && out.contains("out/b}"), "{out}");
    assert!(!msgs.iter().any(|m| m.contains("probably failed")), "{msgs:?}");
    for (name, w) in [("a", 5), ("b", 5)] {
        let eps = std::fs::read_to_string(out_dir.join(format!("{name}.eps"))).unwrap();
        assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0\n"));
        assert!(eps.contains(&format!("%%BoundingBox: 0 0 {w} 3\n")));
        assert!(eps.contains("colorimage"));
        // drawn one point per pixel, not into the unit square
        assert!(eps.contains(&format!("\n{w} 3 scale\n")), "{eps}");
        // 5 x 3 pixels, 6 hex digits each
        let data = eps
            .split("colorimage\n")
            .nth(1)
            .unwrap()
            .split("\ngrestore")
            .next()
            .unwrap();
        assert_eq!(data.replace('\n', "").len(), 90);
    }
    // the transparent column is white in the png
    let eps = std::fs::read_to_string(out_dir.join("a.eps")).unwrap();
    assert!(eps.contains("ffffff"));
    if let Ok(keep) = std::env::var("KEEP_PST_EPS") {
        std::fs::copy(out_dir.join("a.eps"), keep).unwrap();
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn p_option_warns_on_a_corrupt_picture() {
    let dir = std::env::temp_dir().join(format!("pst-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("c.png");
    std::fs::write(&bad, b"\x89PNG\r\n\x1a\nnot really").unwrap();
    let (out, msgs, _) = run(
        "pstricks",
        &pic_fig(bad.to_str().unwrap()),
        &[('p', Some(dir.to_str().unwrap()))],
    )
    .unwrap();
    assert!(out.contains("/c}"), "{out}");
    assert!(msgs.iter().any(|m| m.contains("probably failed")), "{msgs:?}");
    assert!(!dir.join("c.eps").exists());
    std::fs::remove_dir_all(&dir).ok();
}
