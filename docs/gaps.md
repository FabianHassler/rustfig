# Known gaps (ranked work list)

**Status:** 2026-10-14. Ranked by user impact per hour. Pick work from the top.

| # | Gap | Area | Impact | Hours |
|---|-----|------|--------|------:|
| 0 | Main window differs from xfig: no xfig accelerators (plan phase W; W1–W6 done) | UI | Medium | 6 |
| 1 | LaTeX exports: TikZ, PSTricks, pict2e (special text passed through). Quick route: the fig2dev backend (plan B0, 8 h), then native exporters | Export | High: xfig's main audience writes LaTeX | 40 |
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
| 13 | Read Fig 1.x/2.x files | File format | Low | 24 |
| 14 | EPS/PDF/XBM/XPM/PCX picture objects (currently a crossed box) | File format | Low | 24 |
| 15 | Exact arrowhead shapes for types 10–14; dash-dot tick styles | Attributes | Low | 8 |
| 16 | Windows and Linux smoke tests in CI | Platforms | Low | 8 |
| 17 | Localization of UI strings | Localization | Low | 24 |
| 18 | Spell check | Text | Low | 16 |
| 19 | Accessibility (screen-reader labels for canvas objects) | Platforms | Low | 16 |
| 21 | No visual comparison against xfig's own renderer; plan A8 adds fig2dev reference tests | Quality | Medium | 12 |
| 20 | Real PostScript font metrics when craft-fonts lacks a face (synthetic bold/italic today) | Text | Low | 12 |

## Revision history

| Date | Note |
|------|------|
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
| 2026-10-13 | Linked #1 to the fig2dev backend (B0); added #21 (reference tests, A8). |
