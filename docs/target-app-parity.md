# Target-app parity assessment: RustFig vs xfig 3.2.9

**Status:** 2026-10-13, overall **≈58 %** (W1 landed). Revised whenever a task lands.

## Methodology

The reference is xfig 3.2.9 as described in its public user manual
(`old/xfig-3.2.9a/doc/html`) and the Fig format description (`FORMAT3.2`),
plus fig2dev's export languages. Each area gets a 0–100 % score for the share
of xfig's user-visible behaviour that RustFig reproduces. The overall figure
is the weighted mean. Weights reflect how much everyday xfig use depends on
the area. "Hours" estimates the remaining work to reach 100 %.

Calibration: 100 % means a user of the area notices no missing function;
50 % means the common half works; features that go beyond xfig don't raise
the score.

## Areas

| Area | Weight | Score | Missing | Hours |
|------|-------:|------:|---------|------:|
| Drawing modes | 12 | 85 % | library objects; xfig's click-to-close rules for some shapes | 16 |
| Editing modes | 12 | 65 % | open compound (edit in place), join/split lines, chop, tangent/normal, angle/length/area measure modes, copy to library | 60 |
| Attributes / indicator | 10 | 70 % | colour editor UI, point positioning modes (latex, manhattan, mountain), linked move, arrow types 10–14 exact shapes, dash-dot tick styles | 40 |
| File format | 10 | 85 % | reading Fig 1.x/2.x, exact pre-3.2 spline conversion | 24 |
| Export | 12 | 35 % | PostScript (multi-page, print layout), LaTeX family (TikZ, PSTricks, pict2e, eepic), GIF/JPEG/TIFF/PPM/XBM/XPM, CGM/EMF/HPGL/MetaPost/DXF | 120 |
| Text | 10 | 55 % | character map, LaTeX special-text export, exact PostScript font metrics, compose keys, spell check | 60 |
| View & navigation | 8 | 80 % | rulers, xfig's three-button mouse scheme, zoom box, object snaps | 18 |
| Undo | 4 | 100 % | — | 0 |
| Libraries | 5 | 0 % | library browser and placement | 40 |
| Printing | 5 | 10 % | print dialog and printer output | 30 |
| Settings & styles | 5 | 15 % | settings persistence, recent files, style manager, units dialog | 30 |
| Platforms | 7 | 50 % | Windows/Linux verification, web build and file I/O on the web | 40 |
| **Overall** | 100 | **≈58 %** | | **478** |

## Evidence of what ships

- Drawing tools: `crates/engine/src/tools/mod.rs` (38 tools; tests draw polylines, boxes and text and move objects).
- Commands: `crates/engine/src/cmd/*` (124), swept with hostile parameters in `crates/engine/tests/command_sweep.rs`.
- Fig I/O: `crates/format/src/fig_read.rs` and `fig_write.rs`; round trip and fuzzing in `crates/format/tests/`.
- Rendering: `crates/render/` (scene, tiny-skia raster, SVG, EPS, PDF; all 22 fill patterns; arrowheads; dashes).
- UI: `crates/ui-egui/` (mode panel, attribute panel, depth manager, edit/export/search/settings dialogs, menus with shortcuts).
- Automation: control channel `crates/ui-egui/src/control.rs`, MCP `crates/mcp/`, CLI `apps/cli/`.
- Main window: classic xfig layout (`crates/ui-egui/src/panels.rs` `classic_top`), mouse hints from the engine (`Engine::mouse_hints`), headless UI tests in `crates/ui-egui/tests/ui_smoke.rs`.
- Verified on macOS by driving the app over the control channel and reviewing screenshots.

## Revision history

| Date | Overall | Note |
|------|--------:|------|
| 2026-10-13 | 58 % | First assessment. |
| 2026-10-13 | 58 % | W1: View & navigation 75 → 80 % (rounds to the same overall). |
