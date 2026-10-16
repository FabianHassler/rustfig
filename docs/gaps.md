# Known gaps (ranked work list)

**Status:** 2026-10-16 (B2, G1 landed; B11 added from the fig2dev audit). Ranked by user impact per hour. Pick work from the top.

| # | Gap | Area | Impact | Hours |
|---|-----|------|--------|------:|
| 1 | LaTeX exports: TikZ, PSTricks, pict2e (special text passed through), ported from fig2dev (B1, B3; pstex/pdftex done, B2) | Export | High: xfig's main audience writes LaTeX | 40 |
| 2 | Web build: compiles locally (2026-10-14); still to do: the wasm check in CI, trunk packaging and browser open/save/download | Platforms | High | 24 |
| 3 | PostScript output with page layout (orientation, centring, multi-page) and printing | Export/Printing | High | 30 |
| 4 | Library objects: browse a directory of .fig files and place them | Libraries | Medium | 40 |
| 5 | Open compound (edit inside a compound without breaking it) | Editing | Medium | 16 |
| 6 | Join/split lines, chop, tangent/normal, measure modes | Editing | Medium | 40 |
| 7 | Settings persistence (units, grid, recent files) and a style manager | Settings | Medium | 30 |
| 8 | The zoom box (rulers landed with W6) | View | Medium | 4 |
| 9 | Character map, compose-key input, LaTeX special-text handling | Text | Medium | 30 |
| 10 | Bitmap exports GIF/JPEG/TIFF/PPM (with transparent colour) | Export | Medium | 12 |
| 11 | Colour editor (create and edit user colours in the UI) | Attributes | Medium | 12 |
| 12 | Point positioning modes: latex lines, manhattan, mountain; linked move | Attributes | Low | 20 |
| 14 | EPS/PDF/XBM/XPM/PCX picture objects (currently a crossed box on the canvas); in exports XBM/XPM/PCX and .gz/.Z load and EPS pictures embed in EPS, but PDF pictures, and EPS pictures in PDF/SVG/bitmaps, are a placeholder box | File format | Low | 24 |
| 16 | Windows and Linux smoke tests in CI | Platforms | Low | 8 |
| 17 | Localization of UI strings | Localization | Low | 24 |
| 18 | Spell check | Text | Low | 16 |
| 19 | Accessibility (screen-reader labels for canvas objects) | Platforms | Low | 16 |
| 21 | fig2dev's EMF and Tk/Perl-Tk drivers not ported yet (B9) | Export | Low | 26 |
| 24 | fig2dev port hardening: the audit's partial and missing functions outside the unported drivers (options and messages, text bytes, depth order, FMA in bounds, `-G` grid, EPS/PDF fonts and Unicode, SVG text, `readpics`, JPEG passthrough; B11) | Export | Medium | 66 |
| 20 | Real PostScript font metrics when craft-fonts lacks a face (synthetic bold/italic today); the A10 corpus shows text widths as the main remaining difference from fig2dev | Text | Medium | 12 |

## Revision history

| Date | Note |
|------|------|
| 2026-10-16 | #13 closed (G1: the app reads Fig 1.3–2.1 through fig2dev's reader); #1 shrank (B2 pstex/pdftex), #21 shrank to EMF and Tk, #14 shrank (pictures in exports); #24 added from the fig2dev audit (B11). |
| 2026-10-15 | #23 closed: start values are xfig's (positioning magnet 1/16 inch, alignment none, stick arrows, dot gap 3 apart from dash length 4); the man page's text step 1.2 is a documentation error, xfig's code starts at 1.0 as RustFig does. |
| 2026-10-15 | D12 closed #22 (fill patterns as fig2dev's). |
| 2026-10-14 | Export bounds now fig2dev's (B0); the corpus's largest remaining difference is fill patterns (#22). |
| 2026-10-14 | A10: corpus comparison with fig2dev on xfig's sample figures; metric text size fixed. Remaining differences: export bounding box (B0 `bound.c`), text widths (#20, B8). |
| 2026-10-14 | D9 closed #15 (arrowheads, pie-arc arrows, arc direction, dash patterns as fig2dev's PostScript driver). |
| 2026-10-14 | A8 closed the comparison gap and found arrowhead, pie-arc and line-width differences (line widths fixed); fig2dev to be ported (#1, #21). |
| 2026-10-13 | First list. |
| 2026-10-13 | Added #0 (xfig window parity, phase W). |
| 2026-10-13 | #0 shrank: W1 landed. |
| 2026-10-14 | #0 shrank: W2 landed. |
| 2026-10-14 | #0 shrank: W3 landed. |
| 2026-10-14 | Added #0b from the source map. |
| 2026-10-14 | #0b closed: S1 landed. |
| 2026-10-14 | #0 shrank: W5 landed. |
| 2026-10-14 | #0 and #8 shrank: W6 landed. |
| 2026-10-14 | #0 shrank: W4 landed. |
| 2026-10-14 | #0 closed: W7 landed, phase W done. |
| 2026-10-13 | Linked #1 to the fig2dev backend (B0); added #21 (reference tests, A8). |
