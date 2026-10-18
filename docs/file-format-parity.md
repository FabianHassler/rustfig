# File format parity checklist

**Status:** 2026-10-18 (B9: EMF, Tk; G1: Fig 1.3–2.1; B2: pstex/pdftex; B3: pict2e; B1: tikz; B3: PSTricks; ported languages exported from the app; B12: every export through fig2dev).

| Format | Read | Write | Notes |
|---|---|---|---|
| 2026-10-18 | EMF, Tk/Perl-Tk ported (B9); TikZ, PSTricks rows updated. |
| Fig 3.2 | ✅ | ✅ | header without the centring line (as fig2dev); all object types, user colours, comments, pictures, resolution scaling, UTF-8 and Latin-1 text, octal escapes |
| Fig 3.1, 3.0 | ✅ | — | old splines converted to X-splines (shape ±1) |
| Fig 2.1 and older | ✅ | — | Fig 1.3–2.1 through fig2dev's ported reader (`engine/src/fig2dev/`, G1); the reader's repairs shown as warnings |
| RustFig native (`.rustfig`, JSON) | ✅ | ✅ | versioned envelope |
| PDF | — | ✅ | vector with true curves (ellipses, arcs, rounded boxes), standard-14 fonts, images with alpha; `-P`/`-z` page mode (paper, orientation, centring, multi-page), `-Y` version |
| PostScript (`ps`) | — | ✅ | DSC pages as genps: `-z` paper table, `-l`/`-p`, `-c`/`-e`, `-x`/`-y`, `-M`/`-o`/`-O`; geometry equals fig2dev's under ghostscript |
| SVG | — | ✅ | vector with true curves, patterns via clip paths, embedded PNG images |
| EPS | — | ✅ | Level 2, true curves, ISO-Latin-1 fonts |
| PNG | — | ✅ | any resolution up to 20 000 px edge |
| EPS and PDF together (xfig's pspdf) | — | ✅ | two files, NAME.eps and NAME.pdf |
| LaTeX box (fig2dev `box`) | — | ✅ | ported from `genbox.c`; fig2dev's bounds |
| fig2dev options (`file.export` `options`, CLI `--option`) | — | 🟡 | ported: depths `-D`/`-K`, `-m`, `-Z`, `-s`, border `-b`, background `-g` (X11 names), grayscale `-N`, title `-n`, font `-f`, `-F`; page layout (B4a), previews `-A`/`-T`/`-C` and bounding box `-B`/`-R` (B4b); bitmap options as fig2dev (B6); grid `-G` for PostScript, PDF, pstex and the bitmaps (audit pass 13) |
| PostScript (print) | — | ❌ | gap #3 |
| standalone `fig2dev` (`apps/fig2dev`) | — | 🟡 | fig2dev 3.2.9a's command line, messages and exit codes, input from files or stdin with `-E` encodings (UTF-8, ISO-8859-1/-2 and, through `encoding_rs`, KOI8, CP125x, ISO-8859-N, Mac, CJK; as iconv for the TeX family); writes the ported languages, the others fail naming their task (B10); one reader for every language; fig2dev's suite: 82 pass, 6 fail (`epsi` and two embedded-PostScript bitmap tests that need a PostScript interpreter, the `/MediaBox` helper program, a version-2 font size, the KOI8-R case whose `iconv KOI8R` macOS lacks), 6 need unported languages |
| ported fig2dev languages from the app | — | ✅ | every language a ported driver provides exports from the engine, the export dialog and `rustfig-cli` (availability checked at run time) |
| pstex/pdftex (and xfig's pspdftex pair) | — | ✅ | ported from `genpstex.c` (B2): the `_t` file byte-identical to fig2dev on a test figure; EPS/PDF without special texts, fig2dev's bounding box; the app writes xfig's file pairs |
| pict2e | — | ✅ | ported from `genpict2e.c` (B3): byte-identical to fig2dev 3.2.9a on the fixtures, the suite's figures and 285 of 300 sample figures (the rest: `\ooo`-escaped bytes in texts come out as UTF-8); `-p` converts PNG/JPEG/PNM/TIFF pictures to EPS in-process (no `bmeps`; stderr differs from fig2dev, which warns without bmeps) |
| TikZ | — | ✅ | ported from `gentikz.c` (B1): byte-identical to fig2dev 3.2.9a on the fixtures, the suite's figures and 285 of 300 sample figures (the rest: `\ooo`-escaped bytes in text) |
| PSTricks | — | ✅ | ported from `genpstricks.c` (B3): byte-identical to fig2dev 3.2.9a on the fixtures, the suite's figures and 285 of 300 sample figures (the rest: `\ooo`-escaped bytes in texts come out as UTF-8); `-p` converts PNG/JPEG/PNM/TIFF pictures to EPS in-process (no `bmeps`; stderr differs from fig2dev, which warns without bmeps) |
| GIF/JPEG/TIFF/PPM/PCX/XBM/XPM | — | ✅ | from RustFig's renderer (B6): sizes and options `-S`, `-q`, `-t`, `-b`, `-g`, `-N` as fig2dev (messages and exit codes too); checked against RustFig's PNG; PNG opaque white, XBM dithered as `pgmtopbm`, PCX laid out as `ppmtopcx` (palette order differs) |
| Gerber, shape | — | ✅ | ported from fig2dev (B9): byte-identical to fig2dev 3.2.9a on 2,549/2,552 (Gerber) and 2,551/2,552 (shape) of xfig's samples, with fig2dev's depth order |
| EPS/PS/PDF text beyond Latin-1 | — | ✅ | EPS/PS: Xfig-UTF8-H CMap and `composefont` as genps (2026-10-18); PDF: Type 3 fonts from RustFig's outlines with ToUnicode; `-j` composite fonts and embedded URW fonts not done |
| LaTeX picture (`latex`), PiCTeX, TeXtyl | — | ✅ | ported from fig2dev (B9): identical to fig2dev 3.2.9a on ≈7,590 of 7,701 runs (2,552 samples × 3 option sets); the rest differ in Latin-1 text (written as UTF-8) and one ellipse bound |
| CGM, HPGL (`ibmgl`), DXF | — | ✅ | ported from fig2dev (B9): identical to fig2dev 3.2.9a on 1,231 of 1,254 runs (tests/data, every 25th library figure, several option sets); the rest differ in escaped high bytes in text and fig2dev's out-of-bounds reads; `-a` (ralcgm) not run |
| MetaPost (`mp`, `mmp`), METAFONT, ge, image map | — | ✅ | ported from fig2dev (B9): mf and map identical to fig2dev 3.2.9a on all 342 runs each, mp on 672/684, pictex and gbx on all runs (with `SOURCE_DATE_EPOCH`; without it their dates are UTC, fig2dev's local time), ge on all runs without user colours; the rest are escaped high bytes in text and pre-3.2 interpolated splines. ge's user colours are written as black, where fig2dev reads past its `GE_COLORS` table and prints whatever follows it |
| EMF | — | ✅ | ported from fig2dev (B9, 2026-10-18): texts, PNG pictures, all shapes; byte-identical to fig2dev (texts as the input's bytes) except CJK-locale iconv fonts |
| Tk, Perl-Tk | — | ✅ | ported from fig2dev (B9), byte-identical on the sample (texts as the input's bytes; one figure differs by stack garbage in C) |
| Pictures in exports | ✅ | — | `fig2dev/util/readpics`: .gz/.Z, XBM, XPM, PCX with fig2dev's messages; EPS pictures embedded in EPS as genps does; PDF pictures, and EPS pictures in PDF/SVG/bitmaps, still a placeholder box |
| epic, eepic, eepicemu, tpic, pic | — | ✅ | ported from fig2dev (B9): identical to fig2dev 3.2.9a on ≈99% of runs (every sample, several option sets); the rest differ in Latin-1 text, one ellipse bound, ±1 point in some closed X-splines |
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
| 2026-10-16 | Fig 1.3–2.1 read (G1); pstex/pdftex (B2); ported languages exported from the app; pictures in exports. |
| 2026-10-16 | B12: every export through fig2dev's `convert`; suite 62 pass. |
| 2026-10-17 | pict2e (B3), byte-identical to fig2dev on 285 of 300 sample figures; suite 73 pass. |
| 2026-10-17 | tikz (B1), byte-identical to fig2dev on 285 of 300 sample figures. |
| 2026-10-17 | PSTricks (B3), byte-identical to fig2dev on 285 of 300 sample figures. |
