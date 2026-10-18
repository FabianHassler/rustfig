//! The page layout of fig2dev's PostScript pages: paper sizes, orientation,
//! centring, offsets and the pages of a figure too large for one.
//
// Ported from fig2dev fig2dev.c (`paperdef`) and dev/genps.c (`genps_start`,
// `genps_end`: page size, origin and multi-page loops) (xfig licence, see
// LICENSE-xfig).

use crate::gendev::PAPERDEF;

/// Most pages a figure may be split into.
const MAX_PAGES: usize = 10_000;

/// The paper `name` names (case-insensitive; `ledger` is `tabloid`):
/// its nice name, width and height. fig2dev's message when unknown.
pub fn paper(name: &str) -> Result<(&'static str, i32, i32), String> {
    let name = if name.eq_ignore_ascii_case("ledger") {
        "tabloid"
    } else {
        name
    };
    PAPERDEF
        .iter()
        .find(|(n, _, _)| n.eq_ignore_ascii_case(name))
        .map(|&(n, w, h)| (n, w, h))
        .ok_or_else(|| format!("Unknown paper size `{name}'"))
}

/// What the layout depends on.
#[derive(Debug, Clone)]
pub struct PageInput<'a> {
    pub papersize: &'a str,
    pub landscape: bool,
    pub center: bool,
    pub multi_page: bool,
    pub overlap: bool,
    pub xoff: i32,
    pub yoff: i32,
    /// The figure's box in points on the paper, border included
    /// (`fllx`, `flly`, `furx`, `fury`; y down).
    pub fllx: f64,
    pub flly: f64,
    pub furx: f64,
    pub fury: f64,
}

/// The pages of a PostScript or PDF document.
#[derive(Debug, Clone, PartialEq)]
pub struct PageLayout {
    /// Paper size in points, after landscape.
    pub width: i32,
    pub height: i32,
    /// The paper's name as fig2dev spells it.
    pub paper: String,
    /// The `%%Pages` count (fig2dev's estimate in multi-page mode).
    pub declared_pages: i64,
    /// Per page, where the figure's bottom left corner (the border
    /// included) lies on the page, in points, y up.
    pub pages: Vec<(f64, f64)>,
}

impl PageLayout {
    pub fn compute(i: &PageInput<'_>) -> Result<PageLayout, String> {
        let (name, mut w, mut h) = paper(i.papersize)?;
        if i.landscape {
            std::mem::swap(&mut w, &mut h);
        }
        let (wf, hf) = (f64::from(w), f64::from(h));
        let mut pages = Vec::new();
        let mut declared = 1;
        if i.multi_page {
            declared = ((1.11111 * (i.furx - 0.1 * wf) / wf + 1.0) as i64)
                .saturating_mul((1.11111 * (i.fury - 0.1 * hf) / hf + 1.0) as i64);
            let mul = if i.overlap { 0.9 } else { 1.0 };
            let mut dy = 0.0;
            while dy < i.fury - hf * 0.1 || pages.is_empty() {
                let mut dx = 0.0;
                while dx < i.furx - wf * 0.1 || pages.is_empty() {
                    if pages.len() >= MAX_PAGES {
                        return Err(format!("fig2dev: the figure needs more than {MAX_PAGES} pages"));
                    }
                    pages.push((i.fllx - dx, dy + hf * mul - i.fury));
                    dx += wf * mul;
                    if wf * mul <= 0.0 || !dx.is_finite() {
                        break;
                    }
                }
                dy += hf * mul;
                if !dy.is_finite() {
                    break;
                }
            }
        } else {
            let (mut ox, mut oy) = if i.center {
                ((wf - i.furx - i.fllx) / 2.0, (hf + i.fury + i.flly) / 2.0)
            } else {
                (0.0, hf)
            };
            ox += f64::from(i.xoff);
            oy += f64::from(i.yoff);
            pages.push((ox + i.fllx, oy - i.fury));
        }
        Ok(PageLayout {
            width: w,
            height: h,
            paper: name.to_string(),
            declared_pages: declared,
            pages,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(p: &str) -> PageInput<'_> {
        PageInput {
            papersize: p,
            landscape: false,
            center: false,
            multi_page: false,
            overlap: false,
            xoff: 0,
            yoff: 0,
            fllx: 100.0,
            flly: 50.0,
            furx: 300.0,
            fury: 200.0,
        }
    }

    #[test]
    fn papers() {
        assert_eq!(paper("a4").unwrap(), ("A4", 595, 842));
        assert_eq!(paper("LEDGER").unwrap(), ("Tabloid", 792, 1224));
        assert_eq!(paper("nope").unwrap_err(), "Unknown paper size `nope'");
    }

    #[test]
    fn single_page_flush_and_centred() {
        let l = PageLayout::compute(&input("A4")).unwrap();
        assert_eq!((l.width, l.height, l.pages.len()), (595, 842, 1));
        assert_eq!(l.pages[0], (100.0, 842.0 - 200.0));
        let mut i = input("A4");
        i.center = true;
        i.landscape = true;
        let l = PageLayout::compute(&i).unwrap();
        assert_eq!((l.width, l.height), (842, 595));
        // The 200 x 150 point figure in the middle of the page.
        assert_eq!(l.pages[0], ((842.0 - 200.0) / 2.0, (595.0 - 150.0) / 2.0));
    }

    #[test]
    fn multi_page_counts() {
        let mut i = input("A9");
        i.multi_page = true;
        i.furx = 300.0;
        i.fury = 400.0;
        let l = PageLayout::compute(&i).unwrap();
        assert_eq!(l.pages.len(), 3 * 3);
    }
}
