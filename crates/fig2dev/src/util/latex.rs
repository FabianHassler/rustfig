//! `dev/genlatex.h`: the colour switching of the LaTeX driver
//! (`set_color`, `reset_color`), declared there and defined in genlatex.c.
//! fig2dev is built with `LATEX2E_GRAPHICS` by default, so colours are
//! written with `\color[rgb]{…}`.
//!
//! C keeps the last colour in a static (`lastcolor`); here it lives in
//! [`LatexColor`], which the driver keeps in its state.
//
// Ported from fig2dev dev/genlatex.h, dev/genlatex.c (xfig licence, see
// LICENSE-xfig).

use crate::fprintf;
use crate::gendev::{Ctx, MAX_USR_COLS, NUM_STD_COLS};
use crate::Out;

/// The 32 standard colours as LaTeX rgb triples (genlatex.c `colors[]`).
pub const LATEX_STD_COLORS: [&str; NUM_STD_COLS] = [
    "0,0,0",     // black
    "0,0,1",     // blue
    "0,1,0",     // green
    "0,1,1",     // cyan
    "1,0,0",     // red
    "1,0,1",     // magenta
    "1,1,0",     // yellow
    "1,1,1",     // white
    "0,0,.56",   // dk blue
    "0,0,.69",   // md blue
    "0,0,.82",   // lt blue
    ".53,.81,1", // blue4
    "0,.56,0",   // dk green
    "0,.69,0",   // md green
    "0,.82,0",   // lt green
    "0,.56,.56", // dk cyan
    "0,.69,.69", // md cyan
    "0,.82,.82", // lt cyan
    ".56,0,0",   // dk red
    ".69,0,0",   // md red
    ".82,0,0",   // lt red
    ".56,0,.56", // dk magenta
    ".69,0,.69", // md magenta
    ".82,0,.82", // lt magenta
    ".5,.17,0",  // dk brown
    ".63,.25,0", // md brown1
    ".75,.38,0", // md brown1
    "1,.5,.5",   // dk pink
    "1,.63,.63", // md pink1
    "1,.75,.75", // md pink2
    "1,.88,.88", // lt pink
    "1,.84,0",   // gold
];

/// The colour state of `set_color` / `reset_color` (`lastcolor`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LatexColor {
    /// The colour switched to, −1 for none.
    pub lastcolor: i32,
}

impl Default for LatexColor {
    fn default() -> LatexColor {
        LatexColor { lastcolor: -1 }
    }
}

impl LatexColor {
    /// `set_color(col)`: opens a group (unless one is open, nested colours
    /// are not supported) and switches to colour `col`; nothing for −1.
    pub fn set_color(&mut self, out: &mut Out, ctx: &Ctx, col: i32) {
        if col == -1 {
            return;
        }
        if self.lastcolor == -1 {
            out.puts("{");
        }
        let std = usize::try_from(col).ok().and_then(|i| LATEX_STD_COLORS.get(i));
        if let Some(s) = std {
            fprintf!(out, "\\color[rgb]{{{}}}", s);
        } else {
            // C indexes user_colors[col - NUM_STD_COLS]; look the colour up by
            // its number instead (unknown numbers give black).
            let (r, g, b) = ctx.color_rgb(col).unwrap_or((0, 0, 0));
            fprintf!(
                out,
                "\\color[rgb]{{{:.3},{:.3},{:.3}}}",
                f64::from(r) / 255.0,
                f64::from(g) / 255.0,
                f64::from(b) / 255.0
            );
        }
        self.lastcolor = col;
    }

    /// `reset_color(col)`: closes the group `set_color` opened.
    pub fn reset_color(&mut self, out: &mut Out, col: i32) {
        if col != -1 && col < (NUM_STD_COLS + MAX_USR_COLS) as i32 {
            out.puts("}%\n");
        }
        self.lastcolor = -1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gendev::UserColor;

    #[test]
    fn colors() {
        let mut ctx = Ctx::default();
        ctx.user_colors.push(UserColor {
            c: 32,
            r: 255,
            g: 128,
            b: 0,
        });
        let mut out = Out::new();
        let mut lc = LatexColor::default();
        lc.set_color(&mut out, &ctx, 4);
        lc.set_color(&mut out, &ctx, 32);
        lc.reset_color(&mut out, 32);
        lc.set_color(&mut out, &ctx, -1);
        lc.reset_color(&mut out, -1);
        assert_eq!(
            String::from_utf8(out.buf).unwrap(),
            "{\\color[rgb]{1,0,0}\\color[rgb]{1.000,0.502,0.000}}%\n"
        );
        assert_eq!(lc.lastcolor, -1);
    }
}
