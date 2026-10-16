//! fig2dev's `pstex_t` and `pdftex_t` drivers: the LaTeX half of the
//! combined PostScript/LaTeX and PDF/LaTeX exports. They write a LaTeX
//! `picture` that includes the graphics file (`-p file`, written by the
//! `pstex` or `pdftex` language: the figure without its special texts) and
//! puts the special (LaTeX) texts over it, as the `latex` driver sets
//! texts. Every other object is left to the graphics file.
//!
//! fig2dev is built with `LATEX2E_GRAPHICS`, so the graphics are included
//! with `\includegraphics`. The two languages are the same driver.
//
// Ported from fig2dev dev/genpstex.c (xfig licence, see LICENSE-xfig).

use crate::drivers::latex::Latex;
use crate::object::{Arc, Compound, Ellipse, Line, Spline, Text};
use crate::{fprintf, Ctx, Driver, Out};

/// The driver for `lang` (`pstex_t` or `pdftex_t`).
pub fn new(lang: &str) -> Option<Box<dyn Driver>> {
    matches!(lang, "pstex_t" | "pdftex_t").then(|| Box::new(PsTexT::default()) as Box<dyn Driver>)
}

/// `dev_pstex_t`, `dev_pdftex_t`.
#[derive(Debug, Default)]
pub struct PsTexT {
    latex: Latex,
    /// `-p`: the graphics file to include (`pstex_file`); none if empty.
    pstex_file: String,
}

impl Driver for PsTexT {
    /// `genpstex_t_option`.
    fn option(&mut self, ctx: &mut Ctx, opt: char, arg: Option<&str>) -> Result<(), String> {
        if opt == 'p' {
            // C copies into a 1000-byte buffer.
            self.pstex_file = arg.unwrap_or("").chars().take(999).collect();
            Ok(())
        } else {
            self.latex.option(ctx, opt, arg)
        }
    }

    /// `genpstex_t_start`.
    fn start(&mut self, ctx: &mut Ctx, out: &mut Out, objects: &Compound) -> Result<(), String> {
        if !self.pstex_file.is_empty() {
            out.puts("\\begin{picture}(0,0)%\n");
            fprintf!(out, "\\includegraphics{{{}}}%\n", self.pstex_file);
            out.puts("\\end{picture}%\n");
        }
        self.latex.start(ctx, out, objects)
    }

    fn arc(&mut self, _: &mut Ctx, _: &mut Out, _: &Arc) -> Result<(), String> {
        Ok(())
    }

    fn ellipse(&mut self, _: &mut Ctx, _: &mut Out, _: &Ellipse) -> Result<(), String> {
        Ok(())
    }

    fn line(&mut self, _: &mut Ctx, _: &mut Out, _: &Line) -> Result<(), String> {
        Ok(())
    }

    fn spline(&mut self, _: &mut Ctx, _: &mut Out, _: &Spline) -> Result<(), String> {
        Ok(())
    }

    /// `genpstex_t_text`: special texts only.
    fn text(&mut self, ctx: &mut Ctx, out: &mut Out, t: &Text) -> Result<(), String> {
        if t.special_text() {
            self.latex.text(ctx, out, t)
        } else {
            Ok(())
        }
    }

    fn end(&mut self, ctx: &mut Ctx, out: &mut Out) -> Result<i32, String> {
        self.latex.end(ctx, out)
    }

    /// `INCLUDE_TEXT`: the box is the graphics file's, texts included.
    fn text_include(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(lang: &str, opts: &[(char, Option<&str>)], fig: &str) -> String {
        let mut ctx = Ctx {
            lang: lang.into(),
            ..Ctx::default()
        };
        let mut d = new(lang).unwrap();
        for &(c, a) in opts {
            d.option(&mut ctx, c, a).unwrap();
        }
        let objs = crate::read::readfp_fig(&mut ctx, fig.as_bytes()).unwrap();
        let st = crate::convert(&mut ctx, d.as_mut(), &objs).unwrap();
        String::from_utf8(st.output).unwrap()
    }

    const FIG: &str = "#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n\
        2 2 0 1 0 7 50 -1 -1 0.000 0 0 -1 0 0 5\n\t 0 0 1200 0 1200 1200 0 1200 0 0\n\
        4 0 0 50 -1 0 12 0.0000 2 135 300 300 600 $x^2$\\001\n\
        4 0 0 50 -1 0 12 0.0000 4 135 300 300 900 plain\\001\n";

    #[test]
    fn includes_the_graphics_and_only_special_text() {
        for lang in ["pstex_t", "pdftex_t"] {
            let s = run(lang, &[('p', Some("fig.pdf"))], FIG);
            assert!(
                s.starts_with("\\begin{picture}(0,0)%\n\\includegraphics{fig.pdf}%\n\\end{picture}%\n"),
                "{s}"
            );
            assert!(s.contains("$x^2$"), "{s}");
            assert!(!s.contains("plain"), "{s}");
            assert!(!s.contains("\\put(0,0){\\line"), "lines are the graphics file's: {s}");
            assert!(s.ends_with("\\end{picture}%\n"), "{s}");
        }
        let s = run("pstex_t", &[], FIG);
        assert!(!s.contains("includegraphics"), "{s}");
    }

    #[test]
    fn other_options_go_to_latex() {
        let mut ctx = Ctx::default();
        let mut d = new("pstex_t").unwrap();
        assert!(d.option(&mut ctx, 'v', None).is_ok());
        assert!(d
            .option(&mut ctx, 'q', None)
            .unwrap_err()
            .contains("unknown to latex driver"));
        assert!(new("pstex").is_none());
    }
}
