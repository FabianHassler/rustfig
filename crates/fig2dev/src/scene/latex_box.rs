//! LaTeX box: an empty `\makebox` of the figure's size, to reserve its space.
//
// Ported from fig2dev dev/genbox.c (xfig licence, see LICENSE-xfig).

use figbase::geom::{Rect, FIG_PER_INCH};

/// The box for `bounds` (Fig units) at `scale` (magnification, with the
/// metric correction), preceded by the whole-figure comments.
pub fn to_box(comments: &[String], bounds: &Rect, scale: f64) -> String {
    let mut out = String::new();
    for c in comments {
        for line in c.lines() {
            out.push_str("% ");
            out.push_str(line);
            out.push('\n');
        }
    }
    let inches = |d: f64| d * scale / FIG_PER_INCH;
    out.push_str(&format!(
        "\\makebox[{:.3}in]{{\\rule{{0in}}{{{:.3}in}}}}\n",
        inches(bounds.width()),
        inches(bounds.height())
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use figbase::geom::Point;

    #[test]
    fn box_of_two_by_one_inches() {
        let r = Rect::from_points([Point::new(0.0, 0.0), Point::new(2400.0, 1200.0)]);
        let s = to_box(&["figure\nnotes".into()], &r, 1.0);
        assert_eq!(s, "% figure\n% notes\n\\makebox[2.000in]{\\rule{0in}{1.000in}}\n");
        assert!(to_box(&[], &r, 0.5).contains("[1.000in]"));
    }
}
