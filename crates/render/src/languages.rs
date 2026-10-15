//! xfig's export languages: every output format of fig2dev that xfig's
//! Export panel offers, in its order, with the status of its port.
//
// Ported from xfig src/mode.c (`lang_items`, `lang_texts`) and the default
// file suffixes in src/w_export.c (xfig licence, see LICENSE-xfig).

/// How RustFig writes a language today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Written by RustFig.
    Available,
    /// Written, but parts of fig2dev's driver are still missing (the plan
    /// task names them).
    Partial(&'static str),
    /// Not ported yet; the plan task that ports it.
    Planned(&'static str),
}

/// Kinds of output, as xfig's Export panel groups them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// PostScript and PDF.
    PostScript,
    /// LaTeX and TeX picture languages.
    Latex,
    /// Other vector formats.
    Vector,
    /// Bitmaps.
    Bitmap,
}

/// One export language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    /// fig2dev's `-L` name, also used as the `format` of `file.export`.
    pub id: &'static str,
    /// The description in xfig's Export panel.
    pub description: &'static str,
    /// xfig's default file suffix (without the dot).
    pub extension: &'static str,
    /// Other suffixes that select this language when exporting to a path.
    pub aliases: &'static [&'static str],
    pub kind: Kind,
    pub status: Status,
}

impl Language {
    /// Whether RustFig can write it.
    pub fn available(&self) -> bool {
        !matches!(self.status, Status::Planned(_))
    }

    /// The plan task porting or completing it, if any.
    pub fn task(&self) -> Option<&'static str> {
        match self.status {
            Status::Available => None,
            Status::Partial(t) | Status::Planned(t) => Some(t),
        }
    }
}

const fn lang(
    id: &'static str,
    description: &'static str,
    extension: &'static str,
    aliases: &'static [&'static str],
    kind: Kind,
    status: Status,
) -> Language {
    Language {
        id,
        description,
        extension,
        aliases,
        kind,
        status,
    }
}

use Kind::{Bitmap, Latex, PostScript, Vector};
use Status::{Available, Partial, Planned};

/// xfig's export languages in the order of its Export panel.
pub const LANGUAGES: &[Language] = &[
    lang("ps", "PostScript", "ps", &[], PostScript, Partial("B4")),
    lang(
        "eps",
        "EPS (Encapsulated PostScript)",
        "eps",
        &[],
        PostScript,
        Available,
    ),
    lang(
        "pdf",
        "PDF (Portable Document Format)",
        "pdf",
        &[],
        PostScript,
        Available,
    ),
    lang("pspdf", "EPS and PDF (two files)", "eps", &[], PostScript, Available),
    lang("box", "LaTeX box (figure boundary)", "box", &[], Latex, Available),
    lang("latex", "LaTeX picture", "latex", &[], Latex, Planned("B9")),
    lang(
        "pict2e",
        "LaTeX picture + pict2e macros",
        "pict2e",
        &[],
        Latex,
        Planned("B3"),
    ),
    lang("tikz", "TeX/LaTeX + tikz macros", "tikz", &[], Latex, Planned("B1")),
    lang(
        "pstricks",
        "LaTeX + pstricks macros",
        "pstricks",
        &[],
        Latex,
        Planned("B3"),
    ),
    lang("epic", "LaTeX picture + epic macros", "epic", &[], Latex, Planned("B9")),
    lang(
        "eepic",
        "LaTeX picture + eepic macros",
        "eepic",
        &[],
        Latex,
        Planned("B9"),
    ),
    lang(
        "eepicemu",
        "LaTeX picture + eepicemu macros",
        "eepicemu",
        &[],
        Latex,
        Planned("B9"),
    ),
    lang(
        "pstex",
        "Combined PS/LaTeX (both parts)",
        "pstex",
        &[],
        Latex,
        Planned("B2"),
    ),
    lang(
        "pdftex",
        "Combined PDF/LaTeX (both parts)",
        "pdf",
        &[],
        Latex,
        Planned("B2"),
    ),
    lang(
        "pspdftex",
        "Combined PS/PDF/LaTeX (3 parts)",
        "pspdftex",
        &[],
        Latex,
        Planned("B2"),
    ),
    lang("pictex", "PiCTeX macros", "pictex", &[], Latex, Planned("B9")),
    lang(
        "cgm",
        "CGM (Computer Graphics Metafile)",
        "cgm",
        &[],
        Vector,
        Planned("B9"),
    ),
    lang("emf", "EMF (Enhanced Metafile)", "emf", &[], Vector, Planned("B9")),
    lang(
        "dxf",
        "DXF (Drawing Interchange Format)",
        "dxf",
        &[],
        Vector,
        Planned("B9"),
    ),
    lang("gbx", "GBX (Gerber, RS-247-X)", "gbx", &[], Vector, Planned("B9")),
    lang(
        "ibmgl",
        "HPGL/2 or IBM-GL",
        "ibmgl",
        &["hpgl", "hpl"],
        Vector,
        Planned("B9"),
    ),
    lang("map", "HTML Image Map", "map", &[], Vector, Planned("B9")),
    lang(
        "textyl",
        "Textyl \\special commands",
        "textyl",
        &[],
        Latex,
        Planned("B9"),
    ),
    lang("tpic", "TPIC", "tpic", &[], Latex, Planned("B9")),
    lang("pic", "PIC", "pic", &[], Vector, Planned("B9")),
    lang("mf", "MF (MetaFont)", "mf", &[], Vector, Planned("B9")),
    lang("mp", "MP (MetaPost)", "mp", &[], Vector, Planned("B9")),
    lang("tk", "Tk (Tcl/Tk toolkit)", "tk", &["tcl"], Vector, Planned("B9")),
    lang("ptk", "PTk (Perl/Tk toolkit)", "ptk", &[], Vector, Planned("B9")),
    lang(
        "shape",
        "SHAPE (ShapePar definition)",
        "shape",
        &[],
        Vector,
        Planned("B9"),
    ),
    lang("svg", "SVG (Scalable Vector Graphics)", "svg", &[], Vector, Available),
    lang(
        "gif",
        "GIF (Graphics Interchange Format)",
        "gif",
        &[],
        Bitmap,
        Available,
    ),
    lang(
        "jpeg",
        "JPEG (Joint Photo. Expert Group)",
        "jpg",
        &["jpeg"],
        Bitmap,
        Available,
    ),
    lang("pcx", "PCX (PC Paintbrush)", "pcx", &[], Bitmap, Available),
    lang("png", "PNG (Portable Network Graphics)", "png", &[], Bitmap, Available),
    lang("ppm", "PPM (Portable Pixmap)", "ppm", &[], Bitmap, Available),
    lang("sld", "SLD (AutoCad Slide)", "sld", &[], Bitmap, Planned("B9")),
    lang("tiff", "TIFF (no compression)", "tif", &["tiff"], Bitmap, Available),
    lang("xbm", "XBM (X11 Bitmap)", "xbm", &[], Bitmap, Available),
    lang("xpm", "XPM (X11 Pixmap)", "xpm", &[], Bitmap, Available),
];

