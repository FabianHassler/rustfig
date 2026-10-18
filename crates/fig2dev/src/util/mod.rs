//! Helpers the drivers share (fig2dev's `dev/` support files): TeX and
//! PostScript font tables, `\SetFigFont`, ISO-8859 → TeX, text encoding,
//! picture reading, the creation date.
//!
//! - [`texfonts`]: `dev/texfonts.c` and the parts of `dev/textconvert.c` it
//!   uses (`put_string`, `select_font`, the TeX font tables).
//! - [`textconvert`]: `dev/textconvert.c` (the bytes of a text, conversions).
//! - [`iso2tex`]: `iso2tex.c` (ISO-8859-1/-2 → TeX sequences).
//! - [`psfonts`]: `dev/psfonts.c` (PostScript font names, `unpsfont`).
//! - [`setfigfont`]: `dev/setfigfont.c` (the `\SetFigFont` macro).
//! - [`latex`]: `dev/genlatex.h` (`set_color`, `reset_color`).
//! - [`creationdate`]: `creationdate.c` (honours `SOURCE_DATE_EPOCH`).
//! - [`readpics`]: `dev/readpics.c` and the `dev/read*.c` picture readers.

pub mod creationdate;
pub mod iso2tex;
pub mod latex;
pub mod psfonts;
pub mod readpics;
pub mod setfigfont;
pub mod texfonts;
pub mod textconvert;
pub mod tkpattern;
