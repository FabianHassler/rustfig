# RustFig roadmap

**Status (2026-10-18):** Stage **Alpha**. Overall xfig parity **≈65 %**
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
| Overall parity (weighted) | ≈65 % |
| Commands (all reachable from the control channel and MCP) | 133 engine (incl. 38 tool modes) + 17 UI |
| Interactive tools (xfig modes) | 38 |
| Tests | 509 (unit, integration, property-based fuzzing, headless UI, fig2dev reference comparison) |
| Fig format | read 1.3–3.2, write 3.2 |
| Export formats | all through fig2dev's ported drivers (`convert`): PDF, SVG, EPS, PNG, GIF, JPEG, TIFF, PPM, PCX, XBM, XPM, EPS+PDF pair, LaTeX box from its scene writers (PostScript without page layout yet); LaTeX picture, epic/eepic/eepicemu, PiCTeX, TeXtyl, tpic, pic, pstex/pdftex, pict2e, CGM, HPGL, DXF, MetaPost, METAFONT, ge, image map, Gerber, shape, TikZ, PSTricks, EMF, Tk/Perl-Tk from ported fig2dev drivers (every fig2dev language except `sld`); fig2dev's own suite 79 pass / 9 fail / 1 skipped |
| Perf, 50k paths (render fit / hit test / open / edit + undo) | 260 ms / 0.1 ms / 20 ms / 0.9 ms |

## Parity by dimension

| Dimension | Parity | Details |
|-----------|--------|---------|
| Drawing modes | 88 % | all xfig shapes except library objects; freehand and arc by centre |
| Editing modes | 70 % | missing open-compound, join/split, chop, tangent, measure modes |
| Attributes panel | 75 % | xfig's per-mode buttons; missing colour editor, latex/manhattan positioning, smart links |
| File format | 92 % | Fig 1.3–3.2 read; exact pre-3.2 spline conversion missing |
| Export | 66 % | every fig2dev language but `sld`; PostScript page layout and previews, hardening passes missing |
| Text | 55 % | fallback fonts, no character map or LaTeX special text |
| View & navigation | 96 % | classic xfig layout, xfig's three-button mouse (or a modern scheme), depth panel, rulers and units box, object snaps; zoom box missing |
| Undo | 100 % | multi-level (xfig: single level) |
| Libraries | 0 % | not started |
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
3. Preferences that survive restarts (E1) and xfig's command-line options (S3).
4. Phase B: fig2dev's drivers ported (LaTeX exports, bitmaps, PostScript, the rest), modern printing (PDF to the system print dialog).
5. Phase C, web: trunk packaging and browser file I/O.
6. Every remaining xfig function, including xfig's bundled figure libraries (D6) and D11.

## Progress log

| Date | Change |
|------|--------|
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