/// The language with this id (fig2dev's `-L` name), ignoring case.
pub fn language(id: &str) -> Option<&'static Language> {
    let id = id.trim().trim_start_matches('.');
    LANGUAGES.iter().find(|l| l.id.eq_ignore_ascii_case(id))
}

/// The language a file name asks for, from its suffix. Suffixes several
/// languages share (`.tex`, `.pdf` of pdftex, `.eps` of pspdf) give the
/// plain language or, for `.tex`, none: the caller must name the format.
pub fn language_for_path(path: &str) -> Option<&'static Language> {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let (_, ext) = name.rsplit_once('.')?;
    if ext.eq_ignore_ascii_case("tex") {
        return None;
    }
    let by_id = LANGUAGES
        .iter()
        .find(|l| l.id.eq_ignore_ascii_case(ext) && l.extension.eq_ignore_ascii_case(ext));
    by_id.or_else(|| {
        LANGUAGES.iter().find(|l| {
            l.extension.eq_ignore_ascii_case(ext) && !matches!(l.id, "pspdf" | "pdftex")
                || l.aliases.iter().any(|a| a.eq_ignore_ascii_case(ext))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_found() {
        for (i, l) in LANGUAGES.iter().enumerate() {
            assert!(LANGUAGES.iter().skip(i + 1).all(|o| o.id != l.id), "{}", l.id);
            assert_eq!(language(l.id), Some(l));
            assert_eq!(language(&l.id.to_uppercase()), Some(l));
        }
        assert!(language("bogus").is_none());
    }

    #[test]
    fn suffixes_pick_the_plain_language() {
        let id = |p: &str| language_for_path(p).map(|l| l.id);
        assert_eq!(id("a/b.pdf"), Some("pdf"));
        assert_eq!(id("x.EPS"), Some("eps"));
        assert_eq!(id("x.tif"), Some("tiff"));
        assert_eq!(id("x.tiff"), Some("tiff"));
        assert_eq!(id("x.jpg"), Some("jpeg"));
        assert_eq!(id("x.hpgl"), Some("ibmgl"));
        assert_eq!(id("x.tikz"), Some("tikz"));
        assert_eq!(id("x.tex"), None);
        assert_eq!(id("noext"), None);
        assert_eq!(id("dir.d/noext"), None);
    }

    #[test]
    fn statuses_name_their_task() {
        for l in LANGUAGES {
            assert_eq!(l.available(), !matches!(l.status, Status::Planned(_)));
            if l.status != Status::Available {
                assert!(l.task().is_some_and(|t| t.starts_with('B')), "{}", l.id);
            }
        }
    }
}
