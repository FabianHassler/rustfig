#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! fig2dev's `tikz` driver against output of the real fig2dev 3.2.9a
//! (the expected texts were written by `/opt/homebrew/bin/fig2dev -L
//! tikz`).

mod pic_common;
mod tex_common;

use tex_common::{compare_with_fig2dev, run, HAND};

#[test]
fn dashed_polyline_with_arrows() {
    let body = r#"2 1 1 2 0 7 50 -1 -1 4.000 0 0 -1 1 1 4
	1 1 1.00 60.00 120.00
	2 1 1.00 60.00 120.00
	 600 600 1800 900 2400 2400 3000 2450
"#;
    let (out, msgs, exit) = run("tikz", body, &[]).unwrap();
    assert_eq!(
        out,
        r#"{\pgfkeys{/pgf/fpu/.try=false}%
\ifx\XFigwidth\undefined\dimen1=0pt\else\dimen1\XFigwidth\fi
\divide\dimen1 by 2444
\ifx\XFigheight\undefined\dimen3=0pt\else\dimen3\XFigheight\fi
\divide\dimen3 by 1913
\ifdim\dimen1=0pt\ifdim\dimen3=0pt\dimen1=3946sp\dimen3\dimen1
  \else\dimen1\dimen3\fi\else\ifdim\dimen3=0pt\dimen3\dimen1\fi\fi
\tikzpicture[x=+\dimen1, y=+\dimen3]
{\ifx\XFigu\undefined\catcode`\@11
\def\temp{\alloc@1\dimen\dimendef\insc@unt}\temp\XFigu\catcode`\@12\fi}
\XFigu3946sp
% Uncomment to scale line thicknesses with the same
% factor as width of the drawing.
%\pgfextractx\XFigu{\pgfqpointxy{1}{1}}
\ifdim\XFigu<0pt\XFigu-\XFigu\fi
\pgfdeclarearrow{
  name = xfiga1,
  parameters = {
    \the\pgfarrowlinewidth \the\pgfarrowlength \the\pgfarrowwidth\ifpgfarrowopen o\fi},
  defaults = {
	  line width=+7.5\XFigu, length=+120\XFigu, width=+60\XFigu},
  setup code = {
    % miter protrusion = thk * sqrt(wd^2 + (tipmv*len)^2) / (2 * wd)
    \dimen7 2.1\pgfarrowlength\pgfmathveclen{\the\dimen7}{\the\pgfarrowwidth}
    \dimen7 2\pgfarrowwidth\pgfmathdivide{\pgfmathresult}{\the\dimen7}
    \dimen7 \pgfmathresult\pgfarrowlinewidth
    \pgfarrowssettipend{+\dimen7}
    \pgfarrowssetbackend{+-\pgfarrowlength}
    \dimen9 -\pgfarrowlength\advance\dimen9 by-0.45\pgfarrowlinewidth
    \pgfarrowssetlineend{+\dimen9}
    \dimen9 -\pgfarrowlength\advance\dimen9 by-0.5\pgfarrowlinewidth
    \pgfarrowssetvisualbackend{+\dimen9}
    \pgfarrowshullpoint{+\dimen7}{+0pt}
    \pgfarrowsupperhullpoint{+-\pgfarrowlength}{+0.5\pgfarrowwidth}
    \pgfarrowssavethe\pgfarrowlinewidth
    \pgfarrowssavethe\pgfarrowlength
    \pgfarrowssavethe\pgfarrowwidth
  },
  drawing code = {\pgfsetdash{}{+0pt}
    \ifdim\pgfarrowlinewidth=\pgflinewidth\else\pgfsetlinewidth{+\pgfarrowlinewidth}\fi
    \pgfpathmoveto{\pgfqpoint{-\pgfarrowlength}{-0.5\pgfarrowwidth}}
    \pgfpathlineto{\pgfqpoint{0pt}{0pt}}
    \pgfpathlineto{\pgfqpoint{-\pgfarrowlength}{0.5\pgfarrowwidth}}
    \pgfpathclose
    \ifpgfarrowopen\pgfusepathqstroke\else\pgfsetfillcolor{pgfstrokecolor}
	\ifdim\pgfarrowlinewidth>0pt\pgfusepathqfillstroke\else\pgfusepathqfill\fi\fi
  }
}
\pgfdeclarearrow{
  name = xfiga2,
  parameters = {
    \the\pgfarrowlinewidth \the\pgfarrowlength \the\pgfarrowwidth\ifpgfarrowopen o\fi},
  defaults = {
	  line width=+7.5\XFigu, length=+120\XFigu, width=+60\XFigu},
  setup code = {
    % miter protrusion = thk * sqrt(wd^2 + (tipmv*len)^2) / (2 * wd)
    \dimen7 2.6\pgfarrowlength\pgfmathveclen{\the\dimen7}{\the\pgfarrowwidth}
    \dimen7 2\pgfarrowwidth\pgfmathdivide{\pgfmathresult}{\the\dimen7}
    \dimen7 \pgfmathresult\pgfarrowlinewidth
    \pgfarrowssettipend{+\dimen7}
    \pgfarrowssetbackend{+-1.25\pgfarrowlength}
    \dimen9 -\pgfarrowlength\advance\dimen9 by-0.5\pgfarrowlinewidth
    \pgfarrowssetlineend{+\dimen9}
    \dimen9 -\pgfarrowlength\advance\dimen9 by-0.5\pgfarrowlinewidth
    \pgfarrowssetvisualbackend{+\dimen9}
    \pgfarrowshullpoint{+\dimen7}{+0pt}
    \pgfarrowsupperhullpoint{+-1.25\pgfarrowlength}{+0.5\pgfarrowwidth}
    \pgfarrowssavethe\pgfarrowlinewidth
    \pgfarrowssavethe\pgfarrowlength
    \pgfarrowssavethe\pgfarrowwidth
  },
  drawing code = {\pgfsetdash{}{+0pt}
    \ifdim\pgfarrowlinewidth=\pgflinewidth\else\pgfsetlinewidth{+\pgfarrowlinewidth}\fi
    \pgfpathmoveto{\pgfqpoint{-1.25\pgfarrowlength}{-0.5\pgfarrowwidth}}
    \pgfpathlineto{\pgfqpoint{0pt}{0pt}}
    \pgfpathlineto{\pgfqpoint{-1.25\pgfarrowlength}{0.5\pgfarrowwidth}}
    \pgfpathlineto{\pgfqpoint{-\pgfarrowlength}{0pt}}
    \pgfpathclose
    \ifpgfarrowopen\pgfusepathqstroke\else\pgfsetfillcolor{pgfstrokecolor}
	\ifdim\pgfarrowlinewidth>0pt\pgfusepathqfillstroke\else\pgfusepathqfill\fi\fi
  }
}
\clip(578,-2491) rectangle (3022,-578);
\tikzset{inner sep=+0pt, outer sep=+0pt}
\pgfsetarrows{[line width=7.5\XFigu]}
\pgfsetarrows{xfiga2-xfiga1}
\pgfsetlinewidth{+15\XFigu}
\pgfsetdash{{+60\XFigu}{+60\XFigu}}{++0pt}
\pgfsetstrokecolor{black}
\draw (600,-600)--(1800,-900)--(2400,-2400)--(3000,-2450);
\endtikzpicture}%
"#
    );
    let expected: &[&str] = &[];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn filled_and_pattern_polygons() {
    let body = r#"0 40 #12ab34
2 3 0 1 4 7 50 -1 25 0.000 0 0 -1 0 0 4
	 600 600 1800 600 1200 1500 600 600
2 3 2 2 40 11 50 -1 43 3.000 1 1 -1 0 0 4
	 2600 600 3800 600 3200 1500 2600 600
2 2 0 0 0 0 50 -1 10 0.000 0 0 -1 0 0 5
	 600 2000 1800 2000 1800 2600 600 2600 600 2000
"#;
    let (out, msgs, exit) = run("tikz", body, &[]).unwrap();
    assert_eq!(
        out,
        r#"{\pgfkeys{/pgf/fpu/.try=false}%
\ifx\XFigwidth\undefined\dimen1=0pt\else\dimen1\XFigwidth\fi
\divide\dimen1 by 3234
\ifx\XFigheight\undefined\dimen3=0pt\else\dimen3\XFigheight\fi
\divide\dimen3 by 2023
\ifdim\dimen1=0pt\ifdim\dimen3=0pt\dimen1=3946sp\dimen3\dimen1
  \else\dimen1\dimen3\fi\else\ifdim\dimen3=0pt\dimen3\dimen1\fi\fi
\tikzpicture[x=+\dimen1, y=+\dimen3]
{\ifx\XFigu\undefined\catcode`\@11
\def\temp{\alloc@1\dimen\dimendef\insc@unt}\temp\XFigu\catcode`\@12\fi}
\XFigu3946sp
% Uncomment to scale line thicknesses with the same
% factor as width of the drawing.
%\pgfextractx\XFigu{\pgfqpointxy{1}{1}}
\ifdim\XFigu<0pt\XFigu-\XFigu\fi
\catcode`\@11
\pgfutil@ifundefined{pgf@pattern@name@xfigp2}{
% crosshatch 30 degree
\pgfdeclarepatternformonly{xfigp2}
{\pgfqpoint{-1bp}{-1bp}}{\pgfqpoint{9bp}{5bp}}{\pgfqpoint{8bp}{4bp}}
{	\pgfsetdash{}{0pt}\pgfsetlinewidth{0.45bp}
	\pgfpathqmoveto{-1bp}{-0.5bp}\pgfpathqlineto{9bp}{4.5bp}
	\pgfpathqmoveto{-1bp}{4.5bp}\pgfpathqlineto{9bp}{-0.5bp}
	\pgfusepathqstroke
}
}{}
\catcode`\@12
\definecolor{blue4}{rgb}{0.53,0.81,1}
\definecolor{xfigc32}{rgb}{0.071,0.671,0.204}
\clip(588,-2601) rectangle (3822,-578);
\tikzset{inner sep=+0pt, outer sep=+0pt}
\pgfsetlinewidth{+7.5\XFigu}
\pgfsetstrokecolor{red}
\pgfsetfillcolor{white}
\filldraw (600,-600)--(1800,-600)--(1200,-1500)--cycle;
\pgfsetroundjoin
\pgfsetroundcap
\pgfsetlinewidth{+15\XFigu}
\pgfsetdash{{+15\XFigu}{+45\XFigu}}{+15\XFigu}
\pgfsetstrokecolor{xfigc32}
\pgfsetfillpattern{xfigp2}{xfigc32}
\draw[pattern,preaction={fill=blue4}] (2600,-600)--(3800,-600)--(3200,-1500)--cycle;
\pgfsetmiterjoin
\pgfsetfillcolor{black!50}
\fill (600,-2000) rectangle (1800,-2600);
\endtikzpicture}%
"#
    );
    let expected: &[&str] = &[];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn arc_box_ellipses_and_arcs() {
    let body = r#"2 4 0 3 5 7 50 -1 -1 0.000 2 1 8 0 0 5
	 600 600 2400 600 2400 1800 600 1800 600 600
1 1 0 1 0 7 50 -1 -1 0.000 1 0.0000 3000 1200 300 300 3000 1200 3300 1200
1 2 1 2 1 2 50 -1 20 4.000 1 0.5236 4200 1200 500 300 4200 1200 4700 1200
5 1 0 2 0 7 50 -1 -1 0.000 0 1 1 1 1500 3000 900 3000 1500 2400 2100 3000
	1 1 2.00 90.00 150.00
	2 1 1.00 60.00 120.00
5 2 0 1 0 7 50 -1 30 0.000 0 0 0 0 4000 3000 3400 3000 4000 2400 4600 3000
"#;
    let (out, msgs, exit) = run("tikz", body, &[]).unwrap();
    assert_eq!(
        out,
        r#"{\pgfkeys{/pgf/fpu/.try=false}%
\ifx\XFigwidth\undefined\dimen1=0pt\else\dimen1\XFigwidth\fi
\divide\dimen1 by 4106
\ifx\XFigheight\undefined\dimen3=0pt\else\dimen3\XFigheight\fi
\divide\dimen3 by 3049
\ifdim\dimen1=0pt\ifdim\dimen3=0pt\dimen1=3946sp\dimen3\dimen1
  \else\dimen1\dimen3\fi\else\ifdim\dimen3=0pt\dimen3\dimen1\fi\fi
\tikzpicture[x=+\dimen1, y=+\dimen3]
{\ifx\XFigu\undefined\catcode`\@11
\def\temp{\alloc@1\dimen\dimendef\insc@unt}\temp\XFigu\catcode`\@12\fi}
\XFigu3946sp
% Uncomment to scale line thicknesses with the same
% factor as width of the drawing.
%\pgfextractx\XFigu{\pgfqpointxy{1}{1}}
\ifdim\XFigu<0pt\XFigu-\XFigu\fi
\pgfdeclarearrow{
  name = xfiga1,
  parameters = {
    \the\pgfarrowlinewidth \the\pgfarrowlength \the\pgfarrowwidth\ifpgfarrowopen o\fi},
  defaults = {
	  line width=+7.5\XFigu, length=+120\XFigu, width=+60\XFigu},
  setup code = {
    % miter protrusion = thk * sqrt(wd^2 + (tipmv*len)^2) / (2 * wd)
    \dimen7 2.1\pgfarrowlength\pgfmathveclen{\the\dimen7}{\the\pgfarrowwidth}
    \dimen7 2\pgfarrowwidth\pgfmathdivide{\pgfmathresult}{\the\dimen7}
    \dimen7 \pgfmathresult\pgfarrowlinewidth
    \pgfarrowssettipend{+\dimen7}
    \pgfarrowssetbackend{+-\pgfarrowlength}
    \dimen9 -\pgfarrowlength\advance\dimen9 by-0.45\pgfarrowlinewidth
    \pgfarrowssetlineend{+\dimen9}
    \dimen9 -\pgfarrowlength\advance\dimen9 by-0.5\pgfarrowlinewidth
    \pgfarrowssetvisualbackend{+\dimen9}
    \pgfarrowshullpoint{+\dimen7}{+0pt}
    \pgfarrowsupperhullpoint{+-\pgfarrowlength}{+0.5\pgfarrowwidth}
    \pgfarrowssavethe\pgfarrowlinewidth
    \pgfarrowssavethe\pgfarrowlength
    \pgfarrowssavethe\pgfarrowwidth
  },
  drawing code = {\pgfsetdash{}{+0pt}
    \ifdim\pgfarrowlinewidth=\pgflinewidth\else\pgfsetlinewidth{+\pgfarrowlinewidth}\fi
    \pgfpathmoveto{\pgfqpoint{-\pgfarrowlength}{-0.5\pgfarrowwidth}}
    \pgfpathlineto{\pgfqpoint{0pt}{0pt}}
    \pgfpathlineto{\pgfqpoint{-\pgfarrowlength}{0.5\pgfarrowwidth}}
    \pgfpathclose
    \ifpgfarrowopen\pgfusepathqstroke\else\pgfsetfillcolor{pgfstrokecolor}
	\ifdim\pgfarrowlinewidth>0pt\pgfusepathqfillstroke\else\pgfusepathqfill\fi\fi
  }
}
\pgfdeclarearrow{
  name = xfiga2,
  parameters = {
    \the\pgfarrowlinewidth \the\pgfarrowlength \the\pgfarrowwidth\ifpgfarrowopen o\fi},
  defaults = {
	  line width=+7.5\XFigu, length=+120\XFigu, width=+60\XFigu},
  setup code = {
    % miter protrusion = thk * sqrt(wd^2 + (tipmv*len)^2) / (2 * wd)
    \dimen7 2.6\pgfarrowlength\pgfmathveclen{\the\dimen7}{\the\pgfarrowwidth}
    \dimen7 2\pgfarrowwidth\pgfmathdivide{\pgfmathresult}{\the\dimen7}
    \dimen7 \pgfmathresult\pgfarrowlinewidth
    \pgfarrowssettipend{+\dimen7}
    \pgfarrowssetbackend{+-1.25\pgfarrowlength}
    \dimen9 -\pgfarrowlength\advance\dimen9 by-0.5\pgfarrowlinewidth
    \pgfarrowssetlineend{+\dimen9}
    \dimen9 -\pgfarrowlength\advance\dimen9 by-0.5\pgfarrowlinewidth
    \pgfarrowssetvisualbackend{+\dimen9}
    \pgfarrowshullpoint{+\dimen7}{+0pt}
    \pgfarrowsupperhullpoint{+-1.25\pgfarrowlength}{+0.5\pgfarrowwidth}
    \pgfarrowssavethe\pgfarrowlinewidth
    \pgfarrowssavethe\pgfarrowlength
    \pgfarrowssavethe\pgfarrowwidth
  },
  drawing code = {\pgfsetdash{}{+0pt}
    \ifdim\pgfarrowlinewidth=\pgflinewidth\else\pgfsetlinewidth{+\pgfarrowlinewidth}\fi
    \pgfpathmoveto{\pgfqpoint{-1.25\pgfarrowlength}{-0.5\pgfarrowwidth}}
    \pgfpathlineto{\pgfqpoint{0pt}{0pt}}
    \pgfpathlineto{\pgfqpoint{-1.25\pgfarrowlength}{0.5\pgfarrowwidth}}
    \pgfpathlineto{\pgfqpoint{-\pgfarrowlength}{0pt}}
    \pgfpathclose
    \ifpgfarrowopen\pgfusepathqstroke\else\pgfsetfillcolor{pgfstrokecolor}
	\ifdim\pgfarrowlinewidth>0pt\pgfusepathqfillstroke\else\pgfusepathqfill\fi\fi
  }
}
\clip(567,-3616) rectangle (4673,-567);
\tikzset{inner sep=+0pt, outer sep=+0pt}
\pgfsetarrows{[line width=7.5\XFigu]}
\pgfsetarrows{xfiga2-xfiga1[line width=15*\the\XFigu, width=90*\the\XFigu, length=150*\the\XFigu]}
\pgfsetlinewidth{+15\XFigu}
\pgfsetstrokecolor{black}
\draw (900,-3000) arc[start angle=+-180.00, end angle=+-0.00, radius=+600];
\pgfsetarrows{-}
\pgfsetlinewidth{+7.5\XFigu}
\pgfsetfillcolor{white}
\filldraw (3400,-3000) arc[start angle=+180.00, end angle=+-0.00, radius=+600]--(4000,-3000)--cycle;
\draw  (3000,-1200) circle [radius=+300];
\pgfsetroundcap
\pgfsetlinewidth{+15\XFigu}
\pgfsetdash{{+60\XFigu}{+60\XFigu}}{++0pt}
\pgfsetstrokecolor{blue}
\pgfsetfillcolor{green}
\filldraw  (4200,-1200) ellipse [x radius=+500,y radius=+300,rotate=+30];
\pgfsetlinewidth{+30\XFigu}
\pgfsetdash{}{+0pt}
\pgfsetstrokecolor{magenta}
\draw (600,-600) [rounded corners=+120\XFigu] rectangle (2400,-1800);
\endtikzpicture}%
"#
    );
    let expected: &[&str] = &[];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn rotated_text_and_colors() {
    let body = r#"0 35 #ff8000
4 0 4 50 -1 0 12 0.0000 4 135 600 600 600 Hello\001
4 1 35 50 -1 16 14 0.7854 6 135 900 2000 1500 Rot & text\001
4 2 1 50 -1 4 10 0.0000 2 135 300 3000 2200 100%\001
"#;
    let (out, msgs, exit) = run("tikz", body, &[]).unwrap();
    assert_eq!(
        out,
        r#"{\pgfkeys{/pgf/fpu/.try=false}%
\ifx\XFigwidth\undefined\dimen1=0pt\else\dimen1\XFigwidth\fi
\divide\dimen1 by 2430
\ifx\XFigheight\undefined\dimen3=0pt\else\dimen3\XFigheight\fi
\divide\dimen3 by 1765
\ifdim\dimen1=0pt\ifdim\dimen3=0pt\dimen1=3946sp\dimen3\dimen1
  \else\dimen1\dimen3\fi\else\ifdim\dimen3=0pt\dimen3\dimen1\fi\fi
\tikzpicture[x=+\dimen1, y=+\dimen3]
{\ifx\XFigu\undefined\catcode`\@11
\def\temp{\alloc@1\dimen\dimendef\insc@unt}\temp\XFigu\catcode`\@12\fi}
\XFigu3946sp
% Uncomment to scale line thicknesses with the same
% factor as width of the drawing.
%\pgfextractx\XFigu{\pgfqpointxy{1}{1}}
\ifdim\XFigu<0pt\XFigu-\XFigu\fi
\definecolor{xfigc32}{rgb}{1.000,0.502,0.000}
\clip(585,-2215) rectangle (3015,-450);
\tikzset{inner sep=+0pt, outer sep=+0pt}
\pgfsetfillcolor{red}
\pgftext[base,left,at=\pgfqpointxy{600}{-600}] {\fontsize{12}{14.4}\usefont{T1}{ptm}{m}{n}Hello}
\pgfsetfillcolor{xfigc32}
\pgftext[base,at=\pgfqpointxy{2000}{-1500},rotate=+45] {\fontsize{14}{16.8}\usefont{T1}{phv}{m}{n}Rot & text}
\pgfsetfillcolor{blue}
\pgftext[base,right,at=\pgfqpointxy{3000}{-2200}] {\fontsize{10}{12}\normalfont\sffamily 100%}
\endtikzpicture}%
"#
    );
    let expected: &[&str] = &[];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn dotted_lines_caps_and_joins() {
    let body = r#"2 1 3 3 0 7 50 -1 -1 6.000 1 1 0 0 0 3
	 600 600 1800 900 2400 600
2 1 4 1 2 7 50 -1 -1 8.000 2 2 7 0 0 3
	 600 1200 1800 1500 2400 1200
2 1 5 1 3 7 50 -1 -1 8.000 0 0 7 0 0 2
	 600 1800 1800 1800
2 1 0 2 0 7 50 -1 -1 0.000 0 0 7 0 0 1
	 900 2400
"#;
    let (out, msgs, exit) = run("tikz", body, &[('P', None), ('v', None)]).unwrap();
    assert_eq!(
        out,
        r#"\documentclass{minimal}
\usepackage[papersize={1.556in,1.547in},margin=0pt]{geometry}
\usepackage{tikz}
\parindent0pt
\begin{document}
\tikzpicture[x=+3946sp, y=+3946sp]
\newdimen\XFigu\XFigu3946sp
\newdimen\XFigu\XFigu3946sp
\clip(567,-2422) rectangle (2433,-567);
\tikzset{inner sep=+0pt, outer sep=+0pt}
%
% Fig POLYLINE object
%
\pgfsetroundjoin
\pgfsetroundcap
\pgfsetlinewidth{+30\XFigu}
\pgfsetdash{{+90\XFigu}{+45\XFigu}{+15\XFigu}{+45\XFigu}}{+0pt}
\pgfsetstrokecolor{black}
\draw (600,-600)--(1800,-900)--(2400,-600);
%
% Fig POLYLINE object
%
\pgfsetbeveljoin
\pgfsetrectcap
\pgfsetlinewidth{+7.5\XFigu}
\pgfsetdash{{+120\XFigu}{+54\XFigu}{+15\XFigu}{+40\XFigu}{+15\XFigu}{+54\XFigu}}{+0pt}
\pgfsetstrokecolor{green}
\draw (600,-1200)--(1800,-1500)--(2400,-1200);
%
% Fig POLYLINE object
%
\pgfsetstrokecolor{cyan}
\pgfsetbuttcap
\pgfsetdash{{+120\XFigu}{+48\XFigu}{+15\XFigu}{+36\XFigu}{+15\XFigu}{+36\XFigu}{+15\XFigu}{+48\XFigu}}{+0pt}
\draw (600,-1800)--(1800,-1800);
%
% Fig POLYLINE object
%
\pgfsetlinewidth{+15\XFigu}
\pgfsetstrokecolor{black}
\draw (893,-2400)--(908,-2400);
\endtikzpicture%
\end{document}"#
    );
    let expected: &[&str] = &[];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn pictures_and_boxes() {
    let body = r#"2 5 0 1 0 -1 50 -1 -1 0.000 0 0 -1 0 0 5
	0 pic.png
	 600 600 1800 600 1800 1500 600 1500 600 600
2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5
	 2000 600 3000 600 3000 1500 2000 1500 2000 600
"#;
    let (out, msgs, exit) = run("tikz", body, &[('w', None), ('i', Some("figs/")), ('W', None)]).unwrap();
    assert_eq!(
        out,
        r#"{\pgfkeys{/pgf/fpu/.try=false}%
\tikzpicture[x=+3946sp, y=+3946sp]
\newdimen\XFigu\XFigu3946sp
{\ifx\XFigu\undefined\catcode`\@11
\def\temp{\alloc@1\dimen\dimendef\insc@unt}\temp\XFigu\catcode`\@12\fi}
\XFigu3946sp
\clip(600,-1512) rectangle (3012,-588);
\tikzset{inner sep=+0pt, outer sep=+0pt}
\node[below right] at (600,-600) {\pgfimage[width=+1200\XFigu,height=+900\XFigu]{figs/pic}};
\pgfsetlinewidth{+7.5\XFigu}
\pgfsetstrokecolor{black}
\draw (2000,-600) rectangle (3000,-1500);
\endtikzpicture}%
"#
    );
    let expected: &[&str] = &[];
    assert_eq!(msgs, expected);
    assert_eq!(exit, 0);
}

