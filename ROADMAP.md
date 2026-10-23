# RustFig roadmap

**Status (2026-10-23):** Stage **Alpha**. Overall xfig parity **≈84 %**
([assessment](docs/target-app-parity.md)). The native app draws, edits,
saves and exports. The web build compiles; it is not yet packaged or tested in a browser.

## Stage

| Stage | Meaning | State |
|-------|---------|-------|
| Prototype | Core model, file format, rendering | done |
| **Alpha** | Usable for everyday figures; gaps in export formats, libraries, print | **current** |
| Beta | every everyday xfig function; preferences; native LaTeX exports; Windows, Linux and web verified | next |
| 1.0 | every xfig function available (libraries, digitize, print, all exports), plus web, MCP and automation polish | later |

## Headline numbers

| Measure | Value |
|---------|-------|
| Overall parity (weighted) | ≈84 % |
| Commands (all reachable from the control channel and MCP) | 223 engine (incl. 44 tool modes and "no mode") + 29 UI |
| Interactive tools (xfig modes) | 44 (xfig's 43 + Trim/Split/Extend) |
| Tests | 598 (unit, integration, property-based fuzzing, headless UI, fig2dev reference comparison) |
| Fig format | read 1.3–3.2, write 3.2 |
| Export formats | all through fig2dev's ported drivers (`convert`): PDF, SVG, EPS, PNG, GIF, JPEG, TIFF, PPM, PCX, XBM, XPM, EPS+PDF pair, LaTeX box from its scene writers (PostScript without page layout yet); LaTeX picture, epic/eepic/eepicemu, PiCTeX, TeXtyl, tpic, pic, pstex/pdftex, pict2e, CGM, HPGL, DXF, MetaPost, METAFONT, ge, image map, Gerber, shape, TikZ, PSTricks, EMF, Tk/Perl-Tk from ported fig2dev drivers (every fig2dev language except `sld`); fig2dev's own suite 79 pass / 9 fail / 1 skipped |
| Perf, 50k paths (render fit / hit test / open / edit + undo) | 260 ms / 0.1 ms / 20 ms / 0.9 ms |

## Parity by dimension

| Dimension | Parity | Details |
|-----------|--------|---------|
| Drawing modes | 95 % | all xfig shapes and library objects (D6); freehand and arc by centre |
| Editing modes | 81 % | measure modes done; missing anchor point, array copy, align to canvas |
| Attributes panel | 80 % | xfig's per-mode buttons; missing colour editor, latex/manhattan positioning |
| File format | 92 % | Fig 1.3–3.2 read; exact pre-3.2 spline conversion missing |
| Export | 66 % | every fig2dev language but `sld`; PostScript page layout and previews, hardening passes missing |
| Text | 55 % | fallback fonts, no character map or LaTeX special text |
| View & navigation | 96 % | xfig's layout and three-button mouse (the only ones, W16/W17), depth panel, rulers and units box, object snaps; zoom box missing |
| Undo | 100 % | multi-level (xfig: single level) |
| Libraries | 85 % | 30 xfig libraries bundled, panel with thumbnails, placing, copy to library, user folder (D6); xfig's library list files and the web build missing |
| Printing | 10 % | export PDF only |
| Settings & styles | 15 % | no persistence, style manager |
| Platforms | 50 % | macOS verified; web build compiles; Windows/Linux and the web app at run time unverified |

## Features beyond xfig

- Multi-level undo/redo.
- A modern Select tool (click, shift-click, rubber band, drag to move, vertex handles).
- JSON-lines control channel, MCP server (`rustfig-cli mcp`), command-line converter.
- Native `.rustfig` JSON format.

## Outlook

- A WYSIWYG editor for LaTeX text in figures (special text typeset on the canvas as it will appear in the document). An idea for after 1.0; not planned yet.

## Languages

English only. See [localization parity](docs/localization-parity.md).

## Upcoming

The phased plan is [docs/plan.md](docs/plan.md); every shortfall is ranked in
[docs/gaps.md](docs/gaps.md). Next up:
1. The rest of the fig2dev port: the audit's hardening passes (B11; text bytes done), PostScript page layout and previews (B4a/B4b), `sld`; then fig2dev moves to its own repository `FabianHassler/fig2dev` (see plan decisions).
2. ✅ Phase A, hardening (done 2026-10-15): shared undo snapshots, per-command cost, CI (Linux on every push, all OSes for releases), the web build check, UI tests, export curves, crash recovery, stall-free panning on large drawings.
3. ✅ Preferences that survive restarts (E1) and xfig's command-line options (S3, 2026-10-22: `~/.xfigrc` import and an `xfig` launcher name left).
4. Phase B: fig2dev's drivers ported (LaTeX exports, bitmaps, PostScript, the rest), modern printing (PDF to the system print dialog).
5. Phase C, web: trunk packaging and browser file I/O.
6. Every remaining xfig function, including xfig's bundled figure libraries (D6) and D11.

## Progress log

| Date | Change |
|------|--------|
| 2026-10-23 | D6: library mode. xfig's libraries in `libraries/` (30 libraries, 2,540 figures; Charts, house_plans and transit left out for their licences), `ASSETS.md` rows and an extended `cargo xtask assets`; `library.*` commands, library tool state with xfig's buttons and keys, the library panel with thumbnails, Copy to library, manual topic; parity ≈79 → ≈84 %. |
| 2026-10-23 | E5 (character map lists only drawable characters), S4-7 (Shift/Ctrl meanings in the mouse-function indicator), D10 done (dimension-line lengths as fractions of an inch, example-line preview and arrow pictures in the Dimension line window), W10-14 (saved versions: `file.versions`, `file.restore_version`, File > Saved versions…), W9 fix (cap and join pictures redrawn as xfig's). 2 engine commands. |
| 2026-10-23 | Parity ≈73 → ≈79 %: B5 printing with preview (Printing 10 → 85 %), C1 web packaging, C2 browser file I/O, P2 platform checks (Platforms 50 → 70 %); F1 system fonts (fig2dev v0.7.0). |
| 2026-10-22 | Recount: parity ≈65 → ≈73 % (Text 55 → 65 %, Settings & styles 15 → 75 %). Since the last entry: D5 measure modes, S3 command line, W10 panels (picture, compound, edit panel live, point table, export panel), D10 mode options, S4 (locate, paste, cursors, convert, shape factor, relative pictures, Digitize, Autorefresh, New window), E3 zoom box, D16 Trim mode, D17 Escape levels, E10 text editing, E4/E4b colours and pick from screen, E6 styles, D18 no selection layer (xfig's mode-first way). 214 engine commands, 589 tests. |
| 2026-10-22 | W10-28, W10-2: the bundled, searchable, offline manual: 27 Markdown topics in `docs/manual/` compiled into the app (`crates/ui-egui/src/help.rs`: topic list, search, a small Markdown renderer, links between topics), opened by Help > RustFig Manual (xfig's Help `x`) and by the UI command `help.open` with `topic`; every mode has a section quoting the mouse indicator's words, tests fail when a mode is undocumented, a quote drifts or a link breaks. Help menu: RustFig Manual, command reference, About. |
| 2026-10-22 | P1 (workflow ready, first run pending): `ci.yml` runs the gates on Linux, Windows and macOS (wasm check and perf budgets on Linux only, `RUSTFIG_SKIP_WASM` for the others, rust-cache per OS); `.gitattributes` forces LF. |
| 2026-10-22 | D7: Angle Geom. The indicator button with xfig's six pictures (polyline, polygon, spline, move point and add point modes) and `indicator.set angle_geom`; unconstrained, latex line and arrow slopes, manhattan, mountain, mount-hattan constrain the next point on click and in the rubber band (ported from `u_elastic.c`, `u_geom.c`). Attributes 80 → 83 %. |
| 2026-10-22 | E14: Edit › Copy as Picture (`edit.copy_picture`): the selection, or the whole figure, goes to the system clipboard through fig2dev's drivers as the platform's vector picture (macOS PDF, Linux SVG; Windows EMF, SVG until the driver is used) plus PNG, with the figure's magnification and the export parameters; native only (`clipboard-rs`), tested through a fake clipboard. |
| 2026-10-22 | D14: smart links extended. Targets also circles, ellipses, arcs, polygons, closed splines, text boxes (end within 45 Fig units of the outline); followers also open splines; a line between two moved shapes moves both ends; the attached end goes through move, nudge (`transform.nudge`), rotate, flip, scale, align and align-to; copy copies the attached lines; Move point on a box corner stretches them; Select highlights attached ends and the previews show every follower. `tests/linked_ext.rs` (13), 1 engine command. |
| 2026-10-22 | D8: linked move. Moving a box drags the end points of lines attached to its perimeter (within 45 Fig units), and a compound drags the ends of lines inside it; a Smart Links indicator button (off/move/slide) sets `LinkMode`; `engine/links.rs` ports `u_list.c`'s `get_links`/`point_on_perim`/`get_interior_links`/`point_on_inside`/`adjust_links`; Move moves the end point, Slide also slides the adjacent point. `tests/linked_move.rs` (5). |
| 2026-10-22 | D15: Chop extended: `object.chop` on boxes, arc-boxes, ellipses, splines and compounds (splines and compounds also as axes; X-spline pieces; arrowheads only at the original ends); commands `object.trim`, `object.split_at`, `object.cut_all`, `object.extend`; Chop mode's Shift+left/middle/right; `tests/chop_extended.rs` (14). 4 engine commands, 14 tests. |
| 2026-10-22 | S4-1, S4-2, S4-4, S4-5: Shift+click Locate Object in the picking modes (Chop's and Rotate's Shift variants gave way to it), Paste follows the pointer (left, middle orig posn, right cancel), a cursor per mode, Convert box/arc-box and spline kind round trip. |
| 2026-10-22 | D5: measure modes (`e_measure.c`): angle by three points or at a line corner or arc centre (middle button also stores the rotation angle), length and area with running totals (signed areas), messages in xfig's wording and the drawing's units, commands `object.measure_angle/_length/_area/_reset`, tools `tool.angle_measure/length_measure/area_measure` (Ctrl+G/N/M), manual topic `measure.md` (28 topics). |
| 2026-10-22 | D4: Tangent/normal mode (`e_tangent.c`, `u_smartsearch.c`, `u_geom.c`): command `object.tangent_line` (a line through a curve point of a circle, ellipse, arc, line, polygon, box or spline, along its tangent or normal, one undo step), mode `tool.tangent` (`n`; left button tangent, middle button normal), Tangent/Norm Length setting and button; tests `crates/engine/tests/tangent.rs`. |
| 2026-10-22 | D3: Chop mode (`e_chop.c`, `w_intersect.c`): command `object.chop` (cut a polyline, polygon, arc or circle at every intersection with the axe objects, pieces keeping the object's attributes, one undo step) and mode `tool.chop` (`x`; left collects axe objects, middle chops the log, right clears the axes); intersections in `crates/geom/src/intersect.rs`; elliptical arcs unsupported as xfig; `tests/chop.rs` (10) and a mouse-hint fixture. 2 engine commands, 1 tool, 15 tests. |
| 2026-10-22 | W20: platform edit keys next to xfig's: Cmd+X/C/V on macOS (and Ctrl+X/V elsewhere) now reach cut, copy and paste; menus show xfig's accelerator, or the platform shortcut where xfig has none and its keys leave it free (on Linux and Windows xfig's Ctrl keys win). |
| 2026-10-21 | E15: Export on save — the Export dialog's "Also export on every save" keeps the chosen file and options up to date on every save; `file.export_on_save` command, per-figure `Document::export_on_save`, stored in `.fig` as a `# RustFigExportOnSave` comment and in `.rustfig` as JSON; save stands when the automatic export fails. |
| 2026-10-21 | W10-1: Update Control: every attribute button has an update box in Update mode, plus Set/Clear/Toggle all; Update, update region, lasso and update settings copy only the flagged attributes; commands `indicator.set_update_flag`, `indicator.update_flags`. |
| 2026-10-21 | W15: lasso in every region selection (press and drag with the middle or right button in Glue, Delete, Update, Move and Delete point selects what lies completely inside the freehand path; click, move, click keeps xfig's rectangle; drawn in the overlay, hints say "drag: lasso"); fixed Delete's middle region with an empty selection. |
| 2026-10-21 | W19: attribute panel tightened: xfig's labels and layout kept, each button narrowed to what its label and value need (a short-caption version was tried and reverted at the user's request); thinner join and cap pictures. |
| 2026-10-21 | W16, W17: one mouse scheme (xfig's), one layout, no attribute-panel or icon switch; RustFig's extras in Edit, View, File and a new Object menu (every command reachable from a menu or listed as menu-less, tested); D1 follow-up: xfig's Save All / Save Part / Cancel query when saving inside an open compound (`part` param on save and export commands). One engine command (`tool.mouse_buttons`) and three UI commands fewer. |
| 2026-10-21 | W13, W14: functions on xfig's free mouse buttons (update, move and delete-point regions; add point at an end; reverse arrows; break all levels; align to first; centre boxes and pictures, Shift square; picture keeps aspect; regular polygon by one side; circle through three points; paste text), 8 new commands, `tests/free_buttons.rs` (17), hints checked by `mouse_hints_match_behaviour`; Scale right skipped (already proportional). |
| 2026-10-21 | S4: function-level audit of xfig (`docs/source-audit.md`, `cargo xtask source-audit`): 2,126 functions, each ported, replaced, not needed, partial or missing with its task; seven gaps without a task became S4-1 to S4-7 (20 h); `docs/source-map.md` statuses corrected. |
| 2026-10-21 | W10 group 5 (attribute pop-ups, mode panel) compared with xfig's sources; W10 done. Indicator stepping as xfig (rotation presets, 15 degree angles, font size and text step ladders, fill wrap and tint skip), dash length and dot gap both shown, xfig's mode balloons, Alt+right is middle; W10-31 to W10-33 added. |
| 2026-10-21 | A11: control channel `ui.pointer`, `ui.key`, `ui.click_label` (queued, fed through `raw_input_hook` like real events, reply after processing) and `ui.state` with open pop-ups/dialogs and labelled widget rectangles (egui accessibility tree); tests in `ui_smoke.rs` and `inject.rs`; every top menu, a submenu, an attribute pop-up and the Export dialog imaged in the running app (`target/window-parity/a11-*.png`); W10 group 1 menus re-checked with them: Unzoom showed RustFig's own shortcut and Zoom Out "Z" (xfig: none, "z"), fixed. |
| 2026-10-21 | W11: 3D (bevelled) buttons in every panel with a Button style setting (3D/Flat, saved), mode panel fits at the default size, compact Mouse Buttons box without the keyboard label, Snap Mode chooser with a HOLD mark, editable file-name box (`file.rename`), rulers driven by the button events (left/right/middle/Alt+right/wheel/left-drag, tested for both rulers). |
| 2026-10-21 | W12: xfig's object markers in the editing modes (`u_markers.c`, per-mode masks of `w_modepanel.c`): small squares on box/line/spline points, ellipse and arc points, text anchor and top, compound corners, only on what the mode can act on, hidden depths and hidden text as xfig; culled to the view; perf budgets unchanged. |
| 2026-10-21 | W10 group 3: edit panels of every object kind compared with captures of the real X11 xfig; the panel now has xfig's position rows (box and picture corners, polyline/polygon/spline points, ellipse centre/radii/angle, arc points, text origin) with a Units choice, arrow type and size per arrow, arc type, text hidden/rigid/TeX flags, picture flip, cap/join only where xfig has them, and Cancel restores after Apply; Edit mode's middle button edits the figure comments and the right button a spline point's shape factor; commands `object.set_geometry` and more `object.set_attributes` parameters; findings W10-15 to W10-17 (gaps #31), six proposed modernisations. |
| 2026-10-21 | W10 group 4: settings and tool windows (Global settings, Unit/Scale, Search and Replace, About, Messages, colour, font, style, spell, library, character map windows) compared with the real X11 xfig; Search and Replace now lists matches with positions, has Update settings, matches every text on an empty search and asks twice before removing text; About lists xfig's copyrights; Messages has Dismiss and Clear; Global settings repeats the page-border, depth-manager and balloon switches and the balloon delay; findings W10-23, W10-24 (gaps #32), proposals in docs/window-parity.md. |
| 2026-10-21 | W10 group 1: main window and all menus compared with xfig (from its sources; the Qt build of xfig could not be captured), `docs/window-parity.md` lists all windows in 5 groups; findings W10-1 to W10-3 (gaps #29). |
| 2026-10-21 | W10 group 2: Open, Save As, Merge, Export and Print compared with captures of the real X11 xfig (hidden Xvfb display), group 1 re-checked; findings W10-4 to W10-10 (gaps #30); fixed: merge as one compound, Figure Offset in `file.open`/`file.merge`, export refuses an input picture file; modernisations proposed (print dialog with preview, export preview). |
| 2026-10-21 | B15: PDF and EPS picture objects. PDF pages stay vectors in PDF and PS/EPS output (hayro, hayro-write), are rendered in bitmaps and on the canvas; EPS pictures go through ghostscript when installed (optional, PATH lookup, never on wasm). fig2dev v0.6.0 (MSRV 1.92); fig2dev suite 82/6 unchanged, corpus sample unchanged (EPS/PDF/PNG medians 0.0/0.0020/0.0174, 0 errors). |
| 2026-10-21 | DD9: docs sweep and dead-code cleanup closing the dedup series (reader, bounds, pictures, exports and the `fig2dev` command line all through fig2dev; `fig_read.rs` 662 → 83 lines); no headline number moved. |
| 2026-10-21 | DD7: exports hand fig2dev's drivers the bridge's objects, no Fig text round trip (the text path stays as reference and as fallback where the reader would print a message); byte-identical over fixtures x every language x 9 option sets, the 300-figure sample and 129,108 generated-document exports; fig2dev 0.5.0 exposes the reader's repairs (`repair_line`, `repair_arc`, validators) which the bridge now calls; export SVG (50k paths) 108 → 86 ms; corpus sample unchanged. |
| 2026-10-20 | DD3: one Fig reader (fig2dev's, through the bridge) for versions 1.3–3.2; `fig_read.rs` 662 → 83 lines; fig2dev's reader in `Ctx::editing` mode (xfig's `f_read.c` rules, nothing repaired that xfig keeps); differential run old vs new over 2,572 figures (all parse in both, 2,469 equal, 103 differ only where `f_read.c` does the same); open .fig 19.0 → 15.7 ms; corpus sample unchanged. |
| 2026-10-20 | B13: fig2dev v0.2.0 (`sld`, JPEG passthrough in EPS, gensvg/genps headers, hardened picture handling); `sld` now exports; all gates pass; corpus sample (300 figures, 0 errors) EPS/PDF/PNG medians in the report. |
| 2026-10-20 | DD5: the canvas loads pictures through fig2dev's `readpics` first (PNG gamma, GIF transparency, XBM, XPM, PCX, `.gz`), the `image` crate only as fallback; canvas pixels equal the exports'. |
| 2026-10-21 | DD10: fig2dev bundled: `rustfig fig2dev ARGS`, `rustfig-cli fig2dev ARGS` and the `rustfig` binary started under the name `fig2dev` run fig2dev's command line (stdout, stderr, exit status identical, tested). Packaging of the link is open. |
| 2026-10-20 | DD2: bounds for fit, selection box, `doc.info` and rubber-band selection come from fig2dev's `bound` through the bridge (`bridge/bounds.rs`, cached per object in the engine); `Document::bbox` removed; equals fig2dev's on fixtures, the 300-figure sample and generated documents; perf `bounds (fig2dev)` 26 ms/50k paths, rest unchanged. Audit 12b closed. |
| 2026-10-20 | DD1/DD8/DD10 (fig2dev side): model ↔ fig2dev object bridge in `rustfig-format` (`bridge/`, `to_model` moved down from the engine, new `from_model`; fixtures, 300-figure sample and generated documents round-trip and equal fig2dev's reader); fig2dev gets `Ctx::for_figure`, `Figure`, `splines_to_lines` and `fig2dev::cli::run` (fig2dev v0.3.0). Perf unchanged. |
| 2026-10-20 | B14: fig2dev v0.2.0 (picture readers take `&mut Ctx`, JPEG passthrough, `-L sld` ported, gensvg/genps headers, hardening passes); `sld` export now works. Corpus sample (300): EPS median 0.0015, PDF 0.0020, PNG 0.0174, 0 errors. |
| 2026-10-18 | B13: `figbase`, the fig2dev library and the `fig2dev` program move to their own repository (`FabianHassler/fig2dev`, history kept; prepared locally, not yet published); RustFig takes them as git dependencies with a temporary local `[patch]`. |
| 2026-10-18 | B11 passes 5 and 6: texts and comments keep the input's bytes (`Text::raw`, `Comment`), every ported text driver writes them as fig2dev does (no UTF-8 re-encoding); 300 of 300 sample figures identical in the TeX, plotter, EMF, Tk, map and pic drivers (was 285); EPS/SVG/PDF/PNG unchanged. Gap #27 closed. |
| 2026-10-18 | B1, B3, B9: fig2dev's TikZ, PSTricks (`-p` writes picture EPS in-process, PNG pixel-identical to bmeps 2.2.27), EMF, Tk and Perl/Tk drivers ported, with pict2e: byte-identical to fig2dev 3.2.9a on the fixtures and the suite's figures and 285 of 300 sample figures (the rest: texts with `\ooo` octal escapes; reader keeps raw bytes, audit pass 5, next). Every fig2dev language is ported except `sld`. fig2dev's suite: 79 pass, 9 fail, 1 skipped, 0 unported (was 62/9/17). Export 57 → 66 %, parity ≈65 %. 509 tests. |
| 2026-10-17 | B3: fig2dev's pict2e driver ported, byte-identical to fig2dev 3.2.9a on the fixtures, the suite's figures and 285 of 300 sample figures (the rest: octal-escaped text bytes); every driver now orders objects of one depth with the macOS `qsort`, as the reference. fig2dev's suite: 73 pass, 9 fail, 6 need unported languages. 475 tests. |
| 2026-10-16 | B12: fig2dev is a program of its own that RustFig depends on. Shared crate `figbase` (geometry, fonts, colours); the scene, rasteriser, fill patterns and EPS/PDF/SVG/bitmap writers live in fig2dev's crate, which uses no other workspace crate; every export writes Fig text and runs fig2dev's `convert` with xfig's option letters; `rustfig-render` only draws the canvas. Corpus median/p95 EPS .0028/.0283 → .0025/.0277, PDF .0027/.0281 → .0024/.0273, PNG .0266/.0692 → .0251/.0652; SVG export 249 → 109 ms; fig2dev's suite unchanged (62 pass); 464 tests. |
| 2026-10-16 | The app opens Fig 1.3–2.1 files through fig2dev's ported reader (G1), now the only reader of every fig2dev language; pstex/pdftex ported (B2: the `_t` file byte-identical to fig2dev's), the app writes xfig's file pairs; every ported fig2dev language exports from the app, the export dialog and the CLI; pictures in exports (.gz/.Z, XBM, XPM, PCX; EPS embedded in EPS). fig2dev's own suite: 61 pass, 10 fail (was 44/27). A function-level audit of the port added B11 (hardening, 66 h). Parity ≈64 %. |
| 2026-10-15 | B6: GIF, JPEG, TIFF, PPM, PCX, XBM and XPM export from RustFig's renderer (no ghostscript), antialiased by default. B9 started: fig2dev's reader (Fig 1.3–3.2), bounds, spline conversion and driver helpers ported into a new crate; the standalone fig2dev now reports fig2dev's read messages (its suite: 37 pass). 357 tests. |
| 2026-10-15 | B10: a standalone `fig2dev` with fig2dev 3.2.9a's command line, messages and exit codes (`-E` input encodings too); fig2dev's own test suite runs against it: 15 pass, 35 wait for unported languages. 238 tests. |
| 2026-10-15 | Start values follow xfig's code: point positioning magnet (1/16 inch), no alignment, stick arrows, dot gap 3 apart from dash length 4. 230 tests. |
| 2026-10-15 | E1: preferences survive restarts: settings.toml with xfig's resources under xfig's names (editable in Edit › Global settings › xfig resources), RustFig's switches, recent files (File menu) and your own colours in every colour menu; browser storage on the web. 228 tests. |
| 2026-10-15 | A7: panning and zooming a 50,000-object drawing stays at 1–3 ms per frame (was 330 ms): cached scene, the last image moved at once, the exact one rendered in the background. Phase A done. |
| 2026-10-15 | A6: crash recovery: unsaved drawings are copied a few seconds after each change; after a crash or kill the next start offers to restore them (OS file locks tell running instances from stopped ones). 219 tests. |
| 2026-10-15 | A5: ellipses, arcs and rounded boxes are true curves in SVG, PDF and EPS (an ellipse SVG 169 KB → 2 KB); corpus EPS median difference to fig2dev 0.0031 → 0.0028. 214 tests. |
| 2026-10-15 | W9 done: all 107 of xfig's icon bitmaps redrawn as vectors (sharp on HiDPI; xfig's bitmaps optional), and RustFig has an application icon, drawn in code. 210 tests. |
| 2026-10-15 | W9 started: the attribute pictures (line styles, joins, caps, arrows, arcs, justification) are drawn as vectors at the screen's density by RustFig's own renderer; arrowhead types 10–14 can be chosen. 207 tests. |
| 2026-10-15 | W8 done: every xfig key and mouse binding works, has a modern equivalent, or waits for a named task; added the menu keys and letters, Meta-Y balloons, Alt+wheel mode menus, the compose key (97 sequences), wheel on spinners, Return/Escape in panels. 203 tests. |
| 2026-10-15 | A2, A3: GitHub Actions CI (every push, Linux, wasm check, fig2dev reference tests, perf budgets) and release builds for Linux, Windows and macOS. |
| 2026-10-15 | W8 started: every xfig key and mouse binding audited (`docs/keys-parity.md`); arrow keys pan, Home pans to the origin, Home/End in text, Shift-Meta-X exports with the current settings, the wheel steps indicator values, Escape closes panels. 195 tests. |
| 2026-10-15 | A4: headless UI tests open every dialog, drive Save As/Open/Merge/Import through scripted file dialogs and check the unsaved-changes prompt on Quit. 191 tests. |
| 2026-10-15 | A9: commands re-measure only the texts they changed, and hit testing recomputes only changed boxes: a one-object edit among 5,000 texts 1.7 → 0.4 ms. 188 tests. |
| 2026-10-15 | A1: undo snapshots share unchanged objects: 200 one-object edits on a 50,000-object drawing hold 0.08 MB of undo (was a full copy per step, about 2.9 GB); edit and undo 0.4 ms each; history capped at 200 steps and about 256 MB. 186 tests. |
| 2026-10-15 | B0: fig2dev's export options (`-D`, `-K`, `-m`, `-Z`, `-s`, `-b`, `-g` with X11 colour names, `-N`, `-n`, `-f`, `-F`) on `file.export`, listed per language by `file.export_formats`, CLI `--option`; bounding boxes match fig2dev's with them. 182 tests. |
| 2026-10-15 | D12: fill patterns ported from fig2dev's PostScript prolog (cells, line widths, arcs; physical size in exports). Corpus EPS median difference 0.0031, 95th percentile 0.031. |
| 2026-10-14 | B0: exports sized by fig2dev's bounds (`bound.c`): EPS bounding boxes equal fig2dev's on 2,532 of 2,568 sample figures (the rest within 1 pt); corpus EPS median difference 0.015 → 0.006. 171 tests. |
| 2026-10-14 | A10: RustFig's EPS, PDF and PNG compared with fig2dev's on xfig's ≈2,550 sample figures (opt-in test, ghostscript test-only), with a report ranked worst first; metric text no longer grows with the drawing; files without the centring header line open. Baseline over 2,569 figures: EPS median difference 0.015. 168 tests. |
| 2026-10-14 | B0 started: xfig's export languages (all 40, unported ones greyed with their plan task), LaTeX box ported from fig2dev, EPS+PDF pair, `file.export_formats`, CLI `formats`. 166 tests. |
| 2026-10-14 | D9: arrowheads ported from fig2dev (all 15 types, as its PostScript driver draws them), arrows on pie wedges, arcs follow their direction flag, PostScript dash patterns. 156 tests. |
| 2026-10-14 | A8: fig2dev reference tests (11 fixtures, fig2dev's SVG rasterised with resvg, no ghostscript); line widths now follow fig2dev. Decision: fig2dev is ported to Rust, no fig2dev or ghostscript at runtime. 153 tests. |
| 2026-10-14 | Scope set: every xfig function, cross-platform, no user-specific defaults; modern printing; xfig's libraries bundled; preferences planned (E1, S3); outlook: WYSIWYG LaTeX. |
| 2026-10-14 | Starts with no mode, as xfig: "READY. Select a mode or load a file", empty mouse indicator, canvas clicks ignored until a mode is chosen. |
| 2026-10-14 | Attribute panel pictures are xfig's bitmaps (`w_icons.c`) on the buttons and in the choice panels; Arrow Mode in xfig's order (none, forward, both, backward); Grid Mode offers xfig's four grid steps. |
| 2026-10-14 | Main menus cloned from xfig (`w_cmdpanel.c`): File, Edit, View, Snap, Help with xfig's entries, order, dividers and accelerators; entries RustFig lacks greyed with their plan task; Edit › Global settings holds RustFig's preferences; the Object and Tools menus remain in the modern layout. |
| 2026-10-14 | Mode panel with xfig's own icons (`w_icons.c` bitmaps) in xfig's order; xfig modes RustFig lacks (library, open compound, join/split, chop, tangent, measure) shown greyed with their plan task; RustFig's tools without an xfig mode (Select, Open/close, X-splines, separate ellipse tools) are in the Tools menu only. |
| 2026-10-14 | W7: xfig's keyboard accelerators (`app-defaults/Fig`): Meta (Alt/Option) menu keys, physical-Ctrl mode keys, xfig's mode letters (s spline, Shift-a arrows, …); menus show "Meta-N"; on by default, View › xfig keyboard accelerators. Phase W done. 143 tests. |
| 2026-10-14 | W4: xfig's Snap menu (`w_snap.c`, `w_intersect.c`): Hold, Release, Endpoint, Midpoint, Nearest, Focus, Normal, Tangent, Intersection, Diameter for the next clicked point, with the Snap Mode box above the depth panel; curves use their flattened outlines, so splines snap too. 141 tests. |
| 2026-10-14 | W6: xfig's rulers (`w_rulers.c`: tick and label tables by zoom and units, cursor marker, left/right pan by 1/2 inch or 1 cm, middle drag, Shift ×5) and units box in the ruler corner (left: pan to origin; right: Unit menu with ruler units, figure unit and scale). Parity ≈61 %, 131 tests. |
| 2026-10-14 | W5: xfig's attribute panel (`w_indpanel.c`): its buttons in its order with value pictures, only the attributes the mode uses, choice panels (fill styles and patterns, colours, line styles, arrows, fonts, …), middle/right buttons step values; new Arc Type and Text Step. 125 tests. |
| 2026-10-14 | S2: xfig's mouse indicator layout and wording; ellipses finish with either button, text mode's middle button finishes and right cancels, as in xfig. |
| 2026-10-14 | S0–S1: source map of xfig and fig2dev (`docs/source-map.md`); xfig's draw, save and pick order (by object type within a depth, gray depths behind). 121 tests. |
| 2026-10-14 | W3: xfig's depth panel (object counts, drag across checkboxes, All On/Off/Toggle, Gray/Blank); depths that are off can't be picked or edited; export of active layers only. 118 tests. |
| 2026-10-14 | W2: xfig's three-button mouse per mode (left/middle/right, right cancels; freehand, arc by centre, constrained move, delete region, copy & flip/rotate, update settings) with a modern scheme as a setting; Alt/Ctrl+click for two-button mice; indicator checked against the tools by a test. Parity ≈60 %, 110 tests. |
| 2026-10-13 | W1: classic xfig window layout (menus File/Edit/View/Snap/Help, file name, message panel with history, mouse-function indicator, units box, depth panel shown), headless UI tests with egui_kittest. |
| 2026-10-13 | M0–M3: workspace, geometry, model, Fig 3.x reader/writer, renderer (screen, PNG, SVG, EPS, PDF), engine with 124 commands and 38 tools, egui app, control channel, MCP server, CLI, xtask gates. Parity ≈58 %. |
