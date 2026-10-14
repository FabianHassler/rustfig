# File format parity checklist

**Status:** 2026-10-14 (B0 started: every fig2dev language listed by `file.export_formats`).

| Format | Read | Write | Notes |
|---|---|---|---|
| Fig 3.2 | ✅ | ✅ | all object types, user colours, comments, pictures, resolution scaling, UTF-8 and Latin-1 text, octal escapes |
| Fig 3.1, 3.0 | ✅ | — | old splines converted to X-splines (shape ±1) |
| Fig 2.1 and older | ❌ | — | gap #13 |
| RustFig native (`.rustfig`, JSON) | ✅ | ✅ | versioned envelope |
| PDF | — | ✅ | vector, standard-14 fonts, images with alpha |
| SVG | — | ✅ | vector, patterns via clip paths, embedded PNG images |
| EPS | — | ✅ | Level 2, ISO-Latin-1 fonts |
| PNG | — | ✅ | any resolution up to 20 000 px edge |
| EPS and PDF together (xfig's pspdf) | — | ✅ | two files, NAME.eps and NAME.pdf |
| LaTeX box (fig2dev `box`) | — | ✅ | ported from `genbox.c`; bounds still RustFig's (B0) |
| PostScript (print) | — | ❌ | gap #3 |
| LaTeX (TikZ, PSTricks, pict2e, epic/eepic, latex, pictex, pstex/pdftex) | — | ❌ | being ported from fig2dev (B1–B3, B9); gap #1 |
| GIF/JPEG/TIFF/PPM/XBM/XPM | — | ❌ | gap #10 |
| CGM/EMF/HPGL/MetaPost/METAFONT/DXF/Gerber/map/Tk/PIC/tpic/textyl/shape/SLD | — | ❌ | being ported from fig2dev (B9) |
| Picture import PNG/JPEG/GIF/BMP/TIFF/PNM | ✅ | — | |
| Picture import EPS/PDF/XPM/XBM/PCX | ❌ | — | gap #14 |

Fuzzing: `crates/format/tests/prop_format.rs`, `crates/engine/tests/import_fuzz.rs`.

## Revision history
| Date | Note |
|---|---|
| 2026-10-13 | First checklist. |
| 2026-10-14 | xfig's language list; LaTeX box and pspdf. |
