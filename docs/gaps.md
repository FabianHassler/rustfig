# Known gaps (ranked work list)

2026-10-24 (G4 beta review: stale rows closed or shrunk). 2026-10-23 (D10 done: fractions of an inch landed; leftovers: preview arrowheads plain triangles, lengths while drawing decimal). 2026-10-23 (#34: S4-7 closed). 2026-10-22 (D5: #6 closed). 2026-10-22 (D4: #6 dropped tangent/normal). 2026-10-22 (D14: smart links extended, nothing listed here changed; the leftovers are the file-stored links xfig does not have). 2026-10-22 (D3: #6 dropped Chop; D8: linked move landed, #12 dropped "linked move", #33 dropped the Smart Links button). Earlier: 2026-10-21 (S4: #34 added; B15: #14 shrank to the leftovers; DD9: #25 restated after DD7; #14 restated: EPS/PDF pictures in exports need a PostScript interpreter). 2026-10-18 (B2, G1, B12, pict2e, TikZ, PSTricks, EMF, Tk (B1, B3, B9) landed; B11 added from the fig2dev audit). Ranked by user impact per hour. Pick work from the top.

| # | Gap | Area | Impact | Hours |
|---|-----|------|--------|------:|
| 1 | LaTeX exports closed: TikZ, PSTricks, pict2e, pstex/pdftex ported (B1, B2, B3); text bytes now as C (#27 closed 2026-10-18) | Export | High | 0 |
| 2 | Web build: compiles; trunk packaging (C1), browser open/save/export (C2) and browser autosave (C3) landed 2026-10-23; Merge, pictures (kept in memory by file name) and the bundled libraries (served files, lazy fetch) landed 2026-10-24; left: the manual browser check (nothing has run in a real browser yet), exports in the browser without in-memory pictures, EPS/PDF pictures, no user library or comment search there | Platforms | High | 6 |
| 25 | ✅ 2026-10-23: the Fig writer and the bridge keep arrows on pie-wedge arcs and the arc centre as read (floats, `%.3f` as xfig's `write_arcobject`; `to_model` no longer refits it from the rounded points); tests `bridge.rs::arcs_keep_centre_and_pie_arrows`. | Export/File format | Done | 0 |
| 26 | ✅ closed 2026-10-18 (audit pass 9): pictures get no frame in PS, EPS, PDF and bitmaps (genps strokes none; SVG keeps it) and transparent pixels stay transparent in EPS (colour-key `/ImageType 4`). Left: opaque pixels with the transparent colour are masked too (as fig2dev does for RGB images; for indexed GIFs it masks the index only), PNG with partial alpha is blended over white | Export | Low | 0 |
| 3 | ✅ mostly closed 2026-10-23 (B4a, B4b, B5, B5b: page layout, EPS previews, `-B`/`-R`, printing with the native dialog); left: the printer hand-off checked on Windows and Linux, the browser print tab in a real browser | Export/Printing | Low | 4 |
| 4 | ✅ closed 2026-10-23 (D6): library mode with the bundled libraries and the panel; the web build serves the bundled libraries since 2026-10-24 (C2; browser check left) | Libraries | Low | 6 |
| 5 | ✅ closed 2026-10-21 (D1): open compound; the dimension-line recomputation on close landed 2026-10-22 (D10); the save query landed 2026-10-21 | Editing | Low | 0 |
| 6 | ✅ closed 2026-10-22 (D5): measure modes (join/split D2, chop D3, tangent/normal D4) | Editing | Low | 0 |
| 7 | ✅ 2026-10-22 (E1, E6; E9 theme and scale 2026-10-23): settings persistence (units, grid, recent files, export options per format) and the style manager with families | Settings | Done | 0 |
| 8 | ~~The zoom box~~ (done 2026-10-22, E3; rulers landed with W6) | View | Medium | 4 |
| 9 | ✅ 2026-10-24 (F3 dead keys and compose through IME, F2 LaTeX special text on screen and in exports, F8 WYSIWYG LaTeX, E5 character map, F5 spell check). Right-to-left and complex-script shaping (F6) dropped 2026-10-31 (not needed); fonts without Latin letters are not offered | Text | Low | 6 |
| 10 | ✅ closed (Export panel rows and fig2dev's drivers: GIF with transparent colour, JPEG quality, TIFF, PPM, PCX, XBM, XPM; W10-4, B9) | Export | Done | 0 |
| 11 | ✅ 2026-10-22 (E4, E4b): colour editor for user colours, recent colours, pick from screen. Left: xfig's mix memory and lock | Attributes | Done | 0 |
| 12 | ✅ 2026-10-22 (D7): Angle Geom indicator (unconstrained, latex line/arrow, mountain, manhattan, mount-hattan; `crates/engine/src/anglegeom.rs`); linked move done with D8 | Attributes | Done | 0 |
| 14 | EPS and PDF picture objects (B15 closed most of it, 2026-10-21): PDF pictures are vectors in PDF and EPS/PS output and rendered in bitmaps and on the canvas; EPS pictures go through ghostscript when installed (PDF, bitmaps, canvas), else the preview or a box. Left: PDF pages with transparency, shadings or patterns are an image in PostScript output; EPS pictures need `gs` outside PostScript output; encrypted PDFs; CMYK becomes RGB. | File format/Export | Low | 3 |
| 16 | ✅ 2026-10-24 (P1): `ci.yml` runs `cargo xtask ci` on Linux, Windows and macOS on every push and passes. Left (not CI): the manual Windows/Linux checks in `hardware-parity.md` | Platforms | Done | 0 |
| 17 | ❌ dropped 2026-10-22 (F4, the user: RustFig stays English only) | Localization | Dropped | 0 |
| 18 | ~~Spell check~~ (done 2026-10-23, F5; macOS's own checker as a fallback left) | Text | Low | 0 |
| 19 | Accessibility: G3 done 2026-10-24 (AccessKit on, every interactive widget has an accessible label, `canvas::describe_drawing` names the drawing for the canvas). Left: a real screen-reader check on each OS, per-object navigation on the canvas | Platforms | Low | 6 |
| 21 | closed: EMF, Tk/Perl-Tk ported (B9); the `sld` driver (slide output) is the one fig2dev language left unported | Export | Low | 4 |
| 27 | ✅ closed 2026-10-18: text bytes as C (`Text::raw`, `Comment::raw`; drivers write the input bytes). `-E` charsets beyond UTF-8/ISO-8859-1/-2 convert through `encoding_rs` (audit pass 27, 2026-10-18; no CP437/CP850, stderr messages with raw bytes and tk/pstricks picture names are Unicode) | Export | Low | 2 |
| 24 | fig2dev port hardening, leftovers after passes 1–7 (fig2dev v0.9.1): small items (box comments, readpics zip/bz2/xz, palette order, textconvert messages; pass 8), `-j` composite fonts (pass 9), fill patterns in smoothed bitmaps (B11) | Export | Low | 13 |
| 28 | ✅ 2026-10-18 (audit pass 15): non-Latin-1 text in EPS/PS goes through fig2dev's Xfig-UTF8-H CMap and `composefont` fonts, in PDF through Type 3 fonts from RustFig's glyph outlines with a ToUnicode map (was `?`). Left: the 35 PostScript fonts are not embedded in PDF (substitutes; needs font files that cannot live in this repo), PDF text in Symbol/ZapfDingbats beyond WinAnsi, `-j` composite fonts | Export | Low | 12 |
| 20 | ~~Text widths differ from fig2dev~~ (reclassified 2026-10-25: with the PostScript fonts installed the widths match, F1; the differences the A10 corpus shows come from the built-in fallback fonts those tests force on purpose. Without matching installed fonts, and on the web, bold/italic are synthetic: an environment limit, not a gap) | Text | — | 0 |
| 29 | ✅ closed 2026-10-22: main-window leftovers found by W10 group 1 ([window-parity.md](window-parity.md)): Help documentation entries (W10-2, W10-28), Show vertex numbers (W10-3), Autorefresh and Update Control (S4-9, W10-1) | UI | Done | 0 |
| 30 | File-window leftovers found by W10 group 2 ([window-parity.md](window-parity.md)): Export panel options (W10-4), Open of a missing name (W10-8), `.bak` (W10-9), export preview (W10-10), remembered options (W10-11), background export (W10-13), saved versions (W10-14) and the Print panel (B5) are done; still open: the in-app Open/Merge panel with Figure Offset field, preview and comments (W10-6, W10-12), New xfig button (no task, low value) | UI | Low | 8 |
| 31 | ✅ closed 2026-10-22: edit-panel leftovers found by W10 group 3 ([window-parity.md](window-parity.md)): the picture panel W10-16/W10-20, the compound panel W10-15, the live panel with Done as one undo step W10-17/W10-18, the point table W10-19 and the pictorial menus W10-21 are done; the position rows, arrow rows, text flags, arc type, figure comments and Cancel-restores were fixed in the same pass | UI | Medium | 0 |
| 32 | ✅ closed 2026-10-22: settings and tool windows found by W10 group 4 ([window-parity.md](window-parity.md)): Global settings rows with effect (W10-23), Unit/Scale panel (W10-24), colour panel E4, character map E5, Manage Styles E6, font menus E11, spell check F5, libraries D6, zoom box E3; the optional proposals (W10-22, W10-25 to W10-30) are plan tasks beyond xfig | UI | Done | 0 |
| 33 | ✅ closed 2026-10-24: attribute pop-ups and mode panel found by W10 group 5 ([window-parity.md](window-parity.md)): Arrow Size (W10-31), number pop-ups (W10-32), Align and grid choices (W10-33), and the indicator buttons whose feature was missing (Num Copies / X / Y and Dimension line D10, Tangent/Norm Length D4) | UI | Done | 0 |
| 34 | Functions the xfig function audit found missing with no task ([source-audit.md](source-audit.md), S4): Shift-click cycling and highlight (S4-1 ✅), paste with the mouse (S4-2 ✅), relative picture paths on Save As (S4-3 ✅), a cursor per mode (S4-4 ✅), box ↔ arc-box convert (S4-5 ✅), shape factor on the canvas (S4-6 ✅), Shift/Ctrl variants in the mouse indicator (S4-7 ✅), Digitize (S4-8 ✅), Autorefresh and New window (S4-9 ✅); 447 missing and 66 partial functions in all, each under its task | UI/Editing | Medium | 20 |
| 35 | ~~macOS: a `.fig` double-clicked in Finder reaches the app as an Apple Event, not argv~~ Closed (P3): `rustfig-os` `open_docs` registers an `aevt`/`odoc` handler at `NSApplicationWillFinishLaunchingNotification`; the UI opens the queued files each frame (asks before discarding changes). Packages stay unsigned (P3, user decision). | Platforms | Closed | 0 |

## Revision history

| Date | Note |
|------|------|
| 2026-10-24 | G4 beta review: #7, #9, #10, #11, #12, #16, #29, #32, #33 closed, #17 dropped (F4), #2, #3, #19, #30 shrank, checked against code and plan rows. |
| 2026-10-23 | F1 shrank #20 (system fonts and TeX Gyre; craft-fonts gone). |
| 2026-10-23 | D10 closed its last item (fractions of an inch in dimension-line lengths); small leftovers noted in plan.md. |
| 2026-10-22 | D3 closed Chop from #6 (tangent/normal and measure remain). |
| 2026-10-22 | D4 closed tangent/normal from #6 (measure remains). |
| 2026-10-22 | D5 closed #6 (measure modes). |
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
| 2026-10-26 | #24 shrank (B11 passes 1–7, fig2dev v0.9.1: suite 88 of 88, SVG byte-identical with C, gs-like bitmaps). |
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
| 2026-10-23 | #34: S4-7 done. |
| 2026-10-23 | #2: C1 done (trunk packaging, `cargo xtask web`). |
| 2026-10-23 | #2: C2 done (browser file I/O); pictures, Merge and Export on save are desktop-only for now. |
| 2026-10-24 | #2 shrank (C2): Merge, pictures in memory and the bundled libraries on the web; browser check and exports with pictures left. |
| 2026-10-23 | P2: macOS checks done, two bugs fixed (huge canvas texture, non-UTF-8 arguments); Windows and Linux rows are manual checks in `docs/hardware-parity.md`. |
