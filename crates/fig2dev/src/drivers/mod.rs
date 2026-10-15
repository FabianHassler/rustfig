//! The drivers, one module per fig2dev `dev/gen*.c` file, and the table of
//! fig2dev's languages that this crate writes.

use crate::Driver;

pub mod cgm;
pub mod dxf;
pub mod emf;
pub mod epic;
pub mod gbx;
pub mod ge;
pub mod ibmgl;
pub mod latex;
pub mod map;
pub mod mf;
pub mod mp;
pub mod pic;
pub mod pict2e;
pub mod pictex;
pub mod pstex;
pub mod pstricks;
pub mod shape;
pub mod textyl;
pub mod tikz;
pub mod tk;
pub mod tpic;

/// The driver for fig2dev language `lang` (fig2dev's name, as `-L` takes
/// it), or `None` when this crate doesn't write it (yet). EPS, PS, PDF,
/// SVG, PNG and box are written by `rustfig-render`.
pub fn driver(lang: &str) -> Option<Box<dyn Driver>> {
    match lang {
        "cgm" => cgm::new(lang),
        "dxf" => dxf::new(lang),
        "emf" => emf::new(lang),
        "epic" | "eepic" | "eepicemu" => epic::new(lang),
        "gbx" => gbx::new(lang),
        "ge" => ge::new(lang),
        "ibmgl" => ibmgl::new(lang),
        "latex" => latex::new(lang),
        "map" => map::new(lang),
        "mf" => mf::new(lang),
        "mp" | "mmp" => mp::new(lang),
        "pic" => pic::new(lang),
        "pict2e" => pict2e::new(lang),
        "pictex" => pictex::new(lang),
        "pstex_t" | "pdftex_t" => pstex::new(lang),
        "pstricks" => pstricks::new(lang),
        "shape" => shape::new(lang),
        "textyl" => textyl::new(lang),
        "tikz" => tikz::new(lang),
        "tk" | "ptk" => tk::new(lang),
        "tpic" => tpic::new(lang),
        _ => None,
    }
}
