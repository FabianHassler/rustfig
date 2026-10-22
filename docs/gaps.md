# Known gaps (ranked work list)

**Status:** 2026-10-22 (D4: #6 dropped tangent/normal). 2026-10-22 (D14: smart links extended, nothing listed here changed; the leftovers are the file-stored links xfig does not have). 2026-10-22 (D3: #6 dropped Chop; D8: linked move landed, #12 dropped "linked move", #33 dropped the Smart Links button). Earlier: 2026-10-21 (S4: #34 added; B15: #14 shrank to the leftovers; DD9: #25 restated after DD7; #14 restated: EPS/PDF pictures in exports need a PostScript interpreter). 2026-10-18 (B2, G1, B12, pict2e, TikZ, PSTricks, EMF, Tk (B1, B3, B9) landed; B11 added from the fig2dev audit). Ranked by user impact per hour. Pick work from the top.

| # | Gap | Area | Impact | Hours |
|---|-----|------|--------|------:|
| 1 | LaTeX exports closed: TikZ, PSTricks, pict2e, pstex/pdftex ported (B1, B2, B3); text bytes now as C (#27 closed 2026-10-18) | Export | High | 0 |
| 2 | Web build: compiles locally (2026-10-14); still to do: the wasm check in CI, trunk packaging and browser open/save/download | Platforms | High | 24 |
| 25 | Exports hand fig2dev the objects the Fig 3.2 text would give (the bridge, DD7; the writer's rules, not the model, decide), which drop arrows on pie-wedge arcs and refits arc centres from the rounded points instead of keeping the file's centre; fig2dev draws both from the file (`fig2dev_reference` arcs, 2 corpus bounding boxes). Saving loses them too | Export/File format | Medium | 3 |
| 26 | ✅ closed 2026-10-18 (audit pass 9): pictures get no frame in PS, EPS, PDF and bitmaps (genps strokes none; SVG keeps it) and transparent pixels stay transparent in EPS (colour-key `/ImageType 4`). Left: opaque pixels with the transparent colour are masked too (as fig2dev does for RGB images; for indexed GIFs it masks the index only), PNG with partial alpha is blended over white | Export | Low | 0 |
| 3 | PostScript output: page layout done 2026-10-18 (B4a); EPS previews (`-A`/`-T`/`-C`) and `-B`/`-R` done 2026-10-18 (B4b); printing remains | Export/Printing | Medium | 20 |
| 4 | Library objects: browse a directory of .fig files and place them | Libraries | Medium | 40 |
| 5 | ✅ closed 2026-10-21 (D1): open compound. Left: the dimension-line recomputation on close (D10); the save query landed 2026-10-21 | Editing | Low | 0 |
| 6 | Measure modes (join/split done 2026-10-21 D2, chop done 2026-10-22 D3, tangent/normal done 2026-10-22 D4) | Editing | Medium | 12 |
| 7 | Settings persistence (units, grid, recent files) and a style manager | Settings | Medium | 30 |
| 8 | The zoom box (rulers landed with W6) | View | Medium | 4 |
| 9 | Character map, compose-key input, LaTeX special-text handling | Text | Medium | 30 |
| 10 | Bitmap exports GIF/JPEG/TIFF/PPM (with transparent colour) | Export | Medium | 12 |
| 11 | Colour editor (create and edit user colours in the UI) | Attributes | Medium | 12 |
| 12 | Point positioning modes: latex lines, manhattan, mountain | Attributes | Low | 10 |
| 14 | EPS and PDF picture objects (B15 closed most of it, 2026-10-21): PDF pictures are vectors in PDF and EPS/PS output and rendered in bitmaps and on the canvas; EPS pictures go through ghostscript when installed (PDF, bitmaps, canvas), else the preview or a box. Left: PDF pages with transparency, shadings or patterns are an image in PostScript output; EPS pictures need `gs` outside PostScript output; encrypted PDFs; CMYK becomes RGB. | File format/Export | Low | 3 |
| 16 | Windows and Linux smoke tests in CI | Platforms | Low | 8 |
| 17 | Localization of UI strings | Localization | Low | 24 |
| 18 | Spell check | Text | Low | 16 |
| 19 | Accessibility (screen-reader labels for canvas objects) | Platforms | Low | 16 |
| 21 | closed: EMF, Tk/Perl-Tk ported (B9); the `sld` driver (slide output) is the one fig2dev language left unported | Export | Low | 4 |
| 27 | ✅ closed 2026-10-18: text bytes as C (`Text::raw`, `Comment::raw`; drivers write the input bytes). `-E` charsets beyond UTF-8/ISO-8859-1/-2 convert through `encoding_rs` (audit pass 27, 2026-10-18; no CP437/CP850, stderr messages with raw bytes and tk/pstricks picture names are Unicode) | Export | Low | 2 |
| 24 | fig2dev port hardening: the audit's partial and missing functions outside the unported drivers (options and messages, text bytes, depth order, FMA in bounds, EPS/PDF fonts and Unicode, SVG text, `readpics`, JPEG passthrough; B11) | Export | Medium | 66 |
| 28 | ✅ 2026-10-18 (audit pass 15): non-Latin-1 text in EPS/PS goes through fig2dev's Xfig-UTF8-H CMap and `composefont` fonts, in PDF through Type 3 fonts from RustFig's glyph outlines with a ToUnicode map (was `?`). Left: the 35 PostScript fonts are not embedded in PDF (substitutes; needs font files that cannot live in this repo), PDF text in Symbol/ZapfDingbats beyond WinAnsi, `-j` composite fonts | Export | Low | 12 |
| 20 | Real PostScript font metrics when craft-fonts lacks a face (synthetic bold/italic today); the A10 corpus shows text widths as the main remaining difference from fig2dev | Text | Medium | 12 |
| 29 | Main-window leftovers found by W10 group 1 ([window-parity.md](window-parity.md)): Help documentation entries (done: W10-2, W10-28), Show vertex numbers and Autorefresh (W10-3) | UI | Low | 13 |
| 30 | File-window leftovers found by W10 group 2 ([window-parity.md](window-parity.md)): Export panel options (W10-4), Open/Merge offset field, preview and comments (W10-6), Open of a missing name (W10-8), `.bak` on save (W10-9), New xfig button (no task); the Print panel is B5 | UI/Export | Medium | 25 |
| 31 | Edit-panel leftovers found by W10 group 3 ([window-parity.md](window-parity.md)): compound panel rows (W10-15), picture panel rows (W10-16), Done as one undo step and live point panel (W10-17); the position rows, arrow rows, text flags, arc type, figure comments and Cancel-restores were fixed in the same pass | UI | Medium | 16 |
| 32 | Settings and tool windows found by W10 group 4 ([window-parity.md](window-parity.md)): Global settings rows with effect (W10-23), Unit/Scale panel details (W10-24); the colour panel, character map, Manage Styles, font menus, spell check, libraries and zoom box are E4, E5, E6, E11, F5, D6, E3; Search and Replace results list, Update settings, empty search, About copyrights, Messages Dismiss/Clear and Global settings view rows were fixed in the same pass | UI | Medium | 11 |
| 33 | Attribute pop-ups and mode panel found by W10 group 5 ([window-parity.md](window-parity.md)): Arrow Size panel with absolute / multiple-of-line-width values and thickness (W10-31), Integer zoom and Return-sets for number pop-ups (W10-32), Align distribute/abut and isometric grid choices (W10-33), indicator buttons without their feature yet (Num Copies / X / Y D10, Tangent/Norm Length D4, Dimension line D10) | Unchanged function otherwise; stepping, ranges and tooltips fixed in W10 |
| 34 | Functions the xfig function audit found missing with no task ([source-audit.md](source-audit.md), S4): Shift-click cycling and highlight (S4-1), paste with the mouse (S4-2), relative picture paths on Save As (S4-3), a cursor per mode (S4-4), box ↔ arc-box convert (S4-5), shape factor on the canvas (S4-6), Shift/Ctrl variants in the mouse indicator (S4-7); 447 missing and 66 partial functions in all, each under its task | UI/Editing | Medium | 20 |

## Revision history

| Date | Note |
|------|------|
| 2026-10-22 | D3 closed Chop from #6 (tangent/normal and measure remain). |
| 2026-10-22 | D4 closed tangent/normal from #6 (measure remains). |
| 2026-10-21 | W10-1 (Update Control) done: removed from #29 and #33. |
| 2026-10-21 | #34 added (S4: function-level audit of xfig; S4-1 to S4-7). |
| 2026-10-21 | #30 added (W10 group 2: file windows). |
| 2026-10-21 | #31 added (W10 group 3: edit panels). |
| 2026-10-18 | #26 closed (audit pass 9: genps object fidelity in the scene builder). |
| 2026-10-18 | #27 remainder closed: `-E` charsets through `encoding_rs`. |
| 2026-10-18 | #27 closed (text bytes as C, audit passes 5 and 6). |
| 2026-10-18 | #1 closed (TikZ, PSTricks), #21 shrank to `sld` (EMF, Tk/Perl-Tk ported); #27 added (octal-escape text bytes). |
| 2026-10-17 | #1 shrank: pict2e ported (B3). |
| 2026-10-21 | DD9: #25 restated (exports come from the bridge's objects, not the Fig text; the writer's arc losses are unchanged). |
| 2026-10-16 | #25 added (B12: exports through the Fig writer expose its arc losses); #26 added (the corpus now exports pictures as the app does). |
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
| 2026-10-21 | #5: the "save just the visible part" query landed (D1 follow-up); W16/W17 removed the modern mouse scheme and layout. |
| 2026-10-14 | #0 closed: W7 landed, phase W done. |
| 2026-10-13 | Linked #1 to the fig2dev backend (B0); added #21 (reference tests, A8). |
| 2026-10-21 | #14 restated: EPS/PDF pictures in exports need a PostScript interpreter (fig2dev has none); the canvas half shrank with DD5. |
| 2026-10-21 | #14 shrank (B15): PDF and EPS pictures in exports and on the canvas; leftovers listed in the row. |
