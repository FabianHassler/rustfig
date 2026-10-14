# RustFig roadmap

**Status (2026-10-14):** Stage **Alpha**. Overall xfig parity **≈60 %**
([assessment](docs/target-app-parity.md)). The native app draws, edits,
saves and exports. The web build compiles; it is not yet packaged or tested in a browser.

## Stage

| Stage | Meaning | State |
|-------|---------|-------|
| Prototype | Core model, file format, rendering | done |
| **Alpha** | Usable for everyday figures; gaps in export formats, libraries, print | **current** |
| Beta | xfig parity in all areas except exotic export languages | next |
| 1.0 | Parity plus web, MCP and automation polish | later |

## Headline numbers

| Measure | Value |
|---------|-------|
| Overall parity (weighted) | ≈60 % |
| Commands (all reachable from the control channel and MCP) | 128 engine (incl. 38 tool modes) + 15 UI |
| Interactive tools (xfig modes) | 38 |
| Tests | 121 (unit, integration, property-based fuzzing, headless UI) |
| Fig format | read 3.0–3.2, write 3.2 |
| Export formats | PDF, SVG, EPS, PNG |
| Perf, 50k paths (render fit / hit test / open) | 330 ms / 0.1 ms / 17 ms |

## Parity by dimension

| Dimension | Parity | Details |
|-----------|--------|---------|
| Drawing modes | 88 % | all xfig shapes except library objects; freehand and arc by centre |
| Editing modes | 70 % | missing open-compound, join/split, chop, tangent, measure modes |
| Attributes panel | 70 % | missing colour editor, latex/manhattan positioning |
| File format | 85 % | Fig 1.x/2.x read missing |
| Export | 35 % | LaTeX family, bitmaps besides PNG, print missing |
| Text | 55 % | fallback fonts, no character map or LaTeX special text |
| View & navigation | 88 % | classic xfig layout, xfig's three-button mouse (or a modern scheme), xfig's depth panel; rulers and object snaps missing |
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

## Languages

English only. See [localization parity](docs/localization-parity.md).

## Upcoming

The phased plan is [docs/plan.md](docs/plan.md); every shortfall is ranked in
[docs/gaps.md](docs/gaps.md). Next up:
1. Phase W, xfig window parity (W1 layout, W2 mouse buttons, W3 depth panel done): Snap menu, per-mode attribute panel, rulers.
2. fig2dev: reference tests comparing RustFig with xfig's converter (A8), then fig2dev as an optional export backend (B0).
3. Phase A, hardening: shared undo snapshots, per-command cost, CI (Linux on every push, all OSes for releases), web build, UI tests, export polish, crash recovery, no UI stalls on large drawings.
4. Phase B, exports for LaTeX users: TikZ, PDF/EPS + LaTeX overlay, PostScript layout, printing.
5. Phase C, web: trunk packaging and browser file I/O.

## Progress log

| Date | Change |
|------|--------|
| 2026-10-14 | S0–S1: source map of xfig and fig2dev (`docs/source-map.md`); xfig's draw, save and pick order (by object type within a depth, gray depths behind). 121 tests. |
| 2026-10-14 | W3: xfig's depth panel (object counts, drag across checkboxes, All On/Off/Toggle, Gray/Blank); depths that are off can't be picked or edited; export of active layers only. 118 tests. |
| 2026-10-14 | W2: xfig's three-button mouse per mode (left/middle/right, right cancels; freehand, arc by centre, constrained move, delete region, copy & flip/rotate, update settings) with a modern scheme as a setting; Alt/Ctrl+click for two-button mice; indicator checked against the tools by a test. Parity ≈60 %, 110 tests. |
| 2026-10-13 | W1: classic xfig window layout (menus File/Edit/View/Snap/Help, file name, message panel with history, mouse-function indicator, units box, depth panel shown), headless UI tests with egui_kittest. |
| 2026-10-13 | M0–M3: workspace, geometry, model, Fig 3.x reader/writer, renderer (screen, PNG, SVG, EPS, PDF), engine with 124 commands and 38 tools, egui app, control channel, MCP server, CLI, xtask gates. Parity ≈58 %. |
