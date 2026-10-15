# File format parity checklist

**Status:** 2026-10-15 (B6: bitmaps; B10: standalone fig2dev).

| Format | Read | Write | Notes |
|---|---|---|---|
| Fig 3.2 | ✅ | ✅ | header without the centring line (as fig2dev); all object types, user colours, comments, pictures, resolution scaling, UTF-8 and Latin-1 text, octal escapes |
| Fig 3.1, 3.0 | ✅ | — | old splines converted to X-splines (shape ±1) |
| Fig 2.1 and older | ❌ | — | gap #13 |
| RustFig native (`.rustfig`, JSON) | ✅ | ✅ | versioned envelope |
| PDF | — | ✅ | vector with true curves (ellipses, arcs, rounded boxes), standard-14 fonts, images with alpha |
| SVG | — | ✅ | vector with true curves, patterns via clip paths, embedded PNG images |
| EPS | — | ✅ | Level 2, true curves, ISO-Latin-1 fonts |
| PNG | — | ✅ | any resolution up to 20 000 px edge |
| EPS and PDF together (xfig's pspdf) | — | ✅ | two files, NAME.eps and NAME.pdf |
| LaTeX box (fig2dev `box`) | — | ✅ | ported from `genbox.c`; fig2dev's bounds |
| fig2dev options (`file.export` `options`, CLI `--option`) | — | 🟡 | ported: depths `-D`/`-K`, `-m`, `-Z`, `-s`, border `-b`, background `-g` (X11 names), grayscale `-N`, title `-n`, font `-f`, `-F`; page layout and previews (B4), bitmap options (B6) refused with their task |
| PostScript (print) | — | ❌ | gap #3 |
| standalone `fig2dev` (`apps/fig2dev`) | — | 🟡 | fig2dev 3.2.9a's command line, messages and exit codes, input from files or stdin with `-E` encodings; writes the ported languages, the others fail naming their task (B10) |
| LaTeX (TikZ, PSTricks, pict2e, epic/eepic, latex, pictex, pstex/pdftex) | — | ❌ | being ported from fig2dev (B1–B3, B9); gap #1 |
| GIF/JPEG/TIFF/PPM/PCX/XBM/XPM | — | ✅ | from RustFig's renderer (B6): sizes and options `-S`, `-q`, `-t`, `-b`, `-g`, `-N` as fig2dev; checked against RustFig's PNG |
| Gerber, shape | — | ✅ | ported from fig2dev (B9): byte-identical to fig2dev 3.2.9a on 2,549/2,552 (Gerber) and 2,551/2,552 (shape) of xfig's samples, with fig2dev's depth order |
| CGM/EMF/HPGL/MetaPost/METAFONT/DXF/map/Tk/PIC/tpic/textyl/SLD | — | ❌ | being ported from fig2dev (B9) |
| Picture import PNG/JPEG/GIF/BMP/TIFF/PNM | ✅ | — | |
| Picture import EPS/PDF/XPM/XBM/PCX | ❌ | — | gap #14 |

Fuzzing: `crates/format/tests/prop_format.rs`, `crates/engine/tests/import_fuzz.rs`.

## Revision history
| Date | Note |
|---|---|
| 2026-10-13 | First checklist. |
| 2026-10-14 | xfig's language list; LaTeX box and pspdf. |
| 2026-10-15 | Bitmap languages (B6). |
| 2026-10-15 | Standalone fig2dev (B10). |
| 2026-10-15 | fig2dev's options. |
