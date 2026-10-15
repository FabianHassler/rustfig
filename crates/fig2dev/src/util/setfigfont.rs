//! `define_setfigfont`: writes the `\SetFigFont` macro that LaTeX output
//! uses to select fonts. fig2dev is built with the New Font Selection
//! Scheme by default (`NFSS`), so this is the NFSS variant.
//!
//! The macro takes three arguments (five with NFSS): #1 the font size
//! (without `pt`), #2 the baselineskip (without `pt`), #3 the font family,
//! #4 the series, #5 the shape; with `font_size_only`, just the first two.
//
// Ported from fig2dev dev/setfigfont.c (xfig licence, see LICENSE-xfig).

use crate::Out;

/// `define_setfigfont(tfp)`; `font_size_only` is C's global `FontSizeOnly`
/// (false unless a driver sets it).
pub fn define_setfigfont(out: &mut Out, font_size_only: bool) {
    if font_size_only {
        out.puts(
            "%\n\
             \\begingroup\\makeatletter\\ifx\\SetFigFont\\undefined%\n\
             \\gdef\\SetFigFont#1#2{%\n  \
             \\fontsize{#1}{#2pt}%\n  \
             \\selectfont}%\n\
             \\fi\\endgroup%\n",
        );
    } else {
        out.puts(
            "%\n\
             \\begingroup\\makeatletter\\ifx\\SetFigFont\\undefined%\n\
             \\gdef\\SetFigFont#1#2#3#4#5{%\n  \
             \\reset@font\\fontsize{#1}{#2pt}%\n  \
             \\fontfamily{#3}\\fontseries{#4}\\fontshape{#5}%\n  \
             \\selectfont}%\n\
             \\fi\\endgroup%\n",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfss_macro() {
        let mut out = Out::new();
        define_setfigfont(&mut out, false);
        let s = String::from_utf8(out.buf).unwrap();
        assert_eq!(
            s,
            "%\n\\begingroup\\makeatletter\\ifx\\SetFigFont\\undefined%\n\\gdef\\SetFigFont#1#2#3#4#5{%\n  \\reset@font\\fontsize{#1}{#2pt}%\n  \\fontfamily{#3}\\fontseries{#4}\\fontshape{#5}%\n  \\selectfont}%\n\\fi\\endgroup%\n"
        );
        let mut out = Out::new();
        define_setfigfont(&mut out, true);
        let s = String::from_utf8(out.buf).unwrap();
        assert!(s.contains("\\gdef\\SetFigFont#1#2{%\n  \\fontsize{#1}{#2pt}%\n  \\selectfont}%\n"));
    }
}
