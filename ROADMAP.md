# RustFig roadmap

**Status (2026-10-14):** Stage **Alpha**. Overall xfig parity **≈61 %**
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
| Overall parity (weighted) | ≈61 % |
| Commands (all reachable from the control channel and MCP) | 132 engine (incl. 38 tool modes) + 17 UI |
| Interactive tools (xfig modes) | 38 |
| Tests | 153 (unit, integration, property-based fuzzing, headless UI, fig2dev reference comparison) |
| Fig format | read 3.0–3.2, write 3.2 |
| Export formats | PDF, SVG, EPS, PNG |
| Perf, 50k paths (render fit / hit test / open) | 330 ms / 0.1 ms / 17 ms |

## Parity by dimension

| Dimension | Parity | Details |
|-----------|--------|---------|
| Drawing modes | 88 % | all xfig shapes except library objects; freehand and arc by centre |
| Editing modes | 70 % | missing open-compound, join/split, chop, tangent, measure modes |
| Attributes panel | 75 % | xfig's per-mode buttons; missing colour editor, latex/manhattan positioning, smart links |
| File format | 85 % | Fig 1.x/2.x read missing |
| Export | 35 % | LaTeX family, bitmaps besides PNG, print missing |
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
1. Arrowheads and arcs as fig2dev draws them (D9, found by the A8 reference tests), then the fig2dev port (B0): every export language in Rust, no fig2dev or ghostscript at runtime.
2. Phase A, hardening: shared undo snapshots, per-command cost, CI (Linux on every push, all OSes for releases), web build, UI tests, export polish, crash recovery, no UI stalls on large drawings.
3. Preferences that survive restarts (E1) and xfig's command-line options (S3).
4. Phase B: fig2dev's drivers ported (LaTeX exports, bitmaps, PostScript, the rest), modern printing (PDF to the system print dialog).
5. Phase C, web: trunk packaging and browser file I/O.
6. Every remaining xfig function, including xfig's bundled figure libraries (D6) and D11.

## Progress log

| Date | Change |
|------|--------|
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