#[test]
fn unknown_option_is_an_error() {
    let r = run("tikz", "", &[('e', None)]);
    assert!(r.is_err() || r.is_ok_and(|(_, _, exit)| exit != 0));
}

#[test]
fn hostile_values_dont_panic() {
    // Huge coordinates and radii, tiny dashes, one-point and degenerate
    // lines, odd arrows, arcs with equal points, negative thickness.
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
4 1 0 50 -1 0 12 1e300 4 135 600 -2147483648 2147483647 x\\001
",
    ];
    for body in bodies {
        for opts in [
            &[][..],
            &[('P', None), ('W', None)][..],
            &[('b', Some("1000000000"))][..],
            &[('m', Some("0"))][..],
            &[('G', Some("1:1"))][..],
        ] {
            let _ = run("tikz", body, opts);
        }
    }
}

/// Opt-in: `cargo test -p rustfig-fig2dev --test tikz -- --ignored --nocapture`.
#[test]
#[ignore]
fn same_as_fig2dev() {
    let extra: Vec<&str> = HAND.to_vec();
    let opts: &[&[&str]] = &[
        &[],
        &["-P", "-b", "20", "-v"],
        &["-W", "-o", "-F", "-T", "-C", "4"],
        &["-G", "0.5:1", "-w", "-i", "dir"],
    ];
    if let Some((same, total)) = compare_with_fig2dev("tikz", &extra, opts) {
        assert!(same * 20 >= total * 19, "{same} of {total} identical");
    }
}
